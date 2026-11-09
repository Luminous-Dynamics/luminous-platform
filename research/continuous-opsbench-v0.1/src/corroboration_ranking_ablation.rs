//! Four-arm candidate-ranking ablation for source corroboration.
//!
//! This deliberately does not alter the five-arm context-compilation experiment.
//! It isolates only the evidence-diversity ranking factor: no bonus, naive source
//! ID count, declared control-domain label count, or pairwise dependency-graph
//! no-declared-overlap. Ground truth is consumed only by the evaluator after all
//! rankings are produced. A no-declared-overlap result remains structural only.

use std::collections::{BTreeMap, BTreeSet};

use crate::evidence_corroboration::{
    evaluate_evidence_corroboration, CandidateCorroboration, CandidateCorroborationStatus,
    SourceEvidence,
};
use crate::failure_domains::{
    ControlDomainProfile, FailureDomainGraph, IndependencePolicy,
};
use crate::Digest;

pub const CORROBORATION_RANKING_ARMS: [CorroborationRankingArm; 4] = [
    CorroborationRankingArm::NoDiversityBonus,
    CorroborationRankingArm::NaiveSourceIdCount,
    CorroborationRankingArm::DeclaredControlDomainCount,
    CorroborationRankingArm::GraphQualifiedNoDeclaredOverlap,
];
pub const CORROBORATION_BONUS_POINTS: u64 = 1_000;
pub const MIN_SCENARIO_CLUSTERS_FOR_CI95: usize = 20;
pub const CLUSTER_BOOTSTRAP_REPLICATES: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorroborationRankingArm {
    NoDiversityBonus,
    NaiveSourceIdCount,
    DeclaredControlDomainCount,
    GraphQualifiedNoDeclaredOverlap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateRankingInput {
    pub object_ref: String,
    /// Frozen base relevance score computed without any corroboration factor.
    pub base_score: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluatorRelevance {
    pub object_ref: String,
    /// Graded relevance: 0 is irrelevant; 1..=3 are increasing relevance.
    pub grade: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankingGroundTruth {
    pub relevance: Vec<EvaluatorRelevance>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankedCandidate {
    pub object_ref: String,
    pub base_score: u64,
    pub bonus_points: u64,
    pub final_score: u64,
    pub rank: usize,
    pub declared_source_count: usize,
    pub current_source_count: usize,
    pub current_control_domain_count: usize,
    pub no_declared_overlap_pair_count: usize,
    pub corroboration_status: CandidateCorroborationStatus,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RankingMetrics {
    pub mrr_ppm: u32,
    pub ndcg_at_1_ppm: u32,
    pub ndcg_at_3_ppm: u32,
    pub ndcg_at_5_ppm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankingArmResult {
    pub arm: CorroborationRankingArm,
    pub candidates: Vec<RankedCandidate>,
    pub metrics: RankingMetrics,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioRankingResult {
    pub scenario_id: String,
    pub fault_family_id: String,
    pub attempt_id: u32,
    pub manifest_digest: Digest,
    pub arms: Vec<RankingArmResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorroborationRankingInput<'a> {
    pub scenario_id: &'a str,
    pub fault_family_id: &'a str,
    pub attempt_id: u32,
    pub manifest_digest: Digest,
    pub candidates: &'a [CandidateRankingInput],
    pub evidence: &'a [SourceEvidence],
    /// Evaluator-only relevance; never passed to a ranking policy.
    pub ground_truth: &'a RankingGroundTruth,
    pub profiles: &'a BTreeMap<String, ControlDomainProfile>,
    pub graph: &'a FailureDomainGraph,
    pub audited_at_tick: u64,
    pub maximum_evidence_age_ticks: u64,
    pub independence_policy: IndependencePolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RankingAblationError {
    EmptyScenarioId,
    EmptyFaultFamilyId,
    InvalidAttemptId,
    EmptyCandidates,
    DuplicateCandidate,
    DuplicateTruthCandidate,
    InvalidTruthGrade,
    TruthCandidateMissing,
    EvidenceMissingForCandidate(String),
    Corroboration(String),
    DuplicateScenarioAttempt,
    IncompleteArmSet,
    DuplicateArm,
    ManifestMismatch,
    CandidateUniverseMismatch,
    InvalidRankedCandidate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankingArmAggregate {
    pub fault_family_id: String,
    pub arm: CorroborationRankingArm,
    pub scenario_clusters: usize,
    pub attempts: usize,
    pub mean_mrr_macro_ppm: u32,
    pub mean_ndcg_at_1_macro_ppm: u32,
    pub mean_ndcg_at_3_macro_ppm: u32,
    pub mean_ndcg_at_5_macro_ppm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankingArmContrast {
    pub fault_family_id: String,
    pub arm: CorroborationRankingArm,
    pub scenario_clusters: usize,
    /// Arm minus NoDiversityBonus; repeated attempts are averaged within scenario.
    pub delta_mrr_scenario_macro_ppm: i64,
    pub delta_ndcg_at_1_scenario_macro_ppm: i64,
    pub delta_ndcg_at_3_scenario_macro_ppm: i64,
    pub delta_ndcg_at_5_scenario_macro_ppm: i64,
    /// Scenario-cluster bootstrap percentile intervals; absent below 20 clusters.
    pub delta_mrr_cluster_ci95_ppm: Option<(i64, i64)>,
    pub delta_ndcg_at_1_cluster_ci95_ppm: Option<(i64, i64)>,
    pub delta_ndcg_at_3_cluster_ci95_ppm: Option<(i64, i64)>,
    pub delta_ndcg_at_5_cluster_ci95_ppm: Option<(i64, i64)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankingAblationAnalysis {
    pub aggregates: Vec<RankingArmAggregate>,
    pub contrasts_vs_no_bonus: Vec<RankingArmContrast>,
    pub bootstrap_seed: u64,
    pub bootstrap_replicates: usize,
    pub minimum_scenario_clusters_for_ci95: usize,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn round_ppm(value: f64) -> u32 {
    (value.clamp(0.0, 1.0) * 1_000_000.0).round() as u32
}

fn ndcg_at(ranked: &[RankedCandidate], truth: &BTreeMap<&str, u8>, k: usize) -> u32 {
    let dcg: f64 = ranked
        .iter()
        .take(k)
        .enumerate()
        .map(|(index, candidate)| {
            let grade = f64::from(*truth.get(candidate.object_ref.as_str()).unwrap_or(&0));
            (2_f64.powf(grade) - 1.0) / ((index + 2) as f64).log2()
        })
        .sum();
    let mut ideal: Vec<u8> = truth.values().copied().filter(|grade| *grade > 0).collect();
    ideal.sort_unstable_by(|left, right| right.cmp(left));
    let idcg: f64 = ideal
        .iter()
        .take(k)
        .enumerate()
        .map(|(index, grade)| {
            (2_f64.powf(f64::from(*grade)) - 1.0) / ((index + 2) as f64).log2()
        })
        .sum();
    if idcg == 0.0 {
        0
    } else {
        round_ppm(dcg / idcg)
    }
}

fn ranking_metrics(
    ranked: &[RankedCandidate],
    truth: &BTreeMap<&str, u8>,
) -> RankingMetrics {
    let reciprocal_rank = ranked
        .iter()
        .find(|candidate| truth.get(candidate.object_ref.as_str()).copied().unwrap_or(0) > 0)
        .map(|candidate| 1.0 / candidate.rank as f64)
        .unwrap_or(0.0);
    RankingMetrics {
        mrr_ppm: round_ppm(reciprocal_rank),
        ndcg_at_1_ppm: ndcg_at(ranked, truth, 1),
        ndcg_at_3_ppm: ndcg_at(ranked, truth, 3),
        ndcg_at_5_ppm: ndcg_at(ranked, truth, 5),
    }
}

fn bonus_for(
    arm: CorroborationRankingArm,
    corroboration: &CandidateCorroboration,
) -> u64 {
    match arm {
        CorroborationRankingArm::NoDiversityBonus => 0,
        CorroborationRankingArm::NaiveSourceIdCount => {
            (corroboration.current_source_count.saturating_sub(1) as u64)
                .saturating_mul(CORROBORATION_BONUS_POINTS)
        }
        CorroborationRankingArm::DeclaredControlDomainCount => {
            (corroboration.current_control_domain_count.saturating_sub(1) as u64)
                .saturating_mul(CORROBORATION_BONUS_POINTS)
        }
        CorroborationRankingArm::GraphQualifiedNoDeclaredOverlap => {
            if corroboration.status == CandidateCorroborationStatus::PairwiseNoDeclaredOverlap {
                (corroboration.no_declared_overlap_pair_count as u64)
                    .saturating_mul(CORROBORATION_BONUS_POINTS)
            } else {
                0
            }
        }
    }
}

/// Build four rankings over exactly the same frozen candidate/evidence universe.
/// Only the corroboration bonus changes between arms. Ground truth is read solely
/// after all four rankings have been produced, by this evaluator function.
pub fn evaluate_corroboration_ranking_ablation(
    input: &CorroborationRankingInput<'_>,
) -> Result<ScenarioRankingResult, RankingAblationError> {
    if !non_empty(input.scenario_id) {
        return Err(RankingAblationError::EmptyScenarioId);
    }
    if !non_empty(input.fault_family_id) {
        return Err(RankingAblationError::EmptyFaultFamilyId);
    }
    if input.attempt_id == 0 {
        return Err(RankingAblationError::InvalidAttemptId);
    }
    if input.candidates.is_empty() {
        return Err(RankingAblationError::EmptyCandidates);
    }

    let mut candidate_ids = BTreeSet::new();
    for candidate in input.candidates {
        if !non_empty(&candidate.object_ref) || !candidate_ids.insert(candidate.object_ref.as_str()) {
            return Err(RankingAblationError::DuplicateCandidate);
        }
        if !input.evidence.iter().any(|item| item.object_ref == candidate.object_ref) {
            return Err(RankingAblationError::EvidenceMissingForCandidate(
                candidate.object_ref.clone(),
            ));
        }
    }

    let mut truth_by_object = BTreeMap::new();
    for relevance in &input.ground_truth.relevance {
        if relevance.grade > 3 {
            return Err(RankingAblationError::InvalidTruthGrade);
        }
        if !candidate_ids.contains(relevance.object_ref.as_str()) {
            return Err(RankingAblationError::TruthCandidateMissing);
        }
        if truth_by_object
            .insert(relevance.object_ref.as_str(), relevance.grade)
            .is_some()
        {
            return Err(RankingAblationError::DuplicateTruthCandidate);
        }
    }

    let corroboration = evaluate_evidence_corroboration(
        input.evidence,
        input.profiles,
        input.graph,
        input.audited_at_tick,
        input.maximum_evidence_age_ticks,
        input.independence_policy,
    )
    .map_err(|error| RankingAblationError::Corroboration(format!("{error:?}")))?;
    let by_object: BTreeMap<&str, &CandidateCorroboration> = corroboration
        .iter()
        .map(|row| (row.object_ref.as_str(), row))
        .collect();

    let mut arms = Vec::with_capacity(CORROBORATION_RANKING_ARMS.len());
    for arm in CORROBORATION_RANKING_ARMS {
        let mut ranked = Vec::with_capacity(input.candidates.len());
        for candidate in input.candidates {
            let support = by_object
                .get(candidate.object_ref.as_str())
                .ok_or_else(|| {
                    RankingAblationError::EvidenceMissingForCandidate(candidate.object_ref.clone())
                })?;
            let bonus_points = bonus_for(arm, support);
            ranked.push(RankedCandidate {
                object_ref: candidate.object_ref.clone(),
                base_score: candidate.base_score,
                bonus_points,
                final_score: candidate.base_score.saturating_add(bonus_points),
                rank: 0,
                declared_source_count: support.declared_source_count,
                current_source_count: support.current_source_count,
                current_control_domain_count: support.current_control_domain_count,
                no_declared_overlap_pair_count: support.no_declared_overlap_pair_count,
                corroboration_status: support.status,
            });
        }
        ranked.sort_by(|left, right| {
            right
                .final_score
                .cmp(&left.final_score)
                .then_with(|| left.object_ref.cmp(&right.object_ref))
        });
        for (index, candidate) in ranked.iter_mut().enumerate() {
            candidate.rank = index + 1;
        }
        arms.push(RankingArmResult {
            arm,
            metrics: ranking_metrics(&ranked, &truth_by_object),
            candidates: ranked,
        });
    }

    Ok(ScenarioRankingResult {
        scenario_id: input.scenario_id.to_owned(),
        fault_family_id: input.fault_family_id.to_owned(),
        attempt_id: input.attempt_id,
        manifest_digest: input.manifest_digest.clone(),
        arms,
    })
}

fn next_bootstrap_u64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn cluster_ci95(deltas: &[i64], seed: u64) -> Option<(i64, i64)> {
    if deltas.len() < MIN_SCENARIO_CLUSTERS_FOR_CI95 {
        return None;
    }
    let mut state = seed;
    let count = deltas.len();
    let mut means = Vec::with_capacity(CLUSTER_BOOTSTRAP_REPLICATES);
    for _ in 0..CLUSTER_BOOTSTRAP_REPLICATES {
        let mut sum = 0_i128;
        for _ in 0..count {
            let index = (next_bootstrap_u64(&mut state) % count as u64) as usize;
            sum += i128::from(deltas[index]);
        }
        means.push((sum / count as i128) as i64);
    }
    means.sort_unstable();
    let last = means.len() - 1;
    Some((means[(last * 25) / 1_000], means[(last * 975) / 1_000]))
}

fn mean(values: &[u32]) -> u32 {
    if values.is_empty() {
        0
    } else {
        (values.iter().map(|value| u64::from(*value)).sum::<u64>() / values.len() as u64)
            as u32
    }
}

fn mean_signed(values: &[i64]) -> i64 {
    if values.is_empty() {
        0
    } else {
        (values.iter().map(|value| i128::from(*value)).sum::<i128>() / values.len() as i128)
            as i64
    }
}

/// Aggregate attempts within each scenario first, then treat distinct scenarios
/// as the inference clusters. Attempts never masquerade as independent samples.
pub fn analyze_corroboration_ranking_results(
    runs: &[ScenarioRankingResult],
    bootstrap_seed: u64,
) -> Result<RankingAblationAnalysis, RankingAblationError> {
    if runs.is_empty() {
        return Err(RankingAblationError::EmptyCandidates);
    }
    let mut attempts = BTreeSet::new();
    let mut scenario_manifest: BTreeMap<(&str, &str), &Digest> = BTreeMap::new();
    let mut grouped: BTreeMap<
        (&str, &str),
        BTreeMap<u32, &ScenarioRankingResult>,
    > = BTreeMap::new();

    for run in runs {
        if !non_empty(&run.scenario_id) || !non_empty(&run.fault_family_id) || run.attempt_id == 0 {
            return Err(RankingAblationError::InvalidAttemptId);
        }
        if run.arms.len() != CORROBORATION_RANKING_ARMS.len() {
            return Err(RankingAblationError::IncompleteArmSet);
        }
        let mut seen = BTreeSet::new();
        let mut reference_universe: Option<
            BTreeMap<&str, (u64, usize, usize, usize, usize, CandidateCorroborationStatus)>,
        > = None;
        for result in &run.arms {
            if !seen.insert(result.arm) {
                return Err(RankingAblationError::DuplicateArm);
            }
            if result.candidates.is_empty() {
                return Err(RankingAblationError::CandidateUniverseMismatch);
            }

            let mut universe = BTreeMap::new();
            let mut previous: Option<&RankedCandidate> = None;
            for (index, candidate) in result.candidates.iter().enumerate() {
                if !non_empty(&candidate.object_ref)
                    || candidate.rank != index + 1
                    || candidate.final_score
                        != candidate.base_score.saturating_add(candidate.bonus_points)
                    || universe
                        .insert(
                            candidate.object_ref.as_str(),
                            (
                                candidate.base_score,
                                candidate.declared_source_count,
                                candidate.current_source_count,
                                candidate.current_control_domain_count,
                                candidate.no_declared_overlap_pair_count,
                                candidate.corroboration_status,
                            ),
                        )
                        .is_some()
                {
                    return Err(RankingAblationError::InvalidRankedCandidate);
                }
                if let Some(previous_candidate) = previous {
                    let correctly_ordered = previous_candidate.final_score > candidate.final_score
                        || (previous_candidate.final_score == candidate.final_score
                            && previous_candidate.object_ref < candidate.object_ref);
                    if !correctly_ordered {
                        return Err(RankingAblationError::InvalidRankedCandidate);
                    }
                }
                previous = Some(candidate);
            }
            if let Some(reference) = &reference_universe {
                if reference != &universe {
                    return Err(RankingAblationError::CandidateUniverseMismatch);
                }
            } else {
                reference_universe = Some(universe);
            }
            if result.metrics.mrr_ppm > 1_000_000
                || result.metrics.ndcg_at_1_ppm > 1_000_000
                || result.metrics.ndcg_at_3_ppm > 1_000_000
                || result.metrics.ndcg_at_5_ppm > 1_000_000
            {
                return Err(RankingAblationError::InvalidRankedCandidate);
            }
        }
        if CORROBORATION_RANKING_ARMS
            .iter()
            .any(|arm| !seen.contains(arm))
        {
            return Err(RankingAblationError::IncompleteArmSet);
        }
        let attempt_key = (run.fault_family_id.as_str(), run.scenario_id.as_str(), run.attempt_id);
        if !attempts.insert(attempt_key) {
            return Err(RankingAblationError::DuplicateScenarioAttempt);
        }
        let scenario_key = (run.fault_family_id.as_str(), run.scenario_id.as_str());
        if let Some(previous) = scenario_manifest.get(&scenario_key) {
            if **previous != run.manifest_digest {
                return Err(RankingAblationError::ManifestMismatch);
            }
        } else {
            scenario_manifest.insert(scenario_key, &run.manifest_digest);
        }
        grouped.entry(scenario_key).or_default().insert(run.attempt_id, run);
    }

    let mut by_family: BTreeMap<&str, Vec<(&str, Vec<&ScenarioRankingResult>)>> = BTreeMap::new();
    for ((family, scenario), scenario_attempts) in &grouped {
        by_family
            .entry(family)
            .or_default()
            .push((scenario, scenario_attempts.values().copied().collect()));
    }

    let mut aggregates = Vec::new();
    let mut contrasts = Vec::new();
    for (family, scenarios) in by_family {
        let scenario_count = scenarios.len();
        let mut all_arm_values: BTreeMap<CorroborationRankingArm, Vec<RankingMetrics>> =
            BTreeMap::new();
        let mut per_arm_deltas: BTreeMap<CorroborationRankingArm, [Vec<i64>; 4]> =
            BTreeMap::new();
        for (_, scenario_runs) in &scenarios {
            let mut scenario_means = BTreeMap::new();
            for arm in CORROBORATION_RANKING_ARMS {
                let mut rows = Vec::new();
                for run in scenario_runs {
                    let result = run
                        .arms
                        .iter()
                        .find(|result| result.arm == arm)
                        .ok_or(RankingAblationError::IncompleteArmSet)?;
                    rows.push(result.metrics);
                }
                let metrics = RankingMetrics {
                    mrr_ppm: mean(&rows.iter().map(|row| row.mrr_ppm).collect::<Vec<_>>()),
                    ndcg_at_1_ppm: mean(
                        &rows.iter().map(|row| row.ndcg_at_1_ppm).collect::<Vec<_>>(),
                    ),
                    ndcg_at_3_ppm: mean(
                        &rows.iter().map(|row| row.ndcg_at_3_ppm).collect::<Vec<_>>(),
                    ),
                    ndcg_at_5_ppm: mean(
                        &rows.iter().map(|row| row.ndcg_at_5_ppm).collect::<Vec<_>>(),
                    ),
                };
                all_arm_values.entry(arm).or_default().push(metrics);
                scenario_means.insert(arm, metrics);
            }
            let baseline = scenario_means[&CorroborationRankingArm::NoDiversityBonus];
            for arm in CORROBORATION_RANKING_ARMS {
                if arm == CorroborationRankingArm::NoDiversityBonus {
                    continue;
                }
                let value = scenario_means[&arm];
                let deltas = per_arm_deltas.entry(arm).or_default();
                deltas[0].push(i64::from(value.mrr_ppm) - i64::from(baseline.mrr_ppm));
                deltas[1].push(
                    i64::from(value.ndcg_at_1_ppm) - i64::from(baseline.ndcg_at_1_ppm),
                );
                deltas[2].push(
                    i64::from(value.ndcg_at_3_ppm) - i64::from(baseline.ndcg_at_3_ppm),
                );
                deltas[3].push(
                    i64::from(value.ndcg_at_5_ppm) - i64::from(baseline.ndcg_at_5_ppm),
                );
            }
        }

        for arm in CORROBORATION_RANKING_ARMS {
            let values = &all_arm_values[&arm];
            aggregates.push(RankingArmAggregate {
                fault_family_id: family.to_owned(),
                arm,
                scenario_clusters: scenario_count,
                attempts: scenarios.iter().map(|(_, attempts)| attempts.len()).sum(),
                mean_mrr_macro_ppm: mean(
                    &values.iter().map(|value| value.mrr_ppm).collect::<Vec<_>>(),
                ),
                mean_ndcg_at_1_macro_ppm: mean(
                    &values.iter().map(|value| value.ndcg_at_1_ppm).collect::<Vec<_>>(),
                ),
                mean_ndcg_at_3_macro_ppm: mean(
                    &values.iter().map(|value| value.ndcg_at_3_ppm).collect::<Vec<_>>(),
                ),
                mean_ndcg_at_5_macro_ppm: mean(
                    &values.iter().map(|value| value.ndcg_at_5_ppm).collect::<Vec<_>>(),
                ),
            });
            if arm == CorroborationRankingArm::NoDiversityBonus {
                continue;
            }
            let deltas = &per_arm_deltas[&arm];
            let seed = bootstrap_seed
                ^ (family.bytes().fold(0_u64, |sum, byte| {
                    sum.wrapping_mul(257).wrapping_add(u64::from(byte))
                }))
                ^ (arm as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
            contrasts.push(RankingArmContrast {
                fault_family_id: family.to_owned(),
                arm,
                scenario_clusters: scenario_count,
                delta_mrr_scenario_macro_ppm: mean_signed(&deltas[0]),
                delta_ndcg_at_1_scenario_macro_ppm: mean_signed(&deltas[1]),
                delta_ndcg_at_3_scenario_macro_ppm: mean_signed(&deltas[2]),
                delta_ndcg_at_5_scenario_macro_ppm: mean_signed(&deltas[3]),
                delta_mrr_cluster_ci95_ppm: cluster_ci95(&deltas[0], seed),
                delta_ndcg_at_1_cluster_ci95_ppm: cluster_ci95(&deltas[1], seed ^ 0xa5a5_5a5a_d3c1_b2e7),
                delta_ndcg_at_3_cluster_ci95_ppm: cluster_ci95(&deltas[2], seed ^ 0x13a5_b7c9_d1e3_f507),
                delta_ndcg_at_5_cluster_ci95_ppm: cluster_ci95(&deltas[3], seed ^ 0x7f4a_7c15_9e37_79b9),
            });
        }
    }
    aggregates.sort_by(|a, b| (&a.fault_family_id, a.arm).cmp(&(&b.fault_family_id, b.arm)));
    contrasts.sort_by(|a, b| (&a.fault_family_id, a.arm).cmp(&(&b.fault_family_id, b.arm)));
    Ok(RankingAblationAnalysis {
        aggregates,
        contrasts_vs_no_bonus: contrasts,
        bootstrap_seed,
        bootstrap_replicates: CLUSTER_BOOTSTRAP_REPLICATES,
        minimum_scenario_clusters_for_ci95: MIN_SCENARIO_CLUSTERS_FOR_CI95,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence_corroboration::{EvidenceAvailability, SourceEvidence};
    use crate::failure_domains::{FailureDomainNode, InventoryState};

    fn node(id: &str, deps: &[&str]) -> FailureDomainNode {
        FailureDomainNode {
            id: id.into(),
            class: "test-domain".into(),
            depends_on: deps.iter().map(|value| (*value).into()).collect(),
        }
    }

    fn graph() -> FailureDomainGraph {
        FailureDomainGraph {
            revision: "rev-1".into(),
            nodes: vec![
                node("proc-a", &["host-a"]),
                node("proc-b", &["host-b"]),
                node("proc-c", &["host-c"]),
                node("host-a", &[]),
                node("host-b", &[]),
                node("host-c", &[]),
            ]
            .into_iter()
            .map(|node| (node.id.clone(), node))
            .collect(),
        }
    }

    fn profile(subject: &str, root: &str) -> ControlDomainProfile {
        ControlDomainProfile {
            subject_id: subject.into(),
            profile_digest: format!("sha256:{:064x}", subject.len()),
            graph_revision: "rev-1".into(),
            roots: [root.to_owned()].into_iter().collect(),
            state: InventoryState::Complete,
            observed_at_tick: 1,
            valid_until_tick: 100,
        }
    }

    fn input<'a>(
        candidates: &'a [CandidateRankingInput],
        evidence: &'a [SourceEvidence],
        truth: &'a RankingGroundTruth,
        profiles: &'a BTreeMap<String, ControlDomainProfile>,
        graph: &'a FailureDomainGraph,
    ) -> CorroborationRankingInput<'a> {
        CorroborationRankingInput {
            scenario_id: "scenario-a",
            fault_family_id: "queue",
            attempt_id: 1,
            manifest_digest: Digest::from_byte(9),
            candidates,
            evidence,
            ground_truth: truth,
            profiles,
            graph,
            audited_at_tick: 5,
            maximum_evidence_age_ticks: 10,
            independence_policy: IndependencePolicy {
                maximum_inventory_age_ticks: 10,
            },
        }
    }

    fn ev(id: &str, object: &str, source: &str, domain: &str) -> SourceEvidence {
        SourceEvidence {
            evidence_id: id.into(),
            object_ref: object.into(),
            source_id: source.into(),
            control_domain: domain.into(),
            availability: EvidenceAvailability::Present,
            event_tick: 1,
            observed_at_tick: 2,
            valid_until_tick: Some(30),
        }
    }

    #[test]
    fn no_bonus_arm_preserves_base_ranking_and_only_factor_changes_other_arms() {
        let candidates = vec![
            CandidateRankingInput { object_ref: "root".into(), base_score: 10_000 },
            CandidateRankingInput { object_ref: "decoy".into(), base_score: 9_500 },
        ];
        let evidence = vec![
            ev("r1", "root", "src-a", "domain-a"),
            ev("d1", "decoy", "src-b", "domain-b"),
        ];
        let graph = graph();
        let profiles = [
            ("domain-a".into(), profile("domain-a", "proc-a")),
            ("domain-b".into(), profile("domain-b", "proc-b")),
        ].into_iter().collect();
        let truth = RankingGroundTruth {
            relevance: vec![EvaluatorRelevance { object_ref: "root".into(), grade: 3 }],
        };
        let run = evaluate_corroboration_ranking_ablation(&input(
            &candidates, &evidence, &truth, &profiles, &graph
        )).unwrap();
        assert_eq!(run.arms[0].arm, CorroborationRankingArm::NoDiversityBonus);
        assert_eq!(run.arms[0].candidates[0].object_ref, "root");
        assert_eq!(run.arms[0].candidates[0].bonus_points, 0);
        assert!(run.arms.iter().all(|arm| arm.candidates.len() == 2));
    }

    #[test]
    fn graph_qualified_bonus_requires_all_current_domain_pairs_to_be_disjoint() {
        let candidates = vec![
            CandidateRankingInput { object_ref: "candidate".into(), base_score: 100 },
            CandidateRankingInput { object_ref: "other".into(), base_score: 100 },
        ];
        let evidence = vec![
            ev("e1", "candidate", "s1", "domain-a"),
            ev("e2", "candidate", "s2", "domain-b"),
            ev("e3", "candidate", "s3", "domain-c"),
            ev("e4", "other", "s4", "domain-c"),
        ];
        let mut graph = graph();
        graph.nodes.get_mut("host-a").unwrap().depends_on.insert("shared".into());
        graph.nodes.get_mut("host-c").unwrap().depends_on.insert("shared".into());
        graph.nodes.insert("shared".into(), node("shared", &[]));
        let profiles = [
            ("domain-a".into(), profile("domain-a", "proc-a")),
            ("domain-b".into(), profile("domain-b", "proc-b")),
            ("domain-c".into(), profile("domain-c", "proc-c")),
        ].into_iter().collect();
        let truth = RankingGroundTruth {
            relevance: vec![EvaluatorRelevance { object_ref: "candidate".into(), grade: 3 }],
        };
        let run = evaluate_corroboration_ranking_ablation(&input(
            &candidates, &evidence, &truth, &profiles, &graph
        )).unwrap();
        let graph_arm = run.arms.iter().find(|arm|
            arm.arm == CorroborationRankingArm::GraphQualifiedNoDeclaredOverlap
        ).unwrap();
        let candidate = graph_arm.candidates.iter().find(|item| item.object_ref == "candidate").unwrap();
        assert_eq!(candidate.corroboration_status, CandidateCorroborationStatus::SharedDeclaredDependency);
        assert_eq!(candidate.bonus_points, 0);
    }

    #[test]
    fn graph_qualified_corroboration_can_rescue_the_correct_candidate_ranking() {
        let candidates = vec![
            CandidateRankingInput { object_ref: "true-root".into(), base_score: 1_050 },
            CandidateRankingInput { object_ref: "decoy".into(), base_score: 1_100 },
        ];
        let evidence = vec![
            ev("r1", "true-root", "r-src-a", "domain-a"),
            ev("r2", "true-root", "r-src-b", "domain-b"),
            ev("d1", "decoy", "d-src-c", "domain-c"),
            ev("d2", "decoy", "d-src-d", "domain-d"),
        ];
        let mut graph = graph();
        graph.nodes.get_mut("host-c").unwrap().depends_on.insert("shared".into());
        graph.nodes.get_mut("host-d").unwrap().depends_on.insert("shared".into());
        graph.nodes.insert("shared".into(), node("shared", &[]));
        let profiles = [
            ("domain-a".into(), profile("domain-a", "proc-a")),
            ("domain-b".into(), profile("domain-b", "proc-b")),
            ("domain-c".into(), profile("domain-c", "proc-c")),
            ("domain-d".into(), profile("domain-d", "proc-d")),
        ].into_iter().collect();
        let truth = RankingGroundTruth {
            relevance: vec![EvaluatorRelevance {
                object_ref: "true-root".into(),
                grade: 3,
            }],
        };
        let run = evaluate_corroboration_ranking_ablation(&input(
            &candidates, &evidence, &truth, &profiles, &graph
        )).unwrap();

        for arm in &run.arms[..3] {
            assert_eq!(arm.candidates[0].object_ref, "decoy");
        }
        let graph_arm = &run.arms[3];
        assert_eq!(graph_arm.candidates[0].object_ref, "true-root");
        assert!(graph_arm.metrics.mrr_ppm > run.arms[0].metrics.mrr_ppm);
        assert!(graph_arm.metrics.ndcg_at_1_ppm > run.arms[0].metrics.ndcg_at_1_ppm);
    }

    #[test]
    fn analysis_rejects_candidate_universe_or_base_score_changes_between_arms() {
        let candidate = RankedCandidate {
            object_ref: "root".into(),
            base_score: 100,
            bonus_points: 0,
            final_score: 100,
            rank: 1,
            declared_source_count: 1,
            current_source_count: 1,
            current_control_domain_count: 1,
            no_declared_overlap_pair_count: 0,
            corroboration_status: CandidateCorroborationStatus::SingleDeclaredControlDomain,
        };
        let base = ScenarioRankingResult {
            scenario_id: "scenario".into(),
            fault_family_id: "queue".into(),
            attempt_id: 1,
            manifest_digest: Digest::from_byte(1),
            arms: CORROBORATION_RANKING_ARMS.iter().map(|arm| RankingArmResult {
                arm: *arm,
                candidates: vec![candidate.clone()],
                metrics: RankingMetrics { mrr_ppm: 1_000_000, ndcg_at_1_ppm: 1_000_000, ndcg_at_3_ppm: 1_000_000, ndcg_at_5_ppm: 1_000_000 },
            }).collect(),
        };
        let mut changed = base.clone();
        changed.arms[2].candidates[0].base_score = 99;
        changed.arms[2].candidates[0].final_score = 99;
        assert_eq!(
            analyze_corroboration_ranking_results(&[changed], 7),
            Err(RankingAblationError::CandidateUniverseMismatch)
        );

        let mut bad_rank = base;
        bad_rank.arms[1].candidates[0].rank = 2;
        assert_eq!(
            analyze_corroboration_ranking_results(&[bad_rank], 7),
            Err(RankingAblationError::InvalidRankedCandidate)
        );
    }

    #[test]
    fn current_source_count_and_control_domain_count_are_separate_factors() {
        let candidates = vec![
            CandidateRankingInput { object_ref: "candidate".into(), base_score: 0 },
        ];
        let evidence = vec![
            ev("e1", "candidate", "s1", "domain-a"),
            ev("e2", "candidate", "s2", "domain-a"),
            ev("e3", "candidate", "s3", "domain-a"),
        ];
        let profiles = [("domain-a".into(), profile("domain-a", "proc-a"))]
            .into_iter().collect();
        let truth = RankingGroundTruth {
            relevance: vec![EvaluatorRelevance { object_ref: "candidate".into(), grade: 1 }],
        };
        let graph = graph();
        let run = evaluate_corroboration_ranking_ablation(&input(
            &candidates, &evidence, &truth, &profiles, &graph
        )).unwrap();
        let source_arm = &run.arms[1];
        let domain_arm = &run.arms[2];
        let graph_arm = &run.arms[3];
        assert_eq!(source_arm.candidates[0].bonus_points, 2 * CORROBORATION_BONUS_POINTS);
        assert_eq!(domain_arm.candidates[0].bonus_points, 0);
        assert_eq!(graph_arm.candidates[0].bonus_points, 0);
    }

    #[test]
    fn stale_unknown_and_expired_evidence_never_create_diversity_bonuses() {
        let candidates = vec![
            CandidateRankingInput { object_ref: "candidate".into(), base_score: 0 },
        ];
        let evidence = vec![
            ev("e1", "candidate", "s1", "domain-a"),
            SourceEvidence {
                evidence_id: "stale".into(),
                observed_at_tick: 0,
                ..ev("stale", "candidate", "s2", "domain-b")
            },
        ];
        let profiles = [
            ("domain-a".into(), profile("domain-a", "proc-a")),
            ("domain-b".into(), profile("domain-b", "proc-b")),
        ].into_iter().collect();
        let truth = RankingGroundTruth {
            relevance: vec![EvaluatorRelevance { object_ref: "candidate".into(), grade: 1 }],
        };
        let graph = graph();
        let result = evaluate_corroboration_ranking_ablation(&input(
            &candidates, &evidence, &truth, &profiles, &graph
        )).unwrap();
        assert_eq!(result.arms[1].candidates[0].bonus_points, 0);
        assert_eq!(result.arms[2].candidates[0].bonus_points, 0);
        assert_eq!(result.arms[3].candidates[0].bonus_points, 0);
    }

    #[test]
    fn evaluator_truth_changes_metrics_but_not_candidate_scores() {
        let candidates = vec![
            CandidateRankingInput { object_ref: "a".into(), base_score: 100 },
            CandidateRankingInput { object_ref: "b".into(), base_score: 90 },
        ];
        let evidence = vec![
            ev("a1", "a", "s1", "domain-a"),
            ev("b1", "b", "s2", "domain-b"),
        ];
        let profiles = [
            ("domain-a".into(), profile("domain-a", "proc-a")),
            ("domain-b".into(), profile("domain-b", "proc-b")),
        ].into_iter().collect();
        let graph = graph();
        let truth_a = RankingGroundTruth { relevance: vec![
            EvaluatorRelevance { object_ref: "a".into(), grade: 3 },
        ]};
        let truth_b = RankingGroundTruth { relevance: vec![
            EvaluatorRelevance { object_ref: "b".into(), grade: 3 },
        ]};
        let run_a = evaluate_corroboration_ranking_ablation(&input(
            &candidates, &evidence, &truth_a, &profiles, &graph
        )).unwrap();
        let run_b = evaluate_corroboration_ranking_ablation(&input(
            &candidates, &evidence, &truth_b, &profiles, &graph
        )).unwrap();
        assert_eq!(run_a.arms[0].candidates, run_b.arms[0].candidates);
        assert_ne!(run_a.arms[0].metrics, run_b.arms[0].metrics);
    }

    #[test]
    fn incomplete_or_duplicate_arm_results_fail_analysis() {
        let empty = ScenarioRankingResult {
            scenario_id: "s".into(),
            fault_family_id: "f".into(),
            attempt_id: 1,
            manifest_digest: Digest::from_byte(1),
            arms: Vec::new(),
        };
        assert_eq!(
            analyze_corroboration_ranking_results(&[empty], 7),
            Err(RankingAblationError::IncompleteArmSet)
        );
    }

    #[test]
    fn repeated_attempts_are_averaged_within_scenario_before_cluster_analysis() {
        let baseline = RankingMetrics { mrr_ppm: 500_000, ndcg_at_1_ppm: 300_000, ndcg_at_3_ppm: 400_000, ndcg_at_5_ppm: 450_000 };
        let candidate = RankingMetrics { mrr_ppm: 700_000, ndcg_at_1_ppm: 600_000, ndcg_at_3_ppm: 650_000, ndcg_at_5_ppm: 680_000 };
        let mk = |scenario: &str, attempt: u32, manifest: Digest| {
            ScenarioRankingResult {
                scenario_id: scenario.into(),
                fault_family_id: "queue".into(),
                attempt_id: attempt,
                manifest_digest: manifest,
                arms: CORROBORATION_RANKING_ARMS
                    .iter()
                    .map(|arm| RankingArmResult {
                        arm: *arm,
                        candidates: Vec::new(),
                        metrics: if *arm == CorroborationRankingArm::NoDiversityBonus {
                            baseline
                        } else { candidate },
                    }).collect(),
            }
        };
        let runs = vec![
            mk("scenario-a", 1, Digest::from_byte(1)),
            mk("scenario-a", 2, Digest::from_byte(1)),
            mk("scenario-b", 1, Digest::from_byte(2)),
        ];
        let analysis = analyze_corroboration_ranking_results(&runs, 42).unwrap();
        let contrast = analysis.contrasts_vs_no_bonus.iter().find(|item|
            item.arm == CorroborationRankingArm::GraphQualifiedNoDeclaredOverlap
        ).unwrap();
        assert_eq!(contrast.scenario_clusters, 2);
        assert_eq!(contrast.delta_mrr_scenario_macro_ppm, 200_000);
        assert_eq!(contrast.delta_ndcg_at_1_scenario_macro_ppm, 300_000);
        assert!(contrast.delta_mrr_cluster_ci95_ppm.is_none());
    }

    #[test]
    fn cluster_interval_is_reproducible_and_requires_twenty_scenarios() {
        let deltas: Vec<i64> = (0..20).map(|i| i as i64).collect();
        assert_eq!(cluster_ci95(&deltas, 7), cluster_ci95(&deltas, 7));
        assert!(cluster_ci95(&deltas[..19], 7).is_none());
        assert!(cluster_ci95(&deltas, 7).is_some());
    }
}
