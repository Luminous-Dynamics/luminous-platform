//! Durable session journal for deterministic fleet rollout waves.
//!
//! The session journal persists orchestration intent and per-target command
//! identities so a controller restart cannot confuse a partial wave with a
//! complete wave. It does not apply policies or authenticate device evidence.

#![forbid(unsafe_code)]

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};

use super::fleet_rollout::{
    FleetRolloutPlan, FleetTarget, FleetTargetObservation, RolloutWave, WaveDecision,
    WaveEvaluation, digest_plan, evaluate_wave, plan_rollout, validate_target,
};

const SCHEMA_VERSION: i64 = 1;
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

const CREATE_SESSIONS: &str = "
CREATE TABLE rollout_sessions (
    plan_sha256 TEXT PRIMARY KEY NOT NULL CHECK(length(plan_sha256) = 64),
    state TEXT NOT NULL CHECK(state IN ('planned', 'running', 'halted', 'completed')),
    active_wave_index INTEGER,
    created_at_unix_ms INTEGER NOT NULL CHECK(created_at_unix_ms >= 0),
    updated_at_unix_ms INTEGER NOT NULL CHECK(updated_at_unix_ms >= 0)
);
";

const CREATE_WAVES: &str = "
CREATE TABLE rollout_waves (
    plan_sha256 TEXT NOT NULL,
    wave_index INTEGER NOT NULL CHECK(wave_index >= 0),
    state TEXT NOT NULL CHECK(state IN
        ('planned', 'applying', 'applied', 'verified', 'rollback_required',
         'rolling_back', 'continued', 'rolled_back', 'halted')),
    updated_at_unix_ms INTEGER NOT NULL CHECK(updated_at_unix_ms >= 0),
    PRIMARY KEY(plan_sha256, wave_index),
    FOREIGN KEY(plan_sha256) REFERENCES rollout_sessions(plan_sha256)
);
";

const CREATE_TARGETS: &str = "
CREATE TABLE rollout_targets (
    plan_sha256 TEXT NOT NULL,
    target_token TEXT NOT NULL,
    wave_index INTEGER NOT NULL CHECK(wave_index >= 0),
    failure_domain TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK(generation > 0),
    baseline_policy_sha256 TEXT NOT NULL CHECK(length(baseline_policy_sha256) = 64),
    candidate_policy_sha256 TEXT NOT NULL CHECK(length(candidate_policy_sha256) = 64),
    canary INTEGER NOT NULL CHECK(canary IN (0, 1)),
    state TEXT NOT NULL CHECK(state IN
        ('planned', 'apply_intent', 'applied', 'promoted', 'rollback_intent', 'rolled_back')),
    apply_command_id TEXT,
    rollback_command_id TEXT,
    observed_policy_sha256 TEXT,
    updated_at_unix_ms INTEGER NOT NULL CHECK(updated_at_unix_ms >= 0),
    PRIMARY KEY(plan_sha256, target_token),
    UNIQUE(plan_sha256, apply_command_id),
    UNIQUE(plan_sha256, rollback_command_id),
    FOREIGN KEY(plan_sha256, wave_index)
        REFERENCES rollout_waves(plan_sha256, wave_index)
);
";

/// Durable state for one deterministic rollout plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetSessionSnapshot {
    pub plan_sha256: String,
    pub state: FleetSessionState,
    pub active_wave_index: Option<usize>,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetSessionState {
    Planned,
    Running,
    Halted,
    Completed,
}

impl FleetSessionState {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "planned" => Ok(Self::Planned),
            "running" => Ok(Self::Running),
            "halted" => Ok(Self::Halted),
            "completed" => Ok(Self::Completed),
            _ => Err(format!(
                "unknown fleet session state {value:?}; refusing recovery"
            )),
        }
    }
}

/// Stable idempotency identity committed before the external apply command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyCommand {
    pub plan_sha256: String,
    pub wave_index: usize,
    pub target_token: String,
    pub command_id: String,
    pub candidate_policy_sha256: String,
}

/// Stable idempotency identity committed before external rollback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RollbackCommand {
    pub plan_sha256: String,
    pub wave_index: usize,
    pub target_token: String,
    pub command_id: String,
    pub baseline_policy_sha256: String,
}

/// What a restarted controller must do before it can continue a target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetRecoveryAction {
    InspectActualPolicyBeforeRetry,
    RequireFreshEvidenceAndPromotionReconciliation,
    ReconcilePromotionJournalAndEffectivePolicy,
    PersistRollbackIntentAndEnforceBaseline,
    EnforceBaselinePolicyAndVerify,
    HaltAndReconcile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverableFleetTarget {
    pub plan_sha256: String,
    pub wave_index: usize,
    pub target_token: String,
    pub generation: u64,
    pub state: String,
    pub wave_state: String,
    pub action: FleetRecoveryAction,
}

/// SQLite-backed durable fleet rollout session journal.
///
/// Use a dedicated database file within a trusted, permission-restricted
/// directory. The state journal and target promotion journals are separate
/// durable systems; a controller must reconcile them after an interrupted call.
#[derive(Clone, Debug)]
pub struct FleetSessionJournal {
    path: PathBuf,
}

