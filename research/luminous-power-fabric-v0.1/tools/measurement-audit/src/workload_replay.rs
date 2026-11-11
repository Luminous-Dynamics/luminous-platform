use crate::AuditError;
use std::collections::{BTreeSet, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadClass {
    /// Work that must begin by its service-allowance latest-start timestamp.
    NonDeferrable,
    /// Work that may be delayed, but must still start and finish within its
    /// explicit latest-start and completion-deadline contract.
    Deferrable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResourceVector {
    pub cpu_millicores: u64,
    pub gpu_units: u64,
    pub memory_mib: u64,
}

impl ResourceVector {
    pub const fn is_zero(self) -> bool {
        self.cpu_millicores == 0 && self.gpu_units == 0 && self.memory_mib == 0
    }

    pub const fn fits_within(self, capacity: Self) -> bool {
        self.cpu_millicores <= capacity.cpu_millicores
            && self.gpu_units <= capacity.gpu_units
            && self.memory_mib <= capacity.memory_mib
    }

    fn checked_add(self, other: Self) -> Option<Self> {
        Some(Self {
            cpu_millicores: self.cpu_millicores.checked_add(other.cpu_millicores)?,
            gpu_units: self.gpu_units.checked_add(other.gpu_units)?,
            memory_mib: self.memory_mib.checked_add(other.memory_mib)?,
        })
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        Some(Self {
            cpu_millicores: self.cpu_millicores.checked_sub(other.cpu_millicores)?,
            gpu_units: self.gpu_units.checked_sub(other.gpu_units)?,
            memory_mib: self.memory_mib.checked_sub(other.memory_mib)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadReplayContract {
    /// Stable immutable replay-contract revision.
    pub replay_contract_id: String,
    pub workload_id: String,
    pub dataset_id: String,
    pub dataset_version: String,
    /// Reference to the upstream evidence manifest / source artifact digest.
    pub dataset_evidence_id: String,
    /// Reference to the dataset-registry decision or explicit access grant.
    /// This module preserves the reference; it does not make legal decisions.
    pub use_authorization_reference: String,
    pub normalization_id: String,
    pub scheduling_policy_id: String,
    pub work_unit: String,
    pub full_resource_capacity: ResourceVector,
}

impl WorkloadReplayContract {
    fn validate(&self) -> Result<(), AuditError> {
        for (value, field) in [
            (self.replay_contract_id.as_str(), "replay_contract_id"),
            (self.workload_id.as_str(), "workload_id"),
            (self.dataset_id.as_str(), "dataset_id"),
            (self.dataset_version.as_str(), "dataset_version"),
            (self.dataset_evidence_id.as_str(), "dataset_evidence_id"),
            (
                self.use_authorization_reference.as_str(),
                "use_authorization_reference",
            ),
            (self.normalization_id.as_str(), "normalization_id"),
            (self.scheduling_policy_id.as_str(), "scheduling_policy_id"),
            (self.work_unit.as_str(), "work_unit"),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!(
                    "workload replay contract {field} must not be blank"
                )));
            }
        }
        if self.full_resource_capacity.is_zero() {
            return Err(AuditError(
                "full resource capacity must have at least one non-zero dimension".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadJob {
    pub job_id: String,
    pub source_evidence_id: String,
    pub class: WorkloadClass,
    /// Absolute timestamps on one declared monotonic replay clock.
    pub release_time_ns: u64,
    pub latest_start_ns: u64,
    pub completion_deadline_ns: u64,
    /// v0.1 assumes a non-preemptive, constant-duration execution.
    pub runtime_ns: u64,
    pub resource_demand: ResourceVector,
    pub useful_work_units: u64,
}

impl WorkloadJob {
    fn validate(&self, full_capacity: ResourceVector) -> Result<(), AuditError> {
        if self.job_id.trim().is_empty() || self.source_evidence_id.trim().is_empty() {
            return Err(AuditError(
                "workload job ID and source evidence ID must not be blank".to_string(),
            ));
        }
        if self.runtime_ns == 0
            || self.useful_work_units == 0
            || self.latest_start_ns < self.release_time_ns
            || self.completion_deadline_ns <= self.latest_start_ns
        {
            return Err(AuditError(format!(
                "workload job {:?} has invalid runtime, work units or time bounds",
                self.job_id
            )));
        }
        if self
            .latest_start_ns
            .checked_add(self.runtime_ns)
            .is_none_or(|finish| finish > self.completion_deadline_ns)
        {
            return Err(AuditError(format!(
                "workload job {:?} cannot finish by its deadline from its latest permitted start",
                self.job_id
            )));
        }
        if self.class == WorkloadClass::NonDeferrable
            && self.latest_start_ns != self.release_time_ns
        {
            return Err(AuditError(format!(
                "non-deferrable job {:?} must have latest_start_ns equal to release_time_ns",
                self.job_id
            )));
        }
        if self.resource_demand.is_zero()
            || !self.resource_demand.fits_within(full_capacity)
        {
            return Err(AuditError(format!(
                "workload job {:?} has zero or impossible CPU/GPU/memory demand",
                self.job_id
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchCapacityWindow {
    pub window_id: String,
    pub start_ns: u64,
    pub end_ns: u64,
    /// Admission limits for newly started jobs. Running jobs are not stopped
    /// or preempted when the window begins.
    pub admission_capacity: ResourceVector,
}

impl DispatchCapacityWindow {
    fn validate(&self, full_capacity: ResourceVector) -> Result<(), AuditError> {
        if self.window_id.trim().is_empty()
            || self.end_ns <= self.start_ns
            || !self.admission_capacity.fits_within(full_capacity)
        {
            return Err(AuditError(format!(
                "dispatch window {:?} has an invalid ID, interval or resource cap",
                self.window_id
            )));
        }
        Ok(())
    }

    pub fn duration_ns(&self) -> u64 {
        self.end_ns - self.start_ns
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobState {
    Pending,
    Running { start_ns: u64, finish_ns: u64 },
    Completed { start_ns: u64, finish_ns: u64 },
    Missed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadRunSummary {
    pub total_job_count: usize,
    pub completed_job_count: usize,
    pub missed_job_count: usize,
    pub non_deferrable_missed_job_count: usize,
    pub deferrable_missed_job_count: usize,
    pub total_useful_work_units: u64,
    pub completed_useful_work_units: u64,
    pub missed_useful_work_units: u64,
    /// GPU-capacity-unit nanoseconds actually occupied by scheduled jobs
    /// inside the declared dispatch windows. This is not energy.
    pub gpu_resource_time_during_dispatch_unit_ns: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadJobReplayResult {
    pub job_id: String,
    pub class: WorkloadClass,
    pub baseline_start_ns: Option<u64>,
    pub baseline_finish_ns: Option<u64>,
    pub dispatch_start_ns: Option<u64>,
    pub dispatch_finish_ns: Option<u64>,
    /// Positive delay only; zero means the dispatch replay did not start later.
    /// None means the job did not start in one or both schedules.
    pub positive_start_delay_ns: Option<u64>,
    pub baseline_deadline_met: bool,
    pub dispatch_deadline_met: bool,
    pub useful_work_units: u64,
    pub useful_work_completed_in_dispatch_replay: u64,
    pub deferred_and_completed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkloadReplayReport {
    pub contract: WorkloadReplayContract,
    pub baseline: WorkloadRunSummary,
    pub dispatch_constrained: WorkloadRunSummary,
    pub useful_work_retention_fraction: Option<f64>,
    pub deferred_and_completed_job_count: usize,
    pub deferred_and_completed_work_units: u64,
    /// Percentiles are calculated over strictly positive start delays only.
    pub p50_positive_start_delay_ns: Option<u64>,
    pub p95_positive_start_delay_ns: Option<u64>,
    pub p99_positive_start_delay_ns: Option<u64>,
    pub maximum_positive_start_delay_ns: Option<u64>,
    /// Exact integral of (full GPU admission capacity - dispatch GPU admission
    /// capacity) over dispatch windows. Not a power or energy quantity.
    pub gpu_admission_restriction_unit_ns: u128,
    pub total_dispatch_window_ns: u128,
    pub longest_dispatch_window_ns: u64,
    /// Some(0) means no delayed completion extended beyond the final window.
    /// Missed jobs are reported separately and are never hidden in this tail.
    pub recovery_tail_after_last_dispatch_ns: Option<u64>,
    /// None means candidate GPU resource-time in dispatch windows exceeded
    /// baseline resource-time; both raw values remain available.
    pub gpu_resource_time_reduction_unit_ns: Option<u128>,
    pub job_results: Vec<WorkloadJobReplayResult>,
    pub claim_ceiling: &'static str,
}

fn effective_capacity_at(
    now_ns: u64,
    full_capacity: ResourceVector,
    windows: &[DispatchCapacityWindow],
) -> ResourceVector {
    let index = windows.partition_point(|window| window.end_ns <= now_ns);
    windows
        .get(index)
        .filter(|window| window.start_ns <= now_ns && now_ns < window.end_ns)
        .map(|window| window.admission_capacity)
        .unwrap_or(full_capacity)
}

type ReadyPriority = (u8, u64, u64, u64, String, usize);

fn ready_priority(job: &WorkloadJob, index: usize) -> ReadyPriority {
    let class_rank = match job.class {
        WorkloadClass::NonDeferrable => 0,
        WorkloadClass::Deferrable => 1,
    };
    (
        class_rank,
        job.latest_start_ns,
        job.completion_deadline_ns,
        job.release_time_ns,
        job.job_id.clone(),
        index,
    )
}

fn next_event_time(
    now_ns: u64,
    jobs: &[WorkloadJob],
    release_order: &[usize],
    next_release_index: usize,
    ready: &BTreeSet<ReadyPriority>,
    running: &BTreeSet<(u64, usize)>,
    windows: &[DispatchCapacityWindow],
) -> Option<u64> {
    let mut next = None;
    let mut consider = |time_ns: u64| {
        if time_ns > now_ns {
            next = Some(next.map_or(time_ns, |current: u64| current.min(time_ns)));
        }
    };

    if let Some(index) = release_order.get(next_release_index) {
        consider(jobs[*index].release_time_ns);
    }
    if let Some(priority) = ready.first() {
        consider(priority.1);
    }
    if let Some((finish_ns, _)) = running.first() {
        consider(*finish_ns);
    }
    for window in windows {
        consider(window.start_ns);
        consider(window.end_ns);
    }
    next
}

fn run_schedule(
    jobs: &[WorkloadJob],
    full_capacity: ResourceVector,
    windows: &[DispatchCapacityWindow],
) -> Result<Vec<JobState>, AuditError> {
    let mut states = vec![JobState::Pending; jobs.len()];
    let mut used_resources = ResourceVector::default();
    let mut completed_or_missed_count = 0_usize;
    let Some(mut now_ns) = jobs.iter().map(|job| job.release_time_ns).min() else {
        return Err(AuditError(
            "workload replay requires at least one job".to_string(),
        ));
    };

    // Sort arrivals once, then maintain ordered ready and completion sets.
    // The v0.1 policy is strict priority: non-deferrable first, then earliest
    // latest-start, completion deadline, release time and stable job ID.
    // It deliberately does not backfill around a higher-priority job that
    // cannot currently fit; alternative policies can be compared later.
    let mut release_order: Vec<usize> = (0..jobs.len()).collect();
    release_order.sort_by(|left, right| {
        jobs[*left]
            .release_time_ns
            .cmp(&jobs[*right].release_time_ns)
            .then_with(|| {
                ready_priority(&jobs[*left], *left)
                    .cmp(&ready_priority(&jobs[*right], *right))
            })
    });
    let mut next_release_index = 0_usize;
    let mut ready: BTreeSet<ReadyPriority> = BTreeSet::new();
    let mut running: BTreeSet<(u64, usize)> = BTreeSet::new();

    loop {
        // Complete only jobs whose finish event is due. This avoids scanning
        // every job at each scheduler event.
        while let Some((finish_ns, index)) = running.first().copied() {
            if finish_ns > now_ns {
                break;
            }
            running.remove(&(finish_ns, index));
            let JobState::Running {
                start_ns,
                finish_ns: expected_finish_ns,
            } = states[index]
            else {
                return Err(AuditError(
                    "running-completion queue is inconsistent with job state".to_string(),
                ));
            };
            if expected_finish_ns != finish_ns {
                return Err(AuditError(
                    "running-completion timestamp does not match job state".to_string(),
                ));
            }
            used_resources = used_resources
                .checked_sub(jobs[index].resource_demand)
                .ok_or_else(|| {
                    AuditError("completed job resource accounting underflowed".to_string())
                })?;
            states[index] = JobState::Completed {
                start_ns,
                finish_ns,
            };
            completed_or_missed_count += 1;
        }

        while let Some(index) = release_order.get(next_release_index).copied() {
            if jobs[index].release_time_ns > now_ns {
                break;
            }
            ready.insert(ready_priority(&jobs[index], index));
            next_release_index += 1;
        }

        loop {
            let Some(priority) = ready.first().cloned() else {
                break;
            };
            let index = priority.5;
            let admission_capacity =
                effective_capacity_at(now_ns, full_capacity, windows);
            let after = used_resources.checked_add(jobs[index].resource_demand);

            if let Some(after) = after.filter(|resources| resources.fits_within(admission_capacity)) {
                used_resources = after;
                ready.remove(&priority);
                let finish_ns = now_ns
                    .checked_add(jobs[index].runtime_ns)
                    .ok_or_else(|| {
                        AuditError(format!(
                            "job {:?} finish timestamp overflowed",
                            jobs[index].job_id
                        ))
                    })?;
                if finish_ns > jobs[index].completion_deadline_ns {
                    states[index] = JobState::Missed;
                    completed_or_missed_count += 1;
                    continue;
                }
                states[index] = JobState::Running {
                    start_ns: now_ns,
                    finish_ns,
                };
                if !running.insert((finish_ns, index)) {
                    return Err(AuditError(
                        "duplicate running-completion event was generated".to_string(),
                    ));
                }
                continue;
            }

            // At the exact latest-start instant, failure to fit is a miss.
            // Before that instant, wait for a completion or capacity change.
            if jobs[index].latest_start_ns <= now_ns {
                ready.remove(&priority);
                states[index] = JobState::Missed;
                completed_or_missed_count += 1;
                continue;
            }
            break;
        }

        if completed_or_missed_count == jobs.len()
            && next_release_index == release_order.len()
        {
            break;
        }

        let Some(next_ns) = next_event_time(
            now_ns,
            jobs,
            &release_order,
            next_release_index,
            &ready,
            &running,
            windows,
        ) else {
            // No future release, completion, latest-start or capacity
            // transition remains. Any residual ready work cannot be admitted.
            for priority in &ready {
                states[priority.5] = JobState::Missed;
                completed_or_missed_count += 1;
            }
            break;
        };
        if next_ns <= now_ns {
            return Err(AuditError(
                "workload replay event clock failed to advance".to_string(),
            ));
        }
        now_ns = next_ns;
    }

    Ok(states)
}

fn gpu_resource_time_during_windows(
    jobs: &[WorkloadJob],
    states: &[JobState],
    windows: &[DispatchCapacityWindow],
) -> Result<u128, AuditError> {
    let mut total = 0_u128;
    for (job, state) in jobs.iter().zip(states.iter()) {
        let JobState::Completed {
            start_ns,
            finish_ns,
        } = *state
        else {
            continue;
        };
        if job.resource_demand.gpu_units == 0 {
            continue;
        }
        let first_window = windows.partition_point(|window| window.end_ns <= start_ns);
        for window in &windows[first_window..] {
            if window.start_ns >= finish_ns {
                break;
            }
            let overlap_start = start_ns.max(window.start_ns);
            let overlap_end = finish_ns.min(window.end_ns);
            if overlap_end > overlap_start {
                let duration_ns = (overlap_end - overlap_start) as u128;
                let resource_time = (job.resource_demand.gpu_units as u128)
                    .checked_mul(duration_ns)
                    .ok_or_else(|| {
                        AuditError("GPU resource-time integral overflowed".to_string())
                    })?;
                total = total.checked_add(resource_time).ok_or_else(|| {
                    AuditError("GPU resource-time sum overflowed".to_string())
                })?;
            }
        }
    }
    Ok(total)
}

fn run_summary(
    jobs: &[WorkloadJob],
    states: &[JobState],
    windows_for_measurement: &[DispatchCapacityWindow],
) -> Result<WorkloadRunSummary, AuditError> {
    let mut summary = WorkloadRunSummary {
        total_job_count: jobs.len(),
        completed_job_count: 0,
        missed_job_count: 0,
        non_deferrable_missed_job_count: 0,
        deferrable_missed_job_count: 0,
        total_useful_work_units: 0,
        completed_useful_work_units: 0,
        missed_useful_work_units: 0,
        gpu_resource_time_during_dispatch_unit_ns: 0,
    };

    for (job, state) in jobs.iter().zip(states.iter()) {
        summary.total_useful_work_units = summary
            .total_useful_work_units
            .checked_add(job.useful_work_units)
            .ok_or_else(|| AuditError("total useful-work units overflowed".to_string()))?;
        match state {
            JobState::Completed { .. } => {
                summary.completed_job_count += 1;
                summary.completed_useful_work_units = summary
                    .completed_useful_work_units
                    .checked_add(job.useful_work_units)
                    .ok_or_else(|| {
                        AuditError("completed useful-work units overflowed".to_string())
                    })?;
            }
            JobState::Missed => {
                summary.missed_job_count += 1;
                summary.missed_useful_work_units = summary
                    .missed_useful_work_units
                    .checked_add(job.useful_work_units)
                    .ok_or_else(|| {
                        AuditError("missed useful-work units overflowed".to_string())
                    })?;
                match job.class {
                    WorkloadClass::NonDeferrable => {
                        summary.non_deferrable_missed_job_count += 1;
                    }
                    WorkloadClass::Deferrable => {
                        summary.deferrable_missed_job_count += 1;
                    }
                }
            }
            JobState::Pending | JobState::Running { .. } => {
                return Err(AuditError(
                    "workload replay ended with pending or running jobs".to_string(),
                ));
            }
        }
    }
    summary.gpu_resource_time_during_dispatch_unit_ns =
        gpu_resource_time_during_windows(jobs, states, windows_for_measurement)?;
    Ok(summary)
}

fn nearest_rank_positive(values: &mut [u64], quantile: f64) -> Option<u64> {
    if values.is_empty() || !quantile.is_finite() || !(0.0..=1.0).contains(&quantile) || quantile == 0.0 {
        return None;
    }
    values.sort_unstable();
    let rank = (quantile * values.len() as f64).ceil() as usize;
    values.get(rank.saturating_sub(1)).copied()
}

fn gpu_admission_restriction_time(
    full_capacity: ResourceVector,
    windows: &[DispatchCapacityWindow],
) -> Result<(u128, u128, u64), AuditError> {
    let mut restriction = 0_u128;
    let mut total_duration = 0_u128;
    let mut longest = 0_u64;
    for window in windows {
        let duration = window.duration_ns();
        let withheld_gpu_units =
            full_capacity.gpu_units - window.admission_capacity.gpu_units;
        let units_ns = (withheld_gpu_units as u128)
            .checked_mul(duration as u128)
            .ok_or_else(|| {
                AuditError("GPU admission restriction integral overflowed".to_string())
            })?;
        restriction = restriction.checked_add(units_ns).ok_or_else(|| {
            AuditError("GPU admission restriction sum overflowed".to_string())
        })?;
        total_duration = total_duration
            .checked_add(duration as u128)
            .ok_or_else(|| AuditError("dispatch duration sum overflowed".to_string()))?;
        longest = longest.max(duration);
    }
    Ok((restriction, total_duration, longest))
}

/// Compare the same normalized job set under a full-capacity baseline and a
/// dispatch-constrained admission-capacity profile.
///
/// The v0.1 scheduler is deterministic, non-preemptive and event-driven. A
/// dispatch window limits new job admission only; in-flight jobs continue.
/// Outputs are resource-time and service metrics, never watts, joules or
/// accredited grid capacity.
pub fn evaluate_workload_replay(
    contract: WorkloadReplayContract,
    jobs: Vec<WorkloadJob>,
    dispatch_windows: Vec<DispatchCapacityWindow>,
) -> Result<WorkloadReplayReport, AuditError> {
    contract.validate()?;
    if jobs.is_empty() {
        return Err(AuditError(
            "workload replay requires at least one job".to_string(),
        ));
    }

    let mut job_ids = HashSet::new();
    for job in &jobs {
        job.validate(contract.full_resource_capacity)?;
        if !job_ids.insert(job.job_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate workload job ID {:?}",
                job.job_id
            )));
        }
    }

    let mut window_ids = HashSet::new();
    let mut previous_end_ns = None;
    for window in &dispatch_windows {
        window.validate(contract.full_resource_capacity)?;
        if !window_ids.insert(window.window_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate dispatch window ID {:?}",
                window.window_id
            )));
        }
        if previous_end_ns.is_some_and(|end| window.start_ns < end) {
            return Err(AuditError(
                "dispatch capacity windows must be ordered and non-overlapping".to_string(),
            ));
        }
        previous_end_ns = Some(window.end_ns);
    }

    let baseline_states = run_schedule(&jobs, contract.full_resource_capacity, &[])?;
    let dispatch_states = run_schedule(
        &jobs,
        contract.full_resource_capacity,
        &dispatch_windows,
    )?;

    // Measure both schedules over the exact same dispatch-window intervals.
    let baseline = run_summary(&jobs, &baseline_states, &dispatch_windows)?;
    let dispatch_constrained =
        run_summary(&jobs, &dispatch_states, &dispatch_windows)?;

    let useful_work_retention_fraction = if baseline.completed_useful_work_units > 0 {
        Some(
            dispatch_constrained.completed_useful_work_units as f64
                / baseline.completed_useful_work_units as f64,
        )
    } else {
        None
    };

    let mut job_results = Vec::with_capacity(jobs.len());
    let mut positive_delays = Vec::new();
    let mut deferred_and_completed_job_count = 0_usize;
    let mut deferred_and_completed_work_units = 0_u64;
    let mut recovery_tail_after_last_dispatch_ns = None;
    let last_dispatch_end_ns = dispatch_windows.last().map(|window| window.end_ns);

    for index in 0..jobs.len() {
        let baseline_times = match baseline_states[index] {
            JobState::Completed {
                start_ns,
                finish_ns,
            } => (Some(start_ns), Some(finish_ns)),
            _ => (None, None),
        };
        let dispatch_times = match dispatch_states[index] {
            JobState::Completed {
                start_ns,
                finish_ns,
            } => (Some(start_ns), Some(finish_ns)),
            _ => (None, None),
        };
        let positive_start_delay_ns = baseline_times
            .0
            .zip(dispatch_times.0)
            .map(|(baseline_start, dispatch_start)| {
                dispatch_start.saturating_sub(baseline_start)
            });
        let deferred_and_completed = positive_start_delay_ns.is_some_and(|delay| delay > 0)
            && dispatch_times.1.is_some();

        if let Some(delay) = positive_start_delay_ns.filter(|delay| *delay > 0) {
            if dispatch_times.1.is_some() {
                positive_delays.push(delay);
                deferred_and_completed_job_count += 1;
                deferred_and_completed_work_units = deferred_and_completed_work_units
                    .checked_add(jobs[index].useful_work_units)
                    .ok_or_else(|| {
                        AuditError("deferred useful-work units overflowed".to_string())
                    })?;
                if let (Some(finish), Some(last_end)) =
                    (dispatch_times.1, last_dispatch_end_ns)
                {
                    let tail = finish.saturating_sub(last_end);
                    recovery_tail_after_last_dispatch_ns = Some(
                        recovery_tail_after_last_dispatch_ns
                            .map_or(tail, |current: u64| current.max(tail)),
                    );
                }
            }
        }

        job_results.push(WorkloadJobReplayResult {
            job_id: jobs[index].job_id.clone(),
            class: jobs[index].class,
            baseline_start_ns: baseline_times.0,
            baseline_finish_ns: baseline_times.1,
            dispatch_start_ns: dispatch_times.0,
            dispatch_finish_ns: dispatch_times.1,
            positive_start_delay_ns,
            baseline_deadline_met: baseline_times.1
                .is_some_and(|finish| finish <= jobs[index].completion_deadline_ns),
            dispatch_deadline_met: dispatch_times.1
                .is_some_and(|finish| finish <= jobs[index].completion_deadline_ns),
            useful_work_units: jobs[index].useful_work_units,
            useful_work_completed_in_dispatch_replay: if dispatch_times.1.is_some() {
                jobs[index].useful_work_units
            } else {
                0
            },
            deferred_and_completed,
        });
    }

    let (gpu_admission_restriction_unit_ns, total_dispatch_window_ns, longest_dispatch_window_ns) =
        gpu_admission_restriction_time(contract.full_resource_capacity, &dispatch_windows)?;
    let gpu_resource_time_reduction_unit_ns = baseline
        .gpu_resource_time_during_dispatch_unit_ns
        .checked_sub(dispatch_constrained.gpu_resource_time_during_dispatch_unit_ns);

    let p50_positive_start_delay_ns = nearest_rank_positive(&mut positive_delays.clone(), 0.50);
    let p95_positive_start_delay_ns = nearest_rank_positive(&mut positive_delays.clone(), 0.95);
    let p99_positive_start_delay_ns = nearest_rank_positive(&mut positive_delays.clone(), 0.99);
    let maximum_positive_start_delay_ns = positive_delays.iter().copied().max();

    Ok(WorkloadReplayReport {
        contract,
        baseline,
        dispatch_constrained,
        useful_work_retention_fraction,
        deferred_and_completed_job_count,
        deferred_and_completed_work_units,
        p50_positive_start_delay_ns,
        p95_positive_start_delay_ns,
        p99_positive_start_delay_ns,
        maximum_positive_start_delay_ns,
        gpu_admission_restriction_unit_ns,
        total_dispatch_window_ns,
        longest_dispatch_window_ns,
        recovery_tail_after_last_dispatch_ns,
        gpu_resource_time_reduction_unit_ns,
        job_results,
        claim_ceiling: "WORKLOAD_REPLAY_RESOURCE_TIME_ONLY_NOT_POWER_OR_GRID_QUALIFICATION",
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_workload_replay, DispatchCapacityWindow, ResourceVector, WorkloadClass,
        WorkloadJob, WorkloadReplayContract,
    };

    fn capacity() -> ResourceVector {
        ResourceVector {
            cpu_millicores: 8_000,
            gpu_units: 4,
            memory_mib: 64_000,
        }
    }

    fn contract() -> WorkloadReplayContract {
        WorkloadReplayContract {
            replay_contract_id: "replay-v1".to_string(),
            workload_id: "workload-synthetic-01".to_string(),
            dataset_id: "synthetic-fixture".to_string(),
            dataset_version: "1".to_string(),
            dataset_evidence_id: "synthetic-source-evidence".to_string(),
            use_authorization_reference: "synthetic-only-authorization".to_string(),
            normalization_id: "normalization-v1".to_string(),
            scheduling_policy_id: "earliest-latest-start-deadline-release-id-v1".to_string(),
            work_unit: "accepted-job".to_string(),
            full_resource_capacity: capacity(),
        }
    }

    fn job(id: &str, class: WorkloadClass, release: u64, latest: u64,
           deadline: u64, runtime: u64, demand: ResourceVector, work: u64) -> WorkloadJob {
        WorkloadJob {
            job_id: id.to_string(),
            source_evidence_id: format!("evidence-{id}"),
            class,
            release_time_ns: release,
            latest_start_ns: latest,
            completion_deadline_ns: deadline,
            runtime_ns: runtime,
            resource_demand: demand,
            useful_work_units: work,
        }
    }

    fn gpu_job(id: &str, class: WorkloadClass, release: u64, latest: u64,
               deadline: u64, runtime: u64, gpu_units: u64, work: u64) -> WorkloadJob {
        job(
            id,
            class,
            release,
            latest,
            deadline,
            runtime,
            ResourceVector {
                cpu_millicores: 0,
                gpu_units,
                memory_mib: 1_000,
            },
            work,
        )
    }

    fn window(id: &str, start: u64, end: u64, gpu_units: u64) -> DispatchCapacityWindow {
        DispatchCapacityWindow {
            window_id: id.to_string(),
            start_ns: start,
            end_ns: end,
            admission_capacity: ResourceVector {
                cpu_millicores: 8_000,
                gpu_units,
                memory_mib: 64_000,
            },
        }
    }

    #[test]
    fn identical_baseline_and_empty_dispatch_profile_are_deterministic() {
        let jobs = vec![
            gpu_job("job-a", WorkloadClass::NonDeferrable, 0, 0, 10, 5, 2, 1),
            gpu_job("job-b", WorkloadClass::NonDeferrable, 0, 0, 10, 5, 2, 1),
        ];
        let report =
            evaluate_workload_replay(contract(), jobs, vec![]).unwrap();
        assert_eq!(report.baseline.completed_job_count, 2);
        assert_eq!(report.dispatch_constrained.completed_job_count, 2);
        assert_eq!(report.useful_work_retention_fraction, Some(1.0));
        assert_eq!(report.job_results[0].baseline_start_ns, Some(0));
        assert_eq!(report.job_results[0].dispatch_start_ns, Some(0));
        assert_eq!(report.job_results[1].baseline_start_ns, Some(5));
        assert_eq!(report.job_results[1].dispatch_start_ns, Some(5));
    }

    #[test]
    fn deferrable_work_shifts_to_capacity_restoration_and_completes() {
        let jobs = vec![gpu_job(
            "batch",
            WorkloadClass::Deferrable,
            0,
            10,
            20,
            5,
            2,
            100,
        )];
        let report =
            evaluate_workload_replay(contract(), jobs, vec![window("dispatch", 0, 10, 0)])
                .unwrap();
        assert_eq!(report.baseline.completed_job_count, 1);
        assert_eq!(report.dispatch_constrained.completed_job_count, 1);
        assert_eq!(report.job_results[0].baseline_start_ns, Some(0));
        assert_eq!(report.job_results[0].dispatch_start_ns, Some(10));
        assert_eq!(report.job_results[0].dispatch_finish_ns, Some(15));
        assert_eq!(report.useful_work_retention_fraction, Some(1.0));
        assert_eq!(report.deferred_and_completed_work_units, 100);
        assert_eq!(report.maximum_positive_start_delay_ns, Some(10));
        assert_eq!(report.recovery_tail_after_last_dispatch_ns, Some(5));
        assert!(report.gpu_resource_time_reduction_unit_ns.is_some());
    }

    #[test]
    fn non_deferrable_work_is_not_delayed_to_improve_flexibility_headline() {
        let jobs = vec![gpu_job(
            "interactive",
            WorkloadClass::NonDeferrable,
            0,
            0,
            20,
            5,
            2,
            10,
        )];
        let report =
            evaluate_workload_replay(contract(), jobs, vec![window("dispatch", 0, 10, 0)])
                .unwrap();
        assert_eq!(report.baseline.completed_job_count, 1);
        assert_eq!(report.dispatch_constrained.completed_job_count, 0);
        assert_eq!(report.dispatch_constrained.non_deferrable_missed_job_count, 1);
        assert_eq!(report.useful_work_retention_fraction, Some(0.0));
        assert_eq!(report.job_results[0].dispatch_start_ns, None);
    }

    #[test]
    fn already_running_job_is_not_preempted_when_dispatch_window_starts() {
        let jobs = vec![
            gpu_job("in-flight", WorkloadClass::NonDeferrable, 0, 0, 20, 10, 4, 1),
            gpu_job("queued", WorkloadClass::Deferrable, 5, 15, 25, 5, 4, 1),
        ];
        let report =
            evaluate_workload_replay(contract(), jobs, vec![window("dispatch", 5, 12, 0)])
                .unwrap();
        assert_eq!(report.job_results[0].dispatch_start_ns, Some(0));
        assert_eq!(report.job_results[0].dispatch_finish_ns, Some(10));
        assert_eq!(report.job_results[1].dispatch_start_ns, Some(12));
        assert_eq!(report.job_results[1].dispatch_finish_ns, Some(17));
    }

    #[test]
    fn cpu_gpu_and_memory_are_enforced_as_a_resource_vector() {
        let jobs = vec![job(
            "memory-bound",
            WorkloadClass::Deferrable,
            0,
            10,
            20,
            5,
            ResourceVector {
                cpu_millicores: 2_000,
                gpu_units: 1,
                memory_mib: 32_000,
            },
            1,
        )];
        let cap = DispatchCapacityWindow {
            window_id: "memory-cap".to_string(),
            start_ns: 0,
            end_ns: 10,
            admission_capacity: ResourceVector {
                cpu_millicores: 8_000,
                gpu_units: 4,
                memory_mib: 16_000,
            },
        };
        let report = evaluate_workload_replay(contract(), jobs, vec![cap]).unwrap();
        assert_eq!(report.job_results[0].dispatch_start_ns, Some(10));
        assert_eq!(report.job_results[0].dispatch_finish_ns, Some(15));
    }

    #[test]
    fn exact_deadline_start_is_admissible_but_later_start_is_not() {
        let jobs = vec![gpu_job(
            "deadline-cliff",
            WorkloadClass::Deferrable,
            0,
            10,
            15,
            5,
            2,
            1,
        )];
        let report =
            evaluate_workload_replay(contract(), jobs, vec![window("dispatch", 0, 10, 0)])
                .unwrap();
        assert_eq!(report.job_results[0].dispatch_start_ns, Some(10));
        assert_eq!(report.job_results[0].dispatch_finish_ns, Some(15));

        let jobs = vec![gpu_job(
            "past-deadline",
            WorkloadClass::Deferrable,
            0,
            9,
            14,
            5,
            2,
            1,
        )];
        let report = evaluate_workload_replay(
            contract(),
            jobs,
            vec![window("dispatch", 0, 10, 0)]
        )
        .unwrap();
        assert_eq!(report.dispatch_constrained.missed_job_count, 1);
        assert_eq!(report.job_results[0].dispatch_start_ns, None);
    }

    #[test]
    fn restored_capacity_drains_backlog_without_losing_work() {
        let jobs = vec![
            gpu_job("batch-a", WorkloadClass::Deferrable, 0, 10, 30, 5, 2, 1),
            gpu_job("batch-b", WorkloadClass::Deferrable, 0, 15, 35, 5, 2, 1),
        ];
        let report =
            evaluate_workload_replay(contract(), jobs, vec![window("dispatch", 0, 10, 0)])
                .unwrap();
        assert_eq!(report.dispatch_constrained.completed_job_count, 2);
        assert_eq!(report.dispatch_constrained.completed_useful_work_units, 2);
        assert_eq!(report.job_results[0].dispatch_start_ns, Some(10));
        assert_eq!(report.job_results[1].dispatch_start_ns, Some(15));
    }

    #[test]
    fn dispatch_resource_time_is_not_mislabeled_as_watts_or_energy() {
        let jobs = vec![gpu_job("batch", WorkloadClass::Deferrable, 0, 10, 20, 5, 2, 1)];
        let report =
            evaluate_workload_replay(contract(), jobs, vec![window("dispatch", 0, 10, 0)])
                .unwrap();
        assert_eq!(report.baseline.gpu_resource_time_during_dispatch_unit_ns, 10);
        assert_eq!(
            report.dispatch_constrained.gpu_resource_time_during_dispatch_unit_ns,
            0
        );
        assert_eq!(report.gpu_admission_restriction_unit_ns, 40);
        assert_eq!(
            report.claim_ceiling,
            "WORKLOAD_REPLAY_RESOURCE_TIME_ONLY_NOT_POWER_OR_GRID_QUALIFICATION"
        );
    }

    #[test]
    fn stable_job_id_breaks_equal_priority_ties() {
        let jobs = vec![
            gpu_job("z-job", WorkloadClass::NonDeferrable, 0, 0, 10, 5, 4, 1),
            gpu_job("a-job", WorkloadClass::NonDeferrable, 0, 0, 10, 5, 4, 1),
        ];
        let report = evaluate_workload_replay(contract(), jobs, vec![]).unwrap();
        let a_job = report.job_results.iter().find(|item| item.job_id == "a-job").unwrap();
        let z_job = report.job_results.iter().find(|item| item.job_id == "z-job").unwrap();
        assert_eq!(a_job.baseline_start_ns, Some(0));
        assert_eq!(z_job.baseline_start_ns, Some(5));
    }

    #[test]
    fn abutting_dispatch_windows_are_valid_but_overlapping_windows_are_rejected() {
        let jobs = vec![gpu_job("batch", WorkloadClass::Deferrable, 0, 20, 30, 5, 2, 1)];
        let abutting = vec![
            window("first", 0, 10, 0),
            window("second", 10, 20, 1),
        ];
        assert!(evaluate_workload_replay(contract(), jobs.clone(), abutting).is_ok());

        let overlapping = vec![
            window("first", 0, 10, 0),
            window("second", 9, 20, 1),
        ];
        assert!(evaluate_workload_replay(contract(), jobs, overlapping).is_err());
    }

    #[test]
    fn impossible_resource_requests_and_invalid_windows_fail_closed() {
        let impossible = vec![gpu_job(
            "too-large",
            WorkloadClass::Deferrable,
            0,
            10,
            20,
            5,
            5,
            1,
        )];
        assert!(evaluate_workload_replay(contract(), impossible, vec![]).is_err());

        let jobs = vec![gpu_job("valid", WorkloadClass::Deferrable, 0, 10, 20, 5, 2, 1)];
        assert!(evaluate_workload_replay(
            contract(),
            jobs.clone(),
            vec![window("invalid-cap", 0, 10, 5)]
        )
        .is_err());
        assert!(evaluate_workload_replay(
            contract(),
            jobs,
            vec![window("invalid-time", 10, 10, 0)]
        )
        .is_err());
    }

    #[test]
    fn invalid_deadlines_nondeferrable_slack_and_duplicate_ids_are_rejected() {
        let invalid_deadline = vec![gpu_job(
            "too-late",
            WorkloadClass::Deferrable,
            0,
            10,
            14,
            5,
            2,
            1,
        )];
        assert!(evaluate_workload_replay(contract(), invalid_deadline, vec![]).is_err());

        let nondeferrable_slack = vec![gpu_job(
            "interactive",
            WorkloadClass::NonDeferrable,
            0,
            1,
            10,
            5,
            2,
            1,
        )];
        assert!(evaluate_workload_replay(contract(), nondeferrable_slack, vec![]).is_err());

        let duplicate = vec![
            gpu_job("same", WorkloadClass::Deferrable, 0, 10, 20, 5, 2, 1),
            gpu_job("same", WorkloadClass::Deferrable, 0, 10, 20, 5, 2, 1),
        ];
        assert!(evaluate_workload_replay(contract(), duplicate, vec![]).is_err());
    }

    #[test]
    fn missing_provenance_authorization_or_work_unit_is_rejected() {
        let jobs = vec![gpu_job("valid", WorkloadClass::Deferrable, 0, 10, 20, 5, 2, 1)];
        let mut no_authorization = contract();
        no_authorization.use_authorization_reference.clear();
        assert!(evaluate_workload_replay(no_authorization, jobs.clone(), vec![]).is_err());

        let mut no_work_unit = contract();
        no_work_unit.work_unit.clear();
        assert!(evaluate_workload_replay(no_work_unit, jobs, vec![]).is_err());
    }

    #[test]
    fn recovery_tail_and_work_loss_are_reported_separately() {
        let jobs = vec![
            gpu_job("shifted", WorkloadClass::Deferrable, 0, 10, 20, 5, 2, 10),
            gpu_job("missed", WorkloadClass::Deferrable, 0, 5, 10, 5, 2, 20),
        ];
        let report =
            evaluate_workload_replay(contract(), jobs, vec![window("dispatch", 0, 10, 0)])
                .unwrap();
        assert_eq!(report.dispatch_constrained.completed_job_count, 1);
        assert_eq!(report.dispatch_constrained.missed_job_count, 1);
        assert_eq!(report.dispatch_constrained.missed_useful_work_units, 20);
        assert_eq!(report.deferred_and_completed_work_units, 10);
        assert_eq!(report.recovery_tail_after_last_dispatch_ns, Some(5));
        assert_eq!(report.useful_work_retention_fraction, Some(1.0 / 3.0));
    }

    #[test]
    fn checked_arithmetic_rejects_timestamp_overflow() {
        let jobs = vec![gpu_job(
            "overflow",
            WorkloadClass::Deferrable,
            u64::MAX - 5,
            u64::MAX - 5,
            u64::MAX,
            10,
            2,
            1,
        )];
        assert!(evaluate_workload_replay(contract(), jobs, vec![]).is_err());
    }

    #[test]
    fn percentile_helpers_ignore_zero_delays_and_expose_exact_ranks() {
        let jobs = vec![
            gpu_job("a", WorkloadClass::Deferrable, 0, 10, 20, 5, 2, 1),
            gpu_job("b", WorkloadClass::Deferrable, 0, 15, 25, 5, 2, 1),
            gpu_job("c", WorkloadClass::Deferrable, 0, 20, 30, 5, 2, 1),
        ];
        let report =
            evaluate_workload_replay(contract(), jobs, vec![window("dispatch", 0, 10, 0)])
                .unwrap();
        assert_eq!(report.p50_positive_start_delay_ns, Some(10));
        assert_eq!(report.p95_positive_start_delay_ns, Some(10));
        assert_eq!(report.p99_positive_start_delay_ns, Some(10));
    }
}
