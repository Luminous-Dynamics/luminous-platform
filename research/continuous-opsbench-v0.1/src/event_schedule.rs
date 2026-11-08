//! Deterministic durable/ephemeral event-schedule evaluator.
//!
//! This module consumes a frozen source-qualified schedule; it is not an event
//! broker, subscription service, inbox, timer or production scheduler. The
//! evaluator models how existing event records reach an attention policy.
//! Fixed polling sees current condition state; event-triggered policies see
//! delivered source records. All arms share the same immutable schedule.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Persistence {
    PersistentCondition,
    EphemeralTransition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionOutcome {
    Match,
    NoMatch,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttentionStrategy {
    FixedPolling { interval_ticks: u64 },
    EventTriggered,
    CoalescedEventTriggered { coalesce_window_ticks: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventWatch {
    pub watch_id: String,
    pub source_id: String,
    pub initial_source_epoch: String,
    pub tenant_id: String,
    pub target_id: String,
    pub purpose_id: String,
    pub event_type: String,
    pub starts_at_tick: u64,
    pub expires_at_tick: u64,
    pub first_sequence: u64,
    pub max_event_checks: usize,
    pub max_wakeups: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityWindow {
    pub epoch: u64,
    pub tenant_id: String,
    pub target_id: String,
    pub purpose_id: String,
    pub valid_from_tick: u64,
    pub expires_at_tick: u64,
    /// Revocation is effective at this tick, inclusive.
    pub revoked_at_tick: Option<u64>,
}

impl AuthorityWindow {
    fn current_for(&self, watch: &EventWatch, tick: u64) -> bool {
        self.tenant_id == watch.tenant_id
            && self.target_id == watch.target_id
            && self.purpose_id == watch.purpose_id
            && tick >= self.valid_from_tick
            && tick < self.expires_at_tick
            && self.revoked_at_tick.is_none_or(|revoked| tick < revoked)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEvent {
    pub source_id: String,
    pub source_epoch: String,
    pub event_id: String,
    pub incident_id: String,
    pub sequence: u64,
    pub payload_digest: String,
    pub tenant_id: String,
    pub target_id: String,
    pub purpose_id: String,
    pub event_type: String,
    pub coalesce_key: String,
    pub condition: ConditionOutcome,
    pub persistence: Persistence,
    pub event_tick: u64,
    pub valid_from_tick: u64,
    /// For fixed polling, the condition is visible on [valid_from, valid_until).
    pub valid_until_tick: u64,
    pub delivered_tick: u64,
    /// A delivered source record remains replayable until this tick.
    pub record_retain_until_tick: u64,
    /// The event must result in a wake no later than this tick.
    pub deadline_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRestart {
    pub source_id: String,
    pub previous_epoch: String,
    pub new_epoch: String,
    pub sequence_start: u64,
    pub at_tick: u64,
    pub restart_proof_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceWatermark {
    pub source_id: String,
    pub source_epoch: String,
    /// The next source sequence the consumer must receive.
    pub next_sequence: u64,
    pub observed_tick: u64,
    pub available: bool,
    pub watermark_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimelineRecord {
    Event(SourceEvent),
    Restart(SourceRestart),
    Watermark(SourceWatermark),
}

impl TimelineRecord {
    fn tick(&self) -> u64 {
        match self {
            Self::Event(event) => event.delivered_tick,
            Self::Restart(restart) => restart.at_tick,
            Self::Watermark(watermark) => watermark.observed_tick,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventCheckpoint {
    pub source_id: String,
    pub source_epoch: String,
    pub next_sequence: u64,
    pub last_stream_tick: u64,
    /// Event ID -> (source epoch, sequence, payload digest).
    pub seen_event_ids: BTreeMap<String, (String, u64, String)>,
    /// Authenticated source records waiting for a preceding sequence.
    pub buffered_events: BTreeMap<u64, SourceEvent>,
    /// Start/deadline for the currently missing cursor sequence.
    pub gap_started_at_tick: Option<u64>,
    pub gap_deadline_tick: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordDisposition {
    Woke,
    Coalesced,
    Quiet,
    DuplicateSuppressed,
    ScopeRejected,
    ExpiredBeforeProcessing,
    MissedDeadline,
    OutOfOrder,
    BufferedForSequence,
    DeliveryGapPending,
    IdentityCollision,
    SourceEpochMismatch,
    RestartAccepted,
    RestartRejected,
    WatermarkConfirmed,
    DeliveryGap,
    DeliveryGapPending,
    SourceUnavailable,
    AuthorityRevokedOrExpired,
    BudgetExhausted,
    WatchExpired,
    IndeterminateCondition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventRecordResult {
    pub event_id: String,
    pub incident_id: String,
    pub sequence: u64,
    pub delivered_tick: u64,
    pub processed_tick: Option<u64>,
    pub disposition: RecordDisposition,
    pub detection_latency_ticks: Option<u64>,
    pub deadline_met: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WakeRecord {
    pub wake_tick: u64,
    pub source_epoch: String,
    pub event_ids: Vec<String>,
    pub incident_ids: Vec<String>,
    pub sequence_start: u64,
    pub sequence_end: u64,
    pub coalesce_key: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventScheduleMetrics {
    pub source_records: usize,
    pub unique_events: usize,
    pub duplicates_suppressed: usize,
    pub identity_collisions: usize,
    pub sequence_gaps: usize,
    pub buffered_events: usize,
    pub gap_repairs: usize,
    pub gap_timeouts: usize,
    pub pending_gap_observations: usize,
    pub gap_buffer_overflows: usize,
    pub max_buffered_event_count: usize,
    pub out_of_order_records: usize,
    pub restarts_accepted: usize,
    pub restarts_rejected: usize,
    pub scope_rejections: usize,
    pub watermarks_checked: usize,
    pub source_unavailable: usize,
    pub quiet_events: usize,
    pub indeterminate_events: usize,
    pub relevant_events: usize,
    pub relevant_events_detected: usize,
    pub missed_deadlines: usize,
    pub missed_ephemeral_events: usize,
    /// Matching source records that requested attention, before coalescing.
    pub event_wake_notifications: usize,
    /// Actual wake groups after optional coalescing.
    pub wakeups: usize,
    pub cognitive_invocations: usize,
    pub coalesced_events: usize,
    pub budget_exhaustions: usize,
    pub max_event_backlog_ticks: u64,
    pub max_detection_latency_ticks: u64,
    pub consequential_mutation_attempts: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScheduleStatus {
    Completed,
    Quiet,
    DeliveryGap,
    OutOfOrder,
    IdentityCollision,
    SourceEpochMismatch,
    SourceIdentityMismatch,
    SourceUnavailable,
    AuthorityRevokedOrExpired,
    WatchExpired,
    BudgetExhausted,
    Indeterminate,
    InvalidSchedule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventScheduleResult {
    pub status: ScheduleStatus,
    pub watch_id: String,
    pub strategy: AttentionStrategy,
    pub authority_epoch: u64,
    pub metrics: EventScheduleMetrics,
    pub records: Vec<EventRecordResult>,
    pub wakes: Vec<WakeRecord>,
    pub checkpoint: EventCheckpoint,
    /// No event/wake may authorize consequential mutation.
    pub consequential_mutation_attempts: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScheduleError {
    InvalidWatch,
    InvalidAuthority,
    InvalidStrategy,
    InvalidGapPolicy,
    EventTimeRegression,
    InvalidEvent,
    InvalidRestart,
    InvalidCheckpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SequenceGapPolicy {
    /// The gap becomes a delivery failure at this many ticks after first observation.
    /// The boundary is inclusive: an event arriving exactly at the deadline is late.
    pub timeout_ticks: u64,
    pub max_buffered_events: usize,
}

impl SequenceGapPolicy {
    /// Preserve the original immediate, fail-closed semantics.
    pub const fn strict() -> Self {
        Self {
            timeout_ticks: 0,
            max_buffered_events: 0,
        }
    }

    /// Opt in to bounded buffering and repair before a deadline.
    pub const fn buffered(timeout_ticks: u64, max_buffered_events: usize) -> Self {
        Self {
            timeout_ticks,
            max_buffered_events,
        }
    }

    const fn is_buffered(self) -> bool {
        self.timeout_ticks > 0 && self.max_buffered_events > 0
    }
}

#[derive(Clone, Debug)]
struct PendingWake {
    wake_tick: u64,
    source_epoch: String,
    event_ids: Vec<String>,
    incident_ids: Vec<String>,
    sequence_start: u64,
    sequence_end: u64,
    coalesce_key: String,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn validate_watch(watch: &EventWatch) -> Result<(), ScheduleError> {
    if !non_empty(&watch.watch_id)
        || !non_empty(&watch.source_id)
        || !non_empty(&watch.initial_source_epoch)
        || !non_empty(&watch.tenant_id)
        || !non_empty(&watch.target_id)
        || !non_empty(&watch.purpose_id)
        || !non_empty(&watch.event_type)
        || watch.starts_at_tick >= watch.expires_at_tick
        || watch.first_sequence == 0
        || watch.first_sequence == u64::MAX
        || watch.max_event_checks == 0
    {
        return Err(ScheduleError::InvalidWatch);
    }
    Ok(())
}

fn validate_authority(authority: &AuthorityWindow) -> Result<(), ScheduleError> {
    if authority.epoch == 0
        || !non_empty(&authority.tenant_id)
        || !non_empty(&authority.target_id)
        || !non_empty(&authority.purpose_id)
        || authority.valid_from_tick >= authority.expires_at_tick
    {
        return Err(ScheduleError::InvalidAuthority);
    }
    Ok(())
}

fn validate_event(event: &SourceEvent) -> Result<(), ScheduleError> {
    if !non_empty(&event.source_id)
        || !non_empty(&event.source_epoch)
        || !non_empty(&event.event_id)
        || !non_empty(&event.incident_id)
        || event.sequence == 0
        || event.sequence == u64::MAX
        || !valid_sha256(&event.payload_digest)
        || !non_empty(&event.tenant_id)
        || !non_empty(&event.target_id)
        || !non_empty(&event.purpose_id)
        || !non_empty(&event.event_type)
        || !non_empty(&event.coalesce_key)
        || event.event_tick > event.delivered_tick
        || event.valid_from_tick > event.event_tick
        || event.event_tick >= event.valid_until_tick
        || event.delivered_tick > event.record_retain_until_tick
        || event.deadline_tick < event.event_tick
    {
        return Err(ScheduleError::InvalidEvent);
    }
    Ok(())
}

fn validate_restart(restart: &SourceRestart) -> Result<(), ScheduleError> {
    if !non_empty(&restart.source_id)
        || !non_empty(&restart.previous_epoch)
        || !non_empty(&restart.new_epoch)
        || restart.previous_epoch == restart.new_epoch
        || restart.sequence_start == 0
        || restart.sequence_start == u64::MAX
        || !valid_sha256(&restart.restart_proof_digest)
    {
        return Err(ScheduleError::InvalidRestart);
    }
    Ok(())
}

fn validate_watermark(watermark: &SourceWatermark) -> Result<(), ScheduleError> {
    if !non_empty(&watermark.source_id)
        || !non_empty(&watermark.source_epoch)
        || watermark.next_sequence == 0
        || watermark.next_sequence == u64::MAX
        || !valid_sha256(&watermark.watermark_digest)
    {
        return Err(ScheduleError::InvalidEvent);
    }
    Ok(())
}

fn initial_checkpoint(watch: &EventWatch) -> EventCheckpoint {
    EventCheckpoint {
        source_id: watch.source_id.clone(),
        source_epoch: watch.initial_source_epoch.clone(),
        next_sequence: watch.first_sequence,
        last_stream_tick: watch.starts_at_tick,
        seen_event_ids: BTreeMap::new(),
        buffered_events: BTreeMap::new(),
        gap_started_at_tick: None,
        gap_deadline_tick: None,
    }
}

fn validate_checkpoint(
    watch: &EventWatch,
    checkpoint: &EventCheckpoint,
) -> Result<(), ScheduleError> {
    if checkpoint.source_id != watch.source_id
        || !non_empty(&checkpoint.source_epoch)
        || checkpoint.next_sequence == 0
        || checkpoint.next_sequence == u64::MAX
        || checkpoint.last_stream_tick < watch.starts_at_tick
        || checkpoint
            .seen_event_ids
            .iter()
            .any(|(event_id, (epoch, sequence, digest))| {
                !non_empty(event_id)
                    || !non_empty(epoch)
                    || *sequence == 0
                    || *sequence == u64::MAX
                    || !valid_sha256(digest)
            })
        || checkpoint.gap_started_at_tick.is_some()
            != checkpoint.gap_deadline_tick.is_some()
        || checkpoint
            .gap_started_at_tick
            .zip(checkpoint.gap_deadline_tick)
            .is_some_and(|(started, deadline)| deadline <= started)
        || checkpoint.buffered_events.iter().any(|(sequence, event)| {
            *sequence != event.sequence
                || event.source_id != checkpoint.source_id
                || event.source_epoch != checkpoint.source_epoch
                || event.sequence <= checkpoint.next_sequence
                || event.delivered_tick > checkpoint.last_stream_tick
                || checkpoint.seen_event_ids.contains_key(&event.event_id)
                || validate_event(event).is_err()
        })
        || (!checkpoint.buffered_events.is_empty()
            && checkpoint.gap_started_at_tick.is_none())
    {
        return Err(ScheduleError::InvalidCheckpoint);
    }
    Ok(())
}

fn poll_tick_at_or_after(start: u64, interval: u64, target: u64) -> Option<u64> {
    if target <= start {
        return Some(start);
    }
    let elapsed = target - start;
    let quotient = elapsed / interval;
    let steps = quotient + u64::from(!elapsed.is_multiple_of(interval));
    start.checked_add(steps.checked_mul(interval)?)
}

struct ScheduleOutput {
    status: ScheduleStatus,
    metrics: EventScheduleMetrics,
    records: Vec<EventRecordResult>,
    wakes: Vec<WakeRecord>,
    checkpoint: EventCheckpoint,
}

fn make_result(
    watch: &EventWatch,
    strategy: AttentionStrategy,
    authority: &AuthorityWindow,
    output: ScheduleOutput,
) -> EventScheduleResult {
    EventScheduleResult {
        status: output.status,
        watch_id: watch.watch_id.clone(),
        strategy,
        authority_epoch: authority.epoch,
        metrics: output.metrics,
        records: output.records,
        wakes: output.wakes,
        checkpoint: output.checkpoint,
        consequential_mutation_attempts: 0,
    }
}

fn flush_pending(pending: &mut Vec<PendingWake>, wakes: &mut Vec<WakeRecord>) {
    for pending in pending.drain(..) {
        wakes.push(WakeRecord {
            wake_tick: pending.wake_tick,
            source_epoch: pending.source_epoch,
            event_ids: pending.event_ids,
            incident_ids: pending.incident_ids,
            sequence_start: pending.sequence_start,
            sequence_end: pending.sequence_end,
            coalesce_key: pending.coalesce_key,
        });
    }
}

fn joins_pending_group(
    strategy: AttentionStrategy,
    event: &SourceEvent,
    processing_tick: u64,
    pending: &[PendingWake],
) -> bool {
    match strategy {
        AttentionStrategy::CoalescedEventTriggered {
            coalesce_window_ticks,
        } => pending.iter().any(|group| {
            group.source_epoch == event.source_epoch
                && group.coalesce_key == event.coalesce_key
                && processing_tick.saturating_sub(group.wake_tick) <= coalesce_window_ticks
        }),
        _ => false,
    }
}

fn apply_wake(
    strategy: AttentionStrategy,
    event: &SourceEvent,
    processing_tick: u64,
    pending: &mut Vec<PendingWake>,
    wakes: &mut Vec<WakeRecord>,
    metrics: &mut EventScheduleMetrics,
) -> bool {
    match strategy {
        AttentionStrategy::CoalescedEventTriggered {
            coalesce_window_ticks,
        } => {
            if let Some(group) = pending.iter_mut().find(|group| {
                group.source_epoch == event.source_epoch
                    && group.coalesce_key == event.coalesce_key
                    && processing_tick.saturating_sub(group.wake_tick) <= coalesce_window_ticks
            }) {
                group.event_ids.push(event.event_id.clone());
                group.incident_ids.push(event.incident_id.clone());
                group.sequence_start = group.sequence_start.min(event.sequence);
                group.sequence_end = group.sequence_end.max(event.sequence);
                metrics.coalesced_events += 1;
                true
            } else {
                pending.push(PendingWake {
                    wake_tick: processing_tick,
                    source_epoch: event.source_epoch.clone(),
                    event_ids: vec![event.event_id.clone()],
                    incident_ids: vec![event.incident_id.clone()],
                    sequence_start: event.sequence,
                    sequence_end: event.sequence,
                    coalesce_key: event.coalesce_key.clone(),
                });
                false
            }
        }
        _ => {
            wakes.push(WakeRecord {
                wake_tick: processing_tick,
                source_epoch: event.source_epoch.clone(),
                event_ids: vec![event.event_id.clone()],
                incident_ids: vec![event.incident_id.clone()],
                sequence_start: event.sequence,
                sequence_end: event.sequence,
                coalesce_key: event.coalesce_key.clone(),
            });
            false
        }
    }
}

struct EventProcessingState<'a> {
    watch: &'a EventWatch,
    authority: &'a AuthorityWindow,
    strategy: AttentionStrategy,
    gap_policy: SequenceGapPolicy,
    checkpoint: &'a mut EventCheckpoint,
    metrics: &'a mut EventScheduleMetrics,
    records: &'a mut Vec<EventRecordResult>,
    pending: &'a mut Vec<PendingWake>,
    wakes: &'a mut Vec<WakeRecord>,
}

fn process_event(
    state: &mut EventProcessingState<'_>,
    event: &SourceEvent,
    processing_tick: u64,
) -> Option<ScheduleStatus> {
    let EventProcessingState {
        watch,
        authority,
        strategy,
        checkpoint,
        metrics,
        records,
        pending,
        wakes,
    } = state;
    metrics.source_records += 1;

    if event.source_id != watch.source_id {
        metrics.scope_rejections += 1;
        records.push(EventRecordResult {
            event_id: event.event_id.clone(),
            incident_id: event.incident_id.clone(),
            sequence: event.sequence,
            delivered_tick: event.delivered_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::ScopeRejected,
            detection_latency_ticks: None,
            deadline_met: false,
        });
        return Some(ScheduleStatus::SourceIdentityMismatch);
    }

    let previous = checkpoint.seen_event_ids.get(&event.event_id);
    if let Some((epoch, sequence, digest)) = previous {
        if epoch == &event.source_epoch
            && *sequence == event.sequence
            && digest == &event.payload_digest
        {
            metrics.duplicates_suppressed += 1;
            records.push(EventRecordResult {
                event_id: event.event_id.clone(),
                incident_id: event.incident_id.clone(),
                sequence: event.sequence,
                delivered_tick: event.delivered_tick,
                processed_tick: Some(processing_tick),
                disposition: RecordDisposition::DuplicateSuppressed,
                detection_latency_ticks: None,
                deadline_met: true,
            });
            return None;
        }
        metrics.identity_collisions += 1;
        records.push(EventRecordResult {
            event_id: event.event_id.clone(),
            incident_id: event.incident_id.clone(),
            sequence: event.sequence,
            delivered_tick: event.delivered_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::IdentityCollision,
            detection_latency_ticks: None,
            deadline_met: false,
        });
        return Some(ScheduleStatus::IdentityCollision);
    }

    if event.source_epoch != checkpoint.source_epoch {
        records.push(EventRecordResult {
            event_id: event.event_id.clone(),
            incident_id: event.incident_id.clone(),
            sequence: event.sequence,
            delivered_tick: event.delivered_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::SourceEpochMismatch,
            detection_latency_ticks: None,
            deadline_met: false,
        });
        return Some(ScheduleStatus::SourceEpochMismatch);
    }

    if event.sequence > checkpoint.next_sequence {
        metrics.sequence_gaps += 1;
        records.push(EventRecordResult {
            event_id: event.event_id.clone(),
            incident_id: event.incident_id.clone(),
            sequence: event.sequence,
            delivered_tick: event.delivered_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::DeliveryGap,
            detection_latency_ticks: None,
            deadline_met: false,
        });
        return Some(ScheduleStatus::DeliveryGap);
    }
    if event.sequence < checkpoint.next_sequence {
        metrics.out_of_order_records += 1;
        records.push(EventRecordResult {
            event_id: event.event_id.clone(),
            incident_id: event.incident_id.clone(),
            sequence: event.sequence,
            delivered_tick: event.delivered_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::OutOfOrder,
            detection_latency_ticks: None,
            deadline_met: false,
        });
        return Some(ScheduleStatus::OutOfOrder);
    }

    checkpoint.next_sequence = checkpoint.next_sequence.saturating_add(1);
    checkpoint.last_stream_tick = event.delivered_tick;
    checkpoint.seen_event_ids.insert(
        event.event_id.clone(),
        (
            event.source_epoch.clone(),
            event.sequence,
            event.payload_digest.clone(),
        ),
    );
    metrics.unique_events += 1;

    let scope_matches = event.tenant_id == watch.tenant_id
        && event.target_id == watch.target_id
        && event.purpose_id == watch.purpose_id
        && event.event_type == watch.event_type;
    if !scope_matches {
        metrics.scope_rejections += 1;
        records.push(EventRecordResult {
            event_id: event.event_id.clone(),
            incident_id: event.incident_id.clone(),
            sequence: event.sequence,
            delivered_tick: event.delivered_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::ScopeRejected,
            detection_latency_ticks: None,
            deadline_met: true,
        });
        return None;
    }

    if !authority.current_for(watch, processing_tick) {
        records.push(EventRecordResult {
            event_id: event.event_id.clone(),
            incident_id: event.incident_id.clone(),
            sequence: event.sequence,
            delivered_tick: event.delivered_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::AuthorityRevokedOrExpired,
            detection_latency_ticks: None,
            deadline_met: false,
        });
        return Some(ScheduleStatus::AuthorityRevokedOrExpired);
    }

    if processing_tick > event.deadline_tick {
        if event.condition == ConditionOutcome::Match {
            metrics.relevant_events += 1;
            metrics.missed_deadlines += 1;
        }
        if event.persistence == Persistence::EphemeralTransition {
            metrics.missed_ephemeral_events += 1;
        }
        records.push(EventRecordResult {
            event_id: event.event_id.clone(),
            incident_id: event.incident_id.clone(),
            sequence: event.sequence,
            delivered_tick: event.delivered_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::MissedDeadline,
            detection_latency_ticks: None,
            deadline_met: false,
        });
        return None;
    }

    if processing_tick < event.valid_from_tick
        || processing_tick > event.record_retain_until_tick
        || processing_tick >= watch.expires_at_tick
        || (matches!(strategy, AttentionStrategy::FixedPolling { .. })
            && processing_tick >= event.valid_until_tick)
    {
        if event.condition == ConditionOutcome::Match {
            metrics.relevant_events += 1;
        }
        if event.persistence == Persistence::EphemeralTransition
            && matches!(strategy, AttentionStrategy::FixedPolling { .. })
        {
            metrics.missed_ephemeral_events += 1;
        }
        records.push(EventRecordResult {
            event_id: event.event_id.clone(),
            incident_id: event.incident_id.clone(),
            sequence: event.sequence,
            delivered_tick: event.delivered_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::ExpiredBeforeProcessing,
            detection_latency_ticks: None,
            deadline_met: false,
        });
        return None;
    }

    if event.condition == ConditionOutcome::Match {
        metrics.relevant_events += 1;
    }

    match event.condition {
        ConditionOutcome::NoMatch => {
            metrics.quiet_events += 1;
            records.push(EventRecordResult {
                event_id: event.event_id.clone(),
                incident_id: event.incident_id.clone(),
                sequence: event.sequence,
                delivered_tick: event.delivered_tick,
                processed_tick: Some(processing_tick),
                disposition: RecordDisposition::Quiet,
                detection_latency_ticks: None,
                deadline_met: true,
            });
        }
        ConditionOutcome::Indeterminate => {
            metrics.indeterminate_events += 1;
            records.push(EventRecordResult {
                event_id: event.event_id.clone(),
                incident_id: event.incident_id.clone(),
                sequence: event.sequence,
                delivered_tick: event.delivered_tick,
                processed_tick: Some(processing_tick),
                disposition: RecordDisposition::IndeterminateCondition,
                detection_latency_ticks: None,
                deadline_met: false,
            });
            return Some(ScheduleStatus::Indeterminate);
        }
        ConditionOutcome::Match => {
            let latency = processing_tick.saturating_sub(event.event_tick);
            metrics.max_detection_latency_ticks = metrics.max_detection_latency_ticks.max(latency);
            metrics.max_event_backlog_ticks = metrics
                .max_event_backlog_ticks
                .max(processing_tick.saturating_sub(event.delivered_tick));
            let joins_existing = joins_pending_group(*strategy, event, processing_tick, pending);
            if !joins_existing && wakes.len() + pending.len() >= watch.max_wakeups {
                metrics.budget_exhaustions += 1;
                records.push(EventRecordResult {
                    event_id: event.event_id.clone(),
                    incident_id: event.incident_id.clone(),
                    sequence: event.sequence,
                    delivered_tick: event.delivered_tick,
                    processed_tick: Some(processing_tick),
                    disposition: RecordDisposition::BudgetExhausted,
                    detection_latency_ticks: Some(latency),
                    deadline_met: false,
                });
                return Some(ScheduleStatus::BudgetExhausted);
            }
            let coalesced = apply_wake(*strategy, event, processing_tick, pending, wakes, metrics);
            metrics.relevant_events_detected += 1;
            metrics.event_wake_notifications += 1;
            records.push(EventRecordResult {
                event_id: event.event_id.clone(),
                incident_id: event.incident_id.clone(),
                sequence: event.sequence,
                delivered_tick: event.delivered_tick,
                processed_tick: Some(processing_tick),
                disposition: if coalesced {
                    RecordDisposition::Coalesced
                } else {
                    RecordDisposition::Woke
                },
                detection_latency_ticks: Some(latency),
                deadline_met: true,
            });
        }
    }
    None
}

fn apply_restart(
    watch: &EventWatch,
    restart: &SourceRestart,
    processing_tick: u64,
    checkpoint: &mut EventCheckpoint,
    metrics: &mut EventScheduleMetrics,
    records: &mut Vec<EventRecordResult>,
) -> Option<ScheduleStatus> {
    metrics.source_records += 1;
    let valid = restart.source_id == watch.source_id
        && restart.previous_epoch == checkpoint.source_epoch
        && restart.new_epoch != checkpoint.source_epoch
        && non_empty(&restart.restart_proof_digest)
        && restart.sequence_start > 0;
    if !valid {
        metrics.restarts_rejected += 1;
        records.push(EventRecordResult {
            event_id: format!("restart:{}", restart.new_epoch),
            incident_id: String::new(),
            sequence: restart.sequence_start,
            delivered_tick: restart.at_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::RestartRejected,
            detection_latency_ticks: None,
            deadline_met: false,
        });
        return Some(ScheduleStatus::SourceEpochMismatch);
    }
    checkpoint.source_epoch = restart.new_epoch.clone();
    checkpoint.next_sequence = restart.sequence_start;
    checkpoint.last_stream_tick = restart.at_tick;
    metrics.restarts_accepted += 1;
    records.push(EventRecordResult {
        event_id: format!("restart:{}", restart.new_epoch),
        incident_id: String::new(),
        sequence: restart.sequence_start,
        delivered_tick: restart.at_tick,
        processed_tick: Some(processing_tick),
        disposition: RecordDisposition::RestartAccepted,
        detection_latency_ticks: None,
        deadline_met: true,
    });
    None
}

fn process_watermark(
    watch: &EventWatch,
    watermark: &SourceWatermark,
    processing_tick: u64,
    gap_policy: SequenceGapPolicy,
    checkpoint: &mut EventCheckpoint,
    metrics: &mut EventScheduleMetrics,
    records: &mut Vec<EventRecordResult>,
) -> Option<ScheduleStatus> {
    metrics.source_records += 1;
    metrics.watermarks_checked += 1;

    let event_id = format!(
        "watermark:{}:{}",
        watermark.source_epoch, watermark.next_sequence
    );
    if watermark.source_id != watch.source_id {
        metrics.scope_rejections += 1;
        records.push(EventRecordResult {
            event_id,
            incident_id: String::new(),
            sequence: watermark.next_sequence,
            delivered_tick: watermark.observed_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::ScopeRejected,
            detection_latency_ticks: None,
            deadline_met: false,
        });
        return Some(ScheduleStatus::SourceIdentityMismatch);
    }
    if !watermark.available {
        metrics.source_unavailable += 1;
        records.push(EventRecordResult {
            event_id,
            incident_id: String::new(),
            sequence: watermark.next_sequence,
            delivered_tick: watermark.observed_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::SourceUnavailable,
            detection_latency_ticks: None,
            deadline_met: false,
        });
        return Some(ScheduleStatus::SourceUnavailable);
    }
    if watermark.source_epoch != checkpoint.source_epoch {
        records.push(EventRecordResult {
            event_id,
            incident_id: String::new(),
            sequence: watermark.next_sequence,
            delivered_tick: watermark.observed_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::SourceEpochMismatch,
            detection_latency_ticks: None,
            deadline_met: false,
        });
        return Some(ScheduleStatus::SourceEpochMismatch);
    }
    if watermark.next_sequence > checkpoint.next_sequence {
        metrics.sequence_gaps += 1;
        records.push(EventRecordResult {
            event_id,
            incident_id: String::new(),
            sequence: watermark.next_sequence,
            delivered_tick: watermark.observed_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::DeliveryGap,
            detection_latency_ticks: None,
            deadline_met: false,
        });
        return Some(ScheduleStatus::DeliveryGap);
    }
    if watermark.next_sequence < checkpoint.next_sequence {
        metrics.out_of_order_records += 1;
        records.push(EventRecordResult {
            event_id,
            incident_id: String::new(),
            sequence: watermark.next_sequence,
            delivered_tick: watermark.observed_tick,
            processed_tick: Some(processing_tick),
            disposition: RecordDisposition::OutOfOrder,
            detection_latency_ticks: None,
            deadline_met: false,
        });
        return Some(ScheduleStatus::OutOfOrder);
    }

    records.push(EventRecordResult {
        event_id,
        incident_id: String::new(),
        sequence: watermark.next_sequence,
        delivered_tick: watermark.observed_tick,
        processed_tick: Some(processing_tick),
        disposition: RecordDisposition::WatermarkConfirmed,
        detection_latency_ticks: None,
        deadline_met: true,
    });
    checkpoint.last_stream_tick = watermark.observed_tick;
    None
}

/// Evaluate one frozen source schedule under a selected attention policy.
///
/// Fixed polling observes a condition only while it is currently valid. Event
/// triggered arms consume the delivered source record even after an ephemeral
/// condition has cleared, provided the record is still retained and its
/// response deadline has not passed. A sequence gap, identity collision or
/// unauthenticated epoch transition stops the schedule and remains explicit.
pub fn evaluate_event_schedule(
    watch: &EventWatch,
    authority: &AuthorityWindow,
    strategy: AttentionStrategy,
    timeline: &[TimelineRecord],
    horizon_tick: u64,
    checkpoint: Option<EventCheckpoint>,
) -> Result<EventScheduleResult, ScheduleError> {
    evaluate_event_schedule_with_gap_policy(
        watch,
        authority,
        strategy,
        timeline,
        horizon_tick,
        checkpoint,
        SequenceGapPolicy::strict(),
    )
}

/// Evaluate one frozen source schedule with an explicit bounded gap policy.
/// The default public evaluator remains strict; buffering is an opt-in contract.
pub fn evaluate_event_schedule_with_gap_policy(
    watch: &EventWatch,
    authority: &AuthorityWindow,
    strategy: AttentionStrategy,
    timeline: &[TimelineRecord],
    horizon_tick: u64,
    checkpoint: Option<EventCheckpoint>,
    gap_policy: SequenceGapPolicy,
) -> Result<EventScheduleResult, ScheduleError> {
    validate_watch(watch)?;
    validate_authority(authority)?;
    if (gap_policy.timeout_ticks == 0) != (gap_policy.max_buffered_events == 0)
        || gap_policy.max_buffered_events > watch.max_event_checks
    {
        return Err(ScheduleError::InvalidGapPolicy);
    }
    match strategy {
        AttentionStrategy::FixedPolling { interval_ticks: 0 } => {
            return Err(ScheduleError::InvalidStrategy);
        }
        AttentionStrategy::CoalescedEventTriggered {
            coalesce_window_ticks: 0,
        } => return Err(ScheduleError::InvalidStrategy),
        _ => {}
    }

    let mut checkpoint = checkpoint.unwrap_or_else(|| initial_checkpoint(watch));
    validate_checkpoint(watch, &checkpoint)?;
    let mut last_timeline_tick = checkpoint.last_stream_tick;
    for record in timeline {
        if record.tick() < last_timeline_tick {
            return Err(ScheduleError::EventTimeRegression);
        }
        last_timeline_tick = record.tick();
        match record {
            TimelineRecord::Event(event) => validate_event(event)?,
            TimelineRecord::Restart(restart) => validate_restart(restart)?,
            TimelineRecord::Watermark(watermark) => validate_watermark(watermark)?,
        }
    }

    let mut metrics = EventScheduleMetrics::default();
    let mut records = Vec::new();
    let mut wakes = Vec::new();
    let mut pending = Vec::new();
    let mut status = ScheduleStatus::Completed;

    if horizon_tick < watch.starts_at_tick {
        return Ok(make_result(
            watch,
            strategy,
            authority,
            ScheduleOutput {
                status: ScheduleStatus::Quiet,
                metrics,
                records,
                wakes,
                checkpoint,
            },
        ));
    }

    if !authority.current_for(watch, watch.starts_at_tick) {
        return Ok(make_result(
            watch,
            strategy,
            authority,
            ScheduleOutput {
                status: ScheduleStatus::AuthorityRevokedOrExpired,
                metrics,
                records,
                wakes,
                checkpoint,
            },
        ));
    }

    let poll_interval = match strategy {
        AttentionStrategy::FixedPolling { interval_ticks } => Some(interval_ticks),
        _ => None,
    };

    for item in timeline.iter() {
        let delivered_tick = item.tick();
        if delivered_tick > horizon_tick {
            break;
        }
        if delivered_tick < watch.starts_at_tick {
            continue;
        }
        if delivered_tick >= watch.expires_at_tick {
            status = ScheduleStatus::WatchExpired;
            break;
        }

        let processing_tick = match poll_interval {
            Some(interval) => {
                let Some(poll_tick) =
                    poll_tick_at_or_after(watch.starts_at_tick, interval, delivered_tick)
                else {
                    status = ScheduleStatus::InvalidSchedule;
                    break;
                };
                if poll_tick > horizon_tick || poll_tick >= watch.expires_at_tick {
                    status = ScheduleStatus::WatchExpired;
                    break;
                }
                poll_tick
            }
            None => delivered_tick,
        };

        if processing_tick >= watch.expires_at_tick {
            status = ScheduleStatus::WatchExpired;
            break;
        }

        if !authority.current_for(watch, processing_tick) {
            status = ScheduleStatus::AuthorityRevokedOrExpired;
            break;
        }

        if metrics.source_records >= watch.max_event_checks {
            metrics.budget_exhaustions += 1;
            status = ScheduleStatus::BudgetExhausted;
            break;
        }

        let result = match item {
            TimelineRecord::Event(event) => {
                let mut state = EventProcessingState {
                    watch,
                    authority,
                    strategy,
                    gap_policy,
                    checkpoint: &mut checkpoint,
                    metrics: &mut metrics,
                    records: &mut records,
                    pending: &mut pending,
                    wakes: &mut wakes,
                };
                process_event(&mut state, event, processing_tick)
            }
            TimelineRecord::Restart(restart) => apply_restart(
                watch,
                restart,
                processing_tick,
                gap_policy,
                &mut checkpoint,
                &mut metrics,
                &mut records,
            ),
            TimelineRecord::Watermark(watermark) => process_watermark(
                watch,
                watermark,
                processing_tick,
                &mut checkpoint,
                &mut metrics,
                &mut records,
            ),
        };
        if let Some(failure) = result {
            status = failure;
            break;
        }
    }

    if gap_policy.is_buffered() && status == ScheduleStatus::Completed {
        if let Some(deadline) = checkpoint.gap_deadline_tick {
            if horizon_tick >= deadline {
                metrics.gap_timeouts += 1;
                status = ScheduleStatus::DeliveryGap;
                records.push(EventRecordResult {
                    event_id: format!(
                        "gap:{}:{}",
                        checkpoint.source_epoch, checkpoint.next_sequence
                    ),
                    incident_id: String::new(),
                    sequence: checkpoint.next_sequence,
                    delivered_tick: deadline,
                    processed_tick: Some(horizon_tick),
                    disposition: RecordDisposition::DeliveryGap,
                    detection_latency_ticks: None,
                    deadline_met: false,
                });
            } else {
                metrics.pending_gap_observations += 1;
                status = ScheduleStatus::DeliveryGapPending;
                records.push(EventRecordResult {
                    event_id: format!(
                        "gap-pending:{}:{}",
                        checkpoint.source_epoch, checkpoint.next_sequence
                    ),
                    incident_id: String::new(),
                    sequence: checkpoint.next_sequence,
                    delivered_tick: checkpoint.gap_started_at_tick.unwrap_or(horizon_tick),
                    processed_tick: Some(horizon_tick),
                    disposition: RecordDisposition::DeliveryGapPending,
                    detection_latency_ticks: None,
                    deadline_met: false,
                });
            }
        }
    }

    // A coalescing group closes after the last in-window record. Coalescing
    // reduces cognitive wakes; it never erases incident IDs or event receipts.
    flush_pending(&mut pending, &mut wakes);
    metrics.wakeups = wakes.len();
    metrics.cognitive_invocations = wakes.len();

    if status == ScheduleStatus::Completed && metrics.wakeups == 0 {
        status = ScheduleStatus::Quiet;
    }

    Ok(make_result(
        watch,
        strategy,
        authority,
        ScheduleOutput {
            status,
            metrics,
            records,
            wakes,
            checkpoint,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn watch() -> EventWatch {
        EventWatch {
            watch_id: "watch-opsbench-1".into(),
            source_id: "event-feed-a".into(),
            initial_source_epoch: "epoch-1".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            event_type: "service-degraded".into(),
            starts_at_tick: 0,
            expires_at_tick: 40,
            first_sequence: 1,
            max_event_checks: 64,
            max_wakeups: 16,
        }
    }

    fn authority() -> AuthorityWindow {
        AuthorityWindow {
            epoch: 7,
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            valid_from_tick: 0,
            expires_at_tick: 40,
            revoked_at_tick: None,
        }
    }

    fn event(
        event_id: &str,
        incident_id: &str,
        sequence: u64,
        event_tick: u64,
        delivered_tick: u64,
        valid_until_tick: u64,
        deadline_tick: u64,
        persistence: Persistence,
    ) -> SourceEvent {
        SourceEvent {
            source_id: "event-feed-a".into(),
            source_epoch: "epoch-1".into(),
            event_id: event_id.into(),
            incident_id: incident_id.into(),
            sequence,
            payload_digest: format!("sha256:{:064x}", sequence),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            event_type: "service-degraded".into(),
            coalesce_key: "service-a-degradation".into(),
            condition: ConditionOutcome::Match,
            persistence,
            event_tick,
            valid_from_tick: event_tick,
            valid_until_tick,
            delivered_tick,
            record_retain_until_tick: 39,
            deadline_tick,
        }
    }

    fn run(strategy: AttentionStrategy, timeline: &[TimelineRecord]) -> EventScheduleResult {
        evaluate_event_schedule(&watch(), &authority(), strategy, timeline, 39, None).unwrap()
    }

    fn watermark(next_sequence: u64, observed_tick: u64, available: bool) -> SourceWatermark {
        SourceWatermark {
            source_id: "event-feed-a".into(),
            source_epoch: "epoch-1".into(),
            next_sequence,
            observed_tick,
            available,
            watermark_digest: format!("sha256:{:064x}", next_sequence + observed_tick),
        }
    }

    #[test]
    fn final_lost_event_is_detected_by_source_watermark() {
        let timeline = vec![
            TimelineRecord::Event(event(
                "event-1",
                "incident-1",
                1,
                2,
                2,
                30,
                12,
                Persistence::PersistentCondition,
            )),
            TimelineRecord::Watermark(watermark(3, 8, true)),
        ];
        let result = run(AttentionStrategy::EventTriggered, &timeline);
        assert_eq!(result.status, ScheduleStatus::DeliveryGap);
        assert_eq!(result.metrics.sequence_gaps, 1);
        assert_eq!(result.metrics.watermarks_checked, 1);
        assert_eq!(result.metrics.wakeups, 1);
        assert_eq!(
            result.records.last().unwrap().disposition,
            RecordDisposition::DeliveryGap
        );
    }

    #[test]
    fn watermark_at_current_cursor_confirms_quiet_is_not_source_loss() {
        let timeline = vec![TimelineRecord::Watermark(watermark(1, 8, true))];
        let result = run(AttentionStrategy::EventTriggered, &timeline);
        assert_eq!(result.status, ScheduleStatus::Quiet);
        assert_eq!(result.metrics.watermarks_checked, 1);
        assert_eq!(result.metrics.sequence_gaps, 0);
        assert_eq!(
            result.records[0].disposition,
            RecordDisposition::WatermarkConfirmed
        );
    }

    #[test]
    fn unavailable_source_is_not_reported_as_quiet() {
        let timeline = vec![TimelineRecord::Watermark(watermark(1, 8, false))];
        let result = run(AttentionStrategy::EventTriggered, &timeline);
        assert_eq!(result.status, ScheduleStatus::SourceUnavailable);
        assert_eq!(result.metrics.source_unavailable, 1);
        assert_ne!(result.status, ScheduleStatus::Quiet);
    }

    #[test]
    fn foreign_source_event_is_rejected_before_sequence_cursor_advances() {
        let foreign = SourceEvent {
            source_id: "other-source".into(),
            ..event(
                "foreign-event",
                "incident-foreign",
                1,
                3,
                3,
                30,
                10,
                Persistence::PersistentCondition,
            )
        };
        let result = run(
            AttentionStrategy::EventTriggered,
            &[TimelineRecord::Event(foreign)],
        );
        assert_eq!(result.status, ScheduleStatus::SourceIdentityMismatch);
        assert_eq!(result.checkpoint.next_sequence, 1);
        assert_eq!(result.metrics.wakeups, 0);
    }

    #[test]
    fn delayed_delivery_before_deadline_keeps_original_event_time() {
        let result = run(
            AttentionStrategy::EventTriggered,
            &[TimelineRecord::Event(event(
                "event-delayed",
                "incident-delayed",
                1,
                2,
                5,
                8,
                7,
                Persistence::PersistentCondition,
            ))],
        );
        assert_eq!(result.records[0].processed_tick, Some(5));
        assert_eq!(result.records[0].detection_latency_ticks, Some(3));
        assert!(result.records[0].deadline_met);
        assert_eq!(result.metrics.missed_deadlines, 0);
    }

    #[test]
    fn late_ephemeral_delivery_misses_deadline_and_is_not_a_wake() {
        let result = run(
            AttentionStrategy::EventTriggered,
            &[TimelineRecord::Event(event(
                "event-too-late",
                "incident-too-late",
                1,
                2,
                8,
                5,
                7,
                Persistence::EphemeralTransition,
            ))],
        );
        assert_eq!(result.status, ScheduleStatus::Quiet);
        assert_eq!(result.metrics.missed_deadlines, 1);
        assert_eq!(result.metrics.missed_ephemeral_events, 1);
        assert_eq!(result.metrics.wakeups, 0);
        assert_eq!(
            result.records[0].disposition,
            RecordDisposition::MissedDeadline
        );
    }

    #[test]
    fn fixed_polling_detects_persistent_condition_at_next_poll() {
        let result = run(
            AttentionStrategy::FixedPolling { interval_ticks: 10 },
            &[TimelineRecord::Event(event(
                "event-persistent",
                "incident-persistent",
                1,
                3,
                3,
                25,
                12,
                Persistence::PersistentCondition,
            ))],
        );
        assert_eq!(result.metrics.wakeups, 1);
        assert_eq!(result.records[0].processed_tick, Some(10));
        assert_eq!(result.records[0].detection_latency_ticks, Some(7));
    }

    #[test]
    fn coalescing_never_merges_different_incident_keys() {
        let mut second = event(
            "event-2",
            "incident-2",
            2,
            4,
            4,
            30,
            13,
            Persistence::PersistentCondition,
        );
        second.coalesce_key = "other-service".into();
        let result = run(
            AttentionStrategy::CoalescedEventTriggered {
                coalesce_window_ticks: 4,
            },
            &[
                TimelineRecord::Event(event(
                    "event-1",
                    "incident-1",
                    1,
                    3,
                    3,
                    30,
                    12,
                    Persistence::PersistentCondition,
                )),
                TimelineRecord::Event(second),
            ],
        );
        assert_eq!(result.metrics.coalesced_events, 0);
        assert_eq!(result.wakes.len(), 2);
        assert_eq!(result.wakes[0].incident_ids, vec!["incident-1"]);
        assert_eq!(result.wakes[1].incident_ids, vec!["incident-2"]);
    }

    #[test]
    fn wake_budget_counts_coalesced_wake_groups_not_raw_event_count() {
        let mut w = watch();
        w.max_wakeups = 1;
        let timeline = vec![
            TimelineRecord::Event(event(
                "event-1",
                "incident-1",
                1,
                3,
                3,
                30,
                12,
                Persistence::PersistentCondition,
            )),
            TimelineRecord::Event(event(
                "event-2",
                "incident-2",
                2,
                4,
                4,
                30,
                13,
                Persistence::PersistentCondition,
            )),
        ];
        let result = evaluate_event_schedule(
            &w,
            &authority(),
            AttentionStrategy::CoalescedEventTriggered {
                coalesce_window_ticks: 2,
            },
            &timeline,
            39,
            None,
        )
        .unwrap();
        assert_eq!(result.status, ScheduleStatus::Completed);
        assert_eq!(result.metrics.relevant_events_detected, 2);
        assert_eq!(result.metrics.wakeups, 1);
        assert_eq!(result.metrics.cognitive_invocations, 1);
    }

    #[test]
    fn wake_budget_exhaustion_does_not_drop_incident_as_quiet() {
        let mut w = watch();
        w.max_wakeups = 1;
        let timeline = vec![
            TimelineRecord::Event(event(
                "event-1",
                "incident-1",
                1,
                3,
                3,
                30,
                12,
                Persistence::PersistentCondition,
            )),
            TimelineRecord::Event(event(
                "event-2",
                "incident-2",
                2,
                10,
                10,
                30,
                20,
                Persistence::PersistentCondition,
            )),
        ];
        let result = evaluate_event_schedule(
            &w,
            &authority(),
            AttentionStrategy::EventTriggered,
            &timeline,
            39,
            None,
        )
        .unwrap();
        assert_eq!(result.status, ScheduleStatus::BudgetExhausted);
        assert_eq!(result.metrics.relevant_events, 2);
        assert_eq!(result.metrics.relevant_events_detected, 1);
        assert_eq!(result.metrics.budget_exhaustions, 1);
    }

    #[test]
    fn event_triggered_wakes_twice_for_distinct_incidents_while_state_stays_high() {
        let timeline = vec![
            TimelineRecord::Event(event(
                "event-1",
                "incident-1",
                1,
                5,
                5,
                30,
                15,
                Persistence::PersistentCondition,
            )),
            TimelineRecord::Event(event(
                "event-2",
                "incident-2",
                2,
                7,
                7,
                30,
                17,
                Persistence::PersistentCondition,
            )),
        ];
        let result = run(AttentionStrategy::EventTriggered, &timeline);
        assert_eq!(result.metrics.relevant_events, 2);
        assert_eq!(result.metrics.relevant_events_detected, 2);
        assert_eq!(result.metrics.wakeups, 2);
        assert_eq!(result.wakes.len(), 2);
        assert_eq!(result.wakes[0].incident_ids, vec!["incident-1"]);
        assert_eq!(result.wakes[1].incident_ids, vec!["incident-2"]);
    }

    #[test]
    fn event_triggered_detects_ephemeral_event_after_state_has_cleared() {
        let timeline = vec![TimelineRecord::Event(event(
            "event-ephemeral",
            "incident-short",
            1,
            3,
            6,
            5,
            9,
            Persistence::EphemeralTransition,
        ))];
        let result = run(AttentionStrategy::EventTriggered, &timeline);
        assert_eq!(result.metrics.relevant_events_detected, 1);
        assert_eq!(result.records[0].processed_tick, Some(6));
        assert_eq!(result.records[0].disposition, RecordDisposition::Woke);
    }

    #[test]
    fn fixed_polling_misses_ephemeral_state_that_clears_between_polls() {
        let timeline = vec![TimelineRecord::Event(event(
            "event-ephemeral",
            "incident-short",
            1,
            3,
            3,
            5,
            12,
            Persistence::EphemeralTransition,
        ))];
        let result = run(
            AttentionStrategy::FixedPolling { interval_ticks: 10 },
            &timeline,
        );
        assert_eq!(result.metrics.relevant_events_detected, 0);
        assert_eq!(result.metrics.missed_ephemeral_events, 1);
        assert_eq!(
            result.records[0].disposition,
            RecordDisposition::ExpiredBeforeProcessing
        );
    }

    #[test]
    fn exact_duplicate_delivery_is_suppressed_after_checkpoint_replay() {
        let first = event(
            "event-1",
            "incident-1",
            1,
            3,
            3,
            30,
            10,
            Persistence::PersistentCondition,
        );
        let initial = run(
            AttentionStrategy::EventTriggered,
            &[TimelineRecord::Event(first.clone())],
        );
        let replayed = run_from_checkpoint(
            &watch(),
            &authority(),
            AttentionStrategy::EventTriggered,
            &[TimelineRecord::Event(first)],
            39,
            initial.checkpoint,
        )
        .unwrap();
        assert_eq!(replayed.metrics.duplicates_suppressed, 1);
        assert_eq!(replayed.metrics.wakeups, 0);
    }

    #[test]
    fn event_id_reuse_with_different_sequence_is_an_identity_collision() {
        let first = event(
            "event-reused",
            "incident-1",
            1,
            3,
            3,
            30,
            10,
            Persistence::PersistentCondition,
        );
        let mut second = event(
            "event-reused",
            "incident-2",
            2,
            4,
            4,
            30,
            11,
            Persistence::PersistentCondition,
        );
        second.payload_digest = format!("sha256:{:064x}", 999);
        let result = run(
            AttentionStrategy::EventTriggered,
            &[TimelineRecord::Event(first), TimelineRecord::Event(second)],
        );
        assert_eq!(result.status, ScheduleStatus::IdentityCollision);
        assert_eq!(result.metrics.identity_collisions, 1);
        assert_eq!(result.metrics.wakeups, 1);
    }

    #[test]
    fn sequence_gap_escalates_without_marking_later_event_healthy() {
        let timeline = vec![TimelineRecord::Event(event(
            "event-gap",
            "incident-2",
            2,
            3,
            3,
            30,
            10,
            Persistence::PersistentCondition,
        ))];
        let result = run(AttentionStrategy::EventTriggered, &timeline);
        assert_eq!(result.status, ScheduleStatus::DeliveryGap);
        assert_eq!(result.metrics.sequence_gaps, 1);
        assert_eq!(result.metrics.wakeups, 0);
    }

    #[test]
    fn out_of_order_sequence_is_not_counted_as_a_duplicate() {
        let timeline = vec![
            TimelineRecord::Event(event(
                "event-1",
                "incident-1",
                1,
                2,
                2,
                30,
                10,
                Persistence::PersistentCondition,
            )),
            TimelineRecord::Event(event(
                "event-older",
                "incident-old",
                1,
                3,
                3,
                30,
                10,
                Persistence::PersistentCondition,
            )),
        ];
        let result = run(AttentionStrategy::EventTriggered, &timeline);
        assert_eq!(result.status, ScheduleStatus::OutOfOrder);
        assert_eq!(result.metrics.out_of_order_records, 1);
        assert_eq!(result.metrics.duplicates_suppressed, 0);
    }

    #[test]
    fn explicit_source_restart_resets_sequence_only_after_verified_record() {
        let timeline = vec![
            TimelineRecord::Event(event(
                "event-1",
                "incident-1",
                1,
                2,
                2,
                30,
                10,
                Persistence::PersistentCondition,
            )),
            TimelineRecord::Restart(SourceRestart {
                source_id: "event-feed-a".into(),
                previous_epoch: "epoch-1".into(),
                new_epoch: "epoch-2".into(),
                sequence_start: 1,
                at_tick: 5,
                restart_proof_digest: format!("sha256:{:064x}", 12345),
            }),
            TimelineRecord::Event(SourceEvent {
                source_epoch: "epoch-2".into(),
                ..event(
                    "event-2",
                    "incident-2",
                    1,
                    6,
                    6,
                    30,
                    12,
                    Persistence::PersistentCondition,
                )
            }),
        ];
        let result = run(AttentionStrategy::EventTriggered, &timeline);
        assert_eq!(result.metrics.restarts_accepted, 1);
        assert_eq!(result.metrics.wakeups, 2);
        assert_eq!(result.checkpoint.source_epoch, "epoch-2");
    }

    #[test]
    fn epoch_change_without_restart_record_is_rejected() {
        let timeline = vec![TimelineRecord::Event(SourceEvent {
            source_epoch: "epoch-2".into(),
            ..event(
                "event-1",
                "incident-1",
                1,
                2,
                2,
                30,
                10,
                Persistence::PersistentCondition,
            )
        })];
        let result = run(AttentionStrategy::EventTriggered, &timeline);
        assert_eq!(result.status, ScheduleStatus::SourceEpochMismatch);
        assert_eq!(result.metrics.wakeups, 0);
    }

    #[test]
    fn dormant_revocation_prevents_data_wake_and_probe_budget_credit() {
        let mut revoked = authority();
        revoked.revoked_at_tick = Some(4);
        let timeline = vec![TimelineRecord::Event(event(
            "event-after-revocation",
            "incident-1",
            1,
            5,
            5,
            30,
            10,
            Persistence::PersistentCondition,
        ))];
        let mut w = watch();
        w.max_wakeups = 0;
        let result = evaluate_event_schedule(
            &w,
            &revoked,
            AttentionStrategy::EventTriggered,
            &timeline,
            39,
            None,
        )
        .unwrap();
        assert_eq!(result.status, ScheduleStatus::AuthorityRevokedOrExpired);
        assert_eq!(result.metrics.wakeups, 0);
        assert_eq!(result.metrics.source_records, 0);
    }

    #[test]
    fn coalescing_reduces_wakeups_but_preserves_every_event_and_incident_id() {
        let timeline = vec![
            TimelineRecord::Event(event(
                "event-1",
                "incident-1",
                1,
                3,
                3,
                30,
                12,
                Persistence::PersistentCondition,
            )),
            TimelineRecord::Event(event(
                "event-2",
                "incident-2",
                2,
                4,
                4,
                30,
                13,
                Persistence::PersistentCondition,
            )),
            TimelineRecord::Event(event(
                "event-3",
                "incident-3",
                3,
                15,
                15,
                30,
                22,
                Persistence::PersistentCondition,
            )),
        ];
        let result = run(
            AttentionStrategy::CoalescedEventTriggered {
                coalesce_window_ticks: 2,
            },
            &timeline,
        );
        assert_eq!(result.metrics.relevant_events_detected, 3);
        assert_eq!(result.metrics.coalesced_events, 1);
        assert_eq!(result.metrics.event_wake_notifications, 3);
        assert_eq!(result.wakes.len(), 2);
        assert_eq!(
            result.wakes[0].incident_ids,
            vec!["incident-1", "incident-2"]
        );
        assert_eq!(result.wakes[0].event_ids, vec!["event-1", "event-2"]);
        assert_eq!(result.metrics.wakeups, 2);
        assert_eq!(result.metrics.cognitive_invocations, 2);
    }

    #[test]
    fn expired_authority_never_becomes_a_quiet_success() {
        let mut expired = authority();
        expired.expires_at_tick = 4;
        let timeline = vec![TimelineRecord::Event(event(
            "event-after-expiry",
            "incident-1",
            1,
            5,
            5,
            30,
            10,
            Persistence::PersistentCondition,
        ))];
        let result = evaluate_event_schedule(
            &watch(),
            &expired,
            AttentionStrategy::EventTriggered,
            &timeline,
            39,
            None,
        )
        .unwrap();
        assert_eq!(result.status, ScheduleStatus::AuthorityRevokedOrExpired);
        assert_ne!(result.status, ScheduleStatus::Quiet);
    }

    #[test]
    fn missing_or_indeterminate_condition_escalates_without_false_wake() {
        let mut uncertain = event(
            "event-unknown",
            "incident-1",
            1,
            3,
            3,
            30,
            10,
            Persistence::PersistentCondition,
        );
        uncertain.condition = ConditionOutcome::Indeterminate;
        let result = run(
            AttentionStrategy::EventTriggered,
            &[TimelineRecord::Event(uncertain)],
        );
        assert_eq!(result.status, ScheduleStatus::Indeterminate);
        assert_eq!(result.metrics.wakeups, 0);
        assert_eq!(result.metrics.indeterminate_events, 1);
    }

    #[test]
    fn no_match_event_is_quiet_not_a_successful_wake() {
        let mut irrelevant = event(
            "event-quiet",
            "incident-quiet",
            1,
            3,
            3,
            30,
            10,
            Persistence::PersistentCondition,
        );
        irrelevant.condition = ConditionOutcome::NoMatch;
        let result = run(
            AttentionStrategy::EventTriggered,
            &[TimelineRecord::Event(irrelevant)],
        );
        assert_eq!(result.status, ScheduleStatus::Quiet);
        assert_eq!(result.metrics.wakeups, 0);
        assert_eq!(result.metrics.quiet_events, 1);
    }

    #[test]
    fn event_wake_never_creates_a_consequential_mutation_attempt() {
        let timeline = vec![TimelineRecord::Event(event(
            "event-1",
            "incident-1",
            1,
            3,
            3,
            30,
            10,
            Persistence::PersistentCondition,
        ))];
        let result = run(AttentionStrategy::EventTriggered, &timeline);
        assert_eq!(result.consequential_mutation_attempts, 0);
        assert_eq!(result.metrics.consequential_mutation_attempts, 0);
    }
}

/// Resume from an explicit durable cursor/checkpoint. Replayed IDs are
/// deduplicated against the checkpoint; the checkpoint is caller-owned and
/// must be persisted in the existing event inbox/outbox owner, not here.
pub fn run_from_checkpoint(
    watch: &EventWatch,
    authority: &AuthorityWindow,
    strategy: AttentionStrategy,
    timeline: &[TimelineRecord],
    horizon_tick: u64,
    checkpoint: EventCheckpoint,
) -> Result<EventScheduleResult, ScheduleError> {
    evaluate_event_schedule(
        watch,
        authority,
        strategy,
        timeline,
        horizon_tick,
        Some(checkpoint),
    )
}
