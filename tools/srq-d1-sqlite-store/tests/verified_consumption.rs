use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use coset::{cbor::value::Value, iana, CborSerializable, CoseSign1Builder, HeaderBuilder};
use ed25519_dalek::{Signer, SigningKey};
use srq_d1_sqlite_store::{IssuedChallenge, SqliteD1Store};
use srq_eat_cose_verifier::{
    hex_encode, verify_and_consume, ExpectedChallenge, TrustedKey, PROFILE,
};

const KEY_ID: &[u8] = b"srq-test-key-01";
const NONCE: &[u8] = b"0123456789abcdeffresh-nonce";
const TARGET: &str = "target-token-001";
const AUDIENCE: &str = "srq-verifier-test";
const CHALLENGE_ID: &str = "challenge-e2e-001";
const DECISION_ID: &str = "decision-e2e-001";
const DEPLOYMENT: &str = "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-nixos-system-demo";

fn temporary_database_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "srq-d1-e2e-{}-{nonce}.sqlite",
        std::process::id()
    ))
}

fn signed_token(signing_key: &SigningKey, now: u64, expires: u64) -> Vec<u8> {
    let claims = vec![
        (Value::Integer(10.into()), Value::Bytes(NONCE.to_vec())),
        (Value::Integer(2.into()), Value::Text(TARGET.into())),
        (Value::Integer(3.into()), Value::Text(AUDIENCE.into())),
        (Value::Integer(4.into()), Value::Integer(expires.into())),
        (Value::Integer(6.into()), Value::Integer((now - 10).into())),
        (Value::Text("srq_profile".into()), Value::Text(PROFILE.into())),
        (Value::Text("srq_challenge_id".into()), Value::Text(CHALLENGE_ID.into())),
        (Value::Text("srq_decision_id".into()), Value::Text(DECISION_ID.into())),
        (Value::Text("srq_target_token".into()), Value::Text(TARGET.into())),
        (Value::Text("srq_policy_generation".into()), Value::Integer(42_u64.into())),
        (Value::Text("srq_approved_artifact_sha256".into()), Value::Text("a".repeat(64))),
        (Value::Text("srq_expected_effective_policy_sha256".into()), Value::Text("b".repeat(64))),
        (Value::Text("srq_observed_effective_policy_sha256".into()), Value::Text("b".repeat(64))),
        (Value::Text("srq_corpus_sha256".into()), Value::Text("c".repeat(64))),
        (Value::Text("srq_boot_instance_id".into()), Value::Text("boot-instance-e2e".into())),
        (Value::Text("srq_expected_deployment_id".into()), Value::Text(DEPLOYMENT.into())),
        (Value::Text("srq_booted_deployment_id".into()), Value::Text(DEPLOYMENT.into())),
        (Value::Text("srq_current_userspace_deployment_id".into()), Value::Text(DEPLOYMENT.into())),
        (Value::Text("srq_observation_status".into()), Value::Text("consistent".into())),
        (Value::Text("srq_assurance_tier".into()), Value::Text("D1-software-key".into())),
        (Value::Text("srq_measurement_profile".into()), Value::Null),
        (Value::Text("srq_measurement_digest".into()), Value::Null),
    ];
    let mut payload = Vec::new();
    coset::cbor::ser::into_writer(&Value::Map(claims), &mut payload)
        .expect("serialize EAT claims");

    CoseSign1Builder::new()
        .protected(
            HeaderBuilder::new()
                .algorithm(iana::Algorithm::Ed25519)
                .key_id(KEY_ID.to_vec())
                .build(),
        )
        .payload(payload)
        .create_signature(&[], |signed| signing_key.sign(signed).to_bytes().to_vec())
        .build()
        .to_vec()
        .expect("encode signed COSE_Sign1")
}

