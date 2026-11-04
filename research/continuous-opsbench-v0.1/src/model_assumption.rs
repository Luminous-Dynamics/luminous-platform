//! Operational model-assumption monitor for predictive safety appraisal.
//!
//! Pure, deterministic evaluation of pre-issued predictions against independently
//! sourced outcomes. It can qualify, suspect, quarantine or mark evidence
//! indeterminate. It cannot authorize or execute a mutation.

use crate::Digest;
use std::collections::{BTreeMap, BTreeSet};

pub const MODEL_ASSUMPTION_SCHEMA: &str =
    "continuous-opsbench-model-assumption-monitor-v0.1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forecast {
    pub forecast_id: String,
    pub metric_id: String,
    pub units: String,
    pub issued_at_ms: u64,
    pub target_at_ms: u64,
    pub source_tick: u64,
    pub target_tick: u64,
    pub lower: i64,
    pub point: i64,
    pub upper: i64,
    pub model_revision: String,
    pub model_digest: Digest,
    pub plant_revision: String,
    pub plant_digest: Digest,
    pub workload_profile_digest: Digest,
    pub metric_policy_set_digest: Digest,
    pub topology_epoch: u64,
    pub topology_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndependentObservation {
    pub observation_id: String,
    pub forecast_id: String,
    pub metric_id: String,
    pub units: String,
    pub target_tick: u64,
    pub observed_at_ms: u64,
    pub ingested_at_ms: u64,
    pub clock_uncertainty_ms: u64,
    pub source_sequence: u64,
    pub actual: i64,
    pub evidence_digest: Digest,
    pub complete: bool,
    pub model_digest: Digest,
    pub plant_digest: Digest,
    pub workload_profile_digest: Digest,
    pub topology_epoch: u64,
    pub topology_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricAssumptionPolicy {
    pub metric_id: String,
    pub units: String,
    pub minimum_observations: usize,
    pub maximum_interval_width: u64,
    pub residual_scale: u64,
    pub cusum_allowance: u64,
    pub cusum_warning_threshold: u64,
    pub cusum_quarantine_threshold: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelAssumptionInput {
    pub run_id: String,
    pub scenario_id: String,
    pub arm_id: String,
    pub dependence_cluster_id: String,
    pub model_revision: String,
    pub model_digest: Digest,
    pub plant_revision: String,
    pub plant_digest: Digest,
    pub workload_profile_digest: Digest,
    pub topology_epoch: u64,
    pub topology_digest: Digest,
    pub model_producer_id: String,
    pub model_control_domain: String,
    pub observer_id: String,
    pub observer_control_domain: String,
    pub observer_adapter_digest: Digest,
    pub metric_policy_set_digest: Digest,
    pub maximum_observation_age_ms: u64,
    pub maximum_target_time_skew_ms: u64,
    pub metric_policies: Vec<MetricAssumptionPolicy>,
    pub forecasts: Vec<Forecast>,
    /// Must be in authoritative source-sequence order.
    pub observations: Vec<IndependentObservation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelDisposition {
    Unqualified,
    Qualified,
    DriftSuspected,
    Quarantined,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelFinding {
    InvalidIdentity,
    TrustDomainNotSeparated,
    InvalidThresholds,
    InvalidForecast,
    DuplicateForecast,
    MissingObservation,
    DuplicateObservation,
    DuplicateEvidenceDigest,
    SequenceRegression,
    SequenceGap,
    IncompleteObservation,
    ObservationBindingMismatch,
    ModelIdentityMismatch,
    FutureObservation,
    StaleObservation,
    EnvelopeMiss,
    IntervalTooWide,
    InsufficientSamples,
    DriftWarning,
    DriftQuarantine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricAssumptionSummary {
    pub metric_id: String,
    pub forecast_count: usize,
    pub matched_observation_count: usize,
    pub covered_count: usize,
    pub envelope_miss_count: usize,
    pub too_wide_count: usize,
    pub final_positive_cusum: i128,
    pub final_negative_cusum: i128,
    pub worst_interval_width: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelAssumptionReport {
    pub schema: String,
    pub disposition: ModelDisposition,
    pub run_id: String,
    pub scenario_id: String,
    pub arm_id: String,
    pub dependence_cluster_id: String,
    pub model_revision: String,
    pub model_digest: Digest,
    pub plant_revision: String,
    pub plant_digest: Digest,
    pub workload_profile_digest: Digest,
    pub topology_epoch: u64,
    pub topology_digest: Digest,
    pub model_control_domain: String,
    pub observer_control_domain: String,
    pub forecast_count: usize,
    pub observation_count: usize,
    pub matched_observation_count: usize,
    pub envelope_miss_count: usize,
    pub too_wide_count: usize,
    pub maximum_positive_cusum: i128,
    pub maximum_negative_cusum: i128,
    pub findings: Vec<ModelFinding>,
    pub metrics: Vec<MetricAssumptionSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelAssumptionError {
    ArithmeticOverflow,
}

#[derive(Default)]
struct MetricAccumulator {
    forecast_count: usize,
    matched: usize,
    covered: usize,
    misses: usize,
    too_wide: usize,
    positive_cusum: i128,
    negative_cusum: i128,
    worst_width: u128,
}

fn nonzero(digest: &Digest) -> bool {
    digest.0.iter().any(|byte| *byte != 0)
}

fn scope_identity_matches(
    input: &ModelAssumptionInput,
    model_revision: &str,
    model_digest: &Digest,
    plant_revision: &str,
    plant_digest: &Digest,
    workload_digest: &Digest,
    topology_epoch: u64,
    topology_digest: &Digest,
) -> bool {
    input.model_revision == model_revision
        && &input.model_digest == model_digest
        && input.plant_revision == plant_revision
        && &input.plant_digest == plant_digest
        && &input.workload_profile_digest == workload_digest
        && input.topology_epoch == topology_epoch
        && &input.topology_digest == topology_digest
}

fn build_report(
    input: &ModelAssumptionInput,
    disposition: ModelDisposition,
    findings: BTreeSet<ModelFinding>,
    accumulators: BTreeMap<String, MetricAccumulator>,
    matched_observation_count: usize,
    envelope_miss_count: usize,
    too_wide_count: usize,
    maximum_positive_cusum: i128,
    maximum_negative_cusum: i128,
) -> ModelAssumptionReport {
    let metrics = accumulators
        .into_iter()
        .map(|(metric_id, value)| MetricAssumptionSummary {
            metric_id,
            forecast_count: value.forecast_count,
            matched_observation_count: value.matched,
            covered_count: value.covered,
            envelope_miss_count: value.misses,
            too_wide_count: value.too_wide,
            final_positive_cusum: value.positive_cusum,
            final_negative_cusum: value.negative_cusum,
            worst_interval_width: value.worst_width,
        })
        .collect();

    ModelAssumptionReport {
        schema: MODEL_ASSUMPTION_SCHEMA.to_owned(),
        disposition,
        run_id: input.run_id.clone(),
        scenario_id: input.scenario_id.clone(),
        arm_id: input.arm_id.clone(),
        dependence_cluster_id: input.dependence_cluster_id.clone(),
        model_revision: input.model_revision.clone(),
        model_digest: input.model_digest.clone(),
        plant_revision: input.plant_revision.clone(),
        plant_digest: input.plant_digest.clone(),
        workload_profile_digest: input.workload_profile_digest.clone(),
        metric_policy_set_digest: input.metric_policy_set_digest.clone(),
        topology_epoch: input.topology_epoch,
        topology_digest: input.topology_digest.clone(),
        model_control_domain: input.model_control_domain.clone(),
        observer_control_domain: input.observer_control_domain.clone(),
        forecast_count: input.forecasts.len(),
        observation_count: input.observations.len(),
        matched_observation_count,
        envelope_miss_count,
        too_wide_count,
        maximum_positive_cusum,
        maximum_negative_cusum,
        findings: findings.into_iter().collect(),
        metrics,
    }
}

/// Evaluate one frozen model-assumption window.
///
/// A model can become Qualified only for the exact identity and workload
/// supplied in this input. It must not be treated as a durable certificate:
/// the caller must re-evaluate after identity changes and on every new window.
///
/// Envelope misses and hard CUSUM crossings quarantine the model scope.
/// Invalid/missing evidence is Indeterminate; insufficient but otherwise valid
/// samples are Unqualified. This function has no mutation or authorization port.
pub fn evaluate_model_assumptions(
    input: &ModelAssumptionInput,
) -> Result<ModelAssumptionReport, ModelAssumptionError> {
    let mut findings = BTreeSet::new();

    let required_strings = [
        input.run_id.as_str(),
        input.scenario_id.as_str(),
        input.arm_id.as_str(),
        input.dependence_cluster_id.as_str(),
        input.model_revision.as_str(),
        input.plant_revision.as_str(),
        input.model_producer_id.as_str(),
        input.model_control_domain.as_str(),
        input.observer_id.as_str(),
        input.observer_control_domain.as_str(),
    ];
    if required_strings.iter().any(|value| value.is_empty())
        || !nonzero(&input.model_digest)
        || !nonzero(&input.plant_digest)
        || !nonzero(&input.workload_profile_digest)
        || !nonzero(&input.topology_digest)
        || !nonzero(&input.observer_adapter_digest)
        || !nonzero(&input.metric_policy_set_digest)
    {
        findings.insert(ModelFinding::InvalidIdentity);
    }
    if input.model_control_domain == input.observer_control_domain
        || input.model_producer_id == input.observer_id
    {
        findings.insert(ModelFinding::TrustDomainNotSeparated);
    }
    let mut policy_by_metric = BTreeMap::new();
    if input.metric_policies.is_empty() {
        findings.insert(ModelFinding::InvalidThresholds);
    }
    for policy in &input.metric_policies {
        if policy.metric_id.is_empty()
            || policy.units.is_empty()
            || policy.minimum_observations == 0
            || policy.residual_scale == 0
            || policy.cusum_warning_threshold == 0
            || policy.cusum_quarantine_threshold <= policy.cusum_warning_threshold
            || policy_by_metric.insert(policy.metric_id.as_str(), policy).is_some()
        {
            findings.insert(ModelFinding::InvalidThresholds);
        }
    }
    if findings.contains(&ModelFinding::InvalidIdentity)
        || findings.contains(&ModelFinding::TrustDomainNotSeparated)
        || findings.contains(&ModelFinding::InvalidThresholds)
    {
        return Ok(build_report(
            input,
            ModelDisposition::Indeterminate,
            findings,
            BTreeMap::new(),
            0,
            0,
            0,
            0,
            0,
        ));
    }

    let mut forecast_by_id = BTreeMap::new();
    let mut accumulators: BTreeMap<String, MetricAccumulator> = BTreeMap::new();
    for forecast in &input.forecasts {
        let policy = match policy_by_metric.get(forecast.metric_id.as_str()) {
            Some(policy) => *policy,
            None => {
                findings.insert(ModelFinding::InvalidForecast);
                continue;
            }
        };
        if forecast.forecast_id.is_empty()
            || forecast.metric_id.is_empty()
            || forecast.units.is_empty()
            || forecast.units != policy.units
            || forecast.model_revision.is_empty()
            || forecast.plant_revision.is_empty()
            || forecast.issued_at_ms >= forecast.target_at_ms
            || forecast.source_tick >= forecast.target_tick
            || forecast.lower > forecast.point
            || forecast.point > forecast.upper
            || !scope_identity_matches(
                input,
                &forecast.model_revision,
                &forecast.model_digest,
                &forecast.plant_revision,
                &forecast.plant_digest,
                &forecast.workload_profile_digest,
                forecast.topology_epoch,
                &forecast.topology_digest,
            )
        {
            findings.insert(ModelFinding::InvalidForecast);
            continue;
        }
        if forecast_by_id.insert(forecast.forecast_id.as_str(), forecast).is_some() {
            findings.insert(ModelFinding::DuplicateForecast);
            continue;
        }
        let width = (i128::from(forecast.upper) - i128::from(forecast.lower)) as u128;
        let metric = accumulators.entry(forecast.metric_id.clone()).or_default();
        metric.forecast_count += 1;
        metric.worst_width = metric.worst_width.max(width);
        if width > u128::from(policy.maximum_interval_width) {
            metric.too_wide += 1;
            findings.insert(ModelFinding::IntervalTooWide);
        }
    }

    let mut observation_ids = BTreeSet::new();
    let mut evidence_digests = BTreeSet::new();
    let mut matched_forecast_ids = BTreeSet::new();
    let mut previous_sequence: Option<u64> = None;
    let mut previous_target_by_metric: BTreeMap<String, u64> = BTreeMap::new();
    let mut matched_observations = 0usize;
    let mut misses = 0usize;
    let mut too_wide = accumulators.values().map(|item| item.too_wide).sum::<usize>();
    let mut max_positive = 0i128;
    let mut max_negative = 0i128;
    let mut hard_drift = false;
    let mut warning_drift = false;

    for observation in &input.observations {
        if observation.observation_id.is_empty()
            || !observation_ids.insert(observation.observation_id.as_str())
        {
            findings.insert(ModelFinding::DuplicateObservation);
            continue;
        }
        if !nonzero(&observation.evidence_digest)
            || !evidence_digests.insert(observation.evidence_digest.clone())
        {
            findings.insert(ModelFinding::DuplicateEvidenceDigest);
            continue;
        }
        if let Some(previous) = previous_sequence {
            if observation.source_sequence <= previous {
                findings.insert(ModelFinding::SequenceRegression);
                continue;
            }
            if previous.checked_add(1) != Some(observation.source_sequence) {
                findings.insert(ModelFinding::SequenceGap);
                continue;
            }
        }
        previous_sequence = Some(observation.source_sequence);

        if !observation.complete {
            findings.insert(ModelFinding::IncompleteObservation);
            continue;
        }
        if observation.ingested_at_ms < observation.observed_at_ms {
            findings.insert(ModelFinding::FutureObservation);
            continue;
        }
        let ingest_age = match observation
            .ingested_at_ms
            .checked_sub(observation.observed_at_ms)
            .and_then(|age| age.checked_add(observation.clock_uncertainty_ms))
        {
            Some(age) => age,
            None => {
                findings.insert(ModelFinding::StaleObservation);
                continue;
            }
        };
        if ingest_age > input.maximum_observation_age_ms {
            findings.insert(ModelFinding::StaleObservation);
            continue;
        }

        let forecast = match forecast_by_id.get(observation.forecast_id.as_str()) {
            Some(forecast) => *forecast,
            None => {
                findings.insert(ModelFinding::ObservationBindingMismatch);
                continue;
            }
        };
        if forecast.metric_id != observation.metric_id
            || forecast.units != observation.units
            || forecast.target_tick != observation.target_tick
            || !scope_identity_matches(
                input,
                &forecast.model_revision,
                &observation.model_digest,
                &forecast.plant_revision,
                &observation.plant_digest,
                &observation.workload_profile_digest,
                observation.topology_epoch,
                &observation.topology_digest,
            )
            || forecast.model_digest != observation.model_digest
            || forecast.plant_digest != observation.plant_digest
            || forecast.workload_profile_digest != observation.workload_profile_digest
            || forecast.topology_epoch != observation.topology_epoch
            || forecast.topology_digest != observation.topology_digest
        {
            findings.insert(ModelFinding::ModelIdentityMismatch);
            continue;
        }
        let target_skew = observation.observed_at_ms.abs_diff(forecast.target_at_ms);
        if target_skew
            .checked_add(observation.clock_uncertainty_ms)
            .map(|skew| skew > input.maximum_target_time_skew_ms)
            .unwrap_or(true)
        {
            findings.insert(ModelFinding::StaleObservation);
            continue;
        }
        if let Some(previous_target) = previous_target_by_metric.insert(
            observation.metric_id.clone(),
            observation.target_tick,
        ) {
            if observation.target_tick <= previous_target {
                findings.insert(ModelFinding::SequenceRegression);
                continue;
            }
        }
        if !matched_forecast_ids.insert(observation.forecast_id.as_str()) {
            findings.insert(ModelFinding::DuplicateObservation);
            continue;
        }

        let policy = match policy_by_metric.get(observation.metric_id.as_str()) {
            Some(policy) => *policy,
            None => {
                findings.insert(ModelFinding::ObservationBindingMismatch);
                continue;
            }
        };
        let metric = accumulators
            .entry(observation.metric_id.clone())
            .or_default();
        metric.matched += 1;
        matched_observations += 1;

        if observation.actual < forecast.lower || observation.actual > forecast.upper {
            metric.misses += 1;
            misses += 1;
            findings.insert(ModelFinding::EnvelopeMiss);
        } else {
            metric.covered += 1;
        }

        let residual = i128::from(observation.actual) - i128::from(forecast.point);
        let normalized = residual / i128::from(policy.residual_scale);
        let allowance = i128::from(policy.cusum_allowance);
        metric.positive_cusum = metric
            .positive_cusum
            .checked_add(normalized)
            .and_then(|value| value.checked_sub(allowance))
            .ok_or(ModelAssumptionError::ArithmeticOverflow)?
            .max(0);
        metric.negative_cusum = metric
            .negative_cusum
            .checked_sub(normalized)
            .and_then(|value| value.checked_sub(allowance))
            .ok_or(ModelAssumptionError::ArithmeticOverflow)?
            .max(0);
        max_positive = max_positive.max(metric.positive_cusum);
        max_negative = max_negative.max(metric.negative_cusum);
        if metric.positive_cusum >= i128::from(policy.cusum_quarantine_threshold)
            || metric.negative_cusum >= i128::from(policy.cusum_quarantine_threshold)
        {
            hard_drift = true;
        } else if metric.positive_cusum >= i128::from(policy.cusum_warning_threshold)
            || metric.negative_cusum >= i128::from(policy.cusum_warning_threshold)
        {
            warning_drift = true;
        }
    }

    let missing = forecast_by_id
        .keys()
        .any(|forecast_id| !matched_forecast_ids.contains(forecast_id));
    if missing {
        findings.insert(ModelFinding::MissingObservation);
    }
    if warning_drift {
        findings.insert(ModelFinding::DriftWarning);
    }
    if hard_drift {
        findings.insert(ModelFinding::DriftQuarantine);
    }
    too_wide = accumulators.values().map(|item| item.too_wide).sum::<usize>();

    let invalid_evidence = [
        ModelFinding::InvalidForecast,
        ModelFinding::DuplicateForecast,
        ModelFinding::MissingObservation,
        ModelFinding::DuplicateObservation,
        ModelFinding::DuplicateEvidenceDigest,
        ModelFinding::SequenceRegression,
        ModelFinding::SequenceGap,
        ModelFinding::IncompleteObservation,
        ModelFinding::ObservationBindingMismatch,
        ModelFinding::ModelIdentityMismatch,
        ModelFinding::FutureObservation,
        ModelFinding::StaleObservation,
    ]
    .iter()
    .any(|finding| findings.contains(finding));

    let insufficient = accumulators.iter().any(|(metric_id, metric)| {
        policy_by_metric
            .get(metric_id.as_str())
            .map(|policy| metric.matched < policy.minimum_observations)
            .unwrap_or(true)
    }) || accumulators.is_empty();

    let disposition = if misses > 0 || hard_drift {
        ModelDisposition::Quarantined
    } else if invalid_evidence {
        ModelDisposition::Indeterminate
    } else if warning_drift {
        ModelDisposition::DriftSuspected
    } else if too_wide > 0 || insufficient {
        if too_wide > 0 {
            ModelDisposition::Unqualified
        } else {
            ModelDisposition::Unqualified
        }
    } else {
        ModelDisposition::Qualified
    };

    if insufficient {
        findings.insert(ModelFinding::InsufficientSamples);
    }

    Ok(build_report(
        input,
        disposition,
        findings,
        accumulators,
        matched_observations,
        misses,
        too_wide,
        max_positive,
        max_negative,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u8) -> Digest {
        Digest::from_byte(value)
    }

    fn forecast(id: &str, point: i64, index: usize) -> Forecast {
        let source_tick = index as u64 + 1;
        let target_tick = source_tick + 1;
        let target_at_ms = 200 + index as u64 * 100;
        Forecast {
            forecast_id: id.into(),
            metric_id: "critical_queue".into(),
            units: "requests".into(),
            issued_at_ms: target_at_ms - 100,
            target_at_ms,
            source_tick,
            target_tick,
            lower: point - 5,
            point,
            upper: point + 5,
            model_revision: "model-v1".into(),
            model_digest: digest(1),
            plant_revision: "plant-v1".into(),
            plant_digest: digest(2),
            workload_profile_digest: digest(3),
            topology_epoch: 7,
            topology_digest: digest(4),
        }
    }

    fn observation(id: &str, forecast_id: &str, actual: i64, sequence: u64, index: usize) -> IndependentObservation {
        let target_tick = index as u64 + 2;
        let observed_at_ms = 200 + index as u64 * 100;
        IndependentObservation {
            observation_id: id.into(),
            forecast_id: forecast_id.into(),
            metric_id: "critical_queue".into(),
            units: "requests".into(),
            target_tick,
            observed_at_ms,
            ingested_at_ms: observed_at_ms + 5,
            clock_uncertainty_ms: 1,
            source_sequence: sequence,
            actual,
            evidence_digest: digest((sequence + 10) as u8),
            complete: true,
            model_digest: digest(1),
            plant_digest: digest(2),
            workload_profile_digest: digest(3),
            topology_epoch: 7,
            topology_digest: digest(4),
        }
    }

    fn input(count: usize) -> ModelAssumptionInput {
        let forecasts = (0..count)
            .map(|index| forecast(&format!("forecast-{index}"), 100, index))
            .collect::<Vec<_>>();
        let observations = (0..count)
            .map(|index| {
                observation(
                    &format!("observation-{index}"),
                    &format!("forecast-{index}"),
                    100,
                    index as u64 + 1,
                    index,
                )
            })
            .collect();
        ModelAssumptionInput {
            run_id: "run-1".into(),
            scenario_id: "scenario-1".into(),
            arm_id: "candidate".into(),
            dependence_cluster_id: "cluster-1".into(),
            model_revision: "model-v1".into(),
            model_digest: digest(1),
            plant_revision: "plant-v1".into(),
            plant_digest: digest(2),
            workload_profile_digest: digest(3),
            topology_epoch: 7,
            topology_digest: digest(4),
            model_producer_id: "model-builder".into(),
            model_control_domain: "model-domain".into(),
            observer_id: "service-observer".into(),
            observer_control_domain: "observer-domain".into(),
            observer_adapter_digest: digest(5),
            metric_policy_set_digest: digest(6),
            maximum_observation_age_ms: 10,
            maximum_target_time_skew_ms: 5,
            metric_policies: vec![MetricAssumptionPolicy {
                metric_id: "critical_queue".into(),
                units: "requests".into(),
                minimum_observations: 3,
                maximum_interval_width: 10,
                residual_scale: 1,
                cusum_allowance: 1,
                cusum_warning_threshold: 3,
                cusum_quarantine_threshold: 6,
            }],
            forecasts,
            observations,
        }
    }

    #[test]
    fn exact_boundary_coverage_qualifies_only_after_minimum_samples() {
        let result = evaluate_model_assumptions(&input(3)).unwrap();
        assert_eq!(result.disposition, ModelDisposition::Qualified);
        assert_eq!(result.matched_observation_count, 3);
        assert_eq!(result.envelope_miss_count, 0);
    }

    #[test]
    fn insufficient_sample_count_is_unqualified() {
        let result = evaluate_model_assumptions(&input(2)).unwrap();
        assert_eq!(result.disposition, ModelDisposition::Unqualified);
        assert!(result.findings.contains(&ModelFinding::InsufficientSamples));
    }

    #[test]
    fn one_unit_outside_interval_quarantines() {
        let mut evidence = input(3);
        evidence.observations[1].actual = 106;
        let result = evaluate_model_assumptions(&evidence).unwrap();
        assert_eq!(result.disposition, ModelDisposition::Quarantined);
        assert!(result.findings.contains(&ModelFinding::EnvelopeMiss));
    }

    #[test]
    fn wide_forecasts_cannot_qualify_even_with_perfect_coverage() {
        let mut evidence = input(3);
        for forecast in &mut evidence.forecasts {
            forecast.lower = 0;
            forecast.upper = 1_000;
        }
        let result = evaluate_model_assumptions(&evidence).unwrap();
        assert_eq!(result.disposition, ModelDisposition::Unqualified);
        assert!(result.findings.contains(&ModelFinding::IntervalTooWide));
    }

    #[test]
    fn positive_residual_drift_is_detected_before_interval_failure() {
        let mut evidence = input(4);
        for observation in &mut evidence.observations {
            observation.actual = 102;
        }
        let result = evaluate_model_assumptions(&evidence).unwrap();
        assert_eq!(result.disposition, ModelDisposition::DriftSuspected);
        assert!(result.findings.contains(&ModelFinding::DriftWarning));
    }

    #[test]
    fn severe_negative_residual_drift_quarantines() {
        let mut evidence = input(4);
        for observation in &mut evidence.observations {
            observation.actual = 97;
        }
        let result = evaluate_model_assumptions(&evidence).unwrap();
        assert_eq!(result.disposition, ModelDisposition::Quarantined);
        assert!(result.findings.contains(&ModelFinding::DriftQuarantine));
    }

    #[test]
    fn independent_control_domains_are_required() {
        let mut evidence = input(3);
        evidence.observer_control_domain = evidence.model_control_domain.clone();
        let result = evaluate_model_assumptions(&evidence).unwrap();
        assert_eq!(result.disposition, ModelDisposition::Indeterminate);
        assert!(result.findings.contains(&ModelFinding::TrustDomainNotSeparated));
    }

    #[test]
    fn duplicate_evidence_digest_is_indeterminate() {
        let mut evidence = input(3);
        evidence.observations[1].evidence_digest =
            evidence.observations[0].evidence_digest.clone();
        let result = evaluate_model_assumptions(&evidence).unwrap();
        assert_eq!(result.disposition, ModelDisposition::Indeterminate);
        assert!(result.findings.contains(&ModelFinding::DuplicateEvidenceDigest));
    }

    #[test]
    fn source_sequence_gap_is_not_treated_as_a_clean_observation() {
        let mut evidence = input(3);
        evidence.observations[1].source_sequence = 4;
        let result = evaluate_model_assumptions(&evidence).unwrap();
        assert_eq!(result.disposition, ModelDisposition::Indeterminate);
        assert!(result.findings.contains(&ModelFinding::SequenceGap));
    }

    #[test]
    fn stale_observation_is_indeterminate() {
        let mut evidence = input(3);
        evidence.observations[0].ingested_at_ms = 1_000;
        let result = evaluate_model_assumptions(&evidence).unwrap();
        assert_eq!(result.disposition, ModelDisposition::Indeterminate);
        assert!(result.findings.contains(&ModelFinding::StaleObservation));
    }

    #[test]
    fn topology_identity_mismatch_invalidates_the_window() {
        let mut evidence = input(3);
        evidence.observations[0].topology_epoch = 8;
        evidence.observations[0].topology_digest = digest(8);
        let result = evaluate_model_assumptions(&evidence).unwrap();
        assert_eq!(result.disposition, ModelDisposition::Indeterminate);
        assert!(result.findings.contains(&ModelFinding::ModelIdentityMismatch));
    }

    #[test]
    fn incomplete_observation_is_indeterminate() {
        let mut evidence = input(3);
        evidence.observations[0].complete = false;
        let result = evaluate_model_assumptions(&evidence).unwrap();
        assert_eq!(result.disposition, ModelDisposition::Indeterminate);
        assert!(result.findings.contains(&ModelFinding::IncompleteObservation));
    }

    #[test]
    fn forecast_must_precede_its_target() {
        let mut evidence = input(3);
        evidence.forecasts[0].issued_at_ms = 250;
        let result = evaluate_model_assumptions(&evidence).unwrap();
        assert_eq!(result.disposition, ModelDisposition::Indeterminate);
        assert!(result.findings.contains(&ModelFinding::InvalidForecast));
    }

    #[test]
    fn model_revision_change_is_not_silently_recalibrated() {
        let mut evidence = input(3);
        evidence.forecasts[1].model_revision = "model-v2".into();
        evidence.forecasts[1].model_digest = digest(9);
        let result = evaluate_model_assumptions(&evidence).unwrap();
        assert_eq!(result.disposition, ModelDisposition::Indeterminate);
        assert!(result.findings.contains(&ModelFinding::InvalidForecast));
    }
}
