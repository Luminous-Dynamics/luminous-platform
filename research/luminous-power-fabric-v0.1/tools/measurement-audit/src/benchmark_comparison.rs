use crate::capability_envelope::EvidenceStatus;
use crate::AuditError;

/// Provenance class for one benchmark run. These classes are deliberately not
/// interchangeable: a synthetic arithmetic oracle cannot establish a physical
/// SOTA claim, and a vendor specification is not a measured observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkEvidenceClass {
    VendorAssertion,
    Synthetic,
    Modelled,
    Measured,
    IndependentlyWitnessed,
    Accredited,
}

impl BenchmarkEvidenceClass {
    const fn is_physical_measurement(self) -> bool {
        matches!(
            self,
            Self::Measured | Self::IndependentlyWitnessed | Self::Accredited
        )
    }

    const fn is_independent(self) -> bool {
        matches!(self, Self::IndependentlyWitnessed | Self::Accredited)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkInstrumentEvidence {
    pub instrument_id: String,
    /// Digest of the calibrated measurand/range/channel scope, not the
    /// certificate itself. The certificate remains an evidence reference.
    pub calibration_scope_sha256: String,
    pub calibration_method_sha256: String,
    pub calibration_status: EvidenceStatus,
    pub calibration_valid_until_epoch_s: u64,
    pub calibration_evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkMetricObservation {
    pub metric_id: String,
    pub unit: String,
    pub value: f64,
    /// Absolute uncertainty bound in the metric's declared unit. This evaluator
    /// does not assume independence or calculate covariance.
    pub absolute_uncertainty_bound: f64,
    pub measurement_method_sha256: String,
    pub measurand_scope_sha256: String,
    pub instrument_ids: Vec<String>,
    pub evidence_status: EvidenceStatus,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkRun {
    pub run_id: String,
    pub evidence_class: BenchmarkEvidenceClass,
    pub product_configuration_sha256: String,
    pub test_protocol_sha256: String,
    pub service_boundary_sha256: String,
    pub service_profile_sha256: String,
    pub event_schedule_sha256: String,
    pub environment_profile_sha256: String,
    pub source_fuel_condition_sha256: String,
    pub initial_storage_condition_sha256: String,
    pub reserve_policy_sha256: String,
    pub data_treatment_policy_sha256: String,
    pub mission_duration_s: f64,
    pub reporting_window_start_s: f64,
    pub reporting_window_end_s: f64,
    pub sample_coverage_fraction: f64,
    pub minimum_required_sample_coverage_fraction: f64,
    pub missing_sample_count: u64,
    pub maximum_missing_sample_count: u64,
    pub raw_data_sha256: Option<String>,
    pub timebase_method_sha256: String,
    pub timebase_status: EvidenceStatus,
    pub timebase_valid_until_epoch_s: u64,
    pub timebase_evidence_reference: String,
    pub instruments: Vec<BenchmarkInstrumentEvidence>,
    pub metrics: Vec<BenchmarkMetricObservation>,
    pub independent_assessment_reference: Option<String>,
    pub data_rights_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchedMetricDelta {
    pub metric_id: String,
    pub unit: String,
    pub reference_value: f64,
    pub candidate_value: f64,
    /// Candidate minus reference. This is diagnostic only; no direction of
    /// "better" or superiority threshold is inferred.
    pub candidate_minus_reference: f64,
    /// Conservative first-order sum of the two absolute uncertainty bounds.
    /// A tighter combination requires a declared covariance/coverage model.
    pub conservative_combined_uncertainty_bound: f64,
    pub reference_evidence_reference: String,
    pub candidate_evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchedBenchmarkReport {
    pub reference_run_id: String,
    pub candidate_run_id: String,
    pub structural_match: bool,
    pub physical_measurement_eligible: bool,
    pub independent_claim_evidence_eligible: bool,
    pub blockers: Vec<String>,
    pub matched_metric_deltas: Vec<MatchedMetricDelta>,
}

impl MatchedBenchmarkReport {
    /// Eligibility is not a performance win, standards conformity or SOTA result.
    pub const fn claim_ceiling(&self) -> &'static str {
        "MATCHED_BENCHMARK_ELIGIBILITY_ONLY_NOT_SUPERIORITY_OR_CONFORMITY"
    }
}

pub struct MatchedBenchmarkInput<'a> {
    pub evaluation_time_epoch_s: u64,
    pub reference: &'a BenchmarkRun,
    pub candidate: &'a BenchmarkRun,
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn validate_run(run: &BenchmarkRun) -> Result<(), AuditError> {
    if !nonblank(&run.run_id) || !nonblank(&run.data_rights_reference) {
        return Err(AuditError(
            "benchmark run identity and data-rights reference must be non-empty".to_string(),
        ));
    }

    let digests = [
        ("product configuration", run.product_configuration_sha256.as_str()),
        ("test protocol", run.test_protocol_sha256.as_str()),
        ("service boundary", run.service_boundary_sha256.as_str()),
        ("service profile", run.service_profile_sha256.as_str()),
        ("event schedule", run.event_schedule_sha256.as_str()),
        ("environment profile", run.environment_profile_sha256.as_str()),
        ("source/fuel condition", run.source_fuel_condition_sha256.as_str()),
        ("initial storage condition", run.initial_storage_condition_sha256.as_str()),
        ("reserve policy", run.reserve_policy_sha256.as_str()),
        ("data treatment policy", run.data_treatment_policy_sha256.as_str()),
        ("timebase method", run.timebase_method_sha256.as_str()),
    ];
    for (label, digest) in digests {
        if !valid_sha256(digest) {
            return Err(AuditError(format!(
                "{label} digest must be 64 hexadecimal SHA-256 characters"
            )));
        }
    }

    if !run.mission_duration_s.is_finite()
        || run.mission_duration_s <= 0.0
        || !run.reporting_window_start_s.is_finite()
        || !run.reporting_window_end_s.is_finite()
        || run.reporting_window_start_s < 0.0
        || run.reporting_window_end_s <= run.reporting_window_start_s
        || run.reporting_window_end_s > run.mission_duration_s
    {
        return Err(AuditError(
            "mission duration or reporting window is invalid".to_string(),
        ));
    }

    if !run.sample_coverage_fraction.is_finite()
        || !(0.0..=1.0).contains(&run.sample_coverage_fraction)
        || !run.minimum_required_sample_coverage_fraction.is_finite()
        || run.minimum_required_sample_coverage_fraction <= 0.0
        || run.minimum_required_sample_coverage_fraction > 1.0
        || run.sample_coverage_fraction < run.minimum_required_sample_coverage_fraction
        || run.missing_sample_count > run.maximum_missing_sample_count
    {
        return Err(AuditError(
            "sample coverage or missing-sample acceptance is invalid".to_string(),
        ));
    }

    if run.evidence_class.is_physical_measurement() {
        let raw_digest = run.raw_data_sha256.as_deref().ok_or_else(|| {
            AuditError("physical benchmark run requires a raw-data SHA-256 digest".to_string())
        })?;
        if !valid_sha256(raw_digest) {
            return Err(AuditError(
                "physical benchmark raw-data digest must be SHA-256".to_string(),
            ));
        }
    } else if let Some(raw_digest) = run.raw_data_sha256.as_deref() {
        if !valid_sha256(raw_digest) {
            return Err(AuditError(
                "declared benchmark data digest must be SHA-256".to_string(),
            ));
        }
    }

    let mut instrument_ids: Vec<&str> = Vec::with_capacity(run.instruments.len());
    for instrument in &run.instruments {
        if !nonblank(&instrument.instrument_id)
            || !nonblank(&instrument.calibration_evidence_reference)
        {
            return Err(AuditError(
                "instrument identity and calibration reference must be non-empty".to_string(),
            ));
        }
        if !valid_sha256(&instrument.calibration_scope_sha256)
            || !valid_sha256(&instrument.calibration_method_sha256)
        {
            return Err(AuditError(
                "instrument calibration scope and method must be SHA-256 digests".to_string(),
            ));
        }
        if instrument_ids.contains(&instrument.instrument_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate instrument id {:?}",
                instrument.instrument_id
            )));
        }
        instrument_ids.push(&instrument.instrument_id);
    }

    let mut metric_ids: Vec<&str> = Vec::with_capacity(run.metrics.len());
    for metric in &run.metrics {
        if !nonblank(&metric.metric_id)
            || !nonblank(&metric.unit)
            || !nonblank(&metric.evidence_reference)
        {
            return Err(AuditError(
                "metric identity, unit and evidence reference must be non-empty".to_string(),
            ));
        }
        if !valid_sha256(&metric.measurement_method_sha256)
            || !valid_sha256(&metric.measurand_scope_sha256)
        {
            return Err(AuditError(
                "metric method and measurand-scope values must be SHA-256 digests".to_string(),
            ));
        }
        if !metric.value.is_finite()
            || !finite_nonnegative(metric.absolute_uncertainty_bound)
        {
            return Err(AuditError(format!(
                "metric {:?} value/uncertainty is invalid",
                metric.metric_id
            )));
        }
        if metric.instrument_ids.is_empty() {
            return Err(AuditError(format!(
                "metric {:?} must cite at least one instrument",
                metric.metric_id
            )));
        }
        if metric_ids.contains(&metric.metric_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate metric id {:?}",
                metric.metric_id
            )));
        }
        metric_ids.push(&metric.metric_id);
        for instrument_id in &metric.instrument_ids {
            if !instrument_ids.contains(&instrument_id.as_str()) {
                return Err(AuditError(format!(
                    "metric {:?} references unknown instrument {:?}",
                    metric.metric_id, instrument_id
                )));
            }
        }
    }

    Ok(())
}

