//! Research-only reference implementation of a fail-closed agent mutation gate.
//!
//! This crate demonstrates E0/E1 semantics with an injected participant port. It is
//! deliberately not a production broker: grants and participant evidence are trusted
//! inputs supplied by adapters; receipts and the permit-replay guard are process-local;
//! no cryptographic signatures, durable broker journal, process isolation, or host executor
//! are implemented here. Participant attempt state belongs to the participant ledger.
//! Nixward remains the sole host-transaction authority.

use std::collections::HashSet;

pub mod verified_inputs;
pub use verified_inputs::{
    AttestationVerificationError, AttestationVerifier, ClockError, ClockReading,
    EvidenceBinding, EvidenceClaim, EvidenceGateError, EvidencePayload, EvidenceRole,
    EvidenceReplayGuard, GateEvidenceBundle, PublisherRule, ReplayGuardError,
    SequenceObservation, SignedSourceEvidence, TrustPolicy, TrustedClock,
    VerifiedAttestation, VerifiedEffectBroker, VerifiedGateContext, VerifiedPublisher,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Digest(pub [u8; 32]);

impl Digest {
    pub const ZERO: Self = Self([0; 32]);
    pub fn from_byte(byte: u8) -> Self { Self([byte; 32]) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectClass { ReadOnly, BoundedMutation, IrreversibleMutation, Unknown }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentDecision { Act, Clarify, RequestAuthority, Reconcile, Escalate, Halt }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolManifest {
    pub tool_id: String,
    pub operation_id: String,
    pub semantic_version: String,
    pub manifest_digest: Digest,
    pub toolchain_id: String,
    pub toolchain_digest: Digest,
    pub policy_version: String,
    pub policy_digest: Digest,
    pub effect_class: EffectClass,
    pub qualified: bool,
    pub required_scope: String,
    pub required_grant_issuer: String,
    pub participant_id: String,
    pub idempotency_supported: bool,
    pub participant_ledger_supported: bool,
    pub preflight_verifier_id: String,
    pub observer_verifier_id: String,
    pub max_risk_level: u8,
    pub allowed_environments: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ManifestRegistry { manifests: Vec<ToolManifest> }

impl ManifestRegistry {
    pub fn register(&mut self, manifest: ToolManifest) -> Result<(), RegistryError> {
        if self.manifests.iter().any(|m| m.tool_id == manifest.tool_id
            && m.operation_id == manifest.operation_id
            && m.semantic_version == manifest.semantic_version) {
            return Err(RegistryError::DuplicateManifest);
        }
        self.manifests.push(manifest);
        Ok(())
    }

    fn resolve(&self, tool: &str, operation: &str, version: &str) -> Option<&ToolManifest> {
        self.manifests.iter().find(|m| m.tool_id == tool
            && m.operation_id == operation && m.semantic_version == version)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistryError { DuplicateManifest }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutationProposal {
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
    pub grant_id: String,
    pub attempt_id: String,
    pub idempotency_key: String,
    pub environment: String,
    pub risk_level: u8,
    pub decision: AgentDecision,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityGrant {
    pub grant_id: String,
    pub issuer: String,
    pub scope: String,
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
    pub target_digest: Digest,
    pub request_digest: Digest,
    pub attempt_id: String,
    pub idempotency_key: String,
    pub environment: String,
    pub risk_level: u8,
    pub policy_version: String,
    pub policy_digest: Digest,
    /// Digest of the exact authority-grant subject bytes verified in its signed attestation.
    pub subject_digest: Digest,
    pub expires_at_ms: u64,
    pub revoked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GateContext {
    pub now_ms: u64,
    pub preconditions_satisfied: bool,
    pub observer_available: bool,
    pub authority_source_available: bool,
    pub manifest_source_available: bool,
    pub policy_source_available: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateDenial {
    NonActDecision, MissingGrant, AuthoritySourceUnavailable, ManifestSourceUnavailable,
    PolicySourceUnavailable, ManifestNotFound, ManifestDigestMismatch, ManifestUnqualified,
    UnknownEffectClass, ReadOnlyOperation, IdempotencyUnsupported, ParticipantLedgerUnsupported,
    MissingObserverVerifier, ObserverUnavailable, MissingPreflightVerifier, PreconditionsFailed,
    RiskExceedsManifest, EnvironmentNotAllowed, RevokedGrant, ExpiredGrant,
    GrantBindingMismatch, GrantIssuerMismatch, GrantScopeMismatch, PolicyVersionMismatch,
    PolicyDigestMismatch, ToolchainMismatch,
    MissingRequiredField, AttemptIdReplay, PriorAttemptUnresolved, PermitBindingMismatch,
    ParticipantEvidenceMismatch, ParticipantLedgerUnavailable, IdempotencyKeyConflict,
    EvidenceVerificationFailed, EvidenceBindingMismatch, EvidenceClaimFailed, EvidenceExpired,
    EvidenceStale, EvidenceSequenceRollback, EvidenceSequenceStoreUnavailable, EvidenceFutureTimestamp, ObserverNotIndependent,
    ClockUnavailable, ClockUntrusted, ClockUncertaintyExceeded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateDecision { Allowed, Denied(GateDenial) }

/// Read-only result of a gate decision. Persistence/signing is an adapter responsibility.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateReceipt {
    sequence: u64,
    tenant_id: String,
    episode_id: String,
    attempt_id: String,
    request_digest: Digest,
    decision: GateDecision,
    evidence_ids: Vec<String>,
}

impl GateReceipt {
    pub fn sequence(&self) -> u64 { self.sequence }
    pub fn tenant_id(&self) -> &str { &self.tenant_id }
    pub fn episode_id(&self) -> &str { &self.episode_id }
    pub fn attempt_id(&self) -> &str { &self.attempt_id }
    pub fn request_digest(&self) -> &Digest { &self.request_digest }
    pub fn decision(&self) -> GateDecision { self.decision }
    pub fn evidence_ids(&self) -> &[String] { &self.evidence_ids }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticipantState { NeverAccepted, AcceptedPending, Committed, Rejected, Unknown }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParticipantEvidence {
    pub participant_id: String,
    pub tenant_id: String,
    pub idempotency_key: String,
    pub attempt_id: String,
    pub request_digest: Digest,
    pub ledger_cursor: String,
    pub state: ParticipantState,
}

/// Strong participant-owned index for one logical idempotency key.
/// latest_attempt=None must mean an authoritative ledger lookup found no prior attempt,
/// not merely that the participant's local cache did not contain one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicalActionEvidence {
    pub participant_id: String,
    pub tenant_id: String,
    pub idempotency_key: String,
    pub ledger_cursor: String,
    pub latest_attempt: Option<ParticipantEvidence>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticipantError { TransportOrOutcomeUnknown }

/// Participant ledger is authoritative for logical-key and same-attempt reconciliation.
/// submit() must atomically enforce tenant+idempotency-key+request-digest semantics so
/// an inspect/submit race cannot produce a duplicate effect.
pub trait ParticipantPort {
    fn inspect_logical_action(&mut self, tenant_id: &str, idempotency_key: &str)
        -> Result<LogicalActionEvidence, ParticipantError>;
    fn submit(&mut self, request: &DispatchRequest) -> Result<ParticipantEvidence, ParticipantError>;
    fn query_same_attempt(&mut self, tenant_id: &str, idempotency_key: &str,
        attempt_id: &str, request_digest: &Digest) -> Result<ParticipantEvidence, ParticipantError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchRequest {
    pub participant_id: String,
    pub tenant_id: String,
    pub episode_id: String,
    pub step_id: String,
    pub tool_id: String,
    pub operation_id: String,
    pub target_digest: Digest,
    pub request_digest: Digest,
    pub attempt_id: String,
    pub idempotency_key: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptStatus { OutcomeUnknown, AcceptedPending, Committed, Rejected, NeverAccepted }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchReport {
    pub gate_receipt: GateReceipt,
    pub participant_status: Option<AttemptStatus>,
    pub participant_evidence: Option<ParticipantEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconcileReport {
    pub status: AttemptStatus,
    pub evidence: Option<ParticipantEvidence>,
}

/// Internal one-use permit. It is non-Clone, non-Serialize, and never exposed to the model.
struct DispatchPermit {
    request_digest: Digest,
    target_digest: Digest,
    attempt_id: String,
}

impl DispatchPermit {
    fn consume_once(self, request_digest: &Digest, target_digest: &Digest,
        attempt_id: &str) -> Result<(), GateDenial> {
        if &self.request_digest != request_digest || &self.target_digest != target_digest
            || self.attempt_id != attempt_id {
            return Err(GateDenial::PermitBindingMismatch);
        }
        Ok(())
    }
}

/// Process-local reference gate. Production adapters must reuse Saga and participant ledgers.
pub(crate) struct EffectBroker<P: ParticipantPort> {
    registry: ManifestRegistry,
    participant: P,
    // This is only an in-process permit-replay guard, not an attempt/effect ledger.
    // The participant's ledger remains authoritative across process restarts.
    used_attempts: HashSet<(String, String)>,
    receipt_sequence: u64,
}

impl<P: ParticipantPort> EffectBroker<P> {
    pub(crate) fn new(registry: ManifestRegistry, participant: P) -> Self {
        Self { registry, participant, used_attempts: HashSet::new(), receipt_sequence: 0 }
    }

    pub(crate) fn dispatch(&mut self, proposal: &MutationProposal, grant: Option<&AuthorityGrant>,
        context: GateContext) -> DispatchReport {
        let manifest = match self.authorize(proposal, grant, context) {
            Ok(m) => m.clone(),
            Err(reason) => return self.denied_report(proposal, reason),
        };

        let attempt_key = (proposal.tenant_id.clone(), proposal.attempt_id.clone());
        if self.used_attempts.contains(&attempt_key) {
            return self.denied_report(proposal, GateDenial::AttemptIdReplay);
        }
        // Consult the participant-owned logical-action ledger on every dispatch.
        // No broker-local attempt/effect ledger is created.
        let logical = match self.participant.inspect_logical_action(
            &proposal.tenant_id, &proposal.idempotency_key) {
            Ok(e) => e,
            Err(_) => return self.denied_report(proposal, GateDenial::ParticipantLedgerUnavailable),
        };
        if logical.participant_id != manifest.participant_id
            || logical.tenant_id != proposal.tenant_id
            || logical.idempotency_key != proposal.idempotency_key
            || logical.ledger_cursor.is_empty() {
            return self.denied_report(proposal, GateDenial::ParticipantEvidenceMismatch);
        }
        if let Some(previous) = logical.latest_attempt.as_ref() {
            if previous.participant_id != manifest.participant_id
                || previous.tenant_id != proposal.tenant_id
                || previous.idempotency_key != proposal.idempotency_key
                || previous.ledger_cursor.is_empty() {
                return self.denied_report(proposal, GateDenial::ParticipantEvidenceMismatch);
            }
            if previous.attempt_id == proposal.attempt_id {
                return self.denied_report(proposal, GateDenial::AttemptIdReplay);
            }
            if previous.request_digest != proposal.request_digest {
                return self.denied_report(proposal, GateDenial::IdempotencyKeyConflict);
            }
            if previous.state != ParticipantState::NeverAccepted {
                return self.denied_report(proposal, GateDenial::PriorAttemptUnresolved);
            }
        }

        let permit = DispatchPermit {
            request_digest: proposal.request_digest.clone(),
            target_digest: proposal.target_digest.clone(),
            attempt_id: proposal.attempt_id.clone(),
        };
        if let Err(reason) = permit.consume_once(&proposal.request_digest,
            &proposal.target_digest, &proposal.attempt_id) {
            return self.denied_report(proposal, reason);
        }
        // Authorization receipt precedes participant dispatch; it does not prove effect.
        let gate_receipt = self.allowed_receipt(proposal);

        // Consume this attempt identifier locally before calling out. The participant
        // ledger, not this broker, persists the authoritative attempt/effect state.
        self.used_attempts.insert(attempt_key);

        let request = DispatchRequest {
            participant_id: manifest.participant_id.clone(),
            tenant_id: proposal.tenant_id.clone(),
            episode_id: proposal.episode_id.clone(),
            step_id: proposal.step_id.clone(),
            tool_id: proposal.tool_id.clone(),
            operation_id: proposal.operation_id.clone(),
            target_digest: proposal.target_digest.clone(),
            request_digest: proposal.request_digest.clone(),
            attempt_id: proposal.attempt_id.clone(),
            idempotency_key: proposal.idempotency_key.clone(),
        };

        let (status, evidence) = match self.participant.submit(&request) {
            Ok(e) if self.evidence_matches(&e, &manifest, proposal)
                && e.state != ParticipantState::NeverAccepted
                && e.state != ParticipantState::Unknown => (map_state(e.state), Some(e)),
            _ => (AttemptStatus::OutcomeUnknown, None),
        };
        DispatchReport { gate_receipt, participant_status: Some(status),
            participant_evidence: evidence }
    }

    /// Query the exact attempt, then cross-check it against the participant's current
    /// logical-key index. The broker stores no participant attempt history itself.
    #[cfg(test)]
    pub fn reconcile_same_attempt(&mut self, participant_id: &str, tenant_id: &str,
        idempotency_key: &str, attempt_id: &str, request_digest: &Digest)
        -> Result<ReconcileReport, GateDenial> {
        let evidence = self.participant.query_same_attempt(
            tenant_id, idempotency_key, attempt_id, request_digest)
            .map_err(|_| GateDenial::ParticipantEvidenceMismatch)?;
        if evidence.participant_id != participant_id || evidence.tenant_id != tenant_id
            || evidence.idempotency_key != idempotency_key || evidence.attempt_id != attempt_id
            || &evidence.request_digest != request_digest || evidence.ledger_cursor.is_empty() {
            return Err(GateDenial::ParticipantEvidenceMismatch);
        }
        let logical = self.participant.inspect_logical_action(tenant_id, idempotency_key)
            .map_err(|_| GateDenial::ParticipantLedgerUnavailable)?;
        if logical.participant_id != participant_id || logical.tenant_id != tenant_id
            || logical.idempotency_key != idempotency_key || logical.ledger_cursor.is_empty() {
            return Err(GateDenial::ParticipantEvidenceMismatch);
        }
        let latest = logical.latest_attempt.ok_or(GateDenial::ParticipantEvidenceMismatch)?;
        if latest.participant_id != participant_id || latest.tenant_id != tenant_id
            || latest.idempotency_key != idempotency_key || latest.attempt_id != attempt_id
            || latest.request_digest != *request_digest || latest.state != evidence.state
            || latest.ledger_cursor != evidence.ledger_cursor {
            return Err(GateDenial::ParticipantEvidenceMismatch);
        }
        let status = map_state(evidence.state);
        Ok(ReconcileReport { status, evidence: Some(evidence) })
    }

    #[cfg(test)]
    pub(crate) fn participant_mut(&mut self) -> &mut P { &mut self.participant }

    fn authorize(&self, p: &MutationProposal, grant: Option<&AuthorityGrant>,
        c: GateContext) -> Result<&ToolManifest, GateDenial> {
        if p.decision != AgentDecision::Act { return Err(GateDenial::NonActDecision); }
        if !c.authority_source_available { return Err(GateDenial::AuthoritySourceUnavailable); }
        if !c.manifest_source_available { return Err(GateDenial::ManifestSourceUnavailable); }
        if !c.policy_source_available { return Err(GateDenial::PolicySourceUnavailable); }
        let g = grant.ok_or(GateDenial::MissingGrant)?;
        let m = self.registry.resolve(&p.tool_id, &p.operation_id, &p.manifest_version)
            .ok_or(GateDenial::ManifestNotFound)?;

        if p.tenant_id.is_empty() || p.principal_id.is_empty() || p.episode_id.is_empty()
            || p.step_id.is_empty() || p.operation_id.is_empty() || p.attempt_id.is_empty()
            || p.idempotency_key.is_empty() || p.environment.is_empty() {
            return Err(GateDenial::MissingRequiredField);
        }
        if p.manifest_version != m.semantic_version || p.manifest_digest != m.manifest_digest {
            return Err(GateDenial::ManifestDigestMismatch);
        }
        if p.toolchain_id != m.toolchain_id || p.toolchain_digest != m.toolchain_digest {
            return Err(GateDenial::ToolchainMismatch);
        }
        if !m.qualified { return Err(GateDenial::ManifestUnqualified); }
        match m.effect_class {
            EffectClass::Unknown => return Err(GateDenial::UnknownEffectClass),
            EffectClass::ReadOnly => return Err(GateDenial::ReadOnlyOperation),
            EffectClass::BoundedMutation | EffectClass::IrreversibleMutation => {}
        }
        if !m.idempotency_supported { return Err(GateDenial::IdempotencyUnsupported); }
        if !m.participant_ledger_supported { return Err(GateDenial::ParticipantLedgerUnsupported); }
        if m.preflight_verifier_id.is_empty() { return Err(GateDenial::MissingPreflightVerifier); }
        if m.observer_verifier_id.is_empty() { return Err(GateDenial::MissingObserverVerifier); }
        if !c.observer_available { return Err(GateDenial::ObserverUnavailable); }
        if !c.preconditions_satisfied { return Err(GateDenial::PreconditionsFailed); }
        if p.risk_level > m.max_risk_level { return Err(GateDenial::RiskExceedsManifest); }
        if !m.allowed_environments.iter().any(|e| e == &p.environment) {
            return Err(GateDenial::EnvironmentNotAllowed);
        }
        if g.revoked { return Err(GateDenial::RevokedGrant); }
        if c.now_ms >= g.expires_at_ms { return Err(GateDenial::ExpiredGrant); }
        if g.issuer != m.required_grant_issuer { return Err(GateDenial::GrantIssuerMismatch); }
        if g.scope != m.required_scope { return Err(GateDenial::GrantScopeMismatch); }
        if p.policy_version != m.policy_version || g.policy_version != m.policy_version {
            return Err(GateDenial::PolicyVersionMismatch);
        }
        if p.policy_digest != m.policy_digest || g.policy_digest != p.policy_digest {
            return Err(GateDenial::PolicyDigestMismatch);
        }
        let exact = g.grant_id == p.grant_id && g.tenant_id == p.tenant_id
            && g.principal_id == p.principal_id && g.episode_id == p.episode_id
            && g.plan_digest == p.plan_digest && g.step_id == p.step_id
            && g.tool_id == p.tool_id && g.operation_id == p.operation_id
            && g.manifest_version == p.manifest_version && g.manifest_digest == p.manifest_digest
            && g.toolchain_id == p.toolchain_id && g.toolchain_digest == p.toolchain_digest
            && g.policy_digest == p.policy_digest
            && g.target_digest == p.target_digest && g.request_digest == p.request_digest
            && g.attempt_id == p.attempt_id && g.idempotency_key == p.idempotency_key
            && g.environment == p.environment && g.risk_level == p.risk_level;
        if !exact { return Err(GateDenial::GrantBindingMismatch); }
        Ok(m)
    }

    fn evidence_matches(&self, e: &ParticipantEvidence, m: &ToolManifest,
        p: &MutationProposal) -> bool {
        e.participant_id == m.participant_id && e.tenant_id == p.tenant_id
            && e.idempotency_key == p.idempotency_key && e.attempt_id == p.attempt_id
            && e.request_digest == p.request_digest && !e.ledger_cursor.is_empty()
    }

    fn next_receipt_sequence(&mut self) -> u64 {
        self.receipt_sequence = self.receipt_sequence.saturating_add(1);
        self.receipt_sequence
    }

    fn denied_report(&mut self, p: &MutationProposal, reason: GateDenial) -> DispatchReport {
        let sequence = self.next_receipt_sequence();
        DispatchReport {
            gate_receipt: GateReceipt { sequence, tenant_id: p.tenant_id.clone(),
                episode_id: p.episode_id.clone(), attempt_id: p.attempt_id.clone(),
                request_digest: p.request_digest.clone(), decision: GateDecision::Denied(reason),
            evidence_ids: Vec::new() },
            participant_status: None, participant_evidence: None,
        }
    }

    fn allowed_receipt(&mut self, p: &MutationProposal) -> GateReceipt {
        let sequence = self.next_receipt_sequence();
        GateReceipt { sequence, tenant_id: p.tenant_id.clone(),
            episode_id: p.episode_id.clone(), attempt_id: p.attempt_id.clone(),
            request_digest: p.request_digest.clone(), decision: GateDecision::Allowed,
            evidence_ids: Vec::new() }
    }
}

fn map_state(state: ParticipantState) -> AttemptStatus {
    match state {
        ParticipantState::NeverAccepted => AttemptStatus::NeverAccepted,
        ParticipantState::AcceptedPending => AttemptStatus::AcceptedPending,
        ParticipantState::Committed => AttemptStatus::Committed,
        ParticipantState::Rejected => AttemptStatus::Rejected,
        ParticipantState::Unknown => AttemptStatus::OutcomeUnknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct FakeParticipant {
        submit_result: Result<ParticipantEvidence, ParticipantError>,
        query_result: Result<ParticipantEvidence, ParticipantError>,
        logical_action: LogicalActionEvidence,
        query_updates_logical_action: bool,
        submit_calls: usize,
    }

    impl ParticipantPort for FakeParticipant {
        fn inspect_logical_action(&mut self, tenant: &str, key: &str)
            -> Result<LogicalActionEvidence, ParticipantError> {
            if self.logical_action.tenant_id != tenant || self.logical_action.idempotency_key != key {
                return Err(ParticipantError::TransportOrOutcomeUnknown);
            }
            Ok(self.logical_action.clone())
        }
        fn submit(&mut self, r: &DispatchRequest) -> Result<ParticipantEvidence, ParticipantError> {
            self.submit_calls += 1;
            let result = self.submit_result.clone();
            let observed = match &result {
                Ok(e) => e.clone(),
                Err(_) => ParticipantEvidence {
                    participant_id: r.participant_id.clone(), tenant_id: r.tenant_id.clone(),
                    idempotency_key: r.idempotency_key.clone(), attempt_id: r.attempt_id.clone(),
                    request_digest: r.request_digest.clone(), ledger_cursor: "ledger:2".into(),
                    state: ParticipantState::Unknown,
                },
            };
            self.logical_action.ledger_cursor = observed.ledger_cursor.clone();
            self.logical_action.latest_attempt = Some(observed);
            result
        }
        fn query_same_attempt(&mut self, _tenant: &str, _key: &str, _a: &str, _d: &Digest)
            -> Result<ParticipantEvidence, ParticipantError> {
            let result = self.query_result.clone();
            if self.query_updates_logical_action {
                if let Ok(e) = &result {
                    self.logical_action.ledger_cursor = e.ledger_cursor.clone();
                    self.logical_action.latest_attempt = Some(e.clone());
                }
            }
            result
        }
    }

    fn evidence(attempt: &str, digest: Digest, state: ParticipantState) -> ParticipantEvidence {
        ParticipantEvidence { participant_id: "participant-a".into(), tenant_id: "tenant-a".into(),
            idempotency_key: "logical-action-1".into(), attempt_id: attempt.into(),
            request_digest: digest, ledger_cursor: "ledger:1".into(), state }
    }

    fn fixture() -> (EffectBroker<FakeParticipant>, MutationProposal, AuthorityGrant, GateContext) {
        let manifest = ToolManifest {
            tool_id: "deployment".into(), operation_id: "restart-service".into(),
            semantic_version: "1.0.0".into(), manifest_digest: Digest::from_byte(1),
            toolchain_id: "nix-toolchain-2026-11".into(), toolchain_digest: Digest::from_byte(5),
            policy_version: "policy-v1".into(), policy_digest: Digest::from_byte(6),
            effect_class: EffectClass::BoundedMutation,
            qualified: true, required_scope: "service:restart".into(),
            required_grant_issuer: "authority-v1".into(), participant_id: "participant-a".into(),
            idempotency_supported: true, participant_ledger_supported: true,
            preflight_verifier_id: "preflight-v1".into(), observer_verifier_id: "observer-v1".into(),
            max_risk_level: 3, allowed_environments: vec!["test".into()],
        };
        let mut registry = ManifestRegistry::default();
        registry.register(manifest).unwrap();
        let proposal = MutationProposal {
            tenant_id: "tenant-a".into(), principal_id: "operator-a".into(),
            episode_id: "episode-a".into(), plan_digest: Digest::from_byte(2),
            step_id: "step-1".into(), tool_id: "deployment".into(),
            operation_id: "restart-service".into(), manifest_version: "1.0.0".into(),
            manifest_digest: Digest::from_byte(1), toolchain_id: "nix-toolchain-2026-11".into(),
            toolchain_digest: Digest::from_byte(5), policy_version: "policy-v1".into(),
            policy_digest: Digest::from_byte(6), target_digest: Digest::from_byte(3), request_digest: Digest::from_byte(4),
            grant_id: "grant-1".into(), attempt_id: "attempt-1".into(),
            idempotency_key: "logical-action-1".into(), environment: "test".into(),
            risk_level: 2, decision: AgentDecision::Act,
        };
        let grant = AuthorityGrant {
            grant_id: "grant-1".into(), issuer: "authority-v1".into(),
            scope: "service:restart".into(), tenant_id: "tenant-a".into(),
            principal_id: "operator-a".into(), episode_id: "episode-a".into(),
            plan_digest: Digest::from_byte(2), step_id: "step-1".into(),
            tool_id: "deployment".into(), operation_id: "restart-service".into(),
            manifest_version: "1.0.0".into(), manifest_digest: Digest::from_byte(1),
            toolchain_id: "nix-toolchain-2026-11".into(), toolchain_digest: Digest::from_byte(5),
            target_digest: Digest::from_byte(3), request_digest: Digest::from_byte(4),
            attempt_id: "attempt-1".into(), idempotency_key: "logical-action-1".into(),
            environment: "test".into(), risk_level: 2, policy_version: "policy-v1".into(),
            policy_digest: Digest::from_byte(6), subject_digest: Digest::from_byte(10),
            expires_at_ms: 10_000, revoked: false,
        };
        let context = GateContext { now_ms: 1_000, preconditions_satisfied: true,
            observer_available: true, authority_source_available: true,
            manifest_source_available: true, policy_source_available: true };
        let participant = FakeParticipant {
            submit_result: Ok(evidence("attempt-1", Digest::from_byte(4), ParticipantState::Committed)),
            query_result: Ok(evidence("attempt-1", Digest::from_byte(4), ParticipantState::NeverAccepted)),
            logical_action: LogicalActionEvidence { participant_id: "participant-a".into(),
                tenant_id: "tenant-a".into(), idempotency_key: "logical-action-1".into(),
                ledger_cursor: "ledger:0".into(), latest_attempt: None },
            query_updates_logical_action: true,
            submit_calls: 0,
        };
        (EffectBroker::new(registry, participant), proposal, grant, context)
    }

    #[test]
    fn exact_grant_dispatches_once_and_separates_gate_and_effect_evidence() {
        let (mut broker, p, g, c) = fixture();
        let r = broker.dispatch(&p, Some(&g), c);
        assert_eq!(r.gate_receipt.decision(), GateDecision::Allowed);
        assert_eq!(r.participant_status, Some(AttemptStatus::Committed));
        assert_eq!(broker.participant_mut().submit_calls, 1);
    }

    #[test]
    fn non_act_decisions_never_reach_participant() {
        for d in [AgentDecision::Clarify, AgentDecision::RequestAuthority, AgentDecision::Reconcile,
            AgentDecision::Escalate, AgentDecision::Halt] {
            let (mut b, mut p, g, c) = fixture(); p.decision = d;
            let r = b.dispatch(&p, Some(&g), c);
            assert_eq!(r.gate_receipt.decision(), GateDecision::Denied(GateDenial::NonActDecision));
            assert_eq!(b.participant_mut().submit_calls, 0);
        }
    }

    #[test]
    fn unknown_effect_class_fails_closed() {
        let (mut b, p, g, c) = fixture();
        b.registry.manifests[0].effect_class = EffectClass::Unknown;
        let r = b.dispatch(&p, Some(&g), c);
        assert_eq!(r.gate_receipt.decision(), GateDecision::Denied(GateDenial::UnknownEffectClass));
        assert_eq!(b.participant_mut().submit_calls, 0);
    }

    #[test]
    fn ambiguous_dispatch_blocks_new_attempt_until_same_attempt_reconciled() {
        let (mut b, p, g, c) = fixture();
        b.participant.submit_result = Err(ParticipantError::TransportOrOutcomeUnknown);
        let r = b.dispatch(&p, Some(&g), c);
        assert_eq!(r.participant_status, Some(AttemptStatus::OutcomeUnknown));
        let mut p2 = p.clone(); p2.attempt_id = "attempt-2".into();
        let mut g2 = g.clone(); g2.attempt_id = "attempt-2".into();
        let blocked = b.dispatch(&p2, Some(&g2), c);
        assert_eq!(blocked.gate_receipt.decision(), GateDecision::Denied(GateDenial::PriorAttemptUnresolved));
        let reconciled = b.reconcile_same_attempt("participant-a", "tenant-a",
            "logical-action-1", "attempt-1", &Digest::from_byte(4)).unwrap();
        assert_eq!(reconciled.status, AttemptStatus::NeverAccepted);
        let allowed = b.dispatch(&p2, Some(&g2), c);
        assert_eq!(allowed.gate_receipt.decision(), GateDecision::Allowed);
        assert_eq!(b.participant_mut().submit_calls, 2);
    }

    #[test]
    fn mismatched_participant_receipt_becomes_unknown_and_blocks_retry() {
        let (mut b, p, g, c) = fixture();
        b.participant.submit_result = Ok(evidence("other-attempt", Digest::from_byte(4), ParticipantState::Committed));
        let r = b.dispatch(&p, Some(&g), c);
        assert_eq!(r.participant_status, Some(AttemptStatus::OutcomeUnknown));
        assert!(r.participant_evidence.is_none());
        let mut p2 = p.clone(); p2.attempt_id = "attempt-2".into();
        let mut g2 = g.clone(); g2.attempt_id = "attempt-2".into();
        let blocked = b.dispatch(&p2, Some(&g2), c);
        assert_eq!(blocked.gate_receipt.decision(), GateDecision::Denied(GateDenial::PriorAttemptUnresolved));
    }

    #[test]
    fn participant_owned_committed_state_blocks_a_fresh_attempt() {
        let (mut b, p, g, c) = fixture();
        let first = b.dispatch(&p, Some(&g), c);
        assert_eq!(first.participant_status, Some(AttemptStatus::Committed));
        let mut p2 = p.clone(); p2.attempt_id = "attempt-2".into();
        let mut g2 = g.clone(); g2.attempt_id = "attempt-2".into();
        let r = b.dispatch(&p2, Some(&g2), c);
        assert_eq!(r.gate_receipt.decision(),
            GateDecision::Denied(GateDenial::PriorAttemptUnresolved));
        assert_eq!(b.participant_mut().submit_calls, 1);
    }

    #[test]
    fn idempotency_key_cannot_be_rebound_to_a_different_request() {
        let (mut b, p, g, c) = fixture();
        let first = b.dispatch(&p, Some(&g), c);
        assert_eq!(first.participant_status, Some(AttemptStatus::Committed));
        let mut p2 = p.clone();
        p2.attempt_id = "attempt-2".into();
        p2.request_digest = Digest::from_byte(9);
        let mut g2 = g.clone();
        g2.attempt_id = "attempt-2".into();
        g2.request_digest = Digest::from_byte(9);
        let r = b.dispatch(&p2, Some(&g2), c);
        assert_eq!(r.gate_receipt.decision(),
            GateDecision::Denied(GateDenial::IdempotencyKeyConflict));
        assert_eq!(b.participant_mut().submit_calls, 1);
    }

    #[test]
    fn participant_ledger_lookup_failure_fails_closed() {
        let (mut b, p, g, c) = fixture();
        b.participant.logical_action.tenant_id = "wrong-tenant".into();
        let r = b.dispatch(&p, Some(&g), c);
        assert_eq!(r.gate_receipt.decision(),
            GateDecision::Denied(GateDenial::ParticipantLedgerUnavailable));
        assert_eq!(b.participant_mut().submit_calls, 0);
    }

    #[test]
    fn one_attempt_id_cannot_be_replayed_under_another_logical_key() {
        let (mut b, p, g, c) = fixture();
        let _ = b.dispatch(&p, Some(&g), c);
        let mut p2 = p.clone(); p2.idempotency_key = "logical-action-2".into();
        let mut g2 = g.clone(); g2.idempotency_key = "logical-action-2".into();
        let r = b.dispatch(&p2, Some(&g2), c);
        assert_eq!(r.gate_receipt.decision(), GateDecision::Denied(GateDenial::AttemptIdReplay));
    }

    #[derive(Clone, Copy, Debug)]
    enum NegativeVector {
        NonAct, NoGrant, AuthorityDown, ManifestDown, PolicyDown, ObserverDown, Preconditions,
        Revoked, Expired, WrongTenant, WrongPrincipal, WrongEpisode, WrongPlan, WrongStep,
        WrongTool, WrongOperation, WrongTarget, WrongRequest, WrongGrantId, WrongIssuer,
        WrongScope, WrongPolicy, WrongAttempt, WrongIdempotency, WrongEnvironment, WrongRisk,
        ManifestDigest, Unqualified, UnknownEffect, ReadOnly, NoIdempotency, NoLedger,
        NoPreflight, NoObserverVerifier, TooMuchRisk, BadEnvironment, MissingTenant,
        MissingPrincipal, MissingEpisode, MissingStep, MissingOperation, MissingAttempt,
        MissingIdempotency, MissingEnvironment, BlankIssuer, BlankScope, WrongManifestVersion,
        ManifestWrongPolicy, GrantWrongTool, GrantWrongTarget, GrantWrongRequest,
        WrongGrantManifestVersion, WrongGrantManifestDigest, WrongToolchainId, WrongToolchainDigest,
    }

    fn apply_negative(v: NegativeVector, b: &mut EffectBroker<FakeParticipant>,
        p: &mut MutationProposal, g: &mut AuthorityGrant, c: &mut GateContext) -> bool {
        match v {
            NegativeVector::NonAct => p.decision = AgentDecision::Halt,
            NegativeVector::NoGrant => return false,
            NegativeVector::AuthorityDown => c.authority_source_available = false,
            NegativeVector::ManifestDown => c.manifest_source_available = false,
            NegativeVector::PolicyDown => c.policy_source_available = false,
            NegativeVector::ObserverDown => c.observer_available = false,
            NegativeVector::Preconditions => c.preconditions_satisfied = false,
            NegativeVector::Revoked => g.revoked = true,
            NegativeVector::Expired => g.expires_at_ms = c.now_ms,
            NegativeVector::WrongTenant => g.tenant_id.push_str("-other"),
            NegativeVector::WrongPrincipal => g.principal_id.push_str("-other"),
            NegativeVector::WrongEpisode => g.episode_id.push_str("-other"),
            NegativeVector::WrongPlan => g.plan_digest = Digest::from_byte(9),
            NegativeVector::WrongStep => g.step_id.push_str("-other"),
            NegativeVector::WrongTool => g.tool_id.push_str("-other"),
            NegativeVector::WrongOperation => g.operation_id.push_str("-other"),
            NegativeVector::WrongTarget => g.target_digest = Digest::from_byte(9),
            NegativeVector::WrongRequest => g.request_digest = Digest::from_byte(9),
            NegativeVector::WrongGrantId => g.grant_id.push_str("-other"),
            NegativeVector::WrongIssuer => g.issuer.push_str("-other"),
            NegativeVector::WrongScope => g.scope.push_str(":other"),
            NegativeVector::WrongPolicy => g.policy_version.push_str("-other"),
            NegativeVector::WrongAttempt => g.attempt_id.push_str("-other"),
            NegativeVector::WrongIdempotency => g.idempotency_key.push_str("-other"),
            NegativeVector::WrongEnvironment => g.environment.push_str("-other"),
            NegativeVector::WrongRisk => g.risk_level = 1,
            NegativeVector::ManifestDigest => p.manifest_digest = Digest::from_byte(9),
            NegativeVector::Unqualified => b.registry.manifests[0].qualified = false,
            NegativeVector::UnknownEffect => b.registry.manifests[0].effect_class = EffectClass::Unknown,
            NegativeVector::ReadOnly => b.registry.manifests[0].effect_class = EffectClass::ReadOnly,
            NegativeVector::NoIdempotency => b.registry.manifests[0].idempotency_supported = false,
            NegativeVector::NoLedger => b.registry.manifests[0].participant_ledger_supported = false,
            NegativeVector::NoPreflight => b.registry.manifests[0].preflight_verifier_id.clear(),
            NegativeVector::NoObserverVerifier => b.registry.manifests[0].observer_verifier_id.clear(),
            NegativeVector::TooMuchRisk => b.registry.manifests[0].max_risk_level = 1,
            NegativeVector::BadEnvironment => b.registry.manifests[0].allowed_environments.clear(),
            NegativeVector::MissingTenant => p.tenant_id.clear(),
            NegativeVector::MissingPrincipal => p.principal_id.clear(),
            NegativeVector::MissingEpisode => p.episode_id.clear(),
            NegativeVector::MissingStep => p.step_id.clear(),
            NegativeVector::MissingOperation => p.operation_id.clear(),
            NegativeVector::MissingAttempt => p.attempt_id.clear(),
            NegativeVector::MissingIdempotency => p.idempotency_key.clear(),
            NegativeVector::MissingEnvironment => p.environment.clear(),
            NegativeVector::BlankIssuer => b.registry.manifests[0].required_grant_issuer.clear(),
            NegativeVector::BlankScope => b.registry.manifests[0].required_scope.clear(),
            NegativeVector::WrongManifestVersion => p.manifest_version = "9.9.9".into(),
            NegativeVector::ManifestWrongPolicy => b.registry.manifests[0].policy_version = "policy-other".into(),
            NegativeVector::GrantWrongTool => g.tool_id = "other-tool".into(),
            NegativeVector::GrantWrongTarget => g.target_digest = Digest::from_byte(8),
            NegativeVector::GrantWrongRequest => g.request_digest = Digest::from_byte(8),
            NegativeVector::WrongGrantManifestVersion => g.manifest_version.push_str("-other"),
            NegativeVector::WrongGrantManifestDigest => g.manifest_digest = Digest::from_byte(8),
            NegativeVector::WrongToolchainId => p.toolchain_id.push_str("-other"),
            NegativeVector::WrongToolchainDigest => p.toolchain_digest = Digest::from_byte(8),
        }
        true
    }

    #[test]
    fn fifty_five_negative_conformance_vectors_have_zero_external_effects() {
        use NegativeVector::*;
        let vectors = [NonAct, NoGrant, AuthorityDown, ManifestDown, PolicyDown, ObserverDown,
            Preconditions, Revoked, Expired, WrongTenant, WrongPrincipal, WrongEpisode,
            WrongPlan, WrongStep, WrongTool, WrongOperation, WrongTarget, WrongRequest,
            WrongGrantId, WrongIssuer, WrongScope, WrongPolicy, WrongAttempt, WrongIdempotency,
            WrongEnvironment, WrongRisk, ManifestDigest, Unqualified, UnknownEffect, ReadOnly,
            NoIdempotency, NoLedger, NoPreflight, NoObserverVerifier, TooMuchRisk, BadEnvironment,
            MissingTenant, MissingPrincipal, MissingEpisode, MissingStep, MissingOperation,
            MissingAttempt, MissingIdempotency, MissingEnvironment, BlankIssuer, BlankScope,
            WrongManifestVersion, ManifestWrongPolicy, GrantWrongTool, GrantWrongTarget, GrantWrongRequest,
            WrongGrantManifestVersion, WrongGrantManifestDigest, WrongToolchainId, WrongToolchainDigest];
        assert_eq!(vectors.len(), 55);
        for vector in vectors {
            let (mut b, mut p, mut g, mut c) = fixture();
            let with_grant = apply_negative(vector, &mut b, &mut p, &mut g, &mut c);
            let r = b.dispatch(&p, if with_grant { Some(&g) } else { None }, c);
            assert!(matches!(r.gate_receipt.decision(), GateDecision::Denied(_)), "vector: {vector:?}");
            assert_eq!(b.participant_mut().submit_calls, 0, "vector: {vector:?}");
        }
    }

    #[test]
    fn never_accepted_cannot_contradict_prior_committed_evidence() {
        let (mut b, p, g, c) = fixture();
        let dispatched = b.dispatch(&p, Some(&g), c);
        assert_eq!(dispatched.participant_status, Some(AttemptStatus::Committed));
        // Simulate a query that contradicts the still-committed logical-key ledger view.
        b.participant.query_updates_logical_action = false;
        let result = b.reconcile_same_attempt("participant-a", "tenant-a",
            "logical-action-1", "attempt-1", &Digest::from_byte(4));
        assert_eq!(result, Err(GateDenial::ParticipantEvidenceMismatch));
    }

    #[test]
    fn reconciliation_rejects_wrong_attempt_and_digest() {
        let (mut b, p, g, c) = fixture();
        b.participant.submit_result = Err(ParticipantError::TransportOrOutcomeUnknown);
        let _ = b.dispatch(&p, Some(&g), c);
        assert_eq!(b.reconcile_same_attempt("participant-a", "tenant-a", "logical-action-1",
            "wrong", &Digest::from_byte(4)), Err(GateDenial::ParticipantEvidenceMismatch));
        assert_eq!(b.reconcile_same_attempt("participant-a", "tenant-a", "logical-action-1",
            "attempt-1", &Digest::from_byte(8)), Err(GateDenial::ParticipantEvidenceMismatch));
    }
}
