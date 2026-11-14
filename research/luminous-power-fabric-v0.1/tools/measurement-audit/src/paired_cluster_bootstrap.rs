use crate::capability_envelope::EvidenceStatus;
use crate::lifecycle_service_cost::LifecycleEvidenceClass;
use crate::AuditError;
use std::collections::BTreeMap;

const MIN_CLUSTERS: usize = 30;
const MAX_CLUSTERS: usize = 100_000;
const MIN_ITERATIONS: usize = 1_000;
const MAX_ITERATIONS: usize = 100_000;
const CLAIM: &str = "PAIRED_CLUSTER_BOOTSTRAP_SUMMARY_ONLY_NOT_INDEPENDENCE_PROOF_OR_FIELD_SUPERIORITY";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterLevel {
    Mission,
    SiteMission,
    EventMission,
    SiteEventMission,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MissionClusterOutcome {
    pub cluster_id: String,
    pub scenario_seed_sha256: String,
    pub mission_count: usize,
    pub completed_mission_count: usize,
    pub total_cost_minor_units: u128,
    pub required_critical_service_wh: f64,
    pub delivered_critical_service_wh: f64,
    pub unserved_critical_service_wh: f64,
    pub system_unavailable_hours: f64,
    pub evidence_status: EvidenceStatus,
    pub evidence_class: LifecycleEvidenceClass,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairedBootstrapCandidate {
    pub candidate_id: String,
    pub cluster_level: ClusterLevel,
    /// Data-owner protocol/design reference. This code cannot prove independence.
    pub cluster_independence_evidence_reference: String,
    pub scenario_manifest_sha256: String,
    pub clusters: Vec<MissionClusterOutcome>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PracticalBenefitThresholds {
    pub cost_per_delivered_critical_kwh_minor_units: f64,
    pub unserved_critical_service_wh: f64,
    pub system_unavailable_hours: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonFinding {
    PracticalSuperioritySupported,
    PracticalHarmSupported,
    NotEstablished,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairedClusterBootstrapInput {
    pub reference_candidate_id: String,
    pub scenario_manifest_sha256: String,
    pub bootstrap_seed_sha256: String,
    /// Digest of the preregistered endpoint family, confidence and decision rules.
    pub preregistered_protocol_sha256: String,
    pub minimum_practical_benefit: PracticalBenefitThresholds,
    pub confidence_level: f64,
    pub bootstrap_iterations: usize,
    pub candidates: Vec<PairedBootstrapCandidate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BootstrapInterval {
    /// Candidate minus reference. Negative is favorable for these metrics.
    pub paired_mean_delta: f64,
    /// Pointwise percentile interval; not multiplicity-adjusted.
    pub lower_bound: f64,
    pub upper_bound: f64,
    /// Simultaneous standardized-bootstrap interval across all candidate/endpoint hypotheses.
    pub familywise_lower_bound: f64,
    pub familywise_upper_bound: f64,
    pub minimum_practical_benefit: f64,
    pub finding: ComparisonFinding,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairedBootstrapComparison {
    pub reference_candidate_id: String,
    pub candidate_id: String,
    pub paired_cluster_count: usize,
    pub failed_mission_count_reference: usize,
    pub failed_mission_count_candidate: usize,
    pub cost_per_delivered_critical_kwh_delta: BootstrapInterval,
    pub unserved_critical_service_wh_delta: BootstrapInterval,
    pub system_unavailable_hours_delta: BootstrapInterval,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairedClusterBootstrapReport {
    pub reference_candidate_id: String,
    pub scenario_manifest_sha256: String,
    pub preregistered_protocol_sha256: String,
    pub confidence_level: f64,
    pub familywise_confidence_level: f64,
    pub hypothesis_family_size: usize,
    pub bootstrap_iterations: usize,
    pub bootstrap_algorithm: &'static str,
    pub simultaneous_interval_method: &'static str,
    pub resampling_unit: &'static str,
    pub comparisons: Vec<PairedBootstrapComparison>,
    pub claim_ceiling: &'static str,
}

impl PairedClusterBootstrapReport {
    pub const fn claim_ceiling(&self) -> &'static str { CLAIM }
}

fn nonblank(value: &str) -> bool { !value.trim().is_empty() }

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn finite_nonnegative(value: f64) -> bool { value.is_finite() && value >= 0.0 }

fn seed_from_digest(digest: &str) -> Result<u64, AuditError> {
    if !valid_sha256(digest) {
        return Err(AuditError("bootstrap seed must be a SHA-256 digest".to_string()));
    }
    u64::from_str_radix(&digest[..16], 16)
        .map_err(|_| AuditError("bootstrap seed prefix is invalid".to_string()))
}

#[derive(Debug, Clone, Copy)]
struct SplitMix64 { state: u64 }

impl SplitMix64 {
    fn new(seed: u64) -> Self { Self { state: seed } }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }

    fn index(&mut self, length: usize) -> usize {
        let bound = length as u64;
        let zone = u64::MAX - (u64::MAX % bound);
        loop {
            let value = self.next_u64();
            if value < zone { return (value % bound) as usize; }
        }
    }
}

fn percentile(sorted: &[f64], probability: f64) -> Result<f64, AuditError> {
    if sorted.is_empty() || !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
        return Err(AuditError("bootstrap percentile input is invalid".to_string()));
    }
    let index = (probability * (sorted.len() - 1) as f64).floor() as usize;
    sorted.get(index).copied()
        .ok_or_else(|| AuditError("bootstrap percentile index is out of range".to_string()))
}

fn mean(values: &[f64]) -> Result<f64, AuditError> {
    if values.is_empty() {
        return Err(AuditError("cannot average an empty paired cluster set".to_string()));
    }
    let sum = values.iter().try_fold(0.0_f64, |accumulator, value| {
        let next = accumulator + value;
        next.is_finite().then_some(next)
            .ok_or_else(|| AuditError("paired delta sum overflowed".to_string()))
    })?;
    let result = sum / values.len() as f64;
    if !result.is_finite() {
        return Err(AuditError("paired delta mean is not finite".to_string()));
    }
    Ok(result)
}

#[derive(Debug, Clone, Copy)]
struct Aggregate {
    cost_minor_units: u128,
    delivered_wh: f64,
    unserved_wh_mean: f64,
    unavailable_hours_mean: f64,
}

fn aggregate(
    clusters: &[&MissionClusterOutcome],
    indices: &[usize],
) -> Result<Aggregate, AuditError> {
    let mut cost_minor_units = 0_u128;
    let mut delivered_wh = 0.0_f64;
    let mut unserved_wh = 0.0_f64;
    let mut unavailable_hours = 0.0_f64;

    for index in indices {
        let cluster = clusters.get(*index)
            .ok_or_else(|| AuditError("bootstrap cluster index is invalid".to_string()))?;
        cost_minor_units = cost_minor_units.checked_add(cluster.total_cost_minor_units)
            .ok_or_else(|| AuditError("bootstrap cost sum overflowed".to_string()))?;
        delivered_wh += cluster.delivered_critical_service_wh;
        unserved_wh += cluster.unserved_critical_service_wh;
        unavailable_hours += cluster.system_unavailable_hours;
    }

    let count = indices.len() as f64;
    if !delivered_wh.is_finite() || !unserved_wh.is_finite() || !unavailable_hours.is_finite() {
        return Err(AuditError("bootstrap service or downtime aggregate overflowed".to_string()));
    }
    Ok(Aggregate {
        cost_minor_units,
        delivered_wh,
        unserved_wh_mean: unserved_wh / count,
        unavailable_hours_mean: unavailable_hours / count,
    })
}

fn cost_per_delivered_kwh(aggregate: Aggregate) -> Result<f64, AuditError> {
    if aggregate.delivered_wh <= 0.0 {
        return Err(AuditError(
            "bootstrap resample has zero delivered critical service; comparison cannot silently censor this draw"
                .to_string()
        ));
    }
    if aggregate.cost_minor_units > (1_u128 << 53) {
        return Err(AuditError("bootstrap cost exceeds exactly representable f64 integer range".to_string()));
    }
    let result = aggregate.cost_minor_units as f64 * 1_000.0 / aggregate.delivered_wh;
    if !result.is_finite() {
        return Err(AuditError("bootstrap cost per delivered kWh is not finite".to_string()));
    }
    Ok(result)
}

fn validate_candidate<'a>(
    candidate: &'a PairedBootstrapCandidate,
    manifest: &str,
) -> Result<BTreeMap<String, &'a MissionClusterOutcome>, AuditError> {
    if !nonblank(&candidate.candidate_id)
        || !nonblank(&candidate.cluster_independence_evidence_reference)
        || candidate.scenario_manifest_sha256 != manifest
        || candidate.clusters.len() < MIN_CLUSTERS
        || candidate.clusters.len() > MAX_CLUSTERS
    {
        return Err(AuditError(format!(
            "candidate {:?} has invalid cluster metadata or count",
            candidate.candidate_id
        )));
    }

    let mut clusters = BTreeMap::new();
    for cluster in &candidate.clusters {
        if !nonblank(&cluster.cluster_id)
            || !valid_sha256(&cluster.scenario_seed_sha256)
            || cluster.mission_count == 0
            || cluster.completed_mission_count > cluster.mission_count
            || !finite_nonnegative(cluster.required_critical_service_wh)
            || !finite_nonnegative(cluster.delivered_critical_service_wh)
            || !finite_nonnegative(cluster.unserved_critical_service_wh)
            || cluster.delivered_critical_service_wh > cluster.required_critical_service_wh + 1.0e-9
            || !finite_nonnegative(cluster.system_unavailable_hours)
            || cluster.evidence_status != EvidenceStatus::Current
            || cluster.evidence_class == LifecycleEvidenceClass::Assumption
            || !nonblank(&cluster.evidence_reference)
            || (cluster.required_critical_service_wh - cluster.delivered_critical_service_wh - cluster.unserved_critical_service_wh).abs() > 1.0e-6
        {
            return Err(AuditError(format!(
                "candidate {:?} cluster {:?} contains invalid outcomes",
                candidate.candidate_id, cluster.cluster_id
            )));
        }
        if clusters.insert(cluster.cluster_id.clone(), cluster).is_some() {
            return Err(AuditError(format!(
                "candidate {:?} has duplicate cluster IDs",
                candidate.candidate_id
            )));
        }
    }
    Ok(clusters)
}

fn sample_stddev(values: &[f64]) -> Result<f64, AuditError> {
    if values.len() < 2 || values.iter().any(|value| !value.is_finite()) {
        return Err(AuditError("bootstrap standard deviation input is invalid".to_string()));
    }
    let center = mean(values)?;
    let squared = values.iter().try_fold(0.0_f64, |sum, value| {
        let delta = value - center;
        let next = sum + delta * delta;
        next.is_finite().then_some(next)
            .ok_or_else(|| AuditError("bootstrap variance overflowed".to_string()))
    })?;
    let result = (squared / (values.len() - 1) as f64).sqrt();
    if !result.is_finite() {
        return Err(AuditError("bootstrap standard deviation is not finite".to_string()));
    }
    Ok(result)
}

fn pointwise_interval(
    samples: &[f64],
    iterations: usize,
    confidence: f64,
) -> Result<(f64, f64), AuditError> {
    if samples.len() != iterations || samples.iter().any(|value| !value.is_finite()) {
        return Err(AuditError("bootstrap samples are invalid".to_string()));
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let alpha = 1.0 - confidence;
    Ok((
        percentile(&sorted, alpha * 0.5)?,
        percentile(&sorted, 1.0 - alpha * 0.5)?,
    ))
}

fn finding(lower: f64, upper: f64, minimum_benefit: f64) -> ComparisonFinding {
    if upper < -minimum_benefit {
        ComparisonFinding::PracticalSuperioritySupported
    } else if lower > minimum_benefit {
        ComparisonFinding::PracticalHarmSupported
    } else {
        ComparisonFinding::NotEstablished
    }
}

struct CandidateBootstrapWork<'a> {
    candidate_id: String,
    candidate_clusters: Vec<&'a MissionClusterOutcome>,
    failed_mission_count: usize,
    cost_observed: f64,
    unserved_observed: f64,
    downtime_observed: f64,
    cost_samples: Vec<f64>,
    unserved_samples: Vec<f64>,
    downtime_samples: Vec<f64>,
}

fn update_max_statistics(
    samples: &[f64],
    observed: f64,
    standard_deviation: f64,
    maxima: &mut [f64],
) -> Result<(), AuditError> {
    if samples.len() != maxima.len() || !observed.is_finite() || !standard_deviation.is_finite() {
        return Err(AuditError("simultaneous statistic dimensions are invalid".to_string()));
    }
    if standard_deviation <= f64::EPSILON {
        if samples.iter().any(|sample| (sample - observed).abs() > 1.0e-8) {
            return Err(AuditError("zero-variance bootstrap samples do not reconcile to the observed estimate".to_string()));
        }
        return Ok(());
    }
    for (index, sample) in samples.iter().enumerate() {
        let statistic = ((sample - observed) / standard_deviation).abs();
        if !statistic.is_finite() {
            return Err(AuditError("standardized bootstrap statistic is not finite".to_string()));
        }
        maxima[index] = maxima[index].max(statistic);
    }
    Ok(())
}

fn build_interval(
    samples: &[f64],
    observed: f64,
    standard_deviation: f64,
    critical_value: f64,
    confidence: f64,
    minimum_benefit: f64,
) -> Result<BootstrapInterval, AuditError> {
    let (lower_bound, upper_bound) = pointwise_interval(samples, samples.len(), confidence)?;
    let radius = critical_value * standard_deviation;
    let familywise_lower_bound = observed - radius;
    let familywise_upper_bound = observed + radius;
    if !familywise_lower_bound.is_finite() || !familywise_upper_bound.is_finite() {
        return Err(AuditError("simultaneous interval is not finite".to_string()));
    }
    Ok(BootstrapInterval {
        paired_mean_delta: observed,
        lower_bound,
        upper_bound,
        familywise_lower_bound,
        familywise_upper_bound,
        minimum_practical_benefit: minimum_benefit,
        finding: finding(familywise_lower_bound, familywise_upper_bound, minimum_benefit),
    })
}

/// Paired cluster bootstrap with one shared resample stream across candidates and
/// endpoints. A max-absolute-standardized-error envelope produces simultaneous
/// familywise intervals. Pointwise percentile intervals are retained descriptively.
/// The practical-effect finding requires the simultaneous bound to clear a
/// preregistered minimum benefit; this function never creates a scalar overall winner.
pub fn evaluate_paired_cluster_bootstrap(
    input: &PairedClusterBootstrapInput,
) -> Result<PairedClusterBootstrapReport, AuditError> {
    let thresholds = [
        input.minimum_practical_benefit.cost_per_delivered_critical_kwh_minor_units,
        input.minimum_practical_benefit.unserved_critical_service_wh,
        input.minimum_practical_benefit.system_unavailable_hours,
    ];
    if !nonblank(&input.reference_candidate_id)
        || !valid_sha256(&input.scenario_manifest_sha256)
        || !valid_sha256(&input.bootstrap_seed_sha256)
        || !valid_sha256(&input.preregistered_protocol_sha256)
        || thresholds.iter().any(|value| !value.is_finite() || *value <= 0.0)
        || !input.confidence_level.is_finite()
        || input.confidence_level <= 0.5
        || input.confidence_level >= 1.0
        || input.bootstrap_iterations < MIN_ITERATIONS
        || input.bootstrap_iterations > MAX_ITERATIONS
        || input.candidates.len() < 2
        || input.candidates.len() > 32
    {
        return Err(AuditError("paired bootstrap metadata, protocol digest, or practical thresholds are invalid".to_string()));
    }

    let mut candidate_ids = std::collections::BTreeSet::new();
    let mut validated = BTreeMap::new();
    for candidate in &input.candidates {
        if !candidate_ids.insert(candidate.candidate_id.as_str()) {
            return Err(AuditError("paired bootstrap candidate IDs must be unique".to_string()));
        }
        if candidate.cluster_level != input.candidates[0].cluster_level {
            return Err(AuditError("cluster level must match across candidates".to_string()));
        }
        let clusters = validate_candidate(candidate, &input.scenario_manifest_sha256)?;
        validated.insert(candidate.candidate_id.as_str(), clusters);
    }

    let reference = validated.get(input.reference_candidate_id.as_str())
        .ok_or_else(|| AuditError("reference candidate is missing".to_string()))?;
    let reference_ids: Vec<String> = reference.keys().cloned().collect();
    let reference_clusters: Vec<&MissionClusterOutcome> = reference_ids.iter()
        .map(|id| reference.get(id).copied()
            .ok_or_else(|| AuditError("reference cluster lookup failed".to_string())))
        .collect::<Result<_, _>>()?;

    for (candidate_id, clusters) in &validated {
        if clusters.len() != reference.len() || clusters.keys().ne(reference.keys()) {
            return Err(AuditError(format!("candidate {candidate_id:?} does not contain the exact reference cluster-ID set")));
        }
        for (cluster_id, reference_cluster) in reference.iter() {
            let candidate_cluster = clusters.get(cluster_id)
                .ok_or_else(|| AuditError("paired cluster lookup failed".to_string()))?;
            if candidate_cluster.scenario_seed_sha256 != reference_cluster.scenario_seed_sha256 {
                return Err(AuditError(format!("scenario seed mismatch for cluster {cluster_id:?}")));
            }
            if candidate_cluster.mission_count != reference_cluster.mission_count
                || (candidate_cluster.required_critical_service_wh
                    - reference_cluster.required_critical_service_wh).abs() > 1.0e-6 {
                return Err(AuditError(format!("paired workload mismatch for cluster {cluster_id:?}")));
            }
        }
    }

    let full_indices: Vec<usize> = (0..reference_ids.len()).collect();
    let reference_full = aggregate(&reference_clusters, &full_indices)?;
    let mut work = Vec::new();
    for (candidate_id, clusters) in &validated {
        if *candidate_id == input.reference_candidate_id.as_str() { continue; }
        let candidate_clusters: Vec<&MissionClusterOutcome> = reference_ids.iter()
            .map(|id| clusters.get(id).copied()
                .ok_or_else(|| AuditError("candidate cluster lookup failed".to_string())))
            .collect::<Result<_, _>>()?;
        let candidate_full = aggregate(&candidate_clusters, &full_indices)?;
        let failed_mission_count = candidate_clusters.iter().try_fold(0_usize, |sum, cluster| {
            sum.checked_add(cluster.mission_count - cluster.completed_mission_count)
                .ok_or_else(|| AuditError("candidate failed-mission count overflowed".to_string()))
        })?;
        work.push(CandidateBootstrapWork {
            candidate_id: (*candidate_id).to_string(),
            candidate_clusters,
            failed_mission_count,
            cost_observed: cost_per_delivered_kwh(candidate_full)? - cost_per_delivered_kwh(reference_full)?,
            unserved_observed: candidate_full.unserved_wh_mean - reference_full.unserved_wh_mean,
            downtime_observed: candidate_full.unavailable_hours_mean - reference_full.unavailable_hours_mean,
            cost_samples: Vec::with_capacity(input.bootstrap_iterations),
            unserved_samples: Vec::with_capacity(input.bootstrap_iterations),
            downtime_samples: Vec::with_capacity(input.bootstrap_iterations),
        });
    }

    let family_size = work.len().checked_mul(3)
        .ok_or_else(|| AuditError("hypothesis family size overflowed".to_string()))?;
    let statistic_count = family_size.checked_mul(input.bootstrap_iterations)
        .ok_or_else(|| AuditError("bootstrap statistic count overflowed".to_string()))?;
    if statistic_count > 10_000_000 {
        return Err(AuditError("comparison family and iteration count exceed the bounded memory budget".to_string()));
    }

    // Every candidate and endpoint uses the same resampled cluster indices.
    let mut rng = SplitMix64::new(seed_from_digest(&input.bootstrap_seed_sha256)?);
    for _ in 0..input.bootstrap_iterations {
        let indices: Vec<usize> = (0..reference_ids.len())
            .map(|_| rng.index(reference_ids.len())).collect();
        let reference_sample = aggregate(&reference_clusters, &indices)?;
        let reference_cost = cost_per_delivered_kwh(reference_sample)?;
        for candidate in &mut work {
            let sample = aggregate(&candidate.candidate_clusters, &indices)?;
            candidate.cost_samples.push(cost_per_delivered_kwh(sample)? - reference_cost);
            candidate.unserved_samples.push(sample.unserved_wh_mean - reference_sample.unserved_wh_mean);
            candidate.downtime_samples.push(sample.unavailable_hours_mean - reference_sample.unavailable_hours_mean);
        }
    }

    let mut family_maxima = vec![0.0_f64; input.bootstrap_iterations];
    for candidate in &work {
        let cost_sd = sample_stddev(&candidate.cost_samples)?;
        let unserved_sd = sample_stddev(&candidate.unserved_samples)?;
        let downtime_sd = sample_stddev(&candidate.downtime_samples)?;
        update_max_statistics(&candidate.cost_samples,candidate.cost_observed,cost_sd,&mut family_maxima)?;
        update_max_statistics(&candidate.unserved_samples,candidate.unserved_observed,unserved_sd,&mut family_maxima)?;
        update_max_statistics(&candidate.downtime_samples,candidate.downtime_observed,downtime_sd,&mut family_maxima)?;
    }
    family_maxima.sort_by(f64::total_cmp);
    let critical_value = percentile(&family_maxima,input.confidence_level)?;

    let mut comparisons = Vec::with_capacity(work.len());
    let failed_reference = reference_clusters.iter().try_fold(0_usize, |sum, cluster| {
        sum.checked_add(cluster.mission_count - cluster.completed_mission_count)
            .ok_or_else(|| AuditError("reference failed-mission count overflowed".to_string()))
    })?;
    for candidate in work {
        let cost_sd = sample_stddev(&candidate.cost_samples)?;
        let unserved_sd = sample_stddev(&candidate.unserved_samples)?;
        let downtime_sd = sample_stddev(&candidate.downtime_samples)?;
        comparisons.push(PairedBootstrapComparison {
            reference_candidate_id: input.reference_candidate_id.clone(),
            candidate_id: candidate.candidate_id,
            paired_cluster_count: reference_ids.len(),
            failed_mission_count_reference: failed_reference,
            failed_mission_count_candidate: candidate.failed_mission_count,
            cost_per_delivered_critical_kwh_delta: build_interval(
                &candidate.cost_samples,candidate.cost_observed,cost_sd,critical_value,input.confidence_level,
                input.minimum_practical_benefit.cost_per_delivered_critical_kwh_minor_units
            )?,
            unserved_critical_service_wh_delta: build_interval(
                &candidate.unserved_samples,candidate.unserved_observed,unserved_sd,critical_value,input.confidence_level,
                input.minimum_practical_benefit.unserved_critical_service_wh
            )?,
            system_unavailable_hours_delta: build_interval(
                &candidate.downtime_samples,candidate.downtime_observed,downtime_sd,critical_value,input.confidence_level,
                input.minimum_practical_benefit.system_unavailable_hours
            )?,
        });
    }
    comparisons.sort_by(|a,b|a.candidate_id.cmp(&b.candidate_id));
    Ok(PairedClusterBootstrapReport {
        reference_candidate_id:input.reference_candidate_id.clone(),
        scenario_manifest_sha256:input.scenario_manifest_sha256.clone(),
        preregistered_protocol_sha256:input.preregistered_protocol_sha256.clone(),
        confidence_level:input.confidence_level,
        familywise_confidence_level:input.confidence_level,
        hypothesis_family_size:family_size,
        bootstrap_iterations:input.bootstrap_iterations,
        bootstrap_algorithm:"SHARED_PAIRED_CLUSTER_BOOTSTRAP_SPLITMIX64_V0_2",
        simultaneous_interval_method:"MAX_ABS_STANDARDIZED_BOOTSTRAP_ERROR_V0_1",
        resampling_unit:"DECLARED_INDEPENDENT_MISSION_SITE_EVENT_CLUSTER",
        comparisons,
        claim_ceiling:CLAIM,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String { std::iter::repeat(ch).take(64).collect() }

    fn candidate(id: &str, delta: f64) -> PairedBootstrapCandidate {
        let clusters = (0..30).map(|n| {
            let unserved_wh = 10.0 + n as f64 + delta * 0.05;
            MissionClusterOutcome {
                cluster_id: format!("cluster-{n:02}"),
                scenario_seed_sha256: format!("{:064x}", 1000 + n),
                mission_count: 1,
                completed_mission_count: 1,
                total_cost_minor_units: (1000.0 + n as f64 + delta).max(1.0) as u128,
                required_critical_service_wh: 1000.0,
                delivered_critical_service_wh: 1000.0 - unserved_wh,
                unserved_critical_service_wh: unserved_wh,
                system_unavailable_hours: 2.0 + n as f64 * 0.1 + delta * 0.01,
                evidence_status: EvidenceStatus::Current,
                evidence_class: LifecycleEvidenceClass::SyntheticFixture,
                evidence_reference: format!("synthetic://cluster-{n:02}"),
            }
        }).collect();
        PairedBootstrapCandidate {
            candidate_id: id.to_string(),
            cluster_level: ClusterLevel::Mission,
            cluster_independence_evidence_reference: "synthetic://cluster-design".to_string(),
            scenario_manifest_sha256: digest('f'),
            clusters,
        }
    }

    fn input() -> PairedClusterBootstrapInput {
        PairedClusterBootstrapInput {
            reference_candidate_id: "reference".to_string(),
            scenario_manifest_sha256: digest('f'),
            bootstrap_seed_sha256: digest('a'),
            preregistered_protocol_sha256: digest('b'),
            minimum_practical_benefit: PracticalBenefitThresholds {
                cost_per_delivered_critical_kwh_minor_units: 1.0,
                unserved_critical_service_wh: 1.0,
                system_unavailable_hours: 0.01,
            },
            confidence_level: 0.95,
            bootstrap_iterations: 2_000,
            candidates: vec![candidate("reference", 0.0), candidate("candidate", -100.0)],
        }
    }

    #[test]
    fn paired_bootstrap_is_deterministic_and_reports_candidate_minus_reference() {
        let value = input();
        let first = evaluate_paired_cluster_bootstrap(&value).unwrap();
        let second = evaluate_paired_cluster_bootstrap(&value).unwrap();
        assert_eq!(first, second);
        let comparison = &first.comparisons[0];
        assert_eq!(comparison.paired_cluster_count, 30);
        assert!(comparison.cost_per_delivered_critical_kwh_delta.paired_mean_delta < 0.0);
        assert!(comparison.unserved_critical_service_wh_delta.paired_mean_delta < 0.0);
        assert!(comparison.system_unavailable_hours_delta.paired_mean_delta < 0.0);
    }

    #[test]
    fn failed_missions_remain_in_cluster_counts_and_unserved_service() {
        let mut value = input();
        value.candidates[1].clusters[0].completed_mission_count = 0;
        value.candidates[1].clusters[0].delivered_critical_service_wh = 500.0;
        value.candidates[1].clusters[0].unserved_critical_service_wh = 500.0;
        let report = evaluate_paired_cluster_bootstrap(&value).unwrap();
        assert_eq!(report.comparisons[0].failed_mission_count_candidate, 1);
    }

    #[test]
    fn rejects_seed_mismatch_even_when_cluster_ids_match() {
        let mut value = input();
        value.candidates[1].clusters[0].scenario_seed_sha256 = digest('b');
        assert!(evaluate_paired_cluster_bootstrap(&value).is_err());
    }

    #[test]
    fn rejects_cluster_set_mismatch() {
        let mut value = input();
        value.candidates[1].clusters.pop();
        assert!(evaluate_paired_cluster_bootstrap(&value).is_err());
    }

    #[test]
    fn rejects_too_few_clusters() {
        let mut value = input();
        for candidate in &mut value.candidates { candidate.clusters.pop(); }
        assert!(evaluate_paired_cluster_bootstrap(&value).is_err());
    }

    #[test]
    fn rejects_missing_cluster_independence_reference() {
        let mut value = input();
        value.candidates[1].cluster_independence_evidence_reference.clear();
        assert!(evaluate_paired_cluster_bootstrap(&value).is_err());
    }

    #[test]
    fn rejects_duplicate_cluster_ids() {
        let mut value = input();
        value.candidates[0].clusters[1].cluster_id =
            value.candidates[0].clusters[0].cluster_id.clone();
        assert!(evaluate_paired_cluster_bootstrap(&value).is_err());
    }

    #[test]
    fn rejects_zero_delivery_draws_instead_of_censoring_them() {
        let mut value = input();
        for cluster in &mut value.candidates[1].clusters {
            cluster.delivered_critical_service_wh = 0.0;
            cluster.unserved_critical_service_wh = 1000.0;
        }
        assert!(evaluate_paired_cluster_bootstrap(&value).is_err());
    }

    #[test]
    fn candidate_order_does_not_change_report() {
        let mut value = input();
        let first = evaluate_paired_cluster_bootstrap(&value).unwrap();
        value.candidates.reverse();
        let second = evaluate_paired_cluster_bootstrap(&value).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn rejects_invalid_iterations_or_confidence() {
        let mut value = input();
        value.bootstrap_iterations = 99;
        assert!(evaluate_paired_cluster_bootstrap(&value).is_err());
        value = input();
        value.confidence_level = 1.0;
        assert!(evaluate_paired_cluster_bootstrap(&value).is_err());
    }

    #[test]
    fn rejects_inconsistent_cluster_level() {
        let mut value = input();
        value.candidates[1].cluster_level = ClusterLevel::SiteEventMission;
        assert!(evaluate_paired_cluster_bootstrap(&value).is_err());
    }

    #[test]
    fn rejects_cluster_service_arithmetic_that_does_not_reconcile() {
        let mut value = input();
        value.candidates[1].clusters[0].unserved_critical_service_wh = 25.0;
        assert!(evaluate_paired_cluster_bootstrap(&value).is_err());
    }

    #[test]
    fn rejects_per_cluster_required_service_mismatch() {
        let mut value = input();
        value.candidates[1].clusters[0].required_critical_service_wh = 900.0;
        value.candidates[1].clusters[0].delivered_critical_service_wh = 900.0;
        assert!(evaluate_paired_cluster_bootstrap(&value).is_err());
    }

    #[test]
    fn rejects_assumption_only_cluster_evidence() {
        let mut value = input();
        value.candidates[1].clusters[0].evidence_class = LifecycleEvidenceClass::Assumption;
        assert!(evaluate_paired_cluster_bootstrap(&value).is_err());
    }


    #[test]
    fn report_uses_familywise_max_statistic_across_all_endpoints() {
        let report = evaluate_paired_cluster_bootstrap(&input()).unwrap();
        assert_eq!(report.hypothesis_family_size, 3);
        assert_eq!(report.familywise_confidence_level, 0.95);
        assert_eq!(report.simultaneous_interval_method, "MAX_ABS_STANDARDIZED_BOOTSTRAP_ERROR_V0_1");
        let comparison = &report.comparisons[0];
        assert!(comparison.cost_per_delivered_critical_kwh_delta.familywise_lower_bound
            <= comparison.cost_per_delivered_critical_kwh_delta.paired_mean_delta);
        assert!(comparison.cost_per_delivered_critical_kwh_delta.familywise_upper_bound
            >= comparison.cost_per_delivered_critical_kwh_delta.paired_mean_delta);
    }

    #[test]
    fn practical_superiority_requires_simultaneous_bound_to_clear_margin() {
        let report = evaluate_paired_cluster_bootstrap(&input()).unwrap();
        assert_eq!(
            report.comparisons[0].cost_per_delivered_critical_kwh_delta.finding,
            ComparisonFinding::PracticalSuperioritySupported
        );
    }

    #[test]
    fn protocol_digest_and_positive_practical_thresholds_are_required() {
        let mut value = input();
        value.preregistered_protocol_sha256 = "not-a-digest".into();
        assert!(evaluate_paired_cluster_bootstrap(&value).is_err());
        value = input();
        value.minimum_practical_benefit.unserved_critical_service_wh = 0.0;
        assert!(evaluate_paired_cluster_bootstrap(&value).is_err());
    }

    #[test]
    fn shared_family_resamples_are_invariant_to_candidate_input_order() {
        let mut value = input();
        value.candidates.push(candidate("candidate-2", -80.0));
        let first = evaluate_paired_cluster_bootstrap(&value).unwrap();
        value.candidates.reverse();
        let second = evaluate_paired_cluster_bootstrap(&value).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.hypothesis_family_size, 6);
    }

}
