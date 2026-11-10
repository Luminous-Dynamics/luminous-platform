use crate::AuditError;

/// Sample quality is deliberately strict: only VALID samples can enter a
/// matched benchmark. Missing or questionable data are never interpolated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSampleQuality {
    Valid,
    Saturated,
    Dropped,
    ClockUncertain,
    MissingCalibration,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputePowerSample {
    pub timestamp_ns: u64,
    /// Net active power at the declared PCC; import is positive and export may
    /// be negative. This is not apparent power or a DC-bus power measurement.
    pub pcc_active_power_w: f64,
    /// Delivered useful work per second in the declared work_unit.
    pub useful_work_rate_per_s: f64,
    pub quality: PowerSampleQuality,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputePowerTrace {
    pub trace_id: String,
    pub workload_id: String,
    pub boundary_id: String,
    pub work_unit: String,
    pub nominal_sample_interval_ns: u64,
    pub maximum_allowed_sample_gap_ns: u64,
    pub clock_sync_uncertainty_ns: u64,
    pub maximum_allowed_clock_sync_uncertainty_ns: u64,
    /// Stored energy inside the benchmark boundary at the first sample.
    pub initial_stored_energy_j: f64,
    /// Stored energy inside the benchmark boundary at the final sample.
    pub final_stored_energy_j: f64,
    /// A common PCC import target for baseline/candidate comparison.
    pub pcc_reference_power_w: f64,
    pub samples: Vec<ComputePowerSample>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputePowerAudit {
    pub trace_id: String,
    pub workload_id: String,
    pub boundary_id: String,
    pub work_unit: String,
    pub sample_count: usize,
    pub duration_s: f64,
    pub maximum_sample_gap_ns: u64,
    pub pcc_energy_j: f64,
    /// final stored energy minus initial stored energy; positive means charging.
    pub stored_energy_change_j: f64,
    /// PCC energy corrected for the change in stored energy.
    pub effective_input_energy_j: f64,
    pub useful_work_units: f64,
    pub effective_input_energy_per_work_unit_j: f64,
    pub time_weighted_mean_pcc_power_w: f64,
    /// RMS deviation around the trace's own time-weighted mean.
    pub pcc_rms_fluctuation_w: f64,
    /// RMS deviation around the declared common PCC reference power.
    pub pcc_rms_tracking_error_w: f64,
    pub peak_absolute_pcc_deviation_w: f64,
    pub maximum_upward_ramp_w_per_s: f64,
    pub maximum_downward_ramp_w_per_s: f64,
}

impl ComputePowerAudit {
    pub const fn claim_ceiling(&self) -> &'static str {
        "TRACE_ACCOUNTING_AND_TIME_DOMAIN_COMPARISON_ONLY_NOT_CONTROL_NOT_GRID_OR_HARDWARE_QUALIFICATION"
    }
}

/// Audit a time-aligned PCC active-power and useful-work trace.
///
/// Linear interpolation is used only for integration between valid samples;
/// no samples are synthesized, repaired or silently time-aligned. Irregular
/// intervals are supported as long as each gap is within the declared bound.
pub fn audit_compute_power_trace(
    trace: ComputePowerTrace,
) -> Result<ComputePowerAudit, AuditError> {
    if trace.trace_id.trim().is_empty()
        || trace.workload_id.trim().is_empty()
        || trace.boundary_id.trim().is_empty()
        || trace.work_unit.trim().is_empty()
    {
        return Err(AuditError(
            "trace, workload, boundary and work-unit identities must not be blank".to_string(),
        ));
    }
    if trace.samples.len() < 2
        || trace.nominal_sample_interval_ns == 0
        || trace.maximum_allowed_sample_gap_ns == 0
    {
        return Err(AuditError(
            "trace requires at least two samples and positive interval/gap limits".to_string(),
        ));
    }
    if trace.clock_sync_uncertainty_ns > trace.maximum_allowed_clock_sync_uncertainty_ns {
        return Err(AuditError(
            "clock synchronization uncertainty exceeds the declared budget".to_string(),
        ));
    }
    let metadata_values = [
        trace.initial_stored_energy_j,
        trace.final_stored_energy_j,
        trace.pcc_reference_power_w,
    ];
    if metadata_values.iter().any(|value| !value.is_finite())
        || trace.initial_stored_energy_j < 0.0
        || trace.final_stored_energy_j < 0.0
    {
        return Err(AuditError(
            "stored-energy metadata and PCC reference must be finite; stored energy must be non-negative"
                .to_string(),
        ));
    }

    let first = &trace.samples[0];
    if first.quality != PowerSampleQuality::Valid {
        return Err(AuditError("sample 0 is not VALID".to_string()));
    }
    if !first.pcc_active_power_w.is_finite()
        || !first.useful_work_rate_per_s.is_finite()
        || first.useful_work_rate_per_s < 0.0
    {
        return Err(AuditError(
            "sample 0 contains non-finite power or invalid useful-work rate".to_string(),
        ));
    }

    let mut pcc_energy_j = 0.0_f64;
    let mut useful_work_units = 0.0_f64;
    let mut pcc_square_integral = 0.0_f64;
    let mut target_error_square_integral = 0.0_f64;
    let mut maximum_sample_gap_ns = 0_u64;
    let mut maximum_upward_ramp_w_per_s = 0.0_f64;
    let mut maximum_downward_ramp_w_per_s = 0.0_f64;
    let mut peak_absolute_pcc_deviation_w = 0.0_f64;

    for sample in &trace.samples {
        if sample.quality != PowerSampleQuality::Valid {
            return Err(AuditError(format!(
                "sample quality {:?} is not eligible for a benchmark",
                sample.quality
            )));
        }
        if !sample.pcc_active_power_w.is_finite()
            || !sample.useful_work_rate_per_s.is_finite()
            || sample.useful_work_rate_per_s < 0.0
        {
            return Err(AuditError(
                "sample contains non-finite power or invalid useful-work rate".to_string(),
            ));
        }
        peak_absolute_pcc_deviation_w = peak_absolute_pcc_deviation_w.max(
            (sample.pcc_active_power_w - trace.pcc_reference_power_w).abs(),
        );
    }

    for pair in trace.samples.windows(2) {
        let previous = &pair[0];
        let current = &pair[1];
        if current.timestamp_ns <= previous.timestamp_ns {
            return Err(AuditError(
                "sample timestamps must be strictly increasing".to_string(),
            ));
        }
        let gap_ns = current.timestamp_ns - previous.timestamp_ns;
        if gap_ns > trace.maximum_allowed_sample_gap_ns {
            return Err(AuditError(format!(
                "sample gap {gap_ns} ns exceeds declared maximum {} ns",
                trace.maximum_allowed_sample_gap_ns
            )));
        }
        maximum_sample_gap_ns = maximum_sample_gap_ns.max(gap_ns);
        let duration_s = gap_ns as f64 / 1_000_000_000.0;

        let p0 = previous.pcc_active_power_w;
        let p1 = current.pcc_active_power_w;
        let w0 = previous.useful_work_rate_per_s;
        let w1 = current.useful_work_rate_per_s;
        let e0 = p0 - trace.pcc_reference_power_w;
        let e1 = p1 - trace.pcc_reference_power_w;

        pcc_energy_j += 0.5 * (p0 + p1) * duration_s;
        useful_work_units += 0.5 * (w0 + w1) * duration_s;

        // Exact time integral of the square of a linearly interpolated value.
        pcc_square_integral +=
            (p0 * p0 + p0 * p1 + p1 * p1) * duration_s / 3.0;
        target_error_square_integral +=
            (e0 * e0 + e0 * e1 + e1 * e1) * duration_s / 3.0;

        let ramp_w_per_s = (p1 - p0) / duration_s;
        if ramp_w_per_s >= 0.0 {
            maximum_upward_ramp_w_per_s =
                maximum_upward_ramp_w_per_s.max(ramp_w_per_s);
        } else {
            maximum_downward_ramp_w_per_s =
                maximum_downward_ramp_w_per_s.max(-ramp_w_per_s);
        }
    }

    let first_timestamp = trace.samples[0].timestamp_ns;
    let last_timestamp = trace.samples[trace.samples.len() - 1].timestamp_ns;
    let duration_s = (last_timestamp - first_timestamp) as f64 / 1_000_000_000.0;
    let stored_energy_change_j =
        trace.final_stored_energy_j - trace.initial_stored_energy_j;
    let effective_input_energy_j = pcc_energy_j - stored_energy_change_j;

    let derived = [
        duration_s,
        pcc_energy_j,
        useful_work_units,
        pcc_square_integral,
        target_error_square_integral,
        stored_energy_change_j,
        effective_input_energy_j,
        maximum_upward_ramp_w_per_s,
        maximum_downward_ramp_w_per_s,
    ];
    if derived.iter().any(|value| !value.is_finite())
        || duration_s <= 0.0
        || pcc_energy_j <= 0.0
        || useful_work_units <= 0.0
        || effective_input_energy_j <= 0.0
        || pcc_square_integral < 0.0
        || target_error_square_integral < 0.0
    {
        return Err(AuditError(
            "integrated energy, useful work or derived trace metrics are outside their valid domain"
                .to_string(),
        ));
    }

    let mean_pcc_power_w = pcc_energy_j / duration_s;
    let variance = (pcc_square_integral / duration_s - mean_pcc_power_w * mean_pcc_power_w)
        .max(0.0);
    let pcc_rms_fluctuation_w = variance.sqrt();
    let pcc_rms_tracking_error_w = (target_error_square_integral / duration_s).sqrt();
    let effective_input_energy_per_work_unit_j =
        effective_input_energy_j / useful_work_units;

    let report_values = [
        mean_pcc_power_w,
        pcc_rms_fluctuation_w,
        pcc_rms_tracking_error_w,
        peak_absolute_pcc_deviation_w,
        effective_input_energy_per_work_unit_j,
    ];
    if report_values.iter().any(|value| !value.is_finite() || *value < 0.0) {
        return Err(AuditError(
            "derived benchmark report contains a non-finite or negative metric".to_string(),
        ));
    }

    Ok(ComputePowerAudit {
        trace_id: trace.trace_id,
        workload_id: trace.workload_id,
        boundary_id: trace.boundary_id,
        work_unit: trace.work_unit,
        sample_count: trace.samples.len(),
        duration_s,
        maximum_sample_gap_ns,
        pcc_energy_j,
        stored_energy_change_j,
        effective_input_energy_j,
        useful_work_units,
        effective_input_energy_per_work_unit_j,
        time_weighted_mean_pcc_power_w: mean_pcc_power_w,
        pcc_rms_fluctuation_w,
        pcc_rms_tracking_error_w,
        peak_absolute_pcc_deviation_w,
        maximum_upward_ramp_w_per_s,
        maximum_downward_ramp_w_per_s,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputePowerComparison {
    pub baseline: ComputePowerAudit,
    pub candidate: ComputePowerAudit,
    pub useful_work_retention_fraction: f64,
    pub candidate_meets_work_budget: bool,
    /// Positive means the candidate uses less storage-adjusted input energy
    /// per useful-work unit. Negative means it uses more.
    pub energy_per_work_improvement_fraction: f64,
    /// Positive means the candidate has lower PCC RMS deviation from its
    /// declared common reference. None means the baseline metric was zero.
    pub pcc_tracking_error_reduction_fraction: Option<f64>,
    pub pcc_variability_reduction_fraction: Option<f64>,
    pub peak_absolute_deviation_reduction_fraction: Option<f64>,
    pub maximum_upward_ramp_reduction_fraction: Option<f64>,
    pub maximum_downward_ramp_reduction_fraction: Option<f64>,
}

fn reduction_fraction(baseline: f64, candidate: f64) -> Option<f64> {
    if baseline > 0.0 {
        Some(1.0 - candidate / baseline)
    } else {
        None
    }
}

/// Compare baseline and candidate only when their workload, boundary, common
/// target, time window and every timestamp are identical. This deliberately
/// rejects resampling and interpolation across architectures.
pub fn compare_compute_power_traces(
    baseline: ComputePowerTrace,
    candidate: ComputePowerTrace,
    maximum_work_loss_fraction: f64,
) -> Result<ComputePowerComparison, AuditError> {
    if !maximum_work_loss_fraction.is_finite()
        || maximum_work_loss_fraction < 0.0
        || maximum_work_loss_fraction >= 1.0
    {
        return Err(AuditError(
            "maximum work-loss fraction must be finite and in [0, 1)".to_string(),
        ));
    }
    if baseline.workload_id != candidate.workload_id
        || baseline.boundary_id != candidate.boundary_id
        || baseline.work_unit != candidate.work_unit
        || baseline.pcc_reference_power_w.to_bits() != candidate.pcc_reference_power_w.to_bits()
        || baseline.initial_stored_energy_j.to_bits() != candidate.initial_stored_energy_j.to_bits()
    {
        return Err(AuditError(
            "baseline and candidate must share workload, boundary, work unit, PCC target and initial stored energy"
                .to_string(),
        ));
    }
    if baseline.samples.len() != candidate.samples.len()
        || baseline.samples.iter().zip(candidate.samples.iter()).any(|(left, right)| {
            left.timestamp_ns != right.timestamp_ns
        })
    {
        return Err(AuditError(
            "baseline and candidate must have identical sample counts and exact timestamps"
                .to_string(),
        ));
    }

    let baseline_report = audit_compute_power_trace(baseline)?;
    let candidate_report = audit_compute_power_trace(candidate)?;
    if baseline_report.duration_s != candidate_report.duration_s {
        return Err(AuditError(
            "baseline and candidate durations must be identical".to_string(),
        ));
    }

    let useful_work_retention_fraction =
        candidate_report.useful_work_units / baseline_report.useful_work_units;
    if !useful_work_retention_fraction.is_finite() {
        return Err(AuditError(
            "useful-work retention is non-finite".to_string(),
        ));
    }
    let candidate_meets_work_budget =
        useful_work_retention_fraction + 1.0e-12 >= 1.0 - maximum_work_loss_fraction;

    let energy_per_work_improvement_fraction = 1.0
        - candidate_report.effective_input_energy_per_work_unit_j
            / baseline_report.effective_input_energy_per_work_unit_j;

    Ok(ComputePowerComparison {
        pcc_tracking_error_reduction_fraction: reduction_fraction(
            baseline_report.pcc_rms_tracking_error_w,
            candidate_report.pcc_rms_tracking_error_w,
        ),
        pcc_variability_reduction_fraction: reduction_fraction(
            baseline_report.pcc_rms_fluctuation_w,
            candidate_report.pcc_rms_fluctuation_w,
        ),
        peak_absolute_deviation_reduction_fraction: reduction_fraction(
            baseline_report.peak_absolute_pcc_deviation_w,
            candidate_report.peak_absolute_pcc_deviation_w,
        ),
        maximum_upward_ramp_reduction_fraction: reduction_fraction(
            baseline_report.maximum_upward_ramp_w_per_s,
            candidate_report.maximum_upward_ramp_w_per_s,
        ),
        maximum_downward_ramp_reduction_fraction: reduction_fraction(
            baseline_report.maximum_downward_ramp_w_per_s,
            candidate_report.maximum_downward_ramp_w_per_s,
        ),
        baseline: baseline_report,
        candidate: candidate_report,
        useful_work_retention_fraction,
        candidate_meets_work_budget,
        energy_per_work_improvement_fraction,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct SitePowerSample {
    pub timestamp_ns: u64,
    pub pcc_active_power_w: f64,
    pub quality: PowerSampleQuality,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SitePowerTrace {
    pub site_id: String,
    pub maximum_allowed_sample_gap_ns: u64,
    pub clock_sync_uncertainty_ns: u64,
    pub maximum_allowed_clock_sync_uncertainty_ns: u64,
    pub samples: Vec<SitePowerSample>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SiteVariability {
    pub site_id: String,
    pub time_weighted_mean_power_w: f64,
    pub rms_fluctuation_w: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiSiteCorrelationAudit {
    pub site_count: usize,
    pub sample_count: usize,
    pub duration_s: f64,
    pub sites: Vec<SiteVariability>,
    pub mean_pairwise_correlation: Option<f64>,
    pub minimum_pairwise_correlation: Option<f64>,
    pub maximum_pairwise_correlation: Option<f64>,
    /// RMS of the sum of site deviations using the observed covariance.
    pub aggregate_rms_fluctuation_w: f64,
    /// Zero-covariance reference: sqrt(sum of each site's variance).
    pub independent_reference_rms_fluctuation_w: f64,
    /// Actual aggregate RMS divided by the zero-covariance reference.
    pub variability_amplification_ratio: Option<f64>,
}

fn integrate_square_of_linear(x0: f64, x1: f64, duration_s: f64) -> f64 {
    (x0 * x0 + x0 * x1 + x1 * x1) * duration_s / 3.0
}

fn integrate_product_of_linear(
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    duration_s: f64,
) -> f64 {
    (2.0 * x0 * y0 + x0 * y1 + x1 * y0 + 2.0 * x1 * y1) * duration_s / 6.0
}

/// Calculate synchronous cross-site covariance without assuming independent
/// loads. All sites must already share exactly the same timestamps and valid
/// sample quality; the function does not resample or phase-shift traces.
pub fn audit_multisite_load_correlation(
    traces: Vec<SitePowerTrace>,
) -> Result<MultiSiteCorrelationAudit, AuditError> {
    if traces.len() < 2 {
        return Err(AuditError(
            "multi-site correlation requires at least two site traces".to_string(),
        ));
    }
    let sample_count = traces[0].samples.len();
    if sample_count < 2 {
        return Err(AuditError(
            "each site trace requires at least two samples".to_string(),
        ));
    }
    let reference_timestamps: Vec<u64> = traces[0]
        .samples
        .iter()
        .map(|sample| sample.timestamp_ns)
        .collect();
    let mut seen_ids = std::collections::HashSet::new();
    let mut means = Vec::with_capacity(traces.len());
    let mut variances = Vec::with_capacity(traces.len());

    for trace in &traces {
        if trace.site_id.trim().is_empty() || !seen_ids.insert(trace.site_id.as_str()) {
            return Err(AuditError(
                "site identities must be non-blank and unique".to_string(),
            ));
        }
        if trace.samples.len() != sample_count
            || trace.samples.iter().zip(reference_timestamps.iter()).any(
                |(sample, timestamp)| sample.timestamp_ns != *timestamp,
            )
        {
            return Err(AuditError(
                "all site traces must have identical sample counts and exact timestamps"
                    .to_string(),
            ));
        }
        if trace.maximum_allowed_sample_gap_ns == 0
            || trace.clock_sync_uncertainty_ns > trace.maximum_allowed_clock_sync_uncertainty_ns
        {
            return Err(AuditError(
                "site sample-gap limit or clock synchronization budget is invalid".to_string(),
            ));
        }
        if trace.samples.iter().any(|sample| {
            sample.quality != PowerSampleQuality::Valid
                || !sample.pcc_active_power_w.is_finite()
        }) {
            return Err(AuditError(
                "all site samples must be finite and VALID".to_string(),
            ));
        }
        for pair in trace.samples.windows(2) {
            if pair[1].timestamp_ns <= pair[0].timestamp_ns {
                return Err(AuditError(
                    "site timestamps must be strictly increasing".to_string(),
                ));
            }
            if pair[1].timestamp_ns - pair[0].timestamp_ns
                > trace.maximum_allowed_sample_gap_ns
            {
                return Err(AuditError(
                    "site sample gap exceeds its declared maximum".to_string(),
                ));
            }
        }

        let first = trace.samples[0].timestamp_ns;
        let last = trace.samples[sample_count - 1].timestamp_ns;
        let duration_s = (last - first) as f64 / 1_000_000_000.0;
        if !duration_s.is_finite() || duration_s <= 0.0 {
            return Err(AuditError("site trace duration is invalid".to_string()));
        }
        let mut energy_j = 0.0_f64;
        let mut square_integral = 0.0_f64;
        for pair in trace.samples.windows(2) {
            let dt = (pair[1].timestamp_ns - pair[0].timestamp_ns) as f64 / 1_000_000_000.0;
            let p0 = pair[0].pcc_active_power_w;
            let p1 = pair[1].pcc_active_power_w;
            energy_j += 0.5 * (p0 + p1) * dt;
            square_integral += integrate_square_of_linear(p0, p1, dt);
        }
        let mean = energy_j / duration_s;
        let variance = (square_integral / duration_s - mean * mean).max(0.0);
        if !mean.is_finite() || !variance.is_finite() {
            return Err(AuditError(
                "site mean or variance is non-finite".to_string(),
            ));
        }
        means.push(mean);
        variances.push(variance);
    }

    let first_timestamp = reference_timestamps[0];
    let last_timestamp = reference_timestamps[sample_count - 1];
    let duration_s = (last_timestamp - first_timestamp) as f64 / 1_000_000_000.0;
    let mut covariance_sum = 0.0_f64;
    let mut correlation_sum = 0.0_f64;
    let mut pair_count = 0_usize;
    let mut minimum_correlation = f64::INFINITY;
    let mut maximum_correlation = f64::NEG_INFINITY;

    for left in 0..traces.len() {
        for right in (left + 1)..traces.len() {
            let mut product_integral = 0.0_f64;
            for index in 0..(sample_count - 1) {
                let a0 = traces[left].samples[index].pcc_active_power_w;
                let a1 = traces[left].samples[index + 1].pcc_active_power_w;
                let b0 = traces[right].samples[index].pcc_active_power_w;
                let b1 = traces[right].samples[index + 1].pcc_active_power_w;
                let dt = (reference_timestamps[index + 1] - reference_timestamps[index])
                    as f64
                    / 1_000_000_000.0;
                product_integral += integrate_product_of_linear(a0, a1, b0, b1, dt);
            }
            let covariance = product_integral / duration_s - means[left] * means[right];
            covariance_sum += covariance;
            if variances[left] > 0.0 && variances[right] > 0.0 {
                let correlation =
                    covariance / (variances[left] * variances[right]).sqrt();
                correlation_sum += correlation;
                pair_count += 1;
                minimum_correlation = minimum_correlation.min(correlation);
                maximum_correlation = maximum_correlation.max(correlation);
            }
        }
    }

    let independent_variance: f64 = variances.iter().sum();
    let aggregate_variance = independent_variance + 2.0 * covariance_sum;
    let variance_tolerance = 1.0e-10 * independent_variance.max(1.0);
    if !independent_variance.is_finite()
        || !aggregate_variance.is_finite()
        || aggregate_variance < -variance_tolerance
    {
        return Err(AuditError(
            "aggregate covariance produced a non-finite or physically inconsistent variance"
                .to_string(),
        ));
    }
    let aggregate_rms_fluctuation_w = aggregate_variance.max(0.0).sqrt();
    let independent_reference_rms_fluctuation_w = independent_variance.sqrt();
    let variability_amplification_ratio =
        if independent_reference_rms_fluctuation_w > 0.0 {
            Some(aggregate_rms_fluctuation_w / independent_reference_rms_fluctuation_w)
        } else {
            None
        };
    let sites = traces
        .iter()
        .enumerate()
        .map(|(index, trace)| SiteVariability {
            site_id: trace.site_id.clone(),
            time_weighted_mean_power_w: means[index],
            rms_fluctuation_w: variances[index].sqrt(),
        })
        .collect();

    Ok(MultiSiteCorrelationAudit {
        site_count: traces.len(),
        sample_count,
        duration_s,
        sites,
        mean_pairwise_correlation: if pair_count > 0 {
            Some(correlation_sum / pair_count as f64)
        } else {
            None
        },
        minimum_pairwise_correlation: if pair_count > 0 {
            Some(minimum_correlation)
        } else {
            None
        },
        maximum_pairwise_correlation: if pair_count > 0 {
            Some(maximum_correlation)
        } else {
            None
        },
        aggregate_rms_fluctuation_w,
        independent_reference_rms_fluctuation_w,
        variability_amplification_ratio,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        audit_compute_power_trace, audit_multisite_load_correlation,
        compare_compute_power_traces, ComputePowerSample, ComputePowerTrace,
        PowerSampleQuality, SitePowerSample, SitePowerTrace,
    };

    fn sample(timestamp_ns: u64, pcc_w: f64, work_rate: f64) -> ComputePowerSample {
        ComputePowerSample {
            timestamp_ns,
            pcc_active_power_w: pcc_w,
            useful_work_rate_per_s: work_rate,
            quality: PowerSampleQuality::Valid,
        }
    }

    fn trace(id: &str, powers: &[f64], work_rates: &[f64]) -> ComputePowerTrace {
        let timestamps = [0_u64, 1_000_000_000, 2_000_000_000];
        ComputePowerTrace {
            trace_id: id.to_string(),
            workload_id: "workload-a".to_string(),
            boundary_id: "pcc-a".to_string(),
            work_unit: "normalized-work-unit".to_string(),
            nominal_sample_interval_ns: 1_000_000_000,
            maximum_allowed_sample_gap_ns: 1_000_000_000,
            clock_sync_uncertainty_ns: 0,
            maximum_allowed_clock_sync_uncertainty_ns: 10,
            initial_stored_energy_j: 1000.0,
            final_stored_energy_j: 1000.0,
            pcc_reference_power_w: 1000.0,
            samples: timestamps
                .iter()
                .enumerate()
                .map(|(index, timestamp)| sample(*timestamp, powers[index], work_rates[index]))
                .collect(),
        }
    }

    fn site(id: &str, powers: &[f64]) -> SitePowerTrace {
        let timestamps = [0_u64, 1_000_000_000, 2_000_000_000];
        SitePowerTrace {
            site_id: id.to_string(),
            maximum_allowed_sample_gap_ns: 1_000_000_000,
            clock_sync_uncertainty_ns: 0,
            maximum_allowed_clock_sync_uncertainty_ns: 10,
            samples: timestamps
                .iter()
                .enumerate()
                .map(|(index, timestamp)| SitePowerSample {
                    timestamp_ns: *timestamp,
                    pcc_active_power_w: powers[index],
                    quality: PowerSampleQuality::Valid,
                })
                .collect(),
        }
    }

    #[test]
    fn integrates_irregular_power_and_work_and_corrects_storage_change() {
        let mut t = trace("irregular", &[1000.0, 2000.0, 1000.0], &[10.0, 20.0, 10.0]);
        t.samples = vec![
            sample(0, 1000.0, 10.0),
            sample(1_000_000_000, 2000.0, 20.0),
            sample(3_000_000_000, 1000.0, 10.0),
        ];
        t.maximum_allowed_sample_gap_ns = 2_000_000_000;
        t.initial_stored_energy_j = 1000.0;
        t.final_stored_energy_j = 500.0;

        let report = audit_compute_power_trace(t).unwrap();

        assert!((report.duration_s - 3.0).abs() < 1.0e-12);
        assert!((report.pcc_energy_j - 4500.0).abs() < 1.0e-9);
        assert!((report.useful_work_units - 45.0).abs() < 1.0e-9);
        assert!((report.stored_energy_change_j + 500.0).abs() < 1.0e-9);
        assert!((report.effective_input_energy_j - 5000.0).abs() < 1.0e-9);
        assert!((report.effective_input_energy_per_work_unit_j - (5000.0 / 45.0)).abs() < 1.0e-9);
        assert!((report.pcc_rms_tracking_error_w - (1_000_000.0_f64 / 3.0).sqrt()).abs() < 1.0e-9);
        assert_eq!(report.maximum_sample_gap_ns, 2_000_000_000);
    }

    #[test]
    fn matched_candidate_reduces_pcc_error_without_reducing_useful_work() {
        let baseline = trace("baseline", &[900.0, 1100.0, 900.0], &[10.0, 10.0, 10.0]);
        let candidate = trace("candidate", &[950.0, 1050.0, 950.0], &[10.0, 10.0, 10.0]);

        let report = compare_compute_power_traces(baseline, candidate, 0.005).unwrap();

        assert!(report.candidate_meets_work_budget);
        assert!((report.useful_work_retention_fraction - 1.0).abs() < 1.0e-12);
        assert!(report.pcc_tracking_error_reduction_fraction.unwrap() > 0.49);
        assert!(report.pcc_variability_reduction_fraction.unwrap() > 0.49);
        assert!((report.energy_per_work_improvement_fraction).abs() < 1.0e-12);
    }

    #[test]
    fn smoother_trace_fails_the_declared_work_preservation_gate() {
        let baseline = trace("baseline", &[900.0, 1100.0, 900.0], &[10.0, 10.0, 10.0]);
        let candidate = trace("candidate", &[950.0, 1050.0, 950.0], &[9.0, 9.0, 9.0]);

        let report = compare_compute_power_traces(baseline, candidate, 0.005).unwrap();

        assert!(!report.candidate_meets_work_budget);
        assert!((report.useful_work_retention_fraction - 0.9).abs() < 1.0e-12);
    }

    #[test]
    fn storage_depletion_is_charged_to_energy_per_work() {
        let mut t = trace("depleted", &[1000.0, 1000.0, 1000.0], &[10.0, 10.0, 10.0]);
        t.final_stored_energy_j = 500.0;

        let report = audit_compute_power_trace(t).unwrap();

        assert!((report.pcc_energy_j - 2000.0).abs() < 1.0e-9);
        assert!((report.effective_input_energy_j - 2500.0).abs() < 1.0e-9);
        assert!((report.effective_input_energy_per_work_unit_j - 125.0).abs() < 1.0e-9);
    }

    #[test]
    fn rejects_mismatched_timestamps_clock_uncertainty_gaps_and_quality() {
        let baseline = trace("baseline", &[900.0, 1100.0, 900.0], &[10.0, 10.0, 10.0]);
        let mut candidate = trace("candidate", &[950.0, 1050.0, 950.0], &[10.0, 10.0, 10.0]);
        candidate.samples[1].timestamp_ns += 1;

        assert!(compare_compute_power_traces(baseline, candidate, 0.005).is_err());

        let mut bad_clock = trace("clock", &[1000.0, 1000.0, 1000.0], &[10.0, 10.0, 10.0]);
        bad_clock.clock_sync_uncertainty_ns = 11;
        assert!(audit_compute_power_trace(bad_clock).is_err());

        let mut bad_gap = trace("gap", &[1000.0, 1000.0, 1000.0], &[10.0, 10.0, 10.0]);
        bad_gap.maximum_allowed_sample_gap_ns = 999_999_999;
        assert!(audit_compute_power_trace(bad_gap).is_err());

        let mut bad_quality = trace("quality", &[1000.0, 1000.0, 1000.0], &[10.0, 10.0, 10.0]);
        bad_quality.samples[1].quality = PowerSampleQuality::Dropped;
        assert!(audit_compute_power_trace(bad_quality).is_err());
    }

    #[test]
    fn in_phase_sites_amplify_variability_against_independent_reference() {
        let report = audit_multisite_load_correlation(vec![
            site("site-a", &[0.0, 1.0, 0.0]),
            site("site-b", &[0.0, 1.0, 0.0]),
        ])
        .unwrap();

        assert!((report.mean_pairwise_correlation.unwrap() - 1.0).abs() < 1.0e-12);
        assert!((report.variability_amplification_ratio.unwrap() - 2.0_f64.sqrt()).abs() < 1.0e-12);
        assert!(report.aggregate_rms_fluctuation_w > report.independent_reference_rms_fluctuation_w);
    }

    #[test]
    fn anti_phase_sites_cancel_aggregate_variability() {
        let report = audit_multisite_load_correlation(vec![
            site("site-a", &[0.0, 1.0, 0.0]),
            site("site-b", &[1.0, 0.0, 1.0]),
        ])
        .unwrap();

        assert!((report.mean_pairwise_correlation.unwrap() + 1.0).abs() < 1.0e-12);
        assert!(report.aggregate_rms_fluctuation_w < 1.0e-12);
        assert!(report.variability_amplification_ratio.unwrap() < 1.0e-12);
    }

    #[test]
    fn rejects_unsynchronized_site_timestamps() {
        let mut second = site("site-b", &[0.0, 1.0, 0.0]);
        second.samples[1].timestamp_ns += 1;

        assert!(audit_multisite_load_correlation(vec![
            site("site-a", &[0.0, 1.0, 0.0]),
            second,
        ])
        .is_err());
    }
}