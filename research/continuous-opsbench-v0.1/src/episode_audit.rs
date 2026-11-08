//! Independent structural audit for watch -> wake -> probe -> evidence -> decision.
//!
//! This evaluator composes the existing event-schedule and watch/probe contracts.
//! It does not execute collectors, authenticate publishers, verify cryptographic
//! signatures, or establish production isolation. Missing evidence is explicit
//! and never interpreted as zero cost or a successful decision.

use std::collections::{BTreeMap, BTreeSet};

use crate::event_schedule::{EventScheduleResult, RecordDisposition, ScheduleStatus};
use crate::watch_probe::{
    AuthoritySnapshot, ImpactObservation, ProbeCatalogEntry, SourceObservation,
    TerminalDisposition, WatchSpec, MAX_RISK_PPM,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditStatus {
    Qualified,
    Indeterminate,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WakeDisposition {
    ProbeRequested { request_id: String },
    StopWithoutProbe,
    AskPrincipal,
    Abstain,
    Escalate,
}

/// One explicit decision for each event included in an evaluator wake batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WakeDecisionReceipt {
    pub decision_id: String,
    pub watch_id: String,
    pub source_epoch: String,
    pub event_id: String,
    pub incident_id: String,
    pub event_sequence: u64,
    pub wake_tick: u64,
    pub authority_epoch: u64,
    pub disposition: WakeDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeRequestReceipt {
    pub request_id: String,
    pub wake_decision_id: String,
    pub watch_id: String,
    pub source_epoch: String,
    pub event_id: String,
    pub incident_id: String,
    pub event_sequence: u64,
    pub wake_tick: u64,
    pub requested_at_tick: u64,
    pub admitted_at_tick: u64,
    pub deadline_tick: u64,
    pub authority_epoch: u64,
    pub probe_id: String,
    pub operation_digest: String,
    pub tenant_id: String,
    pub target_id: String,
    pub purpose_id: String,
    pub maximum_duration_ticks: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationReceipt {
    pub receipt_id: String,
    pub request_id: String,
    pub event_id: String,
    pub received_at_tick: u64,
    pub observation: SourceObservation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImpactReceipt {
    pub receipt_id: String,
    pub request_id: String,
    pub event_id: String,
    pub received_at_tick: u64,
    pub impact: ImpactObservation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReceiptState<T> {
    Present(T),
    Missing,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeEvidenceRecord {
    pub request_id: String,
    pub event_id: String,
    pub observation: ReceiptState<ObservationReceipt>,
    pub impact: ReceiptState<ImpactReceipt>,
    pub decision: Option<DecisionReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionReceipt {
    pub decision_id: String,
    pub request_id: String,
    pub observation_receipt_id: String,
    pub impact_receipt_id: String,
    pub decided_at_tick: u64,
    pub loss_before_ppm: u32,
    pub loss_after_ppm: u32,
    /// Agent/evaluator claim, independently recomputed by this auditor.
    pub claimed_loss_reduction_ppm: i64,
    pub terminal: TerminalDisposition,
}

pub struct EpisodeAuditInput<'a> {
    pub watch: &'a WatchSpec,
    pub authority: &'a AuthoritySnapshot,
    pub catalog: &'a [ProbeCatalogEntry],
    pub schedule: &'a EventScheduleResult,
    pub wake_decisions: &'a [WakeDecisionReceipt],
    pub requests: &'a [ProbeRequestReceipt],
    pub evidence: &'a [ProbeEvidenceRecord],
    pub audited_at_tick: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditFailure {
    InvalidWatchOrAuthority,
    ScheduleBindingMismatch,
    ScheduleMutationAttempt,
    WakeRecordMalformed,
    WakeRecordUnmatched,
    DuplicateWakeDecision,
    MissingWakeDecision,
    OrphanWakeDecision,
    DuplicateRequestId,
    DuplicateRequestForEvent,
    RequestNotAuthorizedByWake,
    RequestBindingMismatch,
    RequestOutsideWatchOrAuthority,
    ProbeNotCataloguedReadOnly,
    DuplicateReceiptId,
    DuplicateEvidenceRecord,
    EvidenceBindingMismatch,
    OrphanEvidenceRecord,
    DecisionWithoutCompleteEvidence,
    DuplicateDecisionId,
    DecisionBindingMismatch,
    DecisionValueMismatch,
    UnpairedScheduleWake,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndeterminateReason {
    ScheduleNotComplete,
    MissingObservation,
    UnknownObservation,
    StaleObservation,
    MissingImpact,
    UnknownImpact,
    IncompleteImpact,
    ImpactIndependenceUnestablished,
    MissingDecision,
    ReceiptAfterDeadline,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditedDecision {
    pub request_id: String,
    pub realized_loss_reduction_ppm: i64,
    pub terminal: TerminalDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpisodeAuditReport {
    pub status: AuditStatus,
    pub wake_events: usize,
    pub wake_decisions: usize,
    pub probe_requests: usize,
    pub observations_verified: usize,
    pub impacts_verified: usize,
    pub decisions_verified: usize,
    pub missing_observations: usize,
    pub unknown_observations: usize,
    pub missing_impacts: usize,
    pub unknown_impacts: usize,
    pub missing_decisions: usize,
    pub failures: Vec<AuditFailure>,
    pub indeterminate_reasons: Vec<IndeterminateReason>,
    pub decisions: Vec<AuditedDecision>,
}

impl EpisodeAuditReport {
    fn new() -> Self {
        Self {
            status: AuditStatus::Qualified,
            wake_events: 0,
            wake_decisions: 0,
            probe_requests: 0,
            observations_verified: 0,
            impacts_verified: 0,
            decisions_verified: 0,
            missing_observations: 0,
            unknown_observations: 0,
            missing_impacts: 0,
            unknown_impacts: 0,
            missing_decisions: 0,
            failures: Vec::new(),
            indeterminate_reasons: Vec::new(),
            decisions: Vec::new(),
        }
    }

    fn fail(&mut self, failure: AuditFailure) {
        self.status = AuditStatus::Failed;
        self.failures.push(failure);
    }

    fn indeterminate(&mut self, reason: IndeterminateReason) {
        if self.status != AuditStatus::Failed {
            self.status = AuditStatus::Indeterminate;
        }
        self.indeterminate_reasons.push(reason);
    }
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
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

fn watch_and_authority_valid(watch: &WatchSpec, authority: &AuthoritySnapshot) -> bool {
    non_empty(&watch.watch_id)
        && non_empty(&watch.tenant_id)
        && non_empty(&watch.target_id)
        && non_empty(&watch.purpose_id)
        && non_empty(&watch.source_id)
        && non_empty(&watch.event_type)
        && watch.first_sequence > 0
        && watch.starts_at_tick < watch.expires_at_tick
        && watch.max_wakeups > 0
        && watch.max_probe_duration_ticks > 0
        && watch.max_observation_age_ticks > 0
        && authority.epoch > 0
        && authority.tenant_id == watch.tenant_id
        && authority.target_id == watch.target_id
        && authority.purpose_id == watch.purpose_id
        && authority.valid_from_tick < authority.expires_at_tick
}

fn matching_schedule_event<'a>(
    schedule: &'a EventScheduleResult,
    event_id: &str,
    incident_id: &str,
    sequence: u64,
    wake_tick: u64,
) -> Vec<&'a crate::event_schedule::EventRecordResult> {
    schedule
        .records
        .iter()
        .filter(|record| {
            record.event_id == event_id
                && record.incident_id == incident_id
                && record.sequence == sequence
                && record.processed_tick == Some(wake_tick)
                && record.deadline_met
                && matches!(
                    record.disposition,
                    RecordDisposition::Woke | RecordDisposition::Coalesced
                )
        })
        .collect()
}

fn wake_contains_event(
    schedule: &EventScheduleResult,
    source_epoch: &str,
    event_id: &str,
    incident_id: &str,
    sequence: u64,
    wake_tick: u64,
) -> bool {
    schedule.wakes.iter().any(|wake| {
        wake.source_epoch == source_epoch
            && wake.wake_tick == wake_tick
            && sequence >= wake.sequence_start
            && sequence <= wake.sequence_end
            && wake
                .event_ids
                .iter()
                .zip(wake.incident_ids.iter())
                .any(|(id, incident)| id.as_str() == event_id && incident.as_str() == incident_id)
    })
}

fn validate_observation(
    request: &ProbeRequestReceipt,
    entry: &ProbeCatalogEntry,
    receipt: &ObservationReceipt,
    audited_at_tick: u64,
    maximum_age: u64,
) -> Result<(), IndeterminateReason> {
    let observation = &receipt.observation;
    if receipt.received_at_tick > request.deadline_tick {
        return Err(IndeterminateReason::ReceiptAfterDeadline);
    }
    if receipt.received_at_tick > audited_at_tick
        || observation.observed_at_tick > receipt.received_at_tick
    {
        return Err(IndeterminateReason::StaleObservation);
    }
    if audited_at_tick.saturating_sub(observation.observed_at_tick) > maximum_age
        || observation.observed_at_tick < request.admitted_at_tick
    {
        return Err(IndeterminateReason::StaleObservation);
    }
    if !non_empty(&receipt.receipt_id)
        || !non_empty(&observation.observation_id)
        || !non_empty(&observation.digest)
        || receipt.request_id != request.request_id
        || receipt.event_id != request.event_id
        || observation.probe_id != request.probe_id
        || observation.operation_digest != request.operation_digest
        || observation.tenant_id != request.tenant_id
        || observation.target_id != request.target_id
        || observation.purpose_id != request.purpose_id
        || observation.source_id != entry.source_id
        || observation.control_domain != entry.collector_control_domain
        || !non_empty(&observation.control_domain)
    {
        return Err(IndeterminateReason::StaleObservation);
    }
    Ok(())
}

fn observation_binding_matches(
    request: &ProbeRequestReceipt,
    entry: &ProbeCatalogEntry,
    receipt: &ObservationReceipt,
) -> bool {
    let observation = &receipt.observation;
    non_empty(&receipt.receipt_id)
        && non_empty(&observation.observation_id)
        && non_empty(&observation.digest)
        && receipt.request_id == request.request_id
        && receipt.event_id == request.event_id
        && observation.probe_id == request.probe_id
        && observation.operation_digest == request.operation_digest
        && observation.tenant_id == request.tenant_id
        && observation.target_id == request.target_id
        && observation.purpose_id == request.purpose_id
        && observation.source_id == entry.source_id
        && observation.control_domain == entry.collector_control_domain
        && non_empty(&observation.control_domain)
}

fn impact_binding_matches(
    request: &ProbeRequestReceipt,
    receipt: &ImpactReceipt,
    audited_at_tick: u64,
) -> bool {
    let impact = &receipt.impact;
    non_empty(&receipt.receipt_id)
        && non_empty(&impact.impact_id)
        && non_empty(&impact.digest)
        && receipt.request_id == request.request_id
        && receipt.event_id == request.event_id
        && impact.probe_id == request.probe_id
        && impact.operation_digest == request.operation_digest
        && impact.tenant_id == request.tenant_id
        && impact.target_id == request.target_id
        && impact.purpose_id == request.purpose_id
        && non_empty(&impact.observer_control_domain)
        && impact.complete
        && impact.window_start_tick >= request.admitted_at_tick
        && impact.window_start_tick <= impact.window_end_tick
        && impact.window_end_tick <= impact.measured_at_tick
        && impact.measured_at_tick <= receipt.received_at_tick
        && receipt.received_at_tick <= request.deadline_tick
        && receipt.received_at_tick <= audited_at_tick
}

fn request_matches_catalog(
    request: &ProbeRequestReceipt,
    entry: &ProbeCatalogEntry,
    watch: &WatchSpec,
) -> bool {
    entry.read_only
        && entry.probe_id == request.probe_id
        && entry.operation_digest == request.operation_digest
        && entry.tenant_id == request.tenant_id
        && entry.target_id == request.target_id
        && entry.purpose_id == request.purpose_id
        && entry.tenant_id == watch.tenant_id
        && entry.target_id == watch.target_id
        && entry.purpose_id == watch.purpose_id
        && entry.maximum_duration_ticks > 0
        && request.maximum_duration_ticks > 0
        && request.maximum_duration_ticks <= entry.maximum_duration_ticks
        && request.maximum_duration_ticks <= watch.max_probe_duration_ticks
        && non_empty(&entry.source_id)
        && non_empty(&entry.collector_control_domain)
}

struct DecisionAuditContext<'a> {
    request: &'a ProbeRequestReceipt,
    evidence: &'a ProbeEvidenceRecord,
    observation: &'a ObservationReceipt,
    impact: &'a ImpactReceipt,
    decision: &'a DecisionReceipt,
    audited_at_tick: u64,
}

fn audit_decision(
    context: DecisionAuditContext<'_>,
    report: &mut EpisodeAuditReport,
    decision_ids: &mut BTreeSet<String>,
) {
    let DecisionAuditContext {
        request,
        evidence,
        observation,
        impact,
        decision,
        audited_at_tick,
    } = context;
    if !decision_ids.insert(decision.decision_id.clone()) {
        report.fail(AuditFailure::DuplicateDecisionId);
        return;
    }
    let reduction = decision.loss_before_ppm as i64 - decision.loss_after_ppm as i64;
    let expected_terminal = if reduction > 0 {
        TerminalDisposition::Stop
    } else {
        TerminalDisposition::Escalate
    };
    if !non_empty(&decision.decision_id)
        || decision.request_id != request.request_id
        || decision.observation_receipt_id != observation.receipt_id
        || decision.impact_receipt_id != impact.receipt_id
        || decision.decided_at_tick < observation.received_at_tick
        || decision.decided_at_tick < impact.received_at_tick
        || decision.decided_at_tick > request.deadline_tick
        || decision.decided_at_tick > audited_at_tick
        || decision.loss_before_ppm > MAX_RISK_PPM
        || decision.loss_after_ppm > MAX_RISK_PPM
        || decision.claimed_loss_reduction_ppm != reduction
        || decision.terminal != expected_terminal
        || evidence.request_id != request.request_id
    {
        report.fail(AuditFailure::DecisionValueMismatch);
        report.fail(AuditFailure::DecisionBindingMismatch);
        return;
    }
    report.decisions_verified += 1;
    report.decisions.push(AuditedDecision {
        request_id: request.request_id.clone(),
        realized_loss_reduction_ppm: reduction,
        terminal: decision.terminal,
    });
}

/// Independently audit the join from scheduled event through probe and evidence.
///
/// This checks record identity, scope, timing, authority, read-only catalog
/// admission, source/observer separation, missingness and decision arithmetic.
/// It is an offline structural verifier, not a cryptographic authenticity proof.
pub fn audit_episode(input: &EpisodeAuditInput<'_>) -> EpisodeAuditReport {
    let mut report = EpisodeAuditReport::new();
    let watch = input.watch;
    let authority = input.authority;
    let schedule = input.schedule;

    if !watch_and_authority_valid(watch, authority) {
        report.fail(AuditFailure::InvalidWatchOrAuthority);
    }
    if schedule.watch_id != watch.watch_id
        || schedule.authority_epoch != authority.epoch
        || schedule.checkpoint.source_id != watch.source_id
    {
        report.fail(AuditFailure::ScheduleBindingMismatch);
    }
    if schedule.consequential_mutation_attempts != 0
        || schedule.metrics.consequential_mutation_attempts != 0
    {
        report.fail(AuditFailure::ScheduleMutationAttempt);
    }
    if schedule.metrics.wakeups != schedule.wakes.len() {
        report.fail(AuditFailure::WakeRecordMalformed);
    }

    if schedule.status != ScheduleStatus::Completed
        && !(schedule.status == ScheduleStatus::Quiet
            && schedule.wakes.is_empty()
            && input.requests.is_empty())
    {
        report.indeterminate(IndeterminateReason::ScheduleNotComplete);
    }

    let mut catalog_by_id: BTreeMap<&str, &ProbeCatalogEntry> = BTreeMap::new();
    for entry in input.catalog {
        if catalog_by_id
            .insert(entry.probe_id.as_str(), entry)
            .is_some()
        {
            report.fail(AuditFailure::ProbeNotCataloguedReadOnly);
        }
    }

    let mut wake_decision_ids = BTreeSet::new();
    let mut wake_decision_keys = BTreeSet::new();
    let mut request_ids = BTreeSet::new();
    let mut request_event_keys = BTreeSet::new();
    let mut referenced_request_ids = BTreeSet::new();
    let mut decision_ids = BTreeSet::new();
    let mut observation_receipt_ids = BTreeSet::new();
    let mut impact_receipt_ids = BTreeSet::new();
    let mut evidence_request_ids = BTreeSet::new();

    // Every event inside a wake group must have one explicit policy disposition.
    for wake in &schedule.wakes {
        if wake.event_ids.is_empty()
            || wake.event_ids.len() != wake.incident_ids.len()
            || wake.sequence_start == 0
            || wake.sequence_start > wake.sequence_end
            || !non_empty(&wake.source_epoch)
        {
            report.fail(AuditFailure::WakeRecordMalformed);
            continue;
        }
        for (event_id, incident_id) in wake.event_ids.iter().zip(wake.incident_ids.iter()) {
            let candidates: Vec<_> = schedule
                .records
                .iter()
                .filter(|record| {
                    record.event_id == *event_id
                        && record.incident_id == *incident_id
                        && record.processed_tick == Some(wake.wake_tick)
                        && record.deadline_met
                        && record.sequence >= wake.sequence_start
                        && record.sequence <= wake.sequence_end
                        && matches!(
                            record.disposition,
                            RecordDisposition::Woke | RecordDisposition::Coalesced
                        )
                })
                .collect();
            if candidates.len() != 1 {
                report.fail(AuditFailure::WakeRecordUnmatched);
                continue;
            }
            let event = candidates[0];
            report.wake_events += 1;
            let key = (
                wake.source_epoch.clone(),
                event.event_id.clone(),
                event.sequence,
                event.incident_id.clone(),
                wake.wake_tick,
            );
            let matching: Vec<_> = input
                .wake_decisions
                .iter()
                .filter(|decision| {
                    decision.watch_id == watch.watch_id
                        && decision.source_epoch == wake.source_epoch
                        && decision.event_id == event.event_id
                        && decision.incident_id == event.incident_id
                        && decision.event_sequence == event.sequence
                        && decision.wake_tick == wake.wake_tick
                        && decision.authority_epoch == authority.epoch
                })
                .collect();
            if matching.is_empty() {
                report.fail(AuditFailure::MissingWakeDecision);
                continue;
            }
            if matching.len() != 1 {
                report.fail(AuditFailure::DuplicateWakeDecision);
                continue;
            }
            let decision = matching[0];
            if !non_empty(&decision.decision_id)
                || !wake_decision_ids.insert(decision.decision_id.clone())
                || !wake_decision_keys.insert(key)
            {
                report.fail(AuditFailure::DuplicateWakeDecision);
                continue;
            }
            report.wake_decisions += 1;
            match &decision.disposition {
                WakeDisposition::ProbeRequested { request_id } => {
                    referenced_request_ids.insert(request_id.clone());
                    let linked: Vec<_> = input
                        .requests
                        .iter()
                        .filter(|request| request.request_id == *request_id)
                        .collect();
                    if linked.len() != 1 {
                        report.fail(AuditFailure::RequestNotAuthorizedByWake);
                    } else {
                        let request = linked[0];
                        if request.wake_decision_id != decision.decision_id
                            || request.event_id != event.event_id
                            || request.incident_id != event.incident_id
                            || request.event_sequence != event.sequence
                            || request.source_epoch != wake.source_epoch
                            || request.wake_tick != wake.wake_tick
                        {
                            report.fail(AuditFailure::RequestNotAuthorizedByWake);
                        }
                    }
                }
                WakeDisposition::StopWithoutProbe
                | WakeDisposition::AskPrincipal
                | WakeDisposition::Abstain
                | WakeDisposition::Escalate => {
                    if input.requests.iter().any(|request| {
                        request.event_id == event.event_id
                            && request.source_epoch == wake.source_epoch
                            && request.event_sequence == event.sequence
                    }) {
                        report.fail(AuditFailure::RequestNotAuthorizedByWake);
                    }
                }
            }
        }
    }

    // Every scheduled Woke/Coalesced record must also be represented by exactly
    // one wake batch. This catches a schedule-side wake omitted downstream.
    for record in schedule.records.iter().filter(|record| {
        matches!(
            record.disposition,
            RecordDisposition::Woke | RecordDisposition::Coalesced
        )
    }) {
        let matching = schedule
            .wakes
            .iter()
            .filter(|wake| {
                wake.wake_tick == record.processed_tick.unwrap_or(u64::MAX)
                    && record.sequence >= wake.sequence_start
                    && record.sequence <= wake.sequence_end
                    && wake.event_ids.iter().zip(wake.incident_ids.iter()).any(
                        |(event_id, incident_id)| {
                            event_id == &record.event_id && incident_id == &record.incident_id
                        },
                    )
            })
            .count();
        if matching != 1 {
            report.fail(AuditFailure::UnpairedScheduleWake);
        }
    }

    // Supplied wake decisions must not be orphaned or silently duplicated.
    for decision in input.wake_decisions {
        let exists =
            schedule.wakes.iter().any(|wake| {
                wake.source_epoch == decision.source_epoch
                    && wake.wake_tick == decision.wake_tick
                    && wake.event_ids.iter().zip(wake.incident_ids.iter()).any(
                        |(event, incident)| {
                            event == &decision.event_id && incident == &decision.incident_id
                        },
                    )
            });
        if !exists || !wake_decision_ids.contains(&decision.decision_id) {
            report.fail(AuditFailure::OrphanWakeDecision);
        }
    }

    // Validate each probe request against the exact wake, authority and catalog.
    for request in input.requests {
        report.probe_requests += 1;
        if !request_ids.insert(request.request_id.clone()) {
            report.fail(AuditFailure::DuplicateRequestId);
        }
        if !referenced_request_ids.contains(&request.request_id) {
            report.fail(AuditFailure::RequestNotAuthorizedByWake);
        }
        if !request_event_keys.insert((
            request.source_epoch.clone(),
            request.event_id.clone(),
            request.event_sequence,
        )) {
            report.fail(AuditFailure::DuplicateRequestForEvent);
        }

        let matching_records: Vec<_> = matching_schedule_event(
            schedule,
            &request.event_id,
            &request.incident_id,
            request.event_sequence,
            request.wake_tick,
        );
        if matching_records.len() != 1
            || !wake_contains_event(
                schedule,
                &request.source_epoch,
                &request.event_id,
                &request.incident_id,
                request.event_sequence,
                request.wake_tick,
            )
        {
            report.fail(AuditFailure::RequestNotAuthorizedByWake);
        }

        if request.watch_id != watch.watch_id
            || request.authority_epoch != authority.epoch
            || request.tenant_id != watch.tenant_id
            || request.target_id != watch.target_id
            || request.purpose_id != watch.purpose_id
            || !non_empty(&request.request_id)
            || !non_empty(&request.probe_id)
            || !non_empty(&request.operation_digest)
            || request.wake_tick < watch.starts_at_tick
            || request.wake_tick >= watch.expires_at_tick
            || request.requested_at_tick < request.wake_tick
            || request.admitted_at_tick < request.requested_at_tick
            || request.admitted_at_tick > request.deadline_tick
            || request.admitted_at_tick > input.audited_at_tick
            || request.deadline_tick > watch.expires_at_tick
            || !authority_current_at(authority, watch, request.wake_tick)
            || !authority_current_at(authority, watch, request.admitted_at_tick)
        {
            report.fail(AuditFailure::RequestOutsideWatchOrAuthority);
        }

        let entry = match catalog_by_id.get(request.probe_id.as_str()) {
            Some(entry) => *entry,
            None => {
                report.fail(AuditFailure::ProbeNotCataloguedReadOnly);
                continue;
            }
        };
        if !request_matches_catalog(request, entry, watch) {
            report.fail(AuditFailure::ProbeNotCataloguedReadOnly);
        }
    }

    // The evidence table is one-to-one with requests. Missing/unknown evidence
    // remains visible as indeterminate; it is never converted to a numeric zero.
    for evidence in input.evidence {
        if !evidence_request_ids.insert(evidence.request_id.clone()) {
            report.fail(AuditFailure::DuplicateEvidenceRecord);
        }
        if !request_ids.contains(&evidence.request_id) {
            report.fail(AuditFailure::OrphanEvidenceRecord);
        }
    }

    for request in input.requests {
        let rows: Vec<_> = input
            .evidence
            .iter()
            .filter(|row| row.request_id == request.request_id)
            .collect();
        if rows.is_empty() {
            report.missing_observations += 1;
            report.missing_impacts += 1;
            report.indeterminate(IndeterminateReason::MissingObservation);
            report.indeterminate(IndeterminateReason::MissingImpact);
            continue;
        }
        if rows.len() != 1 {
            report.fail(AuditFailure::DuplicateEvidenceRecord);
            continue;
        }
        let row = rows[0];
        if row.event_id != request.event_id {
            report.fail(AuditFailure::EvidenceBindingMismatch);
            continue;
        }

        let entry = match catalog_by_id.get(request.probe_id.as_str()) {
            Some(entry) => *entry,
            None => continue,
        };

        let observation = match &row.observation {
            ReceiptState::Present(receipt) => {
                if !observation_receipt_ids.insert(receipt.receipt_id.clone()) {
                    report.fail(AuditFailure::DuplicateReceiptId);
                }
                if receipt.request_id != request.request_id
                    || receipt.event_id != request.event_id
                    || !observation_binding_matches(request, entry, receipt)
                {
                    report.fail(AuditFailure::EvidenceBindingMismatch);
                    None
                } else {
                    match validate_observation(
                        request,
                        entry,
                        receipt,
                        input.audited_at_tick,
                        watch.max_observation_age_ticks,
                    ) {
                        Ok(()) => {
                            report.observations_verified += 1;
                            Some(receipt)
                        }
                        Err(reason) => {
                            report.indeterminate(reason);
                            None
                        }
                    }
                }
            }
            ReceiptState::Missing => {
                report.missing_observations += 1;
                report.indeterminate(IndeterminateReason::MissingObservation);
                None
            }
            ReceiptState::Unknown => {
                report.unknown_observations += 1;
                report.indeterminate(IndeterminateReason::UnknownObservation);
                None
            }
        };

        let impact = match &row.impact {
            ReceiptState::Present(receipt) => {
                if receipt.received_at_tick > request.deadline_tick {
                    report.indeterminate(IndeterminateReason::ReceiptAfterDeadline);
                    None
                } else {
                    if !impact_receipt_ids.insert(receipt.receipt_id.clone()) {
                        report.fail(AuditFailure::DuplicateReceiptId);
                    }
                    if receipt.request_id != request.request_id
                        || receipt.event_id != request.event_id
                        || !impact_binding_matches(request, receipt, input.audited_at_tick)
                    {
                        if !receipt.impact.complete {
                            report.indeterminate(IndeterminateReason::IncompleteImpact);
                        } else {
                            report.fail(AuditFailure::EvidenceBindingMismatch);
                        }
                        None
                    } else if let Some(observation) = observation {
                        if receipt.impact.observer_control_domain
                            == observation.observation.control_domain
                            || receipt.impact.observer_control_domain
                                == entry.collector_control_domain
                        {
                            report.indeterminate(
                                IndeterminateReason::ImpactIndependenceUnestablished,
                            );
                            None
                        } else {
                            report.impacts_verified += 1;
                            Some(receipt)
                        }
                    } else {
                        // An impact receipt cannot close the chain if its paired
                        // source observation is stale, absent or unknown.
                        None
                    }
                }
            }
            ReceiptState::Missing => {
                report.missing_impacts += 1;
                report.indeterminate(IndeterminateReason::MissingImpact);
                None
            }
            ReceiptState::Unknown => {
                report.unknown_impacts += 1;
                report.indeterminate(IndeterminateReason::UnknownImpact);
                None
            }
        };

        match (&observation, &impact, &row.decision) {
            (Some(observation), Some(impact), Some(decision)) => {
                audit_decision(
                    DecisionAuditContext {
                        request,
                        evidence: row,
                        observation,
                        impact,
                        decision,
                        audited_at_tick: input.audited_at_tick,
                    },
                    &mut report,
                    &mut decision_ids,
                );
            }
            (Some(_), Some(_), None) => {
                report.missing_decisions += 1;
                report.indeterminate(IndeterminateReason::MissingDecision);
            }
            (None, _, Some(_)) | (_, None, Some(_)) => {
                report.fail(AuditFailure::DecisionWithoutCompleteEvidence);
            }
            _ => {}
        }
    }

    // A quiet, valid schedule may have no wake decisions or probes. Otherwise,
    // every accepted wake and request must have a complete explicit lineage.
    if schedule.status == ScheduleStatus::Quiet
        && (!schedule.wakes.is_empty() || !input.wake_decisions.is_empty())
    {
        report.fail(AuditFailure::ScheduleBindingMismatch);
    }
    if input
        .requests
        .iter()
        .any(|request| !referenced_request_ids.contains(&request.request_id))
    {
        report.fail(AuditFailure::RequestNotAuthorizedByWake);
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_schedule::{
        AttentionStrategy, EventCheckpoint, EventRecordResult, EventScheduleMetrics,
        ScheduleStatus, WakeRecord,
    };
    use std::collections::BTreeMap;

    fn watch() -> WatchSpec {
        WatchSpec {
            watch_id: "watch-a".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            source_id: "event-feed-a".into(),
            event_type: "service-degraded".into(),
            first_sequence: 10,
            starts_at_tick: 1,
            expires_at_tick: 40,
            max_wakeups: 4,
            max_probe_duration_ticks: 5,
            max_observation_age_ticks: 10,
        }
    }

    fn authority() -> AuthoritySnapshot {
        AuthoritySnapshot {
            epoch: 7,
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            valid_from_tick: 0,
            expires_at_tick: 40,
            revoked_at_tick: None,
        }
    }

    fn catalog() -> Vec<ProbeCatalogEntry> {
        vec![ProbeCatalogEntry {
            probe_id: "probe-queue".into(),
            operation_digest: "sha256:operation-a".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            source_id: "collector-a".into(),
            collector_control_domain: "collector-domain".into(),
            read_only: true,
            maximum_duration_ticks: 3,
        }]
    }

    fn schedule() -> EventScheduleResult {
        EventScheduleResult {
            status: ScheduleStatus::Completed,
            watch_id: "watch-a".into(),
            strategy: AttentionStrategy::EventTriggered,
            authority_epoch: 7,
            metrics: EventScheduleMetrics {
                wakeups: 1,
                consequential_mutation_attempts: 0,
                ..EventScheduleMetrics::default()
            },
            records: vec![EventRecordResult {
                event_id: "event-10".into(),
                incident_id: "incident-a".into(),
                sequence: 10,
                delivered_tick: 5,
                processed_tick: Some(6),
                disposition: RecordDisposition::Woke,
                detection_latency_ticks: Some(1),
                deadline_met: true,
            }],
            wakes: vec![WakeRecord {
                wake_tick: 6,
                source_epoch: "epoch-1".into(),
                event_ids: vec!["event-10".into()],
                incident_ids: vec!["incident-a".into()],
                sequence_start: 10,
                sequence_end: 10,
                coalesce_key: "service-a".into(),
            }],
            checkpoint: EventCheckpoint {
                source_id: "event-feed-a".into(),
                source_epoch: "epoch-1".into(),
                next_sequence: 11,
                last_stream_tick: 5,
                seen_event_ids: BTreeMap::new(),
            },
            consequential_mutation_attempts: 0,
        }
    }

    fn wake_decision() -> Vec<WakeDecisionReceipt> {
        vec![WakeDecisionReceipt {
            decision_id: "wake-decision-1".into(),
            watch_id: "watch-a".into(),
            source_epoch: "epoch-1".into(),
            event_id: "event-10".into(),
            incident_id: "incident-a".into(),
            event_sequence: 10,
            wake_tick: 6,
            authority_epoch: 7,
            disposition: WakeDisposition::ProbeRequested {
                request_id: "request-1".into(),
            },
        }]
    }

    fn request() -> Vec<ProbeRequestReceipt> {
        vec![ProbeRequestReceipt {
            request_id: "request-1".into(),
            wake_decision_id: "wake-decision-1".into(),
            watch_id: "watch-a".into(),
            source_epoch: "epoch-1".into(),
            event_id: "event-10".into(),
            incident_id: "incident-a".into(),
            event_sequence: 10,
            wake_tick: 6,
            requested_at_tick: 6,
            admitted_at_tick: 7,
            deadline_tick: 15,
            authority_epoch: 7,
            probe_id: "probe-queue".into(),
            operation_digest: "sha256:operation-a".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            maximum_duration_ticks: 3,
        }]
    }

    fn observation_receipt() -> ObservationReceipt {
        ObservationReceipt {
            receipt_id: "obs-receipt-1".into(),
            request_id: "request-1".into(),
            event_id: "event-10".into(),
            received_at_tick: 9,
            observation: SourceObservation {
                observation_id: "observation-1".into(),
                digest: "sha256:observation-a".into(),
                probe_id: "probe-queue".into(),
                operation_digest: "sha256:operation-a".into(),
                tenant_id: "tenant-a".into(),
                target_id: "service-a".into(),
                purpose_id: "incident-diagnosis".into(),
                source_id: "collector-a".into(),
                control_domain: "collector-domain".into(),
                observed_at_tick: 8,
            },
        }
    }

    fn impact_receipt() -> ImpactReceipt {
        ImpactReceipt {
            receipt_id: "impact-receipt-1".into(),
            request_id: "request-1".into(),
            event_id: "event-10".into(),
            received_at_tick: 10,
            impact: ImpactObservation {
                impact_id: "impact-1".into(),
                digest: "sha256:impact-a".into(),
                probe_id: "probe-queue".into(),
                operation_digest: "sha256:operation-a".into(),
                tenant_id: "tenant-a".into(),
                target_id: "service-a".into(),
                purpose_id: "incident-diagnosis".into(),
                observer_control_domain: "observer-domain".into(),
                window_start_tick: 7,
                window_end_tick: 9,
                measured_at_tick: 9,
                cpu_milliticks: 0,
                io_bytes: 0,
                complete: true,
            },
        }
    }

    fn decision() -> DecisionReceipt {
        DecisionReceipt {
            decision_id: "decision-1".into(),
            request_id: "request-1".into(),
            observation_receipt_id: "obs-receipt-1".into(),
            impact_receipt_id: "impact-receipt-1".into(),
            decided_at_tick: 11,
            loss_before_ppm: 500_000,
            loss_after_ppm: 200_000,
            claimed_loss_reduction_ppm: 300_000,
            terminal: TerminalDisposition::Stop,
        }
    }

    fn evidence() -> Vec<ProbeEvidenceRecord> {
        vec![ProbeEvidenceRecord {
            request_id: "request-1".into(),
            event_id: "event-10".into(),
            observation: ReceiptState::Present(observation_receipt()),
            impact: ReceiptState::Present(impact_receipt()),
            decision: Some(decision()),
        }]
    }

    #[allow(clippy::too_many_arguments)]
    fn run(
        watch: &WatchSpec,
        authority: &AuthoritySnapshot,
        catalog: &[ProbeCatalogEntry],
        schedule: &EventScheduleResult,
        wake_decisions: &[WakeDecisionReceipt],
        requests: &[ProbeRequestReceipt],
        evidence: &[ProbeEvidenceRecord],
        audited_at_tick: u64,
    ) -> EpisodeAuditReport {
        audit_episode(&EpisodeAuditInput {
            watch,
            authority,
            catalog,
            schedule,
            wake_decisions,
            requests,
            evidence,
            audited_at_tick,
        })
    }

    #[test]
    fn exact_event_probe_observation_impact_and_decision_chain_qualifies() {
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &wake_decision(),
            &request(),
            &evidence(),
            12,
        );
        assert_eq!(report.status, AuditStatus::Qualified);
        assert_eq!(report.wake_events, 1);
        assert_eq!(report.probe_requests, 1);
        assert_eq!(report.observations_verified, 1);
        assert_eq!(report.impacts_verified, 1);
        assert_eq!(report.decisions_verified, 1);
        assert_eq!(report.decisions[0].realized_loss_reduction_ppm, 300_000);
    }

    #[test]
    fn missing_impact_is_indeterminate_and_never_zero_interference() {
        let mut rows = evidence();
        rows[0].impact = ReceiptState::Missing;
        rows[0].decision = None;
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &wake_decision(),
            &request(),
            &rows,
            12,
        );
        assert_eq!(report.status, AuditStatus::Indeterminate);
        assert_eq!(report.missing_impacts, 1);
        assert_eq!(report.impacts_verified, 0);
        assert_eq!(report.decisions_verified, 0);
        assert!(report.decisions.is_empty());
    }

    #[test]
    fn unknown_observation_and_impact_are_not_credited_as_zero() {
        let mut rows = evidence();
        rows[0].observation = ReceiptState::Unknown;
        rows[0].impact = ReceiptState::Unknown;
        rows[0].decision = None;
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &wake_decision(),
            &request(),
            &rows,
            12,
        );
        assert_eq!(report.status, AuditStatus::Indeterminate);
        assert_eq!(report.unknown_observations, 1);
        assert_eq!(report.unknown_impacts, 1);
        assert_eq!(report.decisions_verified, 0);
    }

    #[test]
    fn measured_zero_impact_is_distinct_from_missing_impact() {
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &wake_decision(),
            &request(),
            &evidence(),
            12,
        );
        assert_eq!(report.status, AuditStatus::Qualified);
        assert_eq!(report.impacts_verified, 1);
        assert_eq!(impact_receipt().impact.cpu_milliticks, 0);
        assert_eq!(impact_receipt().impact.io_bytes, 0);
    }

    #[test]
    fn write_capable_catalog_entry_is_a_hard_failure() {
        let mut entries = catalog();
        entries[0].read_only = false;
        let report = run(
            &watch(),
            &authority(),
            &entries,
            &schedule(),
            &wake_decision(),
            &request(),
            &evidence(),
            12,
        );
        assert_eq!(report.status, AuditStatus::Failed);
        assert!(report
            .failures
            .contains(&AuditFailure::ProbeNotCataloguedReadOnly));
    }

    #[test]
    fn request_without_matching_wake_is_rejected() {
        let mut requests = request();
        requests[0].event_id = "other-event".into();
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &wake_decision(),
            &requests,
            &evidence(),
            12,
        );
        assert_eq!(report.status, AuditStatus::Failed);
        assert!(report
            .failures
            .contains(&AuditFailure::RequestNotAuthorizedByWake));
    }

    #[test]
    fn cross_request_observation_substitution_is_rejected() {
        let mut rows = evidence();
        rows[0].observation = ReceiptState::Present(ObservationReceipt {
            request_id: "other-request".into(),
            ..observation_receipt()
        });
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &wake_decision(),
            &request(),
            &rows,
            12,
        );
        assert_eq!(report.status, AuditStatus::Failed);
        assert!(report
            .failures
            .contains(&AuditFailure::EvidenceBindingMismatch));
    }

    #[test]
    fn stale_observation_is_indeterminate_not_a_valid_decision_input() {
        let mut rows = evidence();
        let mut stale = observation_receipt();
        stale.observation.observed_at_tick = 1;
        rows[0].observation = ReceiptState::Present(stale);
        rows[0].decision = None;
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &wake_decision(),
            &request(),
            &rows,
            12,
        );
        assert_eq!(report.status, AuditStatus::Indeterminate);
        assert_eq!(report.observations_verified, 0);
        assert!(report
            .indeterminate_reasons
            .contains(&IndeterminateReason::StaleObservation));
    }

    #[test]
    fn common_control_domain_does_not_establish_independent_impact() {
        let mut rows = evidence();
        let mut same_domain = impact_receipt();
        same_domain.impact.observer_control_domain = "collector-domain".into();
        rows[0].impact = ReceiptState::Present(same_domain);
        rows[0].decision = None;
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &wake_decision(),
            &request(),
            &rows,
            12,
        );
        assert_eq!(report.status, AuditStatus::Indeterminate);
        assert_eq!(report.impacts_verified, 0);
        assert!(report
            .indeterminate_reasons
            .contains(&IndeterminateReason::ImpactIndependenceUnestablished));
    }

    #[test]
    fn impact_window_before_admission_is_rejected_as_binding_failure() {
        let mut rows = evidence();
        let mut early = impact_receipt();
        early.impact.window_start_tick = 6;
        rows[0].impact = ReceiptState::Present(early);
        rows[0].decision = None;
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &wake_decision(),
            &request(),
            &rows,
            12,
        );
        assert_eq!(report.status, AuditStatus::Failed);
        assert!(report
            .failures
            .contains(&AuditFailure::EvidenceBindingMismatch));
    }

    #[test]
    fn revocation_before_admission_rejects_probe_request() {
        let mut revoked = authority();
        revoked.revoked_at_tick = Some(7);
        let report = run(
            &watch(),
            &revoked,
            &catalog(),
            &schedule(),
            &wake_decision(),
            &request(),
            &evidence(),
            12,
        );
        assert_eq!(report.status, AuditStatus::Failed);
        assert!(report
            .failures
            .contains(&AuditFailure::RequestOutsideWatchOrAuthority));
    }

    #[test]
    fn decision_arithmetic_is_recomputed_and_not_agent_accepted() {
        let mut rows = evidence();
        rows[0]
            .decision
            .as_mut()
            .unwrap()
            .claimed_loss_reduction_ppm = 1;
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &wake_decision(),
            &request(),
            &rows,
            12,
        );
        assert_eq!(report.status, AuditStatus::Failed);
        assert_eq!(report.decisions_verified, 0);
        assert!(report
            .failures
            .contains(&AuditFailure::DecisionValueMismatch));
    }

    #[test]
    fn every_wake_requires_an_explicit_no_probe_or_probe_decision() {
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &[],
            &[],
            &[],
            12,
        );
        assert_eq!(report.status, AuditStatus::Failed);
        assert!(report.failures.contains(&AuditFailure::MissingWakeDecision));
    }

    #[test]
    fn explicit_stop_without_probe_is_a_valid_wake_disposition() {
        let mut decisions = wake_decision();
        decisions[0].disposition = WakeDisposition::StopWithoutProbe;
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &decisions,
            &[],
            &[],
            12,
        );
        assert_eq!(report.status, AuditStatus::Qualified);
        assert_eq!(report.wake_decisions, 1);
        assert_eq!(report.probe_requests, 0);
    }

    #[test]
    fn quiet_no_event_schedule_is_a_valid_noop_control() {
        let mut quiet = schedule();
        quiet.status = ScheduleStatus::Quiet;
        quiet.metrics.wakeups = 0;
        quiet.records.clear();
        quiet.wakes.clear();
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &quiet,
            &[],
            &[],
            &[],
            12,
        );
        assert_eq!(report.status, AuditStatus::Qualified);
        assert_eq!(report.wake_events, 0);
        assert_eq!(report.probe_requests, 0);
    }

    #[test]
    fn duplicate_observation_receipt_id_is_rejected() {
        let mut rows = evidence();
        let second = ProbeEvidenceRecord {
            request_id: "request-1".into(),
            event_id: "event-10".into(),
            observation: ReceiptState::Present(observation_receipt()),
            impact: ReceiptState::Present(impact_receipt()),
            decision: Some(decision()),
        };
        rows.push(second);
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &wake_decision(),
            &request(),
            &rows,
            12,
        );
        assert_eq!(report.status, AuditStatus::Failed);
        assert!(report
            .failures
            .contains(&AuditFailure::DuplicateEvidenceRecord));
    }

    #[test]
    fn a_missing_receipt_cannot_support_a_decision() {
        let mut rows = evidence();
        rows[0].impact = ReceiptState::Unknown;
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &wake_decision(),
            &request(),
            &rows,
            12,
        );
        assert_eq!(report.status, AuditStatus::Failed);
        assert!(report
            .failures
            .contains(&AuditFailure::DecisionWithoutCompleteEvidence));
    }

    #[test]
    fn mutation_attempt_in_event_schedule_is_a_hard_failure() {
        let mut changed = schedule();
        changed.metrics.consequential_mutation_attempts = 1;
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &changed,
            &wake_decision(),
            &request(),
            &evidence(),
            12,
        );
        assert_eq!(report.status, AuditStatus::Failed);
        assert!(report
            .failures
            .contains(&AuditFailure::ScheduleMutationAttempt));
    }

    #[test]
    fn future_decision_after_audit_clock_is_rejected() {
        let mut rows = evidence();
        rows[0].decision.as_mut().unwrap().decided_at_tick = 13;
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &wake_decision(),
            &request(),
            &rows,
            12,
        );
        assert_eq!(report.status, AuditStatus::Failed);
        assert_eq!(report.decisions_verified, 0);
        assert!(report
            .failures
            .contains(&AuditFailure::DecisionBindingMismatch));
    }

    #[test]
    fn late_observation_and_impact_receipts_are_indeterminate() {
        let mut requests = request();
        requests[0].deadline_tick = 8;
        let mut rows = evidence();
        rows[0].decision = None;
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &schedule(),
            &wake_decision(),
            &requests,
            &rows,
            12,
        );
        assert_eq!(report.status, AuditStatus::Indeterminate);
        assert_eq!(report.observations_verified, 0);
        assert_eq!(report.impacts_verified, 0);
        assert!(report
            .indeterminate_reasons
            .contains(&IndeterminateReason::ReceiptAfterDeadline));
    }

    #[test]
    fn scheduled_wake_record_cannot_be_omitted_from_wake_batches() {
        let mut changed = schedule();
        changed.records.push(EventRecordResult {
            event_id: "event-orphan".into(),
            incident_id: "incident-orphan".into(),
            sequence: 11,
            delivered_tick: 7,
            processed_tick: Some(8),
            disposition: RecordDisposition::Woke,
            detection_latency_ticks: Some(1),
            deadline_met: true,
        });
        let report = run(
            &watch(),
            &authority(),
            &catalog(),
            &changed,
            &wake_decision(),
            &request(),
            &evidence(),
            12,
        );
        assert_eq!(report.status, AuditStatus::Failed);
        assert!(report
            .failures
            .contains(&AuditFailure::UnpairedScheduleWake));
    }
}
