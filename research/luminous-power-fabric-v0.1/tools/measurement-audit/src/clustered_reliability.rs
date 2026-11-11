use crate::grid_service::{wilson_lower_bound, GridServiceQualificationReport};
use crate::AuditError;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Frozen analysis contract for dependence-aware repeated-event qualification.
///
/// Each cluster is treated as one independent sampling unit. Events inside a
/// cluster may be arbitrarily dependent: the cluster succeeds only if every
/// event succeeds, and its capacity is the minimum capacity observed in that
/// cluster. Strata are evaluated separately to prevent pooled results from
/// hiding a weak season or operating regime.
#[derive(Debug, Clone, PartialEq)]
pub struct ClusteredReliabilityContract {
    pub analysis_id: String,
    pub source_service_contract_id: String,
    pub source_event_sampling_design_id: String,
    pub source_report_evidence_id: String,
    pub cluster_sampling_design_id: String,
    pub cluster_assignment_evidence_id: String,
    /// The review record must justify the cluster unit, separation and
    /// cross-cluster independence. A reference is evidence lineage, not proof.
    pub independence_review_evidence_id: Option<String>,
    pub independent_cluster_assumption: bool,
    /// Every declared stratum must have enough independent clusters on its own.
    pub required_strata: Vec<String>,
    /// Lower-tail capacity quantile, normally 0.10 for P10.
    pub target_lower_quantile: f64,
    /// One-sided confidence required for the distribution-free quantile bound.
    pub one_sided_confidence: f64,
    /// Explicit normal-score z used for the cluster-success Wilson bound.
    /// Kept separate from confidence so the source event contract cannot
    /// silently override the cluster analysis's declared confidence level.
    pub success_confidence_z_score: f64,
    /// All clusters in this analysis use the same observation-window duration,
    /// so cluster minima have comparable exposure.
    pub cluster_window_duration_ns: u64,
    /// Exact dispatch count per cluster. Exact equality keeps cluster minima
    /// comparable when events within each cluster may be dependent.
    pub events_per_cluster: usize,
    /// Minimum gap between cluster windows. Derive it from the sampling design;
    /// non-overlap alone does not prove statistical independence.
    pub minimum_cluster_separation_ns: u64,
}

impl ClusteredReliabilityContract {
    fn validate(&self) -> Result<(), AuditError> {
        for (value, field) in [
            (self.analysis_id.as_str(), "analysis_id"),
            (
                self.source_service_contract_id.as_str(),
                "source_service_contract_id",
            ),
            (
                self.source_event_sampling_design_id.as_str(),
                "source_event_sampling_design_id",
            ),
            (
                self.source_report_evidence_id.as_str(),
                "source_report_evidence_id",
            ),
            (
                self.cluster_sampling_design_id.as_str(),
                "cluster_sampling_design_id",
            ),
            (
                self.cluster_assignment_evidence_id.as_str(),
                "cluster_assignment_evidence_id",
            ),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!(
                    "clustered reliability contract {field} must not be blank"
                )));
            }
        }

        if self.required_strata.is_empty() {
            return Err(AuditError(
                "at least one required operating stratum must be declared".to_string(),
            ));
        }
        let mut strata = HashSet::new();
        for stratum in &self.required_strata {
            if stratum.trim().is_empty() || !strata.insert(stratum.as_str()) {
                return Err(AuditError(
                    "required strata must be non-blank and unique".to_string(),
                ));
            }
        }
        if !self.target_lower_quantile.is_finite()
            || self.target_lower_quantile <= 0.0
            || self.target_lower_quantile >= 1.0
            || !self.one_sided_confidence.is_finite()
            || self.one_sided_confidence <= 0.5
            || self.one_sided_confidence >= 1.0
        {
            return Err(AuditError(
                "quantile must be in (0, 1) and one-sided confidence must be in (0.5, 1)"
                    .to_string(),
            ));
        }

        if !self.success_confidence_z_score.is_finite()
            || self.success_confidence_z_score <= 0.0
            || !(self.success_confidence_z_score * self.success_confidence_z_score).is_finite()
        {
            return Err(AuditError(
                "cluster success Wilson z-score must be finite, positive and square safely"
                    .to_string(),
            ));
        }
        if self.cluster_window_duration_ns == 0 || self.events_per_cluster == 0 {
            return Err(AuditError(
                "cluster window duration and exact events per cluster must be positive"
                    .to_string(),
            ));
        }

        match (
            self.independent_cluster_assumption,
            self.independence_review_evidence_id.as_deref(),
        ) {
            (true, Some(value)) if !value.trim().is_empty() => {
                if self.minimum_cluster_separation_ns == 0 {
                    return Err(AuditError(
                        "independent clusters require a positive minimum separation".to_string(),
                    ));
                }
            }
            (true, _) => {
                return Err(AuditError(
                    "independent clusters require a non-blank independence review reference"
                        .to_string(),
                ));
            }
            (false, Some(value)) if value.trim().is_empty() => {
                return Err(AuditError(
                    "independence review reference must be absent or non-blank".to_string(),
                ));
            }
            (false, _) => {}
        }

        minimum_clusters_for_quantile_confidence(
            self.target_lower_quantile,
            self.one_sided_confidence,
        )
        .ok_or_else(|| {
            AuditError(
                "quantile/confidence combination does not produce a finite cluster requirement"
                    .to_string(),
            )
        })?;

        Ok(())
    }
}

