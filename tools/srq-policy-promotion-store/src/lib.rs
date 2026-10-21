//! SQLite policy-promotion prototype: verify a signed authorization, then
//! atomically consume its one-use challenge and advance target policy state.
//!
//! This crate is not wired to the deployed updater. Its API accepts a trusted
//! key configured by the caller and only promotes through verify_and_promote();
//! the internal verified authorization cannot be constructed downstream.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use coset::{CborSerializable, CoseSign1, iana};
use ed25519_dalek::{Signature, VerifyingKey};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use srq_eat_cose_verifier::{hex_encode, sha256_hex};

const SCHEMA_VERSION: i64 = 1;
const APPLICATION_ID: i64 = 0x5352_5150; // ASCII "SRQP".
const MIN_SQLITE_VERSION_FOR_WAL: (u64, u64, u64) = (3, 51, 3);
const PROFILE: &str = "srq-policy-promotion-v1";
const MAX_TOKEN_BYTES: usize = 65_536;
const MAX_PAYLOAD_BYTES: usize = 32_768;
const SCHEMA: &str = include_str!("../../../benchmarks/srq-v1/sqlite-policy-promotion-store-v1.sql");

#[derive(Clone, Debug)]
pub struct TrustedPromotionKey {
    pub key_id: Vec<u8>,
    pub public_key: [u8; 32],
    pub target_id: String,
    pub audience: String,
}

/// Trusted issuer input. The raw nonce is hashed before persistence.
#[derive(Clone)]
pub struct IssuedPromotionChallenge {
    pub challenge_id: String,
    pub target_id: String,
    pub audience: String,
    pub decision_id: String,
    pub previous_generation: u64,
    pub next_generation: u64,
    pub previous_approved_artifact_sha256: String,
    pub previous_effective_policy_sha256: String,
    pub approved_artifact_sha256: String,
    pub next_effective_policy_sha256: String,
    pub corpus_sha256: String,
    pub deployment_id: String,
    pub expected_signer_key_id_hex: String,
    pub nonce: Vec<u8>,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

impl std::fmt::Debug for IssuedPromotionChallenge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IssuedPromotionChallenge")
            .field("challenge_id", &self.challenge_id)
            .field("target_id", &self.target_id)
            .field("audience", &self.audience)
            .field("decision_id", &self.decision_id)
            .field("previous_generation", &self.previous_generation)
            .field("next_generation", &self.next_generation)
            .field("previous_approved_artifact_sha256", &self.previous_approved_artifact_sha256)
            .field("approved_artifact_sha256", &self.approved_artifact_sha256)
            .field("next_effective_policy_sha256", &self.next_effective_policy_sha256)
            .field("deployment_id", &self.deployment_id)
            .field("nonce", &"[REDACTED]")
            .field("issued_at_unix_seconds", &self.issued_at_unix_seconds)
            .field("expires_at_unix_seconds", &self.expires_at_unix_seconds)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitialTargetState {
    pub target_id: String,
    pub generation: u64,
    pub approved_artifact_sha256: String,
    pub effective_policy_sha256: String,
    pub corpus_sha256: String,
    pub deployment_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromotedDecision {
    pub decision_id: String,
    pub target_id: String,
    pub challenge_id: String,
    pub previous_generation: u64,
    pub next_generation: u64,
    pub previous_approved_artifact_sha256: String,
    pub approved_artifact_sha256: String,
    pub effective_policy_sha256: String,
    pub corpus_sha256: String,
    pub deployment_id: String,
    pub signer_key_id_hex: String,
    pub envelope_sha256: String,
    pub committed_at_unix_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreError {
    InvalidInput(&'static str),
    InvalidToken(String),
    UnknownChallenge,
    ChallengeConsumed,
    ExpiredChallenge,
    StaleTargetState,
    BindingMismatch(&'static str),
    UnsupportedSqlite(String),
    CorruptStore(String),
    Sqlite(String),
    Clock,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for StoreError {}

#[derive(Debug)]
struct StoredChallenge {
    challenge_id: String,
    target_id: String,
    audience: String,
    decision_id: String,
    previous_generation: i64,
    next_generation: i64,
    previous_approved_artifact_sha256: String,
    previous_effective_policy_sha256: String,
    approved_artifact_sha256: String,
    next_effective_policy_sha256: String,
    corpus_sha256: String,
    deployment_id: String,
    expected_signer_key_id_hex: String,
    nonce_sha256: String,
    issued_at_unix_seconds: i64,
    expires_at_unix_seconds: i64,
    consumed_decision_id: Option<String>,
}

/// Private verified value: downstream crates cannot construct this struct.
#[derive(Debug)]
struct VerifiedAuthorization {
    challenge_id: String,
    target_id: String,
    audience: String,
    decision_id: String,
    previous_generation: u64,
    next_generation: u64,
    previous_approved_artifact_sha256: String,
    previous_effective_policy_sha256: String,
    approved_artifact_sha256: String,
    next_effective_policy_sha256: String,
    corpus_sha256: String,
    deployment_id: String,
    signer_key_id_hex: String,
    nonce_sha256: String,
    issued_at_unix_seconds: u64,
    expires_at_unix_seconds: u64,
    envelope_sha256: String,
}

pub struct SqlitePolicyPromotionStore {
    connection: Connection,
}

impl SqlitePolicyPromotionStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let mut connection = Connection::open(path)
            .map_err(|error| StoreError::Sqlite(format!("open store: {error}")))?;
        connection.busy_timeout(std::time::Duration::from_secs(10))
            .map_err(|error| StoreError::Sqlite(format!("set busy timeout: {error}")))?;

        let version: String = connection.query_row("SELECT sqlite_version()", [], |row| row.get(0))
            .map_err(|error| StoreError::Sqlite(format!("read SQLite version: {error}")))?;
        let parsed = sqlite_version_tuple(&version)
            .ok_or_else(|| StoreError::UnsupportedSqlite(format!("malformed version {version}")))?;
        if parsed < MIN_SQLITE_VERSION_FOR_WAL {
            return Err(StoreError::UnsupportedSqlite(format!(
                "SQLite {version} is below WAL safety floor 3.51.3"
            )));
        }

        let journal_mode: String = connection.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
            .map_err(|error| StoreError::Sqlite(format!("enable WAL: {error}")))?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            return Err(StoreError::UnsupportedSqlite(format!("WAL refused: {journal_mode}")));
        }
        connection.pragma_update(None, "synchronous", "FULL")
            .map_err(|error| StoreError::Sqlite(format!("set synchronous FULL: {error}")))?;
        connection.pragma_update(None, "foreign_keys", "ON")
            .map_err(|error| StoreError::Sqlite(format!("enable foreign keys: {error}")))?;
        let synchronous: i64 = connection.query_row("PRAGMA synchronous", [], |row| row.get(0))
            .map_err(|error| StoreError::Sqlite(format!("read synchronous: {error}")))?;
        let foreign_keys: i64 = connection.query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .map_err(|error| StoreError::Sqlite(format!("read foreign keys: {error}")))?;
        if synchronous != 2 || foreign_keys != 1 {
            return Err(StoreError::UnsupportedSqlite("WAL/FULL/foreign keys are not active".into()));
        }

        let user_version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| StoreError::Sqlite(format!("read schema version: {error}")))?;
        let app_id: i64 = connection.query_row("PRAGMA application_id", [], |row| row.get(0))
            .map_err(|error| StoreError::Sqlite(format!("read application ID: {error}")))?;
        let count: i64 = connection.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type IN ('table','index','trigger','view') AND name NOT LIKE 'sqlite_%'",
            [], |row| row.get(0)
        ).map_err(|error| StoreError::Sqlite(format!("inspect schema: {error}")))?;

        match user_version {
            0 if app_id == 0 && count == 0 => {
                let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| StoreError::Sqlite(format!("begin schema install: {error}")))?;
                let ddl_start = SCHEMA.find("CREATE TABLE IF NOT EXISTS promotion_targets")
                    .ok_or_else(|| StoreError::CorruptStore("embedded schema lacks promotion_targets".into()))?;
                tx.execute_batch(&SCHEMA[ddl_start..])
                    .map_err(|error| StoreError::Sqlite(format!("install schema: {error}")))?;
                tx.execute_batch(&format!("PRAGMA user_version={SCHEMA_VERSION}; PRAGMA application_id={APPLICATION_ID};"))
                    .map_err(|error| StoreError::Sqlite(format!("version schema: {error}")))?;
                tx.commit().map_err(|error| StoreError::Sqlite(format!("commit schema: {error}")))?;
            }
            SCHEMA_VERSION if app_id == APPLICATION_ID => {}
            _ => return Err(StoreError::CorruptStore(format!(
                "unknown/nonempty schema identity (application_id={app_id}, user_version={user_version}, objects={count})"
            ))),
        }
        verify_schema_inventory(&connection)?;
        Ok(Self { connection })
    }

