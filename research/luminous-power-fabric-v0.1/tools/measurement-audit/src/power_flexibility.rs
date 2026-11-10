use crate::AuditError;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub struct FlexibilityRun {
    pub run_id: String,
    pub evidence_id: String,
    pub workload_contract_id: String,
    pub request_mix_id: String,
    pub quality_contract_id: String,
    pub boundary_id: String,
    pub window_start_ns: u64,
    pub window_end_ns: u64,
    pub initial_stored_energy_j: f64,
    pub final_stored_energy_j: f64,
    pub pcc_energy_j: f64,
    pub useful_work_units: f64,
    pub p99_latency_ms: f64,
    pub p99_time_to_first_token_ms: f64,
    pub slo_violation_fraction: f64,
    pub p99_absolute_ramp_w_per_s: f64,
    /// Time from action onset until the declared power target is first reached.
    pub response_latency_ms: f64,
    /// Maximum fractional excursion beyond the target during the response.
    pub power_overshoot_fraction: f64,
    /// Time to return inside the declared recovery band after action release.
    pub recovery_settling_ms: Option<f64>,
    /// Additional active energy measured during a separately declared
    /// recovery window after the flexibility action has been released.
    pub recovery_rebound_energy_j: Option<f64>,
    pub sustained_action_duration_s: f64,
    /// None for the uncontrolled baseline; non-empty for controlled runs.
    pub control_action_id: Option<String>,
    pub measurement_contract_valid: bool,
}

