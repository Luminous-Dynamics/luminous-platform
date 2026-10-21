//! Strict D1 verifier for the SRQ EAT/COSE boot-observation profile.
//!
//! The verifier authenticates a COSE_Sign1 Ed25519 envelope against an
//! externally configured target key and checks exact challenge-bound claims.
//! It does not provide hardware measurements, TPM attestation or qualification.
//! Its plain verification result is explicitly not replay-safe until a caller
//! consumes the nonce in a durable transaction through ChallengeConsumer.

use coset::{iana, CborSerializable, CoseSign1};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PROFILE: &str = "srq-eat-boot-observation-v1";
pub const MAX_TOKEN_BYTES: usize = 65_536;
pub const MAX_PAYLOAD_BYTES: usize = 32_768;
pub const MIN_NONCE_BYTES: usize = 16;
pub const MAX_NONCE_BYTES: usize = 64;

#[derive(Clone, Debug)]
pub struct TrustedKey {
    pub key_id: Vec<u8>,
    pub public_key: [u8; 32],
    pub target_token: String,
    pub audience: String,
}

#[derive(Clone, Debug)]
pub struct ExpectedChallenge {
    pub nonce: Vec<u8>,
    pub challenge_id: String,
    pub decision_id: String,
    pub target_token: String,
    pub audience: String,
    pub policy_generation: u64,
    pub approved_artifact_sha256: String,
    pub expected_effective_policy_sha256: String,
    pub corpus_sha256: String,
    pub expected_deployment_id: String,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    pub max_attester_clock_skew_seconds: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct VerifiedObservation {
    pub schema: &'static str,
    pub envelope_sha256: String,
    pub key_id_hex: String,
    pub target_token: String,
    pub audience: String,
    pub challenge_id: String,
    pub decision_id: String,
    pub policy_generation: u64,
    pub deployment_id: String,
    pub boot_instance_id: String,
    pub artifact_sha256: String,
    pub effective_policy_sha256: String,
    pub corpus_sha256: String,
    pub assurance_tier: &'static str,
    pub challenge_consumed: bool,
    pub release_gate_eligible: bool,
    pub non_claims: Vec<&'static str>,
}

/// Production implementations must consume the challenge exactly once in a
/// durable transaction that binds challenge ID, nonce, decision ID and the
/// verified envelope digest. In-memory implementations are test-only.
pub trait ChallengeConsumer {
    fn consume_once(
        &mut self,
        challenge_id: &str,
        nonce: &[u8],
        decision_id: &str,
        envelope_sha256: &str,
    ) -> Result<(), String>;
}

/// Verify a signed response and all strict SRQ D1 claim bindings.
/// A successful result has challenge_consumed=false and
/// release_gate_eligible=false; it is not safe to rely on until consumed.
pub fn verify_cose_eat(
    token_bytes: &[u8],
    trusted_key: &TrustedKey,
    expected: &ExpectedChallenge,
    now_unix_seconds: u64,
) -> Result<VerifiedObservation, String> {
    validate_trust_and_challenge(trusted_key, expected, now_unix_seconds)?;

    if token_bytes.is_empty() || token_bytes.len() > MAX_TOKEN_BYTES {
        return Err("token size is empty or exceeds the profile limit".into());
    }

    let sign1 = CoseSign1::from_slice(token_bytes)
        .map_err(|e| format!("COSE_Sign1 decode failed: {e}"))?;

    if sign1.protected.header.alg != Some(iana::Algorithm::EdDSA.into()) {
        return Err("protected COSE alg must be EdDSA".into());
    }
    if sign1.protected.header.key_id.as_slice() != trusted_key.key_id.as_slice() {
        return Err("protected COSE kid is not the selected trusted key".into());
    }
    if !sign1.unprotected.is_empty() {
        return Err("unprotected COSE headers are not permitted by SRQ D1".into());
    }
    let protected = &sign1.protected.header;
    if !protected.crit.is_empty()
        || protected.content_type.is_some()
        || !protected.iv.is_empty()
        || !protected.partial_iv.is_empty()
        || !protected.counter_signatures.is_empty()
        || !protected.rest.is_empty()
    {
        return Err("unsupported protected COSE header parameter".into());
    }

    let payload = sign1
        .payload
        .as_deref()
        .ok_or("detached COSE payloads are not permitted")?;
    if payload.is_empty() || payload.len() > MAX_PAYLOAD_BYTES {
        return Err("EAT payload size is empty or exceeds the profile limit".into());
    }

    let public_key = VerifyingKey::from_bytes(&trusted_key.public_key)
        .map_err(|e| format!("configured Ed25519 public key is invalid: {e}"))?;
    if public_key.is_weak() {
        return Err("configured Ed25519 public key is weak".into());
    }
    sign1
        .verify_signature(&[], |signature_bytes, signed_structure| {
            let signature = Signature::from_slice(signature_bytes)
                .map_err(|e| format!("Ed25519 signature encoding is invalid: {e}"))?;
            public_key
                .verify_strict(signed_structure, &signature)
                .map_err(|e| format!("Ed25519 signature rejected: {e}"))
        })
        .map_err(|e| format!("COSE signature verification failed: {e}"))?;

    let value: coset::cbor::value::Value =
        coset::cbor::de::from_reader(payload)
            .map_err(|e| format!("EAT claims CBOR decode failed: {e}"))?;
    let claims = match value {
        coset::cbor::value::Value::Map(entries) => entries,
        _ => return Err("EAT claims payload must be a CBOR map".into()),
    };
    validate_claim_inventory(&claims)?;

    let nonce = bytes_claim(&claims, 10, "eat_nonce")?;
    if !(MIN_NONCE_BYTES..=MAX_NONCE_BYTES).contains(&nonce.len()) {
        return Err("EAT nonce length is outside SRQ profile bounds".into());
    }
    if nonce != expected.nonce.as_slice() {
        return Err("EAT nonce does not match the fresh verifier challenge".into());
    }

    let subject = text_int_claim(&claims, 2, "sub")?;
    if subject != expected.target_token || subject != trusted_key.target_token {
        return Err("EAT subject does not match the trusted target binding".into());
    }
    let audience = audience_claim(&claims)?;
    if audience != expected.audience || audience != trusted_key.audience {
        return Err("EAT audience does not match the verifier policy".into());
    }

    let exp = uint_int_claim(&claims, 4, "exp")?;
    let iat = uint_int_claim(&claims, 6, "iat")?;
    if exp != expected.expires_at_unix_seconds {
        return Err("EAT exp does not exactly match challenge expiry".into());
    }
    let earliest_iat = expected
        .issued_at_unix_seconds
        .saturating_sub(expected.max_attester_clock_skew_seconds);
    let latest_iat = now_unix_seconds.saturating_add(expected.max_attester_clock_skew_seconds);
    if iat < earliest_iat || iat > latest_iat || iat >= exp {
        return Err("EAT iat falls outside the configured challenge/time window".into());
    }

    if text_claim(&claims, "srq_profile")? != PROFILE {
        return Err("SRQ profile identifier is unknown".into());
    }
    let challenge_id = text_claim(&claims, "srq_challenge_id")?;
    let decision_id = text_claim(&claims, "srq_decision_id")?;
    let target_token = text_claim(&claims, "srq_target_token")?;
    if challenge_id != expected.challenge_id {
        return Err("challenge ID does not match verifier challenge".into());
    }
    if decision_id != expected.decision_id {
        return Err("decision ID does not match authorized promotion".into());
    }
    if target_token != expected.target_token {
        return Err("SRQ target token does not match expected target".into());
    }

    let generation = uint_text_claim(&claims, "srq_policy_generation")?;
    if generation != expected.policy_generation {
        return Err("logical policy generation does not match promotion".into());
    }

    let artifact_sha256 = text_claim(&claims, "srq_approved_artifact_sha256")?;
    let expected_policy = text_claim(&claims, "srq_expected_effective_policy_sha256")?;
    let observed_policy = text_claim(&claims, "srq_observed_effective_policy_sha256")?;
    let corpus_sha256 = text_claim(&claims, "srq_corpus_sha256")?;
    for (name, digest) in [
        ("srq_approved_artifact_sha256", artifact_sha256),
        ("srq_expected_effective_policy_sha256", expected_policy),
        ("srq_observed_effective_policy_sha256", observed_policy),
        ("srq_corpus_sha256", corpus_sha256),
    ] {
        if !is_sha256(digest) {
            return Err(format!("{name} is not a lowercase SHA-256 value"));
        }
    }
    if artifact_sha256 != expected.approved_artifact_sha256 {
        return Err("approved artifact digest does not match promotion".into());
    }
    if expected_policy != expected.expected_effective_policy_sha256
        || observed_policy != expected.expected_effective_policy_sha256
    {
        return Err("effective policy digest is missing or divergent".into());
    }
    if corpus_sha256 != expected.corpus_sha256 {
        return Err("scenario/corpus digest does not match verifier policy".into());
    }

    let boot_instance_id = text_claim(&claims, "srq_boot_instance_id")?;
    if !valid_identifier(boot_instance_id) {
        return Err("boot instance identifier is malformed".into());
    }
    let deployment_id = text_claim(&claims, "srq_expected_deployment_id")?;
    let booted_deployment_id = text_claim(&claims, "srq_booted_deployment_id")?;
    let current_deployment_id = text_claim(&claims, "srq_current_userspace_deployment_id")?;
    for (name, value) in [
        ("srq_expected_deployment_id", deployment_id),
        ("srq_booted_deployment_id", booted_deployment_id),
        ("srq_current_userspace_deployment_id", current_deployment_id),
    ] {
        if !valid_deployment_id(value) {
            return Err(format!("{name} is malformed"));
        }
    }
    if deployment_id != expected.expected_deployment_id
        || booted_deployment_id != expected.expected_deployment_id
    {
        return Err("booted deployment does not match authorized immutable deployment".into());
    }
    if current_deployment_id != booted_deployment_id {
        return Err("current userspace deployment differs from booted deployment".into());
    }
    if text_claim(&claims, "srq_observation_status")? != "consistent" {
        return Err("boot observation is divergent or unavailable".into());
    }
    if text_claim(&claims, "srq_assurance_tier")? != "D1-software-key" {
        return Err("this verifier accepts only D1-software-key claims".into());
    }
    if !is_null_claim(&claims, "srq_measurement_profile")?
        || !is_null_claim(&claims, "srq_measurement_digest")?
    {
        return Err("D1 software-key claims must not imply hardware measurement".into());
    }

    Ok(VerifiedObservation {
        schema: "srq-eat-cose-verification-v1",
        envelope_sha256: sha256_hex(token_bytes),
        key_id_hex: hex_encode(&trusted_key.key_id),
        target_token: target_token.to_owned(),
        audience: audience.to_owned(),
        challenge_id: challenge_id.to_owned(),
        decision_id: decision_id.to_owned(),
        policy_generation: generation,
        deployment_id: deployment_id.to_owned(),
        boot_instance_id: boot_instance_id.to_owned(),
        artifact_sha256: artifact_sha256.to_owned(),
        effective_policy_sha256: observed_policy.to_owned(),
        corpus_sha256: corpus_sha256.to_owned(),
        assurance_tier: "D1-software-key",
        challenge_consumed: false,
        release_gate_eligible: false,
        non_claims: vec![
            "The software-key response does not prove privileged OS observations are truthful.",
            "No TPM quote, secure-boot measurement, hardware root-of-trust or physical recovery claim is made.",
            "The result is not replay-safe until a durable challenge store atomically consumes the challenge.",
            "The approved artifact digest is a claim binding, not proof the artifact was independently measured.",
            "This result cannot independently satisfy an SRQ release gate.",
        ],
    })
}

/// Verify then consume a challenge through a caller-supplied store contract.
/// A production store must be durable and transactional across process restart.
pub fn verify_and_consume<S: ChallengeConsumer>(
    token_bytes: &[u8],
    trusted_key: &TrustedKey,
    expected: &ExpectedChallenge,
    now_unix_seconds: u64,
    store: &mut S,
) -> Result<VerifiedObservation, String> {
    let mut verified = verify_cose_eat(token_bytes, trusted_key, expected, now_unix_seconds)?;
    store
        .consume_once(
            &verified.challenge_id,
            &expected.nonce,
            &verified.decision_id,
            &verified.envelope_sha256,
        )
        .map_err(|e| format!("durable challenge consumption failed: {e}"))?;
    verified.challenge_consumed = true;
    verified.release_gate_eligible = false;
    Ok(verified)
}

fn validate_trust_and_challenge(
    trusted_key: &TrustedKey,
    expected: &ExpectedChallenge,
    now: u64,
) -> Result<(), String> {
    if trusted_key.key_id.is_empty() || trusted_key.key_id.len() > 64 {
        return Err("configured COSE key ID is empty or too long".into());
    }
    if !valid_identifier(&trusted_key.target_token)
        || !valid_identifier(&trusted_key.audience)
        || !valid_identifier(&expected.target_token)
        || !valid_identifier(&expected.audience)
        || !valid_identifier(&expected.challenge_id)
        || !valid_identifier(&expected.decision_id)
    {
        return Err("trusted target, audience, challenge or decision identifier is malformed".into());
    }
    if trusted_key.target_token != expected.target_token
        || trusted_key.audience != expected.audience
    {
        return Err("trusted key binding conflicts with verifier challenge".into());
    }
    if !(MIN_NONCE_BYTES..=MAX_NONCE_BYTES).contains(&expected.nonce.len()) {
        return Err("challenge nonce must contain 16..=64 bytes".into());
    }
    if expected.issued_at_unix_seconds >= expected.expires_at_unix_seconds
        || now < expected.issued_at_unix_seconds
        || now >= expected.expires_at_unix_seconds
    {
        return Err("challenge is outside verifier-controlled validity window".into());
    }
    if !is_sha256(&expected.approved_artifact_sha256)
        || !is_sha256(&expected.expected_effective_policy_sha256)
        || !is_sha256(&expected.corpus_sha256)
    {
        return Err("verifier challenge contains malformed digest".into());
    }
    if !valid_deployment_id(&expected.expected_deployment_id) {
        return Err("expected deployment ID is malformed".into());
    }
    Ok(())
}

type ClaimMap = Vec<(coset::cbor::value::Value, coset::cbor::value::Value)>;

fn validate_claim_inventory(claims: &ClaimMap) -> Result<(), String> {
    use coset::cbor::value::Value;
    for i in 0..claims.len() {
        if claims[i + 1..].iter().any(|(key, _)| key == &claims[i].0) {
            return Err("duplicate EAT claim key".into());
        }
    }
    const TEXT_KEYS: &[&str] = &[
        "srq_profile",
        "srq_challenge_id",
        "srq_decision_id",
        "srq_target_token",
        "srq_policy_generation",
        "srq_approved_artifact_sha256",
        "srq_expected_effective_policy_sha256",
        "srq_observed_effective_policy_sha256",
        "srq_corpus_sha256",
        "srq_boot_instance_id",
        "srq_expected_deployment_id",
        "srq_booted_deployment_id",
        "srq_current_userspace_deployment_id",
        "srq_observation_status",
        "srq_assurance_tier",
        "srq_measurement_profile",
        "srq_measurement_digest",
    ];
    for (key, _) in claims {
        match key {
            Value::Integer(number)
                if [2_i64, 3, 4, 6, 10].contains(&i64::try_from(*number).unwrap_or(i64::MIN)) => {}
            Value::Text(name) if TEXT_KEYS.contains(&name.as_str()) => {}
            _ => return Err("unknown EAT claim key is not permitted by SRQ D1".into()),
        }
    }
    if claims.len() != 22 {
        return Err(format!("EAT claim inventory incomplete: expected 22 claims, got {}", claims.len()));
    }
    Ok(())
}

fn int_claim<'a>(claims: &'a ClaimMap, label: i64, name: &str) -> Result<&'a coset::cbor::value::Value, String> {
    use coset::cbor::value::Value;
    claims.iter()
        .find(|(key, _)| matches!(key, Value::Integer(n) if i64::try_from(*n).ok() == Some(label)))
        .map(|(_, value)| value)
        .ok_or_else(|| format!("required EAT claim {name} is missing"))
}

