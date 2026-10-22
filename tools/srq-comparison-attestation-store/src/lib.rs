//! Durable, single-use appraisal for SRQ in-toto/DSSE comparison envelopes.
//!
//! This crate is intentionally separate from the shell envelope verifier: it
//! re-verifies the signature and closed-world evidence itself, then consumes a
//! verifier-issued challenge and writes the immutable appraisal receipt in one
//! SQLite transaction. It accepts only non-fixture evidence. A valid signature
//! means the configured key signed the payload; it is not a hardware attestation
//! and does not, by itself, qualify any SRQ release gate.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde_json::Value;
use sha2::{Digest, Sha256};
use srq_sqlite_runtime_policy::is_supported_sqlite_version;

const SCHEMA_VERSION: i64 = 1;
const APPLICATION_ID: i64 = 0x5352_4331; // ASCII "SRC1".
const SCHEMA: &str = include_str!("../../../benchmarks/srq-v1/sqlite-comparison-envelope-consumption.sql");
const PAYLOAD_TYPE: &str = "application/vnd.in-toto+json";
const STATEMENT_TYPE: &str = "https://in-toto.io/Statement/v1";
const PREDICATE_TYPE: &str = "https://luminousdynamics.io/attestations/srq-matched-comparison/v1";
const PROFILE_ID: &str = "SRQ-MATCHED-COMPARISON-ATTESTATION-V1";
const CLAIM_CEILING: &str = "artifact_integrity_and_structural_comparability_only";
const MAX_ENVELOPE_BYTES: usize = 16 * 1024 * 1024;
const MAX_RECEIPT_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct IssuedComparisonChallenge {
    pub challenge_id: String,
    /// Lowercase hex encoding of 32 unpredictable bytes, kept by the trusted issuer.
    pub run_challenge_hex: String,
    pub expected_key_id: String,
    /// SHA-256 of the trusted raw 32-byte Ed25519 public key, provisioned out-of-band.
    pub expected_public_key_sha256: String,
    pub baseline_id: String,
    pub expected_srq_source_sha: String,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsumedComparisonEnvelope {
    /// SHA-256 of the complete DSSE envelope bytes accepted by the store.
    pub receipt_sha256: String,
    pub payload_sha256: String,
    pub challenge_id: String,
    pub key_id: String,
    pub baseline_id: String,
    pub srq_source_sha: String,
    pub subject_count: u64,
    pub run_challenge_sha256: String,
    pub consumed_at_unix_seconds: u64,
}

#[derive(Debug)]
struct StoredChallenge {
    run_challenge_sha256: String,
    expected_key_id: String,
    expected_public_key_sha256: String,
    baseline_id: String,
    expected_srq_source_sha: String,
    issued_at_unix_seconds: u64,
    expires_at_unix_seconds: u64,
    consumed_receipt_sha256: Option<String>,
}

pub struct SqliteComparisonAttestationStore {
    connection: Connection,
}

impl SqliteComparisonAttestationStore {
    /// Open a new empty database or an existing store with the exact known schema.
    /// The parent directory must exist. Unsupported/unknown SQLite or schemas fail closed.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let mut connection = Connection::open(path).map_err(|e| format!("open comparison store: {e}"))?;
        let sqlite_version: String = connection.query_row("SELECT sqlite_version()", [], |row| row.get(0))
            .map_err(|e| format!("read SQLite runtime version: {e}"))?;
        if !is_supported_sqlite_version(&sqlite_version) {
            return Err(format!("SQLite {sqlite_version} is outside SRQ's supported fixed-release policy"));
        }
        connection.busy_timeout(std::time::Duration::from_secs(10))
            .map_err(|e| format!("set SQLite busy timeout: {e}"))?;

        let user_version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|e| format!("read comparison schema version: {e}"))?;
        let application_id: i64 = connection.query_row("PRAGMA application_id", [], |row| row.get(0))
            .map_err(|e| format!("read comparison application ID: {e}"))?;
        let object_count: i64 = connection.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type IN ('table','trigger','view') AND name NOT LIKE 'sqlite_%'",
            [], |row| row.get(0)
        ).map_err(|e| format!("inspect comparison schema: {e}"))?;

        let initialize = match user_version {
            0 if application_id == 0 && object_count == 0 => true,
            SCHEMA_VERSION if application_id == APPLICATION_ID && object_count == 8 => {
                verify_schema_inventory(&connection)?;
                false
            }
            _ => return Err(format!(
                "refusing unknown/nonempty comparison schema (application_id={application_id}, user_version={user_version}, objects={object_count})"
            )),
        };

        configure_connection(&connection)?;
        if initialize {
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|e| format!("begin comparison schema initialization: {e}"))?;
            tx.execute_batch(SCHEMA).map_err(|e| format!("create comparison schema: {e}"))?;
            tx.pragma_update(None, "user_version", SCHEMA_VERSION)
                .map_err(|e| format!("set comparison schema version: {e}"))?;
            tx.pragma_update(None, "application_id", APPLICATION_ID)
                .map_err(|e| format!("set comparison application ID: {e}"))?;
            tx.commit().map_err(|e| format!("commit comparison schema initialization: {e}"))?;
            verify_schema_inventory(&connection)?;
        }
        Ok(Self { connection })
    }

    /// Register an issuer-generated challenge. The raw challenge is never persisted;
    /// only its SHA-256, trusted key fingerprint, baseline and source scope are stored.
    pub fn issue_challenge(&mut self, challenge: &IssuedComparisonChallenge) -> Result<(), String> {
        validate_challenge(challenge)?;
        let now = unix_now()?;
        if challenge.issued_at_unix_seconds > now || now >= challenge.expires_at_unix_seconds {
            return Err("comparison challenge is not currently fresh".into());
        }
        let run_challenge_sha256 = sha256_hex(challenge.run_challenge_hex.as_bytes());
        self.connection.execute(
            "INSERT INTO srq_comparison_challenges
             (challenge_id,run_challenge_sha256,expected_key_id,expected_public_key_sha256,
              baseline_id,expected_srq_source_sha,issued_at_unix_seconds,expires_at_unix_seconds,
              consumed_receipt_sha256)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,NULL)",
            params![
                challenge.challenge_id, run_challenge_sha256, challenge.expected_key_id,
                challenge.expected_public_key_sha256, challenge.baseline_id,
                challenge.expected_srq_source_sha, sql_i64(challenge.issued_at_unix_seconds)?,
                sql_i64(challenge.expires_at_unix_seconds)?
            ],
        ).map_err(|e| format!("persist comparison challenge: {e}"))?;
        Ok(())
    }

    /// Verify the envelope and all subject bytes, then atomically consume the external
    /// challenge and persist an immutable receipt. The caller cannot assert "verified"
    /// with a boolean or bypass the signature/subject checks using a caller-built struct.
    pub fn consume_envelope(
        &mut self,
        challenge_id: &str,
        envelope_bytes: &[u8],
        trusted_public_key: &[u8; 32],
        evidence_root: impl AsRef<Path>,
    ) -> Result<ConsumedComparisonEnvelope, String> {
        self.consume_with_clock(challenge_id, envelope_bytes, trusted_public_key, evidence_root.as_ref(), unix_now)
    }

    /// Recover an already committed receipt after the process dies between COMMIT and
    /// acknowledgement. This is read-only reconciliation and never re-accepts an envelope.
    pub fn committed_receipt_for_challenge(
        &self,
        challenge_id: &str,
    ) -> Result<Option<ConsumedComparisonEnvelope>, String> {
        if !valid_identifier(challenge_id) {
            return Err("comparison challenge ID is malformed".into());
        }
        let receipt = self.connection.query_row(
            "SELECT r.receipt_sha256,r.payload_sha256,r.challenge_id,r.key_id,r.baseline_id,
                    r.srq_source_sha,r.subject_count,r.consumed_at_unix_seconds,r.run_challenge_sha256
               FROM srq_comparison_challenges c
               JOIN srq_comparison_envelopes r ON r.receipt_sha256=c.consumed_receipt_sha256
              WHERE c.challenge_id=?1 AND r.challenge_id=c.challenge_id",
            params![challenge_id],
            |row| Ok(ConsumedComparisonEnvelope {
                receipt_sha256: row.get(0)?,
                payload_sha256: row.get(1)?,
                challenge_id: row.get(2)?,
                key_id: row.get(3)?,
                baseline_id: row.get(4)?,
                srq_source_sha: row.get(5)?,
                subject_count: row.get::<_, i64>(6)? as u64,
                consumed_at_unix_seconds: row.get::<_, i64>(7)? as u64,
                run_challenge_sha256: row.get(8)?,
            })
        ).optional().map_err(|e| format!("read committed comparison receipt: {e}"))?;
        Ok(receipt)
    }

    /// Check SQLite structural integrity, foreign keys, and one-to-one challenge/receipt links.
    pub fn integrity_check(&self) -> Result<(), String> {
        let result: String = self.connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|e| format!("SQLite integrity_check: {e}"))?;
        if result != "ok" {
            return Err(format!("SQLite integrity_check returned: {result}"));
        }
        let foreign_key_error: Option<(String, i64, String, i64)> = self.connection.query_row(
            "PRAGMA foreign_key_check",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        ).optional().map_err(|e| format!("SQLite foreign_key_check: {e}"))?;
        if foreign_key_error.is_some() {
            return Err("comparison store has a foreign-key integrity violation".into());
        }
        let dangling: i64 = self.connection.query_row(
            "SELECT count(*) FROM srq_comparison_challenges c
              LEFT JOIN srq_comparison_envelopes r ON r.receipt_sha256=c.consumed_receipt_sha256
             WHERE (c.consumed_receipt_sha256 IS NULL AND EXISTS
                    (SELECT 1 FROM srq_comparison_envelopes x WHERE x.challenge_id=c.challenge_id))
                OR (c.consumed_receipt_sha256 IS NOT NULL AND
                    (r.receipt_sha256 IS NULL OR r.challenge_id<>c.challenge_id))",
            [], |row| row.get(0)
        ).map_err(|e| format!("check challenge/receipt cardinality: {e}"))?;
        if dangling != 0 {
            return Err("comparison challenge/receipt cardinality is inconsistent".into());
        }
        Ok(())
    }

    fn consume_with_clock<F>(
        &mut self,
        challenge_id: &str,
        envelope_bytes: &[u8],
        trusted_public_key: &[u8; 32],
        evidence_root: &Path,
        mut clock: F,
    ) -> Result<ConsumedComparisonEnvelope, String>
    where
        F: FnMut() -> Result<u64, String>,
    {
        if !valid_identifier(challenge_id) {
            return Err("comparison challenge ID is malformed".into());
        }
        if envelope_bytes.len() > MAX_ENVELOPE_BYTES {
            return Err("DSSE envelope exceeds the configured 16 MiB appraisal limit".into());
        }
        let preflight_now = clock()?;
        let stored = load_challenge(&self.connection, challenge_id)?
            .ok_or_else(|| "unknown comparison challenge".to_string())?;
        validate_fresh(&stored, preflight_now)?;
        let public_key_sha256 = sha256_hex(trusted_public_key);
        if public_key_sha256 != stored.expected_public_key_sha256 {
            return Err("trusted public key fingerprint differs from issued challenge".into());
        }

        let (statement, payload_bytes, key_id, subject_count) =
            verify_signed_statement(envelope_bytes, trusted_public_key, &stored, evidence_root)?;
        let baseline_id = json_string(&statement, &["predicate", "baseline_id"])?;
        let srq_source_sha = json_string(&statement, &["predicate", "source", "actual_srq_checkout_sha"])?;
        let envelope_sha256 = sha256_hex(envelope_bytes);
        let payload_sha256 = sha256_hex(&payload_bytes);

        let tx = self.connection.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| format!("begin comparison appraisal transaction: {e}"))?;
        let locked = load_challenge(&tx, challenge_id)?
            .ok_or_else(|| "comparison challenge disappeared before commit".to_string())?;
        validate_fresh(&locked, clock()?)?;
        if locked.consumed_receipt_sha256.is_some() {
            return Err("comparison challenge was consumed by another appraisal".into());
        }
        if locked.run_challenge_sha256 != stored.run_challenge_sha256
            || locked.expected_key_id != stored.expected_key_id
            || locked.expected_public_key_sha256 != stored.expected_public_key_sha256
            || locked.baseline_id != stored.baseline_id
            || locked.expected_srq_source_sha != stored.expected_srq_source_sha
        {
            return Err("comparison challenge bindings changed during appraisal".into());
        }

        tx.execute(
            "INSERT INTO srq_comparison_envelopes
             (receipt_sha256,challenge_id,payload_sha256,key_id,public_key_sha256,
              baseline_id,srq_source_sha,subject_count,consumed_at_unix_seconds,run_challenge_sha256)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                envelope_sha256, challenge_id, payload_sha256, key_id, public_key_sha256,
                baseline_id, srq_source_sha, i64::try_from(subject_count).map_err(|_| "subject count overflows SQLite")?,
                sql_i64(clock()?)?,
                stored.run_challenge_sha256,
            ],
        ).map_err(|e| format!("insert immutable comparison appraisal receipt: {e}"))?;
        let changed = tx.execute(
            "UPDATE srq_comparison_challenges SET consumed_receipt_sha256=?1
              WHERE challenge_id=?2 AND consumed_receipt_sha256 IS NULL",
            params![envelope_sha256, challenge_id],
        ).map_err(|e| format!("consume comparison challenge: {e}"))?;
        if changed != 1 {
            return Err("comparison challenge compare-and-swap did not update exactly one row".into());
        }
        let commit_now = clock()?;
        validate_fresh(&locked, commit_now)?;
        tx.commit().map_err(|e| format!("commit comparison appraisal: {e}"))?;
        Ok(ConsumedComparisonEnvelope {
            receipt_sha256: envelope_sha256,
            payload_sha256,
            challenge_id: challenge_id.to_string(),
            key_id,
            baseline_id,
            srq_source_sha,
            subject_count: subject_count as u64,
            run_challenge_sha256: stored.run_challenge_sha256.clone(),
            consumed_at_unix_seconds: commit_now,
        })
    }
}