impl FleetSessionJournal {
    /// Open or initialize a durable fleet session journal.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if path.as_os_str().is_empty() || path.to_string_lossy() == ":memory:" {
            return Err("fleet session journal requires a durable file path".to_owned());
        }
        ensure_private_file(&path)?;
        let connection = connect(&path)?;
        let has_sessions = table_exists(&connection, "rollout_sessions")?;
        if !has_sessions {
            let existing_tables: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master
                     WHERE type = 'table' AND name NOT LIKE 'sqlite_%';",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| format!("cannot inspect fleet journal database: {error}"))?;
            let version = schema_version(&connection)?;
            if existing_tables != 0 || version != 0 {
                return Err(
                    "refusing to initialize a non-empty database as a fleet session journal"
                        .to_owned(),
                );
            }
            connection
                .execute_batch(&format!(
                    "PRAGMA journal_mode = WAL;\n{CREATE_SESSIONS}\n{CREATE_WAVES}\n{CREATE_TARGETS}\nPRAGMA user_version = {SCHEMA_VERSION};"
                ))
                .map_err(|error| format!("cannot initialize fleet session schema: {error}"))?;
        } else {
            connection
                .execute_batch("PRAGMA journal_mode = WAL;")
                .map_err(|error| format!("cannot enable fleet session WAL mode: {error}"))?;
        }
        verify_schema(&connection)?;
        Ok(Self { path })
    }

    /// Register an immutable plan before any external actuation.
    ///
    /// Re-registering the same canonical plan is idempotent. A plan digest binds
    /// the declared constraints and target/wave contents but is not authorization.
    pub fn register_plan(
        &self,
        plan: &FleetRolloutPlan,
        now_unix_ms: u64,
    ) -> Result<FleetSessionSnapshot, String> {
        validate_plan(plan)?;
        let now = to_sql_integer(now_unix_ms, "registration timestamp")?;
        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin fleet plan registration: {error}"))?;

        if let Some(existing) = get_session_conn(&transaction, &plan.plan_sha256)? {
            let target_count: i64 = transaction
                .query_row(
                    "SELECT COUNT(*) FROM rollout_targets WHERE plan_sha256 = ?1;",
                    params![plan.plan_sha256],
                    |row| row.get(0),
                )
                .map_err(|error| format!("cannot validate registered target count: {error}"))?;
            let wave_count: i64 = transaction
                .query_row(
                    "SELECT COUNT(*) FROM rollout_waves WHERE plan_sha256 = ?1;",
                    params![plan.plan_sha256],
                    |row| row.get(0),
                )
                .map_err(|error| format!("cannot validate registered wave count: {error}"))?;
            let expected_targets: usize = plan.waves.iter().map(|wave| wave.targets.len()).sum();
            if usize::try_from(target_count).ok() != Some(expected_targets)
                || usize::try_from(wave_count).ok() != Some(plan.waves.len())
            {
                return Err(
                    "existing fleet session does not match the registered plan shape".to_owned(),
                );
            }
            transaction
                .commit()
                .map_err(|error| format!("cannot commit idempotent registration: {error}"))?;
            return Ok(existing);
        }

        transaction
            .execute(
                "INSERT INTO rollout_sessions
                 (plan_sha256, state, active_wave_index, created_at_unix_ms, updated_at_unix_ms)
                 VALUES (?1, 'planned', NULL, ?2, ?2);",
                params![plan.plan_sha256, now],
            )
            .map_err(|error| format!("cannot persist fleet session: {error}"))?;

        for wave in &plan.waves {
            transaction
                .execute(
                    "INSERT INTO rollout_waves (plan_sha256, wave_index, state, updated_at_unix_ms)
                     VALUES (?1, ?2, 'planned', ?3);",
                    params![
                        plan.plan_sha256,
                        to_sql_integer(wave.index as u64, "wave index")?,
                        now
                    ],
                )
                .map_err(|error| format!("cannot persist planned wave: {error}"))?;
            for target in &wave.targets {
                transaction
                    .execute(
                        "INSERT INTO rollout_targets
                         (plan_sha256, target_token, wave_index, failure_domain, generation,
                          baseline_policy_sha256, candidate_policy_sha256, canary, state,
                          apply_command_id, rollback_command_id, observed_policy_sha256,
                          updated_at_unix_ms)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'planned', NULL, NULL, NULL, ?9);",
                        params![
                            plan.plan_sha256,
                            target.target_token,
                            to_sql_integer(wave.index as u64, "wave index")?,
                            target.failure_domain,
                            to_sql_integer(target.generation, "policy generation")?,
                            target.baseline_policy_sha256,
                            target.candidate_policy_sha256,
                            if target.canary { 1_i64 } else { 0_i64 },
                            now
                        ],
                    )
                    .map_err(|error| format!("cannot persist planned target: {error}"))?;
            }
        }
        let snapshot = get_session_conn(&transaction, &plan.plan_sha256)?
            .ok_or_else(|| "registered fleet session disappeared before commit".to_owned())?;
        transaction
            .commit()
            .map_err(|error| format!("cannot commit fleet plan registration: {error}"))?;
        Ok(snapshot)
    }

    /// Start the next eligible wave. Repeating a start for the current active
    /// wave is idempotent; skipping a wave is not allowed.
    pub fn begin_wave(
        &self,
        plan_sha256: &str,
        wave_index: usize,
        now_unix_ms: u64,
    ) -> Result<(), String> {
        validate_digest(plan_sha256, "plan_sha256")?;
        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin wave-start transaction: {error}"))?;
        let session = get_session_conn(&transaction, plan_sha256)?
            .ok_or_else(|| "unknown fleet plan digest".to_owned())?;
        if matches!(
            session.state,
            FleetSessionState::Halted | FleetSessionState::Completed
        ) {
            return Err(format!(
                "cannot start a wave for session in state {:?}",
                session.state
            ));
        }
        if let Some(active) = session.active_wave_index {
            if active == wave_index {
                let state = get_wave_state(&transaction, plan_sha256, wave_index)?;
                if matches!(
                    state.as_str(),
                    "applying" | "applied" | "verified" | "rollback_required" | "rolling_back"
                ) {
                    transaction
                        .commit()
                        .map_err(|error| format!("cannot commit idempotent wave start: {error}"))?;
                    return Ok(());
                }
            }
            return Err(format!(
                "wave {active} is already active; cannot start wave {wave_index}"
            ));
        }

        if wave_index > 0 {
            let prior = get_wave_state(&transaction, plan_sha256, wave_index - 1)?;
            if prior != "continued" {
                return Err(format!(
                    "wave {} cannot begin until prior wave is continued; prior state is {prior:?}",
                    wave_index
                ));
            }
        }
        let wave = get_wave_state(&transaction, plan_sha256, wave_index)?;
        if wave != "planned" {
            return Err(format!("cannot begin wave {wave_index} in state {wave:?}"));
        }
        let now = to_sql_integer(now_unix_ms, "wave start timestamp")?;
        let changed = transaction
            .execute(
                "UPDATE rollout_waves SET state = 'applying', updated_at_unix_ms = ?1
                 WHERE plan_sha256 = ?2 AND wave_index = ?3 AND state = 'planned';",
                params![
                    now,
                    plan_sha256,
                    to_sql_integer(wave_index as u64, "wave index")?
                ],
            )
            .map_err(|error| format!("cannot start rollout wave: {error}"))?;
        if changed != 1 {
            return Err("wave state changed concurrently; refusing start".to_owned());
        }
        transaction
            .execute(
                "UPDATE rollout_sessions SET state = 'running', active_wave_index = ?1,
                    updated_at_unix_ms = ?2 WHERE plan_sha256 = ?3;",
                params![
                    to_sql_integer(wave_index as u64, "wave index")?,
                    now,
                    plan_sha256
                ],
            )
            .map_err(|error| format!("cannot persist active fleet wave: {error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("cannot commit wave start: {error}"))?;
        Ok(())
    }

    /// Persist an apply intent and deterministic idempotency key before sending
    /// an external command. A retry in ApplyIntent returns the same key.
    pub fn record_apply_intent(
        &self,
        plan_sha256: &str,
        wave_index: usize,
        target_token: &str,
        now_unix_ms: u64,
    ) -> Result<ApplyCommand, String> {
        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin target apply-intent transaction: {error}"))?;
        require_active_wave(&transaction, plan_sha256, wave_index)?;
        let target = get_target_conn(&transaction, plan_sha256, target_token)?
            .ok_or_else(|| format!("target {target_token:?} is not registered in this plan"))?;
        if target.wave_index != wave_index {
            return Err(format!(
                "target {target_token:?} belongs to wave {}, not {wave_index}",
                target.wave_index
            ));
        }
        if target.state == "apply_intent" {
            let command = target
                .apply_command_id
                .ok_or_else(|| "apply_intent target lacks a durable command ID".to_owned())?;
            transaction
                .commit()
                .map_err(|error| format!("cannot commit idempotent apply intent: {error}"))?;
            return Ok(ApplyCommand {
                plan_sha256: plan_sha256.to_owned(),
                wave_index,
                target_token: target_token.to_owned(),
                command_id: command,
                candidate_policy_sha256: target.candidate_policy_sha256,
            });
        }
        if target.state != "planned" {
            return Err(format!(
                "cannot record apply intent for target in state {:?}",
                target.state
            ));
        }
        let command_id = command_identity(
            plan_sha256,
            wave_index,
            target_token,
            target.generation,
            "apply",
        );
        let now = to_sql_integer(now_unix_ms, "apply-intent timestamp")?;
        let changed = transaction
            .execute(
                "UPDATE rollout_targets SET state = 'apply_intent', apply_command_id = ?1,
                    updated_at_unix_ms = ?2
                 WHERE plan_sha256 = ?3 AND target_token = ?4 AND state = 'planned';",
                params![command_id, now, plan_sha256, target_token],
            )
            .map_err(|error| format!("cannot persist target apply intent: {error}"))?;
        if changed != 1 {
            return Err("target state changed concurrently; refusing apply intent".to_owned());
        }
        transaction
            .commit()
            .map_err(|error| format!("cannot commit target apply intent: {error}"))?;
        Ok(ApplyCommand {
            plan_sha256: plan_sha256.to_owned(),
            wave_index,
            target_token: target_token.to_owned(),
            command_id,
            candidate_policy_sha256: target.candidate_policy_sha256,
        })
    }

    /// Record a target's applied state only after live effective-policy readback
    /// exactly equals the candidate digest. Identical retry is idempotent.
    pub fn record_applied(
        &self,
        plan_sha256: &str,
        wave_index: usize,
        target_token: &str,
        effective_policy_sha256: &str,
        now_unix_ms: u64,
    ) -> Result<(), String> {
        validate_digest(effective_policy_sha256, "effective_policy_sha256")?;
        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin applied-state transaction: {error}"))?;
        require_active_wave(&transaction, plan_sha256, wave_index)?;
        let target = get_target_conn(&transaction, plan_sha256, target_token)?
            .ok_or_else(|| format!("target {target_token:?} is not registered in this plan"))?;
        if target.wave_index != wave_index {
            return Err(format!(
                "target {target_token:?} is not in wave {wave_index}"
            ));
        }
        if effective_policy_sha256 != target.candidate_policy_sha256 {
            return Err("effective policy readback does not match the candidate digest".to_owned());
        }
        if target.state == "applied"
            && target.observed_policy_sha256.as_deref() == Some(effective_policy_sha256)
        {
            transaction
                .commit()
                .map_err(|error| format!("cannot commit idempotent applied state: {error}"))?;
            return Ok(());
        }
        if target.state != "apply_intent" {
            return Err(format!(
                "cannot record applied policy from target state {:?}",
                target.state
            ));
        }
        let now = to_sql_integer(now_unix_ms, "applied timestamp")?;
        transaction
            .execute(
                "UPDATE rollout_targets SET state = 'applied', observed_policy_sha256 = ?1,
                    updated_at_unix_ms = ?2
                 WHERE plan_sha256 = ?3 AND target_token = ?4 AND state = 'apply_intent';",
                params![effective_policy_sha256, now, plan_sha256, target_token],
            )
            .map_err(|error| format!("cannot persist applied policy digest: {error}"))?;
        let pending: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM rollout_targets
                 WHERE plan_sha256 = ?1 AND wave_index = ?2 AND state != 'applied';",
                params![
                    plan_sha256,
                    to_sql_integer(wave_index as u64, "wave index")?
                ],
                |row| row.get(0),
            )
            .map_err(|error| format!("cannot count pending wave targets: {error}"))?;
        if pending == 0 {
            transaction
                .execute(
                    "UPDATE rollout_waves SET state = 'applied', updated_at_unix_ms = ?1
                     WHERE plan_sha256 = ?2 AND wave_index = ?3 AND state = 'applying';",
                    params![
                        now,
                        plan_sha256,
                        to_sql_integer(wave_index as u64, "wave index")?
                    ],
                )
                .map_err(|error| format!("cannot mark wave applied: {error}"))?;
        }
        transaction
            .commit()
            .map_err(|error| format!("cannot commit applied-state record: {error}"))?;
        Ok(())
    }

    /// Recompute the wave decision from the supplied observations and durably
    /// record it. The coordinator must obtain observations through the verified
    /// freshness/replay gate; this method is not an evidence authenticator.
    pub fn evaluate_and_record_wave(
        &self,
        plan_sha256: &str,
        wave_index: usize,
        observations: &[FleetTargetObservation],
        now_unix_ms: u64,
    ) -> Result<WaveEvaluation, String> {
        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin wave evaluation transaction: {error}"))?;
        let session = get_session_conn(&transaction, plan_sha256)?
            .ok_or_else(|| "unknown fleet plan digest".to_owned())?;
        let state = get_wave_state(&transaction, plan_sha256, wave_index)?;
        let wave = get_wave_conn(&transaction, plan_sha256, wave_index)?;
        let evaluation = evaluate_wave(&wave, observations);

        // Retries after a lost database response must not alter a persisted
        // decision. They succeed only if the same class of decision is reproduced.
        if state != "applied" {
            let persisted = match state.as_str() {
                "verified" | "continued" => Some(WaveDecision::Continue),
                "rollback_required" | "rolling_back" | "rolled_back" => {
                    Some(WaveDecision::RollbackWave)
                }
                "halted" => Some(WaveDecision::HaltAndReconcile),
                _ => None,
            };
            if persisted == Some(evaluation.decision) {
                transaction.commit().map_err(|error| {
                    format!("cannot commit idempotent wave evaluation: {error}")
                })?;
                return Ok(evaluation);
            }
            return Err(format!(
                "wave {wave_index} is already in state {state:?}; refusing a different or premature decision"
            ));
        }
        if session.state != FleetSessionState::Running
            || session.active_wave_index != Some(wave_index)
        {
            return Err(format!("wave {wave_index} is not the active fleet wave"));
        }

        let now = to_sql_integer(now_unix_ms, "wave evaluation timestamp")?;
        match evaluation.decision {
            WaveDecision::Continue => {
                transaction
                    .execute(
                        "UPDATE rollout_waves SET state = 'verified', updated_at_unix_ms = ?1
                         WHERE plan_sha256 = ?2 AND wave_index = ?3 AND state = 'applied';",
                        params![
                            now,
                            plan_sha256,
                            to_sql_integer(wave_index as u64, "wave index")?
                        ],
                    )
                    .map_err(|error| format!("cannot persist verified wave decision: {error}"))?;
            }
            WaveDecision::RollbackWave => {
                transaction
                    .execute(
                        "UPDATE rollout_waves SET state = 'rollback_required', updated_at_unix_ms = ?1
                         WHERE plan_sha256 = ?2 AND wave_index = ?3 AND state = 'applied';",
                        params![now, plan_sha256, to_sql_integer(wave_index as u64, "wave index")?],
                    )
                    .map_err(|error| format!("cannot persist rollback-required decision: {error}"))?;
            }
            WaveDecision::HaltAndReconcile => {
                transaction
                    .execute(
                        "UPDATE rollout_waves SET state = 'halted', updated_at_unix_ms = ?1
                         WHERE plan_sha256 = ?2 AND wave_index = ?3 AND state = 'applied';",
                        params![
                            now,
                            plan_sha256,
                            to_sql_integer(wave_index as u64, "wave index")?
                        ],
                    )
                    .map_err(|error| format!("cannot persist halt decision: {error}"))?;
                transaction
                    .execute(
                        "UPDATE rollout_sessions SET state = 'halted', active_wave_index = NULL,
                            updated_at_unix_ms = ?1 WHERE plan_sha256 = ?2;",
                        params![now, plan_sha256],
                    )
                    .map_err(|error| format!("cannot halt fleet rollout session: {error}"))?;
            }
        }
        transaction
            .commit()
            .map_err(|error| format!("cannot commit wave decision: {error}"))?;
        Ok(evaluation)
    }

    /// Record one target promoted by its PromotionJournal after promotion
    /// returned successfully and the effective policy was re-read exactly.
    pub fn record_target_promoted(
        &self,
        plan_sha256: &str,
        wave_index: usize,
        target_token: &str,
        effective_policy_sha256: &str,
        now_unix_ms: u64,
    ) -> Result<(), String> {
        validate_digest(effective_policy_sha256, "effective_policy_sha256")?;
        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin target promotion transaction: {error}"))?;
        let target = get_target_conn(&transaction, plan_sha256, target_token)?
            .ok_or_else(|| format!("target {target_token:?} is not registered"))?;
        if target.wave_index != wave_index {
            return Err(format!(
                "target {target_token:?} is not in wave {wave_index}"
            ));
        }
        if target.candidate_policy_sha256 != effective_policy_sha256
            || target.observed_policy_sha256.as_deref() != Some(effective_policy_sha256)
        {
            return Err("promotion digest does not match the exact applied candidate".to_owned());
        }
        // A caller may retry after the PromotionJournal and fleet journal commit
        // but before its own response arrives. Accept only the same exact result.
        if target.state == "promoted" {
            transaction
                .commit()
                .map_err(|error| format!("cannot commit idempotent target promotion: {error}"))?;
            return Ok(());
        }
        require_active_wave(&transaction, plan_sha256, wave_index)?;
        let wave = get_wave_state(&transaction, plan_sha256, wave_index)?;
        if wave != "verified" {
            return Err(format!(
                "cannot record target promotion from wave state {wave:?}"
            ));
        }
        if target.state != "applied" {
            return Err("target is not an applied member of the verified wave".to_owned());
        }
        let now = to_sql_integer(now_unix_ms, "promotion timestamp")?;
        transaction
            .execute(
                "UPDATE rollout_targets SET state = 'promoted', updated_at_unix_ms = ?1
                 WHERE plan_sha256 = ?2 AND target_token = ?3 AND state = 'applied';",
                params![now, plan_sha256, target_token],
            )
            .map_err(|error| format!("cannot persist promoted target state: {error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("cannot commit target promotion: {error}"))?;
        Ok(())
    }

    /// Close the wave only when every target has a durable promoted state.
    /// A subsequent wave cannot start until this method succeeds.
    pub fn complete_wave(
        &self,
        plan_sha256: &str,
        wave_index: usize,
        now_unix_ms: u64,
    ) -> Result<FleetSessionSnapshot, String> {
        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin wave-completion transaction: {error}"))?;
        let state = get_wave_state(&transaction, plan_sha256, wave_index)?;
        let session = get_session_conn(&transaction, plan_sha256)?
            .ok_or_else(|| "unknown fleet plan digest".to_owned())?;
        if state == "continued" && session.active_wave_index != Some(wave_index) {
            transaction
                .commit()
                .map_err(|error| format!("cannot commit idempotent wave completion: {error}"))?;
            return Ok(session);
        }
        require_active_wave(&transaction, plan_sha256, wave_index)?;
        if state != "verified" {
            return Err(format!("cannot complete wave in state {state:?}"));
        }
        let not_promoted: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM rollout_targets
                 WHERE plan_sha256 = ?1 AND wave_index = ?2 AND state != 'promoted';",
                params![
                    plan_sha256,
                    to_sql_integer(wave_index as u64, "wave index")?
                ],
                |row| row.get(0),
            )
            .map_err(|error| format!("cannot count unpromoted wave targets: {error}"))?;
        if not_promoted != 0 {
            return Err(format!(
                "cannot complete wave while {not_promoted} target(s) are not durably promoted"
            ));
        }
        let now = to_sql_integer(now_unix_ms, "wave completion timestamp")?;
        transaction
            .execute(
                "UPDATE rollout_waves SET state = 'continued', updated_at_unix_ms = ?1
                 WHERE plan_sha256 = ?2 AND wave_index = ?3 AND state = 'verified';",
                params![
                    now,
                    plan_sha256,
                    to_sql_integer(wave_index as u64, "wave index")?
                ],
            )
            .map_err(|error| format!("cannot close rollout wave: {error}"))?;
        let wave_count: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM rollout_waves WHERE plan_sha256 = ?1;",
                params![plan_sha256],
                |row| row.get(0),
            )
            .map_err(|error| format!("cannot count rollout waves: {error}"))?;
        let is_final = i64::try_from(wave_index + 1).ok() == Some(wave_count);
        transaction
            .execute(
                "UPDATE rollout_sessions SET state = ?1, active_wave_index = NULL,
                    updated_at_unix_ms = ?2 WHERE plan_sha256 = ?3;",
                params![
                    if is_final { "completed" } else { "running" },
                    now,
                    plan_sha256
                ],
            )
            .map_err(|error| format!("cannot persist wave completion: {error}"))?;
        let updated = get_session_conn(&transaction, plan_sha256)?
            .ok_or_else(|| "fleet session disappeared at wave completion".to_owned())?;
        transaction
            .commit()
            .map_err(|error| format!("cannot commit wave completion: {error}"))?;
        Ok(updated)
    }

    /// Persist rollback intent for every applied/promoted target in the current
    /// wave and return stable rollback command IDs. Repeated calls are idempotent.
    pub fn request_wave_rollback(
        &self,
        plan_sha256: &str,
        wave_index: usize,
        now_unix_ms: u64,
    ) -> Result<Vec<RollbackCommand>, String> {
        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin wave rollback transaction: {error}"))?;
        let wave_state = get_wave_state(&transaction, plan_sha256, wave_index)?;
        let session = get_session_conn(&transaction, plan_sha256)?
            .ok_or_else(|| "unknown fleet plan digest".to_owned())?;

        // A retry after the final rollback commit returns the same complete
        // command set instead of reporting an error after the side effect exists.
        if wave_state == "rolled_back"
            && session.state == FleetSessionState::Halted
            && session.active_wave_index.is_none()
        {
            let targets = get_wave_targets_conn(&transaction, plan_sha256, wave_index)?;
            let mut commands = Vec::with_capacity(targets.len());
            for target in targets {
                if target.state != "rolled_back" {
                    return Err("rolled_back wave contains a nonterminal target".to_owned());
                }
                commands.push(RollbackCommand {
                    plan_sha256: plan_sha256.to_owned(),
                    wave_index,
                    target_token: target.target_token,
                    command_id: target
                        .rollback_command_id
                        .ok_or_else(|| "rolled_back target lacks rollback command ID".to_owned())?,
                    baseline_policy_sha256: target.baseline_policy_sha256,
                });
            }
            transaction
                .commit()
                .map_err(|error| format!("cannot commit idempotent rollback request: {error}"))?;
            return Ok(commands);
        }

        require_active_wave(&transaction, plan_sha256, wave_index)?;
        if !matches!(
            wave_state.as_str(),
            "verified" | "rollback_required" | "rolling_back"
        ) {
            return Err(format!(
                "cannot request rollback from wave state {wave_state:?}"
            ));
        }
        let targets = get_wave_targets_conn(&transaction, plan_sha256, wave_index)?;
        let now = to_sql_integer(now_unix_ms, "rollback-intent timestamp")?;
        let mut commands = Vec::new();
        for target in targets {
            let command_id = match target.state.as_str() {
                "applied" | "promoted" => {
                    let id = command_identity(
                        plan_sha256,
                        wave_index,
                        &target.target_token,
                        target.generation,
                        "rollback",
                    );
                    transaction
                        .execute(
                            "UPDATE rollout_targets SET state = 'rollback_intent',
                                rollback_command_id = ?1, updated_at_unix_ms = ?2
                             WHERE plan_sha256 = ?3 AND target_token = ?4 AND state IN ('applied', 'promoted');",
                            params![id, now, plan_sha256, target.target_token],
                        )
                        .map_err(|error| format!("cannot persist target rollback intent: {error}"))?;
                    id
                }
                "rollback_intent" | "rolled_back" => target
                    .rollback_command_id
                    .ok_or_else(|| "rollback target lacks durable command ID".to_owned())?,
                other => {
                    return Err(format!(
                        "target {:?} in state {other:?} cannot be rolled back as part of a complete applied wave",
                        target.target_token
                    ));
                }
            };
            // Return the full stable command set even when part of the wave has
            // already rolled back; adapters can safely deduplicate by command ID.
            commands.push(RollbackCommand {
                plan_sha256: plan_sha256.to_owned(),
                wave_index,
                target_token: target.target_token,
                command_id,
                baseline_policy_sha256: target.baseline_policy_sha256,
            });
        }
        let wave_changed = transaction
            .execute(
                "UPDATE rollout_waves SET state = 'rolling_back', updated_at_unix_ms = ?1
                 WHERE plan_sha256 = ?2 AND wave_index = ?3 AND state IN ('verified', 'rollback_required', 'rolling_back');",
                params![now, plan_sha256, to_sql_integer(wave_index as u64, "wave index")?],
            )
            .map_err(|error| format!("cannot set rolling-back wave state: {error}"))?;
        if wave_changed != 1 {
            return Err("wave rollback state changed concurrently".to_owned());
        }
        transaction
            .commit()
            .map_err(|error| format!("cannot commit wave rollback intent: {error}"))?;
        Ok(commands)
    }

    /// Record rollback completion only when live readback exactly equals the
    /// target's original baseline digest. A command acknowledgement is not enough.
    pub fn record_target_rolled_back(
        &self,
        plan_sha256: &str,
        wave_index: usize,
        target_token: &str,
        effective_policy_sha256: &str,
        now_unix_ms: u64,
    ) -> Result<bool, String> {
        validate_digest(effective_policy_sha256, "effective_policy_sha256")?;
        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin target rollback completion: {error}"))?;
        let wave_state = get_wave_state(&transaction, plan_sha256, wave_index)?;
        let target = get_target_conn(&transaction, plan_sha256, target_token)?
            .ok_or_else(|| format!("target {target_token:?} is not registered"))?;
        if target.wave_index != wave_index {
            return Err(format!(
                "target {target_token:?} is not in wave {wave_index}"
            ));
        }
        if target.baseline_policy_sha256 != effective_policy_sha256 {
            return Err(
                "rollback readback does not match the exact baseline policy digest".to_owned(),
            );
        }
        // A retry after the journal commit is idempotent, including the case
        // where the entire wave and session have already reached terminal state.
        if target.state == "rolled_back"
            && target.observed_policy_sha256.as_deref() == Some(effective_policy_sha256)
        {
            let complete = wave_state == "rolled_back";
            if !matches!(wave_state.as_str(), "rolling_back" | "rolled_back") {
                return Err(format!(
                    "rolled-back target is in unexpected wave state {wave_state:?}"
                ));
            }
            transaction
                .commit()
                .map_err(|error| format!("cannot commit idempotent rollback record: {error}"))?;
            return Ok(complete);
        }

        require_active_wave(&transaction, plan_sha256, wave_index)?;
        if wave_state != "rolling_back" || target.state != "rollback_intent" {
            return Err("target rollback completion requires a rollback_intent target in a rolling_back wave".to_owned());
        }
        let now = to_sql_integer(now_unix_ms, "rollback completion timestamp")?;
        transaction
            .execute(
                "UPDATE rollout_targets SET state = 'rolled_back',
                    observed_policy_sha256 = ?1, updated_at_unix_ms = ?2
                 WHERE plan_sha256 = ?3 AND target_token = ?4 AND state = 'rollback_intent';",
                params![effective_policy_sha256, now, plan_sha256, target_token],
            )
            .map_err(|error| format!("cannot persist target rollback completion: {error}"))?;
        let remaining: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM rollout_targets
                 WHERE plan_sha256 = ?1 AND wave_index = ?2 AND state != 'rolled_back';",
                params![
                    plan_sha256,
                    to_sql_integer(wave_index as u64, "wave index")?
                ],
                |row| row.get(0),
            )
            .map_err(|error| format!("cannot count outstanding rollbacks: {error}"))?;
        let complete = remaining == 0;
        if complete {
            transaction
                .execute(
                    "UPDATE rollout_waves SET state = 'rolled_back', updated_at_unix_ms = ?1
                     WHERE plan_sha256 = ?2 AND wave_index = ?3 AND state = 'rolling_back';",
                    params![
                        now,
                        plan_sha256,
                        to_sql_integer(wave_index as u64, "wave index")?
                    ],
                )
                .map_err(|error| format!("cannot finalize wave rollback: {error}"))?;
            transaction
                .execute(
                    "UPDATE rollout_sessions SET state = 'halted', active_wave_index = NULL,
                        updated_at_unix_ms = ?1 WHERE plan_sha256 = ?2;",
                    params![now, plan_sha256],
                )
                .map_err(|error| format!("cannot halt rollout after rollback: {error}"))?;
        }
        transaction
            .commit()
            .map_err(|error| format!("cannot commit target rollback record: {error}"))?;
        Ok(complete)
    }

    pub fn get_session(&self, plan_sha256: &str) -> Result<Option<FleetSessionSnapshot>, String> {
        validate_digest(plan_sha256, "plan_sha256")?;
        let connection = connect(&self.path)?;
        get_session_conn(&connection, plan_sha256)
    }

    /// Report every nonterminal target whose command/evidence may need explicit
    /// reconciliation. This does not retry or roll back anything automatically.
    pub fn list_recoverable_targets(&self) -> Result<Vec<RecoverableFleetTarget>, String> {
        let connection = connect(&self.path)?;
        let mut statement = connection
            .prepare(
                "SELECT t.plan_sha256, t.wave_index, t.target_token, t.generation, t.state, w.state
                 FROM rollout_targets AS t
                 JOIN rollout_waves AS w
                   ON w.plan_sha256 = t.plan_sha256 AND w.wave_index = t.wave_index
                 WHERE
                   (t.state = 'apply_intent' AND w.state IN ('applying', 'halted'))
                   OR (t.state = 'applied' AND w.state IN
                       ('applied', 'verified', 'rollback_required', 'rolling_back', 'halted'))
                   OR (t.state = 'promoted' AND w.state IN ('verified', 'rolling_back'))
                   OR (t.state = 'rollback_intent' AND w.state = 'rolling_back')
                 ORDER BY t.plan_sha256, t.wave_index, t.target_token;",
            )
            .map_err(|error| format!("cannot query recoverable fleet targets: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                let state: String = row.get(4)?;
                let wave_state: String = row.get(5)?;
                let action = match (state.as_str(), wave_state.as_str()) {
                    ("apply_intent", _) => FleetRecoveryAction::InspectActualPolicyBeforeRetry,
                    ("applied", "rollback_required") => {
                        FleetRecoveryAction::PersistRollbackIntentAndEnforceBaseline
                    }
                    ("applied", "halted") => FleetRecoveryAction::HaltAndReconcile,
                    ("applied", _) => {
                        FleetRecoveryAction::RequireFreshEvidenceAndPromotionReconciliation
                    }
                    ("promoted", _) => {
                        FleetRecoveryAction::ReconcilePromotionJournalAndEffectivePolicy
                    }
                    ("rollback_intent", _) => FleetRecoveryAction::EnforceBaselinePolicyAndVerify,
                    _ => {
                        return Err(rusqlite::Error::InvalidColumnType(
                            4,
                            "state".to_owned(),
                            rusqlite::types::Type::Text,
                        ));
                    }
                };
                Ok(RecoverableFleetTarget {
                    plan_sha256: row.get(0)?,
                    wave_index: u64_to_usize(row.get(1)?)
                        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(1, i64::MAX))?,
                    target_token: row.get(2)?,
                    generation: u64::try_from(row.get::<_, i64>(3)?)
                        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(3, i64::MAX))?,
                    state,
                    wave_state,
                    action,
                })
            })
            .map_err(|error| format!("cannot read recoverable fleet targets: {error}"))?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|error| format!("invalid recoverable fleet target: {error}"))?);
        }
        Ok(items)
    }
}

