use crate::AuditError;
use crate::cllc_t_network::{
    screen_cllc_low_frequency_boost_window, ClLcLowFrequencyBoostWindowInput,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClLcParameterDesignInput {
    pub primary_resonant_inductance_h: f64,
    pub primary_resonant_capacitance_f: f64,
    pub secondary_resonant_inductance_h: f64,
    pub secondary_resonant_capacitance_f: f64,
    pub magnetizing_inductance_h: f64,
    pub turns_ratio_primary_to_secondary: f64,
    pub primary_dc_input_voltage_v: f64,
    pub rated_output_power_w: f64,
    /// Required normalized DC voltage gain M for the declared operating point.
    pub required_voltage_gain: f64,
    pub controller_minimum_frequency_hz: f64,
    pub controller_maximum_frequency_hz: f64,
    /// Relative tolerance for the paper's fr1=fr2 and k1=k2 assumptions.
    pub symmetry_relative_tolerance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClLcParameterDesignResult {
    pub primary_series_resonant_frequency_hz: f64,
    pub secondary_series_resonant_frequency_hz: f64,
    pub primary_magnetizing_ratio_k1: f64,
    pub secondary_magnetizing_ratio_k2: f64,
    pub symmetry_assumption_valid: bool,
    pub low_frequency_boost_window: crate::cllc_t_network::ClLcLowFrequencyBoostWindowResult,
    pub normalized_frequency_lower: f64,
    pub normalized_frequency_upper: f64,
    pub critical_quality_factor_q_max: f64,
    pub constant_power_quality_factor_q_cmp: f64,
    pub unity_gain_boundary_quality_factor: f64,
    pub unity_gain_phase_boundary_valid: bool,
    /// Conservative Lr1 upper bound from the paper's constant-power ZVS
    /// constraint, evaluated at unity gain as the worst case for M >= 1.
    pub max_primary_resonant_inductance_constant_power_zvs_h: f64,
    /// Lr1 upper bound from the paper's unity-gain ZVS constraint.
    pub max_primary_resonant_inductance_unity_gain_zvs_h: f64,
    /// Maximum gain from the larger positive root of the paper's Eq. (20).
    /// None means the discriminant has no real feasible root at the lower
    /// admissible normalized frequency.
    pub maximum_boost_gain: Option<f64>,
    pub constant_power_zvs_constraint_passes: bool,
    pub unity_gain_zvs_constraint_passes: bool,
    pub maximum_gain_constraint_passes: bool,
    pub frequency_constraint_passes: bool,
    pub overall_paper_constraint_screen_passes: bool,
}

impl ClLcParameterDesignResult {
    pub const fn claim_ceiling(&self) -> &'static str {
        "PAPER_DERIVED_ANALYTICAL_PARAMETER_CONSTRAINT_SCREEN_ONLY_NOT_ZVS_OR_HARDWARE_QUALIFICATION"
    }
}

fn relative_difference(a: f64, b: f64) -> f64 {
    (a - b).abs() / a.abs().max(b.abs()).max(f64::MIN_POSITIVE)
}

/// Evaluate the paper's analytical CLLC parameter-design constraints.
///
/// Implemented equations:
/// - Eq. (10): low-frequency boost topology-state switching-frequency window.
/// - Eq. (12): forward constant-maximum-power quality-factor trajectory.
/// - Eq. (14)–(16): FHA phase-boundary / critical-Q constant-power ZVS screen.
/// - Eq. (17)–(18): effective normalized switching-frequency interval.
/// - Eq. (19)–(20): maximum boost-gain quadratic at the lower admissible frequency.
/// - Eq. (22)–(23): unity-gain ZVS boundary.
///
/// The paper derives these design constraints for symmetric primary/secondary
/// resonant frequencies and magnetizing ratios. This evaluator returns an error
/// when the actual input tank violates those symmetry assumptions beyond the
/// declared tolerance; it does not silently force an asymmetric tank into the
/// symmetric equations.
///
/// Important: passing this analytical screen is NOT proof of device ZVS. It does
/// not model switch Coss, dead-time, gate drive, detailed current waveforms,
/// resonant current stress, harmonic error, losses, magnetics saturation,
/// parasitics, thermal limits or transient control. Use the state/impedance model,
/// time-domain switching simulation and physical waveform measurements as
/// separate qualification layers.
pub fn evaluate_cllc_parameter_design(
    input: ClLcParameterDesignInput,
) -> Result<ClLcParameterDesignResult, AuditError> {
    let values = [
        input.primary_resonant_inductance_h,
        input.primary_resonant_capacitance_f,
        input.secondary_resonant_inductance_h,
        input.secondary_resonant_capacitance_f,
        input.magnetizing_inductance_h,
        input.turns_ratio_primary_to_secondary,
        input.primary_dc_input_voltage_v,
        input.rated_output_power_w,
        input.required_voltage_gain,
        input.controller_minimum_frequency_hz,
        input.controller_maximum_frequency_hz,
    ];
    if values.iter().any(|value| !value.is_finite() || *value <= 0.0) {
        return Err(AuditError(
            "CLLC parameter-design L/C, turns ratio, voltage, power, gain and frequency inputs must be positive and finite"
                .to_string(),
        ));
    }
    if !input.symmetry_relative_tolerance.is_finite()
        || input.symmetry_relative_tolerance < 0.0
        || input.symmetry_relative_tolerance >= 1.0
    {
        return Err(AuditError(
            "CLLC symmetry tolerance must be finite and in [0, 1)".to_string(),
        ));
    }
    if input.controller_minimum_frequency_hz >= input.controller_maximum_frequency_hz {
        return Err(AuditError(
            "controller minimum frequency must be below controller maximum frequency".to_string(),
        ));
    }

    let fr1 = 1.0
        / (2.0 * std::f64::consts::PI
            * (input.primary_resonant_inductance_h * input.primary_resonant_capacitance_f)
                .sqrt());
    let fr2 = 1.0
        / (2.0 * std::f64::consts::PI
            * (input.secondary_resonant_inductance_h * input.secondary_resonant_capacitance_f)
                .sqrt());
    let k1 = input.magnetizing_inductance_h / input.primary_resonant_inductance_h;
    let k2 = input.magnetizing_inductance_h
        / (input.turns_ratio_primary_to_secondary.powi(2)
            * input.secondary_resonant_inductance_h);

    if [fr1, fr2, k1, k2]
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(AuditError(
            "derived CLLC resonance frequencies or magnetizing ratios are invalid".to_string(),
        ));
    }

    let symmetric_fr = relative_difference(fr1, fr2) <= input.symmetry_relative_tolerance;
    let symmetric_k = relative_difference(k1, k2) <= input.symmetry_relative_tolerance;
    let symmetry_assumption_valid = symmetric_fr && symmetric_k;
    if !symmetry_assumption_valid {
        return Err(AuditError(format!(
            "paper's symmetric design equations require fr1≈fr2 and k1≈k2; fr relative difference={:.6e}, k relative difference={:.6e}, tolerance={:.6e}",
            relative_difference(fr1, fr2),
            relative_difference(k1, k2),
            input.symmetry_relative_tolerance
        )));
    }

    let k = k1;
    let frequency_window = screen_cllc_low_frequency_boost_window(
        ClLcLowFrequencyBoostWindowInput {
            primary_series_resonant_frequency_hz: fr1,
            secondary_series_resonant_frequency_hz: fr2,
            magnetizing_to_resonant_inductance_ratio: k,
            controller_minimum_frequency_hz: input.controller_minimum_frequency_hz,
            controller_maximum_frequency_hz: input.controller_maximum_frequency_hz,
        },
    )?;

    let fn_lower = frequency_window.effective_lower_bound_hz / fr1;
    let fn_upper = frequency_window.effective_upper_bound_hz / fr1;
    if !fn_lower.is_finite() || !fn_upper.is_finite() || fn_lower <= 0.0 {
        return Err(AuditError(
            "normalized CLLC frequency window is invalid".to_string(),
        ));
    }

    let q_max = 1.0 / ((2.0 * k + 1.0).sqrt() - 1.0);
    let q_cmp = std::f64::consts::PI.powi(3)
        * input.primary_resonant_inductance_h
        * fr1
        * input.rated_output_power_w
        / (4.0 * (input.required_voltage_gain * input.primary_dc_input_voltage_v).powi(2));

    let phase_boundary_lower = 1.0 / (2.0 * k + 1.0).sqrt();
    let unity_gain_phase_boundary_valid =
        fn_lower > phase_boundary_lower && fn_lower < 1.0;
    let q_boundary =
        (k + 1.0).sqrt() / (k * input.required_voltage_gain);

    // Eqs. (16) and (23) express the two design limits at the unity-gain
    // boundary. This is retained as a distinct, conservative design gate.
    let max_l_z1 = 4.0
        * q_max
        * input.primary_dc_input_voltage_v.powi(2)
        / (std::f64::consts::PI.powi(3) * fr1 * input.rated_output_power_w);
    let max_l_z2 = 4.0
        * input.primary_dc_input_voltage_v.powi(2)
        * (k + 1.0).sqrt()
        / (std::f64::consts::PI.powi(3)
            * input.rated_output_power_w
            * k
            * fr1);

    // Eq. (20): A²*M⁴ - M² + (C*B)² = 0, solved as a quadratic
    // in x=M². Use the lower frequency boundary imposed by the actual
    // intersection of the topology-state and controller frequency windows.
    let fn_l = fn_lower;
    let a = 1.0 + 1.0 / k - 1.0 / (k * fn_l.powi(2));
    let b = ((2.0 * k + 1.0) * fn_l
        - (2.0 * k + 2.0) / fn_l
        + 1.0 / fn_l.powi(3))
        / k;
    let c = std::f64::consts::PI.powi(3)
        * fr1
        * input.rated_output_power_w
        * input.primary_resonant_inductance_h
        / (4.0 * input.primary_dc_input_voltage_v.powi(2));
    let cb = c * b;
    let a2 = a * a;
    let discriminant = 1.0 - 4.0 * a2 * cb * cb;

    if [q_max, q_cmp, q_boundary, max_l_z1, max_l_z2, a, b, c, discriminant]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(AuditError(
            "CLLC parameter-design equation produced a non-finite value".to_string(),
        ));
    }

    let maximum_boost_gain = if discriminant < 0.0 {
        None
    } else if a2 <= 1e-24 {
        Some(cb.abs())
    } else {
        let x_max = (1.0 + discriminant.sqrt()) / (2.0 * a2);
        if x_max.is_finite() && x_max > 0.0 {
            Some(x_max.sqrt())
        } else {
            None
        }
    };

    let constant_power_zvs_constraint_passes =
        q_cmp <= q_max && input.primary_resonant_inductance_h <= max_l_z1;
    let unity_gain_zvs_constraint_passes =
        input.primary_resonant_inductance_h <= max_l_z2;
    let maximum_gain_constraint_passes = maximum_boost_gain
        .is_some_and(|maximum| input.required_voltage_gain <= maximum);
    let frequency_constraint_passes =
        frequency_window.feasible && fn_lower < fn_upper && unity_gain_phase_boundary_valid;

    let overall = constant_power_zvs_constraint_passes
        && unity_gain_zvs_constraint_passes
        && maximum_gain_constraint_passes
        && frequency_constraint_passes;

    Ok(ClLcParameterDesignResult {
        primary_series_resonant_frequency_hz: fr1,
        secondary_series_resonant_frequency_hz: fr2,
        primary_magnetizing_ratio_k1: k1,
        secondary_magnetizing_ratio_k2: k2,
        symmetry_assumption_valid,
        low_frequency_boost_window: frequency_window,
        normalized_frequency_lower: fn_lower,
        normalized_frequency_upper: fn_upper,
        critical_quality_factor_q_max: q_max,
        constant_power_quality_factor_q_cmp: q_cmp,
        unity_gain_boundary_quality_factor: q_boundary,
        unity_gain_phase_boundary_valid,
        max_primary_resonant_inductance_constant_power_zvs_h: max_l_z1,
        max_primary_resonant_inductance_unity_gain_zvs_h: max_l_z2,
        maximum_boost_gain,
        constant_power_zvs_constraint_passes,
        unity_gain_zvs_constraint_passes,
        maximum_gain_constraint_passes,
        frequency_constraint_passes,
        overall_paper_constraint_screen_passes: overall,
    })
}

