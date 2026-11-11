use crate::temporal_scenarios::{
    ScenarioEvaluationReport, ScenarioGeneratorKind,
};
use crate::power_state::PowerModelContract;
use crate::AuditError;
use std::collections::HashSet;

const EXPECTED_CLAIM_CEILING: &str =
    "MODELLED_SCENARIO_AND_MEASURED_HORIZON_SCORE_NOT_GRID_RESERVE_QUALIFICATION";

const REQUIRED_GENERATORS: [ScenarioGeneratorKind; 4] = [
    ScenarioGeneratorKind::IndependentQuantileSampling,
    ScenarioGeneratorKind::GaussianCopula,
    ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap,
    ScenarioGeneratorKind::SchaakeShuffle,
];

const CANDIDATE_GENERATORS: [ScenarioGeneratorKind; 3] = [
    ScenarioGeneratorKind::GaussianCopula,
    ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap,
    ScenarioGeneratorKind::SchaakeShuffle,
];

/// Preregistered paired-decision tournament contract.
///
/// A cluster, not an episode row, is the inferential unit. Episodes within one
/// cluster may be arbitrarily dependent. Every episode is paired across all
/// four scenario generators on the exact same forecast and measurement.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioTournamentContract {
    pub tournament_id: String,
    pub tournament_revision: String,
    pub scenario_count: usize,
    pub reserve_cost_ratio: f64,
    pub target_relative_cost_saving_fraction: f64,
    pub maximum_shortfall_episode_fraction: f64,
    /// Required lower confidence bound on the probability that an independent
    /// operating cluster achieves at least the target cost saving.
    pub minimum_cluster_target_success_probability: f64,
    pub one_sided_confidence: f64,
    pub minimum_independent_clusters_per_stratum: usize,
    pub required_strata: Vec<String>,
    pub episodes_per_cluster: usize,
    pub episode_window_duration_ns: u64,
    pub cluster_window_duration_ns: u64,
    pub minimum_cluster_separation_ns: u64,
    pub independent_cluster_assumption: bool,
    pub cluster_independence_review_evidence_id: Option<String>,
}

impl ScenarioTournamentContract {
    fn validate(&self) -> Result<(), AuditError> {
        for (value, field) in [
            (self.tournament_id.as_str(), "tournament_id"),
            (self.tournament_revision.as_str(), "tournament_revision"),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!(
                    "scenario tournament contract {field} must not be blank"
                )));
            }
        }
        if self.scenario_count < 2
            || self.scenario_count > 512
            || !self.reserve_cost_ratio.is_finite()
            || self.reserve_cost_ratio <= 0.0
            || self.reserve_cost_ratio >= 1.0
            || !self.target_relative_cost_saving_fraction.is_finite()
            || self.target_relative_cost_saving_fraction <= 0.0
            || self.target_relative_cost_saving_fraction >= 1.0
            || !self.maximum_shortfall_episode_fraction.is_finite()
            || self.maximum_shortfall_episode_fraction < 0.0
            || self.maximum_shortfall_episode_fraction > 1.0
            || !self.minimum_cluster_target_success_probability.is_finite()
            || self.minimum_cluster_target_success_probability <= 0.0
            || self.minimum_cluster_target_success_probability >= 1.0
            || !self.one_sided_confidence.is_finite()
            || self.one_sided_confidence <= 0.5
            || self.one_sided_confidence >= 1.0
        {
            return Err(AuditError(
                "scenario tournament contract has invalid scenario, cost, savings, shortfall or confidence thresholds"
                    .to_string(),
            ));
        }
        if self.minimum_independent_clusters_per_stratum == 0
            || self.episodes_per_cluster == 0
            || self.episode_window_duration_ns == 0
            || self.cluster_window_duration_ns == 0
            || self.minimum_cluster_separation_ns == 0
        {
            return Err(AuditError(
                "scenario tournament support, episode count, windows and cluster separation must be positive"
                    .to_string(),
            ));
        }
        let total_episode_duration = self
            .episode_window_duration_ns
            .checked_mul(self.episodes_per_cluster as u64)
            .ok_or_else(|| AuditError("tournament cluster exposure overflowed".to_string()))?;
        if total_episode_duration > self.cluster_window_duration_ns {
            return Err(AuditError(
                "cluster window cannot contain the frozen episode count and exposure".to_string(),
            ));
        }
        if self.required_strata.is_empty() {
            return Err(AuditError(
                "scenario tournament must declare at least one required operating stratum"
                    .to_string(),
            ));
        }
        let mut strata = HashSet::new();
        for stratum in &self.required_strata {
            if stratum.trim().is_empty() || !strata.insert(stratum.as_str()) {
                return Err(AuditError(
                    "required operating strata must be non-blank and unique".to_string(),
                ));
            }
        }
        match (
            self.independent_cluster_assumption,
            self.cluster_independence_review_evidence_id.as_deref(),
        ) {
            (true, Some(value)) if !value.trim().is_empty() => {}
            (true, _) => {
                return Err(AuditError(
                    "independent tournament clusters require an independence-review evidence reference"
                        .to_string(),
                ));
            }
            (false, Some(value)) if value.trim().is_empty() => {
                return Err(AuditError(
                    "cluster independence evidence must be absent or non-blank".to_string(),
                ));
            }
            (false, _) => {}
        }
        Ok(())
    }
}

/// One preregistered operating episode and the four paired method evaluations.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioTournamentEpisode {
    pub episode_id: String,
    pub episode_evidence_id: String,
    pub window_start_ns: u64,
    pub window_end_ns: u64,
    pub evaluations: Vec<ScenarioEvaluationReport>,
}

