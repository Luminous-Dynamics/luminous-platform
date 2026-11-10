use crate::AuditError;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetOverheadEvidence {
    Measured,
    Modelled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FleetOperatingPoint {
    pub run_id: String,
    pub evidence_id: String,
    pub workload_contract_id: String,
    pub request_mix_id: String,
    pub quality_contract_id: String,
    pub accelerator_sku: String,
    pub software_config_id: String,
    pub boundary_id: String,
    pub window_start_ns: u64,
    pub window_end_ns: u64,
    /// Offered workload rate in the declared work_unit.
    pub offered_work_rate_per_s: f64,
    pub work_unit: String,
    /// Required spare capacity above offered demand, e.g. 0.10 for 10%.
    pub capacity_headroom_fraction: f64,
    /// Installed count in the matched fixed-fleet experiment.
    pub installed_gpu_count: u64,
    /// Measured single-GPU goodput at this operating point.
    pub per_gpu_goodput_units_per_s: f64,
    /// Actually completed useful work during the fixed-fleet experiment.
    pub completed_work_units: f64,
    /// Measured GPU-board power for one accelerator at this operating point.
    pub per_gpu_board_power_w: f64,
    /// Server/rack support power per installed GPU, when known.
    pub per_gpu_support_power_w: Option<f64>,
    /// Fixed facility/rack overhead for this operating point, when known.
    /// The evaluator never scales this value automatically with GPU count.
    pub fixed_fleet_overhead_power_w: Option<f64>,
    /// Fixed overhead at the capacity-scaled GPU count. This is supplied
    /// separately; the evaluator never assumes fixed overhead is invariant.
    pub fixed_capacity_scaled_overhead_power_w: Option<f64>,
    pub overhead_evidence: FleetOverheadEvidence,
    pub initial_stored_energy_j: f64,
    pub final_stored_energy_j: f64,
    pub p99_latency_ms: f64,
    pub p99_time_to_first_token_ms: f64,
    pub p99_latency_slo_ms: f64,
    pub p99_ttft_slo_ms: f64,
    pub slo_violation_fraction: f64,
    pub maximum_slo_violation_fraction: f64,
    /// Optional verified cost per installed GPU in one declared currency.
    pub capex_per_gpu: Option<f64>,
    pub capex_currency: Option<String>,
    pub capex_source_reference: Option<String>,
    /// None for the unmodified baseline; non-blank for controlled candidates.
    pub control_action_id: Option<String>,
    pub measurement_contract_valid: bool,
}

impl FleetOperatingPoint {
    pub fn duration_s(&self) -> f64 {
        (self.window_end_ns - self.window_start_ns) as f64 / 1_000_000_000.0
    }

    pub fn offered_work_units(&self) -> f64 {
        self.offered_work_rate_per_s * self.duration_s()
    }

    pub fn required_capacity_units_per_s(&self) -> f64 {
        self.offered_work_rate_per_s * (1.0 + self.capacity_headroom_fraction)
    }

    fn validate(&self) -> Result<(), AuditError> {
        for (value, field) in [
            (self.run_id.as_str(), "run_id"),
            (self.evidence_id.as_str(), "evidence_id"),
            (self.workload_contract_id.as_str(), "workload_contract_id"),
            (self.request_mix_id.as_str(), "request_mix_id"),
            (self.quality_contract_id.as_str(), "quality_contract_id"),
            (self.accelerator_sku.as_str(), "accelerator_sku"),
            (self.software_config_id.as_str(), "software_config_id"),
            (self.boundary_id.as_str(), "boundary_id"),
            (self.work_unit.as_str(), "work_unit"),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!(
                    "fleet operating point {field} must not be blank"
                )));
            }
        }
        if !self.measurement_contract_valid {
            return Err(AuditError(format!(
                "fleet operating point {:?} does not satisfy its upstream measurement contract",
                self.run_id
            )));
        }
        if self.window_end_ns <= self.window_start_ns || self.installed_gpu_count == 0 {
            return Err(AuditError(
                "fleet operating point requires a positive window and non-zero integer GPU count"
                    .to_string(),
            ));
        }

        let values = [
            self.offered_work_rate_per_s,
            self.capacity_headroom_fraction,
            self.per_gpu_goodput_units_per_s,
            self.completed_work_units,
            self.per_gpu_board_power_w,
            self.initial_stored_energy_j,
            self.final_stored_energy_j,
            self.p99_latency_ms,
            self.p99_time_to_first_token_ms,
            self.p99_latency_slo_ms,
            self.p99_ttft_slo_ms,
            self.slo_violation_fraction,
            self.maximum_slo_violation_fraction,
        ];
        if values.iter().any(|value| !value.is_finite())
            || self.offered_work_rate_per_s <= 0.0
            || !(0.0..1.0).contains(&self.capacity_headroom_fraction)
            || self.per_gpu_goodput_units_per_s <= 0.0
            || self.completed_work_units < 0.0
            || self.per_gpu_board_power_w <= 0.0
            || self.initial_stored_energy_j < 0.0
            || self.final_stored_energy_j < 0.0
            || self.p99_latency_ms <= 0.0
            || self.p99_time_to_first_token_ms <= 0.0
            || self.p99_latency_slo_ms <= 0.0
            || self.p99_ttft_slo_ms <= 0.0
            || !(0.0..=1.0).contains(&self.slo_violation_fraction)
            || !(0.0..=1.0).contains(&self.maximum_slo_violation_fraction)
        {
            return Err(AuditError(format!(
                "fleet operating point {:?} contains non-finite or out-of-domain measurements",
                self.run_id
            )));
        }

        match self.overhead_evidence {
            FleetOverheadEvidence::Unknown => {
                if self.per_gpu_support_power_w.is_some()
                    || self.fixed_fleet_overhead_power_w.is_some()
                    || self.fixed_capacity_scaled_overhead_power_w.is_some()
                {
                    return Err(AuditError(
                        "unknown overhead evidence cannot supply numeric overhead values"
                            .to_string(),
                    ));
                }
            }
            FleetOverheadEvidence::Measured | FleetOverheadEvidence::Modelled => {
                let support = self.per_gpu_support_power_w.ok_or_else(|| {
                    AuditError("declared overhead evidence requires per-GPU support power".to_string())
                })?;
                let fixed = self.fixed_fleet_overhead_power_w.ok_or_else(|| {
                    AuditError("declared overhead evidence requires fixed fleet overhead".to_string())
                })?;
                if !support.is_finite() || support < 0.0 || !fixed.is_finite() || fixed < 0.0 {
                    return Err(AuditError(
                        "support and fixed overhead power must be finite and non-negative"
                            .to_string(),
                    ));
                }
            }
        }

        match (
            self.capex_per_gpu,
            self.capex_currency.as_ref(),
            self.capex_source_reference.as_ref(),
        ) {
            (Some(cost), Some(currency), Some(source))
                if cost.is_finite()
                    && cost > 0.0
                    && currency.len() == 3
                    && currency.bytes().all(|byte| byte.is_ascii_uppercase())
                    && !source.trim().is_empty() => {}
            (None, None, None) => {}
            _ => {
                return Err(AuditError(
                    "GPU capex must be absent or include a positive amount, three-letter currency and source reference"
                        .to_string(),
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
        if !self.offered_work_units().is_finite()
            || !self.required_capacity_units_per_s().is_finite()
        {
            return Err(AuditError(
                "derived fleet demand or work total overflowed".to_string(),
            ));
        }
        Ok(())
    }

    fn total_power_w_for_gpu_count(
        &self,
        gpu_count: u64,
        is_capacity_scaled_model: bool,
    ) -> Option<f64> {
        let support = self.per_gpu_support_power_w?;
        let fixed = if is_capacity_scaled_model {
            self.fixed_capacity_scaled_overhead_power_w?
        } else {
            self.fixed_fleet_overhead_power_w?
        };
        let total = gpu_count as f64
            * (self.per_gpu_board_power_w + support)
            + fixed;
        total.is_finite().then_some(total)
    }

    fn board_power_w_for_gpu_count(&self, gpu_count: u64) -> f64 {
        gpu_count as f64 * self.per_gpu_board_power_w
    }

    fn effective_energy(total_power_w: f64, duration_s: f64, initial_j: f64, final_j: f64) -> f64 {
        total_power_w * duration_s - (final_j - initial_j)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FleetCapacityGate {
    pub maximum_useful_work_loss_fraction: f64,
    pub maximum_p99_latency_increase_fraction: f64,
    pub maximum_p99_ttft_increase_fraction: f64,
    pub maximum_slo_violation_fraction: f64,
}

impl FleetCapacityGate {
    fn validate(self) -> Result<(), AuditError> {
        let fractions = [
            self.maximum_useful_work_loss_fraction,
            self.maximum_p99_latency_increase_fraction,
            self.maximum_p99_ttft_increase_fraction,
            self.maximum_slo_violation_fraction,
        ];
        if fractions
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0 || *value >= 1.0)
        {
            return Err(AuditError(
                "fleet capacity gate fractions must be finite and in [0, 1)"
                    .to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FleetScenarioResult {
    pub installed_gpu_count: u64,
    pub required_gpu_count: u64,
    pub required_capacity_units_per_s: f64,
    pub available_capacity_units_per_s: f64,
    pub capacity_feasible: bool,
    pub useful_work_feasible: bool,
    pub qos_feasible: bool,
    pub service_feasible: bool,
    pub is_capacity_scaled_model: bool,
    pub completed_work_units: Option<f64>,
    pub gpu_board_power_w: f64,
    pub support_overhead_power_w: Option<f64>,
    pub fixed_fleet_overhead_power_w: Option<f64>,
    pub total_system_power_w: Option<f64>,
    pub gpu_board_energy_j: f64,
    pub total_system_energy_j: Option<f64>,
    pub storage_adjusted_total_system_energy_j: Option<f64>,
    pub gpu_board_energy_per_work_unit_j: Option<f64>,
    pub total_system_energy_per_work_unit_j: Option<f64>,
    pub installed_gpu_capex: Option<f64>,
    pub capex_currency: Option<String>,
    pub capex_source_reference: Option<String>,
    pub overhead_evidence: FleetOverheadEvidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FleetCapacityCandidateResult {
    pub run_id: String,
    pub control_action_id: String,
    pub per_gpu_power_change_fraction: f64,
    pub per_gpu_goodput_change_fraction: f64,
    pub fixed_fleet: FleetScenarioResult,
    pub capacity_scaled: FleetScenarioResult,
    pub pareto_frontier_member: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FleetCapacitySweepReport {
    pub baseline_run_id: String,
    pub accelerator_sku: String,
    pub workload_contract_id: String,
    pub request_mix_id: String,
    pub work_unit: String,
    pub offered_work_rate_per_s: f64,
    pub duration_s: f64,
    pub baseline_fixed_fleet: FleetScenarioResult,
    pub baseline_capacity_scaled: FleetScenarioResult,
    pub candidates: Vec<FleetCapacityCandidateResult>,
    /// Non-dominated candidates on the capacity-scaled scenario. Unknown
    /// overhead/capex dimensions are never treated as zero cost.
    pub pareto_frontier_run_ids: Vec<String>,
    pub claim_ceiling: &'static str,
}

fn required_gpu_count(required_capacity: f64, per_gpu_goodput: f64) -> Result<u64, AuditError> {
    let ratio = required_capacity / per_gpu_goodput;
    if !ratio.is_finite() || ratio <= 0.0 || ratio.ceil() > u64::MAX as f64 {
        return Err(AuditError(
            "required GPU count is non-finite or exceeds the supported integer range"
                .to_string(),
        ));
    }
    Ok(ratio.ceil().max(1.0) as u64)
}

fn qos_feasible(
    baseline: &FleetOperatingPoint,
    candidate: &FleetOperatingPoint,
    gate: FleetCapacityGate,
) -> bool {
    candidate.p99_latency_ms
        <= baseline.p99_latency_ms * (1.0 + gate.maximum_p99_latency_increase_fraction)
        && candidate.p99_time_to_first_token_ms
            <= baseline.p99_time_to_first_token_ms
                * (1.0 + gate.maximum_p99_ttft_increase_fraction)
        && candidate.p99_latency_ms <= candidate.p99_latency_slo_ms
        && candidate.p99_time_to_first_token_ms <= candidate.p99_ttft_slo_ms
        && candidate.slo_violation_fraction
            <= candidate.maximum_slo_violation_fraction.min(gate.maximum_slo_violation_fraction)
}

fn scenario_result(
    point: &FleetOperatingPoint,
    installed_gpu_count: u64,
    required_gpu_count: u64,
    capacity_feasible: bool,
    useful_work_feasible: bool,
    qos_ok: bool,
    completed_work_units: Option<f64>,
    is_capacity_scaled_model: bool,
) -> Result<FleetScenarioResult, AuditError> {
    let duration = point.duration_s();
    let required_capacity = point.required_capacity_units_per_s();
    let available_capacity = installed_gpu_count as f64 * point.per_gpu_goodput_units_per_s;
    let service_feasible = capacity_feasible && useful_work_feasible && qos_ok;

    let gpu_board_power_w = point.board_power_w_for_gpu_count(installed_gpu_count);
    let gpu_board_energy_j = gpu_board_power_w * duration;
    if !gpu_board_power_w.is_finite()
        || !gpu_board_energy_j.is_finite()
        || gpu_board_energy_j <= 0.0
        || !available_capacity.is_finite()
    {
        return Err(AuditError(
            "fleet scenario derived board power, energy or capacity is invalid".to_string(),
        ));
    }

    let total_system_power_w =
        point.total_power_w_for_gpu_count(installed_gpu_count, is_capacity_scaled_model);
    let support_overhead_power_w = point
        .per_gpu_support_power_w
        .map(|per_gpu| per_gpu * installed_gpu_count as f64);
    let fixed_fleet_overhead_power_w = if is_capacity_scaled_model {
        point.fixed_capacity_scaled_overhead_power_w
    } else {
        point.fixed_fleet_overhead_power_w
    };
    let total_system_energy_j = total_system_power_w.map(|power| power * duration);
    let storage_adjusted_total_system_energy_j = total_system_energy_j.map(|energy| {
        energy - (point.final_stored_energy_j - point.initial_stored_energy_j)
    });
    if total_system_energy_j.is_some_and(|energy| !energy.is_finite())
        || storage_adjusted_total_system_energy_j
            .is_some_and(|energy| !energy.is_finite() || energy <= 0.0)
        || support_overhead_power_w.is_some_and(|power| !power.is_finite())
    {
        return Err(AuditError(
            "fleet scenario derived system power or energy is invalid".to_string(),
        ));
    }

    let accepted_work =
        completed_work_units.filter(|work| service_feasible && work.is_finite() && *work > 0.0);
    let gpu_board_energy_per_work_unit_j =
        accepted_work.map(|work| gpu_board_energy_j / work);
    let total_system_energy_per_work_unit_j = accepted_work.and_then(|work| {
        storage_adjusted_total_system_energy_j.map(|energy| energy / work)
    });
    let installed_gpu_capex = point
        .capex_per_gpu
        .map(|cost| cost * installed_gpu_count as f64);
    if installed_gpu_capex.is_some_and(|cost| !cost.is_finite()) {
        return Err(AuditError(
            "fleet capex calculation overflowed".to_string(),
        ));
    }

    Ok(FleetScenarioResult {
        installed_gpu_count,
        required_gpu_count,
        required_capacity_units_per_s: required_capacity,
        available_capacity_units_per_s: available_capacity,
        capacity_feasible,
        useful_work_feasible,
        qos_feasible: qos_ok,
        service_feasible,
        is_capacity_scaled_model,
        completed_work_units: accepted_work,
        gpu_board_power_w,
        support_overhead_power_w,
        fixed_fleet_overhead_power_w,
        total_system_power_w,
        gpu_board_energy_j,
        total_system_energy_j,
        storage_adjusted_total_system_energy_j,
        gpu_board_energy_per_work_unit_j,
        total_system_energy_per_work_unit_j,
        installed_gpu_capex,
        capex_currency: point.capex_currency.clone(),
        capex_source_reference: point.capex_source_reference.clone(),
        overhead_evidence: point.overhead_evidence,
    })
}

fn validate_matched_points(
    baseline: &FleetOperatingPoint,
    candidate: &FleetOperatingPoint,
) -> Result<(), AuditError> {
    if baseline.workload_contract_id != candidate.workload_contract_id
        || baseline.request_mix_id != candidate.request_mix_id
        || baseline.quality_contract_id != candidate.quality_contract_id
        || baseline.accelerator_sku != candidate.accelerator_sku
        || baseline.software_config_id != candidate.software_config_id
        || baseline.boundary_id != candidate.boundary_id
        || baseline.work_unit != candidate.work_unit
        || baseline.window_start_ns != candidate.window_start_ns
        || baseline.window_end_ns != candidate.window_end_ns
        || baseline.offered_work_rate_per_s.to_bits()
            != candidate.offered_work_rate_per_s.to_bits()
        || baseline.capacity_headroom_fraction.to_bits()
            != candidate.capacity_headroom_fraction.to_bits()
        || baseline.installed_gpu_count != candidate.installed_gpu_count
        || baseline.initial_stored_energy_j.to_bits()
            != candidate.initial_stored_energy_j.to_bits()
    {
        return Err(AuditError(format!(
            "fleet candidate {:?} does not match workload, request mix, SKU/software, quality, boundary, demand, window, headroom, fixed fleet size or initial stored energy",
            candidate.run_id
        )));
    }
    Ok(())
}

fn pareto_dominates(left: &FleetCapacityCandidateResult, right: &FleetCapacityCandidateResult) -> bool {
    let (Some(left_energy), Some(right_energy), Some(left_power), Some(right_power)) = (
        left.capacity_scaled.total_system_energy_per_work_unit_j,
        right.capacity_scaled.total_system_energy_per_work_unit_j,
        left.capacity_scaled.total_system_power_w,
        right.capacity_scaled.total_system_power_w,
    ) else {
        return false;
    };

    let mut no_worse = left_energy <= right_energy
        && left_power <= right_power
        && left.capacity_scaled.installed_gpu_count <= right.capacity_scaled.installed_gpu_count;
    let mut strictly_better = left_energy < right_energy
        || left_power < right_power
        || left.capacity_scaled.installed_gpu_count < right.capacity_scaled.installed_gpu_count;

    match (
        left.capacity_scaled.installed_gpu_capex,
        right.capacity_scaled.installed_gpu_capex,
        left.capacity_scaled.capex_currency.as_deref(),
        right.capacity_scaled.capex_currency.as_deref(),
    ) {
        (Some(a), Some(b), Some(currency_a), Some(currency_b)) if currency_a == currency_b => {
            no_worse &= a <= b;
            strictly_better |= a < b;
        }
        (None, None, None, None) => {}
        _ => return false,
    }

    no_worse && strictly_better
}

/// Evaluate fixed-fleet and capacity-scaled scenarios from matched measured
/// single-GPU operating points.
///
/// Capacity-scaled counts are a linear-goodput projection, rounded up to an
/// integer GPU count. They are not measured fleet results. The caller must
/// supply explicit non-GPU overhead for a total-system comparison; overhead is
/// never inferred from GPU board power or TDP. The function is read-only.
pub fn evaluate_fleet_capacity_sweep(
    baseline: FleetOperatingPoint,
    candidates: Vec<FleetOperatingPoint>,
    gate: FleetCapacityGate,
) -> Result<FleetCapacitySweepReport, AuditError> {
    gate.validate()?;
    baseline.validate()?;
    if baseline.control_action_id.is_some() {
        return Err(AuditError(
            "baseline fleet run must not declare a control action".to_string(),
        ));
    }
    if candidates.is_empty() {
        return Err(AuditError(
            "fleet capacity sweep requires at least one candidate".to_string(),
        ));
    }

    let baseline_required_count = required_gpu_count(
        baseline.required_capacity_units_per_s(),
        baseline.per_gpu_goodput_units_per_s,
    )?;
    let baseline_fixed_capacity = baseline.installed_gpu_count as f64
        * baseline.per_gpu_goodput_units_per_s;
    let baseline_fixed_qos = baseline.p99_latency_ms <= baseline.p99_latency_slo_ms
        && baseline.p99_time_to_first_token_ms <= baseline.p99_ttft_slo_ms
        && baseline.slo_violation_fraction
            <= baseline.maximum_slo_violation_fraction.min(gate.maximum_slo_violation_fraction);
    let baseline_fixed_feasible = baseline_fixed_capacity
        >= baseline.required_capacity_units_per_s()
        && baseline.completed_work_units
            >= baseline.offered_work_units() * (1.0 - gate.maximum_useful_work_loss_fraction);
    if !baseline_fixed_feasible || !baseline_fixed_qos {
        return Err(AuditError(
            "baseline fixed fleet does not meet the offered workload and QoS contract"
                .to_string(),
        ));
    }

    let baseline_fixed = scenario_result(
        &baseline,
        baseline.installed_gpu_count,
        baseline_required_count,
        true,
        true,
        true,
        Some(baseline.completed_work_units),
        false,
    )?;
    let baseline_scaled = scenario_result(
        &baseline,
        baseline_required_count,
        baseline_required_count,
        true,
        true,
        true,
        Some(baseline.offered_work_units()),
        true,
    )?;

    let mut seen = HashSet::new();
    seen.insert(baseline.run_id.as_str());
    let mut results = Vec::with_capacity(candidates.len());

    for candidate in &candidates {
        candidate.validate()?;
        validate_matched_points(&baseline, candidate)?;
        if !seen.insert(candidate.run_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate fleet operating point ID {:?}",
                candidate.run_id
            )));
        }
        let action = candidate
            .control_action_id
            .as_ref()
            .filter(|action| !action.trim().is_empty())
            .ok_or_else(|| {
                AuditError(format!(
                    "fleet candidate {:?} must declare control_action_id",
                    candidate.run_id
                ))
            })?;

        let qos_ok = qos_feasible(&baseline, candidate, gate);
        let useful_work_retention =
            candidate.completed_work_units / baseline.completed_work_units;
        if !useful_work_retention.is_finite() {
            return Err(AuditError(
                "fleet useful-work retention is non-finite".to_string(),
            ));
        }
        let fixed_capacity = candidate.installed_gpu_count as f64
            * candidate.per_gpu_goodput_units_per_s;
        let fixed_capacity_ok = fixed_capacity >= candidate.required_capacity_units_per_s();
        let fixed_work_ok = candidate.completed_work_units
            >= candidate.offered_work_units()
                * (1.0 - gate.maximum_useful_work_loss_fraction);
        let fixed_fleet = scenario_result(
            candidate,
            candidate.installed_gpu_count,
            required_gpu_count(
                candidate.required_capacity_units_per_s(),
                candidate.per_gpu_goodput_units_per_s,
            )?,
            fixed_capacity_ok,
            fixed_work_ok,
            qos_ok,
            Some(candidate.completed_work_units),
            false,
        )?;

        let scaled_gpu_count = required_gpu_count(
            candidate.required_capacity_units_per_s(),
            candidate.per_gpu_goodput_units_per_s,
        )?;
        let capacity_scaled = scenario_result(
            candidate,
            scaled_gpu_count,
            scaled_gpu_count,
            true,
            true,
            qos_ok,
            Some(candidate.offered_work_units()),
            true,
        )?;

        results.push(FleetCapacityCandidateResult {
            run_id: candidate.run_id.clone(),
            control_action_id: action.clone(),
            per_gpu_power_change_fraction: candidate.per_gpu_board_power_w
                / baseline.per_gpu_board_power_w
                - 1.0,
            per_gpu_goodput_change_fraction: candidate.per_gpu_goodput_units_per_s
                / baseline.per_gpu_goodput_units_per_s
                - 1.0,
            fixed_fleet,
            capacity_scaled,
            pareto_frontier_member: false,
        });
    }

    let eligible: Vec<usize> = results
        .iter()
        .enumerate()
        .filter_map(|(index, result)| {
            (result.capacity_scaled.service_feasible
                && result.capacity_scaled.total_system_energy_per_work_unit_j.is_some()
                && result.capacity_scaled.total_system_power_w.is_some())
                .then_some(index)
        })
        .collect();
    let mut frontier = Vec::new();
    for &index in &eligible {
        let dominated = eligible.iter().copied().any(|other| {
            other != index && pareto_dominates(&results[other], &results[index])
        });
        if !dominated {
            results[index].pareto_frontier_member = true;
            frontier.push(results[index].run_id.clone());
        }
    }
    frontier.sort();

    Ok(FleetCapacitySweepReport {
        baseline_run_id: baseline.run_id.clone(),
        accelerator_sku: baseline.accelerator_sku.clone(),
        workload_contract_id: baseline.workload_contract_id.clone(),
        request_mix_id: baseline.request_mix_id.clone(),
        work_unit: baseline.work_unit.clone(),
        offered_work_rate_per_s: baseline.offered_work_rate_per_s,
        duration_s: baseline.duration_s(),
        baseline_fixed_fleet: baseline_fixed,
        baseline_capacity_scaled: baseline_scaled,
        candidates: results,
        pareto_frontier_run_ids: frontier,
        claim_ceiling: "FLEET_CAPACITY_LINEAR_GOODPUT_PROJECTION_WITH_DECLARED_OVERHEAD_ONLY_NOT_MEASURED_SCALED_FLEET_NOT_LIVE_SCHEDULING_OR_FACILITY_QUALIFICATION",
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_fleet_capacity_sweep, FleetCapacityGate, FleetOperatingPoint,
        FleetOverheadEvidence,
    };

    fn point(
        id: &str,
        action: Option<&str>,
        per_gpu_goodput: f64,
        per_gpu_power: f64,
        completed_work: f64,
    ) -> FleetOperatingPoint {
        FleetOperatingPoint {
            run_id: id.to_string(),
            evidence_id: format!("evidence-{id}"),
            workload_contract_id: "llm-model-a-bf16".to_string(),
            request_mix_id: "request-mix-01".to_string(),
            quality_contract_id: "quality-contract-01".to_string(),
            accelerator_sku: "GPU-X-80GB".to_string(),
            software_config_id: "serving-stack-17".to_string(),
            boundary_id: "rack-input-01".to_string(),
            window_start_ns: 0,
            window_end_ns: 60_000_000_000,
            offered_work_rate_per_s: 80.0,
            work_unit: "accepted-output-tokens".to_string(),
            capacity_headroom_fraction: 0.0,
            installed_gpu_count: 8,
            per_gpu_goodput_units_per_s: per_gpu_goodput,
            completed_work_units: completed_work,
            per_gpu_board_power_w: per_gpu_power,
            per_gpu_support_power_w: Some(50.0),
            fixed_fleet_overhead_power_w: Some(500.0),
            fixed_capacity_scaled_overhead_power_w: Some(500.0),
            overhead_evidence: FleetOverheadEvidence::Measured,
            initial_stored_energy_j: 10_000.0,
            final_stored_energy_j: 10_000.0,
            p99_latency_ms: 1000.0,
            p99_time_to_first_token_ms: 150.0,
            p99_latency_slo_ms: 1050.0,
            p99_ttft_slo_ms: 160.0,
            slo_violation_fraction: 0.001,
            maximum_slo_violation_fraction: 0.01,
            capex_per_gpu: Some(20_000.0),
            capex_currency: Some("USD".to_string()),
            capex_source_reference: Some("synthetic-fixture-cost-basis".to_string()),
            control_action_id: action.map(str::to_string),
            measurement_contract_valid: true,
        }
    }

    fn gate() -> FleetCapacityGate {
        FleetCapacityGate {
            maximum_useful_work_loss_fraction: 0.01,
            maximum_p99_latency_increase_fraction: 0.05,
            maximum_p99_ttft_increase_fraction: 0.05,
            maximum_slo_violation_fraction: 0.01,
        }
    }

    #[test]
    fn twenty_percent_goodput_reduction_requires_at_least_25_percent_more_gpus() {
        let baseline = point("baseline", None, 10.0, 400.0, 4_800.0);
        let candidate = point("capped", Some("cap-200w"), 8.0, 200.0, 3_840.0);

        let report =
            evaluate_fleet_capacity_sweep(baseline, vec![candidate], gate()).unwrap();
        let result = &report.candidates[0];

        assert_eq!(report.baseline_capacity_scaled.installed_gpu_count, 8);
        assert_eq!(result.capacity_scaled.installed_gpu_count, 10);
        assert!((result.capacity_scaled.installed_gpu_count as f64
            / report.baseline_capacity_scaled.installed_gpu_count as f64
            - 1.25)
            .abs()
            < 1.0e-12);
    }

    #[test]
    fn fixed_fleet_can_fail_while_capacity_scaled_scenario_preserves_service() {
        let baseline = point("baseline", None, 10.0, 400.0, 4_800.0);
        let candidate = point("capped", Some("cap"), 8.0, 200.0, 3_840.0);

        let report =
            evaluate_fleet_capacity_sweep(baseline, vec![candidate], gate()).unwrap();
        let result = &report.candidates[0];

        assert!(!result.fixed_fleet.service_feasible);
        assert!(result.capacity_scaled.service_feasible);
        assert_eq!(result.fixed_fleet.installed_gpu_count, 8);
        assert_eq!(result.capacity_scaled.installed_gpu_count, 10);
    }

    #[test]
    fn per_gpu_power_reduction_is_not_reported_as_same_fleet_power_reduction() {
        let baseline = point("baseline", None, 10.0, 400.0, 4_800.0);
        let candidate = point("capped", Some("cap"), 8.0, 200.0, 3_840.0);

        let report =
            evaluate_fleet_capacity_sweep(baseline, vec![candidate], gate()).unwrap();
        let result = &report.candidates[0];

        assert!((result.per_gpu_power_change_fraction + 0.5).abs() < 1.0e-12);
        assert!((result.capacity_scaled.total_system_power_w.unwrap() - 3000.0).abs() < 1.0e-9);
        assert!((report.baseline_capacity_scaled.total_system_power_w.unwrap() - 4100.0).abs() < 1.0e-9);
        assert!(result.capacity_scaled.total_system_power_w.unwrap()
            < report.baseline_capacity_scaled.total_system_power_w.unwrap());
    }

    #[test]
    fn unknown_overhead_blocks_total_system_claim_but_preserves_gpu_board_result() {
        let baseline = point("baseline", None, 10.0, 400.0, 4_800.0);
        let mut candidate = point("unknown-overhead", Some("cap"), 10.0, 380.0, 4_800.0);
        candidate.per_gpu_support_power_w = None;
        candidate.fixed_fleet_overhead_power_w = None;
        candidate.fixed_capacity_scaled_overhead_power_w = None;
        candidate.overhead_evidence = FleetOverheadEvidence::Unknown;

        let report =
            evaluate_fleet_capacity_sweep(baseline, vec![candidate], gate()).unwrap();
        let result = &report.candidates[0].capacity_scaled;

        assert!(result.gpu_board_power_w > 0.0);
        assert!(result.total_system_power_w.is_none());
        assert!(result.total_system_energy_per_work_unit_j.is_none());
        assert!(!report.candidates[0].pareto_frontier_member);
    }

    #[test]
    fn storage_depletion_is_charged_to_fleet_energy_per_work() {
        let baseline = point("baseline", None, 10.0, 400.0, 4_800.0);
        let mut candidate = point("depleted", Some("cap"), 10.0, 380.0, 4_800.0);
        candidate.final_stored_energy_j = 8_000.0;

        let report =
            evaluate_fleet_capacity_sweep(baseline, vec![candidate], gate()).unwrap();
        let result = &report.candidates[0].capacity_scaled;

        assert!(result.gpu_board_energy_j > 0.0);
        assert!(result.storage_adjusted_total_system_energy_j.unwrap()
            > result.total_system_energy_j.unwrap());
    }

    #[test]
    fn latency_or_slo_regression_removes_capacity_scaled_candidate_from_service_frontier() {
        let baseline = point("baseline", None, 10.0, 400.0, 4_800.0);
        let mut candidate = point("slow", Some("cap"), 10.0, 380.0, 4_800.0);
        candidate.p99_latency_ms = 1100.0;
        candidate.slo_violation_fraction = 0.02;

        let report =
            evaluate_fleet_capacity_sweep(baseline, vec![candidate], gate()).unwrap();
        let result = &report.candidates[0];

        assert!(result.capacity_scaled.capacity_feasible);
        assert!(!result.capacity_scaled.qos_feasible);
        assert!(!result.capacity_scaled.service_feasible);
        assert!(report.pareto_frontier_run_ids.is_empty());
    }

    #[test]
    fn unknown_cost_remains_unknown_in_scaled_capital_estimate() {
        let baseline = point("baseline", None, 10.0, 400.0, 4_800.0);
        let mut candidate = point("unknown-cost", Some("cap"), 10.0, 380.0, 4_800.0);
        candidate.capex_per_gpu = None;
        candidate.capex_currency = None;
        candidate.capex_source_reference = None;

        let report =
            evaluate_fleet_capacity_sweep(baseline, vec![candidate], gate()).unwrap();

        assert!(report.candidates[0].capacity_scaled.installed_gpu_capex.is_none());
        // The energy/power frontier remains available; capital is reported
        // as unknown and is not used as a hidden zero-cost objective.
        assert!(report.candidates[0].pareto_frontier_member);
    }

    #[test]
    fn supplied_capex_scales_with_integer_gpu_count() {
        let baseline = point("baseline", None, 10.0, 400.0, 4_800.0);
        let candidate = point("capped", Some("cap"), 8.0, 200.0, 3_840.0);

        let report =
            evaluate_fleet_capacity_sweep(baseline, vec![candidate], gate()).unwrap();

        assert_eq!(
            report.candidates[0].capacity_scaled.installed_gpu_capex,
            Some(200_000.0)
        );
        assert_eq!(
            report.baseline_capacity_scaled.installed_gpu_capex,
            Some(160_000.0)
        );
    }

    #[test]
    fn rejects_changed_demand_sku_boundary_or_initial_soc() {
        let baseline = point("baseline", None, 10.0, 400.0, 4_800.0);

        let mut changed_demand = point("demand", Some("cap"), 10.0, 380.0, 4_800.0);
        changed_demand.offered_work_rate_per_s = 81.0;
        assert!(evaluate_fleet_capacity_sweep(baseline.clone(), vec![changed_demand], gate()).is_err());

        let mut changed_sku = point("sku", Some("cap"), 10.0, 380.0, 4_800.0);
        changed_sku.accelerator_sku = "GPU-Y-96GB".to_string();
        assert!(evaluate_fleet_capacity_sweep(baseline.clone(), vec![changed_sku], gate()).is_err());

        let mut changed_boundary = point("boundary", Some("cap"), 10.0, 380.0, 4_800.0);
        changed_boundary.boundary_id = "other-boundary".to_string();
        assert!(evaluate_fleet_capacity_sweep(baseline.clone(), vec![changed_boundary], gate()).is_err());

        let mut changed_soc = point("soc", Some("cap"), 10.0, 380.0, 4_800.0);
        changed_soc.initial_stored_energy_j += 1.0;
        assert!(evaluate_fleet_capacity_sweep(baseline, vec![changed_soc], gate()).is_err());
    }

    #[test]
    fn allowed_useful_work_loss_is_reported_and_energy_is_normalized_to_actual_work() {
        let baseline = point("baseline", None, 10.0, 400.0, 4_800.0);
        let candidate = point("small-work-loss", Some("cap"), 10.0, 380.0, 4_776.0);
        // A 0.5% loss is within the declared 1% gate.
        let report = evaluate_fleet_capacity_sweep(baseline, vec![candidate], gate()).unwrap();
        let result = &report.candidates[0].fixed_fleet;

        assert!(result.capacity_feasible);
        assert!(result.useful_work_feasible);
        assert!(result.qos_feasible);
        assert!(result.service_feasible);
        assert_eq!(result.completed_work_units, Some(4_776.0));
        assert!(result.gpu_board_energy_per_work_unit_j.is_some());
    }

    #[test]
    fn rejects_unmet_baseline_and_invalid_overhead_metadata() {
        let unmet_baseline = point("baseline", None, 10.0, 400.0, 100.0);
        assert!(evaluate_fleet_capacity_sweep(
            unmet_baseline.clone(),
            vec![point("candidate", Some("cap"), 10.0, 380.0, 4_800.0)],
            gate()
        )
        .is_err());

        let mut invalid_overhead = point("invalid", Some("cap"), 10.0, 380.0, 4_800.0);
        invalid_overhead.per_gpu_support_power_w = None;
        invalid_overhead.fixed_fleet_overhead_power_w = Some(100.0);
        invalid_overhead.overhead_evidence = FleetOverheadEvidence::Measured;
        assert!(evaluate_fleet_capacity_sweep(
            point("baseline", None, 10.0, 400.0, 4_800.0),
            vec![invalid_overhead],
            gate()
        )
        .is_err());
    }
}