use std::env;
use std::process::Command;

use luminous_operations_event_contract::{
    AuthenticatedConnector, IngestOutcome, OutboxFailureCode, OutboxFailureOutcome,
    PostgresOperationsStore, StoreError,
};
use serde_json::{Value, json};
use sqlx::{PgPool, Row, postgres::PgPoolOptions, types::Json};

const TENANT: &str = "tenant-demo-001";
const CONNECTION: &str = "connectwise-connection-demo";
const SOURCE: &str = "urn:luminousdynamics:integration:connectwise-connection-demo";
const INCIDENT: &str = "incident-local-7301";

fn authenticated() -> AuthenticatedConnector {
    AuthenticatedConnector {
        connection_id: CONNECTION.to_owned(),
        tenant_id: TENANT.to_owned(),
    }
}

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../examples/incident.snapshot.json"))
        .expect("checked-in synthetic fixture must be valid JSON")
}

fn revised_event(revision: &str, event_id: &str, idempotency_key: &str, summary: &str) -> Value {
    let mut event = fixture();
    event["id"] = json!(event_id);
    event["idempotencykey"] = json!(idempotency_key);
    event["data"]["source_reference"]["revision"] = json!(revision);
    event["data"]["summary"] = json!(summary);
    event
}

async fn scalar_count(pool: &PgPool, table: &str) -> i64 {
    let query = format!("SELECT COUNT(*)::bigint AS count FROM ops.{table}");
    sqlx::query(&query)
        .fetch_one(pool)
        .await
        .expect("table count query")
        .try_get("count")
        .expect("count column")
}

async fn reset_and_seed(pool: &PgPool) {
    sqlx::raw_sql(
        "TRUNCATE TABLE \
          ops.quarantined_events, ops.outbox_events, ops.incident_activity, \
          ops.business_effects, ops.inbox_events, ops.incident_heads, \
          ops.resource_mappings, ops.connector_connections, ops.tenants \
         RESTART IDENTITY CASCADE",
    )
    .execute(pool)
    .await
    .expect("clear isolated CI database");

    sqlx::query("INSERT INTO ops.tenants (tenant_id) VALUES ($1), ($2)")
        .bind(TENANT)
        .bind("tenant-other-001")
        .execute(pool)
        .await
        .expect("seed isolated tenants");

    sqlx::query(
        "INSERT INTO ops.connector_connections \
         (connection_id, tenant_id, source_uri, producer_system, revision_mode, revision_prefix) \
         VALUES ($1, $2, $3, 'connectwise-psa', 'numeric_suffix', 'revision-')",
    )
    .bind(CONNECTION)
    .bind(TENANT)
    .bind(SOURCE)
    .execute(pool)
    .await
    .expect("seed trusted connector configuration");

    sqlx::query(
        "INSERT INTO ops.resource_mappings \
         (tenant_id, connection_id, external_company_id, external_resource_type, external_resource_id, local_incident_id) \
         VALUES ($1, $2, 'company-42', 'service_ticket', '7301', $3)",
    )
    .bind(TENANT)
    .bind(CONNECTION)
    .bind(INCIDENT)
    .execute(pool)
    .await
    .expect("seed explicit source-to-local mapping");
}

async fn row_count_snapshot(pool: &PgPool) -> [i64; 5] {
    [
        scalar_count(pool, "inbox_events").await,
        scalar_count(pool, "business_effects").await,
        scalar_count(pool, "incident_heads").await,
        scalar_count(pool, "incident_activity").await,
        scalar_count(pool, "outbox_events").await,
    ]
}