#[derive(Clone, Debug)]
struct StoredTarget {
    target_token: String,
    wave_index: usize,
    generation: u64,
    baseline_policy_sha256: String,
    candidate_policy_sha256: String,
    state: String,
    apply_command_id: Option<String>,
    rollback_command_id: Option<String>,
    observed_policy_sha256: Option<String>,
}

fn connect(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path).map_err(|error| {
        format!(
            "cannot open fleet session journal {}: {error}",
            path.display()
        )
    })?;
    connection
        .busy_timeout(BUSY_TIMEOUT)
        .map_err(|error| format!("cannot set fleet journal busy timeout: {error}"))?;
    connection
        .execute_batch("PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;")
        .map_err(|error| format!("cannot configure fleet journal durability: {error}"))?;
    Ok(connection)
}

fn table_exists(connection: &Connection, name: &str) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1);",
            params![name],
            |row| row.get(0),
        )
        .map_err(|error| format!("cannot inspect fleet journal schema: {error}"))
}

fn schema_version(connection: &Connection) -> Result<i64, String> {
    connection
        .query_row("PRAGMA user_version;", [], |row| row.get(0))
        .map_err(|error| format!("cannot read fleet journal schema version: {error}"))
}

fn verify_schema(connection: &Connection) -> Result<(), String> {
    let version = schema_version(connection)?;
    if version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported fleet journal schema version {version}; expected {SCHEMA_VERSION}"
        ));
    }
    for (table, expected_columns) in [
        (
            "rollout_sessions",
            &[
                "plan_sha256",
                "state",
                "active_wave_index",
                "created_at_unix_ms",
                "updated_at_unix_ms",
            ][..],
        ),
        (
            "rollout_waves",
            &["plan_sha256", "wave_index", "state", "updated_at_unix_ms"][..],
        ),
        (
            "rollout_targets",
            &[
                "plan_sha256",
                "target_token",
                "wave_index",
                "failure_domain",
                "generation",
                "baseline_policy_sha256",
                "candidate_policy_sha256",
                "canary",
                "state",
                "apply_command_id",
                "rollback_command_id",
                "observed_policy_sha256",
                "updated_at_unix_ms",
            ][..],
        ),
    ] {
        if !table_exists(connection, table)? {
            return Err(format!("fleet journal schema is missing table {table:?}"));
        }
        let query = format!("PRAGMA table_info({table});");
        let mut statement = connection
            .prepare(&query)
            .map_err(|error| format!("cannot inspect fleet journal table {table:?}: {error}"))?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|error| format!("cannot query fleet journal table {table:?}: {error}"))?;
        let mut actual = Vec::new();
        for row in rows {
            actual.push(row.map_err(|error| format!("cannot read fleet journal column: {error}"))?);
        }
        if actual.iter().map(String::as_str).collect::<Vec<_>>() != expected_columns {
            return Err(format!(
                "fleet journal table {table:?} columns do not match schema version {SCHEMA_VERSION}"
            ));
        }
    }
    Ok(())
}

