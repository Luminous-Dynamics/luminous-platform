use crate::AuditError;

/// Evidence class is carried through the result and never promoted by this audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdnEvidenceKind {
    Synthetic,
    Analytical,
    Simulated,
    VendorReported,
    PhysicalMeasurement,
}

/// One frequency component of the load-current spectrum.
///
/// The voltage budget is an explicit allocation for this frequency bin. The
/// RSS sum of all allocations must fit within the available AC ripple budget.
#[derive(Debug, Clone, PartialEq)]
pub struct PdnCurrentSpectrumBin {
    pub frequency_hz: f64,
    pub current_ripple_rms_a: f64,
    pub allocated_voltage_ripple_rms_v: f64,
}

/// One complex self-impedance sample in ohms, with standard uncertainty for
/// the real and imaginary components.
#[derive(Debug, Clone, PartialEq)]
pub struct PdnImpedancePoint {
    pub frequency_hz: f64,
    pub real_ohm: f64,
    pub imaginary_ohm: f64,
    pub real_standard_uncertainty_ohm: f64,
    pub imaginary_standard_uncertainty_ohm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PdnEvidenceMetadata {
    pub evidence_kind: PdnEvidenceKind,
    pub configuration_id: String,
    /// Electrical reference plane, e.g. package-die sense port or board VRM port.
    pub reference_plane_id: String,
    pub instrument_bandwidth_min_hz: f64,
    pub instrument_bandwidth_max_hz: f64,
    pub calibration_reference: Option<String>,
    /// True only when the supplied scan has been de-embedded to reference_plane_id.
    /// This module does not perform de-embedding.
    pub deembedded_to_reference_plane: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PdnFrequencyAuditInput {
    pub nominal_rail_voltage_v: f64,
    /// AC ripple budget remaining after DC load-line, regulator ripple,
    /// measurement and other reserved terms have been subtracted.
    pub available_ac_ripple_budget_rms_v: f64,
    /// Coverage factor applied to component standard uncertainties.
    pub uncertainty_coverage_factor: f64,
    /// A local peak is flagged when its magnitude / largest adjacent magnitude
    /// meets or exceeds this ratio. It is a resonance candidate, not diagnosis.
    pub minimum_local_peak_ratio: f64,
    pub current_spectrum: Vec<PdnCurrentSpectrumBin>,
    pub impedance_scan: Vec<PdnImpedancePoint>,
    pub evidence: PdnEvidenceMetadata,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PdnFrequencyPointResult {
    pub frequency_hz: f64,
    pub current_ripple_rms_a: f64,
    pub allocated_voltage_ripple_rms_v: f64,
    pub target_impedance_ohm: f64,
    pub measured_impedance_magnitude_ohm: f64,
    /// Conservative upper bound: |Z| + k * (sigma_real + sigma_imag).
    /// The linear uncertainty sum avoids assuming component covariance.
    pub impedance_magnitude_upper_bound_ohm: f64,
    pub upper_bound_to_target_ratio: f64,
    pub within_target: bool,
    pub local_peak_ratio: Option<f64>,
    pub anti_resonance_candidate: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PdnFrequencyAuditReport {
    pub point_count: usize,
    pub nominal_rail_voltage_v: f64,
    pub available_ac_ripple_budget_rms_v: f64,
    pub allocated_ac_ripple_budget_rms_v: f64,
    pub ripple_budget_within_limit: bool,
    pub worst_frequency_hz: f64,
    pub worst_upper_bound_to_target_ratio: f64,
    pub impedance_envelope_within_target: bool,
    pub anti_resonance_candidate_count: usize,
    pub measurement_reference_plane_confirmed: bool,
    pub physical_measurement_calibration_present: bool,
    pub evidence: PdnEvidenceMetadata,
    pub points: Vec<PdnFrequencyPointResult>,
}

impl PdnFrequencyAuditReport {
    pub const fn claim_ceiling(&self) -> &'static str {
        "FREQUENCY_GRID_IMPEDANCE_ENVELOPE_SCREEN_ONLY_NOT_DEEMBEDDING_NOT_LOOP_STABILITY_NOT_PACKAGE_QUALIFICATION"
    }

    /// This only states whether the numerical impedance envelope passes at
    /// the supplied points and the declared budget is internally consistent.
    /// It is not a hardware or product qualification.
    pub fn numerical_screen_pass(&self) -> bool {
        self.ripple_budget_within_limit && self.impedance_envelope_within_target
    }

    /// Physical measurement context is present only when the source is
    /// explicitly physical, a calibration reference is declared, and the
    /// scan was de-embedded to the named reference plane.
    pub fn physical_measurement_context_complete(&self) -> bool {
        self.evidence.evidence_kind == PdnEvidenceKind::PhysicalMeasurement
            && self.physical_measurement_calibration_present
            && self.measurement_reference_plane_confirmed
    }
}

fn validate_increasing_frequencies(
    frequencies: impl Iterator<Item = f64>,
    label: &str,
) -> Result<(), AuditError> {
    let mut previous: Option<f64> = None;
    for (index, frequency) in frequencies.enumerate() {
        if !frequency.is_finite() || frequency <= 0.0 {
            return Err(AuditError(format!(
                "{label} frequency at index {index} must be positive and finite"
            )));
        }
        if let Some(prev) = previous {
            if frequency <= prev {
                return Err(AuditError(format!(
                    "{label} frequencies must be strictly increasing; index {index} is duplicated or out of order"
                )));
            }
        }
        previous = Some(frequency);
    }
    Ok(())
}

/// Derive target impedance from explicit frequency-bin current and voltage
/// budgets, then compare against a same-grid complex impedance scan.
///
/// No interpolation, extrapolation, smoothing, spectral reconstruction,
/// de-embedding, field solving or transient simulation is performed. The
/// caller is responsible for supplying a defensible current spectrum, voltage
/// budget allocation and impedance data at the same reference plane.
pub fn audit_pdn_frequency_response(
    input: PdnFrequencyAuditInput,
) -> Result<PdnFrequencyAuditReport, AuditError> {
    if !input.nominal_rail_voltage_v.is_finite()
        || input.nominal_rail_voltage_v <= 0.0
        || !input.available_ac_ripple_budget_rms_v.is_finite()
        || input.available_ac_ripple_budget_rms_v <= 0.0
        || input.available_ac_ripple_budget_rms_v >= input.nominal_rail_voltage_v
        || !input.uncertainty_coverage_factor.is_finite()
        || input.uncertainty_coverage_factor <= 0.0
        || !input.minimum_local_peak_ratio.is_finite()
        || input.minimum_local_peak_ratio <= 1.0
    {
        return Err(AuditError(
            "rail voltage, ripple budget, uncertainty factor or peak ratio is outside its valid domain"
                .to_string(),
        ));
    }

    if input.evidence.configuration_id.trim().is_empty()
        || input.evidence.reference_plane_id.trim().is_empty()
        || !input.evidence.instrument_bandwidth_min_hz.is_finite()
        || !input.evidence.instrument_bandwidth_max_hz.is_finite()
        || input.evidence.instrument_bandwidth_min_hz <= 0.0
        || input.evidence.instrument_bandwidth_max_hz
            <= input.evidence.instrument_bandwidth_min_hz
    {
        return Err(AuditError(
            "configuration, reference plane and instrument bandwidth metadata are invalid"
                .to_string(),
        ));
    }
    if input.evidence.evidence_kind == PdnEvidenceKind::PhysicalMeasurement
        && input
            .evidence
            .calibration_reference
            .as_ref()
            .is_none_or(|reference| reference.trim().is_empty())
    {
        return Err(AuditError(
            "physical measurement evidence requires a non-blank calibration reference"
                .to_string(),
        ));
    }

    if input.current_spectrum.len() < 3
        || input.current_spectrum.len() != input.impedance_scan.len()
    {
        return Err(AuditError(
            "current spectrum and impedance scan must contain the same number of at least three frequency points"
                .to_string(),
        ));
    }

    validate_increasing_frequencies(
        input.current_spectrum.iter().map(|bin| bin.frequency_hz),
        "current spectrum",
    )?;
    validate_increasing_frequencies(
        input.impedance_scan.iter().map(|point| point.frequency_hz),
        "impedance scan",
    )?;

    let mut allocated_ripple_square_sum = 0.0_f64;
    let mut targets = Vec::with_capacity(input.current_spectrum.len());

    for (index, (bin, point)) in input
        .current_spectrum
        .iter()
        .zip(input.impedance_scan.iter())
        .enumerate()
    {
        if bin.frequency_hz.to_bits() != point.frequency_hz.to_bits() {
            return Err(AuditError(format!(
                "frequency mismatch at index {index}; v0.1 requires exact frequency-grid correspondence"
            )));
        }
        if bin.frequency_hz < input.evidence.instrument_bandwidth_min_hz
            || bin.frequency_hz > input.evidence.instrument_bandwidth_max_hz
        {
            return Err(AuditError(format!(
                "frequency {} Hz is outside the declared instrument bandwidth",
                bin.frequency_hz
            )));
        }
        let bin_values = [
            bin.current_ripple_rms_a,
            bin.allocated_voltage_ripple_rms_v,
            point.real_ohm,
            point.imaginary_ohm,
            point.real_standard_uncertainty_ohm,
            point.imaginary_standard_uncertainty_ohm,
        ];
        if bin_values.iter().any(|value| !value.is_finite())
            || bin.current_ripple_rms_a <= 0.0
            || bin.allocated_voltage_ripple_rms_v <= 0.0
            || point.real_standard_uncertainty_ohm < 0.0
            || point.imaginary_standard_uncertainty_ohm < 0.0
        {
            return Err(AuditError(format!(
                "spectrum or impedance values at index {index} are non-finite or outside their valid domain"
            )));
        }
        let target = bin.allocated_voltage_ripple_rms_v / bin.current_ripple_rms_a;
        if !target.is_finite() || target <= 0.0 {
            return Err(AuditError(format!(
                "target impedance at index {index} is non-finite or non-positive"
            )));
        }
        allocated_ripple_square_sum +=
            bin.allocated_voltage_ripple_rms_v * bin.allocated_voltage_ripple_rms_v;
        targets.push(target);
    }

    let allocated_ac_ripple_budget_rms_v = allocated_ripple_square_sum.sqrt();
    let budget_tolerance = 1.0e-12
        * input
            .available_ac_ripple_budget_rms_v
            .max(allocated_ac_ripple_budget_rms_v)
            .max(1.0);
    let ripple_budget_within_limit = allocated_ac_ripple_budget_rms_v
        <= input.available_ac_ripple_budget_rms_v + budget_tolerance;

    let mut points = Vec::with_capacity(input.current_spectrum.len());
    for (index, ((bin, measured), target)) in input
        .current_spectrum
        .iter()
        .zip(input.impedance_scan.iter())
        .zip(targets.iter())
        .enumerate()
    {
        let magnitude = measured.real_ohm.hypot(measured.imaginary_ohm);
        // Triangle inequality gives a conservative radius bound without
        // assuming correlation between real and imaginary uncertainties.
        let uncertainty_bound = input.uncertainty_coverage_factor
            * (measured.real_standard_uncertainty_ohm
                + measured.imaginary_standard_uncertainty_ohm);
        let upper_bound = magnitude + uncertainty_bound;
        let ratio = upper_bound / target;
        if !magnitude.is_finite()
            || !uncertainty_bound.is_finite()
            || !upper_bound.is_finite()
            || !ratio.is_finite()
        {
            return Err(AuditError(format!(
                "derived impedance value at index {index} is non-finite"
            )));
        }
        points.push(PdnFrequencyPointResult {
            frequency_hz: bin.frequency_hz,
            current_ripple_rms_a: bin.current_ripple_rms_a,
            allocated_voltage_ripple_rms_v: bin.allocated_voltage_ripple_rms_v,
            target_impedance_ohm: *target,
            measured_impedance_magnitude_ohm: magnitude,
            impedance_magnitude_upper_bound_ohm: upper_bound,
            upper_bound_to_target_ratio: ratio,
            within_target: ratio <= 1.0,
            local_peak_ratio: None,
            anti_resonance_candidate: false,
        });
    }

    // A three-point local peak is only a screening flag. Real anti-resonance
    // attribution needs topology, phase, de-embedding and model correlation.
    for index in 1..(points.len() - 1) {
        let left = points[index - 1].measured_impedance_magnitude_ohm;
        let center = points[index].measured_impedance_magnitude_ohm;
        let right = points[index + 1].measured_impedance_magnitude_ohm;
        let adjacent_max = left.max(right);
        if center > left && center > right && adjacent_max > 0.0 {
            let peak_ratio = center / adjacent_max;
            points[index].local_peak_ratio = Some(peak_ratio);
            points[index].anti_resonance_candidate =
                peak_ratio >= input.minimum_local_peak_ratio;
        }
    }

    let worst = points
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| {
            left.upper_bound_to_target_ratio
                .total_cmp(&right.upper_bound_to_target_ratio)
        })
        .ok_or_else(|| AuditError("frequency audit produced no points".to_string()))?;
    let anti_resonance_candidate_count = points
        .iter()
        .filter(|point| point.anti_resonance_candidate)
        .count();
    let impedance_envelope_within_target =
        points.iter().all(|point| point.within_target);
    let physical_measurement_calibration_present = input
        .evidence
        .calibration_reference
        .as_ref()
        .is_some_and(|reference| !reference.trim().is_empty());

    Ok(PdnFrequencyAuditReport {
        point_count: points.len(),
        nominal_rail_voltage_v: input.nominal_rail_voltage_v,
        available_ac_ripple_budget_rms_v: input.available_ac_ripple_budget_rms_v,
        allocated_ac_ripple_budget_rms_v,
        ripple_budget_within_limit,
        worst_frequency_hz: points[worst.0].frequency_hz,
        worst_upper_bound_to_target_ratio: points[worst.0].upper_bound_to_target_ratio,
        impedance_envelope_within_target,
        anti_resonance_candidate_count,
        measurement_reference_plane_confirmed: input.evidence.deembedded_to_reference_plane,
        physical_measurement_calibration_present,
        evidence: input.evidence,
        points,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        audit_pdn_frequency_response, PdnCurrentSpectrumBin, PdnEvidenceKind,
        PdnEvidenceMetadata, PdnFrequencyAuditInput, PdnImpedancePoint,
    };

    fn input(magnitudes: [f64; 3]) -> PdnFrequencyAuditInput {
        let frequencies = [1_000.0, 10_000.0, 100_000.0];
        PdnFrequencyAuditInput {
            nominal_rail_voltage_v: 0.9,
            available_ac_ripple_budget_rms_v: 0.02,
            uncertainty_coverage_factor: 2.0,
            minimum_local_peak_ratio: 1.5,
            current_spectrum: frequencies
                .iter()
                .map(|frequency_hz| PdnCurrentSpectrumBin {
                    frequency_hz: *frequency_hz,
                    current_ripple_rms_a: 100.0,
                    allocated_voltage_ripple_rms_v: 0.01,
                })
                .collect(),
            impedance_scan: frequencies
                .iter()
                .zip(magnitudes.iter())
                .map(|(frequency_hz, magnitude)| PdnImpedancePoint {
                    frequency_hz: *frequency_hz,
                    real_ohm: *magnitude,
                    imaginary_ohm: 0.0,
                    real_standard_uncertainty_ohm: 0.0,
                    imaginary_standard_uncertainty_ohm: 0.0,
                })
                .collect(),
            evidence: PdnEvidenceMetadata {
                evidence_kind: PdnEvidenceKind::Synthetic,
                configuration_id: "synthetic-pdn-v1".to_string(),
                reference_plane_id: "synthetic-die-sense".to_string(),
                instrument_bandwidth_min_hz: 100.0,
                instrument_bandwidth_max_hz: 1_000_000.0,
                calibration_reference: None,
                deembedded_to_reference_plane: true,
            },
        }
    }

    #[test]
    fn derives_per_frequency_targets_from_explicit_ripple_allocations() {
        let report = audit_pdn_frequency_response(input([0.00005, 0.00006, 0.00004])).unwrap();

        assert!((report.points[0].target_impedance_ohm - 0.0001).abs() < 1.0e-15);
        assert!((report.allocated_ac_ripple_budget_rms_v - (0.0003_f64).sqrt()).abs() < 1.0e-12);
        assert!(report.ripple_budget_within_limit);
        assert!(report.impedance_envelope_within_target);
        assert!(report.numerical_screen_pass());
        assert_eq!(report.claim_ceiling(),
            "FREQUENCY_GRID_IMPEDANCE_ENVELOPE_SCREEN_ONLY_NOT_DEEMBEDDING_NOT_LOOP_STABILITY_NOT_PACKAGE_QUALIFICATION");
    }

    #[test]
    fn rejects_voltage_budget_allocation_above_available_ripple_budget() {
        let mut test = input([0.00005, 0.00006, 0.00004]);
        for bin in &mut test.current_spectrum {
            bin.allocated_voltage_ripple_rms_v = 0.02;
        }

        let report = audit_pdn_frequency_response(test).unwrap();

        assert!(!report.ripple_budget_within_limit);
        assert!(!report.numerical_screen_pass());
    }

    #[test]
    fn reports_the_frequency_that_exceeds_target() {
        let report = audit_pdn_frequency_response(input([0.00005, 0.0002, 0.00004])).unwrap();

        assert!(!report.impedance_envelope_within_target);
        assert_eq!(report.worst_frequency_hz, 10_000.0);
        assert!(report.worst_upper_bound_to_target_ratio > 1.9);
        assert!(!report.points[1].within_target);
    }

    #[test]
    fn flags_local_impedance_peak_as_an_antiresonance_candidate() {
        let report = audit_pdn_frequency_response(input([0.00005, 0.0002, 0.00004])).unwrap();

        assert_eq!(report.anti_resonance_candidate_count, 1);
        assert!(report.points[1].anti_resonance_candidate);
        assert!(report.points[1].local_peak_ratio.unwrap() >= 1.5);
    }

    #[test]
    fn uncertainty_upper_bound_can_fail_an_otherwise_nominal_pass() {
        let mut test = input([0.00005, 0.00005, 0.00005]);
        test.impedance_scan[1].real_standard_uncertainty_ohm = 0.00003;

        let report = audit_pdn_frequency_response(test).unwrap();

        assert!(report.points[1].measured_impedance_magnitude_ohm < report.points[1].target_impedance_ohm);
        assert!(report.points[1].impedance_magnitude_upper_bound_ohm > report.points[1].target_impedance_ohm);
        assert!(!report.points[1].within_target);
    }

    #[test]
    fn requires_exact_matching_frequency_vectors() {
        let mut test = input([0.00005, 0.00005, 0.00005]);
        test.impedance_scan[1].frequency_hz += 0.01;

        assert!(audit_pdn_frequency_response(test).is_err());
    }

    #[test]
    fn rejects_duplicate_or_unsorted_frequency_points() {
        let mut duplicate = input([0.00005, 0.00005, 0.00005]);
        duplicate.current_spectrum[1].frequency_hz =
            duplicate.current_spectrum[0].frequency_hz;
        assert!(audit_pdn_frequency_response(duplicate).is_err());

        let mut unsorted = input([0.00005, 0.00005, 0.00005]);
        unsorted.current_spectrum.swap(0, 1);
        assert!(audit_pdn_frequency_response(unsorted).is_err());
    }

    #[test]
    fn physical_measurement_needs_calibration_and_exposes_deembedding_state() {
        let mut missing_calibration = input([0.00005, 0.00005, 0.00005]);
        missing_calibration.evidence.evidence_kind = PdnEvidenceKind::PhysicalMeasurement;
        assert!(audit_pdn_frequency_response(missing_calibration).is_err());

        let mut not_deembedded = input([0.00005, 0.00005, 0.00005]);
        not_deembedded.evidence.evidence_kind = PdnEvidenceKind::PhysicalMeasurement;
        not_deembedded.evidence.calibration_reference = Some("cal-record-001".to_string());
        not_deembedded.evidence.deembedded_to_reference_plane = false;
        let report = audit_pdn_frequency_response(not_deembedded).unwrap();

        assert!(!report.measurement_reference_plane_confirmed);
        assert!(report.physical_measurement_calibration_present);
        assert!(!report.physical_measurement_context_complete());
    }

    #[test]
    fn rejects_out_of_band_and_nonfinite_measurement_data() {
        let mut out_of_band = input([0.00005, 0.00005, 0.00005]);
        out_of_band.evidence.instrument_bandwidth_max_hz = 50_000.0;
        assert!(audit_pdn_frequency_response(out_of_band).is_err());

        let mut nonfinite = input([0.00005, 0.00005, 0.00005]);
        nonfinite.impedance_scan[1].imaginary_ohm = f64::NAN;
        assert!(audit_pdn_frequency_response(nonfinite).is_err());
    }
}