fn configure_connection(connection: &Connection) -> Result<(), String> {
    let journal_mode: String = connection.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
        .map_err(|e| format!("enable SQLite WAL: {e}"))?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(format!("SQLite refused WAL mode (reported {journal_mode})"));
    }
    connection.pragma_update(None, "synchronous", "FULL")
        .map_err(|e| format!("set SQLite synchronous=FULL: {e}"))?;
    connection.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| format!("enable SQLite foreign keys: {e}"))?;
    let synchronous: i64 = connection.query_row("PRAGMA synchronous", [], |row| row.get(0))
        .map_err(|e| format!("read SQLite synchronous mode: {e}"))?;
    let foreign_keys: i64 = connection.query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .map_err(|e| format!("read SQLite foreign-key mode: {e}"))?;
    if synchronous != 2 || foreign_keys != 1 {
        return Err("comparison store did not enforce WAL/FULL/foreign-key policy".into());
    }
    Ok(())
}

fn schema_inventory(connection: &Connection) -> Result<Vec<(String, String, String, Option<String>)>, String> {
    let mut statement = connection.prepare(
        "SELECT type,name,tbl_name,sql FROM sqlite_master
          WHERE type IN ('table','trigger','view') AND name NOT LIKE 'sqlite_%'
          ORDER BY type,name"
    ).map_err(|e| format!("prepare schema inventory: {e}"))?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)))
        .map_err(|e| format!("query schema inventory: {e}"))?;
    rows.collect::<Result<Vec<_>,_>>().map_err(|e| format!("read schema inventory: {e}"))
}