fn get_session_conn(
    connection: &Connection,
    plan_sha256: &str,
) -> Result<Option<FleetSessionSnapshot>, String> {
    connection
        .query_row(
            "SELECT plan_sha256, state, active_wave_index, created_at_unix_ms, updated_at_unix_ms
             FROM rollout_sessions WHERE plan_sha256 = ?1;",
            params![plan_sha256],
            session_from_row,
        )
        .optional()
        .map_err(|error| format!("cannot read fleet session: {error}"))
}

fn session_from_row(row: &Row<'_>) -> rusqlite::Result<FleetSessionSnapshot> {
    let state: String = row.get(1)?;
    let active: Option<i64> = row.get(2)?;
    let created: i64 = row.get(3)?;
    let updated: i64 = row.get(4)?;
    let state = FleetSessionState::parse(&state).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
        )
    })?;
    let active_wave_index = active
        .map(|value| {
            usize::try_from(value).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Integer,
                    Box::new(error),
                )
            })
        })
        .transpose()?;
    let created_at_unix_ms = u64::try_from(created).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })?;
    let updated_at_unix_ms = u64::try_from(updated).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })?;
    Ok(FleetSessionSnapshot {
        plan_sha256: row.get(0)?,
        state,
        active_wave_index,
        created_at_unix_ms,
        updated_at_unix_ms,
    })
}

