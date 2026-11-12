use crate::temporal_scenarios::{
    validate_scenario_set, ForecastScenarioSet, ScenarioGenerationContract,
};
use crate::AuditError;

/// This module qualifies an offline modelled reserve decision only. It does not
/// establish physical reserve delivery, PCC capability or grid-service eligibility.
pub const RESERVE_RISK_CLAIM_CEILING: &str =
    "MODELLED_FINITE_SUPPORT_RESERVE_RISK_NOT_PHYSICAL_GRID_RESERVE_QUALIFICATION";

const FLOAT_EPSILON: f64 = 1.0e-10;
const MAX_RESERVE_CANDIDATES: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservePolicyKind {
    /// Minimum empirical expected reserve-plus-shortfall cost on the frozen grid.
    EmpiricalCriticalFractile,
    /// Minimum empirical expected cost subject to empirical CVaR and chance gates.
    EmpiricalCvarConstrained,
    /// Minimum worst-case expected cost subject to finite-support TV-DRO CVaR
    /// and chance gates.
    TotalVariationDroCvar,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReserveRiskContract {
    pub contract_id: String,
    pub contract_revision: String,
    pub decision_evidence_id: String,
    /// CVaR confidence alpha; 0.95 means the worst 5% of shortfall outcomes.
    pub tail_confidence: f64,
    /// Total-variation radius over probability reweightings of the observed
    /// scenario paths. This does not add probability to unseen support.
    pub total_variation_radius: f64,
    pub reserve_cost_ratio: f64,
    pub maximum_empirical_cvar_shortfall_mw: u64,
    pub maximum_robust_cvar_shortfall_mw: u64,
    pub maximum_shortfall_probability: f64,
    pub minimum_scenario_count: usize,
    /// Minimum effective number of paths in the CVaR tail, e.g. 10 for
    /// 200 scenarios at alpha=0.95.
    pub minimum_tail_path_mass: f64,
    pub minimum_reserve_mw: u64,
    pub maximum_reserve_mw: u64,
    pub reserve_step_mw: u64,
}

impl ReserveRiskContract {
    pub fn validate(&self) -> Result<(), AuditError> {
        for (value, field) in [
            (self.contract_id.as_str(), "contract_id"),
            (self.contract_revision.as_str(), "contract_revision"),
            (self.decision_evidence_id.as_str(), "decision_evidence_id"),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!(
                    "reserve-risk contract {field} must not be blank"
                )));
            }
        }

        if !self.tail_confidence.is_finite()
            || self.tail_confidence <= 0.5
            || self.tail_confidence >= 1.0
            || !self.total_variation_radius.is_finite()
            || self.total_variation_radius < 0.0
            || self.total_variation_radius > 1.0
            || !self.reserve_cost_ratio.is_finite()
            || self.reserve_cost_ratio <= 0.0
            || self.reserve_cost_ratio >= 1.0
            || !self.maximum_shortfall_probability.is_finite()
            || self.maximum_shortfall_probability < 0.0
            || self.maximum_shortfall_probability > 1.0
            || !self.minimum_tail_path_mass.is_finite()
            || self.minimum_tail_path_mass <= 0.0
        {
            return Err(AuditError(
                "reserve-risk contract has an invalid confidence, ambiguity radius, cost ratio, probability or tail support threshold"
                    .to_string(),
            ));
        }

        if self.minimum_scenario_count < 2
            || self.reserve_step_mw == 0
            || self.minimum_reserve_mw > self.maximum_reserve_mw
        {
            return Err(AuditError(
                "reserve-risk scenario support and reserve grid must be positive and ordered"
                    .to_string(),
            ));
        }

        let reserve_span = self.maximum_reserve_mw - self.minimum_reserve_mw;
        if reserve_span % self.reserve_step_mw != 0 {
            return Err(AuditError(
                "reserve-risk maximum must lie exactly on the frozen reserve grid".to_string(),
            ));
        }
        let candidate_count = reserve_span / self.reserve_step_mw + 1;
        if candidate_count > MAX_RESERVE_CANDIDATES {
            return Err(AuditError(format!(
                "reserve-risk candidate grid has {candidate_count} points; maximum is {MAX_RESERVE_CANDIDATES}"
            )));
        }
        Ok(())
    }

    fn validate_scenario_support(&self, scenario_count: usize) -> Result<f64, AuditError> {
        self.validate()?;
        if scenario_count < self.minimum_scenario_count {
            return Err(AuditError(format!(
                "reserve-risk evaluation has {scenario_count} scenarios; contract requires at least {}",
                self.minimum_scenario_count
            )));
        }
        let tail_path_mass = (1.0 - self.tail_confidence) * scenario_count as f64;
        if tail_path_mass + FLOAT_EPSILON < self.minimum_tail_path_mass {
            return Err(AuditError(format!(
                "CVaR tail has {tail_path_mass:.6} effective paths; contract requires at least {:.6}",
                self.minimum_tail_path_mass
            )));
        }
        Ok(tail_path_mass)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReserveRiskFrontierPoint {
    pub reserve_power_mw: u64,
    pub empirical_expected_cost_ratio_units: f64,
    pub worst_case_expected_cost_ratio_units: f64,
    pub empirical_shortfall_probability: f64,
    pub worst_case_shortfall_probability: f64,
    pub empirical_mean_shortfall_mw: f64,
    pub worst_case_mean_shortfall_mw: f64,
    pub empirical_cvar_shortfall_mw: f64,
    pub worst_case_cvar_shortfall_mw: f64,
    pub empirical_cvar_gate_met: bool,
    pub empirical_chance_gate_met: bool,
    pub robust_cvar_gate_met: bool,
    pub robust_chance_gate_met: bool,
    /// Pareto status over worst-case expected cost, worst-case CVaR and
    /// worst-case shortfall probability. It is not a scalar utility score.
    pub on_worst_case_cost_risk_frontier: bool,
    pub effective_tail_path_mass: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReservePolicyDecision {
    pub policy: ReservePolicyKind,
    pub reserve_power_mw: u64,
    pub empirical_expected_cost_ratio_units: f64,
    pub worst_case_expected_cost_ratio_units: f64,
    pub empirical_shortfall_probability: f64,
    pub worst_case_shortfall_probability: f64,
    pub empirical_cvar_shortfall_mw: f64,
    pub worst_case_cvar_shortfall_mw: f64,
    pub qualified_under_declared_risk_contract: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReserveRiskReport {
    pub contract: ReserveRiskContract,
    pub forecast_id: String,
    pub forecast_origin_id: String,
    pub forecast_origin_timestamp_ns: u64,
    pub forecast_evidence_id: String,
    pub source_timeline_digest_reference: String,
    pub scenario_generation_contract: ScenarioGenerationContract,
    pub scenario_ids: Vec<String>,
    pub scenario_count: usize,
    pub horizon_steps: usize,
    /// Horizon maximum positive deviation from the scheduled median for each
    /// validated scenario path. The complete paths remain in the source set.
    pub horizon_surplus_samples_mw: Vec<u64>,
    pub effective_tail_path_mass: f64,
    pub empirical_critical_fractile: ReservePolicyDecision,
    pub empirical_cvar_constrained: Option<ReservePolicyDecision>,
    pub total_variation_dro_cvar: Option<ReservePolicyDecision>,
    pub frontier: Vec<ReserveRiskFrontierPoint>,
    pub claim_ceiling: &'static str,
}

fn validate_nonnegative_samples(values: &[f64]) -> Result<(), AuditError> {
    if values.is_empty()
        || values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(AuditError(
            "risk samples must be non-empty, finite and non-negative".to_string(),
        ));
    }
    Ok(())
}

/// Expected value under the most adverse reweighting q of the empirical
/// uniform distribution p, subject to TV(q,p) <= radius.
///
/// The ambiguity set only reweights observed support. It cannot assign mass to
/// events absent from the scenario pool. The greedy transfer is exact for a
/// finite support and an unrestricted simplex: move mass from the lowest
/// values to a highest-valued atom until the TV budget or donor mass is spent.
fn worst_case_expectation(values: &[f64], radius: f64) -> Result<f64, AuditError> {
    validate_nonnegative_samples(values)?;
    if !radius.is_finite() || !(0.0..=1.0).contains(&radius) {
        return Err(AuditError(
            "total-variation radius must be finite and in [0, 1]".to_string(),
        ));
    }

    let count = values.len() as f64;
    let empirical_mean = values.iter().sum::<f64>() / count;
    if radius == 0.0 {
        return Ok(empirical_mean);
    }

    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut ascending = values.to_vec();
    ascending.sort_by(f64::total_cmp);

    let mut remaining_mass = radius;
    let mut improvement = 0.0;
    let atom_mass = 1.0 / count;
    for value in ascending {
        if value >= maximum || remaining_mass <= FLOAT_EPSILON {
            break;
        }
        let transferred = atom_mass.min(remaining_mass);
        improvement += transferred * (maximum - value);
        remaining_mass -= transferred;
    }

    let result = empirical_mean + improvement;
    if !result.is_finite() {
        return Err(AuditError(
            "worst-case expectation overflowed".to_string(),
        ));
    }
    Ok(result)
}

/// Empirical upper-tail CVaR using exact fractional weighting of the final
/// order statistic. This avoids rounding a non-integer tail sample count.
fn empirical_cvar(losses: &[f64], alpha: f64) -> Result<f64, AuditError> {
    validate_nonnegative_samples(losses)?;
    if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
        return Err(AuditError(
            "CVaR confidence alpha must be finite and in (0, 1)".to_string(),
        ));
    }

    let mut descending = losses.to_vec();
    descending.sort_by(|left, right| right.total_cmp(left));
    let mut remaining = (1.0 - alpha) * descending.len() as f64;
    let tail_mass = remaining;
    let mut weighted_sum = 0.0;
    for loss in descending {
        if remaining <= FLOAT_EPSILON {
            break;
        }
        let weight = remaining.min(1.0);
        weighted_sum += weight * loss;
        remaining -= weight;
    }
    let result = weighted_sum / tail_mass;
    if !result.is_finite() {
        return Err(AuditError("empirical CVaR overflowed".to_string()));
    }
    Ok(result)
}

/// Worst-case CVaR over all reweightings of the empirical scenario support
/// inside a total-variation ball. It solves
///
/// min_eta [ eta + sup_{q: TV(q,p)<=rho} E_q[(L-eta)+] / (1-alpha) ].
///
/// The inner finite-support worst-case expectation is exact. This is a
/// finite-support DRO sensitivity envelope, not a guarantee against unseen
/// out-of-support events or a statistically calibrated ambiguity radius.
fn worst_case_cvar(losses: &[f64], alpha: f64, radius: f64) -> Result<f64, AuditError> {
    validate_nonnegative_samples(losses)?;
    if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
        return Err(AuditError(
            "CVaR confidence alpha must be finite and in (0, 1)".to_string(),
        ));
    }
    if !radius.is_finite() || !(0.0..=1.0).contains(&radius) {
        return Err(AuditError(
            "total-variation radius must be finite and in [0, 1]".to_string(),
        ));
    }

    let mut thresholds = losses.to_vec();
    thresholds.sort_by(f64::total_cmp);
    thresholds.dedup_by(|left, right| left.to_bits() == right.to_bits());

    let mut best = f64::INFINITY;
    for threshold in thresholds {
        let excess: Vec<f64> = losses
            .iter()
            .map(|loss| (loss - threshold).max(0.0))
            .collect();
        let candidate = threshold
            + worst_case_expectation(&excess, radius)? / (1.0 - alpha);
        best = best.min(candidate);
    }
    if !best.is_finite() {
        return Err(AuditError("worst-case CVaR could not be evaluated".to_string()));
    }
    Ok(best)
}