fn verify_schema_inventory(connection: &Connection) -> Result<(), String> {
    let expected = Connection::open_in_memory().map_err(|e| format!("open schema reference: {e}"))?;
    expected.execute_batch(SCHEMA).map_err(|e| format!("build schema reference: {e}"))?;
    if schema_inventory(connection)? != schema_inventory(&expected)? {
        return Err("comparison store schema differs from its exact canonical table/trigger inventory".into());
    }
    Ok(())
}

fn load_challenge(connection: &Connection, challenge_id: &str) -> Result<Option<StoredChallenge>, String> {
    connection.query_row(
        "SELECT run_challenge_sha256,expected_key_id,expected_public_key_sha256,
                baseline_id,expected_srq_source_sha,issued_at_unix_seconds,
                expires_at_unix_seconds,consumed_receipt_sha256
           FROM srq_comparison_challenges WHERE challenge_id=?1",
        params![challenge_id],
        |row| {
            let issued: i64 = row.get(5)?;
            let expires: i64 = row.get(6)?;
            Ok(StoredChallenge {
                run_challenge_sha256: row.get(0)?,
                expected_key_id: row.get(1)?,
                expected_public_key_sha256: row.get(2)?,
                baseline_id: row.get(3)?,
                expected_srq_source_sha: row.get(4)?,
                issued_at_unix_seconds: u64::try_from(issued).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(5, issued))?,
                expires_at_unix_seconds: u64::try_from(expires).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(6, expires))?,
                consumed_receipt_sha256: row.get(7)?,
            })
        }
    ).optional().map_err(|e| format!("read comparison challenge: {e}"))
}

