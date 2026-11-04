//! Deterministic closed-loop operational stability plant.
//!
//! This small synthetic plant exposes delayed actuation/observation, retry
//! amplification, API/database bottlenecks, workload priority, action churn,
//! and a no-intervention recovery tail. It is a scenario/evaluator reference,
//! not a model of arbitrary production systems or a proof of stability.

use std::collections::{BTreeMap, BTreeSet};

pub const PPM: u64 = 1_000_000;
pub const MAX_RETRY_MULTIPLIER_PPM: u32 = 3_000_000;
pub const NOISE_GENERATOR_V1: &str = "splitmix64-rejection-v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeterministicNoiseTrace {
    pub algorithm: String,
    pub seed: u64,
    pub queue_noise_by_tick: Vec<i64>,
    pub failed_ppm_noise_by_tick: Vec<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoiseGenerationError {
    ZeroTicks,
    QueueBoundTooLarge,
    FailureNoiseBoundTooLarge,
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self { Self { state: seed } }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    fn bounded(&mut self, upper_exclusive: u64) -> u64 {
        // Rejection sampling avoids modulo bias for the finite integer interval.
        let threshold = upper_exclusive.wrapping_neg() % upper_exclusive;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return value % upper_exclusive;
            }
        }
    }

    fn signed(&mut self, absolute_bound: u64) -> i64 {
        let width = absolute_bound * 2 + 1;
        (self.bounded(width) as i128 - absolute_bound as i128) as i64
    }
}

