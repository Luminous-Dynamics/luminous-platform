//! Semantic and temporal verification boundary for capacity dependency statements.
//!
//! Cryptographic verification, trust-root selection, issuer authorization,
//! SCITT receipt verification, calibration trust and replay state are delegated
//! to the configured verifier implementation. The core never treats a digest,
//! confidence number or transparency receipt as proof of topology correctness.
//!
//! A successful result means issuer-attested under the supplied trust policy.
//! It does not mean the inventory has been independently corroborated.

use std::collections::BTreeSet;

use crate::resource_allocation::ResourceCapacity;
use crate::CapacityDomainEvidence;

const MAX_CLAIMS: usize = 128;
const MAX_PATHS: usize = 64;
const MAX_RESOURCES_PER_PATH: usize = 32;
const MAX_ID_BYTES: usize = 128;
const MAX_STATEMENT_BYTES: usize = 1_048_576;
const MAX_SIGNATURE_BYTES: usize = 16_384;
const MAX_RECEIPT_BYTES: usize = 1_048_576;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceClass {
    Access,
    Aggregation,
    Backhaul,
    ProviderEgress,
    Transit,
    ServerEgress,
    OtherDeclared,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrafficDirection {
    Ingress,
    Egress,
    Bidirectional,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceClaim {
    pub resource_id: String,
    pub resource_class: ResourceClass,
    pub direction: TrafficDirection,
    pub capacity_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathResourceIncidence {
    pub path_id: String,
    pub resource_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedCapacityDependencyStatement {
    pub statement_id: String,
    pub issuer_id: String,
    pub signing_key_id: String,
    pub authority_policy_id: String,
    pub topology_sha256: String,
    pub inventory_sha256: String,
    pub statement_sha256: String,
    /// Exact canonical bytes covered by the issuer signature.
    pub canonical_statement: Vec<u8>,
    pub signature: Vec<u8>,
    pub resource_claims: Vec<ResourceClaim>,
    pub path_incidence: Vec<PathResourceIncidence>,
    pub coverage_complete: bool,
    pub unresolved_dependency_count: u16,
    /// Issuer claim. It is usable for ranking only with a trusted calibration.
    pub confidence_ppm: u32,
    pub calibration_profile_sha256: Option<String>,
    pub issued_at_unix_ms: u64,
    pub observed_at_unix_ms: u64,
    pub valid_until_unix_ms: u64,
    /// Monotonic per issuer/key; replay storage belongs to the verifier.
    pub sequence_number: u64,
    pub scitt_receipt: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapacityEvidencePolicy {
    /// Set to None to accept issuer-attested inventory without using its
    /// uncalibrated confidence claim for ranking.
    pub minimum_confidence_ppm: Option<u32>,
    pub maximum_observation_age_ms: u64,
    pub require_calibration_profile: bool,
    pub require_scitt_receipt: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustVerificationError {
    SignatureInvalid,
    IssuerNotTrusted,
    SigningKeyNotAuthorized,
    AuthorityScopeInsufficient,
    StatementDigestMismatch,
    CanonicalStatementMismatch,
    ReplayDetected,
    ScittReceiptInvalid,
    CalibrationProfileUnknown,
    CalibrationProfileNotAuthorized,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapacityEvidenceError {
    InvalidPolicy,
    InvalidIdentifier,
    InvalidTopologyDigest,
    InvalidInventoryDigest,
    InvalidStatementDigest,
    EmptyCanonicalStatement,
    CanonicalStatementTooLarge,
    EmptySignature,
    SignatureTooLarge,
    EmptyScittReceipt,
    ScittReceiptTooLarge,
    EmptyResourceClaims,
    TooManyResourceClaims,
    InvalidResourceId,
    DuplicateResourceId,
    ZeroResourceCapacity,
    EmptyPathIncidence,
    TooManyPaths,
    InvalidPathId,
    DuplicatePathId,
    EmptyPathResources,
    TooManyResourcesPerPath,
    DuplicatePathResource,
    UnknownPathResource,
    IncompleteCoverage,
    InvalidConfidence,
    ConfidenceBelowPolicyFloor,
    CalibrationProfileRequired,
    InvalidCalibrationProfileDigest,
    ScittReceiptRequired,
    InvalidIssuedAt,
    InvalidObservedAt,
    InvalidValidityWindow,
    StatementExpired,
    ObservationTooOld,
    InvalidSequenceNumber,
    EvidenceNotCalibrated,
    PathNotInInventory,
    EvidenceExpiredAtUse,
    EvidenceStaleAtUse,
    InvalidCurrentTime,
    Trust(TrustVerificationError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceMaturity {
    IssuerAttested,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedCapacityInventory {
    statement_id: String,
    issuer_id: String,
    signing_key_id: String,
    authority_policy_id: String,
    topology_sha256: String,
    inventory_sha256: String,
    statement_sha256: String,
    resource_claims: Vec<ResourceClaim>,
    path_incidence: Vec<PathResourceIncidence>,
    confidence_ppm: u32,
    calibration_profile_sha256: Option<String>,
    calibration_profile_verified: bool,
    transparency_receipt_verified: bool,
    issued_at_unix_ms: u64,
    observed_at_unix_ms: u64,
    valid_until_unix_ms: u64,
    sequence_number: u64,
}

impl VerifiedCapacityInventory {
    pub fn statement_id(&self) -> &str { &self.statement_id }
    pub fn issuer_id(&self) -> &str { &self.issuer_id }
    pub fn signing_key_id(&self) -> &str { &self.signing_key_id }
    pub fn authority_policy_id(&self) -> &str { &self.authority_policy_id }
    pub fn topology_sha256(&self) -> &str { &self.topology_sha256 }
    pub fn inventory_sha256(&self) -> &str { &self.inventory_sha256 }
    pub fn statement_sha256(&self) -> &str { &self.statement_sha256 }
    pub fn resource_claims(&self) -> &[ResourceClaim] { &self.resource_claims }
    pub fn path_incidence(&self) -> &[PathResourceIncidence] { &self.path_incidence }
    pub fn maturity(&self) -> EvidenceMaturity { EvidenceMaturity::IssuerAttested }
    pub fn confidence_claim_ppm(&self) -> u32 { self.confidence_ppm }
    pub fn calibration_profile_sha256(&self) -> Option<&str> {
        self.calibration_profile_sha256.as_deref()
    }
    pub fn calibration_profile_verified(&self) -> bool { self.calibration_profile_verified }
    pub fn transparency_receipt_verified(&self) -> bool { self.transparency_receipt_verified }
    pub fn issued_at_unix_ms(&self) -> u64 { self.issued_at_unix_ms }
    pub fn observed_at_unix_ms(&self) -> u64 { self.observed_at_unix_ms }
    pub fn valid_until_unix_ms(&self) -> u64 { self.valid_until_unix_ms }
    pub fn sequence_number(&self) -> u64 { self.sequence_number }

    /// Confidence is withheld from ranking unless a trusted calibration
    /// profile was verified for this issuer and statement class.
    pub fn ranking_confidence_ppm(&self) -> Option<u32> {
        self.calibration_profile_verified.then_some(self.confidence_ppm)
    }

    pub fn resource_capacities(&self) -> Vec<ResourceCapacity> {
        self.resource_claims.iter().map(|claim| ResourceCapacity {
            resource_id: claim.resource_id.clone(),
            capacity_bps: claim.capacity_bps,
        }).collect()
    }

    /// Project this statement's verified dependency evidence onto one path in
    /// the already-validated incidence manifest. Confidence is only forwarded
    /// when the verifier approved a calibration profile for this issuer.
    pub fn path_dependency_evidence(
        &self,
        path_id: &str,
        now_unix_ms: u64,
        maximum_observation_age_ms: u64,
    ) -> Result<(Vec<String>, CapacityDomainEvidence), CapacityEvidenceError> {
        if now_unix_ms == 0 || maximum_observation_age_ms == 0 {
            return Err(CapacityEvidenceError::InvalidCurrentTime);
        }
        if self.valid_until_unix_ms <= now_unix_ms {
            return Err(CapacityEvidenceError::EvidenceExpiredAtUse);
        }
        if now_unix_ms.saturating_sub(self.observed_at_unix_ms) > maximum_observation_age_ms {
            return Err(CapacityEvidenceError::EvidenceStaleAtUse);
        }
        if !self.calibration_profile_verified {
            return Err(CapacityEvidenceError::EvidenceNotCalibrated);
        }
        let incidence = self.path_incidence.iter()
            .find(|incidence| incidence.path_id == path_id)
            .ok_or(CapacityEvidenceError::PathNotInInventory)?;
        let mut capacity_domain_ids = incidence.resource_ids.clone();
        capacity_domain_ids.sort();
        Ok((capacity_domain_ids, CapacityDomainEvidence {
            source_id: self.issuer_id.clone(),
            inventory_sha256: self.inventory_sha256.clone(),
            topology_sha256: self.topology_sha256.clone(),
            complete: true,
            confidence_ppm: self.confidence_ppm,
            observed_at_unix_ms: self.observed_at_unix_ms,
            valid_until_unix_ms: self.valid_until_unix_ms,
        }))
    }
}

/// Production implementations must bind the signature to the exact canonical
/// bytes, verify issuer/key/authority scope for the claimed topology and
/// resource set, and enforce monotonic sequence/replay policy per issuer/key.
///
/// A SCITT receipt proves registration/inclusion of the statement digest; it
/// does not prove the truth of the resource claims. Calibration verification
/// must bind the calibration profile to the issuer/evidence class and must not
/// accept a caller-supplied digest without checking its trust.
pub trait CapacityInventoryTrustVerifier {
    /// Decode the canonical bytes using the versioned statement schema and
    /// prove that every semantic field passed separately in the typed object
    /// (issuer, key, topology, inventory digest, claims, incidence, coverage,
    /// confidence, timestamps, sequence and optional receipts) equals the
    /// decoded signed content. This prevents valid signed bytes being paired
    /// with substituted in-memory claims.
    fn verify_canonical_statement_binding(
        &self,
        statement: &SignedCapacityDependencyStatement,
    ) -> Result<(), TrustVerificationError>;

    fn verify_statement_signature_and_authority(
        &self,
        statement: &SignedCapacityDependencyStatement,
    ) -> Result<(), TrustVerificationError>;

    fn verify_scitt_receipt(
        &self,
        statement_sha256: &str,
        receipt: &[u8],
    ) -> Result<(), TrustVerificationError>;

    fn verify_calibration_profile(
        &self,
        issuer_id: &str,
        profile_sha256: &str,
        confidence_ppm: u32,
    ) -> Result<(), TrustVerificationError>;
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_ID_BYTES
        && !value.chars().any(char::is_control)
}

fn validate_policy(policy: CapacityEvidencePolicy) -> Result<(), CapacityEvidenceError> {
    if policy.maximum_observation_age_ms == 0
        || policy.minimum_confidence_ppm.is_some_and(|value| value == 0 || value > 1_000_000)
    {
        return Err(CapacityEvidenceError::InvalidPolicy);
    }
    Ok(())
}

fn validate_statement(
    statement: &SignedCapacityDependencyStatement,
    now_unix_ms: u64,
    policy: CapacityEvidencePolicy,
) -> Result<(), CapacityEvidenceError> {
    validate_policy(policy)?;
    if !valid_id(&statement.statement_id)
        || !valid_id(&statement.issuer_id)
        || !valid_id(&statement.signing_key_id)
        || !valid_id(&statement.authority_policy_id)
    {
        return Err(CapacityEvidenceError::InvalidIdentifier);
    }
    if !is_sha256(&statement.topology_sha256) {
        return Err(CapacityEvidenceError::InvalidTopologyDigest);
    }
    if !is_sha256(&statement.inventory_sha256) {
        return Err(CapacityEvidenceError::InvalidInventoryDigest);
    }
    if !is_sha256(&statement.statement_sha256) {
        return Err(CapacityEvidenceError::InvalidStatementDigest);
    }
    if statement.canonical_statement.is_empty() {
        return Err(CapacityEvidenceError::EmptyCanonicalStatement);
    }
    if statement.canonical_statement.len() > MAX_STATEMENT_BYTES {
        return Err(CapacityEvidenceError::CanonicalStatementTooLarge);
    }
    if statement.signature.is_empty() {
        return Err(CapacityEvidenceError::EmptySignature);
    }
    if statement.signature.len() > MAX_SIGNATURE_BYTES {
        return Err(CapacityEvidenceError::SignatureTooLarge);
    }
    if statement.sequence_number == 0 {
        return Err(CapacityEvidenceError::InvalidSequenceNumber);
    }
    if statement.issued_at_unix_ms == 0 || statement.issued_at_unix_ms > now_unix_ms {
        return Err(CapacityEvidenceError::InvalidIssuedAt);
    }
    if statement.observed_at_unix_ms == 0
        || statement.observed_at_unix_ms > statement.issued_at_unix_ms
        || statement.observed_at_unix_ms > now_unix_ms
    {
        return Err(CapacityEvidenceError::InvalidObservedAt);
    }
    if statement.valid_until_unix_ms <= statement.issued_at_unix_ms
        || statement.valid_until_unix_ms <= statement.observed_at_unix_ms
    {
        return Err(CapacityEvidenceError::InvalidValidityWindow);
    }
    if statement.valid_until_unix_ms <= now_unix_ms {
        return Err(CapacityEvidenceError::StatementExpired);
    }
    if now_unix_ms - statement.observed_at_unix_ms > policy.maximum_observation_age_ms {
        return Err(CapacityEvidenceError::ObservationTooOld);
    }
    if statement.confidence_ppm == 0 || statement.confidence_ppm > 1_000_000 {
        return Err(CapacityEvidenceError::InvalidConfidence);
    }
    if policy.minimum_confidence_ppm.is_some_and(|floor| statement.confidence_ppm < floor) {
        return Err(CapacityEvidenceError::ConfidenceBelowPolicyFloor);
    }
    if policy.require_calibration_profile && statement.calibration_profile_sha256.is_none() {
        return Err(CapacityEvidenceError::CalibrationProfileRequired);
    }
    if statement.calibration_profile_sha256.as_ref().is_some_and(|digest| !is_sha256(digest)) {
        return Err(CapacityEvidenceError::InvalidCalibrationProfileDigest);
    }
    if policy.require_scitt_receipt && statement.scitt_receipt.is_none() {
        return Err(CapacityEvidenceError::ScittReceiptRequired);
    }
    if let Some(receipt) = &statement.scitt_receipt {
        if receipt.is_empty() {
            return Err(CapacityEvidenceError::EmptyScittReceipt);
        }
        if receipt.len() > MAX_RECEIPT_BYTES {
            return Err(CapacityEvidenceError::ScittReceiptTooLarge);
        }
    }
    if statement.resource_claims.is_empty() {
        return Err(CapacityEvidenceError::EmptyResourceClaims);
    }
    if statement.resource_claims.len() > MAX_CLAIMS {
        return Err(CapacityEvidenceError::TooManyResourceClaims);
    }
    let mut resource_ids = BTreeSet::new();
    for claim in &statement.resource_claims {
        if !valid_id(&claim.resource_id) {
            return Err(CapacityEvidenceError::InvalidResourceId);
        }
        if !resource_ids.insert(claim.resource_id.as_str()) {
            return Err(CapacityEvidenceError::DuplicateResourceId);
        }
        if claim.capacity_bps == 0 {
            return Err(CapacityEvidenceError::ZeroResourceCapacity);
        }
    }
    if statement.path_incidence.is_empty() {
        return Err(CapacityEvidenceError::EmptyPathIncidence);
    }
    if statement.path_incidence.len() > MAX_PATHS {
        return Err(CapacityEvidenceError::TooManyPaths);
    }
    let mut path_ids = BTreeSet::new();
    for path in &statement.path_incidence {
        if !valid_id(&path.path_id) {
            return Err(CapacityEvidenceError::InvalidPathId);
        }
        if !path_ids.insert(path.path_id.as_str()) {
            return Err(CapacityEvidenceError::DuplicatePathId);
        }
        if path.resource_ids.is_empty() {
            return Err(CapacityEvidenceError::EmptyPathResources);
        }
        if path.resource_ids.len() > MAX_RESOURCES_PER_PATH {
            return Err(CapacityEvidenceError::TooManyResourcesPerPath);
        }
        let mut path_resources = BTreeSet::new();
        for id in &path.resource_ids {
            if !valid_id(id) {
                return Err(CapacityEvidenceError::InvalidResourceId);
            }
            if !path_resources.insert(id.as_str()) {
                return Err(CapacityEvidenceError::DuplicatePathResource);
            }
            if !resource_ids.contains(id.as_str()) {
                return Err(CapacityEvidenceError::UnknownPathResource);
            }
        }
    }
    if !statement.coverage_complete || statement.unresolved_dependency_count != 0 {
        return Err(CapacityEvidenceError::IncompleteCoverage);
    }
    Ok(())
}

/// Validate the signed statement and create an issuer-attested inventory only
/// after the configured verifier has approved signature, source authority,
/// replay state, optional transparency receipt and optional calibration.
pub fn verify_capacity_inventory<V: CapacityInventoryTrustVerifier>(
    statement: &SignedCapacityDependencyStatement,
    now_unix_ms: u64,
    policy: CapacityEvidencePolicy,
    verifier: &V,
) -> Result<VerifiedCapacityInventory, CapacityEvidenceError> {
    validate_statement(statement, now_unix_ms, policy)?;
    verifier.verify_canonical_statement_binding(statement)
        .map_err(CapacityEvidenceError::Trust)?;
    verifier.verify_statement_signature_and_authority(statement)
        .map_err(CapacityEvidenceError::Trust)?;

    let transparency_receipt_verified = if let Some(receipt) = &statement.scitt_receipt {
        verifier.verify_scitt_receipt(&statement.statement_sha256, receipt)
            .map_err(CapacityEvidenceError::Trust)?;
        true
    } else {
        false
    };

    let calibration_profile_verified = if let Some(profile) = &statement.calibration_profile_sha256 {
        verifier.verify_calibration_profile(&statement.issuer_id, profile, statement.confidence_ppm)
            .map_err(CapacityEvidenceError::Trust)?;
        true
    } else {
        false
    };

    Ok(VerifiedCapacityInventory {
        statement_id: statement.statement_id.clone(),
        issuer_id: statement.issuer_id.clone(),
        signing_key_id: statement.signing_key_id.clone(),
        authority_policy_id: statement.authority_policy_id.clone(),
        topology_sha256: statement.topology_sha256.clone(),
        inventory_sha256: statement.inventory_sha256.clone(),
        statement_sha256: statement.statement_sha256.clone(),
        resource_claims: statement.resource_claims.clone(),
        path_incidence: statement.path_incidence.clone(),
        confidence_ppm: statement.confidence_ppm,
        calibration_profile_sha256: statement.calibration_profile_sha256.clone(),
        calibration_profile_verified,
        transparency_receipt_verified,
        issued_at_unix_ms: statement.issued_at_unix_ms,
        observed_at_unix_ms: statement.observed_at_unix_ms,
        valid_until_unix_ms: statement.valid_until_unix_ms,
        sequence_number: statement.sequence_number,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct MockVerifier {
        signature_ok: bool,
        authority_ok: bool,
        receipt_ok: bool,
        calibration_ok: bool,
        replay: bool,
    }

    impl Default for MockVerifier {
        fn default() -> Self {
            Self {
                signature_ok: true,
                authority_ok: true,
                receipt_ok: true,
                calibration_ok: true,
                replay: false,
            }
        }
    }

    impl CapacityInventoryTrustVerifier for MockVerifier {
        fn verify_canonical_statement_binding(
            &self,
            _statement: &SignedCapacityDependencyStatement,
        ) -> Result<(), TrustVerificationError> {
            Ok(())
        }

        fn verify_statement_signature_and_authority(
            &self,
            _statement: &SignedCapacityDependencyStatement,
        ) -> Result<(), TrustVerificationError> {
            if !self.signature_ok { return Err(TrustVerificationError::SignatureInvalid); }
            if !self.authority_ok { return Err(TrustVerificationError::AuthorityScopeInsufficient); }
            if self.replay { return Err(TrustVerificationError::ReplayDetected); }
            Ok(())
        }

        fn verify_scitt_receipt(
            &self,
            _statement_sha256: &str,
            _receipt: &[u8],
        ) -> Result<(), TrustVerificationError> {
            if self.receipt_ok { Ok(()) } else { Err(TrustVerificationError::ScittReceiptInvalid) }
        }

        fn verify_calibration_profile(
            &self,
            _issuer_id: &str,
            _profile_sha256: &str,
            _confidence_ppm: u32,
        ) -> Result<(), TrustVerificationError> {
            if self.calibration_ok { Ok(()) } else { Err(TrustVerificationError::CalibrationProfileUnknown) }
        }
    }

    fn statement() -> SignedCapacityDependencyStatement {
        SignedCapacityDependencyStatement {
            statement_id: "statement-001".to_owned(),
            issuer_id: "operator-authority-01".to_owned(),
            signing_key_id: "operator-key-01".to_owned(),
            authority_policy_id: "regional-topology-authority-v1".to_owned(),
            topology_sha256: "a".repeat(64),
            inventory_sha256: "b".repeat(64),
            statement_sha256: "c".repeat(64),
            canonical_statement: b"canonical-capacity-inventory".to_vec(),
            signature: vec![1, 2, 3],
            resource_claims: vec![ResourceClaim {
                resource_id: "backhaul".to_owned(),
                resource_class: ResourceClass::Backhaul,
                direction: TrafficDirection::Egress,
                capacity_bps: 100_000_000,
            }],
            path_incidence: vec![PathResourceIncidence {
                path_id: "path-a".to_owned(),
                resource_ids: vec!["backhaul".to_owned()],
            }],
            coverage_complete: true,
            unresolved_dependency_count: 0,
            confidence_ppm: 970_000,
            calibration_profile_sha256: Some("d".repeat(64)),
            issued_at_unix_ms: 1_000_000,
            observed_at_unix_ms: 990_000,
            valid_until_unix_ms: 1_200_000,
            sequence_number: 7,
            scitt_receipt: Some(vec![4, 5, 6]),
        }
    }

    fn policy() -> CapacityEvidencePolicy {
        CapacityEvidencePolicy {
            minimum_confidence_ppm: Some(950_000),
            maximum_observation_age_ms: 60_000,
            require_calibration_profile: true,
            require_scitt_receipt: true,
        }
    }

    #[test]
    fn valid_statement_creates_issuer_attested_inventory() {
        let verified = verify_capacity_inventory(&statement(), 1_050_000, policy(), &MockVerifier::default())
            .expect("valid statement");
        assert_eq!(verified.maturity(), EvidenceMaturity::IssuerAttested);
        assert!(verified.transparency_receipt_verified());
        assert!(verified.calibration_profile_verified());
        assert_eq!(verified.ranking_confidence_ppm(), Some(970_000));
        assert_eq!(verified.resource_capacities()[0].capacity_bps, 100_000_000);
    }

    #[test]
    fn verified_inventory_projects_path_dependencies_with_provenance() {
        let verified = verify_capacity_inventory(
            &statement(), 1_050_000, policy(), &MockVerifier::default()
        ).expect("verified inventory");
        let (domains, evidence) = verified.path_dependency_evidence("path-a", 1_060_000, 60_000)
            .expect("path dependency evidence");
        assert_eq!(domains, vec!["backhaul".to_owned()]);
        assert_eq!(evidence.source_id, "operator-authority-01");
        assert_eq!(evidence.inventory_sha256, "b".repeat(64));
        assert_eq!(evidence.topology_sha256, "a".repeat(64));
        assert_eq!(evidence.confidence_ppm, 970_000);
    }

    #[test]
    fn uncalibrated_inventory_cannot_project_ranking_evidence() {
        let mut input = statement();
        input.calibration_profile_sha256 = None;
        let mut policy = policy();
        policy.minimum_confidence_ppm = None;
        policy.require_calibration_profile = false;
        let verified = verify_capacity_inventory(
            &input, 1_050_000, policy, &MockVerifier::default()
        ).expect("issuer-attested inventory");
        assert_eq!(
            verified.path_dependency_evidence("path-a", 1_060_000, 60_000).unwrap_err(),
            CapacityEvidenceError::EvidenceNotCalibrated
        );
    }

    #[test]
    fn projected_path_evidence_rechecks_expiry() {
        let verified = verify_capacity_inventory(
            &statement(), 1_050_000, policy(), &MockVerifier::default()
        ).expect("verified inventory");
        assert_eq!(
            verified.path_dependency_evidence("path-a", 1_200_000, 60_000).unwrap_err(),
            CapacityEvidenceError::EvidenceExpiredAtUse
        );
    }

    #[test]
    fn semantic_fields_not_bound_to_canonical_bytes_are_rejected() {
        struct MismatchedCanonicalVerifier;
        impl CapacityInventoryTrustVerifier for MismatchedCanonicalVerifier {
            fn verify_canonical_statement_binding(
                &self,
                _statement: &SignedCapacityDependencyStatement,
            ) -> Result<(), TrustVerificationError> {
                Err(TrustVerificationError::CanonicalStatementMismatch)
            }
            fn verify_statement_signature_and_authority(
                &self,
                _statement: &SignedCapacityDependencyStatement,
            ) -> Result<(), TrustVerificationError> {
                Ok(())
            }
            fn verify_scitt_receipt(
                &self,
                _statement_sha256: &str,
                _receipt: &[u8],
            ) -> Result<(), TrustVerificationError> {
                Ok(())
            }
            fn verify_calibration_profile(
                &self,
                _issuer_id: &str,
                _profile_sha256: &str,
                _confidence_ppm: u32,
            ) -> Result<(), TrustVerificationError> {
                Ok(())
            }
        }

        let result = verify_capacity_inventory(
            &statement(),
            1_050_000,
            policy(),
            &MismatchedCanonicalVerifier,
        );
        assert_eq!(
            result.unwrap_err(),
            CapacityEvidenceError::Trust(TrustVerificationError::CanonicalStatementMismatch)
        );
    }

    #[test]
    fn invalid_signature_is_rejected() {
        let result = verify_capacity_inventory(&statement(), 1_050_000, policy(), &MockVerifier {
            signature_ok: false,
            ..MockVerifier::default()
        });
        assert_eq!(result.unwrap_err(), CapacityEvidenceError::Trust(TrustVerificationError::SignatureInvalid));
    }

    #[test]
    fn unauthorized_issuer_scope_is_rejected() {
        let result = verify_capacity_inventory(&statement(), 1_050_000, policy(), &MockVerifier {
            authority_ok: false,
            ..MockVerifier::default()
        });
        assert_eq!(result.unwrap_err(), CapacityEvidenceError::Trust(TrustVerificationError::AuthorityScopeInsufficient));
    }

    #[test]
    fn replay_is_rejected_by_trust_verifier() {
        let result = verify_capacity_inventory(&statement(), 1_050_000, policy(), &MockVerifier {
            replay: true,
            ..MockVerifier::default()
        });
        assert_eq!(result.unwrap_err(), CapacityEvidenceError::Trust(TrustVerificationError::ReplayDetected));
    }

    #[test]
    fn statement_with_bad_digest_is_rejected_before_crypto_verification() {
        let mut input = statement();
        input.statement_sha256 = "bad".to_owned();
        let result = verify_capacity_inventory(&input, 1_050_000, policy(), &MockVerifier::default());
        assert_eq!(result.unwrap_err(), CapacityEvidenceError::InvalidStatementDigest);
    }

    #[test]
    fn expired_inventory_is_rejected() {
        let result = verify_capacity_inventory(&statement(), 1_200_000, policy(), &MockVerifier::default());
        assert_eq!(result.unwrap_err(), CapacityEvidenceError::StatementExpired);
    }

    #[test]
    fn stale_inventory_is_rejected() {
        let result = verify_capacity_inventory(&statement(), 1_100_001, policy(), &MockVerifier::default());
        assert_eq!(result.unwrap_err(), CapacityEvidenceError::ObservationTooOld);
    }

    #[test]
    fn incomplete_inventory_is_rejected() {
        let mut input = statement();
        input.coverage_complete = false;
        let result = verify_capacity_inventory(&input, 1_050_000, policy(), &MockVerifier::default());
        assert_eq!(result.unwrap_err(), CapacityEvidenceError::IncompleteCoverage);
    }

    #[test]
    fn scitt_receipt_is_not_optional_when_policy_requires_it() {
        let mut input = statement();
        input.scitt_receipt = None;
        let result = verify_capacity_inventory(&input, 1_050_000, policy(), &MockVerifier::default());
        assert_eq!(result.unwrap_err(), CapacityEvidenceError::ScittReceiptRequired);
    }

    #[test]
    fn confidence_below_policy_floor_is_rejected() {
        let mut input = statement();
        input.confidence_ppm = 949_999;
        let result = verify_capacity_inventory(&input, 1_050_000, policy(), &MockVerifier::default());
        assert_eq!(result.unwrap_err(), CapacityEvidenceError::ConfidenceBelowPolicyFloor);
    }

    #[test]
    fn uncalibrated_confidence_is_not_exposed_for_ranking() {
        let mut input = statement();
        input.calibration_profile_sha256 = None;
        let mut policy = policy();
        policy.minimum_confidence_ppm = None;
        policy.require_calibration_profile = false;
        let verified = verify_capacity_inventory(&input, 1_050_000, policy, &MockVerifier::default())
            .expect("issuer-attested but uncalibrated");
        assert_eq!(verified.maturity(), EvidenceMaturity::IssuerAttested);
        assert_eq!(verified.ranking_confidence_ppm(), None);
    }

    #[test]
    fn invalid_scitt_receipt_is_rejected() {
        let result = verify_capacity_inventory(&statement(), 1_050_000, policy(), &MockVerifier {
            receipt_ok: false,
            ..MockVerifier::default()
        });
        assert_eq!(result.unwrap_err(), CapacityEvidenceError::Trust(TrustVerificationError::ScittReceiptInvalid));
    }

    #[test]
    fn invalid_calibration_profile_is_rejected() {
        let result = verify_capacity_inventory(&statement(), 1_050_000, policy(), &MockVerifier {
            calibration_ok: false,
            ..MockVerifier::default()
        });
        assert_eq!(result.unwrap_err(), CapacityEvidenceError::Trust(TrustVerificationError::CalibrationProfileUnknown));
    }

    #[test]
    fn duplicate_resource_inventory_is_rejected() {
        let mut input = statement();
        input.resource_claims.push(input.resource_claims[0].clone());
        let result = verify_capacity_inventory(&input, 1_050_000, policy(), &MockVerifier::default());
        assert_eq!(result.unwrap_err(), CapacityEvidenceError::DuplicateResourceId);
    }

    #[test]
    fn unknown_path_resource_reference_is_rejected() {
        let mut input = statement();
        input.path_incidence[0].resource_ids.push("unknown".to_owned());
        let result = verify_capacity_inventory(&input, 1_050_000, policy(), &MockVerifier::default());
        assert_eq!(result.unwrap_err(), CapacityEvidenceError::UnknownPathResource);
    }

    #[test]
    fn path_incidence_without_resources_is_rejected() {
        let mut input = statement();
        input.path_incidence[0].resource_ids.clear();
        let result = verify_capacity_inventory(&input, 1_050_000, policy(), &MockVerifier::default());
        assert_eq!(result.unwrap_err(), CapacityEvidenceError::EmptyPathResources);
    }
}