fn horizon_surplus(power_mw: &[u64], scheduled_median_mw: &[u64]) -> Result<u64, AuditError> {
    if power_mw.is_empty() || power_mw.len() != scheduled_median_mw.len() {
        return Err(AuditError(
            "scenario and scheduled-median paths must have the same non-zero horizon".to_string(),
        ));
    }
    Ok(power_mw
        .iter()
        .zip(scheduled_median_mw)
        .map(|(power, median)| power.saturating_sub(*median))
        .max()
        .unwrap_or(0))
}

fn evaluate_distribution(
    horizon_surplus_samples_mw: &[u64],
    contract: &ReserveRiskContract,
) -> Result<(Vec<ReserveRiskFrontierPoint>, f64), AuditError> {
    let tail_path_mass = contract.validate_scenario_support(horizon_surplus_samples_mw.len())?;
    let shortfall_limit = contract.maximum_shortfall_probability;
    let radius = contract.total_variation_radius;
    let mut frontier = Vec::new();

    let mut reserve = contract.minimum_reserve_mw;
    loop {
        let shortfalls_mw: Vec<f64> = horizon_surplus_samples_mw
            .iter()
            .map(|surplus| surplus.saturating_sub(reserve) as f64)
            .collect();
        let indicators: Vec<f64> = shortfalls_mw
            .iter()
            .map(|shortfall| if *shortfall > 0.0 { 1.0 } else { 0.0 })
            .collect();

        let scenario_count = shortfalls_mw.len() as f64;
        let empirical_mean_shortfall =
            shortfalls_mw.iter().sum::<f64>() / scenario_count;
        let empirical_shortfall_probability =
            indicators.iter().sum::<f64>() / scenario_count;
        let worst_case_mean_shortfall = worst_case_expectation(&shortfalls_mw, radius)?;
        let worst_case_shortfall_probability = worst_case_expectation(&indicators, radius)?;
        let empirical_cvar =
            empirical_cvar(&shortfalls_mw, contract.tail_confidence)?;
        let robust_cvar = worst_case_cvar(
            &shortfalls_mw,
            contract.tail_confidence,
            radius,
        )?;
        let reserve_cost = contract.reserve_cost_ratio * reserve as f64;
        let empirical_cost = reserve_cost + empirical_mean_shortfall;
        let worst_case_cost = reserve_cost + worst_case_mean_shortfall;

        frontier.push(ReserveRiskFrontierPoint {
            reserve_power_mw: reserve,
            empirical_expected_cost_ratio_units: empirical_cost,
            worst_case_expected_cost_ratio_units: worst_case_cost,
            empirical_shortfall_probability,
            worst_case_shortfall_probability,
            empirical_mean_shortfall_mw: empirical_mean_shortfall,
            worst_case_mean_shortfall_mw: worst_case_mean_shortfall,
            empirical_cvar_shortfall_mw: empirical_cvar,
            worst_case_cvar_shortfall_mw: robust_cvar,
            empirical_cvar_gate_met: empirical_cvar
                <= contract.maximum_empirical_cvar_shortfall_mw as f64 + FLOAT_EPSILON,
            empirical_chance_gate_met: empirical_shortfall_probability
                <= shortfall_limit + FLOAT_EPSILON,
            robust_cvar_gate_met: robust_cvar
                <= contract.maximum_robust_cvar_shortfall_mw as f64 + FLOAT_EPSILON,
            robust_chance_gate_met: worst_case_shortfall_probability
                <= shortfall_limit + FLOAT_EPSILON,
            on_worst_case_cost_risk_frontier: false,
            effective_tail_path_mass: tail_path_mass,
        });

        if reserve == contract.maximum_reserve_mw {
            break;
        }
        reserve = reserve
            .checked_add(contract.reserve_step_mw)
            .ok_or_else(|| AuditError("reserve candidate grid overflowed".to_string()))?;
    }

    for index in 0..frontier.len() {
        let candidate = &frontier[index];
        let dominated = frontier.iter().enumerate().any(|(other_index, other)| {
            if index == other_index {
                return false;
            }
            let no_worse = other.worst_case_expected_cost_ratio_units
                <= candidate.worst_case_expected_cost_ratio_units + FLOAT_EPSILON
                && other.worst_case_cvar_shortfall_mw
                    <= candidate.worst_case_cvar_shortfall_mw + FLOAT_EPSILON
                && other.worst_case_shortfall_probability
                    <= candidate.worst_case_shortfall_probability + FLOAT_EPSILON;
            let strictly_better = other.worst_case_expected_cost_ratio_units
                < candidate.worst_case_expected_cost_ratio_units - FLOAT_EPSILON
                || other.worst_case_cvar_shortfall_mw
                    < candidate.worst_case_cvar_shortfall_mw - FLOAT_EPSILON
                || other.worst_case_shortfall_probability
                    < candidate.worst_case_shortfall_probability - FLOAT_EPSILON;
            no_worse && strictly_better
        });
        frontier[index].on_worst_case_cost_risk_frontier = !dominated;
    }

    Ok((frontier, tail_path_mass))
}

