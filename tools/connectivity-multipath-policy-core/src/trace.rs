//! Deterministic packet-trace scheduling model for multipath comparison.
//!
//! This is an offline discrete-event model, not a packet transport. It uses
//! caller-supplied path metadata, fixed-size stream segments, per-path
//! serialization queues, deterministic loss keyed by declared failure domain,
//! and receiver-side in-order reassembly accounting. It does not model TCP/QUIC
//! congestion-control feedback, ACK clocks, retransmission, pacing, kernel
//! socket buffers, or real packet captures.

use super::{
    CapacityDomainEvidence, PathObservation, ScheduleError, ScheduleRequest,
    canonical_capacity_domain_ids, capacity_domains_disjoint, estimated_cost, exclusion_reason,
    validate_path_set, validate_request,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_TRACE_SEGMENTS: u32 = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceStrategy {
    /// Send every segment on the eligible path with the lowest RTT.
    MinRtt,
    /// Rotate eligible paths in stable path-ID order.
    RoundRobin,
    /// Select the path with the earliest modelled receiver arrival.
    EarliestCompletion,
    /// Prefer early arrival unless it would exceed the declared reorder buffer.
    ReorderAware,
    /// Send a copy over two distinct declared failure domains.
    RedundantDistinctDomains,
    /// Use reorder-aware scheduling for ordered/interactive work and
    /// earliest-completion scheduling for bulk work.
    IntentAware,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceRequest {
    pub segment_bytes: u32,
    pub deterministic_seed: u64,
    /// Maximum time to retain out-of-order bytes after the final observed arrival.
    pub reassembly_timeout_us: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceTransmission {
    pub segment_index: u32,
    pub path_id: String,
    pub send_at_us: u64,
    pub arrival_at_us: u64,
    pub payload_bytes: u32,
    pub lost: bool,
    pub redundant_copy: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathTraceSummary {
    pub path_id: String,
    pub failure_domain: String,
    pub capacity_domain_ids: Vec<String>,
    pub capacity_domain_evidence: CapacityDomainEvidence,
    pub transmitted_bytes: u64,
    pub transmissions: u64,
    pub lost_transmissions: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceReport {
    pub task_id: String,
    pub topology_sha256: String,
    pub decision_time_unix_ms: u64,
    pub strategy: TraceStrategy,
    pub total_segments: u32,
    pub scheduled_segments: u32,
    pub network_received_segments: u32,
    pub network_received_unique_bytes: u64,
    pub received_transmission_bytes: u64,
    pub delivered_segments: u32,
    pub failed_segments: u32,
    pub receiver_unreleased_buffered_bytes_at_timeout: u64,
    pub delivery_complete: bool,
    pub delivered_by_deadline: bool,
    pub completion_us: Option<u64>,
    pub network_arrival_p50_us: Option<u64>,
    pub network_arrival_p95_us: Option<u64>,
    pub network_arrival_p99_us: Option<u64>,
    pub application_release_p50_us: Option<u64>,
    pub application_release_p95_us: Option<u64>,
    pub application_release_p99_us: Option<u64>,
    pub total_bytes_transmitted: u64,
    pub useful_bytes_delivered: u64,
    pub redundant_bytes: u64,
    pub lost_transmissions: u64,
    pub receiver_reorder_peak_bytes: u64,
    pub receiver_reorder_byte_microseconds: u128,
    pub receiver_reorder_limit_exceeded: bool,
    pub estimated_cost_micros: u64,
    pub estimated_energy_uwh: u64,
    pub cost_currency: String,
    pub distinct_failure_domains_used: usize,
    pub distinct_capacity_domains_used: usize,
    pub capacity_domain_ids_used: Vec<String>,
    pub path_summaries: Vec<PathTraceSummary>,
    pub transmissions: Vec<TraceTransmission>,
    pub exclusions: Vec<(String, &'static str)>,
    pub limitations: Vec<&'static str>,
}

#[derive(Clone, Debug)]
struct PathState {
    path: PathObservation,
    ready_at_us: u64,
    bytes_used: u64,
    cost_used: u64,
    energy_used: u64,
    transmissions: u64,
    lost_transmissions: u64,
}

#[derive(Clone, Copy, Debug)]
struct CandidatePath {
    index: usize,
    send_at_us: u64,
    arrival_at_us: u64,
}

#[derive(Clone, Debug)]
struct SegmentDelivery {
    segment_index: u32,
    bytes: u32,
    first_arrival_us: u64,
    application_release_us: u64,
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn stable_hash(value: &str) -> u64 {
    value
        .as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
        })
}

fn is_lost(seed: u64, segment: u32, path: &PathObservation) -> bool {
    let domain_hash = stable_hash(&path.shared_risk_group);
    let sample =
        mix64(seed ^ (u64::from(segment).wrapping_mul(0x9e37_79b9_7f4a_7c15)) ^ domain_hash)
            % 1_000_000;
    sample < u64::from(path.loss_ppm)
}

fn serialization_us(bytes: u32, capacity_bps: u64) -> u64 {
    if capacity_bps == 0 {
        return u64::MAX;
    }
    let numerator = u128::from(bytes) * 8 * 1_000_000;
    let denominator = u128::from(capacity_bps);
    let duration = numerator.div_ceil(denominator);
    u64::try_from(duration).unwrap_or(u64::MAX)
}

fn candidate_path(state: &PathState, index: usize, bytes: u32) -> Option<CandidatePath> {
    let next_bytes = state.bytes_used.checked_add(u64::from(bytes))?;
    if next_bytes > state.path.data_remaining_bytes {
        return None;
    }
    let send_at = state.ready_at_us;
    let serialize = serialization_us(bytes, super::effective_capacity(&state.path));
    let arrival = send_at
        .saturating_add(serialize)
        .saturating_add(state.path.rtt_us)
        .saturating_add(state.path.jitter_us);
    Some(CandidatePath {
        index,
        send_at_us: send_at,
        arrival_at_us: arrival,
    })
}

fn candidate_increment(
    request: &ScheduleRequest,
    state: &PathState,
    bytes: u32,
    aggregate_cost_used: u64,
    aggregate_energy_used: u64,
) -> Option<(u64, u64)> {
    let next_bytes = state.bytes_used.checked_add(u64::from(bytes))?;
    if next_bytes > state.path.data_remaining_bytes {
        return None;
    }
    let next_cost = estimated_cost(next_bytes, state.path.cost_micros_per_mib)?;
    let previous_cost = estimated_cost(state.bytes_used, state.path.cost_micros_per_mib)?;
    let cost_delta = next_cost.checked_sub(previous_cost)?;
    let next_energy = estimated_cost(next_bytes, state.path.energy_uwh_per_mib)?;
    let previous_energy = estimated_cost(state.bytes_used, state.path.energy_uwh_per_mib)?;
    let energy_delta = next_energy.checked_sub(previous_energy)?;
    if aggregate_cost_used.checked_add(cost_delta)? > request.max_cost_micros {
        return None;
    }
    if aggregate_energy_used.checked_add(energy_delta)? > request.max_energy_uwh {
        return None;
    }
    Some((cost_delta, energy_delta))
}

fn eligible_states(
    request: &ScheduleRequest,
    paths: &[PathObservation],
) -> (Vec<PathState>, Vec<(String, &'static str)>) {
    let mut states = Vec::new();
    let mut exclusions = Vec::new();
    let mut ordered = paths.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.id.cmp(&right.id));
    for path in ordered {
        if let Some(reason) = exclusion_reason(request, path) {
            exclusions.push((path.id.clone(), reason));
        } else {
            states.push(PathState {
                path: path.clone(),
                ready_at_us: 0,
                bytes_used: 0,
                cost_used: 0,
                energy_used: 0,
                transmissions: 0,
                lost_transmissions: 0,
            });
        }
    }
    (states, exclusions)
}

fn projected_reorder_bytes(
    transmissions: &[TraceTransmission],
    segment_sizes: &[u32],
    current_segment: u32,
    candidate_arrival_us: u64,
) -> u64 {
    let mut earliest_by_segment: BTreeMap<u32, u64> = BTreeMap::new();
    for transmission in transmissions.iter().filter(|item| !item.lost) {
        earliest_by_segment
            .entry(transmission.segment_index)
            .and_modify(|arrival| *arrival = (*arrival).min(transmission.arrival_at_us))
            .or_insert(transmission.arrival_at_us);
    }

    let first_gap = (0..current_segment).find(|segment| {
        earliest_by_segment
            .get(segment)
            .is_none_or(|arrival| *arrival > candidate_arrival_us)
    });
    let Some(first_gap) = first_gap else {
        return 0;
    };

    let already_buffered = (first_gap + 1..current_segment)
        .filter(|segment| {
            earliest_by_segment
                .get(segment)
                .is_some_and(|arrival| *arrival <= candidate_arrival_us)
        })
        .map(|segment| u64::from(segment_sizes[segment as usize]))
        .sum::<u64>();

    already_buffered.saturating_add(u64::from(segment_sizes[current_segment as usize]))
}

fn choose_candidate(
    strategy: TraceStrategy,
    candidates: &[CandidatePath],
    states: &[PathState],
    transmissions: &[TraceTransmission],
    segment_index: u32,
    segment_sizes: &[u32],
    reorder_limit: u64,
    round_robin_cursor: usize,
) -> Option<CandidatePath> {
    match strategy {
        TraceStrategy::MinRtt => candidates.iter().copied().min_by(|left, right| {
            states[left.index]
                .path
                .rtt_us
                .cmp(&states[right.index].path.rtt_us)
                .then_with(|| left.arrival_at_us.cmp(&right.arrival_at_us))
                .then_with(|| states[left.index].path.id.cmp(&states[right.index].path.id))
        }),
        TraceStrategy::RoundRobin => (0..states.len())
            .map(|offset| (round_robin_cursor + offset) % states.len())
            .find_map(|index| {
                candidates
                    .iter()
                    .find(|candidate| candidate.index == index)
                    .copied()
            }),
        TraceStrategy::EarliestCompletion | TraceStrategy::IntentAware => {
            candidates.iter().copied().min_by(|left, right| {
                left.arrival_at_us
                    .cmp(&right.arrival_at_us)
                    .then_with(|| states[left.index].path.id.cmp(&states[right.index].path.id))
            })
        }
        TraceStrategy::ReorderAware => candidates
            .iter()
            .copied()
            .filter(|candidate| {
                projected_reorder_bytes(
                    transmissions,
                    segment_sizes,
                    segment_index,
                    candidate.arrival_at_us,
                ) <= reorder_limit
            })
            .min_by(|left, right| {
                left.arrival_at_us
                    .cmp(&right.arrival_at_us)
                    .then_with(|| states[left.index].path.id.cmp(&states[right.index].path.id))
            }),
        TraceStrategy::RedundantDistinctDomains => {
            candidates.iter().copied().min_by(|left, right| {
                left.arrival_at_us
                    .cmp(&right.arrival_at_us)
                    .then_with(|| states[left.index].path.id.cmp(&states[right.index].path.id))
            })
        }
    }
}

fn record_transmission(
    request: &ScheduleRequest,
    trace_request: &TraceRequest,
    state: &mut PathState,
    segment_index: u32,
    bytes: u32,
    redundant_copy: bool,
    aggregate_cost_used: &mut u64,
    aggregate_energy_used: &mut u64,
    records: &mut Vec<TraceTransmission>,
) -> Option<()> {
    let (cost, energy) = candidate_increment(
        request,
        state,
        bytes,
        *aggregate_cost_used,
        *aggregate_energy_used,
    )?;
    let path_candidate = candidate_path(state, 0, bytes)?;
    let loss = is_lost(trace_request.deterministic_seed, segment_index, &state.path);
    state.ready_at_us = path_candidate.send_at_us.saturating_add(serialization_us(
        bytes,
        super::effective_capacity(&state.path),
    ));
    state.bytes_used = state.bytes_used.checked_add(u64::from(bytes))?;
    state.cost_used = state.cost_used.checked_add(cost)?;
    state.energy_used = state.energy_used.checked_add(energy)?;
    *aggregate_cost_used = aggregate_cost_used.checked_add(cost)?;
    *aggregate_energy_used = aggregate_energy_used.checked_add(energy)?;
    state.transmissions = state.transmissions.checked_add(1)?;
    if loss {
        state.lost_transmissions = state.lost_transmissions.checked_add(1)?;
    }
    records.push(TraceTransmission {
        segment_index,
        path_id: state.path.id.clone(),
        send_at_us: path_candidate.send_at_us,
        arrival_at_us: path_candidate.arrival_at_us,
        payload_bytes: bytes,
        lost: loss,
        redundant_copy,
    });
    Some(())
}

fn pctl(sorted: &[u64], numerator: usize, denominator: usize) -> Option<u64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = (sorted.len() * numerator).div_ceil(denominator).max(1);
    sorted
        .get(rank - 1)
        .copied()
        .or_else(|| sorted.last().copied())
}

fn simulate_receiver(
    transmissions: &[TraceTransmission],
    segment_sizes: &[u32],
    reorder_limit: u64,
    reassembly_timeout_us: u64,
) -> (
    Vec<SegmentDelivery>,
    u64,
    u128,
    bool,
    u32,
    u64,
    u64,
    Vec<u64>,
) {
    let mut first_arrival: BTreeMap<u32, u64> = BTreeMap::new();
    for transmission in transmissions.iter().filter(|item| !item.lost) {
        first_arrival
            .entry(transmission.segment_index)
            .and_modify(|arrival| *arrival = (*arrival).min(transmission.arrival_at_us))
            .or_insert(transmission.arrival_at_us);
    }

    let network_received_segments = u32::try_from(first_arrival.len()).unwrap_or(u32::MAX);
    let network_received_unique_bytes = first_arrival
        .keys()
        .map(|segment| u64::from(segment_sizes[*segment as usize]))
        .sum::<u64>();
    let network_arrival_times = first_arrival.values().copied().collect::<Vec<_>>();
    let mut events = first_arrival
        .iter()
        .map(|(segment, arrival)| (*arrival, *segment))
        .collect::<Vec<_>>();
    events.sort_by_key(|(arrival, segment)| (*arrival, *segment));

    let mut next_expected = 0_u32;
    let mut buffered: BTreeMap<u32, u64> = BTreeMap::new();
    let mut buffered_bytes = 0_u64;
    let mut peak_buffered_bytes = 0_u64;
    let mut byte_microseconds = 0_u128;
    let mut previous_time = 0_u64;
    let mut exceeded = false;
    let mut delivered = Vec::new();

    for (arrival, segment) in events {
        let delta = arrival.saturating_sub(previous_time);
        byte_microseconds =
            byte_microseconds.saturating_add(u128::from(buffered_bytes) * u128::from(delta));
        previous_time = arrival;

        if segment == next_expected {
            delivered.push(SegmentDelivery {
                segment_index: segment,
                bytes: segment_sizes[segment as usize],
                first_arrival_us: arrival,
                application_release_us: arrival,
            });
            next_expected = next_expected.saturating_add(1);

            while let Some(first_arrival_us) = buffered.remove(&next_expected) {
                let bytes = segment_sizes[next_expected as usize];
                buffered_bytes = buffered_bytes.saturating_sub(u64::from(bytes));
                delivered.push(SegmentDelivery {
                    segment_index: next_expected,
                    bytes,
                    first_arrival_us,
                    application_release_us: arrival,
                });
                next_expected = next_expected.saturating_add(1);
            }
        } else if segment > next_expected && !buffered.contains_key(&segment) {
            buffered.insert(segment, arrival);
            buffered_bytes =
                buffered_bytes.saturating_add(u64::from(segment_sizes[segment as usize]));
            peak_buffered_bytes = peak_buffered_bytes.max(buffered_bytes);
            exceeded |= buffered_bytes > reorder_limit;
        }
    }

    if buffered_bytes > 0 {
        byte_microseconds = byte_microseconds
            .saturating_add(u128::from(buffered_bytes) * u128::from(reassembly_timeout_us));
    }

    delivered.sort_by_key(|item| item.segment_index);
    (
        delivered,
        peak_buffered_bytes,
        byte_microseconds,
        exceeded,
        network_received_segments,
        network_received_unique_bytes,
        buffered_bytes,
        network_arrival_times,
    )
}

fn received_transmission_bytes(transmissions: &[TraceTransmission]) -> u64 {
    transmissions
        .iter()
        .filter(|item| !item.lost)
        .map(|item| u64::from(item.payload_bytes))
        .sum()
}

/// Run a deterministic packet-trace schedule over broker-admitted path observations.
///
/// The model schedules all segments without congestion-window or ACK-clock
/// feedback. Loss is a deterministic synthetic trace keyed by seed, segment,
/// and declared shared-risk group. Identical risk groups therefore experience
/// identical per-segment loss events, providing a conservative common-cause
/// control; it is not a radio/channel simulator.
pub fn simulate(
    request: &ScheduleRequest,
    trace_request: &TraceRequest,
    paths: &[PathObservation],
    strategy: TraceStrategy,
) -> Result<TraceReport, ScheduleError> {
    validate_request(request)?;
    validate_path_set(paths)?;
    if trace_request.segment_bytes == 0 {
        return Err(ScheduleError::ZeroTraceSegmentSize);
    }
    if trace_request.reassembly_timeout_us == 0 {
        return Err(ScheduleError::ZeroReassemblyTimeout);
    }

    let (mut states, exclusions) = eligible_states(request, paths);
    let mut transmissions = Vec::new();
    let total_segments = u32::try_from(
        request
            .bytes
            .div_ceil(u64::from(trace_request.segment_bytes)),
    )
    .unwrap_or(u32::MAX);
    if total_segments == 0 {
        return Err(ScheduleError::ZeroPayload);
    }
    if total_segments > MAX_TRACE_SEGMENTS {
        return Err(ScheduleError::TooManyTraceSegments);
    }

    let mut segment_sizes = Vec::with_capacity(total_segments as usize);
    for segment in 0..total_segments {
        let offset = u64::from(segment) * u64::from(trace_request.segment_bytes);
        let bytes = request
            .bytes
            .saturating_sub(offset)
            .min(u64::from(trace_request.segment_bytes));
        segment_sizes.push(u32::try_from(bytes).unwrap_or(trace_request.segment_bytes));
    }

    let active_strategy = match strategy {
        TraceStrategy::IntentAware if request.ordered_delivery => TraceStrategy::ReorderAware,
        TraceStrategy::IntentAware => TraceStrategy::EarliestCompletion,
        other => other,
    };

    let mut round_robin_cursor = 0_usize;
    let mut aggregate_cost_used = 0_u64;
    let mut aggregate_energy_used = 0_u64;
    for segment_index in 0..total_segments {
        let bytes = segment_sizes[segment_index as usize];
        let candidates = states
            .iter()
            .enumerate()
            .filter_map(|(index, state)| {
                let path = candidate_path(state, index, bytes)?;
                candidate_increment(
                    request,
                    state,
                    bytes,
                    aggregate_cost_used,
                    aggregate_energy_used,
                )?;
                Some(path)
            })
            .collect::<Vec<_>>();

        if active_strategy == TraceStrategy::RedundantDistinctDomains {
            let mut ordered = candidates.clone();
            ordered.sort_by(|left, right| {
                left.arrival_at_us
                    .cmp(&right.arrival_at_us)
                    .then_with(|| states[left.index].path.id.cmp(&states[right.index].path.id))
            });
            let first = ordered.first().copied();
            let second = ordered.iter().copied().find(|candidate| {
                first.is_some_and(|first| {
                    states[first.index].path.shared_risk_group
                        != states[candidate.index].path.shared_risk_group
                        && capacity_domains_disjoint(
                            &states[first.index].path.capacity_domain_ids,
                            &states[candidate.index].path.capacity_domain_ids,
                        )
                })
            });
            let Some(first) = first else {
                continue;
            };
            if record_transmission(
                request,
                trace_request,
                &mut states[first.index],
                segment_index,
                bytes,
                false,
                &mut aggregate_cost_used,
                &mut aggregate_energy_used,
                &mut transmissions,
            )
            .is_none()
            {
                return Err(ScheduleError::InternalInvariant);
            }
            if let Some(second) = second {
                if candidate_increment(
                    request,
                    &states[second.index],
                    bytes,
                    aggregate_cost_used,
                    aggregate_energy_used,
                )
                .is_some()
                {
                    if record_transmission(
                        request,
                        trace_request,
                        &mut states[second.index],
                        segment_index,
                        bytes,
                        true,
                        &mut aggregate_cost_used,
                        &mut aggregate_energy_used,
                        &mut transmissions,
                    )
                    .is_none()
                    {
                        return Err(ScheduleError::InternalInvariant);
                    }
                }
            }
            continue;
        }

        let selected = choose_candidate(
            active_strategy,
            &candidates,
            &states,
            &transmissions,
            segment_index,
            &segment_sizes,
            request.reorder_buffer_limit_bytes,
            round_robin_cursor,
        );
        let Some(selected) = selected else {
            continue;
        };

        if record_transmission(
            request,
            trace_request,
            &mut states[selected.index],
            segment_index,
            bytes,
            false,
            &mut aggregate_cost_used,
            &mut aggregate_energy_used,
            &mut transmissions,
        )
        .is_none()
        {
            return Err(ScheduleError::InternalInvariant);
        }
        if active_strategy == TraceStrategy::RoundRobin && !states.is_empty() {
            round_robin_cursor = (selected.index + 1) % states.len();
        }
    }

    transmissions.sort_by(|left, right| {
        left.segment_index
            .cmp(&right.segment_index)
            .then_with(|| left.path_id.cmp(&right.path_id))
            .then_with(|| left.send_at_us.cmp(&right.send_at_us))
    });

    let (
        delivered,
        reorder_peak,
        reorder_byte_us,
        reorder_exceeded,
        network_received_segments,
        network_received_unique_bytes,
        unreleased_buffered_bytes,
        mut network_arrival_times,
    ) = simulate_receiver(
        &transmissions,
        &segment_sizes,
        request.reorder_buffer_limit_bytes,
        trace_request.reassembly_timeout_us,
    );
    let delivered_segments = u32::try_from(delivered.len()).unwrap_or(u32::MAX);
    let delivery_complete = delivered_segments == total_segments;
    let completion_us = if delivery_complete {
        delivered
            .iter()
            .map(|item| item.application_release_us)
            .max()
    } else {
        None
    };
    let delivered_by_deadline = delivery_complete
        && request
            .deadline_us
            .is_none_or(|deadline| completion_us.is_some_and(|finish| finish <= deadline));

    network_arrival_times.sort_unstable();
    let mut application_releases = delivered
        .iter()
        .map(|item| item.application_release_us)
        .collect::<Vec<_>>();
    application_releases.sort_unstable();

    let total_bytes_transmitted = transmissions
        .iter()
        .map(|item| u64::from(item.payload_bytes))
        .sum::<u64>();
    let useful_bytes_delivered = delivered
        .iter()
        .map(|item| u64::from(item.bytes))
        .sum::<u64>();
    let redundant_bytes = transmissions
        .iter()
        .filter(|item| item.redundant_copy)
        .map(|item| u64::from(item.payload_bytes))
        .sum::<u64>();
    let lost_transmissions = transmissions.iter().filter(|item| item.lost).count() as u64;

    let mut path_summaries = states
        .iter()
        .map(|state| PathTraceSummary {
            path_id: state.path.id.clone(),
            failure_domain: state.path.shared_risk_group.clone(),
            capacity_domain_ids: canonical_capacity_domain_ids(&state.path.capacity_domain_ids),
            capacity_domain_evidence: state.path.capacity_domain_evidence.clone(),
            transmitted_bytes: state.bytes_used,
            transmissions: state.transmissions,
            lost_transmissions: state.lost_transmissions,
        })
        .collect::<Vec<_>>();
    path_summaries.sort_by(|left, right| left.path_id.cmp(&right.path_id));

    let mut domains = BTreeSet::new();
    let mut capacity_domains = BTreeSet::new();
    for summary in &path_summaries {
        if summary.transmissions > 0 {
            domains.insert(summary.failure_domain.as_str());
            capacity_domains.extend(summary.capacity_domain_ids.iter().map(String::as_str));
        }
    }
    let capacity_domain_ids_used = capacity_domains
        .iter()
        .map(|domain| (*domain).to_owned())
        .collect();

    let estimated_cost_micros = states
        .iter()
        .try_fold(0_u64, |sum, state| sum.checked_add(state.cost_used))
        .ok_or(ScheduleError::ArithmeticOverflow)?;
    let estimated_energy_uwh = states
        .iter()
        .try_fold(0_u64, |sum, state| sum.checked_add(state.energy_used))
        .ok_or(ScheduleError::ArithmeticOverflow)?;
    if estimated_cost_micros > request.max_cost_micros
        || estimated_energy_uwh > request.max_energy_uwh
    {
        return Err(ScheduleError::InternalInvariant);
    }

    Ok(TraceReport {
        task_id: request.task_id.clone(),
        topology_sha256: request.topology_sha256.clone(),
        decision_time_unix_ms: request.now_unix_ms,
        strategy,
        total_segments,
        network_received_segments,
        network_received_unique_bytes,
        received_transmission_bytes: received_transmission_bytes(&transmissions),
        scheduled_segments: u32::try_from(
            transmissions
                .iter()
                .map(|item| item.segment_index)
                .collect::<BTreeSet<_>>()
                .len(),
        )
        .unwrap_or(u32::MAX),
        delivered_segments,
        failed_segments: total_segments.saturating_sub(delivered_segments),
        receiver_unreleased_buffered_bytes_at_timeout: unreleased_buffered_bytes,
        delivery_complete,
        delivered_by_deadline,
        completion_us,
        network_arrival_p50_us: pctl(&network_arrival_times, 50, 100),
        network_arrival_p95_us: pctl(&network_arrival_times, 95, 100),
        network_arrival_p99_us: pctl(&network_arrival_times, 99, 100),
        application_release_p50_us: pctl(&application_releases, 50, 100),
        application_release_p95_us: pctl(&application_releases, 95, 100),
        application_release_p99_us: pctl(&application_releases, 99, 100),
        total_bytes_transmitted,
        useful_bytes_delivered,
        redundant_bytes,
        lost_transmissions,
        receiver_reorder_peak_bytes: reorder_peak,
        receiver_reorder_byte_microseconds: reorder_byte_us,
        receiver_reorder_limit_exceeded: reorder_exceeded,
        estimated_cost_micros,
        estimated_energy_uwh,
        cost_currency: request.cost_currency.clone(),
        distinct_failure_domains_used: domains.len(),
        distinct_capacity_domains_used: capacity_domains.len(),
        capacity_domain_ids_used,
        path_summaries,
        transmissions,
        exclusions,
        limitations: vec![
            "deterministic_model_not_kernel_or_transport_measurement",
            "no_ack_clock_congestion_control_pacing_or_retransmission",
            "loss_is_synthetic_and_correlated_by_declared_failure_domain",
            "receiver_reassembly_is_a_bounded_stream_order_model",
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ScheduleRequest, TrafficIntent};

    fn path(id: &str, rtt: u64, capacity: u64, risk: &str) -> PathObservation {
        PathObservation {
            id: id.to_owned(),
            broker_eligible: true,
            authorization_valid: true,
            telemetry_fresh: true,
            trust_level: 2,
            rtt_us: rtt,
            jitter_us: 0,
            loss_ppm: 0,
            capacity_bps: capacity,
            mtu_bytes: 1500,
            data_remaining_bytes: 16 * 1_048_576,
            cost_currency: "ZAR".to_owned(),
            cost_micros_per_mib: 100,
            energy_uwh_per_mib: 10,
            shared_risk_group: risk.to_owned(),
            capacity_domain_ids: vec![format!("capacity-{id}")],
            capacity_domain_evidence: CapacityDomainEvidence {
                source_id: "operator-inventory-1".to_owned(),
                inventory_sha256: "a".repeat(64),
                topology_sha256: "f".repeat(64),
                complete: true,
                confidence_ppm: 1_000_000,
                observed_at_unix_ms: 900_000,
                valid_until_unix_ms: 1_100_000,
            },
        }
    }

    fn request() -> ScheduleRequest {
        ScheduleRequest {
            task_id: "trace-task".to_owned(),
            topology_sha256: "f".repeat(64),
            now_unix_ms: 1_000_000,
            minimum_capacity_domain_confidence_ppm: 950_000,
            intent: TrafficIntent::BulkTransfer,
            bytes: 128 * 1024,
            deadline_us: Some(5_000_000),
            max_cost_micros: 1_000_000,
            cost_currency: "ZAR".to_owned(),
            max_energy_uwh: 1_000_000,
            minimum_trust: 1,
            minimum_mtu_bytes: 1200,
            allow_multipath: true,
            maximum_paths: 2,
            ordered_delivery: true,
            reorder_buffer_limit_bytes: 64 * 1024,
            minimum_gain_ppm: 10_000,
        }
    }

    fn trace() -> TraceRequest {
        TraceRequest {
            segment_bytes: 1200,
            deterministic_seed: 0x4c4e_462d_5452_4143,
            reassembly_timeout_us: 500_000,
        }
    }

    #[test]
    fn min_rtt_uses_only_the_lowest_rtt_path() {
        let report = simulate(
            &request(),
            &trace(),
            &[
                path("fast", 10_000, 20_000_000, "a"),
                path("slow", 80_000, 100_000_000, "b"),
            ],
            TraceStrategy::MinRtt,
        )
        .expect("valid trace");
        assert!(
            report
                .path_summaries
                .iter()
                .filter(|p| p.transmissions > 0)
                .all(|p| p.path_id == "fast")
        );
    }

    #[test]
    fn round_robin_uses_both_available_paths() {
        let report = simulate(
            &request(),
            &trace(),
            &[
                path("a", 10_000, 20_000_000, "a"),
                path("b", 20_000, 20_000_000, "b"),
            ],
            TraceStrategy::RoundRobin,
        )
        .expect("valid trace");
        assert_eq!(
            report
                .path_summaries
                .iter()
                .filter(|p| p.transmissions > 0)
                .count(),
            2
        );
    }

    #[test]
    fn redundant_sends_distinct_domain_copies_and_counts_overhead() {
        let report = simulate(
            &request(),
            &trace(),
            &[
                path("a", 10_000, 20_000_000, "a"),
                path("b", 20_000, 20_000_000, "b"),
            ],
            TraceStrategy::RedundantDistinctDomains,
        )
        .expect("valid trace");
        assert!(report.redundant_bytes > 0);
        assert!(report.distinct_failure_domains_used >= 2);
        assert!(report.distinct_capacity_domains_used >= 2);
        assert!(report.total_bytes_transmitted >= report.useful_bytes_delivered);
    }

    #[test]
    fn redundant_strategy_does_not_duplicate_inside_one_failure_domain() {
        let report = simulate(
            &request(),
            &trace(),
            &[
                path("a", 10_000, 20_000_000, "same-backhaul"),
                path("b", 20_000, 20_000_000, "same-backhaul"),
            ],
            TraceStrategy::RedundantDistinctDomains,
        )
        .expect("valid trace");
        assert_eq!(report.redundant_bytes, 0);
        assert_eq!(report.distinct_failure_domains_used, 1);
    }

    #[test]
    fn redundant_strategy_does_not_duplicate_same_capacity_group() {
        let mut first = path("a", 10_000, 20_000_000, "failure-a");
        let mut second = path("b", 20_000, 20_000_000, "failure-b");
        first.capacity_domain_ids = vec!["access-a".to_owned(), "shared-backhaul".to_owned()];
        second.capacity_domain_ids = vec!["access-b".to_owned(), "shared-backhaul".to_owned()];
        let report = simulate(
            &request(),
            &trace(),
            &[first, second],
            TraceStrategy::RedundantDistinctDomains,
        )
        .expect("valid trace");
        assert_eq!(report.redundant_bytes, 0);
        assert_eq!(report.distinct_failure_domains_used, 2);
        // Both links may carry primary segments, but the shared backhaul may
        // not be treated as independent capacity for redundant copies.
        assert_eq!(report.distinct_capacity_domains_used, 3);
        assert_eq!(
            report.capacity_domain_ids_used,
            vec![
                "access-a".to_owned(),
                "access-b".to_owned(),
                "shared-backhaul".to_owned()
            ]
        );
        assert_eq!(report.topology_sha256, request().topology_sha256);
        assert_eq!(report.decision_time_unix_ms, request().now_unix_ms);
        assert!(report.path_summaries.iter().all(|summary| {
            summary.capacity_domain_evidence.complete
                && summary.capacity_domain_evidence.confidence_ppm == 1_000_000
                && summary.capacity_domain_evidence.topology_sha256 == report.topology_sha256
        }));
    }

    #[test]
    fn two_paths_with_distinct_capacity_domains_are_reported_separately() {
        let report = simulate(
            &request(),
            &trace(),
            &[
                path("a", 10_000, 20_000_000, "risk-a"),
                path("b", 20_000, 20_000_000, "risk-b"),
            ],
            TraceStrategy::RedundantDistinctDomains,
        )
        .expect("valid trace");
        assert_eq!(report.distinct_failure_domains_used, 2);
        assert_eq!(report.distinct_capacity_domains_used, 2);
    }

    #[test]
    fn identical_seed_and_inputs_produce_identical_reports() {
        let paths = [
            path("a", 10_000, 20_000_000, "a"),
            path("b", 30_000, 40_000_000, "b"),
        ];
        let first = simulate(&request(), &trace(), &paths, TraceStrategy::ReorderAware)
            .expect("valid trace");
        let second = simulate(&request(), &trace(), &paths, TraceStrategy::ReorderAware)
            .expect("valid trace");
        assert_eq!(first, second);
    }

    #[test]
    fn stale_or_ineligible_path_is_never_used() {
        let mut blocked = path("blocked", 1, 1_000_000_000, "blocked");
        blocked.broker_eligible = false;
        let report = simulate(
            &request(),
            &trace(),
            &[blocked, path("allowed", 20_000, 20_000_000, "allowed")],
            TraceStrategy::EarliestCompletion,
        )
        .expect("valid trace");
        assert!(
            report
                .transmissions
                .iter()
                .all(|item| item.path_id != "blocked")
        );
        assert_eq!(report.exclusions[0].1, "broker_ineligible");
    }

    #[test]
    fn same_failure_domain_has_identical_loss_for_a_given_segment() {
        let mut first = path("a", 10_000, 20_000_000, "shared-backhaul");
        let mut second = path("b", 20_000, 40_000_000, "shared-backhaul");
        first.loss_ppm = 500_000;
        second.loss_ppm = 500_000;
        for segment in 0..1_000 {
            assert_eq!(
                is_lost(trace().deterministic_seed, segment, &first),
                is_lost(trace().deterministic_seed, segment, &second)
            );
        }
    }

    #[test]
    fn trace_cost_is_rounded_per_path_not_per_packet() {
        let mut req = request();
        req.bytes = 1_048_576;
        req.max_cost_micros = 1_000;
        let paths = [path("a", 10_000, 100_000_000, "a")];
        let mut costed = paths[0].clone();
        costed.cost_micros_per_mib = 1_000;
        let mut tr = trace();
        tr.segment_bytes = 1200;
        let report = simulate(&req, &tr, &[costed], TraceStrategy::MinRtt).expect("valid trace");
        assert_eq!(report.estimated_cost_micros, 1_000);
    }

    #[test]
    fn trace_enforces_cost_budget_across_all_paths() {
        let mut req = request();
        req.bytes = 1_048_576;
        req.max_cost_micros = 50;
        let mut first = path("a", 10_000, 20_000_000, "a");
        let mut second = path("b", 20_000, 20_000_000, "b");
        first.cost_micros_per_mib = 100;
        second.cost_micros_per_mib = 100;
        let report = simulate(&req, &trace(), &[first, second], TraceStrategy::RoundRobin)
            .expect("valid trace can be partial");
        assert!(report.estimated_cost_micros <= req.max_cost_micros);
        assert!(!report.delivery_complete);
    }

    #[test]
    fn independent_failure_domains_use_different_loss_streams() {
        let mut first = path("a", 10_000, 20_000_000, "wan-a");
        let mut second = path("b", 10_000, 20_000_000, "wan-b");
        first.loss_ppm = 500_000;
        second.loss_ppm = 500_000;
        let disagreements = (0..1_000)
            .filter(|segment| {
                is_lost(trace().deterministic_seed, *segment, &first)
                    != is_lost(trace().deterministic_seed, *segment, &second)
            })
            .count();
        assert!(disagreements > 100);
    }

    #[test]
    fn reorder_projection_counts_bytes_already_buffered_behind_a_gap() {
        let transmissions = vec![TraceTransmission {
            segment_index: 1,
            path_id: "b".to_owned(),
            send_at_us: 0,
            arrival_at_us: 10,
            payload_bytes: 1200,
            lost: false,
            redundant_copy: false,
        }];
        assert_eq!(
            projected_reorder_bytes(&transmissions, &[1200, 1200, 1200], 2, 20),
            2400
        );
        assert_eq!(
            projected_reorder_bytes(&transmissions, &[1200, 1200, 1200], 0, 20),
            0
        );
    }

    #[test]
    fn received_out_of_order_bytes_are_not_useful_until_stream_release() {
        let transmissions = vec![
            TraceTransmission {
                segment_index: 0,
                path_id: "a".to_owned(),
                send_at_us: 0,
                arrival_at_us: 10,
                payload_bytes: 1200,
                lost: false,
                redundant_copy: false,
            },
            TraceTransmission {
                segment_index: 2,
                path_id: "b".to_owned(),
                send_at_us: 0,
                arrival_at_us: 20,
                payload_bytes: 1200,
                lost: false,
                redundant_copy: false,
            },
            TraceTransmission {
                segment_index: 1,
                path_id: "a".to_owned(),
                send_at_us: 0,
                arrival_at_us: 30,
                payload_bytes: 1200,
                lost: false,
                redundant_copy: false,
            },
        ];
        let (
            delivered,
            peak,
            byte_us,
            exceeded,
            received,
            unique_bytes,
            pending,
            _network_arrivals,
        ) = simulate_receiver(&transmissions, &[1200, 1200, 1200], 2400, 500_000);
        assert_eq!(received, 3);
        assert_eq!(unique_bytes, 3600);
        assert_eq!(delivered.len(), 3);
        assert_eq!(delivered[2].first_arrival_us, 20);
        assert_eq!(delivered[2].application_release_us, 30);
        assert_eq!(peak, 1200);
        assert_eq!(byte_us, 12_000);
        assert!(!exceeded);
        assert_eq!(pending, 0);
    }

    #[test]
    fn lost_gap_keeps_later_segments_out_of_useful_delivery_until_timeout() {
        let mut path = path("one-path", 10_000, 100_000_000, "one-domain");
        path.loss_ppm = 250_000;
        let seed = (0..10_000)
            .find(|seed| {
                is_lost(*seed, 0, &path) && (1..4).all(|segment| !is_lost(*seed, segment, &path))
            })
            .expect("a deterministic seed with a first-segment loss exists");
        let mut req = request();
        req.bytes = 4 * 1200;
        let mut tr = trace();
        tr.deterministic_seed = seed;
        let report = simulate(&req, &tr, &[path], TraceStrategy::MinRtt).expect("valid trace");
        assert_eq!(report.network_received_segments, 3);
        assert_eq!(report.network_received_unique_bytes, 3600);
        assert_eq!(report.delivered_segments, 0);
        assert_eq!(report.useful_bytes_delivered, 0);
        assert_eq!(report.receiver_unreleased_buffered_bytes_at_timeout, 3600);
        assert!(report.receiver_reorder_byte_microseconds > 0);
        assert!(!report.delivery_complete);
        assert_eq!(report.completion_us, None);
    }

    #[test]
    fn reorder_aware_schedule_respects_small_buffer_when_feasible() {
        let mut req = request();
        req.bytes = 64 * 1200;
        req.reorder_buffer_limit_bytes = 1200;
        let paths = [
            path("low-latency", 1_000, 20_000_000, "a"),
            path("high-latency", 250_000, 20_000_000, "b"),
        ];
        let rr = simulate(&req, &trace(), &paths, TraceStrategy::RoundRobin).expect("valid trace");
        let aware =
            simulate(&req, &trace(), &paths, TraceStrategy::ReorderAware).expect("valid trace");
        assert!(aware.receiver_reorder_peak_bytes <= rr.receiver_reorder_peak_bytes);
    }

    #[test]
    fn zero_reassembly_timeout_is_rejected() {
        let mut invalid = trace();
        invalid.reassembly_timeout_us = 0;
        assert_eq!(
            simulate(
                &request(),
                &invalid,
                &[path("a", 1000, 1_000_000, "a")],
                TraceStrategy::MinRtt
            )
            .unwrap_err(),
            ScheduleError::ZeroReassemblyTimeout
        );
    }

    #[test]
    fn segment_size_must_be_nonzero() {
        let invalid = TraceRequest {
            segment_bytes: 0,
            deterministic_seed: 1,
            reassembly_timeout_us: 500_000,
        };
        assert_eq!(
            simulate(
                &request(),
                &invalid,
                &[path("a", 1000, 1_000_000, "a")],
                TraceStrategy::MinRtt
            )
            .unwrap_err(),
            ScheduleError::ZeroTraceSegmentSize
        );
    }
}
