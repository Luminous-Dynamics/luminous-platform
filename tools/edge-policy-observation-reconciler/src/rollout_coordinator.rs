//! Fail-closed orchestration contract for a single-target policy rollout.
//!
//! The adapter must implement a real atomic generation/revision compare-and-swap.
//! Acknowledgements never count as applied state: every transition follows an
//! independent effective-policy readback. This module is not a vendor adapter.

#![forbid(unsafe_code)]

use std::fs::File;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::promotion_journal::{
    PromotionJournal, PromotionOperation, PromotionSpec, PromotionState, PromotionVerificationInput,
};
use super::replay_ledger::SqliteReplayLedger;

const DEFAULT_CHALLENGE_TTL_MS: u64 = 120_000;
const DEFAULT_MAX_RECEIPT_AGE_MS: u64 = 60_000;

/// Policy snapshot obtained from the device/controller's effective-state read API.
///
/// The digest must use the same canonicalization contract for the approved
/// baseline, candidate artifact, and readback. Revision is an opaque CAS token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicySnapshot {
    pub target_token: String,
    pub generation: u64,
    pub revision_token: String,
    pub effective_policy_sha256: String,
}

/// Capability claims supplied by the adapter implementation.
///
/// These are assertions from trusted integration code, not remotely attested
/// properties. An adapter must never advertise CAS unless the device/controller
/// atomically rejects writes against a stale generation/revision/digest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdapterCapabilities {
    pub exclusive_target_lease: bool,
    pub atomic_generation_revision_cas: bool,
    pub stable_effective_policy_digest: bool,
}

/// Candidate or rollback policy artifact to apply.
///
/// The artifact digest is SHA-256 of the exact bytes, so callers must supply the
/// canonical bytes that the device readback layer hashes with the same contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyArtifact {
    pub target_token: String,
    pub generation: u64,
    pub policy_sha256: String,
    pub bytes: Vec<u8>,
}

/// The device API's successful response is only an acknowledgement. The coordinator
/// always performs readback before advancing the journal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyAcknowledgement {
    pub reported_generation: Option<u64>,
    pub request_id: Option<String>,
}

/// Evidence returned by the independent data-plane probe after it receives the
/// exact challenge JSON. The coordinator builds the challenge-bound envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbeOutput {
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
    pub receipt_source: String,
}

/// A target-scoped lease held across the whole change protocol.
///
/// The lease must exclude all policy writers that could invalidate the candidate
/// between final readback and promotion. `renew` must fail if exclusive control
/// can no longer be guaranteed. A local mutex that vendor controllers or other
/// operators can bypass does not satisfy this production contract.
pub trait PolicyDeviceLease {
    fn capabilities(&self) -> AdapterCapabilities;

    /// Renew/validate the target lease at a protocol stage boundary.
    fn renew(&mut self) -> Result<(), String>;

    fn read_effective_policy(&mut self) -> Result<PolicySnapshot, String>;

    /// Must atomically reject a stale generation, revision token, or digest.
    /// Candidate generation must exceed the observed generation; it may skip
    /// values consumed by operations abandoned before actuation.
    fn compare_and_apply(
        &mut self,
        expected: &PolicySnapshot,
        candidate: &PolicyArtifact,
    ) -> Result<ApplyAcknowledgement, String>;
}

/// Adapter capable of acquiring an exclusive lease for one target.
///
/// The lease guard owns writer exclusion until dropped. It must cover writes
/// performed outside this process too, or refuse to advertise this capability.
pub trait PolicyDeviceAdapter {
    type Lease<'a>: PolicyDeviceLease
    where
        Self: 'a;

    fn acquire_target_lease<'a>(
        &'a mut self,
        target_token: &str,
    ) -> Result<Self::Lease<'a>, String>;
}

/// Independent probe contract. Its implementation should run from a separate
/// observation path, not infer reachability from the policy controller response.
pub trait IndependentPolicyProbe {
    fn run_challenged_probe(&mut self, challenge_source: &str) -> Result<ProbeOutput, String>;
}

/// Typed inputs for one rollout attempt.
#[derive(Clone, Debug)]
pub struct RolloutRequest<'a> {
    pub operation_id: &'a str,
    pub target_token: &'a str,
    /// Exact approved canonical policy bytes needed for digest verification and rollback.
    pub approved_baseline_policy: &'a [u8],
    /// Exact canonical candidate policy bytes to install.
    pub candidate_policy: &'a [u8],
    pub corpus_schema_source: &'a str,
    pub corpus_source: &'a str,
}

/// One-target coordinator. The target lease is held for the full apply → probe →
/// promote or rollback lifecycle; adapters without this guarantee are refused.
#[derive(Debug)]
pub struct PolicyRolloutCoordinator<'a> {
    journal: &'a PromotionJournal,
    ledger: &'a SqliteReplayLedger,
    challenge_ttl_ms: u64,
    max_receipt_age_ms: u64,
}

impl<'a> PolicyRolloutCoordinator<'a> {
    #[must_use]
    pub fn new(journal: &'a PromotionJournal, ledger: &'a SqliteReplayLedger) -> Self {
        Self {
            journal,
            ledger,
            challenge_ttl_ms: DEFAULT_CHALLENGE_TTL_MS,
            max_receipt_age_ms: DEFAULT_MAX_RECEIPT_AGE_MS,
        }
    }

