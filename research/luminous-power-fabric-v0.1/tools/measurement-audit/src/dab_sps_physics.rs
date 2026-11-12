use crate::AuditError;

/// Ideal single-phase-shift DAB physics inputs using source-side-referred values.
///
/// For each direction, express the active source voltage as source_voltage_v,
/// refer the destination voltage to that source side as destination_voltage_referred_v,
/// and use the corresponding source-side-referred series inductance. The signed
/// phase shift sets power direction; its magnitude is constrained to (0, pi/2].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DabSpsPhysicsInput {
    pub source_voltage_v: f64,
    pub destination_voltage_referred_v: f64,
    pub series_inductance_referred_h: f64,
    pub switching_frequency_hz: f64,
    /// Positive is source-to-destination; negative reverses power flow.
    pub signed_phase_shift_rad: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DabSpsPhysicsResult {
    pub voltage_ratio_k: f64,
    pub phase_shift_magnitude_rad: f64,
    pub signed_exact_power_w: f64,
    pub maximum_ideal_power_w: f64,
    pub current_scale_a: f64,
    pub initial_current_a: f64,
    pub transition_current_a: f64,
    pub current_ripple_a: f64,
    pub inductor_rms_current_a: f64,
    pub inductor_peak_current_a: f64,
    pub fha_power_w: f64,
    pub fha_overestimation_percent: f64,
    pub minimum_zvs_phase_shift_rad: f64,
    /// This is the paper's SPS switching-current-polarity criterion, not a
    /// device-specific Coss/dead-time energy proof.
    pub meets_analytic_zvs_polarity_screen: bool,
}

impl DabSpsPhysicsResult {
    pub const fn claim_ceiling(&self) -> &'static str {
        "IDEAL_DAB_SPS_PIECEWISE_POWER_RMS_AND_POLARITY_SCREEN_NOT_DEVICE_ZVS_OR_QUALIFICATION"
    }
}

