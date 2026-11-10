use crate::AuditError;

/// Steady DC power-flow result for a two-wire path with declared loop resistance.
#[derive(Debug, Clone, PartialEq)]
pub struct DcRailPathInput {
    /// Useful power delivered at the load terminals.
    pub delivered_load_power_w: f64,
    /// Nominal voltage at the load terminals, not at the source.
    pub load_voltage_v: f64,
    /// End-to-end loop resistance including both conductors and declared joints.
    pub loop_resistance_ohm: f64,
    /// Maximum permitted conductor drop as a fraction of load voltage.
    pub maximum_voltage_drop_fraction: f64,
    /// Maximum permitted conductor dissipation as a fraction of useful load power.
    pub maximum_conduction_loss_fraction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DcRailPathResult {
    pub delivered_load_power_w: f64,
    pub load_voltage_v: f64,
    pub load_current_a: f64,
    pub loop_resistance_ohm: f64,
    pub source_voltage_required_v: f64,
    pub conductor_voltage_drop_v: f64,
    pub conductor_voltage_drop_percent: f64,
    pub conductor_loss_w: f64,
    pub source_input_power_w: f64,
    pub path_efficiency_percent: f64,
    pub maximum_loop_resistance_for_voltage_drop_ohm: f64,
    pub maximum_loop_resistance_for_conduction_loss_ohm: f64,
    pub within_declared_drop_limit: bool,
    pub within_declared_loss_limit: bool,
}

impl DcRailPathResult {
    pub const fn claim_ceiling(&self) -> &'static str {
        "STEADY_DC_TWO_WIRE_RESISTIVE_PATH_SCREEN_ONLY_NOT_THERMAL_EMI_OR_INSTALLATION_QUALIFICATION"
    }
}

/// Evaluate a constant-power load fed through a resistive DC conductor loop.
///
/// I_load = P_load / V_load
/// P_conductor = I_load^2 * R_loop
/// V_source = V_load + I_load * R_loop
///
/// This is useful when comparing 50 V, 12.5 V and 6.25 V distribution choices,
/// but each rail must use its own measured or justified loop resistance. It
/// must not assume equal resistance or identical geometry across architectures.
pub fn evaluate_dc_rail_path(
    input: DcRailPathInput,
) -> Result<DcRailPathResult, AuditError> {
    let values = [
        input.delivered_load_power_w,
        input.load_voltage_v,
        input.loop_resistance_ohm,
        input.maximum_voltage_drop_fraction,
        input.maximum_conduction_loss_fraction,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(AuditError(
            "DC rail-path inputs must all be finite".to_string(),
        ));
    }
    if input.delivered_load_power_w <= 0.0
        || input.load_voltage_v <= 0.0
        || input.loop_resistance_ohm < 0.0
        || input.maximum_voltage_drop_fraction <= 0.0
        || input.maximum_voltage_drop_fraction >= 1.0
        || input.maximum_conduction_loss_fraction <= 0.0
        || input.maximum_conduction_loss_fraction >= 1.0
    {
        return Err(AuditError(
            "DC rail-path power, voltage and limits must be positive; loop resistance must be non-negative"
                .to_string(),
        ));
    }

    let load_current_a = input.delivered_load_power_w / input.load_voltage_v;
    let conductor_voltage_drop_v = load_current_a * input.loop_resistance_ohm;
    let conductor_loss_w =
        load_current_a * load_current_a * input.loop_resistance_ohm;
    let source_voltage_required_v = input.load_voltage_v + conductor_voltage_drop_v;
    let source_input_power_w = input.delivered_load_power_w + conductor_loss_w;
    let conductor_voltage_drop_percent =
        100.0 * conductor_voltage_drop_v / input.load_voltage_v;
    let path_efficiency_percent =
        100.0 * input.delivered_load_power_w / source_input_power_w;
    let maximum_loop_resistance_for_voltage_drop_ohm =
        input.maximum_voltage_drop_fraction * input.load_voltage_v / load_current_a;
    let maximum_loop_resistance_for_conduction_loss_ohm =
        input.maximum_conduction_loss_fraction * input.delivered_load_power_w
            / (load_current_a * load_current_a);

    let derived = [
        load_current_a,
        conductor_voltage_drop_v,
        conductor_loss_w,
        source_voltage_required_v,
        source_input_power_w,
        conductor_voltage_drop_percent,
        path_efficiency_percent,
        maximum_loop_resistance_for_voltage_drop_ohm,
        maximum_loop_resistance_for_conduction_loss_ohm,
    ];
    if derived.iter().any(|value| !value.is_finite() || *value < 0.0)
        || load_current_a <= 0.0
        || source_voltage_required_v <= 0.0
        || source_input_power_w <= 0.0
        || path_efficiency_percent <= 0.0
        || path_efficiency_percent > 100.0
        || maximum_loop_resistance_for_voltage_drop_ohm <= 0.0
        || maximum_loop_resistance_for_conduction_loss_ohm <= 0.0
    {
        return Err(AuditError(
            "derived DC rail-path result is non-finite or outside its physical domain".to_string(),
        ));
    }

    Ok(DcRailPathResult {
        delivered_load_power_w: input.delivered_load_power_w,
        load_voltage_v: input.load_voltage_v,
        load_current_a,
        loop_resistance_ohm: input.loop_resistance_ohm,
        source_voltage_required_v,
        conductor_voltage_drop_v,
        conductor_voltage_drop_percent,
        conductor_loss_w,
        source_input_power_w,
        path_efficiency_percent,
        maximum_loop_resistance_for_voltage_drop_ohm,
        maximum_loop_resistance_for_conduction_loss_ohm,
        within_declared_drop_limit: conductor_voltage_drop_percent
            <= input.maximum_voltage_drop_fraction * 100.0,
        within_declared_loss_limit: conductor_loss_w
            <= input.maximum_conduction_loss_fraction * input.delivered_load_power_w,
    })
}