#[tokio::test]
async fn postgres_transactional_inbox_outbox_and_rls_contract() {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must point at the ephemeral PostgreSQL 18 CI service");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("connect to PostgreSQL CI service");
    sqlx::raw_sql(include_str!("../deploy/postgres/00-operations-app-role.sql"))
        .execute(&pool)
        .await
        .expect("provision the non-login application role as the CI cluster administrator");
    PostgresOperationsStore::migrate(&pool)
        .await
        .expect("apply versioned SQLx migrations");
    // Create a restricted runtime login for the actual store pool. It must
    // explicitly SET ROLE into the NOLOGIN application role for each transaction.
    // The test database is ephemeral; this password is test-only.
    sqlx::raw_sql(
        "DO $role$ BEGIN \
           IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'luminous_ops_runtime_test') THEN \
             EXECUTE 'CREATE ROLE luminous_ops_runtime_test LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS PASSWORD ''runtime-test-password'''; \
           END IF; \
         END $role$; \
         ALTER ROLE luminous_ops_runtime_test LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS PASSWORD 'runtime-test-password'; \
         GRANT luminous_ops_app TO luminous_ops_runtime_test;",
    )
    .execute(&pool)
    .await
    .expect("provision restricted non-superuser runtime login");

    reset_and_seed(&pool).await;

    let app_database_url = env::var("APP_DATABASE_URL")
        .expect("APP_DATABASE_URL must use the restricted runtime login");
    let app_pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&app_database_url)
        .await
        .expect("connect as restricted non-superuser runtime login");
    // The NOINHERIT runtime login must not use application-table privileges
    // unless it explicitly assumes luminous_ops_app inside a transaction.
    assert!(
        sqlx::query("SELECT COUNT(*) FROM ops.tenants")
            .fetch_one(&app_pool)
            .await
            .is_err(),
        "runtime login must not inherit application-table privileges"
    );
    // A one-connection pool deliberately reuses the same backend connection for
    // the tenant-context leak/default-deny test below.
    let scope_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&app_database_url)
        .await
        .expect("connect scope probe as restricted runtime login");

    // Verify the runtime role can mutate only fields the store needs to mutate.
    // This prevents later refactors from quietly widening the persistence role.
    let privileges = sqlx::query(
        "SELECT \
           has_column_privilege('luminous_ops_app', 'ops.incident_heads', 'tenant_id', 'INSERT') AS head_insert_key, \
           has_column_privilege('luminous_ops_app', 'ops.incident_heads', 'state_payload', 'INSERT') AS head_insert_state, \
           has_column_privilege('luminous_ops_app', 'ops.inbox_events', 'outcome', 'INSERT') AS inbox_insert_outcome, \
           has_column_privilege('luminous_ops_app', 'ops.inbox_events', 'outcome', 'UPDATE') AS inbox_update_outcome, \
           has_column_privilege('luminous_ops_app', 'ops.inbox_events', 'content_digest', 'UPDATE') AS inbox_update_digest, \
           has_column_privilege('luminous_ops_app', 'ops.outbox_events', 'status', 'UPDATE') AS outbox_update_status, \
           has_column_privilege('luminous_ops_app', 'ops.outbox_events', 'attempts', 'UPDATE') AS outbox_update_attempts, \
           has_column_privilege('luminous_ops_app', 'ops.outbox_events', 'lease_owner', 'UPDATE') AS outbox_update_lease, \
           has_column_privilege('luminous_ops_app', 'ops.outbox_events', 'available_at', 'UPDATE') AS outbox_update_schedule, \
           has_column_privilege('luminous_ops_app', 'ops.outbox_events', 'delivered_at', 'UPDATE') AS outbox_update_delivered_at, \
           has_function_privilege('luminous_ops_app', 'ops.schedule_outbox_retry(text, text, text)', 'EXECUTE') AS legacy_retry_function, \
           has_function_privilege('luminous_ops_app', 'ops.claim_next_outbox(text, text, integer)', 'EXECUTE') AS outbox_claim_function, \
           has_function_privilege('luminous_ops_app', 'ops.acknowledge_outbox(text, text, text)', 'EXECUTE') AS outbox_ack_function, \
           has_function_privilege('luminous_ops_app', 'ops.record_outbox_failure(text, text, text, text)', 'EXECUTE') AS outbox_failure_function, \
           has_column_privilege('luminous_ops_app', 'ops.outbox_events', 'payload', 'UPDATE') AS outbox_update_payload"
    )
    .fetch_one(&pool)
    .await
    .expect("query column-level role privileges");
    assert!(privileges.try_get::<bool, _>("head_insert_key").unwrap());
    assert!(!privileges.try_get::<bool, _>("head_insert_state").unwrap());
    assert!(!privileges.try_get::<bool, _>("inbox_insert_outcome").unwrap());
    assert!(privileges.try_get::<bool, _>("inbox_update_outcome").unwrap());
    assert!(!privileges.try_get::<bool, _>("inbox_update_digest").unwrap());
    assert!(!privileges.try_get::<bool, _>("outbox_update_status").unwrap());
    assert!(!privileges.try_get::<bool, _>("outbox_update_attempts").unwrap());
    assert!(!privileges.try_get::<bool, _>("outbox_update_lease").unwrap());
    assert!(!privileges.try_get::<bool, _>("outbox_update_schedule").unwrap());
    assert!(!privileges.try_get::<bool, _>("outbox_update_delivered_at").unwrap());
    assert!(!privileges.try_get::<bool, _>("legacy_retry_function").unwrap());
    assert!(privileges.try_get::<bool, _>("outbox_claim_function").unwrap());
    assert!(privileges.try_get::<bool, _>("outbox_ack_function").unwrap());
    assert!(privileges.try_get::<bool, _>("outbox_failure_function").unwrap());
    assert!(!privileges.try_get::<bool, _>("outbox_update_payload").unwrap());

    let retry_owner = sqlx::query(
        "SELECT r.rolname::text AS rolname, r.rolsuper, r.rolbypassrls, r.rolcanlogin, \
                has_table_privilege(r.rolname, 'ops.outbox_events', 'SELECT') AS can_select_outbox, \
                has_column_privilege(r.rolname, 'ops.outbox_events', 'status', 'UPDATE') AS can_update_status, \
                has_column_privilege(r.rolname, 'ops.outbox_events', 'attempts', 'UPDATE') AS can_update_attempts, \
                has_column_privilege(r.rolname, 'ops.outbox_events', 'available_at', 'UPDATE') AS can_schedule, \
                has_column_privilege(r.rolname, 'ops.outbox_events', 'last_failure_code', 'UPDATE') AS can_record_failure, \
                has_column_privilege(r.rolname, 'ops.outbox_events', 'payload', 'UPDATE') AS can_rewrite_payload \
         FROM pg_proc AS p JOIN pg_roles AS r ON r.oid = p.proowner \
         WHERE p.oid = 'ops.record_outbox_failure(text,text,text,text)'::regprocedure"
    )
    .fetch_one(&pool)
    .await
    .expect("inspect exact SECURITY DEFINER owner");
    assert_eq!(retry_owner.try_get::<String, _>("rolname").unwrap(), "luminous_ops_retry_owner");
    assert!(!retry_owner.try_get::<bool, _>("rolsuper").unwrap());
    assert!(!retry_owner.try_get::<bool, _>("rolbypassrls").unwrap());
    assert!(!retry_owner.try_get::<bool, _>("rolcanlogin").unwrap());
    assert!(retry_owner.try_get::<bool, _>("can_select_outbox").unwrap());
    assert!(retry_owner.try_get::<bool, _>("can_update_status").unwrap());
    assert!(retry_owner.try_get::<bool, _>("can_update_attempts").unwrap());
    assert!(retry_owner.try_get::<bool, _>("can_schedule").unwrap());
    assert!(retry_owner.try_get::<bool, _>("can_record_failure").unwrap());
    assert!(!retry_owner.try_get::<bool, _>("can_rewrite_payload").unwrap());

    // All callable delivery-state routines must use the dedicated safe owner,
    // SECURITY DEFINER, and the fixed trusted search path.
    let safe_routines: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM pg_proc AS p \
         JOIN pg_roles AS r ON r.oid = p.proowner \
         WHERE p.oid IN ( \
           'ops.claim_next_outbox(text,text,integer)'::regprocedure, \
           'ops.acknowledge_outbox(text,text,text)'::regprocedure, \
           'ops.record_outbox_failure(text,text,text,text)'::regprocedure \
         ) \
           AND r.rolname = 'luminous_ops_retry_owner' \
           AND NOT r.rolsuper AND NOT r.rolbypassrls AND NOT r.rolcanlogin \
           AND p.prosecdef \
           AND p.proconfig @> ARRAY['search_path=pg_catalog, ops, pg_temp']"
    )
    .fetch_one(&pool)
    .await
    .expect("inspect all SECURITY DEFINER outbox functions");
    assert_eq!(safe_routines, 3, "every lifecycle function needs the dedicated owner and fixed search path");

    let runtime_can_assume_retry_owner: bool = sqlx::query_scalar(
        "SELECT pg_has_role('luminous_ops_runtime_test', 'luminous_ops_retry_owner', 'MEMBER')"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!runtime_can_assume_retry_owner, "runtime login cannot SET ROLE to the definer owner");

    // The database must reject state claiming an external resource that is not
    // explicitly mapped to the canonical local resource, even if application
    // code accidentally bypasses the Rust mapping check.
    let unmapped_head = sqlx::query(
        "INSERT INTO ops.incident_heads \
         (tenant_id, incident_id, source_connection_id, source_company_id, \
          source_resource_type, source_resource_id, current_revision, state_digest, \
          state_payload, outbox_sequence) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 1)",
    )
    .bind(TENANT)
    .bind("incident-unmapped-head")
    .bind(CONNECTION)
    .bind("company-42")
    .bind("service_ticket")
    .bind("ticket-does-not-exist")
    .bind("revision-1")
    .bind("a".repeat(64))
    .bind(Json(json!({"synthetic": true})))
    .execute(&pool)
    .await;
    assert!(unmapped_head.is_err(), "the relational mapping foreign key must reject an unmapped provider resource");

    let store = PostgresOperationsStore::from_pool(app_pool.clone());
    let auth = authenticated();

    // A bad first provider revision cannot become the baseline for an incident.
    let invalid_first_revision = revised_event(
        "opaque-token",
        "event-ci-invalid-first-revision",
        "idem-ci-invalid-first-revision",
        "Unorderable initial revision",
    );
    assert_eq!(
        store.ingest_event(&auth, &invalid_first_revision).await.unwrap(),
        IngestOutcome::Quarantined {
            reason_code: "REVISION_TOKEN_INVALID".to_owned()
        }
    );
    assert_eq!(scalar_count(&pool, "incident_heads").await, 0);
    assert_eq!(scalar_count(&pool, "outbox_events").await, 0);
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 1);
    assert!(matches!(
        store.ingest_event(&auth, &invalid_first_revision).await.unwrap(),
        IngestOutcome::Quarantined { reason_code } if reason_code == "REVISION_TOKEN_INVALID"
    ));
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 1);

    // A different payload reusing the quarantined event ID has its own digest
    // receipt. Replaying the original bytes must still return the original
    // revision-token reason, not the newer conflicting-identity reason.
    let mut conflicting_invalid_first = invalid_first_revision.clone();
    conflicting_invalid_first["data"]["summary"] =
        json!("Conflicting content for an already quarantined identity");
    assert!(matches!(
        store.ingest_event(&auth, &conflicting_invalid_first).await,
        Err(StoreError::EventIdentityConflict)
    ));
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 2);
    assert!(matches!(
        store.ingest_event(&auth, &invalid_first_revision).await.unwrap(),
        IngestOutcome::Quarantined { reason_code } if reason_code == "REVISION_TOKEN_INVALID"
    ));
    assert!(matches!(
        store.ingest_event(&auth, &conflicting_invalid_first).await,
        Err(StoreError::EventIdentityConflict)
    ));
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 2);

    let initial = fixture();
    assert_eq!(
        store.ingest_event(&auth, &initial).await.unwrap(),
        IngestOutcome::Accepted { outbox_sequence: 1 }
    );
    assert_eq!(row_count_snapshot(&pool).await, [2, 1, 1, 1, 1]);

    // A key reused with different semantics remains an idempotency conflict,
    // even when the competing event also carries a malformed revision token.
    let malformed_revision_key_conflict = revised_event(
        "opaque-token",
        "event-ci-invalid-revision-key-conflict",
        "connectwise-psa:company-42:ticket-7301:snapshot-v1",
        "Changed semantics with a malformed revision token",
    );
    assert!(matches!(
        store.ingest_event(&auth, &malformed_revision_key_conflict).await,
        Err(StoreError::IdempotencyConflict)
    ));
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 3);
    assert_eq!(row_count_snapshot(&pool).await, [3, 1, 1, 1, 1]);

    // observedat is receiver-local and excluded from replay identity.
    let mut replay = initial.clone();
    replay["observedat"] = json!("2026-10-09T08:00:09Z");
    assert_eq!(
        store.ingest_event(&auth, &replay).await.unwrap(),
        IngestOutcome::DuplicateEvent
    );
    assert_eq!(row_count_snapshot(&pool).await, [3, 1, 1, 1, 1]);

    // Reusing an accepted source+id with different content is a conflict. Keep
    // the original inbox result unchanged; retain only a digest-only receipt.
    let mut conflicting_event_identity = initial.clone();
    conflicting_event_identity["data"]["summary"] =
        json!("Different content reusing the same event identity");
    assert!(matches!(
        store.ingest_event(&auth, &conflicting_event_identity).await,
        Err(StoreError::EventIdentityConflict)
    ));
    assert_eq!(row_count_snapshot(&pool).await, [3, 1, 1, 1, 1]);
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 4);
    let original_outcome: String = sqlx::query(
        "SELECT outcome FROM ops.inbox_events \
         WHERE tenant_id = $1 AND connection_id = $2 AND source_uri = $3 AND event_id = $4",
    )
    .bind(TENANT)
    .bind(CONNECTION)
    .bind(SOURCE)
    .bind("0199f3a2-5b41-7a10-8d23-1a8bfe29d641")
    .fetch_one(&pool)
    .await
    .unwrap()
    .try_get("outcome")
    .unwrap();
    assert_eq!(original_outcome, "ACCEPTED");
    assert!(matches!(
        store.ingest_event(&auth, &conflicting_event_identity).await,
        Err(StoreError::EventIdentityConflict)
    ));
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 4);

    // Same business effect delivered with a different CloudEvents ID is
    // deduplicated by its idempotency key, not merely by source+event ID.
    let mut alternate_delivery = initial.clone();
    alternate_delivery["id"] = json!("event-ci-same-effect-alternate-delivery");
    assert_eq!(
        store.ingest_event(&auth, &alternate_delivery).await.unwrap(),
        IngestOutcome::DuplicateEffect
    );
    assert_eq!(row_count_snapshot(&pool).await, [4, 1, 1, 1, 1]);

    let revision_two = revised_event(
        "revision-2",
        "event-ci-revision-2",
        "idem-ci-revision-2",
        "Synthetic updated incident state",
    );
    assert_eq!(
        store.ingest_event(&auth, &revision_two).await.unwrap(),
        IngestOutcome::Accepted { outbox_sequence: 2 }
    );

    // Check idempotency before stale-revision handling: changed semantics under
    // an existing key must not be misreported as merely stale.
    let stale_key_conflict = revised_event(
        "revision-1",
        "event-ci-stale-key-conflict",
        "idem-ci-revision-2",
        "Changed semantics under an existing idempotency key",
    );
    assert!(matches!(
        store.ingest_event(&auth, &stale_key_conflict).await,
        Err(StoreError::IdempotencyConflict)
    ));
    assert_eq!(scalar_count(&pool, "outbox_events").await, 2);
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 5);
    assert!(matches!(
        store.ingest_event(&auth, &stale_key_conflict).await.unwrap(),
        IngestOutcome::Quarantined { reason_code } if reason_code == "IDEMPOTENCY_KEY_CONTENT_CONFLICT"
    ));

    let stale = revised_event(
        "revision-1",
        "event-ci-stale",
        "idem-ci-stale",
        "An old observed state",
    );
    assert_eq!(
        store.ingest_event(&auth, &stale).await.unwrap(),
        IngestOutcome::StaleRevision
    );
    assert_eq!(scalar_count(&pool, "outbox_events").await, 2);
    assert_eq!(
        store.ingest_event(&auth, &stale).await.unwrap(),
        IngestOutcome::StaleRevision
    );

    let stale_key_reuse = revised_event(
        "revision-3",
        "event-ci-stale-key-reuse",
        "idem-ci-stale",
        "Different semantics reusing a stale observation key",
    );
    assert!(matches!(
        store.ingest_event(&auth, &stale_key_reuse).await,
        Err(StoreError::IdempotencyConflict)
    ));
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 6);
    assert_eq!(scalar_count(&pool, "outbox_events").await, 2);

    sqlx::query(
        "UPDATE ops.connector_connections SET revision_mode = 'opaque', revision_prefix = NULL \
         WHERE tenant_id = $1 AND connection_id = $2",
    )
    .bind(TENANT)
    .bind(CONNECTION)
    .execute(&pool)
    .await
    .expect("switch to the explicitly opaque provider revision policy");

    let unordered = revised_event(
        "opaque-token",
        "event-ci-unordered",
        "idem-ci-unordered",
        "Unorderable provider revision",
    );
    assert_eq!(
        store.ingest_event(&auth, &unordered).await.unwrap(),
        IngestOutcome::Quarantined { reason_code: "REVISION_ORDER_UNPROVEN".to_owned() }
    );
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 7);
    assert_eq!(scalar_count(&pool, "outbox_events").await, 2);

    sqlx::query(
        "UPDATE ops.connector_connections SET revision_mode = 'numeric_suffix', revision_prefix = 'revision-' \
         WHERE tenant_id = $1 AND connection_id = $2",
    )
    .bind(TENANT)
    .bind(CONNECTION)
    .execute(&pool)
    .await
    .expect("restore the documented numeric-suffix revision policy");

    let conflicting_idempotency = revised_event(
        "revision-3",
        "event-ci-conflicting-effect",
        "idem-ci-revision-2",
        "Changed content under reused idempotency key",
    );
    let conflict = store.ingest_event(&auth, &conflicting_idempotency).await;
    assert!(matches!(conflict, Err(StoreError::IdempotencyConflict)));
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 8);
    assert_eq!(scalar_count(&pool, "outbox_events").await, 2);

    // Independent pooled connections race on one event identity. One state
    // mutation and one outbox row should commit.
    let revision_three = revised_event(
        "revision-3",
        "event-ci-revision-3",
        "idem-ci-revision-3",
        "Synthetic third revision",
    );
    let (left, right) = tokio::join!(
        store.ingest_event(&auth, &revision_three),
        store.ingest_event(&auth, &revision_three),
    );
    let outcomes = [left.unwrap(), right.unwrap()];
    assert_eq!(
        outcomes.iter().filter(|v| matches!(v, IngestOutcome::Accepted { outbox_sequence: 3 })).count(),
        1
    );
    assert_eq!(
        outcomes.iter().filter(|v| **v == IngestOutcome::DuplicateEvent).count(),
        1
    );
    assert_eq!(scalar_count(&pool, "outbox_events").await, 3);
    assert_eq!(scalar_count(&pool, "incident_activity").await, 3);

    let current_revision: String = sqlx::query(
        "SELECT current_revision FROM ops.incident_heads WHERE tenant_id = $1 AND incident_id = $2",
    )
    .bind(TENANT)
    .bind(INCIDENT)
    .fetch_one(&pool)
    .await
    .unwrap()
    .try_get("current_revision")
    .unwrap();
    assert_eq!(current_revision, "revision-3");

    // Cause a real PostgreSQL error at the final outbox insert. All preceding
    // writes in the transaction must roll back, including the state head.
    sqlx::raw_sql(
        "CREATE OR REPLACE FUNCTION ops.test_fail_outbox_insert() RETURNS trigger \
         LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic outbox failure'; END $$; \
         CREATE TRIGGER test_fail_outbox_insert BEFORE INSERT ON ops.outbox_events \
         FOR EACH ROW EXECUTE FUNCTION ops.test_fail_outbox_insert();",
    )
    .execute(&pool)
    .await
    .expect("install one-test rollback trigger");
    let before_failure = row_count_snapshot(&pool).await;
    let revision_four = revised_event(
        "revision-4",
        "event-ci-trigger-failure",
        "idem-ci-trigger-failure",
        "Must roll back before commit",
    );
    let failed = store.ingest_event(&auth, &revision_four).await;
    assert!(matches!(failed, Err(StoreError::Database(_))));
    assert_eq!(row_count_snapshot(&pool).await, before_failure);
    let current_revision_after_failure: String = sqlx::query(
        "SELECT current_revision FROM ops.incident_heads WHERE tenant_id = $1 AND incident_id = $2",
    )
    .bind(TENANT)
    .bind(INCIDENT)
    .fetch_one(&pool)
    .await
    .unwrap()
    .try_get("current_revision")
    .unwrap();
    assert_eq!(current_revision_after_failure, "revision-3");
    sqlx::raw_sql(
        "DROP TRIGGER test_fail_outbox_insert ON ops.outbox_events; \
         DROP FUNCTION ops.test_fail_outbox_insert();",
    )
    .execute(&pool)
    .await
    .expect("remove rollback trigger");

    // RLS defaults to no visible tenant rows if context is absent and correctly
    // isolates a configured tenant. Both role and tenant scope are transaction-local.
    let mut scoped = scope_pool.begin().await.unwrap();
    sqlx::raw_sql("SET LOCAL ROLE luminous_ops_app")
        .execute(&mut *scoped)
        .await
        .unwrap();
    let identity = sqlx::query(
        "SELECT session_user::text AS session_user, current_user::text AS current_user",
    )
    .fetch_one(&mut *scoped)
    .await
    .unwrap();
    assert_eq!(
        identity.try_get::<String, _>("session_user").unwrap(),
        "luminous_ops_runtime_test"
    );
    assert_eq!(
        identity.try_get::<String, _>("current_user").unwrap(),
        "luminous_ops_app"
    );
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(TENANT)
        .execute(&mut *scoped)
        .await
        .unwrap();
    let visible: i64 = sqlx::query("SELECT COUNT(*)::bigint AS count FROM ops.tenants")
        .fetch_one(&mut *scoped)
        .await
        .unwrap()
        .try_get("count")
        .unwrap();
    assert_eq!(visible, 1);
    let foreign_visible: i64 = sqlx::query(
        "SELECT COUNT(*)::bigint AS count FROM ops.tenants WHERE tenant_id = 'tenant-other-001'",
    )
    .fetch_one(&mut *scoped)
    .await
    .unwrap()
    .try_get("count")
    .unwrap();
    assert_eq!(foreign_visible, 0);
    scoped.commit().await.unwrap();

    let mut no_tenant = scope_pool.begin().await.unwrap();
    sqlx::raw_sql("SET LOCAL ROLE luminous_ops_app")
        .execute(&mut *no_tenant)
        .await
        .unwrap();
    let default_deny: i64 = sqlx::query("SELECT COUNT(*)::bigint AS count FROM ops.tenants")
        .fetch_one(&mut *no_tenant)
        .await
        .unwrap()
        .try_get("count")
        .unwrap();
    assert_eq!(default_deny, 0);
    no_tenant.commit().await.unwrap();

    // Leased predecessors block later messages. A wrong owner or expired lease
    // cannot acknowledge; a later worker can safely reclaim and retry.
    let lease_one = store.claim_next_outbox(TENANT, "worker-a", 30).await.unwrap().unwrap();
    assert_eq!(lease_one.sequence_no, 1);
    assert!(store.claim_next_outbox(TENANT, "worker-b", 30).await.unwrap().is_none());
    assert!(!store.acknowledge_outbox(TENANT, &lease_one.outbox_id, "worker-b").await.unwrap());
    assert!(store.acknowledge_outbox(TENANT, &lease_one.outbox_id, "worker-a").await.unwrap());

    let lease_two = store.claim_next_outbox(TENANT, "worker-b", 30).await.unwrap().unwrap();
    assert_eq!(lease_two.sequence_no, 2);
    sqlx::query(
        "UPDATE ops.outbox_events SET lease_until = clock_timestamp() - interval '1 second' \
         WHERE tenant_id = $1 AND outbox_id = $2",
    )
    .bind(TENANT)
    .bind(&lease_two.outbox_id)
    .execute(&pool)
    .await
    .expect("expire lease for deterministic recovery test");
    assert!(!store.acknowledge_outbox(TENANT, &lease_two.outbox_id, "worker-b").await.unwrap());
    let lease_two_reclaimed = store.claim_next_outbox(TENANT, "worker-c", 30).await.unwrap().unwrap();
    assert_eq!(lease_two_reclaimed.sequence_no, 2);
    assert_eq!(lease_two_reclaimed.attempts, 2);
    assert!(store.acknowledge_outbox(TENANT, &lease_two_reclaimed.outbox_id, "worker-c").await.unwrap());

    let lease_three = store.claim_next_outbox(TENANT, "worker-d", 30).await.unwrap().unwrap();
    assert_eq!(lease_three.sequence_no, 3);

    // Even an otherwise valid worker cannot reschedule a row by presenting a
    // tenant parameter that differs from this transaction's RLS context.
    let mut mismatched_tenant = scope_pool.begin().await.unwrap();
    sqlx::raw_sql("SET LOCAL ROLE luminous_ops_app")
        .execute(&mut *mismatched_tenant)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(TENANT)
        .execute(&mut *mismatched_tenant)
        .await
        .unwrap();
    let mismatched_tenant_rescheduled: String = sqlx::query_scalar(
        "SELECT ops.record_outbox_failure($1, $2, $3, $4)"
    )
    .bind("tenant-other-001")
    .bind(&lease_three.outbox_id)
    .bind("worker-d")
    .bind("TRANSIENT_NETWORK")
    .fetch_one(&mut *mismatched_tenant)
    .await
    .unwrap();
    assert_eq!(mismatched_tenant_rescheduled, "STALE_LEASE",
        "function must bind parameter tenant to RLS tenant context");
    mismatched_tenant.commit().await.unwrap();

    // SQL-level callers cannot inject arbitrary or provider-controlled failure
    // text into the durable failure classification.
    let mut invalid_failure_code = scope_pool.begin().await.unwrap();
    sqlx::raw_sql("SET LOCAL ROLE luminous_ops_app")
        .execute(&mut *invalid_failure_code)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(TENANT)
        .execute(&mut *invalid_failure_code)
        .await
        .unwrap();
    let invalid_code_result = sqlx::query_scalar::<_, String>(
        "SELECT ops.record_outbox_failure($1, $2, $3, $4)"
    )
    .bind(TENANT)
    .bind(&lease_three.outbox_id)
    .bind("worker-d")
    .bind("raw provider error must never be persisted")
    .fetch_one(&mut *invalid_failure_code)
    .await;
    assert!(invalid_code_result.is_err(), "database function rejects unrecognized failure strings");
    invalid_failure_code.rollback().await.unwrap();
    let failure_text_was_not_persisted: bool = sqlx::query(
        "SELECT last_failure_code IS NULL AS clean FROM ops.outbox_events \
         WHERE tenant_id = $1 AND outbox_id = $2"
    )
    .bind(TENANT)
    .bind(&lease_three.outbox_id)
    .fetch_one(&pool)
    .await
    .unwrap()
    .try_get("clean")
    .unwrap();
    assert!(failure_text_was_not_persisted, "invalid raw error must leave no failure record");

    assert_eq!(
        store.record_outbox_failure(TENANT, &lease_three.outbox_id, "worker-not-owner",
            OutboxFailureCode::TransientNetwork).await.unwrap(),
        OutboxFailureOutcome::StaleLease,
        "a different worker cannot record failure or release the live lease"
    );
    assert_eq!(
        store.record_outbox_failure(TENANT, &lease_three.outbox_id, "worker-d",
            OutboxFailureCode::TransientNetwork).await.unwrap(),
        OutboxFailureOutcome::Rescheduled,
        "the current owner may return a transient failure to the bounded retry schedule"
    );
    assert!(store.claim_next_outbox(TENANT, "worker-e", 30).await.unwrap().is_none(),
        "the failed event must not be immediately claimable during backoff");
    let first_backoff_is_bounded: bool = sqlx::query(
        "SELECT available_at - last_failure_at BETWEEN interval '4 seconds' AND interval '6 seconds' AS bounded \
         FROM ops.outbox_events WHERE tenant_id = $1 AND outbox_id = $2",
    )
    .bind(TENANT)
    .bind(&lease_three.outbox_id)
    .fetch_one(&pool)
    .await
    .unwrap()
    .try_get("bounded")
    .unwrap();
    assert!(first_backoff_is_bounded, "first failed attempt schedules approximately five seconds of backoff");

    // Fast-forward only the disposable test database instead of sleeping for
    // the production retry delay.
    sqlx::query(
        "UPDATE ops.outbox_events SET available_at = clock_timestamp() - interval '1 second' \
         WHERE tenant_id = $1 AND outbox_id = $2",
    )
    .bind(TENANT)
    .bind(&lease_three.outbox_id)
    .execute(&pool)
    .await
    .expect("make scheduled retry eligible in isolated test database");
    let lease_three_retry = store.claim_next_outbox(TENANT, "worker-e", 30).await.unwrap().unwrap();
    assert_eq!(lease_three_retry.sequence_no, 3);
    assert_eq!(lease_three_retry.attempts, 2);
    assert_eq!(
        store.record_outbox_failure(TENANT, &lease_three_retry.outbox_id, "worker-e",
            OutboxFailureCode::RemoteRateLimited).await.unwrap(),
        OutboxFailureOutcome::Rescheduled
    );
    let second_backoff_is_bounded: bool = sqlx::query(
        "SELECT available_at - last_failure_at BETWEEN interval '9 seconds' AND interval '11 seconds' AS bounded \
         FROM ops.outbox_events WHERE tenant_id = $1 AND outbox_id = $2",
    )
    .bind(TENANT)
    .bind(&lease_three_retry.outbox_id)
    .fetch_one(&pool)
    .await
    .unwrap()
    .try_get("bounded")
    .unwrap();
    assert!(second_backoff_is_bounded, "second failed attempt doubles the delay to approximately ten seconds");

    sqlx::query(
        "UPDATE ops.outbox_events SET available_at = clock_timestamp() - interval '1 second' \
         WHERE tenant_id = $1 AND outbox_id = $2",
    )
    .bind(TENANT)
    .bind(&lease_three_retry.outbox_id)
    .execute(&pool)
    .await
    .expect("make the second scheduled retry eligible in the isolated test database");
    let lease_three_final = store.claim_next_outbox(TENANT, "worker-f", 30).await.unwrap().unwrap();
    assert_eq!(lease_three_final.sequence_no, 3);
    assert_eq!(lease_three_final.attempts, 3);
    assert!(store.acknowledge_outbox(TENANT, &lease_three_final.outbox_id, "worker-f").await.unwrap());
    assert!(store.claim_next_outbox(TENANT, "worker-g", 30).await.unwrap().is_none());

    // Race two distinct deliveries that reuse one idempotency key with
    // different semantics. The per-key advisory lock must make the loser an
    // idempotency conflict, not a stale/equal-revision outcome, regardless of
    // which process wins the scheduling race.
    let race_left = revised_event(
        "revision-4",
        "event-ci-idem-race-left",
        "idem-ci-racing-conflicting-effects",
        "Synthetic winner candidate A",
    );
    let race_right = revised_event(
        "revision-4",
        "event-ci-idem-race-right",
        "idem-ci-racing-conflicting-effects",
        "Synthetic winner candidate B with different semantics",
    );
    let before_race = row_count_snapshot(&pool).await;
    let quarantine_before_race = scalar_count(&pool, "quarantined_events").await;
    let (race_left_result, race_right_result) = tokio::join!(
        store.ingest_event(&auth, &race_left),
        store.ingest_event(&auth, &race_right),
    );
    let accepted = |result: &Result<IngestOutcome, StoreError>| {
        matches!(result, Ok(IngestOutcome::Accepted { outbox_sequence: 4 }))
    };
    let conflicted = |result: &Result<IngestOutcome, StoreError>| {
        matches!(result, Err(StoreError::IdempotencyConflict))
    };
    assert_eq!(
        accepted(&race_left_result) as usize + accepted(&race_right_result) as usize,
        1
    );
    assert_eq!(
        conflicted(&race_left_result) as usize + conflicted(&race_right_result) as usize,
        1
    );
    assert_eq!(
        row_count_snapshot(&pool).await,
        [
            before_race[0] + 2,
            before_race[1] + 1,
            before_race[2],
            before_race[3] + 1,
            before_race[4] + 1,
        ]
    );
    assert_eq!(
        scalar_count(&pool, "quarantined_events").await,
        quarantine_before_race + 1
    );

    // Drain the one accepted race result so the next sequence is available for
    // the crash-before-acknowledgement probe.
    let race_lease = store.claim_next_outbox(TENANT, "worker-idem-race", 30).await.unwrap().unwrap();
    assert_eq!(race_lease.sequence_no, 4);
    assert!(store.acknowledge_outbox(TENANT, &race_lease.outbox_id, "worker-idem-race").await.unwrap());
    assert!(store.claim_next_outbox(TENANT, "worker-idem-race", 30).await.unwrap().is_none());

    // Kill a separate receiver process after ingest_event has committed but
    // before the caller can acknowledge the source event. Redelivery after the
    // process death must return DuplicateEvent without another local effect.
    let before_crash = row_count_snapshot(&pool).await;
    let probe = Command::new(env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "crash_probe_after_commit_before_ack",
            "--ignored",
            "--nocapture",
        ])
        .env("LUMINOUS_OPS_CRASH_PROBE", "1")
        .status()
        .expect("spawn crash-probe child process");
    assert_eq!(
        probe.code(),
        Some(73),
        "child must terminate with the deliberate post-commit exit code"
    );
    let after_crash = row_count_snapshot(&pool).await;
    assert_eq!(
        after_crash,
        [
            before_crash[0] + 1,
            before_crash[1] + 1,
            before_crash[2],
            before_crash[3] + 1,
            before_crash[4] + 1,
        ],
        "the child transaction must be durable despite process termination"
    );

    let crash_event = revised_event(
        "revision-5",
        "event-ci-crash-before-ack",
        "idem-ci-crash-before-ack",
        "Synthetic post-commit crash/replay scenario",
    );
    assert_eq!(
        store.ingest_event(&auth, &crash_event).await.unwrap(),
        IngestOutcome::DuplicateEvent
    );
    assert_eq!(row_count_snapshot(&pool).await, after_crash);
    let recovered_lease = store.claim_next_outbox(TENANT, "worker-after-crash", 30).await.unwrap().unwrap();
    assert_eq!(recovered_lease.sequence_no, 5);
    assert!(store.acknowledge_outbox(TENANT, &recovered_lease.outbox_id, "worker-after-crash").await.unwrap());
    assert!(store.claim_next_outbox(TENANT, "worker-after-crash", 30).await.unwrap().is_none());

    // A permanent failure is dead-lettered immediately, but remains an
    // undelivered predecessor and therefore blocks the next incident event.
    sqlx::query(
        "INSERT INTO ops.resource_mappings \
         (tenant_id, connection_id, external_company_id, external_resource_type, external_resource_id, local_incident_id) \
         VALUES ($1, $2, 'company-42', 'service_ticket', '7302', 'incident-dead-letter-order')",
    )
    .bind(TENANT)
    .bind(CONNECTION)
    .execute(&pool)
    .await
    .expect("seed a second explicitly mapped incident");
    let mut dead_letter_first = revised_event(
        "revision-1", "event-ci-dead-letter-first", "idem-ci-dead-letter-first",
        "Synthetic permanent delivery failure",
    );
    dead_letter_first["data"]["source_reference"]["resource_id"] = json!("7302");
    dead_letter_first["subject"] = json!("incident/incident-dead-letter-order");
    assert_eq!(
        store.ingest_event(&auth, &dead_letter_first).await.unwrap(),
        IngestOutcome::Accepted { outbox_sequence: 1 }
    );
    let mut dead_letter_successor = revised_event(
        "revision-2", "event-ci-dead-letter-successor", "idem-ci-dead-letter-successor",
        "Must remain blocked behind the dead-lettered predecessor",
    );
    dead_letter_successor["data"]["source_reference"]["resource_id"] = json!("7302");
    dead_letter_successor["subject"] = json!("incident/incident-dead-letter-order");
    assert_eq!(
        store.ingest_event(&auth, &dead_letter_successor).await.unwrap(),
        IngestOutcome::Accepted { outbox_sequence: 2 }
    );
    let dead_letter_lease = store.claim_next_outbox(TENANT, "worker-permanent", 30)
        .await.unwrap().expect("claim the first event for the isolated incident");
    assert_eq!(dead_letter_lease.sequence_no, 1);
    assert_eq!(
        store.record_outbox_failure(TENANT, &dead_letter_lease.outbox_id, "worker-permanent",
            OutboxFailureCode::AuthenticationRejected).await.unwrap(),
        OutboxFailureOutcome::DeadLettered
    );
    let dead_letters = store.list_dead_lettered_outbox(TENANT, 100).await.unwrap();
    assert_eq!(dead_letters.len(), 1);
    assert_eq!(dead_letters[0].outbox_id, dead_letter_lease.outbox_id);
    assert_eq!(dead_letters[0].failure_code, "AUTHENTICATION_REJECTED");
    assert_eq!(dead_letters[0].attempts, 1);
    assert!(matches!(
        store.list_dead_lettered_outbox(TENANT, 101).await,
        Err(StoreError::InvalidOutboxQuery)
    ));
    assert!(store.claim_next_outbox(TENANT, "worker-blocked", 30).await.unwrap().is_none(),
        "a dead-lettered predecessor must not be skipped by a later incident event");

    sqlx::query(
        "INSERT INTO ops.resource_mappings \
         (tenant_id, connection_id, external_company_id, external_resource_type, external_resource_id, local_incident_id) \
         VALUES ($1, $2, 'company-42', 'service_ticket', '7303', 'incident-attempt-ceiling')",
    )
    .bind(TENANT)
    .bind(CONNECTION)
    .execute(&pool)
    .await
    .expect("seed an isolated attempt-ceiling incident");
    let mut exhausted_event = revised_event(
        "revision-1", "event-ci-attempt-limit", "idem-ci-attempt-limit",
        "Synthetic attempt ceiling probe",
    );
    exhausted_event["data"]["source_reference"]["resource_id"] = json!("7303");
    exhausted_event["subject"] = json!("incident/incident-attempt-ceiling");
    assert_eq!(
        store.ingest_event(&auth, &exhausted_event).await.unwrap(),
        IngestOutcome::Accepted { outbox_sequence: 1 }
    );
    sqlx::query(
        "UPDATE ops.outbox_events SET attempts = 11 \
         WHERE tenant_id = $1 AND source_event_id = $2 AND status = 'PENDING'",
    )
    .bind(TENANT)
    .bind("event-ci-attempt-limit")
    .execute(&pool)
    .await
    .expect("position test fixture immediately before the final attempt");
    let last_attempt = store.claim_next_outbox(TENANT, "worker-attempt-limit", 30)
        .await.unwrap().expect("claim the final permitted attempt");
    assert_eq!(last_attempt.attempts, 12);
    assert_eq!(
        store.record_outbox_failure(TENANT, &last_attempt.outbox_id, "worker-attempt-limit",
            OutboxFailureCode::Unclassified).await.unwrap(),
        OutboxFailureOutcome::DeadLettered
    );

    sqlx::query(
        "INSERT INTO ops.resource_mappings \
         (tenant_id, connection_id, external_company_id, external_resource_type, external_resource_id, local_incident_id) \
         VALUES ($1, $2, 'company-42', 'service_ticket', '7304', 'incident-expired-attempt-ceiling')",
    )
    .bind(TENANT)
    .bind(CONNECTION)
    .execute(&pool)
    .await
    .expect("seed an isolated expired-lease incident");
    let mut expired_ceiling_event = revised_event(
        "revision-1", "event-ci-expired-attempt-limit", "idem-ci-expired-attempt-limit",
        "Synthetic expired-lease ceiling probe",
    );
    expired_ceiling_event["data"]["source_reference"]["resource_id"] = json!("7304");
    expired_ceiling_event["subject"] = json!("incident/incident-expired-attempt-ceiling");
    assert_eq!(
        store.ingest_event(&auth, &expired_ceiling_event).await.unwrap(),
        IngestOutcome::Accepted { outbox_sequence: 1 }
    );
    let final_lease = store.claim_next_outbox(TENANT, "worker-crash-at-limit", 30)
        .await.unwrap().expect("claim before simulating a final-attempt crash");
    sqlx::query(
        "UPDATE ops.outbox_events SET attempts = 12, lease_until = clock_timestamp() - interval '1 second' \
         WHERE tenant_id = $1 AND outbox_id = $2",
    )
    .bind(TENANT)
    .bind(&final_lease.outbox_id)
    .execute(&pool)
    .await
    .expect("simulate an expired lease at the attempt ceiling");
    assert!(store.claim_next_outbox(TENANT, "worker-reclaim-at-limit", 30).await.unwrap().is_none(),
        "an expired final attempt must dead-letter instead of being claimed again");
    let dead_letters = store.list_dead_lettered_outbox(TENANT, 100).await.unwrap();
    assert_eq!(dead_letters.len(), 3);
    assert!(dead_letters.iter().any(|row| row.outbox_id == final_lease.outbox_id
        && row.failure_code == "ATTEMPT_LIMIT_EXHAUSTED"));
}

#[tokio::test]
#[ignore = "child process probe invoked by the PostgreSQL recovery integration scenario"]
async fn crash_probe_after_commit_before_ack() {
    assert_eq!(
        env::var("LUMINOUS_OPS_CRASH_PROBE").as_deref(),
        Ok("1"),
        "this test is only expected to run in the parent test's child process"
    );
    let database_url = env::var("APP_DATABASE_URL")
        .expect("APP_DATABASE_URL must use the restricted runtime login");
    let store = PostgresOperationsStore::connect(&database_url, 2)
        .await
        .expect("connect from isolated crash-probe process");
    let outcome = store
        .ingest_event(
            &authenticated(),
            &revised_event(
                "revision-5",
                "event-ci-crash-before-ack",
                "idem-ci-crash-before-ack",
                "Synthetic post-commit crash/replay scenario",
            ),
        )
        .await
        .expect("commit durable local effect before simulated process death");
    assert_eq!(outcome, IngestOutcome::Accepted { outbox_sequence: 5 });

    // Calling process exit deliberately skips destructors, approximating abrupt
    // process loss immediately after the database commit has returned.
    std::process::exit(73);
}