fn select_decision(
    frontier: &[ReserveRiskFrontierPoint],
    policy: ReservePolicyKind,
) -> Option<ReservePolicyDecision> {
    let mut selected: Option<&ReserveRiskFrontierPoint> = None;
    for point in frontier {
        let feasible = match policy {
            ReservePolicyKind::EmpiricalCriticalFractile => true,
            ReservePolicyKind::EmpiricalCvarConstrained => {
                point.empirical_cvar_gate_met && point.empirical_chance_gate_met
            }
            ReservePolicyKind::TotalVariationDroCvar => {
                point.robust_cvar_gate_met && point.robust_chance_gate_met
            }
        };
        if !feasible {
            continue;
        }
        let objective = match policy {
            ReservePolicyKind::EmpiricalCriticalFractile
            | ReservePolicyKind::EmpiricalCvarConstrained => {
                point.empirical_expected_cost_ratio_units
            }
            ReservePolicyKind::TotalVariationDroCvar => {
                point.worst_case_expected_cost_ratio_units
            }
        };
        let should_replace = selected.is_none_or(|current| {
            let current_objective = match policy {
                ReservePolicyKind::EmpiricalCriticalFractile
                | ReservePolicyKind::EmpiricalCvarConstrained => {
                    current.empirical_expected_cost_ratio_units
                }
                ReservePolicyKind::TotalVariationDroCvar => {
                    current.worst_case_expected_cost_ratio_units
                }
            };
            objective < current_objective - FLOAT_EPSILON
                || ((objective - current_objective).abs() <= FLOAT_EPSILON
                    && point.reserve_power_mw < current.reserve_power_mw)
        });
        if should_replace {
            selected = Some(point);
        }
    }

    selected.map(|point| ReservePolicyDecision {
        policy,
        reserve_power_mw: point.reserve_power_mw,
        empirical_expected_cost_ratio_units: point.empirical_expected_cost_ratio_units,
        worst_case_expected_cost_ratio_units: point.worst_case_expected_cost_ratio_units,
        empirical_shortfall_probability: point.empirical_shortfall_probability,
        worst_case_shortfall_probability: point.worst_case_shortfall_probability,
        empirical_cvar_shortfall_mw: point.empirical_cvar_shortfall_mw,
        worst_case_cvar_shortfall_mw: point.worst_case_cvar_shortfall_mw,
        qualified_under_declared_risk_contract: match policy {
            ReservePolicyKind::EmpiricalCriticalFractile => true,
            ReservePolicyKind::EmpiricalCvarConstrained => {
                point.empirical_cvar_gate_met && point.empirical_chance_gate_met
            }
            ReservePolicyKind::TotalVariationDroCvar => {
                point.robust_cvar_gate_met && point.robust_chance_gate_met
            }
        },
    })
}

