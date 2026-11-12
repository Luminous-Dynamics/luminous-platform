use crate::AuditError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DabSpsResistiveInput {
    pub source_voltage_v: f64,
    pub destination_voltage_referred_v: f64,
    pub series_inductance_referred_h: f64,
    pub switching_frequency_hz: f64,
    /// Positive is source-to-destination; negative reverses power flow.
    pub signed_phase_shift_rad: f64,
    /// Effective series resistance in the leakage-inductance current path.
    pub equivalent_series_resistance_ohm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DabSpsResistiveResult {
    pub voltage_ratio_k: f64,
    pub cycle_period_s: f64,
    pub steady_state_initial_current_a: f64,
    pub steady_state_final_current_a: f64,
    pub periodicity_residual_a: f64,
    pub inductor_rms_current_a: f64,
    pub inductor_peak_current_a: f64,
    /// Average bridge port powers using the two bridge-square-wave voltage signs.
    /// Positive means power flows from the corresponding DC port into the AC bridge.
    pub source_port_power_w: f64,
    pub destination_port_power_w: f64,
    pub series_resistive_loss_w: f64,
    /// source_port_power - destination_port_power - I_rms²*R.
    pub energy_balance_residual_w: f64,
}

impl DabSpsResistiveResult {
    pub const fn claim_ceiling(&self) -> &'static str {
        "PIECEWISE_RL_SPS_PERIODIC_STEADY_STATE_SCREEN_NOT_SWITCHING_DEVICE_OR_QUALIFICATION"
    }
}

#[derive(Debug, Clone, Copy)]
struct Segment {
    source_bridge_voltage_v: f64,
    destination_bridge_voltage_v: f64,
    duration_s: f64,
}

#[derive(Debug, Clone, Copy)]
struct SegmentIntegral {
    end_current_a: f64,
    current_integral_as: f64,
    current_squared_integral_a2s: f64,
}

fn propagate_segment(
    start_current_a: f64,
    segment: Segment,
    inductance_h: f64,
    resistance_ohm: f64,
) -> Result<SegmentIntegral, AuditError> {
    let voltage_v =
        segment.source_bridge_voltage_v - segment.destination_bridge_voltage_v;
    let dt = segment.duration_s;

    if resistance_ohm == 0.0 {
        let slope_a_per_s = voltage_v / inductance_h;
        let end_current_a = start_current_a + slope_a_per_s * dt;
        let current_integral_as =
            start_current_a * dt + 0.5 * slope_a_per_s * dt * dt;
        let current_squared_integral_a2s = start_current_a * start_current_a * dt
            + start_current_a * slope_a_per_s * dt * dt
            + slope_a_per_s * slope_a_per_s * dt * dt * dt / 3.0;

        return Ok(SegmentIntegral {
            end_current_a,
            current_integral_as,
            current_squared_integral_a2s,
        });
    }

    let tau_s = inductance_h / resistance_ohm;
    let exponent = -dt / tau_s;
    let decay = exponent.exp();
    let one_minus_decay = -exponent.exp_m1();
    let equilibrium_current_a = voltage_v / resistance_ohm;
    let difference_a = start_current_a - equilibrium_current_a;

    let end_current_a = equilibrium_current_a + difference_a * decay;
    let current_integral_as = equilibrium_current_a * dt
        + difference_a * tau_s * one_minus_decay;
    let current_squared_integral_a2s = equilibrium_current_a.powi(2) * dt
        + 2.0 * equilibrium_current_a * difference_a * tau_s * one_minus_decay
        + difference_a.powi(2) * (tau_s / 2.0) * (1.0 - decay * decay);

    let values = [
        end_current_a,
        current_integral_as,
        current_squared_integral_a2s,
    ];
    if values.iter().any(|value| !value.is_finite())
        || current_squared_integral_a2s < 0.0
    {
        return Err(AuditError(
            "DAB resistive segment integration produced an invalid result".to_string(),
        ));
    }

    Ok(SegmentIntegral {
        end_current_a,
        current_integral_as,
        current_squared_integral_a2s,
    })
}

