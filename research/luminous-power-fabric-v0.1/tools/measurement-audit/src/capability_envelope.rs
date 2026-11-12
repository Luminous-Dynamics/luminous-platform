use crate::AuditError;

/// Evidence state for a value that gates an electrical capability claim.
///
/// Only Current is eligible. Unknown, missing, expired, invalid and unverified
/// states all fail closed. This enum does not assert that an observation is true;
/// it records the state supplied by the caller's evidence-verification process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStatus {
    Current,
    Missing,
    Expired,
    Invalid,
    Unverified,
}

impl EvidenceStatus {
    const fn is_current(self) -> bool {
        matches!(self, Self::Current)
    }
}

/// State of the independent protection subsystem.
///
/// Only Ready is eligible. This evaluator never commands or resets protection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionStatus {
    Ready,
    Tripped,
    Faulted,
    Bypassed,
    Unknown,
}

/// Thermal observation and the declared steady-state derating interval.
///
/// The temperature must be measured at the boundary represented by
/// measurement_boundary. This is not a junction-temperature estimator.
#[derive(Debug, Clone, PartialEq)]
pub struct ThermalCapabilityInput {
    pub observed_temperature_c: f64,
    pub derate_start_temperature_c: f64,
    pub inhibit_temperature_c: f64,
    pub measurement_boundary: String,
    pub evidence_reference: String,
}

/// Inputs required to calculate a time-bounded capability envelope.
///
/// Power ratings are continuous nameplate/qualification inputs for the exact
/// hardware configuration. The result is a conservative software screen, not
/// a protection setting, dispatch authorization, or physical safety claim.
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityEnvelopeInput {
    /// SHA-256 of the exact configuration for which the qualification applies.
    pub qualified_configuration_sha256: String,
    /// SHA-256 of the currently observed hardware/firmware/control configuration.
    pub observed_configuration_sha256: String,
    pub qualification_valid_until_epoch_s: u64,
    pub evidence_status: EvidenceStatus,
    pub calibration_status: EvidenceStatus,
    pub sensor_status: EvidenceStatus,
    pub protection_status: ProtectionStatus,
    pub telemetry_observed_at_epoch_s: u64,
    pub telemetry_max_age_s: u64,
    pub evaluation_time_epoch_s: u64,
    pub forward_rated_power_w: f64,
    pub reverse_rated_power_w: f64,
    /// A declared factor for source, cabling, installation or ambient constraints.
    /// It must be in [0, 1] and supported by the caller's evidence.
    pub external_derating_factor: f64,
    /// Conservative absolute bounds, not standard deviations or confidence intervals.
    pub measurement_uncertainty_bound_w: f64,
    pub model_uncertainty_bound_w: f64,
    pub uncertainty_reference: String,
    pub external_derating_reference: String,
    pub thermal: ThermalCapabilityInput,
}

/// Reasons why a capability commitment is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityBlocker {
    ConfigurationDigestMalformed,
    ConfigurationMismatch,
    QualificationExpired,
    EvidenceNotCurrent,
    CalibrationNotCurrent,
    SensorEvidenceNotCurrent,
    ProtectionNotReady,
    TelemetryMissing,
    TelemetryFromFuture,
    TelemetryStale,
    ThermalInputInvalid,
    ThermalInhibitReached,
}

/// Conservative continuous-power capability in both directions.
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityEnvelopeReport {
    pub forward_available_power_w: f64,
    pub reverse_available_power_w: f64,
    pub thermal_derating_factor: f64,
    pub external_derating_factor: f64,
    pub total_uncertainty_bound_w: f64,
    pub eligible: bool,
    pub blockers: Vec<CapabilityBlocker>,
}

impl CapabilityEnvelopeReport {
    /// This result only screens a caller-supplied capability contract.
    /// It is not a relay action, physical measurement, safety qualification,
    /// breaker rating, dispatch approval or certification.
    pub const fn claim_ceiling(&self) -> &'static str {
        "EVIDENCE_GATED_CAPABILITY_SCREEN_ONLY_NOT_CONTROL_OR_PHYSICAL_QUALIFICATION"
    }

    pub fn has_available_capability(&self) -> bool {
        self.eligible
            && (self.forward_available_power_w > 0.0
                || self.reverse_available_power_w > 0.0)
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_nonnegative_finite(value: f64, field: &str) -> Result<(), AuditError> {
    if !value.is_finite() || value < 0.0 {
        return Err(AuditError(format!(
            "{field} must be finite and non-negative"
        )));
    }
    Ok(())
}

fn validate_positive_finite(value: f64, field: &str) -> Result<(), AuditError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(AuditError(format!(
            "{field} must be finite and positive"
        )));
    }
    Ok(())
}

