use crate::AuditError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerProfileQuality {
    Valid,
    Saturated,
    Dropped,
    ClockUncertain,
    MissingCalibration,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FacilityPowerSample {
    pub timestamp_ns: u64,
    /// Positive active power consumed behind the declared facility boundary.
    pub load_power_w: f64,
    pub quality: PowerProfileQuality,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StorageBufferInput {
    pub profile_id: String,
    pub samples: Vec<FacilityPowerSample>,
    /// Desired constant import at the declared grid / PCC boundary.
    pub grid_import_target_w: f64,
    /// Fraction of charging input energy retained in the storage state.
    pub charge_efficiency_fraction: f64,
    /// Fraction of stored energy delivered as useful discharge at the load boundary.
    pub discharge_efficiency_fraction: f64,
    pub available_usable_energy_capacity_j: f64,
    pub initial_stored_energy_j: f64,
    pub maximum_charge_power_w: f64,
    pub maximum_discharge_power_w: f64,
    pub maximum_allowed_sample_gap_ns: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StorageBufferSizingResult {
    pub profile_id: String,
    pub sample_count: usize,
    pub duration_s: f64,
    pub grid_import_target_w: f64,
    pub grid_import_energy_j: f64,
    pub load_energy_j: f64,
    /// Energy delivered by storage to the load boundary.
    pub discharge_energy_delivered_j: f64,
    /// Energy drawn from grid surplus at the load boundary for charging.
    pub charge_energy_input_j: f64,
    /// Internal energy change after charge/discharge efficiency.
    pub storage_internal_energy_change_j: f64,
    pub storage_conversion_loss_j: f64,
    pub minimum_required_usable_energy_capacity_j: f64,
    pub minimum_initial_stored_energy_j: f64,
    pub initial_stored_energy_j: f64,
    pub terminal_stored_energy_j: f64,
    pub minimum_actual_stored_energy_j: f64,
    pub maximum_actual_stored_energy_j: f64,
    pub peak_required_charge_power_w: f64,
    pub peak_required_discharge_power_w: f64,
    pub available_usable_energy_capacity_j: f64,
    pub maximum_charge_power_w: f64,
    pub maximum_discharge_power_w: f64,
    pub energy_capacity_within_limits: bool,
    pub charge_power_within_limit: bool,
    pub discharge_power_within_limit: bool,
    pub energy_balance_residual_j: f64,
    pub maximum_sample_gap_ns: u64,
}

impl StorageBufferSizingResult {
    pub const fn claim_ceiling(&self) -> &'static str {
        "IDEAL_CONSTANT_GRID_IMPORT_BUFFER_SIZING_ONLY_NOT_BATTERY_DYNAMICS_NOT_GFM_CONTROL_OR_GRID_QUALIFICATION"
    }
}

/// Estimate the ideal storage power and usable-energy requirement to hold a
/// constant grid-import target against a sampled facility load profile.
///
/// Positive (load - grid target) means storage discharges to the load.
/// Negative (load - grid target) means grid surplus charges storage.
///
/// Load power is linearly interpolated between adjacent samples for energy
/// integration only. When an interval crosses the grid target, the interval is
/// split at the exact linear crossing so charging and discharging energies are
/// not cancelled incorrectly. No samples are created and no missing data are
/// filled.
///
/// This is a sizing lower bound, not a storage controller or battery model. It
/// assumes ideal instantaneous power tracking at the declared boundary, fixed
/// charge/discharge efficiencies and no ramp, SOC, temperature, aging, voltage,
/// current, inverter or communication dynamics.
pub fn size_constant_import_buffer(
    input: StorageBufferInput,
) -> Result<StorageBufferSizingResult, AuditError> {
    if input.profile_id.trim().is_empty() {
        return Err(AuditError("storage profile id must not be blank".to_string()));
    }
    if input.samples.len() < 2 {
        return Err(AuditError(
            "storage sizing requires at least two power samples".to_string(),
        ));
    }
    let values = [
        input.grid_import_target_w,
        input.charge_efficiency_fraction,
        input.discharge_efficiency_fraction,
        input.available_usable_energy_capacity_j,
        input.initial_stored_energy_j,
        input.maximum_charge_power_w,
        input.maximum_discharge_power_w,
    ];
    if values.iter().any(|value| !value.is_finite())
        || input.grid_import_target_w < 0.0
        || input.charge_efficiency_fraction <= 0.0
        || input.charge_efficiency_fraction > 1.0
        || input.discharge_efficiency_fraction <= 0.0
        || input.discharge_efficiency_fraction > 1.0
        || input.available_usable_energy_capacity_j < 0.0
        || input.initial_stored_energy_j < 0.0
        || input.initial_stored_energy_j > input.available_usable_energy_capacity_j
        || input.maximum_charge_power_w < 0.0
        || input.maximum_discharge_power_w < 0.0
        || input.maximum_allowed_sample_gap_ns == 0
    {
        return Err(AuditError(
            "storage buffer limits, efficiencies or energy state are invalid".to_string(),
        ));
    }

    for (index, sample) in input.samples.iter().enumerate() {
        if sample.quality != PowerProfileQuality::Valid {
            return Err(AuditError(format!(
                "power sample {index}: quality {:?} is not eligible",
                sample.quality
            )));
        }
        if !sample.load_power_w.is_finite() || sample.load_power_w < 0.0 {
            return Err(AuditError(format!(
                "power sample {index}: load power must be finite and non-negative"
            )));
        }
        if index > 0 && sample.timestamp_ns <= input.samples[index - 1].timestamp_ns {
            return Err(AuditError(format!(
                "power sample {index}: timestamps must be strictly increasing"
            )));
        }
    }

    let mut maximum_sample_gap_ns = 0_u64;
    let mut load_energy_j = 0.0_f64;
    let mut grid_import_energy_j = 0.0_f64;
    let mut charge_energy_input_j = 0.0_f64;
    let mut discharge_energy_delivered_j = 0.0_f64;
    let mut cumulative_storage_change_j = 0.0_f64;
    let mut minimum_cumulative_change_j = 0.0_f64;
    let mut maximum_cumulative_change_j = 0.0_f64;
    let mut peak_charge_power_w = 0.0_f64;
    let mut peak_discharge_power_w = 0.0_f64;

    for pair in input.samples.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        let gap_ns = right.timestamp_ns - left.timestamp_ns;
        maximum_sample_gap_ns = maximum_sample_gap_ns.max(gap_ns);
        if gap_ns > input.maximum_allowed_sample_gap_ns {
            return Err(AuditError(format!(
                "power profile gap {gap_ns} ns exceeds declared maximum {} ns",
                input.maximum_allowed_sample_gap_ns
            )));
        }

        let duration_s = gap_ns as f64 / 1_000_000_000.0;
        let average_load_power_w = (left.load_power_w + right.load_power_w) * 0.5;
        let interval_load_energy_j = average_load_power_w * duration_s;
        let interval_grid_energy_j = input.grid_import_target_w * duration_s;
        if !interval_load_energy_j.is_finite() || !interval_grid_energy_j.is_finite() {
            return Err(AuditError(
                "integrated load or grid energy is non-finite".to_string(),
            ));
        }
        load_energy_j += interval_load_energy_j;
        grid_import_energy_j += interval_grid_energy_j;

        let net_left_w = left.load_power_w - input.grid_import_target_w;
        let net_right_w = right.load_power_w - input.grid_import_target_w;
        peak_discharge_power_w =
            peak_discharge_power_w.max(net_left_w.max(0.0)).max(net_right_w.max(0.0));
        peak_charge_power_w =
            peak_charge_power_w.max((-net_left_w).max(0.0)).max((-net_right_w).max(0.0));

        let mut segments: Vec<(bool, f64)> = Vec::with_capacity(2);
        // bool=true means discharge; false means charge. Each energy is measured
        // at the facility/load boundary before applying storage efficiency.
        if net_left_w >= 0.0 && net_right_w >= 0.0 {
            let energy = (net_left_w + net_right_w) * 0.5 * duration_s;
            segments.push((true, energy));
        } else if net_left_w <= 0.0 && net_right_w <= 0.0 {
            let energy = (-net_left_w - net_right_w) * 0.5 * duration_s;
            segments.push((false, energy));
        } else {
            let crossing_fraction =
                -net_left_w / (net_right_w - net_left_w);
            if !crossing_fraction.is_finite()
                || crossing_fraction <= 0.0
                || crossing_fraction >= 1.0
            {
                return Err(AuditError(
                    "storage power crossing fraction is outside its physical domain".to_string(),
                ));
            }
            let left_duration_s = crossing_fraction * duration_s;
            let right_duration_s = (1.0 - crossing_fraction) * duration_s;
            if net_left_w < 0.0 {
                let charge_energy =
                    (-net_left_w) * left_duration_s * 0.5;
                let discharge_energy =
                    net_right_w * right_duration_s * 0.5;
                segments.push((false, charge_energy));
                segments.push((true, discharge_energy));
            } else {
                let discharge_energy =
                    net_left_w * left_duration_s * 0.5;
                let charge_energy =
                    (-net_right_w) * right_duration_s * 0.5;
                segments.push((true, discharge_energy));
                segments.push((false, charge_energy));
            }
        }

        for (is_discharge, boundary_energy_j) in segments {
            if !boundary_energy_j.is_finite() || boundary_energy_j < 0.0 {
                return Err(AuditError(
                    "derived storage interval energy is invalid".to_string(),
                ));
            }
            if is_discharge {
                discharge_energy_delivered_j += boundary_energy_j;
                cumulative_storage_change_j -=
                    boundary_energy_j / input.discharge_efficiency_fraction;
            } else {
                charge_energy_input_j += boundary_energy_j;
                cumulative_storage_change_j +=
                    boundary_energy_j * input.charge_efficiency_fraction;
            }
            minimum_cumulative_change_j =
                minimum_cumulative_change_j.min(cumulative_storage_change_j);
            maximum_cumulative_change_j =
                maximum_cumulative_change_j.max(cumulative_storage_change_j);
        }
    }

    let first = input.samples.first().ok_or_else(|| {
        AuditError("storage profile has no first sample".to_string())
    })?;
    let last = input.samples.last().ok_or_else(|| {
        AuditError("storage profile has no last sample".to_string())
    })?;
    let duration_ns = last.timestamp_ns - first.timestamp_ns;
    let duration_s = duration_ns as f64 / 1_000_000_000.0;
    let storage_internal_energy_change_j =
        charge_energy_input_j * input.charge_efficiency_fraction
            - discharge_energy_delivered_j / input.discharge_efficiency_fraction;
    let storage_conversion_loss_j =
        charge_energy_input_j * (1.0 - input.charge_efficiency_fraction)
            + discharge_energy_delivered_j
                * (1.0 / input.discharge_efficiency_fraction - 1.0);
    let minimum_required_usable_energy_capacity_j =
        maximum_cumulative_change_j - minimum_cumulative_change_j;
    let minimum_initial_stored_energy_j = -minimum_cumulative_change_j;
    let terminal_stored_energy_j =
        input.initial_stored_energy_j + storage_internal_energy_change_j;
    let minimum_actual_stored_energy_j =
        input.initial_stored_energy_j + minimum_cumulative_change_j;
    let maximum_actual_stored_energy_j =
        input.initial_stored_energy_j + maximum_cumulative_change_j;
    let energy_balance_residual_j = grid_import_energy_j
        + discharge_energy_delivered_j
        - load_energy_j
        - charge_energy_input_j;

    let derived = [
        duration_s,
        load_energy_j,
        grid_import_energy_j,
        charge_energy_input_j,
        discharge_energy_delivered_j,
        storage_internal_energy_change_j,
        storage_conversion_loss_j,
        minimum_required_usable_energy_capacity_j,
        minimum_initial_stored_energy_j,
        terminal_stored_energy_j,
        minimum_actual_stored_energy_j,
        maximum_actual_stored_energy_j,
        peak_charge_power_w,
        peak_discharge_power_w,
        energy_balance_residual_j,
    ];
    if derived.iter().any(|value| !value.is_finite())
        || duration_s <= 0.0
        || minimum_required_usable_energy_capacity_j < 0.0
        || storage_conversion_loss_j < 0.0
        || peak_charge_power_w < 0.0
        || peak_discharge_power_w < 0.0
    {
        return Err(AuditError(
            "derived storage buffer result is non-finite or outside its physical domain".to_string(),
        ));
    }

    let energy_tolerance_j = 1.0e-9
        * grid_import_energy_j
            .abs()
            .max(load_energy_j.abs())
            .max(charge_energy_input_j.abs())
            .max(discharge_energy_delivered_j.abs())
            .max(1.0);
    if energy_balance_residual_j.abs() > energy_tolerance_j {
        return Err(AuditError(
            "storage buffer bus-side energy balance does not close".to_string(),
        ));
    }

    let state_tolerance_j = 1.0e-9 * input.available_usable_energy_capacity_j.max(1.0);
    let energy_capacity_within_limits =
        minimum_actual_stored_energy_j >= -state_tolerance_j
            && maximum_actual_stored_energy_j
                <= input.available_usable_energy_capacity_j + state_tolerance_j;
    let charge_power_within_limit =
        peak_charge_power_w <= input.maximum_charge_power_w;
    let discharge_power_within_limit =
        peak_discharge_power_w <= input.maximum_discharge_power_w;

    Ok(StorageBufferSizingResult {
        profile_id: input.profile_id,
        sample_count: input.samples.len(),
        duration_s,
        grid_import_target_w: input.grid_import_target_w,
        grid_import_energy_j,
        load_energy_j,
        discharge_energy_delivered_j,
        charge_energy_input_j,
        storage_internal_energy_change_j,
        storage_conversion_loss_j,
        minimum_required_usable_energy_capacity_j,
        minimum_initial_stored_energy_j,
        initial_stored_energy_j: input.initial_stored_energy_j,
        terminal_stored_energy_j,
        minimum_actual_stored_energy_j,
        maximum_actual_stored_energy_j,
        peak_required_charge_power_w: peak_charge_power_w,
        peak_required_discharge_power_w: peak_discharge_power_w,
        available_usable_energy_capacity_j: input.available_usable_energy_capacity_j,
        maximum_charge_power_w: input.maximum_charge_power_w,
        maximum_discharge_power_w: input.maximum_discharge_power_w,
        energy_capacity_within_limits,
        charge_power_within_limit,
        discharge_power_within_limit,
        energy_balance_residual_j,
        maximum_sample_gap_ns,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        size_constant_import_buffer, FacilityPowerSample, PowerProfileQuality,
        StorageBufferInput,
    };

    fn sample(timestamp_ns: u64, load_power_w: f64) -> FacilityPowerSample {
        FacilityPowerSample {
            timestamp_ns,
            load_power_w,
            quality: PowerProfileQuality::Valid,
        }
    }

    fn input(samples: Vec<FacilityPowerSample>) -> StorageBufferInput {
        StorageBufferInput {
            profile_id: "synthetic-grid-buffer".to_string(),
            samples,
            grid_import_target_w: 1000.0,
            charge_efficiency_fraction: 0.9,
            discharge_efficiency_fraction: 0.95,
            available_usable_energy_capacity_j: 2000.0,
            initial_stored_energy_j: 1200.0,
            maximum_charge_power_w: 1500.0,
            maximum_discharge_power_w: 1500.0,
            maximum_allowed_sample_gap_ns: 1_000_000_000,
        }
    }

    #[test]
    fn sizes_discharge_power_and_energy_for_a_two_second_load_pulse() {
        let report = size_constant_import_buffer(input(vec![
            sample(0, 1000.0),
            sample(1_000_000_000, 2000.0),
            sample(2_000_000_000, 1000.0),
        ]))
        .unwrap();

        let expected_depletion = 1000.0 / 0.95;
        assert!((report.load_energy_j - 3000.0).abs() < 1.0e-9);
        assert!((report.grid_import_energy_j - 2000.0).abs() < 1.0e-9);
        assert!((report.discharge_energy_delivered_j - 1000.0).abs() < 1.0e-9);
        assert!((report.storage_internal_energy_change_j + expected_depletion).abs() < 1.0e-9);
        assert!((report.minimum_required_usable_energy_capacity_j - expected_depletion).abs() < 1.0e-9);
        assert!((report.minimum_initial_stored_energy_j - expected_depletion).abs() < 1.0e-9);
        assert!(report.energy_capacity_within_limits);
        assert!(report.discharge_power_within_limit);
        assert!(report.energy_balance_residual_j.abs() < 1.0e-9);
    }

    #[test]
    fn integrates_charge_surplus_with_charging_efficiency() {
        let mut input = input(vec![
            sample(0, 1000.0),
            sample(1_000_000_000, 0.0),
            sample(2_000_000_000, 1000.0),
        ]);
        input.initial_stored_energy_j = 0.0;

        let report = size_constant_import_buffer(input).unwrap();

        assert!((report.charge_energy_input_j - 1000.0).abs() < 1.0e-9);
        assert!((report.storage_internal_energy_change_j - 900.0).abs() < 1.0e-9);
        assert!((report.minimum_required_usable_energy_capacity_j - 900.0).abs() < 1.0e-9);
        assert!((report.terminal_stored_energy_j - 900.0).abs() < 1.0e-9);
        assert!(report.charge_power_within_limit);
        assert!(report.energy_capacity_within_limits);
    }

    #[test]
    fn splits_an_interval_that_crosses_the_grid_import_target() {
        let mut input = input(vec![sample(0, 500.0), sample(1_000_000_000, 1500.0)]);
        input.initial_stored_energy_j = 40.0;

        let report = size_constant_import_buffer(input).unwrap();

        assert!((report.charge_energy_input_j - 125.0).abs() < 1.0e-9);
        assert!((report.discharge_energy_delivered_j - 125.0).abs() < 1.0e-9);
        let expected_delta = 125.0 * 0.9 - 125.0 / 0.95;
        assert!((report.storage_internal_energy_change_j - expected_delta).abs() < 1.0e-9);
        assert!(report.energy_balance_residual_j.abs() < 1.0e-9);
    }

    #[test]
    fn reports_energy_and_power_limit_failures_without_clipping() {
        let mut input = input(vec![
            sample(0, 1000.0),
            sample(1_000_000_000, 2000.0),
            sample(2_000_000_000, 1000.0),
        ]);
        input.available_usable_energy_capacity_j = 100.0;
        input.initial_stored_energy_j = 100.0;
        input.maximum_discharge_power_w = 500.0;

        let report = size_constant_import_buffer(input).unwrap();

        assert!(!report.energy_capacity_within_limits);
        assert!(!report.discharge_power_within_limit);
        assert!(report.minimum_actual_stored_energy_j < 0.0);
        assert!(report.peak_required_discharge_power_w > 500.0);
    }

    #[test]
    fn rejects_bad_timestamps_gaps_quality_and_invalid_efficiency() {
        let duplicate = size_constant_import_buffer(input(vec![
            sample(0, 1000.0),
            sample(0, 1100.0),
        ]));
        let excessive_gap = size_constant_import_buffer(input(vec![
            sample(0, 1000.0),
            sample(2_000_000_000, 1100.0),
        ]));
        let mut bad_quality = sample(1_000_000_000, 1100.0);
        bad_quality.quality = PowerProfileQuality::Dropped;
        let quality = size_constant_import_buffer(input(vec![
            sample(0, 1000.0),
            bad_quality,
        ]));
        let mut bad_efficiency = input(vec![
            sample(0, 1000.0),
            sample(1_000_000_000, 1100.0),
        ]);
        bad_efficiency.charge_efficiency_fraction = 1.1;

        assert!(duplicate.is_err());
        assert!(excessive_gap.is_err());
        assert!(quality.is_err());
        assert!(size_constant_import_buffer(bad_efficiency).is_err());
    }

    #[test]
    fn rejects_nonfinite_and_impossible_storage_state() {
        let nonfinite = size_constant_import_buffer(input(vec![
            sample(0, 1000.0),
            sample(1_000_000_000, f64::INFINITY),
        ]));
        let mut impossible = input(vec![
            sample(0, 1000.0),
            sample(1_000_000_000, 1000.0),
        ]);
        impossible.initial_stored_energy_j = 2500.0;

        assert!(nonfinite.is_err());
        assert!(size_constant_import_buffer(impossible).is_err());
    }
}
