//! Durable, cross-process replay ledger for verifier-issued freshness nonces.
//!
//! SQLite-backed uniqueness and transaction durability do not authenticate the
//! caller, evidence producer, clock, or database host. Callers must insert only
//! after freshness and conformance checks pass, and fail closed if insertion
//! fails. Atomicity with external actuation belongs to the promotion protocol.

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, Error as SqlError, ErrorCode, TransactionBehavior, params};
use serde_json::Value;

const LEDGER_SCHEMA_VERSION: i64 = 1;
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

const CREATE_TABLE: &str = "
CREATE TABLE consumed_challenges (
    nonce_sha256 TEXT PRIMARY KEY NOT NULL CHECK(length(nonce_sha256) = 64),
    challenge_id TEXT NOT NULL,
    consumed_at_unix_ms INTEGER NOT NULL CHECK(consumed_at_unix_ms >= 0),
    receipt_sha256 TEXT NOT NULL CHECK(length(receipt_sha256) = 64),
    policy_generation INTEGER NOT NULL CHECK(policy_generation > 0),
    target_token TEXT NOT NULL
);
";

/// Metadata recorded atomically when a freshness nonce is consumed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayConsumption<'a> {
    /// Original verifier-generated nonce; only its SHA-256 is stored.
    pub nonce: &'a str,
    /// Challenge identifier bound into the freshness envelope.
    pub challenge_id: &'a str,
    /// Verifier-clock time, not a timestamp claimed by the receipt.
    pub consumed_at_unix_ms: u64,
    /// SHA-256 of the exact receipt bytes that passed conformance verification.
    pub receipt_sha256: &'a str,
    /// Monotonic policy generation supplied by the verifier/control plane.
    pub policy_generation: u64,
    /// Stable, non-secret target token used by the challenge.
    pub target_token: &'a str,
}

/// SQLite-backed nonce-consumption ledger.
///
/// A database path must reside in a trusted, permission-restricted directory on
/// durable storage. SQLite uses WAL and FULL synchronous mode. Methods open a
/// connection per operation, allowing independent instances/processes to
/// serialize through SQLite transactions and the nonce primary-key constraint.
#[derive(Clone, Debug)]
pub struct SqliteReplayLedger {
    path: PathBuf,
}

