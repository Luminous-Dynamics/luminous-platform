//! Research-only act-bound execution finality reference.
//!
//! This module demonstrates the binding and participant-ledger atomicity contract.
//! It does not implement signatures, a key store, a network transport, or a production
//! policy engine. Production adapters must authenticate the broker receipt, runtime
//! evidence, challenge-authority receipt, and finality issuer through the existing
//! typed source-evidence infrastructure. The participant ledger remains authoritative
//! for one-use consumption and remote effect state.

use crate::sealed::EvaluationArm;
use crate::Digest;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

pub const CANDIDATE_ACT_PROFILE: &str = "luminous-candidate-act-v0.1";
pub const EXECUTION_HANDLE_PROFILE: &str = "luminous-execution-handle-v0.1";
pub const MAX_FINALITY_HANDLE_TTL_MS: u64 = 30_000;
pub const MAX_FINALITY_CLOCK_UNCERTAINTY_MS: u64 = 1_000;

pub trait DigestProvider: Send + Sync {
    /// Production adapters must use an established SHA-256 implementation over the
    /// provided domain-separated canonical bytes. No digest algorithm is implemented here.
    fn digest(&self, domain: &'static [u8], canonical_bytes: &[u8]) -> Digest;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateAct {
    pub profile: String,
    pub tenant_id: String,
    pub principal_id: String,
    pub episode_id: String,
    pub plan_digest: Digest,
    pub step_id: String,
    pub grant_id: String,
    pub grant_digest: Digest,
    pub grant_scope: String,
    pub tool_id: String,
    pub operation_id: String,
    pub manifest_version: String,
    pub manifest_digest: Digest,
    pub toolchain_id: String,
    pub toolchain_digest: Digest,
    pub policy_version: String,
    pub policy_digest: Digest,
    pub risk_level: u8,
    /// Participant is the handle audience, not a display label.
    pub participant_id: String,
    pub finality_sink_id: String,
    pub target_digest: Digest,
    /// Exact canonical operation payload produced by the existing broker adapter.
    /// The effect sink compares these bytes with the bytes actually received.
    pub canonical_request_bytes: Vec<u8>,
    pub attempt_id: String,
    pub idempotency_key: String,
    pub environment_digest: Digest,
    pub run_id: String,
    pub trial_id: String,
    pub arm: EvaluationArm,
    pub arm_identity_digest: Digest,
    pub scenario_commitment: Digest,
    pub runtime_evidence_digest: Digest,
    pub broker_receipt_digest: Digest,
    pub created_at_ms: u64,
}

impl CandidateAct {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, FinalityError> {
        validate_text("profile", &self.profile)?;
        if self.profile != CANDIDATE_ACT_PROFILE {
            return Err(FinalityError::UnsupportedActProfile);
        }
        let mut out = Vec::with_capacity(1024 + self.canonical_request_bytes.len());
        out.extend_from_slice(b"org.luminousdynamics.opsbench.candidate-act.v0.1\0");
        put_text(&mut out, &self.profile)?;
        put_text(&mut out, &self.tenant_id)?;
        put_text(&mut out, &self.principal_id)?;
        put_text(&mut out, &self.episode_id)?;
        put_digest(&mut out, &self.plan_digest);
        put_text(&mut out, &self.step_id)?;
        put_text(&mut out, &self.grant_id)?;
        put_digest(&mut out, &self.grant_digest);
        put_text(&mut out, &self.grant_scope)?;
        put_text(&mut out, &self.tool_id)?;
        put_text(&mut out, &self.operation_id)?;
        put_text(&mut out, &self.manifest_version)?;
        put_digest(&mut out, &self.manifest_digest);
        put_text(&mut out, &self.toolchain_id)?;
        put_digest(&mut out, &self.toolchain_digest);
        put_text(&mut out, &self.policy_version)?;
        put_digest(&mut out, &self.policy_digest);
        out.push(self.risk_level);
        put_text(&mut out, &self.participant_id)?;
        put_text(&mut out, &self.finality_sink_id)?;
        put_digest(&mut out, &self.target_digest);
        put_bytes(&mut out, &self.canonical_request_bytes)?;
        put_text(&mut out, &self.attempt_id)?;
        put_text(&mut out, &self.idempotency_key)?;
        put_digest(&mut out, &self.environment_digest);
        put_text(&mut out, &self.run_id)?;
        put_text(&mut out, &self.trial_id)?;
        out.push(match self.arm {
            EvaluationArm::Candidate => 1,
            EvaluationArm::Baseline => 2,
        });
        put_digest(&mut out, &self.arm_identity_digest);
        put_digest(&mut out, &self.scenario_commitment);
        put_digest(&mut out, &self.runtime_evidence_digest);
        put_digest(&mut out, &self.broker_receipt_digest);
        out.extend_from_slice(&self.created_at_ms.to_be_bytes());
        Ok(out)
    }

    pub fn digest(&self, digester: &dyn DigestProvider) -> Result<Digest, FinalityError> {
        Ok(digester.digest(
            b"org.luminousdynamics.opsbench.candidate-act.digest.v0.1\0",
            &self.canonical_bytes()?,
        ))
    }