fn text_key_claim<'a>(claims: &'a ClaimMap, label: &str) -> Result<&'a coset::cbor::value::Value, String> {
    use coset::cbor::value::Value;
    claims.iter()
        .find(|(key, _)| matches!(key, Value::Text(name) if name == label))
        .map(|(_, value)| value)
        .ok_or_else(|| format!("required EAT claim {label} is missing"))
}

fn uint_int_claim(claims: &ClaimMap, label: i64, name: &str) -> Result<u64, String> {
    use coset::cbor::value::Value;
    match int_claim(claims, label, name)? {
        Value::Integer(value) => u64::try_from(*value).map_err(|_| format!("EAT claim {name} is not unsigned")),
        _ => Err(format!("EAT claim {name} must be an integer")),
    }
}

fn text_int_claim<'a>(claims: &'a ClaimMap, label: i64, name: &str) -> Result<&'a str, String> {
    use coset::cbor::value::Value;
    match int_claim(claims, label, name)? {
        Value::Text(value) => Ok(value),
        _ => Err(format!("EAT claim {name} must be text")),
    }
}

fn bytes_claim<'a>(claims: &'a ClaimMap, label: i64, name: &str) -> Result<&'a [u8], String> {
    use coset::cbor::value::Value;
    match int_claim(claims, label, name)? {
        Value::Bytes(value) => Ok(value),
        _ => Err(format!("EAT claim {name} must be a byte string")),
    }
}