/// Evaluate the lossless piecewise-linear SPS DAB analytical model.
///
/// Based on Mahmoud et al., Engineering Proceedings 2026, 142(1), 20,
/// Sections 3.1–3.3 and 4.2:
///
/// k = V2_referred / V1
/// c = V1 / (2*pi*fs*L)
/// i0 = c * (-pi*(1-k)/2 - k*phi)
/// delta_i = c * (1+k)*phi
/// i1 = i0 + delta_i
///
/// P_exact = V1*V2_referred*phi*(pi-phi)/(2*pi²*fs*L)
///
/// Irms² = [phi*(i0² + i0*delta_i + delta_i²/3) +
///          (pi-phi)*i1²] / pi
///
/// P_FHA = V1*V2_referred*sin(phi)/(2*pi*fs*L)
/// phi_ZVS,min = (pi/2)*max(0, 1-k)
///
/// The current formula is the ideal lossless closed form. With appreciable
/// equivalent series resistance, use a periodicity-solved piecewise exponential
/// model; this helper does not approximate that resistive case as exact.
pub fn evaluate_dab_sps_physics(
    input: DabSpsPhysicsInput,
) -> Result<DabSpsPhysicsResult, AuditError> {
    let values = [
        input.source_voltage_v,
        input.destination_voltage_referred_v,
        input.series_inductance_referred_h,
        input.switching_frequency_hz,
        input.signed_phase_shift_rad,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(AuditError(
            "DAB SPS physical model inputs must be finite".to_string(),
        ));
    }
    if input.source_voltage_v <= 0.0
        || input.destination_voltage_referred_v <= 0.0
        || input.series_inductance_referred_h <= 0.0
        || input.switching_frequency_hz <= 0.0
    {
        return Err(AuditError(
            "DAB voltages, referred inductance and switching frequency must be positive"
                .to_string(),
        ));
    }

    let phi = input.signed_phase_shift_rad.abs();
    if phi <= 0.0 || phi > std::f64::consts::PI / 2.0 {
        return Err(AuditError(
            "DAB SPS phase-shift magnitude must be in (0, pi/2] for this model"
                .to_string(),
        ));
    }

    let k = input.destination_voltage_referred_v / input.source_voltage_v;
    let c = input.source_voltage_v
        / (2.0
            * std::f64::consts::PI
            * input.switching_frequency_hz
            * input.series_inductance_referred_h);
    let i0 = c * (-std::f64::consts::PI * (1.0 - k) / 2.0 - k * phi);
    let delta_i = c * (1.0 + k) * phi;
    let i1 = i0 + delta_i;

    let exact_power_magnitude = input.source_voltage_v
        * input.destination_voltage_referred_v
        * phi
        * (std::f64::consts::PI - phi)
        / (2.0
            * std::f64::consts::PI.powi(2)
            * input.switching_frequency_hz
            * input.series_inductance_referred_h);
    let maximum_ideal_power = input.source_voltage_v
        * input.destination_voltage_referred_v
        / (8.0 * input.switching_frequency_hz * input.series_inductance_referred_h);

    let rms_squared = (phi
        * (i0 * i0 + i0 * delta_i + delta_i * delta_i / 3.0)
        + (std::f64::consts::PI - phi) * i1 * i1)
        / std::f64::consts::PI;
    if !rms_squared.is_finite() || rms_squared < 0.0 {
        return Err(AuditError(
            "DAB SPS RMS-current expression produced an invalid value".to_string(),
        ));
    }

    let fha_power_magnitude = input.source_voltage_v
        * input.destination_voltage_referred_v
        * phi.sin()
        / (2.0
            * std::f64::consts::PI
            * input.switching_frequency_hz
            * input.series_inductance_referred_h);
    let fha_overestimation_percent =
        100.0 * (fha_power_magnitude - exact_power_magnitude) / exact_power_magnitude;
    let minimum_zvs_phase_shift = std::f64::consts::PI / 2.0 * (1.0 - k).max(0.0);
    let power_sign = input.signed_phase_shift_rad.signum();

    let outputs = [
        k,
        c,
        i0,
        delta_i,
        i1,
        exact_power_magnitude,
        maximum_ideal_power,
        fha_power_magnitude,
        fha_overestimation_percent,
        minimum_zvs_phase_shift,
    ];
    if outputs.iter().any(|value| !value.is_finite()) {
        return Err(AuditError(
            "DAB SPS model produced a non-finite result".to_string(),
        ));
    }
    if exact_power_magnitude <= 0.0 || maximum_ideal_power <= 0.0 {
        return Err(AuditError(
            "DAB SPS power must be positive for the declared phase magnitude".to_string(),
        ));
    }

    Ok(DabSpsPhysicsResult {
        voltage_ratio_k: k,
        phase_shift_magnitude_rad: phi,
        signed_exact_power_w: power_sign * exact_power_magnitude,
        maximum_ideal_power_w: maximum_ideal_power,
        current_scale_a: c,
        initial_current_a: i0,
        transition_current_a: i1,
        current_ripple_a: delta_i,
        inductor_rms_current_a: rms_squared.sqrt(),
        inductor_peak_current_a: i0.abs().max(i1.abs()),
        fha_power_w: power_sign * fha_power_magnitude,
        fha_overestimation_percent,
        minimum_zvs_phase_shift_rad: minimum_zvs_phase_shift,
        meets_analytic_zvs_polarity_screen: phi >= minimum_zvs_phase_shift,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DabSpsThermalInput {
    /// Lumped equivalent series resistance for the inductor/conduction path.
    pub equivalent_resistance_ohm: f64,
    /// Hard-switching loss at this exact operating point, supplied by a
    /// separately justified device/loss model or measurement.
    pub hard_switching_loss_w: f64,
    /// Residual switching-loss fraction after the analytic ZVS polarity screen.
    /// The cited study uses 0.05 as an illustrative nominal assumption.
    pub residual_zvs_loss_fraction: f64,
    pub ambient_temperature_c: f64,
    pub junction_to_ambient_thermal_resistance_k_per_w: f64,
    pub junction_temperature_limit_c: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DabSpsThermalResult {
    pub conduction_loss_w: f64,
    pub switching_loss_w: f64,
    pub total_loss_w: f64,
    pub estimated_junction_temperature_c: f64,
    pub within_declared_thermal_limit: bool,
}

impl DabSpsThermalResult {
    pub const fn claim_ceiling(&self) -> &'static str {
        "LUMPED_USER_PARAMETERIZED_THERMAL_SCREEN_NOT_DEVICE_THERMAL_VALIDATION"
    }
}

/// Apply a user-supplied one-node loss/thermal screen to an SPS model result.
///
/// When the analytic ZVS polarity criterion is met, the hard-switching loss is
/// multiplied by the supplied residual fraction; otherwise the full supplied
/// hard-switching loss is used. The residual fraction is an explicit assumption,
/// not a claim that the converter has physically achieved ZVS.
pub fn screen_dab_sps_lumped_thermal(
    physics: &DabSpsPhysicsResult,
    input: DabSpsThermalInput,
) -> Result<DabSpsThermalResult, AuditError> {
    let values = [
        input.equivalent_resistance_ohm,
        input.hard_switching_loss_w,
        input.residual_zvs_loss_fraction,
        input.ambient_temperature_c,
        input.junction_to_ambient_thermal_resistance_k_per_w,
        input.junction_temperature_limit_c,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(AuditError(
            "DAB lumped thermal-screen inputs must be finite".to_string(),
        ));
    }
    if input.equivalent_resistance_ohm < 0.0
        || input.hard_switching_loss_w < 0.0
        || !(0.0..=1.0).contains(&input.residual_zvs_loss_fraction)
        || input.junction_to_ambient_thermal_resistance_k_per_w < 0.0
    {
        return Err(AuditError(
            "DAB thermal-screen resistance, losses and residual fraction are outside their valid ranges"
                .to_string(),
        ));
    }

    let conduction_loss = physics.inductor_rms_current_a.powi(2)
        * input.equivalent_resistance_ohm;
    let switching_loss = if physics.meets_analytic_zvs_polarity_screen {
        input.hard_switching_loss_w * input.residual_zvs_loss_fraction
    } else {
        input.hard_switching_loss_w
    };
    let total_loss = conduction_loss + switching_loss;
    let junction_temperature = input.ambient_temperature_c
        + total_loss * input.junction_to_ambient_thermal_resistance_k_per_w;

    if !conduction_loss.is_finite()
        || !switching_loss.is_finite()
        || !total_loss.is_finite()
        || !junction_temperature.is_finite()
    {
        return Err(AuditError(
            "DAB lumped thermal-screen calculation overflowed".to_string(),
        ));
    }

    Ok(DabSpsThermalResult {
        conduction_loss_w: conduction_loss,
        switching_loss_w: switching_loss,
        total_loss_w: total_loss,
        estimated_junction_temperature_c: junction_temperature,
        within_declared_thermal_limit: junction_temperature
            <= input.junction_temperature_limit_c,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_dab_sps_physics, screen_dab_sps_lumped_thermal, DabSpsPhysicsInput,
        DabSpsThermalInput,
    };

    fn baseline(phase_degrees: f64, k: f64) -> DabSpsPhysicsInput {
        DabSpsPhysicsInput {
            source_voltage_v: 100.0,
            destination_voltage_referred_v: 100.0 * k,
            series_inductance_referred_h: 20e-6,
            switching_frequency_hz: 50_000.0,
            signed_phase_shift_rad: phase_degrees.to_radians(),
        }
    }

    #[test]
    fn reproduces_published_15_degree_power_and_rms_current() {
        let result = evaluate_dab_sps_physics(baseline(15.0, 1.0)).unwrap();
        assert!((result.signed_exact_power_w - 381.9444444).abs() < 1e-6);
        assert!((result.inductor_rms_current_a - 4.04927215).abs() < 1e-7);
        assert!((result.fha_overestimation_percent - 7.851).abs() < 0.01);
    }

    #[test]
    fn reproduces_published_45_degree_power_and_rms_current() {
        let result = evaluate_dab_sps_physics(baseline(45.0, 1.0)).unwrap();
        assert!((result.signed_exact_power_w - 937.5).abs() < 1e-9);
        assert!((result.inductor_rms_current_a - 11.4108866).abs() < 1e-6);
        assert!((result.fha_overestimation_percent - 20.04).abs() < 0.02);
    }

    #[test]
    fn reproduces_published_high_phase_current_stress() {
        let result = evaluate_dab_sps_physics(baseline(88.0, 1.0)).unwrap();
        assert!((result.inductor_rms_current_a - 20.0693813).abs() < 1e-6);
    }

    #[test]
    fn computes_voltage_ratio_dependent_zvs_polarity_boundary() {
        let k070 = evaluate_dab_sps_physics(baseline(28.0, 0.70)).unwrap();
        let k085 = evaluate_dab_sps_physics(baseline(14.0, 0.85)).unwrap();
        let k100 = evaluate_dab_sps_physics(baseline(1.0, 1.0)).unwrap();

        assert!((k070.minimum_zvs_phase_shift_rad.to_degrees() - 27.0).abs() < 1e-12);
        assert!((k085.minimum_zvs_phase_shift_rad.to_degrees() - 13.5).abs() < 1e-12);
        assert_eq!(k100.minimum_zvs_phase_shift_rad, 0.0);
        assert!(k070.meets_analytic_zvs_polarity_screen);
        assert!(k085.meets_analytic_zvs_polarity_screen);
        assert!(k100.meets_analytic_zvs_polarity_screen);
    }

    #[test]
    fn rejects_phase_below_zvs_screen_without_claiming_zvs() {
        let result = evaluate_dab_sps_physics(baseline(15.0, 0.70)).unwrap();
        assert!(!result.meets_analytic_zvs_polarity_screen);
    }

    #[test]
    fn phase_sign_reverses_power_but_not_rms_current() {
        let forward = evaluate_dab_sps_physics(baseline(30.0, 1.0)).unwrap();
        let reverse = evaluate_dab_sps_physics(baseline(-30.0, 1.0)).unwrap();

        assert!((forward.signed_exact_power_w + reverse.signed_exact_power_w).abs() < 1e-9);
        assert!((forward.inductor_rms_current_a - reverse.inductor_rms_current_a).abs() < 1e-12);
    }

    #[test]
    fn lumped_thermal_screen_flags_high_phase_stress() {
        let result = evaluate_dab_sps_physics(baseline(45.0, 1.0)).unwrap();
        let thermal = screen_dab_sps_lumped_thermal(
            &result,
            DabSpsThermalInput {
                equivalent_resistance_ohm: 0.05,
                hard_switching_loss_w: 0.5,
                residual_zvs_loss_fraction: 0.05,
                ambient_temperature_c: 40.0,
                junction_to_ambient_thermal_resistance_k_per_w: 20.0,
                junction_temperature_limit_c: 125.0,
            },
        )
        .unwrap();

        assert!((thermal.conduction_loss_w - 6.5104167).abs() < 0.001);
        assert!(thermal.estimated_junction_temperature_c > 125.0);
        assert!(!thermal.within_declared_thermal_limit);
    }

    #[test]
    fn invalid_parameters_fail_closed() {
        let mut p = baseline(30.0, 1.0);
        p.series_inductance_referred_h = 0.0;
        assert!(evaluate_dab_sps_physics(p).is_err());

        let mut p = baseline(30.0, 1.0);
        p.signed_phase_shift_rad = 0.0;
        assert!(evaluate_dab_sps_physics(p).is_err());

        let mut p = baseline(30.0, 1.0);
        p.signed_phase_shift_rad = std::f64::consts::PI;
        assert!(evaluate_dab_sps_physics(p).is_err());
    }
}
