use crate::capability_envelope::EvidenceStatus;
use crate::lifecycle_service_cost::LifecycleEvidenceClass;
use crate::AuditError;

const CLAIM: &str = "CONSERVATIVE_METROLOGY_INTERVAL_SCREEN_ONLY_NOT_GUM_EVALUATION_OR_CONFORMITY_DECISION";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementUnit {
    Watt,
    WattHour,
    MinorCurrencyUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UncertaintyComponentKind {
    InstrumentAccuracy,
    CalibrationCorrection,
    Resolution,
    Repeatability,
    Drift,
    CurrentTransformerRatio,
    VoltageTransformerRatio,
    TimebaseIntegration,
    TemperatureInfluence,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonDirection {
    LowerIsBetter,
    HigherIsBetter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarginDecision {
    ClearsPreregisteredMargin,
    DoesNotClearMargin,
    OverlapsDecisionBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceEligibility {
    IndependentPhysicalMeasurement,
    ModelOrVendorEvidence,
    SyntheticOrAssumption,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AbsoluteUncertaintyComponent {
    pub component_id: String,
    /// Guaranteed absolute bound in the measurement's exact unit.
    /// This is not a standard deviation or a standard uncertainty.
    pub absolute_bound: f64,
    pub unit: MeasurementUnit,
    pub kind: UncertaintyComponentKind,
    pub evidence_status: EvidenceStatus,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeterMeasurement {
    pub comparison_set_id: String,
    pub measurand_id: String,
    /// Exact physical POC/boundary identity. Ratios may join distinct POCs.
    pub boundary_id: String,
    pub instrument_id: String,
    pub channel_id: String,
    pub scope_sha256: String,
    pub configuration_sha256: String,
    pub value: f64,
    pub unit: MeasurementUnit,
    pub observed_at_epoch_s: u64,
    pub evaluation_time_epoch_s: u64,
    pub calibration_id: String,
    pub calibration_valid_until_epoch_s: u64,
    pub calibration_status: EvidenceStatus,
    pub timebase_reference: String,
    pub timebase_status: EvidenceStatus,
    pub evidence_status: EvidenceStatus,
    pub evidence_class: LifecycleEvidenceClass,
    pub evidence_reference: String,
    pub uncertainty_components: Vec<AbsoluteUncertaintyComponent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundedMeasurement {
    pub comparison_set_id: String,
    pub measurand_id: String,
    pub boundary_id: String,
    pub instrument_id: String,
    pub channel_id: String,
    pub scope_sha256: String,
    pub configuration_sha256: String,
    pub value: f64,
    pub absolute_bound: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub unit: MeasurementUnit,
    pub evidence_class: LifecycleEvidenceClass,
    pub evidence_references: Vec<String>,
    pub propagation_method: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundedRatio {
    pub comparison_set_id: String,
    pub scope_sha256: String,
    pub numerator_boundary_id: String,
    pub denominator_boundary_id: String,
    pub value: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub evidence_eligibility: EvidenceEligibility,
    pub propagation_method: &'static str,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundedPairedDifference {
    pub comparison_set_id: String,
    pub measurand_id: String,
    pub boundary_id: String,
    pub scope_sha256: String,
    /// Candidate minus reference.
    pub point_delta: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub preregistered_margin: f64,
    pub direction: ComparisonDirection,
    pub margin_decision: MarginDecision,
    pub evidence_eligibility: EvidenceEligibility,
    pub physical_superiority_claim_eligible: bool,
    pub propagation_method: &'static str,
    pub claim_ceiling: &'static str,
}

fn nonblank(value: &str) -> bool { !value.trim().is_empty() }

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn finite_nonnegative(value: f64) -> bool { value.is_finite() && value >= 0.0 }

fn evidence_eligibility(classes: &[LifecycleEvidenceClass]) -> EvidenceEligibility {
    if classes.iter().any(|class| matches!(
        class,
        LifecycleEvidenceClass::SyntheticFixture | LifecycleEvidenceClass::Assumption
    )) {
        EvidenceEligibility::SyntheticOrAssumption
    } else if classes.iter().all(|class| *class == LifecycleEvidenceClass::IndependentPhysicalMeasurement) {
        EvidenceEligibility::IndependentPhysicalMeasurement
    } else {
        EvidenceEligibility::ModelOrVendorEvidence
    }
}

/// Builds a conservative rectangular interval by summing declared absolute
/// error bounds linearly. No independence, Gaussian, RSS or covariance
/// assumption is made or inferred by this function.
pub fn bound_measurement(measurement: &MeterMeasurement) -> Result<BoundedMeasurement, AuditError> {
    if !nonblank(&measurement.comparison_set_id)
        || !nonblank(&measurement.measurand_id)
        || !nonblank(&measurement.boundary_id)
        || !nonblank(&measurement.instrument_id)
        || !nonblank(&measurement.channel_id)
        || !nonblank(&measurement.calibration_id)
        || !nonblank(&measurement.timebase_reference)
        || !nonblank(&measurement.evidence_reference)
        || !valid_sha256(&measurement.scope_sha256)
        || !valid_sha256(&measurement.configuration_sha256)
        || !finite_nonnegative(measurement.value)
    {
        return Err(AuditError("measurement identity, scope, digest or point value is invalid".to_string()));
    }
    if measurement.evidence_status != EvidenceStatus::Current
        || measurement.calibration_status != EvidenceStatus::Current
        || measurement.timebase_status != EvidenceStatus::Current
        || measurement.calibration_valid_until_epoch_s <= measurement.evaluation_time_epoch_s
        || measurement.observed_at_epoch_s > measurement.evaluation_time_epoch_s
    {
        return Err(AuditError("measurement, calibration or timebase evidence is stale, invalid or future-dated".to_string()));
    }
    if measurement.uncertainty_components.is_empty() {
        return Err(AuditError("measurement requires at least one explicit uncertainty component".to_string()));
    }

    let mut ids = std::collections::BTreeSet::new();
    let mut absolute_bound = 0.0_f64;
    let mut references = vec![
        measurement.evidence_reference.clone(),
        measurement.calibration_id.clone(),
        measurement.timebase_reference.clone(),
    ];
    for component in &measurement.uncertainty_components {
        if !nonblank(&component.component_id)
            || !nonblank(&component.evidence_reference)
            || !finite_nonnegative(component.absolute_bound)
            || component.unit != measurement.unit
            || component.evidence_status != EvidenceStatus::Current
            || !ids.insert(component.component_id.as_str())
        {
            return Err(AuditError("uncertainty component is duplicate, stale, unit-mismatched or invalid".to_string()));
        }
        absolute_bound += component.absolute_bound;
        if !absolute_bound.is_finite() {
            return Err(AuditError("conservative uncertainty bound overflowed".to_string()));
        }
        references.push(component.evidence_reference.clone());
    }

    let lower_bound = (measurement.value - absolute_bound).max(0.0);
    let upper_bound = measurement.value + absolute_bound;
    if !lower_bound.is_finite() || !upper_bound.is_finite() {
        return Err(AuditError("measurement interval is not finite".to_string()));
    }

    Ok(BoundedMeasurement {
        comparison_set_id: measurement.comparison_set_id.clone(),
        measurand_id: measurement.measurand_id.clone(),
        boundary_id: measurement.boundary_id.clone(),
        instrument_id: measurement.instrument_id.clone(),
        channel_id: measurement.channel_id.clone(),
        scope_sha256: measurement.scope_sha256.clone(),
        configuration_sha256: measurement.configuration_sha256.clone(),
        value: measurement.value,
        absolute_bound,
        lower_bound,
        upper_bound,
        unit: measurement.unit,
        evidence_class: measurement.evidence_class,
        evidence_references: references,
        propagation_method: "CONSERVATIVE_LINEAR_SUM_OF_ABSOLUTE_BOUNDS_V0_1",
    })
}

/// Conservative interval propagation for a positive ratio, such as output
/// energy divided by input energy. Numerator and denominator may be measured at
/// different POCs, but must belong to the same frozen comparison set and scope.
pub fn bounded_positive_ratio(
    numerator: &BoundedMeasurement,
    denominator: &BoundedMeasurement,
) -> Result<BoundedRatio, AuditError> {
    if numerator.comparison_set_id != denominator.comparison_set_id
        || numerator.scope_sha256 != denominator.scope_sha256
        || numerator.unit != denominator.unit
    {
        return Err(AuditError("ratio inputs must share comparison set, scope and compatible units".to_string()));
    }
    if denominator.lower_bound <= 0.0 {
        return Err(AuditError("ratio denominator lower bound must be strictly positive".to_string()));
    }
    let value = numerator.value / denominator.value;
    let lower_bound = numerator.lower_bound / denominator.upper_bound;
    let upper_bound = numerator.upper_bound / denominator.lower_bound;
    if !value.is_finite() || !lower_bound.is_finite() || !upper_bound.is_finite()
        || lower_bound > value || value > upper_bound
    {
        return Err(AuditError("propagated ratio interval is invalid".to_string()));
    }
    Ok(BoundedRatio {
        comparison_set_id: numerator.comparison_set_id.clone(),
        scope_sha256: numerator.scope_sha256.clone(),
        numerator_boundary_id: numerator.boundary_id.clone(),
        denominator_boundary_id: denominator.boundary_id.clone(),
        value,
        lower_bound,
        upper_bound,
        evidence_eligibility: evidence_eligibility(&[numerator.evidence_class, denominator.evidence_class]),
        propagation_method: "CONSERVATIVE_RECTANGULAR_RATIO_INTERVAL_V0_1",
        claim_ceiling: CLAIM,
    })
}

/// Conservative candidate-minus-reference interval. Correlated/systematic
/// errors are not cancelled; common-mode cancellation requires a separately
/// reviewed covariance model and is not available in v0.1.
pub fn bounded_paired_difference(
    candidate: &BoundedMeasurement,
    reference: &BoundedMeasurement,
    preregistered_margin: f64,
    direction: ComparisonDirection,
) -> Result<BoundedPairedDifference, AuditError> {
    if candidate.comparison_set_id != reference.comparison_set_id
        || candidate.scope_sha256 != reference.scope_sha256
        || candidate.measurand_id != reference.measurand_id
        || candidate.boundary_id != reference.boundary_id
        || candidate.unit != reference.unit
    {
        return Err(AuditError("paired measurements must match comparison set, scope, measurand, POC and unit".to_string()));
    }
    if !finite_nonnegative(preregistered_margin) {
        return Err(AuditError("preregistered decision margin must be finite and non-negative".to_string()));
    }
    let point_delta = candidate.value - reference.value;
    let lower_bound = candidate.lower_bound - reference.upper_bound;
    let upper_bound = candidate.upper_bound - reference.lower_bound;
    if !point_delta.is_finite() || !lower_bound.is_finite() || !upper_bound.is_finite()
        || lower_bound > point_delta || point_delta > upper_bound
    {
        return Err(AuditError("paired difference interval is invalid".to_string()));
    }
    let margin_decision = match direction {
        ComparisonDirection::LowerIsBetter if upper_bound < -preregistered_margin =>
            MarginDecision::ClearsPreregisteredMargin,
        ComparisonDirection::LowerIsBetter if lower_bound >= -preregistered_margin =>
            MarginDecision::DoesNotClearMargin,
        ComparisonDirection::HigherIsBetter if lower_bound > preregistered_margin =>
            MarginDecision::ClearsPreregisteredMargin,
        ComparisonDirection::HigherIsBetter if upper_bound <= preregistered_margin =>
            MarginDecision::DoesNotClearMargin,
        _ => MarginDecision::OverlapsDecisionBoundary,
    };
    let evidence_eligibility = evidence_eligibility(&[candidate.evidence_class, reference.evidence_class]);
    Ok(BoundedPairedDifference {
        comparison_set_id: candidate.comparison_set_id.clone(),
        measurand_id: candidate.measurand_id.clone(),
        boundary_id: candidate.boundary_id.clone(),
        scope_sha256: candidate.scope_sha256.clone(),
        point_delta,
        lower_bound,
        upper_bound,
        preregistered_margin,
        direction,
        margin_decision,
        evidence_eligibility,
        physical_superiority_claim_eligible: margin_decision == MarginDecision::ClearsPreregisteredMargin
            && evidence_eligibility == EvidenceEligibility::IndependentPhysicalMeasurement,
        propagation_method: "CONSERVATIVE_RECTANGULAR_PAIRED_DIFFERENCE_V0_1",
        claim_ceiling: CLAIM,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(ch: char) -> String { std::iter::repeat(ch).take(64).collect() }

    fn measurement(
        measurand: &str,
        boundary: &str,
        value: f64,
        class: LifecycleEvidenceClass,
    ) -> MeterMeasurement {
        MeterMeasurement {
            comparison_set_id: "set-1".into(),
            measurand_id: measurand.into(),
            boundary_id: boundary.into(),
            instrument_id: format!("meter-{boundary}"),
            channel_id: format!("channel-{boundary}"),
            scope_sha256: h('a'),
            configuration_sha256: h('b'),
            value,
            unit: MeasurementUnit::WattHour,
            observed_at_epoch_s: 100,
            evaluation_time_epoch_s: 200,
            calibration_id: format!("calibration-{boundary}"),
            calibration_valid_until_epoch_s: 300,
            calibration_status: EvidenceStatus::Current,
            timebase_reference: "timebase:200".into(),
            timebase_status: EvidenceStatus::Current,
            evidence_status: EvidenceStatus::Current,
            evidence_class: class,
            evidence_reference: format!("measurement:{boundary}"),
            uncertainty_components: vec![
                AbsoluteUncertaintyComponent {
                    component_id: "accuracy".into(),
                    absolute_bound: 0.3,
                    unit: MeasurementUnit::WattHour,
                    kind: UncertaintyComponentKind::InstrumentAccuracy,
                    evidence_status: EvidenceStatus::Current,
                    evidence_reference: "spec:accuracy".into(),
                },
                AbsoluteUncertaintyComponent {
                    component_id: "calibration".into(),
                    absolute_bound: 0.1,
                    unit: MeasurementUnit::WattHour,
                    kind: UncertaintyComponentKind::CalibrationCorrection,
                    evidence_status: EvidenceStatus::Current,
                    evidence_reference: "certificate:calibration".into(),
                },
            ],
        }
    }

    #[test]
    fn absolute_bounds_are_summed_linearly_not_rss() {
        let result = bound_measurement(&measurement(
            "energy", "poc-a", 100.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement
        )).unwrap();
        assert!((result.absolute_bound - 0.4).abs() < 1e-12);
        assert!((result.lower_bound - 99.6).abs() < 1e-12);
        assert!((result.upper_bound - 100.4).abs() < 1e-12);
        assert_eq!(result.propagation_method, "CONSERVATIVE_LINEAR_SUM_OF_ABSOLUTE_BOUNDS_V0_1");
    }

    #[test]
    fn ratio_interval_retains_distinct_input_and_output_pocs() {
        let output = bound_measurement(&measurement(
            "output-energy", "customer-poc", 95.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement
        )).unwrap();
        let input = bound_measurement(&measurement(
            "input-energy", "source-poc", 100.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement
        )).unwrap();
        let ratio = bounded_positive_ratio(&output, &input).unwrap();
        assert_eq!(ratio.numerator_boundary_id, "customer-poc");
        assert_eq!(ratio.denominator_boundary_id, "source-poc");
        assert!(ratio.lower_bound < ratio.value && ratio.value < ratio.upper_bound);
        assert_eq!(ratio.evidence_eligibility, EvidenceEligibility::IndependentPhysicalMeasurement);
    }

    #[test]
    fn ratio_rejects_nonpositive_denominator_lower_bound() {
        let output = bound_measurement(&measurement(
            "output", "out", 5.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement
        )).unwrap();
        let mut input = measurement("input", "in", 0.2, LifecycleEvidenceClass::IndependentPhysicalMeasurement);
        input.uncertainty_components[0].absolute_bound = 0.3;
        assert!(bounded_positive_ratio(&output, &bound_measurement(&input).unwrap()).is_err());
    }

    #[test]
    fn ratio_requires_same_frozen_scope_and_unit() {
        let output = bound_measurement(&measurement(
            "output", "out", 95.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement
        )).unwrap();
        let mut input = measurement("input", "in", 100.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement);
        input.scope_sha256 = h('c');
        assert!(bounded_positive_ratio(&output, &bound_measurement(&input).unwrap()).is_err());
    }

    #[test]
    fn paired_difference_clears_margin_only_when_entire_interval_is_beyond_it() {
        let reference = bound_measurement(&measurement(
            "cost", "customer-poc", 100.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement
        )).unwrap();
        let candidate = bound_measurement(&measurement(
            "cost", "customer-poc", 95.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement
        )).unwrap();
        let result = bounded_paired_difference(
            &candidate, &reference, 2.0, ComparisonDirection::LowerIsBetter
        ).unwrap();
        assert_eq!(result.margin_decision, MarginDecision::ClearsPreregisteredMargin);
        assert!(result.physical_superiority_claim_eligible);
    }

    #[test]
    fn overlapping_paired_interval_does_not_clear_margin() {
        let reference = bound_measurement(&measurement(
            "cost", "customer-poc", 100.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement
        )).unwrap();
        let mut candidate_input = measurement(
            "cost", "customer-poc", 98.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement
        );
        candidate_input.uncertainty_components[0].absolute_bound = 1.5;
        candidate_input.uncertainty_components[1].absolute_bound = 0.5;
        let candidate = bound_measurement(&candidate_input).unwrap();
        let result = bounded_paired_difference(
            &candidate, &reference, 2.0, ComparisonDirection::LowerIsBetter
        ).unwrap();
        assert_eq!(result.margin_decision, MarginDecision::OverlapsDecisionBoundary);
        assert!(!result.physical_superiority_claim_eligible);
    }

    #[test]
    fn synthetic_measurements_never_become_physical_superiority_evidence() {
        let reference = bound_measurement(&measurement(
            "cost", "customer-poc", 100.0, LifecycleEvidenceClass::SyntheticFixture
        )).unwrap();
        let candidate = bound_measurement(&measurement(
            "cost", "customer-poc", 90.0, LifecycleEvidenceClass::SyntheticFixture
        )).unwrap();
        let result = bounded_paired_difference(
            &candidate, &reference, 1.0, ComparisonDirection::LowerIsBetter
        ).unwrap();
        assert_eq!(result.margin_decision, MarginDecision::ClearsPreregisteredMargin);
        assert_eq!(result.evidence_eligibility, EvidenceEligibility::SyntheticOrAssumption);
        assert!(!result.physical_superiority_claim_eligible);
    }

    #[test]
    fn rejects_stale_calibration_and_timebase() {
        let mut input = measurement("energy", "poc", 100.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement);
        input.calibration_status = EvidenceStatus::Expired;
        assert!(bound_measurement(&input).is_err());
        input = measurement("energy", "poc", 100.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement);
        input.timebase_status = EvidenceStatus::Unverified;
        assert!(bound_measurement(&input).is_err());
    }

    #[test]
    fn rejects_unit_mismatch_and_duplicate_uncertainty_components() {
        let mut input = measurement("energy", "poc", 100.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement);
        input.uncertainty_components[0].unit = MeasurementUnit::Watt;
        assert!(bound_measurement(&input).is_err());
        input = measurement("energy", "poc", 100.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement);
        input.uncertainty_components[1].component_id = "accuracy".into();
        assert!(bound_measurement(&input).is_err());
    }

    #[test]
    fn paired_difference_rejects_boundary_or_scope_mismatch() {
        let reference = bound_measurement(&measurement(
            "cost", "poc-a", 100.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement
        )).unwrap();
        let mut candidate_input = measurement(
            "cost", "poc-b", 90.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement
        );
        let candidate = bound_measurement(&candidate_input).unwrap();
        assert!(bounded_paired_difference(
            &candidate, &reference, 1.0, ComparisonDirection::LowerIsBetter
        ).is_err());
        candidate_input = measurement(
            "cost", "poc-a", 90.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement
        );
        candidate_input.scope_sha256 = h('c');
        let candidate = bound_measurement(&candidate_input).unwrap();
        assert!(bounded_paired_difference(
            &candidate, &reference, 1.0, ComparisonDirection::LowerIsBetter
        ).is_err());
    }

    #[test]
    fn rejects_expired_uncertainty_component_evidence() {
        let mut input = measurement("energy", "poc", 100.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement);
        input.uncertainty_components[0].evidence_status = EvidenceStatus::Expired;
        assert!(bound_measurement(&input).is_err());
    }

    #[test]
    fn higher_is_better_margin_uses_lower_difference_bound() {
        let reference = bound_measurement(&measurement(
            "capacity", "battery-poc", 100.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement
        )).unwrap();
        let candidate = bound_measurement(&measurement(
            "capacity", "battery-poc", 105.0, LifecycleEvidenceClass::IndependentPhysicalMeasurement
        )).unwrap();
        let result = bounded_paired_difference(
            &candidate, &reference, 2.0, ComparisonDirection::HigherIsBetter
        ).unwrap();
        assert_eq!(result.margin_decision, MarginDecision::ClearsPreregisteredMargin);
    }
}
