//! Evidence evaluator for deadline-qualified application utility over multipath.
//!
//! This module consumes paired measured trial summaries. It does not schedule
//! packets, select paths, optimize congestion control or actuate a kernel.
//! It keeps transport identity separate from the frozen network/workload
//! environment and classifies one pair only; programme-level confidence
//! intervals and repeated-trial analysis remain external.

use std::collections::BTreeMap;

const PPM: u128 = 1_000_000;
const BYTES_PER_MIB: u128 = 1_048_576;
const MAX_TASKS: usize = 4_096;
const MAX_COMPANION_FLOWS: usize = 256;
const MAX_PATHS: u16 = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UtilityImplementationProvenance {
    /// Component is included in the binary or kernel build identified by
    /// UtilityArmIdentity::implementation_sha256.
    IncludedInImplementation,
    /// Component is loaded separately and has its own exact artifact digest.
    ExternalArtifact { artifact_sha256: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UtilityEnvironmentIdentity {
    pub kernel_build_sha256: String,
    pub topology_sha256: String,
    pub resource_snapshot_sha256: String,
    pub qdisc_config_sha256: String,
    pub workload_sha256: String,
    pub impairment_sha256: String,
    pub seed: u64,
    pub warmup_us: u64,
    pub measurement_duration_us: u64,
}

impl UtilityEnvironmentIdentity {
    fn validate(&self) -> Result<(), UtilityError> {
        for (name, digest) in [
            ("kernel_build_sha256", &self.kernel_build_sha256),
            ("topology_sha256", &self.topology_sha256),
            ("resource_snapshot_sha256", &self.resource_snapshot_sha256),
            ("qdisc_config_sha256", &self.qdisc_config_sha256),
            ("workload_sha256", &self.workload_sha256),
            ("impairment_sha256", &self.impairment_sha256),
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
pub struct UtilityArmIdentity {
    pub protocol: String,
    pub protocol_revision: String,
    /// Digest of the exact binary/container or kernel build for this arm.
    pub implementation_sha256: String,
    pub scheduler: String,
    pub scheduler_provenance: UtilityImplementationProvenance,
    pub congestion_control: String,
    pub congestion_control_provenance: UtilityImplementationProvenance,
    /// Minimum active paths that must be proved at both endpoints. Set to 1
    /// for a single-path reference and >=2 for a multipath arm.
    pub required_active_paths: u16,
}

impl UtilityArmIdentity {
    fn validate(&self) -> Result<(), UtilityError> {
        if self.protocol.trim().is_empty()
            || self.protocol_revision.trim().is_empty()
            || self.scheduler.trim().is_empty()
            || self.congestion_control.trim().is_empty()
        {
            return Err(UtilityError::MissingArmIdentity);
        }
        if !is_sha256(&self.implementation_sha256) {
            return Err(UtilityError::InvalidDigest("implementation_sha256"));
        }
        if self.required_active_paths == 0 || self.required_active_paths > MAX_PATHS {
            return Err(UtilityError::InvalidRequiredPathCount);
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
pub struct UtilityTaskContract {
    pub task_id: String,
    pub offered_useful_bytes: u64,
    pub deadline_after_us: u64,
    pub value_weight: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UtilityTaskMeasurement {
    pub task_id: String,
    /// Useful application payload received by the task deadline.
    pub on_time_useful_bytes: u64,
    /// Total useful application payload received by measurement end, including late data.
    pub total_useful_bytes: u64,
    /// Completion offset from the start of the measurement window; None means
    /// the task did not complete or completion evidence is unavailable.
    pub completion_offset_us: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompanionFlowMeasurement {
    pub flow_id: String,
    pub offered_load_bps: u64,
    pub goodput_bps: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UtilityTransportEvidence {
    pub supported: bool,
    pub path_state_verified_at_both_endpoints: bool,
    pub local_active_path_count: u16,
    pub remote_active_path_count: u16,
    pub all_required_counters_present: bool,
    pub cleanup_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UtilityTrial {
    pub environment: UtilityEnvironmentIdentity,
    pub arm: UtilityArmIdentity,
    pub evidence: UtilityTransportEvidence,
    pub tasks: Vec<UtilityTaskContract>,
    pub task_measurements: Vec<UtilityTaskMeasurement>,
    pub companion_flows: Vec<CompanionFlowMeasurement>,
    pub total_cost_microunits: u64,
    pub total_energy_microjoules: u64,
    pub total_redundant_bytes: u64,
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
    InvalidRequiredPathCount,
    ZeroMeasurementDuration,
    InvalidWarmupDuration,
    TooManyTasks,
    TooManyCompanionFlows,
    InvalidTaskId,
    DuplicateTaskId,
    InvalidTaskContract,
    InvalidTaskMeasurement,
    MissingTaskMeasurement,
    DuplicateTaskMeasurement,
    InvalidCompanionFlow,
    DuplicateCompanionFlow,
    InvalidTarget,
    ArithmeticOverflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UtilityTargets {
    /// Treatment on-time useful bytes / baseline on-time useful bytes.
    pub minimum_on_time_payload_ratio_ppm: u64,
    /// Treatment completed task-value / baseline completed task-value.
    pub minimum_on_time_task_value_ratio_ppm: u64,
    /// Optional minimum treatment/baseline goodput ratio for every backlogged
    /// companion flow. None disables the cross-traffic gate.
    pub minimum_companion_retention_ppm: Option<u64>,
    /// Optional maximum treatment/baseline cost per on-time MiB ratio.
    pub maximum_cost_per_on_time_mib_ratio_ppm: Option<u64>,
    /// Optional maximum treatment/baseline energy per on-time MiB ratio.
    pub maximum_energy_per_on_time_mib_ratio_ppm: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UtilityTrialMetrics {
    pub offered_useful_bytes: u64,
    pub on_time_useful_bytes: u64,
    pub total_useful_bytes: u64,
    pub late_useful_bytes: u64,
    pub on_time_payload_ratio_ppm: u64,
    pub total_task_value_weight: u64,
    pub on_time_task_value_weight: u64,
    pub on_time_task_value_ratio_ppm: u64,
    pub completed_task_count: u32,
    pub completed_on_time_task_count: u32,
    pub missed_deadline_count: u32,
    pub completion_evidence_missing_count: u32,
    pub deadline_lateness_p50_us: Option<u64>,
    pub deadline_lateness_p95_us: Option<u64>,
    pub deadline_lateness_p99_us: Option<u64>,
    pub cost_per_on_time_mib_microunits: Option<u64>,
    pub energy_per_on_time_mib_microjoules: Option<u64>,
    pub redundant_bytes_per_on_time_byte_ppm: Option<u64>,
    pub companion_min_retention_ppm: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UtilityComparisonReport {
    pub scope: &'static str,
    pub disposition: UtilityDisposition,
    pub reason: &'static str,
    pub baseline_environment: Option<UtilityEnvironmentIdentity>,
    pub treatment_environment: Option<UtilityEnvironmentIdentity>,
    pub baseline_arm: Option<UtilityArmIdentity>,
    pub treatment_arm: Option<UtilityArmIdentity>,
    pub baseline_metrics: Option<UtilityTrialMetrics>,
    pub treatment_metrics: Option<UtilityTrialMetrics>,
    pub on_time_payload_ratio_vs_baseline_ppm: Option<u64>,
    pub on_time_task_value_ratio_vs_baseline_ppm: Option<u64>,
    pub cost_per_on_time_mib_ratio_vs_baseline_ppm: Option<u64>,
    pub energy_per_on_time_mib_ratio_vs_baseline_ppm: Option<u64>,
    pub companion_min_retention_ppm: Option<u64>,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_provenance(
    provenance: &UtilityImplementationProvenance,
    component: &'static str,
) -> Result<(), UtilityError> {
    if let UtilityImplementationProvenance::ExternalArtifact { artifact_sha256 } = provenance {
        if !is_sha256(artifact_sha256) {
            return Err(UtilityError::InvalidImplementationArtifact(component));
        }
    }
    Ok(())
}

fn ratio_ppm(numerator: u128, denominator: u128) -> Result<Option<u64>, UtilityError> {
    if denominator == 0 {
        return Ok(None);
    }
    let value = numerator
        .checked_mul(PPM)
        .ok_or(UtilityError::ArithmeticOverflow)?
        / denominator;
    Ok(Some(
        u64::try_from(value).map_err(|_| UtilityError::ArithmeticOverflow)?,
    ))
}

fn cost_per_mib(
    cost_microunits: u64,
    useful_bytes: u64,
) -> Result<Option<u64>, UtilityError> {
    if useful_bytes == 0 {
        return Ok(None);
    }
    let numerator = u128::from(cost_microunits)
        .checked_mul(BYTES_PER_MIB)
        .ok_or(UtilityError::ArithmeticOverflow)?;
    let denominator = u128::from(useful_bytes);
    // Round presentation values upward so displayed unit-cost never
    // understates a non-integer micro-unit-per-MiB measurement.
    let value = numerator
        .checked_add(denominator - 1)
        .ok_or(UtilityError::ArithmeticOverflow)?
        / denominator;
    Ok(Some(
        u64::try_from(value).map_err(|_| UtilityError::ArithmeticOverflow)?,
    ))
}

fn intensity_ratio_ppm(
    treatment_cost: u64,
    treatment_useful_bytes: u64,
    baseline_cost: u64,
    baseline_useful_bytes: u64,
) -> Result<Option<u64>, UtilityError> {
    if treatment_useful_bytes == 0 || baseline_cost == 0 || baseline_useful_bytes == 0 {
        return Ok(None);
    }
    let numerator = u128::from(treatment_cost)
        .checked_mul(u128::from(baseline_useful_bytes))
        .ok_or(UtilityError::ArithmeticOverflow)?;
    let denominator = u128::from(baseline_cost)
        .checked_mul(u128::from(treatment_useful_bytes))
        .ok_or(UtilityError::ArithmeticOverflow)?;
    ratio_ppm(numerator, denominator)
}

fn percentile_nearest_rank(values: &mut [u64], numerator: usize, denominator: usize) -> Option<u64> {
    if values.is_empty() || denominator == 0 {
        return None;
    }
    values.sort_unstable();
    let rank = (numerator * values.len()).div_ceil(denominator).max(1);
    values.get(rank - 1).copied()
}

fn canonical_tasks(tasks: &[UtilityTaskContract]) -> Result<Vec<UtilityTaskContract>, UtilityError> {
    if tasks.is_empty() || tasks.len() > MAX_TASKS {
        return Err(UtilityError::TooManyTasks);
    }
    let mut canonical = tasks.to_vec();
    canonical.sort_by(|left, right| left.task_id.cmp(&right.task_id));
    for (index, task) in canonical.iter().enumerate() {
        if task.task_id.trim().is_empty()
            || task.task_id.len() > 128
            || task.task_id.chars().any(char::is_control)
        {
            return Err(UtilityError::InvalidTaskId);
        }
        if index > 0 && canonical[index - 1].task_id == task.task_id {
            return Err(UtilityError::DuplicateTaskId);
        }
        if task.offered_useful_bytes == 0
            || task.deadline_after_us == 0
            || task.value_weight == 0
        {
            return Err(UtilityError::InvalidTaskContract);
        }
    }
    Ok(canonical)
}

fn validate_trial(trial: &UtilityTrial) -> Result<Vec<UtilityTaskContract>, UtilityError> {
    trial.environment.validate()?;
    trial.arm.validate()?;
    let tasks = canonical_tasks(&trial.tasks)?;
    for task in &tasks {
        if task.deadline_after_us > trial.environment.measurement_duration_us {
            return Err(UtilityError::InvalidTaskContract);
        }
    }
    if trial.companion_flows.len() > MAX_COMPANION_FLOWS {
        return Err(UtilityError::TooManyCompanionFlows);
    }

    let mut measurements = trial.task_measurements.clone();
    measurements.sort_by(|left, right| left.task_id.cmp(&right.task_id));
    if measurements.len() != tasks.len() {
        return Err(UtilityError::MissingTaskMeasurement);
    }
    for index in 1..measurements.len() {
        if measurements[index - 1].task_id == measurements[index].task_id {
            return Err(UtilityError::DuplicateTaskMeasurement);
        }
    }
    for (task, measurement) in tasks.iter().zip(&measurements) {
        if task.task_id != measurement.task_id {
            return Err(UtilityError::MissingTaskMeasurement);
        }
        if measurement.on_time_useful_bytes > measurement.total_useful_bytes
            || measurement.total_useful_bytes > task.offered_useful_bytes
        {
            return Err(UtilityError::InvalidTaskMeasurement);
        }
        match measurement.completion_offset_us {
            Some(completion_us) => {
                if completion_us > trial.environment.measurement_duration_us
                    || measurement.total_useful_bytes != task.offered_useful_bytes
                {
                    return Err(UtilityError::InvalidTaskMeasurement);
                }
            }
            None => {}
        }
        if measurement
            .completion_offset_us
            .is_some_and(|completion| completion <= task.deadline_after_us)
            && measurement.on_time_useful_bytes != task.offered_useful_bytes
        {
            return Err(UtilityError::InvalidTaskMeasurement);
        }
    }

    let mut flows = trial.companion_flows.clone();
    flows.sort_by(|left, right| left.flow_id.cmp(&right.flow_id));
    for (index, flow) in flows.iter().enumerate() {
        if flow.flow_id.trim().is_empty()
            || flow.flow_id.len() > 128
            || flow.flow_id.chars().any(char::is_control)
            || flow.offered_load_bps == 0
            || flow.goodput_bps > flow.offered_load_bps
        {
            return Err(UtilityError::InvalidCompanionFlow);
        }
        if index > 0 && flows[index - 1].flow_id == flow.flow_id {
            return Err(UtilityError::DuplicateCompanionFlow);
        }
    }
    Ok(tasks)
}

fn trial_metrics(trial: &UtilityTrial) -> Result<UtilityTrialMetrics, UtilityError> {
    let tasks = canonical_tasks(&trial.tasks)?;
    let mut measurements = trial.task_measurements.clone();
    measurements.sort_by(|left, right| left.task_id.cmp(&right.task_id));
    let mut offered_useful_bytes = 0_u64;
    let mut on_time_useful_bytes = 0_u64;
    let mut total_useful_bytes = 0_u64;
    let mut total_task_value_weight = 0_u64;
    let mut on_time_task_value_weight = 0_u64;
    let mut completed_task_count = 0_u32;
    let mut completed_on_time_task_count = 0_u32;
    let mut missed_deadline_count = 0_u32;
    let mut completion_evidence_missing_count = 0_u32;
    let mut deadline_lateness = Vec::new();

    for (task, measurement) in tasks.iter().zip(&measurements) {
        offered_useful_bytes = offered_useful_bytes
            .checked_add(task.offered_useful_bytes)
            .ok_or(UtilityError::ArithmeticOverflow)?;
        on_time_useful_bytes = on_time_useful_bytes
            .checked_add(measurement.on_time_useful_bytes)
            .ok_or(UtilityError::ArithmeticOverflow)?;
        total_useful_bytes = total_useful_bytes
            .checked_add(measurement.total_useful_bytes)
            .ok_or(UtilityError::ArithmeticOverflow)?;
        total_task_value_weight = total_task_value_weight
            .checked_add(u64::from(task.value_weight))
            .ok_or(UtilityError::ArithmeticOverflow)?;

        if let Some(completion_us) = measurement.completion_offset_us {
            completed_task_count = completed_task_count
                .checked_add(1)
                .ok_or(UtilityError::ArithmeticOverflow)?;
            if completion_us <= task.deadline_after_us {
                completed_on_time_task_count = completed_on_time_task_count
                    .checked_add(1)
                    .ok_or(UtilityError::ArithmeticOverflow)?;
                on_time_task_value_weight = on_time_task_value_weight
                    .checked_add(u64::from(task.value_weight))
                    .ok_or(UtilityError::ArithmeticOverflow)?;
            } else {
                missed_deadline_count = missed_deadline_count
                    .checked_add(1)
                    .ok_or(UtilityError::ArithmeticOverflow)?;
                deadline_lateness.push(completion_us - task.deadline_after_us);
            }
        } else if measurement.total_useful_bytes < task.offered_useful_bytes {
            missed_deadline_count = missed_deadline_count
                .checked_add(1)
                .ok_or(UtilityError::ArithmeticOverflow)?;
        } else {
            // All bytes may be present, but without a completion timestamp the
            // evaluator cannot prove whether the application met its deadline.
            completion_evidence_missing_count = completion_evidence_missing_count
                .checked_add(1)
                .ok_or(UtilityError::ArithmeticOverflow)?;
        }
    }

    let late_useful_bytes = total_useful_bytes
        .checked_sub(on_time_useful_bytes)
        .ok_or(UtilityError::ArithmeticOverflow)?;
    let on_time_payload_ratio_ppm = ratio_ppm(
        u128::from(on_time_useful_bytes),
        u128::from(offered_useful_bytes),
    )?
    .unwrap_or(0);
    let on_time_task_value_ratio_ppm = ratio_ppm(
        u128::from(on_time_task_value_weight),
        u128::from(total_task_value_weight),
    )?
    .unwrap_or(0);

    let mut lateness_for_p50 = deadline_lateness.clone();
    let mut lateness_for_p95 = deadline_lateness.clone();
    let mut lateness_for_p99 = deadline_lateness.clone();

    let companion_min_retention_ppm = None;
    let redundant_bytes_per_on_time_byte_ppm = ratio_ppm(
        u128::from(trial.total_redundant_bytes),
        u128::from(on_time_useful_bytes),
    )?;

    Ok(UtilityTrialMetrics {
        offered_useful_bytes,
        on_time_useful_bytes,
        late_useful_bytes,
        total_useful_bytes,
        on_time_payload_ratio_ppm,
        total_task_value_weight,
        on_time_task_value_weight,
        on_time_task_value_ratio_ppm,
        completed_task_count,
        completed_on_time_task_count,
        missed_deadline_count,
        completion_evidence_missing_count,
        deadline_lateness_p50_us: percentile_nearest_rank(&mut lateness_for_p50, 50, 100),
        deadline_lateness_p95_us: percentile_nearest_rank(&mut lateness_for_p95, 95, 100),
        deadline_lateness_p99_us: percentile_nearest_rank(&mut lateness_for_p99, 99, 100),
        cost_per_on_time_mib_microunits: cost_per_mib(
            trial.total_cost_microunits,
            on_time_useful_bytes,
        )?,
        energy_per_on_time_mib_microjoules: cost_per_mib(
            trial.total_energy_microjoules,
            on_time_useful_bytes,
        )?,
        redundant_bytes_per_on_time_byte_ppm,
        companion_min_retention_ppm,
    })
}

fn companion_retention_ppm(
    baseline: &[CompanionFlowMeasurement],
    treatment: &[CompanionFlowMeasurement],
) -> Result<Option<u64>, UtilityError> {
    let mut baseline = baseline.to_vec();
    let mut treatment = treatment.to_vec();
    baseline.sort_by(|left, right| left.flow_id.cmp(&right.flow_id));
    treatment.sort_by(|left, right| left.flow_id.cmp(&right.flow_id));
    if baseline.len() != treatment.len() {
        return Err(UtilityError::InvalidCompanionFlow);
    }
    let mut minimum = None;
    for (control, candidate) in baseline.iter().zip(&treatment) {
        if control.flow_id != candidate.flow_id
            || control.offered_load_bps != candidate.offered_load_bps
        {
            return Err(UtilityError::InvalidCompanionFlow);
        }
        if control.goodput_bps < control.offered_load_bps {
            if control.goodput_bps == 0 {
                return Ok(None);
            }
            let ratio = ratio_ppm(
                u128::from(candidate.goodput_bps),
                u128::from(control.goodput_bps),
            )?
            .ok_or(UtilityError::ArithmeticOverflow)?;
            minimum = Some(minimum.map_or(ratio, |current: u64| current.min(ratio)));
        }
    }
    Ok(minimum)
}

fn make_report(
    disposition: UtilityDisposition,
    reason: &'static str,
    baseline_metrics: Option<UtilityTrialMetrics>,
    treatment_metrics: Option<UtilityTrialMetrics>,
) -> UtilityComparisonReport {
    UtilityComparisonReport {
        scope: "single_paired_trial",
        disposition,
        reason,
        baseline_environment: None,
        treatment_environment: None,
        baseline_arm: None,
        treatment_arm: None,
        baseline_metrics,
        treatment_metrics,
        on_time_payload_ratio_vs_baseline_ppm: None,
        on_time_task_value_ratio_vs_baseline_ppm: None,
        cost_per_on_time_mib_ratio_vs_baseline_ppm: None,
        energy_per_on_time_mib_ratio_vs_baseline_ppm: None,
        companion_min_retention_ppm: None,
    }
}

fn make_bound_report(
    disposition: UtilityDisposition,
    reason: &'static str,
    baseline: &UtilityTrial,
    treatment: &UtilityTrial,
    baseline_metrics: Option<UtilityTrialMetrics>,
    treatment_metrics: Option<UtilityTrialMetrics>,
) -> UtilityComparisonReport {
    let mut report = make_report(
        disposition,
        reason,
        baseline_metrics,
        treatment_metrics,
    );
    report.baseline_environment = Some(baseline.environment.clone());
    report.treatment_environment = Some(treatment.environment.clone());
    report.baseline_arm = Some(baseline.arm.clone());
    report.treatment_arm = Some(treatment.arm.clone());
    report
}

/// Compare a baseline transport/scheduler and a treatment on the same frozen
/// environment and exact task set. A passing result is one paired trial only.
pub fn evaluate_deadline_utility_pair(
    baseline: &UtilityTrial,
    treatment: &UtilityTrial,
    targets: UtilityTargets,
) -> Result<UtilityComparisonReport, UtilityError> {
    if targets.minimum_on_time_payload_ratio_ppm == 0
        || targets.minimum_on_time_payload_ratio_ppm > 10_000_000
        || targets.minimum_on_time_task_value_ratio_ppm == 0
        || targets.minimum_on_time_task_value_ratio_ppm > 10_000_000
        || targets
            .minimum_companion_retention_ppm
            .is_some_and(|target| target == 0 || target > 1_000_000)
        || targets
            .maximum_cost_per_on_time_mib_ratio_ppm
            .is_some_and(|target| target == 0 || target > 10_000_000)
        || targets
            .maximum_energy_per_on_time_mib_ratio_ppm
            .is_some_and(|target| target == 0 || target > 10_000_000)
    {
        return Err(UtilityError::InvalidTarget);
    }

    let baseline_tasks = match validate_trial(baseline) {
        Ok(tasks) => tasks,
        Err(UtilityError::MissingTaskMeasurement) => {
            return Ok(make_bound_report(
                UtilityDisposition::Inconclusive,
                "baseline_task_measurement_missing_or_unmatched",
                baseline,
                treatment,
                None,
                None,
            ));
        }
        Err(error) => return Err(error),
    };
    let treatment_tasks = match validate_trial(treatment) {
        Ok(tasks) => tasks,
        Err(UtilityError::MissingTaskMeasurement) => {
            return Ok(make_bound_report(
                UtilityDisposition::Inconclusive,
                "treatment_task_measurement_missing_or_unmatched",
                baseline,
                treatment,
                None,
                None,
            ));
        }
        Err(error) => return Err(error),
    };
    if baseline.environment != treatment.environment || baseline_tasks != treatment_tasks {
        return Ok(make_bound_report(
            UtilityDisposition::Inconclusive,
            "paired_environment_or_task_contract_mismatch",
            baseline,
            treatment,
            None,
            None,
        ));
    }

    for trial in [baseline, treatment] {
        if !trial.evidence.supported {
            return Ok(make_bound_report(
                UtilityDisposition::Unsupported,
                "pinned_transport_or_scheduler_arm_unsupported",
                baseline,
                treatment,
                None,
                None,
            ));
        }
        if !trial.evidence.all_required_counters_present {
            return Ok(make_bound_report(
                UtilityDisposition::Inconclusive,
                "required_task_transport_or_companion_counters_missing",
                baseline,
                treatment,
                None,
                None,
            ));
        }
        if !trial.evidence.cleanup_verified {
            return Ok(make_bound_report(
                UtilityDisposition::Fail,
                "namespace_or_resource_cleanup_not_verified",
                baseline,
                treatment,
                None,
                None,
            ));
        }
        if !trial.evidence.path_state_verified_at_both_endpoints {
            return Ok(make_bound_report(
                UtilityDisposition::Inconclusive,
                "path_state_not_verified_at_both_endpoints",
                baseline,
                treatment,
                None,
                None,
            ));
        }
        if trial.evidence.local_active_path_count < trial.arm.required_active_paths
            || trial.evidence.remote_active_path_count < trial.arm.required_active_paths
        {
            return Ok(make_bound_report(
                UtilityDisposition::Fail,
                "required_active_path_count_not_met_at_both_endpoints",
                baseline,
                treatment,
                None,
                None,
            ));
        }
        if trial.evidence.local_active_path_count > MAX_PATHS
            || trial.evidence.remote_active_path_count > MAX_PATHS
        {
            return Ok(make_bound_report(
                UtilityDisposition::Inconclusive,
                "active_path_count_exceeds_evaluator_bound",
                baseline,
                treatment,
                None,
                None,
            ));
        }
    }

    let baseline_companion = match companion_retention_ppm(
        &baseline.companion_flows,
        &treatment.companion_flows,
    ) {
        Ok(retention) => retention,
        Err(UtilityError::InvalidCompanionFlow) => {
            return Ok(make_bound_report(
                UtilityDisposition::Inconclusive,
                "companion_flow_identity_or_offered_load_mismatch",
                baseline,
                treatment,
                None,
                None,
            ));
        }
        Err(error) => return Err(error),
    };
    let baseline_metrics = trial_metrics(baseline)?;
    let mut treatment_metrics = trial_metrics(treatment)?;
    treatment_metrics.companion_min_retention_ppm = baseline_companion;

    let payload_ratio = ratio_ppm(
        u128::from(treatment_metrics.on_time_useful_bytes),
        u128::from(baseline_metrics.on_time_useful_bytes),
    )?;
    let value_ratio = if baseline_metrics.completion_evidence_missing_count > 0
        || treatment_metrics.completion_evidence_missing_count > 0
    {
        None
    } else {
        ratio_ppm(
            u128::from(treatment_metrics.on_time_task_value_weight),
            u128::from(baseline_metrics.on_time_task_value_weight),
        )?
    };
    let cost_ratio = intensity_ratio_ppm(
        treatment.total_cost_microunits,
        treatment_metrics.on_time_useful_bytes,
        baseline.total_cost_microunits,
        baseline_metrics.on_time_useful_bytes,
    )?;
    let energy_ratio = intensity_ratio_ppm(
        treatment.total_energy_microjoules,
        treatment_metrics.on_time_useful_bytes,
        baseline.total_energy_microjoules,
        baseline_metrics.on_time_useful_bytes,
    )?;

    let mut report = make_bound_report(
        UtilityDisposition::Pass,
        "deadline_utility_pair_meets_declared_targets",
        baseline,
        treatment,
        Some(baseline_metrics),
        Some(treatment_metrics),
    );
    report.on_time_payload_ratio_vs_baseline_ppm = payload_ratio;
    report.on_time_task_value_ratio_vs_baseline_ppm = value_ratio;
    report.cost_per_on_time_mib_ratio_vs_baseline_ppm = cost_ratio;
    report.energy_per_on_time_mib_ratio_vs_baseline_ppm = energy_ratio;
    report.companion_min_retention_ppm = baseline_companion;

    let checks = [
        (
            payload_ratio,
            Some(targets.minimum_on_time_payload_ratio_ppm),
            true,
            "on_time_useful_payload_regressed_below_target",
        ),
        (
            value_ratio,
            Some(targets.minimum_on_time_task_value_ratio_ppm),
            true,
            "on_time_task_value_regressed_below_target",
        ),
        (
            baseline_companion,
            targets.minimum_companion_retention_ppm,
            true,
            "companion_flow_retention_below_target",
        ),
        (
            cost_ratio,
            targets.maximum_cost_per_on_time_mib_ratio_ppm,
            false,
            "cost_per_on_time_mib_exceeds_target",
        ),
        (
            energy_ratio,
            targets.maximum_energy_per_on_time_mib_ratio_ppm,
            false,
            "energy_per_on_time_mib_exceeds_target",
        ),
    ];

    let mut inconclusive = false;
    for (observed, target, greater_is_better, failure_reason) in checks {
        let Some(target) = target else {
            continue;
        };
        let Some(observed) = observed else {
            inconclusive = true;
            continue;
        };
        let passes = if greater_is_better {
            observed >= target
        } else {
            observed <= target
        };
        if !passes {
            report.disposition = UtilityDisposition::Fail;
            report.reason = failure_reason;
            return Ok(report);
        }
    }
    if inconclusive {
        report.disposition = UtilityDisposition::Inconclusive;
        report.reason = "one_or_more_enabled_utility_metrics_not_measurable";
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment() -> UtilityEnvironmentIdentity {
        UtilityEnvironmentIdentity {
            kernel_build_sha256: "a".repeat(64),
            topology_sha256: "b".repeat(64),
            resource_snapshot_sha256: "c".repeat(64),
            qdisc_config_sha256: "d".repeat(64),
            workload_sha256: "e".repeat(64),
            impairment_sha256: "f".repeat(64),
            seed: 17,
            warmup_us: 1_000_000,
            measurement_duration_us: 30_000_000,
        }
    }

    fn arm(protocol: &str, scheduler: &str, paths: u16) -> UtilityArmIdentity {
        UtilityArmIdentity {
            protocol: protocol.to_owned(),
            protocol_revision: "pinned-test-revision".to_owned(),
            implementation_sha256: "1".repeat(64),
            scheduler: scheduler.to_owned(),
            scheduler_provenance: UtilityImplementationProvenance::IncludedInImplementation,
            congestion_control: "cubic".to_owned(),
            congestion_control_provenance: UtilityImplementationProvenance::IncludedInImplementation,
            required_active_paths: paths,
        }
    }

    fn tasks() -> Vec<UtilityTaskContract> {
        vec![
            UtilityTaskContract {
                task_id: "control".to_owned(),
                offered_useful_bytes: 1_000,
                deadline_after_us: 5_000_000,
                value_weight: 3,
            },
            UtilityTaskContract {
                task_id: "bulk".to_owned(),
                offered_useful_bytes: 4_000,
                deadline_after_us: 20_000_000,
                value_weight: 1,
            },
        ]
    }

    fn evidence(paths: u16) -> UtilityTransportEvidence {
        UtilityTransportEvidence {
            supported: true,
            path_state_verified_at_both_endpoints: true,
            local_active_path_count: paths,
            remote_active_path_count: paths,
            all_required_counters_present: true,
            cleanup_verified: true,
        }
    }

    fn baseline() -> UtilityTrial {
        UtilityTrial {
            environment: environment(),
            arm: arm("tcp", "single-path", 1),
            evidence: evidence(1),
            tasks: tasks(),
            task_measurements: vec![
                UtilityTaskMeasurement {
                    task_id: "control".to_owned(),
                    on_time_useful_bytes: 1_000,
                    total_useful_bytes: 1_000,
                    completion_offset_us: Some(4_000_000),
                },
                UtilityTaskMeasurement {
                    task_id: "bulk".to_owned(),
                    on_time_useful_bytes: 2_000,
                    total_useful_bytes: 4_000,
                    completion_offset_us: Some(25_000_000),
                },
            ],
            companion_flows: vec![CompanionFlowMeasurement {
                flow_id: "background-a".to_owned(),
                offered_load_bps: 20_000_000,
                goodput_bps: 10_000_000,
            }],
            total_cost_microunits: 100,
            total_energy_microjoules: 200,
            total_redundant_bytes: 0,
        }
    }

    fn treatment() -> UtilityTrial {
        UtilityTrial {
            environment: environment(),
            arm: arm("mpquic-draft-21", "deadline-aware", 2),
            evidence: evidence(2),
            tasks: tasks(),
            task_measurements: vec![
                UtilityTaskMeasurement {
                    task_id: "control".to_owned(),
                    on_time_useful_bytes: 1_000,
                    total_useful_bytes: 1_000,
                    completion_offset_us: Some(2_000_000),
                },
                UtilityTaskMeasurement {
                    task_id: "bulk".to_owned(),
                    on_time_useful_bytes: 4_000,
                    total_useful_bytes: 4_000,
                    completion_offset_us: Some(15_000_000),
                },
            ],
            companion_flows: vec![CompanionFlowMeasurement {
                flow_id: "background-a".to_owned(),
                offered_load_bps: 20_000_000,
                goodput_bps: 9_500_000,
            }],
            total_cost_microunits: 90,
            total_energy_microjoules: 180,
            total_redundant_bytes: 100,
        }
    }

    fn targets() -> UtilityTargets {
        UtilityTargets {
            minimum_on_time_payload_ratio_ppm: 1_000_000,
            minimum_on_time_task_value_ratio_ppm: 1_000_000,
            minimum_companion_retention_ppm: Some(900_000),
            maximum_cost_per_on_time_mib_ratio_ppm: Some(1_000_000),
            maximum_energy_per_on_time_mib_ratio_ppm: Some(1_000_000),
        }
    }

    #[test]
    fn deadline_utility_pair_passes_when_timely_value_and_cost_improve() {
        let report = evaluate_deadline_utility_pair(&baseline(), &treatment(), targets())
            .expect("valid trial pair");
        assert_eq!(report.disposition, UtilityDisposition::Pass);
        assert_eq!(report.scope, "single_paired_trial");
        assert_eq!(report.on_time_payload_ratio_vs_baseline_ppm, Some(1_666_666));
        assert_eq!(report.on_time_task_value_ratio_vs_baseline_ppm, Some(1_333_333));
        assert_eq!(report.companion_min_retention_ppm, Some(950_000));
        assert_eq!(report.cost_per_on_time_mib_ratio_vs_baseline_ppm, Some(540_000));
        assert_eq!(report.energy_per_on_time_mib_ratio_vs_baseline_ppm, Some(540_000));
    }

    #[test]
    fn trial_metrics_preserve_total_and_late_useful_payload_separately() {
        let report = evaluate_deadline_utility_pair(
            &baseline(),
            &treatment(),
            targets(),
        )
        .expect("valid pair");
        let metrics = report.treatment_metrics.expect("treatment metrics");
        assert_eq!(metrics.on_time_useful_bytes, 5_000);
        assert_eq!(metrics.total_useful_bytes, 5_000);
        assert_eq!(metrics.late_useful_bytes, 0);
    }

    #[test]
    fn throughput_gain_that_misses_deadlines_fails_task_value_gate() {
        let mut candidate = treatment();
        candidate.task_measurements[0].completion_offset_us = Some(6_000_000);
        candidate.task_measurements[0].on_time_useful_bytes = 800;
        let report = evaluate_deadline_utility_pair(&baseline(), &candidate, targets())
            .expect("valid late completion trial");
        assert_eq!(report.disposition, UtilityDisposition::Fail);
        assert_eq!(report.reason, "on_time_useful_payload_regressed_below_target");
    }

    #[test]
    fn companion_starvation_fails_even_when_task_deadlines_improve() {
        let mut candidate = treatment();
        candidate.companion_flows[0].goodput_bps = 2_000_000;
        let report = evaluate_deadline_utility_pair(&baseline(), &candidate, targets())
            .expect("valid competing-flow trial");
        assert_eq!(report.disposition, UtilityDisposition::Fail);
        assert_eq!(report.reason, "companion_flow_retention_below_target");
    }

    #[test]
    fn environment_or_task_mismatch_is_inconclusive() {
        let mut candidate = treatment();
        candidate.environment.qdisc_config_sha256 = "9".repeat(64);
        let report = evaluate_deadline_utility_pair(&baseline(), &candidate, targets())
            .expect("mismatched pair is classified");
        assert_eq!(report.disposition, UtilityDisposition::Inconclusive);
        assert_eq!(report.reason, "paired_environment_or_task_contract_mismatch");
    }

    #[test]
    fn unsupported_transport_arm_is_not_a_pass() {
        let mut candidate = treatment();
        candidate.evidence.supported = false;
        let report = evaluate_deadline_utility_pair(&baseline(), &candidate, targets())
            .expect("unsupported arm is classified");
        assert_eq!(report.disposition, UtilityDisposition::Unsupported);
    }

    #[test]
    fn multipath_arm_requires_path_count_at_both_endpoints() {
        let mut candidate = treatment();
        candidate.evidence.remote_active_path_count = 1;
        let report = evaluate_deadline_utility_pair(&baseline(), &candidate, targets())
            .expect("single remote path is classified");
        assert_eq!(report.disposition, UtilityDisposition::Fail);
        assert_eq!(
            report.reason,
            "required_active_path_count_not_met_at_both_endpoints"
        );
    }

    #[test]
    fn missing_task_counters_are_inconclusive_not_zero() {
        let mut candidate = treatment();
        candidate.evidence.all_required_counters_present = false;
        let report = evaluate_deadline_utility_pair(&baseline(), &candidate, targets())
            .expect("missing counters are classified");
        assert_eq!(report.disposition, UtilityDisposition::Inconclusive);
    }

    #[test]
    fn cleanup_failure_fails_qualification() {
        let mut candidate = treatment();
        candidate.evidence.cleanup_verified = false;
        let report = evaluate_deadline_utility_pair(&baseline(), &candidate, targets())
            .expect("cleanup failure is classified");
        assert_eq!(report.disposition, UtilityDisposition::Fail);
    }

    #[test]
    fn cost_regression_can_fail_independently_of_deadline_improvement() {
        let mut candidate = treatment();
        candidate.total_cost_microunits = 1_000;
        let report = evaluate_deadline_utility_pair(&baseline(), &candidate, targets())
            .expect("cost regression is classified");
        assert_eq!(report.disposition, UtilityDisposition::Fail);
        assert_eq!(report.reason, "cost_per_on_time_mib_exceeds_target");
    }

    #[test]
    fn energy_metric_is_independently_gated() {
        let mut candidate = treatment();
        candidate.total_energy_microjoules = 5_000;
        let report = evaluate_deadline_utility_pair(&baseline(), &candidate, targets())
            .expect("energy regression is classified");
        assert_eq!(report.disposition, UtilityDisposition::Fail);
        assert_eq!(report.reason, "energy_per_on_time_mib_exceeds_target");
    }

    #[test]
    fn late_bytes_are_not_credited_as_on_time_useful_bytes() {
        let mut candidate = treatment();
        candidate.task_measurements[1].on_time_useful_bytes = 2_000;
        let report = evaluate_deadline_utility_pair(&baseline(), &candidate, targets())
            .expect("late delivery does not count as timely");
        assert_eq!(report.disposition, UtilityDisposition::Fail);
        assert_eq!(
            report.treatment_metrics.expect("metrics").late_useful_bytes,
            2_000
        );
    }

    #[test]
    fn incomplete_task_without_completion_timestamp_is_valid_but_not_complete() {
        let mut candidate = treatment();
        candidate.task_measurements[1].total_useful_bytes = 3_500;
        candidate.task_measurements[1].on_time_useful_bytes = 3_000;
        candidate.task_measurements[1].completion_offset_us = None;
        let report = evaluate_deadline_utility_pair(
            &baseline(),
            &candidate,
            UtilityTargets {
                minimum_on_time_payload_ratio_ppm: 1,
                minimum_on_time_task_value_ratio_ppm: 1,
                minimum_companion_retention_ppm: None,
                maximum_cost_per_on_time_mib_ratio_ppm: None,
                maximum_energy_per_on_time_mib_ratio_ppm: None,
            },
        )
        .expect("partial task outcome is valid");
        assert_eq!(report.treatment_metrics.expect("metrics").completed_task_count, 1);
    }

    #[test]
    fn full_payload_without_completion_timestamp_is_inconclusive_for_task_value() {
        let mut candidate = treatment();
        candidate.task_measurements[0].completion_offset_us = None;
        let report = evaluate_deadline_utility_pair(
            &baseline(),
            &candidate,
            UtilityTargets {
                minimum_on_time_payload_ratio_ppm: 1_000_000,
                minimum_on_time_task_value_ratio_ppm: 1_000_000,
                minimum_companion_retention_ppm: None,
                maximum_cost_per_on_time_mib_ratio_ppm: None,
                maximum_energy_per_on_time_mib_ratio_ppm: None,
            },
        )
        .expect("missing completion timestamp is retained as unknown");
        assert_eq!(report.disposition, UtilityDisposition::Inconclusive);
        assert_eq!(
            report.reason,
            "one_or_more_enabled_utility_metrics_not_measurable"
        );
        assert_eq!(
            report
                .treatment_metrics
                .expect("metrics retained")
                .completion_evidence_missing_count,
            1
        );
    }

    #[test]
    fn task_measurement_cannot_claim_more_useful_bytes_than_offered() {
        let mut candidate = treatment();
        candidate.task_measurements[0].on_time_useful_bytes = 1_001;
        assert_eq!(
            evaluate_deadline_utility_pair(&baseline(), &candidate, targets()),
            Err(UtilityError::InvalidTaskMeasurement)
        );
    }

    #[test]
    fn report_preserves_both_exact_arm_and_environment_identities_on_pass() {
        let control = baseline();
        let candidate = treatment();
        let report = evaluate_deadline_utility_pair(&control, &candidate, targets())
            .expect("valid pair");
        assert_eq!(report.baseline_environment, Some(control.environment));
        assert_eq!(report.treatment_environment, Some(candidate.environment));
        assert_eq!(report.baseline_arm, Some(control.arm));
        assert_eq!(report.treatment_arm, Some(candidate.arm));
    }

    #[test]
    fn report_preserves_arm_identities_on_inconclusive_pair() {
        let control = baseline();
        let mut candidate = treatment();
        candidate.environment.qdisc_config_sha256 = "9".repeat(64);
        let report = evaluate_deadline_utility_pair(&control, &candidate, targets())
            .expect("mismatched pair classified");
        assert_eq!(report.disposition, UtilityDisposition::Inconclusive);
        assert_eq!(report.baseline_arm, Some(control.arm));
        assert_eq!(report.treatment_arm, Some(candidate.arm));
    }

    #[test]
    fn missing_task_measurement_is_inconclusive_and_preserves_pair_identity() {
        let control = baseline();
        let mut candidate = treatment();
        candidate.task_measurements.pop();
        let report = evaluate_deadline_utility_pair(&control, &candidate, targets())
            .expect("missing observation is a disposition");
        assert_eq!(report.disposition, UtilityDisposition::Inconclusive);
        assert_eq!(report.reason, "treatment_task_measurement_missing_or_unmatched");
        assert_eq!(report.baseline_arm, Some(control.arm));
        assert_eq!(report.treatment_arm, Some(candidate.arm));
    }

    #[test]
    fn companion_flow_identity_mismatch_is_inconclusive_not_a_parser_error() {
        let control = baseline();
        let mut candidate = treatment();
        candidate.companion_flows[0].flow_id = "different-flow".to_owned();
        let report = evaluate_deadline_utility_pair(&control, &candidate, targets())
            .expect("mismatched companion evidence is classified");
        assert_eq!(report.disposition, UtilityDisposition::Inconclusive);
        assert_eq!(
            report.reason,
            "companion_flow_identity_or_offered_load_mismatch"
        );
    }

    #[test]
    fn duplicate_task_measurements_are_distinguished_from_missing_measurements() {
        let mut candidate = treatment();
        candidate.task_measurements[1].task_id = "control".to_owned();
        assert_eq!(
            evaluate_deadline_utility_pair(&baseline(), &candidate, targets()),
            Err(UtilityError::DuplicateTaskMeasurement)
        );
    }

    #[test]
    fn out_of_tree_scheduler_requires_its_binary_digest() {
        let mut candidate = treatment();
        candidate.arm.scheduler_provenance =
            UtilityImplementationProvenance::ExternalArtifact {
                artifact_sha256: "not-a-digest".to_owned(),
            };
        assert_eq!(
            evaluate_deadline_utility_pair(&baseline(), &candidate, targets()),
            Err(UtilityError::InvalidImplementationArtifact("scheduler"))
        );
    }

    #[test]
    fn enabled_companion_gate_without_any_backlogged_reference_flow_is_inconclusive() {
        let mut control = baseline();
        let mut candidate = treatment();
        control.companion_flows.clear();
        candidate.companion_flows.clear();
        let report = evaluate_deadline_utility_pair(
            &control,
            &candidate,
            UtilityTargets {
                minimum_on_time_payload_ratio_ppm: 1_000_000,
                minimum_on_time_task_value_ratio_ppm: 1_000_000,
                minimum_companion_retention_ppm: Some(900_000),
                maximum_cost_per_on_time_mib_ratio_ppm: None,
                maximum_energy_per_on_time_mib_ratio_ppm: None,
            },
        )
        .expect("missing companion baseline is classified");
        assert_eq!(report.disposition, UtilityDisposition::Inconclusive);
        assert_eq!(
            report.reason,
            "one_or_more_enabled_utility_metrics_not_measurable"
        );
    }

    #[test]
    fn companion_comparison_rejects_mismatched_flow_sets() {
        let mut candidate = treatment();
        candidate.companion_flows[0].flow_id = "different-flow".to_owned();
        assert_eq!(
            evaluate_deadline_utility_pair(&baseline(), &candidate, targets()),
            Err(UtilityError::InvalidCompanionFlow)
        );
    }

    #[test]
    fn cost_intensity_ratio_uses_exact_byte_totals_not_truncated_per_mib_values() {
        assert_eq!(
            intensity_ratio_ppm(90, 5_000, 100, 3_000).expect("exact ratio"),
            Some(540_000)
        );
        assert_eq!(
            intensity_ratio_ppm(0, 5_000, 0, 3_000).expect("zero/zero is undefined"),
            None
        );
    }

    #[test]
    fn missing_baseline_on_time_bytes_makes_relative_gate_inconclusive() {
        let mut control = baseline();
        for measurement in &mut control.task_measurements {
            measurement.on_time_useful_bytes = 0;
            measurement.total_useful_bytes = if measurement.task_id == "control" { 500 } else { 1_000 };
            measurement.completion_offset_us = None;
        }
        let report = evaluate_deadline_utility_pair(
            &control,
            &treatment(),
            UtilityTargets {
                minimum_on_time_payload_ratio_ppm: 1_000_000,
                minimum_on_time_task_value_ratio_ppm: 1_000_000,
                minimum_companion_retention_ppm: None,
                maximum_cost_per_on_time_mib_ratio_ppm: None,
                maximum_energy_per_on_time_mib_ratio_ppm: None,
            },
        )
        .expect("zero baseline is inconclusive");
        assert_eq!(report.disposition, UtilityDisposition::Inconclusive);
    }

    #[test]
    fn identical_pair_inputs_produce_identical_verdicts() {
        let first = evaluate_deadline_utility_pair(&baseline(), &treatment(), targets())
            .expect("valid pair");
        let second = evaluate_deadline_utility_pair(&baseline(), &treatment(), targets())
            .expect("same pair");
        assert_eq!(first, second);
    }
}