    /// Testable/operationally tunable bounds for freshness challenges.
    ///
    /// Both values must be non-zero and no greater than one hour, matching the
    /// verifier's schema and maximum challenge lifetime.
    pub fn with_freshness_limits(
        mut self,
        challenge_ttl_ms: u64,
        max_receipt_age_ms: u64,
    ) -> Result<Self, String> {
        if challenge_ttl_ms == 0
            || challenge_ttl_ms > 3_600_000
            || max_receipt_age_ms == 0
            || max_receipt_age_ms > 3_600_000
        {
            return Err("freshness limits must be in the range 1..=3600000 ms".to_owned());
        }
        self.challenge_ttl_ms = challenge_ttl_ms;
        self.max_receipt_age_ms = max_receipt_age_ms;
        Ok(self)
    }

    /// Apply, independently verify and promote one candidate.
    ///
    /// Any failure after ApplyIntent triggers a guarded rollback attempt. The
    /// coordinator rolls back only when the observed digest is either the exact
    /// candidate or the exact approved baseline. If a third/unrecognized policy
    /// is present, it leaves RollbackIntent durable and fails closed for manual
    /// reconciliation rather than overwriting unknown state.
    pub fn apply_and_promote<D, P>(
        &self,
        device: &mut D,
        probe: &mut P,
        request: &RolloutRequest<'_>,
    ) -> Result<PromotionOperation, String>
    where
        D: PolicyDeviceAdapter,
        P: IndependentPolicyProbe,
    {
        validate_identifier(request.operation_id, "operation_id")?;
        validate_target_token(request.target_token)?;
        let mut lease = device.acquire_target_lease(request.target_token)?;
        require_safe_capabilities(lease.capabilities())?;
        lease
            .renew()
            .map_err(|error| format!("cannot establish exclusive target lease: {error}"))?;

        let baseline_digest = sha256_hex(request.approved_baseline_policy);
        let candidate_digest = sha256_hex(request.candidate_policy);
        if baseline_digest == candidate_digest {
            return Err("candidate policy is identical to the approved baseline".to_owned());
        }

        let baseline = lease.read_effective_policy()?;
        validate_snapshot(&baseline)?;
        if baseline.target_token != request.target_token {
            return Err("device adapter returned a snapshot for the wrong target".to_owned());
        }
        if baseline.effective_policy_sha256 != baseline_digest {
            return Err(
                "approved baseline bytes do not match the live effective-policy digest".to_owned(),
            );
        }
        let generation = self
            .journal
            .next_generation(request.target_token, baseline.generation)?;

        self.journal.prepare(&PromotionSpec {
            operation_id: request.operation_id,
            target_token: request.target_token,
            generation,
            baseline_policy_sha256: &baseline_digest,
            candidate_policy_sha256: &candidate_digest,
            created_at_unix_ms: unix_now_ms()?,
        })?;
        self.journal
            .record_apply_intent(request.operation_id, unix_now_ms()?)?;

        let candidate = PolicyArtifact {
            target_token: request.target_token.to_owned(),
            generation,
            policy_sha256: candidate_digest.clone(),
            bytes: request.candidate_policy.to_vec(),
        };

        if let Err(error) = lease.renew() {
            return Err(self.with_rollback_result(
                &mut lease,
                request,
                &candidate_digest,
                &baseline_digest,
                format!("exclusive target lease renewal failed before apply: {error}"),
            ));
        }
        if let Err(error) = lease.compare_and_apply(&baseline, &candidate) {
            return Err(self.with_rollback_result(
                &mut lease,
                request,
                &candidate_digest,
                &baseline_digest,
                format!("candidate apply was not acknowledged: {error}"),
            ));
        }

        // Never trust the apply acknowledgement as proof of installation.
        let applied_snapshot = match lease.read_effective_policy() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return Err(self.with_rollback_result(
                    &mut lease,
                    request,
                    &candidate_digest,
                    &baseline_digest,
                    format!("effective-policy readback failed after apply: {error}"),
                ));
            }
        };
        if validate_snapshot(&applied_snapshot).is_err()
            || applied_snapshot.target_token != request.target_token
            || applied_snapshot.generation != generation
            || applied_snapshot.effective_policy_sha256 != candidate_digest
        {
            return Err(self.with_rollback_result(
                &mut lease,
                request,
                &candidate_digest,
                &baseline_digest,
                "post-apply effective-policy readback did not exactly match target, generation, and candidate digest".to_owned(),
            ));
        }

        if let Err(error) = self.journal.record_applied(
            request.operation_id,
            &applied_snapshot.effective_policy_sha256,
            unix_now_ms()?,
        ) {
            return Err(self.with_rollback_result(
                &mut lease,
                request,
                &candidate_digest,
                &baseline_digest,
                format!("journal refused Applied transition: {error}"),
            ));
        }

        let challenge_source = match self.build_challenge(
            request.target_token,
            generation,
            &baseline_digest,
            &candidate_digest,
            request.corpus_source,
        ) {
            Ok(challenge) => challenge,
            Err(error) => {
                return Err(self.with_rollback_result(
                    &mut lease,
                    request,
                    &candidate_digest,
                    &baseline_digest,
                    format!("could not create fresh verifier challenge: {error}"),
                ));
            }
        };

        if let Err(error) = lease.renew() {
            return Err(self.with_rollback_result(
                &mut lease,
                request,
                &candidate_digest,
                &baseline_digest,
                format!("exclusive target lease renewal failed before probe: {error}"),
            ));
        }
        let probe_output = match probe.run_challenged_probe(&challenge_source) {
            Ok(output) => output,
            Err(error) => {
                return Err(self.with_rollback_result(
                    &mut lease,
                    request,
                    &candidate_digest,
                    &baseline_digest,
                    format!("independent challenged probe failed: {error}"),
                ));
            }
        };
        if probe_output.completed_at_unix_ms < probe_output.started_at_unix_ms {
            return Err(self.with_rollback_result(
                &mut lease,
                request,
                &candidate_digest,
                &baseline_digest,
                "probe completion time precedes start time".to_owned(),
            ));
        }

        let envelope_source = match build_envelope(&challenge_source, &probe_output) {
            Ok(envelope) => envelope,
            Err(error) => {
                return Err(self.with_rollback_result(
                    &mut lease,
                    request,
                    &candidate_digest,
                    &baseline_digest,
                    format!("could not bind receipt to the challenge: {error}"),
                ));
            }
        };
        let verifier_now = unix_now_ms()?;
        let verification = self.journal.verify_and_record_evidence(
            self.ledger,
            request.operation_id,
            &PromotionVerificationInput {
                challenge_source: &challenge_source,
                envelope_source: &envelope_source,
                corpus_schema_source: request.corpus_schema_source,
                corpus_source: request.corpus_source,
                receipt_source: &probe_output.receipt_source,
                effective_policy_sha256: &candidate_digest,
                verifier_now_unix_ms: verifier_now,
            },
        );
        let (_report, permit) = match verification {
            Ok(value) => value,
            Err(errors) => {
                return Err(self.with_rollback_result(
                    &mut lease,
                    request,
                    &candidate_digest,
                    &baseline_digest,
                    format!(
                        "freshness/conformance/replay verification failed: {}",
                        errors.join("; ")
                    ),
                ));
            }
        };

        let pre_promotion = match lease.read_effective_policy() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return Err(self.with_rollback_result(
                    &mut lease,
                    request,
                    &candidate_digest,
                    &baseline_digest,
                    format!("pre-promotion policy readback failed: {error}"),
                ));
            }
        };
        if validate_snapshot(&pre_promotion).is_err()
            || pre_promotion.target_token != request.target_token
            || pre_promotion.generation != generation
            || pre_promotion.effective_policy_sha256 != candidate_digest
        {
            return Err(self.with_rollback_result(
                &mut lease,
                request,
                &candidate_digest,
                &baseline_digest,
                "pre-promotion policy readback changed from the verified candidate".to_owned(),
            ));
        }

        match self.journal.promote(
            &permit,
            &pre_promotion.effective_policy_sha256,
            unix_now_ms()?,
        ) {
            Ok(operation) => Ok(operation),
            Err(error) => Err(self.with_rollback_result(
                &mut lease,
                request,
                &candidate_digest,
                &baseline_digest,
                format!("promotion commit failed: {error}"),
            )),
        }
    }

    /// Restore the approved baseline for an existing operation. The journal
    /// records rollback intent before any device-side actuation.
    pub fn rollback<D: PolicyDeviceAdapter>(
        &self,
        device: &mut D,
        target_token: &str,
        operation_id: &str,
        approved_baseline_policy: &[u8],
    ) -> Result<PromotionOperation, String> {
        let baseline_digest = sha256_hex(approved_baseline_policy);
        let operation = self
            .journal
            .get(operation_id)?
            .ok_or_else(|| format!("unknown promotion operation {operation_id:?}"))?;
        if operation.target_token != target_token
            || operation.baseline_policy_sha256 != baseline_digest
        {
            return Err(
                "rollback target or approved baseline digest does not match the journal".to_owned(),
            );
        }
        let mut lease = device.acquire_target_lease(target_token)?;
        require_safe_capabilities(lease.capabilities())?;
        lease
            .renew()
            .map_err(|error| format!("cannot establish rollback lease: {error}"))?;
        self.journal
            .request_rollback(operation_id, unix_now_ms()?)?;
        self.rollback_device_to_baseline(
            &mut lease,
            operation_id,
            target_token,
            approved_baseline_policy,
            &baseline_digest,
            &operation.candidate_policy_sha256,
        )
    }

    fn with_rollback_result<L: PolicyDeviceLease>(
        &self,
        device: &mut L,
        request: &RolloutRequest<'_>,
        candidate_digest: &str,
        baseline_digest: &str,
        reason: String,
    ) -> String {
        let rollback = self
            .journal
            .request_rollback(request.operation_id, unix_now_ms().unwrap_or(u64::MAX))
            .and_then(|_| {
                self.rollback_device_to_baseline(
                    device,
                    request.operation_id,
                    request.target_token,
                    request.approved_baseline_policy,
                    baseline_digest,
                    candidate_digest,
                )
            });
        match rollback {
            Ok(operation) if operation.state == PromotionState::RolledBack => {
                format!("{reason}; verified exact-baseline rollback completed")
            }
            Ok(operation) => format!(
                "{reason}; rollback incomplete in state {:?}; manual recovery required",
                operation.state
            ),
            Err(error) => {
                format!("{reason}; rollback incomplete: {error}; manual recovery required")
            }
        }
    }

    fn rollback_device_to_baseline<L: PolicyDeviceLease>(
        &self,
        device: &mut L,
        operation_id: &str,
        target_token: &str,
        approved_baseline_policy: &[u8],
        baseline_digest: &str,
        candidate_digest: &str,
    ) -> Result<PromotionOperation, String> {
        require_safe_capabilities(device.capabilities())?;
        device
            .renew()
            .map_err(|error| format!("cannot renew rollback lease: {error}"))?;
        let current = device.read_effective_policy()?;
        validate_snapshot(&current)?;
        if current.target_token != target_token {
            return Err("rollback readback returned the wrong target".to_owned());
        }
        if current.effective_policy_sha256 == baseline_digest {
            return self
                .journal
                .record_rolled_back(operation_id, baseline_digest, unix_now_ms()?);
        }
        if current.effective_policy_sha256 != candidate_digest {
            return Err(
                "rollback refused to overwrite an unexpected policy digest; leave RollbackIntent for manual reconciliation"
                    .to_owned(),
            );
        }
        let next_generation = current
            .generation
            .checked_add(1)
            .ok_or_else(|| "device policy generation exhausted during rollback".to_owned())?;
        let baseline_artifact = PolicyArtifact {
            target_token: target_token.to_owned(),
            generation: next_generation,
            policy_sha256: baseline_digest.to_owned(),
            bytes: approved_baseline_policy.to_vec(),
        };
        device
            .renew()
            .map_err(|error| format!("cannot renew rollback lease before apply: {error}"))?;
        device.compare_and_apply(&current, &baseline_artifact)?;
        device
            .renew()
            .map_err(|error| format!("cannot renew rollback lease before readback: {error}"))?;
        let restored = device.read_effective_policy()?;
        validate_snapshot(&restored)?;
        if restored.target_token != target_token
            || restored.effective_policy_sha256 != baseline_digest
        {
            return Err("rollback readback did not match the exact approved baseline".to_owned());
        }
        self.journal
            .record_rolled_back(
                operation_id,
                &restored.effective_policy_sha256,
                unix_now_ms()?,
            )
            .map_err(|error| {
                format!("baseline restored but journal did not confirm rollback: {error}")
            })
    }

    fn build_challenge(
        &self,
        target_token: &str,
        generation: u64,
        baseline_digest: &str,
        candidate_digest: &str,
        corpus_source: &str,
    ) -> Result<String, String> {
        let now = unix_now_ms()?;
        let expires = now
            .checked_add(self.challenge_ttl_ms)
            .ok_or_else(|| "challenge expiry overflow".to_owned())?;
        let nonce = random_hex_32()?;
        let challenge_id = format!("challenge-{}", &sha256_hex(nonce.as_bytes())[..24]);
        let challenge = json!({
            "schema_version": "1.0.0",
            "challenge_id": challenge_id,
            "nonce": nonce,
            "issued_at_unix_ms": now,
            "expires_at_unix_ms": expires,
            "max_receipt_age_ms": self.max_receipt_age_ms,
            "policy_generation": generation,
            "corpus_sha256": sha256_hex(corpus_source.as_bytes()),
            "approved_policy_sha256": baseline_digest,
            "expected_effective_policy_sha256": candidate_digest,
            "target_token": target_token
        });
        serde_json::to_string(&challenge)
            .map_err(|error| format!("cannot serialize challenge: {error}"))
    }
}

