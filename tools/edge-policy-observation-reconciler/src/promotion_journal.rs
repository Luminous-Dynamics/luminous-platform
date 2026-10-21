//! Durable, fail-closed policy-promotion journal.
//!
//! This journal records intent before external actuation and never equates a
//! controller acknowledgement with an observed policy. It is a recovery
//! protocol, not an authenticated control channel or an atomic transaction
//! with an external firewall/AP.

#![forbid(unsafe_code)]

use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use sha2::{Digest, Sha256};

const JOURNAL_SCHEMA_VERSION: i64 = 1;
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

const CREATE_TABLE: &str = "
CREATE TABLE promotion_operations (
    operation_id TEXT PRIMARY KEY NOT NULL,
    target_token TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK (generation > 0),
    baseline_policy_sha256 TEXT NOT NULL CHECK (length(baseline_policy_sha256) = 64),
    candidate_policy_sha256 TEXT NOT NULL CHECK (length(candidate_policy_sha256) = 64),
    state TEXT NOT NULL CHECK (state IN
        ('prepared', 'apply_intent', 'applied', 'verified', 'promoted',
         'rollback_intent', 'rolled_back', 'abandoned')),
    created_at_unix_ms INTEGER NOT NULL CHECK (created_at_unix_ms >= 0),
    updated_at_unix_ms INTEGER NOT NULL CHECK (updated_at_unix_ms >= 0),
    observed_policy_sha256 TEXT,
    challenge_id TEXT,
    receipt_sha256 TEXT,
    evidence_observed_at_unix_ms INTEGER,
    evidence_expires_at_unix_ms INTEGER,
    verification_session_id TEXT,
    UNIQUE (target_token, generation)
);
";

/// Immutable identity and expected policy digests for one change attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionSpec<'a> {
    /// Stable idempotency key for this change attempt.
    pub operation_id: &'a str,
    /// Stable target identifier; this must not be a secret.
    pub target_token: &'a str,
    /// Monotonically increasing policy generation for this target.
    pub generation: u64,
    /// Exact approved effective-policy digest before the change.
    pub baseline_policy_sha256: &'a str,
    /// Exact effective-policy digest expected after applying the candidate.
    pub candidate_policy_sha256: &'a str,
    /// Verifier/control-plane clock, milliseconds since Unix epoch.
    pub created_at_unix_ms: u64,
}

/// Evidence that the independent probe succeeded against the candidate policy.
///
/// Call this only after challenge freshness, conformance, and durable nonce
/// consumption have succeeded. The journal validates binding and expiry; it
/// does not itself authenticate the evidence producer or verify the receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
struct PromotionEvidence<'a> {
    challenge_id: &'a str,
    target_token: &'a str,
    policy_generation: u64,
    receipt_sha256: &'a str,
    effective_policy_sha256: &'a str,
    observed_at_unix_ms: u64,
    /// Expiration from the already-validated freshness envelope.
    expires_at_unix_ms: u64,
}

/// State persisted for a single policy change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionOperation {
    pub operation_id: String,
    pub target_token: String,
    pub generation: u64,
    pub baseline_policy_sha256: String,
    pub candidate_policy_sha256: String,
    pub state: PromotionState,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    pub observed_policy_sha256: Option<String>,
    pub challenge_id: Option<String>,
    pub receipt_sha256: Option<String>,
    pub evidence_observed_at_unix_ms: Option<u64>,
    pub evidence_expires_at_unix_ms: Option<u64>,
}

/// Persisted transition states. Promoted, RolledBack, and Abandoned are terminal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PromotionState {
    Prepared,
    ApplyIntent,
    Applied,
    Verified,
    Promoted,
    RollbackIntent,
    RolledBack,
    Abandoned,
}

impl PromotionState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::ApplyIntent => "apply_intent",
            Self::Applied => "applied",
            Self::Verified => "verified",
            Self::Promoted => "promoted",
            Self::RollbackIntent => "rollback_intent",
            Self::RolledBack => "rolled_back",
            Self::Abandoned => "abandoned",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "prepared" => Ok(Self::Prepared),
            "apply_intent" => Ok(Self::ApplyIntent),
            "applied" => Ok(Self::Applied),
            "verified" => Ok(Self::Verified),
            "promoted" => Ok(Self::Promoted),
            "rollback_intent" => Ok(Self::RollbackIntent),
            "rolled_back" => Ok(Self::RolledBack),
            "abandoned" => Ok(Self::Abandoned),
            _ => Err(format!(
                "unknown promotion state {value:?}; refusing to recover"
            )),
        }
    }

    fn is_terminal(self) -> bool {
        matches!(self, Self::Promoted | Self::RolledBack | Self::Abandoned)
    }
}

/// Suggested recovery action after opening the journal and invalidating any
/// verification that existed only in a previous process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryAction {
    /// No external effect should have been requested yet.
    ApplyOrAbandonPreparedOperation,
    /// The process may have died before, during, or after device actuation.
    /// Read the actual effective policy before deciding to retry or roll back.
    InspectEffectivePolicyBeforeRetry,
    /// The candidate was read back, but a new independent challenge/probe is
    /// required before promotion.
    RequireFreshChallengeAndIndependentProbe,
    /// A rollback was requested; inspect the device and enforce the baseline.
    InspectAndEnforceBaselinePolicy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverableOperation {
    pub operation: PromotionOperation,
    pub action: RecoveryAction,
}

/// Process-local capability produced only after a successful evidence check.
///
/// It is deliberately neither Clone nor serializable. A process restart creates
/// a new journal session and invalidates persisted Verified states.
#[derive(Debug)]
pub struct PromotionPermit {
    operation_id: String,
    session_id: String,
    expires_at_unix_ms: u64,
}

/// Read-only binding produced only when a live, process-local promotion permit
/// still matches the durable journal's Verified state. It is intentionally not
/// Clone, serializable, or publicly constructible. Callers must still account for
/// the time-of-check/time-of-use window before external device actuation.
#[derive(Debug)]
pub struct VerifiedPromotionBinding {
    operation_id: String,
    target_token: String,
    generation: u64,
    baseline_policy_sha256: String,
    candidate_policy_sha256: String,
    challenge_id: String,
    receipt_sha256: String,
    observed_at_unix_ms: u64,
    expires_at_unix_ms: u64,
}

impl VerifiedPromotionBinding {
    pub fn operation_id(&self) -> &str { &self.operation_id }
    pub fn target_token(&self) -> &str { &self.target_token }
    pub fn generation(&self) -> u64 { self.generation }
    pub fn baseline_policy_sha256(&self) -> &str { &self.baseline_policy_sha256 }
    pub fn candidate_policy_sha256(&self) -> &str { &self.candidate_policy_sha256 }
    pub fn challenge_id(&self) -> &str { &self.challenge_id }
    pub fn receipt_sha256(&self) -> &str { &self.receipt_sha256 }
    pub fn observed_at_unix_ms(&self) -> u64 { self.observed_at_unix_ms }
    pub fn expires_at_unix_ms(&self) -> u64 { self.expires_at_unix_ms }
}

/// Inputs for the public challenge-verification and promotion-evidence gate.
///
/// The strings are the exact bytes to be schema-validated and digest-bound.
#[derive(Clone, Copy, Debug)]
pub struct PromotionVerificationInput<'a> {
    pub challenge_source: &'a str,
    pub envelope_source: &'a str,
    pub corpus_schema_source: &'a str,
    pub corpus_source: &'a str,
    pub receipt_source: &'a str,
    /// Digest obtained from the live effective-policy readback associated with
    /// the independent data-plane probe.
    pub effective_policy_sha256: &'a str,
    pub verifier_now_unix_ms: u64,
}