fn get_wave_state(
    connection: &Connection,
    plan_sha256: &str,
    wave_index: usize,
) -> Result<String, String> {
    connection
        .query_row(
            "SELECT state FROM rollout_waves WHERE plan_sha256 = ?1 AND wave_index = ?2;",
            params![
                plan_sha256,
                to_sql_integer(wave_index as u64, "wave index")?
            ],
            |row| row.get(0),
        )
        .map_err(|error| format!("cannot read rollout wave {wave_index}: {error}"))
}

fn require_active_wave(
    connection: &Connection,
    plan_sha256: &str,
    wave_index: usize,
) -> Result<(), String> {
    let session = get_session_conn(connection, plan_sha256)?
        .ok_or_else(|| "unknown fleet plan digest".to_owned())?;
    if session.state != FleetSessionState::Running || session.active_wave_index != Some(wave_index)
    {
        return Err(format!(
            "wave {wave_index} is not the active wave for session {:?}",
            session.state
        ));
    }
    Ok(())
}

fn get_target_conn(
    connection: &Connection,
    plan_sha256: &str,
    target_token: &str,
) -> Result<Option<StoredTarget>, String> {
    connection
        .query_row(
            "SELECT target_token, wave_index, generation, baseline_policy_sha256,
                    candidate_policy_sha256, state, apply_command_id, rollback_command_id,
                    observed_policy_sha256
             FROM rollout_targets WHERE plan_sha256 = ?1 AND target_token = ?2;",
            params![plan_sha256, target_token],
            |row| {
                let wave_index: i64 = row.get(1)?;
                let generation: i64 = row.get(2)?;
                Ok(StoredTarget {
                    target_token: row.get(0)?,
                    wave_index: u64_to_usize(wave_index)
                        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(1, wave_index))?,
                    generation: u64::try_from(generation)
                        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(2, generation))?,
                    baseline_policy_sha256: row.get(3)?,
                    candidate_policy_sha256: row.get(4)?,
                    state: row.get(5)?,
                    apply_command_id: row.get(6)?,
                    rollback_command_id: row.get(7)?,
                    observed_policy_sha256: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(|error| format!("cannot read fleet target {target_token:?}: {error}"))
}

fn get_wave_conn(
    connection: &Connection,
    plan_sha256: &str,
    wave_index: usize,
) -> Result<RolloutWave, String> {
    let wave_is_canary = wave_index == 0;
    let mut statement = connection
        .prepare(
            "SELECT target_token, failure_domain, generation, baseline_policy_sha256,
                    candidate_policy_sha256, canary
             FROM rollout_targets WHERE plan_sha256 = ?1 AND wave_index = ?2
             ORDER BY failure_domain, target_token;",
        )
        .map_err(|error| format!("cannot prepare wave target query: {error}"))?;
    let rows = statement
        .query_map(
            params![
                plan_sha256,
                to_sql_integer(wave_index as u64, "wave index")?
            ],
            |row| {
                let generation: i64 = row.get(2)?;
                Ok(FleetTarget {
                    target_token: row.get(0)?,
                    failure_domain: row.get(1)?,
                    generation: u64::try_from(generation)
                        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(2, generation))?,
                    baseline_policy_sha256: row.get(3)?,
                    candidate_policy_sha256: row.get(4)?,
                    canary: row.get::<_, i64>(5)? == 1,
                })
            },
        )
        .map_err(|error| format!("cannot query wave targets: {error}"))?;
    let mut targets = Vec::new();
    for row in rows {
        targets.push(row.map_err(|error| format!("invalid wave target row: {error}"))?);
    }
    if targets.is_empty() {
        return Err(format!("wave {wave_index} contains no registered targets"));
    }
    Ok(RolloutWave {
        index: wave_index,
        is_canary: wave_is_canary,
        targets,
    })
}

fn get_wave_targets_conn(
    connection: &Connection,
    plan_sha256: &str,
    wave_index: usize,
) -> Result<Vec<StoredTarget>, String> {
    let mut statement = connection
        .prepare(
            "SELECT target_token, wave_index, generation, baseline_policy_sha256,
                    candidate_policy_sha256, state, apply_command_id, rollback_command_id,
                    observed_policy_sha256
             FROM rollout_targets WHERE plan_sha256 = ?1 AND wave_index = ?2
             ORDER BY failure_domain, target_token;",
        )
        .map_err(|error| format!("cannot prepare wave target listing: {error}"))?;
    let rows = statement
        .query_map(
            params![
                plan_sha256,
                to_sql_integer(wave_index as u64, "wave index")?
            ],
            |row| {
                let wave_index_raw: i64 = row.get(1)?;
                let generation: i64 = row.get(2)?;
                Ok(StoredTarget {
                    target_token: row.get(0)?,
                    wave_index: u64_to_usize(wave_index_raw)
                        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(1, wave_index_raw))?,
                    generation: u64::try_from(generation)
                        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(2, generation))?,
                    baseline_policy_sha256: row.get(3)?,
                    candidate_policy_sha256: row.get(4)?,
                    state: row.get(5)?,
                    apply_command_id: row.get(6)?,
                    rollback_command_id: row.get(7)?,
                    observed_policy_sha256: row.get(8)?,
                })
            },
        )
        .map_err(|error| format!("cannot read wave target listing: {error}"))?;
    let mut targets = Vec::new();
    for row in rows {
        targets.push(row.map_err(|error| format!("invalid wave target row: {error}"))?);
    }
    Ok(targets)
}