/// A predeclared independent sampling block, e.g. a site-day or a separately
/// randomized operating episode. Its window must contain every assigned event
/// and its complete recovery observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndependenceCluster {
    pub cluster_id: String,
    pub stratum_id: String,
    pub window_start_ns: u64,
    pub window_end_ns: u64,
    pub design_evidence_id: String,
}

/// Exactly one assignment is required for every event in the source report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventClusterAssignment {
    pub event_id: String,
    pub cluster_id: String,
    pub assignment_evidence_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndependenceClusterOutcome {
    pub cluster_id: String,
    pub stratum_id: String,
    pub window_start_ns: u64,
    pub window_end_ns: u64,
    pub event_count: usize,
    pub successful_event_count: usize,
    /// True only when every event in the cluster met the complete commitment.
    pub all_events_successful: bool,
    /// Worst event-level credited capacity in the cluster.
    pub minimum_credited_capacity_w: f64,
    pub event_ids: Vec<String>,
    pub event_evidence_ids: Vec<String>,
    pub assignment_evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClusterStratumSummary {
    pub stratum_id: String,
    pub independent_cluster_count: usize,
    pub event_count: usize,
    pub successful_cluster_count: usize,
    /// A cluster succeeds only when every event in it succeeds.
    pub all_event_successful_cluster_fraction: f64,
    /// Wilson lower bound over independent clusters, not individual events.
    pub wilson_lower_bound_cluster_success_fraction: Option<f64>,
    /// Empirical nearest-rank quantile of cluster-minimum capacities.
    pub observed_lower_quantile_cluster_capacity_w: Option<f64>,
    /// Distribution-free one-sided lower confidence bound for the population
    /// lower quantile of cluster-minimum capacity. This is the sample minimum
    /// when the required independent-cluster support is available.
    pub lower_quantile_capacity_lower_confidence_bound_w: Option<f64>,
    pub minimum_clusters_for_quantile_confidence: usize,
    pub quantile_support_sufficient: bool,
    pub meets_capacity_commitment: bool,
    pub meets_cluster_success_reliability: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClusteredReliabilityReport {
    pub contract: ClusteredReliabilityContract,
    pub source_service_contract_id: String,
    pub source_report_evidence_id: String,
    pub total_event_count: usize,
    pub total_cluster_count: usize,
    pub required_strata: Vec<String>,
    pub minimum_clusters_per_stratum: usize,
    pub independent_cluster_assumption_available: bool,
    pub cluster_outcomes: Vec<IndependenceClusterOutcome>,
    pub strata: Vec<ClusterStratumSummary>,
    /// Worst supported lower-confidence capacity across all required strata.
    pub overall_lower_quantile_capacity_lower_confidence_bound_w: Option<f64>,
    /// Worst one-sided Wilson lower bound across all required strata.
    pub overall_cluster_success_probability_lower_bound: Option<f64>,
    pub meets_minimum_event_count: bool,
    pub meets_capacity_commitment: bool,
    pub meets_cluster_success_reliability: bool,
    pub all_required_strata_supported: bool,
    pub commitment_qualified: bool,
    pub claim_ceiling: &'static str,
}

/// Minimum independent clusters needed so the sample minimum is a
/// distribution-free one-sided lower confidence bound for population quantile
/// q with confidence gamma:
///
///     P(min(X_1..X_n) <= Q_q) >= 1 - (1 - q)^n.
///
/// This deliberately avoids a parametric tail model or a bootstrap block-size
/// choice for the qualification bound. Independence applies between clusters;
/// no independence is assumed between events inside a cluster.
pub fn minimum_clusters_for_quantile_confidence(
    quantile: f64,
    one_sided_confidence: f64,
) -> Option<usize> {
    if !quantile.is_finite()
        || quantile <= 0.0
        || quantile >= 1.0
        || !one_sided_confidence.is_finite()
        || one_sided_confidence <= 0.0
        || one_sided_confidence >= 1.0
    {
        return None;
    }

    let numerator = (1.0 - one_sided_confidence).ln();
    let denominator = (1.0 - quantile).ln();
    let required = (numerator / denominator).ceil();
    if !required.is_finite() || required < 1.0 || required > usize::MAX as f64 {
        return None;
    }
    Some(required as usize)
}

fn nearest_rank_quantile(values: &[f64], quantile: f64) -> Option<f64> {
    if values.is_empty()
        || !quantile.is_finite()
        || quantile <= 0.0
        || quantile > 1.0
        || values.iter().any(|value| !value.is_finite())
    {
        return None;
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let rank = (quantile * ordered.len() as f64).ceil() as usize;
    ordered.get(rank.saturating_sub(1)).copied()
}

fn validate_source_report(
    source: &GridServiceQualificationReport,
    contract: &ClusteredReliabilityContract,
) -> Result<(), AuditError> {
    if source.contract.service_contract_id != contract.source_service_contract_id
        || source.contract.event_sampling_design_id != contract.source_event_sampling_design_id
    {
        return Err(AuditError(
            "clustered analysis does not match the source service contract or event sampling design"
                .to_string(),
        ));
    }
    if source.total_event_count == 0 || source.total_event_count != source.event_results.len() {
        return Err(AuditError(
            "source grid-service report event count does not match its event results".to_string(),
        ));
    }
    if !source.committed_compute_reduction_w.is_finite()
        || source.committed_compute_reduction_w <= 0.0
        || source.committed_compute_reduction_w != source.contract.committed_compute_reduction_w
    {
        return Err(AuditError(
            "source grid-service report has an invalid or inconsistent commitment".to_string(),
        ));
    }

    let mut event_ids = HashSet::new();
    for event in &source.event_results {
        if event.event_id.trim().is_empty()
            || event.evidence_id.trim().is_empty()
            || !event_ids.insert(event.event_id.as_str())
        {
            return Err(AuditError(
                "source report contains a blank or duplicate event/evidence identity".to_string(),
            ));
        }
        if !event.credited_deliverable_capacity_w.is_finite()
            || event.credited_deliverable_capacity_w < 0.0
            || event.event_success
                != (event.service_contract_pass && event.commitment_power_pass)
            || event.commitment_power_pass
                != (event.credited_deliverable_capacity_w
                    >= source.committed_compute_reduction_w)
            || (!event.service_contract_pass && event.credited_deliverable_capacity_w != 0.0)
        {
            return Err(AuditError(format!(
                "source event {:?} has internally inconsistent service, commitment or credited-capacity fields",
                event.event_id
            )));
        }
        if event.window_end_ns <= event.window_start_ns
            || event.recovery_window_start_ns != event.window_end_ns
            || event.recovery_window_end_ns <= event.recovery_window_start_ns
        {
            return Err(AuditError(format!(
                "source event {:?} has invalid dispatch/recovery chronology",
                event.event_id
            )));
        }
    }

    // The parent evaluator normally guarantees this invariant. Re-check it
    // because this public API accepts a report value supplied by a caller.
    let mut ordered_events: Vec<_> = source.event_results.iter().collect();
    ordered_events.sort_by_key(|event| event.window_start_ns);
    let mut previous_recovery_end_ns = None;
    for event in ordered_events {
        if previous_recovery_end_ns
            .is_some_and(|end| event.window_start_ns < end)
        {
            return Err(AuditError(
                "source dispatch and recovery intervals overlap".to_string(),
            ));
        }
        previous_recovery_end_ns = Some(event.recovery_window_end_ns);
    }
    Ok(())
}

/// Re-evaluate a source report using independent operating clusters.
///
/// The source report remains authoritative for event-level attribution and
/// service gates. This layer changes only the statistical unit: events are
/// grouped into predeclared clusters, and each required stratum is qualified
/// independently. It never upgrades an unmeasured or unverified source report.
pub fn evaluate_clustered_reliability(
    source: &GridServiceQualificationReport,
    contract: ClusteredReliabilityContract,
    clusters: Vec<IndependenceCluster>,
    assignments: Vec<EventClusterAssignment>,
) -> Result<ClusteredReliabilityReport, AuditError> {
    contract.validate()?;
    validate_source_report(source, &contract)?;

    if clusters.is_empty() || clusters.len() > source.event_results.len() {
        return Err(AuditError(
            "cluster definitions must be non-empty and cannot outnumber source events".to_string(),
        ));
    }
    if assignments.len() != source.event_results.len() {
        return Err(AuditError(
            "every source event must have exactly one cluster assignment".to_string(),
        ));
    }

    let required_strata: HashSet<&str> =
        contract.required_strata.iter().map(String::as_str).collect();
    let mut cluster_by_id = BTreeMap::new();
    for cluster in clusters {
        for (value, field) in [
            (cluster.cluster_id.as_str(), "cluster_id"),
            (cluster.stratum_id.as_str(), "stratum_id"),
            (cluster.design_evidence_id.as_str(), "design_evidence_id"),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!(
                    "independence cluster {field} must not be blank"
                )));
            }
        }
        if !required_strata.contains(cluster.stratum_id.as_str()) {
            return Err(AuditError(format!(
                "cluster {:?} uses undeclared stratum {:?}",
                cluster.cluster_id, cluster.stratum_id
            )));
        }
        if cluster.window_end_ns.checked_sub(cluster.window_start_ns)
            != Some(contract.cluster_window_duration_ns)
        {
            return Err(AuditError(format!(
                "cluster {:?} window duration differs from the frozen cluster exposure",
                cluster.cluster_id
            )));
        }
        let id = cluster.cluster_id.clone();
        if cluster_by_id.insert(id.clone(), cluster).is_some() {
            return Err(AuditError(format!(
                "duplicate independence cluster id {id:?}"
            )));
        }
    }

    let mut ordered_clusters: Vec<&IndependenceCluster> = cluster_by_id.values().collect();
    ordered_clusters.sort_by_key(|cluster| cluster.window_start_ns);
    let separation_ns = if contract.independent_cluster_assumption {
        contract.minimum_cluster_separation_ns
    } else {
        0
    };
    let mut previous_cluster_end = None;
    for cluster in &ordered_clusters {
        if let Some(previous_end) = previous_cluster_end {
            let earliest_start = previous_end.checked_add(separation_ns).ok_or_else(|| {
                AuditError("cluster separation arithmetic overflowed".to_string())
            })?;
            if cluster.window_start_ns < earliest_start {
                return Err(AuditError(
                    "independence cluster windows overlap or violate the declared separation"
                        .to_string(),
                ));
            }
        }
        previous_cluster_end = Some(cluster.window_end_ns);
    }

    let event_by_id: HashMap<&str, &crate::grid_service::GridDispatchEventResult> = source
        .event_results
        .iter()
        .map(|event| (event.event_id.as_str(), event))
        .collect();
    let mut assigned_event_ids = HashSet::new();
    let mut assignments_by_cluster: BTreeMap<String, Vec<EventClusterAssignment>> =
        BTreeMap::new();
    for assignment in assignments {
        if assignment.event_id.trim().is_empty()
            || assignment.cluster_id.trim().is_empty()
            || assignment.assignment_evidence_id.trim().is_empty()
        {
            return Err(AuditError(
                "event cluster assignment requires event, cluster and evidence identities"
                    .to_string(),
            ));
        }
        if !event_by_id.contains_key(assignment.event_id.as_str()) {
            return Err(AuditError(format!(
                "cluster assignment references unknown event {:?}",
                assignment.event_id
            )));
        }
        if !cluster_by_id.contains_key(&assignment.cluster_id) {
            return Err(AuditError(format!(
                "event {:?} references unknown cluster {:?}",
                assignment.event_id, assignment.cluster_id
            )));
        }
        if !assigned_event_ids.insert(assignment.event_id.clone()) {
            return Err(AuditError(format!(
                "event {:?} has more than one cluster assignment",
                assignment.event_id
            )));
        }
        assignments_by_cluster
            .entry(assignment.cluster_id.clone())
            .or_default()
            .push(assignment);
    }
    if assigned_event_ids.len() != event_by_id.len() {
        return Err(AuditError(
            "cluster assignments do not cover every source event exactly once".to_string(),
        ));
    }

    let mut cluster_outcomes = Vec::with_capacity(cluster_by_id.len());
    for cluster in &ordered_clusters {
        let mut cluster_assignments = assignments_by_cluster
            .remove(&cluster.cluster_id)
            .ok_or_else(|| {
                AuditError(format!(
                    "independence cluster {:?} contains no assigned events",
                    cluster.cluster_id
                ))
            })?;
        cluster_assignments.sort_by(|left, right| left.event_id.cmp(&right.event_id));

        let mut events = Vec::with_capacity(cluster_assignments.len());
        for assignment in &cluster_assignments {
            let event = event_by_id
                .get(assignment.event_id.as_str())
                .copied()
                .ok_or_else(|| AuditError("assigned source event disappeared".to_string()))?;
            if event.window_start_ns < cluster.window_start_ns
                || event.recovery_window_end_ns > cluster.window_end_ns
            {
                return Err(AuditError(format!(
                    "event {:?} dispatch and recovery evidence are not fully contained in cluster {:?}",
                    event.event_id, cluster.cluster_id
                )));
            }
            events.push(event);
        }

        if events.len() != contract.events_per_cluster {
            return Err(AuditError(format!(
                "cluster {:?} has {} events; exact frozen count is {}",
                cluster.cluster_id,
                events.len(),
                contract.events_per_cluster
            )));
        }

        let successful_event_count = events.iter().filter(|event| event.event_success).count();
        let minimum_credited_capacity_w = events
            .iter()
            .map(|event| event.credited_deliverable_capacity_w)
            .fold(f64::INFINITY, f64::min);
        if !minimum_credited_capacity_w.is_finite() {
            return Err(AuditError(format!(
                "cluster {:?} has no finite event capacity",
                cluster.cluster_id
            )));
        }

        cluster_outcomes.push(IndependenceClusterOutcome {
            cluster_id: cluster.cluster_id.clone(),
            stratum_id: cluster.stratum_id.clone(),
            window_start_ns: cluster.window_start_ns,
            window_end_ns: cluster.window_end_ns,
            event_count: events.len(),
            successful_event_count,
            all_events_successful: successful_event_count == events.len(),
            minimum_credited_capacity_w,
            event_ids: events.iter().map(|event| event.event_id.clone()).collect(),
            event_evidence_ids: events.iter().map(|event| event.evidence_id.clone()).collect(),
            assignment_evidence_ids: cluster_assignments
                .iter()
                .map(|assignment| assignment.assignment_evidence_id.clone())
                .collect(),
        });
    }

    if !assignments_by_cluster.is_empty() {
        return Err(AuditError(
            "cluster assignment references a cluster without a matching definition".to_string(),
        ));
    }

    let minimum_clusters = minimum_clusters_for_quantile_confidence(
        contract.target_lower_quantile,
        contract.one_sided_confidence,
    )
    .ok_or_else(|| AuditError("cannot derive the quantile cluster requirement".to_string()))?;

    let mut outcomes_by_stratum: BTreeMap<String, Vec<&IndependenceClusterOutcome>> =
        BTreeMap::new();
    for outcome in &cluster_outcomes {
        outcomes_by_stratum
            .entry(outcome.stratum_id.clone())
            .or_default()
            .push(outcome);
    }

    let mut strata = Vec::with_capacity(contract.required_strata.len());
    for stratum_id in &contract.required_strata {
        let outcomes = outcomes_by_stratum.get(stratum_id).ok_or_else(|| {
            AuditError(format!(
                "required stratum {stratum_id:?} has no independence clusters"
            ))
        })?;
        let cluster_count = outcomes.len();
        let event_count = outcomes.iter().map(|outcome| outcome.event_count).sum();
        let successful_cluster_count = outcomes
            .iter()
            .filter(|outcome| outcome.all_events_successful)
            .count();
        let all_event_successful_cluster_fraction =
            successful_cluster_count as f64 / cluster_count as f64;
        let mut cluster_capacities: Vec<f64> = outcomes
            .iter()
            .map(|outcome| outcome.minimum_credited_capacity_w)
            .collect();

        let wilson_lower_bound_cluster_success_fraction =
            if contract.independent_cluster_assumption {
                wilson_lower_bound(
                    successful_cluster_count,
                    cluster_count,
                    contract.success_confidence_z_score,
                )
            } else {
                None
            };
        let quantile_support_sufficient =
            contract.independent_cluster_assumption && cluster_count >= minimum_clusters;
        let lower_quantile_capacity_lower_confidence_bound_w =
            if quantile_support_sufficient {
                cluster_capacities.iter().copied().reduce(f64::min)
            } else {
                None
            };
        let observed_lower_quantile_cluster_capacity_w =
            nearest_rank_quantile(&cluster_capacities, contract.target_lower_quantile);
        let meets_capacity_commitment =
            lower_quantile_capacity_lower_confidence_bound_w.is_some_and(|capacity| {
                capacity >= source.committed_compute_reduction_w
            });
        let meets_cluster_success_reliability =
            wilson_lower_bound_cluster_success_fraction.is_some_and(|lower_bound| {
                lower_bound >= source.contract.minimum_success_probability
            });

        strata.push(ClusterStratumSummary {
            stratum_id: stratum_id.clone(),
            independent_cluster_count: cluster_count,
            event_count,
            successful_cluster_count,
            all_event_successful_cluster_fraction,
            wilson_lower_bound_cluster_success_fraction,
            observed_lower_quantile_cluster_capacity_w,
            lower_quantile_capacity_lower_confidence_bound_w,
            minimum_clusters_for_quantile_confidence: minimum_clusters,
            quantile_support_sufficient,
            meets_capacity_commitment,
            meets_cluster_success_reliability,
        });
    }

    let all_required_strata_supported = strata.iter().all(|summary| {
        summary.quantile_support_sufficient
            && summary.wilson_lower_bound_cluster_success_fraction.is_some()
    });
    let overall_lower_quantile_capacity_lower_confidence_bound_w =
        if all_required_strata_supported {
            strata
                .iter()
                .filter_map(|summary| summary.lower_quantile_capacity_lower_confidence_bound_w)
                .reduce(f64::min)
        } else {
            None
        };
    let overall_cluster_success_probability_lower_bound =
        if all_required_strata_supported {
            strata
                .iter()
                .filter_map(|summary| summary.wilson_lower_bound_cluster_success_fraction)
                .reduce(f64::min)
        } else {
            None
        };

    let meets_minimum_event_count =
        source.total_event_count >= source.contract.minimum_event_count;
    let meets_capacity_commitment = all_required_strata_supported
        && strata.iter().all(|summary| summary.meets_capacity_commitment);
    let meets_cluster_success_reliability = all_required_strata_supported
        && strata
            .iter()
            .all(|summary| summary.meets_cluster_success_reliability);
    let commitment_qualified = contract.independent_cluster_assumption
        && meets_minimum_event_count
        && meets_capacity_commitment
        && meets_cluster_success_reliability;

    Ok(ClusteredReliabilityReport {
        contract: contract.clone(),
        source_service_contract_id: source.contract.service_contract_id.clone(),
        source_report_evidence_id: contract.source_report_evidence_id,
        total_event_count: source.total_event_count,
        total_cluster_count: cluster_outcomes.len(),
        required_strata: contract.required_strata,
        minimum_clusters_per_stratum: minimum_clusters,
        independent_cluster_assumption_available: contract.independent_cluster_assumption,
        cluster_outcomes,
        strata,
        overall_lower_quantile_capacity_lower_confidence_bound_w,
        overall_cluster_success_probability_lower_bound,
        meets_minimum_event_count,
        meets_capacity_commitment,
        meets_cluster_success_reliability,
        all_required_strata_supported,
        commitment_qualified,
        claim_ceiling: "CLUSTERED_REPEATED_EVENT_OFFLINE_EVALUATION_NOT_MARKET_ACCREDITATION",
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_clustered_reliability, minimum_clusters_for_quantile_confidence,
        ClusteredReliabilityContract, EventClusterAssignment, IndependenceCluster,
    };
    use crate::grid_service::{
        CounterfactualBaselineMethod, GridDispatchEventResult, GridServiceClass,
        GridServiceContract, GridServiceQualificationReport,
    };

    fn source_contract() -> GridServiceContract {
        GridServiceContract {
            service_contract_id: "service-contract-v1".to_string(),
            resource_id: "site-01".to_string(),
            grid_zone_id: "zone-01".to_string(),
            point_of_interconnection_id: "poi-01".to_string(),
            site_operating_envelope_id: "summer-and-winter-v1".to_string(),
            workload_contract_id: "workload-v1".to_string(),
            request_mix_id: "request-mix-v1".to_string(),
            quality_contract_id: "quality-v1".to_string(),
            boundary_id: "pcc-01".to_string(),
            control_action_id: "defer-batch-v1".to_string(),
            counterfactual_baseline_id: "baseline-v1".to_string(),
            counterfactual_baseline_evidence_id: "baseline-evidence-v1".to_string(),
            baseline_validation_evidence_id: None,
            baseline_method: CounterfactualBaselineMethod::FrozenWorkloadReplay,
            baseline_preregistered: true,
            event_sampling_design_id: "event-design-v1".to_string(),
            independent_event_assumption: false,
            service_class: GridServiceClass::GridServing,
            committed_compute_reduction_w: 150.0,
            minimum_event_count: 30,
            minimum_success_probability: 0.90,
            confidence_z_score: 1.281_551_565_5,
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

    fn event_result(index: usize, cluster_index: usize, success: bool) -> GridDispatchEventResult {
        let cluster_start = cluster_index as u64 * 200_000_000_000;
        let window_start = cluster_start + 1_000_000_000 + (index as u64 % 10) * 2_000_000_000;
        let window_end = window_start + 100_000_000;
        GridDispatchEventResult {
            event_id: format!("event-{index:03}"),
            evidence_id: format!("event-evidence-{index:03}"),
            window_start_ns: window_start,
            window_end_ns: window_end,
            recovery_window_start_ns: window_end,
            recovery_window_end_ns: window_end + 400_000_000,
            site_power_reduction_w: 220.0,
            non_compute_load_reduction_w: 0.0,
            compute_attributed_reduction_before_uncertainty_w: 220.0,
            combined_uncertainty_bound_w: 10.0,
            conservative_compute_reduction_w: 210.0,
            credited_deliverable_capacity_w: if success { 200.0 } else { 0.0 },
            measurement_uncertainty_fraction: 0.05,
            rebound_energy_fraction: Some(0.05),
            service_contract_pass: success,
            commitment_power_pass: success,
            event_success: success,
        }
    }

    fn source_report(cluster_sizes: &[usize], failed_event_index: Option<usize>) -> GridServiceQualificationReport {
        let total_events: usize = cluster_sizes.iter().sum();
        let contract = source_contract();
        let mut event_results = Vec::with_capacity(total_events);
        let mut next_index = 0;
        for (cluster_index, count) in cluster_sizes.iter().copied().enumerate() {
            for _ in 0..count {
                let success = Some(next_index) != failed_event_index;
                event_results.push(event_result(next_index, cluster_index, success));
                next_index += 1;
            }
        }
        let success_count = event_results.iter().filter(|event| event.event_success).count();
        GridServiceQualificationReport {
            contract: contract.clone(),
            resource_id: contract.resource_id.clone(),
            grid_zone_id: contract.grid_zone_id.clone(),
            point_of_interconnection_id: contract.point_of_interconnection_id.clone(),
            service_class: contract.service_class,
            baseline_method: contract.baseline_method,
            total_event_count: total_events,
            service_feasible_event_count: success_count,
            commitment_success_event_count: success_count,
            empirical_event_success_fraction: success_count as f64 / total_events as f64,
            wilson_lower_bound_success_fraction: None,
            p10_credited_compute_capacity_w: None,
            committed_compute_reduction_w: contract.committed_compute_reduction_w,
            meets_minimum_event_count: total_events >= contract.minimum_event_count,
            meets_capacity_commitment: false,
            meets_reliability_lower_bound: false,
            independent_event_assumption_available: false,
            commitment_qualified: false,
            event_results,
            claim_ceiling: "REPEATED_EVENT_OFFLINE_EVALUATION_NOT_MARKET_ACCREDITATION",
        }
    }

    fn cluster_contract(required_strata: &[&str], independent: bool) -> ClusteredReliabilityContract {
        ClusteredReliabilityContract {
            analysis_id: "clustered-analysis-v1".to_string(),
            source_service_contract_id: "service-contract-v1".to_string(),
            source_event_sampling_design_id: "event-design-v1".to_string(),
            source_report_evidence_id: "source-report-digest-v1".to_string(),
            cluster_sampling_design_id: "site-day-cluster-design-v1".to_string(),
            cluster_assignment_evidence_id: "cluster-assignment-digest-v1".to_string(),
            independence_review_evidence_id: independent.then(|| "independence-review-v1".to_string()),
            independent_cluster_assumption: independent,
            required_strata: required_strata.iter().map(|value| (*value).to_string()).collect(),
            target_lower_quantile: 0.10,
            one_sided_confidence: 0.90,
            success_confidence_z_score: 1.281_551_565_5,
            cluster_window_duration_ns: 100_000_000_000,
            events_per_cluster: 1,
            minimum_cluster_separation_ns: if independent { 1_000_000_000 } else { 0 },
        }
    }

    fn fixture(
        cluster_sizes: &[usize],
        failed_event_index: Option<usize>,
        strata: &[&str],
        independent: bool,
    ) -> (
        GridServiceQualificationReport,
        ClusteredReliabilityContract,
        Vec<IndependenceCluster>,
        Vec<EventClusterAssignment>,
    ) {
        let source = source_report(cluster_sizes, failed_event_index);
        let mut clusters = Vec::with_capacity(cluster_sizes.len());
        let mut assignments = Vec::with_capacity(source.total_event_count);
        let mut event_index = 0;
        for (cluster_index, count) in cluster_sizes.iter().copied().enumerate() {
            let start = cluster_index as u64 * 200_000_000_000;
            let stratum = strata[cluster_index];
            clusters.push(IndependenceCluster {
                cluster_id: format!("cluster-{cluster_index:03}"),
                stratum_id: stratum.to_string(),
                window_start_ns: start,
                window_end_ns: start + 100_000_000_000,
                design_evidence_id: format!("cluster-design-evidence-{cluster_index:03}"),
            });
            for _ in 0..count {
                assignments.push(EventClusterAssignment {
                    event_id: format!("event-{event_index:03}"),
                    cluster_id: format!("cluster-{cluster_index:03}"),
                    assignment_evidence_id: format!("assignment-evidence-{event_index:03}"),
                });
                event_index += 1;
            }
        }
        let mut required_strata = strata.to_vec();
        required_strata.sort_unstable();
        required_strata.dedup();
        (
            source,
            cluster_contract(&required_strata, independent),
            clusters,
            assignments,
        )
    }

    fn one_stratum(values: usize) -> Vec<&'static str> {
        vec!["all"; values]
    }

    #[test]
    fn requires_twenty_two_independent_clusters_for_p10_at_ninety_percent_confidence() {
        assert_eq!(
            minimum_clusters_for_quantile_confidence(0.10, 0.90),
            Some(22)
        );
    }

    #[test]
    fn requires_twenty_nine_independent_clusters_for_p10_at_ninety_five_percent_confidence() {
        assert_eq!(
            minimum_clusters_for_quantile_confidence(0.10, 0.95),
            Some(29)
        );
    }

    #[test]
    fn rejects_invalid_quantile_and_confidence_values() {
        assert_eq!(minimum_clusters_for_quantile_confidence(0.0, 0.90), None);
        assert_eq!(minimum_clusters_for_quantile_confidence(0.10, 1.0), None);
        assert_eq!(minimum_clusters_for_quantile_confidence(f64::NAN, 0.90), None);
    }

    #[test]
    fn qualifies_thirty_events_across_thirty_independent_clusters() {
        let sizes = vec![1; 30];
        let strata = one_stratum(sizes.len());
        let (source, contract, clusters, assignments) =
            fixture(&sizes, None, &strata, true);
        let result =
            evaluate_clustered_reliability(&source, contract, clusters, assignments).unwrap();
        assert!(result.commitment_qualified);
        assert_eq!(result.total_event_count, 30);
        assert_eq!(result.total_cluster_count, 30);
        assert_eq!(
            result.minimum_clusters_per_stratum,
            22
        );
        assert_eq!(
            result.overall_lower_quantile_capacity_lower_confidence_bound_w,
            Some(200.0)
        );
        assert!(result
            .overall_cluster_success_probability_lower_bound
            .is_some_and(|value| value >= 0.90));
    }

    #[test]
    fn thirty_correlated_events_in_three_clusters_do_not_count_as_thirty_trials() {
        let sizes = vec![10, 10, 10];
        let strata = one_stratum(sizes.len());
        let (source, mut contract, clusters, assignments) =
            fixture(&sizes, None, &strata, true);
        contract.events_per_cluster = 10;
        let result =
            evaluate_clustered_reliability(&source, contract, clusters, assignments).unwrap();
        assert!(!result.commitment_qualified);
        assert_eq!(result.total_event_count, 30);
        assert_eq!(result.total_cluster_count, 3);
        assert_eq!(
            result.overall_lower_quantile_capacity_lower_confidence_bound_w,
            None
        );
        assert!(!result.strata[0].quantile_support_sufficient);
    }

    #[test]
    fn one_failed_event_fails_its_entire_cluster() {
        let sizes = vec![2, 2, 2];
        let strata = one_stratum(sizes.len());
        let (source, mut contract, clusters, assignments) =
            fixture(&sizes, Some(0), &strata, true);
        contract.events_per_cluster = 2;
        let result =
            evaluate_clustered_reliability(&source, contract, clusters, assignments).unwrap();
        let failed_cluster = result
            .cluster_outcomes
            .iter()
            .find(|cluster| cluster.cluster_id == "cluster-000")
            .unwrap();
        assert!(!failed_cluster.all_events_successful);
        assert_eq!(failed_cluster.successful_event_count, 1);
        assert_eq!(failed_cluster.minimum_credited_capacity_w, 0.0);
    }

    #[test]
    fn every_required_stratum_needs_its_own_independent_cluster_support() {
        let sizes = vec![1; 30];
        let mut strata = one_stratum(30);
        for stratum in strata.iter_mut().skip(15) {
            *stratum = "winter";
        }
        for stratum in strata.iter_mut().take(15) {
            *stratum = "summer";
        }
        let (source, contract, clusters, assignments) =
            fixture(&sizes, None, &strata, true);
        let result =
            evaluate_clustered_reliability(&source, contract, clusters, assignments).unwrap();
        assert!(!result.commitment_qualified);
        assert_eq!(result.strata.len(), 2);
        assert_eq!(result.strata[0].independent_cluster_count, 15);
        assert_eq!(result.strata[1].independent_cluster_count, 15);
        assert!(!result.all_required_strata_supported);
    }

    #[test]
    fn withholds_confidence_bounds_when_cluster_independence_is_not_declared() {
        let sizes = vec![1; 30];
        let strata = one_stratum(sizes.len());
        let (source, contract, clusters, assignments) =
            fixture(&sizes, None, &strata, false);
        let result =
            evaluate_clustered_reliability(&source, contract, clusters, assignments).unwrap();
        assert!(!result.independent_cluster_assumption_available);
        assert!(!result.commitment_qualified);
        assert_eq!(
            result.overall_lower_quantile_capacity_lower_confidence_bound_w,
            None
        );
        assert_eq!(
            result.overall_cluster_success_probability_lower_bound,
            None
        );
    }

    #[test]
    fn rejects_missing_event_assignment() {
        let sizes = vec![1; 30];
        let strata = one_stratum(sizes.len());
        let (source, contract, clusters, mut assignments) =
            fixture(&sizes, None, &strata, true);
        assignments.pop();
        assert!(evaluate_clustered_reliability(&source, contract, clusters, assignments).is_err());
    }

    #[test]
    fn rejects_duplicate_event_assignment() {
        let sizes = vec![1; 30];
        let strata = one_stratum(sizes.len());
        let (source, contract, clusters, mut assignments) =
            fixture(&sizes, None, &strata, true);
        assignments.push(assignments[0].clone());
        assert!(evaluate_clustered_reliability(&source, contract, clusters, assignments).is_err());
    }

    #[test]
    fn rejects_cluster_with_non_matching_exposure_duration() {
        let sizes = vec![1; 30];
        let strata = one_stratum(sizes.len());
        let (source, contract, mut clusters, assignments) =
            fixture(&sizes, None, &strata, true);
        clusters[0].window_end_ns -= 1;
        assert!(evaluate_clustered_reliability(&source, contract, clusters, assignments).is_err());
    }

    #[test]
    fn rejects_cluster_event_count_different_from_frozen_contract() {
        let sizes = vec![1; 30];
        let strata = one_stratum(sizes.len());
        let (source, mut contract, clusters, assignments) =
            fixture(&sizes, None, &strata, true);
        contract.events_per_cluster = 2;
        assert!(evaluate_clustered_reliability(&source, contract, clusters, assignments).is_err());
    }

    #[test]
    fn rejects_invalid_cluster_success_wilson_z_score() {
        let sizes = vec![1; 30];
        let strata = one_stratum(sizes.len());
        let (source, mut contract, clusters, assignments) =
            fixture(&sizes, None, &strata, true);
        contract.success_confidence_z_score = f64::NAN;
        assert!(evaluate_clustered_reliability(&source, contract, clusters, assignments).is_err());
    }

    #[test]
    fn rejects_cluster_windows_that_violate_declared_separation() {
        let sizes = vec![1; 30];
        let strata = one_stratum(sizes.len());
        let (source, contract, mut clusters, assignments) =
            fixture(&sizes, None, &strata, true);
        clusters[1].window_start_ns = clusters[0].window_end_ns;
        assert!(evaluate_clustered_reliability(&source, contract, clusters, assignments).is_err());
    }

    #[test]
    fn rejects_event_whose_recovery_is_outside_its_cluster_window() {
        let sizes = vec![1; 30];
        let strata = one_stratum(sizes.len());
        let (source, contract, mut clusters, assignments) =
            fixture(&sizes, None, &strata, true);
        clusters[0].window_end_ns = clusters[0].window_start_ns + 1_100_000_000;
        assert!(evaluate_clustered_reliability(&source, contract, clusters, assignments).is_err());
    }

    #[test]
    fn rejects_missing_required_stratum() {
        let sizes = vec![1; 30];
        let strata = one_stratum(sizes.len());
        let (source, mut contract, clusters, assignments) =
            fixture(&sizes, None, &strata, true);
        contract.required_strata.push("winter".to_string());
        assert!(evaluate_clustered_reliability(&source, contract, clusters, assignments).is_err());
    }

    #[test]
    fn rejects_source_service_contract_mismatch() {
        let sizes = vec![1; 30];
        let strata = one_stratum(sizes.len());
        let (source, mut contract, clusters, assignments) =
            fixture(&sizes, None, &strata, true);
        contract.source_service_contract_id = "other-contract".to_string();
        assert!(evaluate_clustered_reliability(&source, contract, clusters, assignments).is_err());
    }

    #[test]
    fn rejects_inconsistent_source_event_outcomes() {
        let sizes = vec![1; 30];
        let strata = one_stratum(sizes.len());
        let (mut source, contract, clusters, assignments) =
            fixture(&sizes, None, &strata, true);
        source.event_results[0].event_success = false;
        assert!(evaluate_clustered_reliability(&source, contract, clusters, assignments).is_err());
    }

    #[test]
    fn rejects_cluster_that_has_no_assigned_event() {
        let sizes = vec![1; 30];
        let strata = one_stratum(sizes.len());
        let (source, contract, mut clusters, assignments) =
            fixture(&sizes, None, &strata, true);
        clusters.push(IndependenceCluster {
            cluster_id: "unused".to_string(),
            stratum_id: "all".to_string(),
            window_start_ns: 7_000_000_000_000,
            window_end_ns: 7_100_000_000_000,
            design_evidence_id: "unused-cluster-evidence".to_string(),
        });
        assert!(evaluate_clustered_reliability(&source, contract, clusters, assignments).is_err());
    }

    #[test]
    fn rejects_unknown_event_assignment() {
        let sizes = vec![1; 30];
        let strata = one_stratum(sizes.len());
        let (source, contract, clusters, mut assignments) =
            fixture(&sizes, None, &strata, true);
        assignments[0].event_id = "unknown".to_string();
        assert!(evaluate_clustered_reliability(&source, contract, clusters, assignments).is_err());
    }
}