/// A fixed-exposure operating cluster. All episodes in it can be dependent.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioTournamentCluster {
    pub cluster_id: String,
    pub stratum_id: String,
    pub window_start_ns: u64,
    pub window_end_ns: u64,
    pub cluster_design_evidence_id: String,
    pub episodes: Vec<ScenarioTournamentEpisode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioTournamentBaselineSummary {
    pub episode_count: usize,
    pub mean_realized_cost_ratio_units: f64,
    pub shortfall_episode_count: usize,
    pub shortfall_episode_fraction: f64,
    pub total_shortfall_mw: u128,
    pub mean_shortfall_mw: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioTournamentClusterComparison {
    pub cluster_id: String,
    pub stratum_id: String,
    pub cluster_window_start_ns: u64,
    pub cluster_window_end_ns: u64,
    pub cluster_design_evidence_id: String,
    pub episode_ids: Vec<String>,
    pub episode_evidence_ids: Vec<String>,
    pub forecast_ids: Vec<String>,
    pub forecast_origin_ids: Vec<String>,
    pub measurement_run_ids: Vec<String>,
    pub measurement_evidence_ids: Vec<String>,
    pub measured_sample_evidence_ids: Vec<Vec<String>>,
    pub episode_count: usize,
    pub baseline_mean_cost_ratio_units: f64,
    pub candidate_mean_cost_ratio_units: f64,
    pub relative_cost_saving_fraction: Option<f64>,
    pub target_saving_met: bool,
    pub candidate_shortfall_episode_count: usize,
    pub candidate_shortfall_episode_fraction: f64,
    pub candidate_total_shortfall_mw: u128,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioTournamentArmSummary {
    pub generator_kind: ScenarioGeneratorKind,
    pub generator_id: String,
    pub generator_revision: String,
    pub episode_count: usize,
    pub independent_cluster_count: usize,
    pub mean_baseline_cost_ratio_units: f64,
    pub mean_candidate_cost_ratio_units: f64,
    pub relative_cost_saving_fraction: Option<f64>,
    pub target_saving_cluster_count: usize,
    pub observed_target_saving_cluster_fraction: f64,
    /// Exact one-sided Clopper–Pearson lower bound over independent clusters.
    /// Present for one stratum. The overall multi-stratum descriptive summary
    /// deliberately has no pooled confidence bound.
    pub exact_lower_confidence_bound_target_saving_probability: Option<f64>,
    pub confidence_level: Option<f64>,
    pub baseline_shortfall_episode_fraction: f64,
    pub candidate_shortfall_episode_fraction: f64,
    pub candidate_total_shortfall_mw: u128,
    pub candidate_mean_shortfall_mw: f64,
    pub minimum_cluster_support_met: bool,
    pub mean_saving_target_met: bool,
    pub shortfall_constraint_met: bool,
    pub cluster_target_reliability_met: bool,
    pub qualified_under_declared_benchmark: bool,
    pub cluster_comparisons: Vec<ScenarioTournamentClusterComparison>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioTournamentStratumResult {
    pub stratum_id: String,
    pub independent_cluster_count: usize,
    pub episode_count: usize,
    pub baseline: ScenarioTournamentBaselineSummary,
    pub candidate_arms: Vec<ScenarioTournamentArmSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioTournamentReport {
    pub contract: ScenarioTournamentContract,
    pub total_independent_cluster_count: usize,
    pub total_episode_count: usize,
    pub strata: Vec<ScenarioTournamentStratumResult>,
    pub overall_candidate_arms: Vec<ScenarioTournamentArmSummary>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
struct EpisodeArmCosts {
    baseline_cost: f64,
    candidate_costs: Vec<f64>,
    baseline_shortfall: bool,
    baseline_shortfall_mw: u64,
    candidate_shortfalls: Vec<bool>,
    candidate_shortfall_mw: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq)]
struct ClusterArmCosts {
    cluster_id: String,
    stratum_id: String,
    window_start_ns: u64,
    window_end_ns: u64,
    cluster_design_evidence_id: String,
    episode_ids: Vec<String>,
    episode_evidence_ids: Vec<String>,
    forecast_ids: Vec<String>,
    forecast_origin_ids: Vec<String>,
    measurement_run_ids: Vec<String>,
    measurement_evidence_ids: Vec<String>,
    measured_sample_evidence_ids: Vec<Vec<String>>,
    episode_costs: Vec<EpisodeArmCosts>,
}

fn log_binomial_coefficient(n: usize, k: usize) -> f64 {
    let k = k.min(n.saturating_sub(k));
    let mut total = 0.0;
    for index in 1..=k {
        total += ((n - index + 1) as f64).ln() - (index as f64).ln();
    }
    total
}

/// P[X >= successes] for X ~ Binomial(trials, probability), evaluated from
/// the first requested mass and an upward recurrence to avoid a direct sum
/// of binomial coefficients.
fn binomial_upper_tail(
    successes: usize,
    trials: usize,
    probability: f64,
) -> Option<f64> {
    if trials == 0
        || successes > trials
        || !probability.is_finite()
        || probability < 0.0
        || probability > 1.0
    {
        return None;
    }
    if successes == 0 || probability == 1.0 {
        return Some(1.0);
    }
    if probability == 0.0 {
        return Some(0.0);
    }
    let log_mass = log_binomial_coefficient(trials, successes)
        + successes as f64 * probability.ln()
        + (trials - successes) as f64 * (-probability).ln_1p();
    let mut mass = log_mass.exp();
    if !mass.is_finite() {
        return None;
    }
    let mut tail = mass;
    for value in successes..trials {
        let numerator = (trials - value) as f64 * probability;
        let denominator = (value + 1) as f64 * (1.0 - probability);
        mass *= numerator / denominator;
        tail += mass;
    }
    tail.is_finite().then_some(tail.clamp(0.0, 1.0))
}

/// Exact one-sided Clopper–Pearson lower confidence bound for a binomial
/// success probability. The unit of analysis supplied by this module is one
/// independent operating cluster, not one correlated episode.
pub fn exact_binomial_lower_confidence_bound(
    successes: usize,
    trials: usize,
    confidence: f64,
) -> Option<f64> {
    if trials == 0
        || successes > trials
        || !confidence.is_finite()
        || confidence <= 0.0
        || confidence >= 1.0
    {
        return None;
    }
    if successes == 0 {
        return Some(0.0);
    }
    let alpha = 1.0 - confidence;
    if successes == trials {
        return Some(alpha.powf(1.0 / trials as f64));
    }

    let mut lower = 0.0;
    let mut upper = successes as f64 / trials as f64;
    for _ in 0..100 {
        let middle = (lower + upper) / 2.0;
        let tail = binomial_upper_tail(successes, trials, middle)?;
        if tail < alpha {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    let result = (lower + upper) / 2.0;
    result.is_finite().then_some(result)
}

fn expected_generator_identities() -> [(ScenarioGeneratorKind, usize); 4] {
    [
        (ScenarioGeneratorKind::IndependentQuantileSampling, 0),
        (ScenarioGeneratorKind::GaussianCopula, 1),
        (
            ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap,
            2,
        ),
        (ScenarioGeneratorKind::SchaakeShuffle, 3),
    ]
}

fn validate_generator_evidence(
    report: &ScenarioEvaluationReport,
) -> Result<(), AuditError> {
    let generation = &report.scenario_generation_contract;
    generation.validate()?;
    if generation.generator_kind != report.generator_kind
        || generation.generator_id != report.generator_id
        || generation.generator_revision != report.generator_revision
        || generation.model_contract != report.model_contract
        || generation.horizon_steps != report.horizon_steps
        || generation.scenario_count != report.scenario_count
        || generation.operating_regime_id.trim().is_empty()
        || generation.clock_id != report.clock_id
        || generation.time_origin_id != report.time_origin_id
        || generation.residual_pool_evidence_id != report.residual_pool_evidence_id
    {
        return Err(AuditError(
            "scenario report does not preserve its exact scenario-generation contract"
                .to_string(),
        ));
    }

    if report.selected_residual_trajectories.len() != report.scenario_count {
        return Err(AuditError(
            "scenario evaluation is missing the complete per-scenario residual manifest"
                .to_string(),
        ));
    }

    let arrays = [
        report.selected_residual_ids.len(),
        report.selected_residual_evidence_ids.len(),
        report.selected_residual_forecast_evidence_ids.len(),
        report.selected_residual_generation_evidence_ids.len(),
        report.selected_residual_state_mapping_evidence_ids.len(),
        report.selected_residual_time_alignment_evidence_ids.len(),
        report.selected_residual_timeline_digest_references.len(),
        report.selected_residual_sample_evidence_ids.len(),
        report.selected_residual_sample_timestamps_ns.len(),
    ];

    match report.generator_kind {
        ScenarioGeneratorKind::IndependentQuantileSampling => {
            if report.residual_pool_evidence_id.is_some()
                || report.gaussian_copula_evidence.is_some()
                || report.selected_residual_trajectories.iter().any(Option::is_some)
                || arrays.iter().any(|length| *length != 0)
            {
                return Err(AuditError(
                    "independent marginal baseline must not carry residual-pool or dependence-method lineage"
                        .to_string(),
                ));
            }
        }
        ScenarioGeneratorKind::GaussianCopula => {
            let evidence = report.gaussian_copula_evidence.as_ref().ok_or_else(|| {
                AuditError("Gaussian copula report is missing its residual/correlation evidence".to_string())
            })?;
            crate::temporal_scenarios::validate_gaussian_copula_evidence(
                generation,
                &report.forecast_origin_id,
                report.forecast_origin_timestamp_ns,
                &[],
                evidence,
            )?;
            if report.residual_pool_evidence_id.as_deref()
                != Some(evidence.residual_pool_evidence_id.as_str())
                || report.selected_residual_trajectories.iter().any(Option::is_some)
                || arrays.iter().any(|length| *length != 0)
            {
                return Err(AuditError(
                    "Gaussian copula report has inconsistent residual-pool evidence or per-scenario residual attribution"
                        .to_string(),
                ));
            }
        }
        ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap
        | ScenarioGeneratorKind::SchaakeShuffle => {
            if report.gaussian_copula_evidence.is_some()
                || report.residual_pool_evidence_id.as_deref()
                    .is_none_or(|id| id.trim().is_empty())
                || arrays.iter().any(|length| *length != report.scenario_count)
                || report.selected_residual_trajectories.iter().any(Option::is_none)
                || report.selected_residual_sample_evidence_ids.iter().any(|row| {
                    row.len() != report.horizon_steps
                        || row.iter().any(|id| id.trim().is_empty())
                })
                || report.selected_residual_sample_timestamps_ns.iter().any(|row| {
                    row.len() != report.horizon_steps
                })
                || report.selected_residual_ids.iter().any(|id| id.trim().is_empty())
                || report.selected_residual_evidence_ids.iter().any(|id| id.trim().is_empty())
                || report.selected_residual_forecast_evidence_ids.iter().any(|id| id.trim().is_empty())
                || report.selected_residual_generation_evidence_ids.iter().any(|id| id.trim().is_empty())
                || report.selected_residual_state_mapping_evidence_ids.iter().any(|id| id.trim().is_empty())
                || report.selected_residual_time_alignment_evidence_ids.iter().any(|id| id.trim().is_empty())
                || report.selected_residual_timeline_digest_references.iter().any(|id| id.trim().is_empty())
            {
                return Err(AuditError(
                    "residual-dependent scenario report is missing exact selected-trajectory lineage"
                        .to_string(),
                ));
            }

            let expected_residual_duration = (report.horizon_steps as u64)
                .checked_mul(generation.step_period_ns)
                .ok_or_else(|| AuditError("tournament residual horizon duration overflowed".to_string()))?;
            let mut residual_by_id = std::collections::HashMap::new();
            for row in 0..report.scenario_count {
                let residual = report.selected_residual_trajectories[row].as_ref().ok_or_else(|| {
                    AuditError("selected complete residual trajectory is missing".to_string())
                })?;
                let mut unique_sample_ids = HashSet::new();
                if residual.residual_id != report.selected_residual_ids[row]
                    || residual.measurement_evidence_id != report.selected_residual_evidence_ids[row]
                    || residual.forecast_evidence_id != report.selected_residual_forecast_evidence_ids[row]
                    || residual.residual_generation_evidence_id != report.selected_residual_generation_evidence_ids[row]
                    || residual.state_mapping_evidence_id != report.selected_residual_state_mapping_evidence_ids[row]
                    || residual.time_alignment_evidence_id != report.selected_residual_time_alignment_evidence_ids[row]
                    || residual.source_timeline_digest_reference != report.selected_residual_timeline_digest_references[row]
                    || residual.source_sample_evidence_ids != report.selected_residual_sample_evidence_ids[row]
                    || residual.source_sample_timestamps_ns != report.selected_residual_sample_timestamps_ns[row]
                    || residual.model_contract != report.model_contract
                    || residual.operating_regime_id != generation.operating_regime_id
                    || residual.clock_id != report.clock_id
                    || residual.time_origin_id != report.time_origin_id
                    || residual.origin_id == report.forecast_origin_id
                    || residual.origin_timestamp_ns.checked_add(expected_residual_duration)
                        != Some(residual.target_window_end_ns)
                    || residual.target_window_end_ns > report.forecast_origin_timestamp_ns
                    || residual.source_sample_evidence_ids.len() != report.horizon_steps
                    || residual.source_sample_timestamps_ns.len() != report.horizon_steps
                    || residual.source_sample_evidence_ids.iter()
                        .any(|id| id.trim().is_empty() || !unique_sample_ids.insert(id.as_str()))
                {
                    return Err(AuditError(
                        "selected residual record is not aligned to its report's complete model, regime, clock, evidence and causal cutoff"
                            .to_string(),
                    ));
                }
                for sample_index in 0..report.horizon_steps {
                    let offset = (sample_index as u64 + 1)
                        .checked_mul(generation.step_period_ns)
                        .ok_or_else(|| AuditError("tournament residual sample offset overflowed".to_string()))?;
                    if residual.origin_timestamp_ns.checked_add(offset)
                        != Some(residual.source_sample_timestamps_ns[sample_index])
                    {
                        return Err(AuditError(
                            "selected residual sample timestamp does not match the historical origin and cadence"
                                .to_string(),
                        ));
                    }
                }
                if let Some(existing) = residual_by_id.get(residual.residual_id.as_str()) {
                    if *existing != residual {
                        return Err(AuditError(
                            "bootstrap repeats a residual ID with conflicting residual data".to_string(),
                        ));
                    }
                } else {
                    residual_by_id.insert(residual.residual_id.as_str(), residual);
                }
            }

            if report.generator_kind == ScenarioGeneratorKind::SchaakeShuffle {
                let unique: HashSet<&str> = report
                    .selected_residual_ids
                    .iter()
                    .map(String::as_str)
                    .collect();
                if unique.len() != report.scenario_count {
                    return Err(AuditError(
                        "Schaake-shuffle report must identify one unique historical rank trajectory per scenario"
                            .to_string(),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_evaluation_report(
    report: &ScenarioEvaluationReport,
    contract: &ScenarioTournamentContract,
) -> Result<(), AuditError> {
    if !REQUIRED_GENERATORS.contains(&report.generator_kind) {
        return Err(AuditError(
            "scenario tournament received an undeclared generator arm".to_string(),
        ));
    }
    validate_generator_evidence(report)?;
    if report.claim_ceiling != EXPECTED_CLAIM_CEILING
        || report.forecast_id.trim().is_empty()
        || report.forecast_origin_id.trim().is_empty()
        || report.forecast_evidence_id.trim().is_empty()
        || report.clock_id.trim().is_empty()
        || report.time_origin_id.trim().is_empty()
        || report.state_mapping_evidence_id.trim().is_empty()
        || report.forecast_time_alignment_evidence_id.trim().is_empty()
        || report.forecast_source_timeline_digest_reference.trim().is_empty()
        || report.measurement_run_id.trim().is_empty()
        || report.measurement_evidence_id.trim().is_empty()
        || report.measurement_time_alignment_evidence_id.trim().is_empty()
        || report.measurement_source_timeline_digest_reference.trim().is_empty()
        || report.generator_id.trim().is_empty()
        || report.generator_revision.trim().is_empty()
        || report.scenario_count != contract.scenario_count
        || report.reserve_cost_ratio != contract.reserve_cost_ratio
        || report.reserve_critical_fractile != 1.0 - contract.reserve_cost_ratio
        || report.horizon_steps < 2
        || report.measured_sample_evidence_ids.len() != report.horizon_steps
        || report.measured_sample_evidence_ids.iter().any(|id| id.trim().is_empty())
    {
        return Err(AuditError(
            "scenario evaluation report is missing provenance or mismatches the frozen tournament contract"
                .to_string(),
        ));
    }
    if !report.energy_score_mw.is_finite()
        || report.energy_score_mw < 0.0
        || !report.variogram_score.is_finite()
        || report.variogram_score < 0.0
        || !report.realized_reserve_cost_ratio_units.is_finite()
        || report.realized_reserve_cost_ratio_units < 0.0
    {
        return Err(AuditError(
            "scenario evaluation report has a non-finite or negative score/cost".to_string(),
        ));
    }
    let expected_shortfall = report
        .actual_horizon_max_surplus_mw
        .saturating_sub(report.reserve_power_mw);
    if expected_shortfall != report.actual_shortfall_mw
        || report.reserve_shortfall_occurred != (report.actual_shortfall_mw > 0)
    {
        return Err(AuditError(
            "scenario evaluation report has inconsistent reserve shortfall fields".to_string(),
        ));
    }
    let expected_cost = contract.reserve_cost_ratio * report.reserve_power_mw as f64
        + report.actual_shortfall_mw as f64;
    if (expected_cost - report.realized_reserve_cost_ratio_units).abs() > 1e-8 {
        return Err(AuditError(
            "scenario evaluation report realized reserve cost does not match its power and shortfall quantities"
                .to_string(),
        ));
    }
    Ok(())
}

fn ensure_episode_reports_paired(
    reports: &[ScenarioEvaluationReport],
    contract: &ScenarioTournamentContract,
) -> Result<[ScenarioEvaluationReport; 4], AuditError> {
    if reports.len() != REQUIRED_GENERATORS.len() {
        return Err(AuditError(
            "every scenario tournament episode must contain exactly four generator evaluations"
                .to_string(),
        ));
    }
    let mut ordered: Vec<Option<ScenarioEvaluationReport>> = vec![None; REQUIRED_GENERATORS.len()];
    for report in reports {
        validate_evaluation_report(report, contract)?;
        let (_, index) = expected_generator_identities()
            .iter()
            .find(|(kind, _)| *kind == report.generator_kind)
            .copied()
            .ok_or_else(|| AuditError("unknown scenario generator kind".to_string()))?;
        if ordered[index].replace(report.clone()).is_some() {
            return Err(AuditError(
                "scenario tournament episode contains a duplicate generator arm".to_string(),
            ));
        }
    }
    let ordered: Vec<ScenarioEvaluationReport> = ordered
        .into_iter()
        .map(|report| {
            report.ok_or_else(|| {
                AuditError("scenario tournament episode is missing a required generator arm".to_string())
            })
        })
        .collect::<Result<_, _>>()?;
    let baseline = &ordered[0];
    let gaussian_report = ordered
        .iter()
        .find(|report| report.generator_kind == ScenarioGeneratorKind::GaussianCopula)
        .ok_or_else(|| AuditError("paired tournament is missing its Gaussian-copula residual pool".to_string()))?;
    let gaussian_pool = gaussian_report.gaussian_copula_evidence.as_ref().ok_or_else(|| {
        AuditError("paired Gaussian-copula report is missing its historical residual pool".to_string())
    })?;
    for report in ordered.iter().filter(|report| {
        matches!(
            report.generator_kind,
            ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap
                | ScenarioGeneratorKind::SchaakeShuffle
        )
    }) {
        for residual in report.selected_residual_trajectories.iter().flatten() {
            let pool_index = gaussian_pool.residual_ids
                .iter()
                .position(|id| id == &residual.residual_id)
                .ok_or_else(|| {
                    AuditError("selected residual trajectory is absent from the paired Gaussian historical pool".to_string())
                })?;
            if gaussian_pool.residual_origin_ids[pool_index] != residual.origin_id
                || gaussian_pool.residual_origin_timestamps_ns[pool_index] != residual.origin_timestamp_ns
                || gaussian_pool.residual_target_window_end_timestamps_ns[pool_index]
                    != residual.target_window_end_ns
                || gaussian_pool.residual_operating_regime_ids[pool_index]
                    != residual.operating_regime_id
                || gaussian_pool.residual_clock_ids[pool_index] != residual.clock_id
                || gaussian_pool.residual_time_origin_ids[pool_index] != residual.time_origin_id
                || gaussian_pool.residual_model_contracts[pool_index] != residual.model_contract
                || gaussian_pool.raw_residual_power_mw[pool_index] != residual.residual_power_mw
                || gaussian_pool.measurement_evidence_ids[pool_index] != residual.measurement_evidence_id
                || gaussian_pool.forecast_evidence_ids[pool_index] != residual.forecast_evidence_id
                || gaussian_pool.residual_generation_evidence_ids[pool_index]
                    != residual.residual_generation_evidence_id
                || gaussian_pool.state_mapping_evidence_ids[pool_index]
                    != residual.state_mapping_evidence_id
                || gaussian_pool.time_alignment_evidence_ids[pool_index]
                    != residual.time_alignment_evidence_id
                || gaussian_pool.timeline_digest_references[pool_index]
                    != residual.source_timeline_digest_reference
                || gaussian_pool.source_sample_evidence_ids[pool_index]
                    != residual.source_sample_evidence_ids
                || gaussian_pool.source_sample_timestamps_ns[pool_index]
                    != residual.source_sample_timestamps_ns
            {
                return Err(AuditError(
                    "bootstrap/Schaake selected residual differs from the shared historical residual pool"
                        .to_string(),
                ));
            }
        }
    }

    let reference_residual_pool = ordered
        .iter()
        .filter(|report| report.generator_kind != ScenarioGeneratorKind::IndependentQuantileSampling)
        .next()
        .and_then(|report| report.residual_pool_evidence_id.as_deref())
        .ok_or_else(|| AuditError("paired dependence arms have no common residual pool reference".to_string()))?;
    for report in ordered.iter().skip(1) {
        if report.generator_kind != ScenarioGeneratorKind::IndependentQuantileSampling
            && report.residual_pool_evidence_id.as_deref() != Some(reference_residual_pool)
        {
            return Err(AuditError(
                "paired dependence arms do not use the same frozen residual-pool evidence".to_string(),
            ));
        }
        if report.forecast_id != baseline.forecast_id
            || report.forecast_origin_id != baseline.forecast_origin_id
            || report.forecast_origin_timestamp_ns != baseline.forecast_origin_timestamp_ns
            || report.scenario_generation_contract.operating_regime_id
                != baseline.scenario_generation_contract.operating_regime_id
            || report.scenario_generation_contract.clock_id
                != baseline.scenario_generation_contract.clock_id
            || report.scenario_generation_contract.time_origin_id
                != baseline.scenario_generation_contract.time_origin_id
            || report.scenario_generation_contract.step_period_ns
                != baseline.scenario_generation_contract.step_period_ns
            || report.scenario_generation_contract.horizon_steps
                != baseline.scenario_generation_contract.horizon_steps
            || report.scenario_generation_contract.deterministic_seed
                != baseline.scenario_generation_contract.deterministic_seed
            || report.model_contract != baseline.model_contract
            || report.forecast_evidence_id != baseline.forecast_evidence_id
            || report.clock_id != baseline.clock_id
            || report.time_origin_id != baseline.time_origin_id
            || report.state_mapping_evidence_id != baseline.state_mapping_evidence_id
            || report.forecast_time_alignment_evidence_id != baseline.forecast_time_alignment_evidence_id
            || report.forecast_source_timeline_digest_reference
                != baseline.forecast_source_timeline_digest_reference
            || report.measurement_run_id != baseline.measurement_run_id
            || report.measurement_evidence_id != baseline.measurement_evidence_id
            || report.measurement_time_alignment_evidence_id
                != baseline.measurement_time_alignment_evidence_id
            || report.measurement_source_timeline_digest_reference
                != baseline.measurement_source_timeline_digest_reference
            || report.measured_sample_evidence_ids != baseline.measured_sample_evidence_ids
            || report.scenario_count != baseline.scenario_count
            || report.horizon_steps != baseline.horizon_steps
            || report.actual_energy_mw_ns != baseline.actual_energy_mw_ns
            || report.actual_peak_power_mw != baseline.actual_peak_power_mw
            || report.actual_max_ramp_up_mw_per_step != baseline.actual_max_ramp_up_mw_per_step
            || report.actual_max_ramp_down_mw_per_step != baseline.actual_max_ramp_down_mw_per_step
            || report.actual_horizon_max_surplus_mw != baseline.actual_horizon_max_surplus_mw
            || report.reserve_cost_ratio != baseline.reserve_cost_ratio
            || report.reserve_critical_fractile != baseline.reserve_critical_fractile
        {
            return Err(AuditError(
                "scenario generator arms are not paired on the exact same forecast, measurement, horizon and reserve-cost contract"
                    .to_string(),
            ));
        }
    }
    ordered.try_into().map_err(|_| {
        AuditError("scenario tournament generator ordering failed".to_string())
    })
}

fn frozen_generator_identity(
    versions: &[(ScenarioGeneratorKind, String, String)],
    kind: ScenarioGeneratorKind,
) -> Result<(&str, &str), AuditError> {
    let (_, generator_id, revision) = versions
        .iter()
        .find(|(version_kind, _, _)| *version_kind == kind)
        .ok_or_else(|| {
            AuditError(format!("scenario tournament is missing generator identity for {kind:?}"))
        })?;
    Ok((generator_id.as_str(), revision.as_str()))
}

fn summarize_baseline(
    costs: &[ClusterArmCosts],
) -> Result<ScenarioTournamentBaselineSummary, AuditError> {
    let mut episodes = 0_usize;
    let mut shortfalls = 0_usize;
    let mut total_cost = 0.0_f64;
    let mut total_shortfall = 0_u128;
    for cluster in costs {
        for episode in &cluster.episode_costs {
            episodes = episodes.checked_add(1)
                .ok_or_else(|| AuditError("tournament baseline episode count overflowed".to_string()))?;
            shortfalls = shortfalls.checked_add(usize::from(episode.baseline_shortfall))
                .ok_or_else(|| AuditError("tournament baseline shortfall count overflowed".to_string()))?;
            total_cost += episode.baseline_cost;
            total_shortfall = total_shortfall
                .checked_add(episode.baseline_shortfall_mw as u128)
                .ok_or_else(|| AuditError("baseline total shortfall overflowed".to_string()))?;
        }
    }
    if episodes == 0 || !total_cost.is_finite() {
        return Err(AuditError("tournament baseline has no valid episodes".to_string()));
    }
    Ok(ScenarioTournamentBaselineSummary {
        episode_count: episodes,
        mean_realized_cost_ratio_units: total_cost / episodes as f64,
        shortfall_episode_count: shortfalls,
        shortfall_episode_fraction: shortfalls as f64 / episodes as f64,
        total_shortfall_mw: total_shortfall,
        mean_shortfall_mw: total_shortfall as f64 / episodes as f64,
    })
}

fn summarize_arm(
    contract: &ScenarioTournamentContract,
    generator_kind: ScenarioGeneratorKind,
    clusters: &[ClusterArmCosts],
    baseline_shortfall_fraction: f64,
    generator_id: &str,
    generator_revision: &str,
) -> Result<ScenarioTournamentArmSummary, AuditError> {
    let mut total_episodes = 0_usize;
    let mut total_baseline_cost = 0.0_f64;
    let mut total_candidate_cost = 0.0_f64;
    let mut candidate_shortfall_count = 0_usize;
    let mut candidate_shortfall_mw = 0_u128;
    let mut target_success_clusters = 0_usize;
    let mut cluster_comparisons = Vec::with_capacity(clusters.len());
    for cluster in clusters {
        let mut baseline_cost = 0.0_f64;
        let mut candidate_cost = 0.0_f64;
        let mut shortfall_count = 0_usize;
        let mut cluster_shortfall_mw = 0_u128;
        let mut episode_count = 0_usize;
        for episode in &cluster.episode_costs {
            let candidate_index = match generator_kind {
                ScenarioGeneratorKind::GaussianCopula => 1,
                ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap => 2,
                ScenarioGeneratorKind::SchaakeShuffle => 3,
                ScenarioGeneratorKind::IndependentQuantileSampling => {
                    return Err(AuditError(
                        "the independent marginal arm is the baseline, not a candidate".to_string(),
                    ));
                }
            };
            baseline_cost += episode.baseline_cost;
            candidate_cost += episode.candidate_costs[candidate_index];
            if episode.candidate_shortfalls[candidate_index] {
                shortfall_count += 1;
            }
            cluster_shortfall_mw = cluster_shortfall_mw
                .checked_add(episode.candidate_shortfall_mw[candidate_index] as u128)
                .ok_or_else(|| AuditError("candidate cluster shortfall sum overflowed".to_string()))?;
            episode_count += 1;
        }
        if episode_count != contract.episodes_per_cluster
            || !baseline_cost.is_finite()
            || !candidate_cost.is_finite()
        {
            return Err(AuditError(
                "candidate cluster has invalid exposure or non-finite cost totals".to_string(),
            ));
        }
        let baseline_mean = baseline_cost / episode_count as f64;
        let candidate_mean = candidate_cost / episode_count as f64;
        let saving = if baseline_mean > 0.0 {
            Some((baseline_mean - candidate_mean) / baseline_mean)
        } else {
            None
        };
        let target_met = saving.is_some_and(|value| {
            value >= contract.target_relative_cost_saving_fraction
        });
        target_success_clusters += usize::from(target_met);
        let shortfall_fraction = shortfall_count as f64 / episode_count as f64;

        // Generator identity/revision must remain frozen across all episodes.
        // The actual strings are also checked during pairing in the outer API.
        cluster_comparisons.push(ScenarioTournamentClusterComparison {
            cluster_id: cluster.cluster_id.clone(),
            stratum_id: cluster.stratum_id.clone(),
            cluster_window_start_ns: cluster.window_start_ns,
            cluster_window_end_ns: cluster.window_end_ns,
            cluster_design_evidence_id: cluster.cluster_design_evidence_id.clone(),
            episode_ids: cluster.episode_ids.clone(),
            episode_evidence_ids: cluster.episode_evidence_ids.clone(),
            forecast_ids: cluster.forecast_ids.clone(),
            forecast_origin_ids: cluster.forecast_origin_ids.clone(),
            measurement_run_ids: cluster.measurement_run_ids.clone(),
            measurement_evidence_ids: cluster.measurement_evidence_ids.clone(),
            measured_sample_evidence_ids: cluster.measured_sample_evidence_ids.clone(),
            episode_count,
            baseline_mean_cost_ratio_units: baseline_mean,
            candidate_mean_cost_ratio_units: candidate_mean,
            relative_cost_saving_fraction: saving,
            target_saving_met: target_met,
            candidate_shortfall_episode_count: shortfall_count,
            candidate_shortfall_episode_fraction: shortfall_fraction,
            candidate_total_shortfall_mw: cluster_shortfall_mw,
        });
        total_episodes += episode_count;
        total_baseline_cost += baseline_cost;
        total_candidate_cost += candidate_cost;
        candidate_shortfall_count += shortfall_count;
        candidate_shortfall_mw = candidate_shortfall_mw
            .checked_add(cluster_shortfall_mw)
            .ok_or_else(|| AuditError("candidate shortfall total overflowed".to_string()))?;
    }

    if total_episodes == 0
        || !total_baseline_cost.is_finite()
        || !total_candidate_cost.is_finite()
        || total_baseline_cost <= 0.0
    {
        return Err(AuditError(
            "paired tournament needs positive finite baseline reserve-cost support".to_string(),
        ));
    }

    // This lower bound uses independent clusters as trials. Episodes inside a
    // cluster never increase the binomial denominator.
    let lower_bound = exact_binomial_lower_confidence_bound(
        target_success_clusters,
        clusters.len(),
        contract.one_sided_confidence,
    )
    .ok_or_else(|| AuditError("could not calculate exact cluster-success bound".to_string()))?;
    let saving_fraction = (total_baseline_cost - total_candidate_cost) / total_baseline_cost;
    let shortfall_fraction = candidate_shortfall_count as f64 / total_episodes as f64;
    let mean_shortfall_mw = candidate_shortfall_mw as f64 / total_episodes as f64;
    let minimum_cluster_support_met =
        clusters.len() >= contract.minimum_independent_clusters_per_stratum;
    let mean_saving_target_met =
        saving_fraction >= contract.target_relative_cost_saving_fraction;
    let shortfall_constraint_met =
        shortfall_fraction <= contract.maximum_shortfall_episode_fraction;
    let cluster_target_reliability_met =
        lower_bound >= contract.minimum_cluster_target_success_probability;

    Ok(ScenarioTournamentArmSummary {
        generator_kind,
        generator_id: generator_id.to_string(),
        generator_revision: generator_revision.to_string(),
        episode_count: total_episodes,
        independent_cluster_count: clusters.len(),
        mean_baseline_cost_ratio_units: total_baseline_cost / total_episodes as f64,
        mean_candidate_cost_ratio_units: total_candidate_cost / total_episodes as f64,
        relative_cost_saving_fraction: Some(saving_fraction),
        target_saving_cluster_count: target_success_clusters,
        observed_target_saving_cluster_fraction: target_success_clusters as f64 / clusters.len() as f64,
        exact_lower_confidence_bound_target_saving_probability: Some(lower_bound),
        confidence_level: Some(contract.one_sided_confidence),
        baseline_shortfall_episode_fraction,
        candidate_shortfall_episode_fraction: shortfall_fraction,
        candidate_total_shortfall_mw: candidate_shortfall_mw,
        candidate_mean_shortfall_mw: mean_shortfall_mw,
        minimum_cluster_support_met,
        mean_saving_target_met,
        shortfall_constraint_met,
        cluster_target_reliability_met,
        qualified_under_declared_benchmark: contract.independent_cluster_assumption
            && minimum_cluster_support_met
            && mean_saving_target_met
            && shortfall_constraint_met
            && cluster_target_reliability_met,
        cluster_comparisons,
    })
}

fn build_cluster_costs(
    cluster: &ScenarioTournamentCluster,
    paired_reports: &[(ScenarioTournamentEpisode, [ScenarioEvaluationReport; 4])],
) -> Result<ClusterArmCosts, AuditError> {
    let mut episode_costs = Vec::with_capacity(paired_reports.len());
    let mut episode_ids = Vec::with_capacity(paired_reports.len());
    let mut episode_evidence_ids = Vec::with_capacity(paired_reports.len());
    let mut forecast_ids = Vec::with_capacity(paired_reports.len());
    let mut forecast_origin_ids = Vec::with_capacity(paired_reports.len());
    let mut measurement_run_ids = Vec::with_capacity(paired_reports.len());
    let mut measurement_evidence_ids = Vec::with_capacity(paired_reports.len());
    let mut measured_sample_evidence_ids = Vec::with_capacity(paired_reports.len());
    for (episode, reports) in paired_reports {
        let baseline = &reports[0];
        let mut candidate_costs = Vec::with_capacity(3);
        let mut candidate_shortfalls = Vec::with_capacity(3);
        let mut candidate_shortfall_mw = Vec::with_capacity(3);
        for candidate in reports.iter().skip(1) {
            candidate_costs.push(candidate.realized_reserve_cost_ratio_units);
            candidate_shortfalls.push(candidate.reserve_shortfall_occurred);
            candidate_shortfall_mw.push(candidate.actual_shortfall_mw);
        }
        episode_ids.push(episode.episode_id.clone());
        episode_evidence_ids.push(episode.episode_evidence_id.clone());
        forecast_ids.push(baseline.forecast_id.clone());
        forecast_origin_ids.push(baseline.forecast_origin_id.clone());
        measurement_run_ids.push(baseline.measurement_run_id.clone());
        measurement_evidence_ids.push(baseline.measurement_evidence_id.clone());
        measured_sample_evidence_ids.push(baseline.measured_sample_evidence_ids.clone());
        episode_costs.push(EpisodeArmCosts {
            baseline_cost: baseline.realized_reserve_cost_ratio_units,
            candidate_costs,
            baseline_shortfall: baseline.reserve_shortfall_occurred,
            baseline_shortfall_mw: baseline.actual_shortfall_mw,
            candidate_shortfalls,
            candidate_shortfall_mw,
        });
        let _ = episode;
    }
    Ok(ClusterArmCosts {
        cluster_id: cluster.cluster_id.clone(),
        stratum_id: cluster.stratum_id.clone(),
        window_start_ns: cluster.window_start_ns,
        window_end_ns: cluster.window_end_ns,
        cluster_design_evidence_id: cluster.cluster_design_evidence_id.clone(),
        episode_ids,
        episode_evidence_ids,
        forecast_ids,
        forecast_origin_ids,
        measurement_run_ids,
        measurement_evidence_ids,
        measured_sample_evidence_ids,
        episode_costs,
    })
}

/// Paired, cluster-aware comparison of the four temporal-scenario generators.
///
/// The function accepts only reports produced by evaluate_forecast_scenarios.
/// Every method sees the same forecast origin, model, measurement, horizon,
/// scenario count and reserve/shortfall ratio. The statistical success trial is
/// one independent cluster; all episodes inside a cluster are first aggregated.
pub fn evaluate_scenario_tournament(
    contract: ScenarioTournamentContract,
    clusters: Vec<ScenarioTournamentCluster>,
) -> Result<ScenarioTournamentReport, AuditError> {
    contract.validate()?;
    if !contract.independent_cluster_assumption {
        return Err(AuditError(
            "scenario tournament qualification requires a reviewed independent-cluster design"
                .to_string(),
        ));
    }
    if clusters.is_empty() {
        return Err(AuditError(
            "scenario tournament requires at least one operating cluster".to_string(),
        ));
    }

    let required_strata: HashSet<&str> =
        contract.required_strata.iter().map(String::as_str).collect();
    let mut cluster_ids = HashSet::new();
    let mut episode_ids = HashSet::new();
    let mut forecast_ids = HashSet::new();
    let mut measurement_run_ids = HashSet::new();
    let mut episode_evidence_ids = HashSet::new();
    let mut measurement_evidence_ids = HashSet::new();
    let mut measured_sample_evidence_ids = HashSet::new();
    let mut generator_versions: Vec<(ScenarioGeneratorKind, String, String)> = Vec::new();

    let mut ordered_clusters: Vec<ScenarioTournamentCluster> = clusters;
    ordered_clusters.sort_by_key(|cluster| cluster.window_start_ns);
    let mut previous_cluster_end = None;
    for cluster in &ordered_clusters {
        for (value, field) in [
            (cluster.cluster_id.as_str(), "cluster_id"),
            (cluster.stratum_id.as_str(), "stratum_id"),
            (cluster.cluster_design_evidence_id.as_str(), "cluster_design_evidence_id"),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!(
                    "scenario tournament cluster {field} must not be blank"
                )));
            }
        }
        if !cluster_ids.insert(cluster.cluster_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate scenario tournament cluster {:?}",
                cluster.cluster_id
            )));
        }
        if !required_strata.contains(cluster.stratum_id.as_str()) {
            return Err(AuditError(format!(
                "scenario tournament cluster uses undeclared stratum {:?}",
                cluster.stratum_id
            )));
        }
        if cluster.window_end_ns.checked_sub(cluster.window_start_ns)
            != Some(contract.cluster_window_duration_ns)
        {
            return Err(AuditError(format!(
                "scenario tournament cluster {:?} differs from the frozen cluster exposure",
                cluster.cluster_id
            )));
        }
        if previous_cluster_end.is_some_and(|end| {
            end.checked_add(contract.minimum_cluster_separation_ns)
                .is_none_or(|start| cluster.window_start_ns < start)
        }) {
            return Err(AuditError(
                "scenario tournament cluster windows overlap or violate the frozen independence separation"
                    .to_string(),
            ));
        }
        previous_cluster_end = Some(cluster.window_end_ns);
        if cluster.episodes.len() != contract.episodes_per_cluster {
            return Err(AuditError(format!(
                "cluster {:?} has {} episodes; exact frozen count is {}",
                cluster.cluster_id,
                cluster.episodes.len(),
                contract.episodes_per_cluster
            )));
        }

        let mut episodes: Vec<&ScenarioTournamentEpisode> = cluster.episodes.iter().collect();
        episodes.sort_by_key(|episode| episode.window_start_ns);
        let mut previous_episode_end = None;
        for episode in episodes {
            if episode.episode_id.trim().is_empty() || episode.episode_evidence_id.trim().is_empty() {
                return Err(AuditError(
                    "scenario tournament episode ID and evidence must not be blank".to_string(),
                ));
            }
            if !episode_ids.insert(episode.episode_id.as_str())
                || !episode_evidence_ids.insert(episode.episode_evidence_id.as_str())
            {
                return Err(AuditError(
                    "scenario tournament repeats an episode or episode evidence ID".to_string(),
                ));
            }
            if episode.window_end_ns.checked_sub(episode.window_start_ns)
                != Some(contract.episode_window_duration_ns)
                || episode.window_start_ns < cluster.window_start_ns
                || episode.window_end_ns > cluster.window_end_ns
            {
                return Err(AuditError(format!(
                    "episode {:?} duration or containment differs from its cluster contract",
                    episode.episode_id
                )));
            }
            if previous_episode_end.is_some_and(|end| episode.window_start_ns < end) {
                return Err(AuditError(
                    "scenario tournament episode windows overlap inside a cluster".to_string(),
                ));
            }
            previous_episode_end = Some(episode.window_end_ns);

            let paired = ensure_episode_reports_paired(&episode.evaluations, &contract)?;
            if paired.iter().any(|report| {
                report.forecast_origin_timestamp_ns < episode.window_start_ns
                    || report.forecast_origin_timestamp_ns >= episode.window_end_ns
            }) {
                return Err(AuditError(
                    "paired forecast origin timestamp falls outside the preregistered episode window"
                        .to_string(),
                ));
            }
            let baseline = &paired[0];
            if !forecast_ids.insert(baseline.forecast_id.as_str())
                || !measurement_run_ids.insert(baseline.measurement_run_id.as_str())
                || !measurement_evidence_ids.insert(baseline.measurement_evidence_id.as_str())
            {
                return Err(AuditError(
                    "scenario tournament reuses a forecast origin or physical measurement run across episodes"
                        .to_string(),
                ));
            }
            for evidence_id in &baseline.measured_sample_evidence_ids {
                if !measured_sample_evidence_ids.insert(evidence_id.as_str()) {
                    return Err(AuditError(
                        "scenario tournament reuses a measured source-sample evidence ID across episodes"
                            .to_string(),
                    ));
                }
            }
            for report in &paired {
                if let Some((_, generator_id, revision)) = generator_versions
                    .iter()
                    .find(|(kind, _, _)| *kind == report.generator_kind)
                {
                    if generator_id != &report.generator_id || revision != &report.generator_revision {
                        return Err(AuditError(format!(
                            "generator {:?} ID or revision changed inside the frozen tournament",
                            report.generator_kind
                        )));
                    }
                } else {
                    generator_versions.push((
                        report.generator_kind,
                        report.generator_id.clone(),
                        report.generator_revision.clone(),
                    ));
                }
            }
        }
    }

    // Freeze version identities for all four arms before summarization.
    if generator_versions.len() != REQUIRED_GENERATORS.len() {
        return Err(AuditError(
            "scenario tournament did not observe all four generator identities".to_string(),
        ));
    }

    let mut clusters_by_stratum: Vec<(String, Vec<ClusterArmCosts>)> = Vec::new();
    for stratum in &contract.required_strata {
        let selected: Vec<&ScenarioTournamentCluster> = ordered_clusters
            .iter()
            .filter(|cluster| &cluster.stratum_id == stratum)
            .collect();
        if selected.is_empty() {
            return Err(AuditError(format!(
                "required tournament stratum {stratum:?} has no independent clusters"
            )));
        }

        let mut costs = Vec::with_capacity(selected.len());
        for cluster in selected {
            let mut paired_reports = Vec::with_capacity(cluster.episodes.len());
            for episode in &cluster.episodes {
                paired_reports.push((
                    episode.clone(),
                    ensure_episode_reports_paired(&episode.evaluations, &contract)?,
                ));
            }
            costs.push(build_cluster_costs(cluster, &paired_reports)?);
        }
        clusters_by_stratum.push((stratum.clone(), costs));
    }

    let mut stratum_results = Vec::with_capacity(clusters_by_stratum.len());
    for (stratum_id, costs) in &clusters_by_stratum {
        let baseline = summarize_baseline(costs)?;
        let mut candidate_arms = Vec::with_capacity(CANDIDATE_GENERATORS.len());
        // Bonferroni adjustment makes the collection of one-sided stratum
        // bounds a simultaneous familywise confidence statement.
        let adjusted_stratum_confidence =
            1.0 - (1.0 - contract.one_sided_confidence) / contract.required_strata.len() as f64;
        let adjusted_contract = ScenarioTournamentContract {
            one_sided_confidence: adjusted_stratum_confidence,
            ..contract.clone()
        };
        for generator_kind in CANDIDATE_GENERATORS {
            let (generator_id, generator_revision) =
                frozen_generator_identity(&generator_versions, generator_kind)?;
            candidate_arms.push(summarize_arm(
                &adjusted_contract,
                generator_kind,
                costs,
                baseline.shortfall_episode_fraction,
                generator_id,
                generator_revision,
            )?);
        }
        stratum_results.push(ScenarioTournamentStratumResult {
            stratum_id: stratum_id.clone(),
            independent_cluster_count: costs.len(),
            episode_count: baseline.episode_count,
            baseline,
            candidate_arms,
        });
    }

    // Aggregate all strata while preserving cluster identity. Required-stratum
    // qualification is still separate; the overall comparison never replaces it.
    let all_costs: Vec<ClusterArmCosts> = clusters_by_stratum
        .iter()
        .flat_map(|(_, costs)| costs.clone())
        .collect();
    let all_baseline = summarize_baseline(&all_costs)?;
    let mut overall_candidate_arms = Vec::with_capacity(CANDIDATE_GENERATORS.len());
    for generator_kind in CANDIDATE_GENERATORS {
        let (generator_id, generator_revision) =
            frozen_generator_identity(&generator_versions, generator_kind)?;
        let mut summary = summarize_arm(
            &contract,
            generator_kind,
            &all_costs,
            all_baseline.shortfall_episode_fraction,
            generator_id,
            generator_revision,
        )?;
        // Do not pool distinct seasonal/workload strata into one binomial
        // population. Overall costs are descriptive; qualification is the
        // conjunction of separately multiplicity-adjusted stratum results.
        summary.exact_lower_confidence_bound_target_saving_probability = None;
        summary.confidence_level = None;
        let corresponding_arms: Vec<&ScenarioTournamentArmSummary> = stratum_results
            .iter()
            .filter_map(|stratum| {
                stratum.candidate_arms
                    .iter()
                    .find(|arm| arm.generator_kind == generator_kind)
            })
            .collect();
        if corresponding_arms.len() != stratum_results.len() {
            return Err(AuditError(
                "overall scenario summary is missing a required stratum arm".to_string(),
            ));
        }
        summary.minimum_cluster_support_met =
            corresponding_arms.iter().all(|arm| arm.minimum_cluster_support_met);
        summary.mean_saving_target_met =
            corresponding_arms.iter().all(|arm| arm.mean_saving_target_met);
        summary.shortfall_constraint_met =
            corresponding_arms.iter().all(|arm| arm.shortfall_constraint_met);
        summary.cluster_target_reliability_met =
            corresponding_arms.iter().all(|arm| arm.cluster_target_reliability_met);
        summary.qualified_under_declared_benchmark = contract.independent_cluster_assumption
            && summary.minimum_cluster_support_met
            && summary.mean_saving_target_met
            && summary.shortfall_constraint_met
            && summary.cluster_target_reliability_met;
        overall_candidate_arms.push(summary);
    }

    let total_episode_count = ordered_clusters
        .iter()
        .try_fold(0_usize, |sum, cluster| {
            sum.checked_add(cluster.episodes.len())
                .ok_or_else(|| AuditError("scenario tournament total episode count overflowed".to_string()))
        })?;
    Ok(ScenarioTournamentReport {
        contract: contract.clone(),
        total_independent_cluster_count: ordered_clusters.len(),
        total_episode_count,
        strata: stratum_results,
        overall_candidate_arms,
        claim_ceiling: "OFFLINE_PAIRED_SCENARIO_TOURNAMENT_NOT_RESERVE_DELIVERY_OR_GRID_ACCREDITATION",
    })
}

#[cfg(test)]
mod tests {
    use super::{
        binomial_upper_tail, evaluate_scenario_tournament,
        exact_binomial_lower_confidence_bound, ScenarioTournamentCluster,
        ScenarioTournamentContract, ScenarioTournamentEpisode,
    };
    use crate::temporal_scenarios::{
        GaussianCopulaEvidence, ScenarioEvaluationReport, ScenarioGeneratorKind,
    };
    use crate::power_state::PowerModelContract;

    fn model_contract() -> PowerModelContract {
        PowerModelContract {
            model_id: "power-state-v1".to_string(),
            model_revision: "rev-a".to_string(),
            hardware_configuration_fingerprint: "hardware-sha".to_string(),
            measurement_boundary_id: "pcc-meter-v1".to_string(),
            sensor_id: "sensor-1".to_string(),
            sensor_calibration_evidence_id: "sensor-cert-1".to_string(),
            sensor_uncertainty_mw: 10,
            dataset_id: "dataset-1".to_string(),
            dataset_version: "v1".to_string(),
            dataset_evidence_id: "dataset-evidence-1".to_string(),
            use_authorization_reference: "rights-1".to_string(),
            normalization_id: "norm-v1".to_string(),
            workload_contract_id: "workload-v1".to_string(),
            state_mapper_id: "mapper-v1".to_string(),
            clock_alignment_method_id: "ptp-v1".to_string(),
            sample_period_ns: 10,
            minimum_samples_per_state: 1,
            minimum_samples_per_transition: 1,
        }
    }

    fn contract() -> ScenarioTournamentContract {
        ScenarioTournamentContract {
            tournament_id: "scenario-tournament-v1".to_string(),
            tournament_revision: "rev-1".to_string(),
            scenario_count: 200,
            reserve_cost_ratio: 0.10,
            target_relative_cost_saving_fraction: 0.074,
            maximum_shortfall_episode_fraction: 0.10,
            minimum_cluster_target_success_probability: 0.50,
            one_sided_confidence: 0.90,
            minimum_independent_clusters_per_stratum: 4,
            required_strata: vec!["normal".to_string()],
            episodes_per_cluster: 2,
            episode_window_duration_ns: 10,
            cluster_window_duration_ns: 30,
            minimum_cluster_separation_ns: 10,
            independent_cluster_assumption: true,
            cluster_independence_review_evidence_id: Some("cluster-design-review-v1".to_string()),
        }
    }

    fn residual_fixture(index: usize) -> crate::temporal_scenarios::ForecastResidualTrajectory {
        let origin = 100 + index as u64 * 45;
        crate::temporal_scenarios::ForecastResidualTrajectory {
            residual_id: format!("residual-{index:03}"),
            origin_id: format!("residual-origin-{index:03}"),
            operating_regime_id: "normal".to_string(),
            clock_id: "clock-common".to_string(),
            time_origin_id: "time-origin-common".to_string(),
            origin_timestamp_ns: origin,
            target_window_end_ns: origin + 40,
            model_contract: model_contract(),
            forecast_evidence_id: format!("pool-forecast-{index:03}"),
            measurement_evidence_id: format!("pool-measurement-{index:03}"),
            residual_generation_evidence_id: format!("pool-generation-{index:03}"),
            state_mapping_evidence_id: format!("pool-mapping-{index:03}"),
            time_alignment_evidence_id: format!("pool-alignment-{index:03}"),
            source_timeline_digest_reference: format!("pool-timeline-{index:03}"),
            source_sample_evidence_ids: (0..4)
                .map(|sample| format!("pool-sample-{index:03}-{sample}"))
                .collect(),
            source_sample_timestamps_ns: vec![
                origin + 10,
                origin + 20,
                origin + 30,
                origin + 40,
            ],
            residual_power_mw: vec![
                index as i64,
                ((index * 7) % 200) as i64,
                ((index * 11) % 200) as i64,
                ((index * 13) % 200) as i64,
            ],
        }
    }

    fn gaussian_evidence() -> GaussianCopulaEvidence {
        let mut evidence = GaussianCopulaEvidence {
            residual_pool_evidence_id: "residual-pool-v1".to_string(),
            residual_ids: Vec::new(),
            residual_origin_ids: Vec::new(),
            residual_origin_timestamps_ns: Vec::new(),
            residual_target_window_end_timestamps_ns: Vec::new(),
            residual_operating_regime_ids: Vec::new(),
            residual_clock_ids: Vec::new(),
            residual_time_origin_ids: Vec::new(),
            residual_model_contracts: Vec::new(),
            raw_residual_power_mw: Vec::new(),
            rank_normal_scores: Vec::new(),
            measurement_evidence_ids: Vec::new(),
            forecast_evidence_ids: Vec::new(),
            residual_generation_evidence_ids: Vec::new(),
            state_mapping_evidence_ids: Vec::new(),
            time_alignment_evidence_ids: Vec::new(),
            timeline_digest_references: Vec::new(),
            source_sample_evidence_ids: Vec::new(),
            source_sample_timestamps_ns: Vec::new(),
            tie_policy: "AVERAGE_RANKS_FOR_TIES_SORTED_BY_RESIDUAL_ID",
            plotting_position_method: "MIDRANK_MINUS_HALF_OVER_N",
            shrinkage_lambda: 0.05,
            empirical_rank_correlation: vec![
                vec![1.0, 0.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0, 0.0],
                vec![0.0, 0.0, 1.0, 0.0],
                vec![0.0, 0.0, 0.0, 1.0],
            ],
            regularized_correlation: vec![
                vec![1.0, 0.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0, 0.0],
                vec![0.0, 0.0, 1.0, 0.0],
                vec![0.0, 0.0, 0.0, 1.0],
            ],
            cholesky_lower_factor: vec![
                vec![1.0, 0.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0, 0.0],
                vec![0.0, 0.0, 1.0, 0.0],
                vec![0.0, 0.0, 0.0, 1.0],
            ],
        };
        for index in 0..200 {
            let origin = 100 + index as u64 * 45;
            evidence.residual_ids.push(format!("residual-{index:03}"));
            evidence.residual_origin_ids.push(format!("residual-origin-{index:03}"));
            evidence.residual_origin_timestamps_ns.push(origin);
            evidence.residual_target_window_end_timestamps_ns.push(origin + 40);
            evidence.residual_operating_regime_ids.push("normal".to_string());
            evidence.residual_clock_ids.push("clock-common".to_string());
            evidence.residual_time_origin_ids.push("time-origin-common".to_string());
            evidence.residual_model_contracts.push(model_contract());
            evidence.raw_residual_power_mw.push(vec![
                index as i64,
                ((index * 7) % 200) as i64,
                ((index * 11) % 200) as i64,
                ((index * 13) % 200) as i64,
            ]);
            evidence.rank_normal_scores.push(Vec::new());
            evidence.measurement_evidence_ids.push(format!("pool-measurement-{index:03}"));
            evidence.forecast_evidence_ids.push(format!("pool-forecast-{index:03}"));
            evidence.residual_generation_evidence_ids.push(format!("pool-generation-{index:03}"));
            evidence.state_mapping_evidence_ids.push(format!("pool-mapping-{index:03}"));
            evidence.time_alignment_evidence_ids.push(format!("pool-alignment-{index:03}"));
            evidence.timeline_digest_references.push(format!("pool-timeline-{index:03}"));
            evidence.source_sample_evidence_ids.push((0..4)
                .map(|sample| format!("pool-sample-{index:03}-{sample}"))
                .collect());
            evidence.source_sample_timestamps_ns.push(vec![
                origin + 10,
                origin + 20,
                origin + 30,
                origin + 40,
            ]);
        }
        let residual_ids = evidence.residual_ids.clone();
        evidence.rank_normal_scores =
            crate::temporal_scenarios::rank_normal_scores_from_values(
                &evidence.raw_residual_power_mw,
                &residual_ids,
                4,
            ).unwrap();
        evidence.empirical_rank_correlation =
            crate::temporal_scenarios::empirical_rank_correlation(
                &evidence.rank_normal_scores,
                4,
            ).unwrap();
        evidence.regularized_correlation =
            crate::temporal_scenarios::regularize_correlation(
                &evidence.empirical_rank_correlation,
                0.05,
            ).unwrap();
        evidence.cholesky_lower_factor =
            crate::temporal_scenarios::cholesky_lower(
                &evidence.regularized_correlation,
            ).unwrap();
        evidence
    }

    fn scenario_generation_contract(kind: ScenarioGeneratorKind) -> crate::temporal_scenarios::ScenarioGenerationContract {
        use crate::temporal_scenarios::ScenarioGenerationContract;
        let residual = kind != ScenarioGeneratorKind::IndependentQuantileSampling;
        ScenarioGenerationContract {
            generator_id: format!("{kind:?}-generator"),
            generator_revision: "frozen-rev-1".to_string(),
            generator_kind: kind,
            model_contract: model_contract(),
            operating_regime_id: "normal".to_string(),
            clock_id: "clock-common".to_string(),
            time_origin_id: "time-origin-common".to_string(),
            step_period_ns: 10,
            horizon_steps: 4,
            scenario_count: 200,
            deterministic_seed: 42,
            residual_pool_evidence_id: residual.then(|| "residual-pool-v1".to_string()),
            minimum_residual_trajectory_count: match kind {
                ScenarioGeneratorKind::IndependentQuantileSampling => 0,
                ScenarioGeneratorKind::GaussianCopula => 30,
                ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap => 2,
                ScenarioGeneratorKind::SchaakeShuffle => 200,
            },
            minimum_residual_origin_separation_ns: if residual { 40 } else { 0 },
            gaussian_copula_shrinkage: (kind == ScenarioGeneratorKind::GaussianCopula).then_some(0.05),
        }
    }

    fn report(index: usize, kind: ScenarioGeneratorKind, saving: bool) -> ScenarioEvaluationReport {
        let baseline = kind == ScenarioGeneratorKind::IndependentQuantileSampling;
        let reserve = if baseline || !saving { 80 } else { 100 };
        let shortfall = if baseline || !saving { 20 } else { 0 };
        let cost = 0.10 * reserve as f64 + shortfall as f64;
        let gaussian = kind == ScenarioGeneratorKind::GaussianCopula;
        let residual_dependent = matches!(
            kind,
            ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap
                | ScenarioGeneratorKind::SchaakeShuffle
        );
        let mut selected_residual_ids = Vec::new();
        let mut selected_residual_evidence_ids = Vec::new();
        let mut selected_residual_forecast_evidence_ids = Vec::new();
        let mut selected_residual_generation_evidence_ids = Vec::new();
        let mut selected_residual_state_mapping_evidence_ids = Vec::new();
        let mut selected_residual_time_alignment_evidence_ids = Vec::new();
        let mut selected_residual_timeline_digest_references = Vec::new();
        let mut selected_residual_sample_evidence_ids = Vec::new();
        let mut selected_residual_sample_timestamps_ns = Vec::new();
        let mut selected_residual_trajectories = Vec::new();
        if residual_dependent {
            for scenario in 0..200 {
                let pool_index = if kind == ScenarioGeneratorKind::SchaakeShuffle {
                    scenario
                } else {
                    scenario % 100
                };
                let residual = residual_fixture(pool_index);
                selected_residual_ids.push(residual.residual_id.clone());
                selected_residual_evidence_ids.push(residual.measurement_evidence_id.clone());
                selected_residual_forecast_evidence_ids.push(residual.forecast_evidence_id.clone());
                selected_residual_generation_evidence_ids.push(residual.residual_generation_evidence_id.clone());
                selected_residual_state_mapping_evidence_ids.push(residual.state_mapping_evidence_id.clone());
                selected_residual_time_alignment_evidence_ids.push(residual.time_alignment_evidence_id.clone());
                selected_residual_timeline_digest_references.push(residual.source_timeline_digest_reference.clone());
                selected_residual_sample_evidence_ids.push(residual.source_sample_evidence_ids.clone());
                selected_residual_sample_timestamps_ns.push(residual.source_sample_timestamps_ns.clone());
                selected_residual_trajectories.push(Some(residual));
            }
        } else {
            selected_residual_trajectories = vec![None; 200];
        }
        ScenarioEvaluationReport {
            forecast_id: format!("forecast-{index:03}"),
            forecast_origin_id: format!("origin-{index:03}"),
            forecast_origin_timestamp_ns: 10_000,
            scenario_generation_contract: scenario_generation_contract(kind),
            model_contract: model_contract(),
            generator_id: format!("{kind:?}-generator"),
            generator_revision: "frozen-rev-1".to_string(),
            generator_kind: kind,
            forecast_evidence_id: format!("forecast-evidence-{index:03}"),
            clock_id: "clock-common".to_string(),
            time_origin_id: "time-origin-common".to_string(),
            state_mapping_evidence_id: format!("mapping-{index:03}"),
            forecast_time_alignment_evidence_id: format!("forecast-clock-{index:03}"),
            forecast_source_timeline_digest_reference: format!("forecast-timeline-{index:03}"),
            residual_pool_evidence_id: (gaussian || residual_dependent)
                .then(|| "residual-pool-v1".to_string()),
            gaussian_copula_evidence: gaussian.then(gaussian_evidence),
            selected_residual_ids,
            selected_residual_evidence_ids,
            selected_residual_forecast_evidence_ids,
            selected_residual_generation_evidence_ids,
            selected_residual_state_mapping_evidence_ids,
            selected_residual_time_alignment_evidence_ids,
            selected_residual_timeline_digest_references,
            selected_residual_sample_evidence_ids,
            selected_residual_sample_timestamps_ns,
            selected_residual_trajectories,
            scenario_count: 200,
            horizon_steps: 4,
            energy_score_mw: 1.0,
            variogram_score: 2.0,
            actual_energy_mw_ns: 100_000,
            scenario_energy_p10_mw_ns: 90_000,
            scenario_energy_p50_mw_ns: 100_000,
            scenario_energy_p90_mw_ns: 110_000,
            actual_horizon_energy_pit: 0.5,
            actual_peak_power_mw: 150,
            actual_max_ramp_up_mw_per_step: 20,
            actual_max_ramp_down_mw_per_step: 10,
            actual_horizon_max_surplus_mw: 100,
            scenario_peak_p10_mw: 120,
            scenario_peak_p50_mw: 130,
            scenario_peak_p90_mw: 140,
            scenario_max_ramp_up_p50_mw_per_step: 18,
            scenario_max_ramp_down_p50_mw_per_step: 8,
            actual_peak_pit: 0.5,
            actual_max_ramp_up_pit: 0.5,
            actual_max_ramp_down_pit: 0.5,
            reserve_cost_ratio: 0.10,
            reserve_critical_fractile: 0.90,
            reserve_power_mw: reserve,
            actual_shortfall_mw: shortfall,
            reserve_shortfall_occurred: shortfall > 0,
            realized_reserve_cost_ratio_units: cost,
            measurement_run_id: format!("measurement-run-{index:03}"),
            measurement_evidence_id: format!("measurement-evidence-{index:03}"),
            measurement_time_alignment_evidence_id: format!("measurement-clock-{index:03}"),
            measurement_source_timeline_digest_reference: format!("measurement-timeline-{index:03}"),
            measured_sample_evidence_ids: (0..4).map(|sample| format!("measurement-{index}-{sample}")).collect(),
            claim_ceiling: "MODELLED_SCENARIO_AND_MEASURED_HORIZON_SCORE_NOT_GRID_RESERVE_QUALIFICATION",
        }
    }

    fn clusters(count: usize, saving: bool) -> Vec<ScenarioTournamentCluster> {
        (0..count)
            .map(|cluster_index| {
                let cluster_start = 100_000 + cluster_index as u64 * 100_000;
                let episodes = (0..2)
                    .map(|episode_index| {
                        let index = cluster_index * 2 + episode_index;
                        let window_start_ns = cluster_start + 1 + episode_index as u64 * 10;
                        let window_end_ns = window_start_ns + 10;
                        let forecast_origin_timestamp_ns = window_start_ns + 5;
                        let mut evaluations = [
                            report(index, ScenarioGeneratorKind::IndependentQuantileSampling, saving),
                            report(index, ScenarioGeneratorKind::GaussianCopula, saving),
                            report(index, ScenarioGeneratorKind::EmpiricalResidualTrajectoryBootstrap, saving),
                            report(index, ScenarioGeneratorKind::SchaakeShuffle, saving),
                        ].to_vec();
                        for evaluation in &mut evaluations {
                            evaluation.forecast_origin_timestamp_ns = forecast_origin_timestamp_ns;
                        }
                        ScenarioTournamentEpisode {
                            episode_id: format!("episode-{index:03}"),
                            episode_evidence_id: format!("episode-evidence-{index:03}"),
                            window_start_ns,
                            window_end_ns,
                            evaluations,
                        }
                    })
                    .collect();
                ScenarioTournamentCluster {
                    cluster_id: format!("cluster-{cluster_index:03}"),
                    stratum_id: "normal".to_string(),
                    window_start_ns: cluster_start,
                    window_end_ns: cluster_start + 30,
                    cluster_design_evidence_id: format!("cluster-evidence-{cluster_index:03}"),
                    episodes,
                }
            })
            .collect()
    }

    #[test]
    fn exact_binomial_bound_matches_all_success_closed_form() {
        let lower = exact_binomial_lower_confidence_bound(4, 4, 0.90).unwrap();
        assert!((lower - 0.1_f64.powf(0.25)).abs() < 1e-10);
    }

    #[test]
    fn exact_binomial_bound_withholds_zero_successes() {
        assert_eq!(exact_binomial_lower_confidence_bound(0, 10, 0.90), Some(0.0));
    }

    #[test]
    fn binomial_upper_tail_is_consistent_for_all_successes() {
        let p = 0.75;
        assert!((binomial_upper_tail(4, 4, p).unwrap() - p.powi(4)).abs() < 1e-12);
    }

    #[test]
    fn paired_tournament_qualifies_candidates_that_beat_negative_control() {
        let result = evaluate_scenario_tournament(contract(), clusters(4, true)).unwrap();
        assert_eq!(result.total_independent_cluster_count, 4);
        assert_eq!(result.total_episode_count, 8);
        assert!(result.overall_candidate_arms.iter().all(|arm| arm.qualified_under_declared_benchmark));
        assert!(result.overall_candidate_arms.iter().all(|arm| {
            arm.relative_cost_saving_fraction.is_some_and(|value| value >= 0.074)
        }));
    }

    #[test]
    fn stratum_bounds_use_bonferroni_familywise_confidence_and_overall_has_no_pooled_bound() {
        let mut config = contract();
        config.required_strata = vec!["normal".to_string(), "winter".to_string()];
        config.minimum_cluster_target_success_probability = 0.47;
        let mut input = clusters(8, true);
        for cluster in input.iter_mut().skip(4) {
            cluster.stratum_id = "winter".to_string();
        }
        let result = evaluate_scenario_tournament(config, input).unwrap();
        assert_eq!(result.strata.len(), 2);
        assert!(result.strata.iter().all(|stratum| {
            stratum.candidate_arms.iter().all(|arm| arm.confidence_level == Some(0.95))
        }));
        assert!(result.overall_candidate_arms.iter().all(|arm| {
            arm.exact_lower_confidence_bound_target_saving_probability.is_none()
                && arm.confidence_level.is_none()
                && arm.qualified_under_declared_benchmark
        }));
    }

    #[test]
    fn paired_tournament_fails_target_when_candidate_does_not_improve() {
        let result = evaluate_scenario_tournament(contract(), clusters(4, false)).unwrap();
        assert!(result.overall_candidate_arms.iter().all(|arm| !arm.qualified_under_declared_benchmark));
        assert!(result.overall_candidate_arms.iter().all(|arm| arm.relative_cost_saving_fraction == Some(0.0)));
    }

    #[test]
    fn requires_exactly_four_generator_reports_per_episode() {
        let mut input = clusters(4, true);
        input[0].episodes[0].evaluations.pop();
        assert!(evaluate_scenario_tournament(contract(), input).is_err());
    }

    #[test]
    fn rejects_unpaired_measurement_horizon_across_generator_arms() {
        let mut input = clusters(4, true);
        input[0].episodes[0].evaluations[1].measurement_run_id = "different-run".to_string();
        assert!(evaluate_scenario_tournament(contract(), input).is_err());
    }

    #[test]
    fn rejects_different_residual_pool_across_dependence_arms() {
        let mut input = clusters(4, true);
        input[0].episodes[0].evaluations[2].residual_pool_evidence_id =
            Some("different-residual-pool".to_string());
        input[0].episodes[0].evaluations[2].scenario_generation_contract.residual_pool_evidence_id =
            Some("different-residual-pool".to_string());
        assert!(evaluate_scenario_tournament(contract(), input).is_err());
    }

    #[test]
    fn rejects_selected_residual_not_present_in_shared_historical_pool() {
        let mut input = clusters(4, true);
        let report = &mut input[0].episodes[0].evaluations[2];
        report.selected_residual_ids[0] = "unseen-residual".to_string();
        report.selected_residual_trajectories[0].as_mut().unwrap().residual_id =
            "unseen-residual".to_string();
        assert!(evaluate_scenario_tournament(contract(), input).is_err());
    }

    #[test]
    fn rejects_selected_residual_values_that_differ_from_shared_pool() {
        let mut input = clusters(4, true);
        input[0].episodes[0].evaluations[2].selected_residual_trajectories[0]
            .as_mut().unwrap().residual_power_mw[0] += 1;
        assert!(evaluate_scenario_tournament(contract(), input).is_err());
    }

    #[test]
    fn rejects_different_deterministic_seed_across_generator_arms() {
        let mut input = clusters(4, true);
        input[0].episodes[0].evaluations[1].scenario_generation_contract.deterministic_seed = 43;
        assert!(evaluate_scenario_tournament(contract(), input).is_err());
    }

    #[test]
    fn rejects_generator_revision_change_inside_frozen_tournament() {
        let mut input = clusters(4, true);
        input[1].episodes[0].evaluations[2].generator_revision = "rev-2".to_string();
        assert!(evaluate_scenario_tournament(contract(), input).is_err());
    }

    #[test]
    fn rejects_cluster_exposure_mismatch() {
        let mut input = clusters(4, true);
        input[0].window_end_ns += 1;
        assert!(evaluate_scenario_tournament(contract(), input).is_err());
    }

    #[test]
    fn rejects_non_independent_cluster_assumption() {
        let mut config = contract();
        config.independent_cluster_assumption = false;
        config.cluster_independence_review_evidence_id = None;
        assert!(evaluate_scenario_tournament(config, clusters(4, true)).is_err());
    }

    #[test]
    fn rejects_duplicate_measurement_source_evidence_across_episodes() {
        let mut input = clusters(4, true);
        let reused_sample_id =
            input[0].episodes[0].evaluations[0].measured_sample_evidence_ids[0].clone();
        for report in &mut input[1].episodes[0].evaluations {
            report.measured_sample_evidence_ids[0] = reused_sample_id.clone();
        }
        assert!(evaluate_scenario_tournament(contract(), input).is_err());
    }

    #[test]
    fn rejects_missing_required_stratum() {
        let mut config = contract();
        config.required_strata.push("winter".to_string());
        assert!(evaluate_scenario_tournament(config, clusters(4, true)).is_err());
    }

    #[test]
    fn rejects_tampered_cost_or_shortfall_fields() {
        let mut input = clusters(4, true);
        input[0].episodes[0].evaluations[1].realized_reserve_cost_ratio_units = 0.0;
        assert!(evaluate_scenario_tournament(contract(), input).is_err());
    }

    #[test]
    fn rejects_forecast_origin_outside_preregistered_episode_window() {
        let mut input = clusters(4, true);
        let invalid_timestamp = input[0].episodes[0].window_end_ns;
        for report in &mut input[0].episodes[0].evaluations {
            report.forecast_origin_timestamp_ns = invalid_timestamp;
        }
        assert!(evaluate_scenario_tournament(contract(), input).is_err());
    }

    #[test]
    fn rejects_episode_window_overlap_inside_cluster() {
        let mut input = clusters(4, true);
        input[0].episodes[1].window_start_ns = input[0].episodes[0].window_start_ns;
        assert!(evaluate_scenario_tournament(contract(), input).is_err());
    }

    #[test]
    fn rejects_duplicate_forecast_origin_across_episodes() {
        let mut input = clusters(4, true);
        let reused = input[0].episodes[0].evaluations[0].forecast_id.clone();
        for report in &mut input[1].episodes[0].evaluations {
            report.forecast_id = reused.clone();
        }
        assert!(evaluate_scenario_tournament(contract(), input).is_err());
    }
}