impl SqliteReplayLedger {
    /// Open or initialize a durable ledger at the supplied path.
    ///
    /// On Unix, the database file is created with mode 0600 and existing files
    /// with group/other permissions are rejected. Symlink database paths are
    /// rejected. Parent-directory trust and filesystem durability remain the
    /// caller's responsibility.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if path.as_os_str().is_empty() || path.to_string_lossy() == ":memory:" {
            return Err(
                "replay ledger requires a durable file path, not an in-memory database".to_owned(),
            );
        }

        ensure_private_database_file(&path)?;
        let connection = Self::connect(&path)?;
        connection
            .execute_batch("PRAGMA journal_mode = WAL;")
            .map_err(|error| format!("cannot enable SQLite WAL mode: {error}"))?;
        connection
            .execute_batch(CREATE_TABLE)
            .or_else(|error| {
                // An existing table is accepted only if schema version and shape
                // checks below prove it matches this contract.
                if table_exists(&connection)? {
                    Ok(())
                } else {
                    Err(error)
                }
            })
            .map_err(|error| format!("cannot initialize replay ledger schema: {error}"))?;

        let user_version: i64 = connection
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .map_err(|error| format!("cannot read replay ledger schema version: {error}"))?;
        if user_version == 0 {
            connection
                .execute_batch(&format!("PRAGMA user_version = {LEDGER_SCHEMA_VERSION};"))
                .map_err(|error| format!("cannot set replay ledger schema version: {error}"))?;
        } else if user_version != LEDGER_SCHEMA_VERSION {
            return Err(format!(
                "unsupported replay ledger schema version {user_version}; expected {LEDGER_SCHEMA_VERSION}"
            ));
        }
        verify_schema(&connection)?;
        Ok(Self { path })
    }

    /// Atomically consume a nonce if it has never been recorded.
    ///
    /// Returns an error for duplicate nonces, malformed input, schema/version
    /// mismatch, filesystem failure, SQLite lock timeout, or storage error. A
    /// caller must fail closed and must not promote a policy when this returns an
    /// error. The nonce insert commits before success is returned.
    pub fn consume(&self, event: &ReplayConsumption<'_>) -> Result<(), String> {
        validate_event(event)?;
        let nonce_sha256 = super::sha256_hex(event.nonce.as_bytes());
        let consumed_at = i64::try_from(event.consumed_at_unix_ms)
            .map_err(|error| format!("verifier timestamp exceeds SQLite integer range: {error}"))?;
        let generation = i64::try_from(event.policy_generation)
            .map_err(|error| format!("policy generation exceeds SQLite integer range: {error}"))?;
        let mut connection = Self::connect(&self.path)?;
        verify_schema(&connection)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("cannot begin replay ledger transaction: {error}"))?;
        let inserted = transaction.execute(
            "INSERT INTO consumed_challenges
                (nonce_sha256, challenge_id, consumed_at_unix_ms, receipt_sha256, policy_generation, target_token)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
            params![
                nonce_sha256,
                event.challenge_id,
                consumed_at,
                event.receipt_sha256,
                generation,
                event.target_token
            ],
        );
        match inserted {
            Ok(1) => {}
            Ok(count) => {
                return Err(format!(
                    "replay ledger inserted an unexpected row count: {count}"
                ));
            }
            Err(SqlError::SqliteFailure(code, _))
                if code.code == ErrorCode::ConstraintViolation =>
            {
                return Err("challenge nonce has already been consumed".to_owned());
            }
            Err(error) => return Err(format!("cannot persist consumed challenge: {error}")),
        }
        transaction
            .commit()
            .map_err(|error| format!("cannot commit consumed challenge: {error}"))?;
        Ok(())
    }

    /// Check whether a nonce is already in the ledger.
    ///
    /// Diagnostic only: do not implement check-then-act with this method.
    /// The consume method performs the uniqueness check and insert atomically.
    pub fn contains_nonce(&self, nonce: &str) -> Result<bool, String> {
        if !is_lower_hex_digest(nonce) {
            return Err("nonce must be 32 cryptographically random bytes encoded as 64 lowercase hex characters".to_owned());
        }
        let nonce_sha256 = super::sha256_hex(nonce.as_bytes());
        let connection = Self::connect(&self.path)?;
        verify_schema(&connection)?;
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM consumed_challenges WHERE nonce_sha256 = ?1);",
                [nonce_sha256],
                |row| row.get(0),
            )
            .map_err(|error| format!("cannot query replay ledger: {error}"))
    }

    fn connect(path: &Path) -> Result<Connection, String> {
        let connection = Connection::open(path)
            .map_err(|error| format!("cannot open replay ledger {}: {error}", path.display()))?;
        connection
            .busy_timeout(BUSY_TIMEOUT)
            .map_err(|error| format!("cannot set replay ledger busy timeout: {error}"))?;
        connection
            .execute_batch("PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;")
            .map_err(|error| format!("cannot configure replay ledger durability: {error}"))?;
        Ok(connection)
    }
}

/// Verify challenge freshness and ordinary conformance, then consume the nonce
/// in the durable ledger before returning an approval-eligible result.
///
/// This helper refuses promotion when either verification or the persistent
/// insert fails. It does not make the external actuation itself transactional;
/// callers must not promote or apply the policy before this function succeeds.
pub fn verify_and_consume_durable(
    ledger: &SqliteReplayLedger,
    challenge_source: &str,
    envelope_source: &str,
    corpus_schema_source: &str,
    corpus_source: &str,
    receipt_source: &str,
    verifier_now_unix_ms: u64,
) -> Result<Value, Vec<String>> {
    let mut one_shot_guard = crate::freshness::ChallengeReplayGuard::new();
    let mut report = one_shot_guard.verify_and_consume(
        challenge_source,
        envelope_source,
        corpus_schema_source,
        corpus_source,
        receipt_source,
        verifier_now_unix_ms,
    )?;

    let challenge: Value = serde_json::from_str(challenge_source)
        .map_err(|error| vec![format!("cannot parse validated challenge: {error}")])?;
    let receipt: Value = serde_json::from_str(receipt_source)
        .map_err(|error| vec![format!("cannot parse validated receipt: {error}")])?;
    let string_field = |value: &Value, pointer: &str, label: &str| {
        value
            .pointer(pointer)
            .and_then(Value::as_str)
            .ok_or_else(|| vec![format!("{label} must be a string")])
    };
    let integer_field = |value: &Value, pointer: &str, label: &str| {
        value
            .pointer(pointer)
            .and_then(Value::as_u64)
            .ok_or_else(|| vec![format!("{label} must be an unsigned integer")])
    };

    let nonce = string_field(&challenge, "/nonce", "challenge.nonce")?;
    let challenge_id = string_field(&challenge, "/challenge_id", "challenge.challenge_id")?;
    let receipt_hash = super::sha256_hex(receipt_source.as_bytes());
    let policy_generation = integer_field(
        &challenge,
        "/policy_generation",
        "challenge.policy_generation",
    )?;
    let target_token = string_field(&challenge, "/target_token", "challenge.target_token")?;

    ledger
        .consume(&ReplayConsumption {
            nonce,
            challenge_id,
            consumed_at_unix_ms: verifier_now_unix_ms,
            receipt_sha256: &receipt_hash,
            policy_generation,
            target_token,
        })
        .map_err(|error| vec![format!("durable replay rejection: {error}")])?;

    // The parent verifier report was produced by a throwaway in-memory guard;
    // replace that description so callers see the actual persistence guarantee.
    report["replay_guard"] =
        Value::String("sqlite_durable_transactional_nonce_consumed".to_owned());
    report["durable_replay_ledger"] = Value::String("commit_confirmed_before_return".to_owned());
    Ok(report)
}

