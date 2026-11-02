//! Portable offline evidence bundles and claim-to-evidence graph validation.
//!
//! This is a structural verifier. The existing typed evidence adapter must
//! authenticate signatures and recompute artifact digests before materialized
//! artifact records are supplied here. This module adds no key store or journal.

use crate::sealed::EvaluationLane;
use crate::Digest;
use std::collections::{BTreeMap, BTreeSet};

pub const EVIDENCE_BUNDLE_SCHEMA: &str = "continuous-opsbench-evidence-bundle-v0.1";
pub const EVIDENCE_BUNDLE_VERSION: u16 = 1;
pub const VERIFICATION_RECEIPT_SCHEMA: &str = "continuous-opsbench-verification-receipt-v0.1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BundleDisclosure { Public, Restricted }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactSensitivity {
    Public,
    OperationalEvidence,
    ActiveSealedPayload,
    PrivateOracle,
    RetiredReveal,
    RestrictedPrivate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactDelivery { Embedded, CommitmentOnly, Unavailable, ExternalReference }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimStatus { Pass, Fail, Indeterminate }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClaimKind {
    Authorization,
    ParticipantEffect,
    ServiceOutcome,
    RequestImpact,
    Safety,
    Calibration,
    Privacy,
    RuntimeEnforcement,
    SandboxTier,
    ResourceEnergy,
    StatisticalComparison,
    SealedCommitment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunDomainRole {
    Candidate,
    Baseline,
    Evaluator,
    FreezeAuthority,
    EvidenceSource,
    RuntimeObserver,
    RuntimeChallengeAuthority,
    Verifier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationMode { Offline, Networked }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunDomainBinding {
    pub role: RunDomainRole,
    pub identity_id: String,
    pub control_domain: String,
    pub identity_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedArmSubject {
    pub trial_id: String,
    pub cluster_id: String,
    pub arm: crate::sealed::EvaluationArm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleArtifact {
    pub artifact_id: String,
    pub content_digest: Digest,
    pub byte_length: u64,
    pub media_type: String,
    /// Canonical archive member path. Present only for Embedded artifacts.
    pub archive_path: Option<String>,
    pub publisher_id: String,
    pub publisher_key_id: String,
    pub publisher_key_digest: Digest,
    pub control_domain: String,
    pub source_id: String,
    pub sequence: u64,
    pub issued_at_ms: u64,
    pub sensitivity: ArtifactSensitivity,
    pub delivery: ArtifactDelivery,
    pub required: bool,
    pub retired_at_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterializedArtifact {
    pub artifact_id: String,
    pub archive_path: String,
    /// Recomputed over the exact bytes by the external offline artifact verifier.
    pub computed_digest: Digest,
    pub computed_byte_length: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimSubject {
    pub run_id: String,
    pub manifest_commitment: Digest,
    pub trial_id: Option<String>,
    pub arm: Option<crate::sealed::EvaluationArm>,
    pub scope_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceClaim {
    pub claim_id: String,
    pub kind: ClaimKind,
    pub status: ClaimStatus,
    pub subject: ClaimSubject,
    pub evidence_artifact_ids: Vec<String>,
    pub depends_on_claim_ids: Vec<String>,
    pub assumptions: Vec<String>,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceBundle {
    pub schema: String,
    pub version: u16,
    pub bundle_id: String,
    pub run_id: String,
    pub lane: EvaluationLane,
    pub disclosure: BundleDisclosure,
    pub verification_mode: VerificationMode,
    pub created_at_ms: u64,
    pub manifest_commitment: Digest,
    pub candidate_identity_digest: Digest,
    pub baseline_identity_digest: Digest,
    pub evaluator_digest: Digest,
    pub verifier_id: String,
    pub verifier_digest: Digest,
    pub trust_policy_digest: Digest,
    pub run_domains: Vec<RunDomainBinding>,
    pub planned_arm_subjects: Vec<PlannedArmSubject>,
    pub artifacts: Vec<BundleArtifact>,
    pub claims: Vec<EvidenceClaim>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustedPublisher {
    pub publisher_id: String,
    pub key_id: String,
    pub key_digest: Digest,
    pub control_domain: String,
    pub source_id: String,
    pub valid_from_ms: u64,
    pub valid_until_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustPolicySnapshot {
    pub policy_id: String,
    pub policy_digest: Digest,
    pub trust_roots_digest: Digest,
    pub revocation_policy_digest: Digest,
    pub verifier_id: String,
    pub verifier_version: String,
    pub verifier_digest: Digest,
    pub verifier_control_domain: String,
    pub valid_from_ms: u64,
    pub valid_until_ms: u64,
    pub maximum_artifact_bytes: u64,
    pub trusted_publishers: Vec<TrustedPublisher>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BundleDisposition { Pass, Fail, Indeterminate }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleVerificationReceipt {
    pub schema: String,
    pub bundle_id: String,
    pub bundle_digest: Digest,
    pub run_id: String,
    pub manifest_commitment: Digest,
    pub verifier_id: String,
    pub verifier_digest: Digest,
    pub trust_policy_digest: Digest,
    pub trust_roots_digest: Digest,
    pub revocation_policy_digest: Digest,
    pub verified_at_ms: u64,
    pub artifact_count: usize,
    pub embedded_artifact_count: usize,
    pub commitment_only_artifact_count: usize,
    pub unavailable_required_artifacts: usize,
    pub claim_count: usize,
    pub pass_claims: usize,
    pub fail_claims: usize,
    pub indeterminate_claims: usize,
    pub disposition: BundleDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BundleError {
    UnsupportedSchema,
    InvalidBundleIdentity,
    NetworkedVerificationForbidden,
    InvalidTrustPolicy,
    BundleTrustPolicyMismatch,
    VerifierIdentityMismatch,
    VerifierControlDomainCollision,
    MissingRunDomain(RunDomainRole),
    DuplicateRunDomain(RunDomainRole),
    RunDomainBindingMismatch(RunDomainRole),
    RunControlDomainCollision,
    InvalidPlannedSubject,
    DuplicatePlannedSubject(String, crate::sealed::EvaluationArm),
    MissingPlannedArm(String, crate::sealed::EvaluationArm),
    DuplicateArtifactId(String),
    DuplicateArtifactDigest,
    DuplicateArchivePath(String),
    DuplicatePublisherSequence(String, u64),
    InvalidArtifact(String),
    ArtifactTooLarge(String),
    InvalidArchivePath(String),
    ExternalReferenceForbidden(String),
    ActiveSealedDataLeaked(String),
    RestrictedArtifactInPublicBundle(String),
    RetiredRevealInWrongLane(String),
    RetiredRevealTimeInvalid(String),
    MissingRetiredTimeForReveal(String),
    DuplicateMaterializedArtifact(String),
    DuplicateMaterializedPath(String),
    UnregisteredMaterializedArtifact(String),
    MissingMaterializedArtifact(String),
    MaterializedDigestMismatch(String),
    MaterializedLengthMismatch(String),
    MaterializedPathMismatch(String),
    TrustedPublisherMissing(String),
    PublisherControlDomainCollision(String),
    PublisherKeyMismatch(String),
    PublisherOutsideValidity(String),
    DuplicateClaimId(String),
    DuplicateClaimSubject(String),
    InvalidClaim(String),
    ClaimSubjectMismatch(String),
    ClaimSubjectNotPlanned(String),
    DuplicateClaimEvidence(String),
    MissingClaimEvidence(String, String),
    ClaimUsesUnavailableEvidence(String, String),
    RequiredArtifactUnreferenced(String),
    DuplicateClaimDependency(String, String),
    MissingClaimDependency(String, String),
    ClaimDependencyNotPass(String, String),
    ClaimGraphCycle(String),
}

pub fn validate_evidence_bundle(
    bundle: &EvidenceBundle,
    expected_bundle_digest: &Digest,
    trust: &TrustPolicySnapshot,
    materialized: &[MaterializedArtifact],
    verified_at_ms: u64,
) -> Result<BundleVerificationReceipt, BundleError> {
    validate_bundle_header(bundle, expected_bundle_digest, trust, verified_at_ms)?;
    let planned_subjects = validate_planned_subjects(bundle)?;
    let artifacts = validate_artifacts(bundle, trust, materialized)?;
    let claims = validate_claim_graph(bundle, &planned_subjects, &artifacts)?;

    let unavailable_required_artifacts = artifacts.values()
        .filter(|artifact| artifact.required && artifact.delivery == ArtifactDelivery::Unavailable)
        .count();
    let pass_claims = claims.values().filter(|claim| claim.status == ClaimStatus::Pass).count();
    let fail_claims = claims.values().filter(|claim| claim.status == ClaimStatus::Fail).count();
    let indeterminate_claims = claims.values()
        .filter(|claim| claim.status == ClaimStatus::Indeterminate)
        .count();

    let disposition = if fail_claims > 0 {
        BundleDisposition::Fail
    } else if indeterminate_claims > 0 || unavailable_required_artifacts > 0 {
        BundleDisposition::Indeterminate
    } else {
        BundleDisposition::Pass
    };

    Ok(BundleVerificationReceipt {
        schema: VERIFICATION_RECEIPT_SCHEMA.into(),
        bundle_id: bundle.bundle_id.clone(),
        bundle_digest: expected_bundle_digest.clone(),
        run_id: bundle.run_id.clone(),
        manifest_commitment: bundle.manifest_commitment.clone(),
        verifier_id: trust.verifier_id.clone(),
        verifier_digest: trust.verifier_digest.clone(),
        trust_policy_digest: trust.policy_digest.clone(),
        trust_roots_digest: trust.trust_roots_digest.clone(),
        revocation_policy_digest: trust.revocation_policy_digest.clone(),
        verified_at_ms,
        artifact_count: artifacts.len(),
        embedded_artifact_count: artifacts.values()
            .filter(|artifact| artifact.delivery == ArtifactDelivery::Embedded).count(),
        commitment_only_artifact_count: artifacts.values()
            .filter(|artifact| artifact.delivery == ArtifactDelivery::CommitmentOnly).count(),
        unavailable_required_artifacts,
        claim_count: claims.len(),
        pass_claims,
        fail_claims,
        indeterminate_claims,
        disposition,
    })
}

fn validate_bundle_header(
    bundle: &EvidenceBundle,
    expected_bundle_digest: &Digest,
    trust: &TrustPolicySnapshot,
    verified_at_ms: u64,
) -> Result<(), BundleError> {
    if bundle.schema != EVIDENCE_BUNDLE_SCHEMA || bundle.version != EVIDENCE_BUNDLE_VERSION {
        return Err(BundleError::UnsupportedSchema);
    }
    if !valid_id(&bundle.bundle_id)
        || !valid_id(&bundle.run_id)
        || !valid_id(&bundle.verifier_id)
        || is_zero(expected_bundle_digest)
        || is_zero(&bundle.manifest_commitment)
        || is_zero(&bundle.candidate_identity_digest)
        || is_zero(&bundle.baseline_identity_digest)
        || is_zero(&bundle.evaluator_digest)
        || is_zero(&bundle.verifier_digest)
        || is_zero(&bundle.trust_policy_digest)
        || bundle.candidate_identity_digest == bundle.baseline_identity_digest
    {
        return Err(BundleError::InvalidBundleIdentity);
    }
    if bundle.verification_mode != VerificationMode::Offline {
        return Err(BundleError::NetworkedVerificationForbidden);
    }
    if !valid_id(&trust.policy_id)
        || !valid_id(&trust.verifier_id)
        || !valid_id(&trust.verifier_version)
        || !valid_id(&trust.verifier_control_domain)
        || is_zero(&trust.policy_digest)
        || is_zero(&trust.trust_roots_digest)
        || is_zero(&trust.revocation_policy_digest)
        || is_zero(&trust.verifier_digest)
        || trust.valid_from_ms >= trust.valid_until_ms
        || trust.maximum_artifact_bytes == 0
        || trust.trusted_publishers.is_empty()
    {
        return Err(BundleError::InvalidTrustPolicy);
    }
    if bundle.trust_policy_digest != trust.policy_digest {
        return Err(BundleError::BundleTrustPolicyMismatch);
    }
    if bundle.verifier_id != trust.verifier_id || bundle.verifier_digest != trust.verifier_digest {
        return Err(BundleError::VerifierIdentityMismatch);
    }
    if bundle.created_at_ms < trust.valid_from_ms
        || bundle.created_at_ms > trust.valid_until_ms
        || verified_at_ms < trust.valid_from_ms
        || verified_at_ms > trust.valid_until_ms
        || verified_at_ms < bundle.created_at_ms
    {
        return Err(BundleError::InvalidTrustPolicy);
    }
    validate_run_domains(bundle, trust)?;
    Ok(())
}

fn validate_run_domains(
    bundle: &EvidenceBundle,
    trust: &TrustPolicySnapshot,
) -> Result<(), BundleError> {
    let mut bindings = BTreeMap::new();
    let mut domains = BTreeSet::new();
    for binding in &bundle.run_domains {
        if !valid_id(&binding.identity_id)
            || !valid_id(&binding.control_domain)
            || is_zero(&binding.identity_digest)
        {
            return Err(BundleError::RunDomainBindingMismatch(binding.role));
        }
        if bindings.insert(binding.role, binding).is_some() {
            return Err(BundleError::DuplicateRunDomain(binding.role));
        }
        if !domains.insert(binding.control_domain.as_str()) {
            return Err(BundleError::RunControlDomainCollision);
        }
    }
    let required = [
        RunDomainRole::Candidate,
        RunDomainRole::Baseline,
        RunDomainRole::Evaluator,
        RunDomainRole::FreezeAuthority,
        RunDomainRole::EvidenceSource,
        RunDomainRole::RuntimeObserver,
        RunDomainRole::RuntimeChallengeAuthority,
        RunDomainRole::Verifier,
    ];
    for role in required {
        let binding = bindings.get(&role).ok_or(BundleError::MissingRunDomain(role))?;
        let expected_digest = match role {
            RunDomainRole::Candidate => &bundle.candidate_identity_digest,
            RunDomainRole::Baseline => &bundle.baseline_identity_digest,
            RunDomainRole::Evaluator => &bundle.evaluator_digest,
            RunDomainRole::Verifier => &trust.verifier_digest,
            _ => &binding.identity_digest,
        };
        if &binding.identity_digest != expected_digest {
            return Err(BundleError::RunDomainBindingMismatch(role));
        }
    }
    if bindings[&RunDomainRole::Verifier].control_domain != trust.verifier_control_domain {
        return Err(BundleError::VerifierControlDomainCollision);
    }
    Ok(())
}

fn validate_planned_subjects(
    bundle: &EvidenceBundle,
) -> Result<BTreeSet<(String, crate::sealed::EvaluationArm)>, BundleError> {
    if bundle.planned_arm_subjects.is_empty() {
        return Err(BundleError::InvalidPlannedSubject);
    }
    let mut subjects = BTreeMap::new();
    let mut trials = BTreeMap::new();
    for subject in &bundle.planned_arm_subjects {
        if !valid_id(&subject.trial_id) || !valid_id(&subject.cluster_id) {
            return Err(BundleError::InvalidPlannedSubject);
        }
        let key = (subject.trial_id.clone(), subject.arm);
        if subjects.insert(key.clone(), subject.cluster_id.clone()).is_some() {
            return Err(BundleError::DuplicatePlannedSubject(subject.trial_id.clone(), subject.arm));
        }
        match trials.get(&subject.trial_id) {
            Some(cluster) if cluster != &subject.cluster_id => return Err(BundleError::InvalidPlannedSubject),
            _ => { trials.insert(subject.trial_id.clone(), subject.cluster_id.clone()); }
        }
    }
    for trial_id in trials.keys() {
        for arm in [crate::sealed::EvaluationArm::Candidate, crate::sealed::EvaluationArm::Baseline] {
            if !subjects.contains_key(&(trial_id.clone(), arm)) {
                return Err(BundleError::MissingPlannedArm(trial_id.clone(), arm));
            }
        }
    }
    Ok(subjects.keys().cloned().collect())
}

fn validate_artifacts(
    bundle: &EvidenceBundle,
    trust: &TrustPolicySnapshot,
    materialized: &[MaterializedArtifact],
) -> Result<BTreeMap<String, BundleArtifact>, BundleError> {
    let mut artifact_index = BTreeMap::new();
    let mut digest_index = BTreeSet::new();
    let mut path_index = BTreeSet::new();
    let mut source_sequences = BTreeSet::new();
    let mut materialized_index = BTreeMap::new();
    let mut materialized_paths = BTreeSet::new();

    for item in materialized {
        if materialized_index.insert(item.artifact_id.as_str(), item).is_some() {
            return Err(BundleError::DuplicateMaterializedArtifact(item.artifact_id.clone()));
        }
        if !materialized_paths.insert(item.archive_path.as_str()) {
            return Err(BundleError::DuplicateMaterializedPath(item.archive_path.clone()));
        }
    }

    let mut publisher_keys = BTreeSet::new();
    for publisher in &trust.trusted_publishers {
        if !valid_id(&publisher.publisher_id)
            || !valid_id(&publisher.key_id)
            || !valid_id(&publisher.control_domain)
            || !valid_id(&publisher.source_id)
            || is_zero(&publisher.key_digest)
            || publisher.valid_from_ms >= publisher.valid_until_ms
            || !publisher_keys.insert((publisher.publisher_id.as_str(), publisher.key_id.as_str(), publisher.source_id.as_str()))
        {
            return Err(BundleError::InvalidTrustPolicy);
        }
    }

    for artifact in &bundle.artifacts {
        if artifact_index.contains_key(&artifact.artifact_id) {
            return Err(BundleError::DuplicateArtifactId(artifact.artifact_id.clone()));
        }
        if !valid_id(&artifact.artifact_id)
            || !valid_id(&artifact.publisher_id)
            || !valid_id(&artifact.publisher_key_id)
            || !valid_id(&artifact.control_domain)
            || !valid_id(&artifact.source_id)
            || !valid_media_type(&artifact.media_type)
            || is_zero(&artifact.content_digest)
            || artifact.byte_length == 0
            || artifact.sequence == 0
            || artifact.issued_at_ms > bundle.created_at_ms
        {
            return Err(BundleError::InvalidArtifact(artifact.artifact_id.clone()));
        }
        if artifact.byte_length > trust.maximum_artifact_bytes {
            return Err(BundleError::ArtifactTooLarge(artifact.artifact_id.clone()));
        }
        if !digest_index.insert(artifact.content_digest.clone()) {
            return Err(BundleError::DuplicateArtifactDigest);
        }
        if !source_sequences.insert((artifact.source_id.as_str(), artifact.sequence)) {
            return Err(BundleError::DuplicatePublisherSequence(artifact.source_id.clone(), artifact.sequence));
        }
        if bundle.run_domains.iter().any(|domain| domain.control_domain == artifact.control_domain) {
            return Err(BundleError::PublisherControlDomainCollision(artifact.artifact_id.clone()));
        }
        let publisher = trust.trusted_publishers.iter().find(|publisher| {
            publisher.publisher_id == artifact.publisher_id
                && publisher.control_domain == artifact.control_domain
                && publisher.source_id == artifact.source_id
        }).ok_or_else(|| BundleError::TrustedPublisherMissing(artifact.artifact_id.clone()))?;
        if publisher.key_id != artifact.publisher_key_id
            || publisher.key_digest != artifact.publisher_key_digest
        {
            return Err(BundleError::PublisherKeyMismatch(artifact.artifact_id.clone()));
        }
        if artifact.issued_at_ms < publisher.valid_from_ms
            || artifact.issued_at_ms > publisher.valid_until_ms
            || bundle.created_at_ms > publisher.valid_until_ms
        {
            return Err(BundleError::PublisherOutsideValidity(artifact.artifact_id.clone()));
        }
        if artifact.delivery == ArtifactDelivery::ExternalReference {
            return Err(BundleError::ExternalReferenceForbidden(artifact.artifact_id.clone()));
        }
        match artifact.sensitivity {
            ArtifactSensitivity::ActiveSealedPayload | ArtifactSensitivity::PrivateOracle => {
                if bundle.disclosure == BundleDisclosure::Public
                    && artifact.delivery == ArtifactDelivery::Embedded
                {
                    return Err(BundleError::ActiveSealedDataLeaked(artifact.artifact_id.clone()));
                }
            }
            ArtifactSensitivity::RestrictedPrivate => {
                if bundle.disclosure == BundleDisclosure::Public
                    && artifact.delivery == ArtifactDelivery::Embedded
                {
                    return Err(BundleError::RestrictedArtifactInPublicBundle(artifact.artifact_id.clone()));
                }
            }
            ArtifactSensitivity::RetiredReveal => {
                if bundle.lane != EvaluationLane::RetiredReproducibility {
                    return Err(BundleError::RetiredRevealInWrongLane(artifact.artifact_id.clone()));
                }
                let retired_at = artifact.retired_at_ms
                    .ok_or_else(|| BundleError::MissingRetiredTimeForReveal(artifact.artifact_id.clone()))?;
                if retired_at > bundle.created_at_ms || artifact.issued_at_ms < retired_at {
                    return Err(BundleError::RetiredRevealTimeInvalid(artifact.artifact_id.clone()));
                }
            }
            ArtifactSensitivity::Public | ArtifactSensitivity::OperationalEvidence => {}
        }
        if artifact.delivery == ArtifactDelivery::CommitmentOnly
            && !matches!(
                artifact.sensitivity,
                ArtifactSensitivity::ActiveSealedPayload
                    | ArtifactSensitivity::PrivateOracle
                    | ArtifactSensitivity::RetiredReveal
                    | ArtifactSensitivity::RestrictedPrivate
            )
        {
            return Err(BundleError::InvalidArtifact(artifact.artifact_id.clone()));
        }
        match artifact.delivery {
            ArtifactDelivery::Embedded => {
                let archive_path = artifact.archive_path.as_ref()
                    .ok_or_else(|| BundleError::InvalidArtifact(artifact.artifact_id.clone()))?;
                if !valid_archive_path(archive_path) {
                    return Err(BundleError::InvalidArchivePath(artifact.artifact_id.clone()));
                }
                if !path_index.insert(archive_path.as_str()) {
                    return Err(BundleError::DuplicateArchivePath(archive_path.clone()));
                }
            }
            ArtifactDelivery::CommitmentOnly | ArtifactDelivery::Unavailable => {
                if artifact.archive_path.is_some() {
                    return Err(BundleError::InvalidArtifact(artifact.artifact_id.clone()));
                }
            }
            ArtifactDelivery::ExternalReference => unreachable!("external references were rejected"),
        }
        let should_materialize = artifact.delivery == ArtifactDelivery::Embedded;
        match (should_materialize, materialized_index.get(artifact.artifact_id.as_str())) {
            (true, None) => return Err(BundleError::MissingMaterializedArtifact(artifact.artifact_id.clone())),
            (true, Some(item)) => {
                if item.computed_digest != artifact.content_digest {
                    return Err(BundleError::MaterializedDigestMismatch(artifact.artifact_id.clone()));
                }
                if item.computed_byte_length != artifact.byte_length {
                    return Err(BundleError::MaterializedLengthMismatch(artifact.artifact_id.clone()));
                }
                if Some(&item.archive_path) != artifact.archive_path.as_ref() {
                    return Err(BundleError::MaterializedPathMismatch(artifact.artifact_id.clone()));
                }
            }
            (false, Some(_)) => return Err(BundleError::MaterializedDigestMismatch(artifact.artifact_id.clone())),
            (false, None) => {}
        }
        if artifact_index.insert(artifact.artifact_id.clone(), artifact.clone()).is_some() {
            return Err(BundleError::DuplicateArtifactId(artifact.artifact_id.clone()));
        }
    }
    for artifact_id in materialized_index.keys() {
        if !artifact_index.contains_key(*artifact_id) {
            return Err(BundleError::UnregisteredMaterializedArtifact((*artifact_id).to_owned()));
        }
    }
    Ok(artifact_index)
}

fn validate_claim_graph(
    bundle: &EvidenceBundle,
    planned_subjects: &BTreeSet<(String, crate::sealed::EvaluationArm)>,
    artifacts: &BTreeMap<String, BundleArtifact>,
) -> Result<BTreeMap<String, EvidenceClaim>, BundleError> {
    if bundle.claims.is_empty() {
        return Err(BundleError::InvalidClaim("empty claim graph".into()));
    }
    let mut claim_index = BTreeMap::new();
    let mut referenced_artifacts = BTreeSet::new();
    let mut claim_subjects = BTreeSet::new();

    for claim in &bundle.claims {
        if !valid_id(&claim.claim_id)
            || !valid_id(&claim.subject.run_id)
            || !valid_id(&claim.subject.scope_id)
            || !valid_id(&claim.claim_ceiling)
            || claim.evidence_artifact_ids.is_empty()
            || claim.assumptions.iter().any(|assumption| {
                assumption.trim().is_empty() || assumption.chars().any(char::is_control)
            })
        {
            return Err(BundleError::InvalidClaim(claim.claim_id.clone()));
        }
        if claim.subject.run_id != bundle.run_id
            || claim.subject.manifest_commitment != bundle.manifest_commitment
        {
            return Err(BundleError::ClaimSubjectMismatch(claim.claim_id.clone()));
        }
        match (&claim.subject.trial_id, &claim.subject.arm) {
            (Some(trial_id), Some(arm)) => {
                if !planned_subjects.contains(&(trial_id.clone(), *arm)) {
                    return Err(BundleError::ClaimSubjectNotPlanned(claim.claim_id.clone()));
                }
            }
            (None, None) => {}
            _ => return Err(BundleError::ClaimSubjectMismatch(claim.claim_id.clone())),
        }
        if !claim_subjects.insert((
            claim.kind,
            claim.subject.trial_id.clone(),
            claim.subject.arm,
            claim.subject.scope_id.clone(),
        )) {
            return Err(BundleError::DuplicateClaimSubject(claim.claim_id.clone()));
        }
        let mut local_evidence = BTreeSet::new();
        for artifact_id in &claim.evidence_artifact_ids {
            if !valid_id(artifact_id) || !local_evidence.insert(artifact_id.as_str()) {
                return Err(BundleError::DuplicateClaimEvidence(claim.claim_id.clone()));
            }
            let artifact = artifacts.get(artifact_id)
                .ok_or_else(|| BundleError::MissingClaimEvidence(claim.claim_id.clone(), artifact_id.clone()))?;
            if artifact.delivery == ArtifactDelivery::Unavailable && claim.status == ClaimStatus::Pass {
                return Err(BundleError::ClaimUsesUnavailableEvidence(claim.claim_id.clone(), artifact_id.clone()));
            }
            referenced_artifacts.insert(artifact_id.clone());
        }
        let mut local_dependencies = BTreeSet::new();
        for dependency in &claim.depends_on_claim_ids {
            if !valid_id(dependency) || !local_dependencies.insert(dependency.as_str()) {
                return Err(BundleError::DuplicateClaimDependency(claim.claim_id.clone(), dependency.clone()));
            }
        }
        if claim_index.insert(claim.claim_id.clone(), claim.clone()).is_some() {
            return Err(BundleError::DuplicateClaimId(claim.claim_id.clone()));
        }
    }
    for artifact in artifacts.values() {
        if artifact.required && !referenced_artifacts.contains(&artifact.artifact_id) {
            return Err(BundleError::RequiredArtifactUnreferenced(artifact.artifact_id.clone()));
        }
    }

    let mut marks: BTreeMap<String, u8> = BTreeMap::new();
    let mut ordered = Vec::with_capacity(claim_index.len());
    for claim_id in claim_index.keys() {
        visit_claim(claim_id, &claim_index, &mut marks, &mut ordered)?;
    }
    for claim in bundle.claims.iter().filter(|claim| claim.status == ClaimStatus::Pass) {
        for dependency in &claim.depends_on_claim_ids {
            let dependency_claim = claim_index.get(dependency)
                .ok_or_else(|| BundleError::MissingClaimDependency(claim.claim_id.clone(), dependency.clone()))?;
            if dependency_claim.status != ClaimStatus::Pass {
                return Err(BundleError::ClaimDependencyNotPass(claim.claim_id.clone(), dependency.clone()));
            }
        }
    }
    for claim in &bundle.claims {
        for dependency in &claim.depends_on_claim_ids {
            if !claim_index.contains_key(dependency) {
                return Err(BundleError::MissingClaimDependency(claim.claim_id.clone(), dependency.clone()));
            }
        }
    }
    let _topological_order = ordered;
    Ok(claim_index)
}

fn visit_claim(
    claim_id: &str,
    claims: &BTreeMap<String, EvidenceClaim>,
    marks: &mut BTreeMap<String, u8>,
    order: &mut Vec<String>,
) -> Result<(), BundleError> {
    match marks.get(claim_id).copied().unwrap_or(0) {
        1 => return Err(BundleError::ClaimGraphCycle(claim_id.to_owned())),
        2 => return Ok(()),
        _ => {}
    }
    marks.insert(claim_id.to_owned(), 1);
    let claim = claims.get(claim_id)
        .ok_or_else(|| BundleError::InvalidClaim(claim_id.to_owned()))?;
    for dependency in &claim.depends_on_claim_ids {
        if !claims.contains_key(dependency) {
            return Err(BundleError::MissingClaimDependency(claim_id.to_owned(), dependency.clone()));
        }
        visit_claim(dependency, claims, marks, order)?;
    }
    marks.insert(claim_id.to_owned(), 2);
    order.push(claim_id.to_owned());
    Ok(())
}

fn valid_archive_path(path: &str) -> bool {
    if path.is_empty()
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.contains('\\')
        || path.contains(':')
        || path.chars().any(char::is_control)
    {
        return false;
    }
    let components: Vec<&str> = path.split('/').collect();
    !components.is_empty()
        && components.iter().all(|component| {
            !component.is_empty() && *component != "." && *component != ".."
        })
}

fn valid_media_type(value: &str) -> bool {
    let mut parts = value.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(major), Some(minor), None) => {
            !major.is_empty()
                && !minor.is_empty()
                && major.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
                && minor.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
        }
        _ => false,
    }
}

fn is_zero(digest: &Digest) -> bool {
    digest.0.iter().all(|byte| *byte == 0)
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 256
        && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sealed::EvaluationArm;

    fn d(byte: u8) -> Digest { Digest::from_byte(byte) }

    fn fixture() -> (EvidenceBundle, TrustPolicySnapshot, Vec<MaterializedArtifact>, Digest) {
        let trust = TrustPolicySnapshot {
            policy_id: "trust-v1".into(),
            policy_digest: d(1),
            trust_roots_digest: d(2),
            revocation_policy_digest: d(3),
            verifier_id: "offline-verifier".into(),
            verifier_version: "0.1.0".into(),
            verifier_digest: d(4),
            verifier_control_domain: "verifier-domain".into(),
            valid_from_ms: 10,
            valid_until_ms: 1_000,
            maximum_artifact_bytes: 1_000_000,
            trusted_publishers: vec![
                publisher("participant-ledger", "ledger-key", d(20), "ledger-domain", "effect-source"),
                publisher("service-observer", "observer-key", d(21), "observer-domain", "service-source"),
                publisher("oracle-publisher", "oracle-key", d(22), "oracle-domain", "oracle-source"),
            ],
        };
        let bundle = EvidenceBundle {
            schema: EVIDENCE_BUNDLE_SCHEMA.into(),
            version: EVIDENCE_BUNDLE_VERSION,
            bundle_id: "bundle-001".into(),
            run_id: "run-001".into(),
            lane: EvaluationLane::SealedCertification,
            disclosure: BundleDisclosure::Public,
            verification_mode: VerificationMode::Offline,
            created_at_ms: 100,
            manifest_commitment: d(5),
            candidate_identity_digest: d(6),
            baseline_identity_digest: d(7),
            evaluator_digest: d(8),
            verifier_id: trust.verifier_id.clone(),
            verifier_digest: trust.verifier_digest.clone(),
            trust_policy_digest: trust.policy_digest.clone(),
            run_domains: vec![
                domain(RunDomainRole::Candidate, "candidate", "candidate-domain", d(6)),
                domain(RunDomainRole::Baseline, "baseline", "baseline-domain", d(7)),
                domain(RunDomainRole::Evaluator, "evaluator", "evaluator-domain", d(8)),
                domain(RunDomainRole::FreezeAuthority, "freeze", "freeze-domain", d(9)),
                domain(RunDomainRole::EvidenceSource, "evidence", "evidence-domain", d(10)),
                domain(RunDomainRole::RuntimeObserver, "runtime-observer", "runtime-domain", d(11)),
                domain(RunDomainRole::RuntimeChallengeAuthority, "challenge", "challenge-domain", d(12)),
                domain(RunDomainRole::Verifier, "verifier", "verifier-domain", d(4)),
            ],
            planned_arm_subjects: vec![
                PlannedArmSubject { trial_id: "trial-a".into(), cluster_id: "cluster-a".into(), arm: EvaluationArm::Candidate },
                PlannedArmSubject { trial_id: "trial-a".into(), cluster_id: "cluster-a".into(), arm: EvaluationArm::Baseline },
            ],
            artifacts: vec![
                artifact("effect-receipt", d(30), 128, "application/vnd.in-toto+json", Some("evidence/effect.json"), "participant-ledger", "ledger-key", d(20), "ledger-domain", "effect-source", 1, ArtifactSensitivity::OperationalEvidence, ArtifactDelivery::Embedded, true),
                artifact("service-observation", d(31), 256, "application/json", Some("evidence/service.json"), "service-observer", "observer-key", d(21), "observer-domain", "service-source", 2, ArtifactSensitivity::OperationalEvidence, ArtifactDelivery::Embedded, true),
                artifact("oracle-commitment", d(32), 32, "application/vnd.luminous.commitment", None, "oracle-publisher", "oracle-key", d(22), "oracle-domain", "oracle-source", 3, ArtifactSensitivity::PrivateOracle, ArtifactDelivery::CommitmentOnly, true),
            ],
            claims: vec![
                claim("effect", ClaimKind::ParticipantEffect, ClaimStatus::Pass, Some(EvaluationArm::Candidate), "trial-a", "tenant-a/service-a", vec!["effect-receipt"], vec![]),
                claim("service-recovery", ClaimKind::ServiceOutcome, ClaimStatus::Pass, Some(EvaluationArm::Candidate), "trial-a", "service-a/critical-write", vec!["service-observation"], vec!["effect"]),
                claim("oracle-not-disclosed", ClaimKind::SealedCommitment, ClaimStatus::Pass, None, "", "oracle-commitment", vec!["oracle-commitment"], vec![]),
            ],
        };
        let materialized = vec![
            MaterializedArtifact { artifact_id: "effect-receipt".into(), archive_path: "evidence/effect.json".into(), computed_digest: d(30), computed_byte_length: 128 },
            MaterializedArtifact { artifact_id: "service-observation".into(), archive_path: "evidence/service.json".into(), computed_digest: d(31), computed_byte_length: 256 },
        ];
        (bundle, trust, materialized, d(90))
    }

    fn publisher(id: &str, key: &str, key_digest: Digest, domain: &str, source: &str) -> TrustedPublisher {
        TrustedPublisher { publisher_id: id.into(), key_id: key.into(), key_digest, control_domain: domain.into(), source_id: source.into(), valid_from_ms: 10, valid_until_ms: 1_000 }
    }

    fn domain(role: RunDomainRole, id: &str, control: &str, digest: Digest) -> RunDomainBinding {
        RunDomainBinding { role, identity_id: id.into(), control_domain: control.into(), identity_digest: digest }
    }

    fn artifact(
        id: &str, digest: Digest, length: u64, media: &str, path: Option<&str>,
        publisher: &str, key: &str, key_digest: Digest, domain: &str, source: &str,
        sequence: u64, sensitivity: ArtifactSensitivity, delivery: ArtifactDelivery, required: bool,
    ) -> BundleArtifact {
        BundleArtifact {
            artifact_id: id.into(),
            content_digest: digest,
            byte_length: length,
            media_type: media.into(),
            archive_path: path.map(str::to_owned),
            publisher_id: publisher.into(),
            publisher_key_id: key.into(),
            publisher_key_digest: key_digest,
            control_domain: domain.into(),
            source_id: source.into(),
            sequence,
            issued_at_ms: 90,
            sensitivity,
            delivery,
            required,
            retired_at_ms: None,
        }
    }

    fn claim(
        id: &str, kind: ClaimKind, status: ClaimStatus,
        arm: Option<EvaluationArm>, trial: &str, scope: &str,
        evidence: Vec<&str>, dependencies: Vec<&str>,
    ) -> EvidenceClaim {
        EvidenceClaim {
            claim_id: id.into(),
            kind,
            status,
            subject: ClaimSubject {
                run_id: "run-001".into(),
                manifest_commitment: d(5),
                trial_id: if trial.is_empty() { None } else { Some(trial.into()) },
                arm,
                scope_id: scope.into(),
            },
            evidence_artifact_ids: evidence.into_iter().map(str::to_owned).collect(),
            depends_on_claim_ids: dependencies.into_iter().map(str::to_owned).collect(),
            assumptions: Vec::new(),
            claim_ceiling: "Only the claim stated in the evidence record is supported".into(),
        }
    }

    #[test]
    fn clean_public_bundle_verifies_embedded_artifacts_and_active_commitment_only() {
        let (bundle, trust, materialized, digest) = fixture();
        let result = validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110).unwrap();
        assert_eq!(result.disposition, BundleDisposition::Pass);
        assert_eq!(result.artifact_count, 3);
        assert_eq!(result.embedded_artifact_count, 2);
        assert_eq!(result.commitment_only_artifact_count, 1);
        assert_eq!(result.pass_claims, 3);
    }

    #[test]
    fn public_bundle_rejects_active_payload_and_oracle_bytes() {
        for sensitivity in [ArtifactSensitivity::ActiveSealedPayload, ArtifactSensitivity::PrivateOracle] {
            let (mut bundle, trust, mut materialized, digest) = fixture();
            bundle.artifacts[2].sensitivity = sensitivity;
            bundle.artifacts[2].delivery = ArtifactDelivery::Embedded;
            bundle.artifacts[2].archive_path = Some("private/oracle.bin".into());
            materialized.push(MaterializedArtifact {
                artifact_id: "oracle-commitment".into(),
                archive_path: "private/oracle.bin".into(),
                computed_digest: d(32),
                computed_byte_length: 32,
            });
            assert_eq!(
                validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110),
                Err(BundleError::ActiveSealedDataLeaked("oracle-commitment".into()))
            );
        }
    }

    #[test]
    fn external_references_and_networked_mode_are_rejected() {
        let (mut bundle, trust, materialized, digest) = fixture();
        bundle.artifacts[0].delivery = ArtifactDelivery::ExternalReference;
        assert_eq!(
            validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110),
            Err(BundleError::ExternalReferenceForbidden("effect-receipt".into()))
        );
        let (mut bundle, trust, materialized, digest) = fixture();
        bundle.verification_mode = VerificationMode::Networked;
        assert_eq!(
            validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110),
            Err(BundleError::NetworkedVerificationForbidden)
        );
    }

    #[test]
    fn trust_policy_and_verifier_identity_are_pinned() {
        let (bundle, mut trust, materialized, digest) = fixture();
        trust.policy_digest = d(99);
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::BundleTrustPolicyMismatch));
        let (bundle, mut trust, materialized, digest) = fixture();
        trust.verifier_digest = d(98);
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::VerifierIdentityMismatch));
    }

    #[test]
    fn artifact_bytes_length_and_digest_must_match_the_inventory() {
        let (bundle, trust, mut materialized, digest) = fixture();
        materialized[0].computed_digest = d(99);
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::MaterializedDigestMismatch("effect-receipt".into())));
        let (bundle, trust, mut materialized, digest) = fixture();
        materialized[0].computed_byte_length += 1;
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::MaterializedLengthMismatch("effect-receipt".into())));
    }

    #[test]
    fn archive_path_traversal_and_ambiguous_paths_are_rejected() {
        for invalid in ["../secrets", "/absolute/file", "a//b", "a/./b", "C:/drive", "a\\b"] {
            let (mut bundle, trust, mut materialized, digest) = fixture();
            bundle.artifacts[0].archive_path = Some(invalid.into());
            materialized[0].archive_path = invalid.into();
            assert!(matches!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::InvalidArchivePath(_))));
        }
    }

    #[test]
    fn claim_graph_rejects_nonpass_dependencies_and_cycles() {
        let (mut bundle, trust, materialized, digest) = fixture();
        bundle.claims[0].status = ClaimStatus::Indeterminate;
        assert_eq!(
            validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110),
            Err(BundleError::ClaimDependencyNotPass("service-recovery".into(), "effect".into()))
        );
        let (mut bundle, trust, materialized, digest) = fixture();
        bundle.claims[0].depends_on_claim_ids = vec!["service-recovery".into()];
        assert_eq!(
            validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110),
            Err(BundleError::ClaimGraphCycle("effect".into()))
        );
    }

    #[test]
    fn missing_required_evidence_is_indeterminate_never_zero() {
        let (mut bundle, trust, mut materialized, digest) = fixture();
        materialized.retain(|item| item.artifact_id != "service-observation");
        bundle.artifacts[1].delivery = ArtifactDelivery::Unavailable;
        bundle.artifacts[1].archive_path = None;
        bundle.claims[1].status = ClaimStatus::Indeterminate;
        let result = validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110).unwrap();
        assert_eq!(result.disposition, BundleDisposition::Indeterminate);
        assert_eq!(result.unavailable_required_artifacts, 1);
    }

    #[test]
    fn retired_reveal_requires_retired_lane_and_exact_time_order() {
        let (mut bundle, trust, mut materialized, digest) = fixture();
        bundle.artifacts[2].sensitivity = ArtifactSensitivity::RetiredReveal;
        bundle.artifacts[2].delivery = ArtifactDelivery::Embedded;
        bundle.artifacts[2].archive_path = Some("retired/oracle.bin".into());
        materialized.push(MaterializedArtifact {
            artifact_id: "oracle-commitment".into(),
            archive_path: "retired/oracle.bin".into(),
            computed_digest: d(32),
            computed_byte_length: 32,
        });
        bundle.artifacts[2].retired_at_ms = Some(120);
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 130), Err(BundleError::RetiredRevealInWrongLane("oracle-commitment".into())));
        bundle.lane = EvaluationLane::RetiredReproducibility;
        bundle.created_at_ms = 100;
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 130), Err(BundleError::RetiredRevealTimeInvalid("oracle-commitment".into())));
        bundle.created_at_ms = 150;
        bundle.artifacts[2].issued_at_ms = 130;
        assert!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 160).is_ok());
    }

    #[test]
    fn publisher_key_source_and_validity_must_match_trust_snapshot() {
        let (mut bundle, mut trust, materialized, digest) = fixture();
        bundle.artifacts[0].publisher_key_digest = d(99);
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::TrustedPublisherMissing("effect-receipt".into())));
        let (mut bundle, mut trust, materialized, digest) = fixture();
        trust.trusted_publishers[0].valid_from_ms = 95;
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::PublisherOutsideValidity("effect-receipt".into())));
    }

    #[test]
    fn required_artifacts_must_be_referenced_and_materialized_once() {
        let (mut bundle, trust, mut materialized, digest) = fixture();
        bundle.claims[0].evidence_artifact_ids.clear();
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::InvalidClaim("effect".into())));
        let (bundle, trust, mut materialized, digest) = fixture();
        materialized.push(MaterializedArtifact { artifact_id: "foreign".into(), archive_path: "foreign.json".into(), computed_digest: d(90), computed_byte_length: 1 });
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::UnregisteredMaterializedArtifact("foreign".into())));
    }

    #[test]
    fn duplicate_artifact_claim_and_publisher_sequence_records_are_rejected() {
        let (mut bundle, trust, materialized, digest) = fixture();
        bundle.artifacts.push(bundle.artifacts[0].clone());
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::DuplicateArtifactId("effect-receipt".into())));
        let (mut bundle, trust, materialized, digest) = fixture();
        bundle.artifacts[1].sequence = 1;
        bundle.artifacts[1].source_id = "effect-source".into();
        bundle.artifacts[1].publisher_id = "participant-ledger".into();
        bundle.artifacts[1].publisher_key_id = "ledger-key".into();
        bundle.artifacts[1].publisher_key_digest = d(20);
        bundle.artifacts[1].control_domain = "ledger-domain".into();
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::DuplicatePublisherSequence("effect-source".into(), 1)));
    }

    #[test]
    fn wrong_run_domain_bindings_are_rejected() {
        let (mut bundle, trust, materialized, digest) = fixture();
        bundle.run_domains[0].identity_digest = d(99);
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::RunDomainBindingMismatch(RunDomainRole::Candidate)));
        let (mut bundle, trust, materialized, digest) = fixture();
        bundle.run_domains[1].control_domain = "candidate-domain".into();
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::RunControlDomainCollision));
    }

    #[test]
    fn claim_subject_and_status_laundering_are_rejected() {
        let (mut bundle, trust, materialized, digest) = fixture();
        bundle.claims[0].subject.trial_id = Some("unplanned".into());
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::ClaimSubjectNotPlanned("effect".into())));
        let (mut bundle, trust, materialized, digest) = fixture();
        bundle.claims[0].status = ClaimStatus::Pass;
        bundle.claims[0].evidence_artifact_ids = vec!["missing".into()];
        assert_eq!(validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110), Err(BundleError::MissingClaimEvidence("effect".into(), "missing".into())));
    }

    #[test]
    fn eighty_adversarial_mutations_fail_closed() {
        for vector in 0..80 {
            let (mut bundle, mut trust, mut materialized, digest) = fixture();
            match vector {
                0 => bundle.schema.push_str("-downgrade"),
                1 => bundle.bundle_id.clear(),
                2 => bundle.run_id.clear(),
                3 => bundle.manifest_commitment = d(0),
                4 => bundle.candidate_identity_digest = bundle.baseline_identity_digest.clone(),
                5 => bundle.evaluator_digest = d(0),
                6 => bundle.verifier_digest = d(0),
                7 => bundle.trust_policy_digest = d(0),
                8 => bundle.verification_mode = VerificationMode::Networked,
                9 => trust.policy_digest = d(99),
                10 => trust.maximum_artifact_bytes = 1,
                11 => bundle.created_at_ms = 1,
                12 => bundle.planned_arm_subjects.clear(),
                13 => bundle.planned_arm_subjects[0].cluster_id = "other".into(),
                14 => bundle.planned_arm_subjects.pop(),
                15 => bundle.planned_arm_subjects[1].trial_id = "trial-b".into(),
                16 => bundle.artifacts[0].artifact_id = " ".into(),
                17 => bundle.artifacts[0].content_digest = d(0),
                18 => bundle.artifacts[0].byte_length = 0,
                19 => bundle.artifacts[0].media_type = "bad-type".into(),
                20 => bundle.artifacts[0].publisher_id = "unknown".into(),
                21 => bundle.artifacts[0].control_domain = trust.verifier_control_domain.clone(),
                22 => bundle.artifacts[0].source_id = "unknown".into(),
                23 => bundle.artifacts[0].sequence = 0,
                24 => bundle.artifacts[0].issued_at_ms = bundle.created_at_ms + 1,
                25 => bundle.artifacts[0].delivery = ArtifactDelivery::ExternalReference,
                26 => bundle.artifacts[0].delivery = ArtifactDelivery::Unavailable,
                27 => bundle.artifacts[2].sensitivity = ArtifactSensitivity::ActiveSealedPayload,
                28 => bundle.artifacts[2].delivery = ArtifactDelivery::Embedded,
                29 => bundle.artifacts[2].sensitivity = ArtifactSensitivity::RestrictedPrivate,
                30 => bundle.artifacts[2].delivery = ArtifactDelivery::CommitmentOnly,
                31 => bundle.artifacts[2].sensitivity = ArtifactSensitivity::RetiredReveal,
                32 => {
                    bundle.lane = EvaluationLane::RetiredReproducibility;
                    bundle.artifacts[2].sensitivity = ArtifactSensitivity::RetiredReveal;
                    bundle.artifacts[2].delivery = ArtifactDelivery::Embedded;
                    bundle.artifacts[2].archive_path = Some("retired/oracle.bin".into());
                    bundle.artifacts[2].retired_at_ms = Some(bundle.created_at_ms + 1);
                    materialized.push(MaterializedArtifact {
                        artifact_id: "oracle-commitment".into(),
                        archive_path: "retired/oracle.bin".into(),
                        computed_digest: d(32),
                        computed_byte_length: 32,
                    });
                }
                33 => {
                    bundle.lane = EvaluationLane::RetiredReproducibility;
                    bundle.artifacts[2].sensitivity = ArtifactSensitivity::RetiredReveal;
                    bundle.artifacts[2].retired_at_ms = None;
                }
                34 => materialized[0].computed_digest = d(99),
                35 => materialized[0].computed_byte_length += 1,
                36 => materialized[0].archive_path = "../escape".into(),
                37 => materialized.push(materialized[0].clone()),
                38 => bundle.claims[0].claim_id.clear(),
                39 => bundle.claims[0].subject.run_id = "foreign".into(),
                40 => bundle.claims[0].subject.manifest_commitment = d(98),
                41 => bundle.claims[0].subject.scope_id.clear(),
                42 => bundle.claims[0].subject.arm = None,
                43 => bundle.claims[0].subject.trial_id = Some("foreign".into()),
                44 => bundle.claims[0].evidence_artifact_ids.clear(),
                45 => bundle.claims[0].evidence_artifact_ids.push("missing".into()),
                46 => bundle.claims[0].depends_on_claim_ids = vec!["missing".into()],
                47 => bundle.claims[1].depends_on_claim_ids = vec!["effect".into(), "effect".into()],
                48 => bundle.claims[1].depends_on_claim_ids = vec!["service-recovery".into()],
                49 => bundle.claims[0].status = ClaimStatus::Fail,
                50 => bundle.claims[0].claim_ceiling.clear(),
                51 => bundle.claims[1].status = ClaimStatus::Indeterminate,
                52 => bundle.claims[2].status = ClaimStatus::Fail,
                53 => bundle.claims[2].evidence_artifact_ids = vec!["missing".into()],
                54 => bundle.claims[2].assumptions = vec![" ".into()],
                55 => bundle.claims[2].subject.trial_id = Some("trial-a".into()),
                56 => trust.trusted_publishers.clear(),
                57 => trust.trusted_publishers[0].control_domain = "other-domain".into(),
                58 => trust.trusted_publishers[0].valid_from_ms = 95,
                59 => trust.trusted_publishers[0].valid_until_ms = 50,
                60 => bundle.claims[0].evidence_artifact_ids.push("effect-receipt".into()),
                61 => bundle.artifacts[1].content_digest = bundle.artifacts[0].content_digest.clone(),
                62 => bundle.artifacts[0].retired_at_ms = Some(1),
                63 => bundle.artifacts[0].delivery = ArtifactDelivery::CommitmentOnly,
                64 => {
                    bundle.artifacts[0].sensitivity = ArtifactSensitivity::Public;
                    bundle.artifacts[0].delivery = ArtifactDelivery::CommitmentOnly;
                    bundle.artifacts[0].archive_path = None;
                    materialized.retain(|item| item.artifact_id != "effect-receipt");
                }
                65 => bundle.artifacts[1].delivery = ArtifactDelivery::Unavailable,
                66 => bundle.claims[1].status = ClaimStatus::Fail,
                67 => bundle.claims[2].status = ClaimStatus::Fail,
                68 => {
                    bundle.disclosure = BundleDisclosure::Public;
                    bundle.artifacts[2].sensitivity = ArtifactSensitivity::RestrictedPrivate;
                    bundle.artifacts[2].delivery = ArtifactDelivery::Embedded;
                    bundle.artifacts[2].archive_path = Some("private/oracle.bin".into());
                    materialized.push(MaterializedArtifact {
                        artifact_id: "oracle-commitment".into(),
                        archive_path: "private/oracle.bin".into(),
                        computed_digest: d(32),
                        computed_byte_length: 32,
                    });
                }
                69 => {
                    bundle.disclosure = BundleDisclosure::Public;
                    bundle.artifacts[2].sensitivity = ArtifactSensitivity::PrivateOracle;
                    bundle.artifacts[2].delivery = ArtifactDelivery::Embedded;
                    bundle.artifacts[2].archive_path = Some("private/oracle.bin".into());
                    materialized.push(MaterializedArtifact {
                        artifact_id: "oracle-commitment".into(),
                        archive_path: "private/oracle.bin".into(),
                        computed_digest: d(32),
                        computed_byte_length: 32,
                    });
                }
                70 => bundle.artifacts[1].publisher_id = "oracle-publisher".into(),
                71 => bundle.artifacts[1].source_id = "oracle-source".into(),
                72 => bundle.artifacts[1].issued_at_ms = 1,
                73 => bundle.claims[0].depends_on_claim_ids = vec!["service-recovery".into()],
                74 => bundle.claims[1].evidence_artifact_ids = vec!["oracle-commitment".into()],
                75 => bundle.planned_arm_subjects[0].arm = EvaluationArm::Baseline,
                76 => trust.verifier_id = "substituted".into(),
                77 => bundle.verifier_id = "substituted".into(),
                78 => bundle.verifier_digest = d(99),
                _ => trust.valid_from_ms = trust.valid_until_ms,
            }
            match validate_evidence_bundle(&bundle, &digest, &trust, &materialized, 110) {
                Err(_) => {}
                Ok(receipt) => assert_ne!(
                    receipt.disposition,
                    BundleDisposition::Pass,
                    "bundle mutation {vector} produced a passing receipt"
                ),
            }
        }
    }
}
