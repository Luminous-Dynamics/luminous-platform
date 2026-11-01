//! Request-level impact measurement and exact paired counterfactual cells (v0.2).
//!
//! This module deliberately does not infer unique request overlap from aggregate counters.
//! Each degraded request is represented by an evaluator-local keyed pseudonym token supplied
//! by the independent ingress observer. Raw request IDs must never enter this evaluator.
//! Signature verification and publisher authentication belong in the production evidence
//! adapter; these types alone do not authenticate their source.

use super::Digest;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImpactKind {
    Failed,
    Slow,
    IncorrectBusinessResult,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObservationArm {
    Candidate,
    NoAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DemandContract {
    /// Digest of the preregistered offered-arrival schedule and demand band.
    pub digest: Digest,
    pub minimum_offered_requests: u64,
    pub maximum_offered_requests: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairCellContract {
    pub pair_id: String,
    pub scenario_id: String,
    pub cluster_id: String,
    pub service: String,
    pub request_class: String,
    pub phase: String,
    pub starts_at_ms: u64,
    pub ends_at_ms: u64,
    pub workload_profile_digest: Digest,
    pub phase_schedule_digest: Digest,
    pub demand: DemandContract,
    pub pseudonym_key_epoch: String,
    pub expected_observer_source_id: String,
    pub expected_observer_control_domain: String,
    pub executor_control_domain: String,
    pub evaluation_time_ms: u64,
    pub maximum_receipt_age_ms: u64,
    pub maximum_event_rows: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImpactEvent {
    /// 32-byte keyed pseudonym rendered as 64 lowercase hexadecimal characters.
    /// This is not a plain hash of a request identifier.
    pub request_token: String,
    pub kind: ImpactKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationCell {
    pub pair_id: String,
    pub arm: ObservationArm,
    pub scenario_id: String,
    pub cluster_id: String,
    pub service: String,
    pub request_class: String,
    pub phase: String,
    pub starts_at_ms: u64,
    pub ends_at_ms: u64,
    pub workload_profile_digest: Digest,
    pub phase_schedule_digest: Digest,
    pub demand_contract_digest: Digest,
    pub offered_requests: u64,
    pub pseudonym_key_epoch: String,
    pub observer_source_id: String,
    pub observer_control_domain: String,
    pub receipt_issued_at_ms: u64,
    /// False means the observer could not enumerate/classify the complete offered cohort.
    pub complete: bool,
    /// False means the request-token set was truncated or otherwise not exact.
    pub enumeration_complete: bool,
    pub events: Vec<ImpactEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImpactMetrics {
    pub offered_requests: u64,
    pub failed_requests: u64,
    pub slow_requests: u64,
    pub incorrect_business_requests: u64,
    pub unique_degraded_requests: u64,
    pub duplicate_event_rows: u64,
    pub duration_ms: u64,
    pub unique_degraded_request_ms: u128,
    pub impact_rate_ppm: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellFailure {
    MissingCandidateCell,
    MissingNoActionCell,
    DuplicateCandidateCell,
    DuplicateNoActionCell,
    UnmatchedObservation,
    InvalidPairContract,
    ScenarioMismatch,
    ClusterMismatch,
    ServiceMismatch,
    RequestClassMismatch,
    PhaseMismatch,
    IntervalMismatch,
    WorkloadProfileMismatch,
    PhaseScheduleMismatch,
    DemandContractMismatch,
    PseudonymKeyEpochMismatch,
    ObserverSourceMismatch,
    ObserverControlDomainMismatch,
    ObserverExecutorDomainCollision,
    ReceiptFromFuture,
    ReceiptStale,
    IncompleteObservation,
    IncompleteEnumeration,
    TooManyEventRows,
    InvalidRequestToken,
    OfferedDemandOutsideContract,
    OfferedDemandMismatch,
    UniqueUnionExceedsOffered,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairedCellResult {
    pub pair_id: String,
    pub cluster_id: String,
    pub failures: Vec<CellFailure>,
    pub candidate: Option<ImpactMetrics>,
    pub no_action: Option<ImpactMetrics>,
    /// Positive means the candidate caused more degraded request-time than no-action.
    pub candidate_minus_no_action_request_ms: Option<i128>,
}

impl PairedCellResult {
    pub fn is_valid(&self) -> bool {
        self.failures.is_empty() && self.candidate.is_some() && self.no_action.is_some()
            && self.candidate_minus_no_action_request_ms.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImpactCoverage {
    pub expected_pairs: usize,
    pub valid_pairs: usize,
    pub indeterminate_pairs: usize,
    pub unmatched_observations: usize,
    pub duplicate_candidate_cells: usize,
    pub duplicate_no_action_cells: usize,
    pub missing_candidate_cells: usize,
    pub missing_no_action_cells: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImpactSummary {
    pub coverage: ImpactCoverage,
    /// Only present when every preregistered pair has exactly one valid cell per arm.
    pub complete_candidate_minus_no_action_request_ms: Option<i128>,
    pub pairs: Vec<PairedCellResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImpactError {
    EmptyPairManifest,
    DuplicatePairId(String),
    InvalidPairContract(String),
    ArithmeticOverflow,
}

pub fn evaluate_paired_impact(
    contracts: &[PairCellContract],
    observations: &[ObservationCell],
) -> Result<ImpactSummary, ImpactError> {
    if contracts.is_empty() {
        return Err(ImpactError::EmptyPairManifest);
    }

    let mut contract_index = BTreeMap::new();
    for (index, contract) in contracts.iter().enumerate() {
        validate_pair_contract(contract)?;
        if contract_index.insert(contract.pair_id.as_str(), index).is_some() {
            return Err(ImpactError::DuplicatePairId(contract.pair_id.clone()));
        }
    }

    let mut cells: BTreeMap<(&str, ObservationArm), Vec<&ObservationCell>> = BTreeMap::new();
    let mut unmatched_observations = 0usize;
    for observation in observations {
        if !contract_index.contains_key(observation.pair_id.as_str()) {
            unmatched_observations += 1;
            continue;
        }
        cells.entry((observation.pair_id.as_str(), observation.arm))
            .or_default().push(observation);
    }

    let mut pairs = Vec::with_capacity(contracts.len());
    let mut duplicate_candidate_cells = 0usize;
    let mut duplicate_no_action_cells = 0usize;
    let mut missing_candidate_cells = 0usize;
    let mut missing_no_action_cells = 0usize;
    let mut valid_pairs = 0usize;
    let mut total_delta = 0i128;
    let mut arithmetic_ok = true;

    for contract in contracts {
        let candidate_cells = cells.get(&(contract.pair_id.as_str(), ObservationArm::Candidate))
            .map(Vec::as_slice).unwrap_or(&[]);
        let no_action_cells = cells.get(&(contract.pair_id.as_str(), ObservationArm::NoAction))
            .map(Vec::as_slice).unwrap_or(&[]);
        let mut failures = BTreeSet::new();

        if candidate_cells.is_empty() {
            failures.insert(CellFailure::MissingCandidateCell);
            missing_candidate_cells += 1;
        } else if candidate_cells.len() > 1 {
            failures.insert(CellFailure::DuplicateCandidateCell);
            duplicate_candidate_cells += candidate_cells.len() - 1;
        }

        if no_action_cells.is_empty() {
            failures.insert(CellFailure::MissingNoActionCell);
            missing_no_action_cells += 1;
        } else if no_action_cells.len() > 1 {
            failures.insert(CellFailure::DuplicateNoActionCell);
            duplicate_no_action_cells += no_action_cells.len() - 1;
        }

        let candidate = if candidate_cells.len() == 1 {
            match measure_cell(contract, candidate_cells[0]) {
                Ok(metrics) => Some(metrics),
                Err(failure) => {
                    failures.insert(failure);
                    None
                }
            }
        } else {
            None
        };
        let no_action = if no_action_cells.len() == 1 {
            match measure_cell(contract, no_action_cells[0]) {
                Ok(metrics) => Some(metrics),
                Err(failure) => {
                    failures.insert(failure);
                    None
                }
            }
        } else {
            None
        };

        let delta = match (&candidate, &no_action) {
            (Some(candidate_metrics), Some(no_action_metrics)) => {
                if candidate_metrics.offered_requests != no_action_metrics.offered_requests {
                    failures.insert(CellFailure::OfferedDemandMismatch);
                    None
                } else {
                    let candidate_value = i128::try_from(candidate_metrics.unique_degraded_request_ms)
                        .map_err(|_| ImpactError::ArithmeticOverflow)?;
                    let baseline_value = i128::try_from(no_action_metrics.unique_degraded_request_ms)
                        .map_err(|_| ImpactError::ArithmeticOverflow)?;
                    candidate_value.checked_sub(baseline_value)
                }
            }
            _ => None,
        };

        let delta = match delta {
            Some(value) if failures.is_empty() => Some(value),
            Some(_) => None,
            None => None,
        };

        if let Some(value) = delta {
            match total_delta.checked_add(value) {
                Some(sum) => total_delta = sum,
                None => arithmetic_ok = false,
            }
            valid_pairs += 1;
        }

        pairs.push(PairedCellResult {
            pair_id: contract.pair_id.clone(),
            cluster_id: contract.cluster_id.clone(),
            failures: failures.into_iter().collect(),
            candidate,
            no_action,
            candidate_minus_no_action_request_ms: delta,
        });
    }

    let indeterminate_pairs = pairs.len() - valid_pairs;
    let complete = indeterminate_pairs == 0
        && unmatched_observations == 0
        && duplicate_candidate_cells == 0
        && duplicate_no_action_cells == 0
        && arithmetic_ok;

    Ok(ImpactSummary {
        coverage: ImpactCoverage {
            expected_pairs: contracts.len(),
            valid_pairs,
            indeterminate_pairs,
            unmatched_observations,
            duplicate_candidate_cells,
            duplicate_no_action_cells,
            missing_candidate_cells,
            missing_no_action_cells,
        },
        complete_candidate_minus_no_action_request_ms: if complete {
            Some(total_delta)
        } else {
            None
        },
        pairs,
    })
}

fn validate_pair_contract(contract: &PairCellContract) -> Result<(), ImpactError> {
    let non_empty = [
        contract.pair_id.as_str(),
        contract.scenario_id.as_str(),
        contract.cluster_id.as_str(),
        contract.service.as_str(),
        contract.request_class.as_str(),
        contract.phase.as_str(),
        contract.pseudonym_key_epoch.as_str(),
        contract.expected_observer_source_id.as_str(),
        contract.expected_observer_control_domain.as_str(),
        contract.executor_control_domain.as_str(),
    ].iter().all(|value| !value.trim().is_empty());

    if !non_empty
        || contract.starts_at_ms >= contract.ends_at_ms
        || contract.demand.minimum_offered_requests == 0
        || contract.demand.minimum_offered_requests > contract.demand.maximum_offered_requests
        || contract.maximum_receipt_age_ms == 0
        || contract.maximum_event_rows == 0
        || contract.expected_observer_control_domain == contract.executor_control_domain
    {
        return Err(ImpactError::InvalidPairContract(contract.pair_id.clone()));
    }
    Ok(())
}

fn measure_cell(
    contract: &PairCellContract,
    cell: &ObservationCell,
) -> Result<ImpactMetrics, CellFailure> {
    if cell.scenario_id != contract.scenario_id { return Err(CellFailure::ScenarioMismatch); }
    if cell.cluster_id != contract.cluster_id { return Err(CellFailure::ClusterMismatch); }
    if cell.service != contract.service { return Err(CellFailure::ServiceMismatch); }
    if cell.request_class != contract.request_class { return Err(CellFailure::RequestClassMismatch); }
    if cell.phase != contract.phase { return Err(CellFailure::PhaseMismatch); }
    if cell.starts_at_ms != contract.starts_at_ms || cell.ends_at_ms != contract.ends_at_ms {
        return Err(CellFailure::IntervalMismatch);
    }
    if cell.workload_profile_digest != contract.workload_profile_digest {
        return Err(CellFailure::WorkloadProfileMismatch);
    }
    if cell.phase_schedule_digest != contract.phase_schedule_digest {
        return Err(CellFailure::PhaseScheduleMismatch);
    }
    if cell.demand_contract_digest != contract.demand.digest {
        return Err(CellFailure::DemandContractMismatch);
    }
    if cell.pseudonym_key_epoch != contract.pseudonym_key_epoch {
        return Err(CellFailure::PseudonymKeyEpochMismatch);
    }
    if cell.observer_source_id != contract.expected_observer_source_id {
        return Err(CellFailure::ObserverSourceMismatch);
    }
    if cell.observer_control_domain != contract.expected_observer_control_domain {
        return Err(CellFailure::ObserverControlDomainMismatch);
    }
    if cell.observer_control_domain == contract.executor_control_domain {
        return Err(CellFailure::ObserverExecutorDomainCollision);
    }
    if cell.receipt_issued_at_ms > contract.evaluation_time_ms {
        return Err(CellFailure::ReceiptFromFuture);
    }
    if contract.evaluation_time_ms - cell.receipt_issued_at_ms > contract.maximum_receipt_age_ms {
        return Err(CellFailure::ReceiptStale);
    }
    if !cell.complete { return Err(CellFailure::IncompleteObservation); }
    if !cell.enumeration_complete { return Err(CellFailure::IncompleteEnumeration); }
    if cell.events.len() > contract.maximum_event_rows {
        return Err(CellFailure::TooManyEventRows);
    }
    if cell.offered_requests < contract.demand.minimum_offered_requests
        || cell.offered_requests > contract.demand.maximum_offered_requests
    {
        return Err(CellFailure::OfferedDemandOutsideContract);
    }

    let mut union = BTreeSet::new();
    let mut failed = BTreeSet::new();
    let mut slow = BTreeSet::new();
    let mut incorrect = BTreeSet::new();
    let mut duplicate_event_rows = 0u64;

    for event in &cell.events {
        if !is_keyed_pseudonym_token(&event.request_token) {
            return Err(CellFailure::InvalidRequestToken);
        }
        union.insert(event.request_token.as_str());
        let inserted = match event.kind {
            ImpactKind::Failed => failed.insert(event.request_token.as_str()),
            ImpactKind::Slow => slow.insert(event.request_token.as_str()),
            ImpactKind::IncorrectBusinessResult => incorrect.insert(event.request_token.as_str()),
        };
        if !inserted {
            duplicate_event_rows = duplicate_event_rows.checked_add(1)
                .ok_or(CellFailure::UniqueUnionExceedsOffered)?;
        }
    }

    let union_count = u64::try_from(union.len())
        .map_err(|_| CellFailure::UniqueUnionExceedsOffered)?;
    if union_count > cell.offered_requests {
        return Err(CellFailure::UniqueUnionExceedsOffered);
    }
    let duration_ms = cell.ends_at_ms - cell.starts_at_ms;
    let unique_degraded_request_ms = (union_count as u128)
        .checked_mul(duration_ms as u128)
        .ok_or(CellFailure::UniqueUnionExceedsOffered)?;
    let impact_rate_ppm = (union_count as u128)
        .checked_mul(1_000_000)
        .ok_or(CellFailure::UniqueUnionExceedsOffered)?
        .checked_div(cell.offered_requests as u128)
        .ok_or(CellFailure::OfferedDemandOutsideContract)?;
    let impact_rate_ppm = u64::try_from(impact_rate_ppm)
        .map_err(|_| CellFailure::UniqueUnionExceedsOffered)?;

    Ok(ImpactMetrics {
        offered_requests: cell.offered_requests,
        failed_requests: failed.len() as u64,
        slow_requests: slow.len() as u64,
        incorrect_business_requests: incorrect.len() as u64,
        unique_degraded_requests: union_count,
        duplicate_event_rows,
        duration_ms,
        unique_degraded_request_ms,
        impact_rate_ppm,
    })
}

fn is_keyed_pseudonym_token(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(value: u8) -> String {
        format!("{:02x}", value).repeat(32)
    }

    fn contract() -> PairCellContract {
        PairCellContract {
            pair_id: "pair-001".into(),
            scenario_id: "scenario-001".into(),
            cluster_id: "cluster-001".into(),
            service: "checkout".into(),
            request_class: "critical-write".into(),
            phase: "fault-and-recovery".into(),
            starts_at_ms: 100,
            ends_at_ms: 200,
            workload_profile_digest: Digest::from_byte(1),
            phase_schedule_digest: Digest::from_byte(2),
            demand: DemandContract {
                digest: Digest::from_byte(3),
                minimum_offered_requests: 100,
                maximum_offered_requests: 100,
            },
            pseudonym_key_epoch: "hmac-epoch-7".into(),
            expected_observer_source_id: "ingress-observer-a".into(),
            expected_observer_control_domain: "ingress-domain".into(),
            executor_control_domain: "agent-domain".into(),
            evaluation_time_ms: 1_000,
            maximum_receipt_age_ms: 100,
            maximum_event_rows: 300,
        }
    }

    fn cell(arm: ObservationArm) -> ObservationCell {
        ObservationCell {
            pair_id: "pair-001".into(),
            arm,
            scenario_id: "scenario-001".into(),
            cluster_id: "cluster-001".into(),
            service: "checkout".into(),
            request_class: "critical-write".into(),
            phase: "fault-and-recovery".into(),
            starts_at_ms: 100,
            ends_at_ms: 200,
            workload_profile_digest: Digest::from_byte(1),
            phase_schedule_digest: Digest::from_byte(2),
            demand_contract_digest: Digest::from_byte(3),
            offered_requests: 100,
            pseudonym_key_epoch: "hmac-epoch-7".into(),
            observer_source_id: "ingress-observer-a".into(),
            observer_control_domain: "ingress-domain".into(),
            receipt_issued_at_ms: 990,
            complete: true,
            enumeration_complete: true,
            events: Vec::new(),
        }
    }

    fn pair() -> (PairCellContract, Vec<ObservationCell>) {
        (contract(), vec![
            cell(ObservationArm::Candidate),
            cell(ObservationArm::NoAction),
        ])
    }

    #[test]
    fn disjoint_failure_latency_and_correctness_sets_count_the_exact_union() {
        let (spec, mut cells) = pair();
        for index in 0..10 {
            cells[0].events.push(ImpactEvent { request_token: token(index), kind: ImpactKind::Failed });
            cells[0].events.push(ImpactEvent { request_token: token(index + 10), kind: ImpactKind::Slow });
            cells[0].events.push(ImpactEvent { request_token: token(index + 20), kind: ImpactKind::IncorrectBusinessResult });
        }
        let result = evaluate_paired_impact(&[spec], &cells).unwrap();
        let metrics = result.pairs[0].candidate.as_ref().unwrap();
        assert_eq!(metrics.failed_requests, 10);
        assert_eq!(metrics.slow_requests, 10);
        assert_eq!(metrics.incorrect_business_requests, 10);
        assert_eq!(metrics.unique_degraded_requests, 30);
        assert_eq!(metrics.unique_degraded_request_ms, 3_000);
    }

    #[test]
    fn fully_overlapping_failure_latency_and_correctness_sets_count_once() {
        let (spec, mut cells) = pair();
        for index in 0..10 {
            let id = token(index);
            for kind in [ImpactKind::Failed, ImpactKind::Slow, ImpactKind::IncorrectBusinessResult] {
                cells[0].events.push(ImpactEvent { request_token: id.clone(), kind });
            }
        }
        let result = evaluate_paired_impact(&[spec], &cells).unwrap();
        let metrics = result.pairs[0].candidate.as_ref().unwrap();
        assert_eq!(metrics.failed_requests, 10);
        assert_eq!(metrics.slow_requests, 10);
        assert_eq!(metrics.incorrect_business_requests, 10);
        assert_eq!(metrics.unique_degraded_requests, 10);
    }

    #[test]
    fn cross_category_overlap_and_duplicate_rows_preserve_dimension_counts() {
        let (spec, mut cells) = pair();
        let id = token(1);
        cells[0].events = vec![
            ImpactEvent { request_token: id.clone(), kind: ImpactKind::Failed },
            ImpactEvent { request_token: id.clone(), kind: ImpactKind::Slow },
            ImpactEvent { request_token: id.clone(), kind: ImpactKind::IncorrectBusinessResult },
            ImpactEvent { request_token: id.clone(), kind: ImpactKind::Failed },
        ];
        let result = evaluate_paired_impact(&[spec], &cells).unwrap();
        let metrics = result.pairs[0].candidate.as_ref().unwrap();
        assert_eq!(metrics.unique_degraded_requests, 1);
        assert_eq!(metrics.failed_requests, 1);
        assert_eq!(metrics.slow_requests, 1);
        assert_eq!(metrics.incorrect_business_requests, 1);
        assert_eq!(metrics.duplicate_event_rows, 1);
    }

    #[test]
    fn exact_paired_cells_produce_signed_candidate_minus_no_action_delta() {
        let (spec, mut cells) = pair();
        cells[0].events.push(ImpactEvent { request_token: token(1), kind: ImpactKind::Failed });
        cells[1].events.extend((1..=3).map(|n| ImpactEvent {
            request_token: token(n), kind: ImpactKind::Failed,
        }));
        let result = evaluate_paired_impact(&[spec], &cells).unwrap();
        assert_eq!(result.coverage.valid_pairs, 1);
        assert_eq!(result.complete_candidate_minus_no_action_request_ms, Some(-200));
        assert_eq!(result.pairs[0].candidate_minus_no_action_request_ms, Some(-200));
    }

    #[test]
    fn duplicate_event_rows_do_not_inflate_union_or_dimension_counts() {
        let (spec, mut cells) = pair();
        let event = ImpactEvent { request_token: token(1), kind: ImpactKind::Failed };
        cells[0].events = vec![event.clone(), event.clone(), event];
        let result = evaluate_paired_impact(&[spec], &cells).unwrap();
        let metrics = result.pairs[0].candidate.as_ref().unwrap();
        assert_eq!(metrics.unique_degraded_requests, 1);
        assert_eq!(metrics.duplicate_event_rows, 2);
    }

    #[test]
    fn paired_cell_arithmetic_preserves_negative_and_positive_signs() {
        let (spec, mut cells) = pair();
        cells[0].events = (1..=4).map(|n| ImpactEvent {
            request_token: token(n), kind: ImpactKind::Slow,
        }).collect();
        cells[1].events = (1..=2).map(|n| ImpactEvent {
            request_token: token(n), kind: ImpactKind::Slow,
        }).collect();
        let result = evaluate_paired_impact(&[spec.clone()], &cells).unwrap();
        assert_eq!(result.complete_candidate_minus_no_action_request_ms, Some(200));
        let swapped = vec![
            ObservationCell { arm: ObservationArm::NoAction, ..cells[0].clone() },
            ObservationCell { arm: ObservationArm::Candidate, ..cells[1].clone() },
        ];
        let result = evaluate_paired_impact(&[spec], &swapped).unwrap();
        assert_eq!(result.complete_candidate_minus_no_action_request_ms, Some(-200));
    }

    #[test]
    fn incomplete_pair_never_emits_a_complete_aggregate_delta() {
        let (spec, mut cells) = pair();
        cells.pop();
        let result = evaluate_paired_impact(&[spec], &cells).unwrap();
        assert_eq!(result.coverage.missing_no_action_cells, 1);
        assert_eq!(result.coverage.indeterminate_pairs, 1);
        assert_eq!(result.complete_candidate_minus_no_action_request_ms, None);
    }

    #[test]
    fn unknown_extra_cells_are_not_silently_ignored() {
        let (spec, mut cells) = pair();
        let mut extra = cell(ObservationArm::Candidate);
        extra.pair_id = "unregistered-easy-window".into();
        cells.push(extra);
        let result = evaluate_paired_impact(&[spec], &cells).unwrap();
        assert_eq!(result.coverage.unmatched_observations, 1);
        assert_eq!(result.complete_candidate_minus_no_action_request_ms, None);
    }

    #[test]
    fn paired_manifest_rejects_duplicate_pair_ids() {
        let spec = contract();
        assert_eq!(
            evaluate_paired_impact(&[spec.clone(), spec], &[]),
            Err(ImpactError::DuplicatePairId("pair-001".into()))
        );
    }

    #[test]
    fn paired_manifest_rejects_observer_executor_control_domain_collision() {
        let mut spec = contract();
        spec.expected_observer_control_domain = spec.executor_control_domain.clone();
        assert!(matches!(evaluate_paired_impact(&[spec], &[]), Err(ImpactError::InvalidPairContract(_))));
    }

    #[test]
    fn paired_observation_rejects_request_union_larger_than_offered_demand() {
        let (spec, mut cells) = pair();
        cells[0].events = (0..101).map(|n| ImpactEvent {
            request_token: token(n as u8), kind: ImpactKind::Failed,
        }).collect();
        let result = evaluate_paired_impact(&[spec], &cells).unwrap();
        assert_eq!(result.pairs[0].failures, vec![CellFailure::UniqueUnionExceedsOffered]);
        assert_eq!(result.complete_candidate_minus_no_action_request_ms, None);
    }

    #[test]
    fn forty_eight_adversarial_mutations_fail_closed() {
        for vector in 0..48 {
            let (mut spec, mut cells) = pair();
            match vector {
                0 => { cells.remove(0); }
                1 => { cells.remove(1); }
                2 => { cells.push(cells[0].clone()); }
                3 => { cells.push(cells[1].clone()); }
                4 => { cells[0].pair_id = "unknown-pair".into(); }
                5 => { cells[0].scenario_id = "other-scenario".into(); }
                6 => { cells[0].cluster_id = "other-cluster".into(); }
                7 => { cells[0].service = "other-service".into(); }
                8 => { cells[0].request_class = "other-class".into(); }
                9 => { cells[0].phase = "other-phase".into(); }
                10 => { cells[0].starts_at_ms += 1; }
                11 => { cells[0].ends_at_ms -= 1; }
                12 => { cells[0].workload_profile_digest = Digest::from_byte(99); }
                13 => { cells[0].phase_schedule_digest = Digest::from_byte(99); }
                14 => { cells[0].demand_contract_digest = Digest::from_byte(99); }
                15 => { cells[0].pseudonym_key_epoch = "rolled-key-epoch".into(); }
                16 => { cells[0].offered_requests = 99; }
                17 => { cells[0].offered_requests = 101; }
                18 => { cells[1].offered_requests = 99; }
                19 => { cells[1].offered_requests = 101; }
                20 => { cells[0].observer_source_id = "substituted-source".into(); }
                21 => { cells[1].observer_source_id = "substituted-source".into(); }
                22 => { cells[0].observer_control_domain = "agent-domain".into(); }
                23 => { cells[1].observer_control_domain = "agent-domain".into(); }
                24 => { cells[0].receipt_issued_at_ms = 899; }
                25 => { cells[0].receipt_issued_at_ms = 1_001; }
                26 => { cells[1].receipt_issued_at_ms = 899; }
                27 => { cells[0].complete = false; }
                28 => { cells[1].complete = false; }
                29 => { cells[0].enumeration_complete = false; }
                30 => { cells[1].enumeration_complete = false; }
                31 => { cells[0].events.push(ImpactEvent { request_token: "not-a-keyed-token".into(), kind: ImpactKind::Failed }); }
                32 => {
                    cells[0].offered_requests = 2;
                    cells[1].offered_requests = 2;
                    cells[0].events = (0..3).map(|n| ImpactEvent { request_token: token(n), kind: ImpactKind::Failed }).collect();
                }
                33 => {
                    spec.maximum_event_rows = 1;
                    cells[0].events = vec![
                        ImpactEvent { request_token: token(1), kind: ImpactKind::Failed },
                        ImpactEvent { request_token: token(2), kind: ImpactKind::Slow },
                    ];
                }
                34 => { spec.expected_observer_control_domain = spec.executor_control_domain.clone(); }
                35 => { spec.expected_observer_source_id.clear(); }
                36 => { spec.pseudonym_key_epoch.clear(); }
                37 => { spec.demand.minimum_offered_requests = 101; }
                38 => { spec.starts_at_ms = spec.ends_at_ms; }
                39 => { cells.push(ObservationCell { pair_id: "extra-cell".into(), ..cells[0].clone() }); }
                40 => { cells[1].scenario_id = "other-scenario".into(); }
                41 => { cells[1].service = "other-service".into(); }
                42 => { cells[1].request_class = "other-class".into(); }
                43 => { cells[1].phase_schedule_digest = Digest::from_byte(88); }
                44 => { cells[1].demand_contract_digest = Digest::from_byte(88); }
                45 => { cells[1].pseudonym_key_epoch = "other-epoch".into(); }
                46 => { cells[1].observer_control_domain = "substituted-domain".into(); }
                _ => { cells[1].receipt_issued_at_ms = 500; }
            }
            let result = evaluate_paired_impact(&[spec], &cells);
            match result {
                Err(_) => {}
                Ok(summary) => {
                    assert!(
                        summary.complete_candidate_minus_no_action_request_ms.is_none(),
                        "adversarial vector {vector} was incorrectly treated as complete"
                    );
                    assert!(
                        summary.coverage.indeterminate_pairs > 0
                            || summary.coverage.unmatched_observations > 0,
                        "adversarial vector {vector} produced no visible coverage failure"
                    );
                }
            }
        }
    }
}
