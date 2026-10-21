//! SQLite-backed one-use D1 boot-observation challenge consumer.
//!
//! This adapter persists a verified observation and consumes its challenge in
//! one SQLite transaction. It implements the verifier's ChallengeConsumer
//! trait, but does not verify COSE itself: callers should pass it only through
//! verify_and_consume(). Challenge issuance and target-key provisioning remain
//! trusted caller responsibilities.
//!
//! The schema is a single-host durable store. WAL + synchronous=FULL is set on
//! each connection. This is not a distributed lock or a kernel/power-loss
//! qualification claim.

use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use srq_eat_cose_verifier::{
    ChallengeConsumptionRequest, ChallengeConsumer, PROFILE, VerifiedChallengeConsumption,
};

const SCHEMA_VERSION: i64 = 1;
const APPLICATION_ID: i64 = 0x5352_5131; // ASCII "SRQ1".
/// Accept upstream versions documented to include the WAL-reset corruption fix:
/// the 3.44.6, 3.50.7, 3.51.3 backports and the 3.53.0+ line. The withdrawn
/// 3.52.0 line is deliberately rejected.

const SCHEMA: &str = include_str!("../../../benchmarks/srq-v1/sqlite-d1-challenge-consumption-reference.sql");

#[derive(Clone)]
pub struct IssuedChallenge {
    pub challenge_id: String,
    pub target_token: String,
    pub audience: String,
    pub decision_id: String,
    pub policy_generation: u64,
    pub approved_artifact_sha256: String,
    pub effective_policy_sha256: String,
    pub corpus_sha256: String,
    pub expected_deployment_id: String,
    pub expected_signer_key_id_hex: String,
    /// Raw nonce is held by the issuer; only its SHA-256 is persisted.
    pub nonce: Vec<u8>,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

impl std::fmt::Debug for IssuedChallenge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IssuedChallenge")
            .field("challenge_id", &self.challenge_id)
            .field("target_token", &self.target_token)
            .field("audience", &self.audience)
            .field("decision_id", &self.decision_id)
            .field("policy_generation", &self.policy_generation)
            .field("approved_artifact_sha256", &self.approved_artifact_sha256)
            .field("effective_policy_sha256", &self.effective_policy_sha256)
            .field("corpus_sha256", &self.corpus_sha256)
            .field("expected_deployment_id", &self.expected_deployment_id)
            .field("expected_signer_key_id_hex", &self.expected_signer_key_id_hex)
            .field("nonce", &"[REDACTED]")
            .field("issued_at_unix_seconds", &self.issued_at_unix_seconds)
            .field("expires_at_unix_seconds", &self.expires_at_unix_seconds)
            .finish()
    }
}

/// Committed boot-observation receipt reconstructed from SQLite. This
/// omits the raw nonce and exposes only its digest. It is data for
/// reconciliation, not proof of hardware measurement or release qualification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommittedD1Observation {
    pub receipt_sha256: String,
    pub challenge_id: String,
    pub profile_id: String,
    pub signer_key_id_hex: String,
    pub target_token: String,
    pub audience: String,
    pub decision_id: String,
    pub policy_generation: u64,
    pub approved_artifact_sha256: String,
    pub effective_policy_sha256: String,
    pub corpus_sha256: String,
    pub deployment_id: String,
    pub boot_instance_id: String,
    pub nonce_sha256: String,
    pub challenge_issued_at_unix_seconds: u64,
    pub challenge_expires_at_unix_seconds: u64,
    pub verifier_now_unix_seconds: u64,
    pub envelope_sha256: String,
    pub committed_at_unix_seconds: u64,
}

pub struct SqliteD1Store {
    connection: Connection,
}

#[derive(Debug)]
struct StoredChallenge {
    target_token: String,
    audience: String,
    decision_id: String,
    policy_generation: i64,
    approved_artifact_sha256: String,
    effective_policy_sha256: String,
    corpus_sha256: String,
    expected_deployment_id: String,
    expected_signer_key_id_hex: String,
    nonce_sha256: String,
    issued_at_unix_seconds: i64,
    expires_at_unix_seconds: i64,
    required_assurance_tier: String,
    consumed_receipt_sha256: Option<String>,
}

impl SqliteD1Store {
    /// Open a local database, initialize a genuinely empty file, or reject an
    /// unknown/corrupt schema. Existing unversioned tables are never adopted or
    /// silently replaced. The parent directory must already exist.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let mut connection =
            Connection::open(path).map_err(|error| format!("open D1 store: {error}"))?;
        let sqlite_version: String = connection
            .query_row("SELECT sqlite_version()", [], |row| row.get(0))
            .map_err(|error| format!("read SQLite runtime version: {error}"))?;
        let parsed_version = sqlite_version_tuple(&sqlite_version)
            .ok_or_else(|| format!("SQLite runtime version is malformed: {sqlite_version}"))?;
        if !sqlite_version_has_wal_reset_fix(parsed_version) {
            return Err(format!(
                "SQLite {sqlite_version} is on an unsupported or WAL-reset-unfixed release line; accepted upstream-fixed lines are 3.44.6+, 3.50.7+, 3.51.3+, and 3.53.0+"
            ));
        }
        connection
            .busy_timeout(Duration::from_secs(10))
            .map_err(|error| format!("set SQLite busy timeout: {error}"))?;

