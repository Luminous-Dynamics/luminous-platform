//! Typed source-bound evidence gate for the agent-effect broker.
//!
//! Cryptographic verification is intentionally delegated to a trusted adapter that
//! validates DSSE/in-toto bytes and returns claims decoded from the verified payload.
//! This module owns the role, identity, binding, freshness, sequence and trust-domain
//! checks. The adapter and trust-policy source remain explicit trust roots.

use super::{
    AuthorityGrant, DispatchReport, EffectBroker,
    GateContext, GateDenial, ManifestRegistry, MutationProposal, ParticipantPort, ToolManifest,
};
use std::collections::{BTreeMap, BTreeSet};

pub const DSSE_IN_TOTO_PAYLOAD_TYPE: &str = "application/vnd.in-toto+json";
pub const IN_TOTO_STATEMENT_V1: &str = "https://in-toto.io/Statement/v1";
pub const GATE_EVIDENCE_PREDICATE_V1: &str =
    "https://luminousdynamics.io/attestation/agent-gate-evidence/v0.1";
pub const GATE_EVIDENCE_SCHEMA_V1: &str = "agent-gate-evidence-v0.1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceRole {
    Authority,
    Manifest,
    Policy,
    Preflight,
    Observer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClaim {
    CurrentGrant,
    ManifestQualified,
    PolicyAllows,
    PreconditionsSatisfied,
    ObserverCapable,
}

