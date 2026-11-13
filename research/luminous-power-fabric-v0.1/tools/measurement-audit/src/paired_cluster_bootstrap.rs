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
pub struct PairedClusterBootstrapInput {
    pub reference_candidate_id: String,
    pub scenario_manifest_sha256: String,
    pub bootstrap_seed_sha256: String,
    pub confidence_level: f64,
    pub bootstrap_iterations: usize,
    pub candidates: Vec<PairedBootstrapCandidate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BootstrapInterval {
    /// Candidate minus reference. Negative is favorable for these metrics.
    pub paired_mean_delta: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
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
    pub confidence_level: f64,
    pub bootstrap_iterations: usize,
    pub bootstrap_algorithm: &'static str,
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

fn stable_name_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
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

fn interval(
    samples: &[f64],
    observed_mean: f64,
    iterations: usize,
    confidence: f64,
) -> Result<BootstrapInterval, AuditError> {
    if samples.len() != iterations || samples.iter().any(|value| !value.is_finite()) {
        return Err(AuditError("bootstrap samples are invalid".to_string()));
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let alpha = 1.0 - confidence;
    Ok(BootstrapInterval {
        paired_mean_delta: observed_mean,
        lower_bound: percentile(&sorted, alpha * 0.5)?,
        upper_bound: percentile(&sorted, 1.0 - alpha * 0.5)?,
    })
}

/// Paired percentile bootstrap over pre-aggregated mission/site/event clusters.
/// Failed missions remain in each cluster's counts and unserved-service totals;
/// they are never removed from the paired sample. Cluster independence is a
/// data-owner assertion and is not proven by this evaluator.
pub fn evaluate_paired_cluster_bootstrap(
    input: &PairedClusterBootstrapInput,
) -> Result<PairedClusterBootstrapReport, AuditError> {
    if !nonblank(&input.reference_candidate_id)
        || !valid_sha256(&input.scenario_manifest_sha256)
        || !valid_sha256(&input.bootstrap_seed_sha256)
        || !input.confidence_level.is_finite()
        || input.confidence_level <= 0.5
        || input.confidence_level >= 1.0
        || input.bootstrap_iterations < MIN_ITERATIONS
        || input.bootstrap_iterations > MAX_ITERATIONS
        || input.candidates.len() < 2
        || input.candidates.len() > 10_000
    {
        return Err(AuditError("paired bootstrap comparison metadata is invalid".to_string()));
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

    for (candidate_id, clusters) in &validated {
        if clusters.len() != reference.len() || clusters.keys().ne(reference.keys()) {
            return Err(AuditError(format!(
                "candidate {candidate_id:?} does not contain the exact reference cluster-ID set"
            )));
        }
        for (cluster_id, reference_cluster) in reference.iter() {
            let candidate_cluster = clusters.get(cluster_id)
                .ok_or_else(|| AuditError("paired cluster lookup failed".to_string()))?;
            if candidate_cluster.scenario_seed_sha256 != reference_cluster.scenario_seed_sha256 {
                return Err(AuditError(format!(
                    "scenario seed mismatch for cluster {cluster_id:?}"
                )));
            }
        }
    }

    let base_seed = seed_from_digest(&input.bootstrap_seed_sha256)?;
    let mut comparisons = Vec::new();

    for (candidate_id, clusters) in &validated {
        if *candidate_id == input.reference_candidate_id.as_str() { continue; }

        let reference_clusters: Vec<&MissionClusterOutcome> = reference_ids.iter()
            .map(|id| reference.get(id).copied().ok_or_else(|| AuditError("reference cluster lookup failed".to_string())))
            .collect::<Result<_, _>>()?;
        let candidate_clusters: Vec<&MissionClusterOutcome> = reference_ids.iter()
            .map(|id| clusters.get(id).copied().ok_or_else(|| AuditError("candidate cluster lookup failed".to_string())))
            .collect::<Result<_, _>>()?;

        let full_indices: Vec<usize> = (0..reference_ids.len()).collect();
        let reference_full = aggregate(&reference_clusters, &full_indices)?;
        let candidate_full = aggregate(&candidate_clusters, &full_indices)?;

        let full_cost_delta =
            cost_per_delivered_kwh(candidate_full)? - cost_per_delivered_kwh(reference_full)?;
        let full_unserved_delta =
            candidate_full.unserved_wh_mean - reference_full.unserved_wh_mean;
        let full_downtime_delta =
            candidate_full.unavailable_hours_mean - reference_full.unavailable_hours_mean;

        let failed_reference = reference_clusters.iter().try_fold(0_usize, |sum, cluster| {
            sum.checked_add(cluster.mission_count - cluster.completed_mission_count)
                .ok_or_else(|| AuditError("reference failed-mission count overflowed".to_string()))
        })?;
        let failed_candidate = candidate_clusters.iter().try_fold(0_usize, |sum, cluster| {
            sum.checked_add(cluster.mission_count - cluster.completed_mission_count)
                .ok_or_else(|| AuditError("candidate failed-mission count overflowed".to_string()))
        })?;

        let candidate_seed = base_seed ^ stable_name_hash(candidate_id);
        let mut rng = SplitMix64::new(candidate_seed);
        let mut cost_samples = Vec::with_capacity(input.bootstrap_iterations);
        let mut unserved_samples = Vec::with_capacity(input.bootstrap_iterations);
        let mut downtime_samples = Vec::with_capacity(input.bootstrap_iterations);

        for _ in 0..input.bootstrap_iterations {
            let indices: Vec<usize> =
                (0..reference_ids.len()).map(|_| rng.index(reference_ids.len())).collect();
            let reference_sample = aggregate(&reference_clusters, &indices)?;
            let candidate_sample = aggregate(&candidate_clusters, &indices)?;

            cost_samples.push(
                cost_per_delivered_kwh(candidate_sample)? - cost_per_delivered_kwh(reference_sample)?
            );
            unserved_samples.push(
                candidate_sample.unserved_wh_mean - reference_sample.unserved_wh_mean
            );
            downtime_samples.push(
                candidate_sample.unavailable_hours_mean - reference_sample.unavailable_hours_mean
            );
        }

        comparisons.push(PairedBootstrapComparison {
            reference_candidate_id: input.reference_candidate_id.clone(),
            candidate_id: (*candidate_id).to_string(),
            paired_cluster_count: reference_ids.len(),
            failed_mission_count_reference: failed_reference,
            failed_mission_count_candidate: failed_candidate,
            cost_per_delivered_critical_kwh_delta: interval(
                &cost_samples, full_cost_delta, input.bootstrap_iterations,
                input.confidence_level
            )?,
            unserved_critical_service_wh_delta: interval(
                &unserved_samples, full_unserved_delta, input.bootstrap_iterations,
                input.confidence_level
            )?,
            system_unavailable_hours_delta: interval(
                &downtime_samples, full_downtime_delta, input.bootstrap_iterations,
                input.confidence_level
            )?,
        });
    }

    comparisons.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));
    Ok(PairedClusterBootstrapReport {
        reference_candidate_id: input.reference_candidate_id.clone(),
        scenario_manifest_sha256: input.scenario_manifest_sha256.clone(),
        confidence_level: input.confidence_level,
        bootstrap_iterations: input.bootstrap_iterations,
        bootstrap_algorithm: "PAIRED_CLUSTER_PERCENTILE_BOOTSTRAP_SPLITMIX64_V0_1",
        resampling_unit: "DECLARED_INDEPENDENT_MISSION_SITE_EVENT_CLUSTER",
        comparisons,
        claim_ceiling: CLAIM,
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
                scenario_seed_sha256: digest(char::from(b'a' + (n % 16) as u8)),
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

}