        let journal_mode: String = connection
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
            .map_err(|error| format!("set SQLite WAL mode: {error}"))?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            return Err(format!("SQLite refused WAL mode (reported {journal_mode})"));
        }
        connection
            .pragma_update(None, "synchronous", "FULL")
            .map_err(|error| format!("set SQLite synchronous=FULL: {error}"))?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(|error| format!("enable SQLite foreign keys: {error}"))?;

        let synchronous: i64 = connection
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .map_err(|error| format!("read SQLite synchronous mode: {error}"))?;
        let foreign_keys: i64 = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .map_err(|error| format!("read SQLite foreign-key mode: {error}"))?;
        if synchronous != 2 || foreign_keys != 1 {
            return Err("SQLite connection did not enforce WAL/FULL/foreign-key policy".into());
        }

        let user_version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| format!("read D1 schema version: {error}"))?;
        let application_id: i64 = connection
            .query_row("PRAGMA application_id", [], |row| row.get(0))
            .map_err(|error| format!("read D1 application ID: {error}"))?;
        let object_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master \
                 WHERE type IN ('table','trigger','view') AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .map_err(|error| format!("inspect D1 database objects: {error}"))?;

        match user_version {
            0 if object_count == 0 && application_id == 0 => {
                let ddl_start = SCHEMA
                    .find("CREATE TABLE d1_observation_challenges")
                    .ok_or("embedded D1 schema is missing the challenge table")?;
                let tx = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| format!("begin D1 schema initialization: {error}"))?;
                tx.execute_batch(&SCHEMA[ddl_start..])
                    .map_err(|error| format!("install D1 schema: {error}"))?;
                tx.execute_batch("PRAGMA user_version=1; PRAGMA application_id=1397903665;")
                    .map_err(|error| format!("version D1 schema: {error}"))?;
                tx.commit()
                    .map_err(|error| format!("commit D1 schema initialization: {error}"))?;
            }
            SCHEMA_VERSION if application_id == APPLICATION_ID => {}
            0 => {
                return Err(
                    "refusing to initialize a nonempty or unowned unversioned D1 database".into(),
                );
            }
            _ => {
                return Err(format!(
                    "unsupported D1 database identity/version (application_id={application_id}, user_version={user_version})"
                ));
            }
        }

        verify_schema_inventory(&connection)?;
        Ok(Self { connection })
    }

    /// Register a fresh challenge from the trusted issuer. This operation
    /// inserts only the hash of the nonce; the raw nonce is never written to
    /// SQLite. Duplicate challenge IDs are rejected.
    pub fn issue_challenge(&mut self, challenge: &IssuedChallenge) -> Result<(), String> {
        validate_issued_challenge(challenge)?;
        let nonce_sha256 = sha256_hex(&challenge.nonce);
        let generation = to_sqlite_integer(challenge.policy_generation, "policy generation")?;
        let issued_at = to_sqlite_integer(
            challenge.issued_at_unix_seconds,
            "challenge issue time",
        )?;
        let expires_at = to_sqlite_integer(
            challenge.expires_at_unix_seconds,
            "challenge expiry time",
        )?;

        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("begin D1 challenge issuance: {error}"))?;
        tx.execute(
            "INSERT INTO d1_observation_challenges (
                challenge_id,target_token,audience,decision_id,policy_generation,
                approved_artifact_sha256,effective_policy_sha256,corpus_sha256,
                expected_deployment_id,expected_signer_key_id_hex,nonce_sha256,
                issued_at_unix_seconds,expires_at_unix_seconds,required_assurance_tier,
                consumed_receipt_sha256
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'D1',NULL)",
            params![
                challenge.challenge_id,
                challenge.target_token,
                challenge.audience,
                challenge.decision_id,
                generation,
                challenge.approved_artifact_sha256,
                challenge.effective_policy_sha256,
                challenge.corpus_sha256,
                challenge.expected_deployment_id,
                challenge.expected_signer_key_id_hex,
                nonce_sha256,
                issued_at,
                expires_at
            ],
        )
        .map_err(|error| format!("persist D1 challenge: {error}"))?;
        tx.commit()
            .map_err(|error| format!("commit D1 challenge issuance: {error}"))
    }

    /// Recover the committed receipt after a caller dies after COMMIT but
    /// before receiving the acknowledgement. This is read-only reconciliation,
    /// not a replay acceptance path: consume_once still rejects a consumed nonce.
    pub fn committed_receipt_for_challenge(
        &self,
        challenge_id: &str,
    ) -> Result<Option<String>, String> {
        if !valid_identifier(challenge_id) {
            return Err("challenge ID is malformed".into());
        }
        let pointer: Option<Option<String>> = self
            .connection
            .query_row(
                "SELECT consumed_receipt_sha256 FROM d1_observation_challenges WHERE challenge_id=?1",
                params![challenge_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("read D1 challenge state: {error}"))?;

        let Some(Some(receipt_sha256)) = pointer else {
            return Ok(None);
        };
        let matching_rows: i64 = self
            .connection
            .query_row(
                "SELECT count(*) \
                 FROM d1_observation_challenges c \
                 JOIN d1_verified_observation_receipts r \
                   ON r.receipt_sha256=c.consumed_receipt_sha256 \
                 WHERE c.challenge_id=?1 AND c.consumed_receipt_sha256=?2 \
                   AND r.challenge_id=c.challenge_id",
                params![challenge_id, receipt_sha256],
                |row| row.get(0),
            )
            .map_err(|error| format!("reconcile D1 receipt pointer: {error}"))?;
        if matching_rows != 1 {
            return Err("D1 challenge ledger is inconsistent with its durable receipt".into());
        }
        Ok(Some(receipt_sha256))
    }

    /// Run SQLite's structural and foreign-key integrity checks and verify the
    /// challenge-to-receipt one-to-one invariant explicitly.
    /// Re-read a committed D1 observation from SQLite for cross-layer
    /// reconciliation instead of rebuilding it from in-memory request fields.
    /// A receipt that no longer matches its issued challenge fails closed.
    pub fn committed_observation_for_challenge(
        &self,
        challenge_id: &str,
    ) -> Result<Option<CommittedD1Observation>, String> {
        let Some(receipt_sha256) = self.committed_receipt_for_challenge(challenge_id)? else {
            return Ok(None);
        };
        let matching_rows: i64 = self.connection.query_row(
            "SELECT count(*)
               FROM d1_observation_challenges c
               JOIN d1_verified_observation_receipts r
                 ON r.receipt_sha256=c.consumed_receipt_sha256
                AND r.challenge_id=c.challenge_id
              WHERE c.challenge_id=?1 AND r.receipt_sha256=?2
                AND c.required_assurance_tier='D1'
                AND r.target_token=c.target_token AND r.audience=c.audience
                AND r.decision_id=c.decision_id
                AND r.policy_generation=c.policy_generation
                AND r.approved_artifact_sha256=c.approved_artifact_sha256
                AND r.effective_policy_sha256=c.effective_policy_sha256
                AND r.corpus_sha256=c.corpus_sha256
                AND r.deployment_id=c.expected_deployment_id
                AND r.signer_key_id_hex=c.expected_signer_key_id_hex
                AND r.nonce_sha256=c.nonce_sha256
                AND r.challenge_issued_at_unix_seconds=c.issued_at_unix_seconds
                AND r.challenge_expires_at_unix_seconds=c.expires_at_unix_seconds
                AND r.verifier_now_unix_seconds>=c.issued_at_unix_seconds
                AND r.verifier_now_unix_seconds<c.expires_at_unix_seconds",
            params![challenge_id, receipt_sha256],
            |row| row.get(0),
        ).map_err(|error| format!("verify durable D1 receipt bindings: {error}"))?;
        if matching_rows != 1 {
            return Err("committed D1 receipt does not match exactly one issued challenge".into());
        }

        let row = self.connection.query_row(
            "SELECT receipt_sha256,challenge_id,profile_id,signer_key_id_hex,
                    target_token,audience,decision_id,policy_generation,
                    approved_artifact_sha256,effective_policy_sha256,corpus_sha256,
                    deployment_id,boot_instance_id,nonce_sha256,
                    challenge_issued_at_unix_seconds,challenge_expires_at_unix_seconds,
                    verifier_now_unix_seconds,envelope_sha256,committed_at_unix_seconds
               FROM d1_verified_observation_receipts
              WHERE challenge_id=?1 AND receipt_sha256=?2",
            params![challenge_id, receipt_sha256],
            |row| Ok((
                row.get::<_, String>(0)?, row.get::<_, String>(1)?,
                row.get::<_, String>(2)?, row.get::<_, String>(3)?,
                row.get::<_, String>(4)?, row.get::<_, String>(5)?,
                row.get::<_, String>(6)?, row.get::<_, i64>(7)?,
                row.get::<_, String>(8)?, row.get::<_, String>(9)?,
                row.get::<_, String>(10)?, row.get::<_, String>(11)?,
                row.get::<_, String>(12)?, row.get::<_, String>(13)?,
                row.get::<_, i64>(14)?, row.get::<_, i64>(15)?,
                row.get::<_, i64>(16)?, row.get::<_, String>(17)?,
                row.get::<_, i64>(18)?,
            )),
        ).optional().map_err(|error| format!("read durable D1 receipt record: {error}"))?;
        let Some((
            receipt_sha256, challenge_id, profile_id, signer_key_id_hex,
            target_token, audience, decision_id, generation,
            approved_artifact_sha256, effective_policy_sha256, corpus_sha256,
            deployment_id, boot_instance_id, nonce_sha256, issued, expires,
            verifier_now, envelope_sha256, committed,
        )) = row else {
            return Err("committed D1 receipt pointer is missing its record".into());
        };
        if profile_id != PROFILE {
            return Err("durable D1 receipt has an unexpected profile".into());
        }
        let as_u64 = |value: i64, label: &str| {
            u64::try_from(value).map_err(|_| format!("stored D1 {label} is outside unsigned range"))
        };
        Ok(Some(CommittedD1Observation {
            receipt_sha256,
            challenge_id,
            profile_id,
            signer_key_id_hex,
            target_token,
            audience,
            decision_id,
            policy_generation: as_u64(generation, "generation")?,
            approved_artifact_sha256,
            effective_policy_sha256,
            corpus_sha256,
            deployment_id,
            boot_instance_id,
            nonce_sha256,
            challenge_issued_at_unix_seconds: as_u64(issued, "issue time")?,
            challenge_expires_at_unix_seconds: as_u64(expires, "expiry time")?,
            verifier_now_unix_seconds: as_u64(verifier_now, "verifier time")?,
            envelope_sha256,
            committed_at_unix_seconds: as_u64(committed, "commit time")?,
        }))
    }

    pub fn integrity_check(&self) -> Result<(), String> {
        let mut statement = self
            .connection
            .prepare("PRAGMA integrity_check")
            .map_err(|error| format!("prepare SQLite integrity check: {error}"))?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| format!("run SQLite integrity check: {error}"))?;
        let mut saw_row = false;
        for row in rows {
            saw_row = true;
            let value = row.map_err(|error| format!("read SQLite integrity result: {error}"))?;
            if value != "ok" {
                return Err(format!("SQLite integrity_check reported: {value}"));
            }
        }
        if !saw_row {
            return Err("SQLite integrity_check returned no result".into());
        }
        drop(statement);

        let mut foreign_keys = self
            .connection
            .prepare("PRAGMA foreign_key_check")
            .map_err(|error| format!("prepare SQLite foreign-key check: {error}"))?;
        let mut rows = foreign_keys
            .query([])
            .map_err(|error| format!("run SQLite foreign-key check: {error}"))?;
        if rows
            .next()
            .map_err(|error| format!("read SQLite foreign-key check: {error}"))?
            .is_some()
        {
            return Err("SQLite foreign_key_check found a violation".into());
        }
        drop(rows);
        drop(foreign_keys);

        let mismatches: i64 = self
            .connection
            .query_row(
                "SELECT \
                    (SELECT count(*) FROM d1_observation_challenges c \
                      LEFT JOIN d1_verified_observation_receipts r \
                        ON r.receipt_sha256=c.consumed_receipt_sha256 \
                      WHERE c.consumed_receipt_sha256 IS NOT NULL \
                        AND (r.receipt_sha256 IS NULL OR r.challenge_id<>c.challenge_id)) \
                  + (SELECT count(*) FROM d1_verified_observation_receipts r \
                      LEFT JOIN d1_observation_challenges c ON c.challenge_id=r.challenge_id \
                      WHERE c.challenge_id IS NULL OR c.consumed_receipt_sha256 IS NULL \
                         OR c.consumed_receipt_sha256<>r.receipt_sha256)",
                [],
                |row| row.get(0),
            )
            .map_err(|error| format!("check D1 challenge/receipt consistency: {error}"))?;
        if mismatches != 0 {
            return Err(format!(
                "D1 challenge/receipt ledger has {mismatches} inconsistent row(s)"
            ));
        }
        Ok(())
    }

    fn consume_once_inner(
        &mut self,
        request: &ChallengeConsumptionRequest<'_>,
        test_now_override: Option<u64>,
    ) -> Result<String, String> {
        self.consume_once_with_clock(request, || match test_now_override {
            Some(value) => Ok(value),
            None => system_time_unix_seconds(),
        })
    }

    fn consume_once_with_clock<F>(
        &mut self,
        request: &ChallengeConsumptionRequest<'_>,
        mut clock: F,
    ) -> Result<String, String>
    where
        F: FnMut() -> Result<u64, String>,
    {
        validate_request_shape(request)?;
        let generation =
            to_sqlite_integer(request.policy_generation, "policy generation")?;
        let issued_at = to_sqlite_integer(
            request.challenge_issued_at_unix_seconds,
            "challenge issue time",
        )?;
        let expires_at = to_sqlite_integer(
            request.challenge_expires_at_unix_seconds,
            "challenge expiry time",
        )?;
        let nonce_sha256 = sha256_hex(request.nonce);

        // Acquire SQLite's writer reservation before taking the trusted-time
        // snapshot and checking freshness. All subsequent reads/writes occur
        // under this one transaction, so a rival consumer cannot pass a stale
        // pre-transaction check and also commit.
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("begin D1 consumption transaction: {error}"))?;
        let now = clock()?;
        let now_sql = to_sqlite_integer(now, "verifier time")?;
        let stored = load_challenge(&tx, request.challenge_id)?;

        if stored.consumed_receipt_sha256.is_some() {
            return Err("D1 challenge has already been consumed".into());
        }
        if stored.required_assurance_tier != "D1"
            || stored.target_token != request.target_token
            || stored.audience != request.audience
            || stored.decision_id != request.decision_id
            || stored.policy_generation != generation
            || stored.approved_artifact_sha256 != request.approved_artifact_sha256
            || stored.effective_policy_sha256 != request.effective_policy_sha256
            || stored.corpus_sha256 != request.corpus_sha256
            || stored.expected_deployment_id != request.booted_deployment_id
            || stored.expected_signer_key_id_hex != request.signer_key_id_hex
            || stored.nonce_sha256 != nonce_sha256
            || stored.issued_at_unix_seconds != issued_at
            || stored.expires_at_unix_seconds != expires_at
            || now < request.challenge_issued_at_unix_seconds
            || now >= request.challenge_expires_at_unix_seconds
        {
            return Err("D1 observation does not match a fresh target-bound challenge".into());
        }

        let receipt_sha256 = receipt_digest(request, &nonce_sha256, now);
        tx.execute(
            "INSERT INTO d1_verified_observation_receipts (
                receipt_sha256,challenge_id,profile_id,signer_key_id_hex,target_token,
                audience,decision_id,policy_generation,approved_artifact_sha256,
                effective_policy_sha256,corpus_sha256,deployment_id,boot_instance_id,
                nonce_sha256,challenge_issued_at_unix_seconds,
                challenge_expires_at_unix_seconds,verifier_now_unix_seconds,
                envelope_sha256,committed_at_unix_seconds
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?17)",
            params![
                receipt_sha256,
                request.challenge_id,
                request.profile_id,
                request.signer_key_id_hex,
                request.target_token,
                request.audience,
                request.decision_id,
                generation,
                request.approved_artifact_sha256,
                request.effective_policy_sha256,
                request.corpus_sha256,
                request.booted_deployment_id,
                request.boot_instance_id,
                nonce_sha256,
                issued_at,
                expires_at,
                now_sql,
                request.envelope_sha256
            ],
        )
        .map_err(|error| format!("insert immutable D1 receipt: {error}"))?;
        #[cfg(test)]
        test_pause_at_commit_point("after_receipt_insert");

        let changed = tx
            .execute(
                "UPDATE d1_observation_challenges \
                    SET consumed_receipt_sha256=?1 \
                  WHERE challenge_id=?2 \
                    AND target_token=?3 AND audience=?4 AND decision_id=?5 \
                    AND policy_generation=?6 AND approved_artifact_sha256=?7 \
                    AND effective_policy_sha256=?8 AND corpus_sha256=?9 \
                    AND expected_deployment_id=?10 AND expected_signer_key_id_hex=?11 \
                    AND nonce_sha256=?12 AND issued_at_unix_seconds=?13 \
                    AND expires_at_unix_seconds=?14 AND required_assurance_tier='D1' \
                    AND consumed_receipt_sha256 IS NULL \
                    AND ?15 >= issued_at_unix_seconds AND ?15 < expires_at_unix_seconds",
                params![
                    receipt_sha256,
                    request.challenge_id,
                    request.target_token,
                    request.audience,
                    request.decision_id,
                    generation,
                    request.approved_artifact_sha256,
                    request.effective_policy_sha256,
                    request.corpus_sha256,
                    request.booted_deployment_id,
                    request.signer_key_id_hex,
                    nonce_sha256,
                    issued_at,
                    expires_at,
                    now_sql
                ],
            )
            .map_err(|error| format!("consume D1 challenge: {error}"))?;
        if changed != 1 {
            return Err("D1 challenge compare-and-swap did not update exactly one row".into());
        }
        #[cfg(test)]
        test_pause_at_commit_point("after_challenge_update_before_commit");

        // Recheck challenge freshness after all staged database writes. If the
        // authorization expired while SQLite was doing the work, returning
        // drops this transaction and rolls back both the receipt and the
        // one-use challenge update.
        let commit_now = clock()?;
        if commit_now < request.challenge_issued_at_unix_seconds
            || commit_now >= request.challenge_expires_at_unix_seconds
        {
            return Err("D1 challenge expired before transaction commit".into());
        }

        tx.commit()
            .map_err(|error| format!("commit D1 receipt/challenge transaction: {error}"))?;
        #[cfg(test)]
        test_pause_at_commit_point("after_commit_before_ack");
        Ok(receipt_sha256)
    }
}