fn verify_signed_statement(
    envelope_bytes: &[u8],
    trusted_public_key: &[u8; 32],
    challenge: &StoredChallenge,
    evidence_root: &Path,
) -> Result<(Value, Vec<u8>, String, usize), String> {
    let envelope: Value = serde_json::from_slice(envelope_bytes).map_err(|e| format!("parse DSSE envelope: {e}"))?;
    let object = envelope.as_object().ok_or("DSSE envelope must be an object")?;
    if object.len() != 3 || envelope["payloadType"].as_str() != Some(PAYLOAD_TYPE) {
        return Err("DSSE envelope has unknown fields or unexpected payload type".into());
    }
    let payload_b64 = envelope["payload"].as_str().ok_or("DSSE payload must be a base64 string")?;
    let payload_bytes = decode_base64(payload_b64)?;
    let signatures = envelope["signatures"].as_array().ok_or("DSSE signatures must be an array")?;
    if signatures.len() != 1 {
        return Err("SRQ comparison profile requires exactly one DSSE signature".into());
    }
    let sig_object = signatures[0].as_object().ok_or("DSSE signature must be an object")?;
    if sig_object.len() != 2 {
        return Err("DSSE signature contains unknown fields".into());
    }
    let key_id = signatures[0]["keyid"].as_str().ok_or("DSSE key ID must be a string")?.to_string();
    if key_id != challenge.expected_key_id {
        return Err("DSSE key ID differs from verifier-issued challenge policy".into());
    }
    let signature_bytes = decode_base64(
        signatures[0]["sig"].as_str().ok_or("DSSE signature must be base64")?
    )?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|e| format!("decode Ed25519 signature: {e}"))?;
    let verifying_key = VerifyingKey::from_bytes(trusted_public_key)
        .map_err(|e| format!("decode trusted Ed25519 key: {e}"))?;

    let payload_type_len = PAYLOAD_TYPE.len();
    let mut pae = format!("DSSEv1 {payload_type_len} {PAYLOAD_TYPE} {} ", payload_bytes.len()).into_bytes();
    pae.extend_from_slice(&payload_bytes);
    verifying_key.verify(&pae, &signature).map_err(|_| "DSSE Ed25519 signature did not verify".to_string())?;

    let statement: Value = serde_json::from_slice(&payload_bytes).map_err(|e| format!("parse in-toto statement: {e}"))?;
    if serde_json::to_vec(&statement).map_err(|e| format!("canonicalize in-toto statement: {e}"))? != payload_bytes {
        return Err("in-toto payload must be canonical compact sorted JSON".into());
    }
    validate_statement(&statement, challenge, evidence_root)?;
    let subject_count = statement["subject"].as_array().ok_or("statement subject must be an array")?.len();
    Ok((statement, payload_bytes, key_id, subject_count))
}

fn validate_statement(statement: &Value, challenge: &StoredChallenge, evidence_root: &Path) -> Result<(), String> {
    let root = evidence_root.canonicalize().map_err(|e| format!("canonicalize evidence root: {e}"))?;
    if !root.is_dir() {
        return Err("evidence root must be a directory".into());
    }
    let top = statement.as_object().ok_or("in-toto statement must be an object")?;
    if top.len() != 4
        || statement["_type"].as_str() != Some(STATEMENT_TYPE)
        || statement["predicateType"].as_str() != Some(PREDICATE_TYPE)
    {
        return Err("unexpected in-toto statement type, predicate type or fields".into());
    }
    let predicate = statement["predicate"].as_object().ok_or("comparison predicate must be an object")?;
    if predicate.len() != 15
        || statement["predicate"]["schema_version"].as_u64() != Some(1)
        || statement["predicate"]["profile_id"].as_str() != Some(PROFILE_ID)
        || statement["predicate"]["evidence_status"].as_str() != Some("submitted_for_appraisal")
        || statement["predicate"]["claim_ceiling"].as_str() != Some(CLAIM_CEILING)
    {
        return Err("statement is not a non-fixture SRQ comparison attestation".into());
    }
    let run_challenge = json_string(statement, &["predicate", "run_challenge"])?;
    if !is_hex(run_challenge, 64) || sha256_hex(run_challenge.as_bytes()) != challenge.run_challenge_sha256 {
        return Err("statement challenge differs from the durable verifier-issued challenge".into());
    }
    let baseline_id = json_string(statement, &["predicate", "baseline_id"])?;
    if baseline_id != challenge.baseline_id {
        return Err("statement baseline differs from verifier-issued baseline policy".into());
    }

    let lock_path = json_string(statement, &["predicate", "comparison_lock_path"])?;
    let run_path = json_string(statement, &["predicate", "comparison_run_path"])?;
    validate_relative_path(lock_path)?;
    validate_relative_path(run_path)?;
    if lock_path == run_path {
        return Err("comparison lock and run receipt must be distinct files".into());
    }
    let lock_file = resolve_subject_path(&root, lock_path)?;
    let run_file = resolve_subject_path(&root, run_path)?;
    let lock_bytes = read_bounded_file(&lock_file, MAX_RECEIPT_BYTES, "comparison lock")?;
    let run_bytes = read_bounded_file(&run_file, MAX_RECEIPT_BYTES, "comparison receipt")?;
    let lock_sha = sha256_hex(&lock_bytes);
    let run_sha = sha256_hex(&run_bytes);
    if lock_sha != json_string(statement, &["predicate", "comparison_lock_sha256"])?
        || run_sha != json_string(statement, &["predicate", "comparison_run_sha256"])?
    {
        return Err("predicate lock/run digest does not match the file bytes".into());
    }
    let lock: Value = serde_json::from_slice(&lock_bytes).map_err(|e| format!("parse comparison lock: {e}"))?;
    let run: Value = serde_json::from_slice(&run_bytes).map_err(|e| format!("parse comparison receipt: {e}"))?;
    if lock["fixture"].as_bool() != Some(false)
        || run["fixture"].as_bool() != Some(false)
        || lock["baseline_id"].as_str() != Some(challenge.baseline_id.as_str())
        || lock["srq_source_sha"].as_str() != Some(challenge.expected_srq_source_sha.as_str())
        || run["profile_id"].as_str() != Some("SRQ-MATCHED-COMPARISON-RUN-V1")
        || run["schema_version"].as_u64() != Some(1)
        || run["run_challenge"].as_str() != Some(run_challenge)
        || run["comparison_lock"]["baseline_id"].as_str() != Some(challenge.baseline_id.as_str())
        || run["comparison_lock"]["lock_sha256"].as_str() != Some(lock_sha.as_str())
        || json_string(&run, &["source", "expected_srq_source_sha"])? != challenge.expected_srq_source_sha
        || json_string(&run, &["source", "actual_srq_checkout_sha"])? != challenge.expected_srq_source_sha
    {
        return Err("comparison run and lock identities do not match issued appraisal policy".into());
    }
    if statement["predicate"]["source"] != run["source"]
        || statement["predicate"]["environment"] != run["environment"]
        || statement["predicate"]["pairing"] != run["pairing"]
        || statement["predicate"]["artifacts"] != run["artifacts"]
        || statement["predicate"]["non_claims"] != run["non_claims"]
    {
        return Err("statement predicate metadata disagrees with the hashed comparison receipt".into());
    }

    let artifacts = run["artifacts"].as_object().ok_or("comparison artifacts must be an object")?;
    if artifacts.is_empty() {
        return Err("comparison receipt must declare at least one artifact".into());
    }
    let mut expected = BTreeSet::new();
    expected.insert(lock_path.to_string());
    expected.insert(run_path.to_string());
    let mut artifact_digests = BTreeMap::new();
    for artifact in artifacts.values() {
        let path = artifact["path"].as_str().ok_or("artifact path must be a string")?;
        validate_relative_path(path)?;
        let digest = artifact["sha256"].as_str().ok_or("artifact digest must be a string")?;
        if !is_hex(digest, 64) || path == lock_path || path == run_path || artifact_digests.insert(path.to_string(), digest.to_string()).is_some() {
            return Err("artifact manifest has an invalid digest or colliding/duplicate path".into());
        }
        expected.insert(path.to_string());
    }

    let subjects = statement["subject"].as_array().ok_or("statement subject must be an array")?;
    if subjects.len() != expected.len() {
        return Err("statement subject inventory does not exactly cover declared evidence".into());
    }
    let mut subject_map = BTreeMap::new();
    for subject in subjects {
        let object = subject.as_object().ok_or("subject must be an object")?;
        if object.len() != 2 {
            return Err("subject contains unknown fields".into());
        }
        let name = subject["name"].as_str().ok_or("subject name must be a string")?;
        validate_relative_path(name)?;
        let digest = subject["digest"]["sha256"].as_str().ok_or("subject SHA-256 is missing")?;
        if !is_hex(digest, 64) || subject["digest"].as_object().map(|m| m.len()) != Some(1) {
            return Err("subject digest must contain one lowercase SHA-256".into());
        }
        if subject_map.insert(name.to_string(), digest.to_string()).is_some() {
            return Err("duplicate in-toto subject path".into());
        }
    }
    let actual_names: BTreeSet<String> = subject_map.keys().cloned().collect();
    if actual_names != expected {
        return Err("statement subject inventory does not exactly cover lock, receipt and all artifacts".into());
    }
    for (name, expected_digest) in &subject_map {
        let file = resolve_subject_path(&root, name)?;
        if sha256_file(&file)? != *expected_digest {
            return Err(format!("subject bytes do not match signed digest: {name}"));
        }
        if let Some(manifest_digest) = artifact_digests.get(name) {
            if manifest_digest != expected_digest {
                return Err(format!("signed subject digest disagrees with receipt artifact manifest: {name}"));
            }
        }
    }
    if subject_map.get(lock_path) != Some(&lock_sha) || subject_map.get(run_path) != Some(&run_sha) {
        return Err("lock/run subject digest differs from predicate digest binding".into());
    }
    Ok(())
}