#[test]
fn signed_cose_is_verified_then_consumed_by_real_sqlite_store_end_to_end() {
    // An external integration test exercises the public API surface: it cannot
    // fabricate VerifiedChallengeConsumption, so success has to pass through
    // verify_and_consume before entering the durable ChallengeConsumer.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let issued = now.saturating_sub(60);
    let expires = now.saturating_add(3_600);
    let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
    let trusted = TrustedKey {
        key_id: KEY_ID.to_vec(),
        public_key: signing_key.verifying_key().to_bytes(),
        target_token: TARGET.into(),
        audience: AUDIENCE.into(),
    };
    let expected = ExpectedChallenge {
        nonce: NONCE.to_vec(),
        challenge_id: CHALLENGE_ID.into(),
        decision_id: DECISION_ID.into(),
        target_token: TARGET.into(),
        audience: AUDIENCE.into(),
        policy_generation: 42,
        approved_artifact_sha256: "a".repeat(64),
        expected_effective_policy_sha256: "b".repeat(64),
        corpus_sha256: "c".repeat(64),
        expected_deployment_id: DEPLOYMENT.into(),
        issued_at_unix_seconds: issued,
        expires_at_unix_seconds: expires,
        max_attester_clock_skew_seconds: 30,
    };
    let issued_challenge = IssuedChallenge {
        challenge_id: CHALLENGE_ID.into(),
        target_token: TARGET.into(),
        audience: AUDIENCE.into(),
        decision_id: DECISION_ID.into(),
        policy_generation: 42,
        approved_artifact_sha256: "a".repeat(64),
        effective_policy_sha256: "b".repeat(64),
        corpus_sha256: "c".repeat(64),
        expected_deployment_id: DEPLOYMENT.into(),
        expected_signer_key_id_hex: hex_encode(KEY_ID),
        nonce: NONCE.to_vec(),
        issued_at_unix_seconds: issued,
        expires_at_unix_seconds: expires,
    };
    let token = signed_token(&signing_key, now, expires);
    let path = temporary_database_path();

    let mut store = SqliteD1Store::open(&path).expect("open canonical SQLite D1 store");
    store.issue_challenge(&issued_challenge).expect("persist target-bound challenge");

    // Bad signature must fail before the durable store consumes the challenge.
    let mut bad_signature = token.clone();
    let last = bad_signature.len() - 1;
    bad_signature[last] ^= 1;
    assert!(verify_and_consume(&bad_signature, &trusted, &expected, now, &mut store).is_err());
    let probe = rusqlite::Connection::open(&path).expect("open read-only inspection connection");
    let receipt_rows: i64 = probe.query_row(
        "SELECT count(*) FROM d1_verified_observation_receipts WHERE challenge_id=?1",
        [CHALLENGE_ID],
        |row| row.get(0),
    ).expect("count receipts after rejected signature");
    let consumed_rows: i64 = probe.query_row(
        "SELECT count(*) FROM d1_observation_challenges WHERE challenge_id=?1 AND consumed_receipt_sha256 IS NOT NULL",
        [CHALLENGE_ID],
        |row| row.get(0),
    ).expect("count challenge consumption after rejected signature");
    assert_eq!((receipt_rows, consumed_rows), (0, 0), "invalid signature must not mutate durable state");
    drop(probe);

    let consumed = verify_and_consume(&token, &trusted, &expected, now, &mut store)
        .expect("verified COSE token must commit through the SQLite challenge consumer");
    let receipt = consumed.durable_receipt_sha256().to_owned();
    assert_eq!(receipt.len(), 64);
    assert!(receipt.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
    assert_eq!(
        store.committed_receipt_for_challenge(CHALLENGE_ID).expect("receipt lookup"),
        Some(receipt.clone())
    );
    store.integrity_check().expect("SQLite consistency after acceptance");
    drop(store);

    let mut reopened = SqliteD1Store::open(&path).expect("reopen real database");
    assert_eq!(
        reopened.committed_receipt_for_challenge(CHALLENGE_ID).expect("reconcile receipt"),
        Some(receipt)
    );
    assert!(
        verify_and_consume(&token, &trusted, &expected, now + 1, &mut reopened).is_err(),
        "a valid but already-consumed signed token must not be accepted a second time"
    );
    reopened.integrity_check().expect("SQLite integrity after replay");
    println!("PASS: invalid COSE signature leaves challenge unused; valid signed COSE EAT is consumed, reopened and replay-rejected through public API");
    drop(reopened);

    for file in [&path] {
        let _ = fs::remove_file(file);
        let _ = fs::remove_file(format!("{}-wal", file.display()));
        let _ = fs::remove_file(format!("{}-shm", file.display()));
    }
}


#[test]
fn refusing_foreign_database_does_not_change_its_journal_mode_or_rows() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "srq-d1-foreign-db-{}-{nonce}.sqlite",
        std::process::id()
    ));

    {
        let connection = rusqlite::Connection::open(&path).expect("create foreign database");
        let mode: String = connection
            .query_row("PRAGMA journal_mode=DELETE", [], |row| row.get(0))
            .expect("set foreign database to DELETE mode");
        assert_eq!(mode.to_ascii_lowercase(), "delete");
        connection.execute_batch(
            "CREATE TABLE foreign_payload(id INTEGER PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO foreign_payload(id, value) VALUES (1, 'must-remain');"
        ).expect("write foreign payload");
    }

    assert!(
        SqliteD1Store::open(&path).is_err(),
        "D1 store must reject a nonempty unowned schema"
    );

    {
        let connection = rusqlite::Connection::open(&path).expect("reopen foreign database");
        let mode: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read foreign database journal mode");
        assert_eq!(
            mode.to_ascii_lowercase(),
            "delete",
            "rejecting the foreign database must not persistently change journal mode"
        );
        let value: String = connection
            .query_row("SELECT value FROM foreign_payload WHERE id=1", [], |row| row.get(0))
            .expect("read original foreign payload");
        assert_eq!(value, "must-remain");
    }

    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(format!("{}-wal", path.display()));
    let _ = fs::remove_file(format!("{}-shm", path.display()));
    println!("PASS: refusing foreign D1 database preserved journal mode and payload");
}
 

