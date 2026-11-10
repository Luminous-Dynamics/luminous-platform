use crate::AuditError;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    Synthetic,
    Assumption,
    VendorReported,
    Analytical,
    Simulation,
    CorrelatedSimulation,
    Hil,
    PhysicalMeasurement,
    IndependentLabTest,
    Certification,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationRevision {
    pub item_id: String,
    pub revision: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementContext {
    pub procedure_revision: String,
    pub calibration_reference: String,
    pub instrument_ids: Vec<String>,
    pub instrument_bandwidth_hz: f64,
    pub reference_plane_id: String,
    pub clock_domain_id: String,
    pub clock_sync_uncertainty_ns: u64,
    pub maximum_allowed_clock_sync_uncertainty_ns: u64,
    pub deembedding_method: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationContext {
    pub model_id: String,
    pub model_revision: String,
    pub solver_id: String,
    pub solver_version: String,
    pub parameter_set_sha256: String,
    pub boundary_conditions_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalAttestation {
    pub issuer_id: String,
    pub organization: String,
    pub report_reference: String,
    pub credential_reference: Option<String>,
}

/// Versioned evidence metadata. This type does not parse JSON or authenticate
/// signers. SHA-256 values are syntax-checked and can be compared against a
/// digest computed by a separately reviewed hashing implementation.
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceManifest {
    pub schema_version: String,
    pub evidence_id: String,
    pub evidence_kind: EvidenceKind,
    pub source_reference: String,
    /// RFC 3339 UTC timestamp ending in Z; full calendar parsing is delegated
    /// to the JSON Schema/date-time consumer.
    pub created_at_utc: String,
    pub artifact_path: String,
    pub artifact_sha256: String,
    pub configuration_id: String,
    pub configuration_revisions: Vec<ConfigurationRevision>,
    pub claim_ceiling: String,
    /// Evidence IDs whose correctness is required by this evidence item.
    pub dependencies: Vec<String>,
    pub measurement_context: Option<MeasurementContext>,
    pub simulation_context: Option<SimulationContext>,
    /// Present only for independent-lab or certification evidence. Its
    /// presence does not authenticate the issuer or report.
    pub external_attestation: Option<ExternalAttestation>,
}

impl EvidenceManifest {
    pub const fn claim_ceiling_for_metadata(&self) -> &'static str {
        "MANIFEST_METADATA_VALIDATED_ONLY_NOT_RAW_HASH_VERIFIED_NOT_AUTHENTICATED_NOT_QUALIFICATION"
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_nonblank(value: &str, field: &str) -> Result<(), AuditError> {
    if value.trim().is_empty() {
        return Err(AuditError(format!(
            "evidence manifest {field} must not be blank"
        )));
    }
    Ok(())
}

/// Validate the typed metadata and evidence-class-specific requirements.
///
/// This validates declared values only. It does not open artifact_path, hash
/// file bytes, parse calibration certificates, validate signatures, or
/// independently establish an issuer's authority.
pub fn validate_evidence_manifest(manifest: &EvidenceManifest) -> Result<(), AuditError> {
    if manifest.schema_version != "luminous-evidence-manifest-v0.1" {
        return Err(AuditError(
            "unsupported evidence manifest schema_version".to_string(),
        ));
    }
    require_nonblank(&manifest.evidence_id, "evidence_id")?;
    require_nonblank(&manifest.source_reference, "source_reference")?;
    require_nonblank(&manifest.created_at_utc, "created_at_utc")?;
    require_nonblank(&manifest.artifact_path, "artifact_path")?;
    require_nonblank(&manifest.configuration_id, "configuration_id")?;
    require_nonblank(&manifest.claim_ceiling, "claim_ceiling")?;

    if !manifest.created_at_utc.contains('T') || !manifest.created_at_utc.ends_with('Z') {
        return Err(AuditError(
            "created_at_utc must be an RFC 3339 UTC timestamp ending in Z".to_string(),
        ));
    }
    if !is_sha256_hex(&manifest.artifact_sha256) {
        return Err(AuditError(
            "artifact_sha256 must be exactly 64 lowercase hexadecimal characters".to_string(),
        ));
    }
    if manifest.configuration_revisions.is_empty() {
        return Err(AuditError(
            "at least one configuration revision must be declared".to_string(),
        ));
    }

    let mut configuration_items = HashSet::new();
    for revision in &manifest.configuration_revisions {
        require_nonblank(&revision.item_id, "configuration_revisions.item_id")?;
        require_nonblank(&revision.revision, "configuration_revisions.revision")?;
        require_nonblank(&revision.role, "configuration_revisions.role")?;
        if !configuration_items.insert(revision.item_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate configuration item {:?}",
                revision.item_id
            )));
        }
    }

    let mut dependency_ids = HashSet::new();
    for dependency in &manifest.dependencies {
        require_nonblank(dependency, "dependencies[]")?;
        if dependency == &manifest.evidence_id {
            return Err(AuditError(
                "evidence manifest cannot depend on itself".to_string(),
            ));
        }
        if !dependency_ids.insert(dependency.as_str()) {
            return Err(AuditError(format!(
                "duplicate evidence dependency {dependency:?}"
            )));
        }
    }

    match manifest.evidence_kind {
        EvidenceKind::PhysicalMeasurement => {
            let measurement = manifest.measurement_context.as_ref().ok_or_else(|| {
                AuditError(
                    "physical measurement evidence requires measurement_context".to_string(),
                )
            })?;
            require_nonblank(
                &measurement.procedure_revision,
                "measurement_context.procedure_revision",
            )?;
            require_nonblank(
                &measurement.calibration_reference,
                "measurement_context.calibration_reference",
            )?;
            require_nonblank(
                &measurement.reference_plane_id,
                "measurement_context.reference_plane_id",
            )?;
            require_nonblank(
                &measurement.clock_domain_id,
                "measurement_context.clock_domain_id",
            )?;
            if measurement.instrument_ids.is_empty()
                || measurement
                    .instrument_ids
                    .iter()
                    .any(|instrument| instrument.trim().is_empty())
                || !measurement.instrument_bandwidth_hz.is_finite()
                || measurement.instrument_bandwidth_hz <= 0.0
                || measurement.clock_sync_uncertainty_ns
                    > measurement.maximum_allowed_clock_sync_uncertainty_ns
            {
                return Err(AuditError(
                    "physical measurement context has invalid instrument, bandwidth or clock metadata"
                        .to_string(),
                ));
            }
            let mut instruments = HashSet::new();
            if measurement
                .instrument_ids
                .iter()
                .any(|instrument| !instruments.insert(instrument.as_str()))
            {
                return Err(AuditError(
                    "physical measurement instrument IDs must be unique".to_string(),
                ));
            }
        }
        EvidenceKind::Simulation | EvidenceKind::CorrelatedSimulation => {
            let simulation = manifest.simulation_context.as_ref().ok_or_else(|| {
                AuditError("simulation evidence requires simulation_context".to_string())
            })?;
            require_nonblank(&simulation.model_id, "simulation_context.model_id")?;
            require_nonblank(
                &simulation.model_revision,
                "simulation_context.model_revision",
            )?;
            require_nonblank(&simulation.solver_id, "simulation_context.solver_id")?;
            require_nonblank(
                &simulation.solver_version,
                "simulation_context.solver_version",
            )?;
            if !is_sha256_hex(&simulation.parameter_set_sha256)
                || !is_sha256_hex(&simulation.boundary_conditions_sha256)
            {
                return Err(AuditError(
                    "simulation parameter and boundary-condition digests must be SHA-256 hex"
                        .to_string(),
                ));
            }
        }
        EvidenceKind::IndependentLabTest | EvidenceKind::Certification => {
            let attestation = manifest.external_attestation.as_ref().ok_or_else(|| {
                AuditError(
                    "independent-lab or certification evidence requires external_attestation"
                        .to_string(),
                )
            })?;
            require_nonblank(&attestation.issuer_id, "external_attestation.issuer_id")?;
            require_nonblank(
                &attestation.organization,
                "external_attestation.organization",
            )?;
            require_nonblank(
                &attestation.report_reference,
                "external_attestation.report_reference",
            )?;
        }
        EvidenceKind::Synthetic
        | EvidenceKind::Assumption
        | EvidenceKind::VendorReported
        | EvidenceKind::Analytical
        | EvidenceKind::Hil => {}
    }

    Ok(())
}

/// Compare a digest computed by an external SHA-256 implementation with the
/// manifest declaration. This function does not hash the artifact itself.
pub fn compare_computed_artifact_digest(
    manifest: &EvidenceManifest,
    computed_sha256: &str,
) -> Result<bool, AuditError> {
    validate_evidence_manifest(manifest)?;
    if !is_sha256_hex(computed_sha256) {
        return Err(AuditError(
            "computed artifact digest must be 64 lowercase hexadecimal characters".to_string(),
        ));
    }
    Ok(manifest.artifact_sha256 == computed_sha256)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationRevisionChange {
    pub item_id: String,
    pub new_revision: String,
}

/// Validate dependency references and ensure the evidence graph is acyclic.
pub fn validate_evidence_graph(manifests: &[EvidenceManifest]) -> Result<(), AuditError> {
    let mut by_id: HashMap<&str, &EvidenceManifest> = HashMap::new();
    for manifest in manifests {
        validate_evidence_manifest(manifest)?;
        if by_id
            .insert(manifest.evidence_id.as_str(), manifest)
            .is_some()
        {
            return Err(AuditError(format!(
                "duplicate evidence ID {:?}",
                manifest.evidence_id
            )));
        }
    }

    let mut indegree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
    for manifest in manifests {
        indegree.insert(
            manifest.evidence_id.as_str(),
            manifest.dependencies.len(),
        );
    }
    for manifest in manifests {
        for dependency in &manifest.dependencies {
            if !by_id.contains_key(dependency.as_str()) {
                return Err(AuditError(format!(
                    "evidence {:?} references missing dependency {:?}",
                    manifest.evidence_id, dependency
                )));
            }
            dependents
                .entry(dependency.as_str())
                .or_default()
                .push(manifest.evidence_id.as_str());
        }
    }

    let mut ready: VecDeque<&str> = indegree
        .iter()
        .filter_map(|(evidence_id, degree)| (*degree == 0).then_some(*evidence_id))
        .collect();
    let mut processed = 0_usize;
    while let Some(evidence_id) = ready.pop_front() {
        processed += 1;
        if let Some(children) = dependents.get(evidence_id) {
            for child in children {
                let degree = indegree.get_mut(child).ok_or_else(|| {
                    AuditError("evidence graph indegree entry is missing".to_string())
                })?;
                *degree -= 1;
                if *degree == 0 {
                    ready.push_back(child);
                }
            }
        }
    }
    if processed != manifests.len() {
        return Err(AuditError(
            "evidence dependency graph contains a cycle".to_string(),
        ));
    }
    Ok(())
}

/// Return the sorted transitive evidence closure requiring requalification
/// when one configuration item changes to new_revision.
///
/// Evidence pinned to the new revision remains eligible. Evidence pinned to
/// any other revision is included, and dependent evidence is added transitively.
/// Unrelated components and unrelated evidence remain outside the closure.
pub fn requalification_closure(
    manifests: &[EvidenceManifest],
    change: &ConfigurationRevisionChange,
) -> Result<Vec<String>, AuditError> {
    require_nonblank(&change.item_id, "configuration change item_id")?;
    require_nonblank(&change.new_revision, "configuration change new_revision")?;
    validate_evidence_graph(manifests)?;

    let mut affected: HashSet<String> = manifests
        .iter()
        .filter(|manifest| {
            manifest.configuration_revisions.iter().any(|revision| {
                revision.item_id == change.item_id && revision.revision != change.new_revision
            })
        })
        .map(|manifest| manifest.evidence_id.clone())
        .collect();

    loop {
        let newly_affected: Vec<String> = manifests
            .iter()
            .filter(|manifest| !affected.contains(&manifest.evidence_id))
            .filter(|manifest| {
                manifest
                    .dependencies
                    .iter()
                    .any(|dependency| affected.contains(dependency))
            })
            .map(|manifest| manifest.evidence_id.clone())
            .collect();
        if newly_affected.is_empty() {
            break;
        }
        affected.extend(newly_affected);
    }

    let mut result: Vec<String> = affected.into_iter().collect();
    result.sort();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{
        compare_computed_artifact_digest, requalification_closure,
        validate_evidence_graph, validate_evidence_manifest,
        ConfigurationRevision, ConfigurationRevisionChange, EvidenceKind,
        EvidenceManifest, ExternalAttestation, MeasurementContext, SimulationContext,
    };

    fn revision(item_id: &str, revision: &str) -> ConfigurationRevision {
        ConfigurationRevision {
            item_id: item_id.to_string(),
            revision: revision.to_string(),
            role: "test-configuration".to_string(),
        }
    }

    fn manifest(id: &str, kind: EvidenceKind) -> EvidenceManifest {
        EvidenceManifest {
            schema_version: "luminous-evidence-manifest-v0.1".to_string(),
            evidence_id: id.to_string(),
            evidence_kind: kind,
            source_reference: "test-fixture".to_string(),
            created_at_utc: "2026-11-10T00:00:00Z".to_string(),
            artifact_path: format!("artifacts/{id}.bin"),
            artifact_sha256: "a".repeat(64),
            configuration_id: "config-001".to_string(),
            configuration_revisions: vec![revision("board", "rev-a")],
            claim_ceiling: "TEST_FIXTURE_ONLY".to_string(),
            dependencies: Vec::new(),
            measurement_context: None,
            simulation_context: None,
            external_attestation: None,
        }
    }

    fn physical_manifest(id: &str) -> EvidenceManifest {
        let mut value = manifest(id, EvidenceKind::PhysicalMeasurement);
        value.measurement_context = Some(MeasurementContext {
            procedure_revision: "procedure-v1".to_string(),
            calibration_reference: "calibration-record-001".to_string(),
            instrument_ids: vec!["daq-001".to_string(), "probe-001".to_string()],
            instrument_bandwidth_hz: 10_000_000.0,
            reference_plane_id: "package-sense-port".to_string(),
            clock_domain_id: "daq-master-001".to_string(),
            clock_sync_uncertainty_ns: 5,
            maximum_allowed_clock_sync_uncertainty_ns: 10,
            deembedding_method: Some("2x-thru-v1".to_string()),
        });
        value
    }

    fn simulated_manifest(id: &str) -> EvidenceManifest {
        let mut value = manifest(id, EvidenceKind::Simulation);
        value.simulation_context = Some(SimulationContext {
            model_id: "pdn-model-01".to_string(),
            model_revision: "git:abc123".to_string(),
            solver_id: "solver-x".to_string(),
            solver_version: "1.2.3".to_string(),
            parameter_set_sha256: "b".repeat(64),
            boundary_conditions_sha256: "c".repeat(64),
        });
        value
    }

    #[test]
    fn validates_physical_measurement_with_required_provenance_fields() {
        let value = physical_manifest("measurement-001");
        assert!(validate_evidence_manifest(&value).is_ok());
    }

    #[test]
    fn rejects_physical_measurement_without_calibration_or_context() {
        let missing_context = manifest("measurement-missing", EvidenceKind::PhysicalMeasurement);
        assert!(validate_evidence_manifest(&missing_context).is_err());

        let mut missing_calibration = physical_manifest("measurement-no-calibration");
        missing_calibration
            .measurement_context
            .as_mut()
            .unwrap()
            .calibration_reference = " ".to_string();
        assert!(validate_evidence_manifest(&missing_calibration).is_err());
    }

    #[test]
    fn compares_external_sha256_result_without_claiming_to_hash_the_file() {
        let value = manifest("digest-001", EvidenceKind::Synthetic);
        assert!(compare_computed_artifact_digest(&value, &"a".repeat(64)).unwrap());
        assert!(!compare_computed_artifact_digest(&value, &"b".repeat(64)).unwrap());
        assert!(compare_computed_artifact_digest(&value, "not-a-digest").is_err());
    }

    #[test]
    fn rejects_invalid_digest_and_duplicate_configuration_items() {
        let mut bad_digest = manifest("bad-digest", EvidenceKind::Synthetic);
        bad_digest.artifact_sha256 = "ABC".to_string();
        assert!(validate_evidence_manifest(&bad_digest).is_err());

        let mut duplicate = manifest("duplicate-config", EvidenceKind::Synthetic);
        duplicate.configuration_revisions.push(revision("board", "rev-b"));
        assert!(validate_evidence_manifest(&duplicate).is_err());
    }

    #[test]
    fn requires_simulation_parameters_and_external_test_attestation() {
        let mut no_simulation = manifest("sim-no-context", EvidenceKind::Simulation);
        assert!(validate_evidence_manifest(&no_simulation).is_err());

        let simulation = simulated_manifest("sim-valid");
        assert!(validate_evidence_manifest(&simulation).is_ok());

        no_simulation.external_attestation = Some(ExternalAttestation {
            issuer_id: "lab-accreditation-entity".to_string(),
            organization: "Independent Lab".to_string(),
            report_reference: "report-001".to_string(),
            credential_reference: None,
        });
        no_simulation.evidence_kind = EvidenceKind::IndependentLabTest;
        assert!(validate_evidence_manifest(&no_simulation).is_ok());
    }

    #[test]
    fn requalifies_affected_package_evidence_and_dependents_only() {
        let mut package_measurement = physical_manifest("package-measurement");
        package_measurement.configuration_revisions = vec![
            revision("interposer", "rev-a"),
            revision("board", "board-7"),
        ];

        let mut package_report = manifest("package-report", EvidenceKind::Analytical);
        package_report.configuration_revisions = vec![revision("board", "board-7")];
        package_report.dependencies = vec!["package-measurement".to_string()];

        let mut storage_report = manifest("storage-report", EvidenceKind::Analytical);
        storage_report.configuration_revisions = vec![revision("battery", "battery-3")];

        let result = requalification_closure(
            &[package_measurement, package_report, storage_report],
            &ConfigurationRevisionChange {
                item_id: "interposer".to_string(),
                new_revision: "rev-b".to_string(),
            },
        )
        .unwrap();

        assert_eq!(
            result,
            vec![
                "package-measurement".to_string(),
                "package-report".to_string()
            ]
        );
    }

    #[test]
    fn rejects_missing_dependency_and_dependency_cycles() {
        let mut missing = manifest("missing", EvidenceKind::Analytical);
        missing.dependencies = vec!["absent".to_string()];
        assert!(validate_evidence_graph(&[missing]).is_err());

        let mut left = manifest("left", EvidenceKind::Analytical);
        let mut right = manifest("right", EvidenceKind::Analytical);
        left.dependencies = vec!["right".to_string()];
        right.dependencies = vec!["left".to_string()];
        assert!(validate_evidence_graph(&[left, right]).is_err());
    }

    #[test]
    fn evidence_class_and_claim_ceiling_are_not_promoted_by_validation() {
        let value = manifest("synthetic-only", EvidenceKind::Synthetic);
        validate_evidence_manifest(&value).unwrap();

        assert_eq!(value.evidence_kind, EvidenceKind::Synthetic);
        assert_eq!(value.claim_ceiling, "TEST_FIXTURE_ONLY");
        assert_eq!(
            value.claim_ceiling_for_metadata(),
            "MANIFEST_METADATA_VALIDATED_ONLY_NOT_RAW_HASH_VERIFIED_NOT_AUTHENTICATED_NOT_QUALIFICATION"
        );
    }
}