    pub fn initialize_target(&mut self, state: &InitialTargetState) -> Result<(), StoreError> {
        validate_state(state)?;
        let tx = self.connection.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_err("begin target initialization"))?;
        tx.execute(
            "INSERT INTO promotion_targets (target_id,generation,approved_artifact_sha256,effective_policy_sha256,corpus_sha256,deployment_id,last_decision_id) VALUES (?1,?2,?3,?4,?5,?6,NULL)",
            params![state.target_id, sql_i64(state.generation)?, state.approved_artifact_sha256, state.effective_policy_sha256, state.corpus_sha256, state.deployment_id],
        ).map_err(sql_err("initialize target"))?;
        tx.commit().map_err(sql_err("commit target initialization"))
    }

    /// Trusted issuer registers a challenge before it is sent to the attester.
    /// Only a nonce digest is persisted; raw nonce never enters the database.
    pub fn issue_challenge(&mut self, challenge: &IssuedPromotionChallenge) -> Result<(), StoreError> {
        let now = system_time_unix_seconds()?;
        self.issue_challenge_at(challenge, now)
    }

    fn issue_challenge_at(&mut self, challenge: &IssuedPromotionChallenge, now: u64) -> Result<(), StoreError> {
        validate_challenge_shape(challenge)?;
        if challenge.issued_at_unix_seconds > now || now >= challenge.expires_at_unix_seconds {
            return Err(StoreError::InvalidInput("challenge issue/expiry times are not current"));
        }
        let tx = self.connection.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_err("begin challenge issuance"))?;
        let current: Option<(i64, String, String, String, String)> = tx.query_row(
            "SELECT generation,approved_artifact_sha256,effective_policy_sha256,corpus_sha256,deployment_id FROM promotion_targets WHERE target_id=?1",
            params![challenge.target_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?))
        ).optional().map_err(sql_err("read target during challenge issuance"))?;
        let Some((generation, approved, effective, corpus, deployment)) = current else {
            return Err(StoreError::InvalidInput("target is not initialized"));
        };
        if generation != sql_i64(challenge.previous_generation)? ||
           approved != challenge.previous_approved_artifact_sha256 ||
           effective != challenge.previous_effective_policy_sha256 ||
           corpus != challenge.corpus_sha256 ||
           deployment != challenge.deployment_id {
            return Err(StoreError::StaleTargetState);
        }
        tx.execute(
            "INSERT INTO promotion_challenges (challenge_id,target_id,audience,decision_id,previous_generation,next_generation,previous_approved_artifact_sha256,previous_effective_policy_sha256,approved_artifact_sha256,next_effective_policy_sha256,corpus_sha256,deployment_id,expected_signer_key_id_hex,nonce_sha256,issued_at_unix_seconds,expires_at_unix_seconds,consumed_decision_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,NULL)",
            params![challenge.challenge_id,challenge.target_id,challenge.audience,challenge.decision_id,sql_i64(challenge.previous_generation)?,sql_i64(challenge.next_generation)?,challenge.previous_approved_artifact_sha256,challenge.previous_effective_policy_sha256,challenge.approved_artifact_sha256,challenge.next_effective_policy_sha256,challenge.corpus_sha256,challenge.deployment_id,challenge.expected_signer_key_id_hex,sha256_hex(&challenge.nonce),sql_i64(challenge.issued_at_unix_seconds)?,sql_i64(challenge.expires_at_unix_seconds)?]
        ).map_err(sql_err("persist promotion challenge"))?;
        tx.commit().map_err(sql_err("commit challenge issuance"))
    }

    pub fn verify_and_promote(
        &mut self,
        token: &[u8],
        trusted_key: &TrustedPromotionKey,
    ) -> Result<PromotedDecision, StoreError> {
        let now = system_time_unix_seconds()?;
        self.verify_and_promote_at(token, trusted_key, now)
    }

    fn verify_and_promote_at(
        &mut self,
        token: &[u8],
        trusted_key: &TrustedPromotionKey,
        now: u64,
    ) -> Result<PromotedDecision, StoreError> {
        let verified = verify_signed_authorization(token, trusted_key)?;
        let tx = self.connection.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_err("begin promotion transaction"))?;
        let stored = load_challenge(&tx, &verified.challenge_id)?
            .ok_or(StoreError::UnknownChallenge)?;
        if stored.consumed_decision_id.is_some() {
            return Err(StoreError::ChallengeConsumed);
        }
        validate_verified_binding(&verified, &stored, trusted_key, now)?;
        let target: Option<(i64,String,String,String,String)> = tx.query_row(
            "SELECT generation,approved_artifact_sha256,effective_policy_sha256,corpus_sha256,deployment_id FROM promotion_targets WHERE target_id=?1",
            params![verified.target_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?))
        ).optional().map_err(sql_err("load target state"))?;
        let Some((generation,approved,effective,corpus,deployment)) = target else {
            return Err(StoreError::CorruptStore("challenge target missing".into()));
        };
        if generation != sql_i64(verified.previous_generation)? ||
           approved != verified.previous_approved_artifact_sha256 ||
           effective != verified.previous_effective_policy_sha256 ||
           corpus != verified.corpus_sha256 ||
           deployment != verified.deployment_id {
            return Err(StoreError::StaleTargetState);
        }
        let promoted_at = now;
        tx.execute(
            "INSERT INTO promotion_decisions (decision_id,target_id,challenge_id,previous_generation,next_generation,previous_approved_artifact_sha256,previous_effective_policy_sha256,approved_artifact_sha256,next_effective_policy_sha256,corpus_sha256,deployment_id,signer_key_id_hex,envelope_sha256,committed_at_unix_seconds) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![verified.decision_id,verified.target_id,verified.challenge_id,sql_i64(verified.previous_generation)?,sql_i64(verified.next_generation)?,verified.previous_approved_artifact_sha256,verified.previous_effective_policy_sha256,verified.approved_artifact_sha256,verified.next_effective_policy_sha256,verified.corpus_sha256,verified.deployment_id,verified.signer_key_id_hex,verified.envelope_sha256,sql_i64(promoted_at)?]
        ).map_err(sql_err("insert committed decision"))?;
        let changed = tx.execute(
            "UPDATE promotion_targets SET generation=?1,approved_artifact_sha256=?2,effective_policy_sha256=?3,corpus_sha256=?4,deployment_id=?5,last_decision_id=?6 WHERE target_id=?7 AND generation=?8 AND approved_artifact_sha256=?9 AND effective_policy_sha256=?10 AND corpus_sha256=?11 AND deployment_id=?12",
            params![sql_i64(verified.next_generation)?,verified.approved_artifact_sha256,verified.next_effective_policy_sha256,verified.corpus_sha256,verified.deployment_id,verified.decision_id,verified.target_id,sql_i64(verified.previous_generation)?,verified.previous_approved_artifact_sha256,verified.previous_effective_policy_sha256,verified.corpus_sha256,verified.deployment_id]
        ).map_err(sql_err("compare-and-swap target generation"))?;
        if changed != 1 {
            return Err(StoreError::StaleTargetState);
        }
        let consumed = tx.execute(
            "UPDATE promotion_challenges SET consumed_decision_id=?1 WHERE challenge_id=?2 AND target_id=?3 AND consumed_decision_id IS NULL",
            params![verified.decision_id,verified.challenge_id,verified.target_id]
        ).map_err(sql_err("consume challenge"))?;
        if consumed != 1 {
            return Err(StoreError::ChallengeConsumed);
        }
        tx.commit().map_err(sql_err("commit promotion transaction"))?;
        Ok(PromotedDecision {
            decision_id: verified.decision_id,
            target_id: verified.target_id,
            challenge_id: verified.challenge_id,
            previous_generation: verified.previous_generation,
            next_generation: verified.next_generation,
            previous_approved_artifact_sha256: verified.previous_approved_artifact_sha256,
            approved_artifact_sha256: verified.approved_artifact_sha256,
            effective_policy_sha256: verified.next_effective_policy_sha256,
            corpus_sha256: verified.corpus_sha256,
            deployment_id: verified.deployment_id,
            signer_key_id_hex: verified.signer_key_id_hex,
            envelope_sha256: verified.envelope_sha256,
            committed_at_unix_seconds: promoted_at,
        })
    }

    pub fn committed_decision_for_challenge(&self, challenge_id: &str) -> Result<Option<PromotedDecision>, StoreError> {
        let challenge = load_challenge_conn(&self.connection, challenge_id)?
            .ok_or(StoreError::UnknownChallenge)?;
        let Some(decision_id) = challenge.consumed_decision_id else { return Ok(None); };
        let row = self.connection.query_row(
            "SELECT decision_id,target_id,challenge_id,previous_generation,next_generation,approved_artifact_sha256,next_effective_policy_sha256,corpus_sha256,deployment_id,signer_key_id_hex,envelope_sha256,committed_at_unix_seconds FROM promotion_decisions WHERE decision_id=?1 AND challenge_id=?2 AND target_id=?3",
            params![decision_id,challenge_id,challenge.target_id], |row| Ok(PromotedDecision {
                decision_id: row.get(0)?, target_id: row.get(1)?, challenge_id: row.get(2)?,
                previous_generation: row.get::<_,i64>(3)? as u64, next_generation: row.get::<_,i64>(4)? as u64,
                approved_artifact_sha256: row.get(5)?, effective_policy_sha256: row.get(6)?, corpus_sha256: row.get(7)?,
                deployment_id: row.get(8)?, signer_key_id_hex: row.get(9)?, envelope_sha256: row.get(10)?,
                committed_at_unix_seconds: row.get::<_,i64>(11)? as u64
            })
        ).optional().map_err(sql_err("reconcile committed decision"))?;
        let Some(decision)=row else { return Err(StoreError::CorruptStore("challenge pointer has no decision row".into())); };
        Ok(Some(decision))
    }

    pub fn integrity_check(&self) -> Result<(), StoreError> {
        let integrity: String = self.connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(sql_err("run SQLite integrity_check"))?;
        if integrity != "ok" { return Err(StoreError::CorruptStore(integrity)); }
        let mismatches: i64 = self.connection.query_row(
            "SELECT count(*) FROM promotion_targets t LEFT JOIN promotion_decisions d ON d.decision_id=t.last_decision_id LEFT JOIN promotion_challenges c ON c.challenge_id=d.challenge_id WHERE (t.last_decision_id IS NULL AND t.generation>0) OR (t.last_decision_id IS NOT NULL AND (d.decision_id IS NULL OR c.consumed_decision_id!=d.decision_id OR d.target_id!=t.target_id OR d.next_generation!=t.generation))",
            [], |row| row.get(0)
        ).map_err(sql_err("verify promotion invariants"))?;
        if mismatches != 0 { return Err(StoreError::CorruptStore(format!("{mismatches} lifecycle mismatches"))); }
        Ok(())
    }
}