fn text_claim<'a>(claims: &'a ClaimMap, name: &str) -> Result<&'a str, String> {
    use coset::cbor::value::Value;
    match text_key_claim(claims, name)? {
        Value::Text(value) => Ok(value),
        _ => Err(format!("EAT claim {name} must be text")),
    }
}

fn uint_text_claim(claims: &ClaimMap, name: &str) -> Result<u64, String> {
    use coset::cbor::value::Value;
    match text_key_claim(claims, name)? {
        Value::Integer(value) => u64::try_from(*value).map_err(|_| format!("EAT claim {name} is not unsigned")),
        _ => Err(format!("EAT claim {name} must be an integer")),
    }
}

fn audience_claim(claims: &ClaimMap) -> Result<&str, String> {
    use coset::cbor::value::Value;
    match int_claim(claims, 3, "aud")? {
        Value::Text(value) => Ok(value),
        Value::Array(values) if values.len() == 1 => match &values[0] {
            Value::Text(value) => Ok(value),
            _ => Err("EAT aud array must contain one text audience".into()),
        },
        _ => Err("SRQ D1 requires one exact EAT audience".into()),
    }
}

fn is_null_claim(claims: &ClaimMap, name: &str) -> Result<bool, String> {
    use coset::cbor::value::Value;
    Ok(matches!(text_key_claim(claims, name)?, Value::Null))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b':'))
}