fn table_exists(connection: &Connection) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type = 'table' AND name = 'consumed_challenges'
        );",
        [],
        |row| row.get(0),
    )
}

fn verify_schema(connection: &Connection) -> Result<(), String> {
    let user_version: i64 = connection
        .query_row("PRAGMA user_version;", [], |row| row.get(0))
        .map_err(|error| format!("cannot read replay ledger schema version: {error}"))?;
    if user_version != LEDGER_SCHEMA_VERSION {
        return Err(format!(
            "replay ledger schema version is {user_version}, expected {LEDGER_SCHEMA_VERSION}"
        ));
    }

    let mut statement = connection
        .prepare("PRAGMA table_info(consumed_challenges);")
        .map_err(|error| format!("cannot inspect replay ledger schema: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(|error| format!("cannot read replay ledger schema: {error}"))?;
    let mut actual = Vec::new();
    for row in rows {
        actual.push(row.map_err(|error| format!("invalid replay ledger schema row: {error}"))?);
    }
    let expected = vec![
        ("nonce_sha256".to_owned(), "TEXT".to_owned(), 1, 1),
        ("challenge_id".to_owned(), "TEXT".to_owned(), 1, 0),
        ("consumed_at_unix_ms".to_owned(), "INTEGER".to_owned(), 1, 0),
        ("receipt_sha256".to_owned(), "TEXT".to_owned(), 1, 0),
        ("policy_generation".to_owned(), "INTEGER".to_owned(), 1, 0),
        ("target_token".to_owned(), "TEXT".to_owned(), 1, 0),
    ];
    if actual != expected {
        return Err(
            "replay ledger table structure or nonce primary key does not match the expected schema"
                .to_owned(),
        );
    }
    Ok(())
}

fn ensure_private_database_file(path: &Path) -> Result<(), String> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err("replay ledger path must not be a symbolic link".to_owned());
        }
        if !metadata.is_file() {
            return Err("replay ledger path exists but is not a regular file".to_owned());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(
                    "replay ledger database must not grant group or other permissions".to_owned(),
                );
            }
        }
        return Ok(());
    }

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
            "cannot create replay ledger file {}: {error}",
            path.display()
        )),
    }
}

fn validate_event(event: &ReplayConsumption<'_>) -> Result<(), String> {
    if !is_lower_hex_digest(event.nonce) {
        return Err("nonce must be 32 cryptographically random bytes encoded as 64 lowercase hex characters".to_owned());
    }
    if !is_identifier(event.challenge_id) {
        return Err(
            "challenge_id must be a non-empty ASCII identifier of at most 80 characters".to_owned(),
        );
    }
    if !is_lower_hex_digest(event.receipt_sha256) {
        return Err("receipt_sha256 must be 64 lowercase hexadecimal characters".to_owned());
    }
    if event.policy_generation == 0 || i64::try_from(event.policy_generation).is_err() {
        return Err("policy_generation must be in the positive SQLite integer range".to_owned());
    }
    if i64::try_from(event.consumed_at_unix_ms).is_err() {
        return Err("consumed_at_unix_ms exceeds the SQLite integer range".to_owned());
    }
    if !event.target_token.starts_with("target-")
        || event.target_token.len() < 8
        || event.target_token.len() > 55
        || !event
            .target_token
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(
            "target_token must match the target- prefix and lowercase token alphabet".to_owned(),
        );
    }
    Ok(())
}

