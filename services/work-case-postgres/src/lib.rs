use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, types::Json, PgPool, Postgres, Transaction};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseKind { Incident, Request, Problem, Change }
impl CaseKind {
    fn as_db_str(self) -> &'static str {
        match self { Self::Incident => "incident", Self::Request => "request", Self::Problem => "problem", Self::Change => "change" }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseState { Open, InProgress, Waiting, Resolved, Closed, Cancelled }
impl CaseState {
    fn as_db_str(self) -> &'static str {
        match self { Self::Open => "open", Self::InProgress => "in_progress", Self::Waiting => "waiting",
            Self::Resolved => "resolved", Self::Closed => "closed", Self::Cancelled => "cancelled" }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorRole { Technician, Admin, Integration, Requester }
impl ActorRole {
    fn as_db_str(self) -> &'static str {
        match self { Self::Technician => "technician", Self::Admin => "admin",
            Self::Integration => "integration", Self::Requester => "requester" }
    }
}

/// Construct only from authenticated server-side identity and tenant membership.
/// This service-layer contract is not an authentication mechanism or cryptographic capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedPrincipal { tenant_id: String, actor_id: String, role: ActorRole }
impl VerifiedPrincipal {
    pub(crate) fn from_verified_authentication(
        tenant_id: &str, actor_id: &str, role: ActorRole
    ) -> Result<Self, RepositoryError> {
        let tenant_id = clean_nonempty(tenant_id, 128)?;
        let actor_id = clean_nonempty(actor_id, 128)?;
        Ok(Self { tenant_id, actor_id, role })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct CreateWorkCase {
    pub idempotency_key: String,
    pub command_id: String,
    pub occurred_at: String,
    pub case_id: String,
    pub kind: CaseKind,
    pub title: String,
    pub summary: String,
    pub customer_id: String,
    pub site_id: Option<String>,
    pub asset_ids: Vec<String>,
    pub priority: i16,
    pub assignee_id: Option<String>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ExternalReference {
    pub provider: String, pub connection_id: String, pub external_id: String, pub linked_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EvidenceReference {
    pub evidence_id: String, pub digest_sha256: String, pub classification: String,
    pub evidence_kind: String, pub issuer: String, pub created_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorkCase {
    pub schema_version: String,
    pub tenant_id: String, pub case_id: String, pub kind: CaseKind, pub title: String,
    pub summary: String, pub customer_id: String, pub site_id: Option<String>,
    pub asset_ids: Vec<String>, pub priority: i16, pub state: CaseState, pub revision: i64,
    pub assignee_id: Option<String>, pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc>,
    pub external_refs: Vec<ExternalReference>, pub evidence_refs: Vec<EvidenceReference>,
}
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("database operation failed")] Database(#[from] sqlx::Error),
    #[error("idempotency key reused with different command semantics")] IdempotencyConflict,
    #[error("principal is not permitted to perform this operation")] Forbidden,
    #[error("invalid work-case input")] InvalidInput,
    #[error("persisted work-case/idempotency state is inconsistent")] InconsistentState,
    #[error("case was changed by a competing command or expected revision is stale")] RevisionConflict,
    #[error("requested lifecycle transition is not allowed")] InvalidTransition,
    #[error("work case was not found in this tenant")] NotFound,
    #[error("failed to encode canonical command material")] Encoding(#[from] serde_json::Error),
}
#[derive(Clone)]
pub struct WorkCaseRepository { pool: PgPool }
struct WorkCaseRow {
    tenant_id: String, case_id: String, kind: String, title: String, summary: String,
    customer_id: String, site_id: Option<String>, asset_ids: Vec<String>, priority: i16,
    state: String, revision: i64, assignee_id: Option<String>,
    created_at: DateTime<Utc>, updated_at: DateTime<Utc>,
}
struct IdempotencyRow { command_digest: String, result_case_id: String, result_payload: Json<Value> }
struct ExternalReferenceRow { provider: String, connection_id: String, external_id: String, linked_at: DateTime<Utc> }
struct EvidenceReferenceRow {
    evidence_id: String, digest_sha256: String, classification: String, evidence_kind: String,
    issuer: String, created_at: DateTime<Utc>,
}

impl WorkCaseRepository {
    /// Connect with a bounded pool using the least-privilege application role.
    /// Schema migrations must be run separately with a privileged migration identity.
    pub async fn connect(database_url: &str, max_connections: u32) -> Result<Self, RepositoryError> {
        if max_connections == 0 { return Err(RepositoryError::InvalidInput); }
        let pool = PgPoolOptions::new().max_connections(max_connections).connect(database_url).await?;
        Ok(Self { pool })
    }

    /// Create a contract-shaped Work Case snapshot. Snapshot, history, idempotency,
    /// and outbox event commit or roll back in one PostgreSQL transaction.
    pub async fn create_case(&self, principal: &VerifiedPrincipal, command: CreateWorkCase) -> Result<WorkCase, RepositoryError> {
        let key = clean_nonempty(&command.idempotency_key, 200)?;
        let command_id = clean_nonempty(&command.command_id, 128)?;
        let case_id = clean_case_id(&command.case_id)?;
        let title = clean_nonempty(&command.title, 240)?;
        let summary = clean_nonempty(&command.summary, 4000)?;
        let customer_id = clean_nonempty(&command.customer_id, 128)?;
        let site_id = command.site_id.as_deref().map(|x| clean_nonempty(x, 128)).transpose()?;
        let asset_ids = command.asset_ids.iter().map(|x| clean_nonempty(x, 128)).collect::<Result<Vec<_>, _>>()?;
        if asset_ids.iter().collect::<HashSet<_>>().len() != asset_ids.len() || !(1..=5).contains(&command.priority) {
            return Err(RepositoryError::InvalidInput);
        }
        let assignee_id = command.assignee_id.as_deref().map(|x| clean_nonempty(x, 128)).transpose()?;
        if principal.role == ActorRole::Requester && assignee_id.is_some() {
            return Err(RepositoryError::Forbidden);
        }
        let occurred_at = parse_timestamp(&command.occurred_at)?;

        // Match Work Case V1's semantic command digest: excludes command ID and
        // arrival time, but includes operation, tenant, actor/role, and normalized payload.
        let payload = json!({
            "case_id": case_id, "kind": command.kind.as_db_str(), "title": title, "summary": summary,
            "customer_id": customer_id, "site_id": site_id, "asset_ids": asset_ids,
            "priority": command.priority, "assignee_id": assignee_id
        });
        let digest = command_digest(principal, "case.create", &payload)?;
        let mut tx = self.pool.begin().await?;
        set_tenant(&mut tx, &principal.tenant_id).await?;

        if let Some(prior) = find_idempotency(&mut tx, &principal.tenant_id, &key).await? {
            if prior.command_digest != digest { return Err(RepositoryError::IdempotencyConflict); }
            let original: WorkCase = serde_json::from_value(prior.result_payload.0)?;
            tx.commit().await?;
            return Ok(original);
        }

        // Unique-key insert serializes concurrent attempts; the FK is deferred so
        // the result ID can be claimed before its case exists within this transaction.
        let claim = sqlx::query!(
            r#"INSERT INTO command_idempotency (tenant_id, idempotency_key, command_digest, result_case_id)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (tenant_id, idempotency_key) DO NOTHING
               RETURNING idempotency_key"#,
            principal.tenant_id, key, digest, case_id
        ).fetch_optional(&mut *tx).await?;
        if claim.is_none() {
            let prior = find_idempotency(&mut tx, &principal.tenant_id, &key).await?
                .ok_or(RepositoryError::InconsistentState)?;
            if prior.command_digest != digest { return Err(RepositoryError::IdempotencyConflict); }
            let original: WorkCase = serde_json::from_value(prior.result_payload.0)?;
            tx.commit().await?;
            return Ok(original);
        }

        let row = sqlx::query_as!(
            WorkCaseRow,
            r#"INSERT INTO work_cases
                 (tenant_id, case_id, kind, title, summary, customer_id, site_id, asset_ids,
                  priority, state, revision, assignee_id, created_at, updated_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,'open',1,$10,$11,$11)
               RETURNING tenant_id, case_id, kind, title, summary, customer_id, site_id,
                         asset_ids, priority, state, revision, assignee_id, created_at, updated_at"#,
            principal.tenant_id, case_id, command.kind.as_db_str(), title, summary, customer_id,
            site_id, &asset_ids, command.priority, assignee_id, occurred_at
        ).fetch_one(&mut *tx).await?;

        let activity_id = format!("{case_id}:activity:1");
        let details = Json(json!({"kind": command.kind.as_db_str(), "customer_id": customer_id}));
        sqlx::query!(
            r#"INSERT INTO case_activity
               (tenant_id, case_id, sequence, activity_id, command_id, actor_id, actor_role,
                activity_type, occurred_at, reason, prior_revision, new_revision, details)
               VALUES ($1,$2,1,$3,$4,$5,$6,'case.created',$7,'case created',0,1,$8)"#,
            principal.tenant_id, case_id, activity_id, command_id, principal.actor_id,
            principal.role.as_db_str(), occurred_at, details
        ).execute(&mut *tx).await?;

        sqlx::query!(
            r#"INSERT INTO case_outbox (tenant_id, outbox_id, case_id, revision, event_type, payload)
               VALUES ($1,$2,$3,1,'io.luminousdynamics.workcase.created',$4)"#,
            principal.tenant_id, Uuid::new_v4(), case_id,
            Json(json!({"tenant_id": principal.tenant_id, "case_id": case_id, "revision": 1, "activity_id": activity_id}))
        ).execute(&mut *tx).await?;

        let created = WorkCase::from((row, Vec::new(), Vec::new()));
        let result_payload = Json(serde_json::to_value(&created)?);
        sqlx::query!(
            "UPDATE command_idempotency SET result_payload = $3 WHERE tenant_id = $1 AND idempotency_key = $2",
            principal.tenant_id, key, result_payload
        ).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(created)
    }

    pub async fn get_case(&self, principal: &VerifiedPrincipal, case_id: &str) -> Result<Option<WorkCase>, RepositoryError> {
        let case_id = clean_case_id(case_id)?;
        let mut tx = self.pool.begin().await?;
        set_tenant(&mut tx, &principal.tenant_id).await?;
        let row = fetch_case(&mut tx, &principal.tenant_id, &case_id).await?;
        tx.commit().await?;
        Ok(row)
    }
    /// Apply a lifecycle transition using optimistic revision checks and durable idempotency.
    pub async fn transition_case(
        &self, principal: &VerifiedPrincipal, case_id: &str, target: CaseState,
        expected_revision: i64, reason: &str, idempotency_key: &str,
        command_id: &str, occurred_at: &str,
    ) -> Result<WorkCase, RepositoryError> {
        if principal.role == ActorRole::Requester { return Err(RepositoryError::Forbidden); }
        let case_id = clean_case_id(case_id)?;
        let reason = clean_nonempty(reason, 1000)?;
        let command_id = clean_nonempty(command_id, 128)?;
        let key = clean_nonempty(idempotency_key, 200)?;
        let occurred_at = parse_timestamp(occurred_at)?;
        if expected_revision < 1 { return Err(RepositoryError::InvalidInput); }
        let payload = json!({ "case_id": case_id, "target_state": target.as_db_str(),
            "expected_revision": expected_revision, "reason": reason });
        let digest = command_digest(principal, "case.transition", &payload)?;

        let mut tx = self.pool.begin().await?;
        set_tenant(&mut tx, &principal.tenant_id).await?;
        if let Some(prior) = find_idempotency(&mut tx, &principal.tenant_id, &key).await? {
            if prior.command_digest != digest { return Err(RepositoryError::IdempotencyConflict); }
            let original: WorkCase = serde_json::from_value(prior.result_payload.0)?;
            tx.commit().await?;
            return Ok(original);
        }
        let current = fetch_case(&mut tx, &principal.tenant_id, &case_id).await?
            .ok_or(RepositoryError::NotFound)?;
        if current.revision != expected_revision { return Err(RepositoryError::RevisionConflict); }
        if !transition_allowed(current.state, target) { return Err(RepositoryError::InvalidTransition); }

        let claim = sqlx::query!(
            r#"INSERT INTO command_idempotency (tenant_id, idempotency_key, command_digest, result_case_id)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (tenant_id, idempotency_key) DO NOTHING
               RETURNING idempotency_key"#,
            principal.tenant_id, key, digest, case_id
        ).fetch_optional(&mut *tx).await?;
        if claim.is_none() {
            let prior = find_idempotency(&mut tx, &principal.tenant_id, &key).await?
                .ok_or(RepositoryError::InconsistentState)?;
            if prior.command_digest != digest { return Err(RepositoryError::IdempotencyConflict); }
            let original: WorkCase = serde_json::from_value(prior.result_payload.0)?;
            tx.commit().await?;
            return Ok(original);
        }

        let row = sqlx::query_as!(
            WorkCaseRow,
            r#"UPDATE work_cases SET state = $4, revision = revision + 1, updated_at = $5
               WHERE tenant_id = $1 AND case_id = $2 AND revision = $3
               RETURNING tenant_id, case_id, kind, title, summary, customer_id, site_id,
                         asset_ids, priority, state, revision, assignee_id, created_at, updated_at"#,
            principal.tenant_id, case_id, expected_revision, target.as_db_str(), occurred_at
        ).fetch_optional(&mut *tx).await?;
        let row = row.ok_or(RepositoryError::RevisionConflict)?;
        let sequence = sqlx::query_scalar!(
            "SELECT COALESCE(MAX(sequence), 0) + 1 AS \"next!\" FROM case_activity WHERE tenant_id = $1 AND case_id = $2",
            principal.tenant_id, case_id
        ).fetch_one(&mut *tx).await?;
        let activity_id = Uuid::new_v4().to_string();
        sqlx::query!(
            r#"INSERT INTO case_activity
               (tenant_id, case_id, sequence, activity_id, command_id, actor_id, actor_role,
                activity_type, occurred_at, reason, prior_revision, new_revision, details)
               VALUES ($1,$2,$3,$4,$5,$6,$7,'case.state_changed',$8,$9,$10,$11,$12)"#,
            principal.tenant_id, case_id, sequence, activity_id, command_id, principal.actor_id,
            principal.role.as_db_str(), occurred_at, reason, expected_revision, expected_revision + 1,
            Json(json!({"from": current.state.as_db_str(), "to": target.as_db_str()}))
        ).execute(&mut *tx).await?;
        sqlx::query!(
            r#"INSERT INTO case_outbox (tenant_id, outbox_id, case_id, revision, event_type, payload)
               VALUES ($1,$2,$3,$4,'io.luminousdynamics.workcase.state_changed',$5)"#,
            principal.tenant_id, Uuid::new_v4(), case_id, expected_revision + 1,
            Json(json!({"tenant_id": principal.tenant_id, "case_id": case_id,
                "revision": expected_revision + 1, "activity_id": activity_id,
                "from": current.state.as_db_str(), "to": target.as_db_str()}))
        ).execute(&mut *tx).await?;

        let updated = WorkCase::from((row, current.external_refs, current.evidence_refs));
        let result_payload = Json(serde_json::to_value(&updated)?);
        sqlx::query!(
            "UPDATE command_idempotency SET result_payload = $3 WHERE tenant_id = $1 AND idempotency_key = $2",
            principal.tenant_id, key, result_payload
        ).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(updated)
    }

    #[cfg(test)]
    fn pool(&self) -> &PgPool { &self.pool }
}

impl From<(WorkCaseRow, Vec<ExternalReference>, Vec<EvidenceReference>)> for WorkCase {
    fn from((row, external_refs, evidence_refs): (WorkCaseRow, Vec<ExternalReference>, Vec<EvidenceReference>)) -> Self {
        let kind = match row.kind.as_str() {
            "incident" => CaseKind::Incident, "request" => CaseKind::Request,
            "problem" => CaseKind::Problem, "change" => CaseKind::Change,
            _ => unreachable!("database CHECK constraint restricts case kind"),
        };
        let state = match row.state.as_str() {
            "open" => CaseState::Open, "in_progress" => CaseState::InProgress, "waiting" => CaseState::Waiting,
            "resolved" => CaseState::Resolved, "closed" => CaseState::Closed, "cancelled" => CaseState::Cancelled,
            _ => unreachable!("database CHECK constraint restricts case state"),
        };
        Self { schema_version: "work-case-v1".to_owned(), tenant_id: row.tenant_id, case_id: row.case_id, kind, title: row.title,
            summary: row.summary, customer_id: row.customer_id, site_id: row.site_id,
            asset_ids: row.asset_ids, priority: row.priority, state, revision: row.revision,
            assignee_id: row.assignee_id, created_at: row.created_at, updated_at: row.updated_at,
            external_refs, evidence_refs }
    }
}

fn clean_nonempty(value: &str, max: usize) -> Result<String, RepositoryError> {
    if value.chars().count() > max { return Err(RepositoryError::InvalidInput); }
    let clean = value.trim();
    if clean.is_empty() { return Err(RepositoryError::InvalidInput); }
    Ok(clean.to_owned())
}
fn clean_case_id(value: &str) -> Result<String, RepositoryError> {
    let value = clean_nonempty(value, 128)?;
    let mut chars = value.chars();
    let Some(first) = chars.next() else { return Err(RepositoryError::InvalidInput); };
    if !first.is_ascii_alphanumeric() || !chars.all(|c| c.is_ascii_alphanumeric() || "._:-".contains(c)) {
        return Err(RepositoryError::InvalidInput);
    }
    Ok(value)
}
fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, RepositoryError> {
    DateTime::parse_from_rfc3339(value).map(|v| v.with_timezone(&Utc))
        .map_err(|_| RepositoryError::InvalidInput)
}
fn command_digest(p: &VerifiedPrincipal, operation: &str, payload: &Value) -> Result<String, RepositoryError> {
    let material = json!({"tenant_id": p.tenant_id, "actor_id": p.actor_id,
        "actor_role": p.role.as_db_str(), "operation": operation, "payload": payload});
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(&material)?)))
}
fn transition_allowed(from: CaseState, to: CaseState) -> bool {
    matches!((from, to),
        (CaseState::Open, CaseState::InProgress | CaseState::Waiting | CaseState::Cancelled)
        | (CaseState::InProgress, CaseState::Waiting | CaseState::Resolved | CaseState::Cancelled)
        | (CaseState::Waiting, CaseState::InProgress | CaseState::Resolved | CaseState::Cancelled)
        | (CaseState::Resolved, CaseState::Open | CaseState::Closed))
}
async fn set_tenant(tx: &mut Transaction<'_, Postgres>, tenant_id: &str) -> Result<(), sqlx::Error> {
    let _: String = sqlx::query_scalar("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant_id).fetch_one(&mut **tx).await?;
    Ok(())
}
async fn find_idempotency(tx: &mut Transaction<'_, Postgres>, tenant_id: &str, key: &str)
    -> Result<Option<IdempotencyRow>, sqlx::Error> {
    sqlx::query_as!(IdempotencyRow,
        r#"SELECT command_digest, result_case_id, result_payload AS "result_payload: Json<Value>"
           FROM command_idempotency WHERE tenant_id = $1 AND idempotency_key = $2"#,
        tenant_id, key
    ).fetch_optional(&mut **tx).await
}
async fn fetch_case_row(tx: &mut Transaction<'_, Postgres>, tenant_id: &str, case_id: &str)
    -> Result<Option<WorkCaseRow>, sqlx::Error> {
    sqlx::query_as!(WorkCaseRow,
        r#"SELECT tenant_id, case_id, kind, title, summary, customer_id, site_id, asset_ids, priority,
                  state, revision, assignee_id, created_at, updated_at
           FROM work_cases WHERE tenant_id = $1 AND case_id = $2"#,
        tenant_id, case_id
    ).fetch_optional(&mut **tx).await
}
async fn fetch_case(tx: &mut Transaction<'_, Postgres>, tenant_id: &str, case_id: &str)
    -> Result<Option<WorkCase>, RepositoryError> {
    let Some(row) = fetch_case_row(tx, tenant_id, case_id).await? else { return Ok(None); };
    let external_rows = sqlx::query_as!(ExternalReferenceRow,
        "SELECT provider, connection_id, external_id, linked_at FROM external_case_mappings WHERE tenant_id = $1 AND case_id = $2 ORDER BY linked_at, provider, external_id",
        tenant_id, case_id
    ).fetch_all(&mut **tx).await?;
    let evidence_rows = sqlx::query_as!(EvidenceReferenceRow,
        "SELECT evidence_id, digest_sha256, classification, evidence_kind, issuer, created_at FROM case_evidence_refs WHERE tenant_id = $1 AND case_id = $2 ORDER BY evidence_id",
        tenant_id, case_id
    ).fetch_all(&mut **tx).await?;
    let external_refs = external_rows.into_iter().map(|r| ExternalReference {
        provider: r.provider, connection_id: r.connection_id, external_id: r.external_id, linked_at: r.linked_at
    }).collect();
    let evidence_refs = evidence_rows.into_iter().map(|r| EvidenceReference {
        evidence_id: r.evidence_id, digest_sha256: r.digest_sha256, classification: r.classification,
        evidence_kind: r.evidence_kind, issuer: r.issuer, created_at: r.created_at
    }).collect();
    Ok(Some(WorkCase::from((row, external_refs, evidence_refs))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    async fn pool(max: u32) -> PgPool {
        let url = std::env::var("APP_DATABASE_URL").expect("APP_DATABASE_URL must use the non-owner role");
        PgPoolOptions::new().max_connections(max).connect(&url).await.expect("connect to PostgreSQL as app role")
    }
    fn principal() -> VerifiedPrincipal {
        VerifiedPrincipal::from_verified_authentication(
            &format!("tenant-{}", Uuid::new_v4()), &format!("actor-{}", Uuid::new_v4()), ActorRole::Technician
        ).expect("valid synthetic principal")
    }
    fn command(key: &str, case_id: &str, title: &str) -> CreateWorkCase {
        CreateWorkCase {
            idempotency_key: key.into(), command_id: "command-synthetic-01".into(),
            occurred_at: "2026-10-09T12:00:00Z".into(), case_id: case_id.into(),
            kind: CaseKind::Incident, title: title.into(), summary: "Synthetic service interruption".into(),
            customer_id: "customer-synthetic-01".into(), site_id: Some("site-synthetic-01".into()),
            asset_ids: vec!["asset-synthetic-01".into()], priority: 2, assignee_id: None,
        }
    }
    async fn side_effect_counts(pool: &PgPool, p: &VerifiedPrincipal, case_id: &str) -> Result<(i64, i64), sqlx::Error> {
        let mut tx = pool.begin().await?;
        set_tenant(&mut tx, &p.tenant_id).await?;
        let activity = sqlx::query_scalar!(
            r#"SELECT count(*)::BIGINT AS "count!" FROM case_activity WHERE tenant_id = $1 AND case_id = $2"#,
            p.tenant_id, case_id
        ).fetch_one(&mut *tx).await?;
        let outbox = sqlx::query_scalar!(
            r#"SELECT count(*)::BIGINT AS \"count!\" FROM case_outbox WHERE tenant_id = $1 AND case_id = $2"#,
            p.tenant_id, case_id
        ).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok((activity, outbox))
    }

    async fn unscoped_rls_count(pool: &PgPool, p: &VerifiedPrincipal) -> Result<i64, sqlx::Error> {
        let mut tx = pool.begin().await?;
        set_tenant(&mut tx, &p.tenant_id).await?;
        // Deliberately no tenant predicate: tests PostgreSQL RLS itself.
        let n = sqlx::query_scalar!(r#"SELECT count(*)::BIGINT AS "count!" FROM work_cases"#)
            .fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(n)
    }

    #[tokio::test]
    async fn idempotent_replay_conflict_and_atomic_outbox() {
        let pool = pool(3).await;
        let repo = WorkCaseRepository { pool: pool.clone() };
        let p = principal();
        let first = repo.create_case(&p, command("ticket/retry-key-001", "case-synthetic-001", "Synthetic endpoint incident"))
            .await.expect("create case");
        let replay = repo.create_case(&p, command("ticket/retry-key-001", "case-synthetic-001", "Synthetic endpoint incident"))
            .await.expect("same-semantics replay");
        assert_eq!(first, replay);
        assert!(matches!(repo.create_case(&p, command("ticket/retry-key-001", "case-synthetic-001", "Different command"))
            .await, Err(RepositoryError::IdempotencyConflict)));
        let (activity, outbox) = side_effect_counts(&pool, &p, &first.case_id).await.expect("side-effect counts");
        assert_eq!(activity, 1);
        assert_eq!(outbox, 1);
    }

    #[tokio::test]
    async fn incomplete_idempotency_snapshot_cannot_commit() {
        let pool = pool(1).await;
        let repo = WorkCaseRepository { pool: pool.clone() };
        let p = principal();
        let created = repo.create_case(&p, command(
            "incomplete-seed-key-001", "case-incomplete-seed-001", "Idempotency trigger seed"
        )).await.expect("create FK target");

        let key = format!("incomplete-{}", Uuid::new_v4());
        let mut tx = pool.begin().await.expect("begin incomplete claim transaction");
        set_tenant(&mut tx, &p.tenant_id).await.expect("set trusted tenant context");
        sqlx::query!(
            r#"INSERT INTO command_idempotency
               (tenant_id, idempotency_key, command_digest, result_case_id)
               VALUES ($1, $2, $3, $4)"#,
            p.tenant_id, key, "0".repeat(64), created.case_id
        ).execute(&mut *tx).await.expect("insert staged claim");
        let commit = tx.commit().await;
        assert!(commit.is_err(), "database must reject an incomplete idempotency result snapshot");
    }

    #[tokio::test]
    async fn finalized_idempotency_snapshot_is_immutable() {
        let pool = pool(1).await;
        let repo = WorkCaseRepository { pool: pool.clone() };
        let p = principal();
        let command_value = command(
            "immutable-seed-key-001", "case-immutable-seed-001", "Immutable snapshot test"
        );
        let created = repo.create_case(&p, command_value.clone()).await.expect("create seed case");

        let mut tx = pool.begin().await.expect("begin snapshot tamper transaction");
        set_tenant(&mut tx, &p.tenant_id).await.expect("set tenant context");
        let update = sqlx::query!(
            "UPDATE command_idempotency SET result_payload = $3 WHERE tenant_id = $1 AND idempotency_key = $2",
            p.tenant_id, command_value.idempotency_key, Json(json!({}))
        ).execute(&mut *tx).await;
        assert!(update.is_err(), "database must reject rewriting a finalized idempotency snapshot");
        tx.rollback().await.expect("roll back rejected snapshot rewrite");

        let replay = repo.create_case(&p, command_value).await.expect("replay finalized result");
        assert_eq!(replay, created, "rejected snapshot mutation must preserve the original response");
    }

    #[tokio::test]
    async fn rls_and_single_connection_reuse_isolate_tenants() {
        let pool = pool(1).await;
        let repo = WorkCaseRepository { pool: pool.clone() };
        // The fresh application connection has no tenant context yet: RLS must fail closed.
        let without_context = sqlx::query_scalar!(
            r#"SELECT count(*)::BIGINT AS "count!" FROM work_cases"#
        ).fetch_one(&pool).await.expect("RLS with missing context");
        assert_eq!(without_context, 0, "missing tenant context must expose no rows");
        let a = principal();
        let b = principal();
        let ca = repo.create_case(&a, command("tenant-a-key-001", "case-tenant-a-001", "Tenant A case"))
            .await.expect("create A");
        let cb = repo.create_case(&b, command("tenant-b-key-001", "case-tenant-b-001", "Tenant B case"))
            .await.expect("create B");
        assert!(repo.get_case(&b, &ca.case_id).await.expect("cross-tenant read").is_none());
        assert_eq!(unscoped_rls_count(&pool, &a).await.expect("RLS A"), 1);
        assert_eq!(unscoped_rls_count(&pool, &b).await.expect("RLS B"), 1);
        assert_ne!(ca.tenant_id, cb.tenant_id);
    }

    #[tokio::test]
    async fn transition_is_revision_checked_and_replay_returns_original_snapshot() {
        let pool = pool(3).await;
        let repo = WorkCaseRepository { pool: pool.clone() };
        let p = principal();
        let created = repo.create_case(&p, command("transition-create-key-001", "case-transition-001", "Lifecycle test case"))
            .await.expect("create case");
        let changed = repo.transition_case(&p, &created.case_id, CaseState::InProgress, 1,
            "Technician started work", "transition-key-001", "transition-command-001", "2026-10-09T12:30:00Z")
            .await.expect("transition open to in_progress");
        assert_eq!(changed.state, CaseState::InProgress);
        assert_eq!(changed.revision, 2);
        let replay = repo.transition_case(&p, &created.case_id, CaseState::InProgress, 1,
            "Technician started work", "transition-key-001", "transition-command-001", "2026-10-09T12:30:00Z")
            .await.expect("transition replay");
        assert_eq!(replay, changed);
        let current = repo.get_case(&p, &created.case_id).await.expect("read current").expect("case exists");
        assert_eq!(current.revision, 2);
        assert_eq!(current.state, CaseState::InProgress);
        let create_replay = repo.create_case(&p, command(
            "transition-create-key-001", "case-transition-001", "Lifecycle test case"
        )).await.expect("creation retry after later transition");
        assert_eq!(create_replay, created, "a creation retry must return its original response snapshot");
        assert!(matches!(repo.transition_case(&p, &created.case_id, CaseState::Resolved, 1,
            "Stale attempt", "transition-stale-key-001", "transition-command-002", "2026-10-09T12:31:00Z").await,
            Err(RepositoryError::RevisionConflict)));
        assert!(matches!(repo.transition_case(&p, &created.case_id, CaseState::Closed, 2,
            "Illegal direct close", "transition-illegal-key-001", "transition-command-003", "2026-10-09T12:32:00Z").await,
            Err(RepositoryError::InvalidTransition)));
        let (activity, outbox) = side_effect_counts(&pool, &p, &created.case_id).await.expect("side-effect counts");
        assert_eq!(activity, 2, "one creation and one accepted transition activity");
        assert_eq!(outbox, 2, "one creation and one transition outbox event");
    }

    #[tokio::test]
    async fn concurrent_stale_transitions_allow_exactly_one_winner() {
        let pool = pool(4).await;
        let repo = WorkCaseRepository { pool: pool.clone() };
        let p = principal();
        let created = repo.create_case(&p, command(
            "race-create-key-001", "case-transition-race-001", "Concurrent transition case"
        )).await.expect("create case");

        let left = repo.transition_case(&p, &created.case_id, CaseState::InProgress, 1,
            "Concurrent winner A", "race-transition-key-a", "race-transition-command-a", "2026-10-09T12:40:00Z");
        let right = repo.transition_case(&p, &created.case_id, CaseState::Waiting, 1,
            "Concurrent winner B", "race-transition-key-b", "race-transition-command-b", "2026-10-09T12:40:01Z");
        let (left, right) = tokio::join!(left, right);
        match (left, right) {
            (Ok(_), Err(RepositoryError::RevisionConflict))
            | (Err(RepositoryError::RevisionConflict), Ok(_)) => {}
            other => panic!("exactly one competing expected-revision command must win: {other:?}"),
        }

        let current = repo.get_case(&p, &created.case_id).await.expect("read raced case").expect("case exists");
        assert_eq!(current.revision, 2);
        assert!(matches!(current.state, CaseState::InProgress | CaseState::Waiting));
        let (activity, outbox) = side_effect_counts(&pool, &p, &created.case_id).await.expect("side-effect counts");
        assert_eq!(activity, 2, "one creation plus exactly one winning transition");
        assert_eq!(outbox, 2, "one creation plus exactly one winning outbox event");
    }

    #[tokio::test]
    async fn concurrent_duplicate_claim_commits_one_case_and_outbox() {
        let pool = pool(4).await;
        let repo = WorkCaseRepository { pool };
        let p = principal();
        let command = command("concurrent-key-001", "case-concurrent-001", "Concurrent synthetic case");
        let (a, b) = tokio::join!(repo.create_case(&p, command.clone()), repo.create_case(&p, command));
        let a = a.expect("first command");
        let b = b.expect("duplicate command");
        assert_eq!(a.case_id, b.case_id);
        let (_, n) = side_effect_counts(repo.pool(), &p, &a.case_id).await.expect("side-effect counts");
        assert_eq!(n, 1);
    }
    #[tokio::test]
    async fn application_role_lacks_unneeded_schema_and_mutation_privileges() {
        let pool = pool(1).await;
        let privileges = sqlx::query!(
            r#"SELECT
                has_schema_privilege(current_user, 'public', 'CREATE') AS "schema_create!",
                has_table_privilege(current_user, 'external_case_mappings', 'INSERT') AS "mapping_insert!",
                has_table_privilege(current_user, 'case_evidence_refs', 'INSERT') AS "evidence_insert!",
                has_table_privilege(current_user, 'case_outbox', 'INSERT') AS "outbox_table_insert!",
                has_table_privilege(current_user, 'case_outbox', 'UPDATE') AS "outbox_table_update!",
                has_table_privilege(current_user, 'case_outbox', 'DELETE') AS "outbox_table_delete!",
                has_column_privilege(current_user, 'case_outbox', 'tenant_id', 'INSERT') AS "outbox_tenant_insert!",
                has_column_privilege(current_user, 'case_outbox', 'outbox_id', 'INSERT') AS "outbox_id_insert!",
                has_column_privilege(current_user, 'case_outbox', 'case_id', 'INSERT') AS "outbox_case_insert!",
                has_column_privilege(current_user, 'case_outbox', 'revision', 'INSERT') AS "outbox_revision_insert!",
                has_column_privilege(current_user, 'case_outbox', 'event_type', 'INSERT') AS "outbox_event_type_insert!",
                has_column_privilege(current_user, 'case_outbox', 'payload', 'INSERT') AS "outbox_payload_insert!",
                has_column_privilege(current_user, 'case_outbox', 'created_at', 'INSERT') AS "outbox_created_at_insert!",
                has_column_privilege(current_user, 'case_outbox', 'status', 'UPDATE') AS "outbox_status_update!",
                has_column_privilege(current_user, 'case_outbox', 'status', 'INSERT') AS "outbox_status_insert!",
                has_column_privilege(current_user, 'case_outbox', 'attempts', 'INSERT') AS "outbox_attempts_insert!",
                has_column_privilege(current_user, 'case_outbox', 'lease_owner', 'INSERT') AS "outbox_lease_owner_insert!",
                has_column_privilege(current_user, 'case_outbox', 'lease_until', 'INSERT') AS "outbox_lease_until_insert!",
                has_column_privilege(current_user, 'case_outbox', 'delivered_at', 'UPDATE') AS "outbox_delivered_at_update!",
                has_column_privilege(current_user, 'case_outbox', 'delivered_at', 'INSERT') AS "outbox_delivered_at_insert!",
                has_table_privilege(current_user, 'case_activity', 'UPDATE') AS "activity_update!""#
        ).fetch_one(&pool).await.expect("read application-role privilege boundary");

        assert!(!privileges.schema_create, "runtime role must not create objects in public");
        assert!(!privileges.mapping_insert, "mapping writes are not exposed yet");
        assert!(!privileges.evidence_insert, "evidence writes are not exposed yet");
        assert!(!privileges.outbox_table_insert, "outbox INSERT must be column-scoped");
        assert!(!privileges.outbox_table_update, "the current service has no dispatcher update capability");
        assert!(!privileges.outbox_table_delete, "outbox rows must not be deleted by the application role");
        assert!(privileges.outbox_tenant_insert && privileges.outbox_id_insert && privileges.outbox_case_insert
            && privileges.outbox_revision_insert && privileges.outbox_event_type_insert && privileges.outbox_payload_insert,
            "create and transition paths need their explicit outbox event columns");
        assert!(!privileges.outbox_created_at_insert, "outbox creation time must be set by the database clock");
        assert!(!privileges.outbox_status_update, "outbox delivery state belongs to a qualified dispatcher");
        assert!(!privileges.outbox_status_insert, "the runtime role must use the database default outbox status");
        assert!(!privileges.outbox_attempts_insert, "the runtime role must use the database default attempt counter");
        assert!(!privileges.outbox_lease_owner_insert, "outbox lease ownership belongs to the dispatcher");
        assert!(!privileges.outbox_lease_until_insert, "outbox lease fields belong to the dispatcher");
        assert!(!privileges.outbox_delivered_at_update, "delivery acknowledgement belongs to the dispatcher");
        assert!(!privileges.outbox_delivered_at_insert, "runtime callers cannot backdate or pre-acknowledge delivery");
        assert!(!privileges.activity_update, "activity is append-only");
    }

    #[tokio::test]
    async fn failed_create_rolls_back_idempotency_claim_and_side_effects() {
        let pool = pool(3).await;
        let repo = WorkCaseRepository { pool: pool.clone() };
        let p = principal();

        let existing = repo.create_case(
            &p,
            command("rollback-original-key-001", "case-rollback-existing-001", "Existing case"),
        ).await.expect("create existing case");

        // This request claims its idempotency key, then fails on the duplicate case primary key.
        // The claim and all later side effects must roll back with the failed transaction.
        assert!(
            repo.create_case(
                &p,
                command("rollback-attempt-key-001", &existing.case_id, "Intentional collision"),
            ).await.is_err(),
            "a duplicate tenant/case key must fail"
        );

        // Reusing that key with corrected command semantics must succeed. If the failed
        // transaction left an idempotency record behind, this call would conflict.
        let recovered = repo.create_case(
            &p,
            command("rollback-attempt-key-001", "case-rollback-recovered-001", "Recovered create"),
        ).await.expect("retry after transaction rollback");
        assert_eq!(recovered.case_id, "case-rollback-recovered-001");

        let existing_counts = side_effect_counts(&pool, &p, &existing.case_id)
            .await.expect("existing case side-effect counts");
        let recovered_counts = side_effect_counts(&pool, &p, &recovered.case_id)
            .await.expect("recovered case side-effect counts");
        assert_eq!(existing_counts, (1, 1), "failed create must not add history or outbox effects to the existing case");
        assert_eq!(recovered_counts, (1, 1), "corrected retry commits exactly one activity and outbox event");
    }

    #[tokio::test]
    async fn database_rejects_inconsistent_outbox_lease_state() {
        let app_pool = pool(1).await;
        let repo = WorkCaseRepository { pool: app_pool };
        let p = principal();
        let created = repo.create_case(
            &p,
            command("outbox-state-create-key-001", "case-outbox-state-001", "Outbox state constraint"),
        ).await.expect("create case for outbox constraint");

        // Use the migration role to bypass app-role column grants and prove the schema
        // itself rejects contradictory delivery state, independent of repository code.
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must use the migration/test owner role");
        let owner_pool = PgPoolOptions::new().max_connections(1).connect(&database_url)
            .await.expect("connect with migration/test owner role");
        let mut tx = owner_pool.begin().await.expect("begin owner transaction");
        set_tenant(&mut tx, &p.tenant_id).await.expect("set tenant context");

        let invalid = sqlx::query!(
            r#"INSERT INTO case_outbox
               (tenant_id, outbox_id, case_id, revision, event_type, payload, status, attempts, lease_owner, lease_until)
               VALUES ($1,$2,$3,2,'test.invalid.outbox_lease',$4,'LEASED',1,NULL,NULL)"#,
            p.tenant_id, Uuid::new_v4(), created.case_id, Json(json!({"synthetic": true}))
        ).execute(&mut *tx).await;
        match invalid {
            Err(sqlx::Error::Database(error)) => {
                assert_eq!(error.constraint(), Some("case_outbox_state_shape"));
            }
            Ok(_) => panic!("database accepted a leased outbox row without owner and expiry"),
            Err(error) => panic!("unexpected error instead of state-shape violation: {error}"),
        }
        tx.rollback().await.expect("roll back rejected invalid row");
        owner_pool.close().await;
    }

}