#[test]
fn refusing_spoofed_d1_schema_identity_does_not_change_journal_mode() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "srq-d1-spoofed-schema-{}-{nonce}.sqlite",
        std::process::id()
    ));
    {
        let connection = rusqlite::Connection::open(&path).expect("create spoofed database");
        let mode: String = connection
            .query_row("PRAGMA journal_mode=DELETE", [], |row| row.get(0))
            .expect("set spoofed database to DELETE mode");
        assert_eq!(mode.to_ascii_lowercase(), "delete");
        connection.execute_batch(
            "PRAGMA user_version=1;
             PRAGMA application_id=1397903665;
             CREATE TABLE foreign_payload(id INTEGER PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO foreign_payload(id, value) VALUES (1, 'still-here');"
        ).expect("write spoofed schema identity and payload");
    }

    assert!(
        SqliteD1Store::open(&path).is_err(),
        "D1 adapter must compare canonical SQL rather than trusting version/application ID"
    );
    {
        let connection = rusqlite::Connection::open(&path).expect("reopen spoofed database");
        let mode: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("read spoofed database journal mode");
        assert_eq!(mode.to_ascii_lowercase(), "delete");
        let value: String = connection
            .query_row("SELECT value FROM foreign_payload WHERE id=1", [], |row| row.get(0))
            .expect("read spoofed database payload");
        assert_eq!(value, "still-here");
    }

    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(format!("{}-wal", path.display()));
    let _ = fs::remove_file(format!("{}-shm", path.display()));
    println!("PASS: D1 exact schema verification rejects spoofed identity before journal mutation");
}


#[test]
fn refusing_empty_database_with_spoofed_d1_identity_does_not_change_journal_mode() {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
    let path = std::env::temp_dir().join(format!("srq-d1-empty-spoofed-{}-{nonce}.sqlite", std::process::id()));
    {
        let connection = rusqlite::Connection::open(&path).expect("create empty spoofed database");
        let mode: String = connection.query_row("PRAGMA journal_mode=DELETE", [], |row| row.get(0)).expect("set DELETE mode");
        assert_eq!(mode.to_ascii_lowercase(), "delete");
        connection.execute_batch("PRAGMA user_version=1; PRAGMA application_id=1397903665;").expect("write spoofed D1 ownership markers");
    }
    assert!(SqliteD1Store::open(&path).is_err(), "reject versioned empty database before changing journal mode");
    {
        let connection = rusqlite::Connection::open(&path).expect("reopen empty spoofed database");
        let mode: String = connection.query_row("PRAGMA journal_mode", [], |row| row.get(0)).expect("read journal mode");
        assert_eq!(mode.to_ascii_lowercase(), "delete");
        let user_version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0)).expect("read user_version");
        let application_id: i64 = connection.query_row("PRAGMA application_id", [], |row| row.get(0)).expect("read application_id");
        let count: i64 = connection.query_row("SELECT count(*) FROM sqlite_master WHERE type IN ('table','trigger','view') AND name NOT LIKE 'sqlite_%'", [], |row| row.get(0)).expect("count schema objects");
        assert_eq!(user_version, 1);
        assert_eq!(application_id, 1397903665);
        assert_eq!(count, 0);
    }
    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(format!("{}-wal", path.display()));
    let _ = fs::remove_file(format!("{}-shm", path.display()));
    println!("PASS: empty spoofed D1 schema identity was rejected without journal mutation");
}