fn compare_key(
    blockers: &mut Vec<String>,
    reference_value: &str,
    candidate_value: &str,
    key_name: &str,
) {
    if reference_value != candidate_value {
        blockers.push(format!("mismatch:{key_name}"));
    }
}

fn find_metric<'a>(run: &'a BenchmarkRun, metric_id: &str) -> Option<&'a BenchmarkMetricObservation> {
    run.metrics.iter().find(|metric| metric.metric_id == metric_id)
}

/// Evaluate whether two benchmark runs are structurally matched and carry
/// sufficient current physical measurement evidence for a fair comparison.
///
/// This function does not authenticate digests/certificates, assess standards
/// conformity, infer causal superiority, or calculate statistical significance.
pub fn compare_matched_benchmark_runs(
    input: MatchedBenchmarkInput<'_>,
) -> Result<MatchedBenchmarkReport, AuditError> {
    validate_run(input.reference)?;
    validate_run(input.candidate)?;

    if input.reference.run_id == input.candidate.run_id {
        return Err(AuditError(
            "reference and candidate must be distinct run identities".to_string(),
        ));
    }

    let reference = input.reference;
    let candidate = input.candidate;
    let mut blockers = Vec::new();

    compare_key(&mut blockers, &reference.test_protocol_sha256, &candidate.test_protocol_sha256, "test_protocol");
    compare_key(&mut blockers, &reference.service_boundary_sha256, &candidate.service_boundary_sha256, "service_boundary");
    compare_key(&mut blockers, &reference.service_profile_sha256, &candidate.service_profile_sha256, "service_profile");
    compare_key(&mut blockers, &reference.event_schedule_sha256, &candidate.event_schedule_sha256, "event_schedule");
    compare_key(&mut blockers, &reference.environment_profile_sha256, &candidate.environment_profile_sha256, "environment_profile");
    compare_key(&mut blockers, &reference.source_fuel_condition_sha256, &candidate.source_fuel_condition_sha256, "source_fuel_condition");
    compare_key(&mut blockers, &reference.initial_storage_condition_sha256, &candidate.initial_storage_condition_sha256, "initial_storage_condition");
    compare_key(&mut blockers, &reference.reserve_policy_sha256, &candidate.reserve_policy_sha256, "reserve_policy");
    compare_key(&mut blockers, &reference.data_treatment_policy_sha256, &candidate.data_treatment_policy_sha256, "data_treatment_policy");
    compare_key(&mut blockers, &reference.timebase_method_sha256, &candidate.timebase_method_sha256, "timebase_method");

    if reference.mission_duration_s != candidate.mission_duration_s {
        blockers.push("mismatch:mission_duration_s".to_string());
    }
    if reference.reporting_window_start_s != candidate.reporting_window_start_s
        || reference.reporting_window_end_s != candidate.reporting_window_end_s
    {
        blockers.push("mismatch:reporting_window".to_string());
    }
    if reference.minimum_required_sample_coverage_fraction
        != candidate.minimum_required_sample_coverage_fraction
        || reference.maximum_missing_sample_count != candidate.maximum_missing_sample_count
    {
        blockers.push("mismatch:data_completeness_policy".to_string());
    }

    let reference_is_physical = reference.evidence_class.is_physical_measurement();
    let candidate_is_physical = candidate.evidence_class.is_physical_measurement();
    if !reference_is_physical {
        blockers.push("reference_not_physical_measurement".to_string());
    }
    if !candidate_is_physical {
        blockers.push("candidate_not_physical_measurement".to_string());
    }

    for (role, run) in [("reference", reference), ("candidate", candidate)] {
        if run.timebase_status != EvidenceStatus::Current
            || run.timebase_valid_until_epoch_s <= input.evaluation_time_epoch_s
            || !nonblank(&run.timebase_evidence_reference)
        {
            blockers.push(format!("{role}_timebase_evidence_not_current"));
        }
        if run.instruments.is_empty() {
            blockers.push(format!("{role}_instrument_evidence_missing"));
        }
        for instrument in &run.instruments {
            if instrument.calibration_status != EvidenceStatus::Current
                || instrument.calibration_valid_until_epoch_s <= input.evaluation_time_epoch_s
            {
                blockers.push(format!(
                    "{role}_instrument_calibration_not_current:{}",
                    instrument.instrument_id
                ));
            }
        }
        for metric in &run.metrics {
            if metric.evidence_status != EvidenceStatus::Current {
                blockers.push(format!(
                    "{role}_metric_evidence_not_current:{}",
                    metric.metric_id
                ));
            }
        }
    }

    if reference.metrics.is_empty() || candidate.metrics.is_empty() {
        blockers.push("metric_observations_missing".to_string());
    }

    let mut matched_metric_deltas = Vec::new();
    for reference_metric in &reference.metrics {
        let Some(candidate_metric) = find_metric(candidate, &reference_metric.metric_id) else {
            blockers.push(format!(
                "candidate_metric_missing:{}",
                reference_metric.metric_id
            ));
            continue;
        };
        if reference_metric.unit != candidate_metric.unit {
            blockers.push(format!(
                "metric_unit_mismatch:{}",
                reference_metric.metric_id
            ));
            continue;
        }
        if reference_metric.measurement_method_sha256
            != candidate_metric.measurement_method_sha256
        {
            blockers.push(format!(
                "metric_method_mismatch:{}",
                reference_metric.metric_id
            ));
        }
        if reference_metric.measurand_scope_sha256 != candidate_metric.measurand_scope_sha256 {
            blockers.push(format!(
                "metric_measurand_scope_mismatch:{}",
                reference_metric.metric_id
            ));
        }

        let reference_instrument_scope_ok = reference_metric.instrument_ids.iter().all(|id| {
            reference.instruments.iter().any(|instrument| {
                instrument.instrument_id == *id
                    && instrument.calibration_scope_sha256
                        == reference_metric.measurand_scope_sha256
            })
        });
        let candidate_instrument_scope_ok = candidate_metric.instrument_ids.iter().all(|id| {
            candidate.instruments.iter().any(|instrument| {
                instrument.instrument_id == *id
                    && instrument.calibration_scope_sha256
                        == candidate_metric.measurand_scope_sha256
            })
        });
        if !reference_instrument_scope_ok {
            blockers.push(format!(
                "reference_calibration_scope_mismatch:{}",
                reference_metric.metric_id
            ));
        }
        if !candidate_instrument_scope_ok {
            blockers.push(format!(
                "candidate_calibration_scope_mismatch:{}",
                candidate_metric.metric_id
            ));
        }

        let delta = candidate_metric.value - reference_metric.value;
        let combined = candidate_metric.absolute_uncertainty_bound
            + reference_metric.absolute_uncertainty_bound;
        if !delta.is_finite() || !combined.is_finite() {
            return Err(AuditError(format!(
                "metric {:?} comparison arithmetic overflowed",
                reference_metric.metric_id
            )));
        }
        matched_metric_deltas.push(MatchedMetricDelta {
            metric_id: reference_metric.metric_id.clone(),
            unit: reference_metric.unit.clone(),
            reference_value: reference_metric.value,
            candidate_value: candidate_metric.value,
            candidate_minus_reference: delta,
            conservative_combined_uncertainty_bound: combined,
            reference_evidence_reference: reference_metric.evidence_reference.clone(),
            candidate_evidence_reference: candidate_metric.evidence_reference.clone(),
        });
    }

    for candidate_metric in &candidate.metrics {
        if find_metric(reference, &candidate_metric.metric_id).is_none() {
            blockers.push(format!(
                "reference_metric_missing:{}",
                candidate_metric.metric_id
            ));
        }
    }

    let structural_match = !blockers.iter().any(|blocker| {
        blocker.starts_with("mismatch:")
            || blocker.starts_with("candidate_metric_missing:")
            || blocker.starts_with("reference_metric_missing:")
            || blocker.starts_with("metric_unit_mismatch:")
            || blocker.starts_with("metric_method_mismatch:")
            || blocker.starts_with("metric_measurand_scope_mismatch:")
            || blocker.ends_with("_calibration_scope_mismatch")
    });
    let physical_measurement_eligible = structural_match && blockers.is_empty()
        && reference_is_physical && candidate_is_physical;

    let independent_assessment_present = reference
        .independent_assessment_reference
        .as_deref()
        .is_some_and(nonblank)
        && candidate
            .independent_assessment_reference
            .as_deref()
            .is_some_and(nonblank);
    let independent_evidence_class = reference.evidence_class.is_independent()
        && candidate.evidence_class.is_independent();
    let independent_claim_evidence_eligible = physical_measurement_eligible
        && independent_evidence_class
        && independent_assessment_present;

    Ok(MatchedBenchmarkReport {
        reference_run_id: reference.run_id.clone(),
        candidate_run_id: candidate.run_id.clone(),
        structural_match,
        physical_measurement_eligible,
        independent_claim_evidence_eligible,
        blockers,
        matched_metric_deltas,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        compare_matched_benchmark_runs, BenchmarkEvidenceClass, BenchmarkInstrumentEvidence,
        BenchmarkMetricObservation, BenchmarkRun, MatchedBenchmarkInput,
    };
    use crate::capability_envelope::EvidenceStatus;

    fn digest(byte: char) -> String {
        std::iter::repeat(byte).take(64).collect()
    }

    fn instrument(id: &str) -> BenchmarkInstrumentEvidence {
        BenchmarkInstrumentEvidence {
            instrument_id: id.to_string(),
            calibration_scope_sha256: digest('e'),
            calibration_method_sha256: digest('f'),
            calibration_status: EvidenceStatus::Current,
            calibration_valid_until_epoch_s: 1_000,
            calibration_evidence_reference: format!("mycelix:calibration-{id}"),
        }
    }

    fn metric(id: &str, value: f64, uncertainty: f64) -> BenchmarkMetricObservation {
        BenchmarkMetricObservation {
            metric_id: id.to_string(),
            unit: "Wh".to_string(),
            value,
            absolute_uncertainty_bound: uncertainty,
            measurement_method_sha256: digest('d'),
            measurand_scope_sha256: digest('e'),
            instrument_ids: vec!["meter-1".to_string()],
            evidence_status: EvidenceStatus::Current,
            evidence_reference: format!("mycelix:sample-{id}"),
        }
    }

    fn run(id: &str, class: BenchmarkEvidenceClass, config: char, value: f64) -> BenchmarkRun {
        BenchmarkRun {
            run_id: id.to_string(),
            evidence_class: class,
            product_configuration_sha256: digest(config),
            test_protocol_sha256: digest('a'),
            service_boundary_sha256: digest('b'),
            service_profile_sha256: digest('c'),
            event_schedule_sha256: digest('d'),
            environment_profile_sha256: digest('e'),
            source_fuel_condition_sha256: digest('f'),
            initial_storage_condition_sha256: digest('1'),
            reserve_policy_sha256: digest('2'),
            data_treatment_policy_sha256: digest('3'),
            mission_duration_s: 3_600.0,
            reporting_window_start_s: 0.0,
            reporting_window_end_s: 3_600.0,
            sample_coverage_fraction: 1.0,
            minimum_required_sample_coverage_fraction: 0.999,
            missing_sample_count: 0,
            maximum_missing_sample_count: 0,
            raw_data_sha256: Some(digest(if config == '4' { '4' } else { '5' })),
            timebase_method_sha256: digest('6'),
            timebase_status: EvidenceStatus::Current,
            timebase_valid_until_epoch_s: 1_000,
            timebase_evidence_reference: "mycelix:timebase-1".to_string(),
            instruments: vec![instrument("meter-1")],
            metrics: vec![metric("delivered_energy", value, 2.0)],
            independent_assessment_reference: if class.is_independent() {
                Some("independent:review-1".to_string())
            } else {
                None
            },
            data_rights_reference: "rights:fixture".to_string(),
        }
    }

    #[test]
    fn current_physical_runs_with_matching_conditions_are_eligible() {
        let reference = run("run-ref", BenchmarkEvidenceClass::Measured, '4', 10_000.0);
        let candidate = run("run-candidate", BenchmarkEvidenceClass::Measured, '5', 9_900.0);
        let report = compare_matched_benchmark_runs(MatchedBenchmarkInput {
            evaluation_time_epoch_s: 100,
            reference: &reference,
            candidate: &candidate,
        })
        .unwrap();
        assert!(report.structural_match);
        assert!(report.physical_measurement_eligible);
        assert!(!report.independent_claim_evidence_eligible);
        assert!(report.blockers.is_empty());
        assert_eq!(report.matched_metric_deltas[0].candidate_minus_reference, -100.0);
        assert_eq!(
            report.matched_metric_deltas[0].conservative_combined_uncertainty_bound,
            4.0
        );
    }

    #[test]
    fn different_product_configuration_is_expected_and_retained() {
        let reference = run("run-ref", BenchmarkEvidenceClass::Measured, '4', 10_000.0);
        let candidate = run("run-candidate", BenchmarkEvidenceClass::Measured, '5', 9_900.0);
        let report = compare_matched_benchmark_runs(MatchedBenchmarkInput {
            evaluation_time_epoch_s: 100,
            reference: &reference,
            candidate: &candidate,
        })
        .unwrap();
        assert!(report.physical_measurement_eligible);
        assert_ne!(
            reference.product_configuration_sha256,
            candidate.product_configuration_sha256
        );
    }

    #[test]
    fn a_mismatched_service_profile_blocks_comparison() {
        let reference = run("run-ref", BenchmarkEvidenceClass::Measured, '4', 10_000.0);
        let mut candidate = run("run-candidate", BenchmarkEvidenceClass::Measured, '5', 9_900.0);
        candidate.service_profile_sha256 = digest('9');
        let report = compare_matched_benchmark_runs(MatchedBenchmarkInput {
            evaluation_time_epoch_s: 100,
            reference: &reference,
            candidate: &candidate,
        })
        .unwrap();
        assert!(!report.structural_match);
        assert!(!report.physical_measurement_eligible);
        assert!(report.blockers.contains(&"mismatch:service_profile".to_string()));
    }

    #[test]
    fn a_synthetic_fixture_cannot_establish_physical_superiority() {
        let reference = run("run-ref", BenchmarkEvidenceClass::Synthetic, '4', 10_000.0);
        let candidate = run("run-candidate", BenchmarkEvidenceClass::Synthetic, '5', 9_900.0);
        let report = compare_matched_benchmark_runs(MatchedBenchmarkInput {
            evaluation_time_epoch_s: 100,
            reference: &reference,
            candidate: &candidate,
        })
        .unwrap();
        assert!(report.structural_match);
        assert!(!report.physical_measurement_eligible);
        assert!(!report.independent_claim_evidence_eligible);
        assert!(report.blockers.contains(&"reference_not_physical_measurement".to_string()));
    }

    #[test]
    fn expired_calibration_blocks_physical_comparison() {
        let reference = run("run-ref", BenchmarkEvidenceClass::Measured, '4', 10_000.0);
        let mut candidate = run("run-candidate", BenchmarkEvidenceClass::Measured, '5', 9_900.0);
        candidate.instruments[0].calibration_valid_until_epoch_s = 100;
        let report = compare_matched_benchmark_runs(MatchedBenchmarkInput {
            evaluation_time_epoch_s: 100,
            reference: &reference,
            candidate: &candidate,
        })
        .unwrap();
        assert!(!report.physical_measurement_eligible);
        assert!(report.blockers.iter().any(|item| item.starts_with("candidate_instrument_calibration_not_current")));
    }

    #[test]
    fn missing_raw_data_digest_is_rejected_for_physical_runs() {
        let mut reference = run("run-ref", BenchmarkEvidenceClass::Measured, '4', 10_000.0);
        reference.raw_data_sha256 = None;
        let candidate = run("run-candidate", BenchmarkEvidenceClass::Measured, '5', 9_900.0);
        assert!(compare_matched_benchmark_runs(MatchedBenchmarkInput {
            evaluation_time_epoch_s: 100,
            reference: &reference,
            candidate: &candidate,
        })
        .is_err());
    }

    #[test]
    fn inadequate_sample_coverage_is_rejected() {
        let mut reference = run("run-ref", BenchmarkEvidenceClass::Measured, '4', 10_000.0);
        reference.sample_coverage_fraction = 0.9;
        let candidate = run("run-candidate", BenchmarkEvidenceClass::Measured, '5', 9_900.0);
        assert!(compare_matched_benchmark_runs(MatchedBenchmarkInput {
            evaluation_time_epoch_s: 100,
            reference: &reference,
            candidate: &candidate,
        })
        .is_err());
    }

    #[test]
    fn timebase_and_instrument_evidence_are_required_for_physical_runs() {
        let reference = run("run-ref", BenchmarkEvidenceClass::Measured, '4', 10_000.0);
        let mut candidate = run("run-candidate", BenchmarkEvidenceClass::Measured, '5', 9_900.0);
        candidate.timebase_status = EvidenceStatus::Unverified;
        candidate.instruments.clear();
        let report = compare_matched_benchmark_runs(MatchedBenchmarkInput {
            evaluation_time_epoch_s: 100,
            reference: &reference,
            candidate: &candidate,
        })
        .unwrap();
        assert!(!report.physical_measurement_eligible);
        assert!(report.blockers.contains(&"candidate_timebase_evidence_not_current".to_string()));
        assert!(report.blockers.contains(&"candidate_instrument_evidence_missing".to_string()));
    }

    #[test]
    fn mismatched_measurement_method_blocks_comparison() {
        let reference = run("run-ref", BenchmarkEvidenceClass::Measured, '4', 10_000.0);
        let mut candidate = run("run-candidate", BenchmarkEvidenceClass::Measured, '5', 9_900.0);
        candidate.metrics[0].measurement_method_sha256 = digest('8');
        let report = compare_matched_benchmark_runs(MatchedBenchmarkInput {
            evaluation_time_epoch_s: 100,
            reference: &reference,
            candidate: &candidate,
        })
        .unwrap();
        assert!(!report.structural_match);
        assert!(report.blockers.contains(&"metric_method_mismatch:delivered_energy".to_string()));
    }

    #[test]
    fn independent_claim_requires_independent_review_and_evidence_class() {
        let reference = run("run-ref", BenchmarkEvidenceClass::IndependentlyWitnessed, '4', 10_000.0);
        let candidate = run("run-candidate", BenchmarkEvidenceClass::Accredited, '5', 9_900.0);
        let report = compare_matched_benchmark_runs(MatchedBenchmarkInput {
            evaluation_time_epoch_s: 100,
            reference: &reference,
            candidate: &candidate,
        })
        .unwrap();
        assert!(report.physical_measurement_eligible);
        assert!(report.independent_claim_evidence_eligible);
    }

    #[test]
    fn duplicate_metric_ids_are_rejected() {
        let mut reference = run("run-ref", BenchmarkEvidenceClass::Measured, '4', 10_000.0);
        reference.metrics.push(reference.metrics[0].clone());
        let candidate = run("run-candidate", BenchmarkEvidenceClass::Measured, '5', 9_900.0);
        assert!(compare_matched_benchmark_runs(MatchedBenchmarkInput {
            evaluation_time_epoch_s: 100,
            reference: &reference,
            candidate: &candidate,
        })
        .is_err());
    }

    #[test]
    fn a_metric_without_its_instrument_is_rejected() {
        let mut reference = run("run-ref", BenchmarkEvidenceClass::Measured, '4', 10_000.0);
        reference.metrics[0].instrument_ids = vec!["missing-meter".to_string()];
        let candidate = run("run-candidate", BenchmarkEvidenceClass::Measured, '5', 9_900.0);
        assert!(compare_matched_benchmark_runs(MatchedBenchmarkInput {
            evaluation_time_epoch_s: 100,
            reference: &reference,
            candidate: &candidate,
        })
        .is_err());
    }

    #[test]
    fn expired_timebase_blocks_physical_comparison() {
        let reference = run("run-ref", BenchmarkEvidenceClass::Measured, '4', 10_000.0);
        let mut candidate = run("run-candidate", BenchmarkEvidenceClass::Measured, '5', 9_900.0);
        candidate.timebase_valid_until_epoch_s = 100;
        let report = compare_matched_benchmark_runs(MatchedBenchmarkInput {
            evaluation_time_epoch_s: 100,
            reference: &reference,
            candidate: &candidate,
        })
        .unwrap();
        assert!(!report.physical_measurement_eligible);
        assert!(report.blockers.contains(&"candidate_timebase_evidence_not_current".to_string()));
    }

    #[test]
    fn claim_ceiling_never_claims_sota() {
        let reference = run("run-ref", BenchmarkEvidenceClass::Measured, '4', 10_000.0);
        let candidate = run("run-candidate", BenchmarkEvidenceClass::Measured, '5', 9_900.0);
        let report = compare_matched_benchmark_runs(MatchedBenchmarkInput {
            evaluation_time_epoch_s: 100,
            reference: &reference,
            candidate: &candidate,
        })
        .unwrap();
        assert_eq!(
            report.claim_ceiling(),
            "MATCHED_BENCHMARK_ELIGIBILITY_ONLY_NOT_SUPERIORITY_OR_CONFORMITY"
        );
    }
}