/// SQLite-backed promotion journal. Use a dedicated, permission-restricted
/// database on durable storage; do not share the replay-ledger database path.
#[derive(Debug)]
pub struct PromotionJournal {
    path: PathBuf,
    session_id: String,
}

impl PromotionJournal {
    /// Open a dedicated durable journal. A verified-but-not-promoted operation
    /// from an earlier process is reset to Applied, forcing fresh evidence.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if path.as_os_str().is_empty() || path.to_string_lossy() == ":memory:" {
            return Err("promotion journal requires a durable file path".to_owned());
        }
        ensure_private_database_file(&path)?;
        let mut connection = connect(&path)?;
        let has_table = table_exists(&connection)?;
        let user_version = user_version(&connection)?;
        if !has_table {
            let existing_tables: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master
                     WHERE type = 'table' AND name NOT LIKE 'sqlite_%';",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| format!("cannot inspect promotion journal database: {error}"))?;
            if user_version != 0 || existing_tables != 0 {
                return Err("refusing to initialize a non-empty or versioned database as a promotion journal".to_owned());
            }
            connection
                .execute_batch(&format!(
                    "PRAGMA journal_mode = WAL;\n{CREATE_TABLE}\nPRAGMA user_version = {JOURNAL_SCHEMA_VERSION};"
                ))
                .map_err(|error| format!("cannot initialize promotion journal schema: {error}"))?;
        } else {
            connection
                .execute_batch("PRAGMA journal_mode = WAL;")
                .map_err(|error| format!("cannot enable promotion journal WAL mode: {error}"))?;
        }
        verify_schema(&connection)?;

        // A persisted Verified row cannot authorize promotion after a restart.
        // Leave the operation in Applied so recovery requires a fresh challenge.
        let now = unix_now_ms()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin promotion recovery transaction: {error}"))?;
        transaction
            .execute(
                "UPDATE promotion_operations
                 SET state = 'applied', updated_at_unix_ms = ?1,
                     challenge_id = NULL, receipt_sha256 = NULL,
                     evidence_observed_at_unix_ms = NULL,
                     evidence_expires_at_unix_ms = NULL,
                     verification_session_id = NULL
                 WHERE state = 'verified';",
                params![to_sql_integer(now, "recovery timestamp")?],
            )
            .map_err(|error| format!("cannot invalidate stale promotion evidence: {error}"))?;
        transaction
            .commit()
            .map_err(|error| format!("cannot commit promotion recovery: {error}"))?;

        Ok(Self {
            path,
            session_id: fresh_session_id()?,
        })
    }

    /// Prepare an operation durably before issuing an external apply command.
    /// Repeating the exact same operation ID and immutable specification is
    /// idempotent. Different data for the same ID is rejected.
    pub fn prepare(&self, spec: &PromotionSpec<'_>) -> Result<PromotionOperation, String> {
        validate_spec(spec)?;
        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin prepare transaction: {error}"))?;

        if let Some(existing) = get_operation_conn(&transaction, spec.operation_id)? {
            let identical = existing.target_token == spec.target_token
                && existing.generation == spec.generation
                && existing.baseline_policy_sha256 == spec.baseline_policy_sha256
                && existing.candidate_policy_sha256 == spec.candidate_policy_sha256
                && existing.created_at_unix_ms == spec.created_at_unix_ms;
            if !identical {
                return Err(
                    "operation ID already exists with different immutable inputs".to_owned(),
                );
            }
            transaction
                .commit()
                .map_err(|error| format!("cannot commit idempotent prepare: {error}"))?;
            return Ok(existing);
        }

        let active: Option<String> = transaction
            .query_row(
                "SELECT operation_id FROM promotion_operations
                 WHERE target_token = ?1 AND state NOT IN ('promoted', 'rolled_back', 'abandoned')
                 LIMIT 1;",
                params![spec.target_token],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("cannot inspect active target operation: {error}"))?;
        if let Some(active_id) = active {
            return Err(format!(
                "target already has non-terminal operation {active_id:?}; recover it first"
            ));
        }

        let max_generation: Option<i64> = transaction
            .query_row(
                "SELECT MAX(generation) FROM promotion_operations WHERE target_token = ?1;",
                params![spec.target_token],
                |row| row.get(0),
            )
            .map_err(|error| format!("cannot inspect target generation: {error}"))?;
        if let Some(maximum) = max_generation
            && to_sql_integer(spec.generation, "policy generation")? <= maximum
        {
            return Err(format!(
                "generation {} is not greater than the highest previously used generation {maximum}",
                spec.generation
            ));
        }

        transaction
            .execute(
                "INSERT INTO promotion_operations
                 (operation_id, target_token, generation, baseline_policy_sha256,
                  candidate_policy_sha256, state, created_at_unix_ms, updated_at_unix_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'prepared', ?6, ?6);",
                params![
                    spec.operation_id,
                    spec.target_token,
                    to_sql_integer(spec.generation, "policy generation")?,
                    spec.baseline_policy_sha256,
                    spec.candidate_policy_sha256,
                    to_sql_integer(spec.created_at_unix_ms, "created timestamp")?
                ],
            )
            .map_err(|error| format!("cannot persist prepared operation: {error}"))?;
        let operation = get_operation_conn(&transaction, spec.operation_id)?
            .ok_or_else(|| "prepared operation disappeared before commit".to_owned())?;
        transaction
            .commit()
            .map_err(|error| format!("cannot commit prepared operation: {error}"))?;
        Ok(operation)
    }

    /// Terminally abandon an operation that is still Prepared.
    ///
    /// Prepared is the only state in which this method is legal. Once ApplyIntent
    /// is durable, the device may have changed and recovery must inspect and
    /// reconcile the actual effective policy instead of pretending no actuation
    /// happened.
    pub fn abandon_prepared(
        &self,
        operation_id: &str,
        now_unix_ms: u64,
    ) -> Result<PromotionOperation, String> {
        let operation = self
            .get(operation_id)?
            .ok_or_else(|| format!("unknown operation {operation_id:?}"))?;
        if operation.state != PromotionState::Prepared {
            return Err(format!(
                "only Prepared operations may be abandoned; found {:?}",
                operation.state
            ));
        }
        let updated = self.update_state(operation_id, "prepared", "abandoned", now_unix_ms)?;
        Ok(updated)
    }

    /// Persist an apply intent before interacting with the external device.
    pub fn record_apply_intent(
        &self,
        operation_id: &str,
        now_unix_ms: u64,
    ) -> Result<PromotionOperation, String> {
        let operation = self
            .get(operation_id)?
            .ok_or_else(|| format!("unknown operation {operation_id:?}"))?;
        match operation.state {
            PromotionState::ApplyIntent => return Ok(operation),
            PromotionState::Prepared => {}
            state => return Err(format!("cannot record apply intent from state {state:?}")),
        }
        self.update_state(operation_id, "prepared", "apply_intent", now_unix_ms)
    }

    /// Record application only after a live readback exactly matches the
    /// candidate digest. Command acknowledgement alone is insufficient.
    pub fn record_applied(
        &self,
        operation_id: &str,
        effective_policy_sha256: &str,
        now_unix_ms: u64,
    ) -> Result<PromotionOperation, String> {
        validate_digest(effective_policy_sha256, "effective_policy_sha256")?;
        let operation = self
            .get(operation_id)?
            .ok_or_else(|| format!("unknown operation {operation_id:?}"))?;
        if effective_policy_sha256 != operation.candidate_policy_sha256 {
            return Err(
                "effective policy digest does not match the candidate; do not mark applied"
                    .to_owned(),
            );
        }
        match operation.state {
            PromotionState::Applied
                if operation.observed_policy_sha256.as_deref() == Some(effective_policy_sha256) =>
            {
                return Ok(operation);
            }
            PromotionState::ApplyIntent => {}
            state => return Err(format!("cannot record applied policy from state {state:?}")),
        }
        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin applied-state transaction: {error}"))?;
        let changed = transaction
            .execute(
                "UPDATE promotion_operations SET state = 'applied',
                    observed_policy_sha256 = ?1, updated_at_unix_ms = ?2
                 WHERE operation_id = ?3 AND state = 'apply_intent'
                   AND candidate_policy_sha256 = ?1;",
                params![
                    effective_policy_sha256,
                    to_sql_integer(now_unix_ms, "applied timestamp")?,
                    operation_id
                ],
            )
            .map_err(|error| format!("cannot persist applied-policy readback: {error}"))?;
        if changed != 1 {
            return Err(
                "promotion state changed concurrently; refusing applied transition".to_owned(),
            );
        }
        let updated = get_operation_conn(&transaction, operation_id)?
            .ok_or_else(|| "applied operation disappeared before commit".to_owned())?;
        transaction
            .commit()
            .map_err(|error| format!("cannot commit applied-policy state: {error}"))?;
        Ok(updated)
    }

    /// Verify receipt conformance and freshness, durably consume its nonce,
    /// bind that challenge to this operation, then mint a process-local permit.
    ///
    /// This is the public verification entry point. It performs no device
    /// actuation: the caller supplies a digest obtained from its live effective-
    /// policy readback, and promotion still requires a second readback.
    ///
    /// If the nonce commit succeeds but journal recording fails, the challenge
    /// remains consumed; retry with a newly issued challenge. This is the safe
    /// failure mode for two separate durable stores.
    pub fn verify_and_record_evidence(
        &self,
        ledger: &super::replay_ledger::SqliteReplayLedger,
        operation_id: &str,
        input: &PromotionVerificationInput<'_>,
    ) -> Result<(serde_json::Value, PromotionPermit), Vec<String>> {
        let challenge_source = input.challenge_source;
        let envelope_source = input.envelope_source;
        let corpus_schema_source = input.corpus_schema_source;
        let corpus_source = input.corpus_source;
        let receipt_source = input.receipt_source;
        let effective_policy_sha256 = input.effective_policy_sha256;
        let verifier_now_unix_ms = input.verifier_now_unix_ms;
        validate_digest(effective_policy_sha256, "effective_policy_sha256")
            .map_err(|error| vec![error])?;
        let operation = self
            .get(operation_id)
            .map_err(|error| vec![error])?
            .ok_or_else(|| vec![format!("unknown operation {operation_id:?}")])?;
        if operation.state != PromotionState::Applied {
            return Err(vec![format!(
                "fresh verification requires Applied state; found {:?}",
                operation.state
            )]);
        }
        if effective_policy_sha256 != operation.candidate_policy_sha256
            || operation.observed_policy_sha256.as_deref()
                != Some(operation.candidate_policy_sha256.as_str())
        {
            return Err(vec![
                "live effective policy readback does not match the applied candidate".to_owned(),
            ]);
        }

        let challenge: serde_json::Value = serde_json::from_str(challenge_source)
            .map_err(|error| vec![format!("cannot parse verification challenge: {error}")])?;
        let envelope: serde_json::Value = serde_json::from_str(envelope_source)
            .map_err(|error| vec![format!("cannot parse freshness envelope: {error}")])?;
        let challenge_id =
            json_string_field(&challenge, "/challenge_id", "challenge.challenge_id")?.to_owned();
        let target_token =
            json_string_field(&challenge, "/target_token", "challenge.target_token")?.to_owned();
        let policy_generation = json_integer_field(
            &challenge,
            "/policy_generation",
            "challenge.policy_generation",
        )?;
        let expected_effective_policy_sha256 = json_string_field(
            &challenge,
            "/expected_effective_policy_sha256",
            "challenge.expected_effective_policy_sha256",
        )?;
        let approved_policy_sha256 = json_string_field(
            &challenge,
            "/approved_policy_sha256",
            "challenge.approved_policy_sha256",
        )?;
        let expires_at_unix_ms = json_integer_field(
            &challenge,
            "/expires_at_unix_ms",
            "challenge.expires_at_unix_ms",
        )?;
        let observed_at_unix_ms = json_integer_field(
            &envelope,
            "/completed_at_unix_ms",
            "envelope.completed_at_unix_ms",
        )?;

        if challenge_id.is_empty()
            || target_token != operation.target_token
            || policy_generation != operation.generation
        {
            return Err(vec![
                "freshness challenge target or policy generation does not match the prepared operation"
                    .to_owned(),
            ]);
        }
        if approved_policy_sha256 != operation.baseline_policy_sha256
            || expected_effective_policy_sha256 != operation.candidate_policy_sha256
        {
            return Err(vec![
                "freshness challenge baseline/candidate digests do not match the prepared operation"
                    .to_owned(),
            ]);
        }

        // This validates challenge/envelope/receipt/corpus bindings and commits
        // nonce consumption before the journal is allowed to mint a permit.
        let report = super::replay_ledger::verify_and_consume_durable(
            ledger,
            challenge_source,
            envelope_source,
            corpus_schema_source,
            corpus_source,
            receipt_source,
            verifier_now_unix_ms,
        )?;
        let receipt_sha256 = sha256_hex(receipt_source.as_bytes());
        let evidence = PromotionEvidence {
            challenge_id: &challenge_id,
            target_token: &target_token,
            policy_generation,
            receipt_sha256: &receipt_sha256,
            effective_policy_sha256,
            observed_at_unix_ms,
            expires_at_unix_ms,
        };
        let permit = self
            .record_verified(operation_id, &evidence, verifier_now_unix_ms)
            .map_err(|error| vec![error])?;
        Ok((report, permit))
    }

    /// Internal post-verifier transition. External callers must use
    /// verify_and_record_evidence so durable challenge consumption cannot be
    /// accidentally skipped.
    fn record_verified(
        &self,
        operation_id: &str,
        evidence: &PromotionEvidence<'_>,
        verifier_now_unix_ms: u64,
    ) -> Result<PromotionPermit, String> {
        validate_identifier(evidence.challenge_id, "challenge_id")?;
        validate_target_token(evidence.target_token)?;
        validate_digest(evidence.receipt_sha256, "receipt_sha256")?;
        validate_digest(evidence.effective_policy_sha256, "effective_policy_sha256")?;
        let now = to_sql_integer(verifier_now_unix_ms, "verifier timestamp")?;
        let observed_at = to_sql_integer(evidence.observed_at_unix_ms, "observation timestamp")?;
        let expires_at = to_sql_integer(evidence.expires_at_unix_ms, "evidence expiry")?;
        if observed_at > now || now > expires_at || expires_at < observed_at {
            return Err(
                "independent evidence is future-dated or outside its freshness window".to_owned(),
            );
        }

        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin evidence transaction: {error}"))?;
        let operation = get_operation_conn(&transaction, operation_id)?
            .ok_or_else(|| format!("unknown operation {operation_id:?}"))?;
        if operation.state != PromotionState::Applied {
            return Err(format!(
                "fresh independent evidence is required from Applied state; found {:?}",
                operation.state
            ));
        }
        if evidence.target_token != operation.target_token
            || evidence.policy_generation != operation.generation
        {
            return Err(
                "evidence target or policy generation does not match the prepared operation"
                    .to_owned(),
            );
        }
        if operation.observed_policy_sha256.as_deref()
            != Some(operation.candidate_policy_sha256.as_str())
            || evidence.effective_policy_sha256 != operation.candidate_policy_sha256
        {
            return Err("evidence is not bound to the exact observed candidate policy".to_owned());
        }

        let changed = transaction
            .execute(
                "UPDATE promotion_operations
                 SET state = 'verified', challenge_id = ?1, receipt_sha256 = ?2,
                     evidence_observed_at_unix_ms = ?3,
                     evidence_expires_at_unix_ms = ?4,
                     verification_session_id = ?5, updated_at_unix_ms = ?6
                 WHERE operation_id = ?7 AND state = 'applied';",
                params![
                    evidence.challenge_id,
                    evidence.receipt_sha256,
                    observed_at,
                    expires_at,
                    self.session_id,
                    now,
                    operation_id
                ],
            )
            .map_err(|error| format!("cannot persist verification record: {error}"))?;
        if changed != 1 {
            return Err(
                "promotion state changed concurrently; refusing verification transition".to_owned(),
            );
        }
        transaction
            .commit()
            .map_err(|error| format!("cannot commit verification record: {error}"))?;
        Ok(PromotionPermit {
            operation_id: operation_id.to_owned(),
            session_id: self.session_id.clone(),
            expires_at_unix_ms: evidence.expires_at_unix_ms,
        })
    }

    /// Promote only while the process-local permit is live, the evidence has not
    /// expired, and a second effective-policy readback still equals the candidate.
    pub fn promote(
        &self,
        permit: &PromotionPermit,
        effective_policy_sha256: &str,
        now_unix_ms: u64,
    ) -> Result<PromotionOperation, String> {
        validate_digest(effective_policy_sha256, "effective_policy_sha256")?;
        if permit.session_id != self.session_id {
            return Err(
                "promotion permit belongs to a different verifier process session".to_owned(),
            );
        }
        if now_unix_ms > permit.expires_at_unix_ms {
            return Err("promotion evidence expired before commit".to_owned());
        }

        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin promotion transaction: {error}"))?;
        let operation = get_operation_conn(&transaction, &permit.operation_id)?
            .ok_or_else(|| "promotion permit references an unknown operation".to_owned())?;
        if operation.state != PromotionState::Verified {
            return Err(format!("cannot promote from state {:?}", operation.state));
        }
        if operation.candidate_policy_sha256 != effective_policy_sha256
            || operation.observed_policy_sha256.as_deref() != Some(effective_policy_sha256)
        {
            return Err(
                "promotion readback does not match the verified candidate policy".to_owned(),
            );
        }
        if operation
            .evidence_expires_at_unix_ms
            .is_none_or(|expiry| now_unix_ms > expiry)
            || operation
                .evidence_observed_at_unix_ms
                .is_none_or(|observed| observed > now_unix_ms)
        {
            return Err("persisted evidence is outside its valid time window".to_owned());
        }
        let session: Option<String> = transaction
            .query_row(
                "SELECT verification_session_id FROM promotion_operations WHERE operation_id = ?1;",
                params![permit.operation_id],
                |row| row.get(0),
            )
            .map_err(|error| format!("cannot read verification session binding: {error}"))?;
        if session.as_deref() != Some(self.session_id.as_str()) {
            return Err(
                "verification session binding does not match the current process".to_owned(),
            );
        }
        let changed = transaction
            .execute(
                "UPDATE promotion_operations SET state = 'promoted',
                    observed_policy_sha256 = ?1, verification_session_id = NULL,
                    updated_at_unix_ms = ?2
                 WHERE operation_id = ?3 AND state = 'verified'
                   AND verification_session_id = ?4;",
                params![
                    effective_policy_sha256,
                    to_sql_integer(now_unix_ms, "promotion timestamp")?,
                    permit.operation_id,
                    self.session_id
                ],
            )
            .map_err(|error| format!("cannot persist promotion: {error}"))?;
        if changed != 1 {
            return Err("promotion state changed concurrently; refusing promotion".to_owned());
        }
        let promoted = get_operation_conn(&transaction, &permit.operation_id)?
            .ok_or_else(|| "promoted operation disappeared before commit".to_owned())?;
        transaction
            .commit()
            .map_err(|error| format!("cannot commit policy promotion: {error}"))?;
        Ok(promoted)
    }

    /// Persist intent before attempting rollback. This transition is idempotent
    /// so recovery can safely re-assert the intent after a crash.
    pub fn request_rollback(
        &self,
        operation_id: &str,
        now_unix_ms: u64,
    ) -> Result<PromotionOperation, String> {
        let operation = self
            .get(operation_id)?
            .ok_or_else(|| format!("unknown operation {operation_id:?}"))?;
        if operation.state == PromotionState::RollbackIntent {
            return Ok(operation);
        }
        if !matches!(
            operation.state,
            PromotionState::ApplyIntent | PromotionState::Applied | PromotionState::Verified
        ) {
            return Err(format!(
                "cannot request rollback from state {:?}",
                operation.state
            ));
        }
        self.update_state(
            operation_id,
            operation.state.as_str(),
            "rollback_intent",
            now_unix_ms,
        )
    }

    /// Complete rollback only after effective-policy readback proves exact
    /// restoration of the original baseline digest.
    pub fn record_rolled_back(
        &self,
        operation_id: &str,
        effective_policy_sha256: &str,
        now_unix_ms: u64,
    ) -> Result<PromotionOperation, String> {
        validate_digest(effective_policy_sha256, "effective_policy_sha256")?;
        let operation = self
            .get(operation_id)?
            .ok_or_else(|| format!("unknown operation {operation_id:?}"))?;
        if operation.state != PromotionState::RollbackIntent {
            return Err(format!(
                "cannot record rollback from state {:?}",
                operation.state
            ));
        }
        if effective_policy_sha256 != operation.baseline_policy_sha256 {
            return Err(
                "rollback readback does not match the exact baseline policy digest".to_owned(),
            );
        }
        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin rollback transaction: {error}"))?;
        let changed = transaction
            .execute(
                "UPDATE promotion_operations SET state = 'rolled_back',
                    observed_policy_sha256 = ?1, verification_session_id = NULL,
                    updated_at_unix_ms = ?2
                 WHERE operation_id = ?3 AND state = 'rollback_intent'
                   AND baseline_policy_sha256 = ?1;",
                params![
                    effective_policy_sha256,
                    to_sql_integer(now_unix_ms, "rollback timestamp")?,
                    operation_id
                ],
            )
            .map_err(|error| format!("cannot persist completed rollback: {error}"))?;
        if changed != 1 {
            return Err("rollback state changed concurrently; refusing completion".to_owned());
        }
        let rolled_back = get_operation_conn(&transaction, operation_id)?
            .ok_or_else(|| "rolled-back operation disappeared before commit".to_owned())?;
        transaction
            .commit()
            .map_err(|error| format!("cannot commit rollback state: {error}"))?;
        Ok(rolled_back)
    }

    pub fn get(&self, operation_id: &str) -> Result<Option<PromotionOperation>, String> {
        let connection = connect(&self.path)?;
        get_operation_conn(&connection, operation_id)
    }

    /// Validate a process-local permit against the current durable journal and
    /// return a private-field binding for read-only fleet classification.
    ///
    /// This does not apply or promote a device policy. A caller must revalidate
    /// immediately before actuation and reconcile actual device state after a
    /// timeout; the returned binding does not eliminate external TOCTOU races.
    pub fn validate_permit(
        &self,
        permit: &PromotionPermit,
        now_unix_ms: u64,
    ) -> Result<VerifiedPromotionBinding, String> {
        if permit.session_id != self.session_id {
            return Err("promotion permit belongs to a different verifier process session".to_owned());
        }
        if now_unix_ms > permit.expires_at_unix_ms {
            return Err("promotion evidence expired before fleet evaluation".to_owned());
        }

        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin permit validation transaction: {error}"))?;
        let operation = get_operation_conn(&transaction, &permit.operation_id)?
            .ok_or_else(|| "promotion permit references an unknown operation".to_owned())?;
        if operation.state != PromotionState::Verified {
            return Err(format!("promotion permit is no longer Verified: {:?}", operation.state));
        }
        if operation.evidence_expires_at_unix_ms.is_none_or(|expiry| now_unix_ms > expiry)
            || operation.evidence_observed_at_unix_ms.is_none_or(|observed| observed > now_unix_ms)
        {
            return Err("promotion evidence is outside its durable time window".to_owned());
        }
        let stored_session: Option<String> = transaction
            .query_row(
                "SELECT verification_session_id FROM promotion_operations WHERE operation_id = ?1;",
                params![permit.operation_id],
                |row| row.get(0),
            )
            .map_err(|error| format!("cannot read permit verification session: {error}"))?;
        if stored_session.as_deref() != Some(self.session_id.as_str()) {
            return Err("durable verification session does not match current process".to_owned());
        }
        let challenge_id = operation.challenge_id.clone()
            .ok_or_else(|| "verified operation has no challenge ID".to_owned())?;
        let receipt_sha256 = operation.receipt_sha256.clone()
            .ok_or_else(|| "verified operation has no receipt digest".to_owned())?;
        let observed_at_unix_ms = operation.evidence_observed_at_unix_ms
            .ok_or_else(|| "verified operation has no evidence observation time".to_owned())?;
        let expires_at_unix_ms = operation.evidence_expires_at_unix_ms
            .ok_or_else(|| "verified operation has no evidence expiry".to_owned())?;
        if permit.expires_at_unix_ms != expires_at_unix_ms {
            return Err("process-local permit expiry does not match durable evidence expiry".to_owned());
        }
        if operation.observed_policy_sha256.as_deref()
            != Some(operation.candidate_policy_sha256.as_str())
        {
            return Err("verified operation no longer reads back the candidate policy".to_owned());
        }
        transaction.commit()
            .map_err(|error| format!("cannot commit permit validation transaction: {error}"))?;

        Ok(VerifiedPromotionBinding {
            operation_id: operation.operation_id,
            target_token: operation.target_token,
            generation: operation.generation,
            baseline_policy_sha256: operation.baseline_policy_sha256,
            candidate_policy_sha256: operation.candidate_policy_sha256,
            challenge_id,
            receipt_sha256,
            observed_at_unix_ms,
            expires_at_unix_ms,
        })
    }

    /// Return every non-terminal operation with an explicit fail-closed
    /// recovery action. No action here applies or promotes a device policy.
    pub fn list_recoverable(&self) -> Result<Vec<RecoverableOperation>, String> {
        let connection = connect(&self.path)?;
        let mut statement = connection
            .prepare(
                "SELECT operation_id, target_token, generation, baseline_policy_sha256,
                        candidate_policy_sha256, state, created_at_unix_ms, updated_at_unix_ms,
                        observed_policy_sha256, challenge_id, receipt_sha256,
                        evidence_observed_at_unix_ms, evidence_expires_at_unix_ms
                 FROM promotion_operations WHERE state NOT IN ('promoted', 'rolled_back', 'abandoned')
                 ORDER BY target_token, generation;",
            )
            .map_err(|error| format!("cannot query recoverable operations: {error}"))?;
        let rows = statement
            .query_map([], promotion_operation_from_row)
            .map_err(|error| format!("cannot list recoverable operations: {error}"))?;
        let mut recoverable = Vec::new();
        for row in rows {
            let operation =
                row.map_err(|error| format!("invalid promotion operation row: {error}"))?;
            if operation.state.is_terminal() {
                continue;
            }
            let action = match operation.state {
                PromotionState::Prepared => RecoveryAction::ApplyOrAbandonPreparedOperation,
                PromotionState::ApplyIntent => RecoveryAction::InspectEffectivePolicyBeforeRetry,
                PromotionState::Applied | PromotionState::Verified => {
                    RecoveryAction::RequireFreshChallengeAndIndependentProbe
                }
                PromotionState::RollbackIntent => RecoveryAction::InspectAndEnforceBaselinePolicy,
                PromotionState::Promoted
                | PromotionState::RolledBack
                | PromotionState::Abandoned => {
                    continue;
                }
            };
            recoverable.push(RecoverableOperation { operation, action });
        }
        Ok(recoverable)
    }

    fn update_state(
        &self,
        operation_id: &str,
        expected: &str,
        next: &str,
        now_unix_ms: u64,
    ) -> Result<PromotionOperation, String> {
        let mut connection = connect(&self.path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin state transition: {error}"))?;
        let changed = transaction
            .execute(
                "UPDATE promotion_operations SET state = ?1, updated_at_unix_ms = ?2
                 WHERE operation_id = ?3 AND state = ?4;",
                params![
                    next,
                    to_sql_integer(now_unix_ms, "transition timestamp")?,
                    operation_id,
                    expected
                ],
            )
            .map_err(|error| format!("cannot persist {next} transition: {error}"))?;
        if changed != 1 {
            return Err(format!(
                "operation {operation_id:?} is not in expected state {expected:?}; refusing transition"
            ));
        }
        let operation = get_operation_conn(&transaction, operation_id)?
            .ok_or_else(|| "operation disappeared during state transition".to_owned())?;
        transaction
            .commit()
            .map_err(|error| format!("cannot commit {next} transition: {error}"))?;
        Ok(operation)
    }
}

