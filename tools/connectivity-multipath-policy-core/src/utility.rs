//! Deterministic application-task utility evaluation for matched network trials.
//!
//! This module evaluates measured summaries. It does not schedule packets,
//! choose paths, implement a transport, infer user value or actuate a network.
//! Every verdict is scoped to one paired trial; programme-level conclusions
//! require repeated trials and confidence analysis outside this module.

use std::collections::BTreeSet;

const PPM: u128 = 1_000_000;
const BYTES_PER_MIB: u128 = 1_048_576;
const MAX_TASKS: usize = 512;
const MAX_RESOURCES: usize = 64;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_RATIO_TARGET_PPM: u64 = 10_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImplementationProvenance {
    /// The implementation is part of the exact kernel build digest.
    InTreeKernel,
    /// The implementation is separately bound to its compiled artifact digest.
    ExternalArtifact { artifact_sha256: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransportProtocol {
    Tcp,
    Quic,
    Mptcp,
    MultipathQuic,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArmIdentity {
    pub protocol: TransportProtocol,
    /// Exact protocol revision or negotiated profile, e.g. RFC 8684 or draft-21.
    pub protocol_revision: String,
    pub scheduler: String,
    pub scheduler_provenance: ImplementationProvenance,
    pub congestion_control: String,
    pub congestion_control_provenance: ImplementationProvenance,
}

impl ArmIdentity {
    fn validate(&self) -> Result<(), UtilityError> {
        if !valid_identifier(&self.protocol_revision)
            || !valid_identifier(&self.scheduler)
            || !valid_identifier(&self.congestion_control)
        {
            return Err(UtilityError::MissingArmIdentity);
        }
        validate_provenance(
            &self.scheduler_provenance,
            "scheduler",
        )?;
        validate_provenance(
            &self.congestion_control_provenance,
            "congestion_control",
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvironmentIdentity {
    pub kernel_build_sha256: String,
    pub topology_sha256: String,
    pub resource_snapshot_sha256: String,
    pub qdisc_config_sha256: String,
    pub workload_sha256: String,
    pub impairment_sha256: String,
    pub task_set_sha256: String,
    pub seed: u64,
    pub warmup_us: u64,
    pub measurement_duration_us: u64,
}

impl EnvironmentIdentity {
    fn validate(&self) -> Result<(), UtilityError> {
        for (name, digest) in [
            ("kernel_build_sha256", &self.kernel_build_sha256),
            ("topology_sha256", &self.topology_sha256),
            ("resource_snapshot_sha256", &self.resource_snapshot_sha256),
            ("qdisc_config_sha256", &self.qdisc_config_sha256),
            ("workload_sha256", &self.workload_sha256),
            ("impairment_sha256", &self.impairment_sha256),
            ("task_set_sha256", &self.task_set_sha256),
        ] {
            if !is_sha256(digest) {
                return Err(UtilityError::InvalidDigest(name));
            }
        }
        if self.measurement_duration_us == 0 {
            return Err(UtilityError::ZeroMeasurementDuration);
        }
        if self.warmup_us >= self.measurement_duration_us {
            return Err(UtilityError::InvalidWarmupDuration);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskDefinition {
    pub task_id: String,
    pub required_useful_bytes: u64,
    /// Caller-defined task value in integer micro-value units.
    pub value_weight_micros: u64,
    /// Relative to the start of the measured trial, after warmup.
    pub deadline_us: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskObservation {
    pub definition: TaskDefinition,
    /// Deduplicated application payload bytes received by the deadline.
    pub on_time_useful_bytes: u64,
    /// Deduplicated application payload bytes received after the deadline.
    pub late_useful_bytes: u64,
    /// Caller-defined value actually delivered by the deadline.
    pub on_time_value_micros: u64,
    /// Full task completion time; None means incomplete at measurement end.
    pub completed_at_us: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceLoad {
    pub resource_id: String,
    pub residual_capacity_bps: u64,
    pub allocated_rate_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrialMeasurement {
    pub environment: EnvironmentIdentity,
    pub arm: ArmIdentity,
    /// False when the requested protocol/controller/scheduler was unsupported
    /// or negotiated fallback was detected.
    pub supported: bool,
    pub all_required_counters_present: bool,
    pub cleanup_verified: bool,
    pub tasks: Vec<TaskObservation>,
    pub resources: Vec<ResourceLoad>,
    pub total_cost_micros: u64,
    pub total_energy_uwh: u64,
    pub redundant_bytes: u64,
    pub retransmitted_bytes: u64,
    pub receiver_reorder_peak_bytes: u64,
    pub receiver_reorder_byte_micros: u128,
    /// Goodput of a separately measured competing TCP/QUIC flow, if present.
    pub companion_goodput_bps: Option<u64>,
    pub companion_counters_present: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UtilityDisposition {
    Pass,
    Fail,
    Inconclusive,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UtilityError {
    InvalidDigest(&'static str),
    InvalidImplementationArtifact(&'static str),
    MissingArmIdentity,
    ZeroMeasurementDuration,
    InvalidWarmupDuration,
    TooManyTasks,
    TooManyResources,
    InvalidTaskId,
    DuplicateTaskId,
    InvalidTaskDefinition,
    InvalidTaskDeadline,
    InvalidTaskObservation,
    InvalidResourceId,
    DuplicateResourceId,
    InvalidResourceCapacity,
    InvalidTarget,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UtilityTargets {
    pub minimum_on_time_task_value_ratio_ppm: u64,
    pub minimum_on_time_useful_byte_ratio_ppm: u64,
    pub minimum_companion_retention_ratio_ppm: u64,
    pub maximum_cost_per_useful_mib_ratio_ppm: u64,
    pub maximum_energy_per_useful_mib_ratio_ppm: u64,
}

impl UtilityTargets {
    fn validate(&self) -> Result<(), UtilityError> {
        if self.minimum_on_time_task_value_ratio_ppm == 0
            || self.minimum_on_time_task_value_ratio_ppm > MAX_RATIO_TARGET_PPM
            || self.minimum_on_time_useful_byte_ratio_ppm == 0
            || self.minimum_on_time_useful_byte_ratio_ppm > MAX_RATIO_TARGET_PPM
            || self.minimum_companion_retention_ratio_ppm == 0
            || self.minimum_companion_retention_ratio_ppm > PPM as u64
            || self.maximum_cost_per_useful_mib_ratio_ppm == 0
            || self.maximum_cost_per_useful_mib_ratio_ppm > MAX_RATIO_TARGET_PPM
            || self.maximum_energy_per_useful_mib_ratio_ppm == 0
            || self.maximum_energy_per_useful_mib_ratio_ppm > MAX_RATIO_TARGET_PPM
        {
            return Err(UtilityError::InvalidTarget);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrialSummary {
    pub task_count: u32,
    pub required_useful_bytes: u64,
    pub on_time_useful_bytes: u64,
    pub late_useful_bytes: u64,
    pub on_time_useful_byte_ratio_ppm: u64,
    pub total_task_value_micros: u64,
    pub on_time_task_value_micros: u64,
    pub on_time_task_value_ratio_ppm: u64,
    pub tasks_completed_by_deadline: u32,
    pub deadline_miss_count: u32,
    pub censored_deadline_miss_count: u32,
    pub completed_late_task_count: u32,
    pub completion_lateness_p50_us: Option<u64>,
    pub completion_lateness_p95_us: Option<u64>,
    pub completion_lateness_p99_us: Option<u64>,
    pub cost_per_on_time_useful_mib_micros: Option<u64>,
    pub energy_per_on_time_useful_mib_uwh: Option<u64>,
    pub late_bytes_per_on_time_useful_mib: Option<u64>,
    pub redundant_bytes_per_on_time_useful_mib: Option<u64>,
    pub retransmitted_bytes_per_on_time_useful_mib: Option<u64>,
    pub total_cost_micros: u64,
    pub total_energy_uwh: u64,
    pub redundant_bytes: u64,
    pub retransmitted_bytes: u64,
    pub receiver_reorder_peak_bytes: u64,
    pub receiver_reorder_byte_micros: u128,
    pub companion_goodput_bps: Option<u64>,
    pub resource_loads: Vec<ResourceLoad>,
    pub resource_overcommit_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UtilityGateResult {
    pub gate_id: &'static str,
    pub disposition: UtilityDisposition,
    pub reason: &'static str,
    pub target_ratio_ppm: Option<u64>,
    pub measured_ratio_ppm: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsefulWorkComparison {
    /// A successful result is only a single matched pair, never a programme pass.
    pub scope: &'static str,
    pub disposition: UtilityDisposition,
    pub reason: &'static str,
    pub control: Option<TrialSummary>,
    pub treatment: Option<TrialSummary>,
    pub gates: Vec<UtilityGateResult>,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && !value.chars().any(char::is_control)
}

fn validate_provenance(
    provenance: &ImplementationProvenance,
    name: &'static str,
) -> Result<(), UtilityError> {
    if let ImplementationProvenance::ExternalArtifact { artifact_sha256 } = provenance {
        if !is_sha256(artifact_sha256) {
            return Err(UtilityError::InvalidImplementationArtifact(name));
        }
    }
    Ok(())
}

fn fixed_ratio_ppm(numerator: u64, denominator: u64) -> Option<u64> {
    if denominator == 0 {
        return None;
    }
    u64::try_from(u128::from(numerator) * PPM / u128::from(denominator)).ok()
}

fn ceil_per_mib(total: u64, on_time_bytes: u64) -> Option<u64> {
    if on_time_bytes == 0 {
        return None;
    }
    let numerator = u128::from(total).checked_mul(BYTES_PER_MIB)?;
    let denominator = u128::from(on_time_bytes);
    u64::try_from(numerator.div_ceil(denominator)).ok()
}

fn percentile_nearest_rank(values: &[u64], percentile: u64) -> Option<u64> {
    if values.is_empty() || !(1..=100).contains(&percentile) {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = (u128::from(percentile) * sorted.len() as u128).div_ceil(100);
    let index = usize::try_from(rank.saturating_sub(1)).ok()?;
    sorted.get(index).copied()
}

fn validate_task_observation(
    task: &TaskObservation,
    measurement_duration_us: u64,
) -> Result<(), UtilityError> {
    let definition = &task.definition;
    if !valid_identifier(&definition.task_id) || definition.required_useful_bytes == 0 {
        return Err(UtilityError::InvalidTaskDefinition);
    }
    if definition.value_weight_micros == 0 {
        return Err(UtilityError::InvalidTaskDefinition);
    }
    if definition.deadline_us == 0 || definition.deadline_us > measurement_duration_us {
        return Err(UtilityError::InvalidTaskDeadline);
    }
    if task.on_time_useful_bytes > definition.required_useful_bytes
        || task.late_useful_bytes
            > definition
                .required_useful_bytes
                .saturating_sub(task.on_time_useful_bytes)
        || task.on_time_value_micros > definition.value_weight_micros
    {
        return Err(UtilityError::InvalidTaskObservation);
    }

    let delivered = task
        .on_time_useful_bytes
        .checked_add(task.late_useful_bytes)
        .ok_or(UtilityError::ArithmeticOverflow)?;
    match task.completed_at_us {
        Some(completed_at_us) => {
            if completed_at_us > measurement_duration_us || delivered != definition.required_useful_bytes {
                return Err(UtilityError::InvalidTaskObservation);
            }
            if completed_at_us <= definition.deadline_us
                && (task.on_time_useful_bytes != definition.required_useful_bytes
                    || task.late_useful_bytes != 0
                    || task.on_time_value_micros != definition.value_weight_micros)
            {
                return Err(UtilityError::InvalidTaskObservation);
            }
        }
        None => {
            if delivered == definition.required_useful_bytes {
                return Err(UtilityError::InvalidTaskObservation);
            }
        }
    }
    Ok(())
}

fn summarize_trial(trial: &TrialMeasurement) -> Result<TrialSummary, UtilityError> {
    trial.environment.validate()?;
    trial.arm.validate()?;
    if trial.tasks.is_empty() || trial.tasks.len() > MAX_TASKS {
        return Err(UtilityError::TooManyTasks);
    }
    if trial.resources.is_empty() || trial.resources.len() > MAX_RESOURCES {
        return Err(UtilityError::TooManyResources);
    }

    let mut task_ids = BTreeSet::new();
    let mut ordered_tasks = trial.tasks.iter().collect::<Vec<_>>();
    ordered_tasks.sort_by(|left, right| {
        left.definition.task_id.cmp(&right.definition.task_id)
    });

    let mut required_useful_bytes = 0_u64;
    let mut on_time_useful_bytes = 0_u64;
    let mut late_useful_bytes = 0_u64;
    let mut total_task_value_micros = 0_u64;
    let mut on_time_task_value_micros = 0_u64;
    let mut tasks_completed_by_deadline = 0_u32;
    let mut deadline_miss_count = 0_u32;
    let mut censored_deadline_miss_count = 0_u32;
    let mut completion_lateness_us = Vec::new();

    for task in ordered_tasks {
        if !valid_identifier(&task.definition.task_id) {
            return Err(UtilityError::InvalidTaskId);
        }
        if !task_ids.insert(task.definition.task_id.as_str()) {
            return Err(UtilityError::DuplicateTaskId);
        }
        validate_task_observation(task, trial.environment.measurement_duration_us)?;

        required_useful_bytes = required_useful_bytes
            .checked_add(task.definition.required_useful_bytes)
            .ok_or(UtilityError::ArithmeticOverflow)?;
        on_time_useful_bytes = on_time_useful_bytes
            .checked_add(task.on_time_useful_bytes)
            .ok_or(UtilityError::ArithmeticOverflow)?;
        late_useful_bytes = late_useful_bytes
            .checked_add(task.late_useful_bytes)
            .ok_or(UtilityError::ArithmeticOverflow)?;
        total_task_value_micros = total_task_value_micros
            .checked_add(task.definition.value_weight_micros)
            .ok_or(UtilityError::ArithmeticOverflow)?;
        on_time_task_value_micros = on_time_task_value_micros
            .checked_add(task.on_time_value_micros)
            .ok_or(UtilityError::ArithmeticOverflow)?;

        match task.completed_at_us {
            Some(completed_at_us) if completed_at_us <= task.definition.deadline_us => {
                tasks_completed_by_deadline = tasks_completed_by_deadline
                    .checked_add(1)
                    .ok_or(UtilityError::ArithmeticOverflow)?;
            }
            Some(completed_at_us) => {
                deadline_miss_count = deadline_miss_count
                    .checked_add(1)
                    .ok_or(UtilityError::ArithmeticOverflow)?;
                completion_lateness_us.push(completed_at_us - task.definition.deadline_us);
            }
            None => {
                deadline_miss_count = deadline_miss_count
                    .checked_add(1)
                    .ok_or(UtilityError::ArithmeticOverflow)?;
                censored_deadline_miss_count = censored_deadline_miss_count
                    .checked_add(1)
                    .ok_or(UtilityError::ArithmeticOverflow)?;
            }
        }
    }

    if required_useful_bytes == 0 || total_task_value_micros == 0 {
        return Err(UtilityError::InvalidTaskDefinition);
    }

    let mut resource_ids = BTreeSet::new();
    let mut resource_loads = trial.resources.clone();
    resource_loads.sort_by(|left, right| left.resource_id.cmp(&right.resource_id));
    let mut resource_overcommit_count = 0_u32;
    for resource in &resource_loads {
        if !valid_identifier(&resource.resource_id) {
            return Err(UtilityError::InvalidResourceId);
        }
        if !resource_ids.insert(resource.resource_id.as_str()) {
            return Err(UtilityError::DuplicateResourceId);
        }
        if resource.residual_capacity_bps == 0 {
            return Err(UtilityError::InvalidResourceCapacity);
        }
        if resource.allocated_rate_bps > resource.residual_capacity_bps {
            resource_overcommit_count = resource_overcommit_count
                .checked_add(1)
                .ok_or(UtilityError::ArithmeticOverflow)?;
        }
    }
    if trial.companion_counters_present && trial.companion_goodput_bps.is_none() {
        return Err(UtilityError::InvalidTaskObservation);
    }

    Ok(TrialSummary {
        task_count: u32::try_from(trial.tasks.len()).map_err(|_| UtilityError::ArithmeticOverflow)?,
        required_useful_bytes,
        on_time_useful_bytes,
        late_useful_bytes,
        on_time_useful_byte_ratio_ppm: fixed_ratio_ppm(on_time_useful_bytes, required_useful_bytes)
            .ok_or(UtilityError::ArithmeticOverflow)?,
        total_task_value_micros,
        on_time_task_value_micros,
        on_time_task_value_ratio_ppm: fixed_ratio_ppm(
            on_time_task_value_micros,
            total_task_value_micros,
        )
        .ok_or(UtilityError::ArithmeticOverflow)?,
        tasks_completed_by_deadline,
        deadline_miss_count,
        censored_deadline_miss_count,
        completed_late_task_count: u32::try_from(completion_lateness_us.len())
            .map_err(|_| UtilityError::ArithmeticOverflow)?,
        completion_lateness_p50_us: percentile_nearest_rank(&completion_lateness_us, 50),
        completion_lateness_p95_us: percentile_nearest_rank(&completion_lateness_us, 95),
        completion_lateness_p99_us: percentile_nearest_rank(&completion_lateness_us, 99),
        cost_per_on_time_useful_mib_micros: ceil_per_mib(
            trial.total_cost_micros,
            on_time_useful_bytes,
        ),
        energy_per_on_time_useful_mib_uwh: ceil_per_mib(
            trial.total_energy_uwh,
            on_time_useful_bytes,
        ),
        late_bytes_per_on_time_useful_mib: ceil_per_mib(late_useful_bytes, on_time_useful_bytes),
        redundant_bytes_per_on_time_useful_mib: ceil_per_mib(
            trial.redundant_bytes,
            on_time_useful_bytes,
        ),
        retransmitted_bytes_per_on_time_useful_mib: ceil_per_mib(
            trial.retransmitted_bytes,
            on_time_useful_bytes,
        ),
        total_cost_micros: trial.total_cost_micros,
        total_energy_uwh: trial.total_energy_uwh,
        redundant_bytes: trial.redundant_bytes,
        retransmitted_bytes: trial.retransmitted_bytes,
        receiver_reorder_peak_bytes: trial.receiver_reorder_peak_bytes,
        receiver_reorder_byte_micros: trial.receiver_reorder_byte_micros,
        companion_goodput_bps: trial.companion_goodput_bps,
        resource_loads,
        resource_overcommit_count,
    })
}

fn gate(
    gate_id: &'static str,
    disposition: UtilityDisposition,
    reason: &'static str,
    target_ratio_ppm: Option<u64>,
    measured_ratio_ppm: Option<u64>,
) -> UtilityGateResult {
    UtilityGateResult {
        gate_id,
        disposition,
        reason,
        target_ratio_ppm,
        measured_ratio_ppm,
    }
}

fn early_result(
    disposition: UtilityDisposition,
    reason: &'static str,
) -> UsefulWorkComparison {
    UsefulWorkComparison {
        scope: "single_paired_trial",
        disposition,
        reason,
        control: None,
        treatment: None,
        gates: vec![gate(
            "USEFUL_WORK_PAIR_ELIGIBILITY",
            disposition,
            reason,
            None,
            None,
        )],
    }
}

fn resource_snapshots_match(control: &[ResourceLoad], treatment: &[ResourceLoad]) -> bool {
    control.len() == treatment.len()
        && control.iter().zip(treatment).all(|(left, right)| {
            left.resource_id == right.resource_id
                && left.residual_capacity_bps == right.residual_capacity_bps
        })
}

fn ratio_gate(
    gate_id: &'static str,
    ratio: Option<u64>,
    target: u64,
    minimum: bool,
) -> UtilityGateResult {
    match ratio {
        None => gate(
            gate_id,
            UtilityDisposition::Inconclusive,
            "reference_or_useful_on_time_denominator_is_zero",
            Some(target),
            None,
        ),
        Some(measured) => {
            let passes = if minimum {
                measured >= target
            } else {
                measured <= target
            };
            gate(
                gate_id,
                if passes {
                    UtilityDisposition::Pass
                } else {
                    UtilityDisposition::Fail
                },
                if passes {
                    "declared_paired_utility_target_met"
                } else {
                    "declared_paired_utility_target_missed"
                },
                Some(target),
                Some(measured),
            )
        }
    }
}

/// Compare a control and treatment that carry the exact same task set and
/// frozen environment. A pair-level Pass is not a statistical programme pass.
pub fn evaluate_useful_work_pair(
    control_trial: &TrialMeasurement,
    treatment_trial: &TrialMeasurement,
    targets: &UtilityTargets,
) -> Result<UsefulWorkComparison, UtilityError> {
    targets.validate()?;
    control_trial.environment.validate()?;
    treatment_trial.environment.validate()?;
    control_trial.arm.validate()?;
    treatment_trial.arm.validate()?;

    if !control_trial.supported || !treatment_trial.supported {
        return Ok(early_result(
            UtilityDisposition::Unsupported,
            "one_or_both_transport_arms_are_unsupported_or_fell_back",
        ));
    }
    if !control_trial.cleanup_verified || !treatment_trial.cleanup_verified {
        return Ok(early_result(
            UtilityDisposition::Fail,
            "one_or_both_trial_cleanups_were_not_verified",
        ));
    }
    if !control_trial.all_required_counters_present
        || !treatment_trial.all_required_counters_present
    {
        return Ok(early_result(
            UtilityDisposition::Inconclusive,
            "one_or_both_trials_are_missing_required_counters",
        ));
    }
    if control_trial.environment != treatment_trial.environment {
        return Ok(early_result(
            UtilityDisposition::Inconclusive,
            "control_and_treatment_environment_identity_mismatch",
        ));
    }

    let control_specs = canonical_task_definitions(&control_trial.tasks);
    let treatment_specs = canonical_task_definitions(&treatment_trial.tasks);
    if control_specs.is_err() || treatment_specs.is_err() || control_specs? != treatment_specs? {
        return Ok(early_result(
            UtilityDisposition::Inconclusive,
            "control_and_treatment_task_set_mismatch_or_duplicate_task_id",
        ));
    }

    let control = summarize_trial(control_trial)?;
    let treatment = summarize_trial(treatment_trial)?;
    if !resource_snapshots_match(&control.resource_loads, &treatment.resource_loads) {
        return Ok(early_result(
            UtilityDisposition::Inconclusive,
            "control_and_treatment_resource_snapshot_mismatch",
        ));
    }

    let on_time_value_ratio = fixed_ratio_ppm(
        treatment.on_time_task_value_micros,
        control.on_time_task_value_micros,
    );
    let on_time_byte_ratio = fixed_ratio_ppm(
        treatment.on_time_useful_bytes,
        control.on_time_useful_bytes,
    );
    let cost_ratio = match (
        treatment.cost_per_on_time_useful_mib_micros,
        control.cost_per_on_time_useful_mib_micros,
    ) {
        (Some(treatment), Some(control)) => fixed_ratio_ppm(treatment, control),
        _ => None,
    };
    let energy_ratio = match (
        treatment.energy_per_on_time_useful_mib_uwh,
        control.energy_per_on_time_useful_mib_uwh,
    ) {
        (Some(treatment), Some(control)) => fixed_ratio_ppm(treatment, control),
        _ => None,
    };
    let companion_ratio = match (
        treatment.companion_goodput_bps,
        control.companion_goodput_bps,
    ) {
        (Some(treatment), Some(control)) => fixed_ratio_ppm(treatment, control),
        _ => None,
    };

    let total_overcommit = control
        .resource_overcommit_count
        .checked_add(treatment.resource_overcommit_count)
        .ok_or(UtilityError::ArithmeticOverflow)?;
    let resource_gate = gate(
        "USEFUL_WORK_RESOURCE_CEILING_001",
        if total_overcommit == 0 {
            UtilityDisposition::Pass
        } else {
            UtilityDisposition::Fail
        },
        if total_overcommit == 0 {
            "no_resource_capacity_overcommit_observed"
        } else {
            "resource_capacity_overcommit_observed"
        },
        Some(0),
        Some(u64::from(total_overcommit)),
    );

    let companion_gate = if !control_trial.companion_counters_present
        || !treatment_trial.companion_counters_present
    {
        gate(
            "USEFUL_WORK_COMPANION_RETENTION_001",
            UtilityDisposition::Inconclusive,
            "companion_flow_counters_missing",
            Some(targets.minimum_companion_retention_ratio_ppm),
            None,
        )
    } else {
        ratio_gate(
            "USEFUL_WORK_COMPANION_RETENTION_001",
            companion_ratio,
            targets.minimum_companion_retention_ratio_ppm,
            true,
        )
    };

    let gates = vec![
        ratio_gate(
            "USEFUL_WORK_TASK_VALUE_001",
            on_time_value_ratio,
            targets.minimum_on_time_task_value_ratio_ppm,
            true,
        ),
        ratio_gate(
            "USEFUL_WORK_ON_TIME_BYTES_001",
            on_time_byte_ratio,
            targets.minimum_on_time_useful_byte_ratio_ppm,
            true,
        ),
        ratio_gate(
            "USEFUL_WORK_COST_PER_ON_TIME_MIB_001",
            cost_ratio,
            targets.maximum_cost_per_useful_mib_ratio_ppm,
            false,
        ),
        ratio_gate(
            "USEFUL_WORK_ENERGY_PER_ON_TIME_MIB_001",
            energy_ratio,
            targets.maximum_energy_per_useful_mib_ratio_ppm,
            false,
        ),
        companion_gate,
        resource_gate,
    ];

    let disposition = if gates
        .iter()
        .any(|item| item.disposition == UtilityDisposition::Fail)
    {
        UtilityDisposition::Fail
    } else if gates
        .iter()
        .any(|item| item.disposition == UtilityDisposition::Unsupported)
    {
        UtilityDisposition::Unsupported
    } else if gates
        .iter()
        .any(|item| item.disposition == UtilityDisposition::Inconclusive)
    {
        UtilityDisposition::Inconclusive
    } else {
        UtilityDisposition::Pass
    };

    Ok(UsefulWorkComparison {
        scope: "single_paired_trial",
        disposition,
        reason: match disposition {
            UtilityDisposition::Pass => "all_declared_paired_utility_gates_passed",
            UtilityDisposition::Fail => "one_or_more_paired_utility_gates_failed",
            UtilityDisposition::Inconclusive => "one_or_more_paired_utility_gates_inconclusive",
            UtilityDisposition::Unsupported => "one_or_more_trial_arms_unsupported",
        },
        control: Some(control),
        treatment: Some(treatment),
        gates,
    })
}

fn canonical_task_definitions(
    tasks: &[TaskObservation],
) -> Result<Vec<TaskDefinition>, UtilityError> {
    let mut definitions = tasks
        .iter()
        .map(|task| task.definition.clone())
        .collect::<Vec<_>>();
    definitions.sort_by(|left, right| left.task_id.cmp(&right.task_id));
    for pair in definitions.windows(2) {
        if pair[0].task_id == pair[1].task_id {
            return Err(UtilityError::DuplicateTaskId);
        }
    }
    Ok(definitions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment() -> EnvironmentIdentity {
        EnvironmentIdentity {
            kernel_build_sha256: "a".repeat(64),
            topology_sha256: "b".repeat(64),
            resource_snapshot_sha256: "c".repeat(64),
            qdisc_config_sha256: "d".repeat(64),
            workload_sha256: "e".repeat(64),
            impairment_sha256: "f".repeat(64),
            task_set_sha256: "1".repeat(64),
            seed: 11,
            warmup_us: 1_000_000,
            measurement_duration_us: 30_000_000,
        }
    }

    fn in_tree_arm(protocol: TransportProtocol, scheduler: &str) -> ArmIdentity {
        ArmIdentity {
            protocol,
            protocol_revision: match protocol {
                TransportProtocol::Tcp => "RFC 9293".to_owned(),
                TransportProtocol::Quic => "RFC 9000".to_owned(),
                TransportProtocol::Mptcp => "RFC 8684".to_owned(),
                TransportProtocol::MultipathQuic => "draft-21".to_owned(),
            },
            scheduler: scheduler.to_owned(),
            scheduler_provenance: ImplementationProvenance::InTreeKernel,
            congestion_control: "cubic".to_owned(),
            congestion_control_provenance: ImplementationProvenance::InTreeKernel,
        }
    }

    fn external_quic_arm(scheduler: &str) -> ArmIdentity {
        ArmIdentity {
            protocol: TransportProtocol::MultipathQuic,
            protocol_revision: "draft-21".to_owned(),
            scheduler: scheduler.to_owned(),
            scheduler_provenance: ImplementationProvenance::ExternalArtifact {
                artifact_sha256: "2".repeat(64),
            },
            congestion_control: "olia".to_owned(),
            congestion_control_provenance: ImplementationProvenance::ExternalArtifact {
                artifact_sha256: "3".repeat(64),
            },
        }
    }

    fn tasks() -> Vec<TaskObservation> {
        vec![
            TaskObservation {
                definition: TaskDefinition {
                    task_id: "control-small".to_owned(),
                    required_useful_bytes: 1_000,
                    value_weight_micros: 100,
                    deadline_us: 10_000,
                },
                on_time_useful_bytes: 1_000,
                late_useful_bytes: 0,
                on_time_value_micros: 100,
                completed_at_us: Some(8_000),
            },
            TaskObservation {
                definition: TaskDefinition {
                    task_id: "bulk-stream".to_owned(),
                    required_useful_bytes: 2_000,
                    value_weight_micros: 200,
                    deadline_us: 10_000,
                },
                on_time_useful_bytes: 1_500,
                late_useful_bytes: 500,
                on_time_value_micros: 140,
                completed_at_us: Some(12_000),
            },
        ]
    }

    fn resources(allocated: u64) -> Vec<ResourceLoad> {
        vec![ResourceLoad {
            resource_id: "shared-backhaul".to_owned(),
            residual_capacity_bps: 100_000_000,
            allocated_rate_bps: allocated,
        }]
    }

    fn trial(
        protocol: TransportProtocol,
        scheduler: &str,
        tasks: Vec<TaskObservation>,
        on_time_cost: u64,
        companion: u64,
        allocated: u64,
    ) -> TrialMeasurement {
        TrialMeasurement {
            environment: environment(),
            arm: in_tree_arm(protocol, scheduler),
            supported: true,
            all_required_counters_present: true,
            cleanup_verified: true,
            tasks,
            resources: resources(allocated),
            total_cost_micros: on_time_cost,
            total_energy_uwh: on_time_cost,
            redundant_bytes: 100,
            retransmitted_bytes: 150,
            receiver_reorder_peak_bytes: 4_096,
            receiver_reorder_byte_micros: 10_000_000,
            companion_goodput_bps: Some(companion),
            companion_counters_present: true,
        }
    }

    fn targets() -> UtilityTargets {
        UtilityTargets {
            minimum_on_time_task_value_ratio_ppm: 1_000_000,
            minimum_on_time_useful_byte_ratio_ppm: 1_000_000,
            minimum_companion_retention_ratio_ppm: 900_000,
            maximum_cost_per_useful_mib_ratio_ppm: 1_000_000,
            maximum_energy_per_useful_mib_ratio_ppm: 1_000_000,
        }
    }

    #[test]
    fn treatment_passes_when_useful_value_improves_without_cost_or_starvation_regression() {
        let control = trial(
            TransportProtocol::Tcp,
            "single-path",
            tasks(),
            1_000,
            100_000_000,
            80_000_000,
        );
        let mut treatment_tasks = tasks();
        treatment_tasks[1].on_time_useful_bytes = 1_800;
        treatment_tasks[1].late_useful_bytes = 200;
        treatment_tasks[1].on_time_value_micros = 180;
        treatment_tasks[1].completed_at_us = Some(12_000);
        let treatment = trial(
            TransportProtocol::MultipathQuic,
            "deadline-aware",
            treatment_tasks,
            900,
            95_000_000,
            90_000_000,
        );
        let result = evaluate_useful_work_pair(&control, &treatment, &targets())
            .expect("valid paired trials");
        assert_eq!(result.disposition, UtilityDisposition::Pass);
        assert_eq!(result.scope, "single_paired_trial");
        assert_eq!(
            result
                .treatment
                .as_ref()
                .expect("summary")
                .on_time_useful_bytes,
            2_800
        );
    }

    #[test]
    fn raw_throughput_or_goodput_gain_cannot_hide_task_value_regression() {
        let control = trial(
            TransportProtocol::Tcp,
            "single-path",
            tasks(),
            1_000,
            100_000_000,
            80_000_000,
        );
        let mut treatment_tasks = tasks();
        treatment_tasks[1].on_time_useful_bytes = 1_000;
        treatment_tasks[1].late_useful_bytes = 1_000;
        treatment_tasks[1].on_time_value_micros = 80;
        treatment_tasks[1].completed_at_us = Some(12_000);
        let treatment = trial(
            TransportProtocol::Mptcp,
            "default",
            treatment_tasks,
            800,
            95_000_000,
            90_000_000,
        );
        let result = evaluate_useful_work_pair(&control, &treatment, &targets())
            .expect("valid paired trials");
        assert_eq!(result.disposition, UtilityDisposition::Fail);
        assert!(result
            .gates
            .iter()
            .any(|gate| gate.gate_id == "USEFUL_WORK_TASK_VALUE_001"
                && gate.disposition == UtilityDisposition::Fail));
    }

    #[test]
    fn companion_starvation_fails_even_when_task_value_improves() {
        let control = trial(
            TransportProtocol::Tcp,
            "single-path",
            tasks(),
            1_000,
            100_000_000,
            80_000_000,
        );
        let mut better_tasks = tasks();
        better_tasks[1].on_time_value_micros = 180;
        better_tasks[1].on_time_useful_bytes = 1_800;
        better_tasks[1].late_useful_bytes = 200;
        let treatment = trial(
            TransportProtocol::Mptcp,
            "default",
            better_tasks,
            800,
            10_000_000,
            90_000_000,
        );
        let result =
            evaluate_useful_work_pair(&control, &treatment, &targets()).expect("valid trials");
        assert_eq!(result.disposition, UtilityDisposition::Fail);
        assert!(result.gates.iter().any(|gate| {
            gate.gate_id == "USEFUL_WORK_COMPANION_RETENTION_001"
                && gate.disposition == UtilityDisposition::Fail
        }));
    }

    #[test]
    fn incomplete_tasks_count_as_misses_but_keep_partial_streaming_value() {
        let mut data = tasks();
        data[1].completed_at_us = None;
        data[1].on_time_useful_bytes = 900;
        data[1].late_useful_bytes = 300;
        data[1].on_time_value_micros = 80;
        let control = trial(
            TransportProtocol::Tcp,
            "single-path",
            data.clone(),
            1_000,
            100,
            80,
        );
        let summary = summarize_trial(&control).expect("valid partial task trial");
        assert_eq!(summary.deadline_miss_count, 1);
        assert_eq!(summary.censored_deadline_miss_count, 1);
        assert_eq!(summary.on_time_useful_bytes, 1_900);
        assert_eq!(summary.on_time_task_value_micros, 180);
    }

    #[test]
    fn deadline_lateness_uses_nearest_rank_and_excludes_censored_misses() {
        let mut data = tasks();
        let mut third = data[1].clone();
        third.definition.task_id = "bulk-late-two".to_owned();
        third.completed_at_us = Some(15_000);
        data[1].definition.task_id = "bulk-late-one".to_owned();
        data.push(third);
        let trial = trial(TransportProtocol::Tcp, "single", data, 10, 100, 80);
        let summary = summarize_trial(&trial).expect("valid task sample");
        assert_eq!(summary.completion_lateness_p50_us, Some(2_000));
        assert_eq!(summary.completion_lateness_p95_us, Some(5_000));
        assert_eq!(summary.completion_lateness_p99_us, Some(5_000));
    }

    #[test]
    fn task_set_mismatch_is_inconclusive_not_zero_filled() {
        let control = trial(
            TransportProtocol::Tcp,
            "single",
            tasks(),
            1_000,
            100,
            80,
        );
        let mut changed = tasks();
        changed[0].definition.deadline_us += 1;
        let treatment = trial(
            TransportProtocol::Mptcp,
            "default",
            changed,
            800,
            95,
            90,
        );
        let result =
            evaluate_useful_work_pair(&control, &treatment, &targets()).expect("valid trials");
        assert_eq!(result.disposition, UtilityDisposition::Inconclusive);
        assert_eq!(result.reason, "control_and_treatment_task_set_mismatch_or_duplicate_task_id");
    }

    #[test]
    fn environment_digest_mismatch_is_inconclusive() {
        let control = trial(
            TransportProtocol::Tcp,
            "single",
            tasks(),
            1_000,
            100,
            80,
        );
        let mut treatment = trial(
            TransportProtocol::Mptcp,
            "default",
            tasks(),
            800,
            95,
            90,
        );
        treatment.environment.qdisc_config_sha256 = "f".repeat(64);
        let result =
            evaluate_useful_work_pair(&control, &treatment, &targets()).expect("valid trials");
        assert_eq!(result.disposition, UtilityDisposition::Inconclusive);
    }

    #[test]
    fn resource_capacity_overcommit_fails_closed() {
        let control = trial(
            TransportProtocol::Tcp,
            "single",
            tasks(),
            1_000,
            100,
            80,
        );
        let mut treatment = trial(
            TransportProtocol::Mptcp,
            "default",
            tasks(),
            800,
            95,
            90,
        );
        treatment.resources[0].allocated_rate_bps =
            treatment.resources[0].residual_capacity_bps + 1;
        let result =
            evaluate_useful_work_pair(&control, &treatment, &targets()).expect("valid trials");
        assert_eq!(result.disposition, UtilityDisposition::Fail);
    }

    #[test]
    fn mismatched_resource_snapshot_measurements_are_inconclusive() {
        let control = trial(
            TransportProtocol::Tcp,
            "single",
            tasks(),
            1_000,
            100,
            80,
        );
        let mut treatment = trial(
            TransportProtocol::Mptcp,
            "default",
            tasks(),
            800,
            95,
            90,
        );
        treatment.resources[0].residual_capacity_bps -= 1;
        let result =
            evaluate_useful_work_pair(&control, &treatment, &targets()).expect("valid trials");
        assert_eq!(result.disposition, UtilityDisposition::Inconclusive);
    }

    #[test]
    fn missing_counters_are_inconclusive_and_cleanup_failure_fails() {
        let mut control = trial(
            TransportProtocol::Tcp,
            "single",
            tasks(),
            1_000,
            100,
            80,
        );
        let treatment = trial(
            TransportProtocol::Mptcp,
            "default",
            tasks(),
            800,
            95,
            90,
        );
        control.all_required_counters_present = false;
        assert_eq!(
            evaluate_useful_work_pair(&control, &treatment, &targets())
                .expect("pair is classified")
                .disposition,
            UtilityDisposition::Inconclusive
        );
        control.all_required_counters_present = true;
        control.cleanup_verified = false;
        assert_eq!(
            evaluate_useful_work_pair(&control, &treatment, &targets())
                .expect("pair is classified")
                .disposition,
            UtilityDisposition::Fail
        );
    }

    #[test]
    fn protocol_fallback_is_unsupported_not_a_multipath_pass() {
        let control = trial(
            TransportProtocol::Tcp,
            "single",
            tasks(),
            1_000,
            100,
            80,
        );
        let mut treatment = trial(
            TransportProtocol::MultipathQuic,
            "deadline-aware",
            tasks(),
            800,
            95,
            90,
        );
        treatment.supported = false;
        let result =
            evaluate_useful_work_pair(&control, &treatment, &targets()).expect("pair classified");
        assert_eq!(result.disposition, UtilityDisposition::Unsupported);
    }

    #[test]
    fn external_scheduler_and_congestion_control_need_artifact_digests() {
        let mut arm = external_quic_arm("deadline-aware");
        arm.scheduler_provenance = ImplementationProvenance::ExternalArtifact {
            artifact_sha256: "bad".to_owned(),
        };
        assert_eq!(
            arm.validate(),
            Err(UtilityError::InvalidImplementationArtifact("scheduler"))
        );
    }

    #[test]
    fn invalid_task_counter_conservation_is_rejected() {
        let mut data = tasks();
        data[0].late_useful_bytes = 1;
        let control = trial(TransportProtocol::Tcp, "single", data, 10, 100, 80);
        assert_eq!(
            summarize_trial(&control),
            Err(UtilityError::InvalidTaskObservation)
        );
    }

    #[test]
    fn duplicate_task_and_resource_ids_are_rejected() {
        let mut data = tasks();
        data[1].definition.task_id = data[0].definition.task_id.clone();
        let control = trial(TransportProtocol::Tcp, "single", data, 10, 100, 80);
        assert_eq!(
            summarize_trial(&control),
            Err(UtilityError::DuplicateTaskId)
        );

        let mut control = trial(TransportProtocol::Tcp, "single", tasks(), 10, 100, 80);
        control.resources.push(control.resources[0].clone());
        assert_eq!(
            summarize_trial(&control),
            Err(UtilityError::DuplicateResourceId)
        );
    }

    #[test]
    fn zero_baseline_on_time_bytes_makes_cost_ratio_inconclusive() {
        let mut control_tasks = tasks();
        for task in &mut control_tasks {
            task.on_time_useful_bytes = 0;
            task.late_useful_bytes = 0;
            task.on_time_value_micros = 0;
            task.completed_at_us = None;
        }
        let control = trial(
            TransportProtocol::Tcp,
            "single",
            control_tasks,
            1_000,
            100,
            80,
        );
        let treatment = trial(
            TransportProtocol::Mptcp,
            "default",
            tasks(),
            800,
            95,
            90,
        );
        let result =
            evaluate_useful_work_pair(&control, &treatment, &targets()).expect("pair classified");
        assert_eq!(result.disposition, UtilityDisposition::Inconclusive);
    }

    #[test]
    fn fixed_environment_and_task_inputs_produce_repeatable_comparison() {
        let control = trial(
            TransportProtocol::Tcp,
            "single",
            tasks(),
            1_000,
            100,
            80,
        );
        let treatment = trial(
            TransportProtocol::Mptcp,
            "default",
            tasks(),
            800,
            95,
            90,
        );
        let first =
            evaluate_useful_work_pair(&control, &treatment, &targets()).expect("valid pair");
        let second =
            evaluate_useful_work_pair(&control, &treatment, &targets()).expect("same pair");
        assert_eq!(first, second);
        assert_eq!(first.scope, "single_paired_trial");
    }
}
