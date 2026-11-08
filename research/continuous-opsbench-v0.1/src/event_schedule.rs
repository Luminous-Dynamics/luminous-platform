//! Deterministic event-schedule evaluator for durable and ephemeral attention.
//!
//! This is an offline evaluator over an immutable event schedule. It reuses
//! the existing WatchSpec, AuthoritySnapshot and EventEnvelope contracts; it
//! is not a broker, inbox, subscription, scheduler or production delivery
//! implementation. It preserves incident identity, source sequence, event /
//! delivery / processing time, and explicit unknown/fail-closed dispositions.

use crate::watch_probe::{AuthoritySnapshot, EventEnvelope, WatchSpec};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventPersistence {
    Persistent,
    Ephemeral,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledAttentionEvent {
    pub envelope: EventEnvelope,
    /// Digest of the canonical event payload bytes. This is an identity
    /// commitment supplied by the harness, not producer authentication.
    pub payload_digest: String,
    pub incident_id: String,
    pub persistence: EventPersistence,
    /// When the observed condition cleared. For ephemeral events this may be
    /// before processing; the event-time match remains durable evidence.
    pub condition_cleared_at_tick: Option<u64>,
    pub processed_at_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventScheduleConfig {
    pub max_detection_latency_ticks: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventScheduleStatus {
    NotStarted,
    Quiet,
    Completed,
    Expired,
    DeadlineMissed,
    DeliveryGap,
    EventIdentityCollision,
    SequenceCollision,
    OutOfOrder,
    AuthorityRevokedOrExpired,
    WakeBudgetExhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventRecordDisposition {
    NotInWatchWindow,
    ForeignSource,
    RejectedScope,
    DuplicateSuppressed,
    Irrelevant,
    LateEscalated,
    CoalescedSameIncident,
    Woke,
    DeliveryGap,
    EventIdentityCollision,
    SequenceCollision,
    OutOfOrder,
    AuthorityRevokedOrExpired,
    WatchExpired,
    WakeBudgetExhausted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventRecord {
    pub event_id: String,
    pub incident_id: String,
    pub source_sequence: u64,
    pub event_tick: u64,
    pub delivered_tick: u64,
    pub processed_tick: u64,
    pub event_to_delivery_ticks: u64,
    pub event_to_processing_ticks: u64,
    pub disposition: EventRecordDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncidentWake {
    pub incident_id: String,
    pub first_event_id: String,
    /// Includes every unique qualifying event coalesced into this wake.
    pub event_ids: Vec<String>,
    pub persistence: EventPersistence,
    pub event_tick: u64,
    pub delivered_tick: u64,
    pub processed_tick: u64,
    pub event_to_processing_ticks: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventScheduleMetrics {
    pub scheduled_events: usize,
    pub processed_source_events: usize,
    pub foreign_source_events: usize,
    pub rejected_scope_events: usize,
    pub duplicate_deliveries: usize,
    pub irrelevant_events: usize,
    pub late_events: usize,
    pub distinct_incidents: usize,
    pub wakeups: usize,
    pub coalesced_events: usize,
    pub delivery_gaps: usize,
    pub identity_collisions: usize,
    pub sequence_collisions: usize,
    pub out_of_order_events: usize,
    pub authority_denials: usize,
    pub wake_budget_exhaustions: usize,
    pub unprocessed_events: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventScheduleResult {
    pub status: EventScheduleStatus,
    pub metrics: EventScheduleMetrics,
    pub records: Vec<EventRecord>,
    pub wakes: Vec<IncidentWake>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventScheduleError {
    InvalidWatch,
    InvalidAuthority,
    InvalidConfig,
    InvalidEventIdentity,
    InvalidEventTime,
    ProcessingTimeRegression,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn validate_inputs(
    watch: &WatchSpec,
    authority: &AuthoritySnapshot,
    config: &EventScheduleConfig,
) -> Result<(), EventScheduleError> {
    if !non_empty(&watch.watch_id)
        || !non_empty(&watch.tenant_id)
        || !non_empty(&watch.target_id)
        || !non_empty(&watch.purpose_id)
        || !non_empty(&watch.source_id)
        || !non_empty(&watch.event_type)
        || watch.first_sequence == 0
        || watch.starts_at_tick >= watch.expires_at_tick
        || watch.max_wakeups == 0
        || watch.max_observation_age_ticks == 0
    {
        return Err(EventScheduleError::InvalidWatch);
    }
    if authority.epoch == 0
        || !non_empty(&authority.tenant_id)
        || !non_empty(&authority.target_id)
        || !non_empty(&authority.purpose_id)
        || authority.valid_from_tick >= authority.expires_at_tick
    {
        return Err(EventScheduleError::InvalidAuthority);
    }
    if config.max_detection_latency_ticks == 0 {
        return Err(EventScheduleError::InvalidConfig);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EventIdentity {
    event_id: String,
    source_id: String,
    sequence: u64,
    event_type: String,
    tenant_id: String,
    target_id: String,
    purpose_id: String,
    condition_met: bool,
    observed_at_tick: u64,
    payload_digest: String,
    incident_id: String,
    persistence: EventPersistence,
    condition_cleared_at_tick: Option<u64>,
}

impl From<&ScheduledAttentionEvent> for EventIdentity {
    fn from(event: &ScheduledAttentionEvent) -> Self {
        Self {
            event_id: event.envelope.event_id.clone(),
            source_id: event.envelope.source_id.clone(),
            sequence: event.envelope.sequence,
            event_type: event.envelope.event_type.clone(),
            tenant_id: event.envelope.tenant_id.clone(),
            target_id: event.envelope.target_id.clone(),
            purpose_id: event.envelope.purpose_id.clone(),
            condition_met: event.envelope.condition_met,
            observed_at_tick: event.envelope.observed_at_tick,
            payload_digest: event.payload_digest.clone(),
            incident_id: event.incident_id.clone(),
            persistence: event.persistence,
            condition_cleared_at_tick: event.condition_cleared_at_tick,
        }
    }
}

fn record_for(event: &ScheduledAttentionEvent, disposition: EventRecordDisposition) -> EventRecord {
    EventRecord {
        event_id: event.envelope.event_id.clone(),
        incident_id: event.incident_id.clone(),
        source_sequence: event.envelope.sequence,
        event_tick: event.envelope.observed_at_tick,
        delivered_tick: event.envelope.delivered_at_tick,
        processed_tick: event.processed_at_tick,
        event_to_delivery_ticks: event
            .envelope
            .delivered_at_tick
            .saturating_sub(event.envelope.observed_at_tick),
        event_to_processing_ticks: event
            .processed_at_tick
            .saturating_sub(event.envelope.observed_at_tick),
        disposition,
    }
}

/// Evaluate one frozen delivery schedule. Events are processed in supplied
/// order; source sequence and processing-time monotonicity are independently
/// checked. Exact duplicate delivery receives no second wake or credit.
///
/// Event-time predicate truth is carried in EventEnvelope.condition_met.
/// For an ephemeral condition, a later clear does not erase a timely event
/// that occurred inside the watch window. A persistent condition already true
/// before watch activation can wake only if it remains uncleared at processing.
pub fn evaluate_event_schedule(
    watch: &WatchSpec,
    authority: &AuthoritySnapshot,
    config: &EventScheduleConfig,
    events: &[ScheduledAttentionEvent],
    current_tick: u64,
) -> Result<EventScheduleResult, EventScheduleError> {
    validate_inputs(watch, authority, config)?;
    let mut result = EventScheduleResult {
        status: if current_tick < watch.starts_at_tick {
            EventScheduleStatus::NotStarted
        } else {
            EventScheduleStatus::Quiet
        },
        metrics: EventScheduleMetrics {
            scheduled_events: events.len(),
            ..EventScheduleMetrics::default()
        },
        records: Vec::new(),
        wakes: Vec::new(),
    };
    if current_tick < watch.starts_at_tick {
        result.metrics.unprocessed_events = events.len();
        return Ok(result);
    }

    let mut next_sequence = watch.first_sequence;
    let mut seen_ids: BTreeMap<String, EventIdentity> = BTreeMap::new();
    let mut seen_sequences: BTreeMap<u64, EventIdentity> = BTreeMap::new();
    let mut incident_wake_index: BTreeMap<String, usize> = BTreeMap::new();
    let mut seen_incidents = BTreeSet::new();
    let mut previous_processed_tick = None;
    let mut terminal_stop = false;

    for (index, scheduled) in events.iter().enumerate() {
        let envelope = &scheduled.envelope;
        if previous_processed_tick.is_some_and(|previous| scheduled.processed_at_tick < previous) {
            return Err(EventScheduleError::ProcessingTimeRegression);
        }
        previous_processed_tick = Some(scheduled.processed_at_tick);

        if envelope.observed_at_tick > envelope.delivered_at_tick
            || envelope.delivered_at_tick > scheduled.processed_at_tick
            || scheduled.processed_at_tick > current_tick
            || scheduled
                .condition_cleared_at_tick
                .is_some_and(|cleared| {
                    cleared < envelope.observed_at_tick || cleared > current_tick
                })
        {
            return Err(EventScheduleError::InvalidEventTime);
        }
        if !non_empty(&envelope.event_id)
            || envelope.sequence == 0
            || !non_empty(&scheduled.incident_id)
            || !valid_sha256(&scheduled.payload_digest)
        {
            return Err(EventScheduleError::InvalidEventIdentity);
        }

        if envelope.source_id != watch.source_id {
            result.metrics.foreign_source_events += 1;
            result.records.push(record_for(
                scheduled,
                EventRecordDisposition::ForeignSource,
            ));
            continue;
        }

        if scheduled.processed_at_tick < watch.starts_at_tick {
            result.records.push(record_for(
                scheduled,
                EventRecordDisposition::NotInWatchWindow,
            ));
            continue;
        }
        if scheduled.processed_at_tick >= watch.expires_at_tick {
            result.records.push(record_for(
                scheduled,
                EventRecordDisposition::WatchExpired,
            ));
            result.status = EventScheduleStatus::Expired;
            terminal_stop = true;
            result.metrics.unprocessed_events = events.len() - index - 1;
            break;
        }

        // A dormant watch cannot retain access after authority revocation or
        // expiry. Check at the time the event is actually processed.
        if !authority.current_for(watch, scheduled.processed_at_tick) {
            result.metrics.authority_denials += 1;
            result.records.push(record_for(
                scheduled,
                EventRecordDisposition::AuthorityRevokedOrExpired,
            ));
            result.status = EventScheduleStatus::AuthorityRevokedOrExpired;
            terminal_stop = true;
            result.metrics.unprocessed_events = events.len() - index - 1;
            break;
        }

        let identity = EventIdentity::from(scheduled);
        if let Some(previous_identity) = seen_ids.get(&envelope.event_id) {
            if previous_identity == &identity {
                result.metrics.duplicate_deliveries += 1;
                result.records.push(record_for(
                    scheduled,
                    EventRecordDisposition::DuplicateSuppressed,
                ));
                continue;
            }
            result.metrics.identity_collisions += 1;
            result.records.push(record_for(
                scheduled,
                EventRecordDisposition::EventIdentityCollision,
            ));
            result.status = EventScheduleStatus::EventIdentityCollision;
            terminal_stop = true;
            result.metrics.unprocessed_events = events.len() - index - 1;
            break;
        }

        if let Some(previous_identity) = seen_sequences.get(&envelope.sequence) {
            if previous_identity != &identity {
                result.metrics.sequence_collisions += 1;
                result.records.push(record_for(
                    scheduled,
                    EventRecordDisposition::SequenceCollision,
                ));
                result.status = EventScheduleStatus::SequenceCollision;
                terminal_stop = true;
                result.metrics.unprocessed_events = events.len() - index - 1;
                break;
            }
        }

        if envelope.sequence < next_sequence {
            result.metrics.out_of_order_events += 1;
            result.records.push(record_for(
                scheduled,
                EventRecordDisposition::OutOfOrder,
            ));
            result.status = EventScheduleStatus::OutOfOrder;
            terminal_stop = true;
            result.metrics.unprocessed_events = events.len() - index - 1;
            break;
        }
        if envelope.sequence > next_sequence {
            result.metrics.delivery_gaps += 1;
            result.records.push(record_for(
                scheduled,
                EventRecordDisposition::DeliveryGap,
            ));
            result.status = EventScheduleStatus::DeliveryGap;
            terminal_stop = true;
            result.metrics.unprocessed_events = events.len() - index - 1;
            break;
        }

        seen_ids.insert(envelope.event_id.clone(), identity.clone());
        seen_sequences.insert(envelope.sequence, identity);
        next_sequence = next_sequence.saturating_add(1);
        result.metrics.processed_source_events += 1;

        let in_scope = envelope.tenant_id == watch.tenant_id
            && envelope.target_id == watch.target_id
            && envelope.purpose_id == watch.purpose_id;
        if !in_scope {
            result.metrics.rejected_scope_events += 1;
            result.records.push(record_for(
                scheduled,
                EventRecordDisposition::RejectedScope,
            ));
            continue;
        }

        if envelope.event_type != watch.event_type || !envelope.condition_met {
            result.metrics.irrelevant_events += 1;
            result.records.push(record_for(
                scheduled,
                EventRecordDisposition::Irrelevant,
            ));
            continue;
        }

        let occurred_during_watch = envelope.observed_at_tick >= watch.starts_at_tick
            && envelope.observed_at_tick < watch.expires_at_tick;
        let persistent_from_before_watch = scheduled.persistence == EventPersistence::Persistent
            && envelope.observed_at_tick < watch.starts_at_tick
            && scheduled
                .condition_cleared_at_tick
                .is_none_or(|cleared| cleared > scheduled.processed_at_tick);

        if !occurred_during_watch && !persistent_from_before_watch {
            result.metrics.irrelevant_events += 1;
            result.records.push(record_for(
                scheduled,
                EventRecordDisposition::NotInWatchWindow,
            ));
            continue;
        }

        let event_to_processing = scheduled
            .processed_at_tick
            .saturating_sub(envelope.observed_at_tick);
        if event_to_processing > config.max_detection_latency_ticks {
            result.metrics.late_events += 1;
            result.records.push(record_for(
                scheduled,
                EventRecordDisposition::LateEscalated,
            ));
            result.status = EventScheduleStatus::DeadlineMissed;
            continue;
        }

        if seen_incidents.insert(scheduled.incident_id.clone()) {
            result.metrics.distinct_incidents += 1;
        }

        if let Some(wake_index) = incident_wake_index.get(&scheduled.incident_id).copied() {
            result.wakes[wake_index]
                .event_ids
                .push(envelope.event_id.clone());
            result.metrics.coalesced_events += 1;
            result.records.push(record_for(
                scheduled,
                EventRecordDisposition::CoalescedSameIncident,
            ));
            continue;
        }

        if result.metrics.wakeups >= watch.max_wakeups as usize {
            result.metrics.wake_budget_exhaustions += 1;
            result.records.push(record_for(
                scheduled,
                EventRecordDisposition::WakeBudgetExhausted,
            ));
            result.status = EventScheduleStatus::WakeBudgetExhausted;
            terminal_stop = true;
            result.metrics.unprocessed_events = events.len() - index - 1;
            break;
        }

        let wake = IncidentWake {
            incident_id: scheduled.incident_id.clone(),
            first_event_id: envelope.event_id.clone(),
            event_ids: vec![envelope.event_id.clone()],
            persistence: scheduled.persistence,
            event_tick: envelope.observed_at_tick,
            delivered_tick: envelope.delivered_at_tick,
            processed_tick: scheduled.processed_at_tick,
            event_to_processing_ticks: event_to_processing,
        };
        incident_wake_index.insert(scheduled.incident_id.clone(), result.wakes.len());
        result.wakes.push(wake);
        result.metrics.wakeups += 1;
        result.records.push(record_for(
            scheduled,
            EventRecordDisposition::Woke,
        ));
        if result.status != EventScheduleStatus::DeadlineMissed {
            result.status = EventScheduleStatus::Completed;
        }
    }

    if !terminal_stop
        && !authority.current_for(watch, current_tick)
        && current_tick < watch.expires_at_tick
    {
        result.status = EventScheduleStatus::AuthorityRevokedOrExpired;
    } else if !terminal_stop && current_tick >= watch.expires_at_tick {
        if result.metrics.wakeups == 0 && result.status != EventScheduleStatus::DeadlineMissed {
            result.status = EventScheduleStatus::Expired;
        } else if result.metrics.wakeups > 0 && result.status != EventScheduleStatus::DeadlineMissed {
            result.status = EventScheduleStatus::Completed;
        }
    } else if !terminal_stop
        && result.status == EventScheduleStatus::Quiet
        && result.metrics.wakeups > 0
    {
        result.status = EventScheduleStatus::Completed;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn watch() -> WatchSpec {
        WatchSpec {
            watch_id: "watch-event-schedule".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            source_id: "event-source-a".into(),
            event_type: "service-degraded".into(),
            first_sequence: 1,
            starts_at_tick: 2,
            expires_at_tick: 30,
            max_wakeups: 4,
            max_probe_duration_ticks: 3,
            max_observation_age_ticks: 8,
        }
    }

    fn authority() -> AuthoritySnapshot {
        AuthoritySnapshot {
            epoch: 1,
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            valid_from_tick: 0,
            expires_at_tick: 30,
            revoked_at_tick: None,
        }
    }

    fn config() -> EventScheduleConfig {
        EventScheduleConfig {
            max_detection_latency_ticks: 5,
        }
    }

    fn scheduled(
        sequence: u64,
        event_id: &str,
        incident_id: &str,
        event_tick: u64,
        delivered_tick: u64,
        processed_tick: u64,
        persistence: EventPersistence,
        condition_met: bool,
    ) -> ScheduledAttentionEvent {
        ScheduledAttentionEvent {
            envelope: EventEnvelope {
                event_id: event_id.into(),
                source_id: "event-source-a".into(),
                sequence,
                event_type: "service-degraded".into(),
                tenant_id: "tenant-a".into(),
                target_id: "service-a".into(),
                purpose_id: "incident-diagnosis".into(),
                condition_met,
                observed_at_tick: event_tick,
                delivered_at_tick: delivered_tick,
            },
            payload_digest: format!("sha256:{:064x}", sequence),
            incident_id: incident_id.into(),
            persistence,
            condition_cleared_at_tick: None,
            processed_at_tick: processed_tick,
        }
    }

    fn evaluate(
        events: &[ScheduledAttentionEvent],
        watch: &WatchSpec,
        authority: &AuthoritySnapshot,
    ) -> EventScheduleResult {
        evaluate_event_schedule(watch, authority, &config(), events, 20).unwrap()
    }

    #[test]
    fn persistent_event_wakes_once_and_second_event_same_incident_coalesces() {
        let events = [
            scheduled(1, "event-1", "incident-a", 4, 5, 5, EventPersistence::Persistent, true),
            scheduled(2, "event-2", "incident-a", 6, 6, 7, EventPersistence::Persistent, true),
        ];
        let result = evaluate(&events, &watch(), &authority());
        assert_eq!(result.status, EventScheduleStatus::Completed);
        assert_eq!(result.metrics.wakeups, 1);
        assert_eq!(result.metrics.coalesced_events, 1);
        assert_eq!(result.wakes[0].event_ids, vec!["event-1", "event-2"]);
    }

    #[test]
    fn second_incident_wakes_even_while_first_incident_is_still_active() {
        let events = [
            scheduled(1, "event-1", "incident-a", 4, 5, 5, EventPersistence::Persistent, true),
            scheduled(2, "event-2", "incident-b", 6, 6, 7, EventPersistence::Persistent, true),
        ];
        let result = evaluate(&events, &watch(), &authority());
        assert_eq!(result.metrics.wakeups, 2);
        assert_eq!(result.metrics.distinct_incidents, 2);
        assert_eq!(result.wakes[0].incident_id, "incident-a");
        assert_eq!(result.wakes[1].incident_id, "incident-b");
    }

    #[test]
    fn ephemeral_event_is_not_lost_when_condition_clears_before_processing() {
        let mut event = scheduled(1, "ephemeral-1", "incident-ephemeral", 4, 5, 7, EventPersistence::Ephemeral, true);
        event.condition_cleared_at_tick = Some(6);
        let result = evaluate(&[event], &watch(), &authority());
        assert_eq!(result.metrics.wakeups, 1);
        assert_eq!(result.wakes[0].persistence, EventPersistence::Ephemeral);
        assert_eq!(result.wakes[0].event_to_processing_ticks, 3);
    }

    #[test]
    fn event_outside_detection_deadline_is_recorded_and_not_woken() {
        let event = scheduled(1, "late-1", "incident-late", 3, 7, 10, EventPersistence::Ephemeral, true);
        let result = evaluate(&[event], &watch(), &authority());
        assert_eq!(result.status, EventScheduleStatus::DeadlineMissed);
        assert_eq!(result.metrics.late_events, 1);
        assert_eq!(result.metrics.wakeups, 0);
        assert_eq!(result.records[0].disposition, EventRecordDisposition::LateEscalated);
    }

    #[test]
    fn exact_duplicate_delivery_is_suppressed_without_second_wake() {
        let original = scheduled(1, "event-1", "incident-a", 4, 5, 5, EventPersistence::Persistent, true);
        let mut duplicate = original.clone();
        duplicate.processed_at_tick = 6;
        let result = evaluate(&[original, duplicate], &watch(), &authority());
        assert_eq!(result.metrics.wakeups, 1);
        assert_eq!(result.metrics.duplicate_deliveries, 1);
        assert_eq!(result.records[1].disposition, EventRecordDisposition::DuplicateSuppressed);
    }


    #[test]
    fn same_event_id_and_sequence_with_different_payload_digest_is_collision() {
        let first = scheduled(1, "event-one", "incident-a", 4, 5, 5, EventPersistence::Persistent, true);
        let mut changed = first.clone();
        changed.payload_digest = format!("sha256:{:064x}", 99);
        changed.processed_at_tick = 6;
        let result = evaluate(&[first, changed], &watch(), &authority());
        assert_eq!(result.status, EventScheduleStatus::EventIdentityCollision);
        assert_eq!(result.metrics.identity_collisions, 1);
        assert_eq!(result.metrics.duplicate_deliveries, 0);
    }

    #[test]
    fn late_event_does_not_erase_miss_when_later_event_wakes() {
        let late = scheduled(1, "late-1", "incident-late", 3, 7, 10, EventPersistence::Ephemeral, true);
        let timely = scheduled(2, "timely-2", "incident-timely", 12, 12, 13, EventPersistence::Persistent, true);
        let result = evaluate(&[late, timely], &watch(), &authority());
        assert_eq!(result.status, EventScheduleStatus::DeadlineMissed);
        assert_eq!(result.metrics.late_events, 1);
        assert_eq!(result.metrics.wakeups, 1);
        assert_eq!(result.records[1].disposition, EventRecordDisposition::Woke);
    }

    #[test]
    fn same_event_id_with_different_sequence_is_identity_collision() {
        let first = scheduled(1, "event-reused", "incident-a", 4, 5, 5, EventPersistence::Persistent, true);
        let second = scheduled(2, "event-reused", "incident-b", 6, 6, 7, EventPersistence::Persistent, true);
        let result = evaluate(&[first, second], &watch(), &authority());
        assert_eq!(result.status, EventScheduleStatus::EventIdentityCollision);
        assert_eq!(result.metrics.identity_collisions, 1);
        assert_eq!(result.metrics.unprocessed_events, 0);
        assert_eq!(result.metrics.wakeups, 1);
    }

    #[test]
    fn same_sequence_with_different_event_id_is_sequence_collision() {
        let first = scheduled(1, "event-one", "incident-a", 4, 5, 5, EventPersistence::Persistent, true);
        let second = scheduled(1, "event-two", "incident-b", 6, 6, 7, EventPersistence::Persistent, true);
        let result = evaluate(&[first, second], &watch(), &authority());
        assert_eq!(result.status, EventScheduleStatus::SequenceCollision);
        assert_eq!(result.metrics.sequence_collisions, 1);
        assert_eq!(result.metrics.wakeups, 1);
    }

    #[test]
    fn sequence_gap_stops_schedule_and_preserves_unprocessed_count() {
        let first = scheduled(1, "event-one", "incident-a", 4, 5, 5, EventPersistence::Persistent, true);
        let third = scheduled(3, "event-three", "incident-b", 6, 7, 7, EventPersistence::Persistent, true);
        let fourth = scheduled(4, "event-four", "incident-c", 8, 9, 9, EventPersistence::Persistent, true);
        let result = evaluate(&[first, third, fourth], &watch(), &authority());
        assert_eq!(result.status, EventScheduleStatus::DeliveryGap);
        assert_eq!(result.metrics.delivery_gaps, 1);
        assert_eq!(result.metrics.unprocessed_events, 1);
        assert_eq!(result.metrics.wakeups, 1);
    }

    #[test]
    fn out_of_order_old_sequence_is_not_credited_as_duplicate() {
        let first = scheduled(2, "event-two", "incident-a", 4, 5, 5, EventPersistence::Persistent, true);
        let second = scheduled(1, "event-one", "incident-b", 6, 6, 7, EventPersistence::Persistent, true);
        let mut changed_watch = watch();
        changed_watch.first_sequence = 2;
        let result = evaluate(&[first, second], &changed_watch, &authority());
        assert_eq!(result.status, EventScheduleStatus::OutOfOrder);
        assert_eq!(result.metrics.out_of_order_events, 1);
    }

    #[test]
    fn dormant_revocation_prevents_event_wake() {
        let event = scheduled(1, "event-1", "incident-a", 4, 5, 8, EventPersistence::Persistent, true);
        let mut revoked = authority();
        revoked.revoked_at_tick = Some(7);
        let result = evaluate(&[event], &watch(), &revoked);
        assert_eq!(result.status, EventScheduleStatus::AuthorityRevokedOrExpired);
        assert_eq!(result.metrics.authority_denials, 1);
        assert_eq!(result.metrics.wakeups, 0);
    }

    #[test]
    fn persistent_condition_existing_before_watch_activation_can_wake_if_still_active() {
        let mut event = scheduled(1, "existing-1", "incident-existing", 1, 2, 3, EventPersistence::Persistent, true);
        event.condition_cleared_at_tick = Some(6);
        let result = evaluate(&[event], &watch(), &authority());
        assert_eq!(result.metrics.wakeups, 1);
        assert_eq!(result.records[0].disposition, EventRecordDisposition::Woke);
    }

    #[test]
    fn persistent_condition_cleared_before_processing_does_not_wake_from_pre_watch_history() {
        let mut event = scheduled(1, "existing-1", "incident-existing", 1, 2, 7, EventPersistence::Persistent, true);
        event.condition_cleared_at_tick = Some(6);
        let result = evaluate(&[event], &watch(), &authority());
        assert_eq!(result.metrics.wakeups, 0);
        assert_eq!(result.records[0].disposition, EventRecordDisposition::NotInWatchWindow);
    }

    #[test]
    fn watch_wakeup_budget_exhaustion_is_explicit() {
        let events = [
            scheduled(1, "event-1", "incident-a", 4, 5, 5, EventPersistence::Persistent, true),
            scheduled(2, "event-2", "incident-b", 6, 6, 7, EventPersistence::Persistent, true),
        ];
        let mut bounded = watch();
        bounded.max_wakeups = 1;
        let result = evaluate(&events, &bounded, &authority());
        assert_eq!(result.status, EventScheduleStatus::WakeBudgetExhausted);
        assert_eq!(result.metrics.wakeups, 1);
        assert_eq!(result.metrics.wake_budget_exhaustions, 1);
    }

    #[test]
    fn wrong_tenant_event_consumes_source_cursor_but_never_wakes() {
        let mut wrong_scope = scheduled(1, "wrong-tenant", "incident-a", 4, 5, 5, EventPersistence::Persistent, true);
        wrong_scope.envelope.tenant_id = "tenant-b".into();
        let next = scheduled(2, "event-2", "incident-b", 6, 7, 7, EventPersistence::Persistent, true);
        let result = evaluate(&[wrong_scope, next], &watch(), &authority());
        assert_eq!(result.metrics.rejected_scope_events, 1);
        assert_eq!(result.metrics.wakeups, 1);
        assert_eq!(result.records[0].disposition, EventRecordDisposition::RejectedScope);
    }

    #[test]
    fn missing_payload_digest_is_rejected() {
        let mut event = scheduled(1, "event-1", "incident-a", 4, 5, 5, EventPersistence::Persistent, true);
        event.payload_digest.clear();
        assert_eq!(
            evaluate_event_schedule(&watch(), &authority(), &config(), &[event], 20),
            Err(EventScheduleError::InvalidEventIdentity)
        );
    }

    #[test]
    fn event_processing_time_regression_is_rejected() {
        let first = scheduled(1, "event-1", "incident-a", 4, 5, 7, EventPersistence::Persistent, true);
        let second = scheduled(2, "event-2", "incident-b", 6, 7, 6, EventPersistence::Persistent, true);
        assert_eq!(
            evaluate_event_schedule(&watch(), &authority(), &config(), &[first, second], 20),
            Err(EventScheduleError::ProcessingTimeRegression)
        );
    }
}
