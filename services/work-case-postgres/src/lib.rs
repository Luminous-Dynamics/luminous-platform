use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, types::Json, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaseKind { Incident, Request, Problem, Change }
impl CaseKind {
    fn as_db_str(self) -> &'static str {
        match self { Self::Incident => "INCIDENT", Self::Request => "REQUEST", Self::Problem => "PROBLEM", Self::Change => "CHANGE" }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorRole { Operator, Integration, System }
impl ActorRole {
    fn as_db_str(self) -> &'static str {
        match self { Self::Operator => "OPERATOR", Self::Integration => "INTEGRATION", Self::System => "SYSTEM" }
    }
}

/// Must be constructed only after the server-side authentication layer has
/// verified both actor identity and tenant membership. This is a service-layer
/// contract, not an authentication mechanism or cryptographic capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedPrincipal { tenant_id: Uuid, actor_id: Uuid, role: ActorRole }
impl VerifiedPrincipal {
    pub fn from_verified_authentication(tenant_id: Uuid, actor_id: Uuid, role: ActorRole) -> Self {
        Self { tenant_id, actor_id, role }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct CreateWorkCase {
    pub idempotency_key: Uuid,
    pub kind: CaseKind,
    pub title: String,
    /// Variable extension data only; core identity and lifecycle remain relational.
    pub payload: Value,
}
#[derive(Clone, Debug, PartialEq)]
pub struct WorkCase {
    pub tenant_id: Uuid, pub case_id: Uuid, pub kind: CaseKind,
    pub title: String, pub state: String, pub revision: i64, pub payload: Value,
}
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("database operation failed")] Database(#[from] sqlx::Error),
    #[error("idempotency key reused with different command semantics")] IdempotencyConflict,
    #[error("invalid work-case input")] InvalidInput,
    #[error("persisted work-case/idempotency state is inconsistent")] InconsistentState,
    #[error("failed to encode canonical command material")] Encoding(#[from] serde_json::Error),
}
#[derive(Clone)]
pub struct WorkCaseRepository { pool: PgPool }
struct WorkCaseRow {
    tenant_id: Uuid, case_id: Uuid, kind: String, title: String,
    state: String, revision: i64, payload: Json<Value>,
}
struct IdempotencyRow { command_digest: String, result_case_id: Uuid }

impl WorkCaseRepository {
    /// Use the least-privilege application role; migrations run separately.
    pub async fn connect(database_url: &str, max_connections: u32) -> Result<Self, RepositoryError> {
        if max_connections == 0 { return Err(RepositoryError::InvalidInput); }
        let pool = PgPoolOptions::new().max_connections(max_connections).connect(database_url).await?;
        Ok(Self { pool })
    }