fn validate_plan(plan: &FleetRolloutPlan) -> Result<(), String> {
    if plan.schema_version != "1.0.0" {
        return Err(format!(
            "unsupported fleet plan schema version {:?}",
            plan.schema_version
        ));
    }
    if digest_plan(&plan.waves, plan.constraints) != plan.plan_sha256 {
        return Err(
            "fleet plan SHA-256 does not match its canonical content and constraints".to_owned(),
        );
    }
    let targets: Vec<FleetTarget> = plan
        .waves
        .iter()
        .flat_map(|wave| wave.targets.iter().cloned())
        .collect();
    let canonical = plan_rollout(&targets, plan.constraints)?;
    if &canonical != plan {
        return Err(
            "fleet plan is validly shaped but not the canonical deterministic schedule".to_owned(),
        );
    }
    for target in &targets {
        validate_target(target)?;
    }
    Ok(())
}

fn command_identity(
    plan_sha256: &str,
    wave_index: usize,
    target_token: &str,
    generation: u64,
    action: &str,
) -> String {
    super::sha256_hex(
        format!(
            "luminous-edge-fleet-command-v1|{plan_sha256}|{wave_index}|{target_token}|{generation}|{action}"
        )
        .as_bytes(),
    )
}

fn validate_digest(value: &str, label: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{label} must be 64 lowercase hexadecimal characters"
        ));
    }
    Ok(())
}

