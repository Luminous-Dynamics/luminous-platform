use crate::AuditError;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceEvidenceClass {
    Measured,
    Modelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteSampleQuality {
    Good,
    Missing,
    Saturated,
    ClockUnsynchronized,
    CalibrationInvalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitePowerSample {
    /// Timestamp on the source clock before the declared offset is applied.
    pub timestamp_ns: u64,
    pub state_id: String,
    pub power_mw: u64,
    pub quality: SiteSampleQuality,
    pub source_evidence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitePowerTrace {
    pub site_id: String,
    pub parent_topology_id: String,
    pub region_id: String,
    pub run_id: String,
    pub session_id: String,
    pub dataset_id: String,
    pub dataset_version: String,
    pub dataset_evidence_id: String,
    pub use_authorization_reference: String,
    pub trace_evidence_id: String,
    pub hardware_configuration_fingerprint: String,
    pub measurement_boundary_id: String,
    pub workload_contract_id: String,
    pub orchestration_domain_id: String,
    pub state_mapper_id: String,
    pub state_mapping_evidence_id: String,
    pub source_clock_id: String,
    pub source_time_origin_id: String,
    pub aligned_clock_id: String,
    pub aligned_time_origin_id: String,
    pub clock_alignment_method_id: String,
    pub clock_alignment_evidence_id: String,
    /// Signed offset added to source timestamps to map them to the common clock.
    pub clock_offset_to_common_ns: i64,
    pub clock_alignment_uncertainty_ns: u64,
    pub sample_period_ns: u64,
    pub evidence_class: TraceEvidenceClass,
    pub samples: Vec<SitePowerSample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiSiteAggregationContract {
    pub aggregation_id: String,
    pub parent_topology_id: String,
    pub parent_topology_evidence_id: String,
    pub region_id: String,
    pub common_clock_id: String,
    pub common_time_origin_id: String,
    pub clock_alignment_method_id: String,
    pub sample_period_ns: u64,
    pub maximum_alignment_uncertainty_ns: u64,
    pub minimum_sample_count: usize,
    /// Tail-coexceedance is suppressed until this many aligned observations exist.
    pub minimum_tail_sample_count: usize,
    pub maximum_lag_samples: usize,
}

impl MultiSiteAggregationContract {
    fn validate(&self) -> Result<(), AuditError> {
        for (value, field) in [
            (self.aggregation_id.as_str(), "aggregation_id"),
            (self.parent_topology_id.as_str(), "parent_topology_id"),
            (self.parent_topology_evidence_id.as_str(), "parent_topology_evidence_id"),
            (self.region_id.as_str(), "region_id"),
            (self.common_clock_id.as_str(), "common_clock_id"),
            (self.common_time_origin_id.as_str(), "common_time_origin_id"),
            (self.clock_alignment_method_id.as_str(), "clock_alignment_method_id"),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!(
                    "multi-site aggregation contract {field} must not be blank"
                )));
            }
        }
        if self.sample_period_ns == 0 || self.minimum_sample_count < 3 {
            return Err(AuditError(
                "sample period must be positive and minimum sample count must be at least three"
                    .to_string(),
            ));
        }
        if self.minimum_tail_sample_count < 100 {
            return Err(AuditError(
                "tail co-exceedance support threshold must be at least 100 aligned samples"
                    .to_string(),
            ));
        }
        if self.maximum_lag_samples >= self.minimum_sample_count.saturating_sub(2) {
            return Err(AuditError(
                "maximum lag must leave at least three paired samples for correlation"
                    .to_string(),
            ));
        }
        if self.maximum_lag_samples > i32::MAX as usize
            || self.sample_period_ns > i64::MAX as u64
            || (self.maximum_lag_samples as u128)
                .checked_mul(self.sample_period_ns as u128)
                .is_none_or(|lag_ns| lag_ns > i64::MAX as u128)
        {
            return Err(AuditError(
                "maximum lag duration does not fit the signed nanosecond representation".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SiteDependenceMetric {
    pub first_site_id: String,
    pub second_site_id: String,
    pub contemporaneous_pearson: Option<f64>,
    pub contemporaneous_covariance_mw2: f64,
    pub p90_tail_coexceedance_fraction: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LaggedCorrelationMetric {
    pub first_site_id: String,
    pub second_site_id: String,
    /// Positive lag means the second site's sample occurs later than the first.
    pub lag_samples: i32,
    pub lag_ns: i64,
    pub sample_count: usize,
    pub pearson: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SitePowerSummary {
    pub site_id: String,
    pub sample_count: usize,
    pub mean_power_mw: f64,
    /// Population variance over the common aligned observation window.
    pub variance_power_mw2: f64,
    pub p90_power_mw: u64,
    pub peak_power_mw: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitePowerContribution {
    pub site_id: String,
    pub power_mw: u64,
    pub state_id: String,
    pub source_evidence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregatePowerSample {
    pub timestamp_ns: u64,
    pub total_power_mw: u128,
    /// Exact per-site contributions allow independent row-level sum reconstruction.
    pub site_contributions: Vec<SitePowerContribution>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiSiteAggregationReport {
    pub contract: MultiSiteAggregationContract,
    pub evidence_class: TraceEvidenceClass,
    pub aligned_sample_count: usize,
    pub site_summaries: Vec<SitePowerSummary>,
    pub aggregate_samples: Vec<AggregatePowerSample>,
    pub aggregate_mean_power_mw: f64,
    pub aggregate_variance_power_mw2: f64,
    pub aggregate_p99_power_mw: u128,
    pub aggregate_peak_power_mw: u128,
    /// Sum of marginal site variances: the zero-cross-covariance reference.
    pub independent_variance_reference_mw2: f64,
    /// Scenario-specific ratio Var(sum of sites) / sum(Var(site)).
    pub variance_amplification_factor: Option<f64>,
    pub variance_decomposition_residual_mw2: f64,
    pub pairwise_dependence: Vec<SiteDependenceMetric>,
    pub lagged_correlations: Vec<LaggedCorrelationMetric>,
    pub source_site_ids: Vec<String>,
    pub source_run_ids: Vec<String>,
    pub source_session_ids: Vec<String>,
    pub source_trace_evidence_ids: Vec<String>,
    pub source_state_mapping_evidence_ids: Vec<String>,
    pub clock_alignment_evidence_ids: Vec<String>,
    pub claim_ceiling: &'static str,
}

fn aligned_timestamp(timestamp_ns: u64, offset_ns: i64) -> Result<u64, AuditError> {
    if offset_ns >= 0 {
        timestamp_ns.checked_add(offset_ns as u64).ok_or_else(|| {
            AuditError("aligned site timestamp overflowed the common clock".to_string())
        })
    } else {
        timestamp_ns.checked_sub(offset_ns.unsigned_abs()).ok_or_else(|| {
            AuditError("aligned site timestamp underflowed the common clock".to_string())
        })
    }
}

fn validate_site_trace(
    contract: &MultiSiteAggregationContract,
    trace: &SitePowerTrace,
) -> Result<Vec<u64>, AuditError> {
    for (value, field) in [
        (trace.site_id.as_str(), "site_id"),
        (trace.parent_topology_id.as_str(), "parent_topology_id"),
        (trace.region_id.as_str(), "region_id"),
        (trace.run_id.as_str(), "run_id"),
        (trace.session_id.as_str(), "session_id"),
        (trace.dataset_id.as_str(), "dataset_id"),
        (trace.dataset_version.as_str(), "dataset_version"),
        (trace.dataset_evidence_id.as_str(), "dataset_evidence_id"),
        (trace.use_authorization_reference.as_str(), "use_authorization_reference"),
        (trace.trace_evidence_id.as_str(), "trace_evidence_id"),
        (trace.hardware_configuration_fingerprint.as_str(), "hardware_configuration_fingerprint"),
        (trace.measurement_boundary_id.as_str(), "measurement_boundary_id"),
        (trace.workload_contract_id.as_str(), "workload_contract_id"),
        (trace.orchestration_domain_id.as_str(), "orchestration_domain_id"),
        (trace.state_mapper_id.as_str(), "state_mapper_id"),
        (trace.state_mapping_evidence_id.as_str(), "state_mapping_evidence_id"),
        (trace.source_clock_id.as_str(), "source_clock_id"),
        (trace.source_time_origin_id.as_str(), "source_time_origin_id"),
        (trace.aligned_clock_id.as_str(), "aligned_clock_id"),
        (trace.aligned_time_origin_id.as_str(), "aligned_time_origin_id"),
        (trace.clock_alignment_method_id.as_str(), "clock_alignment_method_id"),
        (trace.clock_alignment_evidence_id.as_str(), "clock_alignment_evidence_id"),
    ] {
        if value.trim().is_empty() {
            return Err(AuditError(format!("site trace {field} must not be blank")));
        }
    }
    if trace.parent_topology_id != contract.parent_topology_id
        || trace.region_id != contract.region_id
        || trace.aligned_clock_id != contract.common_clock_id
        || trace.aligned_time_origin_id != contract.common_time_origin_id
        || trace.clock_alignment_method_id != contract.clock_alignment_method_id
        || trace.sample_period_ns != contract.sample_period_ns
        || trace.clock_alignment_uncertainty_ns > contract.maximum_alignment_uncertainty_ns
    {
        return Err(AuditError(format!(
            "site {:?} does not match the aggregation topology, region, time origin, alignment method, cadence or uncertainty limit",
            trace.site_id
        )));
    }
    if trace.samples.len() < contract.minimum_sample_count {
        return Err(AuditError(format!(
            "site {:?} has {} samples; minimum is {}",
            trace.site_id,
            trace.samples.len(),
            contract.minimum_sample_count
        )));
    }
    let mut aligned = Vec::with_capacity(trace.samples.len());
    let mut previous_source_time = None;
    for (index, sample) in trace.samples.iter().enumerate() {
        if sample.state_id.trim().is_empty() || sample.source_evidence_id.trim().is_empty() {
            return Err(AuditError(format!(
                "site {:?} sample {index} is missing state or source evidence",
                trace.site_id
            )));
        }
        if sample.quality != SiteSampleQuality::Good {
            return Err(AuditError(format!(
                "site {:?} sample {index} is not GOOD; invalid or missing samples cannot be silently removed",
                trace.site_id
            )));
        }
        if let Some(previous) = previous_source_time {
            let observed = sample.timestamp_ns.checked_sub(previous).ok_or_else(|| {
                AuditError(format!(
                    "site {:?} source timestamps are not strictly increasing",
                    trace.site_id
                ))
            })?;
            if observed != contract.sample_period_ns {
                return Err(AuditError(format!(
                    "site {:?} has irregular cadence: expected {} ns, observed {} ns",
                    trace.site_id, contract.sample_period_ns, observed
                )));
            }
        }
        aligned.push(aligned_timestamp(
            sample.timestamp_ns,
            trace.clock_offset_to_common_ns,
        )?);
        previous_source_time = Some(sample.timestamp_ns);
    }
    Ok(aligned)
}

fn mean(values: &[u64]) -> f64 {
    values.iter().map(|value| *value as f64).sum::<f64>() / values.len() as f64
}

fn population_variance(values: &[u64], average: f64) -> f64 {
    values
        .iter()
        .map(|value| {
            let centered = *value as f64 - average;
            centered * centered
        })
        .sum::<f64>()
        / values.len() as f64
}

fn population_covariance(left: &[u64], right: &[u64]) -> f64 {
    let left_mean = mean(left);
    let right_mean = mean(right);
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| (*a as f64 - left_mean) * (*b as f64 - right_mean))
        .sum::<f64>()
        / left.len() as f64
}

fn pearson(left: &[u64], right: &[u64]) -> Option<f64> {
    if left.len() != right.len() || left.len() < 2 {
        return None;
    }
    let covariance = population_covariance(left, right);
    let left_variance = population_variance(left, mean(left));
    let right_variance = population_variance(right, mean(right));
    let denominator = (left_variance * right_variance).sqrt();
    if !denominator.is_finite() || denominator <= f64::EPSILON {
        None
    } else {
        Some((covariance / denominator).clamp(-1.0, 1.0))
    }
}

fn nearest_rank_u64(values: &[u64], numerator: usize, denominator: usize) -> u64 {
    let mut ordered = values.to_vec();
    ordered.sort_unstable();
    let rank = ((ordered.len() * numerator + denominator - 1) / denominator).max(1);
    ordered[rank - 1]
}

fn nearest_rank_u128(values: &[u128], numerator: usize, denominator: usize) -> u128 {
    let mut ordered = values.to_vec();
    ordered.sort_unstable();
    let rank = ((ordered.len() * numerator + denominator - 1) / denominator).max(1);
    ordered[rank - 1]
}

fn paired_at_lag(left: &[u64], right: &[u64], lag: i32) -> (Vec<u64>, Vec<u64>) {
    if lag >= 0 {
        let shift = lag as usize;
        (left[..left.len() - shift].to_vec(), right[shift..].to_vec())
    } else {
        let shift = lag.unsigned_abs() as usize;
        (left[shift..].to_vec(), right[..right.len() - shift].to_vec())
    }
}

/// Aggregate at the exact common timestamps and estimate contemporaneous and
/// lagged pairwise dependence. This never interpolates or randomly permutes samples.
pub fn aggregate_aligned_sites(
    contract: MultiSiteAggregationContract,
    traces: &[SitePowerTrace],
) -> Result<MultiSiteAggregationReport, AuditError> {
    contract.validate()?;
    if traces.len() < 2 {
        return Err(AuditError("multi-site aggregation requires at least two site traces".to_string()));
    }

    let mut ordered_traces: Vec<&SitePowerTrace> = traces.iter().collect();
    ordered_traces.sort_by(|left, right| left.site_id.cmp(&right.site_id));
    let evidence_class = ordered_traces[0].evidence_class;
    let mut site_ids = HashSet::new();
    let mut aligned_times: Option<Vec<u64>> = None;
    let mut summaries = Vec::with_capacity(traces.len());
    let mut aligned_power: Vec<Vec<u64>> = Vec::with_capacity(traces.len());
    let mut source_site_ids = Vec::with_capacity(traces.len());
    let mut source_run_ids = Vec::with_capacity(traces.len());
    let mut source_session_ids = Vec::with_capacity(traces.len());
    let mut source_trace_evidence_ids = Vec::with_capacity(traces.len());
    let mut source_state_mapping_evidence_ids = Vec::with_capacity(traces.len());
    let mut clock_alignment_evidence_ids = Vec::with_capacity(traces.len());
    let mut alignment_uncertainties = Vec::with_capacity(traces.len());

    for trace in &ordered_traces {
        if trace.evidence_class != evidence_class {
            return Err(AuditError(
                "measured and modelled site traces cannot be silently mixed in one aggregate".to_string(),
            ));
        }
        if !site_ids.insert(trace.site_id.as_str()) {
            return Err(AuditError(format!("duplicate site ID {:?}", trace.site_id)));
        }
        let times = validate_site_trace(&contract, trace)?;
        alignment_uncertainties.push(trace.clock_alignment_uncertainty_ns);
        if let Some(reference) = &aligned_times {
            if times != *reference {
                return Err(AuditError(format!(
                    "site {:?} does not share the exact aligned timestamp sequence",
                    trace.site_id
                )));
            }
        } else {
            aligned_times = Some(times);
        }

        let values: Vec<u64> = trace.samples.iter().map(|sample| sample.power_mw).collect();
        let average = mean(&values);
        summaries.push(SitePowerSummary {
            site_id: trace.site_id.clone(),
            sample_count: values.len(),
            mean_power_mw: average,
            variance_power_mw2: population_variance(&values, average),
            p90_power_mw: nearest_rank_u64(&values, 9, 10),
            peak_power_mw: *values.iter().max().expect("sample count validated"),
        });
        aligned_power.push(values);
        source_site_ids.push(trace.site_id.clone());
        source_run_ids.push(trace.run_id.clone());
        source_session_ids.push(trace.session_id.clone());
        source_trace_evidence_ids.push(trace.trace_evidence_id.clone());
        source_state_mapping_evidence_ids.push(trace.state_mapping_evidence_id.clone());
        clock_alignment_evidence_ids.push(trace.clock_alignment_evidence_id.clone());
    }
    for first in 0..alignment_uncertainties.len() {
        for second in first + 1..alignment_uncertainties.len() {
            let worst_case_pairwise = alignment_uncertainties[first]
                .checked_add(alignment_uncertainties[second])
                .ok_or_else(|| AuditError("pairwise clock uncertainty overflowed".to_string()))?;
            if worst_case_pairwise > contract.maximum_alignment_uncertainty_ns {
                return Err(AuditError(format!(
                    "pairwise clock-alignment uncertainty for sites {:?} and {:?} exceeds the contract limit",
                    source_site_ids[first], source_site_ids[second]
                )));
            }
        }
    }

    let times = aligned_times.expect("at least two traces were validated");
    let sample_count = times.len();
    let mut aggregate_samples = Vec::with_capacity(sample_count);
    let mut aggregate_values = Vec::with_capacity(sample_count);
    for index in 0..sample_count {
        let mut site_contributions = Vec::with_capacity(ordered_traces.len());
        let mut total = 0_u128;
        for (trace, site_values) in ordered_traces.iter().zip(aligned_power.iter()) {
            let child_sample = &trace.samples[index];
            let contribution = site_values[index] as u128;
            total = total.checked_add(contribution).ok_or_else(|| {
                AuditError("aggregate site power overflowed u128".to_string())
            })?;
            site_contributions.push(SitePowerContribution {
                site_id: trace.site_id.clone(),
                power_mw: child_sample.power_mw,
                state_id: child_sample.state_id.clone(),
                source_evidence_id: child_sample.source_evidence_id.clone(),
            });
        }
        aggregate_values.push(total);
        aggregate_samples.push(AggregatePowerSample {
            timestamp_ns: times[index],
            total_power_mw: total,
            site_contributions,
        });
    }

    let aggregate_mean = aggregate_values.iter().map(|value| *value as f64).sum::<f64>()
        / sample_count as f64;
    let aggregate_variance = aggregate_values
        .iter()
        .map(|value| {
            let centered = *value as f64 - aggregate_mean;
            centered * centered
        })
        .sum::<f64>()
        / sample_count as f64;
    let independent_variance_reference = summaries
        .iter()
        .map(|summary| summary.variance_power_mw2)
        .sum::<f64>();
    let variance_amplification_factor = if independent_variance_reference > f64::EPSILON {
        Some(aggregate_variance / independent_variance_reference)
    } else {
        None
    };

    let mut pairwise_dependence = Vec::new();
    let mut lagged_correlations = Vec::new();
    let mut covariance_sum = 0.0_f64;
    for first in 0..aligned_power.len() {
        for second in first + 1..aligned_power.len() {
            let left = &aligned_power[first];
            let right = &aligned_power[second];
            let covariance = population_covariance(left, right);
            covariance_sum += covariance;
            let p90_left = summaries[first].p90_power_mw;
            let p90_right = summaries[second].p90_power_mw;
            let simultaneous_exceedances = left
                .iter()
                .zip(right.iter())
                .filter(|(a, b)| **a >= p90_left && **b >= p90_right)
                .count();
            pairwise_dependence.push(SiteDependenceMetric {
                first_site_id: summaries[first].site_id.clone(),
                second_site_id: summaries[second].site_id.clone(),
                contemporaneous_pearson: pearson(left, right),
                contemporaneous_covariance_mw2: covariance,
                p90_tail_coexceedance_fraction: if sample_count >= contract.minimum_tail_sample_count {
                    Some(simultaneous_exceedances as f64 / sample_count as f64)
                } else {
                    None
                },
            });

            for lag in -(contract.maximum_lag_samples as i32)..=(contract.maximum_lag_samples as i32) {
                let (lagged_left, lagged_right) = paired_at_lag(left, right, lag);
                lagged_correlations.push(LaggedCorrelationMetric {
                    first_site_id: summaries[first].site_id.clone(),
                    second_site_id: summaries[second].site_id.clone(),
                    lag_samples: lag,
                    lag_ns: (lag as i64)
                    .checked_mul(contract.sample_period_ns as i64)
                    .ok_or_else(|| AuditError("lag nanoseconds overflowed".to_string()))?,
                    sample_count: lagged_left.len(),
                    pearson: pearson(&lagged_left, &lagged_right),
                });
            }
        }
    }
    let variance_decomposition = independent_variance_reference + 2.0 * covariance_sum;
    let variance_decomposition_residual = (aggregate_variance - variance_decomposition).abs();

    Ok(MultiSiteAggregationReport {
        contract,
        evidence_class,
        aligned_sample_count: sample_count,
        site_summaries: summaries,
        aggregate_samples,
        aggregate_mean_power_mw: aggregate_mean,
        aggregate_variance_power_mw2: aggregate_variance,
        aggregate_p99_power_mw: nearest_rank_u128(&aggregate_values, 99, 100),
        aggregate_peak_power_mw: *aggregate_values.iter().max().expect("samples validated"),
        independent_variance_reference_mw2: independent_variance_reference,
        variance_amplification_factor,
        variance_decomposition_residual_mw2: variance_decomposition_residual,
        pairwise_dependence,
        lagged_correlations,
        source_site_ids,
        source_run_ids,
        source_session_ids,
        source_trace_evidence_ids,
        source_state_mapping_evidence_ids,
        clock_alignment_evidence_ids,
        claim_ceiling: match evidence_class {
            TraceEvidenceClass::Measured => "MEASURED_CHILD_TRACE_AGGREGATION_NOT_PCC_QUALIFICATION",
            TraceEvidenceClass::Modelled => "MODELLED_CHILD_TRACE_AGGREGATION_NOT_MEASURED_OR_GRID_QUALIFICATION",
        },
    })
}

#[cfg(test)]
mod tests {
    use super::{
        aggregate_aligned_sites, MultiSiteAggregationContract, SitePowerSample, SitePowerTrace,
        SiteSampleQuality, TraceEvidenceClass,
    };

    fn contract() -> MultiSiteAggregationContract {
        MultiSiteAggregationContract {
            aggregation_id: "agg-region-a-v1".to_string(),
            parent_topology_id: "region-a-root".to_string(),
            parent_topology_evidence_id: "topology-evidence-region-a".to_string(),
            region_id: "region-a".to_string(),
            common_clock_id: "ptp-root-clock".to_string(),
            common_time_origin_id: "epoch-utc-ns".to_string(),
            clock_alignment_method_id: "ptp-v1".to_string(),
            sample_period_ns: 1_000,
            maximum_alignment_uncertainty_ns: 100,
            minimum_sample_count: 5,
            minimum_tail_sample_count: 100,
            maximum_lag_samples: 2,
        }
    }

    fn trace(site: &str, offset: i64, values: &[u64]) -> SitePowerTrace {
        SitePowerTrace {
            site_id: site.to_string(),
            parent_topology_id: "region-a-root".to_string(),
            region_id: "region-a".to_string(),
            run_id: format!("run-{site}"),
            session_id: format!("session-{site}"),
            dataset_id: "dataset-fixture".to_string(),
            dataset_version: "v1".to_string(),
            dataset_evidence_id: format!("dataset-evidence-{site}"),
            use_authorization_reference: "rights-fixture".to_string(),
            trace_evidence_id: format!("trace-evidence-{site}"),
            hardware_configuration_fingerprint: format!("sha256-config-{site}"),
            measurement_boundary_id: format!("server-input-{site}"),
            workload_contract_id: "workload-contract-v1".to_string(),
            orchestration_domain_id: "shared-serving-domain".to_string(),
            state_mapper_id: "prefill-decode-idle-v1".to_string(),
            state_mapping_evidence_id: format!("state-map-evidence-{site}"),
            source_clock_id: format!("clock-{site}"),
            source_time_origin_id: format!("source-epoch-{site}"),
            aligned_clock_id: "ptp-root-clock".to_string(),
            aligned_time_origin_id: "epoch-utc-ns".to_string(),
            clock_alignment_method_id: "ptp-v1".to_string(),
            clock_alignment_evidence_id: format!("clock-evidence-{site}"),
            clock_offset_to_common_ns: offset,
            clock_alignment_uncertainty_ns: 10,
            sample_period_ns: 1_000,
            evidence_class: TraceEvidenceClass::Measured,
            samples: values.iter().enumerate().map(|(index, power)| SitePowerSample {
                timestamp_ns: 1_000 + index as u64 * 1_000,
                state_id: if index % 2 == 0 { "prefill" } else { "decode" }.to_string(),
                power_mw: *power,
                quality: SiteSampleQuality::Good,
                source_evidence_id: format!("{site}-sample-{index}"),
            }).collect(),
        }
    }

    #[test]
    fn aligns_site_offsets_and_sums_power_at_exact_common_timestamps() {
        let first = trace("site-a", 0, &[100, 200, 300, 200, 100]);
        let mut second = trace("site-b", -100, &[50, 100, 150, 100, 50]);
        for sample in &mut second.samples {
            sample.timestamp_ns += 100;
        }
        let report = aggregate_aligned_sites(contract(), &[first, second]).unwrap();
        assert_eq!(report.aligned_sample_count, 5);
        assert_eq!(report.aggregate_samples[0].timestamp_ns, 1_000);
        assert_eq!(report.aggregate_samples[0].total_power_mw, 150);
        assert_eq!(report.aggregate_samples[0].site_contributions.len(), 2);
        assert_eq!(report.aggregate_samples[0].site_contributions[0].site_id, "site-a");
        assert_eq!(report.aggregate_samples[0].site_contributions[0].source_evidence_id, "site-a-sample-0");
        assert_eq!(
            report.aggregate_samples[0].site_contributions.iter().map(|item| item.power_mw as u128).sum::<u128>(),
            report.aggregate_samples[0].total_power_mw
        );
        assert_eq!(report.aggregate_peak_power_mw, 450);
        assert_eq!(report.evidence_class, TraceEvidenceClass::Measured);
        assert_eq!(report.source_state_mapping_evidence_ids[0], "state-map-evidence-site-a");
    }

    #[test]
    fn identical_loads_show_variance_amplification_above_independence_reference() {
        let values = &[100, 200, 300, 200, 100];
        let report = aggregate_aligned_sites(
            contract(),
            &[trace("site-a", 0, values), trace("site-b", 0, values)],
        ).unwrap();
        assert!((report.pairwise_dependence[0].contemporaneous_pearson.unwrap() - 1.0).abs() < 1e-12);
        assert!((report.variance_amplification_factor.unwrap() - 2.0).abs() < 1e-12);
        assert!(report.variance_decomposition_residual_mw2 < 1e-9);
    }

    #[test]
    fn perfectly_anti_correlated_loads_have_zero_aggregate_variance() {
        let report = aggregate_aligned_sites(
            contract(),
            &[
                trace("site-a", 0, &[100, 200, 300, 200, 100]),
                trace("site-b", 0, &[300, 200, 100, 200, 300]),
            ],
        ).unwrap();
        assert!(report.aggregate_variance_power_mw2.abs() < 1e-9);
        assert!((report.pairwise_dependence[0].contemporaneous_pearson.unwrap() + 1.0).abs() < 1e-12);
    }

    #[test]
    fn lagged_cross_correlation_identifies_one_sample_shift() {
        let report = aggregate_aligned_sites(
            contract(),
            &[
                trace("site-a", 0, &[100, 200, 300, 200, 100]),
                trace("site-b", 0, &[200, 300, 200, 100, 100]),
            ],
        ).unwrap();
        let lag = report.lagged_correlations.iter()
            .find(|metric| metric.lag_samples == 1).unwrap();
        assert!(lag.pearson.is_some());
        assert_eq!(lag.sample_count, 4);
    }

    #[test]
    fn tail_coexceedance_is_reported_for_synchronized_peaks() {
        let values: Vec<u64> = (0..100).collect();
        let report = aggregate_aligned_sites(
            contract(),
            &[trace("site-a", 0, &values), trace("site-b", 0, &values)],
        ).unwrap();
        assert!(report.pairwise_dependence[0].p90_tail_coexceedance_fraction.unwrap() > 0.0);
        assert!(report.pairwise_dependence[0].p90_tail_coexceedance_fraction.unwrap() <= 1.0);
    }

    #[test]
    fn short_traces_do_not_claim_tail_coexceedance_support() {
        let report = aggregate_aligned_sites(
            contract(),
            &[
                trace("site-a", 0, &[100, 200, 300, 200, 100]),
                trace("site-b", 0, &[100, 200, 300, 200, 100]),
            ],
        ).unwrap();
        assert_eq!(report.pairwise_dependence[0].p90_tail_coexceedance_fraction, None);
    }

    #[test]
    fn clock_uncertainty_above_contract_limit_fails_closed() {
        let mut second = trace("site-b", 0, &[10, 20, 30, 20, 10]);
        second.clock_alignment_uncertainty_ns = 101;
        assert!(aggregate_aligned_sites(contract(), &[trace("site-a", 0, &[10, 20, 30, 20, 10]), second]).is_err());
    }

    #[test]
    fn pairwise_clock_uncertainty_uses_conservative_sum() {
        let mut first = trace("site-a", 0, &[10, 20, 30, 20, 10]);
        let mut second = trace("site-b", 0, &[10, 20, 30, 20, 10]);
        first.clock_alignment_uncertainty_ns = 60;
        second.clock_alignment_uncertainty_ns = 60;
        // Each site is below the individual 100 ns limit; their worst-case
        // relative uncertainty is 120 ns and must be rejected.
        assert!(aggregate_aligned_sites(contract(), &[first, second]).is_err());
    }

    #[test]
    fn different_alignment_method_fails_closed() {
        let mut second = trace("site-b", 0, &[10, 20, 30, 20, 10]);
        second.clock_alignment_method_id = "ntp-v2".to_string();
        assert!(aggregate_aligned_sites(contract(), &[trace("site-a", 0, &[10, 20, 30, 20, 10]), second]).is_err());
    }

    #[test]
    fn non_good_samples_are_not_silently_dropped() {
        let mut first = trace("site-a", 0, &[10, 20, 30, 20, 10]);
        first.samples[2].quality = SiteSampleQuality::Missing;
        assert!(aggregate_aligned_sites(contract(), &[first, trace("site-b", 0, &[10, 20, 30, 20, 10])]).is_err());
    }

    #[test]
    fn missing_cadence_and_misaligned_timestamps_fail_closed() {
        let first = trace("site-a", 0, &[10, 20, 30, 20, 10]);
        let mut second = trace("site-b", 0, &[10, 20, 30, 20, 10]);
        second.samples[2].timestamp_ns += 1;
        assert!(aggregate_aligned_sites(contract(), &[first, second]).is_err());
    }

    #[test]
    fn duplicate_sites_and_mixed_evidence_classes_fail_closed() {
        let first = trace("site-a", 0, &[10, 20, 30, 20, 10]);
        let duplicate = trace("site-a", 0, &[20, 30, 40, 30, 20]);
        assert!(aggregate_aligned_sites(contract(), &[first.clone(), duplicate]).is_err());

        let mut modelled = trace("site-b", 0, &[10, 20, 30, 20, 10]);
        modelled.evidence_class = TraceEvidenceClass::Modelled;
        assert!(aggregate_aligned_sites(contract(), &[first, modelled]).is_err());
    }

    #[test]
    fn wrong_parent_region_or_time_origin_fails_closed() {
        let first = trace("site-a", 0, &[10, 20, 30, 20, 10]);
        let mut second = trace("site-b", 0, &[10, 20, 30, 20, 10]);
        second.parent_topology_id = "different-parent".to_string();
        assert!(aggregate_aligned_sites(contract(), &[first.clone(), second]).is_err());

        let mut third = trace("site-c", 0, &[10, 20, 30, 20, 10]);
        third.aligned_time_origin_id = "wrong-epoch".to_string();
        assert!(aggregate_aligned_sites(contract(), &[first, third]).is_err());
    }

    #[test]
    fn signed_clock_offset_underflow_and_overflow_fail_closed() {
        let mut low = trace("site-a", -10_000, &[10, 20, 30, 20, 10]);
        for (index, sample) in low.samples.iter_mut().enumerate() {
            sample.timestamp_ns = 1 + index as u64 * 1_000;
        }
        assert!(aggregate_aligned_sites(contract(), &[low, trace("site-b", 0, &[10, 20, 30, 20, 10])]).is_err());

        let mut high = trace("site-c", 10, &[10, 20, 30, 20, 10]);
        for (index, sample) in high.samples.iter_mut().enumerate() {
            sample.timestamp_ns = u64::MAX - 4_000 + index as u64 * 1_000;
        }
        assert!(aggregate_aligned_sites(contract(), &[high, trace("site-d", 0, &[10, 20, 30, 20, 10])]).is_err());
    }

    #[test]
    fn lagged_correlations_are_reported_with_sample_counts_and_physical_lag() {
        let report = aggregate_aligned_sites(
            contract(),
            &[trace("site-a", 0, &[1, 2, 3, 4, 5]), trace("site-b", 0, &[5, 4, 3, 2, 1])],
        ).unwrap();
        assert_eq!(report.lagged_correlations.len(), 5);
        assert!(report.lagged_correlations.iter().all(|metric| metric.lag_ns == metric.lag_samples as i64 * 1_000));
    }

    #[test]
    fn output_order_is_canonical_by_site_id() {
        let first = trace("site-b", 0, &[10, 20, 30, 20, 10]);
        let second = trace("site-a", 0, &[20, 30, 40, 30, 20]);
        let forward = aggregate_aligned_sites(contract(), &[first.clone(), second.clone()]).unwrap();
        let reverse = aggregate_aligned_sites(contract(), &[second, first]).unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(forward.source_site_ids, vec!["site-a", "site-b"]);
        assert!(forward.aggregate_samples.iter().all(|sample| {
            sample.site_contributions.iter().map(|item| item.power_mw as u128).sum::<u128>()
                == sample.total_power_mw
        }));
    }

    #[test]
    fn mapped_clock_identity_must_match_common_clock() {
        let first = trace("site-a", 0, &[10, 20, 30, 20, 10]);
        let mut second = trace("site-b", 0, &[10, 20, 30, 20, 10]);
        second.aligned_clock_id = "different-clock".to_string();
        assert!(aggregate_aligned_sites(contract(), &[first, second]).is_err());
    }
}