/// Choose three reserve policies and expose the full discrete risk/cost frontier
/// from one validated forecast scenario set. No realized grid response is inferred.
pub fn evaluate_reserve_risk(
    scenarios: &ForecastScenarioSet,
    contract: &ReserveRiskContract,
) -> Result<ReserveRiskReport, AuditError> {
    validate_scenario_set(scenarios)?;
    let tail_path_mass = contract.validate_scenario_support(scenarios.scenarios.len())?;
    if scenarios.contract.scenario_count != scenarios.scenarios.len() {
        return Err(AuditError(
            "scenario generation contract does not match the actual path count".to_string(),
        ));
    }

    let mut horizon_surplus_samples_mw = Vec::with_capacity(scenarios.scenarios.len());
    let mut scenario_ids = Vec::with_capacity(scenarios.scenarios.len());
    for scenario in &scenarios.scenarios {
        if scenario.scenario_id.trim().is_empty() {
            return Err(AuditError("scenario ID must not be blank".to_string()));
        }
        horizon_surplus_samples_mw.push(horizon_surplus(
            &scenario.power_mw,
            &scenarios.scheduled_median_power_mw,
        )?);
        scenario_ids.push(scenario.scenario_id.clone());
    }

    let (frontier, computed_tail_path_mass) =
        evaluate_distribution(&horizon_surplus_samples_mw, contract)?;
    if (computed_tail_path_mass - tail_path_mass).abs() > FLOAT_EPSILON {
        return Err(AuditError(
            "reserve-risk tail support changed during evaluation".to_string(),
        ));
    }

    let empirical_critical_fractile = select_decision(
        &frontier,
        ReservePolicyKind::EmpiricalCriticalFractile,
    )
    .ok_or_else(|| AuditError("critical-fractile reserve selection failed".to_string()))?;
    let empirical_cvar_constrained = select_decision(
        &frontier,
        ReservePolicyKind::EmpiricalCvarConstrained,
    );
    let total_variation_dro_cvar = select_decision(
        &frontier,
        ReservePolicyKind::TotalVariationDroCvar,
    );

    Ok(ReserveRiskReport {
        contract: contract.clone(),
        forecast_id: scenarios.forecast_id.clone(),
        forecast_origin_id: scenarios.forecast_origin_id.clone(),
        forecast_origin_timestamp_ns: scenarios.forecast_origin_timestamp_ns,
        forecast_evidence_id: scenarios.forecast_evidence_id.clone(),
        source_timeline_digest_reference: scenarios.source_timeline_digest_reference.clone(),
        scenario_generation_contract: scenarios.contract.clone(),
        scenario_ids,
        scenario_count: scenarios.scenarios.len(),
        horizon_steps: scenarios.contract.horizon_steps,
        horizon_surplus_samples_mw,
        effective_tail_path_mass: tail_path_mass,
        empirical_critical_fractile,
        empirical_cvar_constrained,
        total_variation_dro_cvar,
        frontier,
        claim_ceiling: RESERVE_RISK_CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> ReserveRiskContract {
        ReserveRiskContract {
            contract_id: "test-risk".to_string(),
            contract_revision: "v1".to_string(),
            decision_evidence_id: "decision-evidence".to_string(),
            tail_confidence: 0.5,
            total_variation_radius: 0.0,
            reserve_cost_ratio: 0.2,
            maximum_empirical_cvar_shortfall_mw: 100,
            maximum_robust_cvar_shortfall_mw: 100,
            maximum_shortfall_probability: 1.0,
            minimum_scenario_count: 2,
            minimum_tail_path_mass: 1.0,
            minimum_reserve_mw: 0,
            maximum_reserve_mw: 20,
            reserve_step_mw: 1,
        }
    }

    fn close(left: f64, right: f64) {
        assert!(
            (left - right).abs() < 1.0e-8,
            "expected {right}, received {left}"
        );
    }

    #[test]
    fn empirical_cvar_uses_exact_integer_tail() {
        close(empirical_cvar(&[1.0, 2.0, 3.0, 4.0], 0.5).unwrap(), 3.5);
    }

    #[test]
    fn empirical_cvar_uses_fractional_last_path() {
        close(
            empirical_cvar(&[1.0, 2.0, 3.0, 4.0], 0.625).unwrap(),
            11.0 / 3.0,
        );
    }

    #[test]
    fn total_variation_zero_returns_empirical_mean() {
        close(
            worst_case_expectation(&[0.0, 2.0, 4.0, 6.0], 0.0).unwrap(),
            3.0,
        );
    }

    #[test]
    fn total_variation_moves_probability_from_lowest_to_highest_path() {
        close(
            worst_case_expectation(&[0.0, 0.0, 0.0, 10.0], 0.5).unwrap(),
            7.5,
        );
    }

    #[test]
    fn full_total_variation_radius_concentrates_on_maximum_support() {
        close(
            worst_case_expectation(&[0.0, 0.0, 0.0, 10.0], 1.0).unwrap(),
            10.0,
        );
    }

    #[test]
    fn equal_support_values_are_unchanged_by_reweighting() {
        close(
            worst_case_expectation(&[7.0, 7.0, 7.0, 7.0], 0.75).unwrap(),
            7.0,
        );
    }

    #[test]
    fn empirical_and_robust_cvar_match_at_zero_radius() {
        let losses = [0.0, 1.0, 4.0, 8.0, 12.0];
        close(
            worst_case_cvar(&losses, 0.6, 0.0).unwrap(),
            empirical_cvar(&losses, 0.6).unwrap(),
        );
    }

    #[test]
    fn robust_cvar_is_not_below_empirical_cvar() {
        let losses = [0.0, 0.0, 2.0, 4.0, 9.0, 15.0];
        let empirical = empirical_cvar(&losses, 0.5).unwrap();
        let robust = worst_case_cvar(&losses, 0.5, 0.2).unwrap();
        assert!(robust + FLOAT_EPSILON >= empirical);
    }

    #[test]
    fn worst_case_shortfall_probability_reweights_toward_failures() {
        close(
            worst_case_expectation(&[0.0, 0.0, 1.0, 1.0], 0.25).unwrap(),
            0.75,
        );
    }

    #[test]
    fn horizon_surplus_is_the_maximum_positive_deviation() {
        assert_eq!(
            horizon_surplus(&[101, 115, 109], &[100, 110, 120]).unwrap(),
            5
        );
    }

    #[test]
    fn horizon_surplus_rejects_empty_or_misaligned_paths() {
        assert!(horizon_surplus(&[], &[]).is_err());
        assert!(horizon_surplus(&[1, 2], &[1]).is_err());
    }

    #[test]
    fn risk_contract_rejects_off_grid_maximum() {
        let mut value = contract();
        value.maximum_reserve_mw = 21;
        value.reserve_step_mw = 2;
        assert!(value.validate().is_err());
    }

    #[test]
    fn risk_contract_rejects_excessive_reserve_grid() {
        let mut value = contract();
        value.maximum_reserve_mw = 20_000;
        value.reserve_step_mw = 1;
        assert!(value.validate().is_err());
    }

    #[test]
    fn risk_contract_rejects_invalid_total_variation_radius() {
        let mut value = contract();
        value.total_variation_radius = 1.01;
        assert!(value.validate().is_err());
    }

    #[test]
    fn risk_contract_rejects_tail_support_below_declared_minimum() {
        let mut value = contract();
        value.tail_confidence = 0.95;
        value.minimum_tail_path_mass = 10.0;
        assert!(value.validate_scenario_support(100).is_err());
    }

    #[test]
    fn distribution_evaluation_rejects_too_few_scenarios() {
        let value = contract();
        assert!(evaluate_distribution(&[1], &value).is_err());
    }

    #[test]
    fn distribution_evaluation_exposes_empirical_and_robust_frontier() {
        let mut value = contract();
        value.total_variation_radius = 0.2;
        value.maximum_empirical_cvar_shortfall_mw = 4;
        value.maximum_robust_cvar_shortfall_mw = 8;
        value.maximum_shortfall_probability = 0.75;
        let (frontier, tail_mass) =
            evaluate_distribution(&[0, 2, 5, 10], &value).unwrap();
        assert_eq!(frontier.len(), 21);
        close(tail_mass, 2.0);
        assert!(frontier.iter().any(|point| point.on_worst_case_cost_risk_frontier));
        assert!(frontier.iter().all(|point| {
            point.worst_case_expected_cost_ratio_units
                + FLOAT_EPSILON
                >= point.empirical_expected_cost_ratio_units
        }));
    }

    #[test]
    fn constrained_policies_do_not_select_a_gate_violating_reserve() {
        let mut value = contract();
        value.maximum_empirical_cvar_shortfall_mw = 0;
        value.maximum_robust_cvar_shortfall_mw = 0;
        value.maximum_shortfall_probability = 0.0;
        let (frontier, _) = evaluate_distribution(&[0, 5, 10, 15], &value).unwrap();
        let empirical =
            select_decision(&frontier, ReservePolicyKind::EmpiricalCvarConstrained);
        let robust = select_decision(&frontier, ReservePolicyKind::TotalVariationDroCvar);
        assert_eq!(empirical.unwrap().reserve_power_mw, 15);
        assert_eq!(robust.unwrap().reserve_power_mw, 15);
    }

    #[test]
    fn no_feasible_tail_policy_is_reported_as_none() {
        let mut value = contract();
        value.maximum_empirical_cvar_shortfall_mw = 0;
        value.maximum_shortfall_probability = 0.0;
        value.maximum_reserve_mw = 0;
        let (frontier, _) = evaluate_distribution(&[1, 2, 3, 4], &value).unwrap();
        assert!(select_decision(
            &frontier,
            ReservePolicyKind::EmpiricalCvarConstrained
        )
        .is_none());
    }

    #[test]
    fn robust_policy_optimizes_worst_case_cost_not_empirical_cost() {
        let mut value = contract();
        value.total_variation_radius = 0.4;
        value.maximum_robust_cvar_shortfall_mw = 100;
        value.maximum_shortfall_probability = 1.0;
        let (frontier, _) = evaluate_distribution(&[0, 0, 0, 20, 30, 40], &value).unwrap();
        let decision =
            select_decision(&frontier, ReservePolicyKind::TotalVariationDroCvar).unwrap();
        assert_eq!(
            decision.worst_case_expected_cost_ratio_units,
            frontier
                .iter()
                .map(|point| point.worst_case_expected_cost_ratio_units)
                .fold(f64::INFINITY, f64::min)
        );
    }
}
