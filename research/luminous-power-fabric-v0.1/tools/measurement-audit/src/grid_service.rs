use crate::AuditError;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridServiceClass {
    /// Energy-market or tariff-facing load shifting and peak management.
    MarketServing,
    /// Distribution/transmission constraint relief at a declared location.
    GridServing,
    /// System-level operational support through contracted demand reduction.
    SystemServing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterfactualBaselineMethod {
    /// Replays a frozen, identical workload under a declared no-dispatch policy.
    FrozenWorkloadReplay,
    /// Uses a concurrent, independently metered matched control.
    ConcurrentMatchedControl,
    /// Uses a pre-registered model with a separate validation evidence reference.
    IndependentValidatedModel,
    /// Not eligible to support a capacity commitment.
    Unqualified,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridServiceContract {
    pub resource_id: String,
    pub grid_zone_id: String,
    pub point_of_interconnection_id: String,
    pub site_operating_envelope_id: String,
    pub workload_contract_id: String,
    pub request_mix_id: String,
    pub quality_contract_id: String,
    pub boundary_id: String,
    pub control_action_id: String,
    pub counterfactual_baseline_id: String,
    pub counterfactual_baseline_evidence_id: String,
    pub baseline_validation_evidence_id: Option<String>,
    pub baseline_method: CounterfactualBaselineMethod,
    /// The baseline method, model and parameters must be frozen before dispatch.
    pub baseline_preregistered: bool,
    /// This identifies how event trials were selected and stratified.
    pub event_sampling_design_id: String,
    /// A declared design review says the events are suitable as independent
    /// Bernoulli trials. This flag is not itself proof of independence.
    pub independent_event_assumption: bool,
    pub service_class: GridServiceClass,
    pub committed_compute_reduction_w: f64,
    pub minimum_event_count: usize,
    pub minimum_success_probability: f64,
    /// Normal-score z value for the one-sided Wilson lower confidence bound.
    pub confidence_z_score: f64,
    pub minimum_useful_work_retention_fraction: f64,
    pub maximum_p99_latency_increase_fraction: f64,
    pub maximum_p99_ttft_increase_fraction: f64,
    pub maximum_slo_violation_fraction: f64,
    pub maximum_rebound_energy_fraction: f64,
    pub maximum_response_latency_ms: f64,
    pub minimum_sustained_duration_s: f64,
    pub minimum_recovery_window_s: f64,
    pub maximum_combined_uncertainty_fraction: f64,
}

impl GridServiceContract {
    fn validate(&self) -> Result<(), AuditError> {
        for (value, field) in [
            (self.resource_id.as_str(), "resource_id"),
            (self.grid_zone_id.as_str(), "grid_zone_id"),
            (self.point_of_interconnection_id.as_str(), "point_of_interconnection_id"),
            (self.site_operating_envelope_id.as_str(), "site_operating_envelope_id"),
            (self.workload_contract_id.as_str(), "workload_contract_id"),
            (self.request_mix_id.as_str(), "request_mix_id"),
            (self.quality_contract_id.as_str(), "quality_contract_id"),
            (self.boundary_id.as_str(), "boundary_id"),
            (self.control_action_id.as_str(), "control_action_id"),
            (self.counterfactual_baseline_id.as_str(), "counterfactual_baseline_id"),
            (self.counterfactual_baseline_evidence_id.as_str(), "counterfactual_baseline_evidence_id"),
            (self.event_sampling_design_id.as_str(), "event_sampling_design_id"),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!("grid-service contract {field} must not be blank")));
            }
        }

        if !self.baseline_preregistered {
            return Err(AuditError(
                "counterfactual baseline must be preregistered before dispatch".to_string(),
            ));
        }
        if self.baseline_method == CounterfactualBaselineMethod::Unqualified {
            return Err(AuditError(
                "an unqualified counterfactual baseline cannot support a grid-service commitment"
                    .to_string(),
            ));
        }
        if self.baseline_method == CounterfactualBaselineMethod::IndependentValidatedModel
            && self
                .baseline_validation_evidence_id
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(AuditError(
                "an independent baseline model requires separate validation evidence".to_string(),
            ));
        }
        if self
            .baseline_validation_evidence_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(AuditError(
                "baseline_validation_evidence_id must be absent or non-blank".to_string(),
            ));
        }
        if self.minimum_event_count < 2 {
            return Err(AuditError(
                "minimum_event_count must be at least two".to_string(),
            ));
        }

        let minimum_fractions = [
            self.minimum_success_probability,
            self.minimum_useful_work_retention_fraction,
        ];
        let maximum_fractions = [
            self.maximum_slo_violation_fraction,
            self.maximum_rebound_energy_fraction,
            self.maximum_combined_uncertainty_fraction,
        ];
        if minimum_fractions
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0 || *value > 1.0)
            || maximum_fractions
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0 || *value > 1.0)
            || !self.maximum_p99_latency_increase_fraction.is_finite()
            || self.maximum_p99_latency_increase_fraction < 0.0
            || !self.maximum_p99_ttft_increase_fraction.is_finite()
            || self.maximum_p99_ttft_increase_fraction < 0.0
            || !self.committed_compute_reduction_w.is_finite()
            || self.committed_compute_reduction_w <= 0.0
            || !self.confidence_z_score.is_finite()
            || self.confidence_z_score <= 0.0
            || !self.maximum_response_latency_ms.is_finite()
            || self.maximum_response_latency_ms <= 0.0
            || !self.minimum_sustained_duration_s.is_finite()
            || self.minimum_sustained_duration_s <= 0.0
            || !self.minimum_recovery_window_s.is_finite()
            || self.minimum_recovery_window_s <= 0.0
        {
            return Err(AuditError(
                "grid-service contract contains a non-finite or out-of-domain threshold"
                    .to_string(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridDispatchEvent {
    pub event_id: String,
    pub evidence_id: String,
    pub resource_id: String,
    pub grid_zone_id: String,
    pub point_of_interconnection_id: String,
    pub site_operating_envelope_id: String,
    pub workload_contract_id: String,
    pub request_mix_id: String,
    pub quality_contract_id: String,
    pub boundary_id: String,
    pub control_action_id: String,
    pub window_start_ns: u64,
    pub window_end_ns: u64,
    /// Mean site/PCC active power under the preregistered no-dispatch baseline.
    pub site_baseline_power_w: f64,
    /// Mean site/PCC active power during the dispatch event.
    pub site_event_power_w: f64,
    /// Independently measured non-compute load reduction. Positive means
    /// HVAC/UPS/other non-compute loads also reduced; this is subtracted so
    /// the result credits only the compute-derived component.
    pub non_compute_load_reduction_w: f64,
    /// Conservative absolute bounds, not standard deviations.
    pub site_measurement_uncertainty_bound_w: f64,
    pub non_compute_measurement_uncertainty_bound_w: f64,
    pub response_latency_ms: f64,
    pub sustained_delivery_duration_s: f64,
    /// Explicit recovery interval after this dispatch event.
    pub recovery_window_start_ns: u64,
    pub recovery_window_end_ns: u64,
    pub useful_work_retention_fraction: f64,
    pub p99_latency_increase_fraction: f64,
    pub p99_ttft_increase_fraction: f64,
    pub slo_violation_fraction: f64,
    /// Additional energy above the preregistered no-dispatch recovery baseline.
    pub recovery_rebound_energy_j: Option<f64>,
    pub measurement_contract_valid: bool,
}

impl GridDispatchEvent {
    pub fn duration_s(&self) -> f64 {
        (self.window_end_ns - self.window_start_ns) as f64 / 1_000_000_000.0
    }

    pub fn recovery_observation_duration_s(&self) -> f64 {
        (self.recovery_window_end_ns - self.recovery_window_start_ns) as f64
            / 1_000_000_000.0
    }

    fn validate(&self, contract: &GridServiceContract) -> Result<(), AuditError> {
        for (value, field) in [
            (self.event_id.as_str(), "event_id"),
            (self.evidence_id.as_str(), "evidence_id"),
            (self.resource_id.as_str(), "resource_id"),
            (self.grid_zone_id.as_str(), "grid_zone_id"),
            (self.point_of_interconnection_id.as_str(), "point_of_interconnection_id"),
            (self.site_operating_envelope_id.as_str(), "site_operating_envelope_id"),
            (self.workload_contract_id.as_str(), "workload_contract_id"),
            (self.request_mix_id.as_str(), "request_mix_id"),
            (self.quality_contract_id.as_str(), "quality_contract_id"),
            (self.boundary_id.as_str(), "boundary_id"),
            (self.control_action_id.as_str(), "control_action_id"),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!("dispatch event {field} must not be blank")));
            }
        }

        if self.resource_id != contract.resource_id
            || self.grid_zone_id != contract.grid_zone_id
            || self.point_of_interconnection_id != contract.point_of_interconnection_id
            || self.site_operating_envelope_id != contract.site_operating_envelope_id
            || self.workload_contract_id != contract.workload_contract_id
            || self.request_mix_id != contract.request_mix_id
            || self.quality_contract_id != contract.quality_contract_id
            || self.boundary_id != contract.boundary_id
            || self.control_action_id != contract.control_action_id
        {
            return Err(AuditError(format!(
                "dispatch event {:?} does not match resource, location, operating envelope, workload, quality, boundary or control-action contract",
                self.event_id
            )));
        }

        if self.window_end_ns <= self.window_start_ns {
            return Err(AuditError(format!(
                "dispatch event {:?} requires a positive time window",
                self.event_id
            )));
        }
        if self.recovery_window_start_ns < self.window_end_ns
            || self.recovery_window_end_ns <= self.recovery_window_start_ns
        {
            return Err(AuditError(format!(
                "dispatch event {:?} requires a positive recovery window starting after dispatch",
                self.event_id
            )));
        }
        if !self.measurement_contract_valid {
            return Err(AuditError(format!(
                "dispatch event {:?} does not satisfy its upstream measurement contract",
                self.event_id
            )));
        }

        let values = [
            self.site_baseline_power_w,
            self.site_event_power_w,
            self.non_compute_load_reduction_w,
            self.site_measurement_uncertainty_bound_w,
            self.non_compute_measurement_uncertainty_bound_w,
            self.response_latency_ms,
            self.sustained_delivery_duration_s,
            self.recovery_observation_duration_s(),
            self.useful_work_retention_fraction,
            self.p99_latency_increase_fraction,
            self.p99_ttft_increase_fraction,
            self.slo_violation_fraction,
        ];
        if values.iter().any(|value| !value.is_finite())
            || self.site_baseline_power_w <= 0.0
            || self.site_event_power_w < 0.0
            || self.site_measurement_uncertainty_bound_w < 0.0
            || self.non_compute_measurement_uncertainty_bound_w < 0.0
            || self.response_latency_ms < 0.0
            || self.sustained_delivery_duration_s <= 0.0
            || self.sustained_delivery_duration_s > self.duration_s()
            || self.useful_work_retention_fraction < 0.0
            || !(0.0..=1.0).contains(&self.slo_violation_fraction)
        {
            return Err(AuditError(format!(
                "dispatch event {:?} contains a non-finite or out-of-domain measurement",
                self.event_id
            )));
        }

        if self
            .recovery_rebound_energy_j
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(AuditError(format!(
                "dispatch event {:?} has invalid recovery rebound energy",
                self.event_id
            )));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridDispatchEventResult {
    pub event_id: String,
    pub site_power_reduction_w: f64,
    pub non_compute_load_reduction_w: f64,
    pub compute_attributed_reduction_before_uncertainty_w: f64,
    pub combined_uncertainty_bound_w: f64,
    /// One-sided conservative estimate after subtracting non-compute actions
    /// and both stated uncertainty bounds. It may be negative.
    pub conservative_compute_reduction_w: f64,
    /// Capacity credited only when all service, response, measurement,
    /// recovery and rebound gates pass; failed events receive zero.
    pub credited_deliverable_capacity_w: f64,
    pub measurement_uncertainty_fraction: f64,
    pub rebound_energy_fraction: Option<f64>,
    pub service_contract_pass: bool,
    pub commitment_power_pass: bool,
    pub event_success: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridServiceQualificationReport {
    pub resource_id: String,
    pub grid_zone_id: String,
    pub point_of_interconnection_id: String,
    pub service_class: GridServiceClass,
    pub baseline_method: CounterfactualBaselineMethod,
    pub total_event_count: usize,
    pub service_feasible_event_count: usize,
    pub commitment_success_event_count: usize,
    pub empirical_event_success_fraction: f64,
    pub wilson_lower_bound_success_fraction: Option<f64>,
    pub p10_credited_compute_capacity_w: Option<f64>,
    pub committed_compute_reduction_w: f64,
    pub meets_minimum_event_count: bool,
    pub meets_capacity_commitment: bool,
    pub meets_reliability_lower_bound: bool,
    pub independent_event_assumption_available: bool,
    pub commitment_qualified: bool,
    pub event_results: Vec<GridDispatchEventResult>,
    pub claim_ceiling: &'static str,
}

/// Wilson score lower bound for a one-sided confidence interval.
/// Its binomial interpretation assumes representative, independent event trials.
pub fn wilson_lower_bound(successes: usize, trials: usize, z_score: f64) -> Option<f64> {
    if trials == 0 || successes > trials || !z_score.is_finite() || z_score <= 0.0 {
        return None;
    }
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = z_score * z_score;
    let denominator = 1.0 + z2 / n;
    let center = p + z2 / (2.0 * n);
    let margin = z_score * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    Some(((center - margin) / denominator).clamp(0.0, 1.0))
}

fn nearest_rank_quantile(values: &mut [f64], quantile: f64) -> Option<f64> {
    if values.is_empty() || !quantile.is_finite() || !(0.0..=1.0).contains(&quantile) || quantile == 0.0 {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let rank = (quantile * values.len() as f64).ceil() as usize;
    values.get(rank.saturating_sub(1)).copied()
}

fn evaluate_event(
    event: &GridDispatchEvent,
    contract: &GridServiceContract,
) -> GridDispatchEventResult {
    let site_power_reduction_w = event.site_baseline_power_w - event.site_event_power_w;
    let compute_attributed_reduction_before_uncertainty_w =
        site_power_reduction_w - event.non_compute_load_reduction_w;
    let combined_uncertainty_bound_w = event.site_measurement_uncertainty_bound_w
        + event.non_compute_measurement_uncertainty_bound_w;
    let conservative_compute_reduction_w =
        compute_attributed_reduction_before_uncertainty_w - combined_uncertainty_bound_w;
    let measurement_uncertainty_fraction =
        combined_uncertainty_bound_w / event.site_baseline_power_w;

    let rebound_energy_fraction = event.recovery_rebound_energy_j.and_then(|rebound| {
        (conservative_compute_reduction_w > 0.0).then_some(
            rebound / (conservative_compute_reduction_w * event.sustained_delivery_duration_s),
        )
    });

    let service_contract_pass = conservative_compute_reduction_w > 0.0
        && measurement_uncertainty_fraction
            <= contract.maximum_combined_uncertainty_fraction
        && event.response_latency_ms <= contract.maximum_response_latency_ms
        && event.sustained_delivery_duration_s >= contract.minimum_sustained_duration_s
        && event.recovery_observation_duration_s() >= contract.minimum_recovery_window_s
        && event.useful_work_retention_fraction
            >= contract.minimum_useful_work_retention_fraction
        && event.p99_latency_increase_fraction
            <= contract.maximum_p99_latency_increase_fraction
        && event.p99_ttft_increase_fraction
            <= contract.maximum_p99_ttft_increase_fraction
        && event.slo_violation_fraction <= contract.maximum_slo_violation_fraction
        && rebound_energy_fraction.is_some_and(|fraction| {
            fraction.is_finite() && fraction <= contract.maximum_rebound_energy_fraction
        });

    let credited_deliverable_capacity_w = if service_contract_pass {
        conservative_compute_reduction_w.max(0.0)
    } else {
        0.0
    };
    let commitment_power_pass =
        credited_deliverable_capacity_w >= contract.committed_compute_reduction_w;
    let event_success = service_contract_pass && commitment_power_pass;

    GridDispatchEventResult {
        event_id: event.event_id.clone(),
        site_power_reduction_w,
        non_compute_load_reduction_w: event.non_compute_load_reduction_w,
        compute_attributed_reduction_before_uncertainty_w,
        combined_uncertainty_bound_w,
        conservative_compute_reduction_w,
        credited_deliverable_capacity_w,
        measurement_uncertainty_fraction,
        rebound_energy_fraction,
        service_contract_pass,
        commitment_power_pass,
        event_success,
    }
}

/// Evaluate repeated compute-derived demand-response events against a single
/// preregistered counterfactual and a location-specific service contract.
///
/// This is an offline evaluator. It does not measure, dispatch, schedule, bid,
/// accredit or certify a resource.
pub fn evaluate_grid_service_sweep(
    contract: GridServiceContract,
    events: Vec<GridDispatchEvent>,
) -> Result<GridServiceQualificationReport, AuditError> {
    contract.validate()?;
    if events.is_empty() {
        return Err(AuditError(
            "grid-service sweep requires at least one dispatch event".to_string(),
        ));
    }

    let mut event_ids = HashSet::new();
    let mut ordered_events = events.clone();
    ordered_events.sort_by_key(|event| event.window_start_ns);
    let mut previous_recovery_end_ns = None;
    for event in &ordered_events {
        event.validate(&contract)?;
        if !event_ids.insert(event.event_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate dispatch event id {:?}",
                event.event_id
            )));
        }
        if previous_recovery_end_ns
            .is_some_and(|end| event.window_start_ns < end)
        {
            return Err(AuditError(
                "dispatch and recovery windows must not overlap for one resource".to_string(),
            ));
        }
        previous_recovery_end_ns = Some(event.recovery_window_end_ns);
    }

    let event_results: Vec<GridDispatchEventResult> = ordered_events
        .iter()
        .map(|event| evaluate_event(event, &contract))
        .collect();

    let service_feasible_event_count = event_results
        .iter()
        .filter(|result| result.service_contract_pass)
        .count();
    let commitment_success_event_count = event_results
        .iter()
        .filter(|result| result.event_success)
        .count();
    let total_event_count = event_results.len();
    let empirical_event_success_fraction =
        commitment_success_event_count as f64 / total_event_count as f64;

    // A failed service event has zero credited capacity. A service-feasible
    // event that misses the declared commitment retains its lower delivered
    // capacity in the distribution, while also counting as a failed event.
    let mut credited_capacities: Vec<f64> = event_results
        .iter()
        .map(|result| result.credited_deliverable_capacity_w)
        .collect();
    let p10_credited_compute_capacity_w =
        nearest_rank_quantile(&mut credited_capacities, 0.10);

    // Do not publish a binomial confidence bound when the event design has not
    // established the independent-trial assumption.
    let wilson_lower_bound_success_fraction = if contract.independent_event_assumption {
        wilson_lower_bound(
            commitment_success_event_count,
            total_event_count,
            contract.confidence_z_score,
        )
    } else {
        None
    };

    let meets_minimum_event_count = total_event_count >= contract.minimum_event_count;
    let meets_capacity_commitment = p10_credited_compute_capacity_w
        .is_some_and(|capacity| capacity >= contract.committed_compute_reduction_w);
    let meets_reliability_lower_bound = wilson_lower_bound_success_fraction
        .is_some_and(|lower_bound| lower_bound >= contract.minimum_success_probability);
    let commitment_qualified = meets_minimum_event_count
        && meets_capacity_commitment
        && meets_reliability_lower_bound;

    Ok(GridServiceQualificationReport {
        resource_id: contract.resource_id,
        grid_zone_id: contract.grid_zone_id,
        point_of_interconnection_id: contract.point_of_interconnection_id,
        service_class: contract.service_class,
        baseline_method: contract.baseline_method,
        total_event_count,
        service_feasible_event_count,
        commitment_success_event_count,
        empirical_event_success_fraction,
        wilson_lower_bound_success_fraction,
        p10_credited_compute_capacity_w,
        committed_compute_reduction_w: contract.committed_compute_reduction_w,
        meets_minimum_event_count,
        meets_capacity_commitment,
        meets_reliability_lower_bound,
        independent_event_assumption_available: contract.independent_event_assumption,
        commitment_qualified,
        event_results,
        claim_ceiling: "REPEATED_EVENT_OFFLINE_EVALUATION_NOT_MARKET_ACCREDITATION",
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_grid_service_sweep, wilson_lower_bound, CounterfactualBaselineMethod,
        GridDispatchEvent, GridServiceClass, GridServiceContract,
    };

    fn contract() -> GridServiceContract {
        GridServiceContract {
            resource_id: "dc-site-01".to_string(),
            grid_zone_id: "zone-west".to_string(),
            point_of_interconnection_id: "poi-11kv-01".to_string(),
            site_operating_envelope_id: "summer-thermal-envelope-v1".to_string(),
            workload_contract_id: "inference-service-v3".to_string(),
            request_mix_id: "frozen-request-mix-v7".to_string(),
            quality_contract_id: "accepted-output-quality-v2".to_string(),
            boundary_id: "site-pcc-01".to_string(),
            control_action_id: "defer-preemptible-work-v1".to_string(),
            counterfactual_baseline_id: "frozen-no-dispatch-baseline-v1".to_string(),
            counterfactual_baseline_evidence_id: "baseline-evidence-sha256".to_string(),
            baseline_validation_evidence_id: None,
            baseline_method: CounterfactualBaselineMethod::FrozenWorkloadReplay,
            baseline_preregistered: true,
            event_sampling_design_id: "stratified-dispatch-trials-v1".to_string(),
            independent_event_assumption: true,
            service_class: GridServiceClass::GridServing,
            committed_compute_reduction_w: 150.0,
            minimum_event_count: 30,
            minimum_success_probability: 0.90,
            confidence_z_score: 1.645,
            minimum_useful_work_retention_fraction: 0.99,
            maximum_p99_latency_increase_fraction: 0.05,
            maximum_p99_ttft_increase_fraction: 0.05,
            maximum_slo_violation_fraction: 0.01,
            maximum_rebound_energy_fraction: 0.10,
            maximum_response_latency_ms: 1_000.0,
            minimum_sustained_duration_s: 60.0,
            minimum_recovery_window_s: 300.0,
            maximum_combined_uncertainty_fraction: 0.05,
        }
    }

    fn event(index: u64) -> GridDispatchEvent {
        let start = 1_000_000_000 + index * 600_000_000_000;
        GridDispatchEvent {
            event_id: format!("dispatch-{index:03}"),
            evidence_id: format!("event-evidence-{index:03}"),
            resource_id: "dc-site-01".to_string(),
            grid_zone_id: "zone-west".to_string(),
            point_of_interconnection_id: "poi-11kv-01".to_string(),
            site_operating_envelope_id: "summer-thermal-envelope-v1".to_string(),
            workload_contract_id: "inference-service-v3".to_string(),
            request_mix_id: "frozen-request-mix-v7".to_string(),
            quality_contract_id: "accepted-output-quality-v2".to_string(),
            boundary_id: "site-pcc-01".to_string(),
            control_action_id: "defer-preemptible-work-v1".to_string(),
            window_start_ns: start,
            window_end_ns: start + 120_000_000_000,
            recovery_window_start_ns: start + 120_000_000_000,
            recovery_window_end_ns: start + 420_000_000_000,
            site_baseline_power_w: 1_000.0,
            site_event_power_w: 800.0,
            non_compute_load_reduction_w: 0.0,
            site_measurement_uncertainty_bound_w: 5.0,
            non_compute_measurement_uncertainty_bound_w: 5.0,
            response_latency_ms: 500.0,
            sustained_delivery_duration_s: 60.0,
            useful_work_retention_fraction: 0.995,
            p99_latency_increase_fraction: 0.03,
            p99_ttft_increase_fraction: 0.02,
            slo_violation_fraction: 0.005,
            recovery_rebound_energy_j: Some(500.0),
            measurement_contract_valid: true,
        }
    }

    fn all_events(count: u64) -> Vec<GridDispatchEvent> {
        (0..count).map(event).collect()
    }

    #[test]
    fn thirty_repeated_events_qualify_at_the_p10_capacity_not_the_best_event() {
        let report = evaluate_grid_service_sweep(contract(), all_events(30)).unwrap();
        assert_eq!(report.total_event_count, 30);
        assert_eq!(report.service_feasible_event_count, 30);
        assert_eq!(report.commitment_success_event_count, 30);
        assert!((report.p10_credited_compute_capacity_w.unwrap() - 190.0).abs() < 1e-12);
        assert!(report.wilson_lower_bound_success_fraction.unwrap() >= 0.90);
        assert!(report.commitment_qualified);
        assert_eq!(
            report.claim_ceiling,
            "REPEATED_EVENT_OFFLINE_EVALUATION_NOT_MARKET_ACCREDITATION"
        );
    }

    #[test]
    fn single_high_performance_event_cannot_qualify_a_resource() {
        let report = evaluate_grid_service_sweep(contract(), vec![event(0)]).unwrap();
        assert_eq!(report.total_event_count, 1);
        assert!(!report.meets_minimum_event_count);
        assert!(!report.commitment_qualified);
    }

    #[test]
    fn one_failed_commitment_event_reduces_the_reliability_lower_bound() {
        let mut events = all_events(30);
        events[0].site_event_power_w = 900.0;
        let report = evaluate_grid_service_sweep(contract(), events).unwrap();
        assert_eq!(report.commitment_success_event_count, 29);
        assert!(report.p10_credited_compute_capacity_w.unwrap() >= 150.0);
        assert!(report.wilson_lower_bound_success_fraction.unwrap() < 0.90);
        assert!(!report.commitment_qualified);
    }

    #[test]
    fn non_compute_load_reduction_is_subtracted_before_crediting_compute() {
        let mut events = all_events(30);
        for item in &mut events {
            item.non_compute_load_reduction_w = 40.0;
        }
        let report = evaluate_grid_service_sweep(contract(), events).unwrap();
        assert!((report.p10_credited_compute_capacity_w.unwrap() - 150.0).abs() < 1e-12);
        assert!(report.commitment_qualified);
    }

    #[test]
    fn measurement_uncertainty_is_subtracted_from_attributed_compute_reduction() {
        let mut events = all_events(30);
        for item in &mut events {
            item.site_measurement_uncertainty_bound_w = 30.0;
            item.non_compute_measurement_uncertainty_bound_w = 30.0;
        }
        let report = evaluate_grid_service_sweep(contract(), events).unwrap();
        assert_eq!(report.event_results[0].credited_deliverable_capacity_w, 0.0);
        assert!(!report.commitment_qualified);
    }

    #[test]
    fn missing_rebound_or_recovery_observation_fails_the_event() {
        let mut events = all_events(30);
        events[0].recovery_rebound_energy_j = None;
        let report = evaluate_grid_service_sweep(contract(), events).unwrap();
        assert_eq!(report.event_results[0].credited_deliverable_capacity_w, 0.0);
        assert_eq!(report.commitment_success_event_count, 29);
        assert!(!report.commitment_qualified);

        let mut events = all_events(30);
        events[0].recovery_window_end_ns = events[0].recovery_window_start_ns + 100_000_000_000;
        let report = evaluate_grid_service_sweep(contract(), events).unwrap();
        assert!(!report.event_results[0].service_contract_pass);
    }

    #[test]
    fn excessive_rebound_is_not_credited_as_dependable_capacity() {
        let mut events = all_events(30);
        events[0].recovery_rebound_energy_j = Some(10_000.0);
        let report = evaluate_grid_service_sweep(contract(), events).unwrap();
        assert_eq!(report.event_results[0].credited_deliverable_capacity_w, 0.0);
        assert_eq!(report.commitment_success_event_count, 29);
    }

    #[test]
    fn useful_work_qos_response_and_sustain_failures_zero_the_event() {
        let mut events = all_events(30);
        events[0].useful_work_retention_fraction = 0.98;
        events[1].p99_latency_increase_fraction = 0.20;
        events[2].p99_ttft_increase_fraction = 0.20;
        events[3].slo_violation_fraction = 0.05;
        events[4].response_latency_ms = 2_000.0;
        events[5].sustained_delivery_duration_s = 30.0;
        let report = evaluate_grid_service_sweep(contract(), events).unwrap();
        for index in 0..6 {
            assert_eq!(
                report.event_results[index].credited_deliverable_capacity_w,
                0.0
            );
        }
        assert_eq!(report.commitment_success_event_count, 24);
        assert!(!report.commitment_qualified);
    }

    #[test]
    fn post_hoc_or_unqualified_counterfactual_is_rejected() {
        let mut post_hoc = contract();
        post_hoc.baseline_preregistered = false;
        assert!(evaluate_grid_service_sweep(post_hoc, all_events(30)).is_err());

        let mut unqualified = contract();
        unqualified.baseline_method = CounterfactualBaselineMethod::Unqualified;
        assert!(evaluate_grid_service_sweep(unqualified, all_events(30)).is_err());
    }

    #[test]
    fn independent_model_requires_separate_validation_evidence() {
        let mut baseline = contract();
        baseline.baseline_method = CounterfactualBaselineMethod::IndependentValidatedModel;
        assert!(evaluate_grid_service_sweep(baseline.clone(), all_events(30)).is_err());

        baseline.baseline_validation_evidence_id =
            Some("independent-model-validation-evidence".to_string());
        assert!(evaluate_grid_service_sweep(baseline, all_events(30)).is_ok());
    }

    #[test]
    fn unproven_event_independence_withholds_wilson_bound_and_qualification() {
        let mut candidate = contract();
        candidate.independent_event_assumption = false;
        let report = evaluate_grid_service_sweep(candidate, all_events(30)).unwrap();
        assert!(report.wilson_lower_bound_success_fraction.is_none());
        assert!(!report.meets_reliability_lower_bound);
        assert!(!report.commitment_qualified);
    }

    #[test]
    fn rejects_changed_location_boundary_workload_or_control_action() {
        let mut events = all_events(30);
        events[0].grid_zone_id = "zone-east".to_string();
        assert!(evaluate_grid_service_sweep(contract(), events).is_err());

        let mut events = all_events(30);
        events[0].point_of_interconnection_id = "other-poi".to_string();
        assert!(evaluate_grid_service_sweep(contract(), events).is_err());

        let mut events = all_events(30);
        events[0].boundary_id = "other-boundary".to_string();
        assert!(evaluate_grid_service_sweep(contract(), events).is_err());

        let mut events = all_events(30);
        events[0].request_mix_id = "different-mix".to_string();
        assert!(evaluate_grid_service_sweep(contract(), events).is_err());

        let mut events = all_events(30);
        events[0].control_action_id = "different-action".to_string();
        assert!(evaluate_grid_service_sweep(contract(), events).is_err());
    }

    #[test]
    fn recovery_windows_must_follow_dispatch_and_not_overlap_later_events() {
        let mut events = all_events(30);
        events[0].recovery_window_start_ns = events[0].window_end_ns - 1;
        assert!(evaluate_grid_service_sweep(contract(), events).is_err());

        let mut events = all_events(30);
        events[0].recovery_window_end_ns = events[1].window_start_ns + 1;
        assert!(evaluate_grid_service_sweep(contract(), events).is_err());
    }

    #[test]
    fn rejects_overlapping_windows_duplicate_ids_and_unverified_measurement() {
        let mut events = all_events(30);
        events[1].window_start_ns = events[0].window_start_ns + 1;
        assert!(evaluate_grid_service_sweep(contract(), events).is_err());

        let mut events = all_events(30);
        events[1].event_id = events[0].event_id.clone();
        assert!(evaluate_grid_service_sweep(contract(), events).is_err());

        let mut events = all_events(30);
        events[0].measurement_contract_valid = false;
        assert!(evaluate_grid_service_sweep(contract(), events).is_err());
    }

    #[test]
    fn zero_or_negative_compute_reduction_receives_no_credit() {
        let mut events = all_events(30);
        for item in &mut events {
            item.site_event_power_w = 1_010.0;
        }
        let report = evaluate_grid_service_sweep(contract(), events).unwrap();
        assert_eq!(report.p10_credited_compute_capacity_w, Some(0.0));
        assert_eq!(report.commitment_success_event_count, 0);
        assert!(!report.commitment_qualified);
    }

    #[test]
    fn zero_maximum_slo_rebound_and_uncertainty_limits_are_valid() {
        let mut contract = contract();
        contract.maximum_slo_violation_fraction = 0.0;
        contract.maximum_rebound_energy_fraction = 0.0;
        contract.maximum_combined_uncertainty_fraction = 0.0;

        let mut events = all_events(30);
        for event in &mut events {
            event.slo_violation_fraction = 0.0;
            event.site_measurement_uncertainty_bound_w = 0.0;
            event.non_compute_measurement_uncertainty_bound_w = 0.0;
            event.recovery_rebound_energy_j = Some(0.0);
        }

        let report = evaluate_grid_service_sweep(contract, events).unwrap();
        assert!(report.commitment_qualified);
    }

    #[test]
    fn invalid_contract_thresholds_and_empty_sweep_are_rejected() {
        let mut bad = contract();
        bad.committed_compute_reduction_w = f64::NAN;
        assert!(evaluate_grid_service_sweep(bad, all_events(30)).is_err());
        assert!(evaluate_grid_service_sweep(contract(), vec![]).is_err());
    }

    #[test]
    fn wilson_bound_rejects_invalid_counts_and_is_below_observed_rate() {
        assert!(wilson_lower_bound(2, 1, 1.645).is_none());
        assert!(wilson_lower_bound(1, 1, f64::NAN).is_none());
        let lower = wilson_lower_bound(29, 30, 1.645).unwrap();
        assert!(lower < 29.0 / 30.0);
        assert!(lower > 0.0);
    }
}