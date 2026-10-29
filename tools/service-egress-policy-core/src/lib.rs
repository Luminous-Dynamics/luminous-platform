//! Dependency-free, fail-closed policy model for broker-defined network egress.
//!
//! This module authorizes a broker-defined service operation and an opaque credential
//! handle. It does not parse HTTP, resolve DNS, open sockets, verify policy signatures,
//! or enforce isolation. Those responsibilities belong to a separately tested broker
//! and OS/network boundary. Never expose raw credential bytes to this API.

use std::collections::BTreeSet;
use std::fmt;

const MAX_ID_BYTES: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentityError {
    Empty,
    TooLong,
    NonCanonical,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("identity must not be empty"),
            Self::TooLong => formatter.write_str("identity exceeds the maximum length"),
            Self::NonCanonical => formatter.write_str(
                "identity must be a lowercase broker-defined token containing only ASCII \
                 letters, digits, dot, underscore, or hyphen, starting and ending with an \
                 ASCII letter or digit and containing no adjacent dots",
            ),
        }
    }
}

impl std::error::Error for IdentityError {}

fn validate_identity(value: &str) -> Result<(), IdentityError> {
    if value.is_empty() {
        return Err(IdentityError::Empty);
    }
    if value.len() > MAX_ID_BYTES {
        return Err(IdentityError::TooLong);
    }
    let bytes = value.as_bytes();
    if !bytes[0].is_ascii_alphanumeric()
        || !bytes[bytes.len() - 1].is_ascii_alphanumeric()
        || bytes.windows(2).any(|pair| pair == b"..")
        || !bytes.iter().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(*byte, b'.' | b'_' | b'-')
        })
    {
        return Err(IdentityError::NonCanonical);
    }
    Ok(())
}

