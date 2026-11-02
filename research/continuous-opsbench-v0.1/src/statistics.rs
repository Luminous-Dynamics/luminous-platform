//! Paired cluster inference and rare-event assurance for Continuous OpsBench.
//!
//! This module computes reproducible statistics over preregistered, paired scenario
//! clusters. It never treats requests, windows, retries or faults within a cluster
//! as independent resampling units. It does not authenticate source records, prove
//! randomization quality, or establish causal identification.

use crate::sealed::EvaluationArm;
use crate::Digest;
use std::collections::BTreeSet;

pub const INFERENCE_SCHEMA: &str = "continuous-opsbench-cluster-inference-v0.1";
pub const BOOTSTRAP_ALGORITHM: &str = "splitmix64-paired-cluster-bootstrap-v1";
pub const DEFAULT_BOOTSTRAP_REPLICATES: u32 = 10_000;
pub const MIN_BOOTSTRAP_REPLICATES: u32 = 1_000;
pub const MAX_BOOTSTRAP_REPLICATES: u32 = 100_000;
pub const MAX_INDEPENDENT_CLUSTERS: usize = 100_000;
pub const MAX_EXACT_RARE_EVENT_COUNT: u64 = 100_000;
pub const MAX_EXACT_RARE_TRIALS: u64 = 1_000_000_000;
/// Confirmatory superiority claims require at least this many independent clusters.
pub const MIN_CONFIRMATORY_CLUSTERS: usize = 500;
/// Confirmatory runs use at least 10,000 paired-cluster bootstrap replicates.
pub const MIN_CONFIRMATORY_BOOTSTRAP_REPLICATES: u32 = 10_000;
pub const MAX_FAULT_FAMILIES: usize = 64;
/// Bound total bootstrap work across clusters and stratified families.
pub const MAX_BOOTSTRAP_WORK_UNITS: u128 = 500_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmOutcomeCounts {
    pub planned_faults: u64,
    pub verified_recoveries: u64,
    pub known_nonrecoveries: u64,
    pub indeterminate_recoveries: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmClusterRecord {
    pub fault_schedule_digest: Digest,
    pub topology_digest: Digest,
    pub workload_profile_digest: Digest,
    pub offered_demand_contract_digest: Digest,
    pub outcomes: ArmOutcomeCounts,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairedFamilyStratum {
    pub family_id: String,
    pub candidate: ArmOutcomeCounts,
    pub baseline: ArmOutcomeCounts,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairedCluster {
    pub cluster_id: String,
    pub candidate: ArmClusterRecord,
    pub baseline: ArmClusterRecord,
    /// Mutually exclusive primary fault-family strata that partition each arm.
    pub family_strata: Vec<PairedFamilyStratum>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InferenceClaimMode {
    Exploratory,
    Confirmatory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferencePlan {
    pub schema: String,
    pub claim_mode: InferenceClaimMode,
    pub algorithm_version: String,
    /// Seed is committed by the external freeze/evidence authority; this module
    /// uses the first 64 bits as a deterministic resampling seed.
    pub bootstrap_seed_commitment: Digest,
    pub minimum_independent_clusters: usize,
    pub bootstrap_replicates: u32,
    /// Integer basis points: 9,500 = 95.00%.
    pub confidence_level_bps: u16,
    /// Minimum meaningful paired effect in parts per million.
    pub minimum_meaningful_effect_ppm: i64,
    /// Maximum tolerated mean indeterminate rate, across either arm, in ppm.
    pub maximum_indeterminate_rate_ppm: u32,
    /// Sorted, unique family IDs fixed before sealed scenario generation.
    pub required_fault_families: Vec<String>,
    /// A family may regress by at most this many ppm; the simultaneous lower
    /// confidence bound must remain above its negative value.
    pub maximum_family_regression_ppm: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuperiorityVerdict {
    ExploratoryOnly,
    SuperiorityEstablished,
    NotEstablished,
    IndeterminateCoverageExceeded,
    FaultFamilyRegressionNotCleared,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PpmInterval {
    pub lower_ppm: i64,
    pub upper_ppm: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmInferenceSummary {
    /// Bootstrap CI for the conservative lower estimator: verified / planned.
    pub verified_rate_ci: PpmInterval,
    /// Bootstrap CI for the upper estimator: (verified + indeterminate) / planned.
    pub maximum_possible_rate_ci: PpmInterval,
    pub indeterminate_rate_ci: PpmInterval,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaultFamilyInferenceSummary {
    pub family_id: String,
    pub conservative_effect_lower_mean_ppm: i64,
    pub conservative_effect_upper_mean_ppm: i64,
    /// Bonferroni-adjusted simultaneous interval across all required families.
    pub conservative_effect_lower_simultaneous_ci: PpmInterval,
    pub conservative_effect_upper_simultaneous_ci: PpmInterval,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairedEffectSummary {
    /// Mean of candidate lower minus baseline upper, in ppm.
    pub conservative_effect_lower_mean_ppm: i64,
    /// Mean of candidate upper minus baseline lower, in ppm.
    pub conservative_effect_upper_mean_ppm: i64,
    /// CI for the conservative lower endpoint. Its lower bound is the
    /// non-compensatory superiority gate.
    pub conservative_effect_lower_ci: PpmInterval,
    /// CI for the conservative upper endpoint.
    pub conservative_effect_upper_ci: PpmInterval,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferenceResult {
    pub schema: String,
    pub claim_mode: InferenceClaimMode,
    pub algorithm_version: String,
    pub bootstrap_seed_commitment: Digest,
    pub independent_clusters: usize,
    pub bootstrap_replicates: u32,
    pub confidence_level_bps: u16,
    pub candidate: ArmInferenceSummary,
    pub baseline: ArmInferenceSummary,
    pub paired_effect: PairedEffectSummary,
    pub fault_families: Vec<FaultFamilyInferenceSummary>,
    pub maximum_family_regression_ppm: u32,
    pub mean_candidate_indeterminate_rate_ppm: i64,
    pub mean_baseline_indeterminate_rate_ppm: i64,
    pub maximum_upper_confidence_arm_indeterminate_rate_ppm: i64,
    pub minimum_meaningful_effect_ppm: i64,
    pub maximum_indeterminate_rate_ppm: u32,
    pub maximum_family_regression_ppm: u32,
    pub verdict: SuperiorityVerdict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InferenceError {
    UnsupportedSchema,
    UnsupportedBootstrapAlgorithm,
    EmptyClusterSet,
    TooManyClusters,
    InsufficientIndependentClusters,
    DuplicateCluster(String),
    InvalidClusterIdentity(String),
    ZeroRequiredDigest(String),
    CandidateBaselineScheduleMismatch(String),
    ZeroFaultDenominator(String),
    OutcomeDenominatorMismatch(String, EvaluationArm),
    InvalidBootstrapReplicates,
    InvalidConfidenceLevel,
    InvalidMinimumEffect,
    InvalidIndeterminateCeiling,
    ZeroBootstrapSeed,
    ArithmeticOverflow,
    InvalidRareEventDenominator,
    RareEventCountExceedsTrials,
    ExactRareEventWorkLimitExceeded,
    InvalidRareEventConfidence,
    InvalidRareEventRiskCeiling,
    ConfirmatoryClusterFloorNotMet,
    ConfirmatoryBootstrapFloorNotMet,
    InvalidFaultFamilyPlan,
    InvalidFaultFamilyId(String),
    MissingFaultFamily(String, String),
    DuplicateFaultFamily(String, String),
    UnplannedFaultFamily(String, String),
    FamilyPartitionMismatch(String, EvaluationArm),
    BootstrapWorkLimitExceeded,
    FaultFamilyRegressionNotCleared(String),
}

#[derive(Clone, Copy)]
struct ClusterRates {
    candidate_lower: f64,
    candidate_upper: f64,
    candidate_indeterminate: f64,
    baseline_lower: f64,
    baseline_upper: f64,
    baseline_indeterminate: f64,
}

#[derive(Clone, Copy)]
struct FamilyClusterRates {
    candidate_lower: f64,
    candidate_upper: f64,
    baseline_lower: f64,
    baseline_upper: f64,
}

#[derive(Clone, Copy, Default)]
struct AggregateRates {
    candidate_lower: f64,
    candidate_upper: f64,
    candidate_indeterminate: f64,
    baseline_lower: f64,
    baseline_upper: f64,
    baseline_indeterminate: f64,
    effect_lower: f64,
    effect_upper: f64,
}

pub fn evaluate_paired_clusters(
    plan: &InferencePlan,
    clusters: &[PairedCluster],
) -> Result<InferenceResult, InferenceError> {
    validate_plan(plan)?;
    if clusters.is_empty() {
        return Err(InferenceError::EmptyClusterSet);
    }
    if clusters.len() > MAX_INDEPENDENT_CLUSTERS {
        return Err(InferenceError::TooManyClusters);
    }
    if clusters.len() < plan.minimum_independent_clusters {
        return Err(InferenceError::InsufficientIndependentClusters);
    }

    let mut ids = BTreeSet::new();
    let mut ordered: Vec<&PairedCluster> = clusters.iter().collect();
    ordered.sort_unstable_by(|left, right| left.cluster_id.cmp(&right.cluster_id));

    let mut rates = Vec::with_capacity(ordered.len());
    for cluster in &ordered {
        if cluster.cluster_id.trim().is_empty()
            || cluster.cluster_id.len() > 256
            || cluster.cluster_id.chars().any(char::is_control)
        {
            return Err(InferenceError::InvalidClusterIdentity(
                cluster.cluster_id.clone(),
            ));
        }
        if !ids.insert(cluster.cluster_id.as_str()) {
            return Err(InferenceError::DuplicateCluster(cluster.cluster_id.clone()));
        }
        for (candidate_digest, baseline_digest, name) in [
            (
                &cluster.candidate.fault_schedule_digest,
                &cluster.baseline.fault_schedule_digest,
                "fault_schedule_digest",
            ),
            (
                &cluster.candidate.topology_digest,
                &cluster.baseline.topology_digest,
                "topology_digest",
            ),
            (
                &cluster.candidate.workload_profile_digest,
                &cluster.baseline.workload_profile_digest,
                "workload_profile_digest",
            ),
            (
                &cluster.candidate.offered_demand_contract_digest,
                &cluster.baseline.offered_demand_contract_digest,
                "offered_demand_contract_digest",
            ),
        ] {
            if digest_is_zero(candidate_digest) || digest_is_zero(baseline_digest) {
                return Err(InferenceError::ZeroRequiredDigest(name.into()));
            }
            if candidate_digest != baseline_digest {
                return Err(InferenceError::CandidateBaselineScheduleMismatch(
                    cluster.cluster_id.clone(),
                ));
            }
        }
        validate_arm_counts(
            &cluster.cluster_id,
            EvaluationArm::Candidate,
            &cluster.candidate.outcomes,
        )?;
        validate_arm_counts(
            &cluster.cluster_id,
            EvaluationArm::Baseline,
            &cluster.baseline.outcomes,
        )?;
        if cluster.candidate.outcomes.planned_faults
            != cluster.baseline.outcomes.planned_faults
        {
            return Err(InferenceError::CandidateBaselineScheduleMismatch(
                cluster.cluster_id.clone(),
            ));
        }
        let candidate_counts = &cluster.candidate.outcomes;
        let baseline_counts = &cluster.baseline.outcomes;
        let denominator = candidate_counts.planned_faults as f64;
        let candidate_lower =
            candidate_counts.verified_recoveries as f64 / denominator;
        let candidate_upper = (candidate_counts.verified_recoveries
            + candidate_counts.indeterminate_recoveries) as f64
            / denominator;
        let baseline_lower =
            baseline_counts.verified_recoveries as f64 / denominator;
        let baseline_upper = (baseline_counts.verified_recoveries
            + baseline_counts.indeterminate_recoveries) as f64
            / denominator;
        rates.push(ClusterRates {
            candidate_lower,
            candidate_upper,
            candidate_indeterminate: candidate_counts.indeterminate_recoveries as f64
                / denominator,
            baseline_lower,
            baseline_upper,
            baseline_indeterminate: baseline_counts.indeterminate_recoveries as f64
                / denominator,
        });
    }

    let cluster_count = rates.len();
    let mut rng = SplitMix64::new(seed_from_digest(&plan.bootstrap_seed_commitment));
    let mut candidate_lower_samples = Vec::with_capacity(plan.bootstrap_replicates as usize);
    let mut candidate_upper_samples = Vec::with_capacity(plan.bootstrap_replicates as usize);
    let mut candidate_indeterminate_samples =
        Vec::with_capacity(plan.bootstrap_replicates as usize);
    let mut baseline_lower_samples = Vec::with_capacity(plan.bootstrap_replicates as usize);
    let mut baseline_upper_samples = Vec::with_capacity(plan.bootstrap_replicates as usize);
    let mut baseline_indeterminate_samples =
        Vec::with_capacity(plan.bootstrap_replicates as usize);
    let mut effect_lower_samples = Vec::with_capacity(plan.bootstrap_replicates as usize);
    let mut effect_upper_samples = Vec::with_capacity(plan.bootstrap_replicates as usize);

    for _ in 0..plan.bootstrap_replicates {
        let mut aggregate = AggregateRates::default();
        for _ in 0..cluster_count {
            let index = rng.sample_index(cluster_count);
            let sample = rates[index];
            aggregate.candidate_lower += sample.candidate_lower;
            aggregate.candidate_upper += sample.candidate_upper;
            aggregate.candidate_indeterminate += sample.candidate_indeterminate;
            aggregate.baseline_lower += sample.baseline_lower;
            aggregate.baseline_upper += sample.baseline_upper;
            aggregate.baseline_indeterminate += sample.baseline_indeterminate;
            aggregate.effect_lower += sample.candidate_lower - sample.baseline_upper;
            aggregate.effect_upper += sample.candidate_upper - sample.baseline_lower;
        }
        let scale = 1_000_000.0 / cluster_count as f64;
        candidate_lower_samples.push(aggregate.candidate_lower * scale);
        candidate_upper_samples.push(aggregate.candidate_upper * scale);
        candidate_indeterminate_samples.push(aggregate.candidate_indeterminate * scale);
        baseline_lower_samples.push(aggregate.baseline_lower * scale);
        baseline_upper_samples.push(aggregate.baseline_upper * scale);
        baseline_indeterminate_samples.push(aggregate.baseline_indeterminate * scale);
        effect_lower_samples.push(aggregate.effect_lower * scale);
        effect_upper_samples.push(aggregate.effect_upper * scale);
    }

    let alpha = (10_000 - plan.confidence_level_bps) as f64 / 10_000.0;
    let tail = alpha / 2.0;

    let candidate = ArmInferenceSummary {
        verified_rate_ci: percentile_interval(
            &mut candidate_lower_samples,
            tail,
        ),
        maximum_possible_rate_ci: percentile_interval(
            &mut candidate_upper_samples,
            tail,
        ),
        indeterminate_rate_ci: percentile_interval(
            &mut candidate_indeterminate_samples,
            tail,
        ),
    };
    let baseline = ArmInferenceSummary {
        verified_rate_ci: percentile_interval(
            &mut baseline_lower_samples,
            tail,
        ),
        maximum_possible_rate_ci: percentile_interval(
            &mut baseline_upper_samples,
            tail,
        ),
        indeterminate_rate_ci: percentile_interval(
            &mut baseline_indeterminate_samples,
            tail,
        ),
    };
    let effect_lower_ci = percentile_interval(&mut effect_lower_samples, tail);
    let effect_upper_ci = percentile_interval(&mut effect_upper_samples, tail);

    let scale = 1_000_000.0 / cluster_count as f64;
    let candidate_indeterminate_mean = rates.iter()
        .map(|sample| sample.candidate_indeterminate)
        .sum::<f64>() * scale;
    let baseline_indeterminate_mean = rates.iter()
        .map(|sample| sample.baseline_indeterminate)
        .sum::<f64>() * scale;
    let maximum_indeterminate_upper_ci = candidate.indeterminate_rate_ci.upper_ppm
        .max(baseline.indeterminate_rate_ci.upper_ppm);
    let effect_lower_mean = rates.iter()
        .map(|sample| sample.candidate_lower - sample.baseline_upper)
        .sum::<f64>() * scale;
    let effect_upper_mean = rates.iter()
        .map(|sample| sample.candidate_upper - sample.baseline_lower)
        .sum::<f64>() * scale;

    let verdict = if maximum_indeterminate_upper_ci
        > plan.maximum_indeterminate_rate_ppm as i64
    {
        SuperiorityVerdict::IndeterminateCoverageExceeded
    } else if plan.claim_mode == InferenceClaimMode::Exploratory {
        SuperiorityVerdict::ExploratoryOnly
    } else if effect_lower_ci.lower_ppm > plan.minimum_meaningful_effect_ppm {
        SuperiorityVerdict::SuperiorityEstablished
    } else {
        SuperiorityVerdict::NotEstablished
    };

    Ok(InferenceResult {
        schema: INFERENCE_SCHEMA.into(),
        claim_mode: plan.claim_mode,
        algorithm_version: plan.algorithm_version.clone(),
        bootstrap_seed_commitment: plan.bootstrap_seed_commitment.clone(),
        independent_clusters: cluster_count,
        bootstrap_replicates: plan.bootstrap_replicates,
        confidence_level_bps: plan.confidence_level_bps,
        candidate,
        baseline,
        paired_effect: PairedEffectSummary {
            conservative_effect_lower_mean_ppm: effect_lower_mean.round() as i64,
            conservative_effect_upper_mean_ppm: effect_upper_mean.round() as i64,
            conservative_effect_lower_ci: effect_lower_ci,
            conservative_effect_upper_ci: effect_upper_ci,
        },
        mean_candidate_indeterminate_rate_ppm: candidate_indeterminate_mean.round() as i64,
        mean_baseline_indeterminate_rate_ppm: baseline_indeterminate_mean.round() as i64,
        maximum_upper_confidence_arm_indeterminate_rate_ppm: maximum_indeterminate_upper_ci,
        minimum_meaningful_effect_ppm: plan.minimum_meaningful_effect_ppm,
        maximum_indeterminate_rate_ppm: plan.maximum_indeterminate_rate_ppm,
        verdict,
    })
}

fn validate_plan(plan: &InferencePlan) -> Result<(), InferenceError> {
    if plan.schema != INFERENCE_SCHEMA {
        return Err(InferenceError::UnsupportedSchema);
    }
    if plan.algorithm_version != BOOTSTRAP_ALGORITHM {
        return Err(InferenceError::UnsupportedBootstrapAlgorithm);
    }
    if plan.minimum_independent_clusters < 2
        || plan.minimum_independent_clusters > MAX_INDEPENDENT_CLUSTERS
    {
        return Err(InferenceError::InsufficientIndependentClusters);
    }
    if plan.claim_mode == InferenceClaimMode::Confirmatory
        && plan.minimum_independent_clusters < MIN_CONFIRMATORY_CLUSTERS
    {
        return Err(InferenceError::ConfirmatoryClusterFloorNotMet);
    }
    if plan.claim_mode == InferenceClaimMode::Confirmatory
        && plan.bootstrap_replicates < MIN_CONFIRMATORY_BOOTSTRAP_REPLICATES
    {
        return Err(InferenceError::ConfirmatoryBootstrapFloorNotMet);
    }
    if !(MIN_BOOTSTRAP_REPLICATES..=MAX_BOOTSTRAP_REPLICATES)
        .contains(&plan.bootstrap_replicates)
    {
        return Err(InferenceError::InvalidBootstrapReplicates);
    }
    if !(5_000..10_000).contains(&plan.confidence_level_bps) {
        return Err(InferenceError::InvalidConfidenceLevel);
    }
    if plan.minimum_meaningful_effect_ppm <= 0
        || plan.minimum_meaningful_effect_ppm > 1_000_000
    {
        return Err(InferenceError::InvalidMinimumEffect);
    }
    if plan.maximum_indeterminate_rate_ppm > 1_000_000
        || plan.maximum_family_regression_ppm > 1_000_000
    {
        return Err(InferenceError::InvalidIndeterminateCeiling);
    }
    if plan.required_fault_families.is_empty()
        || plan.required_fault_families.len() > MAX_FAULT_FAMILIES
        || plan.required_fault_families.windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(InferenceError::InvalidFaultFamilyPlan);
    }
    for family_id in &plan.required_fault_families {
        if family_id.trim().is_empty()
            || family_id.len() > 128
            || family_id.chars().any(char::is_control)
        {
            return Err(InferenceError::InvalidFaultFamilyId(family_id.clone()));
        }
    }
    if digest_is_zero(&plan.bootstrap_seed_commitment) {
        return Err(InferenceError::ZeroBootstrapSeed);
    }
    Ok(())
}

fn validate_arm_counts(
    cluster_id: &str,
    arm: EvaluationArm,
    counts: &ArmOutcomeCounts,
) -> Result<(), InferenceError> {
    if counts.planned_faults == 0 {
        return Err(InferenceError::ZeroFaultDenominator(cluster_id.into()));
    }
    let total = counts
        .verified_recoveries
        .checked_add(counts.known_nonrecoveries)
        .and_then(|value| value.checked_add(counts.indeterminate_recoveries))
        .ok_or(InferenceError::ArithmeticOverflow)?;
    if total != counts.planned_faults {
        return Err(InferenceError::OutcomeDenominatorMismatch(
            cluster_id.into(),
            arm,
        ));
    }
    Ok(())
}

fn accumulate_arm_counts(
    total: &mut ArmOutcomeCounts,
    next: &ArmOutcomeCounts,
) -> Result<(), InferenceError> {
    total.planned_faults = total.planned_faults.checked_add(next.planned_faults)
        .ok_or(InferenceError::ArithmeticOverflow)?;
    total.verified_recoveries = total.verified_recoveries.checked_add(next.verified_recoveries)
        .ok_or(InferenceError::ArithmeticOverflow)?;
    total.known_nonrecoveries = total.known_nonrecoveries.checked_add(next.known_nonrecoveries)
        .ok_or(InferenceError::ArithmeticOverflow)?;
    total.indeterminate_recoveries = total.indeterminate_recoveries
        .checked_add(next.indeterminate_recoveries)
        .ok_or(InferenceError::ArithmeticOverflow)?;
    Ok(())
}

fn zero_arm_counts() -> ArmOutcomeCounts {
    ArmOutcomeCounts {
        planned_faults: 0,
        verified_recoveries: 0,
        known_nonrecoveries: 0,
        indeterminate_recoveries: 0,
    }
}

fn percentile_interval(samples: &mut [f64], tail_probability: f64) -> PpmInterval {
    samples.sort_unstable_by(f64::total_cmp);
    let lower = nearest_rank_index(samples.len(), tail_probability);
    let upper = nearest_rank_index(samples.len(), 1.0 - tail_probability);
    PpmInterval {
        lower_ppm: samples[lower].round() as i64,
        upper_ppm: samples[upper].round() as i64,
    }
}

fn nearest_rank_index(length: usize, probability: f64) -> usize {
    let rank = (probability * length as f64).ceil() as usize;
    rank.saturating_sub(1).min(length.saturating_sub(1))
}

fn seed_from_digest(digest: &Digest) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest.0[..8]);
    u64::from_le_bytes(bytes)
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn sample_index(&mut self, length: usize) -> usize {
        let bound = length as u64;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let product = (self.next_u64() as u128) * (bound as u128);
            let low = product as u64;
            if low >= threshold {
                return (product >> 64) as usize;
            }
        }
    }
}

fn digest_is_zero(digest: &Digest) -> bool {
    digest.0.iter().all(|byte| *byte == 0)
}

/// Exact one-sided Clopper-Pearson upper bound for a binomial event rate.
/// The bisection solves P_p[X <= events] = alpha. It is intentionally bounded
/// to rare-event workloads (at most 100,000 observed events and 1e9 trials).
pub fn clopper_pearson_upper(
    events: u64,
    trials: u64,
    confidence_level_bps: u16,
) -> Result<f64, InferenceError> {
    if trials == 0 {
        return Err(InferenceError::InvalidRareEventDenominator);
    }
    if events > trials {
        return Err(InferenceError::RareEventCountExceedsTrials);
    }
    if events > MAX_EXACT_RARE_EVENT_COUNT || trials > MAX_EXACT_RARE_TRIALS {
        return Err(InferenceError::ExactRareEventWorkLimitExceeded);
    }
    if !(5_000..10_000).contains(&confidence_level_bps) {
        return Err(InferenceError::InvalidRareEventConfidence);
    }
    if events == trials {
        return Ok(1.0);
    }
    let alpha = (10_000 - confidence_level_bps) as f64 / 10_000.0;
    if events == 0 {
        return Ok(-((alpha.ln() / trials as f64).exp_m1()));
    }

    let mut lower = 0.0;
    let mut upper = 1.0;
    for _ in 0..100 {
        let midpoint = (lower + upper) / 2.0;
        let cdf = binomial_lower_tail(trials, events, midpoint);
        if cdf > alpha {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
    }
    Ok(upper)
}

fn binomial_lower_tail(trials: u64, events: u64, probability: f64) -> f64 {
    if probability <= 0.0 {
        return 1.0;
    }
    if probability >= 1.0 {
        return 0.0;
    }
    let log_p = probability.ln();
    let log_q = (-probability).ln_1p();
    let mut log_term = trials as f64 * log_q;
    let mut log_sum = f64::NEG_INFINITY;
    for event_index in 0..=events {
        log_sum = log_add_exp(log_sum, log_term);
        if event_index < events {
            log_term += ((trials - event_index) as f64).ln()
                - ((event_index + 1) as f64).ln()
                + log_p
                - log_q;
        }
    }
    log_sum.exp().clamp(0.0, 1.0)
}

fn log_add_exp(left: f64, right: f64) -> f64 {
    if left.is_infinite() && left.is_sign_negative() {
        return right;
    }
    if right.is_infinite() && right.is_sign_negative() {
        return left;
    }
    let maximum = left.max(right);
    maximum + ((left - maximum).exp() + (right - maximum).exp()).ln()
}

/// Minimum independent zero-event clusters needed for a one-sided confidence
/// upper bound no greater than maximum_risk_ppm.
pub fn zero_event_sample_size(
    maximum_risk_ppm: u32,
    confidence_level_bps: u16,
) -> Result<u64, InferenceError> {
    if maximum_risk_ppm == 0 || maximum_risk_ppm >= 1_000_000 {
        return Err(InferenceError::InvalidRareEventRiskCeiling);
    }
    if !(5_000..10_000).contains(&confidence_level_bps) {
        return Err(InferenceError::InvalidRareEventConfidence);
    }
    let alpha = (10_000 - confidence_level_bps) as f64 / 10_000.0;
    let risk = maximum_risk_ppm as f64 / 1_000_000.0;
    Ok((alpha.ln() / (-risk).ln_1p()).ceil() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> Digest {
        Digest::from_byte(byte)
    }

    fn counts(verified: u64, failed: u64, indeterminate: u64) -> ArmOutcomeCounts {
        ArmOutcomeCounts {
            planned_faults: verified + failed + indeterminate,
            verified_recoveries: verified,
            known_nonrecoveries: failed,
            indeterminate_recoveries: indeterminate,
        }
    }

    fn arm_record(outcomes: ArmOutcomeCounts) -> ArmClusterRecord {
        ArmClusterRecord {
            fault_schedule_digest: digest(1),
            topology_digest: digest(2),
            workload_profile_digest: digest(3),
            offered_demand_contract_digest: digest(4),
            outcomes,
        }
    }

    fn cluster(id: &str, candidate: ArmOutcomeCounts, baseline: ArmOutcomeCounts) -> PairedCluster {
        PairedCluster {
            cluster_id: id.into(),
            candidate: arm_record(candidate),
            baseline: arm_record(baseline),
        }
    }

    fn plan(seed: u8, clusters: usize) -> InferencePlan {
        InferencePlan {
            schema: INFERENCE_SCHEMA.into(),
            claim_mode: InferenceClaimMode::Exploratory,
            algorithm_version: BOOTSTRAP_ALGORITHM.into(),
            bootstrap_seed_commitment: digest(seed),
            minimum_independent_clusters: clusters,
            bootstrap_replicates: 1_000,
            confidence_level_bps: 9_500,
            minimum_meaningful_effect_ppm: 50_000,
            maximum_indeterminate_rate_ppm: 100_000,
        }
    }

    fn clean_pairs() -> Vec<PairedCluster> {
        (0..8)
            .map(|index| {
                cluster(
                    &format!("cluster-{index:02}"),
                    counts(8, 1, 1),
                    counts(5, 4, 1),
                )
            })
            .collect()
    }

    #[test]
    fn perfect_matched_candidate_advantage_survives_indeterminate_upper_bound() {
        let clusters = clean_pairs();
        let result = evaluate_paired_clusters(&plan(90, 8), &clusters).unwrap();
        assert_eq!(result.independent_clusters, 8);
        assert_eq!(result.claim_mode, InferenceClaimMode::Exploratory);
        assert_eq!(result.candidate.verified_rate_ci.lower_ppm, 800_000);
        assert_eq!(result.candidate.maximum_possible_rate_ci.upper_ppm, 900_000);
        assert_eq!(result.baseline.verified_rate_ci.lower_ppm, 500_000);
        assert_eq!(result.paired_effect.conservative_effect_lower_mean_ppm, 200_000);
        assert_eq!(result.paired_effect.conservative_effect_upper_mean_ppm, 400_000);
        assert_eq!(result.verdict, SuperiorityVerdict::ExploratoryOnly);
    }

    #[test]
    fn confirmatory_superiority_requires_five_hundred_clusters_and_ten_thousand_replicates() {
        let clusters: Vec<PairedCluster> = (0..MIN_CONFIRMATORY_CLUSTERS)
            .map(|index| cluster(&format!("confirmatory-{index:04}"), counts(8, 1, 1), counts(5, 4, 1)))
            .collect();
        let mut configured = plan(89, MIN_CONFIRMATORY_CLUSTERS);
        configured.claim_mode = InferenceClaimMode::Confirmatory;
        configured.bootstrap_replicates = MIN_CONFIRMATORY_BOOTSTRAP_REPLICATES;
        let result = evaluate_paired_clusters(&configured, &clusters).unwrap();
        assert_eq!(result.claim_mode, InferenceClaimMode::Confirmatory);
        assert_eq!(result.verdict, SuperiorityVerdict::SuperiorityEstablished);
        assert_eq!(result.independent_clusters, MIN_CONFIRMATORY_CLUSTERS);
        assert_eq!(result.bootstrap_replicates, MIN_CONFIRMATORY_BOOTSTRAP_REPLICATES);

        configured.minimum_independent_clusters = MIN_CONFIRMATORY_CLUSTERS - 1;
        assert_eq!(evaluate_paired_clusters(&configured, &clusters), Err(InferenceError::ConfirmatoryClusterFloorNotMet));
        configured.minimum_independent_clusters = MIN_CONFIRMATORY_CLUSTERS;
        configured.bootstrap_replicates = MIN_CONFIRMATORY_BOOTSTRAP_REPLICATES - 1;
        assert_eq!(evaluate_paired_clusters(&configured, &clusters), Err(InferenceError::ConfirmatoryBootstrapFloorNotMet));
    }

    #[test]
    fn indeterminate_outcomes_widen_effect_bounds_instead_of_becoming_failures_or_passes() {
        let clusters = vec![cluster("c-1", counts(7, 1, 2), counts(5, 4, 1)); 8];
        let result = evaluate_paired_clusters(&plan(91, 8), &clusters).unwrap();
        assert_eq!(result.paired_effect.conservative_effect_lower_mean_ppm, 100_000);
        assert_eq!(result.paired_effect.conservative_effect_upper_mean_ppm, 400_000);
    }

    #[test]
    fn excessive_indeterminate_coverage_blocks_superiority() {
        let clusters = vec![cluster("c-1", counts(5, 0, 5), counts(0, 5, 5)); 8];
        let mut configured = plan(92, 8);
        configured.maximum_indeterminate_rate_ppm = 100_000;
        let result = evaluate_paired_clusters(&configured, &clusters).unwrap();
        assert_eq!(result.verdict, SuperiorityVerdict::IndeterminateCoverageExceeded);
    }

    #[test]
    fn bootstrap_is_deterministic_and_cluster_order_independent() {
        let clusters = clean_pairs();
        let plan = plan(93, 8);
        let forward = evaluate_paired_clusters(&plan, &clusters).unwrap();
        let mut reverse = clusters.clone();
        reverse.reverse();
        let backward = evaluate_paired_clusters(&plan, &reverse).unwrap();
        assert_eq!(forward, backward);
    }

    #[test]
    fn splitmix64_known_answer_stream_is_stable() {
        let mut rng = SplitMix64::new(0);
        assert_eq!(rng.next_u64(), 0xe220_a839_7b1d_cdaf);
        assert_eq!(rng.next_u64(), 0x6e78_9e6a_a1b9_65f4);
        assert_eq!(rng.next_u64(), 0x06c4_5d18_8009_454f);
        assert_eq!(rng.next_u64(), 0xf88b_b8a8_724c_81ec);
    }

    #[test]
    fn nearest_rank_percentiles_use_the_preregistered_rank_rule() {
        assert_eq!(nearest_rank_index(100, 0.025), 2);
        assert_eq!(nearest_rank_index(100, 0.975), 97);
        assert_eq!(nearest_rank_index(1_000, 0.025), 24);
        assert_eq!(nearest_rank_index(1_000, 0.975), 974);
    }

    #[test]
    fn zero_event_exact_upper_bound_matches_closed_form_and_sample_size() {
        let upper = clopper_pearson_upper(0, 500, 9_500).unwrap();
        assert!((upper - 0.0059735).abs() < 0.00001);
        assert_eq!(zero_event_sample_size(1_000, 9_500).unwrap(), 2_995);
    }

    #[test]
    fn one_event_exact_upper_bound_is_above_zero_event_bound() {
        let zero = clopper_pearson_upper(0, 100, 9_500).unwrap();
        let one = clopper_pearson_upper(1, 100, 9_500).unwrap();
        assert!(one > zero);
        assert!((one - 0.04656).abs() < 0.002);
    }

    #[test]
    fn schedule_digest_mismatch_is_rejected() {
        let mut clusters = clean_pairs();
        clusters[0].baseline.workload_profile_digest = digest(99);
        assert_eq!(
            evaluate_paired_clusters(&plan(94, 8), &clusters),
            Err(InferenceError::CandidateBaselineScheduleMismatch("cluster-00".into()))
        );
    }

    #[test]
    fn cluster_ids_must_be_unique_and_denominators_must_close() {
        let mut clusters = clean_pairs();
        clusters[1].cluster_id = clusters[0].cluster_id.clone();
        assert_eq!(
            evaluate_paired_clusters(&plan(95, 8), &clusters),
            Err(InferenceError::DuplicateCluster("cluster-00".into()))
        );
        let mut clusters = clean_pairs();
        clusters[0].candidate.outcomes.known_nonrecoveries -= 1;
        assert_eq!(
            evaluate_paired_clusters(&plan(96, 8), &clusters),
            Err(InferenceError::OutcomeDenominatorMismatch(
                "cluster-00".into(), EvaluationArm::Candidate
            ))
        );
    }

    #[test]
    fn bootstrap_plan_rejects_weak_or_ambiguous_configuration() {
        let clusters = clean_pairs();
        let mut p = plan(97, 8);
        p.bootstrap_replicates = 999;
        assert_eq!(evaluate_paired_clusters(&p, &clusters), Err(InferenceError::InvalidBootstrapReplicates));
        let mut p = plan(98, 8);
        p.confidence_level_bps = 10_000;
        assert_eq!(evaluate_paired_clusters(&p, &clusters), Err(InferenceError::InvalidConfidenceLevel));
        let mut p = plan(99, 8);
        p.bootstrap_seed_commitment = digest(0);
        assert_eq!(evaluate_paired_clusters(&p, &clusters), Err(InferenceError::ZeroBootstrapSeed));
    }

    #[test]
    fn confirmatory_mode_cannot_be_preregistered_below_the_claim_floors() {
        let clusters = clean_pairs();
        let mut configured = plan(88, 8);
        configured.claim_mode = InferenceClaimMode::Confirmatory;
        configured.bootstrap_replicates = MIN_CONFIRMATORY_BOOTSTRAP_REPLICATES;
        assert_eq!(
            evaluate_paired_clusters(&configured, &clusters),
            Err(InferenceError::ConfirmatoryClusterFloorNotMet)
        );
        configured.minimum_independent_clusters = MIN_CONFIRMATORY_CLUSTERS;
        configured.bootstrap_replicates = MIN_CONFIRMATORY_BOOTSTRAP_REPLICATES - 1;
        assert_eq!(
            evaluate_paired_clusters(&configured, &clusters),
            Err(InferenceError::ConfirmatoryBootstrapFloorNotMet)
        );
    }

    #[test]
    fn zero_event_sample_size_is_minimal_at_the_declared_risk_ceiling() {
        let required = zero_event_sample_size(1_000, 9_500).unwrap();
        assert_eq!(required, 2_995);
        assert!(clopper_pearson_upper(0, required, 9_500).unwrap() <= 0.001);
        assert!(clopper_pearson_upper(0, required - 1, 9_500).unwrap() > 0.001);
    }

    #[test]
    fn rare_event_edge_cases_are_explicit() {
        assert_eq!(
            clopper_pearson_upper(0, 0, 9_500),
            Err(InferenceError::InvalidRareEventDenominator)
        );
        assert_eq!(
            clopper_pearson_upper(2, 1, 9_500),
            Err(InferenceError::RareEventCountExceedsTrials)
        );
        assert_eq!(clopper_pearson_upper(100, 100, 9_500).unwrap(), 1.0);
        assert_eq!(
            zero_event_sample_size(0, 9_500),
            Err(InferenceError::InvalidRareEventRiskCeiling)
        );
    }

    #[test]
    fn forty_eight_adversarial_cluster_mutations_fail_closed() {
        for vector in 0..48 {
            let mut clusters = clean_pairs();
            let mut configured = plan(100, 8);
            match vector {
                0 => { clusters.pop(); }
                1 => { let extra = clusters[0].clone(); clusters.push(extra); }
                2 => clusters[0].cluster_id.clear(),
                3 => clusters[0].candidate.fault_schedule_digest = digest(0),
                4 => clusters[0].candidate.topology_digest = digest(0),
                5 => clusters[0].candidate.workload_profile_digest = digest(0),
                6 => clusters[0].candidate.offered_demand_contract_digest = digest(0),
                7 => clusters[0].candidate.outcomes.planned_faults = 0,
                8 => clusters[0].candidate.outcomes.verified_recoveries += 1,
                9 => clusters[0].candidate.outcomes.known_nonrecoveries -= 1,
                10 => clusters[0].candidate.outcomes.indeterminate_recoveries += 1,
                11 => clusters[0].baseline.outcomes.verified_recoveries += 1,
                12 => clusters[0].baseline.outcomes.known_nonrecoveries -= 1,
                13 => clusters[0].baseline.outcomes.indeterminate_recoveries += 1,
                14 => clusters[0].baseline.outcomes.planned_faults += 1,
                15 => clusters[0].candidate.outcomes.planned_faults += 1,
                16 => configured.minimum_independent_clusters = 9,
                17 => configured.minimum_independent_clusters = 1,
                18 => configured.bootstrap_replicates = 0,
                19 => configured.bootstrap_replicates = MAX_BOOTSTRAP_REPLICATES + 1,
                20 => configured.confidence_level_bps = 4_999,
                21 => configured.confidence_level_bps = 10_000,
                22 => configured.minimum_meaningful_effect_ppm = 0,
                23 => configured.minimum_meaningful_effect_ppm = 1_000_001,
                24 => configured.maximum_indeterminate_rate_ppm = 1_000_001,
                25 => configured.bootstrap_seed_commitment = digest(0),
                26 => configured.algorithm_version = "unversioned".into(),
                27 => configured.schema = "wrong".into(),
                28 => clusters[1].baseline.fault_schedule_digest = digest(77),
                29 => clusters[1].baseline.topology_digest = digest(77),
                30 => clusters[1].baseline.workload_profile_digest = digest(77),
                31 => clusters[1].baseline.offered_demand_contract_digest = digest(77),
                32 => clusters[0].baseline.outcomes.verified_recoveries = clusters[0].baseline.outcomes.planned_faults + 1,
                33 => clusters[0].candidate.outcomes.known_nonrecoveries = u64::MAX,
                34 => clusters[0].baseline.outcomes.indeterminate_recoveries = u64::MAX,
                35 => clusters[0].cluster_id = "c\n0".into(),
                36 => clusters[0].cluster_id = "x".repeat(257),
                37 => clusters[0].candidate.outcomes.planned_faults = 1,
                38 => clusters[0].baseline.outcomes.planned_faults = 2,
                39 => configured.bootstrap_replicates = MIN_BOOTSTRAP_REPLICATES - 1,
                40 => configured.minimum_independent_clusters = MAX_INDEPENDENT_CLUSTERS + 1,
                41 => clusters[2].cluster_id = clusters[0].cluster_id.clone(),
                42 => clusters[1].candidate.outcomes.verified_recoveries = clusters[1].candidate.outcomes.planned_faults + 1,
                43 => clusters[1].baseline.outcomes.known_nonrecoveries = u64::MAX,
                44 => clusters[3].candidate.fault_schedule_digest = digest(0),
                45 => clusters[3].candidate.topology_digest = digest(0),
                46 => clusters[4].candidate.workload_profile_digest = digest(0),
                _ => clusters[5].candidate.offered_demand_contract_digest = digest(0),
            }
            assert!(
                evaluate_paired_clusters(&configured, &clusters).is_err(),
                "cluster inference adversarial vector {vector} was accepted"
            );
        }
    }
}