fn verify_schema_inventory(connection: &Connection) -> Result<(), StoreError> {
    let expected: Vec<(&str,&str)> = vec![
        ("table","promotion_challenges"),
        ("table","promotion_decisions"),
        ("table","promotion_targets"),
        ("index","promotion_decisions_target_generation"),
    ];
    let mut stmt=connection.prepare("SELECT type,name FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name")
        .map_err(sql_err("prepare schema inventory"))?;
    let found: Vec<(String,String)> = stmt.query_map([],|row|Ok((row.get(0)?,row.get(1)?)))
        .map_err(sql_err("query schema inventory"))?.collect::<Result<_,_>>()
        .map_err(sql_err("collect schema inventory"))?;
    let expected: Vec<(String,String)> = expected.into_iter().map(|(a,b)|(a.into(),b.into())).collect();
    if found != expected { return Err(StoreError::CorruptStore(format!("schema inventory mismatch: {found:?}"))); }
    Ok(())
}

fn load_challenge(connection: &Connection, id: &str) -> Result<Option<StoredChallenge>, StoreError> {
    load_challenge_conn(connection,id)
}
fn load_challenge_conn(connection: &Connection,id:&str)->Result<Option<StoredChallenge>,StoreError>{
    connection.query_row(
      "SELECT challenge_id,target_id,audience,decision_id,previous_generation,next_generation,previous_effective_policy_sha256,approved_artifact_sha256,next_effective_policy_sha256,corpus_sha256,deployment_id,expected_signer_key_id_hex,nonce_sha256,issued_at_unix_seconds,expires_at_unix_seconds,consumed_decision_id FROM promotion_challenges WHERE challenge_id=?1",
      params![id],|r|Ok(StoredChallenge{
        challenge_id:r.get(0)?,target_id:r.get(1)?,audience:r.get(2)?,decision_id:r.get(3)?,
        previous_generation:r.get(4)?,next_generation:r.get(5)?,previous_effective_policy_sha256:r.get(6)?,
        approved_artifact_sha256:r.get(7)?,next_effective_policy_sha256:r.get(8)?,corpus_sha256:r.get(9)?,
        deployment_id:r.get(10)?,expected_signer_key_id_hex:r.get(11)?,nonce_sha256:r.get(12)?,
        issued_at_unix_seconds:r.get(13)?,expires_at_unix_seconds:r.get(14)?,consumed_decision_id:r.get(15)?
      })
    ).optional().map_err(sql_err("load stored promotion challenge"))
}

