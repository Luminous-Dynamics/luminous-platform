use crate::AuditError;

/// Conservative, parameter-only energy ledger for a DC fault boundary.
///
/// This is an accounting aid, not a fault-current waveform or arc-flash model.
/// Maxima are treated as simultaneous; the resulting sum can intentionally
/// over-count when the source term and stored-energy terms overlap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DcFaultEnergyInput {
    /// Maximum effective DC-link capacitance, including declared tolerance.
    pub dc_link_capacitance_max_f: f64,
    /// Maximum initial voltage across the modeled DC-link capacitance.
    pub dc_link_voltage_max_v: f64,
    /// Maximum cable / bus inductance included in this boundary.
    pub cable_inductance_max_h: f64,
    /// Maximum current through that inductance at the fault boundary.
    pub cable_current_max_a: f64,
    /// Maximum source voltage used for the rectangular source-energy bound.
    pub source_voltage_max_v: f64,
    /// Maximum source current guaranteed by an independent source limit.
    pub source_current_limit_max_a: f64,
    /// Maximum duration for which the source may contribute before isolation.
    pub source_contribution_duration_max_s: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DcFaultEnergyLedger {
    pub dc_link_capacitor_energy_j: f64,
    pub cable_magnetic_energy_j: f64,
    /// Rectangular upper bound V_source,max * I_source,max * t_isolation,max.
    pub source_energy_upper_bound_j: f64,
    /// Sum of the selected maxima; may over-count overlapping energy paths.
    pub conservative_accounting_sum_j: f64,
}

impl DcFaultEnergyLedger {
    pub const fn claim_ceiling(&self) -> &'static str {
        "PARAMETERIZED_ENERGY_LEDGER_ONLY_NOT_FAULT_WAVEFORM_NOT_ARC_FLASH_NOT_PROTECTION_QUALIFICATION"
    }
}

fn all_finite(values: &[f64]) -> bool {
    values.iter().all(|value| value.is_finite())
}

/// Evaluate selected stored-energy terms and a rectangular source contribution
/// bound at one declared DC fault boundary.
///
/// The source terms must be either all zero (no source contribution modeled) or
/// all positive. Cable inductance/current must likewise be both zero or both
/// positive. Missing source-current or isolation-time bounds are rejected.
pub fn evaluate_dc_fault_energy(
    input: DcFaultEnergyInput,
) -> Result<DcFaultEnergyLedger, AuditError> {
    let values = [
        input.dc_link_capacitance_max_f,
        input.dc_link_voltage_max_v,
        input.cable_inductance_max_h,
        input.cable_current_max_a,
        input.source_voltage_max_v,
        input.source_current_limit_max_a,
        input.source_contribution_duration_max_s,
    ];
    if !all_finite(&values) {
        return Err(AuditError(
            "DC fault-energy inputs must all be finite".to_string(),
        ));
    }
    if input.dc_link_capacitance_max_f <= 0.0 || input.dc_link_voltage_max_v <= 0.0 {
        return Err(AuditError(
            "DC-link capacitance and voltage maxima must be positive".to_string(),
        ));
    }
    if input.cable_inductance_max_h < 0.0 || input.cable_current_max_a < 0.0 {
        return Err(AuditError(
            "cable inductance and current maxima must be non-negative".to_string(),
        ));
    }
    let cable_terms_absent =
        input.cable_inductance_max_h == 0.0 && input.cable_current_max_a == 0.0;
    let cable_terms_present =
        input.cable_inductance_max_h > 0.0 && input.cable_current_max_a > 0.0;
    if !cable_terms_absent && !cable_terms_present {
        return Err(AuditError(
            "cable inductance and current must either both be zero or both be positive"
                .to_string(),
        ));
    }

    let source_terms = [
        input.source_voltage_max_v,
        input.source_current_limit_max_a,
        input.source_contribution_duration_max_s,
    ];
    if source_terms.iter().any(|value| *value < 0.0) {
        return Err(AuditError(
            "source voltage, current and duration bounds must be non-negative".to_string(),
        ));
    }
    let source_terms_absent = source_terms.iter().all(|value| *value == 0.0);
    let source_terms_present = source_terms.iter().all(|value| *value > 0.0);
    if !source_terms_absent && !source_terms_present {
        return Err(AuditError(
            "source voltage, current and duration must either all be zero or all be positive"
                .to_string(),
        ));
    }

    let dc_link_capacitor_energy_j =
        0.5 * input.dc_link_capacitance_max_f * input.dc_link_voltage_max_v.powi(2);
    let cable_magnetic_energy_j =
        0.5 * input.cable_inductance_max_h * input.cable_current_max_a.powi(2);
    let source_energy_upper_bound_j = input.source_voltage_max_v
        * input.source_current_limit_max_a
        * input.source_contribution_duration_max_s;
    let conservative_accounting_sum_j = dc_link_capacitor_energy_j
        + cable_magnetic_energy_j
        + source_energy_upper_bound_j;

    let derived = [
        dc_link_capacitor_energy_j,
        cable_magnetic_energy_j,
        source_energy_upper_bound_j,
        conservative_accounting_sum_j,
    ];
    if !all_finite(&derived) || derived.iter().any(|value| *value < 0.0) {
        return Err(AuditError(
            "derived DC fault-energy result is non-finite or negative".to_string(),
        ));
    }

    Ok(DcFaultEnergyLedger {
        dc_link_capacitor_energy_j,
        cable_magnetic_energy_j,
        source_energy_upper_bound_j,
        conservative_accounting_sum_j,
    })
}