fn build_envelope(challenge_source: &str, output: &ProbeOutput) -> Result<String, String> {
    let challenge: Value = serde_json::from_str(challenge_source)
        .map_err(|error| format!("invalid internal challenge JSON: {error}"))?;
    let receipt: Value = serde_json::from_str(&output.receipt_source)
        .map_err(|error| format!("probe receipt is invalid JSON: {error}"))?;
    let challenge_id = challenge["challenge_id"]
        .as_str()
        .ok_or_else(|| "challenge ID missing".to_owned())?;
    let nonce = challenge["nonce"]
        .as_str()
        .ok_or_else(|| "challenge nonce missing".to_owned())?;
    let generation = challenge["policy_generation"]
        .as_u64()
        .ok_or_else(|| "challenge generation missing".to_owned())?;
    let run_id = receipt["run_id"]
        .as_str()
        .ok_or_else(|| "receipt run_id missing".to_owned())?;

    let envelope = json!({
        "schema_version": "1.0.0",
        "challenge_id": challenge_id,
        "nonce": nonce,
        "run_id": run_id,
        "policy_generation": generation,
        "started_at_unix_ms": output.started_at_unix_ms,
        "completed_at_unix_ms": output.completed_at_unix_ms,
        "receipt_sha256": sha256_hex(output.receipt_source.as_bytes())
    });
    serde_json::to_string(&envelope)
        .map_err(|error| format!("cannot serialize freshness envelope: {error}"))
}