macro_rules! identity_type {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, IdentityError> {
                let value = value.into();
                validate_identity(&value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

identity_type!(PolicyId);
identity_type!(CapabilityId);
identity_type!(TargetId);
identity_type!(ServiceId);
identity_type!(OperationId);
identity_type!(CredentialHandleId);
identity_type!(RequestId);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest(String);

impl Digest {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityError> {
        let value = value.into();
        if value.len() != 64 {
            return Err(IdentityError::NonCanonical);
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(IdentityError::NonCanonical);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthenticationState {
    Verified,
    Missing,
    Invalid,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevocationState {
    Clear,
    Revoked,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityState {
    Active,
    Revoked,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockState {
    Fresh,
    Stale,
    Reversed,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceStatus {
    Verified,
    Missing,
    Unverified,
    Contradictory,
}

/// A typed evidence cell. Verified is a declaration by the adapter, not cryptographic
/// proof; the adapter must be part of the trusted computing base and independently tested.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fact<T> {
    pub status: EvidenceStatus,
    pub value: Option<T>,
}

impl<T> Fact<T> {
    pub fn verified(value: T) -> Self {
        Self {
            status: EvidenceStatus::Verified,
            value: Some(value),
        }
    }

    pub fn missing() -> Self {
        Self {
            status: EvidenceStatus::Missing,
            value: None,
        }
    }

    pub fn unverified(value: T) -> Self {
        Self {
            status: EvidenceStatus::Unverified,
            value: Some(value),
        }
    }

    pub fn contradictory(value: T) -> Self {
        Self {
            status: EvidenceStatus::Contradictory,
            value: Some(value),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EgressCapability {
    pub id: CapabilityId,
    pub target_id: TargetId,
    pub subject_image_digest: Digest,
    pub service_id: ServiceId,
    pub operations: Vec<OperationId>,
    /// Opaque reference managed by a credential broker; never a token or secret.
    /// None means the operation is explicitly credential-free.
    pub credential_handle_id: Option<CredentialHandleId>,
    pub valid_from_ms: u64,
    pub expires_at_ms: u64,
    pub state: CapabilityState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicySnapshot {
    pub id: PolicyId,
    pub digest: Digest,
    pub target_id: TargetId,
    pub epoch: u64,
    pub valid_from_ms: u64,
    pub expires_at_ms: u64,
    pub authentication: AuthenticationState,
    pub revocation: RevocationState,
    pub capabilities: Vec<EgressCapability>,
}

/// Atomic, broker-normalized request identity. It deliberately has no free-form URL,
/// header, body, or secret value. The broker must map service_id and operation_id to a
/// fixed method/route/destination before sending traffic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedEgressAttempt {
    pub request_id: RequestId,
    pub target_id: TargetId,
    pub subject_image_digest: Digest,
    pub service_id: ServiceId,
    pub operation_id: OperationId,
    pub credential_handle_id: Option<CredentialHandleId>,
    /// Each redirect hop is a new request and must be independently re-authorized.
    pub redirect_hop: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationContext {
    pub now_ms: Option<u64>,
    pub clock_state: ClockState,
    pub expected_policy_digest: Option<Digest>,
    pub minimum_accepted_epoch: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenyReason {
    MissingClock,
    ClockReversed,
    ClockUnavailable,
    ClockStale,
    MissingPinnedPolicyDigest,
    PolicyDigestMismatch,
    MissingRollbackFloor,
    PolicyEpochRollback,
    MalformedPolicy,
    PolicyAuthenticationMissing,
    PolicyAuthenticationInvalid,
    PolicyAuthenticationUnknown,
    PolicyRevoked,
    PolicyRevocationUnknown,
    PolicyNotYetValid,
    PolicyExpired,
    MissingObservedAttempt,
    UnverifiedObservedAttempt,
    ContradictoryObservedAttempt,
    TargetMismatch,
    SubjectMismatch,
    ServiceNotGranted,
    OperationNotGranted,
    CredentialHandleMismatch,
    NoMatchingCapability,
    CapabilityRevoked,
    CapabilityRevocationUnknown,
    CapabilityNotYetValid,
    CapabilityExpired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationReceipt {
    pub policy_id: PolicyId,
    pub policy_digest: Digest,
    pub policy_epoch: u64,
    pub capability_id: CapabilityId,
    pub request_id: RequestId,
    pub target_id: TargetId,
    pub subject_image_digest: Digest,
    pub service_id: ServiceId,
    pub operation_id: OperationId,
    pub credential_handle_id: Option<CredentialHandleId>,
    pub redirect_hop: u32,
    pub authorized_at_ms: u64,
    /// Authorization is half-open: a broker must stop using this receipt at this time.
    pub valid_until_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Allow(AuthorizationReceipt),
    Deny(DenyReason),
}

pub fn evaluate(
    policy: &PolicySnapshot,
    observed: &Fact<ObservedEgressAttempt>,
    context: &EvaluationContext,
) -> Decision {
    let Some(now_ms) = context.now_ms else {
        return Decision::Deny(DenyReason::MissingClock);
    };

    let clock_denial = match context.clock_state {
        ClockState::Fresh => None,
        ClockState::Stale => Some(DenyReason::ClockStale),
        ClockState::Reversed => Some(DenyReason::ClockReversed),
        ClockState::Unavailable => Some(DenyReason::ClockUnavailable),
    };
    if let Some(reason) = clock_denial {
        return Decision::Deny(reason);
    }

    let Some(expected_digest) = context.expected_policy_digest.as_ref() else {
        return Decision::Deny(DenyReason::MissingPinnedPolicyDigest);
    };
    if expected_digest != &policy.digest {
        return Decision::Deny(DenyReason::PolicyDigestMismatch);
    }

    let Some(minimum_epoch) = context.minimum_accepted_epoch else {
        return Decision::Deny(DenyReason::MissingRollbackFloor);
    };
    if policy.epoch < minimum_epoch {
        return Decision::Deny(DenyReason::PolicyEpochRollback);
    }

    if !policy_is_well_formed(policy) {
        return Decision::Deny(DenyReason::MalformedPolicy);
    }

    match policy.authentication {
        AuthenticationState::Verified => {}
        AuthenticationState::Missing => {
            return Decision::Deny(DenyReason::PolicyAuthenticationMissing);
        }
        AuthenticationState::Invalid => {
            return Decision::Deny(DenyReason::PolicyAuthenticationInvalid);
        }
        AuthenticationState::Unknown => {
            return Decision::Deny(DenyReason::PolicyAuthenticationUnknown);
        }
    }

    match policy.revocation {
        RevocationState::Clear => {}
        RevocationState::Revoked => return Decision::Deny(DenyReason::PolicyRevoked),
        RevocationState::Unknown => {
            return Decision::Deny(DenyReason::PolicyRevocationUnknown);
        }
    }

    if now_ms < policy.valid_from_ms {
        return Decision::Deny(DenyReason::PolicyNotYetValid);
    }
    if now_ms >= policy.expires_at_ms {
        return Decision::Deny(DenyReason::PolicyExpired);
    }

    let attempt = match observed.status {
        EvidenceStatus::Missing => return Decision::Deny(DenyReason::MissingObservedAttempt),
        EvidenceStatus::Unverified => {
            return Decision::Deny(DenyReason::UnverifiedObservedAttempt);
        }
        EvidenceStatus::Contradictory => {
            return Decision::Deny(DenyReason::ContradictoryObservedAttempt);
        }
        EvidenceStatus::Verified => match observed.value.as_ref() {
            Some(attempt) => attempt,
            None => return Decision::Deny(DenyReason::MissingObservedAttempt),
        },
    };

    if attempt.target_id != policy.target_id {
        return Decision::Deny(DenyReason::TargetMismatch);
    }

    let subject_capabilities: Vec<&EgressCapability> = policy
        .capabilities
        .iter()
        .filter(|capability| {
            capability.target_id == attempt.target_id
                && capability.subject_image_digest == attempt.subject_image_digest
        })
        .collect();
    if subject_capabilities.is_empty() {
        return Decision::Deny(DenyReason::SubjectMismatch);
    }

    let service_capabilities: Vec<&EgressCapability> = subject_capabilities
        .iter()
        .copied()
        .filter(|capability| capability.service_id == attempt.service_id)
        .collect();
    if service_capabilities.is_empty() {
        return Decision::Deny(DenyReason::ServiceNotGranted);
    }

    let operation_capabilities: Vec<&EgressCapability> = service_capabilities
        .iter()
        .copied()
        .filter(|capability| capability.operations.contains(&attempt.operation_id))
        .collect();
    if operation_capabilities.is_empty() {
        return Decision::Deny(DenyReason::OperationNotGranted);
    }

    let credential_capabilities: Vec<&EgressCapability> = operation_capabilities
        .iter()
        .copied()
        .filter(|capability| capability.credential_handle_id == attempt.credential_handle_id)
        .collect();
    if credential_capabilities.is_empty() {
        return Decision::Deny(DenyReason::CredentialHandleMismatch);
    }

    // Aggregate all exact-scope grants before deciding. Vector order cannot determine
    // whether a revoked/unknown capability masks a valid active capability.
    let eligible: Vec<&EgressCapability> = credential_capabilities
        .iter()
        .copied()
        .filter(|capability| {
            capability.state == CapabilityState::Active
                && now_ms >= capability.valid_from_ms
                && now_ms < capability.expires_at_ms
        })
        .collect();

    if let Some(capability) = eligible.iter().min_by_key(|item| item.id.as_str()) {
        return Decision::Allow(AuthorizationReceipt {
            policy_id: policy.id.clone(),
            policy_digest: policy.digest.clone(),
            policy_epoch: policy.epoch,
            capability_id: capability.id.clone(),
            request_id: attempt.request_id.clone(),
            target_id: attempt.target_id.clone(),
            subject_image_digest: attempt.subject_image_digest.clone(),
            service_id: attempt.service_id.clone(),
            operation_id: attempt.operation_id.clone(),
            credential_handle_id: attempt.credential_handle_id.clone(),
            redirect_hop: attempt.redirect_hop,
            authorized_at_ms: now_ms,
            valid_until_ms: policy.expires_at_ms.min(capability.expires_at_ms),
        });
    }

    if credential_capabilities
        .iter()
        .any(|capability| capability.state == CapabilityState::Unknown)
    {
        return Decision::Deny(DenyReason::CapabilityRevocationUnknown);
    }
    if credential_capabilities
        .iter()
        .any(|capability| capability.state == CapabilityState::Revoked)
    {
        return Decision::Deny(DenyReason::CapabilityRevoked);
    }
    if credential_capabilities.iter().any(|capability| {
        capability.state == CapabilityState::Active && now_ms < capability.valid_from_ms
    }) {
        return Decision::Deny(DenyReason::CapabilityNotYetValid);
    }
    if credential_capabilities.iter().any(|capability| {
        capability.state == CapabilityState::Active && now_ms >= capability.expires_at_ms
    }) {
        return Decision::Deny(DenyReason::CapabilityExpired);
    }

    Decision::Deny(DenyReason::NoMatchingCapability)
}

fn policy_is_well_formed(policy: &PolicySnapshot) -> bool {
    if policy.epoch == 0 || policy.valid_from_ms >= policy.expires_at_ms {
        return false;
    }

    let mut capability_ids = BTreeSet::new();
    for capability in &policy.capabilities {
        if capability.target_id != policy.target_id
            || capability.valid_from_ms >= capability.expires_at_ms
            || capability.operations.is_empty()
            || !capability_ids.insert(capability.id.as_str())
        {
            return false;
        }

        let mut operations = BTreeSet::new();
        if capability
            .operations
            .iter()
            .any(|operation| !operations.insert(operation.as_str()))
        {
            return false;
        }
    }

    true
}

fn digest(character: char) -> Digest {
    Digest::parse(character.to_string().repeat(64)).expect("test digest is valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id<T>(value: &str, parse: impl FnOnce(String) -> Result<T, IdentityError>) -> T {
        parse(value.to_string()).expect("test identity is valid")
    }

    fn policy_id(value: &str) -> PolicyId {
        id(value, PolicyId::parse)
    }

    fn capability_id(value: &str) -> CapabilityId {
        id(value, CapabilityId::parse)
    }

    fn target_id(value: &str) -> TargetId {
        id(value, TargetId::parse)
    }

    fn service_id(value: &str) -> ServiceId {
        id(value, ServiceId::parse)
    }

    fn operation_id(value: &str) -> OperationId {
        id(value, OperationId::parse)
    }

    fn credential_id(value: &str) -> CredentialHandleId {
        id(value, CredentialHandleId::parse)
    }

    fn request_id(value: &str) -> RequestId {
        id(value, RequestId::parse)
    }

    fn capability() -> EgressCapability {
        EgressCapability {
            id: capability_id("repo-read"),
            target_id: target_id("agent-a"),
            subject_image_digest: digest('b'),
            service_id: service_id("code-host"),
            operations: vec![operation_id("repo.read_file")],
            credential_handle_id: Some(credential_id("cred.github.readonly")),
            valid_from_ms: 1_000,
            expires_at_ms: 8_000,
            state: CapabilityState::Active,
        }
    }

    fn policy() -> PolicySnapshot {
        PolicySnapshot {
            id: policy_id("egress-policy-v1"),
            digest: digest('a'),
            target_id: target_id("agent-a"),
            epoch: 7,
            valid_from_ms: 1_000,
            expires_at_ms: 9_000,
            authentication: AuthenticationState::Verified,
            revocation: RevocationState::Clear,
            capabilities: vec![capability()],
        }
    }

    fn observed() -> Fact<ObservedEgressAttempt> {
        Fact::verified(ObservedEgressAttempt {
            request_id: request_id("req-0001"),
            target_id: target_id("agent-a"),
            subject_image_digest: digest('b'),
            service_id: service_id("code-host"),
            operation_id: operation_id("repo.read_file"),
            credential_handle_id: Some(credential_id("cred.github.readonly")),
            redirect_hop: 0,
        })
    }

    fn context() -> EvaluationContext {
        EvaluationContext {
            now_ms: Some(2_000),
            clock_state: ClockState::Fresh,
            expected_policy_digest: Some(digest('a')),
            minimum_accepted_epoch: Some(7),
        }
    }

    fn deny(
        policy: &PolicySnapshot,
        observed: &Fact<ObservedEgressAttempt>,
        ctx: &EvaluationContext,
        reason: DenyReason,
    ) {
        assert_eq!(evaluate(policy, observed, ctx), Decision::Deny(reason));
    }

    #[test]
    fn exact_broker_service_operation_and_credential_handle_allow() {
        let Decision::Allow(receipt) = evaluate(&policy(), &observed(), &context()) else {
            panic!("exact service operation should be authorized");
        };
        assert_eq!(receipt.capability_id, capability_id("repo-read"));
        assert_eq!(receipt.service_id, service_id("code-host"));
        assert_eq!(receipt.operation_id, operation_id("repo.read_file"));
        assert_eq!(
            receipt.credential_handle_id,
            Some(credential_id("cred.github.readonly"))
        );
        assert_eq!(receipt.valid_until_ms, 8_000);
        assert_eq!(receipt.redirect_hop, 0);
    }

    #[test]
    fn opaque_ids_reject_urls_paths_uppercase_and_noncanonical_tokens() {
        assert_eq!(
            ServiceId::parse("https://api.example").unwrap_err(),
            IdentityError::NonCanonical
        );
        assert_eq!(
            OperationId::parse("../admin").unwrap_err(),
            IdentityError::NonCanonical
        );
        assert_eq!(
            CredentialHandleId::parse("Cred.GitHub").unwrap_err(),
            IdentityError::NonCanonical
        );
        assert_eq!(
            ServiceId::parse("api..internal").unwrap_err(),
            IdentityError::NonCanonical
        );
        assert_eq!(
            OperationId::parse("repo.read_file").unwrap().as_str(),
            "repo.read_file"
        );
        assert_eq!(ServiceId::parse("").unwrap_err(), IdentityError::Empty);
    }

    #[test]
    fn pinned_policy_digest_is_required_and_must_match() {
        let mut ctx = context();
        ctx.expected_policy_digest = None;
        deny(&policy(), &observed(), &ctx, DenyReason::MissingPinnedPolicyDigest);
        ctx.expected_policy_digest = Some(digest('d'));
        deny(&policy(), &observed(), &ctx, DenyReason::PolicyDigestMismatch);
    }

    #[test]
    fn unavailable_reversed_or_stale_clock_fails_closed() {
        let mut ctx = context();
        ctx.now_ms = None;
        deny(&policy(), &observed(), &ctx, DenyReason::MissingClock);
        ctx = context();
        ctx.clock_state = ClockState::Unavailable;
        deny(&policy(), &observed(), &ctx, DenyReason::ClockUnavailable);
        ctx.clock_state = ClockState::Reversed;
        deny(&policy(), &observed(), &ctx, DenyReason::ClockReversed);
        ctx.clock_state = ClockState::Stale;
        deny(&policy(), &observed(), &ctx, DenyReason::ClockStale);
    }

    #[test]
    fn rollback_floor_is_required_and_enforced() {
        let mut ctx = context();
        ctx.minimum_accepted_epoch = None;
        deny(&policy(), &observed(), &ctx, DenyReason::MissingRollbackFloor);
        ctx.minimum_accepted_epoch = Some(8);
        deny(&policy(), &observed(), &ctx, DenyReason::PolicyEpochRollback);
    }

    #[test]
    fn policy_authentication_missing_invalid_or_unknown_never_allows() {
        let mut current = policy();
        current.authentication = AuthenticationState::Missing;
        deny(&current, &observed(), &context(), DenyReason::PolicyAuthenticationMissing);
        current.authentication = AuthenticationState::Invalid;
        deny(&current, &observed(), &context(), DenyReason::PolicyAuthenticationInvalid);
        current.authentication = AuthenticationState::Unknown;
        deny(&current, &observed(), &context(), DenyReason::PolicyAuthenticationUnknown);
    }

    #[test]
    fn revoked_or_unknown_policy_never_allows() {
        let mut current = policy();
        current.revocation = RevocationState::Revoked;
        deny(&current, &observed(), &context(), DenyReason::PolicyRevoked);
        current.revocation = RevocationState::Unknown;
        deny(&current, &observed(), &context(), DenyReason::PolicyRevocationUnknown);
    }

    #[test]
    fn policy_validity_window_is_half_open() {
        let current = policy();
        let mut ctx = context();
        ctx.now_ms = Some(999);
        deny(&current, &observed(), &ctx, DenyReason::PolicyNotYetValid);
        ctx.now_ms = Some(9_000);
        deny(&current, &observed(), &ctx, DenyReason::PolicyExpired);
    }

    #[test]
    fn malformed_policy_or_duplicate_capability_ids_fails_closed() {
        let mut current = policy();
        current.epoch = 0;
        deny(&current, &observed(), &context(), DenyReason::MalformedPolicy);
        current = policy();
        current.capabilities.push(current.capabilities[0].clone());
        deny(&current, &observed(), &context(), DenyReason::MalformedPolicy);
        current = policy();
        current.capabilities[0].operations.push(operation_id("repo.read_file"));
        deny(&current, &observed(), &context(), DenyReason::MalformedPolicy);
    }

    #[test]
    fn missing_unverified_or_contradictory_observation_fails_closed() {
        let current = policy();
        deny(
            &current,
            &Fact::missing(),
            &context(),
            DenyReason::MissingObservedAttempt,
        );
        deny(
            &current,
            &Fact::unverified(observed().value.unwrap()),
            &context(),
            DenyReason::UnverifiedObservedAttempt,
        );
        deny(
            &current,
            &Fact::contradictory(observed().value.unwrap()),
            &context(),
            DenyReason::ContradictoryObservedAttempt,
        );
        deny(
            &current,
            &Fact {
                status: EvidenceStatus::Verified,
                value: None,
            },
            &context(),
            DenyReason::MissingObservedAttempt,
        );
    }

    #[test]
    fn target_and_subject_are_exactly_bound() {
        let current = policy();
        let mut attempt = observed().value.unwrap();
        attempt.target_id = target_id("agent-b");
        deny(
            &current,
            &Fact::verified(attempt.clone()),
            &context(),
            DenyReason::TargetMismatch,
        );
        attempt = observed().value.unwrap();
        attempt.subject_image_digest = digest('e');
        deny(
            &current,
            &Fact::verified(attempt),
            &context(),
            DenyReason::SubjectMismatch,
        );
    }

    #[test]
    fn service_and_operation_must_match_broker_registry_ids() {
        let current = policy();
        let mut attempt = observed().value.unwrap();
        attempt.service_id = service_id("other-service");
        deny(
            &current,
            &Fact::verified(attempt.clone()),
            &context(),
            DenyReason::ServiceNotGranted,
        );
        attempt = observed().value.unwrap();
        attempt.operation_id = operation_id("repo.write_file");
        deny(
            &current,
            &Fact::verified(attempt),
            &context(),
            DenyReason::OperationNotGranted,
        );
    }

    #[test]
    fn credential_handle_must_match_the_same_capability() {
        let current = policy();
        let mut attempt = observed().value.unwrap();
        attempt.credential_handle_id = Some(credential_id("cred.github.admin"));
        deny(
            &current,
            &Fact::verified(attempt),
            &context(),
            DenyReason::CredentialHandleMismatch,
        );
    }

    #[test]
    fn credential_free_operation_does_not_accept_a_credential_handle() {
        let mut current = policy();
        current.capabilities[0].credential_handle_id = None;
        let mut attempt = observed().value.unwrap();
        attempt.credential_handle_id = Some(credential_id("cred.github.readonly"));
        deny(
            &current,
            &Fact::verified(attempt.clone()),
            &context(),
            DenyReason::CredentialHandleMismatch,
        );
        attempt.credential_handle_id = None;
        assert!(matches!(
            evaluate(&current, &Fact::verified(attempt), &context()),
            Decision::Allow(_)
        ));
    }

    #[test]
    fn service_operation_and_credential_are_not_composed_across_capabilities() {
        let mut current = policy();
        let mut operation_only = current.capabilities[0].clone();
        operation_only.id = capability_id("operation-only");
        operation_only.credential_handle_id = None;
        let mut credential_only = current.capabilities[0].clone();
        credential_only.id = capability_id("credential-only");
        credential_only.operations = vec![operation_id("repo.write_file")];
        current.capabilities = vec![operation_only, credential_only];
        deny(
            &current,
            &observed(),
            &context(),
            DenyReason::CredentialHandleMismatch,
        );
    }

    #[test]
    fn revoked_unknown_expired_and_future_capabilities_fail_closed() {
        let mut current = policy();
        current.capabilities[0].state = CapabilityState::Revoked;
        deny(&current, &observed(), &context(), DenyReason::CapabilityRevoked);
        current.capabilities[0].state = CapabilityState::Unknown;
        deny(&current, &observed(), &context(), DenyReason::CapabilityRevocationUnknown);
        current.capabilities[0].state = CapabilityState::Active;
        current.capabilities[0].expires_at_ms = 2_000;
        deny(&current, &observed(), &context(), DenyReason::CapabilityExpired);
        current.capabilities[0].valid_from_ms = 3_000;
        current.capabilities[0].expires_at_ms = 8_000;
        deny(&current, &observed(), &context(), DenyReason::CapabilityNotYetValid);
    }

    #[test]
    fn exact_scope_active_capability_is_selected_independent_of_vector_order() {
        let mut current = policy();
        let mut unknown = current.capabilities[0].clone();
        unknown.id = capability_id("unknown-copy");
        unknown.state = CapabilityState::Unknown;
        current.capabilities.push(unknown);
        assert!(matches!(
            evaluate(&current, &observed(), &context()),
            Decision::Allow(_)
        ));
        current.capabilities.reverse();
        assert!(matches!(
            evaluate(&current, &observed(), &context()),
            Decision::Allow(_)
        ));
    }

    #[test]
    fn redirect_hop_is_recorded_and_must_be_authorized_as_a_new_attempt() {
        let mut attempt = observed().value.unwrap();
        attempt.redirect_hop = 1;
        let Decision::Allow(receipt) =
            evaluate(&policy(), &Fact::verified(attempt), &context())
        else {
            panic!("the broker may authorize a redirect hop only with a fresh evaluation");
        };
        assert_eq!(receipt.redirect_hop, 1);
        assert_eq!(receipt.valid_until_ms, 8_000);
    }

    #[test]
    fn receipt_contains_handle_id_but_not_raw_secret_or_free_form_url() {
        let Decision::Allow(receipt) = evaluate(&policy(), &observed(), &context()) else {
            panic!("exact service operation should be allowed");
        };
        let text = format!("{receipt:?}");
        assert!(!text.contains("https://"));
        assert!(!text.contains("Bearer "));
        assert!(!text.contains("token="));
        assert!(text.contains("cred.github.readonly"));
    }
}