#[cfg(test)]
mod tests {
    use super::{evaluate_cllc_parameter_design, ClLcParameterDesignInput};

    fn input() -> ClLcParameterDesignInput {
        let l1 = 10e-6;
        let l2 = 10e-6;
        let fr = 115_000.0;
        let c1 = 1.0 / ((2.0 * std::f64::consts::PI * fr).powi(2) * l1);
        let c2 = 1.0 / ((2.0 * std::f64::consts::PI * fr).powi(2) * l2);
        ClLcParameterDesignInput {
            primary_resonant_inductance_h: l1,
            primary_resonant_capacitance_f: c1,
            secondary_resonant_inductance_h: l2,
            secondary_resonant_capacitance_f: c2,
            magnetizing_inductance_h: 30e-6,
            turns_ratio_primary_to_secondary: 1.0,
            primary_dc_input_voltage_v: 300.0,
            rated_output_power_w: 2_000.0,
            required_voltage_gain: 1.0,
            controller_minimum_frequency_hz: 75_000.0,
            controller_maximum_frequency_hz: 150_000.0,
            symmetry_relative_tolerance: 1e-9,
        }
    }

    #[test]
    fn computes_paper_critical_quality_and_frequency_window() {
        let result = evaluate_cllc_parameter_design(input()).unwrap();
        assert!((result.primary_series_resonant_frequency_hz - 115_000.0).abs() < 1e-6);
        assert!((result.critical_quality_factor_q_max - 0.6076252185).abs() < 1e-9);
        assert!((result.normalized_frequency_lower - 75_000.0 / 115_000.0).abs() < 1e-12);
        assert!(result.frequency_constraint_passes);
    }