    pub fn request_digest(&self, digester: &dyn DigestProvider) -> Result<Digest, FinalityError> {
        if self.canonical_request_bytes.is_empty() {
            return Err(FinalityError::EmptyRequest);
        }
        Ok(digester.digest(
            b"org.luminousdynamics.opsbench.request.digest.v0.1\0",
            &self.canonical_request_bytes,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerAuthorizationReceipt {
    pub receipt_digest: Digest,
    pub broker_id: String,
    pub broker_control_domain: String,
    pub grant_id: String,
    pub grant_digest: Digest,
    pub tenant_id: String,
    pub principal_id: String,
    pub episode_id: String,
    pub plan_digest: Digest,
    pub step_id: String,
    pub tool_id: String,
    pub operation_id: String,
    pub manifest_version: String,
    pub manifest_digest: Digest,
    pub toolchain_id: String,
    pub toolchain_digest: Digest,
    pub policy_version: String,
    pub policy_digest: Digest,
    pub target_digest: Digest,
    pub request_digest: Digest,
    pub participant_id: String,
    pub required_scope: String,
    pub attempt_id: String,
    pub idempotency_key: String,
    pub environment_digest: Digest,
    pub risk_level: u8,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActRuntimeEvidence {
    /// The runtime observer/challenge authority appraises this exact Candidate Act.
    pub act_digest: Digest,
    pub evidence_digest: Digest,
    pub run_id: String,
    pub trial_id: String,
    pub arm: EvaluationArm,
    pub arm_identity_digest: Digest,
    pub scenario_commitment: Digest,
    pub manifest_commitment: Digest,
    pub environment_digest: Digest,
    pub policy_digest: Digest,
    pub runtime_instance_digest: Digest,
    pub writable_namespace_digest: Digest,
    pub observer_source_id: String,
    pub observer_control_domain: String,
    pub observer_adapter_digest: Digest,
    pub challenge_authority_id: String,
    pub challenge_authority_control_domain: String,
    pub challenge_receipt_digest: Digest,
    pub challenge_nonce_digest: Digest,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub clock_uncertainty_ms: u64,
    /// Monotonic event-stream high-water sequence included in the appraisal.
    pub observer_high_water_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinalityAuthorityProfile {
    pub authority_id: String,
    pub authority_control_domain: String,
    pub broker_id: String,
    pub broker_control_domain: String,
    pub expected_observer_source_id: String,
    pub expected_observer_control_domain: String,
    pub expected_observer_adapter_digest: Digest,
    pub expected_challenge_authority_id: String,
    pub expected_challenge_authority_control_domain: String,
    pub expected_runtime_instance_digest: Digest,
    pub expected_manifest_commitment: Digest,
    pub expected_environment_digest: Digest,
    pub expected_policy_digest: Digest,
    pub max_evidence_age_ms: u64,
    pub max_handle_ttl_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinalityIssueRequest {
    pub handle_id: Digest,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub finality_sink_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionHandle {
    profile: String,
    handle_id: Digest,
    act_digest: Digest,
    request_digest: Digest,
    participant_id: String,
    finality_sink_id: String,
    tenant_id: String,
    principal_id: String,
    grant_id: String,
    grant_digest: Digest,
    manifest_digest: Digest,
    toolchain_digest: Digest,
    policy_digest: Digest,
    target_digest: Digest,
    attempt_id: String,
    idempotency_key: String,
    environment_digest: Digest,
    runtime_evidence_digest: Digest,
    challenge_receipt_digest: Digest,
    challenge_nonce_digest: Digest,
    broker_receipt_digest: Digest,
    issued_at_ms: u64,
    expires_at_ms: u64,
    issuer_id: String,
    issuer_control_domain: String,
}

impl ExecutionHandle {
    pub fn handle_id(&self) -> &Digest { &self.handle_id }
    pub fn act_digest(&self) -> &Digest { &self.act_digest }
    pub fn request_digest(&self) -> &Digest { &self.request_digest }
    pub fn participant_id(&self) -> &str { &self.participant_id }
    pub fn finality_sink_id(&self) -> &str { &self.finality_sink_id }
    pub fn issued_at_ms(&self) -> u64 { self.issued_at_ms }
    pub fn expires_at_ms(&self) -> u64 { self.expires_at_ms }
}

pub fn issue_execution_handle(
    profile: &FinalityAuthorityProfile,
    act: &CandidateAct,
    broker_receipt: &BrokerAuthorizationReceipt,
    runtime: &ActRuntimeEvidence,
    issue: &FinalityIssueRequest,
    now_ms: u64,
    digester: &dyn DigestProvider,
) -> Result<ExecutionHandle, FinalityError> {
    validate_profile(profile)?;
    validate_act(act)?;
    validate_broker_binding(profile, act, broker_receipt, now_ms, digester)?;
    let act_digest = act.digest(digester)?;
    let request_digest = act.request_digest(digester)?;
    validate_runtime_binding(profile, act, runtime, &act_digest, now_ms)?;
    if is_zero(&issue.handle_id) {
        return Err(FinalityError::ZeroHandleId);
    }
    if issue.finality_sink_id != act.finality_sink_id
        || issue.finality_sink_id != act.participant_id
    {
        return Err(FinalityError::FinalitySinkAudienceMismatch);
    }
    if issue.issued_at_ms != now_ms
        || issue.expires_at_ms <= issue.issued_at_ms
        || issue.expires_at_ms > runtime.expires_at_ms
        || issue.expires_at_ms - issue.issued_at_ms > profile.max_handle_ttl_ms
        || issue.expires_at_ms - issue.issued_at_ms > MAX_FINALITY_HANDLE_TTL_MS
    {
        return Err(FinalityError::HandleLifetimeInvalid);
    }
    Ok(ExecutionHandle {
        profile: EXECUTION_HANDLE_PROFILE.into(),
        handle_id: issue.handle_id.clone(),
        act_digest,
        request_digest,
        participant_id: act.participant_id.clone(),
        finality_sink_id: act.finality_sink_id.clone(),
        tenant_id: act.tenant_id.clone(),
        principal_id: act.principal_id.clone(),
        grant_id: act.grant_id.clone(),
        grant_digest: act.grant_digest.clone(),
        manifest_digest: act.manifest_digest.clone(),
        toolchain_digest: act.toolchain_digest.clone(),
        policy_digest: act.policy_digest.clone(),
        target_digest: act.target_digest.clone(),
        attempt_id: act.attempt_id.clone(),
        idempotency_key: act.idempotency_key.clone(),
        environment_digest: act.environment_digest.clone(),
        runtime_evidence_digest: runtime.evidence_digest.clone(),
        challenge_receipt_digest: runtime.challenge_receipt_digest.clone(),
        challenge_nonce_digest: runtime.challenge_nonce_digest.clone(),
        broker_receipt_digest: broker_receipt.receipt_digest.clone(),
        issued_at_ms: issue.issued_at_ms,
        expires_at_ms: issue.expires_at_ms,
        issuer_id: profile.authority_id.clone(),
        issuer_control_domain: profile.authority_control_domain.clone(),
    })
}

fn validate_profile(profile: &FinalityAuthorityProfile) -> Result<(), FinalityError> {
    for value in [
        profile.authority_id.as_str(),
        profile.authority_control_domain.as_str(),
        profile.broker_id.as_str(),
        profile.broker_control_domain.as_str(),
        profile.expected_observer_source_id.as_str(),
        profile.expected_observer_control_domain.as_str(),
        profile.expected_challenge_authority_id.as_str(),
        profile.expected_challenge_authority_control_domain.as_str(),
    ] {
        validate_text("profile identity", value)?;
    }
    if profile.authority_control_domain == profile.broker_control_domain
        || profile.authority_control_domain == profile.expected_observer_control_domain
        || profile.authority_control_domain == profile.expected_challenge_authority_control_domain
        || profile.broker_control_domain == profile.expected_observer_control_domain
        || profile.broker_control_domain == profile.expected_challenge_authority_control_domain
        || profile.expected_observer_control_domain == profile.expected_challenge_authority_control_domain
        || profile.max_evidence_age_ms == 0
        || profile.max_handle_ttl_ms == 0
        || profile.max_handle_ttl_ms > MAX_FINALITY_HANDLE_TTL_MS
        || is_zero(&profile.expected_observer_adapter_digest)
        || is_zero(&profile.expected_runtime_instance_digest)
        || is_zero(&profile.expected_manifest_commitment)
        || is_zero(&profile.expected_environment_digest)
        || is_zero(&profile.expected_policy_digest)
    {
        return Err(FinalityError::TrustDomainOrProfileInvalid);
    }
    Ok(())
}

fn validate_act(act: &CandidateAct) -> Result<(), FinalityError> {
    for (label, value) in [
        ("tenant_id", act.tenant_id.as_str()),
        ("principal_id", act.principal_id.as_str()),
        ("episode_id", act.episode_id.as_str()),
        ("step_id", act.step_id.as_str()),
        ("grant_id", act.grant_id.as_str()),
        ("grant_scope", act.grant_scope.as_str()),
        ("tool_id", act.tool_id.as_str()),
        ("operation_id", act.operation_id.as_str()),
        ("manifest_version", act.manifest_version.as_str()),
        ("toolchain_id", act.toolchain_id.as_str()),
        ("policy_version", act.policy_version.as_str()),
        ("participant_id", act.participant_id.as_str()),
        ("finality_sink_id", act.finality_sink_id.as_str()),
        ("attempt_id", act.attempt_id.as_str()),
        ("idempotency_key", act.idempotency_key.as_str()),
        ("run_id", act.run_id.as_str()),
        ("trial_id", act.trial_id.as_str()),
    ] {
        validate_text(label, value)?;
    }
    for digest in [
        &act.plan_digest,
        &act.grant_digest,
        &act.manifest_digest,
        &act.toolchain_digest,
        &act.policy_digest,
        &act.target_digest,
        &act.environment_digest,
        &act.arm_identity_digest,
        &act.scenario_commitment,
        &act.runtime_evidence_digest,
        &act.broker_receipt_digest,
    ] {
        if is_zero(digest) {
            return Err(FinalityError::ZeroActDigest);
        }
    }
    if act.canonical_request_bytes.is_empty()
        || act.canonical_request_bytes.len() > 1_048_576
        || act.risk_level > 5
        || act.grant_scope.trim().is_empty()
        || act.participant_id != act.finality_sink_id
    {
        return Err(FinalityError::ActMalformed);
    }
    Ok(())
}

fn validate_broker_binding(
    profile: &FinalityAuthorityProfile,
    act: &CandidateAct,
    receipt: &BrokerAuthorizationReceipt,
    now_ms: u64,
    digester: &dyn DigestProvider,
) -> Result<(), FinalityError> {
    let request_digest = act.request_digest(digester)?;
    if receipt.broker_id != profile.broker_id
        || receipt.broker_control_domain != profile.broker_control_domain
        || receipt.grant_id != act.grant_id
        || receipt.grant_digest != act.grant_digest
        || receipt.tenant_id != act.tenant_id
        || receipt.principal_id != act.principal_id
        || receipt.episode_id != act.episode_id
        || receipt.plan_digest != act.plan_digest
        || receipt.step_id != act.step_id
        || receipt.tool_id != act.tool_id
        || receipt.operation_id != act.operation_id
        || receipt.manifest_version != act.manifest_version
        || receipt.manifest_digest != act.manifest_digest
        || receipt.toolchain_id != act.toolchain_id
        || receipt.toolchain_digest != act.toolchain_digest
        || receipt.policy_version != act.policy_version
        || receipt.policy_digest != act.policy_digest
        || receipt.target_digest != act.target_digest
        || receipt.request_digest != request_digest
        || receipt.participant_id != act.participant_id
        || receipt.required_scope != act.grant_scope
        || receipt.attempt_id != act.attempt_id
        || receipt.idempotency_key != act.idempotency_key
        || receipt.environment_digest != act.environment_digest
        || receipt.risk_level != act.risk_level
        || is_zero(&receipt.receipt_digest)
        || is_zero(&receipt.grant_digest)
        || receipt.expires_at_ms <= now_ms
        || receipt.issued_at_ms > now_ms
    {
        return Err(FinalityError::BrokerAuthorizationBindingMismatch);
    }
    Ok(())
}

fn validate_runtime_binding(
    profile: &FinalityAuthorityProfile,
    act: &CandidateAct,
    runtime: &ActRuntimeEvidence,
    expected_act_digest: &Digest,
    now_ms: u64,
) -> Result<(), FinalityError> {
    if runtime.run_id != act.run_id
        || runtime.trial_id != act.trial_id
        || runtime.arm != act.arm
        || runtime.arm_identity_digest != act.arm_identity_digest
        || runtime.scenario_commitment != act.scenario_commitment
        || runtime.manifest_commitment != profile.expected_manifest_commitment
        || runtime.environment_digest != profile.expected_environment_digest
        || runtime.environment_digest != act.environment_digest
        || runtime.policy_digest != profile.expected_policy_digest
        || runtime.policy_digest != act.policy_digest
        || runtime.runtime_instance_digest != profile.expected_runtime_instance_digest
        || runtime.observer_source_id != profile.expected_observer_source_id
        || runtime.observer_control_domain != profile.expected_observer_control_domain
        || runtime.observer_adapter_digest != profile.expected_observer_adapter_digest
        || runtime.challenge_authority_id != profile.expected_challenge_authority_id
        || runtime.challenge_authority_control_domain != profile.expected_challenge_authority_control_domain
        || runtime.act_digest != *expected_act_digest
        || is_zero(&runtime.evidence_digest)
        || is_zero(&runtime.writable_namespace_digest)
        || is_zero(&runtime.challenge_receipt_digest)
        || is_zero(&runtime.challenge_nonce_digest)
        || runtime.observer_high_water_sequence == 0
        || runtime.clock_uncertainty_ms > MAX_FINALITY_CLOCK_UNCERTAINTY_MS
        || runtime.expires_at_ms <= now_ms
        || runtime.issued_at_ms > now_ms
        || now_ms.saturating_sub(runtime.issued_at_ms) > profile.max_evidence_age_ms
        || runtime.expires_at_ms <= runtime.issued_at_ms
    {
        return Err(FinalityError::RuntimeEvidenceBindingMismatch);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParticipantRequest {
    pub participant_id: String,
    pub finality_sink_id: String,
    pub act: CandidateAct,
    /// The exact request bytes received by the participant effect boundary.
    pub received_request_bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParticipantEffectReceipt {
    pub effect_id: Digest,
    pub handle_id: Digest,
    pub handle_digest: Digest,
    pub act_digest: Digest,
    pub request_digest: Digest,
    pub broker_receipt_digest: Digest,
    pub runtime_evidence_digest: Digest,
    pub challenge_receipt_digest: Digest,
    pub tenant_id: String,
    pub participant_id: String,
    pub attempt_id: String,
    pub idempotency_key: String,
    pub committed_at_ms: u64,
    pub ledger_cursor: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParticipantEffectOutcome {
    Committed(ParticipantEffectReceipt),
    IdempotentReplay(ParticipantEffectReceipt),
}

pub trait ParticipantFinalitySink: Send + Sync {
    /// Implementations MUST verify the exact received request and atomically consume
    /// the one-use handle with the protected effect in the participant's own ledger.
    fn consume_and_commit(
        &self,
        handle: &ExecutionHandle,
        received: &ParticipantRequest,
        now_ms: u64,
        digester: &dyn DigestProvider,
    ) -> Result<ParticipantEffectOutcome, FinalityError>;

    fn lookup_by_idempotency(
        &self,
        tenant_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<ParticipantEffectReceipt>, FinalityError>;
}

#[derive(Default)]
struct ParticipantLedgerState {
    consumed_handles: BTreeSet<Digest>,
    effects_by_idempotency: BTreeMap<(String, String), ParticipantEffectReceipt>,
    next_cursor: u64,
}

pub struct ReferenceParticipantLedger {
    participant_id: String,
    sink_id: String,
    state: Mutex<ParticipantLedgerState>,
}

impl ReferenceParticipantLedger {
    pub fn new(participant_id: &str, sink_id: &str) -> Result<Self, FinalityError> {
        validate_text("participant_id", participant_id)?;
        validate_text("sink_id", sink_id)?;
        if participant_id != sink_id {
            return Err(FinalityError::FinalitySinkAudienceMismatch);
        }
        Ok(Self {
            participant_id: participant_id.into(),
            sink_id: sink_id.into(),
            state: Mutex::new(ParticipantLedgerState { next_cursor: 1, ..Default::default() }),
        })
    }

    pub fn committed_effect_count(&self) -> Result<usize, FinalityError> {
        let state = self.state.lock().map_err(|_| FinalityError::ParticipantLedgerPoisoned)?;
        Ok(state.effects_by_idempotency.len())
    }
}

impl ParticipantFinalitySink for ReferenceParticipantLedger {
    fn consume_and_commit(
        &self,
        handle: &ExecutionHandle,
        received: &ParticipantRequest,
        now_ms: u64,
        digester: &dyn DigestProvider,
    ) -> Result<ParticipantEffectOutcome, FinalityError> {
        if handle.profile != EXECUTION_HANDLE_PROFILE
            || received.participant_id != self.participant_id
            || received.finality_sink_id != self.sink_id
            || handle.participant_id != self.participant_id
            || handle.finality_sink_id != self.sink_id
            || handle.participant_id != received.participant_id
            || handle.finality_sink_id != received.finality_sink_id
        {
            return Err(FinalityError::FinalitySinkAudienceMismatch);
        }
        validate_act(&received.act)?;
        let received_canonical = received.act.canonical_bytes()?;
        if received.received_request_bytes != received.act.canonical_request_bytes {
            return Err(FinalityError::ReceivedRequestBytesMismatch);
        }
        let act_digest = received.act.digest(digester)?;
        let request_digest = received.act.request_digest(digester)?;
        let handle_digest = digest_handle(handle, digester)?;

        if handle.act_digest != act_digest
            || handle.request_digest != request_digest
            || handle.tenant_id != received.act.tenant_id
            || handle.principal_id != received.act.principal_id
            || handle.grant_id != received.act.grant_id
            || handle.grant_digest != received.act.grant_digest
            || handle.manifest_digest != received.act.manifest_digest
            || handle.toolchain_digest != received.act.toolchain_digest
            || handle.policy_digest != received.act.policy_digest
            || handle.target_digest != received.act.target_digest
            || handle.attempt_id != received.act.attempt_id
            || handle.idempotency_key != received.act.idempotency_key
            || handle.environment_digest != received.act.environment_digest
            || handle.broker_receipt_digest != received.act.broker_receipt_digest
        {
            return Err(FinalityError::HandleActBindingMismatch);
        }
        if now_ms < handle.issued_at_ms || now_ms >= handle.expires_at_ms {
            return Err(FinalityError::HandleExpired);
        }
        if is_zero(&handle.handle_id) || is_zero(&handle.act_digest) || is_zero(&handle.request_digest) {
            return Err(FinalityError::HandleMalformed);
        }
        let _canonical_len = received_canonical.len();

        // One participant-ledger lock protects handle consumption and effect insertion.
        // The production adapter must do this in one durable participant transaction.
        let mut state = self.state.lock().map_err(|_| FinalityError::ParticipantLedgerPoisoned)?;
        if state.consumed_handles.contains(&handle.handle_id) {
            return Err(FinalityError::HandleReplay);
        }
        let idem_key = (received.act.tenant_id.clone(), received.act.idempotency_key.clone());
        if let Some(existing) = state.effects_by_idempotency.get(&idem_key) {
            if existing.act_digest != act_digest || existing.request_digest != request_digest {
                return Err(FinalityError::IdempotencyConflict);
            }
            state.consumed_handles.insert(handle.handle_id.clone());
            return Ok(ParticipantEffectOutcome::IdempotentReplay(existing.clone()));
        }

        let cursor = state.next_cursor;
        state.next_cursor = state.next_cursor.checked_add(1).ok_or(FinalityError::CursorOverflow)?;
        let effect_id = digester.digest(
            b"org.luminousdynamics.opsbench.participant-effect.v0.1\0",
            &[
                handle.handle_id.0.as_slice(),
                act_digest.0.as_slice(),
                cursor.to_be_bytes().as_slice(),
            ].concat(),
        );
        let receipt = ParticipantEffectReceipt {
            effect_id,
            handle_id: handle.handle_id.clone(),
            handle_digest,
            act_digest,
            request_digest,
            broker_receipt_digest: handle.broker_receipt_digest.clone(),
            runtime_evidence_digest: handle.runtime_evidence_digest.clone(),
            challenge_receipt_digest: handle.challenge_receipt_digest.clone(),
            tenant_id: received.act.tenant_id.clone(),
            participant_id: self.participant_id.clone(),
            attempt_id: received.act.attempt_id.clone(),
            idempotency_key: received.act.idempotency_key.clone(),
            committed_at_ms: now_ms,
            ledger_cursor: cursor,
        };
        // Consume the handle and commit the effect together in this ledger state transition.
        state.consumed_handles.insert(handle.handle_id.clone());
        state.effects_by_idempotency.insert(idem_key, receipt.clone());
        Ok(ParticipantEffectOutcome::Committed(receipt))
    }

    fn lookup_by_idempotency(
        &self,
        tenant_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<ParticipantEffectReceipt>, FinalityError> {
        let state = self.state.lock().map_err(|_| FinalityError::ParticipantLedgerPoisoned)?;
        Ok(state.effects_by_idempotency.get(&(tenant_id.into(), idempotency_key.into())).cloned())
    }
}

pub fn digest_handle(
    handle: &ExecutionHandle,
    digester: &dyn DigestProvider,
) -> Result<Digest, FinalityError> {
    let mut out = Vec::new();
    out.extend_from_slice(b"org.luminousdynamics.opsbench.execution-handle.v0.1\0");
    put_text(&mut out, &handle.profile)?;
    put_digest(&mut out, &handle.handle_id);
    put_digest(&mut out, &handle.act_digest);
    put_digest(&mut out, &handle.request_digest);
    put_text(&mut out, &handle.participant_id)?;
    put_text(&mut out, &handle.finality_sink_id)?;
    put_text(&mut out, &handle.tenant_id)?;
    put_text(&mut out, &handle.principal_id)?;
    put_text(&mut out, &handle.grant_id)?;
    put_digest(&mut out, &handle.grant_digest);
    put_digest(&mut out, &handle.manifest_digest);
    put_digest(&mut out, &handle.toolchain_digest);
    put_digest(&mut out, &handle.policy_digest);
    put_digest(&mut out, &handle.target_digest);
    put_text(&mut out, &handle.attempt_id)?;
    put_text(&mut out, &handle.idempotency_key)?;
    put_digest(&mut out, &handle.environment_digest);
    put_digest(&mut out, &handle.runtime_evidence_digest);
    put_digest(&mut out, &handle.challenge_receipt_digest);
    put_digest(&mut out, &handle.challenge_nonce_digest);
    put_digest(&mut out, &handle.broker_receipt_digest);
    out.extend_from_slice(&handle.issued_at_ms.to_be_bytes());
    out.extend_from_slice(&handle.expires_at_ms.to_be_bytes());
    put_text(&mut out, &handle.issuer_id)?;
    put_text(&mut out, &handle.issuer_control_domain)?;
    Ok(digester.digest(
        b"org.luminousdynamics.opsbench.execution-handle.digest.v0.1\0",
        &out,
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FinalityError {
    UnsupportedActProfile,
    ActMalformed,
    EmptyRequest,
    MissingIdentity(&'static str),
    InvalidText(&'static str),
    ZeroActDigest,
    ZeroHandleId,
    TrustDomainOrProfileInvalid,
    BrokerAuthorizationBindingMismatch,
    RuntimeEvidenceBindingMismatch,
    RuntimeEvidenceExpired,
    RuntimeEvidenceTooOld,
    RuntimeEvidenceClockUncertaintyExceeded,
    FinalitySinkAudienceMismatch,
    HandleLifetimeInvalid,
    HandleExpired,
    HandleMalformed,
    HandleActBindingMismatch,
    HandleReplay,
    IdempotencyConflict,
    ReceivedRequestBytesMismatch,
    ParticipantLedgerPoisoned,
    CursorOverflow,
}

fn validate_text(label: &'static str, value: &str) -> Result<(), FinalityError> {
    if value.trim().is_empty() {
        return Err(FinalityError::MissingIdentity(label));
    }
    if value.len() > 256 || value.chars().any(char::is_control) {
        return Err(FinalityError::InvalidText(label));
    }
    Ok(())
}

fn put_text(out: &mut Vec<u8>, value: &str) -> Result<(), FinalityError> {
    validate_text("canonical field", value)?;
    put_bytes(out, value.as_bytes())
}

fn put_bytes(out: &mut Vec<u8>, value: &[u8]) -> Result<(), FinalityError> {
    let len = u32::try_from(value.len()).map_err(|_| FinalityError::ActMalformed)?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(value);
    Ok(())
}

fn put_digest(out: &mut Vec<u8>, digest: &Digest) {
    out.extend_from_slice(&digest.0);
}

fn is_zero(digest: &Digest) -> bool {
    digest.0.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    struct TestDigest;
    impl DigestProvider for TestDigest {
        fn digest(&self, domain: &'static [u8], bytes: &[u8]) -> Digest {
            // Deterministic test-only mixer. It is intentionally NOT cryptographic and
            // must never be used by a production adapter.
            let mut out = [0u8; 32];
            for (index, byte) in domain.iter().chain(bytes.iter()).enumerate() {
                let slot = index % 32;
                out[slot] = out[slot].wrapping_add(*byte).rotate_left((slot % 7) as u32)
                    ^ (index as u8).wrapping_mul(31);
            }
            Digest(out)
        }
    }

    fn digest(byte: u8) -> Digest { Digest::from_byte(byte) }

    fn act() -> CandidateAct {
        CandidateAct {
            profile: CANDIDATE_ACT_PROFILE.into(),
            tenant_id: "tenant-a".into(),
            principal_id: "principal-a".into(),
            episode_id: "episode-a".into(),
            plan_digest: digest(1),
            step_id: "step-1".into(),
            grant_id: "grant-1".into(),
            grant_digest: digest(2),
            grant_scope: "service:restart".into(),
            tool_id: "systemd".into(),
            operation_id: "restart-unit".into(),
            manifest_version: "v1".into(),
            manifest_digest: digest(3),
            toolchain_id: "nixward".into(),
            toolchain_digest: digest(4),
            policy_version: "policy-1".into(),
            policy_digest: digest(5),
            risk_level: 2,
            participant_id: "participant-a".into(),
            finality_sink_id: "participant-a".into(),
            target_digest: digest(6),
            canonical_request_bytes: b"{\"unit\":\"api.service\",\"action\":\"restart\"}".to_vec(),
            attempt_id: "attempt-1".into(),
            idempotency_key: "idem-1".into(),
            environment_digest: digest(7),
            run_id: "run-1".into(),
            trial_id: "trial-1".into(),
            arm: EvaluationArm::Candidate,
            arm_identity_digest: digest(8),
            scenario_commitment: digest(9),
            runtime_evidence_digest: digest(10),
            broker_receipt_digest: digest(11),
            created_at_ms: 10,
        }
    }

    fn profile() -> FinalityAuthorityProfile {
        FinalityAuthorityProfile {
            authority_id: "finality-authority".into(),
            authority_control_domain: "finality-domain".into(),
            broker_id: "broker".into(),
            broker_control_domain: "broker-domain".into(),
            expected_observer_source_id: "observer".into(),
            expected_observer_control_domain: "observer-domain".into(),
            expected_observer_adapter_digest: digest(12),
            expected_challenge_authority_id: "challenge-authority".into(),
            expected_challenge_authority_control_domain: "challenge-domain".into(),
            expected_runtime_instance_digest: digest(13),
            expected_manifest_commitment: digest(14),
            expected_environment_digest: digest(7),
            expected_policy_digest: digest(5),
            max_evidence_age_ms: 5_000,
            max_handle_ttl_ms: 10_000,
        }
    }

    fn broker(act: &CandidateAct, digester: &dyn DigestProvider) -> BrokerAuthorizationReceipt {
        BrokerAuthorizationReceipt {
            receipt_digest: digest(15),
            broker_id: "broker".into(),
            broker_control_domain: "broker-domain".into(),
            grant_id: act.grant_id.clone(),
            grant_digest: act.grant_digest.clone(),
            tenant_id: act.tenant_id.clone(),
            principal_id: act.principal_id.clone(),
            episode_id: act.episode_id.clone(),
            plan_digest: act.plan_digest.clone(),
            step_id: act.step_id.clone(),
            tool_id: act.tool_id.clone(),
            operation_id: act.operation_id.clone(),
            manifest_version: act.manifest_version.clone(),
            manifest_digest: act.manifest_digest.clone(),
            toolchain_id: act.toolchain_id.clone(),
            toolchain_digest: act.toolchain_digest.clone(),
            policy_version: act.policy_version.clone(),
            policy_digest: act.policy_digest.clone(),
            target_digest: act.target_digest.clone(),
            request_digest: act.request_digest(digester).unwrap(),
            participant_id: act.participant_id.clone(),
            required_scope: act.grant_scope.clone(),
            attempt_id: act.attempt_id.clone(),
            idempotency_key: act.idempotency_key.clone(),
            environment_digest: act.environment_digest.clone(),
            risk_level: act.risk_level,
            issued_at_ms: 90,
            expires_at_ms: 200,
        }
    }

    fn runtime(act: &CandidateAct, profile: &FinalityAuthorityProfile, digester: &dyn DigestProvider) -> ActRuntimeEvidence {
        ActRuntimeEvidence {
            act_digest: act.digest(digester).unwrap(),
            evidence_digest: digest(16),
            run_id: act.run_id.clone(),
            trial_id: act.trial_id.clone(),
            arm: act.arm,
            arm_identity_digest: act.arm_identity_digest.clone(),
            scenario_commitment: act.scenario_commitment.clone(),
            manifest_commitment: profile.expected_manifest_commitment.clone(),
            environment_digest: act.environment_digest.clone(),
            policy_digest: act.policy_digest.clone(),
            runtime_instance_digest: profile.expected_runtime_instance_digest.clone(),
            writable_namespace_digest: digest(17),
            observer_source_id: profile.expected_observer_source_id.clone(),
            observer_control_domain: profile.expected_observer_control_domain.clone(),
            observer_adapter_digest: profile.expected_observer_adapter_digest.clone(),
            challenge_authority_id: profile.expected_challenge_authority_id.clone(),
            challenge_authority_control_domain: profile.expected_challenge_authority_control_domain.clone(),
            challenge_receipt_digest: digest(18),
            challenge_nonce_digest: digest(19),
            issued_at_ms: 95,
            expires_at_ms: 180,
            clock_uncertainty_ms: 5,
            observer_high_water_sequence: 4,
        }
    }

    fn issue(act: &CandidateAct) -> ExecutionHandle {
        let d = TestDigest;
        let p = profile();
        let grant = broker(act, &d);
        let runtime = runtime(act, &p, &d);
        issue_execution_handle(
            &p, act, &grant, &runtime,
            &FinalityIssueRequest {
                handle_id: digest(20),
                issued_at_ms: 100,
                expires_at_ms: 120,
                finality_sink_id: "participant-a".into(),
            },
            100,
            &d,
        ).unwrap()
    }

    fn participant_request(act: &CandidateAct) -> ParticipantRequest {
        ParticipantRequest {
            participant_id: act.participant_id.clone(),
            finality_sink_id: act.finality_sink_id.clone(),
            received_request_bytes: act.canonical_request_bytes.clone(),
            act: act.clone(),
        }
    }

    #[test]
    fn exact_candidate_act_is_consumed_once_with_participant_receipt_binding() {
        let d = TestDigest;
        let act = act();
        let handle = issue(&act);
        let ledger = ReferenceParticipantLedger::new("participant-a", "participant-a").unwrap();
        let result = ledger.consume_and_commit(&handle, &participant_request(&act), 101, &d).unwrap();
        match result {
            ParticipantEffectOutcome::Committed(receipt) => {
                assert_eq!(receipt.act_digest, handle.act_digest);
                assert_eq!(receipt.request_digest, handle.request_digest);
                assert_eq!(receipt.handle_digest, digest_handle(&handle, &d).unwrap());
                assert_eq!(receipt.participant_id, "participant-a");
            }
            _ => panic!("first exact act must commit"),
        }
        assert_eq!(ledger.committed_effect_count().unwrap(), 1);
        assert_eq!(
            ledger.consume_and_commit(&handle, &participant_request(&act), 102, &d),
            Err(FinalityError::HandleReplay)
        );
    }

    #[test]
    fn changed_request_bytes_or_target_cannot_reuse_handle() {
        let d = TestDigest;
        let act = act();
        let handle = issue(&act);
        let ledger = ReferenceParticipantLedger::new("participant-a", "participant-a").unwrap();
        let mut changed = participant_request(&act);
        changed.received_request_bytes = b"{\"unit\":\"other.service\",\"action\":\"restart\"}".to_vec();
        assert_eq!(
            ledger.consume_and_commit(&handle, &changed, 101, &d),
            Err(FinalityError::ReceivedRequestBytesMismatch)
        );

        let mut changed = participant_request(&act);
        changed.act.target_digest = digest(99);
        assert_eq!(
            ledger.consume_and_commit(&handle, &changed, 101, &d),
            Err(FinalityError::HandleActBindingMismatch)
        );
        assert_eq!(ledger.committed_effect_count().unwrap(), 0);
    }

    #[test]
    fn exact_request_payload_and_all_act_security_fields_are_bound() {
        let d = TestDigest;
        let base = act();
        let base_handle = issue(&base);
        let mutate: [fn(&mut CandidateAct); 21] = [
            |a| a.tenant_id = "other-tenant".into(),
            |a| a.principal_id = "other-principal".into(),
            |a| a.episode_id = "other-episode".into(),
            |a| a.plan_digest = digest(90),
            |a| a.step_id = "other-step".into(),
            |a| a.grant_id = "other-grant".into(),
            |a| a.grant_digest = digest(91),
            |a| a.grant_scope = "service:delete".into(),
            |a| a.tool_id = "other-tool".into(),
            |a| a.operation_id = "delete".into(),
            |a| a.manifest_digest = digest(92),
            |a| a.toolchain_digest = digest(93),
            |a| a.policy_digest = digest(94),
            |a| a.risk_level = 5,
            |a| a.participant_id = "other-participant".into(),
            |a| a.target_digest = digest(95),
            |a| a.attempt_id = "other-attempt".into(),
            |a| a.idempotency_key = "other-idem".into(),
            |a| a.environment_digest = digest(96),
            |a| a.runtime_evidence_digest = digest(97),
            |a| a.broker_receipt_digest = digest(98),
        ];
        for mutate in mutate {
            let mut changed = base.clone();
            mutate(&mut changed);
            let request = participant_request(&changed);
            let ledger = ReferenceParticipantLedger::new("participant-a", "participant-a").unwrap();
            assert_eq!(
                ledger.consume_and_commit(&base_handle, &request, 101, &d),
                Err(FinalityError::HandleActBindingMismatch)
            );
            assert_eq!(ledger.committed_effect_count().unwrap(), 0);
        }
    }

    #[test]
    fn stale_or_foreign_runtime_evidence_cannot_issue_handle() {
        let d = TestDigest;
        let act = act();
        let p = profile();
        let grant = broker(&act, &d);
        let mut rt = runtime(&act, &p, &d);
        rt.issued_at_ms = 80;
        assert_eq!(
            issue_execution_handle(&p, &act, &grant, &rt, &FinalityIssueRequest {
                handle_id: digest(20), issued_at_ms: 100, expires_at_ms: 120,
                finality_sink_id: "participant-a".into(),
            }, 100, &d),
            Err(FinalityError::RuntimeEvidenceTooOld)
        );
        let mut rt = runtime(&act, &p, &d);
        rt.challenge_receipt_digest = digest(0);
        assert_eq!(
            issue_execution_handle(&p, &act, &grant, &rt, &FinalityIssueRequest {
                handle_id: digest(20), issued_at_ms: 100, expires_at_ms: 120,
                finality_sink_id: "participant-a".into(),
            }, 100, &d),
            Err(FinalityError::RuntimeEvidenceBindingMismatch)
        );
    }

    #[test]
    fn broker_receipt_must_bind_every_dispatch_field_before_handle_issue() {
        let d = TestDigest;
        let act = act();
        let p = profile();
        let mut grant = broker(&act, &d);
        grant.request_digest = digest(99);
        assert_eq!(
            issue_execution_handle(&p, &act, &grant, &runtime(&act, &p, &d), &FinalityIssueRequest {
                handle_id: digest(20), issued_at_ms: 100, expires_at_ms: 120,
                finality_sink_id: "participant-a".into(),
            }, 100, &d),
            Err(FinalityError::BrokerAuthorizationBindingMismatch)
        );
    }

    #[test]
    fn participant_id_is_audience_and_sink_cannot_be_substituted() {
        let d = TestDigest;
        let act = act();
        let handle = issue(&act);
        let wrong_ledger = ReferenceParticipantLedger::new("participant-b", "participant-b").unwrap();
        assert_eq!(
            wrong_ledger.consume_and_commit(&handle, &ParticipantRequest {
                participant_id: "participant-b".into(),
                finality_sink_id: "participant-b".into(),
                act: act.clone(),
                received_request_bytes: act.canonical_request_bytes.clone(),
            }, 101, &d),
            Err(FinalityError::FinalitySinkAudienceMismatch)
        );
    }

    #[test]
    fn expiry_and_handle_ttl_are_hard_limits() {
        let d = TestDigest;
        let act = act();
        let p = profile();
        let grant = broker(&act, &d);
        let rt = runtime(&act, &p, &d);
        assert_eq!(
            issue_execution_handle(&p, &act, &grant, &rt, &FinalityIssueRequest {
                handle_id: digest(20), issued_at_ms: 100, expires_at_ms: 131,
                finality_sink_id: "participant-a".into(),
            }, 100, &d),
            Err(FinalityError::HandleLifetimeInvalid)
        );
        let handle = issue(&act);
        let ledger = ReferenceParticipantLedger::new("participant-a", "participant-a").unwrap();
        assert_eq!(
            ledger.consume_and_commit(&handle, &participant_request(&act), 120, &d),
            Err(FinalityError::HandleExpired)
        );
    }

    #[test]
    fn ambiguous_response_is_reconciled_from_participant_owned_ledger() {
        let d = TestDigest;
        let act = act();
        let handle = issue(&act);
        let ledger = ReferenceParticipantLedger::new("participant-a", "participant-a").unwrap();
        let _lost_response = ledger.consume_and_commit(&handle, &participant_request(&act), 101, &d).unwrap();
        // Simulated transport loss after participant commit. Recovery reads only
        // the participant-owned idempotency index; it never issues a blind retry.
        let recovered = ledger.lookup_by_idempotency("tenant-a", "idem-1").unwrap().unwrap();
        assert_eq!(recovered.act_digest, handle.act_digest);
        assert_eq!(recovered.handle_digest, digest_handle(&handle, &d).unwrap());
        assert_eq!(ledger.committed_effect_count().unwrap(), 1);
    }

    #[test]
    fn concurrent_replay_of_one_handle_cannot_commit_twice() {
        let d = Arc::new(TestDigest);
        let act = Arc::new(act());
        let handle = Arc::new(issue(&act));
        let ledger = Arc::new(ReferenceParticipantLedger::new("participant-a", "participant-a").unwrap());
        let mut workers = Vec::new();
        for _ in 0..2 {
            let d = Arc::clone(&d);
            let act = Arc::clone(&act);
            let handle = Arc::clone(&handle);
            let ledger = Arc::clone(&ledger);
            workers.push(thread::spawn(move || {
                ledger.consume_and_commit(&handle, &participant_request(&act), 101, d.as_ref())
            }));
        }
        let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|r| matches!(r, Ok(ParticipantEffectOutcome::Committed(_)))).count(), 1);
        assert_eq!(results.iter().filter(|r| matches!(r, Err(FinalityError::HandleReplay))).count(), 1);
        assert_eq!(ledger.committed_effect_count().unwrap(), 1);
    }

    #[test]
    fn different_act_needs_a_new_handle_but_idempotency_conflicts_never_overwrite() {
        let d = TestDigest;
        let act = act();
        let handle = issue(&act);
        let ledger = ReferenceParticipantLedger::new("participant-a", "participant-a").unwrap();
        ledger.consume_and_commit(&handle, &participant_request(&act), 101, &d).unwrap();

        let mut changed = act.clone();
        changed.canonical_request_bytes = b"{\"unit\":\"worker.service\",\"action\":\"restart\"}".to_vec();
        let p = profile();
        let grant = broker(&changed, &d);
        let rt = runtime(&changed, &p, &d);
        let fresh_handle = issue_execution_handle(&p, &changed, &grant, &rt, &FinalityIssueRequest {
            handle_id: digest(21), issued_at_ms: 102, expires_at_ms: 122,
            finality_sink_id: "participant-a".into(),
        }, 102, &d).unwrap();
        assert_eq!(
            ledger.consume_and_commit(&fresh_handle, &participant_request(&changed), 103, &d),
            Err(FinalityError::IdempotencyConflict)
        );
        assert_eq!(ledger.committed_effect_count().unwrap(), 1);
    }
}
