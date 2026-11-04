//! Act-bound handle verification port and fail-closed policy binding.
//!
//! This module separates cryptographic envelope verification from the
//! participant finality sink. The adapter is trusted computing base: it must
//! verify DSSE PAE signatures, signer key identities, expected payload type,
//! payload canonicalization, and the verifier workload's SPIFFE identity and
//! trust bundle before returning VerifiedExecutionHandleEnvelope.
//!
//! The core then independently checks exact CandidateAct binding, audience,
//! issuer/key allowlists, threshold cardinality, policy/trust-bundle identity,
//! envelope-digest binding, validity and maximum verification age before it
//! mints the opaque AuthenticatedExecutionHandle.
//!
//! No cryptography is implemented here. Test adapters demonstrate structural
//! rejection only. A production adapter must be backed by independently
//! reviewed DSSE/JCS/SPIFFE libraries and known-answer/interoperability vectors.

use crate::finality::{
    AuthenticatedExecutionHandle, CandidateAct, ExecutionHandleClaims,
    ExecutionHandleVerificationReceipt, FinalityError,
};
use crate::Digest;
use std::collections::BTreeSet;

pub const EXECUTION_HANDLE_PAYLOAD_TYPE: &str =
    "application/vnd.luminous.execution-handle.v1+json";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionHandleVerificationReceipt {
    pub payload_type: String,
    pub envelope_digest: Digest,
    pub canonical_payload_digest: Digest,
    pub verified_signer_key_ids: Vec<String>,
    pub required_signer_threshold: usize,
    pub verifier_spiffe_id: String,
    pub verifier_trust_bundle_digest: Digest,
    pub signing_policy_digest: Digest,
    pub verified_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandleVerificationPolicy {
    pub expected_payload_type: String,
    pub allowed_issuer_ids: Vec<String>,
    pub trusted_signer_key_ids: Vec<String>,
    pub required_distinct_signers: usize,
    pub verifier_spiffe_id: String,
    pub verifier_trust_bundle_digest: Digest,
    pub signing_policy_digest: Digest,
    pub maximum_handle_lifetime_ms: u64,
    pub maximum_verification_age_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedExecutionHandleEnvelope {
    pub claims: ExecutionHandleClaims,
    pub payload_type: String,
    /// SHA-256 over the exact DSSE envelope bytes as received.
    pub envelope_digest: Digest,
    /// SHA-256 over the exact JCS-canonical payload bytes that were verified.
    pub canonical_payload_digest: Digest,
    /// Distinct key identifiers whose DSSE signatures were actually verified.
    pub verified_signer_key_ids: Vec<String>,
    /// SPIFFE ID of the workload that performed verification.
    pub verifier_spiffe_id: String,
    /// Trust-bundle digest used to authenticate the verifier workload.
    pub verifier_trust_bundle_digest: Digest,
    /// Immutable signer/policy profile selected by the finality authority.
    pub signing_policy_digest: Digest,
    pub verified_at_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvelopeAdapterError {
    InvalidEncoding,
    UnsupportedEnvelopeVersion,
    PayloadTypeMismatch,
    SignatureInvalid,
    SignatureThresholdNotMet,
    UntrustedSigningKey,
    InvalidCanonicalPayload,
    WorkloadIdentityInvalid,
    TrustBundleUnavailable,
    VerificationUnavailable,
}

/// The adapter is part of the trusted computing base.
///
/// A successful return must mean:
/// - DSSE PAE signature verification succeeded over the exact payload type and
///   payload bytes (not a separately decoded/re-serialized payload);
/// - every reported signer key ID corresponds to a distinct verified key;
/// - the payload was parsed as the declared schema and canonicalization profile;
/// - the envelope digest is computed over the exact raw envelope bytes;
/// - the verifier workload's SPIFFE identity was verified against the
///   specified trust bundle, and that bundle digest is reported;
/// - the signing policy used for the act issuer/key set is identified by its
///   immutable digest.
///
/// The core does not trust a model, tool transcript, or caller-supplied boolean.
/// It trusts the configured adapter implementation and independently checks
/// the returned claims against the local verification policy.
pub trait ExecutionHandleEnvelopeVerifier: Send + Sync {
    fn verify_and_decode(
        &self,
        envelope: &[u8],
        policy: &HandleVerificationPolicy,
        now_ms: u64,
    ) -> Result<VerifiedExecutionHandleEnvelope, EnvelopeAdapterError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HandleVerificationError {
    EmptyEnvelope,
    InvalidPolicy(&'static str),
    AdapterRejected(EnvelopeAdapterError),
    PayloadTypeMismatch,
    CandidateActMismatch,
    WrongParticipantAudience,
    UntrustedIssuer,
    UntrustedSigner,
    DuplicateSigner,
    InsufficientDistinctSigners,
    WrongVerifierWorkload,
    TrustBundleMismatch,
    SigningPolicyMismatch,
    InvalidDigestBinding,
    InvalidValidityInterval,
    HandleNotYetValid,
    HandleExpired,
    ExcessiveHandleLifetime,
    VerificationFromFuture,
    VerificationTooOld,
    InvalidHandle(FinalityError),
}

fn validate_policy(policy: &HandleVerificationPolicy) -> Result<(), HandleVerificationError> {
    if policy.expected_payload_type != EXECUTION_HANDLE_PAYLOAD_TYPE {
        return Err(HandleVerificationError::InvalidPolicy("payload type"));
    }
    if policy.allowed_issuer_ids.is_empty()
        || policy.allowed_issuer_ids.iter().any(String::is_empty)
    {
        return Err(HandleVerificationError::InvalidPolicy("issuer allowlist"));
    }
    if policy.trusted_signer_key_ids.is_empty()
        || policy.trusted_signer_key_ids.iter().any(String::is_empty)
    {
        return Err(HandleVerificationError::InvalidPolicy("signer key allowlist"));
    }
    if policy.required_distinct_signers == 0
        || policy.required_distinct_signers > policy.trusted_signer_key_ids.len()
    {
        return Err(HandleVerificationError::InvalidPolicy("signature threshold"));
    }
    if !policy.verifier_spiffe_id.starts_with("spiffe://") {
        return Err(HandleVerificationError::InvalidPolicy("verifier SPIFFE ID"));
    }
    if policy.verifier_trust_bundle_digest == Digest::ZERO {
        return Err(HandleVerificationError::InvalidPolicy("trust bundle digest"));
    }
    if policy.signing_policy_digest == Digest::ZERO {
        return Err(HandleVerificationError::InvalidPolicy("signing policy digest"));
    }
    if policy.maximum_handle_lifetime_ms == 0 {
        return Err(HandleVerificationError::InvalidPolicy("maximum handle lifetime"));
    }
    if policy.maximum_verification_age_ms == 0 {
        return Err(HandleVerificationError::InvalidPolicy("maximum verification age"));
    }
    Ok(())
}

/// Verify a signed envelope and mint the opaque handle capability.
///
/// The returned capability can be consumed by FinalitySink. It is not a
/// transferable bearer token: it is non-Clone/non-Serialize and its fields
/// are private. The participant still rechecks act/audience/expiry/policy
/// epoch and atomically consumes the handle/nonce with the effect.
pub fn verify_execution_handle<V: ExecutionHandleEnvelopeVerifier>(
    verifier: &V,
    envelope: &[u8],
    expected_act: &CandidateAct,
    policy: &HandleVerificationPolicy,
    now_ms: u64,
) -> Result<AuthenticatedExecutionHandle, HandleVerificationError> {
    if envelope.is_empty() {
        return Err(HandleVerificationError::EmptyEnvelope);
    }
    validate_policy(policy)?;

    let verified = verifier
        .verify_and_decode(envelope, policy, now_ms)
        .map_err(HandleVerificationError::AdapterRejected)?;

    if verified.payload_type != policy.expected_payload_type {
        return Err(HandleVerificationError::PayloadTypeMismatch);
    }
    if verified.claims.act != *expected_act {
        return Err(HandleVerificationError::CandidateActMismatch);
    }
    if verified.claims.audience != expected_act.participant_id {
        return Err(HandleVerificationError::WrongParticipantAudience);
    }
    if !policy.allowed_issuer_ids.contains(&verified.claims.issuer_id) {
        return Err(HandleVerificationError::UntrustedIssuer);
    }
    if verified.verifier_spiffe_id != policy.verifier_spiffe_id {
        return Err(HandleVerificationError::WrongVerifierWorkload);
    }
    if verified.verifier_trust_bundle_digest != policy.verifier_trust_bundle_digest {
        return Err(HandleVerificationError::TrustBundleMismatch);
    }
    if verified.signing_policy_digest != policy.signing_policy_digest {
        return Err(HandleVerificationError::SigningPolicyMismatch);
    }

    if verified.claims.handle_id.is_empty()
        || verified.claims.nonce_digest == Digest::ZERO
        || verified.claims.handle_digest == Digest::ZERO
        || verified.envelope_digest == Digest::ZERO
        || verified.canonical_payload_digest == Digest::ZERO
    {
        return Err(HandleVerificationError::InvalidDigestBinding);
    }
    if verified.claims.handle_digest != verified.envelope_digest {
        return Err(HandleVerificationError::InvalidDigestBinding);
    }

    let trusted_signers: BTreeSet<&str> = policy
        .trusted_signer_key_ids
        .iter()
        .map(String::as_str)
        .collect();
    let mut observed_signers = BTreeSet::new();
    for signer in &verified.verified_signer_key_ids {
        if !observed_signers.insert(signer.as_str()) {
            return Err(HandleVerificationError::DuplicateSigner);
        }
        if !trusted_signers.contains(signer.as_str()) {
            return Err(HandleVerificationError::UntrustedSigner);
        }
    }
    if observed_signers.len() < policy.required_distinct_signers {
        return Err(HandleVerificationError::InsufficientDistinctSigners);
    }

    let claims = &verified.claims;
    if claims.issued_at_ms > claims.not_before_ms
        || claims.not_before_ms >= claims.expires_at_ms
    {
        return Err(HandleVerificationError::InvalidValidityInterval);
    }
    if now_ms < claims.issued_at_ms || now_ms < claims.not_before_ms {
        return Err(HandleVerificationError::HandleNotYetValid);
    }
    if now_ms >= claims.expires_at_ms {
        return Err(HandleVerificationError::HandleExpired);
    }
    if claims.expires_at_ms - claims.issued_at_ms > policy.maximum_handle_lifetime_ms {
        return Err(HandleVerificationError::ExcessiveHandleLifetime);
    }
    if verified.verified_at_ms < claims.issued_at_ms || verified.verified_at_ms > now_ms {
        return Err(HandleVerificationError::VerificationFromFuture);
    }
    if now_ms - verified.verified_at_ms > policy.maximum_verification_age_ms {
        return Err(HandleVerificationError::VerificationTooOld);
    }

    let receipt = ExecutionHandleVerificationReceipt {
        payload_type: verified.payload_type,
        envelope_digest: verified.envelope_digest,
        canonical_payload_digest: verified.canonical_payload_digest,
        verified_signer_key_ids: verified.verified_signer_key_ids,
        verifier_spiffe_id: verified.verifier_spiffe_id,
        verifier_trust_bundle_digest: verified.verifier_trust_bundle_digest,
        signing_policy_digest: verified.signing_policy_digest,
        verified_at_ms: verified.verified_at_ms,
    };
    AuthenticatedExecutionHandle::from_verified_claims(verified.claims, receipt)
        .map_err(HandleVerificationError::InvalidHandle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(byte: u8) -> Digest {
        Digest::from_byte(byte)
    }

    fn act() -> CandidateAct {
        CandidateAct {
            canonicalization_profile: "RFC8785".into(),
            tenant_id: "tenant-a".into(),
            principal_id: "principal-a".into(),
            episode_id: "episode-1".into(),
            plan_digest: d(1),
            step_id: "step-1".into(),
            grant_id: "grant-1".into(),
            authorization_scope: "service:api:restart".into(),
            tool_id: "service.restart".into(),
            operation_id: "restart".into(),
            manifest_version: "1".into(),
            manifest_digest: d(2),
            toolchain_id: "toolchain".into(),
            toolchain_digest: d(3),
            policy_version: "p1".into(),
            policy_digest: d(4),
            policy_epoch: 7,
            participant_id: "participant-a".into(),
            target_digest: d(5),
            request_digest: d(6),
            attempt_id: "attempt-1".into(),
            logical_action_key: "episode-1/step-1".into(),
            environment_profile_digest: d(7),
            runtime_evidence_bundle_digest: d(8),
            risk_level: 1,
        }
    }

    fn policy() -> HandleVerificationPolicy {
        HandleVerificationPolicy {
            expected_payload_type: EXECUTION_HANDLE_PAYLOAD_TYPE.into(),
            allowed_issuer_ids: vec!["finality-authority".into()],
            trusted_signer_key_ids: vec!["key-a".into(), "key-b".into()],
            required_distinct_signers: 2,
            verifier_spiffe_id: "spiffe://luminous.example/agent-effect-gate".into(),
            verifier_trust_bundle_digest: d(21),
            signing_policy_digest: d(22),
            maximum_handle_lifetime_ms: 600_000,
            maximum_verification_age_ms: 5_000,
        }
    }

    fn envelope(act: CandidateAct) -> VerifiedExecutionHandleEnvelope {
        VerifiedExecutionHandleEnvelope {
            claims: ExecutionHandleClaims {
                handle_id: "handle-1".into(),
                nonce_digest: d(9),
                handle_digest: d(10),
                issuer_id: "finality-authority".into(),
                audience: "participant-a".into(),
                act,
                issued_at_ms: 100,
                not_before_ms: 100,
                expires_at_ms: 10_000,
            },
            payload_type: EXECUTION_HANDLE_PAYLOAD_TYPE.into(),
            envelope_digest: d(10),
            canonical_payload_digest: d(11),
            verified_signer_key_ids: vec!["key-a".into(), "key-b".into()],
            verifier_spiffe_id: "spiffe://luminous.example/agent-effect-gate".into(),
            verifier_trust_bundle_digest: d(21),
            signing_policy_digest: d(22),
            verified_at_ms: 200,
        }
    }

    struct FakeVerifier {
        result: Result<VerifiedExecutionHandleEnvelope, EnvelopeAdapterError>,
    }

    impl ExecutionHandleEnvelopeVerifier for FakeVerifier {
        fn verify_and_decode(
            &self,
            _envelope: &[u8],
            _policy: &HandleVerificationPolicy,
            _now_ms: u64,
        ) -> Result<VerifiedExecutionHandleEnvelope, EnvelopeAdapterError> {
            self.result.clone()
        }
    }

    fn accepted(act: CandidateAct) -> FakeVerifier {
        FakeVerifier { result: Ok(envelope(act)) }
    }

    #[test]
    fn verified_envelope_mints_opaque_handle_for_exact_act() {
        let act = act();
        let handle = verify_execution_handle(&accepted(act.clone()), b"signed-envelope", &act, &policy(), 300);
        assert!(handle.is_ok());
    }

    #[test]
    fn adapter_failure_fails_closed() {
        let act = act();
        let verifier = FakeVerifier { result: Err(EnvelopeAdapterError::SignatureInvalid) };
        assert_eq!(
            verify_execution_handle(&verifier, b"signed", &act, &policy(), 300),
            Err(HandleVerificationError::AdapterRejected(EnvelopeAdapterError::SignatureInvalid))
        );
    }

    #[test]
    fn empty_envelope_is_rejected_before_adapter_call() {
        let act = act();
        assert_eq!(
            verify_execution_handle(&accepted(act.clone()), b"", &act, &policy(), 300),
            Err(HandleVerificationError::EmptyEnvelope)
        );
    }

    #[test]
    fn wrong_payload_type_or_candidate_act_is_rejected() {
        let mut wrong_type = envelope(act());
        wrong_type.payload_type = "application/json".into();
        assert_eq!(
            verify_execution_handle(
                &FakeVerifier { result: Ok(wrong_type) },
                b"signed", &act(), &policy(), 300
            ),
            Err(HandleVerificationError::PayloadTypeMismatch)
        );

        let mut wrong_act = envelope(act());
        wrong_act.claims.act.target_digest = d(99);
        assert_eq!(
            verify_execution_handle(
                &FakeVerifier { result: Ok(wrong_act) },
                b"signed", &act(), &policy(), 300
            ),
            Err(HandleVerificationError::CandidateActMismatch)
        );
    }

    #[test]
    fn wrong_audience_or_issuer_is_rejected() {
        let mut wrong_audience = envelope(act());
        wrong_audience.claims.audience = "participant-b".into();
        assert_eq!(
            verify_execution_handle(
                &FakeVerifier { result: Ok(wrong_audience) },
                b"signed", &act(), &policy(), 300
            ),
            Err(HandleVerificationError::WrongParticipantAudience)
        );

        let mut wrong_issuer = envelope(act());
        wrong_issuer.claims.issuer_id = "untrusted".into();
        assert_eq!(
            verify_execution_handle(
                &FakeVerifier { result: Ok(wrong_issuer) },
                b"signed", &act(), &policy(), 300
            ),
            Err(HandleVerificationError::UntrustedIssuer)
        );
    }

    #[test]
    fn signer_threshold_counts_distinct_keys_and_rejects_unknown_keys() {
        let mut duplicate = envelope(act());
        duplicate.verified_signer_key_ids = vec!["key-a".into(), "key-a".into()];
        assert_eq!(
            verify_execution_handle(
                &FakeVerifier { result: Ok(duplicate) },
                b"signed", &act(), &policy(), 300
            ),
            Err(HandleVerificationError::DuplicateSigner)
        );

        let mut insufficient = envelope(act());
        insufficient.verified_signer_key_ids = vec!["key-a".into()];
        assert_eq!(
            verify_execution_handle(
                &FakeVerifier { result: Ok(insufficient) },
                b"signed", &act(), &policy(), 300
            ),
            Err(HandleVerificationError::InsufficientDistinctSigners)
        );

        let mut untrusted = envelope(act());
        untrusted.verified_signer_key_ids = vec!["key-a".into(), "rogue".into()];
        assert_eq!(
            verify_execution_handle(
                &FakeVerifier { result: Ok(untrusted) },
                b"signed", &act(), &policy(), 300
            ),
            Err(HandleVerificationError::UntrustedSigner)
        );
    }

    #[test]
    fn workload_identity_and_trust_bundle_are_exactly_bound() {
        let mut wrong_identity = envelope(act());
        wrong_identity.verifier_spiffe_id = "spiffe://attacker/rogue".into();
        assert_eq!(
            verify_execution_handle(
                &FakeVerifier { result: Ok(wrong_identity) },
                b"signed", &act(), &policy(), 300
            ),
            Err(HandleVerificationError::WrongVerifierWorkload)
        );

        let mut wrong_bundle = envelope(act());
        wrong_bundle.verifier_trust_bundle_digest = d(88);
        assert_eq!(
            verify_execution_handle(
                &FakeVerifier { result: Ok(wrong_bundle) },
                b"signed", &act(), &policy(), 300
            ),
            Err(HandleVerificationError::TrustBundleMismatch)
        );
    }

    #[test]
    fn signing_policy_and_envelope_digest_are_bound() {
        let mut wrong_policy = envelope(act());
        wrong_policy.signing_policy_digest = d(90);
        assert_eq!(
            verify_execution_handle(
                &FakeVerifier { result: Ok(wrong_policy) },
                b"signed", &act(), &policy(), 300
            ),
            Err(HandleVerificationError::SigningPolicyMismatch)
        );

        let mut wrong_digest = envelope(act());
        wrong_digest.envelope_digest = d(12);
        assert_eq!(
            verify_execution_handle(
                &FakeVerifier { result: Ok(wrong_digest) },
                b"signed", &act(), &policy(), 300
            ),
            Err(HandleVerificationError::InvalidDigestBinding)
        );
    }

    #[test]
    fn verification_freshness_and_handle_lifetime_are_enforced() {
        let mut future = envelope(act());
        future.verified_at_ms = 301;
        assert_eq!(
            verify_execution_handle(
                &FakeVerifier { result: Ok(future) },
                b"signed", &act(), &policy(), 300
            ),
            Err(HandleVerificationError::VerificationFromFuture)
        );

        let mut stale = envelope(act());
        stale.verified_at_ms = 100;
        assert_eq!(
            verify_execution_handle(
                &FakeVerifier { result: Ok(stale) },
                b"signed", &act(), &policy(), 5_200
            ),
            Err(HandleVerificationError::VerificationTooOld)
        );

        let mut too_long = envelope(act());
        too_long.claims.expires_at_ms = 700_101;
        assert_eq!(
            verify_execution_handle(
                &FakeVerifier { result: Ok(too_long) },
                b"signed", &act(), &policy(), 300
            ),
            Err(HandleVerificationError::ExcessiveHandleLifetime)
        );
    }

    #[test]
    fn invalid_policy_is_rejected_before_adapter_invocation() {
        let mut policy = policy();
        policy.required_distinct_signers = 0;
        let act = act();
        assert_eq!(
            verify_execution_handle(&accepted(act.clone()), b"signed", &act, &policy, 300),
            Err(HandleVerificationError::InvalidPolicy("signature threshold"))
        );
    }
}
