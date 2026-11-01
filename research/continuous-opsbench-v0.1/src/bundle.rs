//! Portable, offline-verifiable claim/evidence bundle structure for OpsBench.
//!
//! This validator does not implement cryptography, fetch artifacts, or authenticate
//! signatures. It consumes observations from the existing typed source-evidence
//! adapter, which must verify DSSE/in-toto (or equivalent) attestations, trust roots,
//! canonical artifact bytes, issuer identity and durable replay before constructing
//! these records. No network or filesystem APIs are used here.

use crate::sealed::EvaluationLane;
use crate::Digest;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const BUNDLE_SCHEMA: &str = "continuous-opsbench-evidence-bundle-v0.1";
pub const BUNDLE_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BundleVisibility {
    Public,
    Restricted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArtifactAvailability {
    Embedded,
    CommitmentOnly,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArtifactSensitivity {
    Public,
    Restricted,
    ActiveSealedScenario,
    PrivateOracle,
    RetiredReveal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceRole {
    FreezeAuthority,
    Evaluator,
    ParticipantLedger,
    ServiceObserver,
    RuntimeObserver,
    SandboxSupervisor,
    PrivacyDisclosureObserver,
    PrivacyGrader,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthenticationDecision {
    Authenticated,
    Rejected,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimStatus {
    Pass,
    Fail,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClaimType {
    Authorization,
    ExternalEffect,
    ServiceOutcome,
    RequestImpact,
    Safety,
    Calibration,
    ContextualPrivacy,
    RuntimeEnforcement,
    SandboxTier,
    ResourceTelemetry,
    EnergyMeasurement,
    StatisticalComparison,
    ContaminationControl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BundleIntegrity {
    Verified,
    Indeterminate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustedPublisher {
    pub publisher_id: String,
    pub source_id: String,
    pub control_domain: String,
    pub role: EvidenceRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustPolicyReference {
    pub policy_id: String,
    pub policy_digest: Digest,
    pub verifier_id: String,
    pub verifier_build_digest: Digest,
    pub trust_roots_digest: Digest,
    pub identity_constraints_digest: Digest,
    pub revocation_policy_digest: Digest,
    pub policy_as_of_ms: u64,
    pub maximum_attestation_age_ms: u64,
    pub minimum_verifier_version: String,
    pub trusted_publishers: Vec<TrustedPublisher>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactDescriptor {
    pub artifact_id: String,
    pub content_digest: Digest,
    /// Required for Embedded artifacts. CommitmentOnly has no payload byte length.
    pub byte_length: Option<u64>,
    pub media_type: String,
    pub publisher_id: String,
    pub source_id: String,
    pub control_domain: String,
    pub role: EvidenceRole,
    pub sequence: u64,
    pub issued_at_ms: u64,
    pub availability: ArtifactAvailability,
    pub sensitivity: ArtifactSensitivity,
    pub required_for_verification: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactAuthenticationReceipt {
    pub artifact_id: String,
    pub subject_digest: Digest,
    pub manifest_commitment: Digest,
    pub publisher_id: String,
    pub source_id: String,
    pub control_domain: String,
    pub role: EvidenceRole,
    pub sequence: u64,
    pub issued_at_ms: u64,
    pub verified_at_ms: u64,
    pub observed_content_digest: Option<Digest>,
    pub observed_byte_length: Option<u64>,
    pub trust_policy_digest: Digest,
    pub verifier_id: String,
    pub verifier_build_digest: Digest,
    pub decision: AuthenticationDecision,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimSubject {
    pub run_id: String,
    pub trial_id: Option<String>,
    pub arm: Option<String>,
    pub subject_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceClaim {
    pub claim_id: String,
    pub claim_type: ClaimType,
    pub subject: ClaimSubject,
    pub status: ClaimStatus,
    pub verifier_id: String,
    pub verifier_digest: Digest,
    pub evidence_digests: Vec<Digest>,
    pub dependencies: Vec<String>,
    pub assumptions: Vec<String>,
    pub claim_ceiling: String,
    pub status_reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceBundleManifest {
    pub schema: String,
    pub version: u16,
    pub bundle_id: String,
    pub run_id: String,
    pub lane: EvaluationLane,
    pub visibility: BundleVisibility,
    pub created_at_ms: u64,
    pub manifest_commitment: Digest,
    /// Calculated by the bundle producer over canonical manifest + inventory bytes.
    /// This validator compares the externally verified value; it does not hash bytes.
    pub bundle_digest: Digest,
    pub verifier_digest: Digest,
    pub trust_policy: TrustPolicyReference,
    pub planned_trial_ids: Vec<String>,
    pub artifacts: Vec<ArtifactDescriptor>,
    pub claims: Vec<EvidenceClaim>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimVerificationResult {
    pub claim_id: String,
    pub claim_type: ClaimType,
    pub status: ClaimStatus,
    pub verified_evidence_count: usize,
    pub indeterminate_evidence_count: usize,
    pub dependencies: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleVerificationReceipt {
    pub schema: String,
    pub bundle_id: String,
    pub run_id: String,
    pub lane: EvaluationLane,
    pub visibility: BundleVisibility,
    pub bundle_digest: Digest,
    pub manifest_commitment: Digest,
    pub verifier_id: String,
    pub verifier_digest: Digest,
    pub trust_policy_digest: Digest,
    pub verified_at_ms: u64,
    pub artifact_count: usize,
    pub authenticated_artifact_count: usize,
    pub indeterminate_artifact_count: usize,
    pub claim_count: usize,
    pub passing_claim_count: usize,
    pub failing_claim_count: usize,
    pub indeterminate_claim_count: usize,
    pub integrity: BundleIntegrity,
    pub claims: Vec<ClaimVerificationResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BundleVerificationError {
    UnsupportedSchema,
    InvalidManifestIdentity,
    InvalidDigest(&'static str),
    InvalidTrustPolicy,
    UntrustedPublisher(String),
    DuplicateTrustedPublisher,
    DuplicateArtifactId(String),
    DuplicateArtifactDigest,
    InvalidArtifact(String),
    ArtifactSourceMismatch(String),
    ArtifactTrustReceiptMissing(String),
    DuplicateArtifactReceipt(String),
    ArtifactReceiptBindingMismatch(String),
    ArtifactReceiptRejected(String),
    EmbeddedArtifactBytesMismatch(String),
    CommitmentArtifactHasBytes(String),
    UnavailableArtifactAuthenticated(String),
    ActiveSealedPayloadEmbedded(String),
    RestrictedArtifactEmbedded(String),
    RetiredRevealInWrongLane(String),
    PublicBundleContainsRestrictedRetiredReveal(String),
    StaleArtifactReceipt(String),
    ReceiptFromFuture(String),
    DuplicateSourceSequence(String),
    SourceSequenceOrderInvalid(String),
    DuplicateTrialId(String),
    UnknownClaimDependency(String, String),
    DuplicateClaimId(String),
    ClaimDependencyCycle,
    ClaimRunMismatch(String),
    ClaimSubjectInvalid(String),
    ClaimVerifierUnpinned(String),
    ClaimEvidenceMissing(String),
    ClaimEvidenceUnknown(String, Digest),
    ClaimPassingOnUnverifiedEvidence(String),
    ClaimStatusLaundering(String),
    RequiredArtifactUnreferenced(String),
    EmptyClaimGraph,
    BundleDigestMismatch,
    ManifestCommitmentMismatch,
    VerificationTimeBeforeBundle,
    ArithmeticOverflow,
}

pub fn verify_evidence_bundle(
    manifest: &EvidenceBundleManifest,
    expected_bundle_digest: &Digest,
    expected_manifest_commitment: &Digest,
    expected_verifier_digest: &Digest,
    verified_at_ms: u64,
    authentication_receipts: &[ArtifactAuthenticationReceipt],
) -> Result<BundleVerificationReceipt, BundleVerificationError> {
    validate_manifest(manifest)?;
    if expected_bundle_digest != &manifest.bundle_digest {
        return Err(BundleVerificationError::BundleDigestMismatch);
    }
    if expected_manifest_commitment != &manifest.manifest_commitment {
        return Err(BundleVerificationError::ManifestCommitmentMismatch);
    }
    if expected_verifier_digest != &manifest.trust_policy.verifier_build_digest
        || expected_verifier_digest != &manifest.verifier_digest
    {
        return Err(BundleVerificationError::InvalidTrustPolicy);
    }
    if verified_at_ms < manifest.created_at_ms
        || verified_at_ms < manifest.trust_policy.policy_as_of_ms
    {
        return Err(BundleVerificationError::VerificationTimeBeforeBundle);
    }

    let mut artifacts_by_id: BTreeMap<&str, &ArtifactDescriptor> = BTreeMap::new();
    let mut artifacts_by_digest: BTreeMap<&Digest, &ArtifactDescriptor> = BTreeMap::new();
    let mut required_artifact_ids = BTreeSet::new();
    for artifact in &manifest.artifacts {
        if artifacts_by_id.insert(artifact.artifact_id.as_str(), artifact).is_some() {
            return Err(BundleVerificationError::DuplicateArtifactId(
                artifact.artifact_id.clone(),
            ));
        }
        if artifacts_by_digest.insert(&artifact.content_digest, artifact).is_some() {
            return Err(BundleVerificationError::DuplicateArtifactDigest);
        }
        validate_artifact_descriptor(manifest, artifact)?;
        if artifact.required_for_verification {
            required_artifact_ids.insert(artifact.artifact_id.as_str());
        }
    }

    let mut receipt_by_id: BTreeMap<&str, &ArtifactAuthenticationReceipt> = BTreeMap::new();
    let mut source_sequences: BTreeSet<(&str, &str, u64)> = BTreeSet::new();
    let mut receipt_order: BTreeMap<(&str, &str), Vec<(u64, u64)>> = BTreeMap::new();
    let mut artifact_indeterminate = 0usize;
    let mut artifact_authenticated = 0usize;
    let mut artifact_referenced: BTreeSet<&Digest> = BTreeSet::new();

    for receipt in authentication_receipts {
        let artifact = artifacts_by_id
            .get(receipt.artifact_id.as_str())
            .copied()
            .ok_or_else(|| BundleVerificationError::ArtifactSourceMismatch(
                receipt.artifact_id.clone(),
            ))?;
        if receipt_by_id.insert(receipt.artifact_id.as_str(), receipt).is_some() {
            return Err(BundleVerificationError::DuplicateArtifactReceipt(
                receipt.artifact_id.clone(),
            ));
        }
        validate_artifact_receipt_binding(
            manifest,
            artifact,
            receipt,
            verified_at_ms,
        )?;
        if !source_sequences.insert((
            receipt.source_id.as_str(),
            receipt.control_domain.as_str(),
            receipt.sequence,
        )) {
            return Err(BundleVerificationError::DuplicateSourceSequence(
                receipt.source_id.clone(),
            ));
        }
        receipt_order.entry((
            receipt.source_id.as_str(),
            receipt.control_domain.as_str(),
        )).or_default().push((receipt.issued_at_ms, receipt.sequence));

        match receipt.decision {
            AuthenticationDecision::Authenticated => {
                validate_available_artifact(artifact, receipt)?;
                artifact_authenticated = artifact_authenticated.checked_add(1)
                    .ok_or(BundleVerificationError::ArithmeticOverflow)?;
            }
            AuthenticationDecision::Rejected => {
                return Err(BundleVerificationError::ArtifactReceiptRejected(
                    artifact.artifact_id.clone(),
                ));
            }
            AuthenticationDecision::Indeterminate => {
                if artifact.availability != ArtifactAvailability::Unavailable {
                    artifact_indeterminate = artifact_indeterminate.checked_add(1)
                        .ok_or(BundleVerificationError::ArithmeticOverflow)?;
                }
            }
        }
        if artifact.availability == ArtifactAvailability::Unavailable
            && receipt.decision != AuthenticationDecision::Indeterminate
        {
            return Err(BundleVerificationError::UnavailableArtifactAuthenticated(
                artifact.artifact_id.clone(),
            ));
        }
    }

    for artifact in &manifest.artifacts {
        if !receipt_by_id.contains_key(artifact.artifact_id.as_str()) {
            return Err(BundleVerificationError::ArtifactTrustReceiptMissing(
                artifact.artifact_id.clone(),
            ));
        }
        if artifact.required_for_verification
            && receipt_by_id[artifact.artifact_id.as_str()].decision
                == AuthenticationDecision::Indeterminate
        {
            artifact_indeterminate = artifact_indeterminate.checked_add(1)
                .ok_or(BundleVerificationError::ArithmeticOverflow)?;
        }
    }

    for order in receipt_order.values_mut() {
        order.sort_unstable_by_key(|(issued_at, sequence)| (*issued_at, *sequence));
        if order.windows(2).any(|pair| pair[0].1 >= pair[1].1) {
            return Err(BundleVerificationError::SourceSequenceOrderInvalid(
                "source".into(),
            ));
        }
    }

    let claim_statuses = validate_claim_graph(manifest, &artifacts_by_digest, &receipt_by_id)?;
    let mut claim_results = Vec::with_capacity(manifest.claims.len());
    let mut passing_claim_count = 0usize;
    let mut failing_claim_count = 0usize;
    let mut indeterminate_claim_count = 0usize;

    for claim in &manifest.claims {
        let mut verified_evidence_count = 0usize;
        let mut indeterminate_evidence_count = 0usize;
        for digest in &claim.evidence_digests {
            artifact_referenced.insert(digest);
            let artifact = artifacts_by_digest[digest];
            match receipt_by_id[artifact.artifact_id.as_str()].decision {
                AuthenticationDecision::Authenticated => verified_evidence_count += 1,
                AuthenticationDecision::Indeterminate => indeterminate_evidence_count += 1,
                AuthenticationDecision::Rejected => {
                    return Err(BundleVerificationError::ArtifactReceiptRejected(
                        artifact.artifact_id.clone(),
                    ));
                }
            }
        }
        match claim.status {
            ClaimStatus::Pass => passing_claim_count += 1,
            ClaimStatus::Fail => failing_claim_count += 1,
            ClaimStatus::Indeterminate => indeterminate_claim_count += 1,
        }
        claim_results.push(ClaimVerificationResult {
            claim_id: claim.claim_id.clone(),
            claim_type: claim.claim_type,
            status: claim.status,
            verified_evidence_count,
            indeterminate_evidence_count,
            dependencies: claim.dependencies.clone(),
        });
    }

    for artifact_id in required_artifact_ids {
        let artifact = artifacts_by_id[artifact_id];
        if !artifact_referenced.contains(&artifact.content_digest) {
            return Err(BundleVerificationError::RequiredArtifactUnreferenced(
                artifact.artifact_id.clone(),
            ));
        }
    }

    Ok(BundleVerificationReceipt {
        schema: "continuous-opsbench-bundle-verification-receipt-v0.1".into(),
        bundle_id: manifest.bundle_id.clone(),
        run_id: manifest.run_id.clone(),
        lane: manifest.lane,
        visibility: manifest.visibility,
        bundle_digest: manifest.bundle_digest.clone(),
        manifest_commitment: manifest.manifest_commitment.clone(),
        verifier_id: manifest.trust_policy.verifier_id.clone(),
        verifier_digest: manifest.trust_policy.verifier_build_digest.clone(),
        trust_policy_digest: manifest.trust_policy.policy_digest.clone(),
        verified_at_ms,
        artifact_count: manifest.artifacts.len(),
        authenticated_artifact_count: artifact_authenticated,
        indeterminate_artifact_count: artifact_indeterminate,
        claim_count: manifest.claims.len(),
        passing_claim_count,
        failing_claim_count,
        indeterminate_claim_count,
        integrity: if artifact_indeterminate == 0 {
            BundleIntegrity::Verified
        } else {
            BundleIntegrity::Indeterminate
        },
        claims: claim_results,
    })
}

fn validate_manifest(manifest: &EvidenceBundleManifest) -> Result<(), BundleVerificationError> {
    if manifest.schema != BUNDLE_SCHEMA || manifest.version != BUNDLE_VERSION {
        return Err(BundleVerificationError::UnsupportedSchema);
    }
    if !valid_id(&manifest.bundle_id)
        || !valid_id(&manifest.run_id)
        || digest_is_zero(&manifest.manifest_commitment)
        || digest_is_zero(&manifest.bundle_digest)
        || digest_is_zero(&manifest.verifier_digest)
    {
        return Err(BundleVerificationError::InvalidManifestIdentity);
    }
    let policy = &manifest.trust_policy;
    if !valid_id(&policy.policy_id)
        || !valid_id(&policy.verifier_id)
        || !valid_id(&policy.minimum_verifier_version)
        || digest_is_zero(&policy.policy_digest)
        || digest_is_zero(&policy.verifier_build_digest)
        || digest_is_zero(&policy.trust_roots_digest)
        || digest_is_zero(&policy.identity_constraints_digest)
        || digest_is_zero(&policy.revocation_policy_digest)
        || policy.verifier_build_digest != manifest.verifier_digest
        || policy.maximum_attestation_age_ms == 0
        || policy.trusted_publishers.is_empty()
    {
        return Err(BundleVerificationError::InvalidTrustPolicy);
    }
    let mut publishers = BTreeSet::new();
    for publisher in &policy.trusted_publishers {
        if !valid_id(&publisher.publisher_id)
            || !valid_id(&publisher.source_id)
            || !valid_id(&publisher.control_domain)
        {
            return Err(BundleVerificationError::InvalidTrustPolicy);
        }
        if !publishers.insert((
            publisher.publisher_id.as_str(),
            publisher.source_id.as_str(),
            publisher.control_domain.as_str(),
            publisher.role,
        )) {
            return Err(BundleVerificationError::DuplicateTrustedPublisher);
        }
    }
    if manifest.planned_trial_ids.is_empty() || manifest.claims.is_empty() {
        return Err(BundleVerificationError::EmptyClaimGraph);
    }
    let mut trials = BTreeSet::new();
    for trial in &manifest.planned_trial_ids {
        if !valid_id(trial) || !trials.insert(trial.as_str()) {
            return Err(BundleVerificationError::DuplicateTrialId(trial.clone()));
        }
    }
    Ok(())
}

fn validate_artifact_descriptor(
    manifest: &EvidenceBundleManifest,
    artifact: &ArtifactDescriptor,
) -> Result<(), BundleVerificationError> {
    let id = artifact.artifact_id.clone();
    if !valid_id(&artifact.artifact_id)
        || !valid_id(&artifact.media_type)
        || !valid_id(&artifact.publisher_id)
        || !valid_id(&artifact.source_id)
        || !valid_id(&artifact.control_domain)
        || artifact.sequence == 0
        || artifact.issued_at_ms > manifest.created_at_ms
        || digest_is_zero(&artifact.content_digest)
    {
        return Err(BundleVerificationError::InvalidArtifact(id));
    }
    match artifact.availability {
        ArtifactAvailability::Embedded => {
            if artifact.byte_length.unwrap_or(0) == 0 {
                return Err(BundleVerificationError::InvalidArtifact(id));
            }
        }
        ArtifactAvailability::CommitmentOnly | ArtifactAvailability::Unavailable => {
            if artifact.byte_length.is_some() {
                return Err(BundleVerificationError::InvalidArtifact(id));
            }
        }
    }
    if manifest.visibility == BundleVisibility::Public {
        match artifact.sensitivity {
            ArtifactSensitivity::Restricted => {
                if artifact.availability == ArtifactAvailability::Embedded {
                    return Err(BundleVerificationError::RestrictedArtifactEmbedded(id));
                }
            }
            ArtifactSensitivity::ActiveSealedScenario | ArtifactSensitivity::PrivateOracle => {
                if artifact.availability == ArtifactAvailability::Embedded {
                    return Err(BundleVerificationError::ActiveSealedPayloadEmbedded(id));
                }
            }
            ArtifactSensitivity::RetiredReveal => {
                if manifest.lane != EvaluationLane::RetiredReproducibility {
                    return Err(BundleVerificationError::RetiredRevealInWrongLane(id));
                }
                if artifact.availability == ArtifactAvailability::Embedded
                    && manifest.lane != EvaluationLane::RetiredReproducibility
                {
                    return Err(BundleVerificationError::PublicBundleContainsRestrictedRetiredReveal(id));
                }
            }
            ArtifactSensitivity::Public => {}
        }
    }
    if artifact.sensitivity == ArtifactSensitivity::RetiredReveal
        && manifest.lane != EvaluationLane::RetiredReproducibility
    {
        return Err(BundleVerificationError::RetiredRevealInWrongLane(id));
    }
    Ok(())
}

fn validate_artifact_receipt_binding(
    manifest: &EvidenceBundleManifest,
    artifact: &ArtifactDescriptor,
    receipt: &ArtifactAuthenticationReceipt,
    verified_at_ms: u64,
) -> Result<(), BundleVerificationError> {
    if receipt.subject_digest != artifact.content_digest
        || receipt.manifest_commitment != manifest.manifest_commitment
        || receipt.publisher_id != artifact.publisher_id
        || receipt.source_id != artifact.source_id
        || receipt.control_domain != artifact.control_domain
        || receipt.role != artifact.role
        || receipt.sequence != artifact.sequence
        || receipt.issued_at_ms != artifact.issued_at_ms
        || receipt.trust_policy_digest != manifest.trust_policy.policy_digest
        || receipt.verifier_id != manifest.trust_policy.verifier_id
        || receipt.verifier_build_digest != manifest.trust_policy.verifier_build_digest
    {
        return Err(BundleVerificationError::ArtifactReceiptBindingMismatch(
            artifact.artifact_id.clone(),
        ));
    }
    if receipt.issued_at_ms > verified_at_ms || receipt.verified_at_ms > verified_at_ms {
        return Err(BundleVerificationError::ReceiptFromFuture(artifact.artifact_id.clone()));
    }
    if verified_at_ms.saturating_sub(receipt.issued_at_ms)
        > manifest.trust_policy.maximum_attestation_age_ms
    {
        return Err(BundleVerificationError::StaleArtifactReceipt(
            artifact.artifact_id.clone(),
        ));
    }
    let trusted = manifest.trust_policy.trusted_publishers.iter().any(|publisher| {
        publisher.publisher_id == receipt.publisher_id
            && publisher.source_id == receipt.source_id
            && publisher.control_domain == receipt.control_domain
            && publisher.role == receipt.role
    });
    if !trusted {
        return Err(BundleVerificationError::UntrustedPublisher(
            receipt.publisher_id.clone(),
        ));
    }
    Ok(())
}

fn validate_available_artifact(
    artifact: &ArtifactDescriptor,
    receipt: &ArtifactAuthenticationReceipt,
) -> Result<(), BundleVerificationError> {
    match artifact.availability {
        ArtifactAvailability::Embedded => {
            if receipt.observed_content_digest.as_ref() != Some(&artifact.content_digest)
                || receipt.observed_byte_length != artifact.byte_length
            {
                return Err(BundleVerificationError::EmbeddedArtifactBytesMismatch(
                    artifact.artifact_id.clone(),
                ));
            }
        }
        ArtifactAvailability::CommitmentOnly => {
            if receipt.observed_content_digest.is_some() || receipt.observed_byte_length.is_some() {
                return Err(BundleVerificationError::CommitmentArtifactHasBytes(
                    artifact.artifact_id.clone(),
                ));
            }
        }
        ArtifactAvailability::Unavailable => {
            return Err(BundleVerificationError::UnavailableArtifactAuthenticated(
                artifact.artifact_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_claim_graph(
    manifest: &EvidenceBundleManifest,
    artifacts: &BTreeMap<&str, &ArtifactDescriptor>,
    receipts: &BTreeMap<&str, &ArtifactAuthenticationReceipt>,
) -> Result<BTreeMap<&str, ClaimStatus>, BundleVerificationError> {
    let mut claims: BTreeMap<&str, &EvidenceClaim> = BTreeMap::new();
    for claim in &manifest.claims {
        if !valid_id(&claim.claim_id) || claims.insert(claim.claim_id.as_str(), claim).is_some() {
            return Err(BundleVerificationError::DuplicateClaimId(claim.claim_id.clone()));
        }
        if claim.subject.run_id != manifest.run_id
            || digest_is_zero(&claim.subject.subject_digest)
            || !valid_id(&claim.verifier_id)
            || !valid_id(&claim.claim_ceiling)
        {
            return Err(BundleVerificationError::ClaimSubjectInvalid(claim.claim_id.clone()));
        }
        if claim.verifier_id != manifest.trust_policy.verifier_id
            || claim.verifier_digest != manifest.trust_policy.verifier_build_digest
        {
            return Err(BundleVerificationError::ClaimVerifierUnpinned(
                claim.claim_id.clone(),
            ));
        }
        if claim.status == ClaimStatus::Pass && claim.evidence_digests.is_empty() {
            return Err(BundleVerificationError::ClaimEvidenceMissing(
                claim.claim_id.clone(),
            ));
        }
        if claim.subject.trial_id.is_none() && claim.subject.arm.is_some() {
            return Err(BundleVerificationError::ClaimSubjectInvalid(claim.claim_id.clone()));
        }
        if let Some(trial_id) = claim.subject.trial_id.as_deref() {
            if !manifest.planned_trial_ids.iter().any(|planned| planned == trial_id) {
                return Err(BundleVerificationError::ClaimSubjectInvalid(claim.claim_id.clone()));
            }
        }
    }

    let mut indegree: BTreeMap<&str, usize> = claims.keys().map(|id| (*id, 0)).collect();
    let mut outgoing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for claim in &manifest.claims {
        let mut dependency_set = BTreeSet::new();
        for dependency in &claim.dependencies {
            if !dependency_set.insert(dependency.as_str()) {
                return Err(BundleVerificationError::ClaimStatusLaundering(
                    claim.claim_id.clone(),
                ));
            }
            if !claims.contains_key(dependency.as_str()) {
                return Err(BundleVerificationError::UnknownClaimDependency(
                    claim.claim_id.clone(),
                    dependency.clone(),
                ));
            }
            outgoing.entry(dependency.as_str()).or_default().push(claim.claim_id.as_str());
            *indegree.get_mut(claim.claim_id.as_str()).unwrap() += 1;
        }
    }

    let mut queue: VecDeque<&str> = indegree.iter()
        .filter_map(|(id, degree)| if *degree == 0 { Some(*id) } else { None })
        .collect();
    let mut visited = 0usize;
    let mut statuses = BTreeMap::new();
    while let Some(id) = queue.pop_front() {
        let claim = claims[id];
        for digest in &claim.evidence_digests {
            let artifact = artifacts.values().find(|artifact| &artifact.content_digest == digest)
                .ok_or_else(|| BundleVerificationError::ClaimEvidenceUnknown(
                    claim.claim_id.clone(),
                    digest.clone(),
                ))?;
            let receipt = receipts.get(artifact.artifact_id.as_str())
                .ok_or_else(|| BundleVerificationError::ArtifactTrustReceiptMissing(
                    artifact.artifact_id.clone(),
                ))?;
            if claim.status == ClaimStatus::Pass
                && receipt.decision != AuthenticationDecision::Authenticated
            {
                return Err(BundleVerificationError::ClaimPassingOnUnverifiedEvidence(
                    claim.claim_id.clone(),
                ));
            }
        }
        if claim.status == ClaimStatus::Pass {
            if claim.dependencies.iter().any(|dependency| {
                statuses.get(dependency.as_str()) != Some(&ClaimStatus::Pass)
            }) {
                return Err(BundleVerificationError::ClaimStatusLaundering(
                    claim.claim_id.clone(),
                ));
            }
        }
        statuses.insert(id, claim.status);
        visited += 1;
        if let Some(next) = outgoing.get(id) {
            for dependent in next {
                let degree = indegree.get_mut(dependent).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(dependent);
                }
            }
        }
    }
    if visited != claims.len() {
        return Err(BundleVerificationError::ClaimDependencyCycle);
    }
    Ok(statuses)
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 256
        && !value.chars().any(char::is_control)
}

fn digest_is_zero(digest: &Digest) -> bool {
    digest.0.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(value: u8) -> Digest {
        Digest::from_byte(value)
    }

    fn fixture() -> (
        EvidenceBundleManifest,
        Digest,
        Digest,
        u64,
        Vec<ArtifactAuthenticationReceipt>,
    ) {
        let manifest_commitment = d(1);
        let bundle_digest = d(2);
        let verifier_digest = d(3);
        let policy_digest = d(4);
        let policy = TrustPolicyReference {
            policy_id: "trust-policy-v1".into(),
            policy_digest: policy_digest.clone(),
            verifier_id: "portable-verifier".into(),
            verifier_build_digest: verifier_digest.clone(),
            trust_roots_digest: d(5),
            identity_constraints_digest: d(6),
            revocation_policy_digest: d(7),
            policy_as_of_ms: 100,
            maximum_attestation_age_ms: 10_000,
            minimum_verifier_version: "0.1.0".into(),
            trusted_publishers: vec![
                TrustedPublisher {
                    publisher_id: "evaluator-publisher".into(),
                    source_id: "evaluator-source".into(),
                    control_domain: "evaluator-domain".into(),
                    role: EvidenceRole::Evaluator,
                },
                TrustedPublisher {
                    publisher_id: "observer-publisher".into(),
                    source_id: "observer-source".into(),
                    control_domain: "observer-domain".into(),
                    role: EvidenceRole::ServiceObserver,
                },
            ],
        };
        let artifacts = vec![
            ArtifactDescriptor {
                artifact_id: "evaluator-attestation".into(),
                content_digest: d(10),
                byte_length: Some(128),
                media_type: "application/vnd.dsse.envelope+json".into(),
                publisher_id: "evaluator-publisher".into(),
                source_id: "evaluator-source".into(),
                control_domain: "evaluator-domain".into(),
                role: EvidenceRole::Evaluator,
                sequence: 1,
                issued_at_ms: 200,
                availability: ArtifactAvailability::Embedded,
                sensitivity: ArtifactSensitivity::Public,
                required_for_verification: true,
            },
            ArtifactDescriptor {
                artifact_id: "service-observer-attestation".into(),
                content_digest: d(11),
                byte_length: None,
                media_type: "application/vnd.in-toto+json".into(),
                publisher_id: "observer-publisher".into(),
                source_id: "observer-source".into(),
                control_domain: "observer-domain".into(),
                role: EvidenceRole::ServiceObserver,
                sequence: 1,
                issued_at_ms: 201,
                availability: ArtifactAvailability::CommitmentOnly,
                sensitivity: ArtifactSensitivity::Public,
                required_for_verification: true,
            },
            ArtifactDescriptor {
                artifact_id: "active-oracle-commitment".into(),
                content_digest: d(12),
                byte_length: None,
                media_type: "application/vnd.luminous.commitment".into(),
                publisher_id: "evaluator-publisher".into(),
                source_id: "evaluator-source".into(),
                control_domain: "evaluator-domain".into(),
                role: EvidenceRole::Evaluator,
                sequence: 2,
                issued_at_ms: 202,
                availability: ArtifactAvailability::CommitmentOnly,
                sensitivity: ArtifactSensitivity::PrivateOracle,
                required_for_verification: true,
            },
        ];
        let subject = ClaimSubject {
            run_id: "run-001".into(),
            trial_id: None,
            arm: None,
            subject_digest: manifest_commitment.clone(),
        };
        let claims = vec![
            EvidenceClaim {
                claim_id: "run-integrity".into(),
                claim_type: ClaimType::Authorization,
                subject: subject.clone(),
                status: ClaimStatus::Pass,
                verifier_id: "portable-verifier".into(),
                verifier_digest: verifier_digest.clone(),
                evidence_digests: vec![d(10), d(11), d(12)],
                dependencies: vec![],
                assumptions: vec!["source adapter trust policy is current".into()],
                claim_ceiling: "evidence-envelope authenticity only".into(),
                status_reason: "all required attestations authenticate".into(),
            },
            EvidenceClaim {
                claim_id: "service-outcome".into(),
                claim_type: ClaimType::ServiceOutcome,
                subject,
                status: ClaimStatus::Pass,
                verifier_id: "portable-verifier".into(),
                verifier_digest: verifier_digest.clone(),
                evidence_digests: vec![d(11)],
                dependencies: vec!["run-integrity".into()],
                assumptions: vec!["observer policy independently approved".into()],
                claim_ceiling: "source-attested service observation".into(),
                status_reason: "service observer receipt matches the run subject".into(),
            },
        ];
        let manifest = EvidenceBundleManifest {
            schema: BUNDLE_SCHEMA.into(),
            version: BUNDLE_VERSION,
            bundle_id: "bundle-001".into(),
            run_id: "run-001".into(),
            lane: EvaluationLane::SealedCertification,
            visibility: BundleVisibility::Public,
            created_at_ms: 250,
            manifest_commitment: manifest_commitment.clone(),
            bundle_digest: bundle_digest.clone(),
            verifier_digest: verifier_digest.clone(),
            trust_policy: policy,
            planned_trial_ids: vec!["trial-1".into()],
            artifacts,
            claims,
        };
        let receipts = vec![
            ArtifactAuthenticationReceipt {
                artifact_id: "evaluator-attestation".into(),
                subject_digest: d(10),
                manifest_commitment: manifest_commitment.clone(),
                publisher_id: "evaluator-publisher".into(),
                source_id: "evaluator-source".into(),
                control_domain: "evaluator-domain".into(),
                role: EvidenceRole::Evaluator,
                sequence: 1,
                issued_at_ms: 200,
                verified_at_ms: 240,
                observed_content_digest: Some(d(10)),
                observed_byte_length: Some(128),
                trust_policy_digest: policy_digest.clone(),
                verifier_id: "portable-verifier".into(),
                verifier_build_digest: verifier_digest.clone(),
                decision: AuthenticationDecision::Authenticated,
            },
            ArtifactAuthenticationReceipt {
                artifact_id: "service-observer-attestation".into(),
                subject_digest: d(11),
                manifest_commitment: manifest_commitment.clone(),
                publisher_id: "observer-publisher".into(),
                source_id: "observer-source".into(),
                control_domain: "observer-domain".into(),
                role: EvidenceRole::ServiceObserver,
                sequence: 1,
                issued_at_ms: 201,
                verified_at_ms: 241,
                observed_content_digest: None,
                observed_byte_length: None,
                trust_policy_digest: policy_digest.clone(),
                verifier_id: "portable-verifier".into(),
                verifier_build_digest: verifier_digest.clone(),
                decision: AuthenticationDecision::Authenticated,
            },
            ArtifactAuthenticationReceipt {
                artifact_id: "active-oracle-commitment".into(),
                subject_digest: d(12),
                manifest_commitment: manifest_commitment.clone(),
                publisher_id: "evaluator-publisher".into(),
                source_id: "evaluator-source".into(),
                control_domain: "evaluator-domain".into(),
                role: EvidenceRole::Evaluator,
                sequence: 2,
                issued_at_ms: 202,
                verified_at_ms: 242,
                observed_content_digest: None,
                observed_byte_length: None,
                trust_policy_digest: policy_digest.clone(),
                verifier_id: "portable-verifier".into(),
                verifier_build_digest: verifier_digest.clone(),
                decision: AuthenticationDecision::Authenticated,
            },
        ];
        (manifest, bundle_digest, manifest_commitment, 300, receipts)
    }

    fn verify(
        fixture: &(
            EvidenceBundleManifest, Digest, Digest, u64,
            Vec<ArtifactAuthenticationReceipt>
        ),
        receipts: &[ArtifactAuthenticationReceipt],
    ) -> Result<BundleVerificationReceipt, BundleVerificationError> {
        verify_evidence_bundle(
            &fixture.0,
            &fixture.1,
            &fixture.2,
            &fixture.0.verifier_digest,
            fixture.3,
            receipts,
        )
    }

    #[test]
    fn public_sealed_bundle_authenticates_commitments_without_payloads() {
        let fixture = fixture();
        let result = verify(&fixture, &fixture.4).unwrap();
        assert_eq!(result.integrity, BundleIntegrity::Verified);
        assert_eq!(result.artifact_count, 3);
        assert_eq!(result.authenticated_artifact_count, 3);
        assert_eq!(result.claim_count, 2);
        assert_eq!(result.passing_claim_count, 2);
    }

    #[test]
    fn public_sealed_bundle_rejects_embedded_active_scenario_oracle_bytes() {
        let mut fixture = fixture();
        fixture.0.artifacts[2].availability = ArtifactAvailability::Embedded;
        fixture.0.artifacts[2].byte_length = Some(64);
        fixture.4[2].observed_content_digest = Some(d(12));
        fixture.4[2].observed_byte_length = Some(64);
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::ActiveSealedPayloadEmbedded(
                "active-oracle-commitment".into()
            ))
        );
    }

    #[test]
    fn public_bundle_rejects_restricted_artifact_bytes_and_wrong_lane_reveals() {
        let mut fixture = fixture();
        fixture.0.artifacts[0].sensitivity = ArtifactSensitivity::Restricted;
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::RestrictedArtifactEmbedded(
                "evaluator-attestation".into()
            ))
        );
        let mut fixture = fixture();
        fixture.0.artifacts[0].sensitivity = ArtifactSensitivity::RetiredReveal;
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::RetiredRevealInWrongLane(
                "evaluator-attestation".into()
            ))
        );
    }

    #[test]
    fn unavailable_required_evidence_makes_integrity_indeterminate() {
        let mut fixture = fixture();
        fixture.0.artifacts[0].availability = ArtifactAvailability::Unavailable;
        fixture.0.artifacts[0].byte_length = None;
        fixture.4[0].decision = AuthenticationDecision::Indeterminate;
        fixture.4[0].observed_content_digest = None;
        fixture.4[0].observed_byte_length = None;
        let result = verify(&fixture, &fixture.4).unwrap();
        assert_eq!(result.integrity, BundleIntegrity::Indeterminate);
        assert_eq!(result.indeterminate_artifact_count, 1);
    }

    #[test]
    fn embedded_artifact_requires_exact_bytes_digest_and_length() {
        let mut fixture = fixture();
        fixture.4[0].observed_content_digest = Some(d(99));
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::EmbeddedArtifactBytesMismatch(
                "evaluator-attestation".into()
            ))
        );
        let mut fixture = fixture();
        fixture.4[0].observed_byte_length = Some(127);
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::EmbeddedArtifactBytesMismatch(
                "evaluator-attestation".into()
            ))
        );
    }

    #[test]
    fn trust_policy_pins_publisher_role_source_domain_and_verifier_build() {
        let mut fixture = fixture();
        fixture.4[0].control_domain = "agent-domain".into();
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::ArtifactReceiptBindingMismatch(
                "evaluator-attestation".into()
            ))
        );
        let mut fixture = fixture();
        fixture.0.trust_policy.trusted_publishers.clear();
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::InvalidTrustPolicy)
        );
        let mut fixture = fixture();
        fixture.0.verifier_digest = d(99);
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::InvalidTrustPolicy)
        );
    }

    #[test]
    fn stale_future_replayed_and_rejected_receipts_fail_closed() {
        let mut fixture = fixture();
        fixture.4[0].issued_at_ms = 1;
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::StaleArtifactReceipt(
                "evaluator-attestation".into()
            ))
        );
        let mut fixture = fixture();
        fixture.4[0].issued_at_ms = 400;
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::ReceiptFromFuture(
                "evaluator-attestation".into()
            ))
        );
        let mut fixture = fixture();
        fixture.4[1].sequence = 1;
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::DuplicateSourceSequence(
                "observer-source".into()
            ))
        );
        let mut fixture = fixture();
        fixture.4[0].decision = AuthenticationDecision::Rejected;
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::ArtifactReceiptRejected(
                "evaluator-attestation".into()
            ))
        );
    }

    #[test]
    fn claim_graph_rejects_missing_dependency_cycles_and_status_laundering() {
        let mut fixture = fixture();
        fixture.0.claims[1].dependencies = vec!["missing-claim".into()];
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::UnknownClaimDependency(
                "service-outcome".into(),
                "missing-claim".into()
            ))
        );
        let mut fixture = fixture();
        fixture.0.claims[0].dependencies = vec!["service-outcome".into()];
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::ClaimDependencyCycle)
        );
        let mut fixture = fixture();
        fixture.0.claims[0].status = ClaimStatus::Fail;
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::ClaimStatusLaundering(
                "service-outcome".into()
            ))
        );
    }

    #[test]
    fn passing_claim_cannot_use_missing_or_indeterminate_evidence() {
        let mut fixture = fixture();
        fixture.0.claims[0].evidence_digests.push(d(99));
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::ClaimEvidenceUnknown(
                "run-integrity".into(), d(99)
            ))
        );
        let mut fixture = fixture();
        fixture.4[0].decision = AuthenticationDecision::Indeterminate;
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::ClaimPassingOnUnverifiedEvidence(
                "run-integrity".into()
            ))
        );
    }

    #[test]
    fn required_evidence_must_be_used_by_at_least_one_claim() {
        let mut fixture = fixture();
        fixture.0.claims[0].evidence_digests = vec![d(10)];
        fixture.0.claims[1].evidence_digests = vec![d(11)];
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::RequiredArtifactUnreferenced(
                "active-oracle-commitment".into()
            ))
        );
    }

    #[test]
    fn bundle_and_manifest_commitments_are_pinned() {
        let mut fixture = fixture();
        fixture.0.bundle_digest = d(99);
        assert_eq!(
            verify(&fixture, &fixture.4),
            Err(BundleVerificationError::BundleDigestMismatch)
        );
        let fixture = fixture();
        let result = verify_evidence_bundle(
            &fixture.0, &fixture.1, &d(99), &fixture.0.verifier_digest, fixture.3, &fixture.4
        );
        assert_eq!(result, Err(BundleVerificationError::ManifestCommitmentMismatch));
    }

    #[test]
    fn eighty_adversarial_bundle_mutations_fail_closed() {
        for vector in 0..80 {
            let mut fixture = fixture();
            match vector {
                0 => fixture.0.schema = "unknown".into(),
                1 => fixture.0.version = 9,
                2 => fixture.0.bundle_id.clear(),
                3 => fixture.0.run_id.clear(),
                4 => fixture.0.manifest_commitment = d(0),
                5 => fixture.0.bundle_digest = d(0),
                6 => fixture.0.verifier_digest = d(0),
                7 => fixture.0.trust_policy.policy_digest = d(0),
                8 => fixture.0.trust_policy.verifier_build_digest = d(0),
                9 => fixture.0.trust_policy.trust_roots_digest = d(0),
                10 => fixture.0.trust_policy.identity_constraints_digest = d(0),
                11 => fixture.0.trust_policy.revocation_policy_digest = d(0),
                12 => fixture.0.trust_policy.maximum_attestation_age_ms = 0,
                13 => fixture.0.trust_policy.trusted_publishers.clear(),
                14 => fixture.0.trust_policy.trusted_publishers[0].source_id = "wrong".into(),
                15 => fixture.0.planned_trial_ids.clear(),
                16 => fixture.0.planned_trial_ids.push("trial-1".into()),
                17 => fixture.0.artifacts[0].artifact_id = "bad\nid".into(),
                18 => fixture.0.artifacts[0].content_digest = d(0),
                19 => fixture.0.artifacts[0].byte_length = Some(0),
                20 => fixture.0.artifacts[0].publisher_id = "untrusted".into(),
                21 => fixture.0.artifacts[0].source_id.clear(),
                22 => fixture.0.artifacts[0].control_domain.clear(),
                23 => fixture.0.artifacts[0].sequence = 0,
                24 => fixture.0.artifacts[0].issued_at_ms = fixture.0.created_at_ms + 1,
                25 => fixture.0.artifacts[0].byte_length = None,
                26 => fixture.0.artifacts[1].byte_length = Some(10),
                27 => fixture.0.artifacts[0].availability = ArtifactAvailability::CommitmentOnly,
                28 => fixture.0.artifacts[0].sensitivity = ArtifactSensitivity::PrivateOracle,
                29 => fixture.0.artifacts[0].sensitivity = ArtifactSensitivity::ActiveSealedScenario,
                30 => fixture.0.artifacts[0].sensitivity = ArtifactSensitivity::Restricted,
                31 => fixture.0.artifacts[0].sensitivity = ArtifactSensitivity::RetiredReveal,
                32 => fixture.0.artifacts[1].availability = ArtifactAvailability::Unavailable,
                33 => fixture.0.artifacts[1].sensitivity = ArtifactSensitivity::RetiredReveal,
                34 => fixture.0.artifacts[0].artifact_id = fixture.0.artifacts[1].artifact_id.clone(),
                35 => fixture.0.artifacts[0].content_digest = fixture.0.artifacts[1].content_digest.clone(),
                36 => fixture.0.artifacts[0].media_type.clear(),
                37 => fixture.0.artifacts[0].role = EvidenceRole::Other,
                38 => fixture.0.artifacts[0].required_for_verification = false,
                39 => fixture.0.artifacts[0].byte_length = Some(256),
                40 => fixture.4[0].subject_digest = d(99),
                41 => fixture.4[0].manifest_commitment = d(99),
                42 => fixture.4[0].publisher_id = "wrong".into(),
                43 => fixture.4[0].source_id = "wrong".into(),
                44 => fixture.4[0].control_domain = "wrong".into(),
                45 => fixture.4[0].role = EvidenceRole::Other,
                46 => fixture.4[0].sequence = 0,
                47 => fixture.4[0].issued_at_ms = 1,
                48 => fixture.4[0].verified_at_ms = fixture.3 + 1,
                49 => fixture.4[0].trust_policy_digest = d(99),
                50 => fixture.4[0].verifier_id = "wrong".into(),
                51 => fixture.4[0].verifier_build_digest = d(99),
                52 => fixture.4[0].observed_content_digest = Some(d(99)),
                53 => fixture.4[0].observed_byte_length = Some(127),
                54 => fixture.4[0].decision = AuthenticationDecision::Rejected,
                55 => fixture.4[0].decision = AuthenticationDecision::Indeterminate,
                56 => fixture.4[1].sequence = fixture.4[0].sequence,
                57 => fixture.4.pop(),
                58 => fixture.4.push(fixture.4[0].clone()),
                59 => fixture.0.claims[0].claim_id.clear(),
                60 => fixture.0.claims[0].status = ClaimStatus::Pass,
                61 => fixture.0.claims[0].verifier_id = "wrong".into(),
                62 => fixture.0.claims[0].verifier_digest = d(99),
                63 => fixture.0.claims[0].evidence_digests.clear(),
                64 => fixture.0.claims[0].evidence_digests.push(d(99)),
                65 => fixture.0.claims[0].subject.subject_digest = d(0),
                66 => fixture.0.claims[0].subject.run_id = "other".into(),
                67 => fixture.0.claims[0].dependencies = vec!["missing".into()],
                68 => fixture.0.claims[1].dependencies = vec!["service-outcome".into()],
                69 => fixture.0.claims[0].dependencies = vec!["service-outcome".into()],
                70 => fixture.0.claims[0].claim_ceiling.clear(),
                71 => fixture.0.claims[0].status_reason.clear(),
                72 => fixture.0.visibility = BundleVisibility::Public,
                73 => fixture.0.lane = EvaluationLane::OpenDevelopment,
                74 => fixture.0.created_at_ms = 0,
                75 => fixture.0.trust_policy.policy_as_of_ms = fixture.3 + 1,
                76 => fixture.0.trust_policy.minimum_verifier_version.clear(),
                77 => fixture.0.claims[0].claim_type = ClaimType::Safety,
                78 => fixture.0.claims[0].status = ClaimStatus::Indeterminate,
                _ => fixture.0.claims[1].status = ClaimStatus::Fail,
            }
            let result = verify(&fixture, &fixture.4);
            assert!(
                result.is_err() || result.as_ref().unwrap().integrity != BundleIntegrity::Verified
                    || result.as_ref().unwrap().passing_claim_count < 2,
                "bundle adversarial vector {vector} was accepted as fully passing"
            );
        }
    }
}
