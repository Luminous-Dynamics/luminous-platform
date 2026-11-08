//! Deterministic event-schedule evaluator for sustained attention.
//!
//! This is an offline evaluator over a frozen source-qualified schedule. It
//! reuses WatchSpec and AuthoritySnapshot and does not implement transport,
//! subscriptions, inboxes, acknowledgement, retry, timers or mutation. Events
//! are processed in source sequence order; out-of-order deliveries are buffered
//! until gaps fill or the declared gap timeout expires.

use std::collections::{BTreeMap, BTreeSet};

use crate::watch_probe::{AuthoritySnapshot, WatchSpec};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventPriority {
    Normal,
    Critical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventPersistence {
    Persistent,
    Ephemeral,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledAttentionEvent {
    pub source_id: String,
    pub source_epoch: u64,
    pub event_id: String,
    pub payload_digest: String,
    pub sequence: u64,
    pub incident_id: String,
    pub tenant_id: String,
    pub target_id: String,
    pub purpose_id: String,
    pub event_type: String,
    pub condition_met: bool,
    pub priority: EventPriority,
    pub persistence: EventPersistence,
    /// True only on the first record from an explicitly declared new source epoch.
    pub source_restart: bool,
    pub event_time_tick: u64,
    pub valid_until_tick: u64,
    pub delivered_at_tick: u64,
    pub processing_at_tick: u64,
    pub response_deadline_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventScheduleConfig {
    pub first_source_epoch: u64,
    pub first_sequence: u64,
    pub gap_timeout_ticks: u64,
    /// Additional cap for this evaluator; effective cap is also bounded by the watch.
    pub max_wakeups: u32,
    /// Reserved wake slots that normal-priority batches cannot consume.
    pub critical_wakeup_reserve: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventRunStatus {
    NotStarted,
    Quiet,
    Completed,
    Expired,
    AuthorityRevokedOrExpired,
    DeliveryGapPending,
    DeliveryGap,
    IdentityCollision,
    InvalidSourceEpoch,
    WakeBudgetExhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventDisposition {
    BufferedForSequence,
    Woken,
    WakeBudgetSuppressed,
    DuplicateSuppressed,
    Irrelevant,
    OutOfScope,
    WrongSource,
    Stale,
    DeadlineMissed,
    ObsoleteEpoch,
    DeliveryGap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventDispositionRecord {
    pub source_epoch: u64,
    pub sequence: u64,
    pub event_id: String,
    pub incident_id: String,
    pub disposition: EventDisposition,
    /// The clock tick at which the evaluator assigned the final disposition.
    pub evaluated_at_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventWakeBatch {
    pub wake_index: u32,
    pub processing_tick: u64,
    pub source_epoch: u64,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub event_ids: Vec<String>,
    pub incident_ids: Vec<String>,
    pub critical_event_ids: Vec<String>,
    pub ephemeral_event_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventScheduleMetrics {
    pub schedule_records: usize,
    pub future_records: usize,
    pub delivered_records: usize,
    pub in_order_records: usize,
    pub duplicate_deliveries: usize,
    pub identity_collisions: usize,
    pub source_epoch_transitions: usize,
    pub invalid_source_epochs: usize,
    pub out_of_order_records_buffered: usize,
    pub sequence_gaps_repaired: usize,
    pub sequence_gaps_escalated: usize,
    pub pending_sequence_records: usize,
    pub rejected_source_records: usize,
    pub out_of_scope_records: usize,
    pub irrelevant_records: usize,
    pub stale_records: usize,
    pub deadline_missed_records: usize,
    pub valid_relevant_events: usize,
    pub critical_events_woken: usize,
    pub normal_events_woken: usize,
    pub ephemeral_events_woken: usize,
    pub persistent_events_woken: usize,
    pub wake_budget_suppressed_events: usize,
    pub wake_batches: usize,
    pub coalesced_events: usize,
    pub maximum_event_to_wake_latency_ticks: Option<u64>,
    pub maximum_delivery_to_wake_latency_ticks: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventScheduleResult {
    pub status: EventRunStatus,
    pub final_source_epoch: u64,
    pub next_expected_sequence: u64,
    pub current_authority_epoch: u64,
    pub metrics: EventScheduleMetrics,
    pub wake_batches: Vec<EventWakeBatch>,
    pub dispositions: Vec<EventDispositionRecord>,
    /// Number of policy invocations represented by the output batches.
    /// This is not an LLM request count and does not claim a probe was executed.
    pub policy_invocations: usize,
    pub consequential_mutation_attempts: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventScheduleError {
    InvalidWatch,
    InvalidAuthority,
    InvalidConfig,
    InvalidEventIdentity,
    InvalidEventTime,
    SequenceOverflow,
}

#[derive(Clone)]
struct BufferedEvent {
    event: ScheduledAttentionEvent,
    record_index: usize,
    was_out_of_order: bool,
}

#[derive(Clone)]
struct WakeMember {
    event_id: String,
    incident_id: String,
    sequence: u64,
    event_time_tick: u64,
    delivered_at_tick: u64,
    priority: EventPriority,
    persistence: EventPersistence,
    record_index: usize,
}

#[derive(Clone, Default)]
struct WakeCandidate {
    members: Vec<WakeMember>,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn authority_current_at(
    authority: &AuthoritySnapshot,
    watch: &WatchSpec,
    tick: u64,
) -> bool {
    authority.tenant_id == watch.tenant_id
        && authority.target_id == watch.target_id
        && authority.purpose_id == watch.purpose_id
        && tick >= authority.valid_from_tick
        && tick < authority.expires_at_tick
        && authority.revoked_at_tick.is_none_or(|revoked| tick < revoked)
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
    {
        return Err(EventScheduleError::InvalidWatch);
    }
    if authority.epoch == 0
        || authority.tenant_id != watch.tenant_id
        || authority.target_id != watch.target_id
        || authority.purpose_id != watch.purpose_id
        || authority.valid_from_tick >= authority.expires_at_tick
    {
        return Err(EventScheduleError::InvalidAuthority);
    }
    if config.first_source_epoch == 0
        || config.first_sequence != watch.first_sequence
        || config.critical_wakeup_reserve > config.max_wakeups.min(watch.max_wakeups)
    {
        return Err(EventScheduleError::InvalidConfig);
    }
    Ok(())
}

fn validate_event(event: &ScheduledAttentionEvent) -> Result<(), EventScheduleError> {
    if !non_empty(&event.source_id)
        || !non_empty(&event.event_id)
        || !non_empty(&event.payload_digest)
        || !non_empty(&event.incident_id)
        || !non_empty(&event.tenant_id)
        || !non_empty(&event.target_id)
        || !non_empty(&event.purpose_id)
        || !non_empty(&event.event_type)
        || event.source_epoch == 0
        || event.sequence == 0
    {
        return Err(EventScheduleError::InvalidEventIdentity);
    }
    if event.event_time_tick > event.delivered_at_tick
        || event.delivered_at_tick > event.processing_at_tick
        || event.valid_until_tick < event.event_time_tick
        || event.response_deadline_tick < event.event_time_tick
    {
        return Err(EventScheduleError::InvalidEventTime);
    }
    Ok(())
}

fn result(
    status: EventRunStatus,
    epoch: u64,
    expected: u64,
    authority_epoch: u64,
    metrics: EventScheduleMetrics,
    wake_batches: Vec<EventWakeBatch>,
    dispositions: Vec<EventDispositionRecord>,
) -> EventScheduleResult {
    EventScheduleResult {
        status,
        final_source_epoch: epoch,
        next_expected_sequence: expected,
        current_authority_epoch: authority_epoch,
        policy_invocations: wake_batches.len(),
        consequential_mutation_attempts: 0,
        metrics,
        wake_batches,
        dispositions,
    }
}

fn disposition(
    records: &mut Vec<EventDispositionRecord>,
    source_epoch: u64,
    sequence: u64,
    event_id: &str,
    incident_id: &str,
    value: EventDisposition,
    evaluated_at_tick: u64,
) -> usize {
    let index = records.len();
    records.push(EventDispositionRecord {
        source_epoch,
        sequence,
        event_id: event_id.to_owned(),
        incident_id: incident_id.to_owned(),
        disposition: value,
        evaluated_at_tick,
    });
    index
}

struct EventProcessingContext<'a> {
    watch: &'a WatchSpec,
    authority: &'a AuthoritySnapshot,
    metrics: &'a mut EventScheduleMetrics,
    dispositions: &'a mut [EventDispositionRecord],
    wake_candidates: &'a mut BTreeMap<(u64, u64), WakeCandidate>,
}

fn process_ordered_event(
    buffered: BufferedEvent,
    effective_processing_tick: u64,
    source_epoch: u64,
    context: &mut EventProcessingContext<'_>,
) -> Result<(), EventRunStatus> {
    let event = buffered.event;
    context.metrics.in_order_records += 1;

    if !authority_current_at(context.authority, context.watch, event.delivered_at_tick)
        || !authority_current_at(
            context.authority,
            context.watch,
            effective_processing_tick,
        )
    {
        return Err(EventRunStatus::AuthorityRevokedOrExpired);
    }

    if event.event_time_tick < context.watch.starts_at_tick
        || event.delivered_at_tick < context.watch.starts_at_tick
        || effective_processing_tick < context.watch.starts_at_tick
    {
        context.dispositions[buffered.record_index].disposition = EventDisposition::Stale;
        context.metrics.stale_records += 1;
        return Ok(());
    }
    if effective_processing_tick >= context.watch.expires_at_tick {
        return Err(EventRunStatus::Expired);
    }

    let in_scope = event.tenant_id == context.watch.tenant_id
        && event.target_id == context.watch.target_id
        && event.purpose_id == context.watch.purpose_id;
    if !in_scope {
        context.dispositions[buffered.record_index].disposition = EventDisposition::OutOfScope;
        context.dispositions[buffered.record_index].evaluated_at_tick = effective_processing_tick;
        context.metrics.out_of_scope_records += 1;
        return Ok(());
    }

    if event.event_type != context.watch.event_type || !event.condition_met {
        context.dispositions[buffered.record_index].disposition = EventDisposition::Irrelevant;
        context.dispositions[buffered.record_index].evaluated_at_tick = effective_processing_tick;
        context.metrics.irrelevant_records += 1;
        return Ok(());
    }

    if effective_processing_tick > event.valid_until_tick {
        context.dispositions[buffered.record_index].disposition = EventDisposition::Stale;
        context.dispositions[buffered.record_index].evaluated_at_tick = effective_processing_tick;
        context.metrics.stale_records += 1;
        return Ok(());
    }
    if effective_processing_tick > event.response_deadline_tick {
        context.dispositions[buffered.record_index].disposition = EventDisposition::DeadlineMissed;
        context.dispositions[buffered.record_index].evaluated_at_tick = effective_processing_tick;
        context.metrics.deadline_missed_records += 1;
        return Ok(());
    }

    context.metrics.valid_relevant_events += 1;
    let candidate = context
        .wake_candidates
        .entry((effective_processing_tick, source_epoch))
        .or_default();
    candidate.members.push(WakeMember {
        event_id: event.event_id,
        incident_id: event.incident_id,
        sequence: event.sequence,
        event_time_tick: event.event_time_tick,
        delivered_at_tick: event.delivered_at_tick,
        priority: event.priority,
        persistence: event.persistence,
        record_index: buffered.record_index,
    });
    if buffered.was_out_of_order {
        context.metrics.sequence_gaps_repaired += 1;
    }
    Ok(())
}

/// Evaluate a deterministic delivered-event schedule against one bounded watch.
///
/// Input order is arrival order for records with the same processing tick.
/// Records with later processing ticks are not visible to this evaluation.
/// Out-of-order source sequences are buffered, then released in contiguous
/// order. A missing sequence becomes an explicit delivery gap after timeout.
/// Distinct incident IDs are preserved even when one cognitive wake is safely
/// coalesced for records processed on the same tick.
pub fn evaluate_event_schedule(
    watch: &WatchSpec,
    authority: &AuthoritySnapshot,
    config: &EventScheduleConfig,
    scheduled_events: &[ScheduledAttentionEvent],
    current_tick: u64,
) -> Result<EventScheduleResult, EventScheduleError> {
    validate_inputs(watch, authority, config)?;
    let mut metrics = EventScheduleMetrics {
        schedule_records: scheduled_events.len(),
        ..EventScheduleMetrics::default()
    };

    let mut epoch = config.first_source_epoch;
    let mut expected_sequence = config.first_sequence;
    let mut seen_ids: BTreeMap<(u64, String), (u64, String)> = BTreeMap::new();
    let mut seen_sequences: BTreeMap<(u64, u64), (String, String)> = BTreeMap::new();
    let mut pending: BTreeMap<u64, BufferedEvent> = BTreeMap::new();
    let mut gap_started_at: Option<u64> = None;
    let mut records = Vec::new();
    let mut wake_candidates: BTreeMap<u64, WakeCandidate> = BTreeMap::new();

    if current_tick < watch.starts_at_tick {
        return Ok(result(
            EventRunStatus::NotStarted,
            epoch,
            expected_sequence,
            authority.epoch,
            metrics,
            Vec::new(),
            records,
        ));
    }
    if current_tick >= watch.expires_at_tick {
        return Ok(result(
            EventRunStatus::Expired,
            epoch,
            expected_sequence,
            authority.epoch,
            metrics,
            Vec::new(),
            records,
        ));
    }
    if !authority_current_at(authority, watch, current_tick) {
        return Ok(result(
            EventRunStatus::AuthorityRevokedOrExpired,
            epoch,
            expected_sequence,
            authority.epoch,
            metrics,
            Vec::new(),
            records,
        ));
    }

    let mut arrival_order: Vec<&ScheduledAttentionEvent> = scheduled_events
        .iter()
        .filter(|event| event.processing_at_tick <= current_tick)
        .collect();
    metrics.future_records = scheduled_events.len() - arrival_order.len();
    arrival_order.sort_by_key(|event| event.processing_at_tick);

    for event in arrival_order {
        validate_event(event)?;
        if event.source_id != watch.source_id {
            metrics.rejected_source_records += 1;
            disposition(
                &mut records,
                event.source_epoch,
                event.sequence,
                &event.event_id,
                &event.incident_id,
                EventDisposition::WrongSource,
                event.processing_at_tick,
            );
            continue;
        }
        metrics.delivered_records += 1;

        // A missing cursor entry cannot be repaired after the declared gap
        // timeout merely because it eventually arrives.
        if let Some(gap_start) = gap_started_at {
            if event.processing_at_tick
                > gap_start.saturating_add(config.gap_timeout_ticks)
            {
                metrics.sequence_gaps_escalated += 1;
                for buffered in pending.values() {
                    records[buffered.record_index].disposition = EventDisposition::DeliveryGap;
                }
                return Ok(result(
                    EventRunStatus::DeliveryGap,
                    epoch,
                    expected_sequence,
                    authority.epoch,
                    metrics,
                    Vec::new(),
                    records,
                ));
            }
        }

        let identity_key = (event.source_epoch, event.event_id.clone());
        if let Some((old_sequence, old_digest)) = seen_ids.get(&identity_key) {
            if *old_sequence == event.sequence && old_digest == &event.payload_digest {
                metrics.duplicate_deliveries += 1;
                disposition(
                    &mut records,
                    event.source_epoch,
                    event.sequence,
                    &event.event_id,
                    &event.incident_id,
                    EventDisposition::DuplicateSuppressed,
                    event.processing_at_tick,
                );
                continue;
            }
            metrics.identity_collisions += 1;
            return Ok(result(
                EventRunStatus::IdentityCollision,
                epoch,
                expected_sequence,
                authority.epoch,
                metrics,
                Vec::new(),
                records,
            ));
        }

        if event.source_epoch < epoch {
            disposition(
                &mut records,
                event.source_epoch,
                event.sequence,
                &event.event_id,
                &event.incident_id,
                EventDisposition::ObsoleteEpoch,
                event.processing_at_tick,
            );
            metrics.rejected_source_records += 1;
            continue;
        }
        if event.source_epoch > epoch {
            let valid_restart = event.source_epoch == epoch.saturating_add(1)
                && event.source_restart
                && event.sequence == config.first_sequence
                && pending.is_empty();
            if !valid_restart {
                metrics.invalid_source_epochs += 1;
                if !pending.is_empty() {
                    metrics.sequence_gaps_escalated += 1;
                    for buffered in pending.values() {
                        records[buffered.record_index].disposition = EventDisposition::DeliveryGap;
                    }
                }
                return Ok(result(
                    if !pending.is_empty() {
                        EventRunStatus::DeliveryGap
                    } else {
                        EventRunStatus::InvalidSourceEpoch
                    },
                    epoch,
                    expected_sequence,
                    authority.epoch,
                    metrics,
                    Vec::new(),
                    records,
                ));
            }
            epoch = event.source_epoch;
            expected_sequence = config.first_sequence;
            gap_started_at = None;
            metrics.source_epoch_transitions += 1;
        } else if event.source_restart {
            metrics.invalid_source_epochs += 1;
            return Ok(result(
                EventRunStatus::InvalidSourceEpoch,
                epoch,
                expected_sequence,
                authority.epoch,
                metrics,
                Vec::new(),
                records,
            ));
        }

        seen_ids.insert(identity_key, (event.sequence, event.payload_digest.clone()));
        let sequence_key = (event.source_epoch, event.sequence);
        if let Some((old_id, old_digest)) = seen_sequences.get(&sequence_key) {
            if old_id != &event.event_id || old_digest != &event.payload_digest {
                metrics.identity_collisions += 1;
                return Ok(result(
                    EventRunStatus::IdentityCollision,
                    epoch,
                    expected_sequence,
                    authority.epoch,
                    metrics,
                    Vec::new(),
                    records,
                ));
            }
        } else {
            seen_sequences.insert(
                sequence_key,
                (event.event_id.clone(), event.payload_digest.clone()),
            );
        }

        if event.sequence < expected_sequence {
            metrics.identity_collisions += 1;
            return Ok(result(
                EventRunStatus::IdentityCollision,
                epoch,
                expected_sequence,
                authority.epoch,
                metrics,
                Vec::new(),
                records,
            ));
        }

        let was_out_of_order = event.sequence > expected_sequence;
        if was_out_of_order {
            metrics.out_of_order_records_buffered += 1;
            gap_started_at.get_or_insert(event.processing_at_tick);
        }

        let record_index = disposition(
            &mut records,
            event.source_epoch,
            event.sequence,
            &event.event_id,
            &event.incident_id,
            if was_out_of_order {
                EventDisposition::BufferedForSequence
            } else {
                EventDisposition::Irrelevant
            },
            event.processing_at_tick,
        );
        pending.insert(
            event.sequence,
            BufferedEvent {
                event: event.clone(),
                record_index,
                was_out_of_order,
            },
        );

        while let Some(buffered) = pending.remove(&expected_sequence) {
            let process_tick = event.processing_at_tick.max(buffered.event.processing_at_tick);
            let process_result = {
                let mut context = EventProcessingContext {
                    watch,
                    authority,
                    metrics: &mut metrics,
                    dispositions: &mut records,
                    wake_candidates: &mut wake_candidates,
                };
                process_ordered_event(buffered, process_tick, epoch, &mut context)
            };
            if let Err(status) = process_result {
                return Ok(result(
                    status,
                    epoch,
                    expected_sequence,
                    authority.epoch,
                    metrics,
                    Vec::new(),
                    records,
                ));
            }
            expected_sequence = expected_sequence
                .checked_add(1)
                .ok_or(EventScheduleError::SequenceOverflow)?;
        }

        if pending.is_empty() {
            gap_started_at = None;
        } else if let Some(gap_start) = gap_started_at {
            let gap_deadline = gap_start.saturating_add(config.gap_timeout_ticks);
            if event.processing_at_tick > gap_deadline {
                metrics.sequence_gaps_escalated += 1;
                for buffered in pending.values() {
                    records[buffered.record_index].disposition = EventDisposition::DeliveryGap;
                }
                return Ok(result(
                    EventRunStatus::DeliveryGap,
                    epoch,
                    expected_sequence,
                    authority.epoch,
                    metrics,
                    Vec::new(),
                    records,
                ));
            }
        }
    }

    let gap_pending = !pending.is_empty();
    if gap_pending {
        let gap_start = gap_started_at.unwrap_or(current_tick);
        if current_tick >= gap_start.saturating_add(config.gap_timeout_ticks) {
            metrics.sequence_gaps_escalated += 1;
            for buffered in pending.values() {
                records[buffered.record_index].disposition = EventDisposition::DeliveryGap;
            }
            return Ok(result(
                EventRunStatus::DeliveryGap,
                epoch,
                expected_sequence,
                authority.epoch,
                metrics,
                Vec::new(),
                records,
            ));
        }
        metrics.pending_sequence_records = pending.len();
        for buffered in pending.values() {
            records[buffered.record_index].disposition = EventDisposition::BufferedForSequence;
        }
    }

    let wake_cap = config.max_wakeups.min(watch.max_wakeups);
    let critical_reserve = config.critical_wakeup_reserve.min(wake_cap);
    let normal_cap = wake_cap - critical_reserve;
    let mut normal_wakes = 0u32;
    let mut wake_batches = Vec::new();

    for ((tick, candidate_epoch), candidate) in wake_candidates {
        let has_critical = candidate
            .members
            .iter()
            .any(|member| member.priority == EventPriority::Critical);
        let admitted = (wake_batches.len() as u32) < wake_cap
            && (has_critical || normal_wakes < normal_cap);

        if !admitted {
            metrics.wake_budget_suppressed_events += candidate.members.len();
            for member in candidate.members {
                records[member.record_index].disposition = EventDisposition::WakeBudgetSuppressed;
            }
            continue;
        }

        if !has_critical {
            normal_wakes += 1;
        }
        let mut event_ids = BTreeSet::new();
        let mut incident_ids = BTreeSet::new();
        let mut critical_ids = BTreeSet::new();
        let mut ephemeral_ids = BTreeSet::new();
        let mut first_sequence = u64::MAX;
        let mut last_sequence = 0u64;

        for member in candidate.members {
            event_ids.insert(member.event_id.clone());
            incident_ids.insert(member.incident_id);
            first_sequence = first_sequence.min(member.sequence);
            last_sequence = last_sequence.max(member.sequence);
            if member.priority == EventPriority::Critical {
                critical_ids.insert(member.event_id.clone());
                metrics.critical_events_woken += 1;
            } else {
                metrics.normal_events_woken += 1;
            }
            if member.persistence == EventPersistence::Ephemeral {
                ephemeral_ids.insert(member.event_id.clone());
                metrics.ephemeral_events_woken += 1;
            } else {
                metrics.persistent_events_woken += 1;
            }
            metrics.maximum_event_to_wake_latency_ticks = Some(
                metrics
                    .maximum_event_to_wake_latency_ticks
                    .unwrap_or(0)
                    .max(tick.saturating_sub(member.event_time_tick)),
            );
            metrics.maximum_delivery_to_wake_latency_ticks = Some(
                metrics
                    .maximum_delivery_to_wake_latency_ticks
                    .unwrap_or(0)
                    .max(tick.saturating_sub(member.delivered_at_tick)),
            );
            records[member.record_index].disposition = EventDisposition::Woken;
            records[member.record_index].evaluated_at_tick = tick;
        }

        let event_count = event_ids.len();
        metrics.coalesced_events += event_count.saturating_sub(1);
        wake_batches.push(EventWakeBatch {
            wake_index: wake_batches.len() as u32 + 1,
            processing_tick: tick,
            source_epoch: candidate_epoch,
            first_sequence: first_sequence.min(last_sequence),
            last_sequence,
            event_ids: event_ids.into_iter().collect(),
            incident_ids: incident_ids.into_iter().collect(),
            critical_event_ids: critical_ids.into_iter().collect(),
            ephemeral_event_ids: ephemeral_ids.into_iter().collect(),
        });
    }

    metrics.wake_batches = wake_batches.len();
    let status = if gap_pending {
        EventRunStatus::DeliveryGapPending
    } else if metrics.wake_budget_suppressed_events > 0 {
        EventRunStatus::WakeBudgetExhausted
    } else if wake_batches.is_empty() {
        EventRunStatus::Quiet
    } else {
        EventRunStatus::Completed
    };
    Ok(result(
        status,
        epoch,
        expected_sequence,
        authority.epoch,
        metrics,
        wake_batches,
        records,
    ))
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
            source_id: "source-a".into(),
            event_type: "service_degraded".into(),
            first_sequence: 10,
            starts_at_tick: 1,
            expires_at_tick: 40,
            max_wakeups: 8,
            max_probe_duration_ticks: 2,
            max_observation_age_ticks: 10,
        }
    }

    fn authority() -> AuthoritySnapshot {
        AuthoritySnapshot {
            epoch: 1,
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            valid_from_tick: 0,
            expires_at_tick: 40,
            revoked_at_tick: None,
        }
    }

    fn config() -> EventScheduleConfig {
        EventScheduleConfig {
            first_source_epoch: 1,
            first_sequence: 10,
            gap_timeout_ticks: 3,
            max_wakeups: 8,
            critical_wakeup_reserve: 1,
        }
    }

    fn event(sequence: u64, id: &str, incident: &str, processing: u64) -> ScheduledAttentionEvent {
        ScheduledAttentionEvent {
            source_id: "source-a".into(),
            source_epoch: 1,
            event_id: id.into(),
            payload_digest: format!("sha256:{sequence:064x}"),
            sequence,
            incident_id: incident.into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            event_type: "service_degraded".into(),
            condition_met: true,
            priority: EventPriority::Normal,
            persistence: EventPersistence::Persistent,
            source_restart: false,
            event_time_tick: processing.saturating_sub(2),
            valid_until_tick: processing.saturating_add(5),
            delivered_at_tick: processing.saturating_sub(1),
            processing_at_tick: processing,
            response_deadline_tick: processing.saturating_add(3),
        }
    }

    fn evaluate(events: &[ScheduledAttentionEvent], now: u64) -> EventScheduleResult {
        evaluate_event_schedule(&watch(), &authority(), &config(), events, now).unwrap()
    }


    #[test]
    fn missing_sequence_arriving_after_gap_deadline_does_not_repair_history() {
        let later = event(11, "evt-11", "incident-b", 5);
        let mut missing = event(10, "evt-10", "incident-a", 9);
        missing.event_time_tick = 8;
        missing.delivered_at_tick = 8;
        let result = evaluate(&[later, missing], 9);
        assert_eq!(result.status, EventRunStatus::DeliveryGap);
        assert_eq!(result.metrics.sequence_gaps_escalated, 1);
        assert!(result.wake_batches.is_empty());
    }

    #[test]
    fn different_incidents_at_different_ticks_receive_distinct_wake_batches() {
        let result = evaluate(
            &[
                event(10, "evt-10", "incident-a", 5),
                event(11, "evt-11", "incident-b", 6),
            ],
            6,
        );
        assert_eq!(result.wake_batches.len(), 2);
        assert_eq!(result.metrics.coalesced_events, 0);
        assert_eq!(result.wake_batches[0].incident_ids, vec!["incident-a"]);
        assert_eq!(result.wake_batches[1].incident_ids, vec!["incident-b"]);
    }

    #[test]
    fn delayed_event_inside_deadline_is_woken_and_reports_both_latencies() {
        let mut delayed = event(10, "evt-delayed", "incident-delayed", 8);
        delayed.event_time_tick = 2;
        delayed.delivered_at_tick = 5;
        delayed.valid_until_tick = 10;
        delayed.response_deadline_tick = 9;
        let result = evaluate(&[delayed], 8);
        assert_eq!(result.wake_batches.len(), 1);
        assert_eq!(result.metrics.maximum_event_to_wake_latency_ticks, Some(6));
        assert_eq!(result.metrics.maximum_delivery_to_wake_latency_ticks, Some(3));
    }

    #[test]
    fn exact_duplicate_from_previous_epoch_remains_suppressed_after_restart() {
        let old = event(10, "evt-old", "incident-old", 5);
        let mut restart = event(10, "evt-new", "incident-new", 6);
        restart.source_epoch = 2;
        restart.source_restart = true;
        let mut replay = old.clone();
        replay.processing_at_tick = 7;
        replay.delivered_at_tick = 6;
        let result = evaluate(&[old, restart, replay], 7);
        assert_eq!(result.status, EventRunStatus::Completed);
        assert_eq!(result.metrics.source_epoch_transitions, 1);
        assert_eq!(result.metrics.duplicate_deliveries, 1);
        assert_eq!(result.wake_batches.len(), 2);
    }


    #[test]
    fn source_epoch_boundary_is_not_coalesced_with_prior_epoch() {
        let old = event(10, "evt-old", "incident-old", 5);
        let mut restart = event(10, "evt-new", "incident-new", 5);
        restart.source_epoch = 2;
        restart.source_restart = true;
        let result = evaluate(&[old, restart], 5);
        assert_eq!(result.wake_batches.len(), 2);
        assert_eq!(result.wake_batches[0].source_epoch, 1);
        assert_eq!(result.wake_batches[1].source_epoch, 2);
    }

    #[test]
    fn event_that_occurred_before_watch_start_is_not_retroactively_matched() {
        let mut pre_watch = event(10, "evt-pre-watch", "incident-old", 5);
        pre_watch.event_time_tick = 0;
        pre_watch.delivered_at_tick = 3;
        let result = evaluate(&[pre_watch], 5);
        assert_eq!(result.status, EventRunStatus::Quiet);
        assert_eq!(result.metrics.stale_records, 1);
        assert!(result.wake_batches.is_empty());
    }

    #[test]
    fn quiet_schedule_does_not_wake() {
        let result = evaluate(&[], 8);
        assert_eq!(result.status, EventRunStatus::Quiet);
        assert!(result.wake_batches.is_empty());
        assert_eq!(result.policy_invocations, 0);
    }

    #[test]
    fn durable_event_produces_one_wake_batch() {
        let result = evaluate(&[event(10, "evt-10", "incident-a", 5)], 5);
        assert_eq!(result.status, EventRunStatus::Completed);
        assert_eq!(result.wake_batches.len(), 1);
        assert_eq!(result.wake_batches[0].incident_ids, vec!["incident-a"]);
        assert_eq!(result.metrics.persistent_events_woken, 1);
        assert_eq!(result.consequential_mutation_attempts, 0);
    }

    #[test]
    fn distinct_incidents_at_same_processing_tick_coalesce_without_losing_identity() {
        let result = evaluate(
            &[
                event(10, "evt-10", "incident-a", 5),
                event(11, "evt-11", "incident-b", 5),
            ],
            5,
        );
        assert_eq!(result.wake_batches.len(), 1);
        assert_eq!(
            result.wake_batches[0].incident_ids,
            vec!["incident-a", "incident-b"]
        );
        assert_eq!(result.metrics.valid_relevant_events, 2);
        assert_eq!(result.metrics.coalesced_events, 1);
    }

    #[test]
    fn ephemeral_event_wakes_even_without_a_persistent_state_sample() {
        let mut ephemeral = event(10, "evt-ephemeral", "incident-short", 5);
        ephemeral.persistence = EventPersistence::Ephemeral;
        ephemeral.valid_until_tick = 5;
        let result = evaluate(&[ephemeral], 5);
        assert_eq!(result.wake_batches.len(), 1);
        assert_eq!(result.metrics.ephemeral_events_woken, 1);
    }

    #[test]
    fn duplicate_same_identity_and_digest_is_suppressed() {
        let first = event(10, "evt-10", "incident-a", 5);
        let duplicate = first.clone();
        let result = evaluate(&[first, duplicate], 5);
        assert_eq!(result.wake_batches.len(), 1);
        assert_eq!(result.metrics.duplicate_deliveries, 1);
    }

    #[test]
    fn same_event_id_with_different_sequence_or_digest_is_a_collision() {
        let first = event(10, "evt-10", "incident-a", 5);
        let mut collision = event(11, "evt-10", "incident-b", 6);
        collision.payload_digest = "sha256:collision".into();
        let result = evaluate(&[first, collision], 6);
        assert_eq!(result.status, EventRunStatus::IdentityCollision);
        assert_eq!(result.metrics.identity_collisions, 1);
        assert!(result.wake_batches.is_empty());
    }

    #[test]
    fn same_sequence_with_different_event_id_is_a_collision() {
        let first = event(10, "evt-10", "incident-a", 5);
        let second = event(10, "evt-other", "incident-b", 5);
        let result = evaluate(&[first, second], 5);
        assert_eq!(result.status, EventRunStatus::IdentityCollision);
        assert_eq!(result.metrics.identity_collisions, 1);
    }

    #[test]
    fn out_of_order_delivery_is_buffered_then_released_in_source_order() {
        let later = event(11, "evt-11", "incident-b", 5);
        let missing = event(10, "evt-10", "incident-a", 6);
        let result = evaluate(&[later, missing], 6);
        assert_eq!(result.status, EventRunStatus::Completed);
        assert_eq!(result.metrics.sequence_gaps_repaired, 1);
        assert_eq!(result.wake_batches.len(), 1);
        assert_eq!(result.wake_batches[0].incident_ids, vec!["incident-a", "incident-b"]);
        assert_eq!(result.wake_batches[0].processing_tick, 6);
    }


    #[test]
    fn unresolved_gap_before_timeout_is_not_reported_as_quiet() {
        let later = event(11, "evt-11", "incident-b", 5);
        let result = evaluate(&[later], 7);
        assert_eq!(result.status, EventRunStatus::DeliveryGapPending);
        assert_eq!(result.metrics.pending_sequence_records, 1);
        assert_eq!(result.metrics.sequence_gaps_escalated, 0);
        assert!(result.wake_batches.is_empty());
    }

    #[test]
    fn unresolved_sequence_gap_escalates_instead_of_reporting_quiet() {
        let later = event(11, "evt-11", "incident-b", 5);
        let result = evaluate(&[later], 8);
        assert_eq!(result.status, EventRunStatus::DeliveryGap);
        assert_eq!(result.metrics.sequence_gaps_escalated, 1);
        assert!(result.wake_batches.is_empty());
    }

    #[test]
    fn event_after_response_deadline_is_not_counted_as_a_wake() {
        let mut late = event(10, "evt-late", "incident-late", 5);
        late.response_deadline_tick = 4;
        let result = evaluate(&[late], 5);
        assert_eq!(result.status, EventRunStatus::Quiet);
        assert_eq!(result.metrics.deadline_missed_records, 1);
        assert!(result.wake_batches.is_empty());
    }

    #[test]
    fn stale_ephemeral_event_is_not_woken() {
        let mut stale = event(10, "evt-stale", "incident-stale", 8);
        stale.persistence = EventPersistence::Ephemeral;
        stale.valid_until_tick = 6;
        let result = evaluate(&[stale], 8);
        assert_eq!(result.status, EventRunStatus::Quiet);
        assert_eq!(result.metrics.stale_records, 1);
    }

    #[test]
    fn authority_revoked_while_watch_is_dormant_prevents_wake() {
        let mut revoked = authority();
        revoked.revoked_at_tick = Some(4);
        let result = evaluate_event_schedule(
            &watch(),
            &revoked,
            &config(),
            &[event(10, "evt-10", "incident-a", 5)],
            5,
        )
        .unwrap();
        assert_eq!(result.status, EventRunStatus::AuthorityRevokedOrExpired);
        assert!(result.wake_batches.is_empty());
    }

    #[test]
    fn source_restart_requires_explicit_next_epoch_marker() {
        let mut restart = event(10, "evt-new-epoch", "incident-b", 6);
        restart.source_epoch = 2;
        restart.source_restart = true;
        let result = evaluate(
            &[
                event(10, "evt-epoch-1", "incident-a", 5),
                restart,
            ],
            6,
        );
        assert_eq!(result.status, EventRunStatus::Completed);
        assert_eq!(result.final_source_epoch, 2);
        assert_eq!(result.metrics.source_epoch_transitions, 1);
    }

    #[test]
    fn unannounced_source_restart_is_rejected() {
        let mut restart = event(10, "evt-unannounced", "incident-b", 6);
        restart.source_epoch = 2;
        let result = evaluate(
            &[
                event(10, "evt-epoch-1", "incident-a", 5),
                restart,
            ],
            6,
        );
        assert_eq!(result.status, EventRunStatus::InvalidSourceEpoch);
        assert_eq!(result.metrics.invalid_source_epochs, 1);
    }

    #[test]
    fn critical_reserve_prevents_normal_event_flood_from_using_all_wakes() {
        let mut cfg = config();
        cfg.max_wakeups = 2;
        cfg.critical_wakeup_reserve = 1;
        let mut normal_a = event(10, "evt-normal-a", "incident-a", 5);
        let mut normal_b = event(11, "evt-normal-b", "incident-b", 6);
        normal_a.priority = EventPriority::Normal;
        normal_b.priority = EventPriority::Normal;
        let mut critical = event(12, "evt-critical", "incident-critical", 7);
        critical.priority = EventPriority::Critical;
        let result = evaluate_event_schedule(
            &watch(),
            &authority(),
            &cfg,
            &[normal_a, normal_b, critical],
            7,
        )
        .unwrap();
        assert_eq!(result.wake_batches.len(), 2);
        assert_eq!(result.metrics.wake_budget_suppressed_events, 1);
        assert_eq!(result.metrics.critical_events_woken, 1);
    }

    #[test]
    fn wrong_tenant_event_consumes_source_sequence_but_never_wakes() {
        let mut wrong_scope = event(10, "evt-wrong-tenant", "incident-a", 5);
        wrong_scope.tenant_id = "tenant-b".into();
        let next = event(11, "evt-valid", "incident-b", 6);
        let result = evaluate(&[wrong_scope, next], 6);
        assert_eq!(result.status, EventRunStatus::Completed);
        assert_eq!(result.metrics.out_of_scope_records, 1);
        assert_eq!(result.wake_batches.len(), 1);
        assert_eq!(result.wake_batches[0].incident_ids, vec!["incident-b"]);
    }

    #[test]
    fn future_scheduled_events_are_not_visible_before_processing_tick() {
        let future = event(10, "evt-future", "incident-future", 8);
        let result = evaluate(&[future], 7);
        assert_eq!(result.status, EventRunStatus::Quiet);
        assert_eq!(result.metrics.future_records, 1);
        assert_eq!(result.metrics.delivered_records, 0);
    }
}
