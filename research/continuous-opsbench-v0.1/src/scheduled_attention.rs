//! Source-qualified event -> policy bridge for the paired OpsBench plant.
//!
//! The runner evaluates the event schedule from the same frozen timeline it
//! later joins to policy decisions. It preserves source identity through the
//! policy boundary and deliberately stops before authority admission, remote
//! collection, impact measurement, or consequential mutation. A selected
//! probe is a proposal, never an execution receipt.

use std::collections::{BTreeMap, BTreeSet};

use crate::event_schedule::{
    evaluate_event_schedule, AttentionStrategy, ConditionOutcome, EventScheduleResult, EventWatch,
    Persistence, RecordDisposition, ScheduleError, ScheduleStatus, SourceEvent, TimelineRecord,
};
use crate::stability::{
    run_stability_scenario, PlantObservation, StabilityConfig, StabilityError, StabilityRun,
};
use crate::watch_probe::{
    eligible_probe_ids_for_watch, AuthoritySnapshot, EpisodeError, ProbeCatalogEntry, WatchSpec,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceQualifiedEventIdentity {
    pub source_id: String,
    pub source_epoch: String,
    pub event_id: String,
    pub incident_id: String,
    pub sequence: u64,
    pub payload_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceQualifiedEvent {
    pub identity: SourceQualifiedEventIdentity,
    pub tenant_id: String,
    pub target_id: String,
    pub purpose_id: String,
    pub event_type: String,
    pub condition: ConditionOutcome,
    pub persistence: Persistence,
    pub event_tick: u64,
    pub valid_from_tick: u64,
    pub valid_until_tick: u64,
    pub delivered_tick: u64,
    pub processing_tick: u64,
    pub record_retain_until_tick: u64,
    pub response_deadline_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceQualifiedWakeBatch {
    pub watch_id: String,
    pub authority_epoch: u64,
    pub source_epoch: String,
    pub wake_tick: u64,
    pub sequence_start: u64,
    pub sequence_end: u64,
    pub events: Vec<SourceQualifiedEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceQualifiedDisposition {
    SelectReadOnlyProbe { probe_id: String },
    StopWithoutProbe,
    AskPrincipal,
    Abstain,
    Escalate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceQualifiedEventDecision {
    pub event: SourceQualifiedEventIdentity,
    pub disposition: SourceQualifiedDisposition,
}

/// This policy surface receives the original event identities and one
/// permitted plant observation. It never receives evaluator-owned plant truth,
/// the raw probe catalog, credentials, or mutation capabilities.
pub trait SourceQualifiedPolicy {
    /// Return exactly one decision for every event in the wake batch.
    fn decide(
        &mut self,
        wake: &SourceQualifiedWakeBatch,
        observation: &PlantObservation,
        eligible_probe_ids: &[String],
    ) -> Vec<SourceQualifiedEventDecision>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledWakeDecision {
    pub wake: SourceQualifiedWakeBatch,
    pub observation: PlantObservation,
    pub decisions: Vec<SourceQualifiedEventDecision>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeSelectionProposal {
    pub event: SourceQualifiedEventIdentity,
    pub probe_id: String,
    /// Frozen catalog operation identity and scope at proposal time.
    pub operation_digest: String,
    pub tenant_id: String,
    pub target_id: String,
    pub purpose_id: String,
    pub source_id: String,
    pub collector_control_domain: String,
    pub read_only: bool,
    pub maximum_duration_ticks: u64,
    pub proposed_at_tick: u64,
    pub observation_source_tick: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScheduledAttentionMetrics {
    /// One policy invocation per wake batch, not per event within a coalesced batch.
    pub policy_invocations: usize,
    pub wake_batches: usize,
    pub wake_events: usize,
    pub event_decisions: usize,
    pub probe_selection_proposals: usize,
    pub stop_without_probe: usize,
    pub ask_principal: usize,
    pub abstentions: usize,
    pub escalations: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledAttentionInput<'a> {
    pub plant: &'a StabilityConfig,
    pub watch: &'a WatchSpec,
    pub authority: &'a AuthoritySnapshot,
    pub catalog: &'a [ProbeCatalogEntry],
    pub event_watch: &'a EventWatch,
    pub event_authority: &'a crate::event_schedule::AuthorityWindow,
    pub strategy: AttentionStrategy,
    pub timeline: &'a [TimelineRecord],
    pub horizon_tick: u64,
    pub checkpoint: Option<crate::event_schedule::EventCheckpoint>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledAttentionResult {
    /// Exact schedule produced from input.timeline in this same invocation.
    pub schedule: EventScheduleResult,
    /// No-action plant replay; the policy has no action interface in this runner.
    pub plant: StabilityRun,
    pub metrics: ScheduledAttentionMetrics,
    pub decisions: Vec<ScheduledWakeDecision>,
    /// Policy proposals only. These are not admitted requests or collector runs.
    pub probe_proposals: Vec<ProbeSelectionProposal>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduledAttentionError {
    WatchContractMismatch,
    ScheduleBindingMismatch,
    ScheduleMutationDetected,
    ScheduleNotComplete(ScheduleStatus),
    MalformedWake,
    UnmatchedScheduleEvent,
    UnpairedScheduleWake,
    SourceEventOutsideWatch,
    AuthorityNotCurrent,
    PlantObservationMissing,
    MissingPolicyDisposition,
    DuplicatePolicyDisposition,
    OrphanPolicyDisposition,
    IneligibleProbeProposal,
    AmbiguousProbeCatalogEntry,
    Schedule(ScheduleError),
    ProbeEligibility(EpisodeError),
    Stability(StabilityError),
}

impl From<ScheduleError> for ScheduledAttentionError {
    fn from(value: ScheduleError) -> Self {
        Self::Schedule(value)
    }
}

impl From<EpisodeError> for ScheduledAttentionError {
    fn from(value: EpisodeError) -> Self {
        Self::ProbeEligibility(value)
    }
}

impl From<StabilityError> for ScheduledAttentionError {
    fn from(value: StabilityError) -> Self {
        Self::Stability(value)
    }
}

fn authority_current_at(authority: &AuthoritySnapshot, watch: &WatchSpec, tick: u64) -> bool {
    authority.tenant_id == watch.tenant_id
        && authority.target_id == watch.target_id
        && authority.purpose_id == watch.purpose_id
        && tick >= authority.valid_from_tick
        && tick < authority.expires_at_tick
        && authority
            .revoked_at_tick
            .is_none_or(|revoked| tick < revoked)
}

fn watch_contracts_match(input: &ScheduledAttentionInput<'_>) -> bool {
    let watch = input.watch;
    let event_watch = input.event_watch;
    let authority = input.authority;
    let event_authority = input.event_authority;
    event_watch.watch_id == watch.watch_id
        && event_watch.source_id == watch.source_id
        && event_watch.tenant_id == watch.tenant_id
        && event_watch.target_id == watch.target_id
        && event_watch.purpose_id == watch.purpose_id
        && event_watch.event_type == watch.event_type
        && event_watch.starts_at_tick == watch.starts_at_tick
        && event_watch.expires_at_tick == watch.expires_at_tick
        && event_watch.first_sequence == watch.first_sequence
        && usize::try_from(watch.max_wakeups).ok() == Some(event_watch.max_wakeups)
        && event_authority.epoch == authority.epoch
        && event_authority.tenant_id == authority.tenant_id
        && event_authority.target_id == authority.target_id
        && event_authority.purpose_id == authority.purpose_id
        && event_authority.valid_from_tick == authority.valid_from_tick
        && event_authority.expires_at_tick == authority.expires_at_tick
        && event_authority.revoked_at_tick == authority.revoked_at_tick
}

fn identity_for(event: &SourceEvent) -> SourceQualifiedEventIdentity {
    SourceQualifiedEventIdentity {
        source_id: event.source_id.clone(),
        source_epoch: event.source_epoch.clone(),
        event_id: event.event_id.clone(),
        incident_id: event.incident_id.clone(),
        sequence: event.sequence,
        payload_digest: event.payload_digest.clone(),
    }
}

fn source_event_for_record<'a>(
    input: &'a ScheduledAttentionInput<'_>,
    source_epoch: &str,
    event_id: &str,
    incident_id: &str,
    sequence: u64,
    delivered_tick: u64,
) -> Result<&'a SourceEvent, ScheduledAttentionError> {
    let matches: Vec<&SourceEvent> = input
        .timeline
        .iter()
        .filter_map(|record| match record {
            TimelineRecord::Event(event)
                if event.source_id == input.watch.source_id
                    && event.source_epoch == source_epoch
                    && event.event_id == event_id
                    && event.incident_id == incident_id
                    && event.sequence == sequence
                    && event.delivered_tick == delivered_tick =>
            {
                Some(event)
            }
            _ => None,
        })
        .collect();

    let Some(first) = matches.first() else {
        return Err(ScheduledAttentionError::UnmatchedScheduleEvent);
    };
    if matches.iter().any(|candidate| *candidate != *first) {
        return Err(ScheduledAttentionError::UnmatchedScheduleEvent);
    }
    Ok(*first)
}

fn schedule_record_count(
    schedule: &EventScheduleResult,
) -> BTreeMap<(String, String, u64, u64), usize> {
    let mut counts = BTreeMap::new();
    for record in schedule.records.iter().filter(|record| {
        matches!(
            record.disposition,
            RecordDisposition::Woke | RecordDisposition::Coalesced
        )
    }) {
        if let Some(processed_tick) = record.processed_tick {
            let key = (
                record.event_id.clone(),
                record.incident_id.clone(),
                record.sequence,
                processed_tick,
            );
            *counts.entry(key).or_insert(0) += 1;
        }
    }
    counts
}

fn scheduled_event(
    event: &SourceEvent,
    wake_tick: u64,
    watch: &WatchSpec,
    authority: &AuthoritySnapshot,
) -> Result<SourceQualifiedEvent, ScheduledAttentionError> {
    if event.source_id != watch.source_id
        || event.tenant_id != watch.tenant_id
        || event.target_id != watch.target_id
        || event.purpose_id != watch.purpose_id
        || event.event_type != watch.event_type
        || event.condition != ConditionOutcome::Match
        || event.delivered_tick > wake_tick
        || event.record_retain_until_tick < wake_tick
        || event.deadline_tick < wake_tick
        || event.event_tick < watch.starts_at_tick
        || wake_tick < watch.starts_at_tick
        || wake_tick >= watch.expires_at_tick
    {
        return Err(ScheduledAttentionError::SourceEventOutsideWatch);
    }
    if !authority_current_at(authority, watch, event.delivered_tick)
        || !authority_current_at(authority, watch, wake_tick)
    {
        return Err(ScheduledAttentionError::AuthorityNotCurrent);
    }
    Ok(SourceQualifiedEvent {
        identity: identity_for(event),
        tenant_id: event.tenant_id.clone(),
        target_id: event.target_id.clone(),
        purpose_id: event.purpose_id.clone(),
        event_type: event.event_type.clone(),
        condition: event.condition,
        persistence: event.persistence,
        event_tick: event.event_tick,
        valid_from_tick: event.valid_from_tick,
        valid_until_tick: event.valid_until_tick,
        delivered_tick: event.delivered_tick,
        processing_tick: wake_tick,
        record_retain_until_tick: event.record_retain_until_tick,
        response_deadline_tick: event.deadline_tick,
    })
}

fn verify_policy_decisions(
    wake: &SourceQualifiedWakeBatch,
    decisions: &[SourceQualifiedEventDecision],
    eligible_probe_ids: &[String],
) -> Result<(), ScheduledAttentionError> {
    let expected: BTreeSet<SourceQualifiedEventIdentity> = wake
        .events
        .iter()
        .map(|event| event.identity.clone())
        .collect();
    if expected.len() != wake.events.len() {
        return Err(ScheduledAttentionError::MalformedWake);
    }
    let mut seen = BTreeSet::new();
    for decision in decisions {
        if !expected.contains(&decision.event) {
            return Err(ScheduledAttentionError::OrphanPolicyDisposition);
        }
        if !seen.insert(decision.event.clone()) {
            return Err(ScheduledAttentionError::DuplicatePolicyDisposition);
        }
        if let SourceQualifiedDisposition::SelectReadOnlyProbe { probe_id } = &decision.disposition
        {
            if !eligible_probe_ids
                .iter()
                .any(|eligible| eligible == probe_id)
            {
                return Err(ScheduledAttentionError::IneligibleProbeProposal);
            }
        }
    }
    if seen != expected {
        return Err(ScheduledAttentionError::MissingPolicyDisposition);
    }
    Ok(())
}

/// Compose the existing source schedule with the existing no-action stability
/// plant. The schedule is generated here from the same timeline subsequently
/// used to reconstruct each wake's exact source event, preventing a caller
/// from swapping an unrelated event list between scheduling and policy input.
pub fn run_scheduled_attention(
    input: &ScheduledAttentionInput<'_>,
    policy: &mut dyn SourceQualifiedPolicy,
) -> Result<ScheduledAttentionResult, ScheduledAttentionError> {
    if !watch_contracts_match(input) {
        return Err(ScheduledAttentionError::WatchContractMismatch);
    }

    let schedule = evaluate_event_schedule(
        input.event_watch,
        input.event_authority,
        input.strategy,
        input.timeline,
        input.horizon_tick,
        input.checkpoint.clone(),
    )?;

    if schedule.watch_id != input.watch.watch_id
        || schedule.authority_epoch != input.authority.epoch
        || schedule.checkpoint.source_id != input.watch.source_id
    {
        return Err(ScheduledAttentionError::ScheduleBindingMismatch);
    }
    if schedule.consequential_mutation_attempts != 0
        || schedule.metrics.consequential_mutation_attempts != 0
    {
        return Err(ScheduledAttentionError::ScheduleMutationDetected);
    }
    if schedule.metrics.wakeups != schedule.wakes.len()
        || schedule.metrics.cognitive_invocations != schedule.wakes.len()
    {
        return Err(ScheduledAttentionError::MalformedWake);
    }
    if !matches!(
        schedule.status,
        ScheduleStatus::Completed | ScheduleStatus::Quiet
    ) {
        return Err(ScheduledAttentionError::ScheduleNotComplete(
            schedule.status,
        ));
    }

    let plant = run_stability_scenario(input.plant, &[])?;
    let expected_records = schedule_record_count(&schedule);
    let mut paired_records: BTreeMap<(String, String, u64, u64), usize> = BTreeMap::new();
    let mut metrics = ScheduledAttentionMetrics::default();
    let mut decisions_out = Vec::new();
    let mut probe_proposals = Vec::new();

    for wake in &schedule.wakes {
        if wake.event_ids.is_empty()
            || wake.event_ids.len() != wake.incident_ids.len()
            || wake.sequence_start == 0
            || wake.sequence_start > wake.sequence_end
            || wake.source_epoch.trim().is_empty()
            || wake.wake_tick >= input.watch.expires_at_tick
            || wake.wake_tick < input.watch.starts_at_tick
        {
            return Err(ScheduledAttentionError::MalformedWake);
        }

        let observation = plant
            .observations
            .iter()
            .find(|observation| observation.tick as u64 == wake.wake_tick)
            .cloned()
            .ok_or(ScheduledAttentionError::PlantObservationMissing)?;

        let mut events = Vec::with_capacity(wake.event_ids.len());
        let mut identities = BTreeSet::new();
        for (event_id, incident_id) in wake.event_ids.iter().zip(wake.incident_ids.iter()) {
            let record_matches: Vec<_> = schedule
                .records
                .iter()
                .filter(|record| {
                    record.event_id == *event_id
                        && record.incident_id == *incident_id
                        && record.processed_tick == Some(wake.wake_tick)
                        && record.sequence >= wake.sequence_start
                        && record.sequence <= wake.sequence_end
                        && record.deadline_met
                        && matches!(
                            record.disposition,
                            RecordDisposition::Woke | RecordDisposition::Coalesced
                        )
                })
                .collect();
            if record_matches.len() != 1 {
                return Err(ScheduledAttentionError::UnmatchedScheduleEvent);
            }
            let record = record_matches[0];
            let source_event = source_event_for_record(
                input,
                &wake.source_epoch,
                event_id,
                incident_id,
                record.sequence,
                record.delivered_tick,
            )?;
            let event =
                scheduled_event(source_event, wake.wake_tick, input.watch, input.authority)?;
            if !identities.insert(event.identity.clone()) {
                return Err(ScheduledAttentionError::MalformedWake);
            }
            *paired_records
                .entry((
                    event.identity.event_id.clone(),
                    event.identity.incident_id.clone(),
                    event.identity.sequence,
                    wake.wake_tick,
                ))
                .or_insert(0) += 1;
            events.push(event);
        }

        let wake_batch = SourceQualifiedWakeBatch {
            watch_id: input.watch.watch_id.clone(),
            authority_epoch: input.authority.epoch,
            source_epoch: wake.source_epoch.clone(),
            wake_tick: wake.wake_tick,
            sequence_start: wake.sequence_start,
            sequence_end: wake.sequence_end,
            events,
        };

        let eligible_probe_ids = eligible_probe_ids_for_watch(
            input.watch,
            input.authority,
            wake.wake_tick,
            input.catalog,
        )?;
        let decisions = policy.decide(&wake_batch, &observation, &eligible_probe_ids);
        verify_policy_decisions(&wake_batch, &decisions, &eligible_probe_ids)?;

        metrics.policy_invocations += 1;
        metrics.wake_batches += 1;
        metrics.wake_events += wake_batch.events.len();
        metrics.event_decisions += decisions.len();

        for decision in &decisions {
            match &decision.disposition {
                SourceQualifiedDisposition::SelectReadOnlyProbe { probe_id } => {
                    metrics.probe_selection_proposals += 1;
                    let matching_catalog_entries: Vec<_> = input
                        .catalog
                        .iter()
                        .filter(|entry| entry.probe_id == *probe_id)
                        .collect();
                    if matching_catalog_entries.len() != 1 {
                        return Err(ScheduledAttentionError::AmbiguousProbeCatalogEntry);
                    }
                    let entry = matching_catalog_entries[0];
                    if !entry.read_only
                        || entry.tenant_id != input.watch.tenant_id
                        || entry.target_id != input.watch.target_id
                        || entry.purpose_id != input.watch.purpose_id
                    {
                        return Err(ScheduledAttentionError::IneligibleProbeProposal);
                    }
                    probe_proposals.push(ProbeSelectionProposal {
                        event: decision.event.clone(),
                        probe_id: probe_id.clone(),
                        operation_digest: entry.operation_digest.clone(),
                        tenant_id: entry.tenant_id.clone(),
                        target_id: entry.target_id.clone(),
                        purpose_id: entry.purpose_id.clone(),
                        source_id: entry.source_id.clone(),
                        collector_control_domain: entry.collector_control_domain.clone(),
                        read_only: entry.read_only,
                        maximum_duration_ticks: entry.maximum_duration_ticks,
                        proposed_at_tick: wake_batch.wake_tick,
                        observation_source_tick: observation.source_tick as u64,
                    });
                }
                SourceQualifiedDisposition::StopWithoutProbe => {
                    metrics.stop_without_probe += 1;
                }
                SourceQualifiedDisposition::AskPrincipal => {
                    metrics.ask_principal += 1;
                }
                SourceQualifiedDisposition::Abstain => {
                    metrics.abstentions += 1;
                }
                SourceQualifiedDisposition::Escalate => {
                    metrics.escalations += 1;
                }
            }
        }

        decisions_out.push(ScheduledWakeDecision {
            wake: wake_batch,
            observation,
            decisions,
        });
    }

    if paired_records != expected_records {
        return Err(ScheduledAttentionError::UnpairedScheduleWake);
    }

    Ok(ScheduledAttentionResult {
        schedule,
        plant,
        metrics,
        decisions: decisions_out,
        probe_proposals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_schedule::{AuthorityWindow, EventCheckpoint, SourceEvent, TimelineRecord};

    struct ScriptedPolicy {
        disposition: SourceQualifiedDisposition,
        calls: usize,
        seen: Vec<SourceQualifiedEventIdentity>,
    }

    impl ScriptedPolicy {
        fn new(disposition: SourceQualifiedDisposition) -> Self {
            Self {
                disposition,
                calls: 0,
                seen: Vec::new(),
            }
        }
    }

    impl SourceQualifiedPolicy for ScriptedPolicy {
        fn decide(
            &mut self,
            wake: &SourceQualifiedWakeBatch,
            _observation: &PlantObservation,
            _eligible_probe_ids: &[String],
        ) -> Vec<SourceQualifiedEventDecision> {
            self.calls += 1;
            self.seen
                .extend(wake.events.iter().map(|event| event.identity.clone()));
            wake.events
                .iter()
                .map(|event| SourceQualifiedEventDecision {
                    event: event.identity.clone(),
                    disposition: self.disposition.clone(),
                })
                .collect()
        }
    }

    struct MissingDecisionPolicy;

    impl SourceQualifiedPolicy for MissingDecisionPolicy {
        fn decide(
            &mut self,
            _wake: &SourceQualifiedWakeBatch,
            _observation: &PlantObservation,
            _eligible_probe_ids: &[String],
        ) -> Vec<SourceQualifiedEventDecision> {
            Vec::new()
        }
    }

    fn plant() -> StabilityConfig {
        StabilityConfig {
            ticks: 32,
            api_capacity: 100,
            database_capacity: 100,
            critical_demand_per_tick: 20,
            bulk_demand_per_tick: 40,
            fault_start_tick: 8,
            fault_end_tick: 20,
            fault_api_capacity_reduction: 50,
            observation_delay_ticks: 1,
            actuation_delay_ticks: 1,
            maximum_observation_age_ticks: 4,
            maximum_failed_ppm: 50_000,
            maximum_latency_ms: 150,
            maximum_critical_failed_ppm: 50_000,
            maximum_bulk_failed_ppm: 100_000,
            maximum_critical_latency_ms: 100,
            maximum_bulk_latency_ms: 150,
            consecutive_healthy_ticks: 2,
            no_intervention_tail_ticks: 6,
            base_latency_ms: 10,
            queue_latency_ms_per_request: 1,
            queue_noise_by_tick: vec![0; 32],
            failed_ppm_noise_by_tick: vec![0; 32],
        }
    }

    fn watch() -> WatchSpec {
        WatchSpec {
            watch_id: "source-qualified-watch".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            source_id: "event-feed-a".into(),
            event_type: "service-degraded".into(),
            first_sequence: 10,
            starts_at_tick: 0,
            expires_at_tick: 32,
            max_wakeups: 8,
            max_probe_duration_ticks: 3,
            max_observation_age_ticks: 4,
        }
    }

    fn authority() -> AuthoritySnapshot {
        AuthoritySnapshot {
            epoch: 7,
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            valid_from_tick: 0,
            expires_at_tick: 32,
            revoked_at_tick: None,
        }
    }

    fn event_watch() -> EventWatch {
        EventWatch {
            watch_id: "source-qualified-watch".into(),
            source_id: "event-feed-a".into(),
            initial_source_epoch: "epoch-1".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            event_type: "service-degraded".into(),
            starts_at_tick: 0,
            expires_at_tick: 32,
            first_sequence: 10,
            max_event_checks: 64,
            max_wakeups: 8,
        }
    }

    fn event_authority() -> AuthorityWindow {
        AuthorityWindow {
            epoch: 7,
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            valid_from_tick: 0,
            expires_at_tick: 32,
            revoked_at_tick: None,
        }
    }

    fn event(
        sequence: u64,
        event_id: &str,
        incident_id: &str,
        delivered_tick: u64,
        coalesce_key: &str,
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
            coalesce_key: coalesce_key.into(),
            condition: ConditionOutcome::Match,
            persistence: Persistence::PersistentCondition,
            event_tick: delivered_tick,
            valid_from_tick: delivered_tick,
            valid_until_tick: 24,
            delivered_tick,
            record_retain_until_tick: 30,
            deadline_tick: 18,
        }
    }

    fn catalog() -> Vec<ProbeCatalogEntry> {
        vec![ProbeCatalogEntry {
            probe_id: "probe-queue".into(),
            operation_digest: "sha256:probe-op".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            source_id: "collector-a".into(),
            collector_control_domain: "collector-domain".into(),
            read_only: true,
            maximum_duration_ticks: 2,
        }]
    }

    fn input<'a>(
        plant: &'a StabilityConfig,
        watch: &'a WatchSpec,
        authority: &'a AuthoritySnapshot,
        catalog: &'a [ProbeCatalogEntry],
        event_watch: &'a EventWatch,
        event_authority: &'a AuthorityWindow,
        timeline: &'a [TimelineRecord],
        strategy: AttentionStrategy,
    ) -> ScheduledAttentionInput<'a> {
        ScheduledAttentionInput {
            plant,
            watch,
            authority,
            catalog,
            event_watch,
            event_authority,
            strategy,
            timeline,
            horizon_tick: 31,
            checkpoint: None::<EventCheckpoint>,
        }
    }

    #[test]
    fn preserves_exact_source_identity_through_policy_decision() {
        let plant = plant();
        let watch = watch();
        let authority = authority();
        let catalog = catalog();
        let event_watch = event_watch();
        let event_authority = event_authority();
        let timeline = vec![TimelineRecord::Event(event(
            10,
            "evt-10",
            "incident-a",
            5,
            "service-a",
        ))];
        let input = input(
            &plant,
            &watch,
            &authority,
            &catalog,
            &event_watch,
            &event_authority,
            &timeline,
            AttentionStrategy::EventTriggered,
        );
        let mut policy = ScriptedPolicy::new(SourceQualifiedDisposition::StopWithoutProbe);

        let result = run_scheduled_attention(&input, &mut policy).unwrap();

        assert_eq!(policy.calls, 1);
        assert_eq!(result.metrics.policy_invocations, 1);
        assert_eq!(result.metrics.wake_events, 1);
        assert_eq!(
            result.decisions[0].wake.events[0].identity.source_epoch,
            "epoch-1"
        );
        assert_eq!(
            result.decisions[0].wake.events[0].identity.event_id,
            "evt-10"
        );
        assert_eq!(
            result.decisions[0].wake.events[0].identity.incident_id,
            "incident-a"
        );
        assert_eq!(result.decisions[0].wake.events[0].identity.sequence, 10);
        assert_eq!(
            result.decisions[0].wake.events[0].identity.payload_digest,
            format!("sha256:{:064x}", 10)
        );
        assert_eq!(result.metrics.stop_without_probe, 1);
        assert!(result.probe_proposals.is_empty());
        assert!(result.plant.action_outcomes.is_empty());
    }

    #[test]
    fn coalesced_batch_is_one_invocation_with_one_disposition_per_event() {
        let plant = plant();
        let watch = watch();
        let authority = authority();
        let catalog = catalog();
        let event_watch = event_watch();
        let event_authority = event_authority();
        let timeline = vec![
            TimelineRecord::Event(event(10, "evt-10", "incident-a", 5, "service-a")),
            TimelineRecord::Event(event(11, "evt-11", "incident-b", 5, "service-a")),
        ];
        let input = input(
            &plant,
            &watch,
            &authority,
            &catalog,
            &event_watch,
            &event_authority,
            &timeline,
            AttentionStrategy::CoalescedEventTriggered {
                coalesce_window_ticks: 2,
            },
        );
        let mut policy = ScriptedPolicy::new(SourceQualifiedDisposition::Abstain);

        let result = run_scheduled_attention(&input, &mut policy).unwrap();

        assert_eq!(result.schedule.wakes.len(), 1);
        assert_eq!(result.metrics.policy_invocations, 1);
        assert_eq!(result.metrics.wake_events, 2);
        assert_eq!(result.metrics.event_decisions, 2);
        assert_eq!(result.metrics.abstentions, 2);
        assert_eq!(policy.calls, 1);
        assert_eq!(
            policy
                .seen
                .iter()
                .map(|event| event.incident_id.as_str())
                .collect::<Vec<_>>(),
            vec!["incident-a", "incident-b"]
        );
    }

    #[test]
    fn read_only_selection_is_recorded_as_proposal_not_admission() {
        let plant = plant();
        let watch = watch();
        let authority = authority();
        let catalog = catalog();
        let event_watch = event_watch();
        let event_authority = event_authority();
        let timeline = vec![TimelineRecord::Event(event(
            10,
            "evt-10",
            "incident-a",
            5,
            "service-a",
        ))];
        let input = input(
            &plant,
            &watch,
            &authority,
            &catalog,
            &event_watch,
            &event_authority,
            &timeline,
            AttentionStrategy::EventTriggered,
        );
        let mut policy = ScriptedPolicy::new(SourceQualifiedDisposition::SelectReadOnlyProbe {
            probe_id: "probe-queue".into(),
        });

        let result = run_scheduled_attention(&input, &mut policy).unwrap();

        assert_eq!(result.metrics.probe_selection_proposals, 1);
        assert_eq!(result.probe_proposals.len(), 1);
        assert_eq!(result.probe_proposals[0].probe_id, "probe-queue");
        assert_eq!(result.probe_proposals[0].event.event_id, "evt-10");
        assert!(result.plant.action_outcomes.is_empty());
    }

    #[test]
    fn invalid_probe_proposal_is_rejected_before_any_admission_claim() {
        let plant = plant();
        let watch = watch();
        let authority = authority();
        let catalog = catalog();
        let event_watch = event_watch();
        let event_authority = event_authority();
        let timeline = vec![TimelineRecord::Event(event(
            10,
            "evt-10",
            "incident-a",
            5,
            "service-a",
        ))];
        let input = input(
            &plant,
            &watch,
            &authority,
            &catalog,
            &event_watch,
            &event_authority,
            &timeline,
            AttentionStrategy::EventTriggered,
        );
        let mut policy = ScriptedPolicy::new(SourceQualifiedDisposition::SelectReadOnlyProbe {
            probe_id: "write-everything".into(),
        });

        assert_eq!(
            run_scheduled_attention(&input, &mut policy),
            Err(ScheduledAttentionError::IneligibleProbeProposal)
        );
    }

    #[test]
    fn incomplete_policy_output_cannot_be_scored_as_a_safe_noop() {
        let plant = plant();
        let watch = watch();
        let authority = authority();
        let catalog = catalog();
        let event_watch = event_watch();
        let event_authority = event_authority();
        let timeline = vec![TimelineRecord::Event(event(
            10,
            "evt-10",
            "incident-a",
            5,
            "service-a",
        ))];
        let input = input(
            &plant,
            &watch,
            &authority,
            &catalog,
            &event_watch,
            &event_authority,
            &timeline,
            AttentionStrategy::EventTriggered,
        );

        assert_eq!(
            run_scheduled_attention(&input, &mut MissingDecisionPolicy),
            Err(ScheduledAttentionError::MissingPolicyDisposition)
        );
    }

    #[test]
    fn sequence_gap_is_not_allowed_to_reach_policy_as_a_wake() {
        let plant = plant();
        let watch = watch();
        let authority = authority();
        let catalog = catalog();
        let event_watch = event_watch();
        let event_authority = event_authority();
        let timeline = vec![TimelineRecord::Event(event(
            11,
            "evt-11",
            "incident-b",
            5,
            "service-a",
        ))];
        let input = input(
            &plant,
            &watch,
            &authority,
            &catalog,
            &event_watch,
            &event_authority,
            &timeline,
            AttentionStrategy::EventTriggered,
        );
        let mut policy = ScriptedPolicy::new(SourceQualifiedDisposition::StopWithoutProbe);

        assert_eq!(
            run_scheduled_attention(&input, &mut policy),
            Err(ScheduledAttentionError::ScheduleNotComplete(
                ScheduleStatus::DeliveryGap
            ))
        );
        assert_eq!(policy.calls, 0);
    }

    #[test]
    fn quiet_control_runs_no_policy_and_preserves_no_action_plant() {
        let plant = plant();
        let watch = watch();
        let authority = authority();
        let catalog = catalog();
        let event_watch = event_watch();
        let event_authority = event_authority();
        let timeline: Vec<TimelineRecord> = Vec::new();
        let input = input(
            &plant,
            &watch,
            &authority,
            &catalog,
            &event_watch,
            &event_authority,
            &timeline,
            AttentionStrategy::EventTriggered,
        );
        let mut policy = ScriptedPolicy::new(SourceQualifiedDisposition::StopWithoutProbe);

        let result = run_scheduled_attention(&input, &mut policy).unwrap();
        let independent_baseline = run_stability_scenario(&plant, &[]).unwrap();

        assert_eq!(result.schedule.status, ScheduleStatus::Quiet);
        assert_eq!(result.metrics.policy_invocations, 0);
        assert_eq!(policy.calls, 0);
        assert_eq!(result.plant, independent_baseline);
        assert!(result.probe_proposals.is_empty());
    }

    #[test]
    fn mismatched_watch_authority_contracts_are_rejected() {
        let plant = plant();
        let mut watch = watch();
        watch.target_id = "other-service".into();
        let authority = authority();
        let catalog = catalog();
        let event_watch = event_watch();
        let event_authority = event_authority();
        let timeline: Vec<TimelineRecord> = Vec::new();
        let input = input(
            &plant,
            &watch,
            &authority,
            &catalog,
            &event_watch,
            &event_authority,
            &timeline,
            AttentionStrategy::EventTriggered,
        );

        assert_eq!(
            run_scheduled_attention(&input, &mut MissingDecisionPolicy),
            Err(ScheduledAttentionError::WatchContractMismatch)
        );
    }
}
