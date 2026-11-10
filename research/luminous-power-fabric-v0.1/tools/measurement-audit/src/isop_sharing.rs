use crate::AuditError;

/// Linear DC Thevenin approximation for one output-parallel ISOP cell.
///
/// `regulated_voltage_setpoint_v` is the cell's equivalent no-load output
/// voltage. `effective_output_resistance_ohm` is the declared static droop
/// plus branch resistance at the operating point. It is not a small-signal
/// impedance or a substitute for closed-loop stability analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct IsopOutputCell {
    pub id: String,
    pub regulated_voltage_setpoint_v: f64,
    pub effective_output_resistance_ohm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IsopOutputSharingInput {
    /// Positive current is delivered from the cells into the common DC bus.
    pub output_load_current_a: f64,
    pub cells: Vec<IsopOutputCell>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IsopCellCurrent {
    pub cell_id: String,
    /// Signed current: negative means this cell absorbs current from the bus.
    pub output_current_a: f64,
    /// Signed DC power at the common bus voltage; negative means backfeed.
    pub output_power_w: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IsopOutputSharingResult {
    pub output_bus_voltage_v: f64,
    pub output_load_current_a: f64,
    pub average_cell_current_a: f64,
    pub maximum_absolute_share_deviation_a: f64,
    /// None at zero load, where relative deviation from an equal-load target
    /// is undefined and circulating current should be reported in amperes.
    pub maximum_absolute_share_deviation_percent: Option<f64>,
    pub peak_to_peak_cell_current_spread_a: f64,
    pub net_cell_current_a: f64,
    pub current_balance_residual_a: f64,
    pub total_output_power_w: f64,
    /// Non-empty when the linear steady-state solution predicts a cell
    /// absorbing current from the common bus.
    pub backfeeding_cell_ids: Vec<String>,
    pub cells: Vec<IsopCellCurrent>,
}

impl IsopOutputSharingResult {
    pub const fn claim_ceiling(&self) -> &'static str {
        "STATIC_THEVENIN_OUTPUT_SHARING_SCREEN_ONLY_NOT_DYNAMIC_STABILITY_NOT_PROTECTION_OR_QUALIFICATION"
    }
}

/// Solve the steady-state current distribution of output-parallel cells.
///
/// For each cell i: I_i = (E_i - V_bus) / R_i.
/// The common bus voltage is chosen so sum(I_i) equals the declared load
/// current. This is an algebraic screening model for a linear, positive-slope
/// output characteristic; it does not simulate switching, feedback loops,
/// current-limit transitions, input-series voltage balance or fault events.
///
/// Negative cell current is preserved and reported as predicted backfeed. The
/// function does not assume that a unidirectional DCX can safely absorb it.
pub fn evaluate_isop_output_sharing(
    input: IsopOutputSharingInput,
) -> Result<IsopOutputSharingResult, AuditError> {
    if input.cells.is_empty() {
        return Err(AuditError(
            "ISOP output sharing requires at least one cell".to_string(),
        ));
    }
    if !input.output_load_current_a.is_finite() || input.output_load_current_a < 0.0 {
        return Err(AuditError(
            "ISOP output load current must be finite and non-negative".to_string(),
        ));
    }

    let mut conductance_sum = 0.0_f64;
    let mut voltage_conductance_sum = 0.0_f64;
    let mut seen_ids: Vec<&str> = Vec::with_capacity(input.cells.len());

    for cell in &input.cells {
        if cell.id.trim().is_empty() {
            return Err(AuditError("ISOP cell id must not be blank".to_string()));
        }
        if seen_ids.contains(&cell.id.as_str()) {
            return Err(AuditError(format!(
                "duplicate ISOP cell id {:?}",
                cell.id
            )));
        }
        seen_ids.push(cell.id.as_str());

        if !cell.regulated_voltage_setpoint_v.is_finite()
            || cell.regulated_voltage_setpoint_v <= 0.0
            || !cell.effective_output_resistance_ohm.is_finite()
            || cell.effective_output_resistance_ohm <= 0.0
        {
            return Err(AuditError(format!(
                "ISOP cell {:?}: voltage setpoint and effective output resistance must be positive and finite",
                cell.id
            )));
        }

        let conductance = 1.0 / cell.effective_output_resistance_ohm;
        let weighted_voltage = cell.regulated_voltage_setpoint_v * conductance;
        if !conductance.is_finite()
            || conductance <= 0.0
            || !weighted_voltage.is_finite()
            || weighted_voltage <= 0.0
        {
            return Err(AuditError(format!(
                "ISOP cell {:?}: derived conductance or weighted voltage is non-finite or underflowed",
                cell.id
            )));
        }
        conductance_sum += conductance;
        voltage_conductance_sum += weighted_voltage;
    }

    if !conductance_sum.is_finite()
        || conductance_sum <= 0.0
        || !voltage_conductance_sum.is_finite()
        || voltage_conductance_sum <= 0.0
    {
        return Err(AuditError(
            "ISOP aggregate conductance is non-finite or outside its physical domain".to_string(),
        ));
    }

    let output_bus_voltage_v =
        (voltage_conductance_sum - input.output_load_current_a) / conductance_sum;
    if !output_bus_voltage_v.is_finite() || output_bus_voltage_v <= 0.0 {
        return Err(AuditError(
            "ISOP linear output-sharing solution has non-positive or non-finite bus voltage"
                .to_string(),
        ));
    }

    let average_cell_current_a = input.output_load_current_a / input.cells.len() as f64;
    let mut cells = Vec::with_capacity(input.cells.len());
    let mut net_cell_current_a = 0.0_f64;
    let mut minimum_current_a = f64::INFINITY;
    let mut maximum_current_a = f64::NEG_INFINITY;
    let mut maximum_absolute_share_deviation_a = 0.0_f64;
    let mut backfeeding_cell_ids = Vec::new();

    for cell in &input.cells {
        let current_a = (cell.regulated_voltage_setpoint_v - output_bus_voltage_v)
            / cell.effective_output_resistance_ohm;
        let output_power_w = output_bus_voltage_v * current_a;
        if !current_a.is_finite() || !output_power_w.is_finite() {
            return Err(AuditError(format!(
                "ISOP cell {:?}: derived current or output power is non-finite",
                cell.id
            )));
        }

        net_cell_current_a += current_a;
        minimum_current_a = minimum_current_a.min(current_a);
        maximum_current_a = maximum_current_a.max(current_a);
        maximum_absolute_share_deviation_a = maximum_absolute_share_deviation_a
            .max((current_a - average_cell_current_a).abs());
        if current_a < 0.0 {
            backfeeding_cell_ids.push(cell.id.clone());
        }

        cells.push(IsopCellCurrent {
            cell_id: cell.id.clone(),
            output_current_a: current_a,
            output_power_w,
        });
    }

    let current_balance_residual_a = net_cell_current_a - input.output_load_current_a;
    let peak_to_peak_cell_current_spread_a = maximum_current_a - minimum_current_a;
    let maximum_absolute_share_deviation_percent = if input.output_load_current_a > 0.0 {
        Some(
            100.0 * maximum_absolute_share_deviation_a
                / average_cell_current_a.abs(),
        )
    } else {
        None
    };
    let total_output_power_w = output_bus_voltage_v * input.output_load_current_a;

    let derived = [
        average_cell_current_a,
        net_cell_current_a,
        current_balance_residual_a,
        peak_to_peak_cell_current_spread_a,
        maximum_absolute_share_deviation_a,
        total_output_power_w,
    ];
    if derived.iter().any(|value| !value.is_finite())
        || !maximum_absolute_share_deviation_percent
            .map(|value| value.is_finite())
            .unwrap_or(true)
        || total_output_power_w < 0.0
        || peak_to_peak_cell_current_spread_a < 0.0
    {
        return Err(AuditError(
            "ISOP output-sharing result is non-finite or outside its physical domain".to_string(),
        ));
    }

    Ok(IsopOutputSharingResult {
        output_bus_voltage_v,
        output_load_current_a: input.output_load_current_a,
        average_cell_current_a,
        maximum_absolute_share_deviation_a,
        maximum_absolute_share_deviation_percent,
        peak_to_peak_cell_current_spread_a,
        net_cell_current_a,
        current_balance_residual_a,
        total_output_power_w,
        backfeeding_cell_ids,
        cells,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_isop_output_sharing, IsopOutputCell, IsopOutputSharingInput,
    };

    fn cell(id: &str, setpoint_v: f64, resistance_ohm: f64) -> IsopOutputCell {
        IsopOutputCell {
            id: id.to_string(),
            regulated_voltage_setpoint_v: setpoint_v,
            effective_output_resistance_ohm: resistance_ohm,
        }
    }

    #[test]
    fn identical_four_cell_isop_shares_load_equally() {
        let report = evaluate_isop_output_sharing(IsopOutputSharingInput {
            output_load_current_a: 80.0,
            cells: vec![
                cell("cell-a", 50.0, 0.01),
                cell("cell-b", 50.0, 0.01),
                cell("cell-c", 50.0, 0.01),
                cell("cell-d", 50.0, 0.01),
            ],
        })
        .unwrap();

        assert!((report.output_bus_voltage_v - 49.8).abs() < 1.0e-9);
        assert!((report.average_cell_current_a - 20.0).abs() < 1.0e-9);
        assert!(report.cells.iter().all(|cell| (cell.output_current_a - 20.0).abs() < 1.0e-9));
        assert!(report.maximum_absolute_share_deviation_a < 1.0e-9);
        assert!(report.backfeeding_cell_ids.is_empty());
        assert!(report.current_balance_residual_a.abs() < 1.0e-9);
    }

    #[test]
    fn unequal_output_resistance_changes_static_current_share() {
        let report = evaluate_isop_output_sharing(IsopOutputSharingInput {
            output_load_current_a: 10.0,
            cells: vec![cell("low-r", 50.0, 0.1), cell("high-r", 50.0, 0.2)],
        })
        .unwrap();

        assert!((report.output_bus_voltage_v - (50.0 - 10.0 / 15.0)).abs() < 1.0e-9);
        assert!((report.cells[0].output_current_a - (20.0 / 3.0)).abs() < 1.0e-9);
        assert!((report.cells[1].output_current_a - (10.0 / 3.0)).abs() < 1.0e-9);
        assert!(report.maximum_absolute_share_deviation_a > 0.0);
    }

    #[test]
    fn setpoint_mismatch_reports_zero_load_circulating_current() {
        let report = evaluate_isop_output_sharing(IsopOutputSharingInput {
            output_load_current_a: 0.0,
            cells: vec![cell("high-setpoint", 50.1, 0.01), cell("low-setpoint", 49.9, 0.01)],
        })
        .unwrap();

        assert!((report.output_bus_voltage_v - 50.0).abs() < 1.0e-9);
        assert!((report.cells[0].output_current_a - 10.0).abs() < 1.0e-9);
        assert!((report.cells[1].output_current_a + 10.0).abs() < 1.0e-9);
        assert_eq!(report.maximum_absolute_share_deviation_percent, None);
        assert_eq!(report.backfeeding_cell_ids, vec!["low-setpoint".to_string()]);
    }

    #[test]
    fn current_balance_is_conserved_with_mismatched_cells() {
        let report = evaluate_isop_output_sharing(IsopOutputSharingInput {
            output_load_current_a: 30.0,
            cells: vec![
                cell("a", 50.05, 0.012),
                cell("b", 49.98, 0.009),
                cell("c", 50.01, 0.015),
            ],
        })
        .unwrap();

        assert!(report.current_balance_residual_a.abs() < 1.0e-9);
        assert!(
            (report.cells.iter().map(|cell| cell.output_current_a).sum::<f64>() - 30.0).abs()
                < 1.0e-9
        );
    }

    #[test]
    fn rejects_empty_duplicate_or_blank_cell_identity() {
        let empty = evaluate_isop_output_sharing(IsopOutputSharingInput {
            output_load_current_a: 1.0,
            cells: vec![],
        });
        let duplicate = evaluate_isop_output_sharing(IsopOutputSharingInput {
            output_load_current_a: 1.0,
            cells: vec![cell("same", 50.0, 0.1), cell("same", 50.0, 0.2)],
        });
        let blank = evaluate_isop_output_sharing(IsopOutputSharingInput {
            output_load_current_a: 1.0,
            cells: vec![cell("  ", 50.0, 0.1)],
        });

        assert!(empty.is_err());
        assert!(duplicate.is_err());
        assert!(blank.is_err());
    }

    #[test]
    fn rejects_invalid_cells_load_and_nonpositive_bus_solution() {
        let bad_resistance = evaluate_isop_output_sharing(IsopOutputSharingInput {
            output_load_current_a: 1.0,
            cells: vec![cell("bad", 50.0, 0.0)],
        });
        let negative_load = evaluate_isop_output_sharing(IsopOutputSharingInput {
            output_load_current_a: -1.0,
            cells: vec![cell("a", 50.0, 0.1)],
        });
        let overloaded_bus = evaluate_isop_output_sharing(IsopOutputSharingInput {
            output_load_current_a: 1000.0,
            cells: vec![cell("a", 50.0, 0.1)],
        });

        assert!(bad_resistance.is_err());
        assert!(negative_load.is_err());
        assert!(overloaded_bus.is_err());
    }

    #[test]
    fn rejects_nonfinite_and_underflowed_aggregate_inputs() {
        let nonfinite = evaluate_isop_output_sharing(IsopOutputSharingInput {
            output_load_current_a: 1.0,
            cells: vec![cell("bad", f64::INFINITY, 0.1)],
        });
        let underflow = evaluate_isop_output_sharing(IsopOutputSharingInput {
            output_load_current_a: 1.0,
            cells: vec![cell("tiny", f64::MIN_POSITIVE, f64::MAX)],
        });

        assert!(nonfinite.is_err());
        assert!(underflow.is_err());
    }
}