impl ChallengeConsumer for SqliteD1Store {
    fn consume_once(&mut self, verified: &VerifiedChallengeConsumption<'_>) -> Result<String, String> {
        self.consume_once_inner(verified.request(), None)
    }
}

fn schema_objects(connection: &Connection) -> Result<Vec<(String, String, String)>, String> {
    let mut statement = connection
        .prepare(
            "SELECT type,name,sql FROM sqlite_master \
             WHERE type IN ('table','trigger','view') AND name NOT LIKE 'sqlite_%' \
             ORDER BY type,name",
        )
        .map_err(|error| format!("prepare D1 schema fingerprint: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| format!("query D1 schema fingerprint: {error}"))?;
    let mut objects = Vec::new();
    for row in rows {
        objects.push(row.map_err(|error| format!("read D1 schema fingerprint: {error}"))?);
    }
    Ok(objects)
}

fn verify_schema_inventory(connection: &Connection) -> Result<(), String> {
    // Compare the persisted SQL definitions—not merely the object names—with
    // the canonical schema parsed by the exact same SQLite library. This
    // rejects a dropped/re-created trigger that keeps the trusted name but
    // weakens its enforcement. A schema change requires a versioned migration.
    let ddl_start = SCHEMA
        .find("CREATE TABLE d1_observation_challenges")
        .ok_or("embedded D1 schema is missing the challenge table")?;
    let canonical = Connection::open_in_memory()
        .map_err(|error| format!("open canonical D1 schema connection: {error}"))?;
    canonical
        .execute_batch(&SCHEMA[ddl_start..])
        .map_err(|error| format!("materialize canonical D1 schema: {error}"))?;

    let actual = schema_objects(connection)?;
    let expected = schema_objects(&canonical)?;
    if actual != expected {
        return Err("D1 schema SQL differs from the canonical versioned schema".into());
    }
    Ok(())
}
fn load_challenge(
    tx: &Transaction<'_>,
    challenge_id: &str,
) -> Result<StoredChallenge, String> {
    tx.query_row(
        "SELECT target_token,audience,decision_id,policy_generation,
                approved_artifact_sha256,effective_policy_sha256,corpus_sha256,
                expected_deployment_id,expected_signer_key_id_hex,nonce_sha256,
                issued_at_unix_seconds,expires_at_unix_seconds,
                required_assurance_tier,consumed_receipt_sha256
           FROM d1_observation_challenges WHERE challenge_id=?1",
        params![challenge_id],
        |row| {
            Ok(StoredChallenge {
                target_token: row.get(0)?,
                audience: row.get(1)?,
                decision_id: row.get(2)?,
                policy_generation: row.get(3)?,
                approved_artifact_sha256: row.get(4)?,
                effective_policy_sha256: row.get(5)?,
                corpus_sha256: row.get(6)?,
                expected_deployment_id: row.get(7)?,
                expected_signer_key_id_hex: row.get(8)?,
                nonce_sha256: row.get(9)?,
                issued_at_unix_seconds: row.get(10)?,
                expires_at_unix_seconds: row.get(11)?,
                required_assurance_tier: row.get(12)?,
                consumed_receipt_sha256: row.get(13)?,
            })
        },
    )
    .map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => "D1 challenge is not registered".into(),
        other => format!("load outstanding D1 challenge: {other}"),
    })
}