    #[test]
    fn computes_constant_power_and_unity_gain_zvs_limits() {
        let result = evaluate_cllc_parameter_design(input()).unwrap();
        assert!((result.constant_power_quality_factor_q_cmp - 0.1980956566).abs() < 1e-9);
        assert!((result.max_primary_resonant_inductance_constant_power_zvs_h - 30.6733236375e-6).abs() < 1e-14);
        assert!((result.max_primary_resonant_inductance_unity_gain_zvs_h - 33.6537750660e-6).abs() < 1e-14);
        assert!(result.constant_power_zvs_constraint_passes);
        assert!(result.unity_gain_zvs_constraint_passes);
    }

    #[test]
    fn computes_maximum_gain_quadratic_root() {
        let result = evaluate_cllc_parameter_design(input()).unwrap();
        assert!((result.maximum_boost_gain.unwrap() - 1.7987167721).abs() < 1e-9);
        assert!(result.maximum_gain_constraint_passes);
    }

    #[test]
    fn rejects_asymmetry_outside_paper_assumptions() {
        let mut p = input();
        p.secondary_resonant_capacitance_f *= 1.2;
        assert!(evaluate_cllc_parameter_design(p).is_err());
    }

    #[test]
    fn rejects_invalid_or_unbounded_parameters() {
        let mut p = input();
        p.magnetizing_inductance_h = 0.0;
        assert!(evaluate_cllc_parameter_design(p).is_err());

        let mut p = input();
        p.required_voltage_gain = f64::NAN;
        assert!(evaluate_cllc_parameter_design(p).is_err());

        let mut p = input();
        p.controller_minimum_frequency_hz = 160_000.0;
        p.controller_maximum_frequency_hz = 180_000.0;
        assert!(evaluate_cllc_parameter_design(p).is_ok_and(|r| !r.frequency_constraint_passes));
    }
}