fn make_segments(
    source_voltage_v: f64,
    destination_voltage_v: f64,
    switching_frequency_hz: f64,
    phase_shift_rad: f64,
) -> [Segment; 4] {
    let phi = phase_shift_rad.abs();
    let omega = 2.0 * std::f64::consts::PI * switching_frequency_hz;
    let short_duration_s = phi / omega;
    let long_duration_s = (std::f64::consts::PI - phi) / omega;

    if phase_shift_rad >= 0.0 {
        // Secondary bridge lags the primary: +V1/-V2, +V1/+V2,
        // -V1/+V2, -V1/-V2.
        [
            Segment {
                source_bridge_voltage_v: source_voltage_v,
                destination_bridge_voltage_v: -destination_voltage_v,
                duration_s: short_duration_s,
            },
            Segment {
                source_bridge_voltage_v: source_voltage_v,
                destination_bridge_voltage_v: destination_voltage_v,
                duration_s: long_duration_s,
            },
            Segment {
                source_bridge_voltage_v: -source_voltage_v,
                destination_bridge_voltage_v: destination_voltage_v,
                duration_s: short_duration_s,
            },
            Segment {
                source_bridge_voltage_v: -source_voltage_v,
                destination_bridge_voltage_v: -destination_voltage_v,
                duration_s: long_duration_s,
            },
        ]
    } else {
        // Secondary bridge leads the primary: +V1/+V2, +V1/-V2,
        // -V1/-V2, -V1/+V2.
        [
            Segment {
                source_bridge_voltage_v: source_voltage_v,
                destination_bridge_voltage_v: destination_voltage_v,
                duration_s: long_duration_s,
            },
            Segment {
                source_bridge_voltage_v: source_voltage_v,
                destination_bridge_voltage_v: -destination_voltage_v,
                duration_s: short_duration_s,
            },
            Segment {
                source_bridge_voltage_v: -source_voltage_v,
                destination_bridge_voltage_v: -destination_voltage_v,
                duration_s: long_duration_s,
            },
            Segment {
                source_bridge_voltage_v: -source_voltage_v,
                destination_bridge_voltage_v: destination_voltage_v,
                duration_s: short_duration_s,
            },
        ]
    }
}