fn valid_deployment_id(value: &str) -> bool {
    if is_sha256(value) {
        return true;
    }
    let Some(rest) = value.strip_prefix("/nix/store/") else { return false; };
    let Some((hash, name)) = rest.split_once('-') else { return false; };
    hash.len() == 32
        && hash.bytes().all(|byte| byte.is_ascii_digit() || b"abcdfghijklmnpqrsvwxyz".contains(&byte))
        && !name.is_empty()
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex_encode(&Sha256::digest(bytes))
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(HEX[(byte >> 4) as usize] as char);
        text.push(HEX[(byte & 0x0f) as usize] as char);
    }
    text
}

pub fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 || !value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return Err("hex value must be lowercase, even-length hexadecimal".into());
    }
    value.as_bytes().chunks_exact(2).map(|pair| {
        let nibble = |ch: u8| match ch {
            b'0'..=b'9' => ch - b'0',
            b'a'..=b'f' => ch - b'a' + 10,
            _ => 0,
        };
        Ok((nibble(pair[0]) << 4) | nibble(pair[1]))
    }).collect()
}

#[derive(Clone, Debug, Deserialize)]
pub struct TrustPolicyFile {
    pub key_id_hex: String,
    pub public_key_hex: String,
    pub target_token: String,
    pub audience: String,
}

