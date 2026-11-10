use crate::AuditError;

/// A deliberately static operating point for one input-series ISOP cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsopInputCellMode {
    /// Cell transfers output power and consumes input power according to eta.
    Active,
    /// Cell output is isolated and its input terminals are represented by an
    /// explicitly declared bypass drop. The bypass hardware is not modelled.
    Bypassed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IsopInputCell {
    pub id: String,
    pub mode: IsopInputCellMode,
    /// Positive delivered output power. Must be zero for a bypassed cell.
    pub output_power_w: f64,
    /// Converter conversion efficiency for the declared operating point.
    pub efficiency_fraction: f64,
    /// Input-side quiescent/auxiliary power, in watts.
    pub no_load_input_power_w: f64,
    /// Bypass-path forward voltage drop. Must be zero for active cells.
    pub bypass_drop_voltage_v: f64,
    /// Declared operating input-voltage envelope for active cells.
    pub minimum_input_voltage_v: f64,
    pub maximum_input_voltage_v: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IsopInputBalanceInput {
    pub total_input_bus_voltage_v: f64,
    pub cells: Vec<IsopInputCell>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IsopInputCellResult {
    pub cell_id: String,
    pub mode: IsopInputCellMode,
    pub input_voltage_v: f64,
    pub input_power_w: f64,
    pub output_power_w: f64,
    pub bypass_loss_w: f64,
    /// None for bypassed cells; active cells are compared with declared limits.
    pub within_declared_input_voltage_limits: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IsopInputBalanceResult {
    pub total_input_bus_voltage_v: f64,
    /// Same input current flows through all cells and the bypass paths in the
    /// declared series chain.
    pub input_series_current_a: f64,
    pub active_cell_count: usize,
    pub bypassed_cell_count: usize,
    pub active_cell_input_voltage_sum_v: f64,
    pub maximum_absolute_deviation_from_equal_active_voltage_v: f64,
    pub maximum_absolute_deviation_from_equal_active_voltage_percent: f64,
    pub input_voltage_balance_residual_v: f64,
    pub total_output_power_w: f64,
    pub total_input_power_w: f64,
    pub total_bypass_loss_w: f64,
    pub total_chain_efficiency_percent: f64,
    /// V_bus*I_series minus the sum of active cell input powers and bypass loss.
    pub input_power_balance_residual_w: f64,
    /// False is a useful model result, not a reason to hide the voltage values.
    pub all_active_cells_within_declared_voltage_limits: bool,
    pub cells: Vec<IsopInputCellResult>,
}

impl IsopInputBalanceResult {
    pub const fn claim_ceiling(&self) -> &'static str {
        "STATIC_INPUT_SERIES_POWER_BALANCE_ONLY_NOT_DYNAMIC_VOLTAGE_SHARING_NOT_CONTROL_STABILITY_OR_PROTECTION_QUALIFICATION"
    }
}

fn finite(values: &[f64]) -> bool {
    values.iter().all(|value| value.is_finite())
}

/// Solve a static input-voltage allocation for an input-series chain.
///
/// Input current is common to all series-connected cells. For active cell i:
///
/// P_in,i = P_out,i / eta_i + P_quiescent,i
/// V_in,i = P_in,i / I_series
///
/// The series current is selected so the active-cell voltage drops plus
/// explicitly modelled bypass drops sum to the supplied total bus voltage.
///
/// This is an energy-balance allocation screen, not a dynamic cell-voltage
/// balancing controller model. It assumes the requested per-cell output powers
/// are already known and feasible on the parallel output bus.
pub fn evaluate_isop_input_voltage_balance(
    input: IsopInputBalanceInput,
) -> Result<IsopInputBalanceResult, AuditError> {
    if !input.total_input_bus_voltage_v.is_finite()
        || input.total_input_bus_voltage_v <= 0.0
    {
        return Err(AuditError(
            "ISOP total input-bus voltage must be positive and finite".to_string(),
        ));
    }
    if input.cells.is_empty() {
        return Err(AuditError(
            "ISOP input balance requires at least one cell".to_string(),
        ));
    }

    let mut ids: Vec<&str> = Vec::with_capacity(input.cells.len());
    let mut active_input_power_sum_w = 0.0_f64;
    let mut total_output_power_w = 0.0_f64;
    let mut bypass_drop_sum_v = 0.0_f64;
    let mut active_cell_count = 0_usize;
    let mut bypassed_cell_count = 0_usize;

    for cell in &input.cells {
        if cell.id.trim().is_empty() {
            return Err(AuditError("ISOP input cell id must not be blank".to_string()));
        }
        if ids.contains(&cell.id.as_str()) {
            return Err(AuditError(format!("duplicate ISOP input cell id {:?}", cell.id)));
        }
        ids.push(cell.id.as_str());

        let values = [
            cell.output_power_w,
            cell.efficiency_fraction,
            cell.no_load_input_power_w,
            cell.bypass_drop_voltage_v,
            cell.minimum_input_voltage_v,
            cell.maximum_input_voltage_v,
        ];
        if !finite(&values) {
            return Err(AuditError(format!(
                "ISOP input cell {:?}: all operating-point values must be finite",
                cell.id
            )));
        }
        if cell.output_power_w < 0.0
            || cell.efficiency_fraction <= 0.0
            || cell.efficiency_fraction > 1.0
            || cell.no_load_input_power_w < 0.0
            || cell.bypass_drop_voltage_v < 0.0
            || cell.minimum_input_voltage_v < 0.0
            || cell.maximum_input_voltage_v <= 0.0
            || cell.minimum_input_voltage_v > cell.maximum_input_voltage_v
        {
            return Err(AuditError(format!(
                "ISOP input cell {:?}: invalid power, efficiency, bypass or voltage-limit domain",
                cell.id
            )));
        }

        match cell.mode {
            IsopInputCellMode::Active => {
                if cell.bypass_drop_voltage_v != 0.0 {
                    return Err(AuditError(format!(
                        "active ISOP input cell {:?} cannot also declare a bypass drop",
                        cell.id
                    )));
                }
                let input_power_w =
                    cell.output_power_w / cell.efficiency_fraction
                        + cell.no_load_input_power_w;
                if !input_power_w.is_finite() || input_power_w <= 0.0 {
                    return Err(AuditError(format!(
                        "active ISOP input cell {:?}: input power must be positive and finite",
                        cell.id
                    )));
                }
                active_input_power_sum_w += input_power_w;
                total_output_power_w += cell.output_power_w;
                active_cell_count += 1;
            }
            IsopInputCellMode::Bypassed => {
                if cell.output_power_w != 0.0 || cell.no_load_input_power_w != 0.0 {
                    return Err(AuditError(format!(
                        "bypassed ISOP input cell {:?} must declare zero output and quiescent input power",
                        cell.id
                    )));
                }
                bypass_drop_sum_v += cell.bypass_drop_voltage_v;
                bypassed_cell_count += 1;
            }
        }
    }

    if active_cell_count == 0 {
        return Err(AuditError(
            "ISOP input chain requires at least one active cell".to_string(),
        ));
    }
    if !active_input_power_sum_w.is_finite()
        || active_input_power_sum_w <= 0.0
        || !bypass_drop_sum_v.is_finite()
        || bypass_drop_sum_v >= input.total_input_bus_voltage_v
    {
        return Err(AuditError(
            "ISOP active input power or bypass voltage sum is invalid".to_string(),
        ));
    }

    let active_cell_input_voltage_sum_v =
        input.total_input_bus_voltage_v - bypass_drop_sum_v;
    let input_series_current_a =
        active_input_power_sum_w / active_cell_input_voltage_sum_v;
    if !input_series_current_a.is_finite() || input_series_current_a <= 0.0 {
        return Err(AuditError(
            "ISOP input-series current is non-positive or non-finite".to_string(),
        ));
    }

    let equal_active_voltage_v =
        active_cell_input_voltage_sum_v / active_cell_count as f64;
    let mut maximum_absolute_deviation_v = 0.0_f64;
    let mut voltage_residual_v = -input.total_input_bus_voltage_v;
    let mut active_input_power_check_w = 0.0_f64;
    let mut total_bypass_loss_w = 0.0_f64;
    let mut all_within_limits = true;
    let mut results = Vec::with_capacity(input.cells.len());

    for cell in &input.cells {
        match cell.mode {
            IsopInputCellMode::Active => {
                let input_power_w =
                    cell.output_power_w / cell.efficiency_fraction
                        + cell.no_load_input_power_w;
                let input_voltage_v = input_power_w / input_series_current_a;
                let within_limits = input_voltage_v >= cell.minimum_input_voltage_v
                    && input_voltage_v <= cell.maximum_input_voltage_v;
                all_within_limits &= within_limits;
                maximum_absolute_deviation_v = maximum_absolute_deviation_v
                    .max((input_voltage_v - equal_active_voltage_v).abs());
                voltage_residual_v += input_voltage_v;
                active_input_power_check_w += input_power_w;
                results.push(IsopInputCellResult {
                    cell_id: cell.id.clone(),
                    mode: cell.mode,
                    input_voltage_v,
                    input_power_w,
                    output_power_w: cell.output_power_w,
                    bypass_loss_w: 0.0,
                    within_declared_input_voltage_limits: Some(within_limits),
                });
            }
            IsopInputCellMode::Bypassed => {
                let bypass_loss_w =
                    input_series_current_a * cell.bypass_drop_voltage_v;
                if !bypass_loss_w.is_finite() {
                    return Err(AuditError(format!(
                        "bypassed ISOP cell {:?}: bypass loss is non-finite",
                        cell.id
                    )));
                }
                voltage_residual_v += cell.bypass_drop_voltage_v;
                total_bypass_loss_w += bypass_loss_w;
                results.push(IsopInputCellResult {
                    cell_id: cell.id.clone(),
                    mode: cell.mode,
                    input_voltage_v: cell.bypass_drop_voltage_v,
                    input_power_w: bypass_loss_w,
                    output_power_w: 0.0,
                    bypass_loss_w,
                    within_declared_input_voltage_limits: None,
                });
            }
        }
    }

    let total_input_power_w = input.total_input_bus_voltage_v * input_series_current_a;
    let total_chain_efficiency_percent =
        100.0 * total_output_power_w / total_input_power_w;
    let input_power_balance_residual_w = total_input_power_w
        - active_input_power_check_w
        - total_bypass_loss_w;
    let maximum_absolute_deviation_percent = if equal_active_voltage_v > 0.0 {
        100.0 * maximum_absolute_deviation_v / equal_active_voltage_v
    } else {
        0.0
    };
    let derived = [
        active_cell_input_voltage_sum_v,
        input_series_current_a,
        equal_active_voltage_v,
        maximum_absolute_deviation_v,
        maximum_absolute_deviation_percent,
        voltage_residual_v,
        total_output_power_w,
        total_input_power_w,
        total_bypass_loss_w,
        total_chain_efficiency_percent,
        input_power_balance_residual_w,
    ];
    if !finite(&derived)
        || derived.iter().any(|value| *value < 0.0 && value != &voltage_residual_v && value != &input_power_balance_residual_w)
        || total_input_power_w <= 0.0
        || total_chain_efficiency_percent < 0.0
        || total_chain_efficiency_percent > 100.0
    {
        return Err(AuditError(
            "ISOP input-balance result is non-finite or outside its physical domain".to_string(),
        ));
    }

    Ok(IsopInputBalanceResult {
        total_input_bus_voltage_v: input.total_input_bus_voltage_v,
        input_series_current_a,
        active_cell_count,
        bypassed_cell_count,
        active_cell_input_voltage_sum_v,
        maximum_absolute_deviation_from_equal_active_voltage_v: maximum_absolute_deviation_v,
        maximum_absolute_deviation_from_equal_active_voltage_percent:
            maximum_absolute_deviation_percent,
        input_voltage_balance_residual_v: voltage_residual_v,
        total_output_power_w,
        total_input_power_w,
        total_bypass_loss_w,
        total_chain_efficiency_percent,
        input_power_balance_residual_w,
        all_active_cells_within_declared_voltage_limits: all_within_limits,
        cells: results,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_isop_input_voltage_balance, IsopInputBalanceInput, IsopInputCell,
        IsopInputCellMode,
    };

    fn active(id: &str, output_power_w: f64) -> IsopInputCell {
        IsopInputCell {
            id: id.to_string(),
            mode: IsopInputCellMode::Active,
            output_power_w,
            efficiency_fraction: 0.98,
            no_load_input_power_w: 0.0,
            bypass_drop_voltage_v: 0.0,
            minimum_input_voltage_v: 90.0,
            maximum_input_voltage_v: 110.0,
        }
    }

    fn bypassed(id: &str, drop_v: f64) -> IsopInputCell {
        IsopInputCell {
            id: id.to_string(),
            mode: IsopInputCellMode::Bypassed,
            output_power_w: 0.0,
            efficiency_fraction: 0.98,
            no_load_input_power_w: 0.0,
            bypass_drop_voltage_v: drop_v,
            minimum_input_voltage_v: 0.0,
            maximum_input_voltage_v: 1000.0,
        }
    }

    #[test]
    fn equal_four_cell_power_gives_equal_200v_input_shares() {
        let report = evaluate_isop_input_voltage_balance(IsopInputBalanceInput {
            total_input_bus_voltage_v: 800.0,
            cells: vec![
                active("a", 750.0),
                active("b", 750.0),
                active("c", 750.0),
                active("d", 750.0),
            ],
        })
        .unwrap();

        assert!((report.input_series_current_a - (3000.0 / 0.98 / 800.0)).abs() < 1.0e-9);
        assert!(report.cells.iter().all(|cell| (cell.input_voltage_v - 200.0).abs() < 1.0e-9));
        assert!(report.all_active_cells_within_declared_voltage_limits);
        assert!(report.input_voltage_balance_residual_v.abs() < 1.0e-9);
        assert!(report.input_power_balance_residual_w.abs() < 1.0e-9);
    }

    #[test]
    fn output_power_and_efficiency_mismatch_changes_voltage_allocation() {
        let mut cells = vec![active("a", 750.0), active("b", 750.0)];
        cells[0].efficiency_fraction = 0.96;
        cells[1].efficiency_fraction = 0.99;

        let report = evaluate_isop_input_voltage_balance(IsopInputBalanceInput {
            total_input_bus_voltage_v: 400.0,
            cells,
        })
        .unwrap();

        assert!(report.cells[0].input_voltage_v > report.cells[1].input_voltage_v);
        assert!(report.maximum_absolute_deviation_from_equal_active_voltage_v > 0.0);
        assert!(report.input_power_balance_residual_w.abs() < 1.0e-9);
    }

    #[test]
    fn bypassed_cell_is_explicit_and_other_cells_take_remaining_bus_voltage() {
        let report = evaluate_isop_input_voltage_balance(IsopInputBalanceInput {
            total_input_bus_voltage_v: 800.0,
            cells: vec![
                active("a", 1000.0),
                active("b", 1000.0),
                active("c", 1000.0),
                bypassed("d", 0.2),
            ],
        })
        .unwrap();

        assert_eq!(report.active_cell_count, 3);
        assert_eq!(report.bypassed_cell_count, 1);
        assert!((report.cells[0].input_voltage_v - 266.6).abs() < 0.1);
        assert!(report.cells[3].input_voltage_v == 0.2);
        assert!(report.total_bypass_loss_w > 0.0);
        assert!(report.input_voltage_balance_residual_v.abs() < 1.0e-9);
    }

    #[test]
    fn voltage_limit_violation_is_reported_without_clipping() {
        let mut first = active("a", 1500.0);
        first.maximum_input_voltage_v = 180.0;

        let report = evaluate_isop_input_voltage_balance(IsopInputBalanceInput {
            total_input_bus_voltage_v: 400.0,
            cells: vec![first, active("b", 500.0)],
        })
        .unwrap();

        assert!(!report.all_active_cells_within_declared_voltage_limits);
        assert!(report.cells[0].input_voltage_v > 180.0);
        assert_eq!(report.cells[0].within_declared_input_voltage_limits, Some(false));
    }

    #[test]
    fn rejects_empty_chain_partial_bypass_and_no_active_cell() {
        let empty = evaluate_isop_input_voltage_balance(IsopInputBalanceInput {
            total_input_bus_voltage_v: 800.0,
            cells: vec![],
        });
        let mut invalid_bypass = bypassed("b", 0.0);
        invalid_bypass.output_power_w = 1.0;
        let partial = evaluate_isop_input_voltage_balance(IsopInputBalanceInput {
            total_input_bus_voltage_v: 800.0,
            cells: vec![active("a", 100.0), invalid_bypass],
        });
        let all_bypassed = evaluate_isop_input_voltage_balance(IsopInputBalanceInput {
            total_input_bus_voltage_v: 800.0,
            cells: vec![bypassed("a", 0.1)],
        });

        assert!(empty.is_err());
        assert!(partial.is_err());
        assert!(all_bypassed.is_err());
    }

    #[test]
    fn rejects_nonfinite_and_underflowed_power_inputs() {
        let nonfinite = evaluate_isop_input_voltage_balance(IsopInputBalanceInput {
            total_input_bus_voltage_v: 800.0,
            cells: vec![active("a", f64::INFINITY)],
        });
        let mut tiny = active("tiny", f64::MIN_POSITIVE);
        tiny.efficiency_fraction = 1.0;
        let underflow = evaluate_isop_input_voltage_balance(IsopInputBalanceInput {
            total_input_bus_voltage_v: f64::MAX,
            cells: vec![tiny],
        });

        assert!(nonfinite.is_err());
        assert!(underflow.is_err());
    }
}