fn validate_challenge(challenge: &IssuedComparisonChallenge) -> Result<(), String> {
    if !valid_identifier(&challenge.challenge_id) {
        return Err("comparison challenge ID is malformed".into());
    }
    if !is_hex(&challenge.run_challenge_hex, 64) {
        return Err("run challenge must be 32 bytes encoded as lowercase hex".into());
    }
    if challenge.expected_key_id.is_empty()
        || challenge.expected_key_id.len() > 121
        || !challenge.expected_key_id.as_bytes()[0].is_ascii_alphanumeric()
        || !challenge.expected_key_id.bytes().all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
    {
        return Err("expected Ed25519 key ID is malformed".into());
    }
    if !is_hex(&challenge.expected_public_key_sha256, 64) {
        return Err("trusted public key fingerprint must be lowercase SHA-256".into());
    }
    if challenge.baseline_id.len() < 2
        || challenge.baseline_id.len() > 81
        || !challenge.baseline_id.as_bytes()[0].is_ascii_lowercase() && !challenge.baseline_id.as_bytes()[0].is_ascii_digit()
        || !challenge.baseline_id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
    {
        return Err("baseline ID is malformed".into());
    }
    if !is_hex(&challenge.expected_srq_source_sha, 40) {
        return Err("expected SRQ source SHA must be a lowercase 40-character Git SHA".into());
    }
    if challenge.expires_at_unix_seconds <= challenge.issued_at_unix_seconds {
        return Err("challenge expiry must be after issuance".into());
    }
    sql_i64(challenge.issued_at_unix_seconds)?;
    sql_i64(challenge.expires_at_unix_seconds)?;
    Ok(())
}

fn validate_fresh(challenge: &StoredChallenge, now: u64) -> Result<(), String> {
    if challenge.consumed_receipt_sha256.is_some() {
        return Err("comparison challenge was already consumed".into());
    }
    if now < challenge.issued_at_unix_seconds || now >= challenge.expires_at_unix_seconds {
        return Err("comparison challenge is outside its valid time window".into());
    }
    Ok(())
}

fn resolve_subject_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    validate_relative_path(relative)?;
    let path = root.join(relative);
    let resolved = path.canonicalize().map_err(|e| format!("resolve subject {relative}: {e}"))?;
    if !resolved.starts_with(root) {
        return Err(format!("subject escapes evidence root: {relative}"));
    }
    if !resolved.is_file() {
        return Err(format!("subject is not a regular file: {relative}"));
    }
    Ok(resolved)
}

fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.len() > 1024
        || !path.as_bytes()[0].is_ascii_alphanumeric()
        || !path.bytes().all(|b| b.is_ascii_alphanumeric() || b"._/-".contains(&b))
        || path.contains("..")
        || path.contains("//")
        || path.ends_with('/')
        || path.split('/').any(|part| part.is_empty() || part == "." || part == "..")
        || Path::new(path).is_absolute()
        || Path::new(path).components().any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!("unsafe relative evidence path: {path}"));
    }
    Ok(())
}

fn json_string<'a>(value: &'a Value, path: &[&str]) -> Result<&'a str, String> {
    let mut current = value;
    for component in path {
        current = current.get(*component).ok_or_else(|| format!("missing JSON field {}", path.join(".")))?;
    }
    current.as_str().ok_or_else(|| format!("JSON field {} must be a string", path.join(".")))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}

fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn sql_i64(value: u64) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| "timestamp exceeds SQLite signed integer range".into())
}

fn read_bounded_file(path: &Path, maximum: u64, label: &str) -> Result<Vec<u8>, String> {
    let metadata = fs::metadata(path).map_err(|e| format!("inspect {label}: {e}"))?;
    if metadata.len() > maximum {
        return Err(format!("{label} exceeds the configured {maximum}-byte limit"));
    }
    fs::read(path).map_err(|e| format!("read {label}: {e}"))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| format!("open subject for hashing: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|e| format!("hash subject bytes: {e}"))?;
        if count == 0 { break; }
        hasher.update(&buffer[..count]);
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    Ok(out)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

fn unix_now() -> Result<u64, String> {
    SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| "system clock precedes Unix epoch".into())
}

fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    if input.is_empty() || input.len() % 4 != 0 {
        return Err("invalid base64 length".into());
    }
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for (index, chunk) in bytes.chunks_exact(4).enumerate() {
        let last = index + 1 == bytes.len() / 4;
        let a = base64_value(chunk[0]).ok_or("invalid base64 character")?;
        let b = base64_value(chunk[1]).ok_or("invalid base64 character")?;
        let pad_c = chunk[2] == b'=';
        let pad_d = chunk[3] == b'=';
        if pad_c && !pad_d || (pad_c || pad_d) && !last {
            return Err("invalid base64 padding".into());
        }
        let c = if pad_c { 0 } else { base64_value(chunk[2]).ok_or("invalid base64 character")? };
        let d = if pad_d { 0 } else { base64_value(chunk[3]).ok_or("invalid base64 character")? };
        if pad_c && (b & 0x0f) != 0 || pad_d && !pad_c && (c & 0x03) != 0 {
            return Err("non-canonical base64 padding bits".into());
        }
        out.push((a << 2) | (b >> 4));
        if !pad_c {
            out.push((b << 4) | (c >> 2));
        }
        if !pad_d {
            out.push((c << 6) | d);
        }
    }
    if encode_base64(&out) != input {
        return Err("base64 encoding is not canonical".into());
    }
    Ok(out)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn encode_base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let a = chunk[0];
        let b = *chunk.get(1).unwrap_or(&0);
        let c = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(a >> 2) as usize] as char);
        out.push(TABLE[(((a & 0x03) << 4) | (b >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 { TABLE[(((b & 0x0f) << 2) | (c >> 6)) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[(c & 0x3f) as usize] as char } else { '=' });
    }
    out
}

fn verify_schema_inventory(connection: &Connection) -> Result<(), String> {
    let expected = Connection::open_in_memory().map_err(|e| format!("open in-memory schema reference: {e}"))?;
    expected.execute_batch(SCHEMA).map_err(|e| format!("create in-memory schema reference: {e}"))?;
    let inventory = |db: &Connection| -> Result<Vec<(String,String,String,Option<String>)>, String> {
        let mut statement = db.prepare(
            "SELECT type,name,tbl_name,sql FROM sqlite_master
              WHERE type IN ('table','trigger','view') AND name NOT LIKE 'sqlite_%'
              ORDER BY type,name"
        ).map_err(|e| format!("prepare schema inventory: {e}"))?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)))
            .map_err(|e| format!("query schema inventory: {e}"))?;
        rows.collect::<Result<Vec<_>,_>>().map_err(|e| format!("read schema inventory: {e}"))
    };
    if inventory(connection)? != inventory(&expected)? {
        return Err("comparison store schema differs from canonical table/trigger inventory".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;
    use std::sync::{Arc, Barrier, atomic::{AtomicU64, Ordering}};
    use std::thread;

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(label: &str) -> Self {
            let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("srq-comparison-store-{label}-{}-{id}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("create test directory");
            Self(path)
        }
        fn path(&self) -> &Path { &self.0 }
    }
    impl Drop for TempDir {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }

    struct Fixture {
        _temp: TempDir,
        root: PathBuf,
        db_path: PathBuf,
        challenge: IssuedComparisonChallenge,
        signing_key: SigningKey,
        public_key: [u8; 32],
        statement: Value,
        envelope: Vec<u8>,
    }

    fn fixture(label: &str) -> Fixture {
        let temp = TempDir::new(label);
        let root = temp.path().join("evidence");
        fs::create_dir_all(&root).unwrap();
        let key = SigningKey::from_bytes(&[7_u8; 32]);
        let public_key = key.verifying_key().to_bytes();
        let challenge_hex = "2a".repeat(32);
        let baseline_id = "fixture-baseline-v1".to_string();
        let source_sha = "a".repeat(40);

        fs::write(root.join("trial-data.json"), br#"{"trial":"one","outcome":"old_complete"}"#).unwrap();
        let trial_sha = sha256_hex(&fs::read(root.join("trial-data.json")).unwrap());
        let lock = json!({
            "fixture": false,
            "baseline_id": baseline_id,
            "srq_source_sha": source_sha
        });
        let lock_bytes = serde_json::to_vec(&lock).unwrap();
        fs::write(root.join("comparison-lock.json"), &lock_bytes).unwrap();
        let lock_sha = sha256_hex(&lock_bytes);
        let artifacts = json!({
            "trial_data": {"path":"trial-data.json","sha256":trial_sha}
        });
        let source = json!({
            "expected_srq_source_sha": source_sha,
            "actual_srq_checkout_sha": source_sha,
            "baseline_source": {}
        });
        let environment = json!({
            "os_image":"ubuntu-24.04-pinned",
            "architecture":"x86_64",
            "kernel":"test-kernel",
            "filesystem":"tmpfs-test",
            "storage_class":"test-only",
            "runner_class":"fixture"
        });
        let pairing = json!({
            "protocol":"paired_fault_trials_v1",
            "minimum_paired_trials":5,
            "order_policy":"randomized_per_pair",
            "order_seed":42
        });
        let non_claims = json!(["Synthetic contract test only.","No runtime safety claim."]);
        let run = json!({
            "schema_version":1,
            "profile_id":"SRQ-MATCHED-COMPARISON-RUN-V1",
            "fixture":false,
            "run_challenge":challenge_hex,
            "comparison_lock":{"baseline_id":baseline_id,"lock_sha256":lock_sha},
            "source":source,
            "environment":environment,
            "pairing":pairing,
            "artifacts":artifacts,
            "non_claims":non_claims
        });
        let run_bytes = serde_json::to_vec(&run).unwrap();
        fs::write(root.join("comparison-run.json"), &run_bytes).unwrap();
        let mut subjects = vec![
            json!({"name":"comparison-lock.json","digest":{"sha256":lock_sha}}),
            json!({"name":"comparison-run.json","digest":{"sha256":sha256_hex(&run_bytes)}}),
            json!({"name":"trial-data.json","digest":{"sha256":trial_sha}})
        ];
        subjects.sort_by(|a,b| a["name"].as_str().cmp(&b["name"].as_str()));
        let statement = json!({
            "_type":STATEMENT_TYPE,
            "subject":subjects,
            "predicateType":PREDICATE_TYPE,
            "predicate":{
                "schema_version":1,
                "profile_id":PROFILE_ID,
                "evidence_status":"submitted_for_appraisal",
                "run_challenge":challenge_hex,
                "comparison_lock_path":"comparison-lock.json",
                "comparison_run_path":"comparison-run.json",
                "comparison_lock_sha256":lock_sha,
                "comparison_run_sha256":sha256_hex(&run_bytes),
                "baseline_id":baseline_id,
                "source":source,
                "environment":environment,
                "pairing":pairing,
                "artifacts":artifacts,
                "non_claims":non_claims,
                "claim_ceiling":CLAIM_CEILING
            }
        });
        let envelope = sign_statement(&key, &statement);
        let challenge = IssuedComparisonChallenge {
            challenge_id: format!("test-challenge-{label}-v1"),
            run_challenge_hex: challenge_hex,
            expected_key_id: "test-attestor-v1".into(),
            expected_public_key_sha256: sha256_hex(&public_key),
            baseline_id,
            expected_srq_source_sha: source_sha,
            issued_at_unix_seconds: unix_now().unwrap().saturating_sub(1),
            expires_at_unix_seconds: unix_now().unwrap() + 600,
        };
        Fixture { _temp: temp, root, db_path: root.parent().unwrap().join("receipts.sqlite"), challenge, signing_key: key, public_key, statement, envelope }
    }

    fn sign_statement(key: &SigningKey, statement: &Value) -> Vec<u8> {
        let payload = serde_json::to_vec(statement).unwrap();
        let mut pae = format!("DSSEv1 {} {} {} ", PAYLOAD_TYPE.len(), PAYLOAD_TYPE, payload.len()).into_bytes();
        pae.extend_from_slice(&payload);
        let signature = key.sign(&pae).to_bytes();
        serde_json::to_vec(&json!({
            "payloadType":PAYLOAD_TYPE,
            "payload":encode_base64(&payload),
            "signatures":[{"keyid":"test-attestor-v1","sig":encode_base64(&signature)}]
        })).unwrap()
    }

    fn opened(f: &Fixture) -> SqliteComparisonAttestationStore {
        let mut store = SqliteComparisonAttestationStore::open(&f.db_path).unwrap();
        store.issue_challenge(&f.challenge).unwrap();
        store
    }

    #[test]
    fn valid_envelope_is_consumed_once_and_receipt_survives_reopen() {
        let f = fixture("valid");
        let mut store = opened(&f);
        let accepted = store.consume_envelope(&f.challenge.challenge_id, &f.envelope, &f.public_key, &f.root).unwrap();
        assert_eq!(accepted.subject_count, 3);
        store.integrity_check().unwrap();
        drop(store);

        let mut reopened = SqliteComparisonAttestationStore::open(&f.db_path).unwrap();
        assert_eq!(reopened.committed_receipt_for_challenge(&f.challenge.challenge_id).unwrap(), Some(accepted.clone()));
        assert!(reopened.consume_envelope(&f.challenge.challenge_id, &f.envelope, &f.public_key, &f.root).is_err());
        reopened.integrity_check().unwrap();
    }

    #[test]
    fn tampered_signature_does_not_consume_challenge() {
        let f = fixture("bad-signature");
        let mut store = opened(&f);
        let mut envelope: Value = serde_json::from_slice(&f.envelope).unwrap();
        let sig = envelope["signatures"][0]["sig"].as_str().unwrap().to_string();
        let replacement = if sig.starts_with('A') { format!("B{}", &sig[1..]) } else { format!("A{}", &sig[1..]) };
        envelope["signatures"][0]["sig"] = Value::String(replacement);
        let bad = serde_json::to_vec(&envelope).unwrap();
        assert!(store.consume_envelope(&f.challenge.challenge_id, &bad, &f.public_key, &f.root).is_err());
        assert_eq!(store.committed_receipt_for_challenge(&f.challenge.challenge_id).unwrap(), None);
    }

    #[test]
    fn signed_statement_cannot_omit_declared_artifact_subject() {
        let f = fixture("missing-subject");
        let mut store = opened(&f);
        let mut statement = f.statement.clone();
        statement["subject"].as_array_mut().unwrap().retain(|s| s["name"] != "trial-data.json");
        let envelope = sign_statement(&f.signing_key, &statement);
        assert!(store.consume_envelope(&f.challenge.challenge_id, &envelope, &f.public_key, &f.root).is_err());
        assert_eq!(store.committed_receipt_for_challenge(&f.challenge.challenge_id).unwrap(), None);
    }

    #[test]
    fn signed_predicate_cannot_substitute_artifact_manifest_digest() {
        let f = fixture("manifest-digest");
        let mut store = opened(&f);
        let mut statement = f.statement.clone();
        statement["predicate"]["artifacts"]["trial_data"]["sha256"] = Value::String("0".repeat(64));
        let envelope = sign_statement(&f.signing_key, &statement);
        assert!(store.consume_envelope(&f.challenge.challenge_id, &envelope, &f.public_key, &f.root).is_err());
        assert_eq!(store.committed_receipt_for_challenge(&f.challenge.challenge_id).unwrap(), None);
    }

    #[test]
    fn mutated_subject_bytes_fail_without_consuming_challenge() {
        let f = fixture("mutated-bytes");
        let mut store = opened(&f);
        fs::write(f.root.join("trial-data.json"), br#"{"trial":"one","outcome":"new_complete"}"#).unwrap();
        assert!(store.consume_envelope(&f.challenge.challenge_id, &f.envelope, &f.public_key, &f.root).is_err());
        assert_eq!(store.committed_receipt_for_challenge(&f.challenge.challenge_id).unwrap(), None);
    }

    #[test]
    fn wrong_public_key_fingerprint_is_rejected() {
        let f = fixture("wrong-key");
        let mut store = opened(&f);
        let wrong = SigningKey::from_bytes(&[8_u8; 32]).verifying_key().to_bytes();
        assert!(store.consume_envelope(&f.challenge.challenge_id, &f.envelope, &wrong, &f.root).is_err());
        assert_eq!(store.committed_receipt_for_challenge(&f.challenge.challenge_id).unwrap(), None);
    }

    #[test]
    fn envelope_challenge_must_match_issuer_record() {
        let mut f = fixture("challenge-binding");
        f.challenge.run_challenge_hex = "2b".repeat(32);
        let mut store = SqliteComparisonAttestationStore::open(&f.db_path).unwrap();
        store.issue_challenge(&f.challenge).unwrap();
        assert!(store.consume_envelope(&f.challenge.challenge_id, &f.envelope, &f.public_key, &f.root).is_err());
        assert_eq!(store.committed_receipt_for_challenge(&f.challenge.challenge_id).unwrap(), None);
    }

    #[test]
    fn fixture_evidence_is_not_appraised_as_real_comparison() {
        let f = fixture("fixture-only");
        let mut store = opened(&f);
        let mut statement = f.statement.clone();
        statement["predicate"]["evidence_status"] = Value::String("fixture_only".into());
        let envelope = sign_statement(&f.signing_key, &statement);
        assert!(store.consume_envelope(&f.challenge.challenge_id, &envelope, &f.public_key, &f.root).is_err());
        assert_eq!(store.committed_receipt_for_challenge(&f.challenge.challenge_id).unwrap(), None);
    }

    #[test]
    fn expiration_at_commit_boundary_rolls_back_receipt_and_challenge() {
        let f = fixture("expires-at-commit");
        let mut store = opened(&f);
        let expires = f.challenge.expires_at_unix_seconds;
        let mut times = vec![expires - 1, expires - 1, expires - 1, expires].into_iter();
        let result = store.consume_with_clock(
            &f.challenge.challenge_id, &f.envelope, &f.public_key, &f.root,
            || Ok(times.next().unwrap())
        );
        assert!(result.is_err());
        assert_eq!(store.committed_receipt_for_challenge(&f.challenge.challenge_id).unwrap(), None);
        store.integrity_check().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_subject_cannot_escape_evidence_root() {
        let f = fixture("symlink-escape");
        let mut store = opened(&f);
        let external = f.root.parent().unwrap().join("outside-secret.json");
        fs::write(&external, br#"{"secret":true}"#).unwrap();
        fs::remove_file(f.root.join("trial-data.json")).unwrap();
        std::os::unix::fs::symlink(&external, f.root.join("trial-data.json")).unwrap();
        assert!(store.consume_envelope(&f.challenge.challenge_id, &f.envelope, &f.public_key, &f.root).is_err());
        assert_eq!(store.committed_receipt_for_challenge(&f.challenge.challenge_id).unwrap(), None);
    }

    #[test]
    fn sixty_four_independent_connections_commit_exactly_one_appraisal() {
        let f = fixture("contention");
        let store = opened(&f);
        drop(store);
        let barrier = Arc::new(Barrier::new(64));
        let mut workers = Vec::new();
        for _ in 0..64 {
            let path = f.db_path.clone();
            let challenge_id = f.challenge.challenge_id.clone();
            let envelope = f.envelope.clone();
            let key = f.public_key;
            let root = f.root.clone();
            let barrier = Arc::clone(&barrier);
            let store = SqliteComparisonAttestationStore::open(&path).unwrap();
            workers.push(thread::spawn(move || {
                barrier.wait();
                let mut store = store;
                store.consume_envelope(&challenge_id, &envelope, &key, &root).is_ok()
            }));
        }
        let winners = workers.into_iter().map(|w| w.join().unwrap()).filter(|won| *won).count();
        assert_eq!(winners, 1);
        let reopened = SqliteComparisonAttestationStore::open(&f.db_path).unwrap();
        assert!(reopened.committed_receipt_for_challenge(&f.challenge.challenge_id).unwrap().is_some());
        reopened.integrity_check().unwrap();
    }

    #[test]
    fn unknown_nonempty_database_is_rejected_without_adoption() {
        let temp = TempDir::new("unknown-schema");
        let path = temp.path().join("unknown.sqlite");
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch("CREATE TABLE unrelated(id INTEGER PRIMARY KEY)").unwrap();
        drop(connection);
        assert!(SqliteComparisonAttestationStore::open(&path).is_err());
        let connection = Connection::open(&path).unwrap();
        let count: i64 = connection.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name='unrelated'", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 1);
    }
}