fn validate_issued_challenge(challenge: &IssuedChallenge) -> Result<(), String> {
    for (label, value) in [
        ("challenge ID", challenge.challenge_id.as_str()),
        ("target token", challenge.target_token.as_str()),
        ("audience", challenge.audience.as_str()),
        ("decision ID", challenge.decision_id.as_str()),
    ] {
        if !valid_identifier(value) {
            return Err(format!("{label} is malformed"));
        }
    }
    for (label, value) in [
        ("approved artifact digest", challenge.approved_artifact_sha256.as_str()),
        ("effective policy digest", challenge.effective_policy_sha256.as_str()),
        ("corpus digest", challenge.corpus_sha256.as_str()),
    ] {
        if !is_sha256(value) {
            return Err(format!("{label} is malformed"));
        }
    }
    if !valid_deployment_id(&challenge.expected_deployment_id) {
        return Err("expected deployment ID is malformed".into());
    }
    if !valid_hex_key_id(&challenge.expected_signer_key_id_hex) {
        return Err("expected signer key ID is malformed".into());
    }
    if !(16..=64).contains(&challenge.nonce.len()) {
        return Err("D1 nonce must contain 16..=64 bytes".into());
    }
    to_sqlite_integer(challenge.policy_generation, "policy generation")?;
    to_sqlite_integer(challenge.issued_at_unix_seconds, "challenge issue time")?;
    to_sqlite_integer(challenge.expires_at_unix_seconds, "challenge expiry time")?;
    if challenge.issued_at_unix_seconds >= challenge.expires_at_unix_seconds {
        return Err("challenge expiry must be later than issue time".into());
    }
    Ok(())
}