/// Inputs for a first-order resistive DC-link discharge screen.
///
/// Resistance limits must already include the declared tolerance and relevant
/// temperature range. The model assumes a constant R and C and an exponential
/// RC discharge; it does not model resistor pulse curves, switching, contactors,
/// leakage, voltage-dependent capacitance, or multiple discharge paths.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DcLinkDischargeInput {
    pub capacitance_max_f: f64,
    pub initial_voltage_max_v: f64,
    pub residual_voltage_threshold_v: f64,
    pub resistance_min_ohm: f64,
    pub resistance_max_ohm: f64,
    pub discharge_time_target_s: f64,
    /// Declared pulse-energy rating for the exact applicable pulse condition.
    pub resistor_pulse_energy_rating_j: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DcLinkDischargeScreen {
    pub initial_stored_energy_j: f64,
    pub residual_energy_at_threshold_j: f64,
    pub energy_to_dissipate_j: f64,
    pub maximum_initial_discharge_current_a: f64,
    pub maximum_initial_resistor_power_w: f64,
    pub worst_case_time_to_threshold_s: f64,
    pub maximum_resistance_for_time_target_ohm: f64,
    pub pulse_energy_rating_margin_j: f64,
    pub discharge_time_numeric_screen_passes: bool,
    pub pulse_energy_numeric_screen_passes: bool,
}

impl DcLinkDischargeScreen {
    pub const fn claim_ceiling(&self) -> &'static str {
        "FIRST_ORDER_RC_SCREEN_ONLY_NOT_SAFE_DISCHARGE_PROOF_NOT_SERVICE_AUTHORIZATION"
    }
}