    /// One transaction commits the case, history, idempotency result, and outbox.
    pub async fn create_case(&self, principal: &VerifiedPrincipal, command: CreateWorkCase) -> Result<WorkCase, RepositoryError> {
        if command.title.trim().is_empty() || command.title.chars().count() > 300 {
            return Err(RepositoryError::InvalidInput);
        }
        let digest = command_digest(principal, &command)?;
        let case_id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        set_tenant(&mut tx, principal.tenant_id).await?;

        if let Some(prior) = find_idempotency(&mut tx, principal.tenant_id, command.idempotency_key).await? {
            if prior.command_digest != digest { return Err(RepositoryError::IdempotencyConflict); }
            let row = fetch_case(&mut tx, principal.tenant_id, prior.result_case_id).await?
                .ok_or(RepositoryError::InconsistentState)?;
            tx.commit().await?;
            return Ok(row.into());
        }

        // Competing inserts serialize on the database unique key. The deferred FK
        // permits the claim to be inserted before its case in this same transaction.
        let claim = sqlx::query!(
            r#"INSERT INTO command_idempotency (tenant_id, idempotency_key, command_digest, result_case_id)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (tenant_id, idempotency_key) DO NOTHING
               RETURNING idempotency_key"#,
            principal.tenant_id, command.idempotency_key, digest, case_id
        ).fetch_optional(&mut *tx).await?;

        if claim.is_none() {
            let prior = find_idempotency(&mut tx, principal.tenant_id, command.idempotency_key).await?
                .ok_or(RepositoryError::InconsistentState)?;
            if prior.command_digest != digest { return Err(RepositoryError::IdempotencyConflict); }
            let row = fetch_case(&mut tx, principal.tenant_id, prior.result_case_id).await?
                .ok_or(RepositoryError::InconsistentState)?;
            tx.commit().await?;
            return Ok(row.into());
        }

        let row = sqlx::query_as!(
            WorkCaseRow,
            r#"INSERT INTO work_cases (tenant_id, case_id, kind, title, state, revision, payload)
               VALUES ($1, $2, $3, $4, 'OPEN', 1, $5)
               RETURNING tenant_id, case_id, kind, title, state, revision,
                         payload AS "payload: Json<Value>""#,
            principal.tenant_id, case_id, command.kind.as_db_str(), command.title, Json(command.payload.clone())
        ).fetch_one(&mut *tx).await?;

        let activity_id = Uuid::new_v4();
        sqlx::query!(
            r#"INSERT INTO case_activity
               (tenant_id, case_id, sequence, activity_id, command_id, actor_id, actor_role,
                activity_type, reason, prior_revision, new_revision, details)
               VALUES ($1, $2, 1, $3, $4, $5, $6, 'CREATED', 'Work case created', 0, 1, $7)"#,
            principal.tenant_id, case_id, activity_id, command.idempotency_key, principal.actor_id,
            principal.role.as_db_str(), Json(json!({"kind": command.kind.as_db_str()}))
        ).execute(&mut *tx).await?;

        let outbox_id = Uuid::new_v4();
        sqlx::query!(
            r#"INSERT INTO case_outbox (tenant_id, outbox_id, case_id, revision, event_type, payload, status, attempts)
               VALUES ($1, $2, $3, 1, 'io.luminousdynamics.workcase.created', $4, 'PENDING', 0)"#,
            principal.tenant_id, outbox_id, case_id,
            Json(json!({"tenant_id": principal.tenant_id, "case_id": case_id, "revision": 1, "activity_id": activity_id}))
        ).execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(row.into())
    }