fn validate_request_shape(request: &ChallengeConsumptionRequest<'_>) -> Result<(), String> {
    if request.profile_id != PROFILE {
        return Err("D1 profile mismatch".into());
    }
    for (label, value) in [
        ("challenge ID", request.challenge_id),
        ("target token", request.target_token),
        ("audience", request.audience),
        ("decision ID", request.decision_id),
        ("boot instance ID", request.boot_instance_id),
    ] {
        if !valid_identifier(value) {
            return Err(format!("{label} is malformed"));
        }
    }
    if !valid_hex_key_id(request.signer_key_id_hex) {
        return Err("signer key ID is malformed".into());
    }
    for (label, value) in [
        ("approved artifact digest", request.approved_artifact_sha256),
        ("effective policy digest", request.effective_policy_sha256),
        ("corpus digest", request.corpus_sha256),
        ("envelope digest", request.envelope_sha256),
    ] {
        if !is_sha256(value) {
            return Err(format!("{label} is malformed"));
        }
    }
    if !valid_deployment_id(request.booted_deployment_id) {
        return Err("booted deployment ID is malformed".into());
    }
    if !(16..=64).contains(&request.nonce.len()) {
        return Err("D1 nonce must contain 16..=64 bytes".into());
    }
    to_sqlite_integer(request.policy_generation, "policy generation")?;
    to_sqlite_integer(
        request.challenge_issued_at_unix_seconds,
        "challenge issue time",
    )?;
    to_sqlite_integer(
        request.challenge_expires_at_unix_seconds,
        "challenge expiry time",
    )?;
    if request.challenge_issued_at_unix_seconds >= request.challenge_expires_at_unix_seconds {
        return Err("challenge expiry must be later than issue time".into());
    }
    Ok(())
}

fn receipt_digest(
    request: &ChallengeConsumptionRequest<'_>,
    nonce_sha256: &str,
    now: u64,
) -> String {
    // Canonical field order/encoding matches the shell reference script. All
    // interpolated identifiers are validated against a newline-free alphabet.
    let fields = [
        format!("profile={}", request.profile_id),
        format!("challenge={}", request.challenge_id),
        format!("target={}", request.target_token),
        format!("audience={}", request.audience),
        format!("decision={}", request.decision_id),
        format!("generation={}", request.policy_generation),
        format!("approved={}", request.approved_artifact_sha256),
        format!("effective={}", request.effective_policy_sha256),
        format!("corpus={}", request.corpus_sha256),
        format!("deployment={}", request.booted_deployment_id),
        format!("signer_key_id={}", request.signer_key_id_hex),
        format!("nonce_sha256={nonce_sha256}"),
        format!("issued={}", request.challenge_issued_at_unix_seconds),
        format!("expires={}", request.challenge_expires_at_unix_seconds),
        format!("verifier_now={now}"),
        format!("envelope_sha256={}", request.envelope_sha256),
        format!("boot_instance_id={}", request.boot_instance_id),
    ];
    sha256_hex(fields.join("\n").as_bytes())
}