fn require_safe_capabilities(capabilities: AdapterCapabilities) -> Result<(), String> {
    if !capabilities.exclusive_target_lease
        || !capabilities.atomic_generation_revision_cas
        || !capabilities.stable_effective_policy_digest
    {
        return Err(
            "automatic rollout refused: adapter must guarantee an exclusive target lease, atomic generation/revision CAS, and stable effective-policy digests"
                .to_owned(),
        );
    }
    Ok(())
}

fn validate_snapshot(snapshot: &PolicySnapshot) -> Result<(), String> {
    validate_target_token(&snapshot.target_token)?;
    validate_revision_token(&snapshot.revision_token)?;
    validate_digest(&snapshot.effective_policy_sha256, "effective policy digest")?;
    if snapshot.generation == 0 {
        return Err("device policy generation must be positive".to_owned());
    }
    Ok(())
}

fn validate_revision_token(value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        return Err("device revision token must contain 1..=256 non-control characters".to_owned());
    }
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
        return Err("target token is outside the supported target- identifier grammar".to_owned());
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
            "{label} must be a 64-character lowercase SHA-256 digest"
        ));
    }
    Ok(())
}

fn random_hex_32() -> Result<String, String> {
    let mut bytes = [0_u8; 32];
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|error| {
            format!("cannot obtain a cryptographically secure challenge nonce: {error}")
        })?;
    let mut output = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(output)
}