impl EvidenceRole {
    fn required_claim(self) -> EvidenceClaim {
        match self {
            Self::Authority => EvidenceClaim::CurrentGrant,
            Self::Manifest => EvidenceClaim::ManifestQualified,
            Self::Policy => EvidenceClaim::PolicyAllows,
            Self::Preflight => EvidenceClaim::PreconditionsSatisfied,
            Self::Observer => EvidenceClaim::ObserverCapable,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceBinding {
    pub tenant_id: String,
    pub principal_id: String,
    pub episode_id: String,
    pub plan_digest: super::Digest,
    pub step_id: String,
    pub tool_id: String,
    pub operation_id: String,
    pub manifest_version: String,
    pub manifest_digest: super::Digest,
    pub toolchain_id: String,
    pub toolchain_digest: super::Digest,
    pub policy_version: String,
    pub policy_digest: super::Digest,
    pub target_digest: super::Digest,
    pub request_digest: super::Digest,
    pub grant_id: String,
    pub attempt_id: String,
    pub idempotency_key: String,
    pub environment: String,
    pub risk_level: u8,
}

impl EvidenceBinding {
    pub fn from_proposal(p: &MutationProposal) -> Self {
        Self {
            tenant_id: p.tenant_id.clone(),
            principal_id: p.principal_id.clone(),
            episode_id: p.episode_id.clone(),
            plan_digest: p.plan_digest.clone(),
            step_id: p.step_id.clone(),
            tool_id: p.tool_id.clone(),
            operation_id: p.operation_id.clone(),
            manifest_version: p.manifest_version.clone(),
            manifest_digest: p.manifest_digest.clone(),
            toolchain_id: p.toolchain_id.clone(),
            toolchain_digest: p.toolchain_digest.clone(),
            policy_version: p.policy_version.clone(),
            policy_digest: p.policy_digest.clone(),
            target_digest: p.target_digest.clone(),
            request_digest: p.request_digest.clone(),
            grant_id: p.grant_id.clone(),
            attempt_id: p.attempt_id.clone(),
            idempotency_key: p.idempotency_key.clone(),
            environment: p.environment.clone(),
            risk_level: p.risk_level,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidencePayload {
    pub evidence_id: String,
    pub role: EvidenceRole,
    pub source_id: String,
    pub schema_version: String,
    pub binding: EvidenceBinding,
    pub claim: EvidenceClaim,
    pub issued_at_ms: u64,
    pub observed_at_ms: u64,
    pub expires_at_ms: u64,
    /// Must be monotonic for this source. The policy's minimum_sequence comes from
    /// an independently maintained sequence floor, not from the agent.
    pub sequence: u64,
    /// Digest of the exact evidence subject/record referenced by this statement.
    pub subject_digest: super::Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedPublisher {
    pub issuer: String,
    pub subject: String,
    pub trust_domain: String,
    pub key_fingerprint: String,
}

/// The verifier returns this structure only after checking the original DSSE envelope,
/// its PAE signature, payload type, in-toto Statement and decoded predicate. Implementors
/// must not construct it from unverified JSON or caller-provided identity strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedAttestation {
    pub dsse_payload_type: String,
    pub statement_type: String,
    pub predicate_type: String,
    pub publisher: VerifiedPublisher,
    pub payload: EvidencePayload,
    /// Present only for Authority and decoded from the verified signed predicate.
    pub authority_grant: Option<AuthorityGrant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttestationVerificationError {
    MalformedEnvelope,
    UnsupportedPayloadType,
    InvalidSignature,
    UntrustedIssuer,
    InvalidCertificateChain,
    MissingTransparencyProof,
    TransparencyInconsistent,
    UnsupportedPredicate,
    SubjectDigestMismatch,
}

pub trait AttestationVerifier {
    /// Verify DSSE using a supported implementation (for example, Sigstore/Cosign or a
    /// SPIFFE-bound runtime verifier), then return claims decoded from those exact bytes.
    fn verify_dsse(&mut self, envelope: &[u8])
        -> Result<VerifiedAttestation, AttestationVerificationError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedSourceEvidence {
    /// Opaque original DSSE envelope bytes. The agent never receives signing credentials.
    pub dsse_envelope: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateEvidenceBundle {
    pub authority: SignedSourceEvidence,
    pub manifest: SignedSourceEvidence,
    pub policy: SignedSourceEvidence,
    pub preflight: SignedSourceEvidence,
    pub observer: SignedSourceEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublisherRule {
    pub role: EvidenceRole,
    pub source_id: String,
    pub issuer: String,
    pub subject: String,
    pub trust_domain: String,
    /// Assigned by the verifier-owned registry. It is not accepted from the attestation.
    pub control_domain: String,
    pub key_fingerprint: String,
    pub schema_version: String,
    pub maximum_age_ms: u64,
    pub minimum_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustPolicy {
    pub policy_version: String,
    pub policy_digest: super::Digest,
    pub executor_control_domain: String,
    pub clock_source_id: String,
    pub clock_trust_domain: String,
    pub clock_control_domain: String,
    pub maximum_clock_uncertainty_ms: u64,
    pub publisher_rules: Vec<PublisherRule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClockReading {
    pub source_id: String,
    pub trust_domain: String,
    pub control_domain: String,
    pub now_ms: u64,
    pub uncertainty_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockError {
    Unavailable,
    Untrusted,
}

pub trait TrustedClock {
    /// Production implementations must use a qualified clock source and report a bounded
    /// uncertainty. The broker fails closed when the source is unavailable or untrusted.
    fn read(&mut self) -> Result<ClockReading, ClockError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceGateError {
    MissingPublisherRule,
    DuplicatePublisherRule,
    WrongDssePayloadType,
    WrongStatementType,
    WrongPredicateType,
    WrongSchemaVersion,
    WrongEvidenceRole,
    WrongSource,
    WrongPublisher,
    WrongTrustDomain,
    WrongControlDomain,
    ObserverNotIndependent,
    WrongEvidenceBinding,
    WrongEvidenceClaim,
    EmptyEvidenceId,
    EmptySubjectDigest,
    InvalidTimeOrder,
    FutureObservation,
    ExpiredEvidence,
    StaleEvidence,
    SequenceRollback,
    ClockUntrusted,
    ClockUncertaintyExceeded,
    PolicyVersionMismatch,
    SignatureVerificationFailed,
    AuthorityGrantMissing,
    AuthorityGrantMismatch,
}

impl From<AttestationVerificationError> for EvidenceGateError {
    fn from(_: AttestationVerificationError) -> Self {
        Self::SignatureVerificationFailed
    }
}

impl From<EvidenceGateError> for GateDenial {
    fn from(value: EvidenceGateError) -> Self {
        match value {
            EvidenceGateError::ObserverNotIndependent => GateDenial::ObserverNotIndependent,
            EvidenceGateError::WrongEvidenceBinding | EvidenceGateError::AuthorityGrantMismatch =>
                GateDenial::EvidenceBindingMismatch,
            EvidenceGateError::WrongEvidenceClaim => GateDenial::EvidenceClaimFailed,
            EvidenceGateError::ExpiredEvidence => GateDenial::EvidenceExpired,
            EvidenceGateError::StaleEvidence => GateDenial::EvidenceStale,
            EvidenceGateError::SequenceRollback => GateDenial::EvidenceSequenceRollback,
            EvidenceGateError::ClockUntrusted => GateDenial::ClockUntrusted,
            EvidenceGateError::ClockUncertaintyExceeded => GateDenial::ClockUncertaintyExceeded,
            EvidenceGateError::FutureObservation | EvidenceGateError::InvalidTimeOrder =>
                GateDenial::EvidenceFutureTimestamp,
            EvidenceGateError::AuthorityGrantMissing => GateDenial::MissingGrant,
            _ => GateDenial::EvidenceVerificationFailed,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedGateContext {
    evidence_ids: Vec<String>,
    now_ms: u64,
    authority_grant: AuthorityGrant,
    sequence_observations: Vec<SequenceObservation>,
}

impl VerifiedGateContext {
    pub fn evidence_ids(&self) -> &[String] { &self.evidence_ids }
    pub fn now_ms(&self) -> u64 { self.now_ms }
}

/// Evidence anti-replay state is owned by a durable, shared trust/attestation store.
/// It is deliberately not an episode journal or participant attempt ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequenceObservation {
    pub role: EvidenceRole,
    pub source_id: String,
    pub sequence: u64,
    pub evidence_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayGuardError {
    Unavailable,
    ReplayOrRollback,
}

/// Implementations must atomically compare and advance all values, durably and across
/// verifier instances/processes. In-memory guards are test-only, not production anti-replay.
pub trait EvidenceReplayGuard {
    fn check_and_advance_batch(
        &mut self,
        observations: &[SequenceObservation],
    ) -> Result<(), ReplayGuardError>;
}

pub struct VerifiedEffectBroker<
    P: ParticipantPort, V: AttestationVerifier, C: TrustedClock, S: EvidenceReplayGuard
> {
    inner: EffectBroker<P>,
    attestation_verifier: V,
    clock: C,
    replay_guard: S,
    trust_policy: TrustPolicy,
}

impl<P: ParticipantPort, V: AttestationVerifier, C: TrustedClock, S: EvidenceReplayGuard>
    VerifiedEffectBroker<P, V, C, S>
{
    pub fn new(
        registry: ManifestRegistry,
        participant: P,
        attestation_verifier: V,
        clock: C,
        replay_guard: S,
        trust_policy: TrustPolicy,
    ) -> Self {
        Self {
            inner: EffectBroker::new(registry, participant),
            attestation_verifier,
            clock,
            replay_guard,
            trust_policy,
        }
    }

    /// Only this public dispatch path can reach the participant port. It obtains time from
    /// the injected trusted clock, verifies five role-specific attestations, checks exact
    /// bindings and source policy, and only then constructs the private GateContext.
    pub fn dispatch(
        &mut self,
        proposal: &MutationProposal,
        evidence: &GateEvidenceBundle,
    ) -> DispatchReport {
        let clock = match self.clock.read() {
            Ok(reading) => reading,
            Err(ClockError::Unavailable) => {
                return self.inner.denied_report(proposal, GateDenial::ClockUnavailable);
            }
            Err(ClockError::Untrusted) => {
                return self.inner.denied_report(proposal, GateDenial::ClockUntrusted);
            }
        };

        let evidence_context = match verify_gate_evidence(
            &mut self.attestation_verifier,
            &self.trust_policy,
            proposal,
            &clock,
            evidence,
        ) {
            Ok(context) => context,
            Err(reason) => return self.inner.denied_report(proposal, reason.into()),
        };

        if let Err(error) = self.replay_guard.check_and_advance_batch(
            &evidence_context.sequence_observations) {
            let denial = match error {
                ReplayGuardError::Unavailable => GateDenial::EvidenceSequenceStoreUnavailable,
                ReplayGuardError::ReplayOrRollback => GateDenial::EvidenceSequenceRollback,
            };
            return self.inner.denied_report(proposal, denial);
        }

        let context = GateContext {
            now_ms: evidence_context.now_ms,
            preconditions_satisfied: true,
            observer_available: true,
            authority_source_available: true,
            manifest_source_available: true,
            policy_source_available: true,
        };
        let mut report = self.inner.dispatch(
            proposal, Some(&evidence_context.authority_grant), context);
        // The exact verified source references remain correlatable to the gate receipt.
        report.gate_receipt.evidence_ids = evidence_context.evidence_ids;
        report
    }

    pub(crate) fn participant_mut(&mut self) -> &mut P { &mut self.inner.participant }
}

pub fn verify_gate_evidence<V: AttestationVerifier>(
    verifier: &mut V,
    policy: &TrustPolicy,
    proposal: &MutationProposal,
    clock: &ClockReading,
    bundle: &GateEvidenceBundle,
) -> Result<VerifiedGateContext, EvidenceGateError> {
    if clock.source_id != policy.clock_source_id
        || clock.trust_domain != policy.clock_trust_domain
        || clock.control_domain != policy.clock_control_domain {
        return Err(EvidenceGateError::ClockUntrusted);
    }
    if clock.uncertainty_ms > policy.maximum_clock_uncertainty_ms {
        return Err(EvidenceGateError::ClockUncertaintyExceeded);
    }
    let latest_safe_now = clock.now_ms.checked_add(clock.uncertainty_ms)
        .ok_or(EvidenceGateError::ClockUncertaintyExceeded)?;

    let expected_binding = EvidenceBinding::from_proposal(proposal);
    let expected = [
        (EvidenceRole::Authority, &bundle.authority),
        (EvidenceRole::Manifest, &bundle.manifest),
        (EvidenceRole::Policy, &bundle.policy),
        (EvidenceRole::Preflight, &bundle.preflight),
        (EvidenceRole::Observer, &bundle.observer),
    ];

    let mut evidence_ids = Vec::with_capacity(expected.len());
    let mut sequence_observations = Vec::with_capacity(expected.len());
    let mut seen = BTreeSet::new();
    let mut verified_authority_grant: Option<AuthorityGrant> = None;

    for (role, signed) in expected {
        let attestation = verifier.verify_dsse(&signed.dsse_envelope)?;
        let payload = &attestation.payload;
        if attestation.dsse_payload_type != DSSE_IN_TOTO_PAYLOAD_TYPE {
            return Err(EvidenceGateError::WrongDssePayloadType);
        }
        if attestation.statement_type != IN_TOTO_STATEMENT_V1 {
            return Err(EvidenceGateError::WrongStatementType);
        }
        if attestation.predicate_type != GATE_EVIDENCE_PREDICATE_V1 {
            return Err(EvidenceGateError::WrongPredicateType);
        }
        if payload.role != role { return Err(EvidenceGateError::WrongEvidenceRole); }
        if payload.schema_version != GATE_EVIDENCE_SCHEMA_V1 {
            return Err(EvidenceGateError::WrongSchemaVersion);
        }
        if payload.evidence_id.is_empty() { return Err(EvidenceGateError::EmptyEvidenceId); }
        if payload.subject_digest == super::Digest::ZERO {
            return Err(EvidenceGateError::EmptySubjectDigest);
        }
        if !seen.insert(payload.evidence_id.as_str()) {
            return Err(EvidenceGateError::WrongEvidenceRole);
        }
        if payload.binding != expected_binding {
            return Err(EvidenceGateError::WrongEvidenceBinding);
        }
        if payload.claim != role.required_claim() {
            return Err(EvidenceGateError::WrongEvidenceClaim);
        }
        match role {
            EvidenceRole::Authority => {
                let grant = attestation.authority_grant.clone()
                    .ok_or(EvidenceGateError::AuthorityGrantMissing)?;
                if grant.issuer != attestation.publisher.issuer
                    || grant.subject_digest != payload.subject_digest
                    || grant.policy_digest != proposal.policy_digest
                    || grant.policy_version != proposal.policy_version {
                    return Err(EvidenceGateError::AuthorityGrantMismatch);
                }
                verified_authority_grant = Some(grant);
            }
            EvidenceRole::Manifest => {
                if attestation.authority_grant.is_some()
                    || payload.subject_digest != proposal.manifest_digest {
                    return Err(EvidenceGateError::WrongEvidenceBinding);
                }
            }
            EvidenceRole::Policy => {
                if attestation.authority_grant.is_some()
                    || payload.subject_digest != policy.policy_digest {
                    return Err(EvidenceGateError::WrongEvidenceBinding);
                }
            }
            EvidenceRole::Preflight | EvidenceRole::Observer => {
                if attestation.authority_grant.is_some() {
                    return Err(EvidenceGateError::WrongEvidenceRole);
                }
            }
        }
        let rule = resolve_rule(policy, role)?;
        if payload.schema_version != rule.schema_version {
            return Err(EvidenceGateError::WrongSchemaVersion);
        }
        if payload.source_id != rule.source_id {
            return Err(EvidenceGateError::WrongSource);
        }
        if attestation.publisher.issuer != rule.issuer
            || attestation.publisher.subject != rule.subject
            || attestation.publisher.key_fingerprint != rule.key_fingerprint {
            return Err(EvidenceGateError::WrongPublisher);
        }
        if attestation.publisher.trust_domain != rule.trust_domain {
            return Err(EvidenceGateError::WrongTrustDomain);
        }
        if role == EvidenceRole::Observer {
            let shares_control_domain = policy.publisher_rules.iter().any(|other| {
                other.role != EvidenceRole::Observer
                    && other.control_domain == rule.control_domain
            });
            if rule.control_domain == policy.executor_control_domain || shares_control_domain {
                return Err(EvidenceGateError::ObserverNotIndependent);
            }
        }
        if payload.sequence < rule.minimum_sequence {
            return Err(EvidenceGateError::SequenceRollback);
        }
        if payload.issued_at_ms > payload.observed_at_ms
            || payload.observed_at_ms > payload.expires_at_ms {
            return Err(EvidenceGateError::InvalidTimeOrder);
        }
        if payload.observed_at_ms > latest_safe_now {
            return Err(EvidenceGateError::FutureObservation);
        }
        if payload.expires_at_ms <= latest_safe_now {
            return Err(EvidenceGateError::ExpiredEvidence);
        }
        if clock.now_ms.saturating_sub(payload.observed_at_ms) > rule.maximum_age_ms {
            return Err(EvidenceGateError::StaleEvidence);
        }
        evidence_ids.push(payload.evidence_id.clone());
        sequence_observations.push(SequenceObservation {
            role,
            source_id: payload.source_id.clone(),
            sequence: payload.sequence,
            evidence_id: payload.evidence_id.clone(),
        });
    }

    // This policy version is itself pinned by the proposal and by the trust-policy registry.
    if policy.policy_version != proposal.policy_version
        || policy.policy_digest != proposal.policy_digest {
        return Err(EvidenceGateError::PolicyVersionMismatch);
    }

    let authority_grant = verified_authority_grant
        .ok_or(EvidenceGateError::AuthorityGrantMissing)?;
    Ok(VerifiedGateContext {
        evidence_ids,
        now_ms: clock.now_ms,
        authority_grant,
        sequence_observations,
    })
}

fn resolve_rule(policy: &TrustPolicy, role: EvidenceRole) -> Result<&PublisherRule, EvidenceGateError> {
    let mut matching = policy.publisher_rules.iter().filter(|r| r.role == role);
    let first = matching.next().ok_or(EvidenceGateError::MissingPublisherRule)?;
    if matching.next().is_some() { return Err(EvidenceGateError::DuplicatePublisherRule); }
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentDecision, Digest, EffectClass};

    #[derive(Clone)]
    struct FakeParticipant {
        submit_calls: usize,
    }

    impl ParticipantPort for FakeParticipant {
        fn inspect_logical_action(&mut self, tenant: &str, key: &str)
            -> Result<super::super::LogicalActionEvidence, super::super::ParticipantError> {
            Ok(super::super::LogicalActionEvidence {
                participant_id: "participant-a".into(),
                tenant_id: tenant.into(),
                idempotency_key: key.into(),
                ledger_cursor: "ledger:0".into(),
                latest_attempt: None,
            })
        }

        fn submit(&mut self, request: &super::super::DispatchRequest)
            -> Result<super::super::ParticipantEvidence, super::super::ParticipantError> {
            self.submit_calls += 1;
            Ok(super::super::ParticipantEvidence {
                participant_id: request.participant_id.clone(),
                tenant_id: request.tenant_id.clone(),
                idempotency_key: request.idempotency_key.clone(),
                attempt_id: request.attempt_id.clone(),
                request_digest: request.request_digest.clone(),
                ledger_cursor: "ledger:1".into(),
                state: super::super::ParticipantState::Committed,
            })
        }

        fn query_same_attempt(&mut self, tenant: &str, key: &str, attempt: &str, digest: &Digest)
            -> Result<super::super::ParticipantEvidence, super::super::ParticipantError> {
            Ok(super::super::ParticipantEvidence {
                participant_id: "participant-a".into(), tenant_id: tenant.into(),
                idempotency_key: key.into(), attempt_id: attempt.into(),
                request_digest: digest.clone(), ledger_cursor: "ledger:1".into(),
                state: super::super::ParticipantState::NeverAccepted,
            })
        }
    }

    struct FakeVerifier {
        results: Vec<VerifiedAttestation>,
        fail_at: Option<usize>,
        calls: usize,
    }

    impl AttestationVerifier for FakeVerifier {
        fn verify_dsse(&mut self, _envelope: &[u8])
            -> Result<VerifiedAttestation, AttestationVerificationError> {
            let index = self.calls;
            self.calls += 1;
            if self.fail_at == Some(index) {
                return Err(AttestationVerificationError::InvalidSignature);
            }
            self.results.get(index).cloned()
                .ok_or(AttestationVerificationError::MalformedEnvelope)
        }
    }

    struct FakeClock(ClockReading);
    impl TrustedClock for FakeClock {
        fn read(&mut self) -> Result<ClockReading, ClockError> { Ok(self.0.clone()) }
    }

    #[derive(Default)]
    struct FakeReplayGuard {
        seen: BTreeMap<(EvidenceRole, String), u64>,
        unavailable: bool,
    }

    impl EvidenceReplayGuard for FakeReplayGuard {
        fn check_and_advance_batch(
            &mut self,
            observations: &[SequenceObservation],
        ) -> Result<(), ReplayGuardError> {
            if self.unavailable { return Err(ReplayGuardError::Unavailable); }
            let mut updates = BTreeMap::new();
            for observation in observations {
                let key = (observation.role, observation.source_id.clone());
                if self.seen.get(&key).map(|last| observation.sequence <= *last).unwrap_or(false)
                    || updates.insert(key, observation.sequence).is_some() {
                    return Err(ReplayGuardError::ReplayOrRollback);
                }
            }
            self.seen.extend(updates);
            Ok(())
        }
    }

    fn proposal() -> MutationProposal {
        MutationProposal {
            tenant_id: "tenant-a".into(), principal_id: "operator-a".into(),
            episode_id: "episode-a".into(), plan_digest: Digest::from_byte(2),
            step_id: "step-1".into(), tool_id: "deployment".into(),
            operation_id: "restart-service".into(), manifest_version: "1.0.0".into(),
            manifest_digest: Digest::from_byte(1), toolchain_id: "rust-1.96".into(),
            toolchain_digest: Digest::from_byte(5), policy_version: "policy-v1".into(),
            policy_digest: Digest::from_byte(9), target_digest: Digest::from_byte(3), request_digest: Digest::from_byte(4),
            grant_id: "grant-1".into(), attempt_id: "attempt-1".into(),
            idempotency_key: "logical-1".into(), environment: "test".into(),
            risk_level: 1, decision: AgentDecision::Act,
        }
    }

    fn manifest() -> ToolManifest {
        ToolManifest {
            tool_id: "deployment".into(), operation_id: "restart-service".into(),
            semantic_version: "1.0.0".into(), manifest_digest: Digest::from_byte(1),
            toolchain_id: "rust-1.96".into(), toolchain_digest: Digest::from_byte(5),
            policy_version: "policy-v1".into(), policy_digest: Digest::from_byte(9),
            effect_class: EffectClass::BoundedMutation,
            qualified: true, required_scope: "service:restart".into(),
            required_grant_issuer: "authority-issuer".into(), participant_id: "participant-a".into(),
            idempotency_supported: true, participant_ledger_supported: true,
            preflight_verifier_id: "preflight-v1".into(), observer_verifier_id: "observer-v1".into(),
            max_risk_level: 2, allowed_environments: vec!["test".into()],
        }
    }

    fn grant(p: &MutationProposal) -> AuthorityGrant {
        AuthorityGrant {
            grant_id: p.grant_id.clone(), issuer: "authority-issuer".into(),
            scope: "service:restart".into(), tenant_id: p.tenant_id.clone(),
            principal_id: p.principal_id.clone(), episode_id: p.episode_id.clone(),
            plan_digest: p.plan_digest.clone(), step_id: p.step_id.clone(),
            tool_id: p.tool_id.clone(), operation_id: p.operation_id.clone(),
            manifest_version: p.manifest_version.clone(), manifest_digest: p.manifest_digest.clone(),
            toolchain_id: p.toolchain_id.clone(), toolchain_digest: p.toolchain_digest.clone(),
            target_digest: p.target_digest.clone(), request_digest: p.request_digest.clone(),
            attempt_id: p.attempt_id.clone(), idempotency_key: p.idempotency_key.clone(),
            environment: p.environment.clone(), risk_level: p.risk_level,
            policy_version: p.policy_version.clone(), policy_digest: p.policy_digest.clone(),
            subject_digest: Digest::from_byte(10), expires_at_ms: 50_000, revoked: false,
        }
    }

    fn rules() -> Vec<PublisherRule> {
        let sources = [
            (EvidenceRole::Authority, "authority-src", "authority-issuer", "authority-key", "authority-domain", "authority-control"),
            (EvidenceRole::Manifest, "manifest-src", "manifest-issuer", "manifest-key", "manifest-domain", "build-control"),
            (EvidenceRole::Policy, "policy-src", "policy-issuer", "policy-key", "policy-domain", "policy-control"),
            (EvidenceRole::Preflight, "preflight-src", "preflight-issuer", "preflight-key", "preflight-domain", "preflight-control"),
            (EvidenceRole::Observer, "observer-src", "observer-issuer", "observer-key", "observer-domain", "observer-control"),
        ];
        sources.into_iter().map(|(role, source_id, issuer, subject, trust_domain, control_domain)| {
            PublisherRule {
                role, source_id: source_id.into(), issuer: issuer.into(), subject: subject.into(),
                trust_domain: trust_domain.into(), control_domain: control_domain.into(),
                key_fingerprint: format!("fp-{subject}"), schema_version: GATE_EVIDENCE_SCHEMA_V1.into(),
                maximum_age_ms: 5_000, minimum_sequence: 4,
            }
        }).collect()
    }

    fn trust_policy() -> TrustPolicy {
        TrustPolicy {
            policy_version: "policy-v1".into(), policy_digest: Digest::from_byte(9),
            executor_control_domain: "agent-executor".into(),
            clock_source_id: "clock-1".into(), clock_trust_domain: "time-domain".into(),
            clock_control_domain: "time-control".into(), maximum_clock_uncertainty_ms: 10,
            publisher_rules: rules(),
        }
    }

    fn attestations(p: &MutationProposal) -> Vec<VerifiedAttestation> {
        let binding = EvidenceBinding::from_proposal(p);
        let sources = [
            (EvidenceRole::Authority, "authority-src", "authority-issuer", "authority-key", "authority-domain", "fp-authority-key"),
            (EvidenceRole::Manifest, "manifest-src", "manifest-issuer", "manifest-key", "manifest-domain", "fp-manifest-key"),
            (EvidenceRole::Policy, "policy-src", "policy-issuer", "policy-key", "policy-domain", "fp-policy-key"),
            (EvidenceRole::Preflight, "preflight-src", "preflight-issuer", "preflight-key", "preflight-domain", "fp-preflight-key"),
            (EvidenceRole::Observer, "observer-src", "observer-issuer", "observer-key", "observer-domain", "fp-observer-key"),
        ];
        sources.into_iter().enumerate().map(|(i, (role, source_id, issuer, subject, domain, key_fingerprint))| {
            VerifiedAttestation {
                dsse_payload_type: DSSE_IN_TOTO_PAYLOAD_TYPE.into(),
                statement_type: IN_TOTO_STATEMENT_V1.into(),
                predicate_type: GATE_EVIDENCE_PREDICATE_V1.into(),
                publisher: VerifiedPublisher { issuer: issuer.into(), subject: subject.into(),
                    trust_domain: domain.into(), key_fingerprint: key_fingerprint.into() },
                authority_grant: if role == EvidenceRole::Authority { Some(grant(p)) } else { None },
                payload: EvidencePayload {
                    evidence_id: format!("evidence-{i}"), role, source_id: source_id.into(),
                    schema_version: GATE_EVIDENCE_SCHEMA_V1.into(), binding: binding.clone(),
                    claim: role.required_claim(), issued_at_ms: 90_000, observed_at_ms: 99_000,
                    expires_at_ms: 105_000, sequence: 4,
                    subject_digest: match role {
                        EvidenceRole::Authority => Digest::from_byte(10),
                        EvidenceRole::Manifest => p.manifest_digest.clone(),
                        EvidenceRole::Policy => p.policy_digest.clone(),
                        EvidenceRole::Preflight | EvidenceRole::Observer => Digest::from_byte((i+10) as u8),
                    },
                },
            }
        }).collect()
    }

    fn bundle() -> GateEvidenceBundle {
        GateEvidenceBundle {
            authority: SignedSourceEvidence { dsse_envelope: b"authority".to_vec() },
            manifest: SignedSourceEvidence { dsse_envelope: b"manifest".to_vec() },
            policy: SignedSourceEvidence { dsse_envelope: b"policy".to_vec() },
            preflight: SignedSourceEvidence { dsse_envelope: b"preflight".to_vec() },
            observer: SignedSourceEvidence { dsse_envelope: b"observer".to_vec() },
        }
    }

    fn clock() -> FakeClock {
        FakeClock(ClockReading {
            source_id: "clock-1".into(), trust_domain: "time-domain".into(),
            control_domain: "time-control".into(), now_ms: 100_000, uncertainty_ms: 2,
        })
    }

    fn verified_broker(fail_at: Option<usize>, mutate: Option<fn(&mut Vec<VerifiedAttestation>)>)
        -> (VerifiedEffectBroker<FakeParticipant, FakeVerifier, FakeClock, FakeReplayGuard>,
            MutationProposal, GateEvidenceBundle) {
        let p = proposal();
        let mut evidence = attestations(&p);
        if let Some(f) = mutate { f(&mut evidence); }
        let verifier = FakeVerifier { results: evidence, fail_at, calls: 0 };
        let registry = {
            let mut registry = ManifestRegistry::default();
            registry.register(manifest()).unwrap();
            registry
        };
        let broker = VerifiedEffectBroker::new(registry, FakeParticipant { submit_calls: 0 },
            verifier, clock(), FakeReplayGuard::default(), trust_policy());
        (broker, p, bundle())
    }

    #[test]
    fn verified_source_bound_evidence_allows_exact_dispatch() {
        let (mut broker, p, e) = verified_broker(None, None);
        let result = broker.dispatch(&p, &e);
        assert_eq!(result.gate_receipt.decision(), super::super::GateDecision::Allowed);
        assert_eq!(result.participant_status, Some(super::super::AttemptStatus::Committed));
        assert_eq!(result.gate_receipt.evidence_ids.len(), 5);
        assert_eq!(broker.participant_mut().submit_calls, 1);
    }

    #[test]
    fn invalid_signature_fails_closed_before_participant_submission() {
        let (mut broker, p, e) = verified_broker(Some(2), None);
        let result = broker.dispatch(&p, &e);
        assert_eq!(result.gate_receipt.decision(),
            super::super::GateDecision::Denied(GateDenial::EvidenceVerificationFailed));
        assert_eq!(broker.participant_mut().submit_calls, 0);
    }

    #[test]
    fn stale_preflight_evidence_fails_closed() {
        fn mutate(v: &mut Vec<VerifiedAttestation>) {
            v[3].payload.observed_at_ms = 90_000;
        }
        let (mut broker, p, e) = verified_broker(None, Some(mutate));
        let result = broker.dispatch(&p, &e);
        assert_eq!(result.gate_receipt.decision(),
            super::super::GateDecision::Denied(GateDenial::EvidenceStale));
        assert_eq!(broker.participant_mut().submit_calls, 0);
    }

    #[test]
    fn observer_must_have_an_independent_control_domain() {
        fn mutate(v: &mut Vec<VerifiedAttestation>) {
            v[4].publisher.trust_domain = "observer-domain".into();
        }
        // Signature identity remains valid, but the policy maps this publisher to the
        // same control domain as the executor.
        let (mut broker, p, e) = verified_broker(None, Some(mutate));
        broker.trust_policy.executor_control_domain = "observer-control".into();
        let result = broker.dispatch(&p, &e);
        assert_eq!(result.gate_receipt.decision(),
            super::super::GateDecision::Denied(GateDenial::ObserverNotIndependent));
        assert_eq!(broker.participant_mut().submit_calls, 0);
    }

    #[test]
    fn signed_binding_must_match_exact_proposal() {
        fn mutate(v: &mut Vec<VerifiedAttestation>) {
            v[0].payload.binding.target_digest = Digest::from_byte(99);
        }
        let (mut broker, p, g, e) = verified_broker(None, Some(mutate));
        let result = broker.dispatch(&p, Some(&g), &e);
        assert_eq!(result.gate_receipt.decision(),
            super::super::GateDecision::Denied(GateDenial::EvidenceBindingMismatch));
        assert_eq!(broker.participant_mut().submit_calls, 0);
    }

    #[test]
    fn stale_sequence_floor_is_rejected() {
        fn mutate(v: &mut Vec<VerifiedAttestation>) { v[1].payload.sequence = 3; }
        let (mut broker, p, g, e) = verified_broker(None, Some(mutate));
        let result = broker.dispatch(&p, Some(&g), &e);
        assert_eq!(result.gate_receipt.decision(),
            super::super::GateDecision::Denied(GateDenial::EvidenceSequenceRollback));
        assert_eq!(broker.participant_mut().submit_calls, 0);
    }

    #[test]
    fn uncertain_clock_fails_closed() {
        let (mut broker, p, e) = verified_broker(None, None);
        broker.clock.0.uncertainty_ms = 99;
        let result = broker.dispatch(&p, &e);
        assert_eq!(result.gate_receipt.decision(),
            super::super::GateDecision::Denied(GateDenial::ClockUncertaintyExceeded));
        assert_eq!(broker.participant_mut().submit_calls, 0);
    }
    #[test]
    fn evidence_sequences_are_consumed_once_across_dispatches() {
        let (mut broker, p, e) = verified_broker(None, None);
        let first = broker.dispatch(&p, &e);
        assert_eq!(first.gate_receipt.decision(), super::super::GateDecision::Allowed);
        let second = broker.dispatch(&p, &e);
        assert_eq!(second.gate_receipt.decision(),
            super::super::GateDecision::Denied(GateDenial::EvidenceSequenceRollback));
        assert_eq!(broker.participant_mut().submit_calls, 1);
    }

    #[test]
    fn evidence_replay_store_outage_fails_closed() {
        let (mut broker, p, e) = verified_broker(None, None);
        broker.replay_guard.unavailable = true;
        let result = broker.dispatch(&p, &e);
        assert_eq!(result.gate_receipt.decision(),
            super::super::GateDecision::Denied(GateDenial::EvidenceSequenceStoreUnavailable));
        assert_eq!(broker.participant_mut().submit_calls, 0);
    }

    fn apply_evidence_attack(
        index: usize,
        broker: &mut VerifiedEffectBroker<FakeParticipant, FakeVerifier, FakeClock, FakeReplayGuard>,
    ) {
        match index {
            0 => broker.attestation_verifier.broker.attestation_verifier.results[0].payload.role = EvidenceRole::Manifest,
            1 => broker.attestation_verifier.results[2].payload.schema_version.push_str("-old"),
            2 => broker.attestation_verifier.results[0].payload.evidence_id.clear(),
            3 => broker.attestation_verifier.results[0].payload.subject_digest = Digest::ZERO,
            4 => broker.attestation_verifier.results[0].dsse_payload_type = "wrong/type".into(),
            5 => broker.attestation_verifier.results[0].statement_type = "https://in-toto.io/Statement/v0".into(),
            6 => broker.attestation_verifier.results[0].predicate_type = "urn:wrong-predicate".into(),
            7 => broker.attestation_verifier.results[0].payload.source_id.push_str("-other"),
            8 => broker.attestation_verifier.results[0].publisher.issuer.push_str("-other"),
            9 => broker.attestation_verifier.results[0].publisher.subject.push_str("-other"),
            10 => broker.attestation_verifier.results[0].publisher.key_fingerprint.push_str("-other"),
            11 => broker.attestation_verifier.results[0].publisher.trust_domain.push_str("-other"),
            12 => {
                let duplicate_id = broker.attestation_verifier.results[0].payload.evidence_id.clone();
                broker.attestation_verifier.results[1].payload.evidence_id = duplicate_id;
            },
            13 => broker.attestation_verifier.results[0].payload.binding.tenant_id.push_str("-other"),
            14 => broker.attestation_verifier.results[0].payload.binding.principal_id.push_str("-other"),
            15 => broker.attestation_verifier.results[0].payload.binding.episode_id.push_str("-other"),
            16 => broker.attestation_verifier.results[0].payload.binding.plan_digest = Digest::from_byte(99),
            17 => broker.attestation_verifier.results[0].payload.binding.step_id.push_str("-other"),
            18 => broker.attestation_verifier.results[0].payload.binding.tool_id.push_str("-other"),
            19 => broker.attestation_verifier.results[0].payload.binding.operation_id.push_str("-other"),
            20 => broker.attestation_verifier.results[0].payload.binding.manifest_version.push_str("-other"),
            21 => broker.attestation_verifier.results[0].payload.binding.manifest_digest = Digest::from_byte(99),
            22 => broker.attestation_verifier.results[0].payload.binding.toolchain_id.push_str("-other"),
            23 => broker.attestation_verifier.results[0].payload.binding.toolchain_digest = Digest::from_byte(99),
            24 => broker.attestation_verifier.results[0].payload.binding.policy_version.push_str("-other"),
            25 => broker.attestation_verifier.results[0].payload.binding.policy_digest = Digest::from_byte(99),
            26 => broker.attestation_verifier.results[0].payload.binding.target_digest = Digest::from_byte(99),
            27 => broker.attestation_verifier.results[0].payload.binding.request_digest = Digest::from_byte(99),
            28 => broker.attestation_verifier.results[0].payload.binding.grant_id.push_str("-other"),
            29 => broker.attestation_verifier.results[0].payload.binding.attempt_id.push_str("-other"),
            30 => broker.attestation_verifier.results[0].payload.binding.idempotency_key.push_str("-other"),
            31 => broker.attestation_verifier.results[0].payload.binding.environment.push_str("-other"),
            32 => broker.attestation_verifier.results[0].payload.binding.risk_level = 99,
            33 => broker.attestation_verifier.results[0].payload.claim = EvidenceClaim::ObserverCapable,
            34 => broker.attestation_verifier.results[3].payload.issued_at_ms = 100_001,
            35 => {
                let issued_at = broker.attestation_verifier.results[3].payload.issued_at_ms;
                broker.attestation_verifier.results[3].payload.observed_at_ms = issued_at - 1;
            },
            36 => broker.attestation_verifier.results[3].payload.expires_at_ms = 100_001,
            37 => broker.attestation_verifier.results[3].payload.observed_at_ms = 90_000,
            38 => broker.attestation_verifier.results[2].payload.sequence = 3,
            39 => broker.trust_policy.publisher_rules[2].schema_version.push_str("-old"),
            40 => {
                let duplicate_rule = broker.trust_policy.publisher_rules[0].clone();
                broker.trust_policy.publisher_rules.push(duplicate_rule);
            },
            41 => {
                broker.trust_policy.executor_control_domain =
                    broker.trust_policy.publisher_rules[4].control_domain.clone();
            }
            42 => {
                broker.trust_policy.publisher_rules[3].control_domain =
                    broker.trust_policy.publisher_rules[4].control_domain.clone();
            }
            43 => broker.clock.0.source_id.push_str("-other"),
            44 => broker.clock.0.trust_domain.push_str("-other"),
            45 => broker.clock.0.control_domain.push_str("-other"),
            46 => broker.clock.0.uncertainty_ms = 99,
            47 => broker.attestation_verifier.results[0].authority_grant = None,
            48 => broker.attestation_verifier.results[0].authority_grant.as_mut().unwrap().issuer.push_str("-other"),
            49 => broker.attestation_verifier.results[0].authority_grant.as_mut().unwrap().subject_digest = Digest::from_byte(99),
            50 => broker.attestation_verifier.results[1].payload.subject_digest = Digest::from_byte(99),
            51 => broker.attestation_verifier.results[2].payload.subject_digest = Digest::from_byte(99),
            52 => broker.trust_policy.policy_version.push_str("-old"),
            53 => broker.replay_guard.unavailable = true,
            54 => broker.attestation_verifier.fail_at = Some(2),
            _ => unreachable!("the conformance vector array is fixed"),
        }
    }

    #[test]
    fn fifty_five_source_evidence_negative_vectors_never_dispatch() {
        let vectors: [usize; 55] = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
            11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
            21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
            31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
            41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
            51, 52, 53, 54,
        ];
        for vector in vectors {
            let (mut broker, proposal, bundle) = verified_broker(None, None);
            apply_evidence_attack(vector, &mut broker);
            let result = broker.dispatch(&proposal, &bundle);
            assert!(
                matches!(result.gate_receipt.decision(), super::super::GateDecision::Denied(_)),
                "evidence attack {vector} was not denied"
            );
            assert_eq!(
                broker.participant_mut().submit_calls, 0,
                "evidence attack {vector} reached participant submit"
            );
        }
    }
}