    /// Both the query predicate and RLS transaction context scope the read.
    pub async fn get_case(&self, principal: &VerifiedPrincipal, case_id: Uuid) -> Result<Option<WorkCase>, RepositoryError> {
        let mut tx = self.pool.begin().await?;
        set_tenant(&mut tx, principal.tenant_id).await?;
        let row = fetch_case(&mut tx, principal.tenant_id, case_id).await?;
        tx.commit().await?;
        Ok(row.map(Into::into))
    }
    #[cfg(test)]
    fn pool(&self) -> &PgPool { &self.pool }
}
impl From<WorkCaseRow> for WorkCase {
    fn from(row: WorkCaseRow) -> Self {
        let kind = match row.kind.as_str() {
            "INCIDENT" => CaseKind::Incident, "REQUEST" => CaseKind::Request,
            "PROBLEM" => CaseKind::Problem, "CHANGE" => CaseKind::Change,
            _ => unreachable!("database CHECK constraint restricts case kind"),
        };
        Self { tenant_id: row.tenant_id, case_id: row.case_id, kind, title: row.title,
            state: row.state, revision: row.revision, payload: row.payload.0 }
    }
}
fn command_digest(p: &VerifiedPrincipal, c: &CreateWorkCase) -> Result<String, RepositoryError> {
    let material = json!({
        "tenant_id": p.tenant_id, "actor_id": p.actor_id, "actor_role": p.role.as_db_str(),
        "kind": c.kind.as_db_str(), "title": c.title, "payload": c.payload
    });
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(&material)?)))
}
async fn set_tenant(tx: &mut Transaction<'_, Postgres>, tenant_id: Uuid) -> Result<(), sqlx::Error> {
    // Transaction-local state resets automatically before a pooled connection is reused.
    let _: String = sqlx::query_scalar("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant_id.to_string()).fetch_one(&mut **tx).await?;
    Ok(())
}
async fn find_idempotency(tx: &mut Transaction<'_, Postgres>, tenant_id: Uuid, key: Uuid)
    -> Result<Option<IdempotencyRow>, sqlx::Error> {
    sqlx::query_as!(
        IdempotencyRow,
        "SELECT command_digest, result_case_id FROM command_idempotency WHERE tenant_id = $1 AND idempotency_key = $2",
        tenant_id, key
    ).fetch_optional(&mut **tx).await
}
async fn fetch_case(tx: &mut Transaction<'_, Postgres>, tenant_id: Uuid, case_id: Uuid)
    -> Result<Option<WorkCaseRow>, sqlx::Error> {
    sqlx::query_as!(
        WorkCaseRow,
        r#"SELECT tenant_id, case_id, kind, title, state, revision, payload AS "payload: Json<Value>"
           FROM work_cases WHERE tenant_id = $1 AND case_id = $2"#,
        tenant_id, case_id
    ).fetch_optional(&mut **tx).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    async fn pool(max: u32) -> PgPool {
        let url = std::env::var("APP_DATABASE_URL")
            .expect("APP_DATABASE_URL must point to the non-owner PostgreSQL test role");
        PgPoolOptions::new().max_connections(max).connect(&url).await
            .expect("connect to PostgreSQL as the application role")
    }
    fn principal() -> VerifiedPrincipal {
        VerifiedPrincipal::from_verified_authentication(Uuid::new_v4(), Uuid::new_v4(), ActorRole::Operator)
    }
    fn command(key: Uuid, title: &str) -> CreateWorkCase {
        CreateWorkCase { idempotency_key: key, kind: CaseKind::Incident, title: title.into(),
            payload: json!({"asset_ref":"asset-synthetic-01","severity":"high"}) }
    }
    async fn unscoped_rls_count(pool: &PgPool, p: &VerifiedPrincipal) -> Result<i64, sqlx::Error> {
        let mut tx = pool.begin().await?;
        set_tenant(&mut tx, p.tenant_id).await?;
        // Deliberately no tenant predicate: this tests PostgreSQL RLS itself.
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
        let key = Uuid::new_v4();
        let first = repo.create_case(&p, command(key, "Synthetic endpoint incident")).await.expect("create");
        let replay = repo.create_case(&p, command(key, "Synthetic endpoint incident")).await.expect("replay");
        assert_eq!(first, replay);
        assert!(matches!(repo.create_case(&p, command(key, "Different command")).await,
            Err(RepositoryError::IdempotencyConflict)));
        let activity = sqlx::query_scalar!(
            r#"SELECT count(*)::BIGINT AS "count!" FROM case_activity WHERE tenant_id = $1 AND case_id = $2"#,
            p.tenant_id, first.case_id
        ).fetch_one(&pool).await.expect("activity count");
        let outbox = sqlx::query_scalar!(
            r#"SELECT count(*)::BIGINT AS "count!" FROM case_outbox WHERE tenant_id = $1 AND case_id = $2"#,
            p.tenant_id, first.case_id
        ).fetch_one(&pool).await.expect("outbox count");
        assert_eq!(activity, 1);
        assert_eq!(outbox, 1);
    }

    #[tokio::test]
    async fn rls_and_single_connection_reuse_isolate_tenants() {
        let pool = pool(1).await;
        let repo = WorkCaseRepository { pool: pool.clone() };
        let a = principal();
        let b = principal();
        let ca = repo.create_case(&a, command(Uuid::new_v4(), "Tenant A case")).await.expect("create A");
        let cb = repo.create_case(&b, command(Uuid::new_v4(), "Tenant B case")).await.expect("create B");
        assert!(repo.get_case(&b, ca.case_id).await.expect("cross-tenant read").is_none());
        assert_eq!(unscoped_rls_count(&pool, &a).await.expect("RLS A"), 1);
        assert_eq!(unscoped_rls_count(&pool, &b).await.expect("RLS B"), 1);
        assert_ne!(ca.tenant_id, cb.tenant_id);
    }

    #[tokio::test]
    async fn concurrent_duplicate_claim_commits_one_case_and_outbox() {
        let pool = pool(4).await;
        let repo = WorkCaseRepository { pool };
        let p = principal();
        let cmd = command(Uuid::new_v4(), "Concurrent synthetic case");
        let (a, b) = tokio::join!(repo.create_case(&p, cmd.clone()), repo.create_case(&p, cmd));
        let a = a.expect("first command");
        let b = b.expect("duplicate command");
        assert_eq!(a.case_id, b.case_id);
        let n = sqlx::query_scalar!(
            r#"SELECT count(*)::BIGINT AS "count!" FROM case_outbox WHERE tenant_id = $1 AND case_id = $2"#,
            p.tenant_id, a.case_id
        ).fetch_one(repo.pool()).await.expect("count outbox");
        assert_eq!(n, 1);
    }
}