impl TryFrom<TrustPolicyFile> for TrustedKey {
    type Error = String;
    fn try_from(value: TrustPolicyFile) -> Result<Self, Self::Error> {
        let key_id = hex_decode(&value.key_id_hex)?;
        let public_key: [u8; 32] = hex_decode(&value.public_key_hex)?
            .try_into()
            .map_err(|_| "Ed25519 public key must be exactly 32 bytes".to_owned())?;
        Ok(Self { key_id, public_key, target_token: value.target_token, audience: value.audience })
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ChallengeFile {
    pub nonce_hex: String,
    pub challenge_id: String,
    pub decision_id: String,
    pub target_token: String,
    pub audience: String,
    pub policy_generation: u64,
    pub approved_artifact_sha256: String,
    pub expected_effective_policy_sha256: String,
    pub corpus_sha256: String,
    pub expected_deployment_id: String,
    pub issued_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    #[serde(default)]
    pub max_attester_clock_skew_seconds: u64,
}

impl TryFrom<ChallengeFile> for ExpectedChallenge {
    type Error = String;
    fn try_from(value: ChallengeFile) -> Result<Self, Self::Error> {
        Ok(Self {
            nonce: hex_decode(&value.nonce_hex)?,
            challenge_id: value.challenge_id,
            decision_id: value.decision_id,
            target_token: value.target_token,
            audience: value.audience,
            policy_generation: value.policy_generation,
            approved_artifact_sha256: value.approved_artifact_sha256,
            expected_effective_policy_sha256: value.expected_effective_policy_sha256,
            corpus_sha256: value.corpus_sha256,
            expected_deployment_id: value.expected_deployment_id,
            issued_at_unix_seconds: value.issued_at_unix_seconds,
            expires_at_unix_seconds: value.expires_at_unix_seconds,
            max_attester_clock_skew_seconds: value.max_attester_clock_skew_seconds,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coset::{CoseSign1Builder, HeaderBuilder};
    use coset::cbor::value::Value;
    use ed25519_dalek::{Signer, SigningKey};

    const NONCE: &[u8] = b"0123456789abcdeffresh-nonce";
    const TARGET: &str = "target-token-001";
    const AUDIENCE: &str = "srq-verifier-test";
    const CHALLENGE_ID: &str = "challenge-001";
    const DECISION_ID: &str = "decision-001";
    const NIX_DEPLOYMENT: &str = "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-nixos-system-demo";

    struct Fixture {
        signing_key: SigningKey,
        trusted: TrustedKey,
        challenge: ExpectedChallenge,
        claims: Vec<(Value, Value)>,
    }

    impl Fixture {
        fn new() -> Self {
            let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
            let key_id = b"srq-test-key-01".to_vec();
            let trusted = TrustedKey {
                key_id: key_id.clone(),
                public_key: signing_key.verifying_key().to_bytes(),
                target_token: TARGET.into(),
                audience: AUDIENCE.into(),
            };
            let challenge = ExpectedChallenge {
                nonce: NONCE.to_vec(),
                challenge_id: CHALLENGE_ID.into(),
                decision_id: DECISION_ID.into(),
                target_token: TARGET.into(),
                audience: AUDIENCE.into(),
                policy_generation: 42,
                approved_artifact_sha256: "a".repeat(64),
                expected_effective_policy_sha256: "b".repeat(64),
                corpus_sha256: "c".repeat(64),
                expected_deployment_id: NIX_DEPLOYMENT.into(),
                issued_at_unix_seconds: 1_000,
                expires_at_unix_seconds: 2_000,
                max_attester_clock_skew_seconds: 30,
            };
            let claims = vec![
                (Value::Integer(10.into()), Value::Bytes(NONCE.to_vec())),
                (Value::Integer(2.into()), Value::Text(TARGET.into())),
                (Value::Integer(3.into()), Value::Text(AUDIENCE.into())),
                (Value::Integer(4.into()), Value::Integer(2_000.into())),
                (Value::Integer(6.into()), Value::Integer(1_100.into())),
                (Value::Text("srq_profile".into()), Value::Text(PROFILE.into())),
                (Value::Text("srq_challenge_id".into()), Value::Text(CHALLENGE_ID.into())),
                (Value::Text("srq_decision_id".into()), Value::Text(DECISION_ID.into())),
                (Value::Text("srq_target_token".into()), Value::Text(TARGET.into())),
                (Value::Text("srq_policy_generation".into()), Value::Integer(42_u64.into())),
                (Value::Text("srq_approved_artifact_sha256".into()), Value::Text("a".repeat(64))),
                (Value::Text("srq_expected_effective_policy_sha256".into()), Value::Text("b".repeat(64))),
                (Value::Text("srq_observed_effective_policy_sha256".into()), Value::Text("b".repeat(64))),
                (Value::Text("srq_corpus_sha256".into()), Value::Text("c".repeat(64))),
                (Value::Text("srq_boot_instance_id".into()), Value::Text("boot-instance-001".into())),
                (Value::Text("srq_expected_deployment_id".into()), Value::Text(NIX_DEPLOYMENT.into())),
                (Value::Text("srq_booted_deployment_id".into()), Value::Text(NIX_DEPLOYMENT.into())),
                (Value::Text("srq_current_userspace_deployment_id".into()), Value::Text(NIX_DEPLOYMENT.into())),
                (Value::Text("srq_observation_status".into()), Value::Text("consistent".into())),
                (Value::Text("srq_assurance_tier".into()), Value::Text("D1-software-key".into())),
                (Value::Text("srq_measurement_profile".into()), Value::Null),
                (Value::Text("srq_measurement_digest".into()), Value::Null),
            ];
            Self { signing_key, trusted, challenge, claims }
        }

        fn token_for(&self, claims: &[(Value, Value)]) -> Vec<u8> {
            let mut payload = Vec::new();
            coset::cbor::ser::into_writer(&Value::Map(claims.to_vec()), &mut payload)
                .expect("serialize claims");
            CoseSign1Builder::new()
                .protected(
                    HeaderBuilder::new()
                        .algorithm(iana::Algorithm::EdDSA)
                        .key_id(self.trusted.key_id.clone())
                        .build(),
                )
                .payload(payload)
                .create_signature(&[], |signed| self.signing_key.sign(signed).to_bytes().to_vec())
                .build()
                .to_vec()
                .expect("encode COSE token")
        }

        fn set_text(&mut self, name: &str, value: &str) {
            let entry = self.claims.iter_mut().find(|(key, _)| *key == Value::Text(name.into())).unwrap();
            entry.1 = Value::Text(value.into());
        }

        fn set_uint(&mut self, name: &str, value: u64) {
            let entry = self.claims.iter_mut().find(|(key, _)| *key == Value::Text(name.into())).unwrap();
            entry.1 = Value::Integer(value.into());
        }

        fn signed(&self) -> Vec<u8> {
            self.token_for(&self.claims)
        }
    }

    #[derive(Default)]
    struct MemoryStore {
        consumed: std::collections::BTreeSet<String>,
    }

    impl ChallengeConsumer for MemoryStore {
        fn consume_once(&mut self, challenge_id: &str, _nonce: &[u8], _decision_id: &str, _digest: &str) -> Result<(), String> {
            if !self.consumed.insert(challenge_id.to_owned()) {
                return Err("challenge already consumed".into());
            }
            Ok(())
        }
    }

    #[test]
    fn valid_signed_d1_envelope_verifies_but_is_not_replay_safe_or_release_eligible() {
        let f = Fixture::new();
        let out = verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 1_200).unwrap();
        assert_eq!(out.assurance_tier, "D1-software-key");
        assert!(!out.challenge_consumed);
        assert!(!out.release_gate_eligible);
        assert_eq!(out.deployment_id, NIX_DEPLOYMENT);
    }

    #[test]
    fn valid_envelope_becomes_consumed_only_through_store_contract() {
        let f = Fixture::new();
        let token = f.signed();
        let mut store = MemoryStore::default();
        let out = verify_and_consume(&token, &f.trusted, &f.challenge, 1_200, &mut store).unwrap();
        assert!(out.challenge_consumed);
        assert!(!out.release_gate_eligible);
        assert!(verify_and_consume(&token, &f.trusted, &f.challenge, 1_200, &mut store).is_err());
    }

    #[test]
    fn modified_signature_is_rejected() {
        let f = Fixture::new();
        let mut token = f.signed();
        let last = token.len() - 1;
        token[last] ^= 1;
        assert!(verify_cose_eat(&token, &f.trusted, &f.challenge, 1_200).is_err());
    }

    #[test]
    fn unknown_signing_key_is_rejected() {
        let f = Fixture::new();
        let mut trusted = f.trusted.clone();
        trusted.key_id = b"untrusted-key".to_vec();
        assert!(verify_cose_eat(&f.signed(), &trusted, &f.challenge, 1_200).is_err());
    }

    #[test]
    fn nonce_target_audience_decision_generation_and_digest_mismatches_fail_closed() {
        let f = Fixture::new();
        let token = f.signed();
        let mut wrong_nonce = f.challenge.clone();
        wrong_nonce.nonce = b"abcdefghijklmnopwrong".to_vec();
        assert!(verify_cose_eat(&token, &f.trusted, &wrong_nonce, 1_200).is_err());

        let mut wrong_target = f.challenge.clone();
        wrong_target.target_token = "other-target".into();
        assert!(verify_cose_eat(&token, &f.trusted, &wrong_target, 1_200).is_err());

        let mut wrong_aud = f.challenge.clone();
        wrong_aud.audience = "other-verifier".into();
        assert!(verify_cose_eat(&token, &f.trusted, &wrong_aud, 1_200).is_err());

        let mut wrong_decision = f.challenge.clone();
        wrong_decision.decision_id = "decision-other".into();
        assert!(verify_cose_eat(&token, &f.trusted, &wrong_decision, 1_200).is_err());

        let mut wrong_generation = f.challenge.clone();
        wrong_generation.policy_generation += 1;
        assert!(verify_cose_eat(&token, &f.trusted, &wrong_generation, 1_200).is_err());

        let mut wrong_artifact = f.challenge.clone();
        wrong_artifact.approved_artifact_sha256 = "d".repeat(64);
        assert!(verify_cose_eat(&token, &f.trusted, &wrong_artifact, 1_200).is_err());

        let mut wrong_policy = f.challenge.clone();
        wrong_policy.expected_effective_policy_sha256 = "e".repeat(64);
        assert!(verify_cose_eat(&token, &f.trusted, &wrong_policy, 1_200).is_err());

        let mut wrong_corpus = f.challenge.clone();
        wrong_corpus.corpus_sha256 = "f".repeat(64);
        assert!(verify_cose_eat(&token, &f.trusted, &wrong_corpus, 1_200).is_err());
    }

    #[test]
    fn expiry_and_attester_clock_skew_are_checked_against_verifier_time() {
        let f = Fixture::new();
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 2_000).is_err());
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 999).is_err());

        let mut f = Fixture::new();
        let entry = f.claims.iter_mut().find(|(key, _)| *key == Value::Integer(6.into())).unwrap();
        entry.1 = Value::Integer(5_000.into());
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 1_200).is_err());
    }

    #[test]
    fn deployment_and_live_userspace_drift_are_rejected() {
        let mut f = Fixture::new();
        f.set_text("srq_booted_deployment_id", "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-nixos-system-old");
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 1_200).is_err());

        let mut f = Fixture::new();
        f.set_text("srq_current_userspace_deployment_id", "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-nixos-system-current");
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 1_200).is_err());

        let mut f = Fixture::new();
        f.set_text("srq_expected_deployment_id", "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-nixos-system-other");
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 1_200).is_err());
    }

    #[test]
    fn divergence_d2_claims_and_software_claims_that_forge_measurement_are_rejected() {
        let mut f = Fixture::new();
        f.set_text("srq_observation_status", "divergent");
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 1_200).is_err());

        let mut f = Fixture::new();
        f.set_text("srq_assurance_tier", "D2-hardware-measured");
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 1_200).is_err());

        let mut f = Fixture::new();
        let entry = f.claims.iter_mut().find(|(key, _)| *key == Value::Text("srq_measurement_digest".into())).unwrap();
        entry.1 = Value::Text("a".repeat(64));
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 1_200).is_err());
    }

    #[test]
    fn malformed_digest_key_and_payload_shapes_are_rejected() {
        let mut f = Fixture::new();
        f.set_text("srq_corpus_sha256", "not-a-sha");
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 1_200).is_err());

        let mut bad_key = f.trusted.clone();
        bad_key.public_key = [0_u8; 32];
        assert!(verify_cose_eat(&f.signed(), &bad_key, &f.challenge, 1_200).is_err());

        assert!(verify_cose_eat(&[], &f.trusted, &f.challenge, 1_200).is_err());
        assert!(verify_cose_eat(&vec![0_u8; MAX_TOKEN_BYTES + 1], &f.trusted, &f.challenge, 1_200).is_err());
    }

    #[test]
    fn duplicate_claims_and_unknown_claims_are_rejected() {
        let mut f = Fixture::new();
        f.claims.push((Value::Text("srq_decision_id".into()), Value::Text("shadow".into())));
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 1_200).is_err());

        let mut f = Fixture::new();
        f.claims.push((Value::Text("unrecognized".into()), Value::Text("value".into())));
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 1_200).is_err());
    }

    #[test]
    fn deployment_identity_syntax_must_match_nixos_or_ostree_profile() {
        let mut f = Fixture::new();
        f.set_text("srq_booted_deployment_id", "../old-system");
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 1_200).is_err());

        let mut f = Fixture::new();
        f.set_text("srq_booted_deployment_id", "nix-store:fake");
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 1_200).is_err());
    }

    #[test]
    fn hex_decoder_rejects_uppercase_odd_and_non_hex_values() {
        assert!(hex_decode("AA").is_err());
        assert!(hex_decode("a").is_err());
        assert!(hex_decode("gg").is_err());
        assert_eq!(hex_decode("00ff").unwrap(), vec![0, 255]);
    }

    #[test]
    fn claim_inventory_requires_exactly_all_profile_fields() {
        let mut f = Fixture::new();
        f.claims.pop();
        assert!(verify_cose_eat(&f.signed(), &f.trusted, &f.challenge, 1_200).is_err());
    }

    #[test]
    fn challenge_consumption_failure_never_returns_consumed_success() {
        struct RejectStore;
        impl ChallengeConsumer for RejectStore {
            fn consume_once(&mut self, _: &str, _: &[u8], _: &str, _: &str) -> Result<(), String> {
                Err("durability unavailable".into())
            }
        }
        let f = Fixture::new();
        let error = verify_and_consume(&f.signed(), &f.trusted, &f.challenge, 1_200, &mut RejectStore).unwrap_err();
        assert!(error.contains("durable challenge consumption failed"));
    }
}