/// Generate deterministic paired observation noise. This is a reproducibility
/// PRNG, not cryptographic randomness. The generator name and seed must be
/// captured in the scenario/run manifest; never change its mapping in-place.
pub fn generate_deterministic_noise_v1(
    seed: u64,
    ticks: u32,
    maximum_absolute_queue_noise: u64,
    maximum_absolute_failed_ppm_noise: u32,
) -> Result<DeterministicNoiseTrace, NoiseGenerationError> {
    if ticks == 0 {
        return Err(NoiseGenerationError::ZeroTicks);
    }
    if maximum_absolute_queue_noise > i64::MAX as u64 {
        return Err(NoiseGenerationError::QueueBoundTooLarge);
    }
    if maximum_absolute_failed_ppm_noise > PPM as u32 {
        return Err(NoiseGenerationError::FailureNoiseBoundTooLarge);
    }

    let mut rng = SplitMix64::new(seed);
    let mut queue_noise_by_tick = Vec::with_capacity(ticks as usize);
    let mut failed_ppm_noise_by_tick = Vec::with_capacity(ticks as usize);
    for _ in 0..ticks {
        queue_noise_by_tick.push(rng.signed(maximum_absolute_queue_noise));
        failed_ppm_noise_by_tick.push(rng.signed(maximum_absolute_failed_ppm_noise as u64));
    }
    Ok(DeterministicNoiseTrace {
        algorithm: NOISE_GENERATOR_V1.to_owned(),
        seed,
        queue_noise_by_tick,
        failed_ppm_noise_by_tick,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StabilityConfig {
    pub ticks: u32,
    pub api_capacity: u64,
    pub database_capacity: u64,
    pub critical_demand_per_tick: u64,
    pub bulk_demand_per_tick: u64,
    /// Fault is active on the half-open interval [start, end).
    pub fault_start_tick: u32,
    pub fault_end_tick: u32,
    pub fault_api_capacity_reduction: u64,
    pub observation_delay_ticks: u32,
    pub actuation_delay_ticks: u32,
    pub maximum_observation_age_ticks: u32,
    pub maximum_failed_ppm: u32,
    pub maximum_latency_ms: u64,
    pub maximum_critical_failed_ppm: u32,
    pub maximum_bulk_failed_ppm: u32,
    pub maximum_critical_latency_ms: u64,
    pub maximum_bulk_latency_ms: u64,
    pub consecutive_healthy_ticks: u32,
    /// Final ticks are reserved for recovery after all controller actions stop.
    pub no_intervention_tail_ticks: u32,
    pub base_latency_ms: u64,
    pub queue_latency_ms_per_request: u64,
    /// Deterministic signed queue-depth measurement noise, one value per tick.
    pub queue_noise_by_tick: Vec<i64>,
    /// Deterministic signed failure-rate measurement noise, one value per tick.
    pub failed_ppm_noise_by_tick: Vec<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Controller {
    Agent,
    Runbook,
    Autoscaler,
    Operator,
}

impl Controller {
    /// Evaluator-owned precedence for simultaneous conflicting controller
    /// proposals. This is a benchmark fixture policy, not a production policy.
    const fn precedence(self) -> u8 {
        match self {
            Self::Autoscaler => 1,
            Self::Agent => 2,
            Self::Runbook => 3,
            Self::Operator => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlOperation {
    SetApiCapacity(u64),
    SetRetryMultiplierPpm(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlAction {
    pub action_id: String,
    pub controller: Controller,
    pub issued_at_tick: u32,
    /// Source tick of the observation that informed this action.
    pub observation_source_tick: u32,
    pub operation: ControlOperation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ControlResource {
    ApiCapacity,
    RetryMultiplier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionDisposition {
    Applied,
    Overridden,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionOutcome {
    pub action_id: String,
    pub controller: Controller,
    pub resource: ControlResource,
    pub issued_at_tick: u32,
    pub effect_tick: u32,
    pub disposition: ActionDisposition,
    pub overridden_by: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlantSample {
    pub tick: u32,
    pub actual_api_capacity: u64,
    pub database_capacity: u64,
    pub effective_capacity: u64,
    pub critical_offered: u64,
    pub bulk_offered: u64,
    /// Newly offered critical attempts that missed their one-tick deadline.
    pub critical_failed: u64,
    /// Newly offered bulk/retry attempts that missed their one-tick deadline.
    pub bulk_failed: u64,
    pub total_failed: u64,
    pub total_attempts: u64,
    pub failed_ppm: u32,
    pub critical_failed_ppm: u32,
    pub bulk_failed_ppm: u32,
    pub critical_queue: u64,
    pub bulk_queue: u64,
    pub queue_depth: u64,
    pub latency_ms: u64,
    pub critical_latency_ms: u64,
    pub bulk_latency_ms: u64,
    pub critical_slo_healthy: bool,
    pub bulk_slo_healthy: bool,
    pub slo_healthy: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlantObservation {
    pub tick: u32,
    pub source_tick: u32,
    pub observed_queue_depth: u64,
    pub observed_latency_ms: u64,
    pub observed_failed_ppm: u32,
}

/// A controller policy receives only the current delayed/noisy observation.
/// It does not receive evaluator fault truth, plant state, oracle results, or
/// future samples. A production agent adapter must preserve this boundary.
pub trait StabilityController {
    /// Stable unique identity within one competing-controller run.
    fn controller_id(&self) -> &str;
    fn controller(&self) -> Controller;
    fn propose(&mut self, observation: &PlantObservation) -> Vec<ControlOperation>;
}

/// Simple zero-hysteresis autoscaler baseline. It switches between normal
/// and elevated API capacity at one failure-rate threshold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BangBangCapacityController {
    id: String,
    normal_capacity: u64,
    elevated_capacity: u64,
    threshold_ppm: u32,
    elevated: bool,
}

impl BangBangCapacityController {
    pub fn new(
        id: impl Into<String>,
        normal_capacity: u64,
        elevated_capacity: u64,
        threshold_ppm: u32,
    ) -> Result<Self, StabilityError> {
        let id = id.into();
        if id.is_empty() || normal_capacity == 0 || elevated_capacity == 0
            || elevated_capacity < normal_capacity || threshold_ppm > PPM as u32
        {
            return Err(StabilityError::InvalidControllerConfig);
        }
        Ok(Self { id, normal_capacity, elevated_capacity, threshold_ppm, elevated: false })
    }
}

impl StabilityController for BangBangCapacityController {
    fn controller_id(&self) -> &str { &self.id }
    fn controller(&self) -> Controller { Controller::Autoscaler }

    fn propose(&mut self, observation: &PlantObservation) -> Vec<ControlOperation> {
        if observation.observed_failed_ppm >= self.threshold_ppm && !self.elevated {
            self.elevated = true;
            vec![ControlOperation::SetApiCapacity(self.elevated_capacity)]
        } else if observation.observed_failed_ppm < self.threshold_ppm && self.elevated {
            self.elevated = false;
            vec![ControlOperation::SetApiCapacity(self.normal_capacity)]
        } else {
            Vec::new()
        }
    }
}

/// Deadband/hysteresis autoscaler baseline. Scaling up and scaling down use
/// separate thresholds, preventing small fluctuations from repeatedly
/// reversing an otherwise healthy configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HysteresisCapacityController {
    id: String,
    normal_capacity: u64,
    elevated_capacity: u64,
    scale_up_threshold_ppm: u32,
    scale_down_threshold_ppm: u32,
    elevated: bool,
}

impl HysteresisCapacityController {
    pub fn new(
        id: impl Into<String>,
        normal_capacity: u64,
        elevated_capacity: u64,
        scale_up_threshold_ppm: u32,
        scale_down_threshold_ppm: u32,
    ) -> Result<Self, StabilityError> {
        let id = id.into();
        if id.is_empty() || normal_capacity == 0 || elevated_capacity == 0
            || elevated_capacity < normal_capacity
            || scale_up_threshold_ppm > PPM as u32
            || scale_down_threshold_ppm >= scale_up_threshold_ppm
        {
            return Err(StabilityError::InvalidControllerConfig);
        }
        Ok(Self {
            id,
            normal_capacity,
            elevated_capacity,
            scale_up_threshold_ppm,
            scale_down_threshold_ppm,
            elevated: false,
        })
    }
}

impl StabilityController for HysteresisCapacityController {
    fn controller_id(&self) -> &str { &self.id }
    fn controller(&self) -> Controller { Controller::Autoscaler }

    fn propose(&mut self, observation: &PlantObservation) -> Vec<ControlOperation> {
        if observation.observed_failed_ppm >= self.scale_up_threshold_ppm && !self.elevated {
            self.elevated = true;
            vec![ControlOperation::SetApiCapacity(self.elevated_capacity)]
        } else if observation.observed_failed_ppm <= self.scale_down_threshold_ppm && self.elevated {
            self.elevated = false;
            vec![ControlOperation::SetApiCapacity(self.normal_capacity)]
        } else {
            Vec::new()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControllerDecision {
    pub controller_id: String,
    pub controller: Controller,
    pub decision_tick: u32,
    pub observation_source_tick: u32,
    pub observed_queue_depth: u64,
    pub observed_latency_ms: u64,
    pub observed_failed_ppm: u32,
    /// Exact delayed/noisy observation delivered to this controller decision.
    pub observation: PlantObservation,
    /// Complete proposed operations, preserved independently of the action list.
    pub proposed_actions: Vec<ControlAction>,
    pub proposed_action_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosedLoopRun {
    pub actions: Vec<ControlAction>,
    pub decisions: Vec<ControllerDecision>,
    pub stability: StabilityRun,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StabilityMetrics {
    pub slo_violation_integral_request_ms: u128,
    pub critical_slo_violation_integral_request_ms: u128,
    pub bulk_slo_violation_integral_request_ms: u128,
    pub settling_time_ticks_after_fault: Option<u32>,
    pub action_count: usize,
    pub action_churn_count: usize,
    pub action_reversal_count: usize,
    pub action_conflict_count: usize,
    pub overridden_action_count: usize,
    pub stale_observation_action_count: usize,
    pub critical_failed_requests: u64,
    pub bulk_failed_requests: u64,
    pub maximum_queue_depth: u64,
    pub maximum_latency_ms: u64,
    pub maximum_critical_latency_ms: u64,
    pub maximum_bulk_latency_ms: u64,
    pub queue_range_after_fault: u64,
    pub api_capacity_range_after_fault: u64,
    pub api_database_capacity_gap_request_units: u128,
    pub final_no_intervention_tail_healthy: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StabilityRun {
    pub samples: Vec<PlantSample>,
    pub observations: Vec<PlantObservation>,
    /// Deterministic winner/override outcomes for simultaneous controllers.
    pub action_outcomes: Vec<ActionOutcome>,
    pub metrics: StabilityMetrics,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StabilityError {
    EmptyRun,
    InvalidFaultInterval,
    InvalidCapacityOrDemand,
    InvalidSloContract,
    InvalidTailContract,
    NoiseLengthMismatch,
    EmptyActionIdentity,
    DuplicateAction(String),
    ActionOutsideRun(String),
    ActionInNoInterventionTail(String),
    ObservationFromFuture(String),
    ObservationNotYetAvailable(String),
    InvalidSetpoint(String),
    InvalidControllerConfig,
    EmptyControllerId,
    DuplicateControllerId(String),
    StreamingSamplesDiverged,
    StreamingObservationsDiverged,
    StreamingActionOutcomesDiverged,
    ClosedLoopEvidenceMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Resource {
    ApiCapacity,
    RetryMultiplier,
}

impl From<Resource> for ControlResource {
    fn from(value: Resource) -> Self {
        match value {
            Resource::ApiCapacity => Self::ApiCapacity,
            Resource::RetryMultiplier => Self::RetryMultiplier,
        }
    }
}

fn validate_config(config: &StabilityConfig) -> Result<(), StabilityError> {
    if config.ticks == 0 {
        return Err(StabilityError::EmptyRun);
    }
    if config.fault_start_tick >= config.fault_end_tick
        || config.fault_end_tick > config.ticks
    {
        return Err(StabilityError::InvalidFaultInterval);
    }
    if config.api_capacity == 0
        || config.database_capacity == 0
        || config.critical_demand_per_tick.saturating_add(config.bulk_demand_per_tick) == 0
        || config.fault_api_capacity_reduction >= config.api_capacity
    {
        return Err(StabilityError::InvalidCapacityOrDemand);
    }
    if config.maximum_failed_ppm > PPM as u32
        || config.maximum_critical_failed_ppm > PPM as u32
        || config.maximum_bulk_failed_ppm > PPM as u32
        || config.maximum_latency_ms == 0
        || config.maximum_critical_latency_ms == 0
        || config.maximum_bulk_latency_ms == 0
    {
        return Err(StabilityError::InvalidSloContract);
    }
    if config.no_intervention_tail_ticks == 0
        || config.no_intervention_tail_ticks > config.ticks
        || config.consecutive_healthy_ticks == 0
        || config.consecutive_healthy_ticks > config.no_intervention_tail_ticks
        || config.fault_end_tick > config.ticks - config.no_intervention_tail_ticks
    {
        return Err(StabilityError::InvalidTailContract);
    }
    if config.queue_noise_by_tick.len() != config.ticks as usize
        || config.failed_ppm_noise_by_tick.len() != config.ticks as usize
    {
        return Err(StabilityError::NoiseLengthMismatch);
    }
    Ok(())
}

/// Execute a deterministic closed-loop controller over the synthetic plant.
///
/// The controller is called once per actionable tick and receives only the
/// delayed/noisy observation available at that tick. The evaluator replays the
/// already-issued action prefix to produce the next observation; this O(T^2)
/// reference runner prioritizes a clear information boundary over throughput.
/// It is suitable for small conformance scenarios, not performance benchmarking.
///
/// No controller may issue an action whose delayed effect would occur in the
/// reserved no-intervention tail.
pub fn run_stability_closed_loop<C: StabilityController>(
    config: &StabilityConfig,
    controller: &mut C,
) -> Result<ClosedLoopRun, StabilityError> {
    let mut controllers: [&mut dyn StabilityController; 1] = [controller];
    run_stability_multi_controller_loop(config, &mut controllers)
}


#[derive(Clone, Debug)]
struct StreamingPlantState {
    api_setpoint: u64,
    retry_multiplier_ppm: u32,
    critical_queue: u64,
    bulk_queue: u64,
    previous_failed: u64,
}

impl StreamingPlantState {
    fn new(config: &StabilityConfig) -> Self {
        Self {
            api_setpoint: config.api_capacity,
            retry_multiplier_ppm: PPM as u32,
            critical_queue: 0,
            bulk_queue: 0,
            previous_failed: 0,
        }
    }
}

fn scheduled_effect_tick(issued_at_tick: u32, actuation_delay_ticks: u32) -> Option<u32> {
    issued_at_tick.checked_add(1)?.checked_add(actuation_delay_ticks)
}

fn validate_stream_action(
    config: &StabilityConfig,
    action: &ControlAction,
) -> Result<(u32, Resource, bool), StabilityError> {
    if action.action_id.is_empty() {
        return Err(StabilityError::EmptyActionIdentity);
    }
    if action.issued_at_tick >= config.ticks {
        return Err(StabilityError::ActionOutsideRun(action.action_id.clone()));
    }
    if action.observation_source_tick > action.issued_at_tick {
        return Err(StabilityError::ObservationFromFuture(action.action_id.clone()));
    }
    let observation_available = action.observation_source_tick == 0
        || action.observation_source_tick
            .checked_add(config.observation_delay_ticks)
            .is_some_and(|available_at| available_at <= action.issued_at_tick);
    if !observation_available {
        return Err(StabilityError::ObservationNotYetAvailable(action.action_id.clone()));
    }
    let stale = action.issued_at_tick - action.observation_source_tick
        > config.maximum_observation_age_ticks;
    let effect_tick = scheduled_effect_tick(action.issued_at_tick, config.actuation_delay_ticks)
        .ok_or_else(|| StabilityError::ActionOutsideRun(action.action_id.clone()))?;
    let tail_start = config.ticks - config.no_intervention_tail_ticks;
    if effect_tick >= config.ticks {
        return Err(StabilityError::ActionOutsideRun(action.action_id.clone()));
    }
    if effect_tick >= tail_start {
        return Err(StabilityError::ActionInNoInterventionTail(action.action_id.clone()));
    }
    let resource = match &action.operation {
        ControlOperation::SetApiCapacity(0) => {
            return Err(StabilityError::InvalidSetpoint(action.action_id.clone()));
        }
        ControlOperation::SetApiCapacity(_) => Resource::ApiCapacity,
        ControlOperation::SetRetryMultiplierPpm(value)
            if *value < PPM as u32 || *value > MAX_RETRY_MULTIPLIER_PPM =>
        {
            return Err(StabilityError::InvalidSetpoint(action.action_id.clone()));
        }
        ControlOperation::SetRetryMultiplierPpm(_) => Resource::RetryMultiplier,
    };
    Ok((effect_tick, resource, stale))
}

fn apply_stream_actions(
    scheduled: &mut BTreeMap<(u32, Resource), Vec<ControlAction>>,
    tick: u32,
    state: &mut StreamingPlantState,
    outcomes: &mut Vec<ActionOutcome>,
) {
    for resource in [Resource::ApiCapacity, Resource::RetryMultiplier] {
        if let Some(mut group) = scheduled.remove(&(tick, resource)) {
            group.sort_by(|a, b| {
                b.controller.precedence().cmp(&a.controller.precedence())
                    .then_with(|| a.action_id.cmp(&b.action_id))
            });
            let winner = &group[0];
            outcomes.push(ActionOutcome {
                action_id: winner.action_id.clone(),
                controller: winner.controller,
                resource: resource.into(),
                issued_at_tick: winner.issued_at_tick,
                effect_tick: tick,
                disposition: ActionDisposition::Applied,
                overridden_by: None,
            });
            for overridden in group.iter().skip(1) {
                outcomes.push(ActionOutcome {
                    action_id: overridden.action_id.clone(),
                    controller: overridden.controller,
                    resource: resource.into(),
                    issued_at_tick: overridden.issued_at_tick,
                    effect_tick: tick,
                    disposition: ActionDisposition::Overridden,
                    overridden_by: Some(winner.action_id.clone()),
                });
            }

            match &winner.operation {
                ControlOperation::SetApiCapacity(value) => {
                    state.api_setpoint = *value;
                }
                ControlOperation::SetRetryMultiplierPpm(value) => {
                    state.retry_multiplier_ppm = *value;
                }
            }
        }
    }
}

fn advance_streaming_plant_tick(
    config: &StabilityConfig,
    tick: u32,
    state: &mut StreamingPlantState,
) -> PlantSample {
    let fault_active = tick >= config.fault_start_tick && tick < config.fault_end_tick;
    let actual_api_capacity = if fault_active {
        state.api_setpoint.saturating_sub(config.fault_api_capacity_reduction).max(1)
    } else {
        state.api_setpoint
    };
    let effective_capacity = actual_api_capacity.min(config.database_capacity);
    let retry_extra = (state.previous_failed as u128)
        .saturating_mul((state.retry_multiplier_ppm - PPM as u32) as u128)
        .checked_div(PPM as u128)
        .unwrap_or(0)
        .min(u64::MAX as u128) as u64;
    let critical_attempts = config.critical_demand_per_tick;
    let bulk_attempts = config.bulk_demand_per_tick.saturating_add(retry_extra);
    let critical_work = state.critical_queue.saturating_add(critical_attempts);
    let bulk_work = state.bulk_queue.saturating_add(bulk_attempts);
    let critical_processed = critical_work.min(effective_capacity);
    let bulk_processed = bulk_work.min(effective_capacity.saturating_sub(critical_processed));
    let critical_completed_new = critical_processed
        .saturating_sub(state.critical_queue)
        .min(critical_attempts);
    let bulk_completed_new = bulk_processed
        .saturating_sub(state.bulk_queue)
        .min(bulk_attempts);

    state.critical_queue = critical_work.saturating_sub(critical_processed);
    state.bulk_queue = bulk_work.saturating_sub(bulk_processed);
    let critical_failed = critical_attempts.saturating_sub(critical_completed_new);
    let bulk_failed = bulk_attempts.saturating_sub(bulk_completed_new);
    let total_failed = critical_failed.saturating_add(bulk_failed);
    let total_attempts = critical_attempts.saturating_add(bulk_attempts);
    let failed_ppm = if total_attempts == 0 { 0 } else {
        ((total_failed as u128 * PPM as u128) / total_attempts as u128)
            .min(PPM as u128) as u32
    };
    let critical_failed_ppm = if critical_attempts == 0 { 0 } else {
        ((critical_failed as u128 * PPM as u128) / critical_attempts as u128)
            .min(PPM as u128) as u32
    };
    let bulk_failed_ppm = if bulk_attempts == 0 { 0 } else {
        ((bulk_failed as u128 * PPM as u128) / bulk_attempts as u128)
            .min(PPM as u128) as u32
    };
    let critical_latency_ms = config.base_latency_ms.saturating_add(
        state.critical_queue.saturating_mul(config.queue_latency_ms_per_request),
    );
    let queue_depth = state.critical_queue.saturating_add(state.bulk_queue);
    let bulk_latency_ms = config.base_latency_ms.saturating_add(
        queue_depth.saturating_mul(config.queue_latency_ms_per_request),
    );
    let latency_ms = bulk_latency_ms;
    let critical_slo_healthy = critical_failed_ppm <= config.maximum_critical_failed_ppm
        && critical_latency_ms <= config.maximum_critical_latency_ms;
    let bulk_slo_healthy = bulk_failed_ppm <= config.maximum_bulk_failed_ppm
        && bulk_latency_ms <= config.maximum_bulk_latency_ms;
    let slo_healthy = failed_ppm <= config.maximum_failed_ppm
        && latency_ms <= config.maximum_latency_ms
        && critical_slo_healthy
        && bulk_slo_healthy;
    state.previous_failed = total_failed;

    PlantSample {
        tick,
        actual_api_capacity,
        database_capacity: config.database_capacity,
        effective_capacity,
        critical_offered: critical_attempts,
        bulk_offered: bulk_attempts,
        critical_failed,
        bulk_failed,
        total_failed,
        total_attempts,
        failed_ppm,
        critical_failed_ppm,
        bulk_failed_ppm,
        critical_queue: state.critical_queue,
        bulk_queue: state.bulk_queue,
        queue_depth,
        latency_ms,
        critical_latency_ms,
        bulk_latency_ms,
        critical_slo_healthy,
        bulk_slo_healthy,
        slo_healthy,
    }
}

fn observation_for_tick(
    config: &StabilityConfig,
    tick: u32,
    samples: &[PlantSample],
) -> PlantObservation {
    let source_tick = tick.saturating_sub(config.observation_delay_ticks);
    let source = &samples[source_tick as usize];
    let noise = config.queue_noise_by_tick[tick as usize] as i128;
    PlantObservation {
        tick,
        source_tick,
        observed_queue_depth: (source.queue_depth as i128 + noise)
            .clamp(0, u64::MAX as i128) as u64,
        observed_latency_ms: source.latency_ms,
        observed_failed_ppm: (source.failed_ppm as i128
            + config.failed_ppm_noise_by_tick[tick as usize] as i128)
            .clamp(0, PPM as i128) as u32,
    }
}

/// Default policy runner uses the incremental implementation with replay
/// equivalence verification. The O(T²) prefix-replay function remains available
/// as an explicit independent orchestration reference.
pub fn run_stability_multi_controller_loop(
    config: &StabilityConfig,
    controllers: &mut [&mut dyn StabilityController],
) -> Result<ClosedLoopRun, StabilityError> {
    run_stability_multi_controller_loop_streaming(config, controllers)
}

/// Incremental O(T + A log A) controller runner. The plant advances exactly
/// once per tick, and each policy receives only the observation delivered at
/// that tick. Before returning, the complete result is replayed through the
/// reference evaluator; any divergence fails closed.
pub fn run_stability_multi_controller_loop_streaming(
    config: &StabilityConfig,
    controllers: &mut [&mut dyn StabilityController],
) -> Result<ClosedLoopRun, StabilityError> {
    validate_config(config)?;
    let tail_start = config.ticks - config.no_intervention_tail_ticks;
    let mut ids = BTreeSet::new();
    for controller in controllers.iter() {
        let id = controller.controller_id();
        if id.is_empty() {
            return Err(StabilityError::EmptyControllerId);
        }
        if !ids.insert(id.to_owned()) {
            return Err(StabilityError::DuplicateControllerId(id.to_owned()));
        }
    }
    controllers.sort_by(|a, b| a.controller_id().cmp(b.controller_id()));

    let mut state = StreamingPlantState::new(config);
    let mut scheduled: BTreeMap<(u32, Resource), Vec<ControlAction>> = BTreeMap::new();
    let mut actions = Vec::new();
    let mut decisions = Vec::new();
    let mut samples = Vec::with_capacity(config.ticks as usize);
    let mut outcomes = Vec::new();
    let mut action_ids = BTreeSet::new();

    for tick in 0..config.ticks {
        apply_stream_actions(&mut scheduled, tick, &mut state, &mut outcomes);
        samples.push(advance_streaming_plant_tick(config, tick, &mut state));
        let observation = observation_for_tick(config, tick, &samples);

        let can_act = scheduled_effect_tick(tick, config.actuation_delay_ticks)
            .is_some_and(|effect_tick| effect_tick < tail_start);
        if !can_act {
            continue;
        }

        for controller in controllers.iter_mut() {
            let controller_id = controller.controller_id().to_owned();
            let controller_kind = controller.controller();
            let proposals = controller.propose(&observation);
            let mut proposed_action_ids = Vec::with_capacity(proposals.len());
            let mut proposed_actions = Vec::with_capacity(proposals.len());
            for (index, operation) in proposals.into_iter().enumerate() {
                let action = ControlAction {
                    action_id: format!("closed-loop-{controller_id}-{tick}-{index}"),
                    controller: controller_kind,
                    issued_at_tick: tick,
                    observation_source_tick: observation.source_tick,
                    operation,
                };
                if !action_ids.insert(action.action_id.clone()) {
                    return Err(StabilityError::DuplicateAction(action.action_id));
                }
                let (effect_tick, resource, _stale) = validate_stream_action(config, &action)?;
                proposed_action_ids.push(action.action_id.clone());
                scheduled.entry((effect_tick, resource)).or_default().push(action.clone());
                actions.push(action.clone());
                proposed_actions.push(action);
            }
            decisions.push(ControllerDecision {
                controller_id,
                controller: controller_kind,
                decision_tick: tick,
                observation_source_tick: observation.source_tick,
                observed_queue_depth: observation.observed_queue_depth,
                observed_latency_ms: observation.observed_latency_ms,
                observed_failed_ppm: observation.observed_failed_ppm,
                observation: observation.clone(),
                proposed_actions,
                proposed_action_ids,
            });
        }
    }

    let replay = run_stability_scenario(config, &actions)?;
    let streamed_observations: Vec<_> = (0..config.ticks)
        .map(|tick| observation_for_tick(config, tick, &samples))
        .collect();
    if samples != replay.samples {
        return Err(StabilityError::StreamingSamplesDiverged);
    }
    if streamed_observations != replay.observations {
        return Err(StabilityError::StreamingObservationsDiverged);
    }
    if outcomes != replay.action_outcomes {
        return Err(StabilityError::StreamingActionOutcomesDiverged);
    }

    let result = ClosedLoopRun {
        actions,
        decisions,
        stability: replay,
    };
    validate_closed_loop_evidence(config, &result)?;
    Ok(result)
}

pub fn run_stability_multi_controller_loop_reference(
    config: &StabilityConfig,
    controllers: &mut [&mut dyn StabilityController],
) -> Result<ClosedLoopRun, StabilityError> {
    validate_config(config)?;
    let tail_start = config.ticks - config.no_intervention_tail_ticks;

    let mut controller_ids = BTreeSet::new();
    for controller in controllers.iter() {
        let id = controller.controller_id();
        if id.is_empty() {
            return Err(StabilityError::EmptyControllerId);
        }
        if !controller_ids.insert(id.to_owned()) {
            return Err(StabilityError::DuplicateControllerId(id.to_owned()));
        }
    }
    controllers.sort_by(|a, b| a.controller_id().cmp(b.controller_id()));

    let mut actions: Vec<ControlAction> = Vec::new();
    let mut decisions: Vec<ControllerDecision> = Vec::new();

    for tick in 0..config.ticks {
        let partial_run = run_stability_scenario(config, &actions)?;
        let observation = partial_run.observations[tick as usize].clone();

        let effect_tick = scheduled_effect_tick(tick, config.actuation_delay_ticks);
        let can_act = effect_tick.is_some_and(|due| due < tail_start);
        if !can_act {
            continue;
        }

        for controller in controllers.iter_mut() {
            let controller_id = controller.controller_id().to_owned();
            let controller_kind = controller.controller();
            let proposals = controller.propose(&observation);
            let mut proposed_action_ids = Vec::with_capacity(proposals.len());
            let mut proposed_actions = Vec::with_capacity(proposals.len());
            for (index, operation) in proposals.into_iter().enumerate() {
                let action_id = format!("closed-loop-{controller_id}-{tick}-{index}");
                proposed_action_ids.push(action_id.clone());
                let action = ControlAction {
                    action_id,
                    controller: controller_kind,
                    issued_at_tick: tick,
                    observation_source_tick: observation.source_tick,
                    operation,
                };
                proposed_actions.push(action.clone());
                actions.push(action);
            }
            decisions.push(ControllerDecision {
                controller_id,
                controller: controller_kind,
                decision_tick: tick,
                observation_source_tick: observation.source_tick,
                observed_queue_depth: observation.observed_queue_depth,
                observed_latency_ms: observation.observed_latency_ms,
                observed_failed_ppm: observation.observed_failed_ppm,
                observation: observation.clone(),
                proposed_actions,
                proposed_action_ids,
            });
        }
    }

    let stability = run_stability_scenario(config, &actions)?;
    let result = ClosedLoopRun {
        actions,
        decisions,
        stability,
    };
    validate_closed_loop_evidence(config, &result)?;
    Ok(result)
}

/// Independently verify that each controller decision is bound to the exact
/// observation delivered at that tick, that its action list is complete, and
/// that the final scored trace is exactly the concatenation of those proposals.
/// This rejects post-hoc action substitution, invented observation values,
/// missing action rows, and orphan actions.
pub fn validate_closed_loop_evidence(
    config: &StabilityConfig,
    run: &ClosedLoopRun,
) -> Result<(), StabilityError> {
    validate_config(config)?;
    if run.stability.samples.len() != config.ticks as usize
        || run.stability.observations.len() != config.ticks as usize
    {
        return Err(StabilityError::ClosedLoopEvidenceMismatch);
    }

    let tail_start = config.ticks - config.no_intervention_tail_ticks;
    let mut decision_keys = BTreeSet::new();
    let mut action_ids = BTreeSet::new();
    let mut flattened_actions = Vec::new();

    for decision in &run.decisions {
        if decision.controller_id.is_empty()
            || decision.decision_tick >= config.ticks
            || !decision_keys.insert((decision.controller_id.as_str(), decision.decision_tick))
        {
            return Err(StabilityError::ClosedLoopEvidenceMismatch);
        }
        if !scheduled_effect_tick(decision.decision_tick, config.actuation_delay_ticks)
            .is_some_and(|effect_tick| effect_tick < tail_start)
        {
            return Err(StabilityError::ClosedLoopEvidenceMismatch);
        }

        let observation = &decision.observation;
        let replayed_observation = run.stability.observations
            .get(decision.decision_tick as usize)
            .ok_or(StabilityError::ClosedLoopEvidenceMismatch)?;

        if observation != replayed_observation
            || observation.tick != decision.decision_tick
            || observation.source_tick != decision.observation_source_tick
            || observation.source_tick
                != decision.decision_tick.saturating_sub(config.observation_delay_ticks)
            || observation.observed_queue_depth != decision.observed_queue_depth
            || observation.observed_latency_ms != decision.observed_latency_ms
            || observation.observed_failed_ppm != decision.observed_failed_ppm
        {
            return Err(StabilityError::ClosedLoopEvidenceMismatch);
        }

        if decision.proposed_action_ids.len() != decision.proposed_actions.len() {
            return Err(StabilityError::ClosedLoopEvidenceMismatch);
        }
        for (index, action) in decision.proposed_actions.iter().enumerate() {
            if decision.proposed_action_ids[index] != action.action_id
                || action.controller != decision.controller
                || action.issued_at_tick != decision.decision_tick
                || action.observation_source_tick != observation.source_tick
                || !action_ids.insert(action.action_id.as_str())
            {
                return Err(StabilityError::ClosedLoopEvidenceMismatch);
            }
            // Preserve the same time, scope, setpoint and tail checks used by
            // the streaming policy boundary.
            validate_stream_action(config, action)?;
            flattened_actions.push(action.clone());
        }
    }

    if flattened_actions != run.actions {
        return Err(StabilityError::ClosedLoopEvidenceMismatch);
    }

    // Recompute the scorer output from the trace. This binds the closed-loop
    // receipt to the same independent evaluator used for offline action arms.
    let replay = run_stability_scenario(config, &run.actions)?;
    if replay != run.stability {
        return Err(StabilityError::ClosedLoopEvidenceMismatch);
    }
    Ok(())
}

/// Run a deterministic plant against an externally supplied controller action
/// trace. The caller is responsible for generating actions only from permitted
/// observations; observation source ticks remain visible for audit.
pub fn run_stability_scenario(
    config: &StabilityConfig,
    actions: &[ControlAction],
) -> Result<StabilityRun, StabilityError> {
    validate_config(config)?;
    let tail_start = config.ticks - config.no_intervention_tail_ticks;

    let mut action_ids = BTreeSet::new();
    let mut scheduled: BTreeMap<(u32, Resource), Vec<&ControlAction>> = BTreeMap::new();
    let mut stale_observation_action_count = 0usize;
    for action in actions {
        if action.action_id.is_empty() {
            return Err(StabilityError::EmptyActionIdentity);
        }
        if !action_ids.insert(action.action_id.as_str()) {
            return Err(StabilityError::DuplicateAction(action.action_id.clone()));
        }
        if action.issued_at_tick >= config.ticks {
            return Err(StabilityError::ActionOutsideRun(action.action_id.clone()));
        }
        if action.observation_source_tick > action.issued_at_tick {
            return Err(StabilityError::ObservationFromFuture(action.action_id.clone()));
        }
        // Tick zero is the initial observation and is available from run start.
        // Later source ticks become observable only after the configured delay.
        let observation_available = action.observation_source_tick == 0
            || action.observation_source_tick
                .checked_add(config.observation_delay_ticks)
                .is_some_and(|available_at| available_at <= action.issued_at_tick);
        if !observation_available {
            return Err(StabilityError::ObservationNotYetAvailable(action.action_id.clone()));
        }
        if action.issued_at_tick - action.observation_source_tick
            > config.maximum_observation_age_ticks
        {
            stale_observation_action_count += 1;
        }
        let effect_tick = scheduled_effect_tick(action.issued_at_tick, config.actuation_delay_ticks)
            .ok_or_else(|| StabilityError::ActionOutsideRun(action.action_id.clone()))?;
        if effect_tick >= config.ticks {
            return Err(StabilityError::ActionOutsideRun(action.action_id.clone()));
        }
        if effect_tick >= tail_start {
            return Err(StabilityError::ActionInNoInterventionTail(action.action_id.clone()));
        }
        match &action.operation {
            ControlOperation::SetApiCapacity(0) => {
                return Err(StabilityError::InvalidSetpoint(action.action_id.clone()));
            }
            ControlOperation::SetRetryMultiplierPpm(value)
                if *value < PPM as u32 || *value > MAX_RETRY_MULTIPLIER_PPM =>
            {
                return Err(StabilityError::InvalidSetpoint(action.action_id.clone()));
            }
            _ => {}
        }
        let resource = match &action.operation {
            ControlOperation::SetApiCapacity(_) => Resource::ApiCapacity,
            ControlOperation::SetRetryMultiplierPpm(_) => Resource::RetryMultiplier,
        };
        scheduled.entry((effect_tick, resource)).or_default().push(action);
    }
    let mut api_setpoint = config.api_capacity;
    let mut retry_multiplier_ppm = PPM as u32;
    let mut previous_direction: BTreeMap<Resource, i8> = BTreeMap::new();
    let mut critical_queue = 0u64;
    let mut bulk_queue = 0u64;
    let mut previous_failed = 0u64;
    let mut metrics = StabilityMetrics {
        action_count: actions.len(),
        stale_observation_action_count,
        ..StabilityMetrics::default()
    };
    let mut samples = Vec::with_capacity(config.ticks as usize);
    let mut action_outcomes = Vec::with_capacity(actions.len());
    let mut tail_capacity_min = u64::MAX;
    let mut tail_capacity_max = 0u64;

    for tick in 0..config.ticks {
        for resource in [Resource::ApiCapacity, Resource::RetryMultiplier] {
            if let Some(mut group) = scheduled.remove(&(tick, resource)) {
                // Highest evaluator-owned controller precedence wins. For equal
                // precedence, action ID gives a stable deterministic tie-break,
                // independent of input-vector ordering.
                group.sort_by(|a, b| {
                    b.controller.precedence().cmp(&a.controller.precedence())
                        .then_with(|| a.action_id.cmp(&b.action_id))
                });
                let winner = group[0];
                if group.len() > 1 {
                    metrics.action_conflict_count += group.len() - 1;
                    metrics.overridden_action_count += group.len() - 1;
                }
                action_outcomes.push(ActionOutcome {
                    action_id: winner.action_id.clone(),
                    controller: winner.controller,
                    resource: resource.into(),
                    issued_at_tick: winner.issued_at_tick,
                    effect_tick: tick,
                    disposition: ActionDisposition::Applied,
                    overridden_by: None,
                });
                for overridden in group.iter().skip(1) {
                    action_outcomes.push(ActionOutcome {
                        action_id: overridden.action_id.clone(),
                        controller: overridden.controller,
                        resource: resource.into(),
                        issued_at_tick: overridden.issued_at_tick,
                        effect_tick: tick,
                        disposition: ActionDisposition::Overridden,
                        overridden_by: Some(winner.action_id.clone()),
                    });
                }

                match &winner.operation {
                    ControlOperation::SetApiCapacity(value) => {
                        if *value != api_setpoint {
                            metrics.action_churn_count += 1;
                            let direction = if *value > api_setpoint { 1 } else { -1 };
                            if previous_direction.get(&Resource::ApiCapacity)
                                .is_some_and(|previous| *previous != direction)
                            {
                                metrics.action_reversal_count += 1;
                            }
                            previous_direction.insert(Resource::ApiCapacity, direction);
                            api_setpoint = *value;
                        }
                    }
                    ControlOperation::SetRetryMultiplierPpm(value) => {
                        if *value != retry_multiplier_ppm {
                            metrics.action_churn_count += 1;
                            let direction = if *value > retry_multiplier_ppm { 1 } else { -1 };
                            if previous_direction.get(&Resource::RetryMultiplier)
                                .is_some_and(|previous| *previous != direction)
                            {
                                metrics.action_reversal_count += 1;
                            }
                            previous_direction.insert(Resource::RetryMultiplier, direction);
                            retry_multiplier_ppm = *value;
                        }
                    }
                }
            }
        }

        let fault_active = tick >= config.fault_start_tick && tick < config.fault_end_tick;
        let actual_api_capacity = if fault_active {
            api_setpoint.saturating_sub(config.fault_api_capacity_reduction).max(1)
        } else {
            api_setpoint
        };
        let effective_capacity = actual_api_capacity.min(config.database_capacity);
        let retry_extra = (previous_failed as u128)
            .saturating_mul((retry_multiplier_ppm - PPM as u32) as u128)
            .checked_div(PPM as u128)
            .unwrap_or(0)
            .min(u64::MAX as u128) as u64;
        let critical_attempts = config.critical_demand_per_tick;
        let bulk_attempts = config.bulk_demand_per_tick.saturating_add(retry_extra);

        // Critical work receives priority. Each class has its own backlog so
        // bulk retries cannot silently consume capacity reserved for critical work.
        // The backlog is residual server-side work after client timeout; a retry
        // is a new offered attempt and does not erase the original queued work.
        let critical_work = critical_queue.saturating_add(critical_attempts);
        let bulk_work = bulk_queue.saturating_add(bulk_attempts);
        let critical_processed = critical_work.min(effective_capacity);
        let bulk_processed = bulk_work.min(effective_capacity.saturating_sub(critical_processed));
        let critical_completed_new = critical_processed
            .saturating_sub(critical_queue)
            .min(critical_attempts);
        let bulk_completed_new = bulk_processed
            .saturating_sub(bulk_queue)
            .min(bulk_attempts);

        critical_queue = critical_work.saturating_sub(critical_processed);
        bulk_queue = bulk_work.saturating_sub(bulk_processed);
        let critical_failed = critical_attempts.saturating_sub(critical_completed_new);
        let bulk_failed = bulk_attempts.saturating_sub(bulk_completed_new);
        let total_failed = critical_failed.saturating_add(bulk_failed);
        let total_attempts = critical_attempts.saturating_add(bulk_attempts);
        let failed_ppm = if total_attempts == 0 {
            0
        } else {
            ((total_failed as u128 * PPM as u128) / total_attempts as u128)
                .min(PPM as u128) as u32
        };
        let critical_failed_ppm = if critical_attempts == 0 {
            0
        } else {
            ((critical_failed as u128 * PPM as u128) / critical_attempts as u128)
                .min(PPM as u128) as u32
        };
        let bulk_failed_ppm = if bulk_attempts == 0 {
            0
        } else {
            ((bulk_failed as u128 * PPM as u128) / bulk_attempts as u128)
                .min(PPM as u128) as u32
        };
        let queue_depth = critical_queue.saturating_add(bulk_queue);
        let critical_latency_ms = config.base_latency_ms.saturating_add(
            critical_queue.saturating_mul(config.queue_latency_ms_per_request),
        );
        let bulk_latency_ms = config.base_latency_ms.saturating_add(
            queue_depth.saturating_mul(config.queue_latency_ms_per_request),
        );
        let latency_ms = bulk_latency_ms;
        let critical_slo_healthy = critical_failed_ppm <= config.maximum_critical_failed_ppm
            && critical_latency_ms <= config.maximum_critical_latency_ms;
        let bulk_slo_healthy = bulk_failed_ppm <= config.maximum_bulk_failed_ppm
            && bulk_latency_ms <= config.maximum_bulk_latency_ms;
        let slo_healthy = failed_ppm <= config.maximum_failed_ppm
            && latency_ms <= config.maximum_latency_ms
            && critical_slo_healthy
            && bulk_slo_healthy;

        metrics.critical_failed_requests = metrics.critical_failed_requests
            .saturating_add(critical_failed);
        metrics.bulk_failed_requests = metrics.bulk_failed_requests
            .saturating_add(bulk_failed);
        metrics.maximum_queue_depth = metrics.maximum_queue_depth.max(queue_depth);
        metrics.maximum_latency_ms = metrics.maximum_latency_ms.max(latency_ms);
        metrics.maximum_critical_latency_ms =
            metrics.maximum_critical_latency_ms.max(critical_latency_ms);
        metrics.maximum_bulk_latency_ms = metrics.maximum_bulk_latency_ms.max(bulk_latency_ms);

        let critical_failed_request_time =
            (critical_failed as u128).saturating_mul(critical_latency_ms as u128);
        let critical_slow_request_time = if critical_latency_ms > config.maximum_critical_latency_ms {
            (critical_completed_new as u128)
                .saturating_mul((critical_latency_ms - config.maximum_critical_latency_ms) as u128)
        } else {
            0
        };
        let bulk_failed_request_time = (bulk_failed as u128).saturating_mul(bulk_latency_ms as u128);
        let bulk_slow_request_time = if bulk_latency_ms > config.maximum_bulk_latency_ms {
            (bulk_completed_new as u128)
                .saturating_mul((bulk_latency_ms - config.maximum_bulk_latency_ms) as u128)
        } else {
            0
        };
        metrics.critical_slo_violation_integral_request_ms = metrics
            .critical_slo_violation_integral_request_ms
            .saturating_add(critical_failed_request_time)
            .saturating_add(critical_slow_request_time);
        metrics.bulk_slo_violation_integral_request_ms = metrics
            .bulk_slo_violation_integral_request_ms
            .saturating_add(bulk_failed_request_time)
            .saturating_add(bulk_slow_request_time);
        metrics.slo_violation_integral_request_ms =
            metrics.critical_slo_violation_integral_request_ms
                .saturating_add(metrics.bulk_slo_violation_integral_request_ms);
        let api_excess = actual_api_capacity.saturating_sub(effective_capacity);
        metrics.api_database_capacity_gap_request_units = metrics
            .api_database_capacity_gap_request_units
            .saturating_add(api_excess as u128);

        if tick >= config.fault_end_tick {
            tail_capacity_min = tail_capacity_min.min(actual_api_capacity);
            tail_capacity_max = tail_capacity_max.max(actual_api_capacity);
        }

        previous_failed = total_failed;
        samples.push(PlantSample {
            tick,
            actual_api_capacity,
            database_capacity: config.database_capacity,
            effective_capacity,
            critical_offered: critical_attempts,
            bulk_offered: bulk_attempts,
            critical_failed,
            bulk_failed,
            total_failed,
            total_attempts,
            failed_ppm,
            critical_failed_ppm,
            bulk_failed_ppm,
            critical_queue,
            bulk_queue,
            queue_depth,
            latency_ms,
            critical_latency_ms,
            bulk_latency_ms,
            critical_slo_healthy,
            bulk_slo_healthy,
            slo_healthy,
        });
    }

    // Settlement is the first post-fault tick after which the service stays
    // healthy for the remainder of this run, not merely the first transient
    // healthy streak. This prevents recovery/re-degradation from looking settled.
    metrics.settling_time_ticks_after_fault = None;
    let post_fault = &samples[config.fault_end_tick as usize..];
    for (offset, _) in post_fault.iter().enumerate() {
        let suffix = &post_fault[offset..];
        if suffix.len() >= config.consecutive_healthy_ticks as usize
            && suffix.iter().all(|sample| sample.slo_healthy)
        {
            metrics.settling_time_ticks_after_fault = Some(offset as u32);
            break;
        }
    }

    metrics.api_capacity_range_after_fault = tail_capacity_max.saturating_sub(tail_capacity_min);
    metrics.queue_range_after_fault = if samples.len() > config.fault_end_tick as usize {
        let tail = &samples[config.fault_end_tick as usize..];
        let min = tail.iter().map(|sample| sample.queue_depth).min().unwrap_or(0);
        let max = tail.iter().map(|sample| sample.queue_depth).max().unwrap_or(0);
        max.saturating_sub(min)
    } else {
        0
    };
    let final_tail = &samples[tail_start as usize..];
    metrics.final_no_intervention_tail_healthy =
        final_tail.iter().all(|sample| sample.slo_healthy);
    if tail_capacity_min == u64::MAX {
        metrics.api_capacity_range_after_fault = 0;
    }

    let observations = samples.iter().enumerate().map(|(tick, _)| {
        let source_tick = tick.saturating_sub(config.observation_delay_ticks as usize);
        let source = &samples[source_tick];
        let noise = config.queue_noise_by_tick[tick] as i128;
        let noisy_queue = (source.queue_depth as i128 + noise)
            .clamp(0, u64::MAX as i128) as u64;
        PlantObservation {
            tick: tick as u32,
            source_tick: source_tick as u32,
            observed_queue_depth: noisy_queue,
            observed_latency_ms: source.latency_ms,
            observed_failed_ppm: (source.failed_ppm as i128
                + config.failed_ppm_noise_by_tick[tick] as i128)
                .clamp(0, PPM as i128) as u32,
        }
    }).collect();

    Ok(StabilityRun { samples, observations, action_outcomes, metrics })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> StabilityConfig {
        StabilityConfig {
            ticks: 16,
            api_capacity: 100,
            database_capacity: 100,
            critical_demand_per_tick: 20,
            bulk_demand_per_tick: 60,
            fault_start_tick: 2,
            fault_end_tick: 5,
            fault_api_capacity_reduction: 50,
            observation_delay_ticks: 1,
            actuation_delay_ticks: 1,
            maximum_observation_age_ticks: 2,
            maximum_failed_ppm: 100_000,
            maximum_latency_ms: 200,
            maximum_critical_failed_ppm: 100_000,
            maximum_bulk_failed_ppm: 200_000,
            maximum_critical_latency_ms: 100,
            maximum_bulk_latency_ms: 200,
            consecutive_healthy_ticks: 2,
            no_intervention_tail_ticks: 4,
            base_latency_ms: 10,
            queue_latency_ms_per_request: 1,
            queue_noise_by_tick: vec![0; 16],
            failed_ppm_noise_by_tick: vec![0; 16],
        }
    }

    struct OneShotReactiveScaleUp {
        id: &'static str,
        kind: Controller,
        issued: bool,
        calls: Vec<(u32, u64, u32)>,
        threshold_ppm: u32,
        capacity: u64,
    }

    impl StabilityController for OneShotReactiveScaleUp {
        fn controller_id(&self) -> &str {
            self.id
        }

        fn controller(&self) -> Controller {
            self.kind
        }

        fn propose(&mut self, observation: &PlantObservation) -> Vec<ControlOperation> {
            self.calls.push((
                observation.source_tick,
                observation.observed_queue_depth,
                observation.observed_failed_ppm,
            ));
            if !self.issued && observation.observed_failed_ppm >= self.threshold_ppm {
                self.issued = true;
                vec![ControlOperation::SetApiCapacity(self.capacity)]
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn controller_loop_acts_only_on_delivered_observations_and_recovers() {
        let mut config = config();
        config.actuation_delay_ticks = 0;
        let baseline = run_stability_scenario(&config, &[]).unwrap();
        let mut controller = OneShotReactiveScaleUp {
            id: "reactive-agent",
            kind: Controller::Agent,
            issued: false,
            calls: Vec::new(),
            threshold_ppm: 300_000,
            capacity: 140,
        };
        let run = run_stability_closed_loop(&config, &mut controller).unwrap();

        assert_eq!(run.actions.len(), 1);
        assert_eq!(run.actions[0].issued_at_tick, 3);
        assert_eq!(run.actions[0].observation_source_tick, 2);
        assert_eq!(run.decisions[3].decision_tick, 3);
        assert_eq!(run.decisions[3].observation_source_tick, 2);
        assert_eq!(run.decisions[3].controller_id, "reactive-agent");
        assert_eq!(run.decisions[3].proposed_action_ids, vec!["closed-loop-reactive-agent-3-0"]);
        assert!(run.stability.metrics.slo_violation_integral_request_ms
            < baseline.metrics.slo_violation_integral_request_ms);
        assert!(run.stability.metrics.final_no_intervention_tail_healthy);
        assert!(controller.calls.iter().all(|(source_tick, _, _)| *source_tick <= 14));
        assert_eq!(run.stability.action_outcomes.len(), 1);
        assert_eq!(run.stability.action_outcomes[0].disposition, ActionDisposition::Applied);
    }

    #[test]
    fn closed_loop_controller_is_never_invoked_for_the_no_intervention_tail() {
        let mut config = config();
        config.actuation_delay_ticks = 1;
        let mut controller = OneShotReactiveScaleUp {
            id: "inactive-agent",
            kind: Controller::Agent,
            issued: false,
            calls: Vec::new(),
            threshold_ppm: u32::MAX,
            capacity: 140,
        };
        let run = run_stability_closed_loop(&config, &mut controller).unwrap();
        let tail_start = config.ticks - config.no_intervention_tail_ticks;
        assert!(run.decisions.iter().all(|decision| {
            decision.decision_tick + config.actuation_delay_ticks + 1 < tail_start
        }));
        assert!(run.actions.iter().all(|action| {
            action.issued_at_tick + config.actuation_delay_ticks + 1 < tail_start
        }));
    }

    #[test]
    fn duplicate_controller_ids_are_rejected_before_any_policy_invocation() {
        let config = config();
        let mut first = OneShotReactiveScaleUp {
            id: "duplicate",
            kind: Controller::Agent,
            issued: false,
            calls: Vec::new(),
            threshold_ppm: 300_000,
            capacity: 140,
        };
        let mut second = OneShotReactiveScaleUp {
            id: "duplicate",
            kind: Controller::Autoscaler,
            issued: false,
            calls: Vec::new(),
            threshold_ppm: 300_000,
            capacity: 120,
        };
        let mut controllers: [&mut dyn StabilityController; 2] = [&mut first, &mut second];
        assert_eq!(
            run_stability_multi_controller_loop(&config, &mut controllers),
            Err(StabilityError::DuplicateControllerId("duplicate".into()))
        );
        assert!(first.calls.is_empty());
        assert!(second.calls.is_empty());
    }

    #[test]
    fn empty_controller_id_is_rejected_before_any_policy_invocation() {
        let config = config();
        let mut controller = OneShotReactiveScaleUp {
            id: "",
            kind: Controller::Agent,
            issued: false,
            calls: Vec::new(),
            threshold_ppm: 300_000,
            capacity: 140,
        };
        let mut controllers: [&mut dyn StabilityController; 1] = [&mut controller];
        assert_eq!(
            run_stability_multi_controller_loop(&config, &mut controllers),
            Err(StabilityError::EmptyControllerId)
        );
        assert!(controller.calls.is_empty());
    }

    #[test]
    fn incremental_multi_controller_runner_matches_prefix_replay_reference() {
        let mut config = config();
        config.observation_delay_ticks = 2;
        config.actuation_delay_ticks = 2;
        config.queue_noise_by_tick[6] = 3;
        config.failed_ppm_noise_by_tick[7] = 20_000;

        let mut streaming_agent = OneShotReactiveScaleUp {
            id: "agent",
            kind: Controller::Agent,
            issued: false,
            calls: Vec::new(),
            threshold_ppm: 250_000,
            capacity: 145,
        };
        let mut streaming_auto = OneShotReactiveScaleUp {
            id: "autoscaler",
            kind: Controller::Autoscaler,
            issued: false,
            calls: Vec::new(),
            threshold_ppm: 250_000,
            capacity: 130,
        };
        let mut streaming_policies: [&mut dyn StabilityController; 2] =
            [&mut streaming_agent, &mut streaming_auto];
        let streamed = run_stability_multi_controller_loop_streaming(
            &config, &mut streaming_policies
        ).unwrap();

        let mut reference_agent = OneShotReactiveScaleUp {
            id: "agent",
            kind: Controller::Agent,
            issued: false,
            calls: Vec::new(),
            threshold_ppm: 250_000,
            capacity: 145,
        };
        let mut reference_auto = OneShotReactiveScaleUp {
            id: "autoscaler",
            kind: Controller::Autoscaler,
            issued: false,
            calls: Vec::new(),
            threshold_ppm: 250_000,
            capacity: 130,
        };
        let mut reference_policies: [&mut dyn StabilityController; 2] =
            [&mut reference_auto, &mut reference_agent];
        let replayed = run_stability_multi_controller_loop_reference(
            &config, &mut reference_policies
        ).unwrap();

        assert_eq!(streamed.actions, replayed.actions);
        assert_eq!(streamed.decisions, replayed.decisions);
        assert_eq!(streamed.stability, replayed.stability);
    }

    #[test]
    fn competing_closed_loop_controllers_are_input_order_independent() {
        fn make_agent() -> OneShotReactiveScaleUp {
            OneShotReactiveScaleUp {
                id: "agent",
                kind: Controller::Agent,
                issued: false,
                calls: Vec::new(),
                threshold_ppm: 300_000,
                capacity: 140,
            }
        }
        fn make_autoscaler() -> OneShotReactiveScaleUp {
            OneShotReactiveScaleUp {
                id: "autoscaler",
                kind: Controller::Autoscaler,
                issued: false,
                calls: Vec::new(),
                threshold_ppm: 300_000,
                capacity: 120,
            }
        }

        let config = config();
        let mut agent_a = make_agent();
        let mut autoscaler_a = make_autoscaler();
        let mut forward: [&mut dyn StabilityController; 2] =
            [&mut agent_a, &mut autoscaler_a];
        let forward_run = run_stability_multi_controller_loop(&config, &mut forward).unwrap();

        let mut agent_b = make_agent();
        let mut autoscaler_b = make_autoscaler();
        let mut reverse: [&mut dyn StabilityController; 2] =
            [&mut autoscaler_b, &mut agent_b];
        let reverse_run = run_stability_multi_controller_loop(&config, &mut reverse).unwrap();

        assert_eq!(forward_run.actions, reverse_run.actions);
        assert_eq!(forward_run.decisions, reverse_run.decisions);
        assert_eq!(forward_run.stability.samples, reverse_run.stability.samples);
        assert_eq!(forward_run.stability.action_outcomes, reverse_run.stability.action_outcomes);
        assert_eq!(forward_run.stability.metrics.action_conflict_count, 1);
        assert_eq!(forward_run.stability.metrics.overridden_action_count, 1);
        let applied = forward_run.stability.action_outcomes.iter()
            .find(|outcome| outcome.disposition == ActionDisposition::Applied)
            .unwrap();
        assert_eq!(applied.controller, Controller::Agent);
        assert_eq!(forward_run.stability.samples[5].actual_api_capacity, 140);
    }

    #[test]
    fn hysteresis_reduces_noise_induced_action_churn_without_harming_service() {
        let mut config = config();
        config.fault_api_capacity_reduction = 10;
        config.observation_delay_ticks = 0;
        config.actuation_delay_ticks = 0;
        for tick in 2..12 {
            config.failed_ppm_noise_by_tick[tick] = if tick % 2 == 0 { 400_000 } else { 200_000 };
        }

        let mut bang_bang = BangBangCapacityController::new("bang-bang", 100, 140, 300_000).unwrap();
        let mut hysteresis =
            HysteresisCapacityController::new("hysteresis", 100, 140, 350_000, 150_000).unwrap();

        let noisy_bang = run_stability_closed_loop(&config, &mut bang_bang).unwrap();
        let noisy_hysteresis = run_stability_closed_loop(&config, &mut hysteresis).unwrap();

        assert!(noisy_bang.stability.metrics.action_churn_count >= 9);
        assert_eq!(noisy_hysteresis.stability.metrics.action_churn_count, 1);
        assert_eq!(
            noisy_bang.stability.metrics.slo_violation_integral_request_ms,
            noisy_hysteresis.stability.metrics.slo_violation_integral_request_ms
        );
        assert_eq!(noisy_bang.stability.metrics.slo_violation_integral_request_ms, 0);
        assert!(noisy_bang.stability.metrics.final_no_intervention_tail_healthy);
        assert!(noisy_hysteresis.stability.metrics.final_no_intervention_tail_healthy);
    }

    #[test]
    fn seeded_noise_v1_is_reproducible_and_paired() {
        let first = generate_deterministic_noise_v1(0x5eed, 64, 12, 50_000).unwrap();
        let replay = generate_deterministic_noise_v1(0x5eed, 64, 12, 50_000).unwrap();
        let other_seed = generate_deterministic_noise_v1(0x5eee, 64, 12, 50_000).unwrap();
        assert_eq!(first, replay);
        assert_eq!(first.algorithm, NOISE_GENERATOR_V1);
        assert_ne!(first.queue_noise_by_tick, other_seed.queue_noise_by_tick);
        assert_ne!(first.failed_ppm_noise_by_tick, other_seed.failed_ppm_noise_by_tick);
        assert!(first.queue_noise_by_tick.iter().all(|value| value.abs() <= 12));
        assert!(first.failed_ppm_noise_by_tick.iter().all(|value| value.abs() <= 50_000));
    }

    #[test]
    fn seeded_noise_v1_rejects_invalid_bounds_and_empty_runs() {
        assert_eq!(
            generate_deterministic_noise_v1(1, 0, 0, 0),
            Err(NoiseGenerationError::ZeroTicks)
        );
        assert_eq!(
            generate_deterministic_noise_v1(1, 1, i64::MAX as u64 + 1, 0),
            Err(NoiseGenerationError::QueueBoundTooLarge)
        );
        assert_eq!(
            generate_deterministic_noise_v1(1, 1, 0, PPM as u32 + 1),
            Err(NoiseGenerationError::FailureNoiseBoundTooLarge)
        );
    }

    #[test]
    fn closed_loop_decision_binds_exact_observation_and_proposed_actions() {
        let mut config = config();
        config.actuation_delay_ticks = 0;
        let mut controller = OneShotReactiveScaleUp {
            id: "evidence-bound-agent",
            kind: Controller::Agent,
            issued: false,
            calls: Vec::new(),
            threshold_ppm: 300_000,
            capacity: 140,
        };
        let run = run_stability_closed_loop(&config, &mut controller).unwrap();
        assert!(validate_closed_loop_evidence(&config, &run).is_ok());
        assert_eq!(run.decisions[3].observation, run.stability.observations[3]);
        assert_eq!(
            run.decisions[3].proposed_actions,
            run.actions.iter()
                .filter(|action| action.issued_at_tick == run.decisions[3].decision_tick)
                .cloned()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn closed_loop_evidence_rejects_tampered_observation_snapshot() {
        let mut config = config();
        config.actuation_delay_ticks = 0;
        let mut controller = OneShotReactiveScaleUp {
            id: "evidence-bound-agent",
            kind: Controller::Agent,
            issued: false,
            calls: Vec::new(),
            threshold_ppm: 300_000,
            capacity: 140,
        };
        let mut run = run_stability_closed_loop(&config, &mut controller).unwrap();
        run.decisions[3].observation.observed_queue_depth =
            run.decisions[3].observation.observed_queue_depth.saturating_add(1);
        assert_eq!(
            validate_closed_loop_evidence(&config, &run),
            Err(StabilityError::ClosedLoopEvidenceMismatch)
        );
    }

    #[test]
    fn closed_loop_evidence_rejects_changed_operation_and_orphan_action() {
        let mut config = config();
        config.actuation_delay_ticks = 0;
        let mut controller = OneShotReactiveScaleUp {
            id: "evidence-bound-agent",
            kind: Controller::Agent,
            issued: false,
            calls: Vec::new(),
            threshold_ppm: 300_000,
            capacity: 140,
        };
        let run = run_stability_closed_loop(&config, &mut controller).unwrap();

        let mut changed_operation = run.clone();
        changed_operation.decisions[3].proposed_actions[0].operation =
            ControlOperation::SetApiCapacity(141);
        assert_eq!(
            validate_closed_loop_evidence(&config, &changed_operation),
            Err(StabilityError::ClosedLoopEvidenceMismatch)
        );

        let mut orphan_action = run;
        orphan_action.actions.pop();
        assert_eq!(
            validate_closed_loop_evidence(&config, &orphan_action),
            Err(StabilityError::ClosedLoopEvidenceMismatch)
        );
    }

    #[test]
    fn observation_trace_preserves_source_time_and_deterministic_noise() {
        let mut config = config();
        config.queue_noise_by_tick[3] = 9;
        let run = run_stability_scenario(&config, &[]).unwrap();
        assert_eq!(run.observations[3].source_tick, 2);
        assert_eq!(
            run.observations[3].observed_queue_depth,
            run.samples[2].queue_depth + 9
        );
    }

    #[test]
    fn actuation_delay_changes_the_plant_only_at_the_due_tick() {
        let mut config = config();
        config.actuation_delay_ticks = 2;
        config.fault_start_tick = 8;
        config.fault_end_tick = 9;
        let actions = [ControlAction {
            action_id: "scale-up".into(),
            controller: Controller::Agent,
            issued_at_tick: 0,
            observation_source_tick: 0,
            operation: ControlOperation::SetApiCapacity(140),
        }];
        let run = run_stability_scenario(&config, &actions).unwrap();
        assert_eq!(run.samples[0].actual_api_capacity, 100);
        assert_eq!(run.samples[1].actual_api_capacity, 100);
        assert_eq!(run.samples[2].actual_api_capacity, 100);
        assert_eq!(run.samples[3].actual_api_capacity, 140);
    }

    #[test]
    fn action_cannot_use_an_observation_before_its_delivery_delay() {
        let mut config = config();
        let actions = [ControlAction {
            action_id: "future-observation".into(),
            controller: Controller::Agent,
            issued_at_tick: 2,
            observation_source_tick: 2,
            operation: ControlOperation::SetApiCapacity(120),
        }];
        assert_eq!(
            run_stability_scenario(&config, &actions),
            Err(StabilityError::ObservationNotYetAvailable("future-observation".into()))
        );
        config.observation_delay_ticks = 0;
        assert!(run_stability_scenario(&config, &actions).is_ok());
    }

    #[test]
    fn action_reversals_and_stale_observation_use_are_counted() {
        let mut config = config();
        config.actuation_delay_ticks = 0;
        config.maximum_observation_age_ticks = 1;
        let actions = [
            ControlAction {
                action_id: "up".into(), controller: Controller::Agent,
                issued_at_tick: 0, observation_source_tick: 0,
                operation: ControlOperation::SetApiCapacity(120),
            },
            ControlAction {
                action_id: "down".into(), controller: Controller::Agent,
                issued_at_tick: 1, observation_source_tick: 0,
                operation: ControlOperation::SetApiCapacity(80),
            },
            ControlAction {
                action_id: "up-again".into(), controller: Controller::Agent,
                issued_at_tick: 2, observation_source_tick: 0,
                operation: ControlOperation::SetApiCapacity(110),
            },
        ];
        let run = run_stability_scenario(&config, &actions).unwrap();
        assert_eq!(run.metrics.action_churn_count, 3);
        assert_eq!(run.metrics.action_reversal_count, 2);
        assert_eq!(run.metrics.stale_observation_action_count, 1);
    }

    #[test]
    fn simultaneous_controller_conflicts_are_input_order_independent() {
        let mut config = config();
        config.actuation_delay_ticks = 1;
        let actions = vec![
            ControlAction {
                action_id: "agent-scale-up".into(),
                controller: Controller::Agent,
                issued_at_tick: 0,
                observation_source_tick: 0,
                operation: ControlOperation::SetApiCapacity(140),
            },
            ControlAction {
                action_id: "operator-limit".into(),
                controller: Controller::Operator,
                issued_at_tick: 0,
                observation_source_tick: 0,
                operation: ControlOperation::SetApiCapacity(90),
            },
            ControlAction {
                action_id: "autoscaler-scale-up".into(),
                controller: Controller::Autoscaler,
                issued_at_tick: 0,
                observation_source_tick: 0,
                operation: ControlOperation::SetApiCapacity(130),
            },
        ];
        let forward = run_stability_scenario(&config, &actions).unwrap();
        let reverse = run_stability_scenario(&config, &actions.into_iter().rev().collect::<Vec<_>>()).unwrap();
        assert_eq!(forward.samples, reverse.samples);
        assert_eq!(forward.action_outcomes, reverse.action_outcomes);
        assert_eq!(forward.metrics.action_conflict_count, 2);
        assert_eq!(forward.metrics.overridden_action_count, 2);
        assert_eq!(forward.samples[2].actual_api_capacity, 40);
        assert_eq!(forward.action_outcomes[0].controller, Controller::Operator);
        assert_eq!(forward.action_outcomes[0].disposition, ActionDisposition::Applied);
        assert!(forward.action_outcomes.iter().filter(|o| o.disposition == ActionDisposition::Overridden)
            .all(|o| o.overridden_by.as_deref() == Some("operator-limit")));
    }

    #[test]
    fn equal_precedence_controller_tie_uses_stable_action_id() {
        let mut config = config();
        config.actuation_delay_ticks = 0;
        let actions = vec![
            ControlAction {
                action_id: "z-agent".into(),
                controller: Controller::Agent,
                issued_at_tick: 0,
                observation_source_tick: 0,
                operation: ControlOperation::SetApiCapacity(130),
            },
            ControlAction {
                action_id: "a-agent".into(),
                controller: Controller::Agent,
                issued_at_tick: 0,
                observation_source_tick: 0,
                operation: ControlOperation::SetApiCapacity(110),
            },
        ];
        let run = run_stability_scenario(&config, &actions).unwrap();
        assert_eq!(run.samples[1].actual_api_capacity, 110);
        assert_eq!(run.metrics.action_conflict_count, 1);
        assert_eq!(run.action_outcomes[0].action_id, "a-agent");
        assert_eq!(run.action_outcomes[1].overridden_by.as_deref(), Some("a-agent"));
    }

    #[test]
    fn simultaneous_actions_on_independent_resources_both_apply() {
        let mut config = config();
        config.actuation_delay_ticks = 1;
        let actions = [
            ControlAction {
                action_id: "api-capacity".into(),
                controller: Controller::Agent,
                issued_at_tick: 0,
                observation_source_tick: 0,
                operation: ControlOperation::SetApiCapacity(120),
            },
            ControlAction {
                action_id: "retry-policy".into(),
                controller: Controller::Runbook,
                issued_at_tick: 0,
                observation_source_tick: 0,
                operation: ControlOperation::SetRetryMultiplierPpm(1_500_000),
            },
        ];
        let run = run_stability_scenario(&config, &actions).unwrap();
        assert_eq!(run.metrics.action_conflict_count, 0);
        assert_eq!(run.action_outcomes.len(), 2);
        assert!(run.action_outcomes.iter().all(|outcome| outcome.disposition == ActionDisposition::Applied));
        assert!(run.action_outcomes.iter().any(|outcome| outcome.resource == ControlResource::ApiCapacity));
        assert!(run.action_outcomes.iter().any(|outcome| outcome.resource == ControlResource::RetryMultiplier));
        assert_eq!(run.samples[2].actual_api_capacity, 70);
    }

    #[test]
    fn retries_amplify_bulk_load_without_consuming_critical_priority() {
        let mut config = config();
        config.actuation_delay_ticks = 0;
        let actions = [ControlAction {
            action_id: "retry-amplification".into(),
            controller: Controller::Agent,
            issued_at_tick: 5,
            observation_source_tick: 4,
            operation: ControlOperation::SetRetryMultiplierPpm(2_000_000),
        }];
        let run = run_stability_scenario(&config, &actions).unwrap();
        assert!(run.metrics.bulk_failed_requests > 0);
        assert!(run.metrics.critical_failed_requests <= run.metrics.bulk_failed_requests);
    }

    #[test]
    fn workload_class_slos_expose_bulk_degradation_separately() {
        let config = config();
        let run = run_stability_scenario(&config, &[]).unwrap();
        let during_fault = &run.samples[config.fault_start_tick as usize];

        assert_eq!(during_fault.critical_failed_ppm, 0);
        assert!(during_fault.bulk_failed_ppm > 0);
        assert!(during_fault.critical_slo_healthy);
        assert!(!during_fault.bulk_slo_healthy);
        assert!(
            run.metrics.bulk_slo_violation_integral_request_ms
                > run.metrics.critical_slo_violation_integral_request_ms
        );
        assert_eq!(
            run.metrics.slo_violation_integral_request_ms,
            run.metrics.critical_slo_violation_integral_request_ms
                + run.metrics.bulk_slo_violation_integral_request_ms
        );
    }

    #[test]
    fn database_bottleneck_is_reported_separately_from_api_capacity() {
        let mut config = config();
        config.database_capacity = 40;
        config.actuation_delay_ticks = 0;
        let actions = [ControlAction {
            action_id: "overscale-api".into(),
            controller: Controller::Autoscaler,
            issued_at_tick: 0,
            observation_source_tick: 0,
            operation: ControlOperation::SetApiCapacity(180),
        }];
        let run = run_stability_scenario(&config, &actions).unwrap();
        assert!(run.metrics.api_database_capacity_gap_request_units > 0);
    }

    #[test]
    fn no_intervention_tail_is_required_for_persistent_recovery() {
        let mut config = config();
        config.fault_api_capacity_reduction = 10;
        let run = run_stability_scenario(&config, &[]).unwrap();
        assert!(run.metrics.final_no_intervention_tail_healthy);
        assert!(run.metrics.settling_time_ticks_after_fault.is_some());
    }

    #[test]
    fn settling_time_ignores_transient_recovery_before_later_degradation() {
        let mut config = config();
        config.fault_api_capacity_reduction = 10;
        config.actuation_delay_ticks = 0;
        let actions = [
            ControlAction {
                action_id: "temporary-capacity-drop".into(),
                controller: Controller::Agent,
                issued_at_tick: 8,
                observation_source_tick: 7,
                operation: ControlOperation::SetApiCapacity(40),
            },
            ControlAction {
                action_id: "restore-capacity".into(),
                controller: Controller::Agent,
                issued_at_tick: 9,
                observation_source_tick: 8,
                operation: ControlOperation::SetApiCapacity(100),
            },
        ];
        let run = run_stability_scenario(&config, &actions).unwrap();
        assert_eq!(run.metrics.settling_time_ticks_after_fault, Some(6));
        assert!(run.metrics.final_no_intervention_tail_healthy);
    }

    #[test]
    fn runaway_retries_are_worse_than_no_retry_baseline_and_fail_recovery_tail() {
        let mut config = config();
        config.actuation_delay_ticks = 0;
        let baseline = run_stability_scenario(&config, &[]).unwrap();
        let retry_actions = [ControlAction {
            action_id: "enable-retry-amplification".into(),
            controller: Controller::Agent,
            issued_at_tick: 5,
            observation_source_tick: 4,
            operation: ControlOperation::SetRetryMultiplierPpm(2_000_000),
        }];
        let retry_run = run_stability_scenario(&config, &retry_actions).unwrap();
        assert!(retry_run.metrics.bulk_failed_requests > baseline.metrics.bulk_failed_requests);
        assert!(retry_run.metrics.slo_violation_integral_request_ms
            > baseline.metrics.slo_violation_integral_request_ms);
        assert!(baseline.metrics.final_no_intervention_tail_healthy);
        assert!(!retry_run.metrics.final_no_intervention_tail_healthy);
    }

    #[test]
    fn actions_cannot_be_scheduled_into_the_no_intervention_tail() {
        let mut config = config();
        config.actuation_delay_ticks = 1;
        let actions = [ControlAction {
            action_id: "late-repair".into(),
            controller: Controller::Agent,
            issued_at_tick: 11,
            observation_source_tick: 10,
            operation: ControlOperation::SetApiCapacity(120),
        }];
        assert_eq!(
            run_stability_scenario(&config, &actions),
            Err(StabilityError::ActionInNoInterventionTail("late-repair".into()))
        );
    }

    #[test]
    fn delayed_noisy_feedback_and_faulted_workload_produce_separate_metrics() {
        let mut config = config();
        config.queue_noise_by_tick[7] = -4;
        let run = run_stability_scenario(&config, &[]).unwrap();
        assert!(run.metrics.slo_violation_integral_request_ms > 0);
        assert!(run.metrics.maximum_latency_ms >= config.base_latency_ms);
        assert!(run.metrics.maximum_queue_depth > 0);
        assert!(run.samples[2].critical_failed <= run.samples[2].bulk_failed);
    }
}
