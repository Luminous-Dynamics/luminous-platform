//! Paired fixed-poll versus event-triggered attention experiment.
//!
//! This adapter reuses the deterministic continuous-ops plant. The plant
//! observation callback runs each actionable tick as a lightweight monitor;
//! expensive policy invocations occur only at fixed poll times or on a
//! rising-edge event, depending on the arm. The policy receives only eligible
//! read-only probe identifiers. This is a synthetic paired experiment, not a
//! production event-bus or collector runtime.

use crate::stability::{
    run_stability_closed_loop, run_stability_scenario, ClosedLoopRun, ControlOperation, Controller,
    PlantObservation, StabilityConfig, StabilityController, StabilityError, StabilityRun,
};
use crate::watch_probe::{
    eligible_probe_ids_for_watch, AuthoritySnapshot, EpisodeError, EventEnvelope,
    ProbeCatalogEntry, ProbePolicy, WakeContext, WatchSpec,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttentionMode {
    FixedPolling,
    EventTriggered,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttentionExperimentConfig {
    pub fixed_poll_interval_ticks: u32,
    pub failed_ppm_wake_threshold: u32,
    pub queue_depth_wake_threshold: u64,
    /// Per-arm ceiling on admitted probe selections, not cognitive invocations.
    pub probe_budget_per_arm: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttentionWakeRecord {
    pub decision_tick: u32,
    pub observation_source_tick: u32,
    pub condition_observed: bool,
    pub mode: AttentionMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeSelectionRecord {
    pub wake: WakeContext,
    pub decision_tick: u32,
    pub observation_source_tick: u32,
    pub observed_failed_ppm: u32,
    pub observed_queue_depth: u64,
    pub selected_probe_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttentionMetrics {
    /// Lightweight monitor callbacks; these do not imply an LLM invocation.
    pub monitor_checks: usize,
    /// Actual policy invocations under the arm's wake rule.
    pub cognitive_invocations: usize,
    pub scheduled_poll_wakeups: usize,
    pub relevant_event_wakeups: usize,
    pub false_wakeups: usize,
    pub missed_fault_episodes: usize,
    pub first_fault_tick: Option<u32>,
    pub first_verified_wake_tick: Option<u32>,
    pub detection_latency_ticks: Option<u32>,
    pub probe_selection_calls: usize,
    pub admitted_probe_selections: usize,
    pub denied_probe_selections: usize,
    pub no_eligible_probe_wakeups: usize,
    pub probe_budget_exhaustions: usize,
    pub wake_budget_exhaustions: usize,
    pub wakes: Vec<AttentionWakeRecord>,
    pub selections: Vec<ProbeSelectionRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttentionArmResult {
    pub mode: AttentionMode,
    pub metrics: AttentionMetrics,
    pub run: ClosedLoopRun,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairedAttentionResult {
    pub fixed_poll: AttentionArmResult,
    pub event_triggered: AttentionArmResult,
    pub no_action_baseline: StabilityRun,
    /// The same immutable config reference (including noise vectors) was used
    /// to execute all three arms.
    pub same_frozen_plant_config: bool,
    /// Signed relative reduction; negative means event-triggered invoked the
    /// policy more often than fixed polling.
    pub cognitive_invocation_reduction_ppm: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttentionError {
    InvalidAttentionConfig,
    Episode(EpisodeError),
    Stability(StabilityError),
}

impl From<EpisodeError> for AttentionError {
    fn from(value: EpisodeError) -> Self {
        Self::Episode(value)
    }
}

impl From<StabilityError> for AttentionError {
    fn from(value: StabilityError) -> Self {
        Self::Stability(value)
    }
}

struct AttentionController<'a> {
    mode: AttentionMode,
    config: &'a AttentionExperimentConfig,
    watch: &'a WatchSpec,
    authority: &'a AuthoritySnapshot,
    catalog: &'a [ProbeCatalogEntry],
    policy: &'a mut dyn ProbePolicy,
    metrics: AttentionMetrics,
    previous_condition: bool,
    next_event_sequence: u64,
}

impl AttentionController<'_> {
    fn condition_met(&self, observation: &PlantObservation) -> bool {
        observation.observed_failed_ppm >= self.config.failed_ppm_wake_threshold
            || observation.observed_queue_depth >= self.config.queue_depth_wake_threshold
    }

    fn should_wake(&self, observation: &PlantObservation, condition_met: bool) -> bool {
        match self.mode {
            AttentionMode::FixedPolling => {
                observation.tick % self.config.fixed_poll_interval_ticks == 0
            }
            AttentionMode::EventTriggered => condition_met && !self.previous_condition,
        }
    }

    fn select_probe(&mut self, observation: &PlantObservation) -> Option<String> {
        let eligible_ids = match eligible_probe_ids_for_watch(
            self.watch,
            self.authority,
            observation.tick as u64,
            self.catalog,
        ) {
            Ok(ids) => ids,
            Err(EpisodeError::InvalidProbeCatalog) => {
                self.metrics.denied_probe_selections += 1;
                return None;
            }
            Err(_) => {
                self.metrics.no_eligible_probe_wakeups += 1;
                return None;
            }
        };

        if eligible_ids.is_empty() {
            self.metrics.no_eligible_probe_wakeups += 1;
            return None;
        }
        if self.metrics.admitted_probe_selections >= self.config.probe_budget_per_arm {
            self.metrics.probe_budget_exhaustions += 1;
            return None;
        }

        self.next_event_sequence = self.next_event_sequence.saturating_add(1);
        let sequence = self.next_event_sequence;
        let event = EventEnvelope {
            event_id: format!(
                "attention-{}-{}",
                match self.mode {
                    AttentionMode::FixedPolling => "fixed-poll",
                    AttentionMode::EventTriggered => "event-triggered",
                },
                sequence
            ),
            source_id: self.watch.source_id.clone(),
            sequence,
            event_type: self.watch.event_type.clone(),
            tenant_id: self.watch.tenant_id.clone(),
            target_id: self.watch.target_id.clone(),
            purpose_id: self.watch.purpose_id.clone(),
            // The event here is the wake record, not a claim that the service
            // is unhealthy. The evaluator later compares it with plant truth.
            condition_met: true,
            observed_at_tick: observation.source_tick as u64,
            delivered_at_tick: observation.tick as u64,
        };
        let wake = WakeContext {
            watch_id: self.watch.watch_id.clone(),
            event,
            authority_epoch: self.authority.epoch,
        };

        self.metrics.probe_selection_calls += 1;
        let selected = self.policy.select_probe(&wake, &eligible_ids);
        match selected {
            Some(probe_id) if eligible_ids.iter().any(|id| id == &probe_id) => {
                self.metrics.admitted_probe_selections += 1;
                self.metrics.selections.push(ProbeSelectionRecord {
                    wake,
                    decision_tick: observation.tick,
                    observation_source_tick: observation.source_tick,
                    observed_failed_ppm: observation.observed_failed_ppm,
                    observed_queue_depth: observation.observed_queue_depth,
                    selected_probe_id: probe_id.clone(),
                });
                Some(probe_id)
            }
            Some(_) => {
                self.metrics.denied_probe_selections += 1;
                None
            }
            None => None,
        }
    }
}

impl StabilityController for AttentionController<'_> {
    fn controller_id(&self) -> &str {
        match self.mode {
            AttentionMode::FixedPolling => "attention-fixed-poll",
            AttentionMode::EventTriggered => "attention-event-triggered",
        }
    }

    fn controller(&self) -> Controller {
        Controller::Agent
    }

    fn propose(&mut self, observation: &PlantObservation) -> Vec<ControlOperation> {
        self.metrics.monitor_checks += 1;
        let condition_met = self.condition_met(observation);
        let watch_active = observation.tick as u64 >= self.watch.starts_at_tick
            && (observation.tick as u64) < self.watch.expires_at_tick;
        if !watch_active {
            // Do not carry a pre-subscription condition across the watch
            // boundary; a condition already true at activation must wake.
            self.previous_condition = false;
            return Vec::new();
        }
        let wake = self.should_wake(observation, condition_met);
        self.previous_condition = condition_met;

        if !wake {
            return Vec::new();
        }

        if self.metrics.cognitive_invocations >= self.watch.max_wakeups as usize {
            self.metrics.wake_budget_exhaustions += 1;
            return Vec::new();
        }

        self.metrics.cognitive_invocations += 1;
        self.metrics.wakes.push(AttentionWakeRecord {
            decision_tick: observation.tick,
            observation_source_tick: observation.source_tick,
            condition_observed: condition_met,
            mode: self.mode,
        });
        match self.mode {
            AttentionMode::FixedPolling => self.metrics.scheduled_poll_wakeups += 1,
            AttentionMode::EventTriggered => self.metrics.relevant_event_wakeups += 1,
        }

        // Fixed polling still pays for a cognitive invocation on quiet polls,
        // but it does not spend a probe budget unless the observation suggests
        // the watched condition is present.
        if !condition_met {
            return Vec::new();
        }

        // Selecting a read-only probe is not authority to mutate the plant.
        // This attention study measures wake and selection behavior only.
        let _ = self.select_probe(observation);
        Vec::new()
    }
}

fn finalize_metrics(metrics: &mut AttentionMetrics, run: &ClosedLoopRun) {
    let first_fault_tick = run
        .stability
        .samples
        .iter()
        .find(|sample| !sample.slo_healthy)
        .map(|sample| sample.tick);
    metrics.first_fault_tick = first_fault_tick;

    let first_true_wake = metrics.wakes.iter().find(|wake| {
        run.stability
            .samples
            .get(wake.observation_source_tick as usize)
            .is_some_and(|sample| !sample.slo_healthy)
    });

    if let Some(wake) = first_true_wake {
        metrics.first_verified_wake_tick = Some(wake.decision_tick);
        if let Some(fault_tick) = first_fault_tick {
            metrics.detection_latency_ticks = Some(wake.decision_tick.saturating_sub(fault_tick));
        }
    } else if first_fault_tick.is_some() {
        metrics.missed_fault_episodes = 1;
    }

    metrics.false_wakeups = metrics
        .wakes
        .iter()
        .filter(|wake| {
            run.stability
                .samples
                .get(wake.observation_source_tick as usize)
                .is_none_or(|sample| sample.slo_healthy)
        })
        .count();
}

/// Run fixed-poll and event-triggered attention policies against the same
/// immutable deterministic plant configuration, plus a no-action counterfactual.
///
/// The policy sees delayed/noisy observations and only eligible read-only probe
/// identifiers. The plant's actual samples are evaluator-only and are used
/// after each run to score misses, false wakes, latency and service outcomes.
pub fn run_paired_attention_experiment(
    plant: &StabilityConfig,
    attention: &AttentionExperimentConfig,
    watch: &WatchSpec,
    authority: &AuthoritySnapshot,
    catalog: &[ProbeCatalogEntry],
    fixed_policy: &mut dyn ProbePolicy,
    event_policy: &mut dyn ProbePolicy,
) -> Result<PairedAttentionResult, AttentionError> {
    if attention.fixed_poll_interval_ticks == 0 || attention.failed_ppm_wake_threshold > 1_000_000 {
        return Err(AttentionError::InvalidAttentionConfig);
    }

    // Validate watch and authority shape before either arm runs, even when a
    // quiet scenario never wakes and therefore never requests a probe.
    let _ = eligible_probe_ids_for_watch(watch, authority, watch.starts_at_tick, catalog)?;
    let no_action_baseline = run_stability_scenario(plant, &[])?;

    let mut fixed_controller = AttentionController {
        mode: AttentionMode::FixedPolling,
        config: attention,
        watch,
        authority,
        catalog,
        policy: fixed_policy,
        metrics: AttentionMetrics::default(),
        previous_condition: false,
        next_event_sequence: watch.first_sequence.saturating_sub(1),
    };
    let fixed_run = run_stability_closed_loop(plant, &mut fixed_controller)?;
    let mut fixed_metrics = fixed_controller.metrics;
    finalize_metrics(&mut fixed_metrics, &fixed_run);

    let mut event_controller = AttentionController {
        mode: AttentionMode::EventTriggered,
        config: attention,
        watch,
        authority,
        catalog,
        policy: event_policy,
        metrics: AttentionMetrics::default(),
        previous_condition: false,
        next_event_sequence: watch.first_sequence.saturating_sub(1),
    };
    let event_run = run_stability_closed_loop(plant, &mut event_controller)?;
    let mut event_metrics = event_controller.metrics;
    finalize_metrics(&mut event_metrics, &event_run);

    let fixed_count = fixed_metrics.cognitive_invocations as i128;
    let event_count = event_metrics.cognitive_invocations as i128;
    let cognitive_invocation_reduction_ppm = if fixed_count > 0 {
        Some(
            (fixed_count - event_count)
                .saturating_mul(1_000_000)
                .checked_div(fixed_count)
                .unwrap_or(0),
        )
    } else {
        None
    };

    Ok(PairedAttentionResult {
        fixed_poll: AttentionArmResult {
            mode: AttentionMode::FixedPolling,
            metrics: fixed_metrics,
            run: fixed_run,
        },
        event_triggered: AttentionArmResult {
            mode: AttentionMode::EventTriggered,
            metrics: event_metrics,
            run: event_run,
        },
        no_action_baseline,
        same_frozen_plant_config: true,
        cognitive_invocation_reduction_ppm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watch_probe::ScriptedProbePolicy;

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

    fn attention() -> AttentionExperimentConfig {
        AttentionExperimentConfig {
            fixed_poll_interval_ticks: 4,
            failed_ppm_wake_threshold: 100_000,
            queue_depth_wake_threshold: 8,
            probe_budget_per_arm: 1,
        }
    }

    fn watch() -> WatchSpec {
        WatchSpec {
            watch_id: "paired-attention-watch".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            source_id: "plant-event-feed".into(),
            event_type: "service-degradation".into(),
            first_sequence: 1,
            starts_at_tick: 0,
            expires_at_tick: 32,
            max_wakeups: 8,
            max_probe_duration_ticks: 3,
            max_observation_age_ticks: 4,
        }
    }

    fn authority() -> AuthoritySnapshot {
        AuthoritySnapshot {
            epoch: 1,
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            valid_from_tick: 0,
            expires_at_tick: 32,
            revoked_at_tick: None,
        }
    }

    fn catalog() -> Vec<ProbeCatalogEntry> {
        vec![ProbeCatalogEntry {
            probe_id: "probe-queue-depth".into(),
            operation_digest: "sha256:queue-depth".into(),
            tenant_id: "tenant-a".into(),
            target_id: "service-a".into(),
            purpose_id: "incident-diagnosis".into(),
            source_id: "diagnostic-collector".into(),
            collector_control_domain: "collector-domain".into(),
            read_only: true,
            maximum_duration_ticks: 2,
        }]
    }

    fn paired() -> PairedAttentionResult {
        let mut fixed = ScriptedProbePolicy {
            selected_probe_id: Some("probe-queue-depth".into()),
        };
        let mut event = ScriptedProbePolicy {
            selected_probe_id: Some("probe-queue-depth".into()),
        };
        run_paired_attention_experiment(
            &plant(),
            &attention(),
            &watch(),
            &authority(),
            &catalog(),
            &mut fixed,
            &mut event,
        )
        .unwrap()
    }

    #[test]
    fn event_triggered_watch_activating_during_existing_fault_still_wakes() {
        let mut watch = watch();
        watch.starts_at_tick = 12;
        let mut fixed = ScriptedProbePolicy {
            selected_probe_id: Some("probe-queue-depth".into()),
        };
        let mut event = ScriptedProbePolicy {
            selected_probe_id: Some("probe-queue-depth".into()),
        };
        let result = run_paired_attention_experiment(
            &plant(),
            &attention(),
            &watch,
            &authority(),
            &catalog(),
            &mut fixed,
            &mut event,
        )
        .unwrap();

        assert_eq!(
            result.event_triggered.metrics.first_verified_wake_tick,
            Some(12)
        );
        assert_eq!(result.event_triggered.metrics.missed_fault_episodes, 0);
        assert_eq!(result.event_triggered.metrics.admitted_probe_selections, 1);
    }

    #[test]
    fn expired_watch_does_not_keep_triggering_on_a_persistent_fault() {
        let mut watch = watch();
        watch.expires_at_tick = 9;
        let mut fixed = ScriptedProbePolicy {
            selected_probe_id: Some("probe-queue-depth".into()),
        };
        let mut event = ScriptedProbePolicy {
            selected_probe_id: Some("probe-queue-depth".into()),
        };
        let result = run_paired_attention_experiment(
            &plant(),
            &attention(),
            &watch,
            &authority(),
            &catalog(),
            &mut fixed,
            &mut event,
        )
        .unwrap();

        assert_eq!(result.event_triggered.metrics.cognitive_invocations, 0);
        assert_eq!(result.event_triggered.metrics.admitted_probe_selections, 0);
        assert_eq!(result.event_triggered.metrics.missed_fault_episodes, 1);
    }

    #[test]
    fn event_triggered_policy_uses_fewer_cognitive_invocations_than_fixed_polling() {
        let result = paired();
        assert!(result.same_frozen_plant_config);
        assert!(
            result.event_triggered.metrics.cognitive_invocations
                < result.fixed_poll.metrics.cognitive_invocations
        );
        assert_eq!(result.event_triggered.metrics.missed_fault_episodes, 0);
        assert_eq!(result.fixed_poll.metrics.missed_fault_episodes, 0);
        assert_eq!(result.event_triggered.metrics.probe_selection_calls, 1);
        assert_eq!(result.fixed_poll.metrics.probe_selection_calls, 1);
        assert!(
            result
                .event_triggered
                .metrics
                .detection_latency_ticks
                .unwrap()
                < result.fixed_poll.metrics.detection_latency_ticks.unwrap()
        );
    }

    #[test]
    fn read_only_probe_selection_never_authorizes_control_actions() {
        let result = paired();
        assert_eq!(result.no_action_baseline.samples.len(), 32);
        assert_eq!(result.fixed_poll.run.stability.samples.len(), 32);
        assert_eq!(result.event_triggered.run.stability.samples.len(), 32);
        assert!(result.fixed_poll.run.actions.is_empty());
        assert!(result.event_triggered.run.actions.is_empty());
        assert_eq!(
            result.no_action_baseline.samples,
            result.fixed_poll.run.stability.samples
        );
        assert_eq!(
            result.no_action_baseline.samples,
            result.event_triggered.run.stability.samples
        );
    }

    #[test]
    fn empty_action_traces_are_replay_qualified_by_existing_plant_runner() {
        let result = paired();
        assert!(result.event_triggered.run.actions.is_empty());
        assert!(result.fixed_poll.run.actions.is_empty());
        assert!(result.event_triggered.run.decisions.len() > 0);
        assert!(result.fixed_poll.run.decisions.len() > 0);
    }

    #[test]
    fn invalid_authority_produces_no_eligible_probe_selection() {
        let mut expired = authority();
        expired.expires_at_tick = 9;
        let mut fixed = ScriptedProbePolicy {
            selected_probe_id: Some("probe-queue-depth".into()),
        };
        let mut event = ScriptedProbePolicy {
            selected_probe_id: Some("probe-queue-depth".into()),
        };
        let result = run_paired_attention_experiment(
            &plant(),
            &attention(),
            &watch(),
            &expired,
            &catalog(),
            &mut fixed,
            &mut event,
        )
        .unwrap();
        assert_eq!(result.event_triggered.metrics.admitted_probe_selections, 0);
        assert!(result.event_triggered.metrics.no_eligible_probe_wakeups > 0);
        assert!(result.event_triggered.run.actions.is_empty());
    }

    #[test]
    fn out_of_scope_policy_proposal_is_denied_without_control_action() {
        let mut fixed = ScriptedProbePolicy {
            selected_probe_id: Some("probe-not-eligible".into()),
        };
        let mut event = ScriptedProbePolicy {
            selected_probe_id: Some("probe-not-eligible".into()),
        };
        let result = run_paired_attention_experiment(
            &plant(),
            &attention(),
            &watch(),
            &authority(),
            &catalog(),
            &mut fixed,
            &mut event,
        )
        .unwrap();
        assert_eq!(result.event_triggered.metrics.denied_probe_selections, 1);
        assert_eq!(result.event_triggered.metrics.admitted_probe_selections, 0);
        assert!(result.event_triggered.run.actions.is_empty());
    }

    #[test]
    fn zero_probe_budget_is_a_valid_no_probe_control() {
        let mut config = attention();
        config.probe_budget_per_arm = 0;
        let mut fixed = ScriptedProbePolicy {
            selected_probe_id: Some("probe-queue-depth".into()),
        };
        let mut event = ScriptedProbePolicy {
            selected_probe_id: Some("probe-queue-depth".into()),
        };
        let result = run_paired_attention_experiment(
            &plant(),
            &config,
            &watch(),
            &authority(),
            &catalog(),
            &mut fixed,
            &mut event,
        )
        .unwrap();
        assert_eq!(result.event_triggered.metrics.admitted_probe_selections, 0);
        assert_eq!(result.fixed_poll.metrics.admitted_probe_selections, 0);
        assert_eq!(result.event_triggered.metrics.probe_budget_exhaustions, 1);
        assert_eq!(result.fixed_poll.metrics.probe_budget_exhaustions, 1);
        assert!(result.event_triggered.run.actions.is_empty());
        assert!(result.fixed_poll.run.actions.is_empty());
    }
}