/// Calculate direction-specific continuous capability for one evidence snapshot.
///
/// Thermal derating uses a declared linear law:
/// - factor = 1 at or below derate_start_temperature_c;
/// - factor falls linearly to 0 at inhibit_temperature_c;
/// - reaching the inhibit temperature adds a hard blocker.
///
/// The conservative power bound is:
///
/// max(0, rated_power * thermal_factor * external_factor
///         - measurement_uncertainty_bound - model_uncertainty_bound)
///
/// Any hard blocker sets both directional capabilities to zero. No averaging,
/// compensation, automatic reset or partial capacity retention is permitted.
///
/// The caller is responsible for providing a trustworthy evaluation clock,
/// evidence-verification results, configuration digests, and appropriately
/// bounded uncertainty inputs. This function does not authenticate them.
pub fn evaluate_capability_envelope(
    input: &CapabilityEnvelopeInput,
) -> Result<CapabilityEnvelopeReport, AuditError> {
    validate_positive_finite(input.forward_rated_power_w, "forward_rated_power_w")?;
    validate_positive_finite(input.reverse_rated_power_w, "reverse_rated_power_w")?;
    validate_nonnegative_finite(
        input.measurement_uncertainty_bound_w,
        "measurement_uncertainty_bound_w",
    )?;
    validate_nonnegative_finite(
        input.model_uncertainty_bound_w,
        "model_uncertainty_bound_w",
    )?;

    if !input.external_derating_factor.is_finite()
        || !(0.0..=1.0).contains(&input.external_derating_factor)
    {
        return Err(AuditError(
            "external_derating_factor must be finite and within [0, 1]".to_string(),
        ));
    }
    if input.telemetry_max_age_s == 0 {
        return Err(AuditError(
            "telemetry_max_age_s must be positive".to_string(),
        ));
    }
    if input.qualification_valid_until_epoch_s == 0
        || input.evaluation_time_epoch_s == 0
    {
        return Err(AuditError(
            "qualification expiry and evaluation time must be positive epoch seconds".to_string(),
        ));
    }
    if input.uncertainty_reference.trim().is_empty() {
        return Err(AuditError(
            "uncertainty_reference is required".to_string(),
        ));
    }
    if input.external_derating_reference.trim().is_empty() {
        return Err(AuditError(
            "external_derating_reference is required".to_string(),
        ));
    }
    if input.thermal.measurement_boundary.trim().is_empty()
        || input.thermal.evidence_reference.trim().is_empty()
    {
        return Err(AuditError(
            "thermal measurement boundary and evidence reference are required".to_string(),
        ));
    }

    let mut blockers = Vec::new();

    let qualified_digest_valid = valid_sha256(&input.qualified_configuration_sha256);
    let observed_digest_valid = valid_sha256(&input.observed_configuration_sha256);
    if !qualified_digest_valid || !observed_digest_valid {
        blockers.push(CapabilityBlocker::ConfigurationDigestMalformed);
    } else if !input
        .qualified_configuration_sha256
        .eq_ignore_ascii_case(&input.observed_configuration_sha256)
    {
        blockers.push(CapabilityBlocker::ConfigurationMismatch);
    }

    if input.evaluation_time_epoch_s > input.qualification_valid_until_epoch_s {
        blockers.push(CapabilityBlocker::QualificationExpired);
    }
    if !input.evidence_status.is_current() {
        blockers.push(CapabilityBlocker::EvidenceNotCurrent);
    }
    if !input.calibration_status.is_current() {
        blockers.push(CapabilityBlocker::CalibrationNotCurrent);
    }
    if !input.sensor_status.is_current() {
        blockers.push(CapabilityBlocker::SensorEvidenceNotCurrent);
    }
    if input.protection_status != ProtectionStatus::Ready {
        blockers.push(CapabilityBlocker::ProtectionNotReady);
    }

    if input.telemetry_observed_at_epoch_s == 0 {
        blockers.push(CapabilityBlocker::TelemetryMissing);
    } else if input.telemetry_observed_at_epoch_s > input.evaluation_time_epoch_s {
        blockers.push(CapabilityBlocker::TelemetryFromFuture);
    } else if input.evaluation_time_epoch_s - input.telemetry_observed_at_epoch_s
        > input.telemetry_max_age_s
    {
        blockers.push(CapabilityBlocker::TelemetryStale);
    }

    let thermal_values_valid = input.thermal.observed_temperature_c.is_finite()
        && input.thermal.derate_start_temperature_c.is_finite()
        && input.thermal.inhibit_temperature_c.is_finite()
        && input.thermal.inhibit_temperature_c
            > input.thermal.derate_start_temperature_c;

    let thermal_span_c =
        input.thermal.inhibit_temperature_c - input.thermal.derate_start_temperature_c;
    let thermal_values_valid = thermal_values_valid && thermal_span_c.is_finite();

    let thermal_derating_factor = if !thermal_values_valid {
        blockers.push(CapabilityBlocker::ThermalInputInvalid);
        0.0
    } else if input.thermal.observed_temperature_c
        >= input.thermal.inhibit_temperature_c
    {
        blockers.push(CapabilityBlocker::ThermalInhibitReached);
        0.0
    } else if input.thermal.observed_temperature_c
        <= input.thermal.derate_start_temperature_c
    {
        1.0
    } else {
        (input.thermal.inhibit_temperature_c
            - input.thermal.observed_temperature_c)
            / (input.thermal.inhibit_temperature_c
                - input.thermal.derate_start_temperature_c)
    };

    let total_uncertainty_bound_w =
        input.measurement_uncertainty_bound_w + input.model_uncertainty_bound_w;
    if !total_uncertainty_bound_w.is_finite() {
        return Err(AuditError(
            "combined uncertainty bound must remain finite".to_string(),
        ));
    }

    let eligible = blockers.is_empty();
    if !eligible {
        return Ok(CapabilityEnvelopeReport {
            forward_available_power_w: 0.0,
            reverse_available_power_w: 0.0,
            thermal_derating_factor,
            external_derating_factor: input.external_derating_factor,
            total_uncertainty_bound_w,
            eligible: false,
            blockers,
        });
    }

    let forward_available_power_w = (input.forward_rated_power_w
        * thermal_derating_factor
        * input.external_derating_factor
        - total_uncertainty_bound_w)
        .max(0.0);

    let reverse_available_power_w = (input.reverse_rated_power_w
        * thermal_derating_factor
        * input.external_derating_factor
        - total_uncertainty_bound_w)
        .max(0.0);

    if !forward_available_power_w.is_finite()
        || !reverse_available_power_w.is_finite()
    {
        return Err(AuditError(
            "derived capability must remain finite".to_string(),
        ));
    }

    Ok(CapabilityEnvelopeReport {
        forward_available_power_w,
        reverse_available_power_w,
        thermal_derating_factor,
        external_derating_factor: input.external_derating_factor,
        total_uncertainty_bound_w,
        eligible,
        blockers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_input() -> CapabilityEnvelopeInput {
        CapabilityEnvelopeInput {
            qualified_configuration_sha256: "a".repeat(64),
            observed_configuration_sha256: "A".repeat(64),
            qualification_valid_until_epoch_s: 2_000,
            evidence_status: EvidenceStatus::Current,
            calibration_status: EvidenceStatus::Current,
            sensor_status: EvidenceStatus::Current,
            protection_status: ProtectionStatus::Ready,
            telemetry_observed_at_epoch_s: 990,
            telemetry_max_age_s: 30,
            evaluation_time_epoch_s: 1_000,
            forward_rated_power_w: 1_000.0,
            reverse_rated_power_w: 800.0,
            external_derating_factor: 1.0,
            measurement_uncertainty_bound_w: 10.0,
            model_uncertainty_bound_w: 20.0,
            uncertainty_reference: "bounded example uncertainty".to_string(),
            external_derating_reference: "declared installation factor".to_string(),
            thermal: ThermalCapabilityInput {
                observed_temperature_c: 40.0,
                derate_start_temperature_c: 60.0,
                inhibit_temperature_c: 90.0,
                measurement_boundary: "module case sensor".to_string(),
                evidence_reference: "thermal calibration record".to_string(),
            },
        }
    }

    fn has_blocker(report: &CapabilityEnvelopeReport, expected: CapabilityBlocker) -> bool {
        report.blockers.contains(&expected)
    }

    #[test]
    fn nominal_capability_subtracts_declared_uncertainty() {
        let report = evaluate_capability_envelope(&valid_input()).unwrap();
        assert!(report.eligible);
        assert_eq!(report.forward_available_power_w, 970.0);
        assert_eq!(report.reverse_available_power_w, 770.0);
        assert!(report.has_available_capability());
    }

    #[test]
    fn configuration_digest_comparison_is_case_insensitive() {
        let report = evaluate_capability_envelope(&valid_input()).unwrap();
        assert!(report.eligible);
        assert!(!has_blocker(
            &report,
            CapabilityBlocker::ConfigurationMismatch
        ));
    }

    #[test]
    fn configuration_mismatch_withdraws_both_directions() {
        let mut input = valid_input();
        input.observed_configuration_sha256 = "b".repeat(64);
        let report = evaluate_capability_envelope(&input).unwrap();
        assert_eq!(report.forward_available_power_w, 0.0);
        assert_eq!(report.reverse_available_power_w, 0.0);
        assert!(has_blocker(
            &report,
            CapabilityBlocker::ConfigurationMismatch
        ));
    }

    #[test]
    fn malformed_configuration_digest_fails_closed() {
        let mut input = valid_input();
        input.observed_configuration_sha256 = "not-a-digest".to_string();
        let report = evaluate_capability_envelope(&input).unwrap();
        assert_eq!(report.forward_available_power_w, 0.0);
        assert!(has_blocker(
            &report,
            CapabilityBlocker::ConfigurationDigestMalformed
        ));
    }

    #[test]
    fn expired_qualification_withdraws_capability() {
        let mut input = valid_input();
        input.qualification_valid_until_epoch_s = 999;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert_eq!(report.forward_available_power_w, 0.0);
        assert!(has_blocker(
            &report,
            CapabilityBlocker::QualificationExpired
        ));
    }

    #[test]
    fn non_current_evidence_withdraws_capability() {
        let mut input = valid_input();
        input.evidence_status = EvidenceStatus::Unverified;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert_eq!(report.forward_available_power_w, 0.0);
        assert!(has_blocker(&report, CapabilityBlocker::EvidenceNotCurrent));
    }

    #[test]
    fn invalid_calibration_withdraws_capability() {
        let mut input = valid_input();
        input.calibration_status = EvidenceStatus::Invalid;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert_eq!(report.reverse_available_power_w, 0.0);
        assert!(has_blocker(
            &report,
            CapabilityBlocker::CalibrationNotCurrent
        ));
    }

    #[test]
    fn missing_sensor_evidence_withdraws_capability() {
        let mut input = valid_input();
        input.sensor_status = EvidenceStatus::Missing;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert_eq!(report.forward_available_power_w, 0.0);
        assert!(has_blocker(
            &report,
            CapabilityBlocker::SensorEvidenceNotCurrent
        ));
    }

    #[test]
    fn protection_trip_withdraws_capability() {
        let mut input = valid_input();
        input.protection_status = ProtectionStatus::Tripped;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert_eq!(report.forward_available_power_w, 0.0);
        assert!(has_blocker(&report, CapabilityBlocker::ProtectionNotReady));
    }

    #[test]
    fn bypassed_protection_withdraws_capability() {
        let mut input = valid_input();
        input.protection_status = ProtectionStatus::Bypassed;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert_eq!(report.reverse_available_power_w, 0.0);
        assert!(has_blocker(&report, CapabilityBlocker::ProtectionNotReady));
    }

    #[test]
    fn stale_telemetry_withdraws_capability() {
        let mut input = valid_input();
        input.telemetry_observed_at_epoch_s = 900;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert_eq!(report.forward_available_power_w, 0.0);
        assert!(has_blocker(&report, CapabilityBlocker::TelemetryStale));
    }

    #[test]
    fn future_dated_telemetry_withdraws_capability() {
        let mut input = valid_input();
        input.telemetry_observed_at_epoch_s = 1_001;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert_eq!(report.forward_available_power_w, 0.0);
        assert!(has_blocker(&report, CapabilityBlocker::TelemetryFromFuture));
    }

    #[test]
    fn missing_telemetry_withdraws_capability() {
        let mut input = valid_input();
        input.telemetry_observed_at_epoch_s = 0;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert_eq!(report.forward_available_power_w, 0.0);
        assert!(has_blocker(&report, CapabilityBlocker::TelemetryMissing));
    }

    #[test]
    fn thermal_derating_is_linear_between_declared_thresholds() {
        let mut input = valid_input();
        input.thermal.observed_temperature_c = 75.0;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert!(report.eligible);
        assert!((report.thermal_derating_factor - 0.5).abs() < 1e-12);
        assert_eq!(report.forward_available_power_w, 470.0);
        assert_eq!(report.reverse_available_power_w, 370.0);
    }

    #[test]
    fn thermal_derating_is_unity_below_start_temperature() {
        let report = evaluate_capability_envelope(&valid_input()).unwrap();
        assert_eq!(report.thermal_derating_factor, 1.0);
    }

    #[test]
    fn reaching_thermal_inhibit_temperature_withdraws_capability() {
        let mut input = valid_input();
        input.thermal.observed_temperature_c = 90.0;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert!(!report.eligible);
        assert_eq!(report.forward_available_power_w, 0.0);
        assert!(has_blocker(
            &report,
            CapabilityBlocker::ThermalInhibitReached
        ));
    }

    #[test]
    fn environmental_derating_is_applied_to_both_directions() {
        let mut input = valid_input();
        input.external_derating_factor = 0.8;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert_eq!(report.forward_available_power_w, 770.0);
        assert_eq!(report.reverse_available_power_w, 610.0);
    }

    #[test]
    fn uncertainty_floor_never_creates_negative_power() {
        let mut input = valid_input();
        input.measurement_uncertainty_bound_w = 2_000.0;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert!(report.eligible);
        assert_eq!(report.forward_available_power_w, 0.0);
        assert_eq!(report.reverse_available_power_w, 0.0);
        assert!(!report.has_available_capability());
    }

    #[test]
    fn invalid_external_derating_factor_is_rejected() {
        let mut input = valid_input();
        input.external_derating_factor = 1.01;
        assert!(evaluate_capability_envelope(&input).is_err());
    }

    #[test]
    fn negative_uncertainty_is_rejected() {
        let mut input = valid_input();
        input.model_uncertainty_bound_w = -1.0;
        assert!(evaluate_capability_envelope(&input).is_err());
    }

    #[test]
    fn non_finite_power_rating_is_rejected() {
        let mut input = valid_input();
        input.forward_rated_power_w = f64::NAN;
        assert!(evaluate_capability_envelope(&input).is_err());
    }

    #[test]
    fn non_positive_rating_is_rejected() {
        let mut input = valid_input();
        input.reverse_rated_power_w = 0.0;
        assert!(evaluate_capability_envelope(&input).is_err());
    }

    #[test]
    fn invalid_thermal_interval_is_rejected_as_a_hard_blocker() {
        let mut input = valid_input();
        input.thermal.inhibit_temperature_c = input.thermal.derate_start_temperature_c;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert_eq!(report.forward_available_power_w, 0.0);
        assert!(has_blocker(
            &report,
            CapabilityBlocker::ThermalInputInvalid
        ));
    }

    #[test]
    fn zero_telemetry_max_age_is_rejected() {
        let mut input = valid_input();
        input.telemetry_max_age_s = 0;
        assert!(evaluate_capability_envelope(&input).is_err());
    }

    #[test]
    fn any_failed_hard_gate_withdraws_all_directional_capacity() {
        let mut input = valid_input();
        input.protection_status = ProtectionStatus::Unknown;
        let report = evaluate_capability_envelope(&input).unwrap();
        assert_eq!(report.forward_available_power_w, 0.0);
        assert_eq!(report.reverse_available_power_w, 0.0);
        assert!(!report.eligible);
    }

    #[test]
    fn combined_uncertainty_overflow_is_rejected() {
        let mut input = valid_input();
        input.measurement_uncertainty_bound_w = f64::MAX;
        input.model_uncertainty_bound_w = f64::MAX;
        assert!(evaluate_capability_envelope(&input).is_err());
    }

    #[test]
    fn report_declares_its_claim_ceiling() {
        let report = evaluate_capability_envelope(&valid_input()).unwrap();
        assert_eq!(
            report.claim_ceiling(),
            "EVIDENCE_GATED_CAPABILITY_SCREEN_ONLY_NOT_CONTROL_OR_PHYSICAL_QUALIFICATION"
        );
    }
}