fn unix_now_ms() -> Result<u64, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock predates the Unix epoch: {error}"))?;
    u64::try_from(now.as_millis())
        .map_err(|error| format!("system timestamp conversion failed: {error}"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    super::sha256_hex(bytes)
}

#[cfg(test)]
mod tests {
    use super::{
        AdapterCapabilities, ApplyAcknowledgement, IndependentPolicyProbe, PolicyDeviceAdapter,
        PolicyDeviceLease, PolicyRolloutCoordinator, PolicySnapshot, ProbeOutput, RolloutRequest,
        validate_snapshot,
    };
    use crate::promotion_journal::{PromotionJournal, PromotionSpec, PromotionState};
    use crate::replay_ledger::SqliteReplayLedger;
    use serde_json::{Value, json};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    const TARGET: &str = "target-coordinator-lab";
    const CORPUS_SCHEMA: &str =
        include_str!("../../../schemas/edge-policy-conformance-corpus-v1.schema.json");
    const CORPUS: &str =
        include_str!("../../../examples/edge-policy-conformance-corpus-synthetic-v1.json");

    fn new_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "luminous-edge-rollout-{label}-{}-{nanos}-{id}.sqlite",
            std::process::id()
        ))
    }

    fn remove_database(path: &PathBuf) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }

    #[derive(Debug)]
    struct FakeDevice {
        snapshot: PolicySnapshot,
        capabilities: AdapterCapabilities,
        writes: usize,
        lease_renewals: usize,
        fail_apply: bool,
        fail_apply_after_mutation: bool,
        unexpected_on_readback: bool,
        unexpected_after_apply: bool,
    }

    impl FakeDevice {
        fn baseline() -> (Self, Vec<u8>) {
            let bytes = b"baseline-policy".to_vec();
            let digest = crate::sha256_hex(&bytes);
            (
                Self {
                    snapshot: PolicySnapshot {
                        target_token: TARGET.to_owned(),
                        generation: 1,
                        revision_token: "revision-1".to_owned(),
                        effective_policy_sha256: digest,
                    },
                    capabilities: AdapterCapabilities {
                        exclusive_target_lease: true,
                        atomic_generation_revision_cas: true,
                        stable_effective_policy_digest: true,
                    },
                    writes: 0,
                    lease_renewals: 0,
                    fail_apply: false,
                    fail_apply_after_mutation: false,
                    unexpected_on_readback: false,
                    unexpected_after_apply: false,
                },
                bytes,
            )
        }
    }

    struct FakeDeviceLease<'a> {
        device: &'a mut FakeDevice,
    }

    impl PolicyDeviceAdapter for FakeDevice {
        type Lease<'a>
            = FakeDeviceLease<'a>
        where
            Self: 'a;

        fn acquire_target_lease<'a>(
            &'a mut self,
            target_token: &str,
        ) -> Result<Self::Lease<'a>, String> {
            if target_token != self.snapshot.target_token {
                return Err("target lease requested for a different target".to_owned());
            }
            Ok(FakeDeviceLease { device: self })
        }
    }

    impl PolicyDeviceLease for FakeDeviceLease<'_> {
        fn capabilities(&self) -> AdapterCapabilities {
            self.device.capabilities
        }

        fn renew(&mut self) -> Result<(), String> {
            if !self.device.capabilities.exclusive_target_lease {
                return Err("exclusive target lease is unavailable".to_owned());
            }
            self.device.lease_renewals += 1;
            Ok(())
        }

        fn read_effective_policy(&mut self) -> Result<PolicySnapshot, String> {
            if self.device.unexpected_on_readback {
                let mut snapshot = self.device.snapshot.clone();
                snapshot.effective_policy_sha256 = "e".repeat(64);
                return Ok(snapshot);
            }
            Ok(self.device.snapshot.clone())
        }

        fn compare_and_apply(
            &mut self,
            expected: &PolicySnapshot,
            candidate: &super::PolicyArtifact,
        ) -> Result<ApplyAcknowledgement, String> {
            if !self.device.capabilities.atomic_generation_revision_cas {
                return Err("atomic CAS unsupported".to_owned());
            }
            if &self.device.snapshot != expected {
                return Err("stale revision/generation/digest".to_owned());
            }
            if candidate.target_token != self.device.snapshot.target_token
                || candidate.generation <= expected.generation
                || candidate.policy_sha256 != crate::sha256_hex(&candidate.bytes)
            {
                return Err("candidate contract mismatch".to_owned());
            }
            self.device.writes += 1;
            if self.device.fail_apply {
                // Model a command failure before mutation.
                return Err("simulated command timeout before mutation".to_owned());
            }
            self.device.snapshot = PolicySnapshot {
                target_token: candidate.target_token.clone(),
                generation: candidate.generation,
                revision_token: format!("revision-{}", candidate.generation),
                effective_policy_sha256: candidate.policy_sha256.clone(),
            };
            if self.device.unexpected_after_apply {
                self.device.unexpected_on_readback = true;
            }
            if self.device.fail_apply_after_mutation {
                // One-shot injection: simulate exactly one lost acknowledgement,
                // then allow the recovery controller's rollback to complete.
                self.device.fail_apply_after_mutation = false;
                return Err("simulated acknowledgement loss after device mutation".to_owned());
            }
            Ok(ApplyAcknowledgement {
                reported_generation: Some(candidate.generation),
                request_id: Some("fake-adapter-request-1".to_owned()),
            })
        }
    }

    struct FailingProbe;

    impl IndependentPolicyProbe for FailingProbe {
        fn run_challenged_probe(&mut self, _challenge_source: &str) -> Result<ProbeOutput, String> {
            Err("simulated independent probe outage".to_owned())
        }
    }

    struct PassingProbe {
        baseline_digest: String,
        candidate_digest: String,
    }

    impl IndependentPolicyProbe for PassingProbe {
        fn run_challenged_probe(&mut self, challenge_source: &str) -> Result<ProbeOutput, String> {
            let challenge: Value =
                serde_json::from_str(challenge_source).map_err(|error| error.to_string())?;
            let observed_start = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| error.to_string())?
                .as_millis() as u64;
            let started = observed_start.max(
                challenge["issued_at_unix_ms"]
                    .as_u64()
                    .unwrap_or(observed_start),
            );
            let mut receipt =
                crate::build_synthetic_receipt(CORPUS).map_err(|errors| errors.join("; "))?;
            receipt["evidence_class"] = json!("namespace_lab_observation");
            receipt["run_id"] = json!("run-independent-coordinator-probe");
            receipt["approved_policy_sha256"] = json!(self.baseline_digest);
            receipt["effective_config_sha256"] = json!(self.candidate_digest);
            receipt["target_token"] = challenge["target_token"].clone();
            receipt["time_basis"] = json!({
                "kind": "monotonic_with_wall_anchor",
                "clock_id": "coordinator-test-clock",
                "wall_anchor_unix_ms": started,
                "monotonic_origin_ms": 0
            });
            for (index, observation) in receipt["observations"]
                .as_array_mut()
                .expect("observations array")
                .iter_mut()
                .enumerate()
            {
                observation["evidence_source"] = json!("independent_dataplane_probe");
                observation["observed_at_unix_ms"] = json!(started + index as u64);
                observation["monotonic_elapsed_ms"] = json!(index as u64);
                observation["duration_ms"] = json!(5);
                observation["evidence_sha256"] = json!("d".repeat(64));
                observation["effective_policy_sha256"] = json!(self.candidate_digest);
            }
            thread::sleep(Duration::from_millis(20));
            let completed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| error.to_string())?
                .as_millis() as u64;
            let receipt_source =
                serde_json::to_string(&receipt).map_err(|error| error.to_string())?;
            Ok(ProbeOutput {
                started_at_unix_ms: started
                    .max(challenge["issued_at_unix_ms"].as_u64().unwrap_or(started)),
                completed_at_unix_ms: completed,
                receipt_source,
            })
        }
    }

    #[test]
    fn refuses_automatic_writes_without_atomic_cas() {
        let (mut device, baseline) = FakeDevice::baseline();
        device.capabilities.atomic_generation_revision_cas = false;
        let candidate = b"candidate-policy".to_vec();
        let journal_path = new_path("no-cas-journal");
        let ledger_path = new_path("no-cas-ledger");
        let journal = PromotionJournal::open(&journal_path).expect("open journal");
        let ledger = SqliteReplayLedger::open(&ledger_path).expect("open ledger");
        let coordinator = PolicyRolloutCoordinator::new(&journal, &ledger);
        let mut probe = PassingProbe {
            baseline_digest: crate::sha256_hex(&baseline),
            candidate_digest: crate::sha256_hex(&candidate),
        };
        let result = coordinator.apply_and_promote(
            &mut device,
            &mut probe,
            &RolloutRequest {
                operation_id: "operation-rollout-cas",
                target_token: TARGET,
                approved_baseline_policy: &baseline,
                candidate_policy: &candidate,
                corpus_schema_source: CORPUS_SCHEMA,
                corpus_source: CORPUS,
            },
        );
        assert!(
            result
                .expect_err("no-CAS adapter rejected")
                .contains("atomic generation/revision CAS")
        );
        assert_eq!(device.writes, 0);
        assert_eq!(
            device.snapshot.effective_policy_sha256,
            crate::sha256_hex(&baseline)
        );
        drop(ledger);
        drop(journal);
        remove_database(&journal_path);
        remove_database(&ledger_path);
    }

    #[test]
    fn ambiguous_apply_failure_after_mutation_is_read_back_and_rolled_back() {
        let (mut device, baseline) = FakeDevice::baseline();
        device.fail_apply_after_mutation = true;
        let candidate = b"candidate-policy-after-timeout".to_vec();
        let baseline_digest = crate::sha256_hex(&baseline);
        let candidate_digest = crate::sha256_hex(&candidate);
        let journal_path = new_path("mutated-before-timeout-journal");
        let ledger_path = new_path("mutated-before-timeout-ledger");
        let journal = PromotionJournal::open(&journal_path).expect("open journal");
        let ledger = SqliteReplayLedger::open(&ledger_path).expect("open ledger");
        let coordinator = PolicyRolloutCoordinator::new(&journal, &ledger);
        let mut probe = PassingProbe {
            baseline_digest: baseline_digest.clone(),
            candidate_digest,
        };

        let error = coordinator
            .apply_and_promote(
                &mut device,
                &mut probe,
                &RolloutRequest {
                    operation_id: "operation-mutated-timeout",
                    target_token: TARGET,
                    approved_baseline_policy: &baseline,
                    candidate_policy: &candidate,
                    corpus_schema_source: CORPUS_SCHEMA,
                    corpus_source: CORPUS,
                },
            )
            .expect_err("lost apply acknowledgement");
        assert!(error.contains("verified exact-baseline rollback completed"));
        let operation = journal
            .get("operation-mutated-timeout")
            .expect("read journal")
            .expect("operation");
        assert_eq!(operation.state, PromotionState::RolledBack);
        assert_eq!(device.snapshot.effective_policy_sha256, baseline_digest);
        assert_eq!(device.writes, 2, "candidate apply plus baseline restore");
        assert!(device.lease_renewals >= 3);
        drop(ledger);
        drop(journal);
        remove_database(&journal_path);
        remove_database(&ledger_path);
    }

    #[test]
    fn probe_failure_restores_exact_baseline_after_candidate_was_applied() {
        let (mut device, baseline) = FakeDevice::baseline();
        let candidate = b"candidate-probe-outage".to_vec();
        let baseline_digest = crate::sha256_hex(&baseline);
        let journal_path = new_path("probe-outage-journal");
        let ledger_path = new_path("probe-outage-ledger");
        let journal = PromotionJournal::open(&journal_path).expect("open journal");
        let ledger = SqliteReplayLedger::open(&ledger_path).expect("open ledger");
        let coordinator = PolicyRolloutCoordinator::new(&journal, &ledger);
        let mut probe = FailingProbe;

        let error = coordinator
            .apply_and_promote(
                &mut device,
                &mut probe,
                &RolloutRequest {
                    operation_id: "operation-probe-outage",
                    target_token: TARGET,
                    approved_baseline_policy: &baseline,
                    candidate_policy: &candidate,
                    corpus_schema_source: CORPUS_SCHEMA,
                    corpus_source: CORPUS,
                },
            )
            .expect_err("probe outage blocks promotion");
        assert!(error.contains("verified exact-baseline rollback completed"));
        let operation = journal
            .get("operation-probe-outage")
            .expect("read journal")
            .expect("operation");
        assert_eq!(operation.state, PromotionState::RolledBack);
        assert_eq!(device.snapshot.effective_policy_sha256, baseline_digest);
        assert_eq!(device.writes, 2);
        drop(ledger);
        drop(journal);
        remove_database(&journal_path);
        remove_database(&ledger_path);
    }

    #[test]
    fn unknown_effective_policy_is_never_overwritten_during_rollback() {
        let (mut device, baseline) = FakeDevice::baseline();
        device.unexpected_after_apply = true;
        let candidate = b"candidate-third-party-state".to_vec();
        let journal_path = new_path("unknown-policy-journal");
        let ledger_path = new_path("unknown-policy-ledger");
        let journal = PromotionJournal::open(&journal_path).expect("open journal");
        let ledger = SqliteReplayLedger::open(&ledger_path).expect("open ledger");
        let coordinator = PolicyRolloutCoordinator::new(&journal, &ledger);
        let mut probe = PassingProbe {
            baseline_digest: crate::sha256_hex(&baseline),
            candidate_digest: crate::sha256_hex(&candidate),
        };

        let error = coordinator
            .apply_and_promote(
                &mut device,
                &mut probe,
                &RolloutRequest {
                    operation_id: "operation-unknown-policy",
                    target_token: TARGET,
                    approved_baseline_policy: &baseline,
                    candidate_policy: &candidate,
                    corpus_schema_source: CORPUS_SCHEMA,
                    corpus_source: CORPUS,
                },
            )
            .expect_err("unrecognized readback blocks promotion");
        assert!(error.contains("unexpected policy digest"));
        let operation = journal
            .get("operation-unknown-policy")
            .expect("read journal")
            .expect("operation");
        assert_eq!(operation.state, PromotionState::RollbackIntent);
        assert_eq!(
            device.snapshot.effective_policy_sha256,
            crate::sha256_hex(&candidate),
            "the coordinator must not overwrite a state whose readback is ambiguous"
        );
        assert_eq!(
            device.writes, 1,
            "only the candidate actuation was attempted"
        );
        drop(ledger);
        drop(journal);
        remove_database(&journal_path);
        remove_database(&ledger_path);
    }

    #[test]
    fn opaque_revision_tokens_can_preserve_vendor_etag_syntax() {
        let snapshot = PolicySnapshot {
            target_token: TARGET.to_owned(),
            generation: 1,
            revision_token: "W/\"edge:policy/v3+blue\"".to_owned(),
            effective_policy_sha256: "a".repeat(64),
        };
        assert!(validate_snapshot(&snapshot).is_ok());
        let invalid = PolicySnapshot {
            revision_token: "revision-1\nrevision-2".to_owned(),
            ..snapshot
        };
        assert!(validate_snapshot(&invalid).is_err());
    }

    #[test]
    fn refuses_automatic_writes_without_exclusive_target_lease() {
        let (mut device, baseline) = FakeDevice::baseline();
        device.capabilities.exclusive_target_lease = false;
        let candidate = b"candidate-policy".to_vec();
        let journal_path = new_path("no-lease-journal");
        let ledger_path = new_path("no-lease-ledger");
        let journal = PromotionJournal::open(&journal_path).expect("open journal");
        let ledger = SqliteReplayLedger::open(&ledger_path).expect("open ledger");
        let coordinator = PolicyRolloutCoordinator::new(&journal, &ledger);
        let mut probe = PassingProbe {
            baseline_digest: crate::sha256_hex(&baseline),
            candidate_digest: crate::sha256_hex(&candidate),
        };
        let error = coordinator
            .apply_and_promote(
                &mut device,
                &mut probe,
                &RolloutRequest {
                    operation_id: "operation-rollout-no-lease",
                    target_token: TARGET,
                    approved_baseline_policy: &baseline,
                    candidate_policy: &candidate,
                    corpus_schema_source: CORPUS_SCHEMA,
                    corpus_source: CORPUS,
                },
            )
            .expect_err("unsafe adapter rejected before writes");
        assert!(error.contains("exclusive target lease"));
        assert_eq!(device.writes, 0);
        assert!(
            journal
                .get("operation-rollout-no-lease")
                .expect("query journal")
                .is_none()
        );
        drop(ledger);
        drop(journal);
        remove_database(&journal_path);
        remove_database(&ledger_path);
    }

    #[test]
    fn coordinator_skips_generation_consumed_by_pre_actuation_abandonment() {
        let (mut device, baseline) = FakeDevice::baseline();
        let candidate = b"candidate-after-abandon".to_vec();
        let baseline_digest = crate::sha256_hex(&baseline);
        let candidate_digest = crate::sha256_hex(&candidate);
        let journal_path = new_path("abandoned-generation-journal");
        let ledger_path = new_path("abandoned-generation-ledger");
        let journal = PromotionJournal::open(&journal_path).expect("open journal");
        let ledger = SqliteReplayLedger::open(&ledger_path).expect("open ledger");
        journal
            .prepare(&PromotionSpec {
                operation_id: "operation-abandoned-before-apply",
                target_token: TARGET,
                generation: 2,
                baseline_policy_sha256: &baseline_digest,
                candidate_policy_sha256: &candidate_digest,
                created_at_unix_ms: 100,
            })
            .expect("reserve generation two");
        journal
            .abandon_prepared("operation-abandoned-before-apply", 101)
            .expect("abandon before apply intent");

        let coordinator = PolicyRolloutCoordinator::new(&journal, &ledger);
        let mut probe = PassingProbe {
            baseline_digest,
            candidate_digest: candidate_digest.clone(),
        };
        let operation = coordinator
            .apply_and_promote(
                &mut device,
                &mut probe,
                &RolloutRequest {
                    operation_id: "operation-after-abandon",
                    target_token: TARGET,
                    approved_baseline_policy: &baseline,
                    candidate_policy: &candidate,
                    corpus_schema_source: CORPUS_SCHEMA,
                    corpus_source: CORPUS,
                },
            )
            .expect("coordinator uses generation three");
        assert_eq!(operation.generation, 3);
        assert_eq!(operation.state, PromotionState::Promoted);
        assert_eq!(device.snapshot.generation, 3);
        drop(ledger);
        drop(journal);
        remove_database(&journal_path);
        remove_database(&ledger_path);
    }

    #[test]
    fn verified_candidate_promotes_only_after_independent_challenge_and_readbacks() {
        let (mut device, baseline) = FakeDevice::baseline();
        let candidate = b"candidate-policy".to_vec();
        let baseline_digest = crate::sha256_hex(&baseline);
        let candidate_digest = crate::sha256_hex(&candidate);
        let journal_path = new_path("success-journal");
        let ledger_path = new_path("success-ledger");
        let journal = PromotionJournal::open(&journal_path).expect("open journal");
        let ledger = SqliteReplayLedger::open(&ledger_path).expect("open ledger");
        let coordinator = PolicyRolloutCoordinator::new(&journal, &ledger);
        let mut probe = PassingProbe {
            baseline_digest,
            candidate_digest: candidate_digest.clone(),
        };
        let operation = coordinator
            .apply_and_promote(
                &mut device,
                &mut probe,
                &RolloutRequest {
                    operation_id: "operation-rollout-success",
                    target_token: TARGET,
                    approved_baseline_policy: &baseline,
                    candidate_policy: &candidate,
                    corpus_schema_source: CORPUS_SCHEMA,
                    corpus_source: CORPUS,
                },
            )
            .expect("candidate promoted");
        assert_eq!(operation.state, PromotionState::Promoted);
        assert_eq!(
            operation.observed_policy_sha256.as_deref(),
            Some(candidate_digest.as_str())
        );
        assert_eq!(device.snapshot.generation, 2);
        assert_eq!(device.writes, 1);
        drop(ledger);
        drop(journal);
        remove_database(&journal_path);
        remove_database(&ledger_path);
    }

    #[test]
    fn ambiguous_apply_failure_rolls_back_only_after_observing_baseline_or_candidate() {
        let (mut device, baseline) = FakeDevice::baseline();
        device.fail_apply = true;
        let candidate = b"candidate-policy".to_vec();
        let journal_path = new_path("failed-apply-journal");
        let ledger_path = new_path("failed-apply-ledger");
        let journal = PromotionJournal::open(&journal_path).expect("open journal");
        let ledger = SqliteReplayLedger::open(&ledger_path).expect("open ledger");
        let coordinator = PolicyRolloutCoordinator::new(&journal, &ledger);
        let mut probe = PassingProbe {
            baseline_digest: crate::sha256_hex(&baseline),
            candidate_digest: crate::sha256_hex(&candidate),
        };
        let result = coordinator.apply_and_promote(
            &mut device,
            &mut probe,
            &RolloutRequest {
                operation_id: "operation-rollout-failed",
                target_token: TARGET,
                approved_baseline_policy: &baseline,
                candidate_policy: &candidate,
                corpus_schema_source: CORPUS_SCHEMA,
                corpus_source: CORPUS,
            },
        );
        assert!(result.is_err());
        let operation = journal
            .get("operation-rollout-failed")
            .expect("read journal")
            .expect("operation");
        assert_eq!(operation.state, PromotionState::RolledBack);
        assert_eq!(
            device.snapshot.effective_policy_sha256,
            crate::sha256_hex(&baseline)
        );
        drop(ledger);
        drop(journal);
        remove_database(&journal_path);
        remove_database(&ledger_path);
    }
}
