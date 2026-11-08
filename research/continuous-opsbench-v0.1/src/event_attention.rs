//! Connect frozen event-schedule wakes to bounded read-only probe selection.
//!
//! The evaluator is offline and deterministic. It invokes the existing
//! ProbePolicy once per distinct incident wake, passes the exact triggering
//! event plus only already-eligible read-only probe IDs, and records a
//! selection proposal separately from any collector execution or effect.
//! A wake or selected probe never grants mutation authority.

use crate::event_schedule::{
    evaluate_event_schedule, EventScheduleConfig, EventScheduleError, EventScheduleResult,
    ScheduledAttentionEvent,
};
use crate::watch_probe::{
    eligible_probe_ids_for_watch, AuthoritySnapshot, EpisodeError, ProbeCatalogEntry,
    ProbePolicy, WakeContext, WatchSpec,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeSelectionDisposition {
    Selected,
    NoSelection,
    NoEligibleProbe,
    DeniedOutOfSet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventProbeSelection {
    pub incident_id: String,
    pub event_id: String,
    pub source_sequence: u64,
    pub event_tick: u64,
    pub delivered_tick: u64,
    pub processed_tick: u64,
    pub decision_tick: u64,
    /// The only catalog information exposed to the policy.
    pub eligible_probe_ids: Vec<String>,
    pub selected_probe_id: Option<String>,
    pub disposition: ProbeSelectionDisposition,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventProbeMetrics {
    pub incident_wakes: usize,
    pub cognitive_policy_invocations: usize,
    pub probe_selection_calls: usize,
    pub admitted_probe_proposals: usize,
    pub denied_probe_proposals: usize,
    pub no_selection: usize,
    pub no_eligible_probe: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventProbeEvaluation {
    pub schedule: EventScheduleResult,
    pub metrics: EventProbeMetrics,
    pub selections: Vec<EventProbeSelection>,
    /// The selection evaluator has no mutation capability.
    pub consequential_mutation_attempts: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventProbeError {
    EventSchedule(EventScheduleError),
    Watch(EpisodeError),
    DecisionBeforeWake,
    DecisionAfterEvaluationClock,
}

impl From<EventScheduleError> for EventProbeError {
    fn from(value: EventScheduleError) -> Self {
        Self::EventSchedule(value)
    }
}

impl From<EpisodeError> for EventProbeError {
    fn from(value: EpisodeError) -> Self {
        Self::Watch(value)
    }
}

/// Evaluate a frozen event schedule and invoke the probe policy once per
/// distinct incident wake.
///
/// `decision_tick` is the simulated time at which the policy actually handles
/// the wake. It may be later than event processing; current authority and the
/// eligible read-only catalog are rechecked at this boundary. An expired or
/// revoked authority produces an empty eligible-ID view, never cached access.
pub fn evaluate_event_wakes_with_probe_policy<P: ProbePolicy>(
    watch: &WatchSpec,
    authority: &AuthoritySnapshot,
    schedule_config: &EventScheduleConfig,
    events: &[ScheduledAttentionEvent],
    current_tick: u64,
    decision_tick: u64,
    catalog: &[ProbeCatalogEntry],
    policy: &mut P,
) -> Result<EventProbeEvaluation, EventProbeError> {
    if decision_tick < watch.starts_at_tick {
        return Err(EventProbeError::DecisionBeforeWake);
    }
    if decision_tick > current_tick {
        return Err(EventProbeError::DecisionAfterEvaluationClock);
    }

    let schedule =
        evaluate_event_schedule(watch, authority, schedule_config, events, current_tick)?;
    let mut metrics = EventProbeMetrics {
        incident_wakes: schedule.wakes.len(),
        ..EventProbeMetrics::default()
    };
    let mut selections = Vec::with_capacity(schedule.wakes.len());

    for wake in &schedule.wakes {
        // Eligibility is re-evaluated at decision/admission time, not copied
        // from the event delivery time or a previous wake.
        let eligible_probe_ids =
            eligible_probe_ids_for_watch(watch, authority, decision_tick, catalog)?;
        let context = WakeContext {
            watch_id: watch.watch_id.clone(),
            event: wake.event.clone(),
            authority_epoch: authority.epoch,
        };

        metrics.cognitive_policy_invocations += 1;
        metrics.probe_selection_calls += 1;
        let proposed = policy.select_probe(&context, &eligible_probe_ids);

        let (selected_probe_id, disposition) = match proposed {
            Some(id) if eligible_probe_ids.iter().any(|eligible| eligible == &id) => {
                metrics.admitted_probe_proposals += 1;
                (Some(id), ProbeSelectionDisposition::Selected)
            }
            Some(id) => {
                metrics.denied_probe_proposals += 1;
                // Retain the rejected identifier for auditing; it is never
                // converted into an admitted catalog entry or action.
                (Some(id), ProbeSelectionDisposition::DeniedOutOfSet)
            }
            None if eligible_probe_ids.is_empty() => {
                metrics.no_eligible_probe += 1;
                (None, ProbeSelectionDisposition::NoEligibleProbe)
            }
            None => {
                metrics.no_selection += 1;
                (None, ProbeSelectionDisposition::NoSelection)
            }
        };

        selections.push(EventProbeSelection {
            incident_id: wake.incident_id.clone(),
            event_id: wake.event.event_id.clone(),
            source_sequence: wake.event.sequence,
            event_tick: wake.event.observed_at_tick,
            delivered_tick: wake.event.delivered_at_tick,
            processed_tick: wake.processed_tick,
            decision_tick,
            eligible_probe_ids,
            selected_probe_id,
            disposition,
        });
    }

    Ok(EventProbeEvaluation {
        schedule,
        metrics,
        selections,
        consequential_mutation_attempts: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_schedule::EventPersistence;
    use crate::watch_probe::{ProbeCatalogEntry, ProbePolicy};

    #[derive(Default)]
    struct RecordingPolicy {
        proposed_probe_id: Option<String>,
        seen_events: Vec<(String, u64, String)>,
        seen_eligible_ids: Vec<Vec<String>>,
    }

    impl ProbePolicy for RecordingPolicy {
        fn select_probe(
            &mut self,
            wake: &WakeContext,
            eligible_probe_ids: &[String],
        ) -> Option<String> {
            self.seen_events.push((
                wake.event.event_id.clone(),
                wake.event.sequence,
                wake.event.source_id.clone(),
            ));
            self.seen_eligible_ids.push(eligible_probe_ids.to_vec());
            self.proposed_probe_id.clone()
        }
    }

    fn watch() -> WatchSpec {
        WatchSpec {
            watch_id: "watch-event-probe".into(),
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
            epoch: 9,
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

    fn event(
        sequence: u64,
        event_id: &str,
        incident_id: &str,
        event_tick: u64,
        delivered_tick: u64,
        processed_tick: u64,
    ) -> ScheduledAttentionEvent {
        ScheduledAttentionEvent {
            envelope: crate::watch_probe::EventEnvelope {
                event_id: event_id.into(),
                source_id: "event-source-a".into(),
                sequence,
                event_type: "service-degraded".into(),
                tenant_id: "tenant-a".into(),
                target_id: "service-a".into(),
                purpose_id: "incident-diagnosis".into(),
                condition_met: true,
                observed_at_tick: event_tick,
                delivered_at_tick: delivered_tick,
            },
            payload_digest: format!("sha256:{:064x}", sequence),
            incident_id: incident_id.into(),
            persistence: EventPersistence::Persistent,
            condition_cleared_at_tick: None,
            processed_at_tick: processed_tick,
        }
    }

    fn probe(id: &str, tenant: &str, target: &str, purpose: &str, read_only: bool) -> ProbeCatalogEntry {
        ProbeCatalogEntry {
            probe_id: id.into(),
            operation_digest: format!("sha256:{}-operation", id),
            tenant_id: tenant.into(),
            target_id: target.into(),
            purpose_id: purpose.into(),
            source_id: "diagnostic-collector".into(),
            collector_control_domain: "diagnostic-domain".into(),
            read_only,
            maximum_duration_ticks: 2,
        }
    }

    fn catalog() -> Vec<ProbeCatalogEntry> {
        vec![
            probe("probe-a", "tenant-a", "service-a", "incident-diagnosis", true),
            probe("probe-other-tenant", "tenant-b", "service-a", "incident-diagnosis", true),
            probe("probe-other-purpose", "tenant-a", "service-a", "data-export", true),
            probe("probe-write", "tenant-a", "service-a", "incident-diagnosis", false),
        ]
    }

    #[test]
    fn one_policy_invocation_per_distinct_incident_wake() {
        let events = [
            event(1, "event-a1", "incident-a", 4, 5, 5),
            event(2, "event-a2", "incident-a", 6, 6, 7),
            event(3, "event-b1", "incident-b", 8, 9, 9),
        ];
        let mut policy = RecordingPolicy {
            proposed_probe_id: Some("probe-a".into()),
            ..RecordingPolicy::default()
        };
        let result = evaluate_event_wakes_with_probe_policy(
            &watch(),
            &authority(),
            &config(),
            &events,
            20,
            10,
            &catalog(),
            &mut policy,
        )
        .unwrap();

        assert_eq!(result.metrics.incident_wakes, 2);
        assert_eq!(result.metrics.cognitive_policy_invocations, 2);
        assert_eq!(result.metrics.probe_selection_calls, 2);
        assert_eq!(result.metrics.admitted_probe_proposals, 2);
        assert_eq!(result.schedule.wakes[0].event_ids, vec!["event-a1", "event-a2"]);
        assert_eq!(policy.seen_events[1].0, "event-b1");
        assert_eq!(result.consequential_mutation_attempts, 0);
    }

    #[test]
    fn policy_receives_only_current_scope_read_only_probe_ids() {
        let mut policy = RecordingPolicy {
            proposed_probe_id: Some("probe-a".into()),
            ..RecordingPolicy::default()
        };
        let result = evaluate_event_wakes_with_probe_policy(
            &watch(),
            &authority(),
            &config(),
            &[event(1, "event-a", "incident-a", 4, 5, 5)],
            20,
            6,
            &catalog(),
            &mut policy,
        )
        .unwrap();

        assert_eq!(policy.seen_eligible_ids, vec![vec!["probe-a".to_string()]]);
        assert_eq!(result.selections[0].eligible_probe_ids, vec!["probe-a"]);
        assert_eq!(result.selections[0].disposition, ProbeSelectionDisposition::Selected);
    }

    #[test]
    fn authority_revoked_after_event_processing_yields_empty_eligible_set() {
        let mut revoked = authority();
        revoked.revoked_at_tick = Some(7);
        let mut policy = RecordingPolicy {
            proposed_probe_id: Some("probe-a".into()),
            ..RecordingPolicy::default()
        };
        let result = evaluate_event_wakes_with_probe_policy(
            &watch(),
            &revoked,
            &config(),
            &[event(1, "event-a", "incident-a", 4, 5, 5)],
            20,
            8,
            &catalog(),
            &mut policy,
        )
        .unwrap();

        assert_eq!(result.schedule.metrics.wakeups, 1);
        assert_eq!(policy.seen_eligible_ids, vec![Vec::<String>::new()]);
        assert_eq!(result.metrics.no_eligible_probe, 1);
        assert_eq!(result.metrics.admitted_probe_proposals, 0);
        assert_eq!(result.consequential_mutation_attempts, 0);
    }

    #[test]
    fn out_of_set_probe_proposal_is_denied() {
        let mut policy = RecordingPolicy {
            proposed_probe_id: Some("probe-other-tenant".into()),
            ..RecordingPolicy::default()
        };
        let result = evaluate_event_wakes_with_probe_policy(
            &watch(),
            &authority(),
            &config(),
            &[event(1, "event-a", "incident-a", 4, 5, 5)],
            20,
            6,
            &catalog(),
            &mut policy,
        )
        .unwrap();

        assert_eq!(result.metrics.denied_probe_proposals, 1);
        assert_eq!(result.selections[0].disposition, ProbeSelectionDisposition::DeniedOutOfSet);
        assert_eq!(result.selections[0].selected_probe_id.as_deref(), Some("probe-other-tenant"));
        assert_eq!(result.selections[0].eligible_probe_ids, vec!["probe-a"]);
    }

    #[test]
    fn invalid_write_capable_or_out_of_scope_catalog_rows_are_never_exposed() {
        let mut policy = RecordingPolicy {
            proposed_probe_id: None,
            ..RecordingPolicy::default()
        };
        let result = evaluate_event_wakes_with_probe_policy(
            &watch(),
            &authority(),
            &config(),
            &[event(1, "event-a", "incident-a", 4, 5, 5)],
            20,
            6,
            &catalog(),
            &mut policy,
        )
        .unwrap();
        assert_eq!(policy.seen_eligible_ids, vec![vec!["probe-a".to_string()]]);
        assert_eq!(result.metrics.no_selection, 1);
        assert_eq!(result.metrics.denied_probe_proposals, 0);
    }

    #[test]
    fn no_wake_means_no_cognitive_or_probe_selection_invocation() {
        let mut irrelevant = event(1, "event-a", "incident-a", 4, 5, 5);
        irrelevant.envelope.event_type = "heartbeat".into();
        let mut policy = RecordingPolicy {
            proposed_probe_id: Some("probe-a".into()),
            ..RecordingPolicy::default()
        };
        let result = evaluate_event_wakes_with_probe_policy(
            &watch(),
            &authority(),
            &config(),
            &[irrelevant],
            20,
            6,
            &catalog(),
            &mut policy,
        )
        .unwrap();

        assert_eq!(result.schedule.metrics.wakeups, 0);
        assert_eq!(result.metrics.cognitive_policy_invocations, 0);
        assert_eq!(result.metrics.probe_selection_calls, 0);
        assert!(policy.seen_events.is_empty());
    }

    #[test]
    fn missed_event_deadline_does_not_trigger_probe_policy() {
        let late = event(1, "event-late", "incident-late", 4, 8, 10);
        let mut policy = RecordingPolicy {
            proposed_probe_id: Some("probe-a".into()),
            ..RecordingPolicy::default()
        };
        let result = evaluate_event_wakes_with_probe_policy(
            &watch(),
            &authority(),
            &config(),
            &[late],
            20,
            11,
            &catalog(),
            &mut policy,
        )
        .unwrap();

        assert_eq!(result.schedule.metrics.late_events, 1);
        assert_eq!(result.metrics.cognitive_policy_invocations, 0);
        assert_eq!(result.metrics.admitted_probe_proposals, 0);
    }

    #[test]
    fn decision_tick_before_wake_or_after_clock_is_rejected() {
        let event = event(1, "event-a", "incident-a", 4, 5, 5);
        let mut policy = RecordingPolicy::default();
        assert_eq!(
            evaluate_event_wakes_with_probe_policy(
                &watch(),
                &authority(),
                &config(),
                std::slice::from_ref(&event),
                20,
                4,
                &catalog(),
                &mut policy,
            ),
            Err(EventProbeError::DecisionBeforeWake)
        );
        assert_eq!(
            evaluate_event_wakes_with_probe_policy(
                &watch(),
                &authority(),
                &config(),
                &[event],
                20,
                21,
                &catalog(),
                &mut policy,
            ),
            Err(EventProbeError::DecisionAfterEvaluationClock)
        );
    }
}