impl FlexibilityRun {
    fn validate(&self) -> Result<(), AuditError> {
        for (value, field) in [
            (self.run_id.as_str(), "run_id"),
            (self.evidence_id.as_str(), "evidence_id"),
            (self.workload_contract_id.as_str(), "workload_contract_id"),
            (self.request_mix_id.as_str(), "request_mix_id"),
            (self.quality_contract_id.as_str(), "quality_contract_id"),
            (self.boundary_id.as_str(), "boundary_id"),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!(
                    "flexibility run {field} must not be blank"
                )));
            }
        }
        if !self.measurement_contract_valid {
            return Err(AuditError(format!(
                "flexibility run {:?} does not satisfy its measurement contract",
                self.run_id
            )));
        }
        if self.window_end_ns <= self.window_start_ns {
            return Err(AuditError(
                "flexibility measurement window must have positive duration".to_string(),
            ));
        }

        let values = [
            self.initial_stored_energy_j,
            self.final_stored_energy_j,
            self.pcc_energy_j,
            self.useful_work_units,
            self.p99_latency_ms,
            self.p99_time_to_first_token_ms,
            self.slo_violation_fraction,
            self.p99_absolute_ramp_w_per_s,
            self.response_latency_ms,
            self.power_overshoot_fraction,
            self.sustained_action_duration_s,
        ];
        if values.iter().any(|value| !value.is_finite())
            || self.initial_stored_energy_j < 0.0
            || self.final_stored_energy_j < 0.0
            || self.pcc_energy_j <= 0.0
            || self.useful_work_units <= 0.0
            || self.p99_latency_ms <= 0.0
            || self.p99_time_to_first_token_ms <= 0.0
            || !(0.0..=1.0).contains(&self.slo_violation_fraction)
            || self.p99_absolute_ramp_w_per_s < 0.0
            || self.response_latency_ms < 0.0
            || self.power_overshoot_fraction < 0.0
            || self.sustained_action_duration_s < 0.0
        {
            return Err(AuditError(format!(
                "flexibility run {:?} contains non-finite or out-of-domain measurements",
                self.run_id
            )));
        }
        if self
            .recovery_settling_ms
            .is_some_and(|settling| !settling.is_finite() || settling < 0.0)
        {
            return Err(AuditError(
                "recovery settling time must be finite and non-negative when present".to_string(),
            ));
        }
        if let Some(rebound) = self.recovery_rebound_energy_j {
            if !rebound.is_finite() || rebound < 0.0 {
                return Err(AuditError(
                    "recovery rebound energy must be finite and non-negative".to_string(),
                ));
            }
        }
        if self
            .control_action_id
            .as_ref()
            .is_some_and(|action| action.trim().is_empty())
        {
            return Err(AuditError(
                "control_action_id must be non-blank when present".to_string(),
            ));
        }

        let effective_input_energy_j = self.effective_input_energy_j();
        if !effective_input_energy_j.is_finite() || effective_input_energy_j <= 0.0 {
            return Err(AuditError(format!(
                "flexibility run {:?} has non-positive storage-adjusted input energy",
                self.run_id
            )));
        }
        Ok(())
    }

    pub fn duration_s(&self) -> f64 {
        (self.window_end_ns - self.window_start_ns) as f64 / 1_000_000_000.0
    }

    /// Derive mean active power from the integrated PCC energy and exact
    /// measurement duration. Do not accept a second independent mean-power
    /// input that could disagree with the energy record.
    pub fn mean_pcc_power_w(&self) -> f64 {
        self.pcc_energy_j / self.duration_s()
    }

    /// Energy corrected for the initial/final stored-energy change.
    pub fn effective_input_energy_j(&self) -> f64 {
        self.pcc_energy_j - (self.final_stored_energy_j - self.initial_stored_energy_j)
    }

    pub fn effective_energy_per_work_unit_j(&self) -> f64 {
        self.effective_input_energy_j() / self.useful_work_units
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlexibilityGate {
    pub minimum_power_shed_fraction: f64,
    pub maximum_useful_work_loss_fraction: f64,
    pub maximum_p99_latency_increase_fraction: f64,
    pub maximum_p99_ttft_increase_fraction: f64,
    pub maximum_slo_violation_fraction: f64,
    pub maximum_energy_per_work_regression_fraction: f64,
    pub maximum_rebound_energy_fraction: f64,
    pub maximum_response_latency_ms: f64,
    pub maximum_power_overshoot_fraction: f64,
    pub maximum_recovery_settling_ms: f64,
    pub minimum_sustained_action_duration_s: f64,
}

impl FlexibilityGate {
    fn validate(self) -> Result<(), AuditError> {
        let fractions = [
            self.minimum_power_shed_fraction,
            self.maximum_useful_work_loss_fraction,
            self.maximum_p99_latency_increase_fraction,
            self.maximum_p99_ttft_increase_fraction,
            self.maximum_slo_violation_fraction,
            self.maximum_energy_per_work_regression_fraction,
            self.maximum_rebound_energy_fraction,
            self.maximum_power_overshoot_fraction,
        ];
        if fractions.iter().any(|value| !value.is_finite() || *value < 0.0 || *value >= 1.0)
            || !self.maximum_response_latency_ms.is_finite()
            || self.maximum_response_latency_ms <= 0.0
            || !self.maximum_recovery_settling_ms.is_finite()
            || self.maximum_recovery_settling_ms <= 0.0
            || !self.minimum_sustained_action_duration_s.is_finite()
            || self.minimum_sustained_action_duration_s <= 0.0
        {
            return Err(AuditError(
                "flexibility gate values must be finite and within their valid domains"
                    .to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlexibilityCandidateResult {
    pub run_id: String,
    pub control_action_id: String,
    pub power_shed_fraction: f64,
    pub useful_work_retention_fraction: f64,
    pub p99_latency_increase_fraction: f64,
    pub p99_ttft_increase_fraction: f64,
    pub slo_violation_fraction: f64,
    pub energy_per_work_change_fraction: f64,
    pub p99_ramp_reduction_fraction: Option<f64>,
    pub response_latency_ms: f64,
    pub power_overshoot_fraction: f64,
    pub recovery_settling_ms: Option<f64>,
    pub effective_energy_saved_j: f64,
    pub rebound_energy_fraction: Option<f64>,
    pub sustained_action_duration_s: f64,
    pub meets_service_contract: bool,
    pub meets_power_shed_gate: bool,
    pub pareto_frontier_member: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerFlexibilitySweepReport {
    pub baseline_run_id: String,
    pub workload_contract_id: String,
    pub request_mix_id: String,
    pub duration_s: f64,
    pub candidate_results: Vec<FlexibilityCandidateResult>,
    /// Feasible, non-dominated controlled operating points.
    pub pareto_frontier_run_ids: Vec<String>,
    pub claim_ceiling: &'static str,
}

fn validate_matched_runs(
    baseline: &FlexibilityRun,
    candidate: &FlexibilityRun,
) -> Result<(), AuditError> {
    if baseline.workload_contract_id != candidate.workload_contract_id
        || baseline.request_mix_id != candidate.request_mix_id
        || baseline.quality_contract_id != candidate.quality_contract_id
        || baseline.boundary_id != candidate.boundary_id
        || baseline.window_start_ns != candidate.window_start_ns
        || baseline.window_end_ns != candidate.window_end_ns
        || baseline.initial_stored_energy_j.to_bits()
            != candidate.initial_stored_energy_j.to_bits()
    {
        return Err(AuditError(format!(
            "candidate {:?} does not match baseline workload, request mix, quality contract, boundary, time window or initial stored energy",
            candidate.run_id
        )));
    }
    Ok(())
}

fn candidate_result(
    baseline: &FlexibilityRun,
    candidate: &FlexibilityRun,
    gate: FlexibilityGate,
) -> Result<FlexibilityCandidateResult, AuditError> {
    validate_matched_runs(baseline, candidate)?;
    let action_id = candidate
        .control_action_id
        .as_ref()
        .filter(|action| !action.trim().is_empty())
        .ok_or_else(|| {
            AuditError(format!(
                "controlled flexibility run {:?} must declare control_action_id",
                candidate.run_id
            ))
        })?;

    let power_shed_fraction =
        1.0 - candidate.mean_pcc_power_w() / baseline.mean_pcc_power_w();
    let useful_work_retention_fraction =
        candidate.useful_work_units / baseline.useful_work_units;
    let p99_latency_increase_fraction =
        candidate.p99_latency_ms / baseline.p99_latency_ms - 1.0;
    let p99_ttft_increase_fraction =
        candidate.p99_time_to_first_token_ms / baseline.p99_time_to_first_token_ms - 1.0;
    let energy_per_work_change_fraction =
        candidate.effective_energy_per_work_unit_j()
            / baseline.effective_energy_per_work_unit_j()
            - 1.0;
    let effective_energy_saved_j =
        baseline.effective_input_energy_j() - candidate.effective_input_energy_j();
    let p99_ramp_reduction_fraction = if baseline.p99_absolute_ramp_w_per_s > 0.0 {
        Some(
            1.0 - candidate.p99_absolute_ramp_w_per_s
                / baseline.p99_absolute_ramp_w_per_s,
        )
    } else {
        None
    };
    let rebound_energy_fraction = candidate
        .recovery_rebound_energy_j
        .and_then(|rebound| {
            (effective_energy_saved_j > 0.0)
                .then_some(rebound / effective_energy_saved_j)
        });

    let derived = [
        power_shed_fraction,
        useful_work_retention_fraction,
        p99_latency_increase_fraction,
        p99_ttft_increase_fraction,
        energy_per_work_change_fraction,
        effective_energy_saved_j,
        candidate.response_latency_ms,
        candidate.power_overshoot_fraction,
    ];
    if derived.iter().any(|value| !value.is_finite())
        || p99_ramp_reduction_fraction.is_some_and(|value| !value.is_finite())
        || candidate.recovery_settling_ms.is_some_and(|value| !value.is_finite())
        || rebound_energy_fraction.is_some_and(|value| !value.is_finite())
    {
        return Err(AuditError(
            "derived power-flexibility metric is non-finite".to_string(),
        ));
    }

    let meets_service_contract = useful_work_retention_fraction
        >= 1.0 - gate.maximum_useful_work_loss_fraction
        && p99_latency_increase_fraction <= gate.maximum_p99_latency_increase_fraction
        && p99_ttft_increase_fraction <= gate.maximum_p99_ttft_increase_fraction
        && candidate.slo_violation_fraction <= gate.maximum_slo_violation_fraction
        && energy_per_work_change_fraction
            <= gate.maximum_energy_per_work_regression_fraction
        && candidate.sustained_action_duration_s >= gate.minimum_sustained_action_duration_s
        && candidate.response_latency_ms <= gate.maximum_response_latency_ms
        && candidate.power_overshoot_fraction <= gate.maximum_power_overshoot_fraction
        && candidate
            .recovery_settling_ms
            .is_some_and(|settling| settling <= gate.maximum_recovery_settling_ms)
        && rebound_energy_fraction
            .is_some_and(|fraction| fraction <= gate.maximum_rebound_energy_fraction);

    let meets_power_shed_gate = power_shed_fraction >= gate.minimum_power_shed_fraction;

    Ok(FlexibilityCandidateResult {
        run_id: candidate.run_id.clone(),
        control_action_id: action_id.clone(),
        power_shed_fraction,
        useful_work_retention_fraction,
        p99_latency_increase_fraction,
        p99_ttft_increase_fraction,
        slo_violation_fraction: candidate.slo_violation_fraction,
        energy_per_work_change_fraction,
        p99_ramp_reduction_fraction,
        response_latency_ms: candidate.response_latency_ms,
        power_overshoot_fraction: candidate.power_overshoot_fraction,
        recovery_settling_ms: candidate.recovery_settling_ms,
        effective_energy_saved_j,
        rebound_energy_fraction,
        sustained_action_duration_s: candidate.sustained_action_duration_s,
        meets_service_contract,
        meets_power_shed_gate,
        pareto_frontier_member: false,
    })
}

fn dominates(left: &FlexibilityCandidateResult, right: &FlexibilityCandidateResult) -> bool {
    // Maximize power shed, work retention and ramp reduction. Minimize p99
    // latency/TTFT, energy/work penalty, response latency, overshoot,
    // recovery settling and rebound energy.
    let mut no_worse = left.power_shed_fraction >= right.power_shed_fraction
        && left.useful_work_retention_fraction >= right.useful_work_retention_fraction
        && left.p99_latency_increase_fraction <= right.p99_latency_increase_fraction
        && left.p99_ttft_increase_fraction <= right.p99_ttft_increase_fraction
        && left.energy_per_work_change_fraction <= right.energy_per_work_change_fraction
        && left.response_latency_ms <= right.response_latency_ms
        && left.power_overshoot_fraction <= right.power_overshoot_fraction;

    let mut strictly_better = left.power_shed_fraction > right.power_shed_fraction
        || left.useful_work_retention_fraction > right.useful_work_retention_fraction
        || left.p99_latency_increase_fraction < right.p99_latency_increase_fraction
        || left.p99_ttft_increase_fraction < right.p99_ttft_increase_fraction
        || left.energy_per_work_change_fraction < right.energy_per_work_change_fraction
        || left.response_latency_ms < right.response_latency_ms
        || left.power_overshoot_fraction < right.power_overshoot_fraction;

    match (
        left.p99_ramp_reduction_fraction,
        right.p99_ramp_reduction_fraction,
    ) {
        (Some(a), Some(b)) => {
            no_worse &= a >= b;
            strictly_better |= a > b;
        }
        (None, None) => {}
        _ => no_worse = false,
    }
    match (left.rebound_energy_fraction, right.rebound_energy_fraction) {
        (Some(a), Some(b)) => {
            no_worse &= a <= b;
            strictly_better |= a < b;
        }
        (None, None) => {}
        _ => no_worse = false,
    }
    match (left.recovery_settling_ms, right.recovery_settling_ms) {
        (Some(a), Some(b)) => {
            no_worse &= a <= b;
            strictly_better |= a < b;
        }
        (None, None) => {}
        _ => no_worse = false,
    }

    no_worse && strictly_better
}

/// Compare controlled power-flexibility operating points against one frozen
/// baseline. This evaluator is read-only. It does not send power-cap, DVFS,
/// workload scheduling or grid-dispatch commands.
///
/// Every run must share the same workload/request mix, quality contract,
/// electrical boundary, measurement time window and initial stored energy.
/// Storage depletion is charged to effective input energy. Candidates are
/// admitted to the Pareto frontier only when the declared useful-work, QoS,
/// energy/work, duration, rebound and minimum-power-shed gates are met.
pub fn evaluate_power_flexibility_sweep(
    baseline: FlexibilityRun,
    candidates: Vec<FlexibilityRun>,
    gate: FlexibilityGate,
) -> Result<PowerFlexibilitySweepReport, AuditError> {
    gate.validate()?;
    baseline.validate()?;
    if baseline.control_action_id.is_some() {
        return Err(AuditError(
            "baseline run must not declare a control action".to_string(),
        ));
    }
    if candidates.is_empty() {
        return Err(AuditError(
            "power-flexibility sweep requires at least one controlled candidate".to_string(),
        ));
    }

    let mut seen_ids = HashSet::new();
    if !seen_ids.insert(baseline.run_id.as_str()) {
        return Err(AuditError("duplicate run ID".to_string()));
    }
    let mut results = Vec::with_capacity(candidates.len());
    for candidate in &candidates {
        candidate.validate()?;
        if !seen_ids.insert(candidate.run_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate flexibility run ID {:?}",
                candidate.run_id
            )));
        }
        results.push(candidate_result(&baseline, candidate, gate)?);
    }

    let eligible_indices: Vec<usize> = results
        .iter()
        .enumerate()
        .filter_map(|(index, result)| {
            (result.meets_service_contract
                && result.meets_power_shed_gate
                && result.power_shed_fraction > 0.0)
                .then_some(index)
        })
        .collect();

    let mut frontier_ids = Vec::new();
    for &index in &eligible_indices {
        let is_dominated = eligible_indices.iter().copied().any(|other| {
            other != index && dominates(&results[other], &results[index])
        });
        if !is_dominated {
            results[index].pareto_frontier_member = true;
            frontier_ids.push(results[index].run_id.clone());
        }
    }
    frontier_ids.sort();

    Ok(PowerFlexibilitySweepReport {
        baseline_run_id: baseline.run_id,
        workload_contract_id: baseline.workload_contract_id,
        request_mix_id: baseline.request_mix_id,
        duration_s: baseline.duration_s(),
        candidate_results: results,
        pareto_frontier_run_ids: frontier_ids,
        claim_ceiling: "MATCHED_TRACE_POWER_FLEXIBILITY_FRONTIER_ONLY_NOT_LIVE_CONTROL_NOT_GRID_SERVICE_QUALIFICATION",
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_power_flexibility_sweep, FlexibilityGate, FlexibilityRun,
    };

    fn run(id: &str, action: Option<&str>, mean_power: f64, useful_work: f64) -> FlexibilityRun {
        FlexibilityRun {
            run_id: id.to_string(),
            evidence_id: format!("evidence-{id}"),
            workload_contract_id: "llm-inference-model-x-bf16".to_string(),
            request_mix_id: "mix-frozen-01".to_string(),
            quality_contract_id: "quality-contract-01".to_string(),
            boundary_id: "rack-pcc-01".to_string(),
            window_start_ns: 0,
            window_end_ns: 60_000_000_000,
            initial_stored_energy_j: 100_000.0,
            final_stored_energy_j: 100_000.0,
            pcc_energy_j: mean_power * 60.0,
            useful_work_units: useful_work,
            p99_latency_ms: 1000.0,
            p99_time_to_first_token_ms: 150.0,
            slo_violation_fraction: 0.001,
            p99_absolute_ramp_w_per_s: 1000.0,
            response_latency_ms: if action.is_some() { 100.0 } else { 0.0 },
            power_overshoot_fraction: 0.0,
            recovery_settling_ms: Some(100.0),
            recovery_rebound_energy_j: Some(0.0),
            sustained_action_duration_s: if action.is_some() { 60.0 } else { 0.0 },
            control_action_id: action.map(str::to_string),
            measurement_contract_valid: true,
        }
    }

    fn gate() -> FlexibilityGate {
        FlexibilityGate {
            minimum_power_shed_fraction: 0.05,
            maximum_useful_work_loss_fraction: 0.01,
            maximum_p99_latency_increase_fraction: 0.05,
            maximum_p99_ttft_increase_fraction: 0.05,
            maximum_slo_violation_fraction: 0.01,
            maximum_energy_per_work_regression_fraction: 0.0,
            maximum_rebound_energy_fraction: 0.10,
            maximum_response_latency_ms: 1000.0,
            maximum_power_overshoot_fraction: 0.02,
            maximum_recovery_settling_ms: 5000.0,
            minimum_sustained_action_duration_s: 60.0,
        }
    }

    #[test]
    fn reports_power_shed_with_preserved_work_and_latency() {
        let baseline = run("baseline", None, 1000.0, 600.0);
        let candidate = run("cap-950w", Some("gpu-cap-950w"), 950.0, 600.0);

        let report =
            evaluate_power_flexibility_sweep(baseline, vec![candidate], gate()).unwrap();
        let result = &report.candidate_results[0];

        assert!((result.power_shed_fraction - 0.05).abs() < 1.0e-12);
        assert!((result.effective_energy_saved_j / 60.0 - 50.0).abs() < 1.0e-12);
        assert!((result.useful_work_retention_fraction - 1.0).abs() < 1.0e-12);
        assert!(result.meets_service_contract);
        assert!(result.meets_power_shed_gate);
        assert!(result.pareto_frontier_member);
        assert_eq!(report.pareto_frontier_run_ids, vec!["cap-950w".to_string()]);
    }

    #[test]
    fn smoother_lower_power_candidate_fails_when_useful_work_is_lost() {
        let baseline = run("baseline", None, 1000.0, 600.0);
        let mut candidate = run("underperforming-cap", Some("cap-700w"), 700.0, 500.0);
        candidate.p99_absolute_ramp_w_per_s = 100.0;

        let report =
            evaluate_power_flexibility_sweep(baseline, vec![candidate], gate()).unwrap();
        let result = &report.candidate_results[0];

        assert!(!result.meets_service_contract);
        assert!(!result.pareto_frontier_member);
        assert!(report.pareto_frontier_run_ids.is_empty());
    }

    #[test]
    fn excessive_p99_latency_or_ttft_increase_fails_service_contract() {
        let baseline = run("baseline", None, 1000.0, 600.0);
        let mut candidate = run("slow-cap", Some("cap-slow"), 900.0, 600.0);
        candidate.p99_latency_ms = 1100.0;

        let report =
            evaluate_power_flexibility_sweep(baseline.clone(), vec![candidate], gate()).unwrap();
        assert!(!report.candidate_results[0].meets_service_contract);

        let mut ttft = run("ttft-cap", Some("cap-ttft"), 900.0, 600.0);
        ttft.p99_time_to_first_token_ms = 160.0;
        let report = evaluate_power_flexibility_sweep(baseline, vec![ttft], gate()).unwrap();
        assert!(!report.candidate_results[0].meets_service_contract);
    }

    #[test]
    fn storage_depletion_is_charged_to_energy_per_work() {
        let baseline = run("baseline", None, 1000.0, 600.0);
        let mut candidate = run("battery-depleted", Some("cap-with-battery"), 950.0, 600.0);
        candidate.final_stored_energy_j = 96_000.0;

        let report =
            evaluate_power_flexibility_sweep(baseline, vec![candidate], gate()).unwrap();
        let result = &report.candidate_results[0];

        assert!(result.effective_energy_saved_j < 0.0);
        assert!(result.energy_per_work_change_fraction > 0.0);
        assert!(!result.meets_service_contract);
    }

    #[test]
    fn rebound_energy_is_bounded_relative_to_storage_adjusted_energy_savings() {
        let baseline = run("baseline", None, 1000.0, 600.0);
        let mut candidate = run("rebound-cap", Some("cap-rebound"), 900.0, 600.0);
        candidate.recovery_rebound_energy_j = Some(700.0);

        let report =
            evaluate_power_flexibility_sweep(baseline, vec![candidate], gate()).unwrap();
        let result = &report.candidate_results[0];

        assert!((result.rebound_energy_fraction.unwrap() - (700.0 / 6000.0)).abs() < 1.0e-12);
        assert!(!result.meets_service_contract);
    }

    #[test]
    fn pareto_frontier_keeps_nondominated_service_feasible_options() {
        let baseline = run("baseline", None, 1000.0, 600.0);
        let mut mild = run("mild", Some("cap-950"), 950.0, 600.0);
        mild.p99_absolute_ramp_w_per_s = 700.0;
        let mut strong = run("strong", Some("cap-900"), 900.0, 600.0);
        strong.p99_absolute_ramp_w_per_s = 800.0;
        let mut dominated = run("dominated", Some("cap-940"), 940.0, 600.0);
        dominated.p99_absolute_ramp_w_per_s = 900.0;

        let report =
            evaluate_power_flexibility_sweep(baseline, vec![mild, strong, dominated], gate())
                .unwrap();

        assert!(report.pareto_frontier_run_ids.contains(&"mild".to_string()));
        assert!(report.pareto_frontier_run_ids.contains(&"strong".to_string()));
        assert!(!report.pareto_frontier_run_ids.contains(&"dominated".to_string()));
    }

    #[test]
    fn rejects_changed_workload_boundary_window_and_initial_storage() {
        let baseline = run("baseline", None, 1000.0, 600.0);
        let mut candidate = run("mismatch", Some("cap"), 900.0, 600.0);
        candidate.request_mix_id = "different-mix".to_string();
        assert!(evaluate_power_flexibility_sweep(
            baseline.clone(),
            vec![candidate],
            gate()
        )
        .is_err());

        let mut boundary = run("boundary", Some("cap"), 900.0, 600.0);
        boundary.boundary_id = "different-pcc".to_string();
        assert!(evaluate_power_flexibility_sweep(
            baseline.clone(),
            vec![boundary],
            gate()
        )
        .is_err());

        let mut changed_start = run("soc", Some("cap"), 900.0, 600.0);
        changed_start.initial_stored_energy_j += 1.0;
        assert!(evaluate_power_flexibility_sweep(
            baseline,
            vec![changed_start],
            gate()
        )
        .is_err());
    }

    #[test]
    fn unknown_rebound_measurement_cannot_pass_the_service_contract() {
        let baseline = run("baseline", None, 1000.0, 600.0);
        let mut candidate = run("unknown-rebound", Some("cap"), 900.0, 600.0);
        candidate.recovery_rebound_energy_j = None;

        let report =
            evaluate_power_flexibility_sweep(baseline, vec![candidate], gate()).unwrap();
        assert!(!report.candidate_results[0].meets_service_contract);
    }

    #[test]
    fn rejects_unverified_measurement_contract_and_duplicate_runs() {
        let baseline = run("baseline", None, 1000.0, 600.0);
        let mut bad = run("bad", Some("cap"), 900.0, 600.0);
        bad.measurement_contract_valid = false;
        assert!(evaluate_power_flexibility_sweep(
            baseline.clone(),
            vec![bad],
            gate()
        )
        .is_err());

        let duplicate = run("baseline", Some("cap"), 900.0, 600.0);
        assert!(evaluate_power_flexibility_sweep(baseline, vec![duplicate], gate()).is_err());
    }

    #[test]
    fn no_candidate_can_enter_frontier_without_power_shed_gate() {
        let baseline = run("baseline", None, 1000.0, 600.0);
        let candidate = run("no-shed", Some("no-op"), 1000.0, 600.0);

        let report =
            evaluate_power_flexibility_sweep(baseline, vec![candidate], gate()).unwrap();
        assert!(!report.candidate_results[0].meets_power_shed_gate);
        assert!(report.pareto_frontier_run_ids.is_empty());
    }

    #[test]
    fn excessive_response_latency_overshoot_or_recovery_settling_fails() {
        let baseline = run("baseline", None, 1000.0, 600.0);
        let mut delayed = run("delayed", Some("cap-delay"), 900.0, 600.0);
        delayed.response_latency_ms = 1200.0;
        let report =
            evaluate_power_flexibility_sweep(baseline.clone(), vec![delayed], gate()).unwrap();
        assert!(!report.candidate_results[0].meets_service_contract);

        let mut overshoot = run("overshoot", Some("cap-overshoot"), 900.0, 600.0);
        overshoot.power_overshoot_fraction = 0.03;
        let report =
            evaluate_power_flexibility_sweep(baseline.clone(), vec![overshoot], gate()).unwrap();
        assert!(!report.candidate_results[0].meets_service_contract);

        let mut slow_recovery = run("slow-recovery", Some("cap-recovery"), 900.0, 600.0);
        slow_recovery.recovery_settling_ms = Some(6000.0);
        let report =
            evaluate_power_flexibility_sweep(baseline, vec![slow_recovery], gate()).unwrap();
        assert!(!report.candidate_results[0].meets_service_contract);
    }

    #[test]
    fn rejects_invalid_gate_values_and_empty_sweeps() {
        let baseline = run("baseline", None, 1000.0, 600.0);
        let invalid = FlexibilityGate {
            maximum_useful_work_loss_fraction: 1.0,
            ..gate()
        };
        assert!(evaluate_power_flexibility_sweep(baseline.clone(), vec![], invalid).is_err());
        assert!(evaluate_power_flexibility_sweep(baseline, vec![], gate()).is_err());
    }
}