fn connect(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path)
        .map_err(|error| format!("cannot open promotion journal {}: {error}", path.display()))?;
    connection
        .busy_timeout(BUSY_TIMEOUT)
        .map_err(|error| format!("cannot configure promotion journal busy timeout: {error}"))?;
    connection
        .execute_batch("PRAGMA synchronous = FULL;")
        .map_err(|error| format!("cannot configure promotion journal durability: {error}"))?;
    Ok(connection)
}

fn table_exists(connection: &Connection) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master
             WHERE type = 'table' AND name = 'promotion_operations');",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("cannot inspect promotion journal tables: {error}"))
}

fn user_version(connection: &Connection) -> Result<i64, String> {
    connection
        .query_row("PRAGMA user_version;", [], |row| row.get(0))
        .map_err(|error| format!("cannot read promotion journal version: {error}"))
}

fn verify_schema(connection: &Connection) -> Result<(), String> {
    let version = user_version(connection)?;
    if version != JOURNAL_SCHEMA_VERSION {
        return Err(format!(
            "unsupported promotion journal schema version {version}; expected {JOURNAL_SCHEMA_VERSION}"
        ));
    }
    let mut statement = connection
        .prepare("PRAGMA table_info(promotion_operations);")
        .map_err(|error| format!("cannot inspect promotion journal schema: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(|error| format!("cannot query promotion journal schema: {error}"))?;
    let mut actual = Vec::new();
    for row in rows {
        actual.push(row.map_err(|error| format!("invalid promotion schema row: {error}"))?);
    }
    let expected = vec![
        ("operation_id", "TEXT", 1, 1),
        ("target_token", "TEXT", 1, 0),
        ("generation", "INTEGER", 1, 0),
        ("baseline_policy_sha256", "TEXT", 1, 0),
        ("candidate_policy_sha256", "TEXT", 1, 0),
        ("state", "TEXT", 1, 0),
        ("created_at_unix_ms", "INTEGER", 1, 0),
        ("updated_at_unix_ms", "INTEGER", 1, 0),
        ("observed_policy_sha256", "TEXT", 0, 0),
        ("challenge_id", "TEXT", 0, 0),
        ("receipt_sha256", "TEXT", 0, 0),
        ("evidence_observed_at_unix_ms", "INTEGER", 0, 0),
        ("evidence_expires_at_unix_ms", "INTEGER", 0, 0),
        ("verification_session_id", "TEXT", 0, 0),
    ]
    .into_iter()
    .map(|(name, kind, not_null, primary_key)| {
        (name.to_owned(), kind.to_owned(), not_null, primary_key)
    })
    .collect::<Vec<_>>();
    if actual != expected {
        return Err("promotion journal table shape does not match the supported schema".to_owned());
    }
    Ok(())
}

fn get_operation_conn(
    connection: &Connection,
    operation_id: &str,
) -> Result<Option<PromotionOperation>, String> {
    connection
        .query_row(
            "SELECT operation_id, target_token, generation, baseline_policy_sha256,
                    candidate_policy_sha256, state, created_at_unix_ms, updated_at_unix_ms,
                    observed_policy_sha256, challenge_id, receipt_sha256,
                    evidence_observed_at_unix_ms, evidence_expires_at_unix_ms
             FROM promotion_operations WHERE operation_id = ?1;",
            params![operation_id],
            promotion_operation_from_row,
        )
        .optional()
        .map_err(|error| format!("cannot read promotion operation: {error}"))
}

fn promotion_operation_from_row(row: &Row<'_>) -> rusqlite::Result<PromotionOperation> {
    let generation: i64 = row.get(2)?;
    let created: i64 = row.get(6)?;
    let updated: i64 = row.get(7)?;
    let observed: Option<i64> = row.get(11)?;
    let expires: Option<i64> = row.get(12)?;
    let observed_policy_sha256: Option<String> = row.get(8)?;
    let state_source: String = row.get(5)?;
    Ok(PromotionOperation {
        operation_id: row.get(0)?,
        target_token: row.get(1)?,
        generation: u64::try_from(generation).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Integer,
                Box::new(error),
            )
        })?,
        baseline_policy_sha256: row.get(3)?,
        candidate_policy_sha256: row.get(4)?,
        state: PromotionState::parse(&state_source).map_err(|_| {
            rusqlite::Error::FromSqlConversionFailure(
                5,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "unknown promotion state",
                )),
            )
        })?,
        created_at_unix_ms: u64::try_from(created).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Integer,
                Box::new(error),
            )
        })?,
        updated_at_unix_ms: u64::try_from(updated).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                7,
                rusqlite::types::Type::Integer,
                Box::new(error),
            )
        })?,
        observed_policy_sha256,
        challenge_id: row.get(9)?,
        receipt_sha256: row.get(10)?,
        evidence_observed_at_unix_ms: observed.and_then(|value| u64::try_from(value).ok()),
        evidence_expires_at_unix_ms: expires.and_then(|value| u64::try_from(value).ok()),
    })
}

