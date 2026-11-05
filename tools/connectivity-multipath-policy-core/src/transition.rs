//! Deterministic, bounded event-trace evaluation for synchronized network transitions.
//!
//! This is an offline evidence evaluator. It does not subscribe to kernel events,
//! perform path discovery, change routes, schedule packets or control a transport.
//! At explicit decision ticks it applies only events already observed and effective,
//! then invokes the existing tenant/resource incidence allocator.
//!
//! Event source digests are shape-checked and recorded, not cryptographically
//! authenticated here. A configured capacity-inventory verifier remains a separate
//! trust boundary. Allocated rates are planning outputs, never measured goodput.

use super::resource::{
    CapacityResource, ResourceAllocationRequest, ResourceDomainUsage, is_sha256,
    valid_identifier,
};
use super::tenant_resource::{
    TenantFlowRateAllocation, TenantRateAllocation, TenantResourceAllocationError,
    TenantResourceAllocationReport, TenantResourceFlow, TenantResourcePath,
    TenantResourcePrincipal, TenantPathRateAllocation, allocate_tenant_resource_incidence,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_EVENTS: usize = 4_096;
const MAX_DECISION_TICKS: usize = 256;
const MAX_SUBFLOWS_PER_PATH: usize = 256;
const MAX_DEGRADED_FACTOR_PPM: u32 = 1_000_000;
const MAX_EVENT_ID_BYTES: usize = 128;
const MAX_SOURCE_ID_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathHealth {
    Up,
    Degraded { capacity_factor_ppm: u32 },
    Down,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubflowState {
    Established,
    Stale,
    Removed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitionEventKind {
    TenantUpsert(TenantResourcePrincipal),
    TenantRemoved { tenant_id: String },
    FlowUpsert(TenantResourceFlow),
    FlowRemoved { flow_id: String },
    PathUpsert(TenantResourcePath),
    PathRemoved { path_id: String },
    PathHealthChanged { path_id: String, health: PathHealth },
    SubflowChanged {
        path_id: String,
        subflow_id: String,
        state: SubflowState,
    },
    CapacityResourceUpsert(CapacityResource),
    CapacityResourceRemoved { resource_id: String },
    /// Advances the topology epoch. The event header is bound to the old epoch
    /// and old topology digest; subsequent events must use the new epoch/digest.
    TopologyEpochAdvanced {
        next_epoch: u64,
        next_topology_sha256: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionEvent {
    pub event_id: String,
    pub source_id: String,
    /// Per-source, one-based contiguous sequence number.
    pub source_sequence: u64,
    /// Caller-computed SHA-256 of the canonical event statement; syntax only here.
    pub source_sha256: String,
    pub topology_epoch: u64,
    pub topology_sha256: String,
    pub occurred_at_unix_ms: u64,
    pub observed_at_unix_ms: u64,
    pub effective_at_unix_ms: u64,
    /// Caller-computed digest of the frozen state snapshot after this event.
    pub state_snapshot_sha256: String,
    pub kind: TransitionEventKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionSnapshot {
    pub topology_epoch: u64,
    pub topology_sha256: String,
    pub state_snapshot_sha256: String,
    pub minimum_evidence_confidence_ppm: u32,
    pub tenants: Vec<TenantResourcePrincipal>,
    pub flows: Vec<TenantResourceFlow>,
    pub paths: Vec<TenantResourcePath>,
    pub resources: Vec<CapacityResource>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventDisposition {
    Pending,
    Applied,
    DeferredNotObserved,
    DeferredNotEffective,
    DeferredSequenceGap,
    DeferredFutureTopologyEpoch,
    RejectedInvalidIdentifier,
    RejectedInvalidDigest,
    RejectedInvalidTime,
    RejectedDuplicateEventId,
    RejectedDuplicateOrOldSequence,
    RejectedOutOfOrderSourceTime,
    RejectedTopologyEpoch,
    RejectedTopologyDigest,
    RejectedInvalidEpochAdvance,
    RejectedMalformedEvent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventDispositionRecord {
    pub event_id: String,
    pub source_id: String,
    pub source_sequence: u64,
    pub disposition: EventDisposition,
    pub reason: &'static str,
    pub first_eligible_tick_unix_ms: Option<u64>,
    pub applied_at_unix_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceFloorStatus {
    Met,
    Unmet,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantOutageState {
    pub tenant_id: String,
    pub service_floor_bps: u64,
    pub allocated_rate_bps: u64,
    pub floor_status: ServiceFloorStatus,
    pub outage_started_at_unix_ms: Option<u64>,
    pub outage_duration_so_far_ms: Option<u64>,
    pub recovered_at_unix_ms: Option<u64>,
    pub last_recovery_duration_ms: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionTickReport {
    pub decision_time_unix_ms: u64,
    pub topology_epoch: u64,
    pub topology_sha256: String,
    pub state_snapshot_sha256: String,
    pub applied_event_ids: Vec<String>,
    pub path_up_count: u32,
    pub path_degraded_count: u32,
    pub path_down_count: u32,
    pub paths_became_down_this_tick: Vec<String>,
    pub paths_recovered_this_tick: Vec<String>,
    pub affected_flow_count: u32,
    pub affected_tenant_count: u32,
    pub synchronized_path_change_count: u32,
    pub zero_rate_tenant_ids: Vec<String>,
    pub service_floor_unmet_tenant_ids: Vec<String>,
    /// Floors that cannot be individually classified because the shared
    /// allocation failed before producing tenant allocations.
    pub service_floor_status_unknown_tenant_ids: Vec<String>,
    pub tenant_outages: Vec<TenantOutageState>,
    pub total_allocated_rate_bps: u64,
    pub tenant_allocations: Vec<TenantRateAllocation>,
    pub flow_allocations: Vec<TenantFlowRateAllocation>,
    pub path_allocations: Vec<TenantPathRateAllocation>,
    pub resource_usage: Vec<ResourceDomainUsage>,
    pub allocation_error: Option<TenantResourceAllocationError>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionTraceReport {
    pub initial_topology_epoch: u64,
    pub final_topology_epoch: u64,
    pub event_count: usize,
    pub applied_event_count: usize,
    pub rejected_event_count: usize,
    pub deferred_event_count: usize,
    pub event_dispositions: Vec<EventDispositionRecord>,
    pub ticks: Vec<TransitionTickReport>,
    pub limitations: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionError {
    InvalidInitialTopologyDigest,
    InvalidInitialSnapshotDigest,
    InvalidInitialEpoch,
    InvalidConfidenceFloor,
    TooManyEvents,
    TooManyDecisionTicks,
    ZeroDecisionTick,
    NonMonotonicDecisionTicks,
    InvalidEventSet,
    ArithmeticOverflow,
}

#[derive(Clone, Debug)]
struct MutableTransitionState {
    topology_epoch: u64,
    topology_sha256: String,
    state_snapshot_sha256: String,
    minimum_evidence_confidence_ppm: u32,
    tenants: BTreeMap<String, TenantResourcePrincipal>,
    flows: BTreeMap<String, TenantResourceFlow>,
    paths: BTreeMap<String, TenantResourcePath>,
    resources: BTreeMap<String, CapacityResource>,
    path_health: BTreeMap<String, PathHealth>,
    subflows: BTreeMap<String, BTreeSet<String>>,
    last_source_sequence: BTreeMap<String, u64>,
    last_source_occurred_at: BTreeMap<String, u64>,
    last_source_effective_at: BTreeMap<String, u64>,
    outage_started_at: BTreeMap<String, u64>,
    last_recovery_duration: BTreeMap<String, u64>,
}

fn bounded_identifier(value: &str, max_bytes: usize) -> bool {
    valid_identifier(value) && value.len() <= max_bytes
}

fn validate_initial(snapshot: &TransitionSnapshot) -> Result<(), TransitionError> {
    if !is_sha256(&snapshot.topology_sha256) {
        return Err(TransitionError::InvalidInitialTopologyDigest);
    }
    if !is_sha256(&snapshot.state_snapshot_sha256) {
        return Err(TransitionError::InvalidInitialSnapshotDigest);
    }
    if snapshot.topology_epoch == 0 {
        return Err(TransitionError::InvalidInitialEpoch);
    }
    if snapshot.minimum_evidence_confidence_ppm == 0
        || snapshot.minimum_evidence_confidence_ppm > 1_000_000
    {
        return Err(TransitionError::InvalidConfidenceFloor);
    }
    Ok(())
}

fn from_snapshot(snapshot: &TransitionSnapshot) -> MutableTransitionState {
    MutableTransitionState {
        topology_epoch: snapshot.topology_epoch,
        topology_sha256: snapshot.topology_sha256.clone(),
        state_snapshot_sha256: snapshot.state_snapshot_sha256.clone(),
        minimum_evidence_confidence_ppm: snapshot.minimum_evidence_confidence_ppm,
        tenants: snapshot
            .tenants
            .iter()
            .cloned()
            .map(|tenant| (tenant.tenant_id.clone(), tenant))
            .collect(),
        flows: snapshot
            .flows
            .iter()
            .cloned()
            .map(|flow| (flow.flow_id.clone(), flow))
            .collect(),
        paths: snapshot
            .paths
            .iter()
            .cloned()
            .map(|path| (path.path_id.clone(), path))
            .collect(),
        resources: snapshot
            .resources
            .iter()
            .cloned()
            .map(|resource| (resource.resource_id.clone(), resource))
            .collect(),
        path_health: BTreeMap::new(),
        subflows: BTreeMap::new(),
        last_source_sequence: BTreeMap::new(),
        last_source_occurred_at: BTreeMap::new(),
        last_source_effective_at: BTreeMap::new(),
        outage_started_at: BTreeMap::new(),
        last_recovery_duration: BTreeMap::new(),
    }
}

fn validate_event_envelope(event: &TransitionEvent) -> Option<EventDisposition> {
    if !bounded_identifier(&event.event_id, MAX_EVENT_ID_BYTES)
        || !bounded_identifier(&event.source_id, MAX_SOURCE_ID_BYTES)
    {
        return Some(EventDisposition::RejectedInvalidIdentifier);
    }
    if !is_sha256(&event.source_sha256) || !is_sha256(&event.topology_sha256)
        || !is_sha256(&event.state_snapshot_sha256)
    {
        return Some(EventDisposition::RejectedInvalidDigest);
    }
    if event.source_sequence == 0
        || event.occurred_at_unix_ms == 0
        || event.observed_at_unix_ms < event.occurred_at_unix_ms
        || event.effective_at_unix_ms < event.occurred_at_unix_ms
    {
        return Some(EventDisposition::RejectedInvalidTime);
    }
    if let TransitionEventKind::TopologyEpochAdvanced {
        next_epoch,
        next_topology_sha256,
    } = &event.kind
    {
        if *next_epoch == 0 || !is_sha256(next_topology_sha256) {
            return Some(EventDisposition::RejectedInvalidEpochAdvance);
        }
    }
    None
}

fn disposition_reason(disposition: EventDisposition) -> &'static str {
    match disposition {
        EventDisposition::Pending => "event_has_not_been_considered_at_a_decision_tick",
        EventDisposition::Applied => "event_applied_at_or_after_observation_and_effective_time",
        EventDisposition::DeferredNotObserved => "event_was_not_yet_observed_at_decision_time",
        EventDisposition::DeferredNotEffective => "event_was_observed_but_not_yet_effective",
        EventDisposition::DeferredSequenceGap => "an_earlier_source_sequence_is_not_yet_available",
        EventDisposition::DeferredFutureTopologyEpoch => "event_waits_for_its_topology_epoch",
        EventDisposition::RejectedInvalidIdentifier => "event_or_source_identifier_is_invalid",
        EventDisposition::RejectedInvalidDigest => "event_source_or_state_digest_is_invalid",
        EventDisposition::RejectedInvalidTime => "event_time_order_is_invalid",
        EventDisposition::RejectedDuplicateEventId => "event_id_is_duplicated",
        EventDisposition::RejectedDuplicateOrOldSequence => "source_sequence_is_duplicate_or_old",
        EventDisposition::RejectedOutOfOrderSourceTime => "source_event_time_or_effective_time_rewinds",
        EventDisposition::RejectedTopologyEpoch => "event_belongs_to_an_obsolete_topology_epoch",
        EventDisposition::RejectedTopologyDigest => "event_topology_digest_does_not_match_active_epoch",
        EventDisposition::RejectedInvalidEpochAdvance => "topology_epoch_advance_is_not_the_next_epoch",
        EventDisposition::RejectedMalformedEvent => "event_references_unknown_or_invalid_state",
    }
}

fn remove_tenant(state: &mut MutableTransitionState, tenant_id: &str) {
    state.tenants.remove(tenant_id);
    let removed_flows = state
        .flows
        .iter()
        .filter(|(_, flow)| flow.tenant_id == tenant_id)
        .map(|(id, _)| id.clone())
        .collect::<BTreeSet<_>>();
    for flow_id in &removed_flows {
        state.flows.remove(flow_id);
    }
    let removed_paths = state
        .paths
        .iter()
        .filter(|(_, path)| removed_flows.contains(&path.flow_id))
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    for path_id in removed_paths {
        state.paths.remove(&path_id);
        state.path_health.remove(&path_id);
        state.subflows.remove(&path_id);
    }
}

fn apply_event(
    state: &mut MutableTransitionState,
    event: &TransitionEvent,
) -> Result<(), EventDisposition> {
    if event.topology_epoch < state.topology_epoch {
        return Err(EventDisposition::RejectedTopologyEpoch);
    }
    if event.topology_epoch > state.topology_epoch {
        return Err(EventDisposition::DeferredFutureTopologyEpoch);
    }
    if event.topology_sha256 != state.topology_sha256 {
        return Err(EventDisposition::RejectedTopologyDigest);
    }

    match &event.kind {
        TransitionEventKind::TenantUpsert(tenant) => {
            if !bounded_identifier(&tenant.tenant_id, MAX_SOURCE_ID_BYTES)
                || tenant.tenant_weight == 0
            {
                return Err(EventDisposition::RejectedMalformedEvent);
            }
            state.tenants.insert(tenant.tenant_id.clone(), tenant.clone());
        }
        TransitionEventKind::TenantRemoved { tenant_id } => {
            if !state.tenants.contains_key(tenant_id) {
                return Err(EventDisposition::RejectedMalformedEvent);
            }
            remove_tenant(state, tenant_id);
        }
        TransitionEventKind::FlowUpsert(flow) => {
            if !state.tenants.contains_key(&flow.tenant_id)
                || !bounded_identifier(&flow.flow_id, MAX_SOURCE_ID_BYTES)
                || flow.demand_bps == 0
                || flow.flow_weight == 0
            {
                return Err(EventDisposition::RejectedMalformedEvent);
            }
            state.flows.insert(flow.flow_id.clone(), flow.clone());
        }
        TransitionEventKind::FlowRemoved { flow_id } => {
            if state.flows.remove(flow_id).is_none() {
                return Err(EventDisposition::RejectedMalformedEvent);
            }
            let removed = state
                .paths
                .iter()
                .filter(|(_, path)| path.flow_id == *flow_id)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            for path_id in removed {
                state.paths.remove(&path_id);
                state.path_health.remove(&path_id);
                state.subflows.remove(&path_id);
            }
        }
        TransitionEventKind::PathUpsert(path) => {
            if !state.flows.contains_key(&path.flow_id)
                || !bounded_identifier(&path.path_id, MAX_SOURCE_ID_BYTES)
                || path.path_ceiling_bps == 0
                || path.split_weight == 0
            {
                return Err(EventDisposition::RejectedMalformedEvent);
            }
            state.paths.insert(path.path_id.clone(), path.clone());
        }
        TransitionEventKind::PathRemoved { path_id } => {
            if state.paths.remove(path_id).is_none() {
                return Err(EventDisposition::RejectedMalformedEvent);
            }
            state.path_health.remove(path_id);
            state.subflows.remove(path_id);
        }
        TransitionEventKind::PathHealthChanged { path_id, health } => {
            if !state.paths.contains_key(path_id) {
                return Err(EventDisposition::RejectedMalformedEvent);
            }
            if matches!(health, PathHealth::Degraded { capacity_factor_ppm } if *capacity_factor_ppm > MAX_DEGRADED_FACTOR_PPM)
            {
                return Err(EventDisposition::RejectedMalformedEvent);
            }
            state.path_health.insert(path_id.clone(), *health);
        }
        TransitionEventKind::SubflowChanged {
            path_id,
            subflow_id,
            state: subflow_state,
        } => {
            if !state.paths.contains_key(path_id)
                || !bounded_identifier(subflow_id, MAX_SOURCE_ID_BYTES)
            {
                return Err(EventDisposition::RejectedMalformedEvent);
            }
            let subflows = state.subflows.entry(path_id.clone()).or_default();
            match subflow_state {
                SubflowState::Established => {
                    if !subflows.contains(subflow_id) && subflows.len() >= MAX_SUBFLOWS_PER_PATH {
                        return Err(EventDisposition::RejectedMalformedEvent);
                    }
                    subflows.insert(subflow_id.clone());
                }
                SubflowState::Stale | SubflowState::Removed => {
                    subflows.remove(subflow_id);
                }
            }
        }
        TransitionEventKind::CapacityResourceUpsert(resource) => {
            if !bounded_identifier(&resource.resource_id, MAX_SOURCE_ID_BYTES)
                || resource.evidence.topology_sha256 != state.topology_sha256
            {
                return Err(EventDisposition::RejectedMalformedEvent);
            }
            state.resources.insert(resource.resource_id.clone(), resource.clone());
        }
        TransitionEventKind::CapacityResourceRemoved { resource_id } => {
            if state.resources.remove(resource_id).is_none() {
                return Err(EventDisposition::RejectedMalformedEvent);
            }
        }
        TransitionEventKind::TopologyEpochAdvanced {
            next_epoch,
            next_topology_sha256,
        } => {
            if *next_epoch != state.topology_epoch.checked_add(1).ok_or(
                EventDisposition::RejectedInvalidEpochAdvance,
            )? {
                return Err(EventDisposition::RejectedInvalidEpochAdvance);
            }
            state.topology_epoch = *next_epoch;
            state.topology_sha256 = next_topology_sha256.clone();
        }
    }

    state.state_snapshot_sha256 = event.state_snapshot_sha256.clone();
    Ok(())
}

fn effective_path(
    path: &TenantResourcePath,
    state: &MutableTransitionState,
) -> Result<Option<TenantResourcePath>, TransitionError> {
    if let Some(subflows) = state.subflows.get(&path.path_id) {
        if subflows.is_empty() {
            return None;
        }
    }
    let health = state.path_health.get(&path.path_id).copied().unwrap_or(PathHealth::Up);
    match health {
        PathHealth::Down => Ok(None),
        PathHealth::Up => Ok(Some(path.clone())),
        PathHealth::Degraded { capacity_factor_ppm } => {
            let mut degraded = path.clone();
            let ceiling = u128::from(path.path_ceiling_bps)
                .checked_mul(u128::from(capacity_factor_ppm))
                .ok_or(TransitionError::ArithmeticOverflow)?;
            degraded.path_ceiling_bps =
                u64::try_from(ceiling / 1_000_000).map_err(|_| TransitionError::ArithmeticOverflow)?;
            Ok((degraded.path_ceiling_bps > 0).then_some(degraded))
        }
    }
}

fn apply_pending_events(
    state: &mut MutableTransitionState,
    events: &[TransitionEvent],
    records: &mut [EventDispositionRecord],
    applied_ids: &mut Vec<String>,
    tick_unix_ms: u64,
) {
    let mut order = (0..events.len()).collect::<Vec<_>>();
    order.sort_by(|left, right| {
        let a = &events[*left];
        let b = &events[*right];
        a.source_id
            .cmp(&b.source_id)
            .then_with(|| a.source_sequence.cmp(&b.source_sequence))
            .then_with(|| a.effective_at_unix_ms.cmp(&b.effective_at_unix_ms))
            .then_with(|| a.event_id.cmp(&b.event_id))
            .then_with(|| left.cmp(right))
    });

    // Re-scan after each application so a topology-advance event or a missing
    // preceding source sequence can make a later event eligible in this tick.
    for _ in 0..=events.len() {
        let mut progress = false;
        for index in &order {
            let event = &events[*index];
            let record = &mut records[*index];
            if matches!(
                record.disposition,
                EventDisposition::Applied
                    | EventDisposition::RejectedInvalidIdentifier
                    | EventDisposition::RejectedInvalidDigest
                    | EventDisposition::RejectedInvalidTime
                    | EventDisposition::RejectedDuplicateEventId
                    | EventDisposition::RejectedDuplicateOrOldSequence
                    | EventDisposition::RejectedOutOfOrderSourceTime
                    | EventDisposition::RejectedTopologyEpoch
                    | EventDisposition::RejectedTopologyDigest
                    | EventDisposition::RejectedInvalidEpochAdvance
                    | EventDisposition::RejectedMalformedEvent
            ) {
                continue;
            }
            if let Some(invalid) = validate_event_envelope(event) {
                record.disposition = invalid;
                record.reason = disposition_reason(invalid);
                progress = true;
                continue;
            }
            if event.observed_at_unix_ms > tick_unix_ms {
                record.disposition = EventDisposition::DeferredNotObserved;
                record.reason = disposition_reason(record.disposition);
                continue;
            }
            if event.effective_at_unix_ms > tick_unix_ms {
                record.disposition = EventDisposition::DeferredNotEffective;
                record.reason = disposition_reason(record.disposition);
                continue;
            }
            if record.first_eligible_tick_unix_ms.is_none() {
                record.first_eligible_tick_unix_ms = Some(tick_unix_ms);
            }
            if event.topology_epoch > state.topology_epoch {
                record.disposition = EventDisposition::DeferredFutureTopologyEpoch;
                record.reason = disposition_reason(record.disposition);
                continue;
            }

            let expected_sequence = state
                .last_source_sequence
                .get(&event.source_id)
                .copied()
                .unwrap_or(0)
                .saturating_add(1);
            if event.source_sequence < expected_sequence {
                record.disposition = EventDisposition::RejectedDuplicateOrOldSequence;
                record.reason = disposition_reason(record.disposition);
                progress = true;
                continue;
            }
            if event.source_sequence > expected_sequence {
                record.disposition = EventDisposition::DeferredSequenceGap;
                record.reason = disposition_reason(record.disposition);
                continue;
            }
            let last_occurred = state
                .last_source_occurred_at
                .get(&event.source_id)
                .copied()
                .unwrap_or(0);
            let last_effective = state
                .last_source_effective_at
                .get(&event.source_id)
                .copied()
                .unwrap_or(0);
            if event.occurred_at_unix_ms < last_occurred
                || event.effective_at_unix_ms < last_effective
            {
                record.disposition = EventDisposition::RejectedOutOfOrderSourceTime;
                record.reason = disposition_reason(record.disposition);
                progress = true;
                continue;
            }

            match apply_event(state, event) {
                Ok(()) => {
                    state
                        .last_source_sequence
                        .insert(event.source_id.clone(), event.source_sequence);
                    state
                        .last_source_occurred_at
                        .insert(event.source_id.clone(), event.occurred_at_unix_ms);
                    state
                        .last_source_effective_at
                        .insert(event.source_id.clone(), event.effective_at_unix_ms);
                    record.disposition = EventDisposition::Applied;
                    record.reason = disposition_reason(record.disposition);
                    record.applied_at_unix_ms = Some(tick_unix_ms);
                    applied_ids.push(event.event_id.clone());
                    progress = true;
                }
                Err(disposition) => {
                    record.disposition = disposition;
                    record.reason = disposition_reason(disposition);
                    if disposition != EventDisposition::DeferredFutureTopologyEpoch {
                        progress = true;
                    }
                }
            }
        }
        if !progress {
            break;
        }
    }
}

fn transition_state_from_snapshot(snapshot: &TransitionSnapshot) -> MutableTransitionState {
    let mut state = from_snapshot(snapshot);
    state.path_health = snapshot
        .paths
        .iter()
        .map(|path| (path.path_id.clone(), PathHealth::Up))
        .collect();
    state
}

/// Replay a finite event stream at explicit decision ticks.
///
/// Events are ordered by source ID/sequence for deterministic replay. An event
/// may affect a tick only when both its observation and effective times are no
/// later than that tick. Sequence gaps and future epochs are deferred and may
/// become applicable at later ticks. No wall clock or hidden look-ahead is used.
pub fn evaluate_transition_trace(
    initial: &TransitionSnapshot,
    events: &[TransitionEvent],
    decision_ticks_unix_ms: &[u64],
) -> Result<TransitionTraceReport, TransitionError> {
    validate_initial(initial)?;
    if events.len() > MAX_EVENTS {
        return Err(TransitionError::TooManyEvents);
    }
    if decision_ticks_unix_ms.is_empty() || decision_ticks_unix_ms.len() > MAX_DECISION_TICKS {
        return Err(TransitionError::TooManyDecisionTicks);
    }
    let mut previous_tick = 0_u64;
    for tick in decision_ticks_unix_ms {
        if *tick == 0 {
            return Err(TransitionError::ZeroDecisionTick);
        }
        if *tick <= previous_tick {
            return Err(TransitionError::NonMonotonicDecisionTicks);
        }
        previous_tick = *tick;
    }

    let mut records = events
        .iter()
        .map(|event| EventDispositionRecord {
            event_id: event.event_id.clone(),
            source_id: event.source_id.clone(),
            source_sequence: event.source_sequence,
            disposition: EventDisposition::Pending,
            reason: disposition_reason(EventDisposition::Pending),
            first_eligible_tick_unix_ms: None,
            applied_at_unix_ms: None,
        })
        .collect::<Vec<_>>();
    let mut seen_ids = BTreeSet::new();
    let mut duplicate_ids = BTreeSet::new();
    for event in events {
        if !seen_ids.insert(event.event_id.as_str()) {
            duplicate_ids.insert(event.event_id.as_str());
        }
    }
    for (index, event) in events.iter().enumerate() {
        if duplicate_ids.contains(event.event_id.as_str()) {
            records[index].disposition = EventDisposition::RejectedDuplicateEventId;
            records[index].reason = disposition_reason(EventDisposition::RejectedDuplicateEventId);
        }
    }

    let mut state = transition_state_from_snapshot(initial);
    let mut previous_path_health = state.path_health.clone();
    let mut ticks = Vec::with_capacity(decision_ticks_unix_ms.len());
    for tick in decision_ticks_unix_ms {
        let mut applied_event_ids = Vec::new();
        apply_pending_events(&mut state, events, &mut records, &mut applied_event_ids, *tick);

        let mut paths = Vec::with_capacity(state.paths.len());
        for path in state.paths.values() {
            if let Some(effective) = effective_path(path, &state)? {
                paths.push(effective);
            }
        }

        let request = ResourceAllocationRequest {
            topology_sha256: state.topology_sha256.clone(),
            resource_snapshot_sha256: state.state_snapshot_sha256.clone(),
            now_unix_ms: *tick,
            minimum_evidence_confidence_ppm: state.minimum_evidence_confidence_ppm,
        };
        let allocation_result = allocate_tenant_resource_incidence(
            &request,
            &state.tenants.values().cloned().collect::<Vec<_>>(),
            &state.flows.values().cloned().collect::<Vec<_>>(),
            &paths,
            &state.resources.values().cloned().collect::<Vec<_>>(),
        );

        let mut total_allocated_rate_bps = 0_u64;
        let mut tenant_allocations = Vec::new();
        let mut flow_allocations = Vec::new();
        let mut path_allocations = Vec::new();
        let mut resource_usage = Vec::new();
        let mut allocation_error = None;
        let mut zero_rate_tenant_ids = Vec::new();
        let mut service_floor_unmet_tenant_ids = Vec::new();
        let mut service_floor_status_unknown_tenant_ids = Vec::new();
        match allocation_result {
            Ok(report) => {
                total_allocated_rate_bps = report.total_allocated_rate_bps;
                tenant_allocations = report.tenant_allocations;
                flow_allocations = report.flow_allocations;
                path_allocations = report.path_allocations;
                resource_usage = report.resource_usage;
                for tenant in &tenant_allocations {
                    if tenant.allocated_rate_bps == 0 {
                        zero_rate_tenant_ids.push(tenant.tenant_id.clone());
                    }
                    if !tenant.minimum_service_met {
                        service_floor_unmet_tenant_ids.push(tenant.tenant_id.clone());
                    }
                }
            }
            Err(error) => {
                if error == TenantResourceAllocationError::MinimumServiceInfeasible {
                    for tenant in state.tenants.values().filter(|tenant| tenant.minimum_service_bps > 0) {
                        let tenant_has_eligible_path = state.flows.values()
                            .filter(|flow| flow.tenant_id == tenant.tenant_id)
                            .flat_map(|flow| paths.iter().filter(move |path| path.flow_id == flow.flow_id))
                            .next()
                            .is_some();
                        if tenant_has_eligible_path {
                            service_floor_status_unknown_tenant_ids.push(tenant.tenant_id.clone());
                        } else {
                            service_floor_unmet_tenant_ids.push(tenant.tenant_id.clone());
                        }
                    }
                } else {
                    service_floor_status_unknown_tenant_ids = state.tenants.values()
                        .filter(|tenant| tenant.minimum_service_bps > 0)
                        .map(|tenant| tenant.tenant_id.clone())
                        .collect();
                }
                allocation_error = Some(error);
            }
        }

        let allocated_by_tenant: BTreeMap<&str, u64> = tenant_allocations
            .iter()
            .map(|tenant| (tenant.tenant_id.as_str(), tenant.allocated_rate_bps))
            .collect();
        let mut tenant_outages = Vec::with_capacity(state.tenants.len());
        for tenant in state.tenants.values() {
            let allocated = allocated_by_tenant
                .get(tenant.tenant_id.as_str())
                .copied()
                .unwrap_or(0);
            let floor_status = if tenant.minimum_service_bps == 0 {
                ServiceFloorStatus::Met
            } else if allocation_error.is_none() {
                if allocated >= tenant.minimum_service_bps {
                    ServiceFloorStatus::Met
                } else {
                    ServiceFloorStatus::Unmet
                }
            } else if service_floor_unmet_tenant_ids.contains(&tenant.tenant_id) {
                ServiceFloorStatus::Unmet
            } else {
                ServiceFloorStatus::Unknown
            };

            let started = state.outage_started_at.get(&tenant.tenant_id).copied();
            let mut recovered_at = None;
            let mut last_recovery = state.last_recovery_duration.get(&tenant.tenant_id).copied();
            match floor_status {
                ServiceFloorStatus::Met => {
                    if let Some(outage_start) = started {
                        let duration = tick
                            .checked_sub(outage_start)
                            .ok_or(TransitionError::ArithmeticOverflow)?;
                        recovered_at = Some(*tick);
                        last_recovery = Some(duration);
                        state.last_recovery_duration.insert(tenant.tenant_id.clone(), duration);
                        state.outage_started_at.remove(&tenant.tenant_id);
                    }
                }
                ServiceFloorStatus::Unmet => {
                    state.outage_started_at
                        .entry(tenant.tenant_id.clone())
                        .or_insert(*tick);
                }
                ServiceFloorStatus::Unknown => {}
            }
            tenant_outages.push(TenantOutageState {
                tenant_id: tenant.tenant_id.clone(),
                service_floor_bps: tenant.minimum_service_bps,
                allocated_rate_bps: allocated,
                floor_status,
                outage_started_at_unix_ms: if recovered_at.is_some() {
                    started
                } else {
                    state.outage_started_at.get(&tenant.tenant_id).copied()
                },
                outage_duration_so_far_ms: state
                    .outage_started_at
                    .get(&tenant.tenant_id)
                    .and_then(|start| tick.checked_sub(*start)),
                recovered_at_unix_ms: recovered_at,
                last_recovery_duration_ms: last_recovery,
            });
        }

        let mut paths_became_down_this_tick = Vec::new();
        let mut paths_recovered_this_tick = Vec::new();
        let mut path_up_count = 0_u32;
        let mut path_degraded_count = 0_u32;
        let mut path_down_count = 0_u32;
        for path_id in state.paths.keys() {
            let health = state.path_health.get(path_id).copied().unwrap_or(PathHealth::Up);
            let subflow_down = state.subflows.get(path_id).is_some_and(BTreeSet::is_empty);
            let effective_down = health == PathHealth::Down
                || matches!(health, PathHealth::Degraded { capacity_factor_ppm: 0 })
                || subflow_down;
            let was_down = previous_path_health.get(path_id).is_some_and(|prior| *prior == PathHealth::Down);
            if effective_down {
                path_down_count += 1;
                if !was_down {
                    paths_became_down_this_tick.push(path_id.clone());
                }
            } else if matches!(health, PathHealth::Degraded { .. }) {
                path_degraded_count += 1;
            } else {
                path_up_count += 1;
                if was_down {
                    paths_recovered_this_tick.push(path_id.clone());
                }
            }

        }
        previous_path_health = state
            .paths
            .keys()
            .map(|path_id| {
                let health = if state.subflows.get(path_id).is_some_and(BTreeSet::is_empty)
                    || matches!(
                        state.path_health.get(path_id),
                        Some(PathHealth::Degraded { capacity_factor_ppm: 0 })
                    )
                {
                    PathHealth::Down
                } else {
                    state.path_health.get(path_id).copied().unwrap_or(PathHealth::Up)
                };
                (path_id.clone(), health)
            })
            .collect();

        let changed = paths_became_down_this_tick.len() + paths_recovered_this_tick.len();
        let affected_flows = state.flows.values().filter(|flow| {
            let flow_paths = state.paths.values().filter(|path| path.flow_id == flow.flow_id).collect::<Vec<_>>();
            flow_paths.iter().all(|path| {
                state.path_health.get(&path.path_id) == Some(&PathHealth::Down)
                    || state.subflows.get(&path.path_id).is_some_and(BTreeSet::is_empty)
            })
        }).count();
        let affected_tenants = state.tenants.values().filter(|tenant| {
            state.flows.values().any(|flow| flow.tenant_id == tenant.tenant_id && {
                let flow_paths = state.paths.values().filter(|path| path.flow_id == flow.flow_id).collect::<Vec<_>>();
                flow_paths.iter().all(|path| {
                    state.path_health.get(&path.path_id) == Some(&PathHealth::Down)
                        || state.subflows.get(&path.path_id).is_some_and(BTreeSet::is_empty)
                })
            })
        }).count();

        applied_event_ids.sort();
        paths_became_down_this_tick.sort();
        paths_recovered_this_tick.sort();
        zero_rate_tenant_ids.sort();
        service_floor_unmet_tenant_ids.sort();
        service_floor_status_unknown_tenant_ids.sort();
        tenant_outages.sort_by(|left, right| left.tenant_id.cmp(&right.tenant_id));

        ticks.push(TransitionTickReport {
            decision_time_unix_ms: *tick,
            topology_epoch: state.topology_epoch,
            topology_sha256: state.topology_sha256.clone(),
            state_snapshot_sha256: state.state_snapshot_sha256.clone(),
            applied_event_ids,
            path_up_count,
            path_degraded_count,
            path_down_count,
            paths_became_down_this_tick,
            paths_recovered_this_tick,
            affected_flow_count: u32::try_from(affected_flows).map_err(|_| TransitionError::ArithmeticOverflow)?,
            affected_tenant_count: u32::try_from(affected_tenants).map_err(|_| TransitionError::ArithmeticOverflow)?,
            synchronized_path_change_count: u32::try_from(changed).map_err(|_| TransitionError::ArithmeticOverflow)?,
            zero_rate_tenant_ids,
            service_floor_unmet_tenant_ids,
            service_floor_status_unknown_tenant_ids,
            tenant_outages,
            total_allocated_rate_bps,
            tenant_allocations,
            flow_allocations,
            path_allocations,
            resource_usage,
            allocation_error,
        });
    }

    let applied_event_count = records.iter().filter(|record| record.disposition == EventDisposition::Applied).count();
    let rejected_event_count = records.iter().filter(|record| matches!(
        record.disposition,
        EventDisposition::RejectedInvalidIdentifier
            | EventDisposition::RejectedInvalidDigest
            | EventDisposition::RejectedInvalidTime
            | EventDisposition::RejectedDuplicateEventId
            | EventDisposition::RejectedDuplicateOrOldSequence
            | EventDisposition::RejectedOutOfOrderSourceTime
            | EventDisposition::RejectedTopologyEpoch
            | EventDisposition::RejectedTopologyDigest
            | EventDisposition::RejectedInvalidEpochAdvance
            | EventDisposition::RejectedMalformedEvent
    )).count();
    let deferred_event_count = records.len() - applied_event_count - rejected_event_count;
    Ok(TransitionTraceReport {
        initial_topology_epoch: initial.topology_epoch,
        final_topology_epoch: state.topology_epoch,
        event_count: events.len(),
        applied_event_count: u32::try_from(applied_event_count).map_err(|_| TransitionError::ArithmeticOverflow)? as usize,
        rejected_event_count: u32::try_from(rejected_event_count).map_err(|_| TransitionError::ArithmeticOverflow)? as usize,
        deferred_event_count: u32::try_from(deferred_event_count).map_err(|_| TransitionError::ArithmeticOverflow)? as usize,
        event_dispositions: records,
        ticks,
        limitations: vec![
            "allocated_rate_is_a_deterministic_resource_model_not_measured_goodput",
            "event_source_sha256_is_syntax_checked_not_cryptographically_authenticated",
            "capacity_inventory_signature_verification_remains_a_separate_trust_boundary",
            "no_kernel_path_manager_scheduler_or_route_actuation_is_performed",
            "recovery_time_is_service_floor_restoration_in_the_supplied_event_trace",
            "topology_discovery_and_hidden_resource_inference_are_not_performed",
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(topology_sha256: &str) -> super::super::resource::CapacityResourceEvidence {
        super::super::resource::CapacityResourceEvidence {
            source_id: "operator-inventory".to_owned(),
            inventory_sha256: "a".repeat(64),
            topology_sha256: topology_sha256.to_owned(),
            complete: true,
            confidence_ppm: 1_000_000,
            measurement_window_start_unix_ms: 800,
            measurement_window_end_unix_ms: 900,
            observed_at_unix_ms: 950,
            valid_until_unix_ms: 10_000,
        }
    }

    fn snapshot() -> TransitionSnapshot {
        let topology = "b".repeat(64);
        TransitionSnapshot {
            topology_epoch: 1,
            topology_sha256: topology.clone(),
            state_snapshot_sha256: "c".repeat(64),
            minimum_evidence_confidence_ppm: 950_000,
            tenants: vec![
                TenantResourcePrincipal {
                    tenant_id: "tenant-a".to_owned(),
                    tenant_weight: 1,
                    minimum_service_bps: 10,
                },
                TenantResourcePrincipal {
                    tenant_id: "tenant-b".to_owned(),
                    tenant_weight: 1,
                    minimum_service_bps: 10,
                },
            ],
            flows: vec![
                TenantResourceFlow {
                    flow_id: "flow-a".to_owned(),
                    tenant_id: "tenant-a".to_owned(),
                    flow_weight: 1,
                    demand_bps: 100,
                },
                TenantResourceFlow {
                    flow_id: "flow-b".to_owned(),
                    tenant_id: "tenant-b".to_owned(),
                    flow_weight: 1,
                    demand_bps: 100,
                },
            ],
            paths: vec![
                TenantResourcePath {
                    path_id: "path-a".to_owned(),
                    flow_id: "flow-a".to_owned(),
                    path_ceiling_bps: 100,
                    split_weight: 1,
                    capacity_domain_ids: vec!["shared".to_owned()],
                },
                TenantResourcePath {
                    path_id: "path-b".to_owned(),
                    flow_id: "flow-b".to_owned(),
                    path_ceiling_bps: 100,
                    split_weight: 1,
                    capacity_domain_ids: vec!["shared".to_owned()],
                },
            ],
            resources: vec![CapacityResource {
                resource_id: "shared".to_owned(),
                declared_capacity_bps: 100,
                measured_background_load_bps: 0,
                committed_reservations_bps: 0,
                safety_margin_bps: 0,
                evidence: evidence(&topology),
            }],
        }
    }

    fn event(
        id: &str,
        source: &str,
        sequence: u64,
        occurred_at: u64,
        observed_at: u64,
        effective_at: u64,
        kind: TransitionEventKind,
    ) -> TransitionEvent {
        TransitionEvent {
            event_id: id.to_owned(),
            source_id: source.to_owned(),
            source_sequence: sequence,
            source_sha256: "d".repeat(64),
            topology_epoch: 1,
            topology_sha256: "b".repeat(64),
            occurred_at_unix_ms: occurred_at,
            observed_at_unix_ms: observed_at,
            effective_at_unix_ms: effective_at,
            state_snapshot_sha256: "e".repeat(64),
            kind,
        }
    }

    #[test]
    fn identical_event_stream_and_ticks_produce_identical_reports() {
        let events = vec![
            event(
                "down-a",
                "gateway",
                1,
                1_100,
                1_100,
                1_100,
                TransitionEventKind::PathHealthChanged {
                    path_id: "path-a".to_owned(),
                    health: PathHealth::Down,
                },
            ),
            event(
                "up-a",
                "gateway",
                2,
                1_200,
                1_200,
                1_200,
                TransitionEventKind::PathHealthChanged {
                    path_id: "path-a".to_owned(),
                    health: PathHealth::Up,
                },
            ),
        ];
        let first = evaluate_transition_trace(&snapshot(), &events, &[1_000, 1_100, 1_200])
            .expect("valid transition trace");
        let second = evaluate_transition_trace(&snapshot(), &events, &[1_000, 1_100, 1_200])
            .expect("same frozen inputs");
        assert_eq!(first, second);
    }

    #[test]
    fn future_observed_event_cannot_influence_an_earlier_tick() {
        let events = vec![event(
            "late-observation",
            "gateway",
            1,
            1_100,
            1_500,
            1_100,
            TransitionEventKind::PathHealthChanged {
                path_id: "path-a".to_owned(),
                health: PathHealth::Down,
            },
        )];
        let report = evaluate_transition_trace(&snapshot(), &events, &[1_200, 1_500])
            .expect("future observation is deferred");
        assert_eq!(report.ticks[0].path_down_count, 0);
        assert_eq!(report.ticks[0].applied_event_ids.len(), 0);
        assert_eq!(report.ticks[1].path_down_count, 1);
        assert_eq!(report.event_dispositions[0].disposition, EventDisposition::Applied);
    }

    #[test]
    fn future_effective_event_waits_until_its_effective_time() {
        let events = vec![event(
            "future-effective",
            "gateway",
            1,
            1_100,
            1_100,
            1_400,
            TransitionEventKind::PathHealthChanged {
                path_id: "path-a".to_owned(),
                health: PathHealth::Down,
            },
        )];
        let report = evaluate_transition_trace(&snapshot(), &events, &[1_200, 1_400])
            .expect("future effective event deferred");
        assert_eq!(report.ticks[0].path_down_count, 0);
        assert_eq!(report.ticks[1].path_down_count, 1);
    }

    #[test]
    fn source_sequence_gap_is_deferred_then_applied_in_order() {
        let events = vec![
            event(
                "seq-two",
                "gateway",
                2,
                1_200,
                1_200,
                1_200,
                TransitionEventKind::PathHealthChanged {
                    path_id: "path-a".to_owned(),
                    health: PathHealth::Down,
                },
            ),
            event(
                "seq-one",
                "gateway",
                1,
                1_100,
                1_300,
                1_300,
                TransitionEventKind::PathHealthChanged {
                    path_id: "path-b".to_owned(),
                    health: PathHealth::Down,
                },
            ),
        ];
        let report = evaluate_transition_trace(&snapshot(), &events, &[1_200, 1_300])
            .expect("sequence gap resolves when predecessor is observed");
        assert_eq!(report.ticks[0].applied_event_ids.len(), 0);
        assert_eq!(report.ticks[1].applied_event_ids.len(), 2);
        assert_eq!(report.applied_event_count, 2);
    }

    #[test]
    fn duplicate_event_ids_are_rejected_without_double_application() {
        let events = vec![
            event(
                "duplicate",
                "gateway",
                1,
                1_100,
                1_100,
                1_100,
                TransitionEventKind::PathHealthChanged {
                    path_id: "path-a".to_owned(),
                    health: PathHealth::Down,
                },
            ),
            event(
                "duplicate",
                "gateway-2",
                1,
                1_100,
                1_100,
                1_100,
                TransitionEventKind::PathHealthChanged {
                    path_id: "path-b".to_owned(),
                    health: PathHealth::Down,
                },
            ),
        ];
        let report = evaluate_transition_trace(&snapshot(), &events, &[1_200])
            .expect("duplicate IDs are classified");
        assert_eq!(report.applied_event_count, 0);
        assert_eq!(report.rejected_event_count, 2);
        assert!(report.event_dispositions.iter().all(|record| {
            record.disposition == EventDisposition::RejectedDuplicateEventId
        }));
    }

    #[test]
    fn invalid_source_digest_is_rejected() {
        let mut invalid = event(
            "invalid-digest",
            "gateway",
            1,
            1_100,
            1_100,
            1_100,
            TransitionEventKind::PathHealthChanged {
                path_id: "path-a".to_owned(),
                health: PathHealth::Down,
            },
        );
        invalid.source_sha256 = "not-a-digest".to_owned();
        let report = evaluate_transition_trace(&snapshot(), &[invalid], &[1_200])
            .expect("invalid digest becomes a disposition");
        assert_eq!(
            report.event_dispositions[0].disposition,
            EventDisposition::RejectedInvalidDigest
        );
    }

    #[test]
    fn topology_epoch_advance_invalidates_old_epoch_events() {
        let events = vec![
            event(
                "epoch-advance",
                "topology",
                1,
                1_100,
                1_100,
                1_100,
                TransitionEventKind::TopologyEpochAdvanced {
                    next_epoch: 2,
                    next_topology_sha256: "f".repeat(64),
                },
            ),
            TransitionEvent {
                event_id: "old-epoch-event".to_owned(),
                source_id: "gateway".to_owned(),
                source_sequence: 1,
                source_sha256: "d".repeat(64),
                topology_epoch: 1,
                topology_sha256: "b".repeat(64),
                occurred_at_unix_ms: 1_200,
                observed_at_unix_ms: 1_200,
                effective_at_unix_ms: 1_200,
                state_snapshot_sha256: "a".repeat(64),
                kind: TransitionEventKind::PathHealthChanged {
                    path_id: "path-a".to_owned(),
                    health: PathHealth::Down,
                },
            },
        ];
        let report = evaluate_transition_trace(&snapshot(), &events, &[1_300])
            .expect("epoch transitions are bounded");
        assert_eq!(report.final_topology_epoch, 2);
        assert_eq!(report.ticks[0].topology_sha256, "f".repeat(64));
        assert_eq!(
            report.event_dispositions[1].disposition,
            EventDisposition::RejectedTopologyEpoch
        );
    }

    #[test]
    fn down_path_records_service_floor_outage_and_recovery() {
        let events = vec![
            event(
                "path-a-down",
                "gateway",
                1,
                1_100,
                1_100,
                1_100,
                TransitionEventKind::PathHealthChanged {
                    path_id: "path-a".to_owned(),
                    health: PathHealth::Down,
                },
            ),
            event(
                "path-a-up",
                "gateway",
                2,
                1_500,
                1_500,
                1_500,
                TransitionEventKind::PathHealthChanged {
                    path_id: "path-a".to_owned(),
                    health: PathHealth::Up,
                },
            ),
        ];
        let report = evaluate_transition_trace(&snapshot(), &events, &[1_000, 1_100, 1_300, 1_500])
            .expect("outage and recovery trace");
        assert!(report.ticks[1]
            .service_floor_unmet_tenant_ids
            .contains(&"tenant-a".to_owned()));
        let tenant_a = report.ticks[3]
            .tenant_outages
            .iter()
            .find(|item| item.tenant_id == "tenant-a")
            .expect("tenant a outage state");
        assert_eq!(tenant_a.last_recovery_duration_ms, Some(400));
        assert_eq!(tenant_a.floor_status, ServiceFloorStatus::Met);
    }

    #[test]
    fn shared_resource_capacity_is_never_overcommitted_during_failover() {
        let events = vec![event(
            "path-a-down",
            "gateway",
            1,
            1_100,
            1_100,
            1_100,
            TransitionEventKind::PathHealthChanged {
                path_id: "path-a".to_owned(),
                health: PathHealth::Down,
            },
        )];
        let report = evaluate_transition_trace(&snapshot(), &events, &[1_100])
            .expect("failover rate allocation");
        assert!(report.ticks[0].resource_usage.iter().all(|usage| {
            usage.allocated_rate_bps <= usage.residual_capacity_bps
        }));
    }

    #[test]
    fn decision_ticks_must_be_strictly_monotonic() {
        assert_eq!(
            evaluate_transition_trace(&snapshot(), &[], &[1_000, 1_000]).unwrap_err(),
            TransitionError::NonMonotonicDecisionTicks
        );
    }
}
