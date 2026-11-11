use crate::AuditError;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MigrationDataAccessMode {
    ReplicatedReadOnly,
    RemoteReadOnly,
    OnDemandCopy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MigrationEvidenceClass {
    Measured,
    Modelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrelaunchState {
    Neither,
    DataOnly,
    RuntimeOnly,
    DataAndRuntime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationResponseContract {
    pub contract_id: String,
    pub workload_contract_id: String,
    pub data_artifact_id: String,
    pub data_version: String,
    pub work_unit: String,
    pub required_useful_work_units: u64,
    pub source_site_id: String,
    pub destination_site_id: String,
    pub source_region_id: String,
    pub destination_region_id: String,
    pub source_pcc_boundary_id: String,
    pub destination_pcc_boundary_id: String,
    pub source_hardware_configuration_fingerprint: String,
    pub destination_hardware_configuration_fingerprint: String,
    pub use_authorization_reference: String,
    pub residency_policy_reference: String,
    pub freshness_policy_id: String,
    pub output_quality_contract_id: String,
    pub service_slo_contract_id: String,
    pub power_stability_criterion_id: String,
    pub common_clock_id: String,
    pub common_time_origin_id: String,
    pub clock_alignment_method_id: String,
    pub maximum_clock_alignment_uncertainty_ns: u64,
    pub source_relief_response_limit_ns: u64,
}

impl MigrationResponseContract {
    fn validate(&self) -> Result<(), AuditError> {
        for (value, field) in [
            (self.contract_id.as_str(), "contract_id"),
            (self.workload_contract_id.as_str(), "workload_contract_id"),
            (self.data_artifact_id.as_str(), "data_artifact_id"),
            (self.data_version.as_str(), "data_version"),
            (self.work_unit.as_str(), "work_unit"),
            (self.source_site_id.as_str(), "source_site_id"),
            (self.destination_site_id.as_str(), "destination_site_id"),
            (self.source_region_id.as_str(), "source_region_id"),
            (self.destination_region_id.as_str(), "destination_region_id"),
            (self.source_pcc_boundary_id.as_str(), "source_pcc_boundary_id"),
            (self.destination_pcc_boundary_id.as_str(), "destination_pcc_boundary_id"),
            (self.source_hardware_configuration_fingerprint.as_str(), "source_hardware_configuration_fingerprint"),
            (self.destination_hardware_configuration_fingerprint.as_str(), "destination_hardware_configuration_fingerprint"),
            (self.use_authorization_reference.as_str(), "use_authorization_reference"),
            (self.residency_policy_reference.as_str(), "residency_policy_reference"),
            (self.freshness_policy_id.as_str(), "freshness_policy_id"),
            (self.output_quality_contract_id.as_str(), "output_quality_contract_id"),
            (self.service_slo_contract_id.as_str(), "service_slo_contract_id"),
            (self.power_stability_criterion_id.as_str(), "power_stability_criterion_id"),
            (self.common_clock_id.as_str(), "common_clock_id"),
            (self.common_time_origin_id.as_str(), "common_time_origin_id"),
            (self.clock_alignment_method_id.as_str(), "clock_alignment_method_id"),
        ] {
            if value.trim().is_empty() {
                return Err(AuditError(format!(
                    "migration response contract {field} must not be blank"
                )));
            }
        }
        if self.source_site_id == self.destination_site_id {
            return Err(AuditError(
                "source and destination sites must be different".to_string(),
            ));
        }
        if self.required_useful_work_units == 0 || self.source_relief_response_limit_ns == 0 {
            return Err(AuditError(
                "required useful work and source-relief response limit must be positive".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationTrial {
    pub trial_id: String,
    pub contract_id: String,
    pub workload_contract_id: String,
    pub data_artifact_id: String,
    pub data_version: String,
    pub work_unit: String,
    pub data_bytes: u128,
    pub data_access_mode: MigrationDataAccessMode,
    pub evidence_class: MigrationEvidenceClass,
    pub source_site_id: String,
    pub destination_site_id: String,
    pub source_region_id: String,
    pub destination_region_id: String,
    pub source_pcc_boundary_id: String,
    pub destination_pcc_boundary_id: String,
    pub source_hardware_configuration_fingerprint: String,
    pub destination_hardware_configuration_fingerprint: String,
    pub common_clock_id: String,
    pub common_time_origin_id: String,
    pub clock_alignment_method_id: String,
    pub clock_alignment_evidence_id: String,
    /// Conservative uncertainty bounds for the source and destination clocks.
    pub source_clock_alignment_uncertainty_ns: u64,
    pub destination_clock_alignment_uncertainty_ns: u64,
    /// All event timestamps below are already mapped to the declared common clock.
    /// Pre-launch readiness timestamps may legitimately precede command_issued_ns.
    pub command_issued_ns: u64,
    pub data_ready_ns: Option<u64>,
    pub data_ready_evidence_id: Option<String>,
    pub runtime_ready_ns: Option<u64>,
    pub runtime_ready_evidence_id: Option<String>,
    pub destination_capacity_reserved_ns: Option<u64>,
    pub destination_capacity_evidence_id: Option<String>,
    pub destination_work_started_ns: Option<u64>,
    pub first_useful_work_accepted_ns: Option<u64>,
    pub accepted_useful_work_units: u64,
    pub accepted_work_evidence_id: String,
    pub output_quality_accepted: bool,
    pub output_quality_evidence_id: String,
    pub service_slo_met: bool,
    pub service_slo_evidence_id: String,
    pub authority_authorized: bool,
    pub authority_evidence_id: String,
    pub residency_permitted: bool,
    pub residency_evidence_id: String,
    pub data_freshness_verified: bool,
    pub data_freshness_evidence_id: String,
    pub data_path_evidence_id: String,
    pub source_power_reduction_stable_ns: Option<u64>,
    pub source_power_trace_evidence_id: String,
    pub source_pcc_measurement_reference: String,
    pub destination_power_increase_stable_ns: Option<u64>,
    pub destination_power_trace_evidence_id: String,
    pub destination_pcc_measurement_reference: String,
    pub trial_outcome_evidence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationTrialResult {
    pub trial_id: String,
    pub data_access_mode: MigrationDataAccessMode,
    pub prelaunch_state: PrelaunchState,
    pub evidence_class: MigrationEvidenceClass,
    pub source_response_latency_ns: Option<u64>,
    pub destination_power_stabilization_latency_ns: Option<u64>,
    pub data_ready_before_command: bool,
    pub runtime_ready_before_command: bool,
    pub destination_capacity_reserved_before_work: bool,
    pub response_deadline_met: bool,
    pub useful_work_preserved: bool,
    pub authority_and_residency_passed: bool,
    pub data_freshness_passed: bool,
    pub service_eligible: bool,
    pub accepted_useful_work_units: u64,
    pub data_bytes: u128,
    pub command_issued_ns: u64,
    pub data_ready_ns: Option<u64>,
    pub runtime_ready_ns: Option<u64>,
    pub destination_capacity_reserved_ns: Option<u64>,
    pub destination_work_started_ns: Option<u64>,
    pub first_useful_work_accepted_ns: Option<u64>,
    pub source_power_reduction_stable_ns: Option<u64>,
    pub destination_power_increase_stable_ns: Option<u64>,
    pub accepted_work_evidence_id: String,
    pub output_quality_accepted: bool,
    pub output_quality_evidence_id: String,
    pub service_slo_met: bool,
    pub service_slo_evidence_id: String,
    pub authority_authorized: bool,
    pub authority_evidence_id: String,
    pub residency_permitted: bool,
    pub residency_evidence_id: String,
    pub data_freshness_verified: bool,
    pub data_freshness_evidence_id: String,
    pub data_path_evidence_id: String,
    pub clock_alignment_evidence_id: String,
    pub source_power_trace_evidence_id: String,
    pub source_pcc_measurement_reference: String,
    pub destination_power_trace_evidence_id: String,
    pub destination_pcc_measurement_reference: String,
    pub trial_outcome_evidence_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MigrationStratumSummary {
    pub data_access_mode: MigrationDataAccessMode,
    pub prelaunch_state: PrelaunchState,
    pub trial_count: usize,
    pub source_response_success_count: usize,
    /// Includes every trial in the denominator, including trials without an observed stable response.
    pub source_response_success_fraction: f64,
    pub observed_response_latency_count: usize,
    pub p50_source_response_latency_ns: Option<u64>,
    pub p95_source_response_latency_ns: Option<u64>,
    pub p99_source_response_latency_ns: Option<u64>,
    pub observed_destination_stabilization_count: usize,
    pub p50_destination_stabilization_latency_ns: Option<u64>,
    pub p95_destination_stabilization_latency_ns: Option<u64>,
    pub p99_destination_stabilization_latency_ns: Option<u64>,
    pub useful_work_preserved_count: usize,
    pub service_eligible_count: usize,
    pub total_data_bytes: u128,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MigrationResponseReport {
    pub contract: MigrationResponseContract,
    pub evidence_class: MigrationEvidenceClass,
    pub trial_count: usize,
    pub strata: Vec<MigrationStratumSummary>,
    pub trial_results: Vec<MigrationTrialResult>,
    pub claim_ceiling: &'static str,
}

fn required_ref(value: &str, field: &str) -> Result<(), AuditError> {
    if value.trim().is_empty() {
        return Err(AuditError(format!(
            "migration trial {field} must not be blank"
        )));
    }
    Ok(())
}

fn validate_optional_event(
    timestamp: Option<u64>,
    evidence_id: &Option<String>,
    field: &str,
) -> Result<(), AuditError> {
    match (timestamp, evidence_id.as_deref()) {
        (Some(_), Some(value)) if !value.trim().is_empty() => Ok(()),
        (Some(_), _) => Err(AuditError(format!(
            "migration trial {field} timestamp requires evidence"
        ))),
        (None, Some(value)) if !value.trim().is_empty() => Err(AuditError(format!(
            "migration trial {field} has evidence without an observed timestamp"
        ))),
        (None, _) => Ok(()),
    }
}

fn prelaunch_state(data_ready: Option<u64>, runtime_ready: Option<u64>, command: u64) -> PrelaunchState {
    match (
        data_ready.is_some_and(|time| time <= command),
        runtime_ready.is_some_and(|time| time <= command),
    ) {
        (false, false) => PrelaunchState::Neither,
        (true, false) => PrelaunchState::DataOnly,
        (false, true) => PrelaunchState::RuntimeOnly,
        (true, true) => PrelaunchState::DataAndRuntime,
    }
}

fn nonnegative_latency(timestamp: Option<u64>, start: u64, field: &str) -> Result<Option<u64>, AuditError> {
    timestamp
        .map(|time| {
            time.checked_sub(start).ok_or_else(|| {
                AuditError(format!(
                    "migration trial {field} timestamp precedes its command timestamp"
                ))
            })
        })
        .transpose()
}

fn evaluate_trial(
    contract: &MigrationResponseContract,
    trial: &MigrationTrial,
) -> Result<MigrationTrialResult, AuditError> {
    for (value, field) in [
        (trial.trial_id.as_str(), "trial_id"),
        (trial.contract_id.as_str(), "contract_id"),
        (trial.workload_contract_id.as_str(), "workload_contract_id"),
        (trial.data_artifact_id.as_str(), "data_artifact_id"),
        (trial.data_version.as_str(), "data_version"),
        (trial.work_unit.as_str(), "work_unit"),
        (trial.accepted_work_evidence_id.as_str(), "accepted_work_evidence_id"),
        (trial.output_quality_evidence_id.as_str(), "output_quality_evidence_id"),
        (trial.service_slo_evidence_id.as_str(), "service_slo_evidence_id"),
        (trial.source_site_id.as_str(), "source_site_id"),
        (trial.destination_site_id.as_str(), "destination_site_id"),
        (trial.source_region_id.as_str(), "source_region_id"),
        (trial.destination_region_id.as_str(), "destination_region_id"),
        (trial.source_pcc_boundary_id.as_str(), "source_pcc_boundary_id"),
        (trial.destination_pcc_boundary_id.as_str(), "destination_pcc_boundary_id"),
        (trial.source_hardware_configuration_fingerprint.as_str(), "source_hardware_configuration_fingerprint"),
        (trial.destination_hardware_configuration_fingerprint.as_str(), "destination_hardware_configuration_fingerprint"),
        (trial.common_clock_id.as_str(), "common_clock_id"),
        (trial.common_time_origin_id.as_str(), "common_time_origin_id"),
        (trial.clock_alignment_method_id.as_str(), "clock_alignment_method_id"),
        (trial.clock_alignment_evidence_id.as_str(), "clock_alignment_evidence_id"),
        (trial.authority_evidence_id.as_str(), "authority_evidence_id"),
        (trial.residency_evidence_id.as_str(), "residency_evidence_id"),
        (trial.data_freshness_evidence_id.as_str(), "data_freshness_evidence_id"),
        (trial.data_path_evidence_id.as_str(), "data_path_evidence_id"),
        (trial.source_power_trace_evidence_id.as_str(), "source_power_trace_evidence_id"),
        (trial.source_pcc_measurement_reference.as_str(), "source_pcc_measurement_reference"),
        (trial.destination_power_trace_evidence_id.as_str(), "destination_power_trace_evidence_id"),
        (trial.destination_pcc_measurement_reference.as_str(), "destination_pcc_measurement_reference"),
        (trial.trial_outcome_evidence_id.as_str(), "trial_outcome_evidence_id"),
    ] {
        required_ref(value, field)?;
    }

    if trial.contract_id != contract.contract_id
        || trial.workload_contract_id != contract.workload_contract_id
        || trial.data_artifact_id != contract.data_artifact_id
        || trial.data_version != contract.data_version
        || trial.work_unit != contract.work_unit
        || trial.source_site_id != contract.source_site_id
        || trial.destination_site_id != contract.destination_site_id
        || trial.source_region_id != contract.source_region_id
        || trial.destination_region_id != contract.destination_region_id
        || trial.source_pcc_boundary_id != contract.source_pcc_boundary_id
        || trial.destination_pcc_boundary_id != contract.destination_pcc_boundary_id
        || trial.source_hardware_configuration_fingerprint
            != contract.source_hardware_configuration_fingerprint
        || trial.destination_hardware_configuration_fingerprint
            != contract.destination_hardware_configuration_fingerprint
        || trial.common_clock_id != contract.common_clock_id
        || trial.common_time_origin_id != contract.common_time_origin_id
        || trial.clock_alignment_method_id != contract.clock_alignment_method_id
    {
        return Err(AuditError(format!(
            "migration trial {:?} does not match its frozen source/destination, data, workload, boundary, configuration or clock contract",
            trial.trial_id
        )));
    }
    let relative_clock_uncertainty = trial.source_clock_alignment_uncertainty_ns
        .checked_add(trial.destination_clock_alignment_uncertainty_ns)
        .ok_or_else(|| AuditError("relative clock-alignment uncertainty overflowed".to_string()))?;
    if relative_clock_uncertainty > contract.maximum_clock_alignment_uncertainty_ns {
        return Err(AuditError(format!(
            "migration trial {:?} conservative pairwise clock-alignment uncertainty exceeds the contract limit",
            trial.trial_id
        )));
    }
    if trial.data_access_mode == MigrationDataAccessMode::ReplicatedReadOnly
        && trial.data_freshness_evidence_id.trim().is_empty()
    {
        return Err(AuditError(
            "replicated read-only migration requires data-freshness evidence".to_string(),
        ));
    }

    validate_optional_event(trial.data_ready_ns, &trial.data_ready_evidence_id, "data_ready")?;
    validate_optional_event(trial.runtime_ready_ns, &trial.runtime_ready_evidence_id, "runtime_ready")?;
    validate_optional_event(
        trial.destination_capacity_reserved_ns,
        &trial.destination_capacity_evidence_id,
        "destination_capacity_reserved",
    )?;

    if let Some(work_started) = trial.destination_work_started_ns {
        if trial.data_ready_ns.is_none_or(|time| time > work_started)
            || trial.runtime_ready_ns.is_none_or(|time| time > work_started)
            || trial.destination_capacity_reserved_ns.is_none_or(|time| time > work_started)
        {
            return Err(AuditError(format!(
                "migration trial {:?} starts destination work before data, runtime or capacity readiness",
                trial.trial_id
            )));
        }
        if work_started < trial.command_issued_ns {
            return Err(AuditError(
                "destination work start cannot precede the migration command".to_string(),
            ));
        }
    }
    match (trial.accepted_useful_work_units, trial.first_useful_work_accepted_ns) {
        (0, Some(_)) => {
            return Err(AuditError(
                "first useful-work acceptance timestamp is present but accepted work is zero".to_string(),
            ));
        }
        (0, None) => {}
        (_, Some(accepted)) => {
            let started = trial.destination_work_started_ns.ok_or_else(|| {
                AuditError("accepted work requires a destination work-start timestamp".to_string())
            })?;
            if accepted < started {
                return Err(AuditError(
                    "first accepted useful work cannot precede destination work start".to_string(),
                ));
            }
        }
        (_, None) => {
            return Err(AuditError(
                "accepted work requires a first-acceptance timestamp".to_string(),
            ));
        }
    }

    let source_latency = nonnegative_latency(
        trial.source_power_reduction_stable_ns,
        trial.command_issued_ns,
        "source_power_reduction_stable",
    )?;
    let destination_latency = nonnegative_latency(
        trial.destination_power_increase_stable_ns,
        trial.command_issued_ns,
        "destination_power_increase_stable",
    )?;
    let prelaunch = prelaunch_state(
        trial.data_ready_ns,
        trial.runtime_ready_ns,
        trial.command_issued_ns,
    );
    let response_deadline_met = source_latency
        .is_some_and(|latency| latency <= contract.source_relief_response_limit_ns);
    let useful_work_preserved = trial.accepted_useful_work_units
        >= contract.required_useful_work_units
        && trial.output_quality_accepted
        && trial.service_slo_met;
    let authority_and_residency_passed =
        trial.authority_authorized && trial.residency_permitted;
    let data_freshness_passed = trial.data_freshness_verified;
    let capacity_ready_before_work = match (
        trial.destination_capacity_reserved_ns,
        trial.destination_work_started_ns,
    ) {
        (Some(reserved), Some(started)) => reserved <= started,
        _ => false,
    };
    let service_eligible = response_deadline_met
        && useful_work_preserved
        && authority_and_residency_passed
        && data_freshness_passed
        && capacity_ready_before_work;

    Ok(MigrationTrialResult {
        trial_id: trial.trial_id.clone(),
        data_access_mode: trial.data_access_mode,
        prelaunch_state: prelaunch,
        evidence_class: trial.evidence_class,
        source_response_latency_ns: source_latency,
        destination_power_stabilization_latency_ns: destination_latency,
        data_ready_before_command: trial.data_ready_ns.is_some_and(|time| time <= trial.command_issued_ns),
        runtime_ready_before_command: trial.runtime_ready_ns.is_some_and(|time| time <= trial.command_issued_ns),
        destination_capacity_reserved_before_work: capacity_ready_before_work,
        response_deadline_met,
        useful_work_preserved,
        authority_and_residency_passed,
        data_freshness_passed,
        service_eligible,
        accepted_useful_work_units: trial.accepted_useful_work_units,
        data_bytes: trial.data_bytes,
        command_issued_ns: trial.command_issued_ns,
        data_ready_ns: trial.data_ready_ns,
        runtime_ready_ns: trial.runtime_ready_ns,
        destination_capacity_reserved_ns: trial.destination_capacity_reserved_ns,
        destination_work_started_ns: trial.destination_work_started_ns,
        first_useful_work_accepted_ns: trial.first_useful_work_accepted_ns,
        source_power_reduction_stable_ns: trial.source_power_reduction_stable_ns,
        destination_power_increase_stable_ns: trial.destination_power_increase_stable_ns,
        accepted_work_evidence_id: trial.accepted_work_evidence_id.clone(),
        output_quality_accepted: trial.output_quality_accepted,
        output_quality_evidence_id: trial.output_quality_evidence_id.clone(),
        service_slo_met: trial.service_slo_met,
        service_slo_evidence_id: trial.service_slo_evidence_id.clone(),
        authority_authorized: trial.authority_authorized,
        authority_evidence_id: trial.authority_evidence_id.clone(),
        residency_permitted: trial.residency_permitted,
        residency_evidence_id: trial.residency_evidence_id.clone(),
        data_freshness_verified: trial.data_freshness_verified,
        data_freshness_evidence_id: trial.data_freshness_evidence_id.clone(),
        data_path_evidence_id: trial.data_path_evidence_id.clone(),
        clock_alignment_evidence_id: trial.clock_alignment_evidence_id.clone(),
        source_power_trace_evidence_id: trial.source_power_trace_evidence_id.clone(),
        source_pcc_measurement_reference: trial.source_pcc_measurement_reference.clone(),
        destination_power_trace_evidence_id: trial.destination_power_trace_evidence_id.clone(),
        destination_pcc_measurement_reference: trial.destination_pcc_measurement_reference.clone(),
        trial_outcome_evidence_id: trial.trial_outcome_evidence_id.clone(),
    })
}

fn nearest_rank(values: &[u64], numerator: usize, denominator: usize) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let n = sorted.len() as u128;
    let rank = ((n * numerator as u128 + denominator as u128 - 1) / denominator as u128)
        .max(1) as usize;
    Some(sorted[rank - 1])
}

/// Evaluate a matched set of migration trials. All trials remain in each stratum's
/// denominator; response percentiles are computed only over observed stable events.
pub fn evaluate_migration_response_trials(
    contract: MigrationResponseContract,
    trials: &[MigrationTrial],
) -> Result<MigrationResponseReport, AuditError> {
    contract.validate()?;
    if trials.is_empty() {
        return Err(AuditError("migration response benchmark has no trials".to_string()));
    }

    let evidence_class = trials[0].evidence_class;
    let mut ids = HashSet::new();
    let mut ordered_trials: Vec<&MigrationTrial> = trials.iter().collect();
    ordered_trials.sort_by(|left, right| left.trial_id.cmp(&right.trial_id));
    let mut trial_results = Vec::with_capacity(ordered_trials.len());
    let mut strata_values: BTreeMap<
        (MigrationDataAccessMode, PrelaunchState),
        Vec<MigrationTrialResult>,
    > = BTreeMap::new();

    for trial in ordered_trials {
        if trial.evidence_class != evidence_class {
            return Err(AuditError(
                "measured and modelled migration trials cannot be mixed in one report".to_string(),
            ));
        }
        if !ids.insert(trial.trial_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate migration trial ID {:?}",
                trial.trial_id
            )));
        }
        let result = evaluate_trial(&contract, trial)?;
        strata_values
            .entry((result.data_access_mode, result.prelaunch_state))
            .or_default()
            .push(result.clone());
        trial_results.push(result);
    }

    let mut strata = Vec::with_capacity(strata_values.len());
    for ((data_access_mode, prelaunch_state), results) in strata_values {
        let trial_count = results.len();
        let response_success_count = results.iter()
            .filter(|result| result.response_deadline_met)
            .count();
        let observed_latencies: Vec<u64> = results.iter()
            .filter_map(|result| result.source_response_latency_ns)
            .collect();
        let observed_destination_latencies: Vec<u64> = results.iter()
            .filter_map(|result| result.destination_power_stabilization_latency_ns)
            .collect();
        let useful_work_preserved_count = results.iter()
            .filter(|result| result.useful_work_preserved)
            .count();
        let service_eligible_count = results.iter()
            .filter(|result| result.service_eligible)
            .count();
        let total_data_bytes = results.iter().try_fold(0_u128, |total, result| {
            total.checked_add(result.data_bytes)
                .ok_or_else(|| AuditError("migration data-byte total overflowed".to_string()))
        })?;

        strata.push(MigrationStratumSummary {
            data_access_mode,
            prelaunch_state,
            trial_count,
            source_response_success_count,
            source_response_success_fraction: response_success_count as f64 / trial_count as f64,
            observed_response_latency_count: observed_latencies.len(),
            p50_source_response_latency_ns: nearest_rank(&observed_latencies, 50, 100),
            p95_source_response_latency_ns: nearest_rank(&observed_latencies, 95, 100),
            p99_source_response_latency_ns: nearest_rank(&observed_latencies, 99, 100),
            observed_destination_stabilization_count: observed_destination_latencies.len(),
            p50_destination_stabilization_latency_ns: nearest_rank(&observed_destination_latencies, 50, 100),
            p95_destination_stabilization_latency_ns: nearest_rank(&observed_destination_latencies, 95, 100),
            p99_destination_stabilization_latency_ns: nearest_rank(&observed_destination_latencies, 99, 100),
            useful_work_preserved_count,
            service_eligible_count,
            total_data_bytes,
        });
    }

    Ok(MigrationResponseReport {
        contract,
        evidence_class,
        trial_count: trial_results.len(),
        strata,
        trial_results,
        claim_ceiling: match evidence_class {
            MigrationEvidenceClass::Measured =>
                "MEASURED_MIGRATION_RESPONSE_TRIALS_NOT_GRID_SERVICE_QUALIFICATION",
            MigrationEvidenceClass::Modelled =>
                "MODELLED_MIGRATION_RESPONSE_TRIALS_NOT_MEASURED_OR_GRID_SERVICE_QUALIFICATION",
        },
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_migration_response_trials, MigrationDataAccessMode, MigrationEvidenceClass,
        MigrationResponseContract, MigrationTrial, PrelaunchState,
    };

    fn contract() -> MigrationResponseContract {
        MigrationResponseContract {
            contract_id: "migration-contract-v1".to_string(),
            workload_contract_id: "inference-work-v1".to_string(),
            data_artifact_id: "model-artifact-v4".to_string(),
            data_version: "sha256:model-v4".to_string(),
            work_unit: "accepted-request".to_string(),
            required_useful_work_units: 10,
            source_site_id: "site-a".to_string(),
            destination_site_id: "site-b".to_string(),
            source_region_id: "region-a".to_string(),
            destination_region_id: "region-b".to_string(),
            source_pcc_boundary_id: "pcc-a".to_string(),
            destination_pcc_boundary_id: "pcc-b".to_string(),
            source_hardware_configuration_fingerprint: "sha256:source-config".to_string(),
            destination_hardware_configuration_fingerprint: "sha256:destination-config".to_string(),
            use_authorization_reference: "rights:model-v4".to_string(),
            residency_policy_reference: "residency:contract-9".to_string(),
            freshness_policy_id: "freshness:current-version".to_string(),
            output_quality_contract_id: "quality:inference-v2".to_string(),
            service_slo_contract_id: "slo:interactive-p99-v1".to_string(),
            power_stability_criterion_id: "stable:window-5s-tolerance-2pct".to_string(),
            common_clock_id: "ptp-region-clock".to_string(),
            common_time_origin_id: "epoch-utc-ns".to_string(),
            clock_alignment_method_id: "ptp-cross-site-v1".to_string(),
            maximum_clock_alignment_uncertainty_ns: 1_000,
            source_relief_response_limit_ns: 5_000_000_000,
        }
    }

    fn trial(id: &str, access: MigrationDataAccessMode, command: u64) -> MigrationTrial {
        MigrationTrial {
            trial_id: id.to_string(),
            contract_id: "migration-contract-v1".to_string(),
            workload_contract_id: "inference-work-v1".to_string(),
            data_artifact_id: "model-artifact-v4".to_string(),
            data_version: "sha256:model-v4".to_string(),
            work_unit: "accepted-request".to_string(),
            data_bytes: 10_000,
            data_access_mode: access,
            evidence_class: MigrationEvidenceClass::Measured,
            source_site_id: "site-a".to_string(),
            destination_site_id: "site-b".to_string(),
            source_region_id: "region-a".to_string(),
            destination_region_id: "region-b".to_string(),
            source_pcc_boundary_id: "pcc-a".to_string(),
            destination_pcc_boundary_id: "pcc-b".to_string(),
            source_hardware_configuration_fingerprint: "sha256:source-config".to_string(),
            destination_hardware_configuration_fingerprint: "sha256:destination-config".to_string(),
            common_clock_id: "ptp-region-clock".to_string(),
            common_time_origin_id: "epoch-utc-ns".to_string(),
            clock_alignment_method_id: "ptp-cross-site-v1".to_string(),
            clock_alignment_evidence_id: format!("clock-align-{id}"),
            source_clock_alignment_uncertainty_ns: 40,
            destination_clock_alignment_uncertainty_ns: 60,
            command_issued_ns: command,
            data_ready_ns: Some(command - 10),
            data_ready_evidence_id: Some(format!("data-ready-{id}")),
            runtime_ready_ns: Some(command - 5),
            runtime_ready_evidence_id: Some(format!("runtime-ready-{id}")),
            destination_capacity_reserved_ns: Some(command - 20),
            destination_capacity_evidence_id: Some(format!("capacity-{id}")),
            destination_work_started_ns: Some(command + 100),
            first_useful_work_accepted_ns: Some(command + 200),
            accepted_useful_work_units: 10,
            accepted_work_evidence_id: format!("accepted-work-{id}"),
            output_quality_accepted: true,
            output_quality_evidence_id: format!("quality-{id}"),
            service_slo_met: true,
            service_slo_evidence_id: format!("slo-{id}"),
            authority_authorized: true,
            authority_evidence_id: format!("authority-{id}"),
            residency_permitted: true,
            residency_evidence_id: format!("residency-{id}"),
            data_freshness_verified: true,
            data_freshness_evidence_id: format!("freshness-{id}"),
            data_path_evidence_id: format!("path-{id}"),
            source_power_reduction_stable_ns: Some(command + 1_000_000_000),
            source_power_trace_evidence_id: format!("source-power-{id}"),
            source_pcc_measurement_reference: format!("pcc-source-{id}"),
            destination_power_increase_stable_ns: Some(command + 1_500_000_000),
            destination_power_trace_evidence_id: format!("destination-power-{id}"),
            destination_pcc_measurement_reference: format!("pcc-destination-{id}"),
            trial_outcome_evidence_id: format!("rollback-{id}"),
        }
    }

    #[test]
    fn prelaunched_read_only_trial_meets_response_and_work_contract() {
        let report = evaluate_migration_response_trials(
            contract(),
            &[trial("warm", MigrationDataAccessMode::RemoteReadOnly, 10_000)],
        ).unwrap();
        let result = &report.trial_results[0];
        assert_eq!(result.prelaunch_state, PrelaunchState::DataAndRuntime);
        assert_eq!(result.source_response_latency_ns, Some(1_000_000_000));
        assert!(result.response_deadline_met);
        assert!(result.useful_work_preserved);
        assert!(result.service_eligible);
    }

    #[test]
    fn cold_on_demand_copy_can_miss_strict_response_even_when_work_completes() {
        let mut cold = trial("cold", MigrationDataAccessMode::OnDemandCopy, 10_000);
        cold.data_ready_ns = Some(10_000 + 10_000_000_000);
        cold.data_ready_evidence_id = Some("copy-finished".to_string());
        cold.runtime_ready_ns = Some(10_000 + 2_000_000_000);
        cold.runtime_ready_evidence_id = Some("runtime-ready-late".to_string());
        cold.destination_capacity_reserved_ns = Some(10_000 + 1_000_000_000);
        cold.destination_capacity_evidence_id = Some("capacity-late".to_string());
        cold.destination_work_started_ns = Some(10_000 + 10_000_000_100);
        cold.first_useful_work_accepted_ns = Some(10_000 + 10_000_000_200);
        cold.source_power_reduction_stable_ns = Some(10_000 + 7_000_000_000);
        let report = evaluate_migration_response_trials(contract(), &[cold]).unwrap();
        let result = &report.trial_results[0];
        assert_eq!(result.prelaunch_state, PrelaunchState::Neither);
        assert!(result.useful_work_preserved);
        assert!(!result.response_deadline_met);
        assert!(!result.service_eligible);
    }

    #[test]
    fn source_relief_and_destination_power_change_are_reported_separately() {
        let report = evaluate_migration_response_trials(
            contract(),
            &[trial("move", MigrationDataAccessMode::ReplicatedReadOnly, 10_000)],
        ).unwrap();
        let result = &report.trial_results[0];
        assert_eq!(result.source_response_latency_ns, Some(1_000_000_000));
        assert_eq!(result.destination_power_stabilization_latency_ns, Some(1_500_000_000));
        assert_eq!(report.strata[0].p95_destination_stabilization_latency_ns, Some(1_500_000_000));
    }

    #[test]
    fn trial_without_observed_source_stability_remains_in_denominator() {
        let mut missing_stability = trial("not-stable", MigrationDataAccessMode::RemoteReadOnly, 10_000);
        missing_stability.source_power_reduction_stable_ns = None;
        let report = evaluate_migration_response_trials(contract(), &[missing_stability]).unwrap();
        assert_eq!(report.strata[0].trial_count, 1);
        assert_eq!(report.strata[0].source_response_success_count, 0);
        assert_eq!(report.strata[0].source_response_success_fraction, 0.0);
        assert_eq!(report.strata[0].observed_response_latency_count, 0);
        assert_eq!(report.strata[0].p95_source_response_latency_ns, None);
    }

    #[test]
    fn failed_authority_or_residency_never_receives_service_credit() {
        let mut unauthorized = trial("unauthorized", MigrationDataAccessMode::ReplicatedReadOnly, 10_000);
        unauthorized.authority_authorized = false;
        let mut wrong_jurisdiction = trial("wrong-jurisdiction", MigrationDataAccessMode::RemoteReadOnly, 20_000);
        wrong_jurisdiction.residency_permitted = false;
        let report = evaluate_migration_response_trials(contract(), &[unauthorized, wrong_jurisdiction]).unwrap();
        assert!(report.trial_results.iter().all(|result| !result.service_eligible));
        assert!(report.trial_results.iter().all(|result| result.response_deadline_met));
    }

    #[test]
    fn accepted_work_cannot_precede_data_runtime_or_capacity_readiness() {
        let mut invalid = trial("early", MigrationDataAccessMode::RemoteReadOnly, 10_000);
        invalid.data_ready_ns = Some(20_000);
        invalid.runtime_ready_ns = Some(10_020);
        invalid.destination_capacity_reserved_ns = Some(10_030);
        invalid.destination_work_started_ns = Some(10_100);
        assert!(evaluate_migration_response_trials(contract(), &[invalid]).is_err());
    }

    #[test]
    fn exact_source_destination_boundary_and_clock_contract_is_required() {
        let mut mismatched = trial("boundary", MigrationDataAccessMode::RemoteReadOnly, 10_000);
        mismatched.destination_pcc_boundary_id = "wrong-pcc".to_string();
        assert!(evaluate_migration_response_trials(contract(), &[mismatched]).is_err());

        let mut wrong_clock = trial("clock", MigrationDataAccessMode::RemoteReadOnly, 10_000);
        wrong_clock.common_clock_id = "host-clock".to_string();
        assert!(evaluate_migration_response_trials(contract(), &[wrong_clock]).is_err());
    }

    #[test]
    fn pairwise_clock_uncertainty_is_the_sum_of_source_and_destination_bounds() {
        let mut over_limit = trial("clock-over", MigrationDataAccessMode::RemoteReadOnly, 10_000);
        over_limit.source_clock_alignment_uncertainty_ns = 60;
        over_limit.destination_clock_alignment_uncertainty_ns = 60;
        assert!(evaluate_migration_response_trials(contract(), &[over_limit]).is_err());
    }

    #[test]
    fn replicated_read_only_requires_freshness_evidence_reference() {
        let mut no_freshness = trial("stale", MigrationDataAccessMode::ReplicatedReadOnly, 10_000);
        no_freshness.data_freshness_evidence_id.clear();
        assert!(evaluate_migration_response_trials(contract(), &[no_freshness]).is_err());
    }

    #[test]
    fn missing_power_trace_reference_fails_closed_even_when_no_stable_event_is_seen() {
        let mut no_power_trace = trial("no-trace", MigrationDataAccessMode::RemoteReadOnly, 10_000);
        no_power_trace.source_power_trace_evidence_id.clear();
        no_power_trace.source_power_reduction_stable_ns = None;
        assert!(evaluate_migration_response_trials(contract(), &[no_power_trace]).is_err());
    }

    #[test]
    fn mixed_measured_and_modelled_trials_cannot_share_a_report() {
        let first = trial("measured", MigrationDataAccessMode::RemoteReadOnly, 10_000);
        let mut second = trial("modelled", MigrationDataAccessMode::RemoteReadOnly, 20_000);
        second.evidence_class = MigrationEvidenceClass::Modelled;
        assert!(evaluate_migration_response_trials(contract(), &[first, second]).is_err());
    }

    #[test]
    fn duplicate_trial_ids_are_rejected() {
        let first = trial("duplicate", MigrationDataAccessMode::RemoteReadOnly, 10_000);
        let second = first.clone();
        assert!(evaluate_migration_response_trials(contract(), &[first, second]).is_err());
    }

    #[test]
    fn reports_are_deterministic_and_stratified_by_access_and_prelaunch_state() {
        let mut cold = trial("cold", MigrationDataAccessMode::OnDemandCopy, 20_000);
        cold.data_ready_ns = Some(30_000);
        cold.data_ready_evidence_id = Some("data-late".to_string());
        cold.runtime_ready_ns = Some(25_000);
        cold.runtime_ready_evidence_id = Some("runtime-late".to_string());
        cold.destination_capacity_reserved_ns = Some(24_000);
        cold.destination_capacity_evidence_id = Some("capacity-late".to_string());
        cold.destination_work_started_ns = Some(30_100);
        cold.first_useful_work_accepted_ns = Some(30_200);
        let warm = trial("warm", MigrationDataAccessMode::RemoteReadOnly, 10_000);
        let first = evaluate_migration_response_trials(contract(), &[cold.clone(), warm.clone()]).unwrap();
        let second = evaluate_migration_response_trials(contract(), &[warm, cold]).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.trial_results[0].trial_id, "cold");
        assert_eq!(first.trial_results[1].trial_id, "warm");
        assert_eq!(first.strata.len(), 2);
    }

    #[test]
    fn negative_outcomes_still_require_quality_and_slo_evidence() {
        let mut failed = trial("failed", MigrationDataAccessMode::RemoteReadOnly, 10_000);
        failed.output_quality_accepted = false;
        failed.output_quality_evidence_id.clear();
        assert!(evaluate_migration_response_trials(contract(), &[failed]).is_err());
    }

    #[test]
    fn zero_accepted_work_cannot_claim_an_acceptance_timestamp() {
        let mut inconsistent = trial("zero-work", MigrationDataAccessMode::RemoteReadOnly, 10_000);
        inconsistent.accepted_useful_work_units = 0;
        inconsistent.first_useful_work_accepted_ns = Some(10_200);
        assert!(evaluate_migration_response_trials(contract(), &[inconsistent]).is_err());
    }

    #[test]
    fn prelaunch_strata_are_derived_from_event_timestamps_not_claimed_flags() {
        let mut data_only = trial("data-only", MigrationDataAccessMode::ReplicatedReadOnly, 10_000);
        data_only.runtime_ready_ns = Some(10_001);
        data_only.runtime_ready_evidence_id = Some("runtime-late".to_string());
        data_only.destination_work_started_ns = Some(10_100);
        data_only.first_useful_work_accepted_ns = Some(10_200);
        let report = evaluate_migration_response_trials(contract(), &[data_only]).unwrap();
        assert_eq!(report.trial_results[0].prelaunch_state, PrelaunchState::DataOnly);
        assert!(report.trial_results[0].data_ready_before_command);
        assert!(!report.trial_results[0].runtime_ready_before_command);
    }
}
