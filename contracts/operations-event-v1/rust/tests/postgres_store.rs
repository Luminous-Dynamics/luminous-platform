use std::env;

use luminous_operations_event_contract::{
    AuthenticatedConnector, IngestOutcome, PostgresOperationsStore, StoreError,
};
use serde_json::{Value, json};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};

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
    reset_and_seed(&pool).await;

    let store = PostgresOperationsStore::from_pool(pool.clone());
    let auth = authenticated();

    let initial = fixture();
    assert_eq!(
        store.ingest_event(&auth, &initial).await.unwrap(),
        IngestOutcome::Accepted { outbox_sequence: 1 }
    );
    assert_eq!(row_count_snapshot(&pool).await, [1, 1, 1, 1, 1]);

    // observedat is receiver-local and excluded from replay identity.
    let mut replay = initial.clone();
    replay["observedat"] = json!("2026-10-09T08:00:09Z");
    assert_eq!(
        store.ingest_event(&auth, &replay).await.unwrap(),
        IngestOutcome::DuplicateEvent
    );
    assert_eq!(row_count_snapshot(&pool).await, [1, 1, 1, 1, 1]);

    // Reusing an accepted source+id with different content is a conflict. Keep
    // the original inbox result unchanged; retain only a digest-only receipt.
    let mut conflicting_event_identity = initial.clone();
    conflicting_event_identity["data"]["summary"] =
        json!("Different content reusing the same event identity");
    assert!(matches!(
        store.ingest_event(&auth, &conflicting_event_identity).await,
        Err(StoreError::EventIdentityConflict)
    ));
    assert_eq!(row_count_snapshot(&pool).await, [1, 1, 1, 1, 1]);
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 1);
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
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 1);

    // Same business effect delivered with a different CloudEvents ID is
    // deduplicated by its idempotency key, not merely by source+event ID.
    let mut alternate_delivery = initial.clone();
    alternate_delivery["id"] = json!("event-ci-same-effect-alternate-delivery");
    assert_eq!(
        store.ingest_event(&auth, &alternate_delivery).await.unwrap(),
        IngestOutcome::DuplicateEffect
    );
    assert_eq!(row_count_snapshot(&pool).await, [2, 1, 1, 1, 1]);

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
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 2);
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
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 3);
    assert_eq!(scalar_count(&pool, "outbox_events").await, 2);

    let conflicting_idempotency = revised_event(
        "revision-3",
        "event-ci-conflicting-effect",
        "idem-ci-revision-2",
        "Changed content under reused idempotency key",
    );
    let conflict = store.ingest_event(&auth, &conflicting_idempotency).await;
    assert!(matches!(conflict, Err(StoreError::IdempotencyConflict)));
    assert_eq!(scalar_count(&pool, "quarantined_events").await, 4);
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
    let mut scoped = pool.begin().await.unwrap();
    sqlx::raw_sql("SET LOCAL ROLE luminous_ops_app")
        .execute(&mut *scoped)
        .await
        .unwrap();
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

    let mut no_tenant = pool.begin().await.unwrap();
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
    assert!(store.acknowledge_outbox(TENANT, &lease_three.outbox_id, "worker-d").await.unwrap());
    assert!(store.claim_next_outbox(TENANT, "worker-e", 30).await.unwrap().is_none());
}
