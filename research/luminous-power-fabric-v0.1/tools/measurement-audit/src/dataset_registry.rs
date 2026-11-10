use crate::AuditError;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetAvailability {
    PublicAvailable,
    AccessControlled,
    PendingRelease,
    Planned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetEvidenceClass {
    PhysicalMeasurement,
    ModelledFacility,
    PreprintDescribedUnverified,
    PlannedResearch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetAnalysisUse {
    EnergyAccounting,
    SlowWorkloadDynamics,
    FacilityLoadModelling,
    PowerFlexibilityAndQos,
    FastElectricalTransient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetUseOperation {
    CommercialUse,
    CreateDerivative,
    Redistribute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetUseDecision {
    Allowed,
    Denied,
    Unknown,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetLicense {
    /// SPDX identifier or explicit "NOASSERTION"/custom identifier.
    pub license_id: Option<String>,
    pub license_url: Option<String>,
    /// URL to the source that declares this licence for this dataset.
    pub verification_source_url: Option<String>,
    pub verified: bool,
    pub commercial_use_allowed: Option<bool>,
    pub derivatives_allowed: Option<bool>,
    pub redistribution_allowed: Option<bool>,
    pub attribution_required: bool,
    pub no_endorsement_required: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatasetRegistryEntry {
    pub dataset_id: String,
    pub title: String,
    pub doi: Option<String>,
    pub source_url: String,
    pub publisher: String,
    pub version: Option<String>,
    pub retrieval_date_utc: String,
    pub availability: DatasetAvailability,
    pub evidence_class: DatasetEvidenceClass,
    pub physical_boundary: String,
    pub file_format: Option<String>,
    pub size_bytes: Option<u64>,
    pub file_count: Option<u64>,
    pub access_method: String,
    pub notes: Vec<String>,
    /// Largest observed/declared timestamp interval. None means unknown.
    pub maximum_sample_interval_ns: Option<u64>,
    /// Analog sensor/measurement-chain bandwidth. None means unknown.
    pub sensor_bandwidth_hz: Option<f64>,
    /// True only when filter response/metadata is verified for this dataset.
    pub anti_alias_filter_verified: Option<bool>,
    /// Highest signal bandwidth this dataset is registered to support.
    /// Present only when all fast-claim conditions are met.
    pub fast_claim_bandwidth_hz: Option<f64>,
    pub supported_uses: Vec<DatasetAnalysisUse>,
    pub license: DatasetLicense,
    pub citation: String,
    pub required_notices: Vec<String>,
    /// True only where the local operator has verified access under any
    /// institutional terms in addition to the public catalogue record.
    pub access_granted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatasetUseRequest {
    pub commercial_use: bool,
    pub create_derivative: bool,
    pub redistribute: bool,
}

impl DatasetRegistryEntry {
    pub const fn claim_ceiling(&self) -> &'static str {
        "DATASET_REGISTRY_METADATA_ONLY_NOT_DATASET_VALIDATION_NOT_LICENCE_OPINION_NOT_ENGINEERING_QUALIFICATION"
    }

    pub fn conservative_nyquist_hz(&self) -> Option<f64> {
        self.maximum_sample_interval_ns
            .filter(|interval| *interval > 0)
            .map(|interval| 1_000_000_000.0 / interval as f64 / 2.0)
    }

    pub fn supports_fast_transient_claim(&self) -> bool {
        let sample_nyquist = self.conservative_nyquist_hz();
        let (Some(nyquist_hz), Some(sensor_bandwidth_hz), Some(claim_bandwidth_hz)) = (
            sample_nyquist,
            self.sensor_bandwidth_hz,
            self.fast_claim_bandwidth_hz,
        ) else {
            return false;
        };

        self.evidence_class == DatasetEvidenceClass::PhysicalMeasurement
            && self
                .anti_alias_filter_verified
                .is_some_and(|verified| verified)
            && sensor_bandwidth_hz.is_finite()
            && sensor_bandwidth_hz > 0.0
            && claim_bandwidth_hz.is_finite()
            && claim_bandwidth_hz > 0.0
            && claim_bandwidth_hz <= nyquist_hz
            && claim_bandwidth_hz <= sensor_bandwidth_hz
            && self.supported_uses.contains(&DatasetAnalysisUse::FastElectricalTransient)
    }

    /// Fail-closed decision from the declared rights metadata. This is not
    /// legal advice and does not replace checking the actual licence text,
    /// institutional agreements, third-party rights or source-specific notices.
    pub fn decide_use(&self, request: DatasetUseRequest) -> DatasetUseDecision {
        match self.availability {
            DatasetAvailability::PendingRelease | DatasetAvailability::Planned => {
                return DatasetUseDecision::Unavailable;
            }
            DatasetAvailability::AccessControlled if !self.access_granted => {
                return DatasetUseDecision::Unavailable;
            }
            DatasetAvailability::PublicAvailable | DatasetAvailability::AccessControlled => {}
        }

        if !self.license.verified {
            return DatasetUseDecision::Unknown;
        }

        let checks = [
            (request.commercial_use, self.license.commercial_use_allowed),
            (request.create_derivative, self.license.derivatives_allowed),
            (request.redistribute, self.license.redistribution_allowed),
        ];

        let mut unknown = false;
        for (requested, permission) in checks {
            if !requested {
                continue;
            }
            match permission {
                Some(true) => {}
                Some(false) => return DatasetUseDecision::Denied,
                None => unknown = true,
            }
        }

        if unknown {
            DatasetUseDecision::Unknown
        } else {
            DatasetUseDecision::Allowed
        }
    }
}

pub fn validate_dataset_registry(
    entries: &[DatasetRegistryEntry],
) -> Result<(), AuditError> {
    if entries.is_empty() {
        return Err(AuditError("dataset registry must contain at least one entry".to_string()));
    }

    let mut ids = HashSet::new();
    for entry in entries {
        for (value, field) in [
            (entry.dataset_id.as_str(), "dataset_id"),
            (entry.title.as_str(), "title"),
            (entry.source_url.as_str(), "source_url"),
            (entry.publisher.as_str(), "publisher"),
            (entry.retrieval_date_utc.as_str(), "retrieval_date_utc"),
            (entry.physical_boundary.as_str(), "physical_boundary"),
            (entry.access_method.as_str(), "access_method"),
            (entry.citation.as_str(), "citation"),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!("dataset registry {field} must not be blank")));
            }
        }
        if !ids.insert(entry.dataset_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate dataset registry ID {:?}",
                entry.dataset_id
            )));
        }
        if !entry.retrieval_date_utc.ends_with('Z') {
            return Err(AuditError(
                "retrieval_date_utc must be represented as a UTC timestamp ending in Z"
                    .to_string(),
            ));
        }

        if let Some(interval) = entry.maximum_sample_interval_ns {
            if interval == 0 {
                return Err(AuditError(
                    "maximum_sample_interval_ns must be positive when present".to_string(),
                ));
            }
        }
        if let Some(bandwidth) = entry.sensor_bandwidth_hz {
            if !bandwidth.is_finite() || bandwidth <= 0.0 {
                return Err(AuditError(
                    "sensor_bandwidth_hz must be finite and positive when present".to_string(),
                ));
            }
        }
        if let Some(claim_bandwidth) = entry.fast_claim_bandwidth_hz {
            if !claim_bandwidth.is_finite() || claim_bandwidth <= 0.0 {
                return Err(AuditError(
                    "fast_claim_bandwidth_hz must be finite and positive when present".to_string(),
                ));
            }
        }
        if entry.supported_uses.contains(&DatasetAnalysisUse::FastElectricalTransient)
            && !entry.supports_fast_transient_claim()
        {
            return Err(AuditError(format!(
                "dataset {:?} declares fast-transient use without measured sample, sensor bandwidth and anti-alias evidence",
                entry.dataset_id
            )));
        }

        if entry.license.verified {
            if entry
                .license
                .license_id
                .as_ref()
                .is_none_or(|license| license.trim().is_empty())
                || entry
                    .license
                    .license_url
                    .as_ref()
                    .is_none_or(|url| url.trim().is_empty())
                || entry
                    .license
                    .verification_source_url
                    .as_ref()
                    .is_none_or(|url| url.trim().is_empty())
            {
                return Err(AuditError(
                    "verified dataset licence requires an identifier and both licence/source URLs"
                        .to_string(),
                ));
            }
        } else if entry.license.commercial_use_allowed.is_some()
            || entry.license.derivatives_allowed.is_some()
            || entry.license.redistribution_allowed.is_some()
        {
            return Err(AuditError(
                "unverified licence cannot assert affirmative or negative use permissions"
                    .to_string(),
            ));
        }

        if matches!(
            entry.availability,
            DatasetAvailability::PendingRelease | DatasetAvailability::Planned
        ) && entry.access_granted
        {
            return Err(AuditError(
                "pending/planned dataset cannot be marked as access_granted".to_string(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        validate_dataset_registry, DatasetAnalysisUse, DatasetAvailability, DatasetEvidenceClass,
        DatasetLicense, DatasetRegistryEntry, DatasetUseDecision, DatasetUseRequest,
    };

    fn license(
        id: Option<&str>,
        verified: bool,
        commercial: Option<bool>,
        derivatives: Option<bool>,
        redistribution: Option<bool>,
    ) -> DatasetLicense {
        DatasetLicense {
            license_id: id.map(str::to_string),
            license_url: verified.then(|| "https://example.org/license".to_string()),
            verification_source_url: verified.then(|| "https://example.org/dataset".to_string()),
            verified,
            commercial_use_allowed: commercial,
            derivatives_allowed: derivatives,
            redistribution_allowed: redistribution,
            attribution_required: true,
            no_endorsement_required: true,
        }
    }

    fn entry(id: &str) -> DatasetRegistryEntry {
        DatasetRegistryEntry {
            dataset_id: id.to_string(),
            title: format!("Dataset {id}"),
            doi: None,
            source_url: "https://example.org/dataset".to_string(),
            publisher: "Example publisher".to_string(),
            version: Some("1".to_string()),
            retrieval_date_utc: "2026-11-10T00:00:00Z".to_string(),
            availability: DatasetAvailability::PublicAvailable,
            evidence_class: DatasetEvidenceClass::PhysicalMeasurement,
            physical_boundary: "GPU board".to_string(),
            file_format: Some("CSV".to_string()),
            size_bytes: None,
            file_count: None,
            access_method: "public URL".to_string(),
            notes: Vec::new(),
            maximum_sample_interval_ns: Some(20_000_000),
            sensor_bandwidth_hz: None,
            anti_alias_filter_verified: None,
            fast_claim_bandwidth_hz: None,
            supported_uses: vec![
                DatasetAnalysisUse::EnergyAccounting,
                DatasetAnalysisUse::SlowWorkloadDynamics,
            ],
            license: license(Some("CC-BY-4.0"), true, Some(true), Some(true), Some(true)),
            citation: "Example citation".to_string(),
            required_notices: vec!["Attribution required".to_string()],
            access_granted: true,
        }
    }

    fn request(commercial: bool, derivative: bool, redistribute: bool) -> DatasetUseRequest {
        DatasetUseRequest {
            commercial_use: commercial,
            create_derivative: derivative,
            redistribute,
        }
    }

    #[test]
    fn public_cc_by_dataset_allows_declared_uses_with_attribution() {
        let value = entry("cc-by");
        assert_eq!(value.decide_use(request(true, true, true)), DatasetUseDecision::Allowed);
    }

    #[test]
    fn cc_by_nc_nd_dataset_blocks_commercial_and_derivative_use() {
        let mut value = entry("cc-by-nc-nd");
        value.license = license(
            Some("CC-BY-NC-ND-4.0"),
            true,
            Some(false),
            Some(false),
            Some(true),
        );

        assert_eq!(value.decide_use(request(true, false, false)), DatasetUseDecision::Denied);
        assert_eq!(value.decide_use(request(false, true, false)), DatasetUseDecision::Denied);
        assert_eq!(value.decide_use(request(false, false, true)), DatasetUseDecision::Allowed);
    }

    #[test]
    fn unknown_licence_blocks_reuse_instead_of_assuming_permission() {
        let mut value = entry("unknown-rights");
        value.license = license(None, false, None, None, None);

        assert_eq!(value.decide_use(request(true, false, false)), DatasetUseDecision::Unknown);
        assert_eq!(value.decide_use(request(false, false, true)), DatasetUseDecision::Unknown);
    }

    #[test]
    fn access_controlled_dataset_needs_separate_access_grant() {
        let mut value = entry("globus");
        value.availability = DatasetAvailability::AccessControlled;
        value.access_granted = false;
        value.license = license(None, false, None, None, None);

        assert_eq!(value.decide_use(request(false, false, false)), DatasetUseDecision::Unavailable);
        value.access_granted = true;
        assert_eq!(value.decide_use(request(false, false, false)), DatasetUseDecision::Unknown);
    }

    #[test]
    fn pending_release_and_planned_projects_are_not_available_datasets() {
        let mut pending = entry("preprint");
        pending.availability = DatasetAvailability::PendingRelease;
        pending.access_granted = false;
        assert_eq!(pending.decide_use(request(false, false, false)), DatasetUseDecision::Unavailable);

        let mut planned = entry("planned");
        planned.availability = DatasetAvailability::Planned;
        planned.access_granted = false;
        assert_eq!(planned.decide_use(request(false, false, false)), DatasetUseDecision::Unavailable);
    }

    #[test]
    fn slow_sampled_workload_trace_has_correct_conservative_nyquist() {
        let mut value = entry("50-hz");
        value.maximum_sample_interval_ns = Some(20_000_000);
        assert!((value.conservative_nyquist_hz().unwrap() - 25.0).abs() < 1.0e-12);
        value.maximum_sample_interval_ns = Some(100_000_000);
        assert!((value.conservative_nyquist_hz().unwrap() - 5.0).abs() < 1.0e-12);
        value.maximum_sample_interval_ns = Some(200_000_000);
        assert!((value.conservative_nyquist_hz().unwrap() - 2.5).abs() < 1.0e-12);
    }

    #[test]
    fn unknown_sensor_bandwidth_cannot_support_fast_transient_claim() {
        let mut value = entry("high-rate-description-only");
        value.maximum_sample_interval_ns = None;
        value.supported_uses.push(DatasetAnalysisUse::FastElectricalTransient);

        assert!(validate_dataset_registry(&[value]).is_err());
    }

    #[test]
    fn a_fast_transient_claim_requires_physical_sample_and_filter_evidence() {
        let mut value = entry("fast-valid");
        value.maximum_sample_interval_ns = Some(10);
        value.sensor_bandwidth_hz = Some(100_000_000.0);
        value.anti_alias_filter_verified = Some(true);
        value.fast_claim_bandwidth_hz = Some(10_000_000.0);
        value.supported_uses.push(DatasetAnalysisUse::FastElectricalTransient);

        assert!(value.supports_fast_transient_claim());
        assert!(validate_dataset_registry(&[value]).is_ok());
    }

    #[test]
    fn nlr_measurement_and_facility_model_must_remain_different_evidence_classes() {
        let measured = entry("nlr-measured");
        let mut modelled = entry("nlr-modelled-facility");
        modelled.evidence_class = DatasetEvidenceClass::ModelledFacility;

        assert_ne!(measured.evidence_class, modelled.evidence_class);
        assert!(validate_dataset_registry(&[measured, modelled]).is_ok());
    }

    #[test]
    fn rejects_duplicate_ids_and_unverified_permission_claims() {
        assert!(validate_dataset_registry(&[entry("duplicate"), entry("duplicate")]).is_err());

        let mut invalid = entry("unverified-but-permitted");
        invalid.license = license(None, false, Some(true), None, None);
        assert!(validate_dataset_registry(&[invalid]).is_err());
    }

    #[test]
    fn rejects_planned_dataset_with_access_granted_and_invalid_bandwidth() {
        let mut planned = entry("planned-access");
        planned.availability = DatasetAvailability::Planned;
        assert!(validate_dataset_registry(&[planned]).is_err());

        let mut invalid_bandwidth = entry("invalid-bandwidth");
        invalid_bandwidth.sensor_bandwidth_hz = Some(f64::NAN);
        assert!(validate_dataset_registry(&[invalid_bandwidth]).is_err());
    }
}