fn sqlite_version_tuple(value: &str) -> Option<(u64, u64, u64)> {
    let mut parts = value.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn sqlite_version_has_wal_reset_fix((major, minor, patch): (u64, u64, u64)) -> bool {
    match (major, minor) {
        (3, 44) => patch >= 6,
        (3, 50) => patch >= 7,
        (3, 51) => patch >= 3,
        (3, 53) => true,
        (3, minor) if minor > 53 => true,
        _ => false,
    }
}

fn system_time_unix_seconds() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| "system clock is before the Unix epoch".into())
}

fn to_sqlite_integer(value: u64, label: &str) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| format!("{label} exceeds SQLite INTEGER range"))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b':'))
}

fn valid_hex_key_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.len().is_multiple_of(2)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_deployment_id(value: &str) -> bool {
    if is_sha256(value) {
        return true;
    }
    let Some(rest) = value.strip_prefix("/nix/store/") else {
        return false;
    };
    let Some((hash, name)) = rest.split_once('-') else {
        return false;
    };
    hash.len() == 32
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || b"abcdfghijklmnpqrsvwxyz".contains(&byte))
        && !name.is_empty()
        && name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-')
        })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
fn test_pause_at_commit_point(point: &str) {
    use std::fs::OpenOptions;
    use std::io::Write as _;
    use std::thread;

    if std::env::var("SRQ_D1_SQLITE_KILL_POINT").ok().as_deref() != Some(point) {
        return;
    }
    let marker = std::env::var_os("SRQ_D1_SQLITE_KILL_MARKER")
        .expect("SIGKILL test must provide a marker path");
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(marker)
        .expect("create D1 SIGKILL marker");
    writeln!(file, "{point}").expect("write D1 SIGKILL marker");
    file.sync_all().expect("sync D1 SIGKILL marker");
    drop(file);
    loop {
        thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};
    use std::sync::{Arc, Barrier};

    fn temporary_path(label: &str, suffix: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "srq-d1-sqlite-store-{label}-{}-{nonce}.{suffix}",
            std::process::id()
        ))
    }

    fn challenge_for(challenge_id: &str, now: u64) -> IssuedChallenge {
        IssuedChallenge {
            challenge_id: challenge_id.into(),
            target_token: "target-token-001".into(),
            audience: "srq-verifier-test".into(),
            decision_id: "decision-001".into(),
            policy_generation: 42,
            approved_artifact_sha256: "a".repeat(64),
            effective_policy_sha256: "b".repeat(64),
            corpus_sha256: "c".repeat(64),
            expected_deployment_id:
                "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-nixos-system-demo".into(),
            expected_signer_key_id_hex: "7372712d746573742d6b65792d3031".into(),
            nonce: b"0123456789abcdeffresh-nonce".to_vec(),
            issued_at_unix_seconds: now.saturating_sub(60),
            expires_at_unix_seconds: now.saturating_add(3_600),
        }
    }

    fn request_for<'a>(
        challenge: &'a IssuedChallenge,
        boot_instance_id: &'a str,
        envelope_sha256: &'a str,
    ) -> ChallengeConsumptionRequest<'a> {
        ChallengeConsumptionRequest {
            profile_id: PROFILE,
            signer_key_id_hex: &challenge.expected_signer_key_id_hex,
            challenge_id: &challenge.challenge_id,
            nonce: &challenge.nonce,
            decision_id: &challenge.decision_id,
            target_token: &challenge.target_token,
            audience: &challenge.audience,
            policy_generation: challenge.policy_generation,
            approved_artifact_sha256: &challenge.approved_artifact_sha256,
            effective_policy_sha256: &challenge.effective_policy_sha256,
            corpus_sha256: &challenge.corpus_sha256,
            booted_deployment_id: &challenge.expected_deployment_id,
            boot_instance_id,
            challenge_issued_at_unix_seconds: challenge.issued_at_unix_seconds,
            challenge_expires_at_unix_seconds: challenge.expires_at_unix_seconds,
            envelope_sha256,
        }
    }

    fn db_row_counts(store: &SqliteD1Store, challenge_id: &str) -> (i64, i64, i64) {
        let receipts = store.connection.query_row(
            "SELECT count(*) FROM d1_verified_observation_receipts WHERE challenge_id=?1",
            params![challenge_id],
            |row| row.get(0),
        ).expect("receipt count");
        let consumed = store.connection.query_row(
            "SELECT count(*) FROM d1_observation_challenges WHERE challenge_id=?1 AND consumed_receipt_sha256 IS NOT NULL",
            params![challenge_id],
            |row| row.get(0),
        ).expect("challenge consumption count");
        let challenges = store.connection.query_row(
            "SELECT count(*) FROM d1_observation_challenges WHERE challenge_id=?1",
            params![challenge_id],
            |row| row.get(0),
        ).expect("challenge count");
        (challenges, receipts, consumed)
    }

    #[test]
    fn runtime_sqlite_is_above_the_upstream_wal_reset_bug_floor() {
        let path = temporary_path("runtime-version", "sqlite");
        let store = SqliteD1Store::open(&path).expect("open patched SQLite");
        let runtime_version: String = store.connection
            .query_row("SELECT sqlite_version()", [], |row| row.get(0))
            .expect("read SQLite version");
        let source_id: String = store.connection
            .query_row("SELECT sqlite_source_id()", [], |row| row.get(0))
            .expect("read SQLite source ID");
        let parsed = sqlite_version_tuple(&runtime_version).expect("parse SQLite runtime version");
        assert!(
            sqlite_version_has_wal_reset_fix(parsed),
            "D1 store runtime must use an upstream-fixed WAL-reset release line: {runtime_version}"
        );
        assert!(!source_id.trim().is_empty(), "SQLite source ID must be reportable");
        println!("PASS: D1 Rust SQLite runtime uses upstream-fixed WAL-reset line: version={runtime_version} source_id={source_id}");
        drop(store);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(format!("{}-wal", path.display()));
        let _ = fs::remove_file(format!("{}-shm", path.display()));
    }

    #[test]
    fn sqlite_version_tuple_and_wal_reset_gate_cover_patched_backports() {
        assert_eq!(sqlite_version_tuple("3.51.3"), Some((3, 51, 3)));
        assert_eq!(sqlite_version_tuple("3.50.12"), Some((3, 50, 12)));
        assert_eq!(sqlite_version_tuple("3.51"), None);
        assert_eq!(sqlite_version_tuple("3.51.3-extra"), None);
        for version in ["3.44.6", "3.50.7", "3.51.3", "3.53.0", "3.53.3", "3.54.1"] {
            assert!(sqlite_version_has_wal_reset_fix(sqlite_version_tuple(version).unwrap()), "expected supported: {version}");
        }
        for version in ["3.44.5", "3.45.0", "3.50.6", "3.51.2", "3.52.0", "3.52.1"] {
            assert!(!sqlite_version_has_wal_reset_fix(sqlite_version_tuple(version).unwrap()), "expected rejected: {version}");
        }
    }

    #[test]
    fn store_consumes_once_and_reconciles_receipt_after_reopen() {
        let now = system_time_unix_seconds().expect("clock");
        let challenge = challenge_for("challenge-success", now);
        let path = temporary_path("success", "sqlite");
        let mut store = SqliteD1Store::open(&path).expect("open new D1 store");
        store.issue_challenge(&challenge).expect("issue challenge");
        let envelope = "d".repeat(64);
        let request = request_for(&challenge, "boot-instance-001", &envelope);
        let receipt = store.consume_once_inner(&request, Some(now)).expect("consume challenge");
        assert!(is_sha256(&receipt));
        assert_eq!(store.committed_receipt_for_challenge(&challenge.challenge_id).unwrap(), Some(receipt.clone()));
        store.integrity_check().expect("integrity check after commit");
        drop(store);

        let mut reopened = SqliteD1Store::open(&path).expect("reopen consumed store");
        assert_eq!(reopened.committed_receipt_for_challenge(&challenge.challenge_id).unwrap(), Some(receipt));
        let replay = request_for(&challenge, "boot-instance-001", &envelope);
        assert!(reopened.consume_once_inner(&replay, Some(now)).is_err(), "replay must fail");
        assert_eq!(db_row_counts(&reopened, &challenge.challenge_id), (1, 1, 1));
        reopened.integrity_check().expect("integrity check after replay");
        println!("PASS: typed Rust D1 store commits once, rejects replay after reopen, and reconciles the original receipt");
        drop(reopened);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(format!("{}-wal", path.display()));
        let _ = fs::remove_file(format!("{}-shm", path.display()));
    }

    #[test]
    fn challenge_expiring_before_commit_rolls_back_receipt_and_consumption() {
        let now = 2_000_000_000;
        let challenge = challenge_for("challenge-expiry-at-commit", now);
        let path = temporary_path("expiry-at-commit", "sqlite");
        let mut store = SqliteD1Store::open(&path).expect("open expiry store");
        store.issue_challenge(&challenge).expect("issue challenge");
        let envelope = "d".repeat(64);
        let request = request_for(&challenge, "boot-instance-commit-expiry", &envelope);
        let mut clock_calls = 0;
        let result = store.consume_once_with_clock(&request, || {
            clock_calls += 1;
            if clock_calls == 1 {
                Ok(now)
            } else {
                Ok(challenge.expires_at_unix_seconds)
            }
        });
        assert!(result.is_err(), "expiry at commit boundary must reject");
        assert_eq!(clock_calls, 2, "store must recheck time at commit");
        assert_eq!(db_row_counts(&store, &challenge.challenge_id), (1, 0, 0));
        store.integrity_check().expect("database remains consistent");
        println!("PASS: D1 challenge expiring during transaction rolls back receipt and consumption");
        drop(store);
        for cleanup in [&path] {
            let _ = fs::remove_file(cleanup);
            let _ = fs::remove_file(format!("{}-wal", cleanup.display()));
            let _ = fs::remove_file(format!("{}-shm", cleanup.display()));
        }
    }

    #[test]
    fn mismatched_binding_expiry_and_unversioned_database_fail_closed() {
        let now = system_time_unix_seconds().expect("clock");
        let challenge = challenge_for("challenge-negative", now);
        let path = temporary_path("negative", "sqlite");
        let mut store = SqliteD1Store::open(&path).expect("open negative store");
        store.issue_challenge(&challenge).expect("issue challenge");
        let envelope = "d".repeat(64);
        let mut wrong = request_for(&challenge, "boot-instance-001", &envelope);
        wrong.target_token = "target-token-B";
        assert!(store.consume_once_inner(&wrong, Some(now)).is_err());
        assert_eq!(db_row_counts(&store, &challenge.challenge_id), (1, 0, 0));

        let late = challenge.expires_at_unix_seconds;
        let request = request_for(&challenge, "boot-instance-001", &envelope);
        assert!(store.consume_once_inner(&request, Some(late)).is_err());
        assert_eq!(db_row_counts(&store, &challenge.challenge_id), (1, 0, 0));
        assert!(
            store.connection.execute(
                "DELETE FROM d1_observation_challenges WHERE challenge_id=?1",
                params![challenge.challenge_id]
            ).is_err(),
            "challenge issuance records must be append-only"
        );
        assert_eq!(db_row_counts(&store, &challenge.challenge_id), (1, 0, 0));
        store.integrity_check().expect("negative controls preserve database");
        println!("PASS: typed Rust D1 store rejects wrong target, expiry, schema adoption, and challenge deletion without mutation");
        drop(store);

        let unknown_path = temporary_path("unknown-schema", "sqlite");
        let unknown = Connection::open(&unknown_path).expect("create unversioned DB");
        unknown.execute_batch("CREATE TABLE unrelated(value TEXT);").unwrap();
        drop(unknown);
        assert!(SqliteD1Store::open(&unknown_path).is_err(), "must not adopt unversioned tables");
        for cleanup in [&path, &unknown_path] {
            let _ = fs::remove_file(cleanup);
            let _ = fs::remove_file(format!("{}-wal", cleanup.display()));
            let _ = fs::remove_file(format!("{}-shm", cleanup.display()));
        }
    }

    #[test]
    fn altered_trigger_sql_fails_closed_even_when_name_is_preserved() {
        let path = temporary_path("trigger-tamper", "sqlite");
        let store = SqliteD1Store::open(&path).expect("initialize canonical D1 schema");
        drop(store);

        let attacker = Connection::open(&path).expect("open schema-tamper fixture");
        attacker
            .execute_batch(
                "DROP TRIGGER d1_challenge_cannot_be_deleted;
                 CREATE TRIGGER d1_challenge_cannot_be_deleted
                 BEFORE DELETE ON d1_observation_challenges
                 BEGIN SELECT 1; END;",
            )
            .expect("replace trigger with same-name non-enforcing trigger");
        drop(attacker);

        assert!(
            SqliteD1Store::open(&path).is_err(),
            "same-name trigger with altered SQL must not be accepted"
        );
        println!("PASS: D1 store rejects altered trigger SQL even when trusted names are preserved");
        for cleanup in [&path] {
            let _ = fs::remove_file(cleanup);
            let _ = fs::remove_file(format!("{}-wal", cleanup.display()));
            let _ = fs::remove_file(format!("{}-shm", cleanup.display()));
        }
    }

    #[test]
    fn sixty_four_independent_connections_allow_exactly_one_consumption() {
        let now = system_time_unix_seconds().expect("clock");
        let challenge = challenge_for("challenge-contention", now);
        let path = temporary_path("race", "sqlite");
        let mut seed = SqliteD1Store::open(&path).expect("open race DB");
        seed.issue_challenge(&challenge).expect("issue challenge");
        drop(seed);

        let barrier = Arc::new(Barrier::new(64));
        let mut workers = Vec::with_capacity(64);
        for _ in 0..64 {
            let path = path.clone();
            let challenge = challenge.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                let mut store = SqliteD1Store::open(&path).expect("open contender connection");
                let envelope = "e".repeat(64);
                let request = request_for(&challenge, "boot-instance-race", &envelope);
                barrier.wait();
                store.consume_once_inner(&request, Some(now)).is_ok()
            }));
        }
        let successes = workers
            .into_iter()
            .map(|worker| worker.join().expect("join worker"))
            .filter(|success| *success)
            .count();
        let reopened = SqliteD1Store::open(&path).expect("reopen contention DB");
        assert_eq!(successes, 1);
        assert_eq!(db_row_counts(&reopened, &challenge.challenge_id), (1, 1, 1));
        reopened.integrity_check().expect("integrity after race");
        println!("PASS: exactly one typed Rust D1 consumption across 64 independent SQLite connections");
        drop(reopened);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(format!("{}-wal", path.display()));
        let _ = fs::remove_file(format!("{}-shm", path.display()));
    }

    #[test]
    fn sqlite_sigkill_worker_entry() {
        let Ok(database_path) = std::env::var("SRQ_D1_SQLITE_WORKER_DB") else {
            return;
        };
        let point = std::env::var("SRQ_D1_SQLITE_KILL_POINT").expect("kill point");
        let issued = std::env::var("SRQ_D1_SQLITE_ISSUED_AT")
            .expect("issued_at").parse().expect("issued timestamp");
        let expiry = std::env::var("SRQ_D1_SQLITE_EXPIRES_AT")
            .expect("expires_at").parse().expect("expiry timestamp");
        let challenge = IssuedChallenge {
            issued_at_unix_seconds: issued,
            expires_at_unix_seconds: expiry,
            ..challenge_for("challenge-kill", system_time_unix_seconds().expect("clock"))
        };
        let mut store = SqliteD1Store::open(database_path).expect("open worker DB");
        let envelope = "f".repeat(64);
        let request = request_for(&challenge, "boot-instance-kill", &envelope);
        let _ = point;
        // This worker tests the SQLite transaction boundaries themselves. The
        // production trait boundary only accepts a verifier-issued capability;
        // verify_and_consume is tested separately in the verifier crate.
        let _ = store
            .consume_once_inner(&request, None)
            .expect("consume should reach injected kill point");
        panic!("worker crossed its configured kill point unexpectedly");
    }

    #[test]
    fn sqlite_adapter_survives_actual_process_kill_at_commit_boundaries() {
        const POINTS: [(&str, bool); 3] = [
            ("after_receipt_insert", false),
            ("after_challenge_update_before_commit", false),
            ("after_commit_before_ack", true),
        ];
        let executable = std::env::current_exe().expect("test executable");
        for (point, should_be_committed) in POINTS {
            for trial in 0..10 {
                let now = system_time_unix_seconds().expect("clock");
                let challenge = challenge_for("challenge-kill", now);
                let path = temporary_path(&format!("kill-{point}-{trial}"), "sqlite");
                let marker = temporary_path(&format!("mark-{point}-{trial}"), "ready");
                let mut seed = SqliteD1Store::open(&path).expect("open test DB");
                seed.issue_challenge(&challenge).expect("issue kill challenge");
                drop(seed);

                let mut child = Command::new(&executable)
                    .args(["--exact", "tests::sqlite_sigkill_worker_entry", "--nocapture"])
                    .env("SRQ_D1_SQLITE_WORKER_DB", &path)
                    .env("SRQ_D1_SQLITE_KILL_POINT", point)
                    .env("SRQ_D1_SQLITE_KILL_MARKER", &marker)
                    .env("SRQ_D1_SQLITE_ISSUED_AT", challenge.issued_at_unix_seconds.to_string())
                    .env("SRQ_D1_SQLITE_EXPIRES_AT", challenge.expires_at_unix_seconds.to_string())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .expect("spawn SQLite transaction worker");

                let deadline = Instant::now() + Duration::from_secs(10);
                while !marker.exists() {
                    if let Some(status) = child.try_wait().expect("inspect worker process") {
                        panic!("worker exited before {point}: {status}");
                    }
                    assert!(Instant::now() < deadline, "worker did not reach {point}");
                    thread::sleep(Duration::from_millis(5));
                }
                let child_pid = child.id().to_string();
                let kill_status = Command::new("kill")
                    .args(["-KILL", child_pid.as_str()])
                    .status()
                    .expect("invoke Linux kill utility");
                assert!(kill_status.success(), "SIGKILL failed at {point}");
                let status = child.wait().expect("wait killed child");
                assert!(!status.success(), "worker should not return normally at {point}");

                let reopened = SqliteD1Store::open(&path).expect("recover store after SIGKILL");
                reopened.integrity_check().expect("integrity after SIGKILL");
                let receipt = reopened
                    .committed_receipt_for_challenge(&challenge.challenge_id)
                    .expect("read post-kill receipt pointer");
                if should_be_committed {
                    assert!(receipt.is_some(), "post-COMMIT kill must preserve committed receipt");
                    assert_eq!(db_row_counts(&reopened, &challenge.challenge_id), (1, 1, 1));
                } else {
                    assert_eq!(receipt, None, "pre-COMMIT kill must roll back receipt pointer");
                    assert_eq!(db_row_counts(&reopened, &challenge.challenge_id), (1, 0, 0));
                }
                println!("PASS: Rust SQLite SIGKILL at {point} trial {trial} recovered the expected complete state");
                drop(reopened);
                let _ = fs::remove_file(&path);
                let _ = fs::remove_file(format!("{}-wal", path.display()));
                let _ = fs::remove_file(format!("{}-shm", path.display()));
                let _ = fs::remove_file(&marker);
            }
        }
        println!("PASS: Rust SQLite adapter survived 30 actual SIGKILL trials across three transaction boundaries");
    }
}
