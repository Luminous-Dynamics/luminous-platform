use crate::AuditError;

#[derive(Debug, Clone, PartialEq)]
pub struct InterleavedPhase {
    pub id: String,
    /// Actual phase offset in radians for the switching fundamental.
    pub phase_offset_radians: f64,
    /// Peak ripple-current amplitudes for each harmonic order in the input.
    pub harmonic_peak_ripple_currents_a: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterleavedRippleInput {
    /// Positive integer harmonic orders relative to the switching fundamental.
    pub harmonic_orders: Vec<u32>,
    pub phases: Vec<InterleavedPhase>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HarmonicRippleResult {
    pub harmonic_order: u32,
    pub combined_peak_ripple_current_a: f64,
    pub combined_rms_ripple_current_a: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterleavedRippleResult {
    pub phase_count: usize,
    pub harmonic_results: Vec<HarmonicRippleResult>,
    /// RSS of harmonic RMS currents, assuming distinct integer harmonics over
    /// a sufficiently long steady-state observation window.
    pub total_rms_ripple_current_a: f64,
    /// Conservative sum-of-harmonic peak envelope, not the exact waveform peak.
    pub peak_ripple_envelope_upper_bound_a: f64,
}

impl InterleavedRippleResult {
    pub const fn claim_ceiling(&self) -> &'static str {
        "HARMONIC_PHASOR_RIPPLE_SCREEN_ONLY_NOT_SWITCHING_WAVEFORM_NOT_CAPACITOR_RMS_OR_EMI_QUALIFICATION"
    }
}

/// Sum phase-displaced harmonic ripple-current phasors.
///
/// For harmonic h, the phase displacement is h * phase_offset. Unequal
/// per-phase amplitudes and non-ideal phase offsets are retained. This captures
/// ideal interleaving cancellation and first-order mismatch sensitivity, but
/// not the underlying switch waveform, dead time, parasitic ringing, modulation
/// dynamics, capacitor ESR/ESL or layout coupling.
pub fn evaluate_interleaved_ripple(
    input: InterleavedRippleInput,
) -> Result<InterleavedRippleResult, AuditError> {
    if input.harmonic_orders.is_empty() || input.phases.is_empty() {
        return Err(AuditError(
            "interleaved ripple requires at least one harmonic and one phase".to_string(),
        ));
    }

    for (index, order) in input.harmonic_orders.iter().enumerate() {
        if *order == 0 || input.harmonic_orders[..index].contains(order) {
            return Err(AuditError(
                "harmonic orders must be positive and unique".to_string(),
            ));
        }
    }

    let mut phase_ids: Vec<&str> = Vec::with_capacity(input.phases.len());
    for phase in &input.phases {
        if phase.id.trim().is_empty() || phase_ids.contains(&phase.id.as_str()) {
            return Err(AuditError(
                "phase identifiers must be non-blank and unique".to_string(),
            ));
        }
        phase_ids.push(phase.id.as_str());

        if !phase.phase_offset_radians.is_finite() {
            return Err(AuditError(format!(
                "phase {:?}: phase offset must be finite",
                phase.id
            )));
        }
        if phase.harmonic_peak_ripple_currents_a.len() != input.harmonic_orders.len() {
            return Err(AuditError(format!(
                "phase {:?}: harmonic amplitude count must match harmonic_orders",
                phase.id
            )));
        }
        if phase
            .harmonic_peak_ripple_currents_a
            .iter()
            .any(|amplitude| !amplitude.is_finite() || *amplitude < 0.0)
        {
            return Err(AuditError(format!(
                "phase {:?}: harmonic peak amplitudes must be finite and non-negative",
                phase.id
            )));
        }
    }

    let mut harmonic_results = Vec::with_capacity(input.harmonic_orders.len());
    let mut rms_square_sum = 0.0_f64;
    let mut peak_envelope = 0.0_f64;

    for (harmonic_index, order) in input.harmonic_orders.iter().enumerate() {
        let harmonic_multiplier = *order as f64;
        let mut real_sum = 0.0_f64;
        let mut imaginary_sum = 0.0_f64;

        for phase in &input.phases {
            let amplitude = phase.harmonic_peak_ripple_currents_a[harmonic_index];
            let angle = harmonic_multiplier * phase.phase_offset_radians;
            real_sum += amplitude * angle.cos();
            imaginary_sum += amplitude * angle.sin();
        }

        let combined_peak = real_sum.hypot(imaginary_sum);
        let combined_rms = combined_peak / std::f64::consts::SQRT_2;
        if !combined_peak.is_finite() || !combined_rms.is_finite() {
            return Err(AuditError(format!(
                "harmonic {order}: combined ripple is non-finite"
            )));
        }

        rms_square_sum += combined_rms * combined_rms;
        peak_envelope += combined_peak;
        harmonic_results.push(HarmonicRippleResult {
            harmonic_order: *order,
            combined_peak_ripple_current_a: combined_peak,
            combined_rms_ripple_current_a: combined_rms,
        });
    }

    let total_rms_ripple_current_a = rms_square_sum.sqrt();
    if !total_rms_ripple_current_a.is_finite()
        || !peak_envelope.is_finite()
        || total_rms_ripple_current_a < 0.0
    {
        return Err(AuditError(
            "aggregate interleaved ripple is non-finite".to_string(),
        ));
    }

    Ok(InterleavedRippleResult {
        phase_count: input.phases.len(),
        harmonic_results,
        total_rms_ripple_current_a,
        peak_ripple_envelope_upper_bound_a: peak_envelope,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_interleaved_ripple, InterleavedPhase, InterleavedRippleInput,
    };

    fn phase(id: &str, offset: f64, fundamental: f64, fourth: f64) -> InterleavedPhase {
        InterleavedPhase {
            id: id.to_string(),
            phase_offset_radians: offset,
            harmonic_peak_ripple_currents_a: vec![fundamental, fourth],
        }
    }

    #[test]
    fn four_uniformly_interleaved_phases_cancel_fundamental() {
        let report = evaluate_interleaved_ripple(InterleavedRippleInput {
            harmonic_orders: vec![1, 4],
            phases: vec![
                phase("a", 0.0, 10.0, 2.0),
                phase("b", std::f64::consts::FRAC_PI_2, 10.0, 2.0),
                phase("c", std::f64::consts::PI, 10.0, 2.0),
                phase("d", 3.0 * std::f64::consts::FRAC_PI_2, 10.0, 2.0),
            ],
        })
        .unwrap();

        assert!(report.harmonic_results[0].combined_peak_ripple_current_a < 1.0e-9);
        assert!((report.harmonic_results[1].combined_peak_ripple_current_a - 8.0).abs() < 1.0e-9);
        assert!((report.total_rms_ripple_current_a - 8.0 / std::f64::consts::SQRT_2).abs() < 1.0e-9);
    }

    #[test]
    fn phase_error_leaves_a_finite_residual_ripple() {
        let report = evaluate_interleaved_ripple(InterleavedRippleInput {
            harmonic_orders: vec![1],
            phases: vec![
                phase("a", 0.0, 1.0, 0.0),
                phase("b", std::f64::consts::PI + 0.1, 1.0, 0.0),
            ],
        })
        .unwrap();

        assert!(report.harmonic_results[0].combined_peak_ripple_current_a > 0.0);
        assert!(report.harmonic_results[0].combined_peak_ripple_current_a < 0.11);
    }

    #[test]
    fn amplitude_mismatch_leaves_the_difference_as_residual_ripple() {
        let report = evaluate_interleaved_ripple(InterleavedRippleInput {
            harmonic_orders: vec![1],
            phases: vec![
                phase("a", 0.0, 1.0, 0.0),
                phase("b", std::f64::consts::PI, 0.8, 0.0),
            ],
        })
        .unwrap();

        assert!((report.harmonic_results[0].combined_peak_ripple_current_a - 0.2).abs() < 1.0e-9);
    }

    #[test]
    fn rejects_invalid_harmonics_phases_and_amplitudes() {
        let empty = evaluate_interleaved_ripple(InterleavedRippleInput {
            harmonic_orders: vec![],
            phases: vec![phase("a", 0.0, 1.0, 0.0)],
        });
        let duplicate_harmonics = evaluate_interleaved_ripple(InterleavedRippleInput {
            harmonic_orders: vec![1, 1],
            phases: vec![phase("a", 0.0, 1.0, 1.0)],
        });
        let mismatched_amplitudes = evaluate_interleaved_ripple(InterleavedRippleInput {
            harmonic_orders: vec![1],
            phases: vec![phase("a", 0.0, 1.0, 1.0)],
        });
        let negative_amplitude = evaluate_interleaved_ripple(InterleavedRippleInput {
            harmonic_orders: vec![1, 4],
            phases: vec![phase("a", 0.0, -1.0, 1.0)],
        });

        assert!(empty.is_err());
        assert!(duplicate_harmonics.is_err());
        assert!(mismatched_amplitudes.is_err());
        assert!(negative_amplitude.is_err());
    }
}