fn validate_verified_binding(v:&VerifiedAuthorization,c:&StoredChallenge,key:&TrustedPromotionKey,now:u64)->Result<(),StoreError>{
    if now < c.issued_at_unix_seconds as u64 || now >= c.expires_at_unix_seconds as u64 || v.expires_at_unix_seconds != c.expires_at_unix_seconds as u64 {
        return Err(StoreError::ExpiredChallenge);
    }
    if v.issued_at_unix_seconds < c.issued_at_unix_seconds as u64 || v.issued_at_unix_seconds > now {
        return Err(StoreError::InvalidToken("iat is outside the challenge window".into()));
    }
    if v.challenge_id!=c.challenge_id || v.target_id!=c.target_id || v.audience!=c.audience || v.decision_id!=c.decision_id {
        return Err(StoreError::BindingMismatch("challenge identity"));
    }
    if v.target_id!=key.target_id || v.audience!=key.audience || v.signer_key_id_hex!=c.expected_signer_key_id_hex || v.signer_key_id_hex!=hex_encode(&key.key_id) {
        return Err(StoreError::BindingMismatch("trusted key / verifier scope"));
    }
    if v.previous_generation != c.previous_generation as u64 || v.next_generation != c.next_generation as u64 ||
       v.previous_effective_policy_sha256!=c.previous_effective_policy_sha256 || v.approved_artifact_sha256!=c.approved_artifact_sha256 ||
       v.next_effective_policy_sha256!=c.next_effective_policy_sha256 || v.corpus_sha256!=c.corpus_sha256 || v.deployment_id!=c.deployment_id ||
       v.nonce_sha256!=c.nonce_sha256 || v.issued_at_unix_seconds < c.issued_at_unix_seconds as u64 || v.expires_at_unix_seconds != c.expires_at_unix_seconds as u64 {
        return Err(StoreError::BindingMismatch("signed claims differ from issued challenge"));
    }
    Ok(())
}

