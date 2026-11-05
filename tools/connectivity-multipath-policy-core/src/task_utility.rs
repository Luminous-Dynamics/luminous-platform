//! Deadline-qualified useful-work evaluation for paired multipath transport trials.
//!
//! This module evaluates measured task observations. It does not schedule packets,
//! implement a transport, infer application value, measure the network or actuate
//! routes. The caller must supply trustworthy task/counter observations and exact
//! environment/implementation identities. Programme-level statistics remain outside
//! this deterministic single-pair evaluator.

use std::collections::BTreeSet;

const PPM: u128 = 1_000_000;
const MIB_BYTES: u128 = 1_048_576;
const MAX_TASKS: usize = 100_000;
const MAX_ID_BYTES: usize = 128;
const MAX_VERSION_BYTES: usize = 256;
const MAX_RATIO_PPM: u64 = 10_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransportProtocol {
    Tcp,
    Quic,
    Mptcp,
    MultipathQuic,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskEnvironmentIdentity {
    pub kernel_build_sha256: String,
    pub system_image_sha256: String,
    pub topology_sha256: String,
    pub resource_snapshot_sha256: String,
    pub qdisc_config_sha256: String,
    pub workload_sha256: String,
    pub task_manifest_sha256: String,
    pub impairment_sha256: String,
    pub tariff_sha256: String,
    pub energy_model_sha256: String,
    pub observer_config_sha256: String,
    pub seed: u64,
    pub warmup_us: u64,
    pub measurement_duration_us: u64,
}

impl TaskEnvironmentIdentity {
    pub fn validate(&self) -> Result<(), TaskUtilityError> {
        for (field, digest) in [
            ("kernel_build_sha256", &self.kernel_build_sha256),
            ("system_image_sha256", &self.system_image_sha256),
            ("topology_sha256", &self.topology_sha256),
            ("resource_snapshot_sha256", &self.resource_snapshot_sha256),
            ("qdisc_config_sha256", &self.qdisc_config_sha256),
            ("workload_sha256", &self.workload_sha256),
            ("task_manifest_sha256", &self.task_manifest_sha256),
            ("impairment_sha256", &self.impairment_sha256),
            ("tariff_sha256", &self.tariff_sha256),
            ("energy_model_sha256", &self.energy_model_sha256),
            ("observer_config_sha256", &self.observer_config_sha256),
        ] {
            if !is_sha256(digest) {
                return Err(TaskUtilityError::InvalidDigest(field));
            }
        }
        if self.measurement_duration_us == 0 {
            return Err(TaskUtilityError::ZeroMeasurementDuration);
        }
        if self.warmup_us >= self.measurement_duration_us {
            return Err(TaskUtilityError::InvalidWarmupDuration);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportArmIdentity {
    pub protocol: TransportProtocol,
    pub transport_version: String,
    pub scheduler_id: String,
    pub congestion_control_id: String,
    pub path_manager_id: String,
    /// Digest of the exact transport/scheduler/controller implementation artifact.
    pub implementation_sha256: String,
    /// Digest of endpoint, path-manager, scheduler and congestion-control settings.
    pub arm_config_sha256: String,
}

impl TransportArmIdentity {
    pub fn validate(&self) -> Result<(), TaskUtilityError> {
        for (field, value, max_len) in [
            ("transport_version", self.transport_version.as_str(), MAX_VERSION_BYTES),
            ("scheduler_id", self.scheduler_id.as_str(), MAX_ID_BYTES),
            (
                "congestion_control_id",
                self.congestion_control_id.as_str(),
                MAX_ID_BYTES,
            ),
            ("path_manager_id", self.path_manager_id.as_str(), MAX_ID_BYTES),
        ] {
            if value.trim().is_empty()
                || value.len() > max_len
                || value.chars().any(char::is_control)
            {
                return Err(TaskUtilityError::InvalidImplementationIdentity(field));
            }
        }
        if !is_sha256(&self.implementation_sha256) {
            return Err(TaskUtilityError::InvalidDigest("implementation_sha256"));
        }
        if !is_sha256(&self.arm_config_sha256) {
            return Err(TaskUtilityError::InvalidDigest("arm_config_sha256"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskObservation {
    pub task_id: String,
    pub release_offset_us: u64,
    pub deadline_offset_us: u64,
    pub offered_useful_bytes: u64,
    /// Useful payload bytes received no later than the task's declared deadline.
    pub on_time_useful_bytes: u64,
    /// Useful payload bytes received after the declared deadline; not timely work.
    pub late_useful_bytes: u64,
    /// Application-provided value units offered for this task/chunk group.
    pub offered_value_units: u64,
    /// Portion of offered task value actually earned before its deadline.
    pub on_time_value_units: u64,
    /// Some(lateness) records a deadline miss; None means no task-level miss.
    pub deadline_miss_lateness_us: Option<u64>,
}

impl TaskObservation {
    fn validate(&self) -> Result<(), TaskUtilityError> {
        if self.task_id.trim().is_empty()
            || self.task_id.len() > MAX_ID_BYTES
            || self.task_id.chars().any(char::is_control)
        {
            return Err(TaskUtilityError::InvalidTaskId);
        }
        if self.deadline_offset_us <= self.release_offset_us {
            return Err(TaskUtilityError::InvalidTaskWindow);
        }
        if self.offered_useful_bytes == 0 || self.offered_value_units == 0 {
            return Err(TaskUtilityError::ZeroTaskDemand);
        }
        if self.on_time_useful_bytes > self.offered_useful_bytes
            || self.late_useful_bytes
                > self
                    .offered_useful_bytes
                    .saturating_sub(self.on_time_useful_bytes)
        {
            return Err(TaskUtilityError::TaskBytesExceedDemand);
        }
        if self.on_time_value_units > self.offered_value_units {
            return Err(TaskUtilityError::TaskValueExceedsDemand);
        }
        if self.deadline_miss_lateness_us == Some(0) {
            return Err(TaskUtilityError::ZeroDeadlineMissLateness);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskTrial {
    pub environment: TaskEnvironmentIdentity,
    pub arm: TransportArmIdentity,
    pub tasks: Vec<TaskObservation>,
    pub declared_task_count: u32,
    pub arm_supported: bool,
    pub task_counters_complete: bool,
    pub cost_energy_counters_complete: bool,
    pub resource_accounting_complete: bool,
    pub companion_counters_complete: bool,
    pub cleanup_verified: bool,
    /// Cost attributed to the measured task workload, in integer micro-currency units.
    pub workload_cost_micros: u64,
    /// Energy attributed to the measured task workload, in micro-watt-hours.
    pub workload_energy_uwh: u64,
    pub total_transmitted_bytes: u64,
    pub duplicate_bytes: u64,
    pub retransmitted_bytes: u64,
    pub max_resource_overcommit_bps: u64,
    pub aggregate_goodput_bps: u64,
    pub bottleneck_capacity_bps: u64,
    pub queue_delay_p95_us: u64,
    pub queue_delay_p99_us: u64,
    pub receiver_reorder_peak_bytes: u64,
    pub companion_demand_bps: u64,
    pub companion_goodput_bps: u64,
    pub companion_backlogged: bool,
    /// Digest of the raw trial evidence bundle (per-task observations, counters and readbacks).
    pub raw_evidence_sha256: String,
    /// Digest of the canonical measurement receipt binding the raw evidence to this arm.
    pub measurement_receipt_sha256: String,
}

impl TaskTrial {
    pub fn validate(&self) -> Result<(), TaskUtilityError> {
        self.environment.validate()?;
        self.arm.validate()?;
        if !is_sha256(&self.raw_evidence_sha256) {
            return Err(TaskUtilityError::InvalidDigest("raw_evidence_sha256"));
        }
        if !is_sha256(&self.measurement_receipt_sha256) {
            return Err(TaskUtilityError::InvalidDigest("measurement_receipt_sha256"));
        }
        if self.declared_task_count == 0
            || self.tasks.len() > self.declared_task_count as usize
            || self.tasks.len() > MAX_TASKS
        {
            return Err(TaskUtilityError::InvalidTaskCount);
        }
        let mut ids = BTreeSet::new();
        let mut observed_useful_bytes = 0_u128;
        for task in &self.tasks {
            task.validate()?;
            if !ids.insert(task.task_id.as_str()) {
                return Err(TaskUtilityError::DuplicateTaskId);
            }
            observed_useful_bytes = observed_useful_bytes
                .checked_add(u128::from(task.on_time_useful_bytes))
                .and_then(|value| value.checked_add(u128::from(task.late_useful_bytes)))
                .ok_or(TaskUtilityError::ArithmeticOverflow)?;
        }
        if observed_useful_bytes > u128::from(self.total_transmitted_bytes) {
            return Err(TaskUtilityError::UsefulBytesExceedTransmitted);
        }
        if self.duplicate_bytes > self.total_transmitted_bytes
            || self.retransmitted_bytes > self.total_transmitted_bytes
        {
            return Err(TaskUtilityError::TransmissionAccountingInvalid);
        }
        if self.bottleneck_capacity_bps == 0 {
            return Err(TaskUtilityError::ZeroBottleneckCapacity);
        }
        if self.companion_backlogged && self.companion_demand_bps == 0 {
            return Err(TaskUtilityError::InvalidCompanionDemand);
        }
        if self.companion_goodput_bps > self.companion_demand_bps {
            return Err(TaskUtilityError::CompanionGoodputExceedsDemand);
        }
        Ok(())
    }

    fn task_rows_complete(&self) -> bool {
        self.task_counters_complete && self.tasks.len() == self.declared_task_count as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskUtilityTargets {
    /// Treatment on-time task value / control on-time task value, in PPM.
    pub minimum_on_time_value_retention_ppm: u64,
    /// Treatment cost per timely useful MiB / control cost per timely useful MiB.
    pub maximum_cost_per_on_time_mib_ratio_ppm: u64,
    /// Treatment energy per timely useful MiB / control energy per timely useful MiB.
    pub maximum_energy_per_on_time_mib_ratio_ppm: u64,
    pub minimum_companion_retention_ppm: u64,
    /// Maximum permitted increase in task-level deadline-miss count.
    pub maximum_deadline_miss_increase: u32,
    pub maximum_resource_overcommit_bps: u64,
}

impl Default for TaskUtilityTargets {
    fn default() -> Self {
        Self {
            minimum_on_time_value_retention_ppm: 1_000_000,
            maximum_cost_per_on_time_mib_ratio_ppm: 1_000_000,
            maximum_energy_per_on_time_mib_ratio_ppm: 1_000_000,
            minimum_companion_retention_ppm: 900_000,
            maximum_deadline_miss_increase: 0,
            maximum_resource_overcommit_bps: 0,
        }
    }
}

impl TaskUtilityTargets {
    fn validate(self) -> Result<(), TaskUtilityError> {
        for target in [
            self.minimum_on_time_value_retention_ppm,
            self.maximum_cost_per_on_time_mib_ratio_ppm,
            self.maximum_energy_per_on_time_mib_ratio_ppm,
            self.minimum_companion_retention_ppm,
        ] {
            if target == 0 || target > MAX_RATIO_PPM {
                return Err(TaskUtilityError::InvalidTarget);
            }
        }
        if self.minimum_companion_retention_ppm > PPM as u64 {
            return Err(TaskUtilityError::InvalidTarget);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UtilityDisposition {
    Pass,
    Fail,
    Inconclusive,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UtilityGateResult {
    pub gate_id: &'static str,
    pub disposition: UtilityDisposition,
    pub reason: &'static str,
    pub target_value: Option<u64>,
    pub observed_value: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskTrialMetrics {
    pub observed_task_count: u32,
    pub offered_useful_bytes: u128,
    pub on_time_useful_bytes: u128,
    pub late_useful_bytes: u128,
    pub offered_value_units: u128,
    pub on_time_value_units: u128,
    pub on_time_useful_byte_ratio_ppm: Option<u64>,
    pub on_time_task_value_ratio_ppm: Option<u64>,
    pub deadline_miss_count: u32,
    pub deadline_miss_lateness_p50_us: Option<u64>,
    pub deadline_miss_lateness_p95_us: Option<u64>,
    pub deadline_miss_lateness_p99_us: Option<u64>,
    pub cost_per_on_time_useful_mib_micros: Option<u64>,
    pub energy_per_on_time_useful_mib_uwh: Option<u64>,
    pub redundant_bytes_per_on_time_useful_mib: Option<u64>,
    pub retransmitted_bytes_per_on_time_useful_mib: Option<u64>,
    pub aggregate_goodput_bps: u64,
    pub bottleneck_capacity_bps: u64,
    pub resource_overcommit_bps: u64,
    pub queue_delay_p95_us: u64,
    pub queue_delay_p99_us: u64,
    pub receiver_reorder_peak_bytes: u64,
    pub companion_demand_bps: u64,
    pub companion_goodput_bps: u64,
    pub companion_backlogged: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskUtilityReport {
    /// Every disposition is scoped to this one exact paired trial.
    pub scope: &'static str,
    pub environment_identity: TaskEnvironmentIdentity,
    pub control_arm_identity: TransportArmIdentity,
    pub treatment_arm_identity: TransportArmIdentity,
    pub control_raw_evidence_sha256: String,
    pub treatment_raw_evidence_sha256: String,
    pub control_measurement_receipt_sha256: String,
    pub treatment_measurement_receipt_sha256: String,
    pub control_metrics: Option<TaskTrialMetrics>,
    pub treatment_metrics: Option<TaskTrialMetrics>,
    pub on_time_value_retention_ppm: Option<u64>,
    pub cost_per_on_time_mib_ratio_ppm: Option<u64>,
    pub energy_per_on_time_mib_ratio_ppm: Option<u64>,
    pub companion_retention_ppm: Option<u64>,
    pub deadline_miss_delta: Option<i64>,
    pub gates: Vec<UtilityGateResult>,
    pub overall_disposition: UtilityDisposition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskUtilityError {
    InvalidDigest(&'static str),
    InvalidImplementationIdentity(&'static str),
    ZeroMeasurementDuration,
    InvalidWarmupDuration,
    InvalidTaskId,
    InvalidTaskWindow,
    ZeroTaskDemand,
    TaskBytesExceedDemand,
    TaskValueExceedsDemand,
    ZeroDeadlineMissLateness,
    InvalidTaskCount,
    DuplicateTaskId,
    UsefulBytesExceedTransmitted,
    TransmissionAccountingInvalid,
    ZeroBottleneckCapacity,
    InvalidCompanionDemand,
    CompanionGoodputExceedsDemand,
    InvalidTarget,
    ArithmeticOverflow,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn fixed_ratio_ppm(numerator: u128, denominator: u128) -> Result<u64, TaskUtilityError> {
    if denominator == 0 {
        return Err(TaskUtilityError::ArithmeticOverflow);
    }
    let scaled = numerator
        .checked_mul(PPM)
        .ok_or(TaskUtilityError::ArithmeticOverflow)?
        / denominator;
    u64::try_from(scaled).map_err(|_| TaskUtilityError::ArithmeticOverflow)
}

fn per_mib_units(units: u64, useful_bytes: u128) -> Result<Option<u64>, TaskUtilityError> {
    if useful_bytes == 0 {
        return Ok(None);
    }
    let numerator = u128::from(units)
        .checked_mul(MIB_BYTES)
        .ok_or(TaskUtilityError::ArithmeticOverflow)?;
    let value = numerator / useful_bytes;
    Ok(Some(
        u64::try_from(value).map_err(|_| TaskUtilityError::ArithmeticOverflow)?,
    ))
}

fn percentile_nearest_rank(values: &mut [u64], percentile: usize) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let rank = values.len().saturating_mul(percentile).div_ceil(100).max(1);
    values.get(rank.saturating_sub(1)).copied()
}

fn calculate_metrics(trial: &TaskTrial) -> Result<TaskTrialMetrics, TaskUtilityError> {
    let mut offered_useful_bytes = 0_u128;
    let mut on_time_useful_bytes = 0_u128;
    let mut late_useful_bytes = 0_u128;
    let mut offered_value_units = 0_u128;
    let mut on_time_value_units = 0_u128;
    let mut deadline_misses = Vec::new();

    for task in &trial.tasks {
        offered_useful_bytes = offered_useful_bytes
            .checked_add(u128::from(task.offered_useful_bytes))
            .ok_or(TaskUtilityError::ArithmeticOverflow)?;
        on_time_useful_bytes = on_time_useful_bytes
            .checked_add(u128::from(task.on_time_useful_bytes))
            .ok_or(TaskUtilityError::ArithmeticOverflow)?;
        late_useful_bytes = late_useful_bytes
            .checked_add(u128::from(task.late_useful_bytes))
            .ok_or(TaskUtilityError::ArithmeticOverflow)?;
        offered_value_units = offered_value_units
            .checked_add(u128::from(task.offered_value_units))
            .ok_or(TaskUtilityError::ArithmeticOverflow)?;
        on_time_value_units = on_time_value_units
            .checked_add(u128::from(task.on_time_value_units))
            .ok_or(TaskUtilityError::ArithmeticOverflow)?;
        if let Some(lateness) = task.deadline_miss_lateness_us {
            deadline_misses.push(lateness);
        }
    }

    let deadline_miss_count = u32::try_from(deadline_misses.len())
        .map_err(|_| TaskUtilityError::ArithmeticOverflow)?;
    let on_time_useful_byte_ratio_ppm =
        Some(fixed_ratio_ppm(on_time_useful_bytes, offered_useful_bytes)?);
    let on_time_task_value_ratio_ppm =
        Some(fixed_ratio_ppm(on_time_value_units, offered_value_units)?);

    Ok(TaskTrialMetrics {
        observed_task_count: u32::try_from(trial.tasks.len())
            .map_err(|_| TaskUtilityError::ArithmeticOverflow)?,
        offered_useful_bytes,
        on_time_useful_bytes,
        late_useful_bytes,
        offered_value_units,
        on_time_value_units,
        on_time_useful_byte_ratio_ppm,
        on_time_task_value_ratio_ppm,
        deadline_miss_count,
        deadline_miss_lateness_p50_us: percentile_nearest_rank(&mut deadline_misses.clone(), 50),
        deadline_miss_lateness_p95_us: percentile_nearest_rank(&mut deadline_misses.clone(), 95),
        deadline_miss_lateness_p99_us: percentile_nearest_rank(&mut deadline_misses, 99),
        cost_per_on_time_useful_mib_micros: per_mib_units(
            trial.workload_cost_micros,
            on_time_useful_bytes,
        )?,
        energy_per_on_time_useful_mib_uwh: per_mib_units(
            trial.workload_energy_uwh,
            on_time_useful_bytes,
        )?,
        redundant_bytes_per_on_time_useful_mib: per_mib_units(
            trial.duplicate_bytes,
            on_time_useful_bytes,
        )?,
        retransmitted_bytes_per_on_time_useful_mib: per_mib_units(
            trial.retransmitted_bytes,
            on_time_useful_bytes,
        )?,
        aggregate_goodput_bps: trial.aggregate_goodput_bps,
        bottleneck_capacity_bps: trial.bottleneck_capacity_bps,
        resource_overcommit_bps: trial.max_resource_overcommit_bps,
        queue_delay_p95_us: trial.queue_delay_p95_us,
        queue_delay_p99_us: trial.queue_delay_p99_us,
        receiver_reorder_peak_bytes: trial.receiver_reorder_peak_bytes,
        companion_demand_bps: trial.companion_demand_bps,
        companion_goodput_bps: trial.companion_goodput_bps,
        companion_backlogged: trial.companion_backlogged,
    })
}

fn task_manifest_matches(left: &TaskTrial, right: &TaskTrial) -> bool {
    if left.tasks.len() != right.tasks.len()
        || left.declared_task_count != right.declared_task_count
        || !left.task_counters_complete
        || !right.task_counters_complete
        || left.tasks.len() != left.declared_task_count as usize
        || right.tasks.len() != right.declared_task_count as usize
    {
        return false;
    }
    let left_rows = left
        .tasks
        .iter()
        .map(|task| {
            (
                task.task_id.as_str(),
                task.release_offset_us,
                task.deadline_offset_us,
                task.offered_useful_bytes,
                task.offered_value_units,
            )
        })
        .collect::<BTreeSet<_>>();
    let right_rows = right
        .tasks
        .iter()
        .map(|task| {
            (
                task.task_id.as_str(),
                task.release_offset_us,
                task.deadline_offset_us,
                task.offered_useful_bytes,
                task.offered_value_units,
            )
        })
        .collect::<BTreeSet<_>>();
    left_rows == right_rows
}

fn gate(
    gate_id: &'static str,
    disposition: UtilityDisposition,
    reason: &'static str,
    target_value: Option<u64>,
    observed_value: Option<u64>,
) -> UtilityGateResult {
    UtilityGateResult {
        gate_id,
        disposition,
        reason,
        target_value,
        observed_value,
    }
}

fn overall_disposition(gates: &[UtilityGateResult]) -> UtilityDisposition {
    if gates.iter().any(|item| item.disposition == UtilityDisposition::Fail) {
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
    }
}

fn inconclusive_report(
    control: &TaskTrial,
    treatment: &TaskTrial,
    control_metrics: Option<TaskTrialMetrics>,
    treatment_metrics: Option<TaskTrialMetrics>,
    reason: &'static str,
) -> TaskUtilityReport {
    let gates = vec![
        gate(
            "TASK_PAIR_IDENTITY",
            UtilityDisposition::Inconclusive,
            reason,
            None,
            None,
        ),
        gate(
            "TASK_ON_TIME_VALUE",
            UtilityDisposition::Inconclusive,
            reason,
            None,
            None,
        ),
        gate(
            "TASK_COST_PER_ON_TIME_MIB",
            UtilityDisposition::Inconclusive,
            reason,
            None,
            None,
        ),
        gate(
            "TASK_ENERGY_PER_ON_TIME_MIB",
            UtilityDisposition::Inconclusive,
            reason,
            None,
            None,
        ),
        gate(
            "TASK_COMPANION_RETENTION",
            UtilityDisposition::Inconclusive,
            reason,
            None,
            None,
        ),
        gate(
            "TASK_DEADLINE_MISSES",
            UtilityDisposition::Inconclusive,
            reason,
            None,
            None,
        ),
        gate(
            "TASK_RESOURCE_OVERCOMMIT",
            UtilityDisposition::Inconclusive,
            reason,
            None,
            None,
        ),
        gate(
            "TASK_CLEANUP",
            UtilityDisposition::Inconclusive,
            reason,
            None,
            None,
        ),
    ];
    TaskUtilityReport {
        scope: "single_paired_trial",
        environment_identity: control.environment.clone(),
        control_arm_identity: control.arm.clone(),
        treatment_arm_identity: treatment.arm.clone(),
        control_raw_evidence_sha256: control.raw_evidence_sha256.clone(),
        treatment_raw_evidence_sha256: treatment.raw_evidence_sha256.clone(),
        control_measurement_receipt_sha256: control.measurement_receipt_sha256.clone(),
        treatment_measurement_receipt_sha256: treatment.measurement_receipt_sha256.clone(),
        control_metrics,
        treatment_metrics,
        on_time_value_retention_ppm: None,
        cost_per_on_time_mib_ratio_ppm: None,
        energy_per_on_time_mib_ratio_ppm: None,
        companion_retention_ppm: None,
        deadline_miss_delta: None,
        gates,
        overall_disposition: UtilityDisposition::Inconclusive,
    }
}

/// Compare two measured transport arms over the exact same task manifest and
/// environment. A passing report describes one paired trial only.
pub fn evaluate_task_utility_pair(
    control: &TaskTrial,
    treatment: &TaskTrial,
    targets: TaskUtilityTargets,
) -> Result<TaskUtilityReport, TaskUtilityError> {
    control.validate()?;
    treatment.validate()?;
    targets.validate()?;

    let control_metrics = if control.task_rows_complete() {
        Some(calculate_metrics(control)?)
    } else {
        None
    };
    let treatment_metrics = if treatment.task_rows_complete() {
        Some(calculate_metrics(treatment)?)
    } else {
        None
    };

    if !control.arm_supported || !treatment.arm_supported {
        let mut report = inconclusive_report(
            control,
            treatment,
            control_metrics,
            treatment_metrics,
            "one_or_both_transport_arms_unsupported",
        );
        report.gates[0] = gate(
            "TASK_PAIR_IDENTITY",
            UtilityDisposition::Unsupported,
            "one_or_both_transport_arms_unsupported",
            None,
            None,
        );
        report.overall_disposition = UtilityDisposition::Unsupported;
        return Ok(report);
    }

    if control.environment != treatment.environment {
        return Ok(inconclusive_report(
            control,
            treatment,
            control_metrics,
            treatment_metrics,
            "control_and_treatment_environment_identity_mismatch",
        ));
    }
    if !task_manifest_matches(control, treatment) {
        return Ok(inconclusive_report(
            control,
            treatment,
            control_metrics,
            treatment_metrics,
            "control_and_treatment_task_manifest_mismatch_or_incomplete",
        ));
    }

    let (Some(control_metrics), Some(treatment_metrics)) =
        (control_metrics, treatment_metrics)
    else {
        return Ok(inconclusive_report(
            control,
            treatment,
            None,
            None,
            "task_observation_denominator_incomplete",
        ));
    };

    let mut gates = Vec::new();
    gates.push(gate(
        "TASK_PAIR_IDENTITY",
        UtilityDisposition::Pass,
        "environment_and_task_manifest_match_exactly",
        Some(1),
        Some(1),
    ));

    let cleanup_ok = control.cleanup_verified && treatment.cleanup_verified;
    gates.push(gate(
        "TASK_CLEANUP",
        if cleanup_ok {
            UtilityDisposition::Pass
        } else {
            UtilityDisposition::Fail
        },
        if cleanup_ok {
            "both_paired_trial_cleanups_verified"
        } else {
            "one_or_both_paired_trial_cleanups_unverified"
        },
        Some(1),
        Some(u64::from(cleanup_ok)),
    ));

    let on_time_value_retention_ppm = if control_metrics.on_time_value_units == 0 {
        None
    } else {
        Some(fixed_ratio_ppm(
            treatment_metrics.on_time_value_units,
            control_metrics.on_time_value_units,
        )?)
    };
    gates.push(match on_time_value_retention_ppm {
        Some(value) => gate(
            "TASK_ON_TIME_VALUE",
            if value >= targets.minimum_on_time_value_retention_ppm {
                UtilityDisposition::Pass
            } else {
                UtilityDisposition::Fail
            },
            if value >= targets.minimum_on_time_value_retention_ppm {
                "treatment_meets_on_time_task_value_non_regression"
            } else {
                "treatment_on_time_task_value_below_control_target"
            },
            Some(targets.minimum_on_time_value_retention_ppm),
            Some(value),
        ),
        None => gate(
            "TASK_ON_TIME_VALUE",
            UtilityDisposition::Inconclusive,
            "control_delivered_zero_on_time_task_value",
            Some(targets.minimum_on_time_value_retention_ppm),
            None,
        ),
    });

    let cost_complete = control.cost_energy_counters_complete
        && treatment.cost_energy_counters_complete;
    let cost_ratio = if cost_complete
        && control_metrics.on_time_useful_bytes > 0
        && treatment_metrics.on_time_useful_bytes > 0
        && control.workload_cost_micros > 0
    {
        let numerator = u128::from(treatment.workload_cost_micros)
            .checked_mul(control_metrics.on_time_useful_bytes)
            .ok_or(TaskUtilityError::ArithmeticOverflow)?;
        let denominator = u128::from(control.workload_cost_micros)
            .checked_mul(treatment_metrics.on_time_useful_bytes)
            .ok_or(TaskUtilityError::ArithmeticOverflow)?;
        Some(fixed_ratio_ppm(numerator, denominator)?)
    } else {
        None
    };
    let cost_gate = if !cost_complete {
        gate(
            "TASK_COST_PER_ON_TIME_MIB",
            UtilityDisposition::Inconclusive,
            "cost_or_energy_counters_incomplete",
            Some(targets.maximum_cost_per_on_time_mib_ratio_ppm),
            None,
        )
    } else if treatment_metrics.on_time_useful_bytes == 0
        && control_metrics.on_time_useful_bytes > 0
    {
        gate(
            "TASK_COST_PER_ON_TIME_MIB",
            UtilityDisposition::Fail,
            "treatment_delivered_no_on_time_useful_bytes",
            Some(targets.maximum_cost_per_on_time_mib_ratio_ppm),
            None,
        )
    } else if control_metrics.on_time_useful_bytes == 0
        || control.workload_cost_micros == 0
    {
        gate(
            "TASK_COST_PER_ON_TIME_MIB",
            UtilityDisposition::Inconclusive,
            "control_cost_or_on_time_useful_bytes_is_zero",
            Some(targets.maximum_cost_per_on_time_mib_ratio_ppm),
            None,
        )
    } else if let Some(value) = cost_ratio {
        gate(
            "TASK_COST_PER_ON_TIME_MIB",
            if value <= targets.maximum_cost_per_on_time_mib_ratio_ppm {
                UtilityDisposition::Pass
            } else {
                UtilityDisposition::Fail
            },
            if value <= targets.maximum_cost_per_on_time_mib_ratio_ppm {
                "treatment_cost_per_on_time_mib_meets_target"
            } else {
                "treatment_cost_per_on_time_mib_exceeds_target"
            },
            Some(targets.maximum_cost_per_on_time_mib_ratio_ppm),
            Some(value),
        )
    } else {
        gate(
            "TASK_COST_PER_ON_TIME_MIB",
            UtilityDisposition::Inconclusive,
            "cost_ratio_not_computable",
            Some(targets.maximum_cost_per_on_time_mib_ratio_ppm),
            None,
        )
    };
    gates.push(cost_gate);

    let energy_ratio = if cost_complete
        && control_metrics.on_time_useful_bytes > 0
        && treatment_metrics.on_time_useful_bytes > 0
        && control.workload_energy_uwh > 0
    {
        let numerator = u128::from(treatment.workload_energy_uwh)
            .checked_mul(control_metrics.on_time_useful_bytes)
            .ok_or(TaskUtilityError::ArithmeticOverflow)?;
        let denominator = u128::from(control.workload_energy_uwh)
            .checked_mul(treatment_metrics.on_time_useful_bytes)
            .ok_or(TaskUtilityError::ArithmeticOverflow)?;
        Some(fixed_ratio_ppm(numerator, denominator)?)
    } else {
        None
    };
    let energy_gate = if !cost_complete {
        gate(
            "TASK_ENERGY_PER_ON_TIME_MIB",
            UtilityDisposition::Inconclusive,
            "cost_or_energy_counters_incomplete",
            Some(targets.maximum_energy_per_on_time_mib_ratio_ppm),
            None,
        )
    } else if treatment_metrics.on_time_useful_bytes == 0
        && control_metrics.on_time_useful_bytes > 0
    {
        gate(
            "TASK_ENERGY_PER_ON_TIME_MIB",
            UtilityDisposition::Fail,
            "treatment_delivered_no_on_time_useful_bytes",
            Some(targets.maximum_energy_per_on_time_mib_ratio_ppm),
            None,
        )
    } else if control_metrics.on_time_useful_bytes == 0
        || control.workload_energy_uwh == 0
    {
        gate(
            "TASK_ENERGY_PER_ON_TIME_MIB",
            UtilityDisposition::Inconclusive,
            "control_energy_or_on_time_useful_bytes_is_zero",
            Some(targets.maximum_energy_per_on_time_mib_ratio_ppm),
            None,
        )
    } else if let Some(value) = energy_ratio {
        gate(
            "TASK_ENERGY_PER_ON_TIME_MIB",
            if value <= targets.maximum_energy_per_on_time_mib_ratio_ppm {
                UtilityDisposition::Pass
            } else {
                UtilityDisposition::Fail
            },
            if value <= targets.maximum_energy_per_on_time_mib_ratio_ppm {
                "treatment_energy_per_on_time_mib_meets_target"
            } else {
                "treatment_energy_per_on_time_mib_exceeds_target"
            },
            Some(targets.maximum_energy_per_on_time_mib_ratio_ppm),
            Some(value),
        )
    } else {
        gate(
            "TASK_ENERGY_PER_ON_TIME_MIB",
            UtilityDisposition::Inconclusive,
            "energy_ratio_not_computable",
            Some(targets.maximum_energy_per_on_time_mib_ratio_ppm),
            None,
        )
    };
    gates.push(energy_gate);

    let companion_pair_valid = control.companion_counters_complete
        && treatment.companion_counters_complete
        && control.companion_backlogged
        && treatment.companion_backlogged
        && control.companion_demand_bps == treatment.companion_demand_bps
        && control.companion_demand_bps > 0;
    let companion_retention_ppm = if companion_pair_valid && control.companion_goodput_bps > 0 {
        Some(fixed_ratio_ppm(
            u128::from(treatment.companion_goodput_bps),
            u128::from(control.companion_goodput_bps),
        )?)
    } else {
        None
    };
    gates.push(match companion_retention_ppm {
        Some(value) => gate(
            "TASK_COMPANION_RETENTION",
            if value >= targets.minimum_companion_retention_ppm {
                UtilityDisposition::Pass
            } else {
                UtilityDisposition::Fail
            },
            if value >= targets.minimum_companion_retention_ppm {
                "companion_flow_retention_meets_floor"
            } else {
                "companion_flow_retention_below_floor"
            },
            Some(targets.minimum_companion_retention_ppm),
            Some(value),
        ),
        None => gate(
            "TASK_COMPANION_RETENTION",
            UtilityDisposition::Inconclusive,
            if !control.companion_counters_complete || !treatment.companion_counters_complete {
                "companion_flow_counters_incomplete"
            } else if !control.companion_backlogged || !treatment.companion_backlogged {
                "companion_flow_not_backlogged_in_both_arms"
            } else if control.companion_demand_bps != treatment.companion_demand_bps {
                "companion_flow_demand_mismatch"
            } else {
                "control_companion_goodput_is_zero"
            },
            Some(targets.minimum_companion_retention_ppm),
            None,
        ),
    });

    let deadline_miss_delta = i64::from(treatment_metrics.deadline_miss_count)
        - i64::from(control_metrics.deadline_miss_count);
    gates.push(gate(
        "TASK_DEADLINE_MISSES",
        if deadline_miss_delta <= i64::from(targets.maximum_deadline_miss_increase) {
            UtilityDisposition::Pass
        } else {
            UtilityDisposition::Fail
        },
        if deadline_miss_delta <= i64::from(targets.maximum_deadline_miss_increase) {
            "deadline_miss_count_meets_declared_non_regression_target"
        } else {
            "deadline_miss_count_increased_beyond_target"
        },
        Some(u64::from(targets.maximum_deadline_miss_increase)),
        Some(u64::try_from(deadline_miss_delta.max(0)).unwrap_or(u64::MAX)),
    ));

    let resource_complete =
        control.resource_accounting_complete && treatment.resource_accounting_complete;
    let maximum_overcommit = control
        .max_resource_overcommit_bps
        .max(treatment.max_resource_overcommit_bps);
    gates.push(if !resource_complete {
        gate(
            "TASK_RESOURCE_OVERCOMMIT",
            UtilityDisposition::Inconclusive,
            "shared_resource_accounting_incomplete",
            Some(targets.maximum_resource_overcommit_bps),
            None,
        )
    } else {
        gate(
            "TASK_RESOURCE_OVERCOMMIT",
            if maximum_overcommit <= targets.maximum_resource_overcommit_bps {
                UtilityDisposition::Pass
            } else {
                UtilityDisposition::Fail
            },
            if maximum_overcommit <= targets.maximum_resource_overcommit_bps {
                "shared_resource_capacity_within_declared_ceiling"
            } else {
                "shared_resource_capacity_overcommitted"
            },
            Some(targets.maximum_resource_overcommit_bps),
            Some(maximum_overcommit),
        )
    });

    let overall = overall_disposition(&gates);
    Ok(TaskUtilityReport {
        scope: "single_paired_trial",
        environment_identity: control.environment.clone(),
        control_arm_identity: control.arm.clone(),
        treatment_arm_identity: treatment.arm.clone(),
        control_raw_evidence_sha256: control.raw_evidence_sha256.clone(),
        treatment_raw_evidence_sha256: treatment.raw_evidence_sha256.clone(),
        control_measurement_receipt_sha256: control.measurement_receipt_sha256.clone(),
        treatment_measurement_receipt_sha256: treatment.measurement_receipt_sha256.clone(),
        control_metrics: Some(control_metrics),
        treatment_metrics: Some(treatment_metrics),
        on_time_value_retention_ppm,
        cost_per_on_time_mib_ratio_ppm: cost_ratio,
        energy_per_on_time_mib_ratio_ppm: energy_ratio,
        companion_retention_ppm,
        deadline_miss_delta: Some(deadline_miss_delta),
        gates,
        overall_disposition: overall,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment() -> TaskEnvironmentIdentity {
        TaskEnvironmentIdentity {
            kernel_build_sha256: "a".repeat(64),
            system_image_sha256: "b".repeat(64),
            topology_sha256: "c".repeat(64),
            resource_snapshot_sha256: "d".repeat(64),
            qdisc_config_sha256: "e".repeat(64),
            workload_sha256: "f".repeat(64),
            task_manifest_sha256: "1".repeat(64),
            impairment_sha256: "2".repeat(64),
            tariff_sha256: "3".repeat(64),
            energy_model_sha256: "4".repeat(64),
            observer_config_sha256: "5".repeat(64),
            seed: 42,
            warmup_us: 1_000_000,
            measurement_duration_us: 31_000_000,
        }
    }

    fn arm(protocol: TransportProtocol, name: &str) -> TransportArmIdentity {
        TransportArmIdentity {
            protocol,
            transport_version: "pinned-v1".to_owned(),
            scheduler_id: name.to_owned(),
            congestion_control_id: "cubic".to_owned(),
            path_manager_id: "kernel".to_owned(),
            implementation_sha256: "6".repeat(64),
            arm_config_sha256: "7".repeat(64),
        }
    }

    fn task(id: &str, on_time_bytes: u64, value: u64, lateness: Option<u64>) -> TaskObservation {
        TaskObservation {
            task_id: id.to_owned(),
            release_offset_us: 0,
            deadline_offset_us: 1_000_000,
            offered_useful_bytes: 10_000,
            on_time_useful_bytes: on_time_bytes,
            late_useful_bytes: 1_000,
            offered_value_units: 100,
            on_time_value_units: value,
            deadline_miss_lateness_us: lateness,
        }
    }

    fn trial(
        protocol: TransportProtocol,
        name: &str,
        task_rows: Vec<TaskObservation>,
        on_time_complete: bool,
    ) -> TaskTrial {
        TaskTrial {
            environment: environment(),
            arm: arm(protocol, name),
            declared_task_count: u32::try_from(task_rows.len()).unwrap_or(1),
            tasks: task_rows,
            arm_supported: true,
            task_counters_complete: on_time_complete,
            cost_energy_counters_complete: true,
            resource_accounting_complete: true,
            companion_counters_complete: true,
            cleanup_verified: true,
            workload_cost_micros: 90,
            workload_energy_uwh: 80,
            total_transmitted_bytes: 1_000_000,
            duplicate_bytes: 1_000,
            retransmitted_bytes: 2_000,
            max_resource_overcommit_bps: 0,
            aggregate_goodput_bps: 80_000_000,
            bottleneck_capacity_bps: 100_000_000,
            queue_delay_p95_us: 10_000,
            queue_delay_p99_us: 20_000,
            receiver_reorder_peak_bytes: 4_096,
            companion_demand_bps: 50_000_000,
            companion_goodput_bps: 45_000_000,
            companion_backlogged: true,
            raw_evidence_sha256: "8".repeat(64),
            measurement_receipt_sha256: "9".repeat(64),
        }
    }

    fn passing_pair() -> (TaskTrial, TaskTrial) {
        let control = trial(
            TransportProtocol::Mptcp,
            "linux-default",
            vec![task("task-a", 9_000, 90, None), task("task-b", 9_000, 90, None)],
            true,
        );
        let mut treatment = trial(
            TransportProtocol::MultipathQuic,
            "minrtt",
            vec![task("task-a", 9_500, 95, None), task("task-b", 9_500, 95, None)],
            true,
        );
        treatment.companion_goodput_bps = 48_000_000;
        (control, treatment)
    }

    #[test]
    fn matched_paired_trial_passes_all_declared_utility_gates() {
        let (control, treatment) = passing_pair();
        let report =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("valid paired trial");
        assert_eq!(report.scope, "single_paired_trial");
        assert_eq!(report.overall_disposition, UtilityDisposition::Pass);
        assert_eq!(report.on_time_value_retention_ppm, Some(1_055_555));
        assert_eq!(report.companion_retention_ppm, Some(1_066_666));
    }

    #[test]
    fn environment_digest_mismatch_is_inconclusive() {
        let (control, mut treatment) = passing_pair();
        treatment.environment.resource_snapshot_sha256 = "8".repeat(64);
        let report =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("mismatched pairs are classified");
        assert_eq!(report.overall_disposition, UtilityDisposition::Inconclusive);
        assert_eq!(
            report.gates[0].reason,
            "control_and_treatment_environment_identity_mismatch"
        );
    }

    #[test]
    fn task_deadline_or_value_manifest_mismatch_is_inconclusive() {
        let (control, mut treatment) = passing_pair();
        treatment.tasks[0].deadline_offset_us = 900_000;
        let report =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("task mismatch is classified");
        assert_eq!(report.overall_disposition, UtilityDisposition::Inconclusive);
    }

    #[test]
    fn on_time_value_regression_fails_even_when_raw_goodput_is_high() {
        let (control, mut treatment) = passing_pair();
        treatment.tasks[0].on_time_value_units = 50;
        treatment.tasks[1].on_time_value_units = 50;
        let report =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("valid pair");
        assert_eq!(
            report.gates.iter().find(|item| item.gate_id == "TASK_ON_TIME_VALUE").unwrap().disposition,
            UtilityDisposition::Fail
        );
    }

    #[test]
    fn cost_per_useful_on_time_mib_regression_fails() {
        let (control, mut treatment) = passing_pair();
        treatment.workload_cost_micros = 200;
        let report =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("valid pair");
        assert_eq!(
            report.gates.iter().find(|item| item.gate_id == "TASK_COST_PER_ON_TIME_MIB").unwrap().disposition,
            UtilityDisposition::Fail
        );
    }

    #[test]
    fn energy_per_useful_on_time_mib_regression_fails() {
        let (control, mut treatment) = passing_pair();
        treatment.workload_energy_uwh = 200;
        let report =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("valid pair");
        assert_eq!(
            report.gates.iter().find(|item| item.gate_id == "TASK_ENERGY_PER_ON_TIME_MIB").unwrap().disposition,
            UtilityDisposition::Fail
        );
    }

    #[test]
    fn companion_flow_starvation_fails_even_when_tasks_improve() {
        let (control, mut treatment) = passing_pair();
        treatment.companion_goodput_bps = 10_000_000;
        let report =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("valid pair");
        assert_eq!(
            report.gates.iter().find(|item| item.gate_id == "TASK_COMPANION_RETENTION").unwrap().disposition,
            UtilityDisposition::Fail
        );
    }

    #[test]
    fn resource_overcommit_fails_independently_of_task_utility() {
        let (control, mut treatment) = passing_pair();
        treatment.max_resource_overcommit_bps = 1;
        let report =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("valid pair");
        assert_eq!(report.gates[6].disposition, UtilityDisposition::Fail);
    }

    #[test]
    fn unverified_cleanup_fails() {
        let (control, mut treatment) = passing_pair();
        treatment.cleanup_verified = false;
        let report =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("valid pair");
        assert_eq!(
            report.gates.iter().find(|item| item.gate_id == "TASK_CLEANUP").unwrap().disposition,
            UtilityDisposition::Fail
        );
    }

    #[test]
    fn missing_task_counters_are_inconclusive_not_zero() {
        let (control, mut treatment) = passing_pair();
        treatment.task_counters_complete = false;
        let report =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("incomplete task counters are classified");
        assert_eq!(report.overall_disposition, UtilityDisposition::Inconclusive);
        assert!(report.treatment_metrics.is_none());
    }

    #[test]
    fn unsupported_transport_arm_is_not_a_pass() {
        let (control, mut treatment) = passing_pair();
        treatment.arm_supported = false;
        let report =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("unsupported arm is classified");
        assert_eq!(report.overall_disposition, UtilityDisposition::Unsupported);
    }

    #[test]
    fn companion_gate_is_inconclusive_when_no_companion_is_backlogged() {
        let (mut control, mut treatment) = passing_pair();
        control.companion_backlogged = false;
        treatment.companion_backlogged = false;
        control.companion_demand_bps = 0;
        treatment.companion_demand_bps = 0;
        control.companion_goodput_bps = 0;
        treatment.companion_goodput_bps = 0;
        let report =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("no companion load is not a fairness pass");
        assert_eq!(
            report.gates.iter().find(|item| item.gate_id == "TASK_COMPANION_RETENTION").unwrap().disposition,
            UtilityDisposition::Inconclusive
        );
    }

    #[test]
    fn task_rows_are_order_independent() {
        let (control, mut treatment) = passing_pair();
        treatment.tasks.reverse();
        let first =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("valid pair");
        let second =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("same inputs");
        assert_eq!(first, second);
    }

    #[test]
    fn task_manifest_duplicate_ids_are_rejected() {
        let (mut control, treatment) = passing_pair();
        control.tasks[1].task_id = control.tasks[0].task_id.clone();
        assert_eq!(
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default()),
            Err(TaskUtilityError::DuplicateTaskId)
        );
    }

    #[test]
    fn value_or_bytes_above_offered_demand_are_rejected() {
        let (mut control, treatment) = passing_pair();
        control.tasks[0].on_time_value_units = 101;
        assert_eq!(
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default()),
            Err(TaskUtilityError::TaskValueExceedsDemand)
        );
    }

    #[test]
    fn malformed_environment_digest_is_rejected() {
        let (mut control, treatment) = passing_pair();
        control.environment.task_manifest_sha256 = "bad".to_owned();
        assert_eq!(
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default()),
            Err(TaskUtilityError::InvalidDigest("task_manifest_sha256"))
        );
    }

    #[test]
    fn late_useful_bytes_do_not_increase_on_time_value_or_bytes() {
        let (mut control, treatment) = passing_pair();
        control.tasks[0].on_time_useful_bytes = 0;
        control.tasks[0].late_useful_bytes = 10_000;
        let report =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("valid task evidence");
        assert_eq!(
            report.control_metrics.as_ref().unwrap().on_time_useful_bytes,
            9_000
        );
        assert_eq!(
            report.control_metrics.as_ref().unwrap().late_useful_bytes,
            11_000
        );
    }

    #[test]
    fn non_monotonic_task_values_are_not_credited() {
        let (mut control, treatment) = passing_pair();
        control.tasks[0].on_time_value_units = 110;
        assert_eq!(
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default()),
            Err(TaskUtilityError::TaskValueExceedsDemand)
        );
    }

    #[test]
    fn invalid_targets_are_rejected() {
        let (control, treatment) = passing_pair();
        let targets = TaskUtilityTargets {
            minimum_on_time_value_retention_ppm: 0,
            ..TaskUtilityTargets::default()
        };
        assert_eq!(
            evaluate_task_utility_pair(&control, &treatment, targets),
            Err(TaskUtilityError::InvalidTarget)
        );
    }

    #[test]
    fn repeated_evaluation_is_deterministic() {
        let (control, treatment) = passing_pair();
        let first =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("valid pair");
        let second =
            evaluate_task_utility_pair(&control, &treatment, TaskUtilityTargets::default())
                .expect("same pair");
        assert_eq!(first, second);
    }
}