fn json_string_field<'a>(
    value: &'a serde_json::Value,
    pointer: &str,
    label: &str,
) -> Result<&'a str, Vec<String>> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| vec![format!("{label} must be a string")])
}

fn json_integer_field(
    value: &serde_json::Value,
    pointer: &str,
    label: &str,
) -> Result<u64, Vec<String>> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| vec![format!("{label} must be an unsigned integer")])
}

fn validate_spec(spec: &PromotionSpec<'_>) -> Result<(), String> {
    validate_identifier(spec.operation_id, "operation_id")?;
    validate_target_token(spec.target_token)?;
    validate_digest(spec.baseline_policy_sha256, "baseline_policy_sha256")?;
    validate_digest(spec.candidate_policy_sha256, "candidate_policy_sha256")?;
    if spec.baseline_policy_sha256 == spec.candidate_policy_sha256 {
        return Err("candidate policy digest must differ from the approved baseline".to_owned());
    }
    if spec.generation == 0 || i64::try_from(spec.generation).is_err() {
        return Err("generation must be in the positive SQLite integer range".to_owned());
    }
    to_sql_integer(spec.created_at_unix_ms, "created timestamp")?;
    Ok(())
}

fn validate_identifier(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 80
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(format!(
            "{label} must be a non-empty ASCII identifier of at most 80 characters"
        ));
    }
    Ok(())
}

