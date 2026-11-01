//! Event-level degradation interval unions (v0.1).
//!
//! Measures observed degraded request-time inside a declared window. It never
//! multiplies a request count by the whole window duration. All intervals are
//! half-open [start, end), and overlapping/adjacent categories are merged once
//! per keyed request token. Right-censored intervals are clipped at the window
//! end and reported; left-censored intervals are indeterminate.

use super::Digest;
use super::impact::{validate_pair_contract, ImpactKind, ObservationArm, PairCellContract};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntervalClockContract {
    pub source_id: String,
    pub clock_epoch: String,
    pub mapping_digest: Digest,
    pub maximum_uncertainty_ms: u64,
    pub required_timestamp_resolution_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntervalPairContract {
    pub pair: PairCellContract,
    pub clock: IntervalClockContract,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DegradationIntervalEvent {
    /// Unique observer event/receipt row identifier within this cell.
    pub event_id: String,
    /// 32-byte keyed HMAC pseudonym, lower-case hexadecimal; never a plain hash.
    pub request_token: String,
    pub kind: ImpactKind,
    /// Inclusive start; event intervals are interpreted as [start, end).
    pub starts_at_ms: u64,
    /// Exclusive recovery boundary. None means still degraded at window end.
    pub ends_at_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntervalObservationCell {
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
    pub complete: bool,
    pub enumeration_complete: bool,
    pub clock_source_id: String,
    pub clock_epoch: String,
    pub clock_mapping_digest: Digest,
    pub observed_clock_uncertainty_ms: u64,
    pub timestamp_resolution_ms: u64,
    pub events: Vec<DegradationIntervalEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntervalImpactMetrics {
    pub offered_requests: u64,
    pub failed_requests: u64,
    pub slow_requests: u64,
    pub incorrect_business_requests: u64,
    pub unique_degraded_requests: u64,
    /// Sum of per-request interval unions, clipped to the exact observation window.
    pub observed_degraded_request_ms: u128,
    /// Longest single continuous degraded episode for any one request.
    pub longest_degraded_episode_ms: u64,
    /// Requests with at least one interval still open at the observation boundary.
    pub right_censored_requests: u64,
    /// Number of input interval rows merged away because they overlapped or touched.
    pub merged_interval_rows: u64,
    pub impact_rate_ppm: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntervalFailure {
    InvalidPairContract,
    MissingCandidateCell,
    MissingNoActionCell,
    DuplicateCandidateCell,
    DuplicateNoActionCell,
    UnmatchedObservation,
    ScenarioMismatch,
    ClusterMismatch,
    ServiceMismatch,
    RequestClassMismatch,
    PhaseMismatch,
    WindowMismatch,
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
    DuplicateEventId,
    LeftCensoredInterval,
    IntervalStartsOutsideWindow,
    InvalidIntervalOrder,
    IntervalEndsOutsideWindow,
    ClockSourceMismatch,
    ClockEpochMismatch,
    ClockMappingMismatch,
    ClockUncertaintyExceeded,
    TimestampResolutionMismatch,
    OfferedDemandOutsideContract,
    OfferedDemandMismatch,
    UniqueUnionExceedsOffered,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairedIntervalResult {
    pub pair_id: String,
    pub cluster_id: String,
    pub failures: Vec<IntervalFailure>,
    pub candidate: Option<IntervalImpactMetrics>,
    pub no_action: Option<IntervalImpactMetrics>,
    /// Candidate minus no-action; positive means more affected requests.
    pub candidate_minus_no_action_requests: Option<i128>,
    /// Candidate minus no-action; positive means more observed degraded request-ms.
    pub candidate_minus_no_action_request_ms: Option<i128>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntervalImpactCoverage {
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
pub struct IntervalImpactSummary {
    pub coverage: IntervalImpactCoverage,
    /// Present only if every preregistered pair has exactly one valid cell per arm.
    pub complete_candidate_minus_no_action_requests: Option<i128>,
    pub complete_candidate_minus_no_action_request_ms: Option<i128>,
    pub pairs: Vec<PairedIntervalResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntervalImpactError {
    EmptyPairManifest,
    DuplicatePairId(String),
    InvalidPairContract(String),
    ArithmeticOverflow,
}

pub fn evaluate_paired_interval_impact(
    contracts: &[IntervalPairContract],
    observations: &[IntervalObservationCell],
) -> Result<IntervalImpactSummary, IntervalImpactError> {
    if contracts.is_empty() {
        return Err(IntervalImpactError::EmptyPairManifest);
    }

    let mut contract_index = BTreeMap::new();
    for (index, contract) in contracts.iter().enumerate() {
        validate_pair_contract(&contract.pair)
            .map_err(|_| IntervalImpactError::InvalidPairContract(contract.pair.pair_id.clone()))?;
        if !validate_clock_contract(&contract.clock) {
            return Err(IntervalImpactError::InvalidPairContract(contract.pair.pair_id.clone()));
        }
        if contract_index.insert(contract.pair.pair_id.as_str(), index).is_some() {
            return Err(IntervalImpactError::DuplicatePairId(contract.pair.pair_id.clone()));
        }
    }

    let mut cells: BTreeMap<(&str, ObservationArm), Vec<&IntervalObservationCell>> = BTreeMap::new();
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
    let mut valid_pairs = 0usize;
    let mut duplicate_candidate_cells = 0usize;
    let mut duplicate_no_action_cells = 0usize;
    let mut missing_candidate_cells = 0usize;
    let mut missing_no_action_cells = 0usize;
    let mut total_request_delta = 0i128;
    let mut total_request_ms_delta = 0i128;
    let mut arithmetic_ok = true;

    for contract in contracts {
        let candidate_cells = cells.get(&(contract.pair.pair_id.as_str(), ObservationArm::Candidate))
            .map(Vec::as_slice).unwrap_or(&[]);
        let no_action_cells = cells.get(&(contract.pair.pair_id.as_str(), ObservationArm::NoAction))
            .map(Vec::as_slice).unwrap_or(&[]);
        let mut failures = BTreeSet::new();

        if candidate_cells.is_empty() {
            failures.insert(IntervalFailure::MissingCandidateCell);
            missing_candidate_cells += 1;
        } else if candidate_cells.len() > 1 {
            failures.insert(IntervalFailure::DuplicateCandidateCell);
            duplicate_candidate_cells += candidate_cells.len() - 1;
        }
        if no_action_cells.is_empty() {
            failures.insert(IntervalFailure::MissingNoActionCell);
            missing_no_action_cells += 1;
        } else if no_action_cells.len() > 1 {
            failures.insert(IntervalFailure::DuplicateNoActionCell);
            duplicate_no_action_cells += no_action_cells.len() - 1;
        }

        let candidate = if candidate_cells.len() == 1 {
            match measure_interval_cell(contract, candidate_cells[0]) {
                Ok(metrics) => Some(metrics),
                Err(failure) => { failures.insert(failure); None }
            }
        } else { None };
        let no_action = if no_action_cells.len() == 1 {
            match measure_interval_cell(contract, no_action_cells[0]) {
                Ok(metrics) => Some(metrics),
                Err(failure) => { failures.insert(failure); None }
            }
        } else { None };

        let deltas = match (&candidate, &no_action) {
            (Some(c), Some(n)) if c.offered_requests == n.offered_requests => {
                let requests = i128::from(c.unique_degraded_requests)
                    .checked_sub(i128::from(n.unique_degraded_requests));
                let request_ms = i128::try_from(c.observed_degraded_request_ms)
                    .ok()
                    .and_then(|cv| i128::try_from(n.observed_degraded_request_ms).ok()
                        .and_then(|nv| cv.checked_sub(nv)));
                match (requests, request_ms) {
                    (Some(a), Some(b)) if failures.is_empty() => Some((a, b)),
                    _ => {
                        if requests.is_none() || request_ms.is_none() {
                            failures.insert(IntervalFailure::ArithmeticOverflow);
                        }
                        None
                    }
                }
            }
            (Some(_), Some(_)) => {
                failures.insert(IntervalFailure::OfferedDemandMismatch);
                None
            }
            _ => None,
        };

        let (request_delta, request_ms_delta) = match deltas {
            Some((a, b)) => {
                match total_request_delta.checked_add(a) {
                    Some(v) => total_request_delta = v,
                    None => arithmetic_ok = false,
                }
                match total_request_ms_delta.checked_add(b) {
                    Some(v) => total_request_ms_delta = v,
                    None => arithmetic_ok = false,
                }
                valid_pairs += 1;
                (Some(a), Some(b))
            }
            None => (None, None),
        };

        pairs.push(PairedIntervalResult {
            pair_id: contract.pair.pair_id.clone(),
            cluster_id: contract.pair.cluster_id.clone(),
            failures: failures.into_iter().collect(),
            candidate,
            no_action,
            candidate_minus_no_action_requests: request_delta,
            candidate_minus_no_action_request_ms: request_ms_delta,
        });
    }

    let indeterminate_pairs = pairs.len() - valid_pairs;
    let complete = indeterminate_pairs == 0 && unmatched_observations == 0
        && duplicate_candidate_cells == 0 && duplicate_no_action_cells == 0 && arithmetic_ok;

    Ok(IntervalImpactSummary {
        coverage: IntervalImpactCoverage {
            expected_pairs: contracts.len(),
            valid_pairs,
            indeterminate_pairs,
            unmatched_observations,
            duplicate_candidate_cells,
            duplicate_no_action_cells,
            missing_candidate_cells,
            missing_no_action_cells,
        },
        complete_candidate_minus_no_action_requests: if complete { Some(total_request_delta) } else { None },
        complete_candidate_minus_no_action_request_ms: if complete { Some(total_request_ms_delta) } else { None },
        pairs,
    })
}

fn validate_clock_contract(clock: &IntervalClockContract) -> bool {
    if clock.source_id.trim().is_empty()
        || clock.clock_epoch.trim().is_empty()
        || clock.maximum_uncertainty_ms == 0
        || clock.required_timestamp_resolution_ms == 0
        || digest_is_zero(&clock.mapping_digest)
    {
        return false;
    }
    true
}

fn measure_interval_cell(
    contract: &IntervalPairContract,
    cell: &IntervalObservationCell,
) -> Result<IntervalImpactMetrics, IntervalFailure> {
    let pair = &contract.pair;
    if cell.scenario_id != pair.scenario_id { return Err(IntervalFailure::ScenarioMismatch); }
    if cell.pair_id != pair.pair_id { return Err(IntervalFailure::WindowMismatch); }
    if cell.cluster_id != pair.cluster_id { return Err(IntervalFailure::ClusterMismatch); }
    if cell.service != pair.service { return Err(IntervalFailure::ServiceMismatch); }
    if cell.request_class != pair.request_class { return Err(IntervalFailure::RequestClassMismatch); }
    if cell.phase != pair.phase { return Err(IntervalFailure::PhaseMismatch); }
    if cell.starts_at_ms != pair.starts_at_ms || cell.ends_at_ms != pair.ends_at_ms {
        return Err(IntervalFailure::WindowMismatch);
    }
    if cell.workload_profile_digest != pair.workload_profile_digest {
        return Err(IntervalFailure::WorkloadProfileMismatch);
    }
    if cell.phase_schedule_digest != pair.phase_schedule_digest {
        return Err(IntervalFailure::PhaseScheduleMismatch);
    }
    if cell.demand_contract_digest != pair.demand.digest {
        return Err(IntervalFailure::DemandContractMismatch);
    }
    if cell.pseudonym_key_epoch != pair.pseudonym_key_epoch {
        return Err(IntervalFailure::PseudonymKeyEpochMismatch);
    }
    if cell.observer_source_id != pair.expected_observer_source_id {
        return Err(IntervalFailure::ObserverSourceMismatch);
    }
    if cell.observer_control_domain != pair.expected_observer_control_domain {
        return Err(IntervalFailure::ObserverControlDomainMismatch);
    }
    if cell.observer_control_domain == pair.executor_control_domain {
        return Err(IntervalFailure::ObserverExecutorDomainCollision);
    }
    if cell.receipt_issued_at_ms > pair.evaluation_time_ms {
        return Err(IntervalFailure::ReceiptFromFuture);
    }
    if pair.evaluation_time_ms - cell.receipt_issued_at_ms > pair.maximum_receipt_age_ms {
        return Err(IntervalFailure::ReceiptStale);
    }
    if !cell.complete { return Err(IntervalFailure::IncompleteObservation); }
    if !cell.enumeration_complete { return Err(IntervalFailure::IncompleteEnumeration); }
    if cell.events.len() > pair.maximum_event_rows { return Err(IntervalFailure::TooManyEventRows); }
    if cell.offered_requests < pair.demand.minimum_offered_requests
        || cell.offered_requests > pair.demand.maximum_offered_requests
    {
        return Err(IntervalFailure::OfferedDemandOutsideContract);
    }

    if cell.clock_source_id != contract.clock.source_id {
        return Err(IntervalFailure::ClockSourceMismatch);
    }
    if cell.clock_epoch != contract.clock.clock_epoch {
        return Err(IntervalFailure::ClockEpochMismatch);
    }
    if cell.clock_mapping_digest != contract.clock.mapping_digest {
        return Err(IntervalFailure::ClockMappingMismatch);
    }
    if cell.observed_clock_uncertainty_ms > contract.clock.maximum_uncertainty_ms {
        return Err(IntervalFailure::ClockUncertaintyExceeded);
    }
    if cell.timestamp_resolution_ms != contract.clock.required_timestamp_resolution_ms {
        return Err(IntervalFailure::TimestampResolutionMismatch);
    }

    let mut event_ids = BTreeSet::new();
    let mut by_request: BTreeMap<&str, Vec<(u64, u64)>> = BTreeMap::new();
    let mut failed = BTreeSet::new();
    let mut slow = BTreeSet::new();
    let mut incorrect = BTreeSet::new();
    let mut right_censored = BTreeSet::new();

    for event in &cell.events {
        if event.event_id.trim().is_empty() || !event_ids.insert(event.event_id.as_str()) {
            return Err(IntervalFailure::DuplicateEventId);
        }
        if !is_keyed_pseudonym_token(&event.request_token) {
            return Err(IntervalFailure::InvalidRequestToken);
        }
        if event.starts_at_ms < pair.starts_at_ms {
            return Err(IntervalFailure::LeftCensoredInterval);
        }
        if event.starts_at_ms >= pair.ends_at_ms {
            return Err(IntervalFailure::IntervalStartsOutsideWindow);
        }
        let end = match event.ends_at_ms {
            Some(end) => {
                if end <= event.starts_at_ms {
                    return Err(IntervalFailure::InvalidIntervalOrder);
                }
                if end > pair.ends_at_ms {
                    return Err(IntervalFailure::IntervalEndsOutsideWindow);
                }
                end
            }
            None => {
                right_censored.insert(event.request_token.as_str());
                pair.ends_at_ms
            }
        };
        by_request.entry(event.request_token.as_str()).or_default()
            .push((event.starts_at_ms, end));
        match event.kind {
            ImpactKind::Failed => { failed.insert(event.request_token.as_str()); }
            ImpactKind::Slow => { slow.insert(event.request_token.as_str()); }
            ImpactKind::IncorrectBusinessResult => { incorrect.insert(event.request_token.as_str()); }
        }
    }

    let unique_requests = u64::try_from(by_request.len())
        .map_err(|_| IntervalFailure::ArithmeticOverflow)?;
    if unique_requests > cell.offered_requests {
        return Err(IntervalFailure::UniqueUnionExceedsOffered);
    }

    let mut degraded_request_ms = 0u128;
    let mut longest_episode_ms = 0u64;
    let mut merged_interval_rows = 0u64;
    for intervals in by_request.values_mut() {
        intervals.sort_unstable();
        let mut iter = intervals.iter().copied();
        let Some((mut current_start, mut current_end)) = iter.next() else { continue; };
        for (start, end) in iter {
            // Merge overlap AND adjacency. An affected request cannot accumulate
            // multiple simultaneous category durations for the same milliseconds.
            if start <= current_end {
                current_end = current_end.max(end);
                merged_interval_rows = merged_interval_rows.checked_add(1)
                    .ok_or(IntervalFailure::ArithmeticOverflow)?;
            } else {
                let duration = current_end.checked_sub(current_start)
                    .ok_or(IntervalFailure::ArithmeticOverflow)?;
                degraded_request_ms = degraded_request_ms.checked_add(duration as u128)
                    .ok_or(IntervalFailure::ArithmeticOverflow)?;
                longest_episode_ms = longest_episode_ms.max(duration);
                current_start = start;
                current_end = end;
            }
        }
        let duration = current_end.checked_sub(current_start)
            .ok_or(IntervalFailure::ArithmeticOverflow)?;
        degraded_request_ms = degraded_request_ms.checked_add(duration as u128)
            .ok_or(IntervalFailure::ArithmeticOverflow)?;
        longest_episode_ms = longest_episode_ms.max(duration);
    }

    let impact_rate_ppm = (unique_requests as u128)
        .checked_mul(1_000_000).ok_or(IntervalFailure::ArithmeticOverflow)?
        .checked_div(cell.offered_requests as u128).ok_or(IntervalFailure::ArithmeticOverflow)?;
    let impact_rate_ppm = u64::try_from(impact_rate_ppm)
        .map_err(|_| IntervalFailure::ArithmeticOverflow)?;

    Ok(IntervalImpactMetrics {
        offered_requests: cell.offered_requests,
        failed_requests: u64::try_from(failed.len()).map_err(|_| IntervalFailure::ArithmeticOverflow)?,
        slow_requests: u64::try_from(slow.len()).map_err(|_| IntervalFailure::ArithmeticOverflow)?,
        incorrect_business_requests: u64::try_from(incorrect.len()).map_err(|_| IntervalFailure::ArithmeticOverflow)?,
        unique_degraded_requests: unique_requests,
        observed_degraded_request_ms: degraded_request_ms,
        longest_degraded_episode_ms: longest_episode_ms,
        right_censored_requests: u64::try_from(right_censored.len()).map_err(|_| IntervalFailure::ArithmeticOverflow)?,
        merged_interval_rows,
        impact_rate_ppm,
    })
}

fn is_keyed_pseudonym_token(token: &str) -> bool {
    token.len() == 64
        && token.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn digest_is_zero(digest: &Digest) -> bool {
    digest.0.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(value: u8) -> String { format!("{:02x}", value).repeat(32) }

    fn contract() -> IntervalPairContract {
        IntervalPairContract {
            pair: PairCellContract {
                pair_id: "pair-interval-001".into(),
                scenario_id: "scenario-001".into(),
                cluster_id: "cluster-001".into(),
                service: "checkout".into(),
                request_class: "critical-write".into(),
                phase: "fault-and-recovery".into(),
                starts_at_ms: 100,
                ends_at_ms: 300,
                workload_profile_digest: Digest::from_byte(1),
                phase_schedule_digest: Digest::from_byte(2),
                demand: super::super::impact::DemandContract {
                    digest: Digest::from_byte(3),
                    minimum_offered_requests: 100,
                    maximum_offered_requests: 100,
                },
                pseudonym_key_epoch: "hmac-epoch-1".into(),
                expected_observer_source_id: "ingress-a".into(),
                expected_observer_control_domain: "observer-domain".into(),
                executor_control_domain: "agent-domain".into(),
                evaluation_time_ms: 1_000,
                maximum_receipt_age_ms: 100,
                maximum_event_rows: 100,
            },
            clock: IntervalClockContract {
                source_id: "ptp-source-a".into(),
                clock_epoch: "clock-epoch-1".into(),
                mapping_digest: Digest::from_byte(4),
                maximum_uncertainty_ms: 2,
                required_timestamp_resolution_ms: 1,
            },
        }
    }

    fn cell(arm: ObservationArm) -> IntervalObservationCell {
        let c = contract();
        IntervalObservationCell {
            pair_id: c.pair.pair_id,
            arm,
            scenario_id: c.pair.scenario_id,
            cluster_id: c.pair.cluster_id,
            service: c.pair.service,
            request_class: c.pair.request_class,
            phase: c.pair.phase,
            starts_at_ms: c.pair.starts_at_ms,
            ends_at_ms: c.pair.ends_at_ms,
            workload_profile_digest: c.pair.workload_profile_digest,
            phase_schedule_digest: c.pair.phase_schedule_digest,
            demand_contract_digest: c.pair.demand.digest,
            offered_requests: 100,
            pseudonym_key_epoch: c.pair.pseudonym_key_epoch,
            observer_source_id: c.pair.expected_observer_source_id,
            observer_control_domain: c.pair.expected_observer_control_domain,
            receipt_issued_at_ms: 990,
            complete: true,
            enumeration_complete: true,
            clock_source_id: c.clock.source_id,
            clock_epoch: c.clock.clock_epoch,
            clock_mapping_digest: c.clock.mapping_digest,
            observed_clock_uncertainty_ms: 1,
            timestamp_resolution_ms: 1,
            events: Vec::new(),
        }
    }

    fn pair() -> (IntervalPairContract, Vec<IntervalObservationCell>) {
        (contract(), vec![cell(ObservationArm::Candidate), cell(ObservationArm::NoAction)])
    }

    fn ev(id: &str, token_value: u8, kind: ImpactKind, start: u64, end: Option<u64>) -> DegradationIntervalEvent {
        DegradationIntervalEvent {
            event_id: id.into(),
            request_token: token(token_value),
            kind,
            starts_at_ms: start,
            ends_at_ms: end,
        }
    }

    #[test]
    fn overlapping_cross_category_intervals_merge_once() {
        let (spec, mut cells) = pair();
        cells[0].events = vec![
            ev("a", 1, ImpactKind::Failed, 110, Some(180)),
            ev("b", 1, ImpactKind::Slow, 150, Some(220)),
            ev("c", 1, ImpactKind::IncorrectBusinessResult, 200, Some(240)),
        ];
        let result = evaluate_paired_interval_impact(&[spec], &cells).unwrap();
        let m = result.pairs[0].candidate.as_ref().unwrap();
        assert_eq!(m.unique_degraded_requests, 1);
        assert_eq!(m.observed_degraded_request_ms, 130);
        assert_eq!(m.longest_degraded_episode_ms, 130);
        assert_eq!(m.merged_interval_rows, 2);
    }

    #[test]
    fn disjoint_episodes_for_one_request_preserve_healthy_gap() {
        let (spec, mut cells) = pair();
        cells[0].events = vec![
            ev("a", 1, ImpactKind::Failed, 110, Some(130)),
            ev("b", 1, ImpactKind::Slow, 150, Some(175)),
        ];
        let result = evaluate_paired_interval_impact(&[spec], &cells).unwrap();
        let m = result.pairs[0].candidate.as_ref().unwrap();
        assert_eq!(m.observed_degraded_request_ms, 45);
        assert_eq!(m.longest_degraded_episode_ms, 25);
        assert_eq!(m.merged_interval_rows, 0);
    }

    #[test]
    fn adjacent_intervals_merge_without_double_count() {
        let (spec, mut cells) = pair();
        cells[0].events = vec![
            ev("a", 1, ImpactKind::Failed, 110, Some(150)),
            ev("b", 1, ImpactKind::Slow, 150, Some(170)),
        ];
        let result = evaluate_paired_interval_impact(&[spec], &cells).unwrap();
        let m = result.pairs[0].candidate.as_ref().unwrap();
        assert_eq!(m.observed_degraded_request_ms, 60);
        assert_eq!(m.longest_degraded_episode_ms, 60);
        assert_eq!(m.merged_interval_rows, 1);
    }

    #[test]
    fn right_censored_interval_is_clipped_and_reported() {
        let (spec, mut cells) = pair();
        cells[0].events.push(ev("active", 1, ImpactKind::Failed, 250, None));
        let result = evaluate_paired_interval_impact(&[spec], &cells).unwrap();
        let m = result.pairs[0].candidate.as_ref().unwrap();
        assert_eq!(m.observed_degraded_request_ms, 50);
        assert_eq!(m.right_censored_requests, 1);
    }

    #[test]
    fn failed_request_with_explicit_recovery_at_boundary_is_not_right_censored() {
        let (spec, mut cells) = pair();
        cells[0].events.push(ev("recovered", 1, ImpactKind::Failed, 250, Some(300)));
        let result = evaluate_paired_interval_impact(&[spec], &cells).unwrap();
        assert_eq!(result.pairs[0].candidate.as_ref().unwrap().right_censored_requests, 0);
        assert_eq!(result.pairs[0].candidate.as_ref().unwrap().observed_degraded_request_ms, 50);
    }

    #[test]
    fn exact_pair_emits_both_count_and_duration_deltas() {
        let (spec, mut cells) = pair();
        cells[0].events.push(ev("c", 1, ImpactKind::Failed, 110, Some(130)));
        cells[1].events.push(ev("n", 2, ImpactKind::Failed, 110, Some(180)));
        let result = evaluate_paired_interval_impact(&[spec], &cells).unwrap();
        assert_eq!(result.complete_candidate_minus_no_action_requests, Some(0));
        assert_eq!(result.complete_candidate_minus_no_action_request_ms, Some(-50));
    }

    #[test]
    fn independent_union_oracle_matches_interval_sweep() {
        let (spec, mut cells) = pair();
        cells[0].events = vec![
            ev("a", 1, ImpactKind::Failed, 101, Some(115)),
            ev("b", 1, ImpactKind::Slow, 112, Some(140)),
            ev("c", 1, ImpactKind::IncorrectBusinessResult, 150, Some(160)),
            ev("d", 2, ImpactKind::Failed, 100, Some(125)),
            ev("e", 2, ImpactKind::Slow, 120, Some(140)),
        ];
        // Oracle uses a per-millisecond occupancy grid; intentionally independent
        // from the evaluator's sorted interval-merging implementation.
        let mut expected = 0u128;
        for request in 0u8..=2 {
            for t in 100u64..300 {
                let active = cells[0].events.iter().any(|event| {
                    event.request_token == token(request)
                        && event.starts_at_ms <= t
                        && event.ends_at_ms.map(|end| t < end).unwrap_or(true)
                });
                if active { expected += 1; }
            }
        }
        let result = evaluate_paired_interval_impact(&[spec], &cells).unwrap();
        assert_eq!(result.pairs[0].candidate.as_ref().unwrap().observed_degraded_request_ms, expected);
    }

    #[test]
    fn left_censored_and_out_of_window_intervals_fail_closed() {
        let (spec, mut cells) = pair();
        cells[0].events.push(ev("left", 1, ImpactKind::Failed, 99, Some(120)));
        let r = evaluate_paired_interval_impact(&[spec.clone()], &cells).unwrap();
        assert_eq!(r.pairs[0].failures, vec![IntervalFailure::LeftCensoredInterval]);
        let mut cells = pair().1;
        cells[0].events.push(ev("outside", 1, ImpactKind::Failed, 280, Some(301)));
        let r = evaluate_paired_interval_impact(&[spec], &cells).unwrap();
        assert_eq!(r.pairs[0].failures, vec![IntervalFailure::IntervalEndsOutsideWindow]);
    }

    #[test]
    fn duplicate_ids_clock_uncertainty_and_domain_substitution_fail_closed() {
        let (spec, mut cells) = pair();
        cells[0].events = vec![
            ev("same", 1, ImpactKind::Failed, 110, Some(120)),
            ev("same", 2, ImpactKind::Failed, 130, Some(140)),
        ];
        let r = evaluate_paired_interval_impact(&[spec.clone()], &cells).unwrap();
        assert_eq!(r.pairs[0].failures, vec![IntervalFailure::DuplicateEventId]);
        let mut cells = pair().1;
        cells[0].observed_clock_uncertainty_ms = 3;
        let r = evaluate_paired_interval_impact(&[spec.clone()], &cells).unwrap();
        assert_eq!(r.pairs[0].failures, vec![IntervalFailure::ClockUncertaintyExceeded]);
        let mut cells = pair().1;
        cells[1].clock_epoch = "substituted-epoch".into();
        let r = evaluate_paired_interval_impact(&[spec], &cells).unwrap();
        assert_eq!(r.pairs[0].failures, vec![IntervalFailure::ClockEpochMismatch]);
    }

    #[test]
    fn missing_or_duplicate_paired_cells_suppress_aggregate() {
        let (spec, mut cells) = pair();
        cells.pop();
        let r = evaluate_paired_interval_impact(&[spec.clone()], &cells).unwrap();
        assert_eq!(r.complete_candidate_minus_no_action_request_ms, None);
        let mut cells = pair().1;
        let duplicate = cells[0].clone();
        cells.push(duplicate);
        let r = evaluate_paired_interval_impact(&[spec], &cells).unwrap();
        assert_eq!(r.coverage.duplicate_candidate_cells, 1);
        assert_eq!(r.complete_candidate_minus_no_action_request_ms, None);
    }

    #[test]
    fn sixty_four_adversarial_interval_mutations_fail_closed() {
        for vector in 0..64 {
            let (mut spec, mut cells) = pair();
            match vector {
                0 => { cells[0].pair_id = "unknown".into(); }
                1 => { cells[0].scenario_id = "other".into(); }
                2 => { cells[0].cluster_id = "other".into(); }
                3 => { cells[0].service = "other".into(); }
                4 => { cells[0].request_class = "other".into(); }
                5 => { cells[0].phase = "other".into(); }
                6 => { cells[0].starts_at_ms += 1; }
                7 => { cells[0].ends_at_ms -= 1; }
                8 => { cells[0].workload_profile_digest = Digest::from_byte(9); }
                9 => { cells[0].phase_schedule_digest = Digest::from_byte(9); }
                10 => { cells[0].demand_contract_digest = Digest::from_byte(9); }
                11 => { cells[0].pseudonym_key_epoch = "other".into(); }
                12 => { cells[0].observer_source_id = "other".into(); }
                13 => { cells[0].observer_control_domain = "agent-domain".into(); }
                14 => { cells[0].receipt_issued_at_ms = 1_001; }
                15 => { cells[0].receipt_issued_at_ms = 899; }
                16 => { cells[0].complete = false; }
                17 => { cells[0].enumeration_complete = false; }
                18 => { cells[0].offered_requests = 99; }
                19 => { cells[0].offered_requests = 101; }
                20 => { cells[0].clock_source_id = "other".into(); }
                21 => { cells[0].clock_epoch = "other".into(); }
                22 => { cells[0].clock_mapping_digest = Digest::from_byte(99); }
                23 => { cells[0].observed_clock_uncertainty_ms = 3; }
                24 => { cells[0].timestamp_resolution_ms = 2; }
                25 => { cells[0].events.push(ev("bad-token", 1, ImpactKind::Failed, 110, Some(120))); cells[0].events[0].request_token = "ab".into(); }
                26 => { cells[0].events.push(ev("left", 1, ImpactKind::Failed, 99, Some(120))); }
                27 => { cells[0].events.push(ev("start-end", 1, ImpactKind::Failed, 300, None)); }
                28 => { cells[0].events.push(ev("zero", 1, ImpactKind::Failed, 120, Some(120))); }
                29 => { cells[0].events.push(ev("reverse", 1, ImpactKind::Failed, 130, Some(120))); }
                30 => { cells[0].events.push(ev("too-late", 1, ImpactKind::Failed, 280, Some(301))); }
                31 => { cells[0].events.push(ev("empty-id", 1, ImpactKind::Failed, 110, Some(120))); cells[0].events[0].event_id.clear(); }
                32 => { cells[0].events = vec![ev("same", 1, ImpactKind::Failed, 110, Some(120)), ev("same", 2, ImpactKind::Slow, 130, Some(140))]; }
                33 => { cells[0].events = (0..101).map(|n| ev(&format!("row-{n}"), n as u8, ImpactKind::Failed, 110, Some(120))).collect(); }
                34 => { spec.pair.starts_at_ms = spec.pair.ends_at_ms; }
                35 => { spec.pair.demand.minimum_offered_requests = 101; }
                36 => { spec.pair.expected_observer_control_domain = spec.pair.executor_control_domain.clone(); }
                37 => { spec.clock.source_id.clear(); }
                38 => { spec.clock.clock_epoch.clear(); }
                39 => { spec.clock.maximum_uncertainty_ms = 0; }
                40 => { spec.clock.required_timestamp_resolution_ms = 0; }
                41 => { spec.clock.mapping_digest = Digest::from_byte(0); }
                42 => { cells[1].cluster_id = "other".into(); }
                43 => { cells[1].service = "other".into(); }
                44 => { cells[1].phase = "other".into(); }
                45 => { cells[1].workload_profile_digest = Digest::from_byte(8); }
                46 => { cells[1].phase_schedule_digest = Digest::from_byte(8); }
                47 => { cells[1].clock_source_id = "other".into(); }
                48 => { cells[1].clock_epoch = "other".into(); }
                49 => { cells[1].clock_mapping_digest = Digest::from_byte(8); }
                50 => { cells[1].observed_clock_uncertainty_ms = 3; }
                51 => { cells[1].timestamp_resolution_ms = 4; }
                52 => { cells[1].receipt_issued_at_ms = 899; }
                53 => { cells[1].enumeration_complete = false; }
                54 => { cells[0].events.push(ev("future", 1, ImpactKind::Failed, 300, None)); }
                55 => { cells[0].events.push(ev("late-start", 1, ImpactKind::Failed, 301, Some(302))); }
                56 => { cells[0].events.push(ev("zero-end", 1, ImpactKind::Failed, 150, Some(100))); }
                57 => { cells[0].events = vec![ev("dup", 1, ImpactKind::Failed, 110, Some(120)), ev("dup", 1, ImpactKind::Slow, 110, Some(120))]; }
                58 => { cells[0].events.push(ev("outside-end", 1, ImpactKind::Failed, 280, Some(301))); }
                59 => { cells[0].events.push(ev("upper-token", 1, ImpactKind::Failed, 110, Some(120))); cells[0].events[0].request_token = "AA".repeat(32); }
                60 => { cells[0].events = (0..101).map(|n| ev(&format!("u-{n}"), n as u8, ImpactKind::Failed, 110, Some(120))).collect(); }
                61 => { cells[0].observer_source_id.clear(); }
                62 => { cells[0].clock_source_id.clear(); }
                _ => { cells[0].pseudonym_key_epoch = "rotated-epoch".into(); }
            }
            let result = evaluate_paired_interval_impact(&[spec], &cells).unwrap();
            assert!(
                result.complete_candidate_minus_no_action_request_ms.is_none()
                    || result.coverage.indeterminate_pairs > 0,
                "interval adversarial vector {vector} was accepted as complete"
            );
        }
    }
}