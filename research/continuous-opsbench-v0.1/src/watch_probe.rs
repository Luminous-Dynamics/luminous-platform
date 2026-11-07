//! Deterministic watch-to-probe episode reference.
//!
//! Offline conformance model for a bounded declarative watch, current authority,
//! catalogued read-only probe, source-owned observation, separate impact
//! observation, and evaluator-only decision truth. It does not subscribe to a
//! production event bus, authenticate publishers, execute a collector, or grant
//! mutation authority.

use std::collections::{BTreeMap, BTreeSet};

pub const MAX_RISK_PPM: u32 = 1_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchSpec {
    pub watch_id: String,
    pub tenant_id: String,
    pub target_id: String,
    pub purpose_id: String,
    pub source_id: String,
    pub event_type: String,
    pub first_sequence: u64,
    pub starts_at_tick: u64,
    pub expires_at_tick: u64,
    pub max_wakeups: u32,
    pub max_probe_duration_ticks: u64,
    pub max_observation_age_ticks: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoritySnapshot {
    pub epoch: u64,
    pub tenant_id: String,
    pub target_id: String,
    pub purpose_id: String,
    pub valid_from_tick: u64,
    pub expires_at_tick: u64,
    /// Revocation is effective at this tick, inclusive.
    pub revoked_at_tick: Option<u64>,
}

impl AuthoritySnapshot {
    fn current_for(&self, watch: &WatchSpec, at_tick: u64) -> bool {
        self.tenant_id == watch.tenant_id
            && self.target_id == watch.target_id
            && self.purpose_id == watch.purpose_id
            && at_tick >= self.valid_from_tick
            && at_tick < self.expires_at_tick
            && self.revoked_at_tick.is_none_or(|revoked| at_tick < revoked)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventEnvelope {
    pub event_id: String,
    pub source_id: String,
    pub sequence: u64,
    pub event_type: String,
    pub tenant_id: String,
    pub target_id: String,
    pub purpose_id: String,
    pub condition_met: bool,
    pub observed_at_tick: u64,
    pub delivered_at_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeCatalogEntry {
    pub probe_id: String,
    pub operation_digest: String,
    pub tenant_id: String,
    pub target_id: String,
    pub purpose_id: String,
    pub source_id: String,
    pub collector_control_domain: String,
    pub read_only: bool,
    pub maximum_duration_ticks: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WakeContext {
    pub watch_id: String,
    pub event: EventEnvelope,
    pub authority_epoch: u64,
}

/// A policy proposes only a catalog identifier. It cannot supply an operation,
/// target, tenant, purpose, credential, or permission.
pub trait ProbePolicy {
    /// Receives only identifiers that have already passed watch-scope and
    /// read-only eligibility checks. It never sees the raw probe catalog.
    fn select_probe(&mut self, wake: &WakeContext, eligible_probe_ids: &[String])
        -> Option<String>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptedProbePolicy {
    pub selected_probe_id: Option<String>,
}

impl ProbePolicy for ScriptedProbePolicy {
    fn select_probe(
        &mut self,
        _wake: &WakeContext,
        _eligible_probe_ids: &[String],
    ) -> Option<String> {
        self.selected_probe_id.clone()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceObservation {
    pub observation_id: String,
    pub digest: String,
    pub probe_id: String,
    pub operation_digest: String,
    pub tenant_id: String,
    pub target_id: String,
    pub purpose_id: String,
    pub source_id: String,
    pub control_domain: String,
    pub observed_at_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImpactObservation {
    pub impact_id: String,
    pub digest: String,
    pub probe_id: String,
    pub operation_digest: String,
    pub tenant_id: String,
    pub target_id: String,
    pub purpose_id: String,
    pub observer_control_domain: String,
    pub window_start_tick: u64,
    pub window_end_tick: u64,
    pub measured_at_tick: u64,
    pub cpu_milliticks: u64,
    pub io_bytes: u64,
    pub complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollectionOutcome {
    NotStarted,
    Completed(Box<SourceObservation>),
    Failed,
    /// The client timed out, but the collector may still consume resources.
    TimedOutStillRunning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImpactOutcome {
    Observed(Box<ImpactObservation>),
    Unknown,
    NotMeasured,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluatorTruth {
    /// Hidden from ProbePolicy; consumed only after acquisition.
    pub decision_loss_before_ppm: u32,
    pub decision_loss_after_ppm: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EpisodeStatus {
    NotStarted,
    Quiet,
    Expired,
    DeliveryGap,
    DuplicateEventIdentity,
    AuthorityRevokedOrExpired,
    ProbeCatalogInvalid,
    NoProbeSelected,
    ProbeDenied,
    ProbeNotStarted,
    ProbeFailed,
    ProbeOutcomeUnknown,
    ObservationRejected,
    ImpactUnknown,
    ImpactIndependenceConflict,
    Completed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalDisposition {
    Stop,
    AskPrincipal,
    Abstain,
    Escalate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImpactDisposition {
    Observed,
    Unknown,
    NotMeasured,
    IndependenceConflict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionValue {
    pub loss_before_ppm: u32,
    pub loss_after_ppm: u32,
    /// Positive means the acquired evidence improved the evaluator's decision.
    pub realized_loss_reduction_ppm: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpisodeResult {
    pub status: EpisodeStatus,
    pub terminal: TerminalDisposition,
    pub watch_id: String,
    pub authority_epoch: u64,
    pub delivered_events: usize,
    pub duplicate_events: usize,
    pub rejected_scope_events: usize,
    pub wakeups: usize,
    pub selected_probe_id: Option<String>,
    pub probe_admitted: bool,
    pub consequential_mutation_attempts: usize,
    pub observation: Option<SourceObservation>,
    pub impact: ImpactDisposition,
    pub impact_observation: Option<ImpactObservation>,
    pub decision_value: Option<DecisionValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EpisodeError {
    InvalidWatch,
    InvalidAuthority,
    InvalidEventIdentity,
    InvalidProbeCatalog,
    EventTimeRegression,
    InvalidEventTime,
    InvalidEvaluatorLoss,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_watch(watch: &WatchSpec) -> Result<(), EpisodeError> {
    if !non_empty(&watch.watch_id)
        || !non_empty(&watch.tenant_id)
        || !non_empty(&watch.target_id)
        || !non_empty(&watch.purpose_id)
        || !non_empty(&watch.source_id)
        || !non_empty(&watch.event_type)
        || watch.first_sequence == 0
        || watch.starts_at_tick >= watch.expires_at_tick
        || watch.max_wakeups == 0
        || watch.max_probe_duration_ticks == 0
        || watch.max_observation_age_ticks == 0
    {
        return Err(EpisodeError::InvalidWatch);
    }
    Ok(())
}

fn validate_authority(authority: &AuthoritySnapshot) -> Result<(), EpisodeError> {
    if authority.epoch == 0
        || !non_empty(&authority.tenant_id)
        || !non_empty(&authority.target_id)
        || !non_empty(&authority.purpose_id)
        || authority.valid_from_tick >= authority.expires_at_tick
    {
        return Err(EpisodeError::InvalidAuthority);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // Explicitly constructs a result from independent counters.
fn empty_result(
    watch: &WatchSpec,
    authority: &AuthoritySnapshot,
    status: EpisodeStatus,
    terminal: TerminalDisposition,
    delivered_events: usize,
    duplicate_events: usize,
    rejected_scope_events: usize,
    wakeups: usize,
) -> EpisodeResult {
    EpisodeResult {
        status,
        terminal,
        watch_id: watch.watch_id.clone(),
        authority_epoch: authority.epoch,
        delivered_events,
        duplicate_events,
        rejected_scope_events,
        wakeups,
        selected_probe_id: None,
        probe_admitted: false,
        consequential_mutation_attempts: 0,
        observation: None,
        impact: ImpactDisposition::NotMeasured,
        impact_observation: None,
        decision_value: None,
    }
}

/// Run one deterministic scripted reference episode.
///
/// The policy receives only a matching event, watch ID, authority epoch and
/// catalog. The current tick, collector result, impact result and evaluator
/// decision-loss truth are not passed to it. One episode can acquire at most
/// one probe; an unknown timed-out collection is never retried here.
#[allow(clippy::too_many_arguments)] // Inputs deliberately keep hidden evaluator truth separate from the policy argument.
pub fn run_scripted_episode<P: ProbePolicy>(
    watch: &WatchSpec,
    authority: &AuthoritySnapshot,
    catalog: &[ProbeCatalogEntry],
    events: &[EventEnvelope],
    current_tick: u64,
    policy: &mut P,
    collection: CollectionOutcome,
    impact_outcome: ImpactOutcome,
    evaluator_truth: &EvaluatorTruth,
) -> Result<EpisodeResult, EpisodeError> {
    validate_watch(watch)?;
    validate_authority(authority)?;
    if evaluator_truth.decision_loss_before_ppm > MAX_RISK_PPM
        || evaluator_truth.decision_loss_after_ppm > MAX_RISK_PPM
    {
        return Err(EpisodeError::InvalidEvaluatorLoss);
    }

    if current_tick < watch.starts_at_tick {
        return Ok(empty_result(
            watch,
            authority,
            EpisodeStatus::NotStarted,
            TerminalDisposition::Stop,
            0,
            0,
            0,
            0,
        ));
    }
    if current_tick >= watch.expires_at_tick {
        return Ok(empty_result(
            watch,
            authority,
            EpisodeStatus::Expired,
            TerminalDisposition::Stop,
            0,
            0,
            0,
            0,
        ));
    }
    if !authority.current_for(watch, current_tick) {
        return Ok(empty_result(
            watch,
            authority,
            EpisodeStatus::AuthorityRevokedOrExpired,
            TerminalDisposition::Escalate,
            0,
            0,
            0,
            0,
        ));
    }

    let mut delivered_events = 0usize;
    let mut duplicate_events = 0usize;
    let mut rejected_scope_events = 0usize;
    let mut last_sequence: Option<u64> = None;
    let mut seen_event_ids: BTreeMap<&str, u64> = BTreeMap::new();
    let mut previous_delivery_tick = None;
    let mut matching_event = None;

    for event in events {
        if previous_delivery_tick.is_some_and(|previous| event.delivered_at_tick < previous) {
            return Err(EpisodeError::EventTimeRegression);
        }
        previous_delivery_tick = Some(event.delivered_at_tick);

        // The harness may hold future scripted events; they are not delivered
        // to the watch until the episode clock reaches their delivery tick.
        if event.delivered_at_tick > current_tick {
            break;
        }
        if !non_empty(&event.event_id) || event.sequence == 0 {
            return Err(EpisodeError::InvalidEventIdentity);
        }
        if event.observed_at_tick > event.delivered_at_tick {
            return Err(EpisodeError::InvalidEventTime);
        }
        if event.source_id != watch.source_id {
            rejected_scope_events += 1;
            continue;
        }
        if let Some(previous_sequence) = seen_event_ids.get(event.event_id.as_str()) {
            if *previous_sequence == event.sequence {
                duplicate_events += 1;
                continue;
            }
            return Ok(empty_result(
                watch,
                authority,
                EpisodeStatus::DuplicateEventIdentity,
                TerminalDisposition::Escalate,
                delivered_events,
                duplicate_events,
                rejected_scope_events,
                0,
            ));
        }
        seen_event_ids.insert(event.event_id.as_str(), event.sequence);
        if event.sequence < watch.first_sequence {
            duplicate_events += 1;
            continue;
        }
        if let Some(last) = last_sequence {
            if event.sequence <= last {
                duplicate_events += 1;
                continue;
            }
            if event.sequence != last.saturating_add(1) {
                return Ok(empty_result(
                    watch,
                    authority,
                    EpisodeStatus::DeliveryGap,
                    TerminalDisposition::Escalate,
                    delivered_events,
                    duplicate_events,
                    rejected_scope_events,
                    0,
                ));
            }
        } else if event.sequence != watch.first_sequence {
            return Ok(empty_result(
                watch,
                authority,
                EpisodeStatus::DeliveryGap,
                TerminalDisposition::Escalate,
                delivered_events,
                duplicate_events,
                rejected_scope_events,
                0,
            ));
        }
        last_sequence = Some(event.sequence);
        delivered_events += 1;

        if event.delivered_at_tick < watch.starts_at_tick
            || event.delivered_at_tick >= watch.expires_at_tick
        {
            continue;
        }

        if !authority.current_for(watch, event.delivered_at_tick) {
            return Ok(empty_result(
                watch,
                authority,
                EpisodeStatus::AuthorityRevokedOrExpired,
                TerminalDisposition::Escalate,
                delivered_events,
                duplicate_events,
                rejected_scope_events,
                0,
            ));
        }

        let in_scope = event.tenant_id == watch.tenant_id
            && event.target_id == watch.target_id
            && event.purpose_id == watch.purpose_id;
        if !in_scope {
            rejected_scope_events += 1;
            continue;
        }

        if event.event_type == watch.event_type && event.condition_met {
            matching_event = Some(event.clone());
            break;
        }
    }

    let event = match matching_event {
        Some(event) => event,
        None => {
            return Ok(empty_result(
                watch,
                authority,
                EpisodeStatus::Quiet,
                TerminalDisposition::Stop,
                delivered_events,
                duplicate_events,
                rejected_scope_events,
                0,
            ));
        }
    };

    let wakeups = 1usize;
    if wakeups > watch.max_wakeups as usize {
        return Ok(empty_result(
            watch,
            authority,
            EpisodeStatus::Expired,
            TerminalDisposition::Escalate,
            delivered_events,
            duplicate_events,
            rejected_scope_events,
            wakeups,
        ));
    }

    let wake = WakeContext {
        watch_id: watch.watch_id.clone(),
        event,
        authority_epoch: authority.epoch,
    };
    // Construct a capability-minimal view before policy selection. The
    // policy never sees other tenants' probes, write-capable entries, or
    // out-of-purpose/out-of-target catalog metadata.
    let mut eligible_probes: Vec<&ProbeCatalogEntry> = Vec::new();
    let mut seen_probe_ids = BTreeSet::new();
    for entry in catalog.iter().filter(|entry| {
        entry.read_only
            && non_empty(&entry.probe_id)
            && non_empty(&entry.operation_digest)
            && non_empty(&entry.collector_control_domain)
            && entry.tenant_id == watch.tenant_id
            && entry.target_id == watch.target_id
            && entry.purpose_id == watch.purpose_id
            && entry.source_id == watch.source_id
            && entry.maximum_duration_ticks > 0
            && entry.maximum_duration_ticks <= watch.max_probe_duration_ticks
    }) {
        if !seen_probe_ids.insert(entry.probe_id.as_str()) {
            return Ok(empty_result(
                watch,
                authority,
                EpisodeStatus::ProbeCatalogInvalid,
                TerminalDisposition::Abstain,
                delivered_events,
                duplicate_events,
                rejected_scope_events,
                wakeups,
            ));
        }
        eligible_probes.push(entry);
    }
    let eligible_probe_ids: Vec<String> = eligible_probes
        .iter()
        .map(|entry| entry.probe_id.clone())
        .collect();

    let selected_probe_id = match policy.select_probe(&wake, &eligible_probe_ids) {
        Some(probe_id) if eligible_probe_ids.iter().any(|eligible| eligible == &probe_id) => {
            probe_id
        }
        _ => {
            return Ok(empty_result(
                watch,
                authority,
                EpisodeStatus::NoProbeSelected,
                TerminalDisposition::AskPrincipal,
                delivered_events,
                duplicate_events,
                rejected_scope_events,
                wakeups,
            ));
        }
    };

    let probe = eligible_probes
        .iter()
        .copied()
        .find(|entry| entry.probe_id == selected_probe_id)
        .ok_or(EpisodeError::InvalidProbeCatalog)?;

    // Re-check current authority at the actual admission boundary.
    let eligible = probe.read_only
        && probe.tenant_id == watch.tenant_id
        && probe.target_id == watch.target_id
        && probe.purpose_id == watch.purpose_id
        && probe.source_id == watch.source_id
        && authority.current_for(watch, current_tick);

    if !eligible {
        let mut result = empty_result(
            watch,
            authority,
            EpisodeStatus::ProbeDenied,
            TerminalDisposition::Abstain,
            delivered_events,
            duplicate_events,
            rejected_scope_events,
            wakeups,
        );
        result.selected_probe_id = Some(selected_probe_id);
        return Ok(result);
    }

    let mut result = empty_result(
        watch,
        authority,
        EpisodeStatus::ProbeNotStarted,
        TerminalDisposition::Abstain,
        delivered_events,
        duplicate_events,
        rejected_scope_events,
        wakeups,
    );
    result.selected_probe_id = Some(selected_probe_id);
    result.probe_admitted = true;

    let observation = match collection {
        CollectionOutcome::NotStarted => return Ok(result),
        CollectionOutcome::Failed => {
            result.status = EpisodeStatus::ProbeFailed;
            return Ok(result);
        }
        CollectionOutcome::TimedOutStillRunning => {
            result.status = EpisodeStatus::ProbeOutcomeUnknown;
            result.terminal = TerminalDisposition::Escalate;
            return Ok(result);
        }
        CollectionOutcome::Completed(observation) => *observation,
    };

    let observation_valid = non_empty(&observation.observation_id)
        && non_empty(&observation.digest)
        && observation.probe_id == probe.probe_id
        && observation.operation_digest == probe.operation_digest
        && observation.tenant_id == watch.tenant_id
        && observation.target_id == watch.target_id
        && observation.purpose_id == watch.purpose_id
        && non_empty(&observation.control_domain)
        && observation.source_id == probe.source_id
        && observation.control_domain == probe.collector_control_domain
        && observation.observed_at_tick >= wake.event.delivered_at_tick
        && observation.observed_at_tick <= current_tick
        && current_tick.saturating_sub(observation.observed_at_tick)
            <= watch.max_observation_age_ticks;

    if !observation_valid {
        result.status = EpisodeStatus::ObservationRejected;
        result.terminal = TerminalDisposition::Abstain;
        return Ok(result);
    }
    result.observation = Some(observation.clone());

    let impact = match impact_outcome {
        ImpactOutcome::Unknown => {
            result.status = EpisodeStatus::ImpactUnknown;
            result.terminal = TerminalDisposition::Escalate;
            result.impact = ImpactDisposition::Unknown;
            return Ok(result);
        }
        ImpactOutcome::NotMeasured => {
            result.status = EpisodeStatus::ImpactUnknown;
            result.terminal = TerminalDisposition::Escalate;
            result.impact = ImpactDisposition::NotMeasured;
            return Ok(result);
        }
        ImpactOutcome::Observed(impact) => *impact,
    };

    if !impact.complete
        || !non_empty(&impact.impact_id)
        || !non_empty(&impact.digest)
        || impact.probe_id != probe.probe_id
        || impact.operation_digest != probe.operation_digest
        || impact.tenant_id != watch.tenant_id
        || impact.target_id != watch.target_id
        || impact.purpose_id != watch.purpose_id
        || !non_empty(&impact.observer_control_domain)
        || impact.window_start_tick < wake.event.delivered_at_tick
        || impact.window_start_tick > impact.window_end_tick
        || impact.window_end_tick > current_tick
        || impact.measured_at_tick < impact.window_start_tick
        || impact.measured_at_tick > impact.window_end_tick
    {
        result.status = EpisodeStatus::ImpactUnknown;
        result.terminal = TerminalDisposition::Escalate;
        result.impact = ImpactDisposition::Unknown;
        return Ok(result);
    }

    if impact.observer_control_domain == observation.control_domain {
        result.status = EpisodeStatus::ImpactIndependenceConflict;
        result.terminal = TerminalDisposition::Escalate;
        result.impact = ImpactDisposition::IndependenceConflict;
        return Ok(result);
    }

    result.impact = ImpactDisposition::Observed;
    result.impact_observation = Some(impact);
    let reduction = evaluator_truth.decision_loss_before_ppm as i64
        - evaluator_truth.decision_loss_after_ppm as i64;
    result.decision_value = Some(DecisionValue {
        loss_before_ppm: evaluator_truth.decision_loss_before_ppm,
        loss_after_ppm: evaluator_truth.decision_loss_after_ppm,
        realized_loss_reduction_ppm: reduction,
    });
    result.status = EpisodeStatus::Completed;
    result.terminal = if reduction > 0 {
        TerminalDisposition::Stop
    } else {
        TerminalDisposition::Escalate
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn watch() -> WatchSpec {
        WatchSpec {
            watch_id: "watch-episode-1".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            source_id: "events-a".into(),
            event_type: "queue_pressure".into(),
            first_sequence: 10,
            starts_at_tick: 1,
            expires_at_tick: 20,
            max_wakeups: 1,
            max_probe_duration_ticks: 3,
            max_observation_age_ticks: 2,
        }
    }

    fn authority() -> AuthoritySnapshot {
        AuthoritySnapshot {
            epoch: 7,
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            valid_from_tick: 0,
            expires_at_tick: 20,
            revoked_at_tick: None,
        }
    }

    fn catalog_entry(read_only: bool) -> ProbeCatalogEntry {
        ProbeCatalogEntry {
            probe_id: "probe-queue-depth".into(),
            operation_digest: "sha256:operation-a".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            source_id: "diagnostic-collector".into(),
            collector_control_domain: "collector-domain".into(),
            read_only,
            maximum_duration_ticks: 2,
        }
    }

    fn event(sequence: u64, event_type: &str, condition_met: bool) -> EventEnvelope {
        EventEnvelope {
            event_id: format!("event-{sequence}"),
            source_id: "events-a".into(),
            sequence,
            event_type: event_type.into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            condition_met,
            observed_at_tick: 3,
            delivered_at_tick: 4,
        }
    }

    fn observation() -> SourceObservation {
        SourceObservation {
            observation_id: "observation-1".into(),
            digest: "sha256:observation-a".into(),
            probe_id: "probe-queue-depth".into(),
            operation_digest: "sha256:operation-a".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            source_id: "diagnostic-collector".into(),
            control_domain: "collector-domain".into(),
            observed_at_tick: 4,
        }
    }

    fn impact() -> ImpactOutcome {
        ImpactOutcome::Observed(Box::new(ImpactObservation {
            impact_id: "impact-1".into(),
            digest: "sha256:impact-a".into(),
            probe_id: "probe-queue-depth".into(),
            operation_digest: "sha256:operation-a".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            observer_control_domain: "independent-observer".into(),
            window_start_tick: 4,
            window_end_tick: 5,
            measured_at_tick: 5,
            cpu_milliticks: 12,
            io_bytes: 128,
            complete: true,
        }))
    }

    fn truth() -> EvaluatorTruth {
        EvaluatorTruth {
            decision_loss_before_ppm: 500_000,
            decision_loss_after_ppm: 200_000,
        }
    }

    fn run(
        events: &[EventEnvelope],
        authority: &AuthoritySnapshot,
        entry: ProbeCatalogEntry,
        collection: CollectionOutcome,
        impact: ImpactOutcome,
    ) -> EpisodeResult {
        let mut policy = ScriptedProbePolicy {
            selected_probe_id: Some(entry.probe_id.clone()),
        };
        run_scripted_episode(
            &watch(),
            authority,
            &[entry],
            events,
            6,
            &mut policy,
            collection,
            impact,
            &truth(),
        )
        .unwrap()
    }

    #[test]
    fn valid_watch_probe_observation_impact_and_decision_path_completes() {
        let result = run(
            &[event(10, "queue_pressure", true)],
            &authority(),
            catalog_entry(true),
            CollectionOutcome::Completed(Box::new(observation())),
            impact(),
        );
        assert_eq!(result.status, EpisodeStatus::Completed);
        assert_eq!(result.terminal, TerminalDisposition::Stop);
        assert_eq!(result.wakeups, 1);
        assert_eq!(result.consequential_mutation_attempts, 0);
        assert_eq!(
            result.decision_value.unwrap().realized_loss_reduction_ppm,
            300_000
        );
    }

    #[test]
    fn observation_from_before_the_wakeup_is_rejected_as_stale() {
        let mut stale = observation();
        stale.observed_at_tick = 3;
        let result = run(
            &[event(10, "queue_pressure", true)],
            &authority(),
            catalog_entry(true),
            CollectionOutcome::Completed(Box::new(stale)),
            impact(),
        );
        assert_eq!(result.status, EpisodeStatus::ObservationRejected);
        assert_eq!(result.terminal, TerminalDisposition::Abstain);
        assert!(result.decision_value.is_none());
    }

    #[test]
    fn observation_bound_to_another_target_is_rejected() {
        let mut mismatched = observation();
        mismatched.target_id = "service-b".into();
        let result = run(
            &[event(10, "queue_pressure", true)],
            &authority(),
            catalog_entry(true),
            CollectionOutcome::Completed(Box::new(mismatched)),
            impact(),
        );
        assert_eq!(result.status, EpisodeStatus::ObservationRejected);
        assert!(result.decision_value.is_none());
    }

    #[test]
    fn impact_bound_to_another_target_remains_unknown() {
        let mismatched = ImpactOutcome::Observed(Box::new(ImpactObservation {
            impact_id: "impact-1".into(),
            digest: "sha256:impact-a".into(),
            probe_id: "probe-queue-depth".into(),
            operation_digest: "sha256:operation-a".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-b".into(),
            purpose_id: "incident-diagnosis".into(),
            observer_control_domain: "independent-observer".into(),
            window_start_tick: 4,
            window_end_tick: 5,
            measured_at_tick: 5,
            cpu_milliticks: 12,
            io_bytes: 128,
            complete: true,
        }));
        let result = run(
            &[event(10, "queue_pressure", true)],
            &authority(),
            catalog_entry(true),
            CollectionOutcome::Completed(Box::new(observation())),
            mismatched,
        );
        assert_eq!(result.status, EpisodeStatus::ImpactUnknown);
        assert_eq!(result.impact, ImpactDisposition::Unknown);
        assert!(result.decision_value.is_none());
    }

    #[test]
    fn future_event_is_not_delivered_early() {
        let mut future = event(10, "queue_pressure", true);
        future.delivered_at_tick = 7;
        let result = run(
            &[future],
            &authority(),
            catalog_entry(true),
            CollectionOutcome::NotStarted,
            ImpactOutcome::NotMeasured,
        );
        assert_eq!(result.status, EpisodeStatus::Quiet);
        assert_eq!(result.wakeups, 0);
    }

    #[test]
    fn empty_event_identity_is_rejected() {
        let mut invalid = event(10, "queue_pressure", true);
        invalid.event_id.clear();
        let mut policy = ScriptedProbePolicy {
            selected_probe_id: Some("probe-queue-depth".into()),
        };
        let result = run_scripted_episode(
            &watch(),
            &authority(),
            &[catalog_entry(true)],
            &[invalid],
            6,
            &mut policy,
            CollectionOutcome::NotStarted,
            ImpactOutcome::NotMeasured,
            &truth(),
        );
        assert_eq!(result, Err(EpisodeError::InvalidEventIdentity));
    }

    #[test]
    fn watch_before_start_does_not_consume_events() {
        let mut not_started = watch();
        not_started.starts_at_tick = 7;
        let result = run_scripted_episode(
            &not_started,
            &authority(),
            &[catalog_entry(true)],
            &[event(10, "queue_pressure", true)],
            6,
            &mut ScriptedProbePolicy {
                selected_probe_id: Some("probe-queue-depth".into()),
            },
            CollectionOutcome::NotStarted,
            ImpactOutcome::NotMeasured,
            &truth(),
        )
        .unwrap();
        assert_eq!(result.status, EpisodeStatus::NotStarted);
        assert_eq!(result.delivered_events, 0);
    }

    #[test]
    fn revoked_during_dormancy_prevents_probe_admission() {
        let mut revoked = authority();
        revoked.revoked_at_tick = Some(3);
        let result = run(
            &[event(10, "queue_pressure", true)],
            &revoked,
            catalog_entry(true),
            CollectionOutcome::Completed(Box::new(observation())),
            impact(),
        );
        assert_eq!(result.status, EpisodeStatus::AuthorityRevokedOrExpired);
        assert!(!result.probe_admitted);
        assert!(result.observation.is_none());
    }

    #[test]
    fn duplicate_event_does_not_receive_duplicate_credit() {
        let result = run(
            &[
                event(10, "irrelevant", false),
                event(10, "irrelevant", false),
                event(11, "queue_pressure", true),
            ],
            &authority(),
            catalog_entry(true),
            CollectionOutcome::Completed(Box::new(observation())),
            impact(),
        );
        assert_eq!(result.duplicate_events, 1);
        assert_eq!(result.wakeups, 1);
        assert_eq!(result.status, EpisodeStatus::Completed);
    }

    #[test]
    fn sequence_gap_escalates_without_probe() {
        let result = run(
            &[
                event(10, "irrelevant", false),
                event(12, "queue_pressure", true),
            ],
            &authority(),
            catalog_entry(true),
            CollectionOutcome::Completed(Box::new(observation())),
            impact(),
        );
        assert_eq!(result.status, EpisodeStatus::DeliveryGap);
        assert!(!result.probe_admitted);
    }

    #[test]
    fn write_capable_probe_is_denied() {
        let result = run(
            &[event(10, "queue_pressure", true)],
            &authority(),
            catalog_entry(false),
            CollectionOutcome::Completed(Box::new(observation())),
            impact(),
        );
        assert_eq!(result.status, EpisodeStatus::ProbeDenied);
        assert!(!result.probe_admitted);
        assert_eq!(result.consequential_mutation_attempts, 0);
    }

    #[test]
    fn unknown_probe_impact_is_not_zero_or_success() {
        let result = run(
            &[event(10, "queue_pressure", true)],
            &authority(),
            catalog_entry(true),
            CollectionOutcome::Completed(Box::new(observation())),
            ImpactOutcome::Unknown,
        );
        assert_eq!(result.status, EpisodeStatus::ImpactUnknown);
        assert_eq!(result.impact, ImpactDisposition::Unknown);
        assert!(result.decision_value.is_none());
        assert_eq!(result.terminal, TerminalDisposition::Escalate);
    }

    #[test]
    fn same_control_domain_cannot_prove_independent_impact() {
        let same_domain = ImpactOutcome::Observed(Box::new(ImpactObservation {
            impact_id: "impact-1".into(),
            digest: "sha256:impact-a".into(),
            probe_id: "probe-queue-depth".into(),
            operation_digest: "sha256:operation-a".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            observer_control_domain: "collector-domain".into(),
            window_start_tick: 4,
            window_end_tick: 5,
            measured_at_tick: 5,
            cpu_milliticks: 12,
            io_bytes: 128,
            complete: true,
        }));
        let result = run(
            &[event(10, "queue_pressure", true)],
            &authority(),
            catalog_entry(true),
            CollectionOutcome::Completed(Box::new(observation())),
            same_domain,
        );
        assert_eq!(result.status, EpisodeStatus::ImpactIndependenceConflict);
        assert_eq!(result.decision_value, None);
    }

    #[test]
    fn timed_out_collector_still_running_is_unknown_and_not_retried() {
        let result = run(
            &[event(10, "queue_pressure", true)],
            &authority(),
            catalog_entry(true),
            CollectionOutcome::TimedOutStillRunning,
            ImpactOutcome::NotMeasured,
        );
        assert_eq!(result.status, EpisodeStatus::ProbeOutcomeUnknown);
        assert_eq!(result.terminal, TerminalDisposition::Escalate);
        assert_eq!(result.wakeups, 1);
        assert!(result.observation.is_none());
    }

    #[test]
    fn no_matching_event_remains_quiet_without_probe() {
        let result = run(
            &[event(10, "irrelevant", false)],
            &authority(),
            catalog_entry(true),
            CollectionOutcome::NotStarted,
            ImpactOutcome::NotMeasured,
        );
        assert_eq!(result.status, EpisodeStatus::Quiet);
        assert_eq!(result.wakeups, 0);
        assert!(!result.probe_admitted);
    }

    #[test]
    fn wrong_tenant_event_is_rejected_without_wakeup() {
        let mut wrong_scope = event(10, "queue_pressure", true);
        wrong_scope.tenant_id = "tenant-b".into();
        let result = run(
            &[wrong_scope],
            &authority(),
            catalog_entry(true),
            CollectionOutcome::NotStarted,
            ImpactOutcome::NotMeasured,
        );
        assert_eq!(result.status, EpisodeStatus::Quiet);
        assert_eq!(result.rejected_scope_events, 1);
        assert_eq!(result.wakeups, 0);
    }
}
