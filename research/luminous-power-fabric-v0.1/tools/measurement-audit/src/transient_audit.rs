use crate::AuditError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransientSampleQuality {
    Valid,
    Saturated,
    Dropped,
    ClockUncertain,
    MissingCalibration,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RailTransientSample {
    pub timestamp_ns: u64,
    /// Positive current means current delivered into the processor rail.
    pub load_current_a: f64,
    pub rail_voltage_v: f64,
    pub quality: TransientSampleQuality,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RailTransientEvent {
    pub event_id: String,
    pub samples: Vec<RailTransientSample>,
    /// Baseline samples begin at this exact timestamp.
    pub baseline_window_start_ns: u64,
    /// The declared load-step event begins at this exact timestamp.
    pub step_start_ns: u64,
    /// Steady-state evaluation window begins at this exact timestamp.
    pub steady_window_start_ns: u64,
    /// The evaluation window ends at this exact timestamp.
    pub evaluation_end_ns: u64,
    pub nominal_rail_voltage_v: f64,
    pub voltage_tolerance_fraction: f64,
    pub target_post_step_load_current_a: f64,
    pub load_current_absolute_tolerance_a: f64,
    pub settling_dwell_ns: u64,
    pub maximum_allowed_sample_gap_ns: u64,
    pub actual_clock_sync_uncertainty_ns: u64,
    pub maximum_allowed_clock_sync_uncertainty_ns: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RailTransientAudit {
    pub event_id: String,
    pub sample_count: usize,
    pub measured_pre_step_load_current_a: f64,
    pub measured_post_step_load_current_a: f64,
    pub measured_load_step_delta_a: f64,
    pub target_load_step_delta_a: f64,
    pub post_step_current_error_a: f64,
    pub post_step_current_within_tolerance: bool,
    pub maximum_upward_load_slew_a_per_us: f64,
    pub maximum_downward_load_slew_a_per_us: f64,
    pub minimum_post_step_rail_voltage_v: f64,
    pub maximum_post_step_rail_voltage_v: f64,
    pub maximum_undershoot_v: f64,
    pub maximum_overshoot_v: f64,
    pub maximum_undershoot_percent: f64,
    pub maximum_overshoot_percent: f64,
    pub rail_voltage_within_tolerance_at_end: bool,
    /// First in-band sample after the step that remains in-band through the
    /// evaluation end, with at least the declared dwell observed.
    pub settling_time_ns: Option<u64>,
    pub declared_settling_dwell_ns: u64,
    pub maximum_sample_gap_ns: u64,
    pub maximum_rail_deviation_percent: f64,
    pub clock_synchronization_within_budget: bool,
}

impl RailTransientAudit {
    pub const fn claim_ceiling(&self) -> &'static str {
        "SAMPLED_RAIL_STEP_METRICS_ONLY_NOT_LOOP_STABILITY_NOT_FULL_BANDWIDTH_PI_OR_PRODUCT_QUALIFICATION"
    }
}

/// Audit a declared processor-rail load-step event using exact sample boundaries.
///
/// No interpolation, deconvolution, filtering or inferred timestamp correction is
/// performed. The caller must provide exact sample timestamps for the baseline
/// start, step start, steady-window start and evaluation end. Every sample must
/// be valid and no adjacent sample gap may exceed the declared maximum.
///
/// The upward/downward slew values are maxima of adjacent sampled differences;
/// they are not estimates of unobserved edge slew between samples. Settling time
/// is reported only if the rail enters its declared tolerance band and remains
/// there through the evaluation end for at least the declared dwell.
pub fn audit_rail_transient(
    event: RailTransientEvent,
) -> Result<RailTransientAudit, AuditError> {
    if event.event_id.trim().is_empty() {
        return Err(AuditError("transient event id must not be blank".to_string()));
    }
    if event.samples.len() < 4 {
        return Err(AuditError(
            "transient event requires at least four samples".to_string(),
        ));
    }
    if !event.nominal_rail_voltage_v.is_finite()
        || event.nominal_rail_voltage_v <= 0.0
        || !event.voltage_tolerance_fraction.is_finite()
        || event.voltage_tolerance_fraction <= 0.0
        || event.voltage_tolerance_fraction >= 1.0
        || !event.target_post_step_load_current_a.is_finite()
        || event.target_post_step_load_current_a < 0.0
        || !event.load_current_absolute_tolerance_a.is_finite()
        || event.load_current_absolute_tolerance_a < 0.0
        || event.settling_dwell_ns == 0
        || event.maximum_allowed_sample_gap_ns == 0
    {
        return Err(AuditError(
            "transient event limits and operating-point inputs are invalid".to_string(),
        ));
    }
    if !(event.baseline_window_start_ns < event.step_start_ns
        && event.step_start_ns < event.steady_window_start_ns
        && event.steady_window_start_ns < event.evaluation_end_ns)
    {
        return Err(AuditError(
            "transient event boundaries must be strictly ordered".to_string(),
        ));
    }

    for (index, sample) in event.samples.iter().enumerate() {
        if sample.quality != TransientSampleQuality::Valid {
            return Err(AuditError(format!(
                "transient sample {index}: quality {:?} is not eligible",
                sample.quality
            )));
        }
        if !sample.load_current_a.is_finite()
            || sample.load_current_a < 0.0
            || !sample.rail_voltage_v.is_finite()
            || sample.rail_voltage_v <= 0.0
        {
            return Err(AuditError(format!(
                "transient sample {index}: current or voltage is invalid"
            )));
        }
        if index > 0 {
            let previous = &event.samples[index - 1];
            if sample.timestamp_ns <= previous.timestamp_ns {
                return Err(AuditError(format!(
                    "transient sample {index}: timestamps must be strictly increasing"
                )));
            }
        }
    }

    let first = event.samples.first().ok_or_else(|| {
        AuditError("transient trace has no first sample".to_string())
    })?;
    let last = event.samples.last().ok_or_else(|| {
        AuditError("transient trace has no last sample".to_string())
    })?;
    if first.timestamp_ns != event.baseline_window_start_ns
        || last.timestamp_ns != event.evaluation_end_ns
        || !event.samples.iter().any(|s| s.timestamp_ns == event.step_start_ns)
        || !event.samples.iter().any(|s| s.timestamp_ns == event.steady_window_start_ns)
    {
        return Err(AuditError(
            "transient event boundaries must match exact captured sample timestamps"
                .to_string(),
        ));
    }

    let mut maximum_sample_gap_ns = 0_u64;
    for pair in event.samples.windows(2) {
        let gap = pair[1].timestamp_ns - pair[0].timestamp_ns;
        maximum_sample_gap_ns = maximum_sample_gap_ns.max(gap);
        if gap > event.maximum_allowed_sample_gap_ns {
            return Err(AuditError(format!(
                "transient sample gap {gap} ns exceeds declared maximum {} ns",
                event.maximum_allowed_sample_gap_ns
            )));
        }
    }

    let baseline: Vec<&RailTransientSample> = event
        .samples
        .iter()
        .filter(|s| s.timestamp_ns >= event.baseline_window_start_ns
            && s.timestamp_ns < event.step_start_ns)
        .collect();
    let steady: Vec<&RailTransientSample> = event
        .samples
        .iter()
        .filter(|s| s.timestamp_ns >= event.steady_window_start_ns
            && s.timestamp_ns <= event.evaluation_end_ns)
        .collect();
    let response: Vec<&RailTransientSample> = event
        .samples
        .iter()
        .filter(|s| s.timestamp_ns >= event.step_start_ns
            && s.timestamp_ns <= event.evaluation_end_ns)
        .collect();

    if baseline.len() < 2 || steady.len() < 2 || response.len() < 2 {
        return Err(AuditError(
            "baseline, steady-state and response windows each require at least two samples"
                .to_string(),
        ));
    }

    let pre_step_current_a =
        baseline.iter().map(|s| s.load_current_a).sum::<f64>() / baseline.len() as f64;
    let post_step_current_a =
        steady.iter().map(|s| s.load_current_a).sum::<f64>() / steady.len() as f64;
    let measured_load_step_delta_a = post_step_current_a - pre_step_current_a;
    let target_load_step_delta_a =
        event.target_post_step_load_current_a - pre_step_current_a;
    let post_step_current_error_a =
        post_step_current_a - event.target_post_step_load_current_a;

    let mut minimum_voltage_v = f64::INFINITY;
    let mut maximum_voltage_v = f64::NEG_INFINITY;
    let mut maximum_upward_slew = 0.0_f64;
    let mut maximum_downward_slew = 0.0_f64;

    for sample in &response {
        minimum_voltage_v = minimum_voltage_v.min(sample.rail_voltage_v);
        maximum_voltage_v = maximum_voltage_v.max(sample.rail_voltage_v);
    }
    for pair in response.windows(2) {
        let duration_us =
            (pair[1].timestamp_ns - pair[0].timestamp_ns) as f64 / 1_000.0;
        let delta_current = pair[1].load_current_a - pair[0].load_current_a;
        let slew = delta_current / duration_us;
        if !slew.is_finite() {
            return Err(AuditError(
                "derived load-current slew is non-finite".to_string(),
            ));
        }
        maximum_upward_slew = maximum_upward_slew.max(slew.max(0.0));
        maximum_downward_slew = maximum_downward_slew.max((-slew).max(0.0));
    }

    let maximum_undershoot_v =
        (event.nominal_rail_voltage_v - minimum_voltage_v).max(0.0);
    let maximum_overshoot_v =
        (maximum_voltage_v - event.nominal_rail_voltage_v).max(0.0);
    let maximum_undershoot_percent =
        100.0 * maximum_undershoot_v / event.nominal_rail_voltage_v;
    let maximum_overshoot_percent =
        100.0 * maximum_overshoot_v / event.nominal_rail_voltage_v;
    let lower_voltage_limit =
        event.nominal_rail_voltage_v * (1.0 - event.voltage_tolerance_fraction);
    let upper_voltage_limit =
        event.nominal_rail_voltage_v * (1.0 + event.voltage_tolerance_fraction);

    let mut settling_time_ns = None;
    for (index, candidate) in response.iter().enumerate() {
        if candidate.rail_voltage_v < lower_voltage_limit
            || candidate.rail_voltage_v > upper_voltage_limit
        {
            continue;
        }
        let dwell_elapsed_ns =
            event.evaluation_end_ns.saturating_sub(candidate.timestamp_ns);
        if dwell_elapsed_ns < event.settling_dwell_ns {
            continue;
        }
        let remains_in_band = response[index..].iter().all(|sample| {
            sample.rail_voltage_v >= lower_voltage_limit
                && sample.rail_voltage_v <= upper_voltage_limit
        });
        if remains_in_band {
            settling_time_ns = Some(candidate.timestamp_ns - event.step_start_ns);
            break;
        }
    }

    let maximum_rail_deviation_percent =
        100.0 * (event.nominal_rail_voltage_v - minimum_voltage_v)
            .abs()
            .max((maximum_voltage_v - event.nominal_rail_voltage_v).abs())
            / event.nominal_rail_voltage_v;
    let derived = [
        pre_step_current_a,
        post_step_current_a,
        measured_load_step_delta_a,
        target_load_step_delta_a,
        post_step_current_error_a,
        maximum_upward_slew,
        maximum_downward_slew,
        minimum_voltage_v,
        maximum_voltage_v,
        maximum_undershoot_v,
        maximum_overshoot_v,
        maximum_undershoot_percent,
        maximum_overshoot_percent,
        maximum_rail_deviation_percent,
    ];
    if derived.iter().any(|value| !value.is_finite())
        || pre_step_current_a < 0.0
        || post_step_current_a < 0.0
        || maximum_upward_slew < 0.0
        || maximum_downward_slew < 0.0
    {
        return Err(AuditError(
            "derived transient metric is non-finite or outside its physical domain".to_string(),
        ));
    }

    Ok(RailTransientAudit {
        event_id: event.event_id,
        sample_count: event.samples.len(),
        measured_pre_step_load_current_a: pre_step_current_a,
        measured_post_step_load_current_a: post_step_current_a,
        measured_load_step_delta_a,
        target_load_step_delta_a,
        post_step_current_error_a,
        post_step_current_within_tolerance: post_step_current_error_a.abs()
            <= event.load_current_absolute_tolerance_a,
        maximum_upward_load_slew_a_per_us: maximum_upward_slew,
        maximum_downward_load_slew_a_per_us: maximum_downward_slew,
        minimum_post_step_rail_voltage_v: minimum_voltage_v,
        maximum_post_step_rail_voltage_v: maximum_voltage_v,
        maximum_undershoot_v,
        maximum_overshoot_v,
        maximum_undershoot_percent,
        maximum_overshoot_percent,
        rail_voltage_within_tolerance_at_end: last.rail_voltage_v >= lower_voltage_limit
            && last.rail_voltage_v <= upper_voltage_limit,
        settling_time_ns,
        declared_settling_dwell_ns: event.settling_dwell_ns,
        maximum_sample_gap_ns,
        maximum_rail_deviation_percent,
        clock_synchronization_within_budget: event.actual_clock_sync_uncertainty_ns
            <= event.maximum_allowed_clock_sync_uncertainty_ns,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        audit_rail_transient, RailTransientEvent, RailTransientSample,
        TransientSampleQuality,
    };

    fn make_samples(voltages: &[f64], currents: &[f64]) -> Vec<RailTransientSample> {
        assert_eq!(voltages.len(), currents.len());
        voltages
            .iter()
            .zip(currents.iter())
            .enumerate()
            .map(|(index, (voltage, current))| RailTransientSample {
                timestamp_ns: index as u64 * 100,
                load_current_a: *current,
                rail_voltage_v: *voltage,
                quality: TransientSampleQuality::Valid,
            })
            .collect()
    }

    fn event(samples: Vec<RailTransientSample>) -> RailTransientEvent {
        RailTransientEvent {
            event_id: "vrm-step-40A-to-360A".to_string(),
            samples,
            baseline_window_start_ns: 0,
            step_start_ns: 1000,
            steady_window_start_ns: 1600,
            evaluation_end_ns: 2000,
            nominal_rail_voltage_v: 0.9,
            voltage_tolerance_fraction: 0.02,
            target_post_step_load_current_a: 360.0,
            load_current_absolute_tolerance_a: 2.0,
            settling_dwell_ns: 300,
            maximum_allowed_sample_gap_ns: 100,
            actual_clock_sync_uncertainty_ns: 5,
            maximum_allowed_clock_sync_uncertainty_ns: 10,
        }
    }

    fn reference_samples() -> Vec<RailTransientSample> {
        let mut volts = vec![0.9; 21];
        let mut amps = vec![40.0; 21];
        volts[11] = 0.87;
        volts[12] = 0.88;
        volts[13] = 0.895;
        volts[14] = 0.9;
        for (index, current) in amps.iter_mut().enumerate().skip(11) {
            *current = 40.0 + ((index - 10).min(4) as f64) * 80.0;
        }
        make_samples(&volts, &amps)
    }

    #[test]
    fn measures_40a_to_360a_step_at_800a_per_microsecond() {
        let report = audit_rail_transient(event(reference_samples())).unwrap();

        assert!((report.measured_pre_step_load_current_a - 40.0).abs() < 1.0e-9);
        assert!((report.measured_post_step_load_current_a - 360.0).abs() < 1.0e-9);
        assert!((report.measured_load_step_delta_a - 320.0).abs() < 1.0e-9);
        assert!((report.maximum_upward_load_slew_a_per_us - 800.0).abs() < 1.0e-9);
        assert!((report.maximum_undershoot_v - 0.03).abs() < 1.0e-9);
        assert_eq!(report.settling_time_ns, Some(300));
        assert!(report.post_step_current_within_tolerance);
    }

    #[test]
    fn settling_requires_remaining_inside_band_through_evaluation_end() {
        let mut samples = reference_samples();
        samples[14].rail_voltage_v = 0.9;
        samples[15].rail_voltage_v = 0.87;
        samples[16].rail_voltage_v = 0.895;
        samples[17].rail_voltage_v = 0.9;
        samples[18].rail_voltage_v = 0.9;
        samples[19].rail_voltage_v = 0.9;
        samples[20].rail_voltage_v = 0.9;

        let report = audit_rail_transient(event(samples)).unwrap();

        assert_eq!(report.settling_time_ns, Some(700));
    }

    #[test]
    fn reports_no_settling_when_dwell_is_not_observed() {
        let mut input = event(reference_samples());
        input.settling_dwell_ns = 2000;

        let report = audit_rail_transient(input).unwrap();

        assert_eq!(report.settling_time_ns, None);
        assert!(!report.rail_voltage_within_tolerance_at_end || report.minimum_post_step_rail_voltage_v < 0.88);
    }

    #[test]
    fn reports_downward_slew_and_current_target_error() {
        let mut samples = reference_samples();
        for (index, sample) in samples.iter_mut().enumerate().skip(16) {
            sample.load_current_a = if index <= 18 {
                360.0 - ((index - 15) as f64) * 100.0
            } else {
                60.0
            };
        }
        let mut input = event(samples);
        input.target_post_step_load_current_a = 60.0;
        input.steady_window_start_ns = 1900;

        let report = audit_rail_transient(input).unwrap();

        assert!(report.maximum_downward_load_slew_a_per_us >= 1000.0);
        assert!(report.post_step_current_within_tolerance);
    }

    #[test]
    fn rejects_bad_timestamps_gaps_and_non_exact_event_boundaries() {
        let mut duplicate = reference_samples();
        duplicate[5].timestamp_ns = duplicate[4].timestamp_ns;
        let mut gap = reference_samples();
        gap[5].timestamp_ns = 1000;
        let mut no_boundary = event(reference_samples());
        no_boundary.step_start_ns = 1050;

        assert!(audit_rail_transient(event(duplicate)).is_err());
        assert!(audit_rail_transient(event(gap)).is_err());
        assert!(audit_rail_transient(no_boundary).is_err());
    }

    #[test]
    fn rejects_invalid_quality_and_nonfinite_samples() {
        let mut quality = reference_samples();
        quality[12].quality = TransientSampleQuality::Saturated;
        let mut invalid = reference_samples();
        invalid[13].rail_voltage_v = f64::NAN;

        assert!(audit_rail_transient(event(quality)).is_err());
        assert!(audit_rail_transient(event(invalid)).is_err());
    }

    #[test]
    fn reports_clock_and_voltage_limit_status_without_clipping() {
        let mut input = event(reference_samples());
        input.actual_clock_sync_uncertainty_ns = 50;
        input.samples[20].rail_voltage_v = 0.93;

        let report = audit_rail_transient(input).unwrap();

        assert!(!report.clock_synchronization_within_budget);
        assert!(!report.rail_voltage_within_tolerance_at_end);
        assert!(report.maximum_overshoot_v > 0.0);
    }
}