/// Integrate the switched RL model over one period and solve the periodic
/// steady-state initial current.
///
/// For R>0, each segment has the exact affine map
/// i_end = exp(-R*dt/L)*i_start + (vL/R)*(1-exp(-R*dt/L)).
/// The four maps are composed to solve i(T)=i(0) without a guessed initial
/// current or iterative bisection. For R=0, the lossless anti-periodic initial
/// current from the closed-form SPS model is used.
///
/// This is an ideal bridge square-wave model with one lumped series R-L branch.
/// It does not include dead time, switch Coss, reverse recovery, transformer
/// magnetizing/leakage partition, parasitic capacitance, EMI or controller delay.
pub fn evaluate_dab_sps_resistive(
    input: DabSpsResistiveInput,
) -> Result<DabSpsResistiveResult, AuditError> {
    let values = [
        input.source_voltage_v,
        input.destination_voltage_referred_v,
        input.series_inductance_referred_h,
        input.switching_frequency_hz,
        input.signed_phase_shift_rad,
        input.equivalent_series_resistance_ohm,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(AuditError(
            "DAB resistive model inputs must be finite".to_string(),
        ));
    }
    if input.source_voltage_v <= 0.0
        || input.destination_voltage_referred_v <= 0.0
        || input.series_inductance_referred_h <= 0.0
        || input.switching_frequency_hz <= 0.0
        || input.equivalent_series_resistance_ohm < 0.0
    {
        return Err(AuditError(
            "DAB voltages, inductance and frequency must be positive; resistance must be non-negative"
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
    let period_s = 1.0 / input.switching_frequency_hz;
    let segments = make_segments(
        input.source_voltage_v,
        input.destination_voltage_referred_v,
        input.switching_frequency_hz,
        input.signed_phase_shift_rad,
    );

    let initial_current_a = if input.equivalent_series_resistance_ohm == 0.0 {
        let c = input.source_voltage_v
            / (2.0
                * std::f64::consts::PI
                * input.switching_frequency_hz
                * input.series_inductance_referred_h);
        c * (-std::f64::consts::PI * (1.0 - k) / 2.0 - k * phi)
    } else {
        let mut accumulated_b_a = 0.0_f64;
        let mut accumulated_a = 1.0_f64;

        for segment in segments {
            let dt = segment.duration_s;
            let voltage_v =
                segment.source_bridge_voltage_v - segment.destination_bridge_voltage_v;
            let exponent = -input.equivalent_series_resistance_ohm * dt
                / input.series_inductance_referred_h;
            let decay = exponent.exp();
            let one_minus_decay = -exponent.exp_m1();
            let forcing_a = (voltage_v / input.equivalent_series_resistance_ohm)
                * one_minus_decay;

            accumulated_b_a = decay * accumulated_b_a + forcing_a;
            accumulated_a *= decay;
        }

        let total_exponent = -input.equivalent_series_resistance_ohm * period_s
            / input.series_inductance_referred_h;
        let one_minus_period_decay = -total_exponent.exp_m1();
        if one_minus_period_decay <= 0.0 || !one_minus_period_decay.is_finite() {
            return Err(AuditError(
                "DAB periodicity denominator is zero or non-finite".to_string(),
            ));
        }
        // accumulated_a is retained for checking that the composed map has
        // the expected one-period decay under constant R/L.
        if !accumulated_a.is_finite() {
            return Err(AuditError(
                "DAB periodicity map is non-finite".to_string(),
            ));
        }
        accumulated_b_a / one_minus_period_decay
    };

    if !initial_current_a.is_finite() {
        return Err(AuditError(
            "DAB periodic steady-state initial current is not finite".to_string(),
        ));
    }

    let mut current_a = initial_current_a;
    let mut peak_current_a = current_a.abs();
    let mut total_current_integral_as = 0.0_f64;
    let mut total_current_squared_integral_a2s = 0.0_f64;
    let mut source_energy_j = 0.0_f64;
    let mut destination_energy_j = 0.0_f64;

    for segment in segments {
        let integrated = propagate_segment(
            current_a,
            segment,
            input.series_inductance_referred_h,
            input.equivalent_series_resistance_ohm,
        )?;
        total_current_integral_as += integrated.current_integral_as;
        total_current_squared_integral_a2s += integrated.current_squared_integral_a2s;
        source_energy_j +=
            segment.source_bridge_voltage_v * integrated.current_integral_as;
        destination_energy_j +=
            segment.destination_bridge_voltage_v * integrated.current_integral_as;
        current_a = integrated.end_current_a;
        peak_current_a = peak_current_a.max(current_a.abs());
    }

    let periodicity_residual_a = current_a - initial_current_a;
    let inductor_rms_current_a =
        (total_current_squared_integral_a2s / period_s).sqrt();
    let series_resistive_loss_w =
        input.equivalent_series_resistance_ohm * inductor_rms_current_a.powi(2);
    let source_port_power_w = source_energy_j / period_s;
    let destination_port_power_w = destination_energy_j / period_s;
    let energy_balance_residual_w =
        source_port_power_w - destination_port_power_w - series_resistive_loss_w;

    let outputs = [
        period_s,
        current_a,
        periodicity_residual_a,
        inductor_rms_current_a,
        peak_current_a,
        series_resistive_loss_w,
        source_port_power_w,
        destination_port_power_w,
        energy_balance_residual_w,
        total_current_integral_as,
    ];
    if outputs.iter().any(|value| !value.is_finite())
        || inductor_rms_current_a < 0.0
        || peak_current_a < 0.0
    {
        return Err(AuditError(
            "DAB resistive periodic model produced an invalid result".to_string(),
        ));
    }

    Ok(DabSpsResistiveResult {
        voltage_ratio_k: k,
        cycle_period_s: period_s,
        steady_state_initial_current_a: initial_current_a,
        steady_state_final_current_a: current_a,
        periodicity_residual_a,
        inductor_rms_current_a,
        inductor_peak_current_a: peak_current_a,
        source_port_power_w,
        destination_port_power_w,
        series_resistive_loss_w,
        energy_balance_residual_w,
    })
}

#[cfg(test)]
mod tests {
    use super::{evaluate_dab_sps_resistive, DabSpsResistiveInput};

    fn input(phase_degrees: f64, resistance_ohm: f64) -> DabSpsResistiveInput {
        DabSpsResistiveInput {
            source_voltage_v: 100.0,
            destination_voltage_referred_v: 100.0,
            series_inductance_referred_h: 20e-6,
            switching_frequency_hz: 50_000.0,
            signed_phase_shift_rad: phase_degrees.to_radians(),
            equivalent_series_resistance_ohm: resistance_ohm,
        }
    }

    #[test]
    fn lossless_limit_matches_closed_form_power_and_rms() {
        let exact = evaluate_dab_sps_resistive(input(15.0, 0.0)).unwrap();
        assert!((exact.destination_port_power_w - 381.9444444).abs() < 1e-6);
        assert!((exact.source_port_power_w - exact.destination_port_power_w).abs() < 1e-9);
        assert!((exact.inductor_rms_current_a - 4.04927215).abs() < 1e-7);
        assert!(exact.periodicity_residual_a.abs() < 1e-9);
    }

    #[test]
    fn reverse_phase_reverses_both_port_power_signs() {
        let forward = evaluate_dab_sps_resistive(input(30.0, 0.0)).unwrap();
        let reverse = evaluate_dab_sps_resistive(input(-30.0, 0.0)).unwrap();

        assert!(forward.source_port_power_w > 0.0);
        assert!(forward.destination_port_power_w > 0.0);
        assert!(reverse.source_port_power_w < 0.0);
        assert!(reverse.destination_port_power_w < 0.0);
        assert!((forward.destination_port_power_w + reverse.destination_port_power_w).abs() < 1e-8);
        assert!((forward.inductor_rms_current_a - reverse.inductor_rms_current_a).abs() < 1e-9);
    }

    #[test]
    fn resistive_periodic_solution_closes_energy_balance() {
        let result = evaluate_dab_sps_resistive(input(45.0, 0.05)).unwrap();
        assert!(result.series_resistive_loss_w > 0.0);
        assert!(result.destination_port_power_w > 0.0);
        assert!(
            result.energy_balance_residual_w.abs()
                < 1e-8 * result.source_port_power_w.abs().max(1.0)
        );
        assert!(result.periodicity_residual_a.abs() < 1e-9);
    }

    #[test]
    fn resistive_model_uses_periodic_fixed_point() {
        let low_r = evaluate_dab_sps_resistive(input(45.0, 1e-3)).unwrap();
        let higher_r = evaluate_dab_sps_resistive(input(45.0, 0.05)).unwrap();

        assert!(low_r.periodicity_residual_a.abs() < 1e-9);
        assert!(higher_r.periodicity_residual_a.abs() < 1e-9);
        assert!(higher_r.series_resistive_loss_w > low_r.series_resistive_loss_w);
    }

    #[test]
    fn rejects_invalid_resistive_model_parameters() {
        let mut p = input(30.0, 0.05);
        p.equivalent_series_resistance_ohm = -0.1;
        assert!(evaluate_dab_sps_resistive(p).is_err());

        let mut p = input(30.0, 0.05);
        p.signed_phase_shift_rad = 0.0;
        assert!(evaluate_dab_sps_resistive(p).is_err());

        let mut p = input(30.0, 0.05);
        p.switching_frequency_hz = 0.0;
        assert!(evaluate_dab_sps_resistive(p).is_err());
    }
}