fn validate_target_token(value: &str) -> Result<(), String> {
    if !value.starts_with("target-")
        || value.len() < 8
        || value.len() > 55
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(
            "target_token must match the target- prefix and lowercase token alphabet".to_owned(),
        );
    }
    Ok(())
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

fn ensure_private_database_file(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err("promotion journal path must not be a symbolic link".to_owned());
            }
            if !metadata.is_file() {
                return Err("promotion journal path must be a regular file".to_owned());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o077 != 0 {
                    return Err(
                        "promotion journal database must not grant group or other permissions"
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
                    ensure_private_database_file(path)
                }
                Err(error) => Err(format!(
                    "cannot create promotion journal {}: {error}",
                    path.display()
                )),
            }
        }
        Err(error) => Err(format!(
            "cannot inspect promotion journal path {}: {error}",
            path.display()
        )),
    }
}

fn fresh_session_id() -> Result<String, String> {
    // A process-local binding, not an authentication credential. /dev/urandom
    // failure is fatal so session freshness is not silently downgraded.
    let mut random = [0_u8; 32];
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut random))
        .map_err(|error| format!("cannot obtain verifier-session randomness: {error}"))?;
    Ok(sha256_hex(&random))
}

fn unix_now_ms() -> Result<u64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock predates Unix epoch: {error}"))?;
    u64::try_from(duration.as_millis())
        .map_err(|error| format!("system timestamp conversion failed: {error}"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{
        PromotionEvidence, PromotionJournal, PromotionPermit, PromotionSpec, PromotionState,
        PromotionVerificationInput, RecoveryAction,
    };
    use crate::replay_ledger::SqliteReplayLedger;
    use serde_json::{Value, json};
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    const BASELINE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const CANDIDATE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const RECEIPT: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "luminous-edge-promotion-journal-{}-{nanos}-{id}.sqlite",
            std::process::id()
        ))
    }

    fn spec(generation: u64) -> PromotionSpec<'static> {
        PromotionSpec {
            operation_id: "operation-policy-0001",
            target_token: "target-policy-lab",
            generation,
            baseline_policy_sha256: BASELINE,
            candidate_policy_sha256: CANDIDATE,
            created_at_unix_ms: 100,
        }
    }

    fn evidence<'a>(observed_at_unix_ms: u64, expires_at_unix_ms: u64) -> PromotionEvidence<'a> {
        PromotionEvidence {
            challenge_id: "challenge-policy-0001",
            target_token: "target-policy-lab",
            policy_generation: 1,
            receipt_sha256: RECEIPT,
            effective_policy_sha256: CANDIDATE,
            observed_at_unix_ms,
            expires_at_unix_ms,
        }
    }

    fn remove_database(path: &PathBuf) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }

    const CORPUS_SCHEMA: &str =
        include_str!("../../../schemas/edge-policy-conformance-corpus-v1.schema.json");
    const CORPUS: &str =
        include_str!("../../../examples/edge-policy-conformance-corpus-synthetic-v1.json");
    const STARTED_AT: u64 = 1_050;
    const COMPLETED_AT: u64 = 1_200;
    const EXPIRES_AT: u64 = 2_000;

    fn freshness_sources(nonce: &str) -> (String, String, String) {
        let mut receipt = crate::build_synthetic_receipt(CORPUS).expect("receipt template");
        receipt["evidence_class"] = json!("namespace_lab_observation");
        receipt["run_id"] = json!("run-promotion-journal-0001");
        receipt["approved_policy_sha256"] = json!(BASELINE);
        receipt["effective_config_sha256"] = json!(CANDIDATE);
        receipt["target_token"] = json!("target-policy-lab");
        receipt["time_basis"] = json!({
            "kind": "monotonic_with_wall_anchor",
            "clock_id": "promotion-journal-test-clock",
            "wall_anchor_unix_ms": STARTED_AT,
            "monotonic_origin_ms": 0
        });
        for (index, observation) in receipt["observations"]
            .as_array_mut()
            .expect("observations array")
            .iter_mut()
            .enumerate()
        {
            observation["evidence_source"] = json!("independent_dataplane_probe");
            observation["observed_at_unix_ms"] = json!(STARTED_AT + index as u64);
            observation["monotonic_elapsed_ms"] = json!(index as u64);
            observation["duration_ms"] = json!(5);
            observation["evidence_sha256"] = json!("d".repeat(64));
            observation["effective_policy_sha256"] = json!(CANDIDATE);
        }
        let receipt_source = serde_json::to_string(&receipt).expect("serialize receipt");
        let challenge_source = json!({
            "schema_version": "1.0.0",
            "challenge_id": "challenge-policy-0001",
            "nonce": nonce,
            "issued_at_unix_ms": 1_000,
            "expires_at_unix_ms": EXPIRES_AT,
            "max_receipt_age_ms": 500,
            "policy_generation": 1,
            "corpus_sha256": crate::sha256_hex(CORPUS.as_bytes()),
            "approved_policy_sha256": BASELINE,
            "expected_effective_policy_sha256": CANDIDATE,
            "target_token": "target-policy-lab"
        })
        .to_string();
        let receipt_value: Value = serde_json::from_str(&receipt_source).expect("receipt JSON");
        let envelope_source = json!({
            "schema_version": "1.0.0",
            "challenge_id": "challenge-policy-0001",
            "nonce": nonce,
            "run_id": receipt_value["run_id"],
            "policy_generation": 1,
            "started_at_unix_ms": STARTED_AT,
            "completed_at_unix_ms": COMPLETED_AT,
            "receipt_sha256": crate::sha256_hex(receipt_source.as_bytes())
        })
        .to_string();
        (challenge_source, envelope_source, receipt_source)
    }

    fn prepared_applied(journal: &PromotionJournal) {
        journal.prepare(&spec(1)).expect("prepare");
        journal
            .record_apply_intent("operation-policy-0001", 110)
            .expect("intent");
        journal
            .record_applied("operation-policy-0001", CANDIDATE, 120)
            .expect("readback");
    }

    #[test]
    fn public_verification_gate_consumes_nonce_before_minting_promotion_permit() {
        let journal_path = path();
        let ledger_path = journal_path.with_file_name(format!(
            "{}-ledger.sqlite",
            journal_path
                .file_name()
                .expect("journal filename")
                .to_string_lossy()
        ));
        let journal = PromotionJournal::open(&journal_path).expect("open journal");
        let ledger = SqliteReplayLedger::open(&ledger_path).expect("open replay ledger");
        prepared_applied(&journal);
        let (challenge, envelope, receipt) =
            freshness_sources("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");

        let (report, permit) = journal
            .verify_and_record_evidence(
                &ledger,
                "operation-policy-0001",
                &PromotionVerificationInput {
                    challenge_source: &challenge,
                    envelope_source: &envelope,
                    corpus_schema_source: CORPUS_SCHEMA,
                    corpus_source: CORPUS,
                    receipt_source: &receipt,
                    effective_policy_sha256: CANDIDATE,
                    verifier_now_unix_ms: 1_400,
                },
            )
            .expect("verified receipt consumes nonce and returns permit");
        assert_eq!(
            report["durable_replay_ledger"],
            "commit_confirmed_before_return"
        );
        assert_eq!(
            journal
                .get("operation-policy-0001")
                .expect("read")
                .expect("operation")
                .state,
            PromotionState::Verified
        );
        let promoted = journal
            .promote(&permit, CANDIDATE, 1_600)
            .expect("promote with exact candidate readback");
        assert_eq!(promoted.state, PromotionState::Promoted);

        drop(ledger);
        drop(journal);
        remove_database(&journal_path);
        remove_database(&ledger_path);
    }

    #[test]
    fn prepared_operation_can_be_abandoned_without_claiming_device_rollback() {
        let path = path();
        let journal = PromotionJournal::open(&path).expect("open");
        journal.prepare(&spec(1)).expect("prepare");
        let abandoned = journal
            .abandon_prepared("operation-policy-0001", 105)
            .expect("abandon before external actuation");
        assert_eq!(abandoned.state, PromotionState::Abandoned);
        assert!(journal.list_recoverable().expect("recoverable").is_empty());

        let next_generation = PromotionSpec {
            operation_id: "operation-policy-0002",
            target_token: "target-policy-lab",
            generation: 2,
            baseline_policy_sha256: BASELINE,
            candidate_policy_sha256: CANDIDATE,
            created_at_unix_ms: 110,
        };
        assert_eq!(
            journal
                .prepare(&next_generation)
                .expect("new generation")
                .state,
            PromotionState::Prepared
        );
        drop(journal);
        remove_database(&path);
    }

    #[test]
    fn cannot_mark_applied_before_durable_apply_intent() {
        let path = path();
        let journal = PromotionJournal::open(&path).expect("open");
        journal.prepare(&spec(1)).expect("prepare");
        assert!(
            journal
                .record_applied("operation-policy-0001", CANDIDATE, 120)
                .expect_err("must require intent")
                .contains("cannot record applied policy")
        );
        assert_eq!(
            journal
                .get("operation-policy-0001")
                .expect("read")
                .expect("row")
                .state,
            PromotionState::Prepared
        );
        drop(journal);
        remove_database(&path);
    }

    #[test]
    fn promotes_only_after_exact_readback_fresh_evidence_and_same_session_permit() {
        let path = path();
        let journal = PromotionJournal::open(&path).expect("open");
        prepared_applied(&journal);
        let wrong_target = PromotionEvidence {
            target_token: "target-other-lab",
            ..evidence(130, 180)
        };
        assert!(
            journal
                .record_verified("operation-policy-0001", &wrong_target, 150)
                .expect_err("wrong target")
                .contains("target or policy generation")
        );
        let wrong_generation = PromotionEvidence {
            policy_generation: 2,
            ..evidence(130, 180)
        };
        assert!(
            journal
                .record_verified("operation-policy-0001", &wrong_generation, 150)
                .expect_err("wrong generation")
                .contains("target or policy generation")
        );
        assert!(
            journal
                .record_verified("operation-policy-0001", &evidence(130, 180), 181)
                .expect_err("expired evidence")
                .contains("outside its freshness window")
        );
        let permit: PromotionPermit = journal
            .record_verified("operation-policy-0001", &evidence(130, 180), 150)
            .expect("fresh independent evidence");
        assert!(
            journal
                .promote(&permit, BASELINE, 160)
                .expect_err("changed policy")
                .contains("does not match")
        );
        let promoted = journal
            .promote(&permit, CANDIDATE, 160)
            .expect("promote exact candidate");
        assert_eq!(promoted.state, PromotionState::Promoted);
        assert_eq!(promoted.observed_policy_sha256.as_deref(), Some(CANDIDATE));
        assert!(
            journal
                .promote(&permit, CANDIDATE, 160)
                .expect_err("permit is one-use")
                .contains("cannot promote")
        );
        drop(journal);
        remove_database(&path);
    }

    #[test]
    fn fleet_wave_binding_requires_current_permit_for_exact_target_and_policy() {
        use crate::fleet_rollout::{
            FleetTarget, RolloutWave, WaveDecision, validate_wave_permits,
        };

        let path = path();
        let journal = PromotionJournal::open(&path).expect("open");
        prepared_applied(&journal);
        let permit = journal
            .record_verified("operation-policy-0001", &evidence(130, 180), 150)
            .expect("verified permit");
        let target = FleetTarget {
            target_token: "target-policy-lab".to_owned(),
            failure_domain: "site-a".to_owned(),
            generation: 1,
            baseline_policy_sha256: BASELINE.to_owned(),
            candidate_policy_sha256: CANDIDATE.to_owned(),
            canary: true,
        };
        let wave = RolloutWave {
            index: 0,
            is_canary: true,
            targets: vec![target.clone()],
        };

        let validated = validate_wave_permits(&journal, &wave, &[&permit], 155)
            .expect("exact journal-bound permit");
        assert_eq!(validated.wave_index(), 0);
        assert_eq!(validated.len(), 1);
        assert_eq!(validated.bindings()[0].target_token(), "target-policy-lab");
        assert_eq!(validated.bindings()[0].generation(), 1);
        assert_eq!(validated.bindings()[0].baseline_policy_sha256(), BASELINE);
        assert_eq!(validated.bindings()[0].candidate_policy_sha256(), CANDIDATE);
        assert_eq!(
            validated.bindings()[0].challenge_id(),
            "challenge-policy-0001"
        );

        let mut wrong_candidate_wave = wave.clone();
        wrong_candidate_wave.targets[0].candidate_policy_sha256 = "d".repeat(64);
        let mismatch = validate_wave_permits(&journal, &wrong_candidate_wave, &[&permit], 155)
            .expect_err("wrong planned candidate");
        assert_eq!(mismatch.decision, WaveDecision::HaltAndReconcile);
        assert!(mismatch.reasons.iter().any(|reason| reason.contains("policy digests")));

        let expired = validate_wave_permits(&journal, &wave, &[&permit], 181)
            .expect_err("expired permit");
        assert_eq!(expired.decision, WaveDecision::HaltAndReconcile);
        assert!(expired.reasons.iter().any(|reason| reason.contains("expired")));

        let missing = validate_wave_permits(&journal, &wave, &[], 155)
            .expect_err("missing permit");
        assert_eq!(missing.decision, WaveDecision::HaltAndReconcile);
        assert!(missing.reasons.iter().any(|reason| reason.contains("permit count mismatch")));

        drop(journal);
        remove_database(&path);
    }

    #[test]
    fn reopen_invalidates_verified_evidence_and_recovery_requires_a_new_challenge() {
        let path = path();
        let journal = PromotionJournal::open(&path).expect("open");
        prepared_applied(&journal);
        let old_permit = journal
            .record_verified("operation-policy-0001", &evidence(130, 300), 150)
            .expect("verify");
        assert_eq!(
            journal
                .get("operation-policy-0001")
                .expect("read")
                .expect("row")
                .state,
            PromotionState::Verified
        );
        drop(journal);

        let reopened = PromotionJournal::open(&path).expect("reopen");
        let operation = reopened
            .get("operation-policy-0001")
            .expect("read")
            .expect("row");
        assert_eq!(operation.state, PromotionState::Applied);
        assert!(operation.challenge_id.is_none());
        assert_eq!(
            reopened.list_recoverable().expect("recovery listing")[0].action,
            RecoveryAction::RequireFreshChallengeAndIndependentProbe
        );
        assert!(
            reopened
                .promote(&old_permit, CANDIDATE, 160)
                .expect_err("previous process permit is invalid")
                .contains("different verifier process session")
        );
        assert!(
            reopened
                .validate_permit(&old_permit, 160)
                .expect_err("previous process fleet permit is invalid")
                .contains("different verifier process session")
        );
        let new_permit = reopened
            .record_verified("operation-policy-0001", &evidence(170, 300), 180)
            .expect("fresh challenge in new session");
        assert_eq!(
            reopened
                .promote(&new_permit, CANDIDATE, 190)
                .expect("promote")
                .state,
            PromotionState::Promoted
        );
        drop(reopened);
        remove_database(&path);
    }

    #[test]
    fn child_process_exits_after_durable_apply_intent() {
        let Ok(path) = std::env::var("LUMINOUS_PROMOTION_CRASH_TEST_DB") else {
            return;
        };
        let journal = PromotionJournal::open(&path).expect("child opens journal");
        journal.prepare(&spec(1)).expect("child prepares operation");
        journal
            .record_apply_intent("operation-policy-0001", 110)
            .expect("child commits apply intent");
        // Deliberately bypass destructors and the test harness to exercise a
        // real process exit after the durable transition committed.
        std::process::exit(77);
    }

    #[test]
    fn apply_intent_survives_abrupt_child_process_exit() {
        let path = path();
        let executable = std::env::current_exe().expect("current test executable");
        let status = Command::new(executable)
            .arg("child_process_exits_after_durable_apply_intent")
            .env(
                "LUMINOUS_PROMOTION_CRASH_TEST_DB",
                path.to_string_lossy().as_ref(),
            )
            .status()
            .expect("launch child process");
        assert_eq!(
            status.code(),
            Some(77),
            "child must reach the deliberate exit after durable intent commit"
        );

        let recovered = PromotionJournal::open(&path).expect("reopen after child exit");
        let operations = recovered
            .list_recoverable()
            .expect("list recoverable state");
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].operation.state, PromotionState::ApplyIntent);
        assert_eq!(
            operations[0].action,
            RecoveryAction::InspectEffectivePolicyBeforeRetry
        );
        drop(recovered);
        remove_database(&path);
    }

    #[test]
    fn apply_intent_survives_restart_and_demands_effective_policy_inspection() {
        let path = path();
        {
            let journal = PromotionJournal::open(&path).expect("open");
            journal.prepare(&spec(1)).expect("prepare");
            journal
                .record_apply_intent("operation-policy-0001", 110)
                .expect("intent");
        }
        let reopened = PromotionJournal::open(&path).expect("reopen");
        let recovery = reopened.list_recoverable().expect("list");
        assert_eq!(recovery.len(), 1);
        assert_eq!(recovery[0].operation.state, PromotionState::ApplyIntent);
        assert_eq!(
            recovery[0].action,
            RecoveryAction::InspectEffectivePolicyBeforeRetry
        );
        drop(reopened);
        remove_database(&path);
    }

    #[test]
    fn rollback_is_terminal_only_after_exact_baseline_readback() {
        let path = path();
        let journal = PromotionJournal::open(&path).expect("open");
        prepared_applied(&journal);
        journal
            .request_rollback("operation-policy-0001", 130)
            .expect("rollback intent");
        assert!(
            journal
                .record_rolled_back("operation-policy-0001", CANDIDATE, 140)
                .expect_err("wrong baseline")
                .contains("exact baseline")
        );
        let rolled_back = journal
            .record_rolled_back("operation-policy-0001", BASELINE, 150)
            .expect("baseline restored");
        assert_eq!(rolled_back.state, PromotionState::RolledBack);
        assert!(
            journal
                .list_recoverable()
                .expect("no live operations")
                .is_empty()
        );
        drop(journal);
        remove_database(&path);
    }

    #[test]
    fn active_target_and_reused_generation_are_rejected() {
        let path = path();
        let journal = PromotionJournal::open(&path).expect("open");
        journal.prepare(&spec(1)).expect("prepare");
        let another = PromotionSpec {
            operation_id: "operation-policy-0002",
            target_token: "target-policy-lab",
            generation: 2,
            baseline_policy_sha256: BASELINE,
            candidate_policy_sha256: CANDIDATE,
            created_at_unix_ms: 101,
        };
        assert!(
            journal
                .prepare(&another)
                .expect_err("active operation")
                .contains("non-terminal operation")
        );
        journal
            .record_apply_intent("operation-policy-0001", 110)
            .expect("intent");
        journal
            .record_applied("operation-policy-0001", CANDIDATE, 120)
            .expect("applied");
        journal
            .request_rollback("operation-policy-0001", 130)
            .expect("rollback intent");
        journal
            .record_rolled_back("operation-policy-0001", BASELINE, 140)
            .expect("rollback");
        let reused_generation = PromotionSpec {
            operation_id: "operation-policy-0002",
            target_token: "target-policy-lab",
            generation: 1,
            baseline_policy_sha256: BASELINE,
            candidate_policy_sha256: CANDIDATE,
            created_at_unix_ms: 200,
        };
        assert!(
            journal
                .prepare(&reused_generation)
                .expect_err("used generation")
                .contains("not greater")
        );
        drop(journal);
        remove_database(&path);
    }
}