#[cfg(test)]
mod tests {
    use super::{evaluate_dc_rail_path, DcRailPathInput};

    fn input(voltage: f64, resistance: f64) -> DcRailPathInput {
        DcRailPathInput {
            delivered_load_power_w: 6000.0,
            load_voltage_v: voltage,
            loop_resistance_ohm: resistance,
            maximum_voltage_drop_fraction: 0.02,
            maximum_conduction_loss_fraction: 0.02,
        }
    }

    #[test]
    fn computes_current_and_conductor_loss_for_three_rail_levels() {
        let at_50v = evaluate_dc_rail_path(input(50.0, 1.0e-3)).unwrap();
        let at_12v5 = evaluate_dc_rail_path(input(12.5, 1.0e-3)).unwrap();
        let at_6v25 = evaluate_dc_rail_path(input(6.25, 1.0e-3)).unwrap();

        assert!((at_50v.load_current_a - 120.0).abs() < 1.0e-9);
        assert!((at_12v5.load_current_a - 480.0).abs() < 1.0e-9);
        assert!((at_6v25.load_current_a - 960.0).abs() < 1.0e-9);
        assert!((at_12v5.conductor_loss_w / at_50v.conductor_loss_w - 16.0).abs() < 1.0e-9);
        assert!((at_6v25.conductor_loss_w / at_50v.conductor_loss_w - 64.0).abs() < 1.0e-9);
    }

    #[test]
    fn zero_resistance_path_has_zero_drop_and_loss() {
        let report = evaluate_dc_rail_path(input(50.0, 0.0)).unwrap();

        assert_eq!(report.conductor_loss_w, 0.0);
        assert_eq!(report.conductor_voltage_drop_v, 0.0);
        assert_eq!(report.path_efficiency_percent, 100.0);
    }

    #[test]
    fn returns_resistance_budgets_for_declared_limits() {
        let report = evaluate_dc_rail_path(input(50.0, 1.0e-3)).unwrap();

        assert!((report.maximum_loop_resistance_for_voltage_drop_ohm - 0.008333333333).abs() < 1.0e-10);
        assert!(report.maximum_loop_resistance_for_conduction_loss_ohm > 0.0);
    }

    #[test]
    fn reports_limit_failures_without_clipping() {
        let report = evaluate_dc_rail_path(input(6.25, 1.0e-3)).unwrap();

        assert!(!report.within_declared_drop_limit);
        assert!(!report.within_declared_loss_limit);
        assert!(report.conductor_loss_w > 0.0);
    }

    #[test]
    fn rejects_fraction_limits_outside_open_unit_interval() {
        let mut too_high = input(50.0, 1.0e-3);
        too_high.maximum_voltage_drop_fraction = 1.0;
        let mut zero_loss = input(50.0, 1.0e-3);
        zero_loss.maximum_conduction_loss_fraction = 0.0;

        assert!(evaluate_dc_rail_path(too_high).is_err());
        assert!(evaluate_dc_rail_path(zero_loss).is_err());
    }

    #[test]
    fn rejects_nonfinite_and_invalid_inputs() {
        let invalid = evaluate_dc_rail_path(input(0.0, 0.0));
        let nan = evaluate_dc_rail_path(input(f64::NAN, 0.0));
        let negative_r = evaluate_dc_rail_path(input(50.0, -0.1));

        assert!(invalid.is_err());
        assert!(nan.is_err());
        assert!(negative_r.is_err());
    }
}
