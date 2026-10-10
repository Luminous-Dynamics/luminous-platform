//! PostgreSQL-backed transactional Operations Event receiver and outbox.
//!
//! Callers must authenticate the connector and derive AuthenticatedConnector
//! from trusted server-side configuration. Tenant identity is never accepted
//! from the CloudEvent as authority. RLS is defense in depth against accidental
//! tenant-scope mistakes, not protection against a compromised application role
//! that can set arbitrary tenant context.
//!
//! This module commits inbox identity, business idempotency, incident state,
//! append-only activity, and outbox intent atomically. It never calls external
//! providers or Holochain inside the database transaction. Dispatch is at-least
//! once, not exactly once.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use sqlx::{
    PgPool, Postgres, Row, Transaction,
    postgres::PgPoolOptions,
    types::Json,
};
use thiserror::Error;

use crate::{TrustedConnectionContext, ValidationIssue, validate_event, validate_event_for_connection};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("event contract rejected")]
    ContractRejected(Vec<ValidationIssue>),
    #[error("authenticated connector is unavailable in the selected tenant")]
    ConnectorUnavailable,
    #[error("event identity was reused with different content")]
    EventIdentityConflict,
    #[error("business idempotency key was reused with different semantics")]
    IdempotencyConflict,
    #[error("invalid outbox lease request")]
    InvalidLeaseRequest,
    #[error("outbox query limit must be between 1 and 100")]
    InvalidOutboxQuery,
    #[error("database returned an unknown outbox failure outcome")]
    UnexpectedOutboxFailureOutcome,
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("database migration failed")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("validated event is missing a required typed field")]
    InvalidEvent,
    #[error("revision policy is malformed")]
    InvalidRevisionPolicy,
    #[error("inbox row has an unexpected nonterminal state")]
    IncompleteEventState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedConnector {
    /// Derived from authenticated server-side connection, not event fields.
    pub connection_id: String,
    /// Derived from trusted principal/configuration, not event tenantid.
    pub tenant_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IngestOutcome {
    Accepted { outbox_sequence: i64 },
    DuplicateEvent,
    DuplicateEffect,
    DuplicateRevision,
    StaleRevision,
    Quarantined { reason_code: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutboxLease {
    pub tenant_id: String,
    pub outbox_id: String,
    pub incident_id: String,
    pub sequence_no: i64,
    pub payload: Value,
    pub attempts: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutboxFailureCode {
    TransientNetwork,
    RemoteRateLimited,
    RemoteServerError,
    RemoteAcknowledgementUnknown,
    AuthenticationRejected,
    AuthorizationRejected,
    DestinationMismatch,
    TlsIdentityRejected,
    PayloadRejected,
    RemoteContractMismatch,
    Unclassified,
}

impl OutboxFailureCode {
    fn as_db_code(self) -> &'static str {
        match self {
            Self::TransientNetwork => "TRANSIENT_NETWORK",
            Self::RemoteRateLimited => "REMOTE_RATE_LIMITED",
            Self::RemoteServerError => "REMOTE_SERVER_ERROR",
            Self::RemoteAcknowledgementUnknown => "REMOTE_ACK_UNKNOWN",
            Self::AuthenticationRejected => "AUTHENTICATION_REJECTED",
            Self::AuthorizationRejected => "AUTHORIZATION_REJECTED",
            Self::DestinationMismatch => "DESTINATION_MISMATCH",
            Self::TlsIdentityRejected => "TLS_IDENTITY_REJECTED",
            Self::PayloadRejected => "PAYLOAD_REJECTED",
            Self::RemoteContractMismatch => "REMOTE_CONTRACT_MISMATCH",
            Self::Unclassified => "UNCLASSIFIED",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutboxFailureOutcome {
    Rescheduled,
    DeadLettered,
    StaleLease,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeadLetteredOutbox {
    pub tenant_id: String,
    pub outbox_id: String,
    pub incident_id: String,
    pub sequence_no: i64,
    pub attempts: i32,
    pub failure_code: String,
    pub last_failure_at: String,
    pub dead_lettered_at: String,
    pub created_at: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RevisionOrder {
    Equal,
    Older,
    Newer,
    Unproven,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RevisionDecision {
    First,
    Newer,
    Older,
    DuplicateRevision,
    ContentConflict,
    Unproven,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EffectClaim {
    Inserted,
    Duplicate,
    Conflict,
}

#[derive(Clone, Debug)]
struct ConnectionPolicy {
    source_uri: String,
    producer_system: String,
    revision_mode: String,
    revision_prefix: Option<String>,
}

pub struct PostgresOperationsStore {
    pool: PgPool,
}

impl PostgresOperationsStore {
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn connect(database_url: &str, max_connections: u32) -> Result<Self, StoreError> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await?;
        Ok(Self::from_pool(pool))
    }

    pub async fn migrate(pool: &PgPool) -> Result<(), StoreError> {
        MIGRATOR.run(pool).await?;
        Ok(())
    }

    async fn tenant_transaction(
        &self,
        tenant_id: &str,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut tx = self.pool.begin().await?;
        sqlx::raw_sql("SET LOCAL ROLE luminous_ops_app")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
            .bind(tenant_id)
            .execute(&mut *tx)
            .await?;
        Ok(tx)
    }

    /// Ingest a normalized, schema-valid incident snapshot.
    ///
    /// A database-configured revision comparator is required for monotonic
    /// updates. Opaque or malformed revisions that cannot be ordered are
    /// quarantined, never ordered lexically or by arrival timestamp.
    pub async fn ingest_event(
        &self,
        authenticated: &AuthenticatedConnector,
        event: &Value,
    ) -> Result<IngestOutcome, StoreError> {
        validate_event(event).map_err(StoreError::ContractRejected)?;

        let source_uri = required_string(event, &["source"])?;
        let event_id = required_string(event, &["id"])?;
        let company_id = required_string(event, &["data", "source_reference", "company_id"])?;
        let resource_type = required_string(event, &["data", "source_reference", "resource_type"])?;
        let resource_id = required_string(event, &["data", "source_reference", "resource_id"])?;
        let revision = required_string(event, &["data", "source_reference", "revision"])?;
        let idempotency_key = required_string(event, &["idempotencykey"])?;
        let payload = event.get("data").cloned().ok_or(StoreError::InvalidEvent)?;

        let event_digest = event_content_digest(event)?;
        let semantic_digest = business_effect_digest(event)?;
        let state_digest = digest_json("luminous.operations.incident-state.v1", &payload)?;

        let mut tx = self.tenant_transaction(&authenticated.tenant_id).await?;

        let connection = sqlx::query(
            "SELECT source_uri, producer_system, revision_mode, revision_prefix \
             FROM ops.connector_connections \
             WHERE tenant_id = $1 AND connection_id = $2 AND enabled = true",
        )
        .bind(&authenticated.tenant_id)
        .bind(&authenticated.connection_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(StoreError::ConnectorUnavailable)?;

        let policy = ConnectionPolicy {
            source_uri: connection.try_get("source_uri")?,
            producer_system: connection.try_get("producer_system")?,
            revision_mode: connection.try_get("revision_mode")?,
            revision_prefix: connection.try_get("revision_prefix")?,
        };

        // A concurrent identical delivery waits on the unique event key and
        // resolves to this row after the first transaction commits.
        let inserted = sqlx::query(
            "INSERT INTO ops.inbox_events \
             (tenant_id, connection_id, source_uri, event_id, content_digest) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT DO NOTHING RETURNING event_id",
        )
        .bind(&authenticated.tenant_id)
        .bind(&authenticated.connection_id)
        .bind(source_uri)
        .bind(event_id)
        .bind(&event_digest)
        .fetch_optional(&mut *tx)
        .await?;

        if inserted.is_none() {
            let prior = sqlx::query(
                "SELECT content_digest, outcome FROM ops.inbox_events \
                 WHERE tenant_id = $1 AND connection_id = $2 AND source_uri = $3 AND event_id = $4",
            )
            .bind(&authenticated.tenant_id)
            .bind(&authenticated.connection_id)
            .bind(source_uri)
            .bind(event_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(StoreError::ConnectorUnavailable)?;
            let old_digest: String = prior.try_get("content_digest")?;
            let old_outcome: String = prior.try_get("outcome")?;

            if old_digest != event_digest {
                // Keep the original inbox result unchanged; record only a digest
                // for the conflicting content and return an explicit conflict.
                record_quarantine(
                    &mut tx,
                    authenticated,
                    source_uri,
                    event_id,
                    &event_digest,
                    "EVENT_IDENTITY_CONTENT_CONFLICT",
                )
                .await?;
                tx.commit().await?;
                return Err(StoreError::EventIdentityConflict);
            }

            let replay_outcome = match old_outcome.as_str() {
                "QUARANTINED" => {
                    let reason_code = latest_quarantine_reason(
                        &mut tx,
                        authenticated,
                        source_uri,
                        event_id,
                    )
                    .await?
                    .unwrap_or_else(|| "QUARANTINED_WITHOUT_REASON".to_owned());
                    Some(IngestOutcome::Quarantined { reason_code })
                }
                "STALE_REVISION" => Some(IngestOutcome::StaleRevision),
                "DUPLICATE_EFFECT" => Some(IngestOutcome::DuplicateEffect),
                "DUPLICATE_REVISION" => Some(IngestOutcome::DuplicateRevision),
                "ACCEPTED" | "DUPLICATE_EVENT" => Some(IngestOutcome::DuplicateEvent),
                _ => None,
            };
            if let Some(outcome) = replay_outcome {
                tx.commit().await?;
                return Ok(outcome);
            }
            tx.rollback().await?;
            return Err(StoreError::IncompleteEventState);
        }

        let mapping = sqlx::query(
            "SELECT local_incident_id FROM ops.resource_mappings \
             WHERE tenant_id = $1 AND connection_id = $2 \
               AND external_company_id = $3 AND external_resource_type = $4 \
               AND external_resource_id = $5",
        )
        .bind(&authenticated.tenant_id)
        .bind(&authenticated.connection_id)
        .bind(company_id)
        .bind(resource_type)
        .bind(resource_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(mapping) = mapping else {
            quarantine(&mut tx, authenticated, source_uri, event_id, &event_digest, "UNMAPPED_SOURCE_RESOURCE").await?;
            tx.commit().await?;
            return Ok(IngestOutcome::Quarantined { reason_code: "UNMAPPED_SOURCE_RESOURCE".to_owned() });
        };
        let incident_id: String = mapping.try_get("local_incident_id")?;

        let mut tenant_by_company = BTreeMap::new();
        tenant_by_company.insert(company_id.to_owned(), authenticated.tenant_id.clone());
        let mut incident_by_company_resource = BTreeMap::new();
        incident_by_company_resource.insert(
            (company_id.to_owned(), resource_id.to_owned()),
            incident_id.clone(),
        );
        let trusted = TrustedConnectionContext {
            source_uri: policy.source_uri.clone(),
            connection_id: authenticated.connection_id.clone(),
            producer_system: policy.producer_system.clone(),
            tenant_by_company,
            incident_by_company_resource,
        };
        if validate_event_for_connection(event, &trusted).is_err() {
            quarantine(&mut tx, authenticated, source_uri, event_id, &event_digest, "TRUSTED_CONTEXT_BINDING_MISMATCH").await?;
            tx.commit().await?;
            return Ok(IngestOutcome::Quarantined { reason_code: "TRUSTED_CONTEXT_BINDING_MISMATCH".to_owned() });
        }

        // Serialize a business idempotency key before checking the durable
        // fence. Advisory-lock hash collisions only add contention; the unique
        // key remains the final integrity constraint. This check precedes
        // revision rejection so a stale replay cannot hide key reuse conflicts.
        let idempotency_lock = advisory_lock_id(
            &authenticated.tenant_id,
            &authenticated.connection_id,
            idempotency_key,
        );
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(idempotency_lock)
            .execute(&mut *tx)
            .await?;

        let existing_effect = sqlx::query(
            "SELECT semantic_digest FROM ops.business_effects \
             WHERE tenant_id = $1 AND connection_id = $2 AND idempotency_key = $3",
        )
        .bind(&authenticated.tenant_id)
        .bind(&authenticated.connection_id)
        .bind(idempotency_key)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(existing_effect) = existing_effect {
            let existing_digest: String = existing_effect.try_get("semantic_digest")?;
            if existing_digest != semantic_digest {
                quarantine(
                    &mut tx,
                    authenticated,
                    source_uri,
                    event_id,
                    &event_digest,
                    "IDEMPOTENCY_KEY_CONTENT_CONFLICT",
                )
                .await?;
                tx.commit().await?;
                return Err(StoreError::IdempotencyConflict);
            }
            set_inbox_outcome(
                &mut tx,
                authenticated,
                source_uri,
                event_id,
                "DUPLICATE_EFFECT",
            )
            .await?;
            tx.commit().await?;
            return Ok(IngestOutcome::DuplicateEffect);
        }

        // Validate the configured revision token even for the initial
        // observation. Otherwise a malformed first revision could become the
        // baseline and make every future revision unorderable.
        if !revision_token_is_valid(&policy, revision) {
            quarantine(
                &mut tx,
                authenticated,
                source_uri,
                event_id,
                &event_digest,
                "REVISION_TOKEN_INVALID",
            )
            .await?;
            tx.commit().await?;
            return Ok(IngestOutcome::Quarantined {
                reason_code: "REVISION_TOKEN_INVALID".to_owned(),
            });
        }

        // The incident head is the serialization point for revision comparison
        // and outbox sequence allocation. A failed transaction rolls back an
        // initial placeholder head.
        sqlx::query(
            "INSERT INTO ops.incident_heads (tenant_id, incident_id) VALUES ($1, $2) \
             ON CONFLICT DO NOTHING",
        )
        .bind(&authenticated.tenant_id)
        .bind(&incident_id)
        .execute(&mut *tx)
        .await?;

        let head = sqlx::query(
            "SELECT source_connection_id, source_company_id, source_resource_type, \
                    source_resource_id, current_revision, state_digest \
             FROM ops.incident_heads WHERE tenant_id = $1 AND incident_id = $2 FOR UPDATE",
        )
        .bind(&authenticated.tenant_id)
        .bind(&incident_id)
        .fetch_one(&mut *tx)
        .await?;

        let current_connection: Option<String> = head.try_get("source_connection_id")?;
        let current_company: Option<String> = head.try_get("source_company_id")?;
        let current_resource_type: Option<String> = head.try_get("source_resource_type")?;
        let current_resource: Option<String> = head.try_get("source_resource_id")?;
        let current_revision: Option<String> = head.try_get("current_revision")?;
        let current_digest: Option<String> = head.try_get("state_digest")?;

        if current_revision.is_some()
            && (current_connection.as_deref() != Some(authenticated.connection_id.as_str())
                || current_company.as_deref() != Some(company_id)
                || current_resource_type.as_deref() != Some(resource_type)
                || current_resource.as_deref() != Some(resource_id))
        {
            quarantine(&mut tx, authenticated, source_uri, event_id, &event_digest, "REVISION_SOURCE_CHANGED").await?;
            tx.commit().await?;
            return Ok(IngestOutcome::Quarantined { reason_code: "REVISION_SOURCE_CHANGED".to_owned() });
        }

        let revision_decision = if let Some(current_revision) = current_revision.as_deref() {
            match compare_revision(&policy, revision, current_revision)? {
                RevisionOrder::Equal => {
                    if current_digest.as_deref() == Some(state_digest.as_str()) {
                        RevisionDecision::DuplicateRevision
                    } else {
                        RevisionDecision::ContentConflict
                    }
                }
                RevisionOrder::Older => RevisionDecision::Older,
                RevisionOrder::Unproven => RevisionDecision::Unproven,
                RevisionOrder::Newer => RevisionDecision::Newer,
            }
        } else {
            RevisionDecision::First
        };

        // Contradictory or unorderable observations are quarantined without
        // reserving a business idempotency key.
        match revision_decision {
            RevisionDecision::ContentConflict => {
                quarantine(
                    &mut tx,
                    authenticated,
                    source_uri,
                    event_id,
                    &event_digest,
                    "REVISION_CONTENT_CONFLICT",
                )
                .await?;
                tx.commit().await?;
                return Ok(IngestOutcome::Quarantined {
                    reason_code: "REVISION_CONTENT_CONFLICT".to_owned(),
                });
            }
            RevisionDecision::Unproven => {
                quarantine(
                    &mut tx,
                    authenticated,
                    source_uri,
                    event_id,
                    &event_digest,
                    "REVISION_ORDER_UNPROVEN",
                )
                .await?;
                tx.commit().await?;
                return Ok(IngestOutcome::Quarantined {
                    reason_code: "REVISION_ORDER_UNPROVEN".to_owned(),
                });
            }
            _ => {}
        }

        // A well-formed stale or already-applied observation still consumes its
        // idempotency key. Without this fence, a later event could reuse the key
        // with different semantics even though the first observation was stale.
        match claim_business_effect(
            &mut tx,
            authenticated,
            idempotency_key,
            &semantic_digest,
            source_uri,
            event_id,
        )
        .await?
        {
            EffectClaim::Inserted => {}
            EffectClaim::Duplicate => {
                set_inbox_outcome(
                    &mut tx,
                    authenticated,
                    source_uri,
                    event_id,
                    "DUPLICATE_EFFECT",
                )
                .await?;
                tx.commit().await?;
                return Ok(IngestOutcome::DuplicateEffect);
            }
            EffectClaim::Conflict => {
                quarantine(
                    &mut tx,
                    authenticated,
                    source_uri,
                    event_id,
                    &event_digest,
                    "IDEMPOTENCY_KEY_CONTENT_CONFLICT",
                )
                .await?;
                tx.commit().await?;
                return Err(StoreError::IdempotencyConflict);
            }
        }

        match revision_decision {
            RevisionDecision::DuplicateRevision => {
                set_inbox_outcome(
                    &mut tx,
                    authenticated,
                    source_uri,
                    event_id,
                    "DUPLICATE_REVISION",
                )
                .await?;
                tx.commit().await?;
                return Ok(IngestOutcome::DuplicateRevision);
            }
            RevisionDecision::Older => {
                set_inbox_outcome(
                    &mut tx,
                    authenticated,
                    source_uri,
                    event_id,
                    "STALE_REVISION",
                )
                .await?;
                tx.commit().await?;
                return Ok(IngestOutcome::StaleRevision);
            }
            RevisionDecision::First | RevisionDecision::Newer => {}
            RevisionDecision::ContentConflict | RevisionDecision::Unproven => {
                unreachable!("unaccepted revision decisions were handled above")
            }
        }

        let updated_head = sqlx::query(
            "UPDATE ops.incident_heads SET \
               source_connection_id = $3, source_company_id = $4, \
               source_resource_type = $5, source_resource_id = $6, \
               current_revision = $7, state_digest = $8, state_payload = $9, \
               outbox_sequence = outbox_sequence + 1, updated_at = clock_timestamp() \
             WHERE tenant_id = $1 AND incident_id = $2 RETURNING outbox_sequence",
        )
        .bind(&authenticated.tenant_id)
        .bind(&incident_id)
        .bind(&authenticated.connection_id)
        .bind(company_id)
        .bind(resource_type)
        .bind(resource_id)
        .bind(revision)
        .bind(&state_digest)
        .bind(Json(payload.clone()))
        .fetch_one(&mut *tx)
        .await?;
        let sequence_no: i64 = updated_head.try_get("outbox_sequence")?;

        let outbox_id = digest_parts(
            "luminous.operations.outbox-id.v1",
            &[&authenticated.tenant_id, &incident_id, source_uri, event_id],
        );
        let activity_id = digest_parts(
            "luminous.operations.activity-id.v1",
            &[&authenticated.tenant_id, &incident_id, source_uri, event_id],
        );
        let outbox_payload = json!({
            "schema_version": 1,
            "event_type": "io.luminousdynamics.ops.incident.snapshot.applied.v1",
            "tenant_id": authenticated.tenant_id.clone(),
            "incident_id": incident_id.clone(),
            "sequence_no": sequence_no,
            "source_event": {
                "source": source_uri,
                "id": event_id,
                "digest": event_digest.clone()
            },
            "state_digest": state_digest.clone(),
            "revision": revision
        });

        sqlx::query(
            "INSERT INTO ops.incident_activity \
             (tenant_id, activity_id, incident_id, connection_id, source_uri, event_id, revision, state_digest) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&authenticated.tenant_id)
        .bind(activity_id)
        .bind(&incident_id)
        .bind(&authenticated.connection_id)
        .bind(source_uri)
        .bind(event_id)
        .bind(revision)
        .bind(&state_digest)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO ops.outbox_events \
             (tenant_id, outbox_id, incident_id, connection_id, sequence_no, source_uri, source_event_id, payload) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&authenticated.tenant_id)
        .bind(&outbox_id)
        .bind(&incident_id)
        .bind(&authenticated.connection_id)
        .bind(sequence_no)
        .bind(source_uri)
        .bind(event_id)
        .bind(Json(outbox_payload))
        .execute(&mut *tx)
        .await?;

        set_inbox_outcome(&mut tx, authenticated, source_uri, event_id, "ACCEPTED").await?;
        tx.commit().await?;
        Ok(IngestOutcome::Accepted { outbox_sequence: sequence_no })
    }

    /// Atomically lease the earliest eligible outbox event for one tenant.
    ///
    /// PostgreSQL owns all lifecycle transitions. A dead-lettered predecessor
    /// remains undelivered and therefore blocks later events for that incident.
    pub async fn claim_next_outbox(
        &self,
        tenant_id: &str,
        worker_id: &str,
        lease_seconds: i32,
    ) -> Result<Option<OutboxLease>, StoreError> {
        if worker_id.trim().is_empty() || !(1..=3600).contains(&lease_seconds) {
            return Err(StoreError::InvalidLeaseRequest);
        }
        let mut tx = self.tenant_transaction(tenant_id).await?;
        let row = sqlx::query(
            "SELECT outbox_id, incident_id, sequence_no, payload, attempts \
             FROM ops.claim_next_outbox($1, $2, $3)",
        )
        .bind(tenant_id)
        .bind(worker_id)
        .bind(lease_seconds)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };
        let outbox_id: String = row.try_get("outbox_id")?;
        let incident_id: String = row.try_get("incident_id")?;
        let sequence_no: i64 = row.try_get("sequence_no")?;
        let payload: Json<Value> = row.try_get("payload")?;
        let attempts: i32 = row.try_get("attempts")?;
        tx.commit().await?;

        Ok(Some(OutboxLease {
            tenant_id: tenant_id.to_owned(),
            outbox_id,
            incident_id,
            sequence_no,
            payload: payload.0,
            attempts,
        }))
    }

    /// Persist a classified delivery failure and either schedule a bounded
    /// retry or terminalize the row. Raw remote errors and response bodies are
    /// deliberately not accepted or persisted.
    pub async fn record_outbox_failure(
        &self,
        tenant_id: &str,
        outbox_id: &str,
        worker_id: &str,
        failure_code: OutboxFailureCode,
    ) -> Result<OutboxFailureOutcome, StoreError> {
        if worker_id.trim().is_empty() {
            return Err(StoreError::InvalidLeaseRequest);
        }
        let mut tx = self.tenant_transaction(tenant_id).await?;
        let db_outcome: String = sqlx::query_scalar(
            "SELECT ops.record_outbox_failure($1, $2, $3, $4)",
        )
        .bind(tenant_id)
        .bind(outbox_id)
        .bind(worker_id)
        .bind(failure_code.as_db_code())
        .fetch_one(&mut *tx)
        .await?;
        let outcome = match db_outcome.as_str() {
            "RESCHEDULED" => OutboxFailureOutcome::Rescheduled,
            "DEAD_LETTERED" => OutboxFailureOutcome::DeadLettered,
            "STALE_LEASE" => OutboxFailureOutcome::StaleLease,
            _ => return Err(StoreError::UnexpectedOutboxFailureOutcome),
        };
        tx.commit().await?;
        Ok(outcome)
    }

    /// Return bounded, redacted metadata for a tenant's dead-letter queue.
    /// Payloads and provider response bodies are never included.
    pub async fn list_dead_lettered_outbox(
        &self,
        tenant_id: &str,
        limit: i64,
    ) -> Result<Vec<DeadLetteredOutbox>, StoreError> {
        if !(1..=100).contains(&limit) {
            return Err(StoreError::InvalidOutboxQuery);
        }
        let mut tx = self.tenant_transaction(tenant_id).await?;
        let rows = sqlx::query(
            "SELECT tenant_id, outbox_id, incident_id, sequence_no, attempts, \
                    last_failure_code, last_failure_at::text AS last_failure_at, \
                    dead_lettered_at::text AS dead_lettered_at, created_at::text AS created_at \
             FROM ops.outbox_events \
             WHERE tenant_id = $1 AND status = 'DEAD_LETTERED' \
             ORDER BY dead_lettered_at ASC, incident_id ASC, sequence_no ASC \
             LIMIT $2",
        )
        .bind(tenant_id)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await?;
        let mut dead_letters = Vec::with_capacity(rows.len());
        for row in rows {
            dead_letters.push(DeadLetteredOutbox {
                tenant_id: row.try_get("tenant_id")?,
                outbox_id: row.try_get("outbox_id")?,
                incident_id: row.try_get("incident_id")?,
                sequence_no: row.try_get("sequence_no")?,
                attempts: row.try_get("attempts")?,
                failure_code: row.try_get("last_failure_code")?,
                last_failure_at: row.try_get("last_failure_at")?,
                dead_lettered_at: row.try_get("dead_lettered_at")?,
                created_at: row.try_get("created_at")?,
            });
        }
        tx.commit().await?;
        Ok(dead_letters)
    }

    /// Acknowledge only the currently owned, unexpired lease.
    pub async fn acknowledge_outbox(
        &self,
        tenant_id: &str,
        outbox_id: &str,
        worker_id: &str,
    ) -> Result<bool, StoreError> {
        if worker_id.trim().is_empty() {
            return Err(StoreError::InvalidLeaseRequest);
        }
        let mut tx = self.tenant_transaction(tenant_id).await?;
        let acknowledged: bool = sqlx::query_scalar(
            "SELECT ops.acknowledge_outbox($1, $2, $3)",
        )
        .bind(tenant_id)
        .bind(outbox_id)
        .bind(worker_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(acknowledged)
    }
}

fn required_string<'a>(value: &'a Value, path: &[&str]) -> Result<&'a str, StoreError> {
    let mut current = value;
    for part in path {
        current = current.get(*part).ok_or(StoreError::InvalidEvent)?;
    }
    current.as_str().ok_or(StoreError::InvalidEvent)
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(result, "{byte:02x}");
    }
    result
}

fn digest_json(domain: &str, value: &Value) -> Result<String, StoreError> {
    let encoded = serde_json::to_vec(value).map_err(|_| StoreError::InvalidEvent)?;
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    hasher.update(encoded);
    Ok(hex_digest(&hasher.finalize()))
}

fn digest_parts(domain: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    for part in parts {
        let bytes = part.as_bytes();
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    hex_digest(&hasher.finalize())
}

fn advisory_lock_id(tenant_id: &str, connection_id: &str, idempotency_key: &str) -> i64 {
    let digest = digest_parts(
        "luminous.operations.idempotency-lock.v1",
        &[tenant_id, connection_id, idempotency_key],
    );
    let high_bits = u64::from_str_radix(&digest[..16], 16).unwrap_or_default();
    high_bits as i64
}

fn event_content_digest(event: &Value) -> Result<String, StoreError> {
    let mut stable = event.clone();
    if let Some(object) = stable.as_object_mut() {
        // observedat is assigned by the receiving boundary and may differ on a
        // replay; it is not part of CloudEvents source+id content identity.
        object.remove("observedat");
    }
    digest_json("luminous.operations.event-content.v1", &stable)
}

fn business_effect_digest(event: &Value) -> Result<String, StoreError> {
    let keys = [
        "tenantid", "source", "type", "subject", "dataschema", "producersystem",
        "actorkind", "actorid", "authorityref", "data",
    ];
    let mut semantic = Map::new();
    for key in keys {
        if let Some(value) = event.get(key) {
            semantic.insert(key.to_owned(), value.clone());
        }
    }
    digest_json("luminous.operations.business-effect.v1", &Value::Object(semantic))
}

fn numeric_revision(policy: &ConnectionPolicy, value: &str) -> Option<u64> {
    let prefix = policy.revision_prefix.as_deref()?;
    let suffix = value.strip_prefix(prefix)?;
    if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    suffix.parse::<u64>().ok()
}

fn revision_token_is_valid(policy: &ConnectionPolicy, value: &str) -> bool {
    if value.trim().is_empty() {
        return false;
    }
    match policy.revision_mode.as_str() {
        "opaque" => true,
        "numeric_suffix" => numeric_revision(policy, value).is_some(),
        _ => false,
    }
}

fn compare_revision(
    policy: &ConnectionPolicy,
    incoming: &str,
    current: &str,
) -> Result<RevisionOrder, StoreError> {
    if incoming == current {
        return Ok(RevisionOrder::Equal);
    }
    if policy.revision_mode == "opaque" {
        return Ok(RevisionOrder::Unproven);
    }
    if policy.revision_mode != "numeric_suffix" {
        return Err(StoreError::InvalidRevisionPolicy);
    }
    match (
        numeric_revision(policy, incoming),
        numeric_revision(policy, current),
    ) {
        (Some(incoming), Some(current)) if incoming < current => Ok(RevisionOrder::Older),
        (Some(incoming), Some(current)) if incoming > current => Ok(RevisionOrder::Newer),
        (Some(_), Some(_)) => Ok(RevisionOrder::Equal),
        _ => Ok(RevisionOrder::Unproven),
    }
}

async fn claim_business_effect(
    tx: &mut Transaction<'_, Postgres>,
    authenticated: &AuthenticatedConnector,
    idempotency_key: &str,
    semantic_digest: &str,
    source_uri: &str,
    event_id: &str,
) -> Result<EffectClaim, StoreError> {
    let inserted = sqlx::query(
        "INSERT INTO ops.business_effects \
         (tenant_id, connection_id, idempotency_key, semantic_digest, source_uri, event_id) \
         VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING \
         RETURNING idempotency_key",
    )
    .bind(&authenticated.tenant_id)
    .bind(&authenticated.connection_id)
    .bind(idempotency_key)
    .bind(semantic_digest)
    .bind(source_uri)
    .bind(event_id)
    .fetch_optional(&mut **tx)
    .await?;

    if inserted.is_some() {
        return Ok(EffectClaim::Inserted);
    }

    let prior = sqlx::query(
        "SELECT semantic_digest FROM ops.business_effects \
         WHERE tenant_id = $1 AND connection_id = $2 AND idempotency_key = $3",
    )
    .bind(&authenticated.tenant_id)
    .bind(&authenticated.connection_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(StoreError::ConnectorUnavailable)?;
    let prior_digest: String = prior.try_get("semantic_digest")?;
    if prior_digest == semantic_digest {
        Ok(EffectClaim::Duplicate)
    } else {
        Ok(EffectClaim::Conflict)
    }
}

async fn set_inbox_outcome(
    tx: &mut Transaction<'_, Postgres>,
    authenticated: &AuthenticatedConnector,
    source_uri: &str,
    event_id: &str,
    outcome: &str,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE ops.inbox_events SET outcome = $5 \
         WHERE tenant_id = $1 AND connection_id = $2 AND source_uri = $3 AND event_id = $4",
    )
    .bind(&authenticated.tenant_id)
    .bind(&authenticated.connection_id)
    .bind(source_uri)
    .bind(event_id)
    .bind(outcome)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn record_quarantine(
    tx: &mut Transaction<'_, Postgres>,
    authenticated: &AuthenticatedConnector,
    source_uri: &str,
    event_id: &str,
    event_digest: &str,
    reason_code: &'static str,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO ops.quarantined_events \
         (tenant_id, connection_id, source_uri, event_id, event_digest, reason_code) \
         VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING",
    )
    .bind(&authenticated.tenant_id)
    .bind(&authenticated.connection_id)
    .bind(source_uri)
    .bind(event_id)
    .bind(event_digest)
    .bind(reason_code)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn quarantine(
    tx: &mut Transaction<'_, Postgres>,
    authenticated: &AuthenticatedConnector,
    source_uri: &str,
    event_id: &str,
    event_digest: &str,
    reason_code: &'static str,
) -> Result<(), StoreError> {
    record_quarantine(
        tx,
        authenticated,
        source_uri,
        event_id,
        event_digest,
        reason_code,
    )
    .await?;
    set_inbox_outcome(tx, authenticated, source_uri, event_id, "QUARANTINED").await
}

async fn latest_quarantine_reason(
    tx: &mut Transaction<'_, Postgres>,
    authenticated: &AuthenticatedConnector,
    source_uri: &str,
    event_id: &str,
) -> Result<Option<String>, StoreError> {
    let row = sqlx::query(
        "SELECT reason_code FROM ops.quarantined_events \
         WHERE tenant_id = $1 AND connection_id = $2 AND source_uri = $3 AND event_id = $4 \
           AND event_digest = (SELECT content_digest FROM ops.inbox_events \
             WHERE tenant_id = $1 AND connection_id = $2 AND source_uri = $3 AND event_id = $4) \
         ORDER BY recorded_at DESC, quarantine_id DESC LIMIT 1",
    )
    .bind(&authenticated.tenant_id)
    .bind(&authenticated.connection_id)
    .bind(source_uri)
    .bind(event_id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(|value| value.try_get("reason_code").map_err(StoreError::from))
        .transpose()
}