/// Screen the first-order RC discharge time and selected resistor stresses.
///
/// Worst-case discharge time uses maximum capacitance and resistance. Initial
/// current and power use maximum voltage and minimum resistance. The pulse
/// energy comparison is only a numeric comparison against the caller's declared
/// rating; it does not establish that the rating applies to the pulse waveform.
pub fn screen_dc_link_discharge(
    input: DcLinkDischargeInput,
) -> Result<DcLinkDischargeScreen, AuditError> {
    let values = [
        input.capacitance_max_f,
        input.initial_voltage_max_v,
        input.residual_voltage_threshold_v,
        input.resistance_min_ohm,
        input.resistance_max_ohm,
        input.discharge_time_target_s,
        input.resistor_pulse_energy_rating_j,
    ];
    if !all_finite(&values) || values.iter().any(|value| *value <= 0.0) {
        return Err(AuditError(
            "discharge-screen inputs must all be positive and finite".to_string(),
        ));
    }
    if input.residual_voltage_threshold_v >= input.initial_voltage_max_v {
        return Err(AuditError(
            "residual-voltage threshold must be below the initial voltage".to_string(),
        ));
    }
    if input.resistance_max_ohm < input.resistance_min_ohm {
        return Err(AuditError(
            "maximum resistance must be greater than or equal to minimum resistance"
                .to_string(),
        ));
    }

    // Subtract logarithms rather than dividing voltages, avoiding ratio overflow.
    let log_voltage_ratio =
        input.initial_voltage_max_v.ln() - input.residual_voltage_threshold_v.ln();
    let initial_stored_energy_j =
        0.5 * input.capacitance_max_f * input.initial_voltage_max_v.powi(2);
    let residual_energy_at_threshold_j =
        0.5 * input.capacitance_max_f * input.residual_voltage_threshold_v.powi(2);
    let energy_to_dissipate_j = initial_stored_energy_j - residual_energy_at_threshold_j;
    let maximum_initial_discharge_current_a =
        input.initial_voltage_max_v / input.resistance_min_ohm;
    let maximum_initial_resistor_power_w =
        input.initial_voltage_max_v.powi(2) / input.resistance_min_ohm;
    let worst_case_time_to_threshold_s =
        input.resistance_max_ohm * input.capacitance_max_f * log_voltage_ratio;
    let maximum_resistance_for_time_target_ohm =
        (input.discharge_time_target_s / input.capacitance_max_f) / log_voltage_ratio;
    let pulse_energy_rating_margin_j =
        input.resistor_pulse_energy_rating_j - energy_to_dissipate_j;

    let derived = [
        log_voltage_ratio,
        initial_stored_energy_j,
        residual_energy_at_threshold_j,
        energy_to_dissipate_j,
        maximum_initial_discharge_current_a,
        maximum_initial_resistor_power_w,
        worst_case_time_to_threshold_s,
        maximum_resistance_for_time_target_ohm,
        pulse_energy_rating_margin_j,
    ];
    if !all_finite(&derived)
        || log_voltage_ratio <= 0.0
        || energy_to_dissipate_j <= 0.0
        || maximum_resistance_for_time_target_ohm <= 0.0
    {
        return Err(AuditError(
            "derived discharge-screen result is non-finite or outside its physical domain"
                .to_string(),
        ));
    }

    Ok(DcLinkDischargeScreen {
        initial_stored_energy_j,
        residual_energy_at_threshold_j,
        energy_to_dissipate_j,
        maximum_initial_discharge_current_a,
        maximum_initial_resistor_power_w,
        worst_case_time_to_threshold_s,
        maximum_resistance_for_time_target_ohm,
        pulse_energy_rating_margin_j,
        discharge_time_numeric_screen_passes: worst_case_time_to_threshold_s
            <= input.discharge_time_target_s,
        pulse_energy_numeric_screen_passes: pulse_energy_rating_margin_j >= 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_dc_fault_energy, screen_dc_link_discharge, DcFaultEnergyInput,
        DcLinkDischargeInput,
    };

    #[test]
    fn fault_energy_ledger_keeps_terms_separate() {
        let report = evaluate_dc_fault_energy(DcFaultEnergyInput {
            dc_link_capacitance_max_f: 1.0e-3,
            dc_link_voltage_max_v: 800.0,
            cable_inductance_max_h: 10.0e-6,
            cable_current_max_a: 120.0,
            source_voltage_max_v: 800.0,
            source_current_limit_max_a: 10.0,
            source_contribution_duration_max_s: 5.0e-3,
        })
        .unwrap();

        assert!((report.dc_link_capacitor_energy_j - 320.0).abs() < 1.0e-9);
        assert!((report.cable_magnetic_energy_j - 0.072).abs() < 1.0e-9);
        assert!((report.source_energy_upper_bound_j - 40.0).abs() < 1.0e-9);
        assert!((report.conservative_accounting_sum_j - 360.072).abs() < 1.0e-9);
        assert!(report.claim_ceiling().contains("NOT_ARC_FLASH"));
    }

    #[test]
    fn fault_energy_ledger_accepts_explicit_absent_optional_terms() {
        let report = evaluate_dc_fault_energy(DcFaultEnergyInput {
            dc_link_capacitance_max_f: 1.0e-3,
            dc_link_voltage_max_v: 50.0,
            cable_inductance_max_h: 0.0,
            cable_current_max_a: 0.0,
            source_voltage_max_v: 0.0,
            source_current_limit_max_a: 0.0,
            source_contribution_duration_max_s: 0.0,
        })
        .unwrap();

        assert!((report.dc_link_capacitor_energy_j - 1.25).abs() < 1.0e-12);
        assert_eq!(report.cable_magnetic_energy_j, 0.0);
        assert_eq!(report.source_energy_upper_bound_j, 0.0);
    }

    #[test]
    fn fault_energy_ledger_rejects_partial_source_bounds() {
        let result = evaluate_dc_fault_energy(DcFaultEnergyInput {
            dc_link_capacitance_max_f: 1.0e-3,
            dc_link_voltage_max_v: 800.0,
            cable_inductance_max_h: 0.0,
            cable_current_max_a: 0.0,
            source_voltage_max_v: 800.0,
            source_current_limit_max_a: 0.0,
            source_contribution_duration_max_s: 5.0e-3,
        });

        assert!(result.is_err());
    }

    #[test]
    fn fault_energy_ledger_rejects_overflow() {
        let result = evaluate_dc_fault_energy(DcFaultEnergyInput {
            dc_link_capacitance_max_f: f64::MAX,
            dc_link_voltage_max_v: f64::MAX,
            cable_inductance_max_h: 0.0,
            cable_current_max_a: 0.0,
            source_voltage_max_v: 0.0,
            source_current_limit_max_a: 0.0,
            source_contribution_duration_max_s: 0.0,
        });

        assert!(result.is_err());
    }

    #[test]
    fn discharge_screen_calculates_rc_time_and_energy() {
        let report = screen_dc_link_discharge(DcLinkDischargeInput {
            capacitance_max_f: 1.0e-3,
            initial_voltage_max_v: 800.0,
            residual_voltage_threshold_v: 60.0,
            resistance_min_ohm: 100_000.0,
            resistance_max_ohm: 110_000.0,
            discharge_time_target_s: 300.0,
            resistor_pulse_energy_rating_j: 350.0,
        })
        .unwrap();

        assert!((report.initial_stored_energy_j - 320.0).abs() < 1.0e-9);
        assert!((report.residual_energy_at_threshold_j - 1.8).abs() < 1.0e-9);
        assert!((report.energy_to_dissipate_j - 318.2).abs() < 1.0e-9);
        assert!((report.maximum_initial_discharge_current_a - 0.008).abs() < 1.0e-12);
        assert!((report.maximum_initial_resistor_power_w - 6.4).abs() < 1.0e-9);
        assert!(report.discharge_time_numeric_screen_passes);
        assert!(report.pulse_energy_numeric_screen_passes);
    }

    #[test]
    fn discharge_screen_reports_failed_numeric_limits_without_hiding_values() {
        let report = screen_dc_link_discharge(DcLinkDischargeInput {
            capacitance_max_f: 1.0e-3,
            initial_voltage_max_v: 800.0,
            residual_voltage_threshold_v: 60.0,
            resistance_min_ohm: 100_000.0,
            resistance_max_ohm: 110_000.0,
            discharge_time_target_s: 120.0,
            resistor_pulse_energy_rating_j: 100.0,
        })
        .unwrap();

        assert!(!report.discharge_time_numeric_screen_passes);
        assert!(!report.pulse_energy_numeric_screen_passes);
        assert!(report.worst_case_time_to_threshold_s > 120.0);
        assert!(report.pulse_energy_rating_margin_j < 0.0);
        assert!(report.claim_ceiling().contains("NOT_SERVICE_AUTHORIZATION"));
    }

    #[test]
    fn discharge_screen_rejects_zero_threshold_and_reversed_resistance_bounds() {
        let zero_threshold = screen_dc_link_discharge(DcLinkDischargeInput {
            capacitance_max_f: 1.0e-3,
            initial_voltage_max_v: 800.0,
            residual_voltage_threshold_v: 0.0,
            resistance_min_ohm: 100_000.0,
            resistance_max_ohm: 110_000.0,
            discharge_time_target_s: 300.0,
            resistor_pulse_energy_rating_j: 350.0,
        });
        let reversed_resistance = screen_dc_link_discharge(DcLinkDischargeInput {
            capacitance_max_f: 1.0e-3,
            initial_voltage_max_v: 800.0,
            residual_voltage_threshold_v: 60.0,
            resistance_min_ohm: 110_000.0,
            resistance_max_ohm: 100_000.0,
            discharge_time_target_s: 300.0,
            resistor_pulse_energy_rating_j: 350.0,
        });

        assert!(zero_threshold.is_err());
        assert!(reversed_resistance.is_err());
    }
}