fn to_sql_integer(value: u64, label: &str) -> Result<i64, String> {
    i64::try_from(value)
        .map_err(|error| format!("{label} exceeds the SQLite integer range: {error}"))
}

fn u64_to_usize(value: i64) -> Result<usize, ()> {
    usize::try_from(value).map_err(|_| ())
}

fn ensure_private_file(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err("fleet journal path must not be a symbolic link".to_owned());
            }
            if !metadata.is_file() {
                return Err("fleet journal path must be a regular file".to_owned());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o077 != 0 {
                    return Err(
                        "fleet journal database must not grant group or other permissions"
                            .to_owned(),
                    );
                }
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(path) {
                Ok(file) => {
                    drop(file);
                    Ok(())
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    ensure_private_file(path)
                }
                Err(error) => Err(format!(
                    "cannot create fleet journal {}: {error}",
                    path.display()
                )),
            }
        }
        Err(error) => Err(format!(
            "cannot inspect fleet journal path {}: {error}",
            path.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{ApplyCommand, FleetRecoveryAction, FleetSessionJournal, FleetSessionState};
    use crate::fleet_rollout::{
        FleetTarget, FleetTargetObservation, ProbeOutcome, RolloutConstraints, WaveDecision,
        plan_rollout,
    };
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    const BASELINE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const CANDIDATE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "luminous-edge-fleet-session-{}-{nanos}-{id}.sqlite",
            std::process::id()
        ))
    }

    fn target(token: &str, domain: &str, canary: bool) -> FleetTarget {
        FleetTarget {
            target_token: token.to_owned(),
            failure_domain: domain.to_owned(),
            generation: 12,
            baseline_policy_sha256: BASELINE.to_owned(),
            candidate_policy_sha256: CANDIDATE.to_owned(),
            canary,
        }
    }

    fn plan() -> crate::fleet_rollout::FleetRolloutPlan {
        plan_rollout(
            &[
                target("target-canary", "site-a", true),
                target("target-a", "site-a", false),
                target("target-b", "site-b", false),
            ],
            RolloutConstraints {
                max_batch_size: 2,
                max_targets_per_failure_domain: 1,
            },
        )
        .expect("plan")
    }

    fn remove_database(path: &PathBuf) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }

    fn observations(wave: &crate::fleet_rollout::RolloutWave) -> Vec<FleetTargetObservation> {
        wave.targets
            .iter()
            .map(|target| FleetTargetObservation {
                target_token: target.target_token.clone(),
                policy_generation: target.generation,
                effective_policy_sha256: target.candidate_policy_sha256.clone(),
                freshness_valid: true,
                nonce_durably_consumed: true,
                independent_probe: true,
                positive_control_passed: true,
                outcome: ProbeOutcome::Passed,
            })
            .collect()
    }

    fn apply_wave(
        journal: &FleetSessionJournal,
        plan: &crate::fleet_rollout::FleetRolloutPlan,
        index: usize,
    ) {
        journal
            .begin_wave(&plan.plan_sha256, index, 100 + index as u64)
            .expect("begin wave");
        for target in &plan.waves[index].targets {
            journal
                .record_apply_intent(
                    &plan.plan_sha256,
                    index,
                    &target.target_token,
                    110 + index as u64,
                )
                .expect("intent");
            journal
                .record_applied(
                    &plan.plan_sha256,
                    index,
                    &target.target_token,
                    CANDIDATE,
                    120 + index as u64,
                )
                .expect("applied");
        }
    }

    fn verify_and_promote_wave(
        journal: &FleetSessionJournal,
        plan: &crate::fleet_rollout::FleetRolloutPlan,
        index: usize,
    ) {
        let wave_observations = observations(&plan.waves[index]);
        let decision = journal
            .evaluate_and_record_wave(
                &plan.plan_sha256,
                index,
                &wave_observations,
                130 + index as u64,
            )
            .expect("evaluate wave");
        assert_eq!(decision.decision, WaveDecision::Continue);
        assert_eq!(
            journal
                .evaluate_and_record_wave(
                    &plan.plan_sha256,
                    index,
                    &wave_observations,
                    131 + index as u64,
                )
                .expect("idempotent evaluate")
                .decision,
            WaveDecision::Continue
        );
        for target in &plan.waves[index].targets {
            journal
                .record_target_promoted(
                    &plan.plan_sha256,
                    index,
                    &target.target_token,
                    CANDIDATE,
                    140 + index as u64,
                )
                .expect("record target promoted");
            journal
                .record_target_promoted(
                    &plan.plan_sha256,
                    index,
                    &target.target_token,
                    CANDIDATE,
                    141 + index as u64,
                )
                .expect("idempotent promotion record");
        }
        let completed = journal
            .complete_wave(&plan.plan_sha256, index, 150 + index as u64)
            .expect("complete wave");
        assert_eq!(
            journal
                .complete_wave(&plan.plan_sha256, index, 151 + index as u64)
                .expect("idempotent completion"),
            completed
        );
    }

    #[test]
    fn canonical_plan_registration_is_idempotent() {
        let path = path();
        let journal = FleetSessionJournal::open(&path).expect("open");
        let plan = plan();
        let first = journal.register_plan(&plan, 100).expect("register");
        let second = journal.register_plan(&plan, 101).expect("register again");
        assert_eq!(first, second);
        assert_eq!(first.state, FleetSessionState::Planned);
        drop(journal);
        remove_database(&path);
    }

    #[test]
    fn apply_intent_is_durable_and_retries_return_the_same_command_id() {
        let path = path();
        let plan = plan();
        let command: ApplyCommand;
        {
            let journal = FleetSessionJournal::open(&path).expect("open");
            journal.register_plan(&plan, 100).expect("register");
            journal
                .begin_wave(&plan.plan_sha256, 0, 101)
                .expect("begin");
            command = journal
                .record_apply_intent(&plan.plan_sha256, 0, "target-canary", 102)
                .expect("intent");
            let retry = journal
                .record_apply_intent(&plan.plan_sha256, 0, "target-canary", 103)
                .expect("idempotent retry");
            assert_eq!(command, retry);
        }
        let recovered = FleetSessionJournal::open(&path).expect("reopen");
        let items = recovered.list_recoverable_targets().expect("recovery");
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].action,
            FleetRecoveryAction::InspectActualPolicyBeforeRetry
        );
        drop(recovered);
        remove_database(&path);
    }

    #[test]
    fn child_exits_after_committing_apply_intent() {
        let Ok(database_path) = std::env::var("LUMINOUS_FLEET_SESSION_CRASH_TEST_DB") else {
            return;
        };
        let journal = FleetSessionJournal::open(&database_path).expect("child opens journal");
        let plan = plan();
        journal
            .register_plan(&plan, 100)
            .expect("child registers plan");
        journal
            .begin_wave(&plan.plan_sha256, 0, 101)
            .expect("child begins canary");
        journal
            .record_apply_intent(&plan.plan_sha256, 0, "target-canary", 102)
            .expect("child commits target apply intent");
        // Simulate abrupt controller termination after the durable intent commit.
        std::process::exit(77);
    }

    #[test]
    fn apply_intent_survives_abrupt_child_process_exit() {
        let database_path = path();
        let executable = std::env::current_exe().expect("current test executable");
        let status = Command::new(executable)
            .arg("child_exits_after_committing_apply_intent")
            .env(
                "LUMINOUS_FLEET_SESSION_CRASH_TEST_DB",
                database_path.to_string_lossy().as_ref(),
            )
            .status()
            .expect("launch crash-test child");
        assert_eq!(status.code(), Some(77));

        let recovered = FleetSessionJournal::open(&database_path).expect("reopen after crash");
        let targets = recovered
            .list_recoverable_targets()
            .expect("recover outstanding target");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].target_token, "target-canary");
        assert_eq!(
            targets[0].action,
            FleetRecoveryAction::InspectActualPolicyBeforeRetry
        );
        drop(recovered);
        remove_database(&database_path);
    }

    #[test]
    fn next_wave_cannot_start_until_every_target_in_prior_wave_is_promoted() {
        let path = path();
        let journal = FleetSessionJournal::open(&path).expect("open");
        let plan = plan();
        journal.register_plan(&plan, 100).expect("register");
        assert!(
            journal
                .begin_wave(&plan.plan_sha256, 1, 101)
                .expect_err("cannot skip canary")
                .contains("cannot begin until prior wave")
        );
        apply_wave(&journal, &plan, 0);
        let eval = journal
            .evaluate_and_record_wave(&plan.plan_sha256, 0, &observations(&plan.waves[0]), 130)
            .expect("evaluate");
        assert_eq!(eval.decision, WaveDecision::Continue);
        assert!(
            journal
                .complete_wave(&plan.plan_sha256, 0, 131)
                .expect_err("must promote each target")
                .contains("not durably promoted")
        );
        journal
            .record_target_promoted(&plan.plan_sha256, 0, "target-canary", CANDIDATE, 132)
            .expect("promote");
        journal
            .complete_wave(&plan.plan_sha256, 0, 133)
            .expect("complete");
        journal
            .begin_wave(&plan.plan_sha256, 1, 134)
            .expect("next wave");
        drop(journal);
        remove_database(&path);
    }

    #[test]
    fn definite_failure_requires_wave_rollback_and_exact_baseline_readback() {
        let path = path();
        let journal = FleetSessionJournal::open(&path).expect("open");
        let plan = plan();
        journal.register_plan(&plan, 100).expect("register");
        apply_wave(&journal, &plan, 0);
        let mut probe = observations(&plan.waves[0]);
        probe[0].outcome = ProbeOutcome::Failed;
        let evaluation = journal
            .evaluate_and_record_wave(&plan.plan_sha256, 0, &probe, 130)
            .expect("evaluate");
        assert_eq!(evaluation.decision, WaveDecision::RollbackWave);
        let recovery = journal
            .list_recoverable_targets()
            .expect("rollback recovery");
        assert_eq!(recovery.len(), 1);
        assert_eq!(
            recovery[0].action,
            FleetRecoveryAction::PersistRollbackIntentAndEnforceBaseline
        );
        let commands = journal
            .request_wave_rollback(&plan.plan_sha256, 0, 131)
            .expect("persist rollback intent");
        assert_eq!(commands.len(), 1);
        assert_eq!(
            journal
                .request_wave_rollback(&plan.plan_sha256, 0, 132)
                .expect("same rollback commands"),
            commands
        );
        assert!(
            journal
                .record_target_rolled_back(&plan.plan_sha256, 0, "target-canary", CANDIDATE, 133)
                .expect_err("candidate is not the baseline")
                .contains("exact baseline")
        );
        assert!(
            journal
                .record_target_rolled_back(&plan.plan_sha256, 0, "target-canary", BASELINE, 134)
                .expect("baseline readback")
        );
        assert_eq!(
            journal
                .get_session(&plan.plan_sha256)
                .expect("read")
                .expect("session")
                .state,
            FleetSessionState::Halted
        );
        drop(journal);
        remove_database(&path);
    }

    #[test]
    fn partial_rollback_retries_return_the_same_command_set() {
        let path = path();
        let journal = FleetSessionJournal::open(&path).expect("open");
        let plan = plan();
        journal.register_plan(&plan, 100).expect("register");
        apply_wave(&journal, &plan, 0);
        verify_and_promote_wave(&journal, &plan, 0);
        apply_wave(&journal, &plan, 1);

        let mut probe = observations(&plan.waves[1]);
        probe[0].outcome = ProbeOutcome::Failed;
        assert_eq!(
            journal
                .evaluate_and_record_wave(&plan.plan_sha256, 1, &probe, 130)
                .expect("evaluate")
                .decision,
            WaveDecision::RollbackWave
        );
        let commands = journal
            .request_wave_rollback(&plan.plan_sha256, 1, 131)
            .expect("request rollback");
        assert_eq!(commands.len(), 2);

        assert!(
            !journal
                .record_target_rolled_back(
                    &plan.plan_sha256,
                    1,
                    &commands[0].target_token,
                    BASELINE,
                    132,
                )
                .expect("first target rolled back")
        );
        assert_eq!(
            journal
                .request_wave_rollback(&plan.plan_sha256, 1, 133)
                .expect("same complete command set after partial rollback"),
            commands
        );
        assert!(
            !journal
                .record_target_rolled_back(
                    &plan.plan_sha256,
                    1,
                    &commands[0].target_token,
                    BASELINE,
                    134,
                )
                .expect("duplicate acknowledgement is idempotent")
        );
        assert!(
            journal
                .record_target_rolled_back(
                    &plan.plan_sha256,
                    1,
                    &commands[1].target_token,
                    BASELINE,
                    135,
                )
                .expect("final baseline restored")
        );
        assert_eq!(
            journal
                .request_wave_rollback(&plan.plan_sha256, 1, 136)
                .expect("same terminal command set"),
            commands
        );
        assert!(
            journal
                .record_target_rolled_back(
                    &plan.plan_sha256,
                    1,
                    &commands[1].target_token,
                    BASELINE,
                    137,
                )
                .expect("duplicate terminal acknowledgement")
        );
        assert!(
            journal
                .list_recoverable_targets()
                .expect("none pending")
                .is_empty()
        );
        drop(journal);
        remove_database(&path);
    }

    #[test]
    fn invalid_or_noncanonical_plan_is_rejected_before_registration() {
        let path = path();
        let journal = FleetSessionJournal::open(&path).expect("open");
        let mut invalid = plan();
        invalid.plan_sha256 = "e".repeat(64);
        assert!(
            journal
                .register_plan(&invalid, 100)
                .expect_err("bad digest")
                .contains("does not match")
        );
        drop(journal);
        remove_database(&path);
    }

    #[test]
    fn altered_session_schema_is_rejected_on_reopen() {
        let database_path = path();
        {
            let journal = FleetSessionJournal::open(&database_path).expect("initialize schema");
            drop(journal);
        }
        {
            let connection =
                rusqlite::Connection::open(&database_path).expect("open for schema mutation");
            connection
                .execute_batch(
                    "ALTER TABLE rollout_targets
                     RENAME COLUMN candidate_policy_sha256 TO candidate_digest_unexpected;",
                )
                .expect("alter schema fixture");
        }
        assert!(
            FleetSessionJournal::open(&database_path)
                .expect_err("altered schema must fail closed")
                .contains("columns do not match")
        );
        remove_database(&database_path);
    }

    #[test]
    fn incomplete_or_invalid_evidence_halts_instead_of_advancing() {
        let path = path();
        let journal = FleetSessionJournal::open(&path).expect("open");
        let plan = plan();
        journal.register_plan(&plan, 100).expect("register");
        apply_wave(&journal, &plan, 0);
        let result = journal
            .evaluate_and_record_wave(&plan.plan_sha256, 0, &[], 130)
            .expect("evaluation returns an explicit halt");
        assert_eq!(result.decision, WaveDecision::HaltAndReconcile);
        assert_eq!(
            journal
                .get_session(&plan.plan_sha256)
                .expect("read")
                .expect("session")
                .state,
            FleetSessionState::Halted
        );
        let recovery = journal
            .list_recoverable_targets()
            .expect("halted state recovery");
        assert_eq!(recovery.len(), 1);
        assert_eq!(recovery[0].action, FleetRecoveryAction::HaltAndReconcile);
        assert!(journal.begin_wave(&plan.plan_sha256, 1, 131).is_err());
        drop(journal);
        remove_database(&path);
    }
}