fn verify_signed_authorization(token:&[u8],key:&TrustedPromotionKey)->Result<VerifiedAuthorization,StoreError>{
    if token.is_empty()||token.len()>MAX_TOKEN_BYTES {return Err(StoreError::InvalidToken("token size invalid".into()));}
    let sign1=CoseSign1::from_slice(token).map_err(|e|StoreError::InvalidToken(format!("COSE decode: {e}")))?;
    if sign1.protected.header.alg != Some(iana::Algorithm::Ed25519.into()) {return Err(StoreError::InvalidToken("protected alg must be Ed25519".into()));}
    if sign1.protected.header.key_id.as_slice()!=key.key_id.as_slice() {return Err(StoreError::InvalidToken("protected kid mismatch".into()));}
    if !sign1.unprotected.is_empty(){return Err(StoreError::InvalidToken("unprotected headers forbidden".into()));}
    let ph=&sign1.protected.header;
    if !ph.crit.is_empty()||ph.content_type.is_some()||!ph.iv.is_empty()||!ph.partial_iv.is_empty()||!ph.counter_signatures.is_empty()||!ph.rest.is_empty(){return Err(StoreError::InvalidToken("unsupported protected header".into()));}
    let payload=sign1.payload.as_deref().ok_or_else(||StoreError::InvalidToken("detached payload forbidden".into()))?;
    if payload.is_empty()||payload.len()>MAX_PAYLOAD_BYTES{return Err(StoreError::InvalidToken("payload size invalid".into()));}
    let public=VerifyingKey::from_bytes(&key.public_key).map_err(|e|StoreError::InvalidToken(format!("configured key invalid: {e}")))?;
    if public.is_weak(){return Err(StoreError::InvalidToken("weak Ed25519 public key".into()));}
    sign1.verify_signature(&[],|sig,structure|{
        let signature=Signature::from_slice(sig).map_err(|e|e.to_string())?;
        public.verify_strict(structure,&signature).map_err(|e|e.to_string())
    }).map_err(|e|StoreError::InvalidToken(format!("signature rejected: {e}")))?;
    let mut remaining=payload;
    let map: coset::cbor::value::Value=coset::cbor::de::from_reader(&mut remaining).map_err(|e|StoreError::InvalidToken(format!("CBOR decode: {e}")))?;
    if !remaining.is_empty(){return Err(StoreError::InvalidToken("trailing CBOR bytes".into()));}
    let entries=match map{coset::cbor::value::Value::Map(e)=>e,_=>return Err(StoreError::InvalidToken("claims must be a CBOR map".into()))};
    let mut known=std::collections::BTreeMap::<String,coset::cbor::value::Value>::new();
    for (k,v) in entries {
        let name=match k{
            coset::cbor::value::Value::Integer(n) if n==10.into()=>"eat_nonce".to_string(),
            coset::cbor::value::Value::Integer(n) if n==2.into()=>"sub".to_string(),
            coset::cbor::value::Value::Integer(n) if n==3.into()=>"aud".to_string(),
            coset::cbor::value::Value::Integer(n) if n==4.into()=>"exp".to_string(),
            coset::cbor::value::Value::Integer(n) if n==6.into()=>"iat".to_string(),
            coset::cbor::value::Value::Text(s)=>s,
            _=>return Err(StoreError::InvalidToken("unknown key form".into()))
        };
        if known.insert(name.clone(),v).is_some(){return Err(StoreError::InvalidToken(format!("duplicate claim {name}")));}
    }
    const REQUIRED:&[&str]=&["eat_nonce","sub","aud","exp","iat","srq_profile","srq_challenge_id","srq_decision_id","srq_target_id","srq_previous_generation","srq_next_generation","srq_previous_effective_policy_sha256","srq_approved_artifact_sha256","srq_next_effective_policy_sha256","srq_corpus_sha256","srq_deployment_id","srq_verifier_id"];
    if known.len()!=REQUIRED.len()||REQUIRED.iter().any(|name|!known.contains_key(*name)){return Err(StoreError::InvalidToken("claim inventory mismatch".into()));}
    let text=|name:&str|->Result<String,StoreError>{match known.get(name){Some(coset::cbor::value::Value::Text(s))=>Ok(s.clone()),_=>Err(StoreError::InvalidToken(format!("{name} must be text")))}};
    let uint=|name:&str|->Result<u64,StoreError>{match known.get(name){Some(coset::cbor::value::Value::Integer(n))=>u64::try_from(*n).map_err(|_|StoreError::InvalidToken(format!("{name} must be uint"))),_=>Err(StoreError::InvalidToken(format!("{name} must be integer")))}};
    let bytes=|name:&str|->Result<Vec<u8>,StoreError>{match known.get(name){Some(coset::cbor::value::Value::Bytes(b))=>Ok(b.clone()),_=>Err(StoreError::InvalidToken(format!("{name} must be bytes")))}};
    let nonce=bytes("eat_nonce")?;
    if !(16..=64).contains(&nonce.len()){return Err(StoreError::InvalidToken("nonce length invalid".into()));}
    let target_id=text("sub")?;
    let target_claim=text("srq_target_id")?;
    if target_id!=target_claim{return Err(StoreError::BindingMismatch("sub and target claim disagree"));}
    let audience=match known.get("aud"){Some(coset::cbor::value::Value::Text(s))=>s.clone(),_=>return Err(StoreError::InvalidToken("aud must be one text audience".into()))};
    let verifier_id=text("srq_verifier_id")?;
    if text("srq_profile")?!=PROFILE{return Err(StoreError::InvalidToken("unknown profile".into()));}
    if verifier_id!=key.audience{return Err(StoreError::BindingMismatch("verifier id"));}
    let sha=|name:&str|->Result<String,StoreError>{let d=text(name)?;if !is_sha256(&d){return Err(StoreError::InvalidToken(format!("{name} is not lowercase SHA-256")));}Ok(d)};
    let claims=VerifiedAuthorization{
        challenge_id:text("srq_challenge_id")?,target_id,audience,decision_id:text("srq_decision_id")?,
        previous_generation:uint("srq_previous_generation")?,next_generation:uint("srq_next_generation")?,
        previous_effective_policy_sha256:sha("srq_previous_effective_policy_sha256")?,
        approved_artifact_sha256:sha("srq_approved_artifact_sha256")?,
        next_effective_policy_sha256:sha("srq_next_effective_policy_sha256")?,
        corpus_sha256:sha("srq_corpus_sha256")?,
        deployment_id:text("srq_deployment_id")?,
        signer_key_id_hex:hex_encode(&key.key_id),nonce_sha256:sha256_hex(&nonce),
        issued_at_unix_seconds:uint("iat")?,expires_at_unix_seconds:uint("exp")?,
        envelope_sha256:sha256_hex(token),
    };
    if !valid_identifier(&claims.challenge_id)||!valid_identifier(&claims.decision_id)||!valid_identifier(&claims.target_id)||!valid_identifier(&claims.audience)||!valid_identifier(&claims.deployment_id) {
        return Err(StoreError::InvalidToken("identifier syntax invalid".into()));
    }
    if claims.next_generation!=claims.previous_generation.checked_add(1).ok_or(StoreError::InvalidInput("generation overflow"))? {
        return Err(StoreError::InvalidToken("generation transition must be exactly one".into()));
    }
    Ok(claims)
}