fn is_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::{ReplayConsumption, SqliteReplayLedger};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    fn database_path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "luminous-edge-replay-ledger-{}-{nanos}-{id}.sqlite",
            std::process::id()
        ))
    }

    fn event(nonce: &'static str) -> ReplayConsumption<'static> {
        ReplayConsumption {
            nonce,
            challenge_id: "challenge-ledger-0001",
            consumed_at_unix_ms: 1_721_234_000_000,
            receipt_sha256: "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            policy_generation: 9,
            target_token: "target-policy-lab",
        }
    }

    fn remove_database(path: &PathBuf) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }

    #[test]
    fn nonce_is_consumed_once_and_survives_reopening_the_database() {
        let path = database_path();
        let nonce = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        {
            let ledger = SqliteReplayLedger::open(&path).expect("open database");
            assert!(!ledger.contains_nonce(nonce).expect("not consumed"));
            ledger.consume(&event(nonce)).expect("first insert");
            assert!(ledger.contains_nonce(nonce).expect("consumed"));
            assert!(
                ledger
                    .consume(&event(nonce))
                    .expect_err("duplicate rejected")
                    .contains("already been consumed")
            );
        }
        {
            let reopened = SqliteReplayLedger::open(&path).expect("reopen database");
            assert!(reopened.contains_nonce(nonce).expect("survives restart"));
            assert!(
                reopened
                    .consume(&event(nonce))
                    .expect_err("duplicate after restart")
                    .contains("already been consumed")
            );
        }
        remove_database(&path);
    }

    #[test]
    fn simultaneous_connections_cannot_consume_the_same_nonce_twice() {
        let path = database_path();
        let nonce = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let seed = SqliteReplayLedger::open(&path).expect("initialize database");
        drop(seed);

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let mut threads = Vec::new();
        for _ in 0..2 {
            let path = path.clone();
            let barrier = barrier.clone();
            threads.push(thread::spawn(move || {
                let ledger = SqliteReplayLedger::open(path).expect("open concurrent connection");
                barrier.wait();
                ledger.consume(&event(nonce))
            }));
        }
        let results: Vec<_> = threads
            .into_iter()
            .map(|handle| handle.join().expect("worker thread"))
            .collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
        remove_database(&path);
    }

    #[test]
    fn invalid_or_non_durable_inputs_fail_closed() {
        let path = database_path();
        let ledger = SqliteReplayLedger::open(&path).expect("open database");
        let invalid_nonce = event("not-a-nonce");
        assert!(ledger.consume(&invalid_nonce).is_err());
        assert!(ledger.contains_nonce("not-a-nonce").is_err());

        let memory = SqliteReplayLedger::open(":memory:");
        assert!(memory.is_err());
        drop(ledger);
        remove_database(&path);
    }

    #[test]
    fn existing_database_with_changed_schema_is_rejected() {
        let path = database_path();
        let connection = rusqlite::Connection::open(&path).expect("make bad database");
        connection
            .execute_batch(
                "CREATE TABLE consumed_challenges (
                nonce_sha256 TEXT NOT NULL,
                challenge_id TEXT NOT NULL,
                consumed_at_unix_ms INTEGER NOT NULL,
                receipt_sha256 TEXT NOT NULL,
                policy_generation INTEGER NOT NULL,
                target_token TEXT NOT NULL
            );
            PRAGMA user_version = 1;",
            )
            .expect("create invalid schema");
        drop(connection);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&path).expect("database metadata");
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            std::fs::set_permissions(&path, permissions).expect("restrict test database");
        }

        let open = SqliteReplayLedger::open(&path);
        assert!(open.is_err());
        remove_database(&path);
    }

    #[test]
    fn metadata_is_bound_to_the_consumed_nonce_digest() {
        let path = database_path();
        let nonce = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
        let ledger = SqliteReplayLedger::open(&path).expect("open database");
        ledger.consume(&event(nonce)).expect("consume nonce");

        let raw = rusqlite::Connection::open(&path).expect("inspect database");
        let (stored_digest, stored_nonce_length): (String, i64) = raw
            .query_row(
                "SELECT nonce_sha256, length(nonce_sha256) FROM consumed_challenges LIMIT 1;",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("stored nonce digest");
        assert_eq!(stored_digest, crate::sha256_hex(nonce.as_bytes()));
        assert_eq!(stored_nonce_length, 64);
        drop(raw);
        drop(ledger);
        remove_database(&path);
    }
}
