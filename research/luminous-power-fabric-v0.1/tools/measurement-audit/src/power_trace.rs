use crate::AuditError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadState {
    Idle,
    Steady,
    Burst,
    Transition,
    Recovery,
    Unknown,
}

impl WorkloadState {
    const ALL: [Self; 6] = [
        Self::Idle,
        Self::Steady,
        Self::Burst,
        Self::Transition,
        Self::Recovery,
        Self::Unknown,
    ];

    fn index(self) -> usize {
        match self {
            Self::Idle => 0,
            Self::Steady => 1,
            Self::Burst => 2,
            Self::Transition => 3,
            Self::Recovery => 4,
            Self::Unknown => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceSampleQuality {
    Valid,
    Saturated,
    DroppedSample,
    ClockUncertain,
    MissingCalibration,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridToChipTraceSample {
    pub timestamp_ns: u64,
    /// Positive current is drawn from the source bus into the conversion system.
    pub source_bus_voltage_v: f64,
    pub source_bus_current_a: f64,
    /// Positive current is delivered from the final converter into the processor rail.
    pub processor_rail_voltage_v: f64,
    pub processor_rail_current_a: f64,
    pub workload_state: WorkloadState,
    pub quality: TraceSampleQuality,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridToChipTraceMetadata {
    pub nominal_sample_interval_ns: u64,
    pub maximum_allowed_sample_gap_ns: u64,
    pub actual_clock_sync_uncertainty_ns: u64,
    pub maximum_allowed_clock_sync_uncertainty_ns: u64,
    pub nominal_processor_rail_voltage_v: f64,
    pub maximum_processor_rail_deviation_fraction: f64,
    /// Total stored energy inside the declared boundary at the first sample.
    pub initial_stored_energy_j: f64,
    /// Total stored energy inside the declared boundary at the final sample.
    pub final_stored_energy_j: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridToChipTrace {
    pub trace_id: String,
    pub architecture_id: String,
    pub workload_id: String,
    pub metadata: GridToChipTraceMetadata,
    pub samples: Vec<GridToChipTraceSample>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkloadStateDuration {
    pub state: WorkloadState,
    /// Each interval is assigned the state declared by its first sample.
    pub duration_s: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridToChipTraceAudit {
    pub trace_id: String,
    pub architecture_id: String,
    pub workload_id: String,
    pub sample_count: usize,
    pub duration_s: f64,
    pub mean_sample_interval_ns: f64,
    pub maximum_sample_gap_ns: u64,
    pub maximum_absolute_interval_deviation_ns: u64,
    pub input_energy_j: f64,
    pub delivered_processor_energy_j: f64,
    /// final stored energy minus initial stored energy; positive means charging.
    pub stored_energy_change_j: f64,
    /// Input energy less stored-energy increase.
    pub effective_energy_available_j: f64,
    pub conversion_chain_loss_j: f64,
    /// Efficiency after accounting for the change in stored energy.
    pub storage_adjusted_efficiency_percent: f64,
    /// Raw delivered/input ratio. May exceed 100% when stored energy is depleted.
    pub raw_boundary_output_to_input_ratio_percent: f64,
    pub maximum_source_input_power_w: f64,
    pub maximum_processor_load_power_w: f64,
    pub minimum_processor_rail_voltage_v: f64,
    pub maximum_processor_rail_voltage_v: f64,
    pub maximum_processor_rail_deviation_percent: f64,
    pub within_declared_processor_rail_deviation: bool,
    pub clock_synchronization_within_budget: bool,
    pub workload_state_durations: Vec<WorkloadStateDuration>,
}

impl GridToChipTraceAudit {
    pub const fn claim_ceiling(&self) -> &'static str {
        "TIME_ALIGNED_BOUNDARY_ENERGY_ACCOUNTING_ONLY_NOT_SWITCHING_MODEL_NOT_THERMAL_OR_POWER_QUALITY_QUALIFICATION"
    }
}

/// Audit a typed, time-aligned source-to-processor trace using trapezoidal energy
/// integration and explicit initial/final stored-energy accounting.
///
/// The source boundary current is positive into the conversion system. The
/// processor rail current is positive delivered to the load. Timestamps must be
/// strictly increasing, every sample must be marked Valid, and no interval may
/// exceed the declared maximum gap. No samples are interpolated or silently
/// repaired.
///
/// For the declared boundary:
///   E_loss = E_source - E_processor - (E_stored,final - E_stored,initial)
///
/// A depleted hold-up store can make the raw E_processor / E_source ratio
/// exceed 100%. The storage-adjusted efficiency uses E_source - delta(E_stored)
/// as the available energy denominator. This is an energy-accounting result,
/// not a converter-only efficiency claim if the boundary includes storage.
pub fn audit_grid_to_chip_trace(
    trace: GridToChipTrace,
) -> Result<GridToChipTraceAudit, AuditError> {
    if trace.trace_id.trim().is_empty()
        || trace.architecture_id.trim().is_empty()
        || trace.workload_id.trim().is_empty()
    {
        return Err(AuditError(
            "trace, architecture and workload identities must not be blank".to_string(),
        ));
    }
    if trace.samples.len() < 2 {
        return Err(AuditError(
            "grid-to-chip trace requires at least two samples".to_string(),
        ));
    }

    let metadata = &trace.metadata;
    if metadata.nominal_sample_interval_ns == 0
        || metadata.maximum_allowed_sample_gap_ns == 0
        || !metadata.nominal_processor_rail_voltage_v.is_finite()
        || metadata.nominal_processor_rail_voltage_v <= 0.0
        || !metadata.maximum_processor_rail_deviation_fraction.is_finite()
        || metadata.maximum_processor_rail_deviation_fraction < 0.0
        || metadata.maximum_processor_rail_deviation_fraction >= 1.0
        || !metadata.initial_stored_energy_j.is_finite()
        || metadata.initial_stored_energy_j < 0.0
        || !metadata.final_stored_energy_j.is_finite()
        || metadata.final_stored_energy_j < 0.0
    {
        return Err(AuditError(
            "grid-to-chip trace metadata is invalid or outside its physical domain".to_string(),
        ));
    }

    let mut input_energy_j = 0.0_f64;
    let mut delivered_energy_j = 0.0_f64;
    let mut maximum_source_power_w = 0.0_f64;
    let mut maximum_processor_power_w = 0.0_f64;
    let mut minimum_processor_voltage_v = f64::INFINITY;
    let mut maximum_processor_voltage_v = f64::NEG_INFINITY;
    let mut maximum_rail_deviation_percent = 0.0_f64;
    let mut maximum_sample_gap_ns = 0_u64;
    let mut maximum_absolute_interval_deviation_ns = 0_u64;
    let mut state_durations_s = [0.0_f64; 6];

    for (index, sample) in trace.samples.iter().enumerate() {
        if sample.quality != TraceSampleQuality::Valid {
            return Err(AuditError(format!(
                "sample {index}: quality state {:?} is not eligible for energy comparison",
                sample.quality
            )));
        }
        let values = [
            sample.source_bus_voltage_v,
            sample.source_bus_current_a,
            sample.processor_rail_voltage_v,
            sample.processor_rail_current_a,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(AuditError(format!(
                "sample {index}: voltage and current values must be finite"
            )));
        }
        if sample.source_bus_voltage_v <= 0.0
            || sample.source_bus_current_a < 0.0
            || sample.processor_rail_voltage_v <= 0.0
            || sample.processor_rail_current_a < 0.0
        {
            return Err(AuditError(format!(
                "sample {index}: invalid voltage or declared positive-current domain"
            )));
        }

        let source_power_w = sample.source_bus_voltage_v * sample.source_bus_current_a;
        let processor_power_w =
            sample.processor_rail_voltage_v * sample.processor_rail_current_a;
        let rail_deviation_percent = 100.0
            * (sample.processor_rail_voltage_v
                - metadata.nominal_processor_rail_voltage_v)
                .abs()
            / metadata.nominal_processor_rail_voltage_v;
        if !source_power_w.is_finite()
            || !processor_power_w.is_finite()
            || !rail_deviation_percent.is_finite()
        {
            return Err(AuditError(format!(
                "sample {index}: derived power or voltage deviation is non-finite"
            )));
        }

        maximum_source_power_w = maximum_source_power_w.max(source_power_w);
        maximum_processor_power_w = maximum_processor_power_w.max(processor_power_w);
        minimum_processor_voltage_v =
            minimum_processor_voltage_v.min(sample.processor_rail_voltage_v);
        maximum_processor_voltage_v =
            maximum_processor_voltage_v.max(sample.processor_rail_voltage_v);
        maximum_rail_deviation_percent =
            maximum_rail_deviation_percent.max(rail_deviation_percent);

        if index > 0 {
            let previous = &trace.samples[index - 1];
            if sample.timestamp_ns <= previous.timestamp_ns {
                return Err(AuditError(format!(
                    "sample {index}: timestamps must be strictly increasing"
                )));
            }
            let gap_ns = sample.timestamp_ns - previous.timestamp_ns;
            if gap_ns > metadata.maximum_allowed_sample_gap_ns {
                return Err(AuditError(format!(
                    "sample {index}: gap {gap_ns} ns exceeds declared maximum {} ns",
                    metadata.maximum_allowed_sample_gap_ns
                )));
            }
            maximum_sample_gap_ns = maximum_sample_gap_ns.max(gap_ns);
            let interval_deviation = gap_ns.abs_diff(metadata.nominal_sample_interval_ns);
            maximum_absolute_interval_deviation_ns =
                maximum_absolute_interval_deviation_ns.max(interval_deviation);

            let duration_s = gap_ns as f64 / 1_000_000_000.0;
            let previous_source_power_w =
                previous.source_bus_voltage_v * previous.source_bus_current_a;
            let previous_processor_power_w =
                previous.processor_rail_voltage_v * previous.processor_rail_current_a;
            input_energy_j +=
                (previous_source_power_w + source_power_w) * 0.5 * duration_s;
            delivered_energy_j +=
                (previous_processor_power_w + processor_power_w) * 0.5 * duration_s;
            state_durations_s[previous.workload_state.index()] += duration_s;
        }
    }

    let first = trace.samples.first().ok_or_else(|| {
        AuditError("grid-to-chip trace lost its first sample during audit".to_string())
    })?;
    let last = trace.samples.last().ok_or_else(|| {
        AuditError("grid-to-chip trace lost its final sample during audit".to_string())
    })?;
    let total_duration_ns = last.timestamp_ns - first.timestamp_ns;
    let duration_s = total_duration_ns as f64 / 1_000_000_000.0;
    if !duration_s.is_finite()
        || duration_s <= 0.0
        || !input_energy_j.is_finite()
        || input_energy_j <= 0.0
        || !delivered_energy_j.is_finite()
        || delivered_energy_j <= 0.0
    {
        return Err(AuditError(
            "trace duration and integrated source / processor energies must be positive and finite"
                .to_string(),
        ));
    }

    let stored_energy_change_j =
        metadata.final_stored_energy_j - metadata.initial_stored_energy_j;
    let effective_energy_available_j = input_energy_j - stored_energy_change_j;
    let conversion_chain_loss_j =
        effective_energy_available_j - delivered_energy_j;
    let loss_tolerance_j =
        1.0e-9 * effective_energy_available_j.abs().max(delivered_energy_j.abs()).max(1.0);
    if !stored_energy_change_j.is_finite()
        || !effective_energy_available_j.is_finite()
        || !conversion_chain_loss_j.is_finite()
        || effective_energy_available_j <= 0.0
        || conversion_chain_loss_j < -loss_tolerance_j
    {
        return Err(AuditError(
            "trace energy balance is invalid; check boundary, storage delta, signs and synchronized channels"
                .to_string(),
        ));
    }

    let storage_adjusted_efficiency_percent =
        100.0 * delivered_energy_j / effective_energy_available_j;
    let raw_boundary_output_to_input_ratio_percent =
        100.0 * delivered_energy_j / input_energy_j;
    let mean_sample_interval_ns =
        total_duration_ns as f64 / (trace.samples.len() - 1) as f64;

    let derived = [
        storage_adjusted_efficiency_percent,
        raw_boundary_output_to_input_ratio_percent,
        mean_sample_interval_ns,
        maximum_source_power_w,
        maximum_processor_power_w,
        minimum_processor_voltage_v,
        maximum_processor_voltage_v,
        maximum_rail_deviation_percent,
    ];
    if derived.iter().any(|value| !value.is_finite() || *value < 0.0)
        || storage_adjusted_efficiency_percent > 100.0 + 1.0e-9
    {
        return Err(AuditError(
            "derived grid-to-chip trace result is non-finite or outside its physical domain"
                .to_string(),
        ));
    }

    let workload_state_durations_s = WorkloadState::ALL
        .iter()
        .enumerate()
        .map(|(index, state)| WorkloadStateDuration {
            state: *state,
            duration_s: state_durations_s[index],
        })
        .collect();

    Ok(GridToChipTraceAudit {
        trace_id: trace.trace_id,
        architecture_id: trace.architecture_id,
        workload_id: trace.workload_id,
        sample_count: trace.samples.len(),
        duration_s,
        mean_sample_interval_ns,
        maximum_sample_gap_ns,
        maximum_absolute_interval_deviation_ns,
        input_energy_j,
        delivered_processor_energy_j: delivered_energy_j,
        stored_energy_change_j,
        effective_energy_available_j,
        conversion_chain_loss_j,
        storage_adjusted_efficiency_percent,
        raw_boundary_output_to_input_ratio_percent,
        maximum_source_input_power_w: maximum_source_power_w,
        maximum_processor_load_power_w: maximum_processor_power_w,
        minimum_processor_rail_voltage_v: minimum_processor_voltage_v,
        maximum_processor_rail_voltage_v: maximum_processor_voltage_v,
        maximum_processor_rail_deviation_percent: maximum_rail_deviation_percent,
        within_declared_processor_rail_deviation: maximum_rail_deviation_percent
            <= metadata.maximum_processor_rail_deviation_fraction * 100.0,
        clock_synchronization_within_budget: metadata.actual_clock_sync_uncertainty_ns
            <= metadata.maximum_allowed_clock_sync_uncertainty_ns,
        workload_state_durations: workload_state_durations_s,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        audit_grid_to_chip_trace, GridToChipTrace, GridToChipTraceMetadata,
        GridToChipTraceSample, TraceSampleQuality, WorkloadState,
    };

    fn metadata() -> GridToChipTraceMetadata {
        GridToChipTraceMetadata {
            nominal_sample_interval_ns: 1_000_000_000,
            maximum_allowed_sample_gap_ns: 3_000_000_000,
            actual_clock_sync_uncertainty_ns: 5,
            maximum_allowed_clock_sync_uncertainty_ns: 10,
            nominal_processor_rail_voltage_v: 0.9,
            maximum_processor_rail_deviation_fraction: 0.02,
            initial_stored_energy_j: 100.0,
            final_stored_energy_j: 200.0,
        }
    }

    fn sample(
        timestamp_ns: u64,
        processor_current_a: f64,
        workload_state: WorkloadState,
    ) -> GridToChipTraceSample {
        GridToChipTraceSample {
            timestamp_ns,
            source_bus_voltage_v: 100.0,
            source_bus_current_a: 10.0,
            processor_rail_voltage_v: 0.9,
            processor_rail_current_a: processor_current_a,
            workload_state,
            quality: TraceSampleQuality::Valid,
        }
    }

    fn trace(samples: Vec<GridToChipTraceSample>) -> GridToChipTrace {
        GridToChipTrace {
            trace_id: "trace-001".to_string(),
            architecture_id: "architecture-001".to_string(),
            workload_id: "workload-001".to_string(),
            metadata: metadata(),
            samples,
        }
    }

    #[test]
    fn integrates_irregular_samples_and_accounts_for_stored_energy_change() {
        let report = audit_grid_to_chip_trace(trace(vec![
            sample(0, 1000.0, WorkloadState::Idle),
            sample(1_000_000_000, 1022.2222222222, WorkloadState::Burst),
            sample(3_000_000_000, 1044.4444444444, WorkloadState::Recovery),
        ]))
        .unwrap();

        assert!((report.input_energy_j - 3000.0).abs() < 1.0e-9);
        assert!((report.delivered_processor_energy_j - 2770.0).abs() < 1.0e-6);
        assert!((report.stored_energy_change_j - 100.0).abs() < 1.0e-9);
        assert!((report.effective_energy_available_j - 2900.0).abs() < 1.0e-9);
        assert!((report.conversion_chain_loss_j - 130.0).abs() < 1.0e-6);
        assert!((report.storage_adjusted_efficiency_percent - (2770.0 / 2900.0 * 100.0)).abs() < 1.0e-8);
        assert_eq!(report.maximum_sample_gap_ns, 2_000_000_000);
        assert_eq!(report.mean_sample_interval_ns, 1_500_000_000.0);
        assert!(report.clock_synchronization_within_budget);
        assert_eq!(report.workload_state_durations[0].duration_s, 1.0);
        assert_eq!(report.workload_state_durations[2].duration_s, 2.0);
        assert!(report.within_declared_processor_rail_deviation);
    }

    #[test]
    fn depleted_storage_can_make_raw_ratio_exceed_one_hundred_percent() {
        let mut meta = metadata();
        meta.nominal_sample_interval_ns = 10_000_000_000;
        meta.maximum_allowed_sample_gap_ns = 10_000_000_000;
        meta.initial_stored_energy_j = 300.0;
        meta.final_stored_energy_j = 100.0;

        let mut t = trace(vec![
            sample(0, 122.2222222222, WorkloadState::Steady),
            sample(10_000_000_000, 122.2222222222, WorkloadState::Steady),
        ]);
        t.metadata = meta;
        t.samples[0].source_bus_current_a = 1.0;
        t.samples[1].source_bus_current_a = 1.0;

        let report = audit_grid_to_chip_trace(t).unwrap();

        assert!(report.raw_boundary_output_to_input_ratio_percent > 100.0);
        assert!(report.storage_adjusted_efficiency_percent < 100.0);
        assert!((report.stored_energy_change_j + 200.0).abs() < 1.0e-9);
        assert!((report.conversion_chain_loss_j - 100.0).abs() < 1.0e-6);
    }

    #[test]
    fn rejects_out_of_order_duplicate_and_excessive_sample_gaps() {
        let out_of_order = audit_grid_to_chip_trace(trace(vec![
            sample(0, 1000.0, WorkloadState::Steady),
            sample(0, 1000.0, WorkloadState::Steady),
        ]));
        let excessive_gap = audit_grid_to_chip_trace(trace(vec![
            sample(0, 1000.0, WorkloadState::Steady),
            sample(4_000_000_000, 1000.0, WorkloadState::Steady),
        ]));

        assert!(out_of_order.is_err());
        assert!(excessive_gap.is_err());
    }

    #[test]
    fn rejects_invalid_or_nonfinite_samples() {
        let mut invalid_quality = sample(1_000_000_000, 1000.0, WorkloadState::Steady);
        invalid_quality.quality = TraceSampleQuality::DroppedSample;
        let invalid = audit_grid_to_chip_trace(trace(vec![
            sample(0, 1000.0, WorkloadState::Steady),
            invalid_quality,
        ]));

        let mut nonfinite = sample(1_000_000_000, 1000.0, WorkloadState::Steady);
        nonfinite.processor_rail_voltage_v = f64::NAN;
        let nan = audit_grid_to_chip_trace(trace(vec![
            sample(0, 1000.0, WorkloadState::Steady),
            nonfinite,
        ]));

        assert!(invalid.is_err());
        assert!(nan.is_err());
    }

    #[test]
    fn reports_clock_and_rail_limit_violations_without_clipping() {
        let mut t = trace(vec![
            sample(0, 1000.0, WorkloadState::Idle),
            sample(1_000_000_000, 1000.0, WorkloadState::Steady),
        ]);
        t.metadata.actual_clock_sync_uncertainty_ns = 50;
        t.samples[1].processor_rail_voltage_v = 0.87;

        let report = audit_grid_to_chip_trace(t).unwrap();

        assert!(!report.clock_synchronization_within_budget);
        assert!(!report.within_declared_processor_rail_deviation);
        assert!(report.minimum_processor_rail_voltage_v < 0.9);
        assert!(report.maximum_processor_rail_deviation_percent > 2.0);
    }

    #[test]
    fn rejects_energy_balance_that_requires_negative_conversion_loss() {
        let mut t = trace(vec![
            sample(0, 1300.0, WorkloadState::Steady),
            sample(1_000_000_000, 1300.0, WorkloadState::Steady),
        ]);
        t.metadata.initial_stored_energy_j = 100.0;
        t.metadata.final_stored_energy_j = 0.0;

        assert!(audit_grid_to_chip_trace(t).is_err());
    }

    #[test]
    fn rejects_empty_identity_bad_metadata_and_zero_delivered_energy() {
        let mut empty_identity = trace(vec![
            sample(0, 1000.0, WorkloadState::Steady),
            sample(1_000_000_000, 1000.0, WorkloadState::Steady),
        ]);
        empty_identity.trace_id.clear();

        let mut invalid_metadata = trace(vec![
            sample(0, 1000.0, WorkloadState::Steady),
            sample(1_000_000_000, 1000.0, WorkloadState::Steady),
        ]);
        invalid_metadata.metadata.nominal_processor_rail_voltage_v = 0.0;

        let zero_output = trace(vec![
            sample(0, 0.0, WorkloadState::Idle),
            sample(1_000_000_000, 0.0, WorkloadState::Idle),
        ]);

        assert!(audit_grid_to_chip_trace(empty_identity).is_err());
        assert!(audit_grid_to_chip_trace(invalid_metadata).is_err());
        assert!(audit_grid_to_chip_trace(zero_output).is_err());
    }
}