fn validate_state(s:&InitialTargetState)->Result<(),StoreError>{
    if !valid_identifier(&s.target_id)||!valid_deployment_id(&s.deployment_id){return Err(StoreError::InvalidInput("target identifier/deployment invalid"));}
    for d in [&s.approved_artifact_sha256,&s.effective_policy_sha256,&s.corpus_sha256]{if !is_sha256(d){return Err(StoreError::InvalidInput("initial target digest invalid"));}}
    sql_i64(s.generation)?;Ok(())
}
fn validate_challenge_shape(c:&IssuedPromotionChallenge)->Result<(),StoreError>{
    if !valid_identifier(&c.challenge_id)||!valid_identifier(&c.target_id)||!valid_identifier(&c.audience)||!valid_identifier(&c.decision_id)||!valid_identifier(&c.deployment_id){return Err(StoreError::InvalidInput("challenge identifier invalid"));}
    if c.nonce.len()<16||c.nonce.len()>64{return Err(StoreError::InvalidInput("nonce must be 16..64 bytes"));}
    if c.expected_signer_key_id_hex.is_empty()||c.expected_signer_key_id_hex.len()>128||c.expected_signer_key_id_hex.bytes().any(|b|!b.is_ascii_hexdigit()||b.is_ascii_uppercase()){return Err(StoreError::InvalidInput("signer key ID must be lowercase hex"));}
    for d in [&c.previous_effective_policy_sha256,&c.approved_artifact_sha256,&c.next_effective_policy_sha256,&c.corpus_sha256]{if !is_sha256(d){return Err(StoreError::InvalidInput("challenge digest invalid"));}}
    if c.next_generation!=c.previous_generation.checked_add(1).ok_or(StoreError::InvalidInput("generation overflow"))?{return Err(StoreError::InvalidInput("generation must increment exactly once"));}
    if c.expires_at_unix_seconds<=c.issued_at_unix_seconds{return Err(StoreError::InvalidInput("challenge expiry must follow issue time"));}
    sql_i64(c.previous_generation)?;sql_i64(c.next_generation)?;sql_i64(c.issued_at_unix_seconds)?;sql_i64(c.expires_at_unix_seconds)?;
    Ok(())
}
fn is_sha256(s:&str)->bool{s.len()==64&&s.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))}
fn valid_identifier(s:&str)->bool{!s.is_empty()&&s.len()<=128&&s.bytes().all(|b|b.is_ascii_alphanumeric()||matches!(b,b'.'|b'_'|b'-'))}
fn valid_deployment_id(s:&str)->bool{
 if is_sha256(s){return true;}
 let Some(rest)=s.strip_prefix("/nix/store/") else{return false;};
 let Some((hash,name))=rest.split_once('-') else{return false;};
 hash.len()==32&&hash.bytes().all(|b|b.is_ascii_digit()||b"abcdfghijklmnpqrsvwxyz".contains(&b))&&!name.is_empty()&&name.bytes().all(|b|b.is_ascii_alphanumeric()||matches!(b,b'.'|b'_'|b'+'|b'-'))
}
fn sqlite_version_tuple(s:&str)->Option<(u64,u64,u64)>{let mut p=s.split('.');let a=p.next()?.parse().ok()?;let b=p.next()?.parse().ok()?;let c=p.next()?.parse().ok()?;if p.next().is_some(){None}else{Some((a,b,c))}}
fn sql_i64(v:u64)->Result<i64,StoreError>{i64::try_from(v).map_err(|_|StoreError::InvalidInput("integer exceeds SQLite signed range"))}
fn system_time_unix_seconds()->Result<u64,StoreError>{SystemTime::now().duration_since(UNIX_EPOCH).map(|d|d.as_secs()).map_err(|_|StoreError::Clock)}
fn sql_err(context:&'static str)->impl FnOnce(rusqlite::Error)->StoreError{move|e|StoreError::Sqlite(format!("{context}: {e}"))}
fn verify_schema_inventory(c:&Connection)->Result<(),StoreError>{
 let mut s=c.prepare("SELECT type,name FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name").map_err(sql_err("prepare schema inventory"))?;
 let got:Vec<(String,String)>=s.query_map([],|r|Ok((r.get(0)?,r.get(1)?))).map_err(sql_err("read schema inventory"))?.collect::<Result<_,_>>().map_err(sql_err("collect schema inventory"))?;
 let want=vec![("index".to_string(),"promotion_decisions_target_generation".to_string()),("table".to_string(),"promotion_challenges".to_string()),("table".to_string(),"promotion_decisions".to_string()),("table".to_string(),"promotion_targets".to_string())];
 if got!=want{return Err(StoreError::CorruptStore(format!("schema inventory mismatch: {got:?}")));}Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use coset::{cbor::value::Value, CoseSign1Builder, HeaderBuilder};
    use ed25519_dalek::{Signer, SigningKey};
    use std::fs;
    use std::path::PathBuf;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    const KEY_ID: &[u8] = b"srq-promotion-key-01";
    const TARGET: &str = "target-test-01";
    const AUDIENCE: &str = "srq-policy-verifier";
    const CHALLENGE_ID: &str = "challenge-test-01";
    const DECISION_ID: &str = "decision-test-01";
    const D0: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const D1: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const D2: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const D3: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const DEPLOYMENT: &str = "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-srq-promoted-system";

    fn temp_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("srq-policy-promotion-{label}-{}-{nonce}.sqlite",std::process::id()))
    }
    fn base_store(path:&Path)->SqlitePolicyPromotionStore {
        let mut store=SqlitePolicyPromotionStore::open(path).expect("open store");
        store.initialize_target(&InitialTargetState{target_id:TARGET.into(),generation:7,approved_artifact_sha256:D0.into(),effective_policy_sha256:D1.into(),corpus_sha256:D3.into(),deployment_id:DEPLOYMENT.into()}).expect("seed target");
        store
    }
    fn issue(store:&mut SqlitePolicyPromotionStore,now:u64)->IssuedPromotionChallenge{
        let challenge=IssuedPromotionChallenge{challenge_id:CHALLENGE_ID.into(),target_id:TARGET.into(),audience:AUDIENCE.into(),decision_id:DECISION_ID.into(),previous_generation:7,next_generation:8,previous_effective_policy_sha256:D1.into(),approved_artifact_sha256:D2.into(),next_effective_policy_sha256:D2.into(),corpus_sha256:D3.into(),deployment_id:DEPLOYMENT.into(),expected_signer_key_id_hex:hex_encode(KEY_ID),nonce:b"0123456789abcdef-policy-nonce".to_vec(),issued_at_unix_seconds:now-1,expires_at_unix_seconds:now+60};
        // Test uses a private test clock, but issuance follows the same trusted issuer checks.
        store.issue_challenge_at(&challenge,now).expect("issue challenge");
        challenge
    }
    fn make_token(challenge:&IssuedPromotionChallenge,now:u64,key:&SigningKey,overrides:&[(&str,Value)])->Vec<u8>{
        let mut claims=vec![
            (Value::Integer(10.into()),Value::Bytes(challenge.nonce.clone())),
            (Value::Integer(2.into()),Value::Text(challenge.target_id.clone())),
            (Value::Integer(3.into()),Value::Text(challenge.audience.clone())),
            (Value::Integer(4.into()),Value::Integer(challenge.expires_at_unix_seconds.into())),
            (Value::Integer(6.into()),Value::Integer(now.into())),
            (Value::Text("srq_profile".into()),Value::Text(PROFILE.into())),
            (Value::Text("srq_challenge_id".into()),Value::Text(challenge.challenge_id.clone())),
            (Value::Text("srq_decision_id".into()),Value::Text(challenge.decision_id.clone())),
            (Value::Text("srq_target_id".into()),Value::Text(challenge.target_id.clone())),
            (Value::Text("srq_previous_generation".into()),Value::Integer(challenge.previous_generation.into())),
            (Value::Text("srq_next_generation".into()),Value::Integer(challenge.next_generation.into())),
            (Value::Text("srq_previous_effective_policy_sha256".into()),Value::Text(challenge.previous_effective_policy_sha256.clone())),
            (Value::Text("srq_approved_artifact_sha256".into()),Value::Text(challenge.approved_artifact_sha256.clone())),
            (Value::Text("srq_next_effective_policy_sha256".into()),Value::Text(challenge.next_effective_policy_sha256.clone())),
            (Value::Text("srq_corpus_sha256".into()),Value::Text(challenge.corpus_sha256.clone())),
            (Value::Text("srq_deployment_id".into()),Value::Text(challenge.deployment_id.clone())),
            (Value::Text("srq_verifier_id".into()),Value::Text(challenge.audience.clone())),
        ];
        for (name,value) in overrides {
            let i=claims.iter().position(|(k,_)|matches!(k,Value::Text(s) if s==name)).expect("override claim exists");
            claims[i].1=value.clone();
        }
        let mut payload=Vec::new();
        coset::cbor::ser::into_writer(&Value::Map(claims),&mut payload).unwrap();
        CoseSign1Builder::new()
            .protected(HeaderBuilder::new().algorithm(iana::Algorithm::Ed25519).key_id(KEY_ID.to_vec()).build())
            .payload(payload)
            .create_signature(&[],|signed|key.sign(signed).to_bytes().to_vec())
            .build().to_vec().unwrap()
    }
    fn trusted(signing:&SigningKey)->TrustedPromotionKey{
        TrustedPromotionKey{key_id:KEY_ID.to_vec(),public_key:signing.verifying_key().to_bytes(),target_id:TARGET.into(),audience:AUDIENCE.into()}
    }
    fn finish(store:&mut SqlitePolicyPromotionStore,path:&Path){store.integrity_check().expect("integrity");drop(store);let _=fs::remove_file(path);let _=fs::remove_file(format!("{}-wal",path.display()));let _=fs::remove_file(format!("{}-shm",path.display()));}

    #[test]
    fn signed_authorization_commits_decision_challenge_and_generation_atomically() {
        let now=2_000_000_000;
        let path=temp_path("commit");
        let mut store=base_store(&path);
        let challenge=issue(&mut store,now);
        let signing=SigningKey::from_bytes(&[7u8;32]);
        let token=make_token(&challenge,now,&signing,&[]);
        let result=store.verify_and_promote_at(&token,&trusted(&signing),now).expect("promote once");
        assert_eq!(result.decision_id,DECISION_ID);
        assert_eq!(result.previous_generation,7);
        assert_eq!(result.next_generation,8);
        assert_eq!(result.signer_key_id_hex,hex_encode(KEY_ID));
        let reopened=SqlitePolicyPromotionStore::open(&path).expect("reopen");
        let durable=reopened.committed_decision_for_challenge(CHALLENGE_ID).unwrap().unwrap();
        assert_eq!(durable,result);
        let gen:i64=reopened.connection.query_row("SELECT generation FROM promotion_targets WHERE target_id=?1",params![TARGET],|r|r.get(0)).unwrap();
        assert_eq!(gen,8);
        assert_eq!(reopened.integrity_check(),Ok(()));
        drop(reopened);let _=fs::remove_file(&path);
    }
    #[test]
    fn invalid_signature_does_not_consume_challenge_or_mutate_target() {
        let now=2_000_000_000;let path=temp_path("bad-sig");let mut store=base_store(&path);let challenge=issue(&mut store,now);
        let signer=SigningKey::from_bytes(&[8u8;32]);let other=SigningKey::from_bytes(&[9u8;32]);
        let token=make_token(&challenge,now,&signer,&[]);
        assert!(matches!(store.verify_and_promote_at(&token,&trusted(&other),now),Err(StoreError::InvalidToken(_))));
        let gen:i64=store.connection.query_row("SELECT generation FROM promotion_targets WHERE target_id=?1",params![TARGET],|r|r.get(0)).unwrap();
        assert_eq!(gen,7);assert!(store.committed_decision_for_challenge(CHALLENGE_ID).unwrap().is_none());
        finish(&mut store,&path);
    }
    #[test]
    fn replay_after_reopen_is_rejected_and_original_decision_is_recoverable() {
        let now=2_000_000_000;let path=temp_path("replay");let mut store=base_store(&path);let challenge=issue(&mut store,now);let signer=SigningKey::from_bytes(&[7u8;32]);let token=make_token(&challenge,now,&signer,&[]);
        let first=store.verify_and_promote_at(&token,&trusted(&signer),now).unwrap();drop(store);
        let mut reopened=SqlitePolicyPromotionStore::open(&path).unwrap();
        assert_eq!(reopened.verify_and_promote_at(&token,&trusted(&signer),now).unwrap_err(),StoreError::ChallengeConsumed);
        assert_eq!(reopened.committed_decision_for_challenge(CHALLENGE_ID).unwrap().unwrap(),first);
        finish(&mut reopened,&path);
    }
    #[test]
    fn expiry_and_wrong_target_policy_generation_digest_are_rejected_without_mutation() {
        let now=2_000_000_000;let path=temp_path("bad-claims");let mut store=base_store(&path);let challenge=issue(&mut store,now);let signer=SigningKey::from_bytes(&[7u8;32]);
        let mut bad_nonce=challenge.clone();bad_nonce.nonce[0]^=1;let token=make_token(&bad_nonce,now,&signer,&[]);
        assert!(store.verify_and_promote_at(&token,&trusted(&signer),now).is_err());
        let token=make_token(&challenge,now,&signer,&[("srq_target_id",Value::Text("target-other".into()))]);
        assert!(store.verify_and_promote_at(&token,&trusted(&signer),now).is_err());
        let token=make_token(&challenge,now,&signer,&[("srq_next_generation",Value::Integer(11.into()))]);
        assert!(store.verify_and_promote_at(&token,&trusted(&signer),now).is_err());
        let token=make_token(&challenge,now,&signer,&[("srq_next_effective_policy_sha256",Value::Text(D0.into()))]);
        assert!(store.verify_and_promote_at(&token,&trusted(&signer),now).is_err());
        assert!(matches!(store.verify_and_promote_at(&make_token(&challenge,now,&signer,&[]),&trusted(&signer),now+60),Err(StoreError::ExpiredChallenge)));
        let gen:i64=store.connection.query_row("SELECT generation FROM promotion_targets WHERE target_id=?1",params![TARGET],|r|r.get(0)).unwrap();
        assert_eq!(gen,7);assert!(store.committed_decision_for_challenge(CHALLENGE_ID).unwrap().is_none());
        finish(&mut store,&path);
    }
    #[test]
    fn one_of_sixty_four_independent_connections_commits() {
        const N:usize=64;
        let now=2_000_000_000;let path=temp_path("race");let mut seeded=base_store(&path);let challenge=issue(&mut seeded,now);drop(seeded);
        let signing=SigningKey::from_bytes(&[7u8;32]);let token=make_token(&challenge,now,&signing,&[]);let trusted=trusted(&signing);
        let gate=std::sync::Arc::new(std::sync::Barrier::new(N));
        let mut handles=Vec::new();
        for _ in 0..N {
            let path=path.clone();let token=token.clone();let trusted=trusted.clone();let gate=gate.clone();
            handles.push(thread::spawn(move||{let mut store=SqlitePolicyPromotionStore::open(&path).unwrap();gate.wait();store.verify_and_promote_at(&token,&trusted,now)}));
        }
        let mut winners=0;let mut rejected=0;
        for h in handles {match h.join().unwrap(){Ok(_)=>winners+=1,Err(StoreError::ChallengeConsumed)|Err(StoreError::StaleTargetState)|Err(StoreError::Sqlite(_))=>rejected+=1,Err(e)=>panic!("unexpected race error {e:?}")}}
        assert_eq!(winners,1);assert_eq!(rejected,63);
        let store=SqlitePolicyPromotionStore::open(&path).unwrap();store.integrity_check().unwrap();
        assert_eq!(store.committed_decision_for_challenge(CHALLENGE_ID).unwrap().unwrap().next_generation,8);
        drop(store);let _=fs::remove_file(&path);
    }
    #[test]
    fn runtime_sqlite_version_gate_rejects_known_wal_reset_bug_range() {
        assert!(sqlite_version_tuple("3.51.2").unwrap()<MIN_SQLITE_VERSION_FOR_WAL);
        assert!(sqlite_version_tuple("3.51.3").unwrap()>=MIN_SQLITE_VERSION_FOR_WAL);
        assert!(sqlite_version_tuple("3.54.1").unwrap()>=MIN_SQLITE_VERSION_FOR_WAL);
        assert_eq!(sqlite_version_tuple("3.51"),None);
    }
}
