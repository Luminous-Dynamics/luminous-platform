//! Tiered sandbox profile and logical/resource/physical telemetry validation.
//!
//! This module validates the attested sandbox boundary and telemetry contract.
//! It does not provision isolation or authenticate the Supervisor. The existing
//! typed evidence verifier must authenticate Supervisor receipts before use.

use crate::runtime::validate_ledger_binding;
use crate::sealed::{
    validate_freeze_receipt, validate_manifest, validate_scenario, EvaluationArm, FreezeReceipt,
    RunLedgerSummary, RunManifest, SandboxTier, ScenarioReceipt, SupervisorKeyClass, TrialResult,
};
use crate::Digest;
use std::collections::{BTreeMap, BTreeSet};

pub const SANDBOX_ATTESTATION_SCHEMA: &str = "continuous-opsbench-sandbox-attestation-v0.1";
pub const MAX_ENERGY_UNCERTAINTY_PPM: u32 = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IsolationMechanism {
    ResearchUnisolated,
    SeccompProcess,
    GVisorProcess,
    ContainerRuntime,
    GVisorContainer,
    Firecracker,
    CloudHypervisor,
    Crosvm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelemetryCoverage {
    Complete,
    NotAvailable,
    Partial,
    Unknown,
    Tampered,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnergyTelemetry {
    pub cpu_package_microjoules: Option<u128>,
    pub gpu_microjoules: Option<u128>,
    pub sample_start_ms: Option<u64>,
    pub sample_end_ms: Option<u64>,
    pub uncertainty_ppm: Option<u32>,
    pub calibration_profile_digest: Option<Digest>,
    pub hardware_source_id: Option<String>,
    pub device_identity_digest: Option<Digest>,
    pub unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxTrialAttestation {
    pub schema: String,
    pub run_id: String,
    pub trial_id: String,
    pub arm: EvaluationArm,
    pub arm_identity_digest: Digest,
    pub scenario_commitment: Digest,
    pub manifest_commitment: Digest,
    pub execution_environment_digest: Digest,
    pub resource_budget_digest: Digest,
    pub sandbox_tier: SandboxTier,
    pub sandbox_profile_digest: Digest,
    pub supervisor_id: String,
    pub supervisor_source_id: String,
    pub supervisor_control_domain: String,
    pub supervisor_adapter_digest: Digest,
    pub supervisor_key_class: SupervisorKeyClass,
    pub receipt_sequence: u64,
    pub runtime_instance_digest: Digest,
    pub writable_state_namespace_digest: Digest,
    pub issued_at_ms: u64,
    pub measurement_start_ms: u64,
    pub measurement_end_ms: u64,
    pub clock_uncertainty_ms: u64,

    // Environment / boundary identity
    pub host_os: String,
    pub host_architecture: String,
    pub host_kernel_digest: Digest,
    pub hardware_identity_digest: Digest,
    pub isolation_mechanism: IsolationMechanism,
    pub runtime_version: String,
    pub seccomp_profile_digest: Option<Digest>,
    pub namespace_profile_digest: Option<Digest>,
    pub cgroup_profile_digest: Option<Digest>,
    pub container_image_digest: Option<Digest>,
    pub mount_policy_digest: Option<Digest>,
    pub network_namespace_digest: Option<Digest>,
    pub tool_subsandbox_digest: Option<Digest>,
    pub hypervisor_digest: Option<Digest>,
    pub guest_kernel_digest: Option<Digest>,
    pub dedicated_cpu_set_digest: Option<Digest>,
    pub accelerator_assignment_digest: Option<Digest>,

    // Logical telemetry
    pub logical_coverage: TelemetryCoverage,
    pub session_identity_digest: Digest,
    pub harness_identity_digest: Digest,
    pub transcript_commitment: Digest,
    pub input_seed_commitment: Digest,
    pub tool_call_count: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,

    // Resource telemetry
    pub resource_coverage: TelemetryCoverage,
    pub wallclock_ms: u64,
    pub cpu_time_ns: u128,
    pub peak_memory_bytes: u64,
    pub disk_read_bytes: u128,
    pub disk_write_bytes: u128,
    pub network_rx_bytes: u128,
    pub network_tx_bytes: u128,
    pub accelerator_utilization_ppm: Option<u32>,
    pub accelerator_memory_bytes: Option<u64>,
    pub retry_count: u32,

    // Physical telemetry
    pub physical_coverage: TelemetryCoverage,
    pub energy: EnergyTelemetry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxTrialResult {
    pub trial_id: String,
    pub arm: EvaluationArm,
    pub isolation_mechanism: IsolationMechanism,
    pub runtime_version: String,
    pub hardware_identity_digest: Digest,
    pub wallclock_ms: u64,
    pub cpu_time_ns: u128,
    pub peak_memory_bytes: u64,
    pub disk_read_bytes: u128,
    pub disk_write_bytes: u128,
    pub network_rx_bytes: u128,
    pub network_tx_bytes: u128,
    pub tool_call_count: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub retry_count: u32,
    pub cpu_package_microjoules: Option<u128>,
    pub gpu_microjoules: Option<u128>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxCertificationSummary {
    pub run_id: String,
    pub tier: SandboxTier,
    pub profile_digest: Digest,
    pub supervisor_id: String,
    pub supervisor_source_id: String,
    pub supervisor_control_domain: String,
    pub supervisor_key_class: SupervisorKeyClass,
    pub planned_arm_trials: usize,
    pub complete_attestations: usize,
    pub common_hardware_identity_digest: Digest,
    pub all_physical_energy_measured: bool,
    pub total_cpu_package_microjoules: Option<u128>,
    pub total_gpu_microjoules: Option<u128>,
    pub trials: Vec<SandboxTrialResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SandboxAttestationError {
    UnsupportedSchema(String),
    ManifestOrFreezeInvalid,
    LedgerManifestMismatch,
    ScenarioMissing(String),
    ScenarioDuplicate(String),
    LedgerTrialMissing(String, EvaluationArm),
    LedgerTrialDuplicate(String, EvaluationArm),
    MissingAttestation(String, EvaluationArm),
    DuplicateAttestation(String, EvaluationArm),
    UnplannedAttestation(String, EvaluationArm),
    AttestationBindingMismatch(String, EvaluationArm),
    SupervisorIdentityMismatch,
    SupervisorControlDomainCollision,
    SupervisorKeyClassInsufficient(SandboxTier),
    TierMechanismMismatch(String, SandboxTier),
    TierEvidenceMissing(String, &'static str),
    TierEvidenceUnexpected(String, &'static str),
    InvalidIdentity(String, &'static str),
    InvalidMeasurementInterval(String, EvaluationArm),
    MeasurementDoesNotCoverTrial(String, EvaluationArm),
    ClockUncertaintyExceeded(String, EvaluationArm),
    IncompleteLogicalTelemetry(String, EvaluationArm),
    IncompleteResourceTelemetry(String, EvaluationArm),
    InvalidResourceMetric(String, EvaluationArm),
    PhysicalTelemetryUnavailable(String, EvaluationArm),
    InvalidEnergyTelemetry(String, EvaluationArm),
    HardwareProfileMismatch(String, EvaluationArm),
    DuplicateRuntimeInstance,
    DuplicateStateNamespace,
    DuplicateSupervisorSequence,
    SupervisorSequenceNotContiguous,
    ArithmeticOverflow,
}

pub fn validate_sandbox_attestations(
    manifest: &RunManifest,
    expected_manifest_commitment: &Digest,
    freeze: &FreezeReceipt,
    ledger: &RunLedgerSummary,
    scenarios: &[ScenarioReceipt],
    attestations: &[SandboxTrialAttestation],
) -> Result<SandboxCertificationSummary, SandboxAttestationError> {
    validate_manifest(manifest).map_err(|_| SandboxAttestationError::ManifestOrFreezeInvalid)?;
    validate_freeze_receipt(manifest, expected_manifest_commitment, freeze)
        .map_err(|_| SandboxAttestationError::ManifestOrFreezeInvalid)?;
    validate_ledger_binding(manifest, ledger)
        .map_err(|_| SandboxAttestationError::LedgerManifestMismatch)?;
    if ledger.manifest_commitment != *expected_manifest_commitment {
        return Err(SandboxAttestationError::LedgerManifestMismatch);
    }

    let mut scenario_index: BTreeMap<&str, &ScenarioReceipt> = BTreeMap::new();
    for scenario in scenarios {
        let trial = manifest.trials.iter()
            .find(|trial| trial.trial_id == scenario.trial_id)
            .ok_or_else(|| SandboxAttestationError::ScenarioMissing(scenario.trial_id.clone()))?;
        validate_scenario(manifest, expected_manifest_commitment, trial, scenario)
            .map_err(|_| SandboxAttestationError::AttestationBindingMismatch(
                scenario.trial_id.clone(), EvaluationArm::Candidate
            ))?;
        if scenario_index.insert(scenario.trial_id.as_str(), scenario).is_some() {
            return Err(SandboxAttestationError::ScenarioDuplicate(scenario.trial_id.clone()));
        }
    }

    let mut ledger_index: BTreeMap<(&str, EvaluationArm), &TrialResult> = BTreeMap::new();
    for result in &ledger.trials {
        if ledger_index.insert((result.trial_id.as_str(), result.arm), result).is_some() {
            return Err(SandboxAttestationError::LedgerTrialDuplicate(
                result.trial_id.clone(), result.arm
            ));
        }
    }

    let mut receipt_index: BTreeMap<(&str, EvaluationArm), &SandboxTrialAttestation> =
        BTreeMap::new();
    let mut runtime_instances = BTreeSet::new();
    let mut state_namespaces = BTreeSet::new();
    let mut supervisor_sequences = BTreeSet::new();
    let mut sequence_order = Vec::with_capacity(attestations.len());
    let mut common_hardware: Option<Digest> = None;
    let mut common_kernel: Option<Digest> = None;
    let mut common_architecture: Option<&str> = None;
    let mut total_cpu_energy = Some(0u128);
    let mut total_gpu_energy = Some(0u128);
    let mut trials = Vec::with_capacity(attestations.len());

    for receipt in attestations {
        if receipt.schema != SANDBOX_ATTESTATION_SCHEMA {
            return Err(SandboxAttestationError::UnsupportedSchema(receipt.trial_id.clone()));
        }
        if !manifest.trials.iter().any(|trial| trial.trial_id == receipt.trial_id) {
            return Err(SandboxAttestationError::UnplannedAttestation(
                receipt.trial_id.clone(), receipt.arm
            ));
        }
        let ledger_result = ledger_index.get(&(receipt.trial_id.as_str(), receipt.arm))
            .copied()
            .ok_or_else(|| SandboxAttestationError::LedgerTrialMissing(
                receipt.trial_id.clone(), receipt.arm
            ))?;
        let scenario = scenario_index.get(receipt.trial_id.as_str())
            .copied()
            .ok_or_else(|| SandboxAttestationError::ScenarioMissing(receipt.trial_id.clone()))?;

        if receipt_index.insert((receipt.trial_id.as_str(), receipt.arm), receipt).is_some() {
            return Err(SandboxAttestationError::DuplicateAttestation(
                receipt.trial_id.clone(), receipt.arm
            ));
        }
        validate_attestation_binding(
            manifest, expected_manifest_commitment, scenario, receipt
        )?;
        validate_supervisor_identity(manifest, receipt)?;
        validate_tier_evidence(manifest, receipt)?;
        validate_telemetry(receipt)?;
        validate_sandbox_coverage_window(receipt, ledger_result)?;

        if receipt.receipt_sequence == 0 || !supervisor_sequences.insert(receipt.receipt_sequence) {
            return Err(SandboxAttestationError::DuplicateSupervisorSequence);
        }
        sequence_order.push((receipt.issued_at_ms, receipt.receipt_sequence));

        if is_zero(&receipt.runtime_instance_digest)
            || !runtime_instances.insert(receipt.runtime_instance_digest.clone())
        {
            return Err(SandboxAttestationError::DuplicateRuntimeInstance);
        }
        if is_zero(&receipt.writable_state_namespace_digest)
            || !state_namespaces.insert(receipt.writable_state_namespace_digest.clone())
        {
            return Err(SandboxAttestationError::DuplicateStateNamespace);
        }

        if common_hardware.as_ref().map(|v| v != &receipt.hardware_identity_digest).unwrap_or(false)
            || common_kernel.as_ref().map(|v| v != &receipt.host_kernel_digest).unwrap_or(false)
            || common_architecture.map(|v| v != receipt.host_architecture.as_str()).unwrap_or(false)
        {
            return Err(SandboxAttestationError::HardwareProfileMismatch(
                receipt.trial_id.clone(), receipt.arm
            ));
        }
        common_hardware.get_or_insert_with(|| receipt.hardware_identity_digest.clone());
        common_kernel.get_or_insert_with(|| receipt.host_kernel_digest.clone());
        common_architecture.get_or_insert(receipt.host_architecture.as_str());

        if let Some(energy) = receipt.energy.cpu_package_microjoules {
            if let Some(sum) = total_cpu_energy {
                total_cpu_energy = Some(sum.checked_add(energy)
                    .ok_or(SandboxAttestationError::ArithmeticOverflow)?);
            }
        } else {
            total_cpu_energy = None;
        }
        if let Some(energy) = receipt.energy.gpu_microjoules {
            if let Some(sum) = total_gpu_energy {
                total_gpu_energy = Some(sum.checked_add(energy)
                    .ok_or(SandboxAttestationError::ArithmeticOverflow)?);
            }
        } else {
            total_gpu_energy = None;
        }

        trials.push(SandboxTrialResult {
            trial_id: receipt.trial_id.clone(),
            arm: receipt.arm,
            isolation_mechanism: receipt.isolation_mechanism,
            runtime_version: receipt.runtime_version.clone(),
            hardware_identity_digest: receipt.hardware_identity_digest.clone(),
            wallclock_ms: receipt.wallclock_ms,
            cpu_time_ns: receipt.cpu_time_ns,
            peak_memory_bytes: receipt.peak_memory_bytes,
            disk_read_bytes: receipt.disk_read_bytes,
            disk_write_bytes: receipt.disk_write_bytes,
            network_rx_bytes: receipt.network_rx_bytes,
            network_tx_bytes: receipt.network_tx_bytes,
            tool_call_count: receipt.tool_call_count,
            input_tokens: receipt.input_tokens,
            output_tokens: receipt.output_tokens,
            retry_count: receipt.retry_count,
            cpu_package_microjoules: receipt.energy.cpu_package_microjoules,
            gpu_microjoules: receipt.energy.gpu_microjoules,
        });

        // A skipped arm still needs a complete logical/resource/physical attestation.
        let _ = ledger_result;
    }

    for planned in &manifest.trials {
        if !scenario_index.contains_key(planned.trial_id.as_str()) {
            return Err(SandboxAttestationError::ScenarioMissing(planned.trial_id.clone()));
        }
        for arm in [EvaluationArm::Candidate, EvaluationArm::Baseline] {
            if !ledger_index.contains_key(&(planned.trial_id.as_str(), arm)) {
                return Err(SandboxAttestationError::LedgerTrialMissing(
                    planned.trial_id.clone(), arm
                ));
            }
            if !receipt_index.contains_key(&(planned.trial_id.as_str(), arm)) {
                return Err(SandboxAttestationError::MissingAttestation(
                    planned.trial_id.clone(), arm
                ));
            }
        }
    }

    if supervisor_sequences.len() != attestations.len() {
        return Err(SandboxAttestationError::DuplicateSupervisorSequence);
    }
    sequence_order.sort_unstable_by_key(|(issued_at, _)| *issued_at);
    if sequence_order.windows(2).any(|pair| pair[0].0 >= pair[1].0)
        || sequence_order.iter().enumerate().any(|(index, (_, sequence))| {
            *sequence != (index as u64) + 1
        })
    {
        return Err(SandboxAttestationError::SupervisorSequenceNotContiguous);
    }

    let expected_arm_trials = manifest.trials.len().saturating_mul(2);
    if attestations.len() != expected_arm_trials {
        return Err(SandboxAttestationError::LedgerManifestMismatch);
    }

    let all_physical_energy_measured = attestations.iter().all(|r| {
        r.physical_coverage == TelemetryCoverage::Complete
            && r.energy.cpu_package_microjoules.is_some()
    });
    Ok(SandboxCertificationSummary {
        run_id: manifest.run_id.clone(),
        tier: manifest.sandbox_tier,
        profile_digest: manifest.sandbox_profile_digest.clone(),
        supervisor_id: manifest.supervisor_id.clone(),
        supervisor_source_id: manifest.supervisor_source_id.clone(),
        supervisor_control_domain: manifest.supervisor_control_domain.clone(),
        supervisor_key_class: manifest.supervisor_key_class,
        planned_arm_trials: expected_arm_trials,
        complete_attestations: attestations.len(),
        common_hardware_identity_digest: common_hardware.unwrap_or_else(|| Digest::from_byte(0)),
        all_physical_energy_measured,
        total_cpu_package_microjoules: total_cpu_energy,
        total_gpu_microjoules: total_gpu_energy,
        trials,
    })
}


fn validate_attestation_binding(
    manifest: &RunManifest,
    expected_manifest_commitment: &Digest,
    scenario: &ScenarioReceipt,
    receipt: &SandboxTrialAttestation,
) -> Result<(), SandboxAttestationError> {
    let identity = match receipt.arm {
        EvaluationArm::Candidate => &manifest.agent_identity_digest,
        EvaluationArm::Baseline => &manifest.baseline_identity_digest,
    };
    if receipt.run_id != manifest.run_id
        || receipt.trial_id != scenario.trial_id
        || receipt.arm_identity_digest != *identity
        || receipt.scenario_commitment != scenario.case_payload_commitment
        || receipt.manifest_commitment != *expected_manifest_commitment
        || scenario.manifest_commitment != *expected_manifest_commitment
        || receipt.execution_environment_digest != manifest.execution_environment_digest
        || receipt.resource_budget_digest != manifest.resource_budget_digest
        || receipt.sandbox_tier != manifest.sandbox_tier
        || receipt.sandbox_profile_digest != manifest.sandbox_profile_digest
        || receipt.supervisor_id != manifest.supervisor_id
        || receipt.supervisor_source_id != manifest.supervisor_source_id
        || receipt.supervisor_control_domain != manifest.supervisor_control_domain
        || receipt.supervisor_adapter_digest != manifest.supervisor_adapter_digest
        || receipt.supervisor_key_class != manifest.supervisor_key_class
    {
        return Err(SandboxAttestationError::AttestationBindingMismatch(
            receipt.trial_id.clone(), receipt.arm
        ));
    }
    Ok(())
}

fn validate_supervisor_identity(
    manifest: &RunManifest,
    receipt: &SandboxTrialAttestation,
) -> Result<(), SandboxAttestationError> {
    if receipt.supervisor_id.trim().is_empty()
        || receipt.supervisor_source_id.trim().is_empty()
        || is_zero(&receipt.supervisor_adapter_digest)
    {
        return Err(SandboxAttestationError::SupervisorIdentityMismatch);
    }
    if receipt.supervisor_control_domain != manifest.supervisor_control_domain
        || receipt.supervisor_control_domain == manifest.agent.control_domain
        || receipt.supervisor_control_domain == manifest.baseline.control_domain
        || receipt.supervisor_control_domain == manifest.evaluator_control_domain
        || receipt.supervisor_control_domain == manifest.freeze_authority_control_domain
        || receipt.supervisor_control_domain == manifest.evidence_control_domain
        || receipt.supervisor_control_domain == manifest.runtime_observer_control_domain
    {
        return Err(SandboxAttestationError::SupervisorControlDomainCollision);
    }
    if manifest.sandbox_tier != SandboxTier::Research
        && receipt.supervisor_key_class != SupervisorKeyClass::HardwareProtected
    {
        return Err(SandboxAttestationError::SupervisorKeyClassInsufficient(
            manifest.sandbox_tier
        ));
    }
    Ok(())
}

fn validate_tier_evidence(
    manifest: &RunManifest,
    receipt: &SandboxTrialAttestation,
) -> Result<(), SandboxAttestationError> {
    let required = |value: &Option<Digest>| value.as_ref().map(|v| !is_zero(v)).unwrap_or(false);
    let wrong_mechanism = || SandboxAttestationError::TierMechanismMismatch(
        receipt.trial_id.clone(), manifest.sandbox_tier
    );

    if receipt.host_os.trim().is_empty() || receipt.host_architecture.trim().is_empty()
        || receipt.runtime_version.trim().is_empty()
    {
        return Err(SandboxAttestationError::InvalidIdentity(
            receipt.trial_id.clone(), "host_os_arch_runtime"
        ));
    }
    if is_zero(&receipt.hardware_identity_digest) {
        return Err(SandboxAttestationError::InvalidIdentity(
            receipt.trial_id.clone(), "hardware_identity"
        ));
    }
    if manifest.sandbox_tier != SandboxTier::Research && is_zero(&receipt.host_kernel_digest) {
        return Err(SandboxAttestationError::InvalidIdentity(
            receipt.trial_id.clone(), "host_kernel"
        ));
    }

    match manifest.sandbox_tier {
        SandboxTier::Research => {
            if receipt.isolation_mechanism != IsolationMechanism::ResearchUnisolated {
                return Err(wrong_mechanism());
            }
        }
        SandboxTier::Process => {
            if !matches!(
                receipt.isolation_mechanism,
                IsolationMechanism::SeccompProcess | IsolationMechanism::GVisorProcess
            ) {
                return Err(wrong_mechanism());
            }
            if !required(&receipt.seccomp_profile_digest) {
                return Err(SandboxAttestationError::TierEvidenceMissing(
                    receipt.trial_id.clone(), "seccomp_profile"
                ));
            }
            if !required(&receipt.namespace_profile_digest) {
                return Err(SandboxAttestationError::TierEvidenceMissing(
                    receipt.trial_id.clone(), "namespace_profile"
                ));
            }
            if !required(&receipt.cgroup_profile_digest) {
                return Err(SandboxAttestationError::TierEvidenceMissing(
                    receipt.trial_id.clone(), "cgroup_v2_profile"
                ));
            }
        }
        SandboxTier::Container => {
            if !matches!(
                receipt.isolation_mechanism,
                IsolationMechanism::ContainerRuntime | IsolationMechanism::GVisorContainer
            ) {
                return Err(wrong_mechanism());
            }
            for (value, label) in [
                (&receipt.container_image_digest, "content_addressed_image"),
                (&receipt.mount_policy_digest, "controlled_mounts"),
                (&receipt.network_namespace_digest, "network_namespace"),
                (&receipt.cgroup_profile_digest, "cgroup_v2_profile"),
                (&receipt.tool_subsandbox_digest, "tool_subsandbox"),
            ] {
                if !required(value) {
                    return Err(SandboxAttestationError::TierEvidenceMissing(
                        receipt.trial_id.clone(), label
                    ));
                }
            }
        }
        SandboxTier::MicroVm => {
            if !matches!(
                receipt.isolation_mechanism,
                IsolationMechanism::Firecracker
                    | IsolationMechanism::CloudHypervisor
                    | IsolationMechanism::Crosvm
            ) {
                return Err(wrong_mechanism());
            }
            for (value, label) in [
                (&receipt.hypervisor_digest, "hypervisor"),
                (&receipt.guest_kernel_digest, "guest_kernel"),
                (&receipt.mount_policy_digest, "guest_mount_policy"),
                (&receipt.network_namespace_digest, "guest_network_namespace"),
                (&receipt.dedicated_cpu_set_digest, "dedicated_cpu_set"),
                (&receipt.cgroup_profile_digest, "resource_cgroup"),
                (&receipt.tool_subsandbox_digest, "tool_subsandbox"),
            ] {
                if !required(value) {
                    return Err(SandboxAttestationError::TierEvidenceMissing(
                        receipt.trial_id.clone(), label
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_telemetry(receipt: &SandboxTrialAttestation) -> Result<(), SandboxAttestationError> {
    let arm = receipt.arm;
    let trial = receipt.trial_id.clone();

    if receipt.measurement_start_ms >= receipt.measurement_end_ms
        || receipt.issued_at_ms < receipt.measurement_end_ms
    {
        return Err(SandboxAttestationError::InvalidMeasurementInterval(trial, arm));
    }
    if receipt.clock_uncertainty_ms > 1_000 {
        return Err(SandboxAttestationError::ClockUncertaintyExceeded(trial, arm));
    }
    if receipt.logical_coverage != TelemetryCoverage::Complete
        || is_zero(&receipt.session_identity_digest)
        || is_zero(&receipt.harness_identity_digest)
        || is_zero(&receipt.transcript_commitment)
        || is_zero(&receipt.input_seed_commitment)
    {
        return Err(SandboxAttestationError::IncompleteLogicalTelemetry(trial, arm));
    }
    if receipt.resource_coverage != TelemetryCoverage::Complete
        || receipt.wallclock_ms == 0
        || receipt.cpu_time_ns == 0
        || receipt.peak_memory_bytes == 0
    {
        return Err(SandboxAttestationError::IncompleteResourceTelemetry(trial, arm));
    }
    match receipt.physical_coverage {
        TelemetryCoverage::Complete => {
            let (start, end, uncertainty, calibration, source, device, cpu_energy) = (
                receipt.energy.sample_start_ms,
                receipt.energy.sample_end_ms,
                receipt.energy.uncertainty_ppm,
                receipt.energy.calibration_profile_digest.as_ref(),
                receipt.energy.hardware_source_id.as_deref(),
                receipt.energy.device_identity_digest.as_ref(),
                receipt.energy.cpu_package_microjoules,
            );
            if cpu_energy.is_none()
                || start.is_none()
                || end.is_none()
                || start.unwrap() >= end.unwrap()
                || start.unwrap() > receipt.measurement_start_ms
                || end.unwrap() < receipt.measurement_end_ms
                || uncertainty.map(|v| v > MAX_ENERGY_UNCERTAINTY_PPM).unwrap_or(true)
                || calibration.map(is_zero).unwrap_or(true)
                || source.map(str::trim).map(str::is_empty).unwrap_or(true)
                || device.map(is_zero).unwrap_or(true)
                || receipt.energy.unavailable_reason.is_some()
            {
                return Err(SandboxAttestationError::InvalidEnergyTelemetry(trial, arm));
            }
            if receipt.accelerator_assignment_digest.is_some()
                && receipt.energy.gpu_microjoules.is_none()
            {
                return Err(SandboxAttestationError::InvalidEnergyTelemetry(trial, arm));
            }
        }
        TelemetryCoverage::NotAvailable => {
            if receipt.sandbox_tier == SandboxTier::MicroVm
                || receipt.energy.cpu_package_microjoules.is_some()
                || receipt.energy.gpu_microjoules.is_some()
                || receipt.energy.sample_start_ms.is_some()
                || receipt.energy.sample_end_ms.is_some()
                || receipt.energy.calibration_profile_digest.is_some()
                || receipt.energy.unavailable_reason.as_deref().unwrap_or("").trim().is_empty()
            {
                return Err(SandboxAttestationError::PhysicalTelemetryUnavailable(trial, arm));
            }
        }
        _ => return Err(SandboxAttestationError::PhysicalTelemetryUnavailable(trial, arm)),
    }
    Ok(())
}

fn validate_sandbox_coverage_window(
    receipt: &SandboxTrialAttestation,
    result: &TrialResult,
) -> Result<(), SandboxAttestationError> {
    let first_activity = result.first_started_at_ms.unwrap_or(result.terminal_at_ms);
    let latest_possible_start = receipt.measurement_start_ms
        .saturating_add(receipt.clock_uncertainty_ms);
    let earliest_possible_end = receipt.measurement_end_ms
        .saturating_sub(receipt.clock_uncertainty_ms);
    if latest_possible_start > first_activity || earliest_possible_end < result.terminal_at_ms {
        return Err(SandboxAttestationError::MeasurementDoesNotCoverTrial(
            receipt.trial_id.clone(), receipt.arm
        ));
    }
    Ok(())
}

fn is_zero(digest: &Digest) -> bool {
    digest.0.iter().all(|byte| *byte == 0)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::sealed::{
        ArmRunSummary, BaselineIdentity, FrozenAgentIdentity, PlannedTrial, RunLedgerSummary,
        TrialDisposition,
    };

    fn d(value: u8) -> Digest {
        Digest::from_byte(value)
    }

    fn fixture(
        tier: SandboxTier,
    ) -> (
        RunManifest,
        Digest,
        FreezeReceipt,
        RunLedgerSummary,
        Vec<ScenarioReceipt>,
        Vec<SandboxTrialAttestation>,
    ) {
        let manifest_commitment = d(20);
        let key_class = if tier == SandboxTier::Research {
            SupervisorKeyClass::SoftwareTestOnly
        } else {
            SupervisorKeyClass::HardwareProtected
        };
        let manifest = RunManifest {
            schema: crate::sealed::SEALED_RUN_SCHEMA.into(),
            version: crate::sealed::SEALED_RUN_VERSION,
            run_id: "tiered-run".into(),
            lane: crate::sealed::EvaluationLane::SealedCertification,
            created_at_ms: 10,
            frozen_at_ms: 20,
            evaluator_digest: d(1),
            execution_environment_digest: d(2),
            resource_budget_digest: d(3),
            arm_order_schedule_digest: d(4),
            state_isolation_policy_digest: d(5),
            artifact_access_policy_digest: d(6),
            network_egress_policy_digest: d(7),
            runtime_observer_source_id: "runtime-observer".into(),
            runtime_observer_control_domain: "runtime-observer-domain".into(),
            runtime_observer_adapter_digest: d(8),
            runtime_challenge_authority_id: "challenge-authority".into(),
            runtime_challenge_authority_control_domain: "challenge-domain".into(),
            sandbox_tier: tier,
            sandbox_profile_digest: d(9),
            supervisor_id: "external-supervisor".into(),
            supervisor_source_id: "supervisor-source".into(),
            supervisor_control_domain: "supervisor-domain".into(),
            supervisor_adapter_digest: d(10),
            supervisor_key_class: key_class,
            agent_identity_digest: d(11),
            baseline_identity_digest: d(12),
            agent: FrozenAgentIdentity {
                agent_id: "candidate".into(),
                agent_image_digest: d(13),
                source_commit: "0123456789abcdef0123456789abcdef01234567".into(),
                model_provider: "provider".into(),
                model_id: "candidate-model".into(),
                model_version: "v1".into(),
                scaffold_digest: d(14),
                prompt_digest: d(15),
                toolchain_digest: d(16),
                runtime_config_digest: d(17),
                tool_policy_digest: d(18),
                control_domain: "candidate-domain".into(),
            },
            baseline: BaselineIdentity {
                baseline_id: "baseline".into(),
                image_digest: d(19),
                source_commit: "abcdef0123456789abcdef0123456789abcdef01".into(),
                model_provider: "provider".into(),
                model_id: "baseline-model".into(),
                model_version: "v1".into(),
                scaffold_digest: d(21),
                prompt_digest: d(22),
                toolchain_digest: d(23),
                runtime_config_digest: d(24),
                tool_policy_digest: d(25),
                control_domain: "baseline-domain".into(),
            },
            freeze_authority_id: "freeze-authority".into(),
            freeze_authority_control_domain: "freeze-domain".into(),
            evaluator_control_domain: "evaluator-domain".into(),
            evidence_source_id: "scenario-source".into(),
            evidence_control_domain: "scenario-domain".into(),
            trials: vec![PlannedTrial {
                trial_id: "trial-1".into(),
                cluster_id: "cluster-1".into(),
                first_arm: EvaluationArm::Candidate,
                maximum_attempts: 1,
            }],
        };
        let freeze = FreezeReceipt {
            run_id: manifest.run_id.clone(),
            manifest_commitment: manifest_commitment.clone(),
            committed_at_ms: 15,
            frozen_at_ms: 20,
            agent_identity_digest: manifest.agent_identity_digest.clone(),
            baseline_identity_digest: manifest.baseline_identity_digest.clone(),
            evaluator_digest: manifest.evaluator_digest.clone(),
            execution_environment_digest: manifest.execution_environment_digest.clone(),
            resource_budget_digest: manifest.resource_budget_digest.clone(),
            arm_order_schedule_digest: manifest.arm_order_schedule_digest.clone(),
            state_isolation_policy_digest: manifest.state_isolation_policy_digest.clone(),
            artifact_access_policy_digest: manifest.artifact_access_policy_digest.clone(),
            network_egress_policy_digest: manifest.network_egress_policy_digest.clone(),
            runtime_challenge_authority_id: manifest.runtime_challenge_authority_id.clone(),
            runtime_challenge_authority_control_domain: manifest.runtime_challenge_authority_control_domain.clone(),
            runtime_observer_source_id: manifest.runtime_observer_source_id.clone(),
            runtime_observer_control_domain: manifest.runtime_observer_control_domain.clone(),
            runtime_observer_adapter_digest: manifest.runtime_observer_adapter_digest.clone(),
            sandbox_tier: manifest.sandbox_tier,
            sandbox_profile_digest: manifest.sandbox_profile_digest.clone(),
            supervisor_id: manifest.supervisor_id.clone(),
            supervisor_source_id: manifest.supervisor_source_id.clone(),
            supervisor_control_domain: manifest.supervisor_control_domain.clone(),
            supervisor_adapter_digest: manifest.supervisor_adapter_digest.clone(),
            supervisor_key_class: manifest.supervisor_key_class,
            publisher_id: manifest.freeze_authority_id.clone(),
            publisher_control_domain: manifest.freeze_authority_control_domain.clone(),
        };
        let scenario = ScenarioReceipt {
            run_id: manifest.run_id.clone(),
            trial_id: "trial-1".into(),
            manifest_commitment: manifest_commitment.clone(),
            commitment_scheme: crate::sealed::SCENARIO_COMMITMENT_SCHEME.into(),
            case_payload_commitment: d(26),
            oracle_commitment: d(27),
            generated_at_ms: 25,
            issued_at_ms: 30,
            retired_at_ms: None,
            evaluator_digest: manifest.evaluator_digest.clone(),
            source_id: manifest.evidence_source_id.clone(),
            control_domain: manifest.evidence_control_domain.clone(),
        };
        let candidate = TrialResult {
            trial_id: "trial-1".into(),
            arm: EvaluationArm::Candidate,
            cluster_id: "cluster-1".into(),
            disposition: TrialDisposition::Completed,
            attempts_started: 1,
            first_started_at_ms: Some(40),
            terminal_at_ms: 50,
        };
        let baseline = TrialResult {
            trial_id: "trial-1".into(),
            arm: EvaluationArm::Baseline,
            cluster_id: "cluster-1".into(),
            disposition: TrialDisposition::Completed,
            attempts_started: 1,
            first_started_at_ms: Some(60),
            terminal_at_ms: 70,
        };
        let arm_summary = ArmRunSummary {
            planned_trials: 1,
            completed_trials: 1,
            failed_trials: 0,
            skipped_trials: 0,
            invalidated_trials: 0,
            total_attempts: 1,
        };
        let ledger = RunLedgerSummary {
            run_id: manifest.run_id.clone(),
            lane: manifest.lane,
            planned_trials: 1,
            planned_arm_trials: 2,
            candidate: arm_summary.clone(),
            baseline: arm_summary,
            completed_trials: 2,
            failed_trials: 0,
            skipped_trials: 0,
            invalidated_trials: 0,
            total_attempts: 2,
            unique_clusters: 1,
            manifest_commitment: manifest_commitment.clone(),
            agent_identity_digest: manifest.agent_identity_digest.clone(),
            baseline_identity_digest: manifest.baseline_identity_digest.clone(),
            evaluator_digest: manifest.evaluator_digest.clone(),
            execution_environment_digest: manifest.execution_environment_digest.clone(),
            resource_budget_digest: manifest.resource_budget_digest.clone(),
            arm_order_schedule_digest: manifest.arm_order_schedule_digest.clone(),
            state_isolation_policy_digest: manifest.state_isolation_policy_digest.clone(),
            artifact_access_policy_digest: manifest.artifact_access_policy_digest.clone(),
            network_egress_policy_digest: manifest.network_egress_policy_digest.clone(),
            runtime_observer_source_id: manifest.runtime_observer_source_id.clone(),
            runtime_observer_control_domain: manifest.runtime_observer_control_domain.clone(),
            runtime_observer_adapter_digest: manifest.runtime_observer_adapter_digest.clone(),
            runtime_challenge_authority_id: manifest.runtime_challenge_authority_id.clone(),
            runtime_challenge_authority_control_domain: manifest.runtime_challenge_authority_control_domain.clone(),
            sandbox_tier: manifest.sandbox_tier,
            sandbox_profile_digest: manifest.sandbox_profile_digest.clone(),
            supervisor_id: manifest.supervisor_id.clone(),
            supervisor_source_id: manifest.supervisor_source_id.clone(),
            supervisor_control_domain: manifest.supervisor_control_domain.clone(),
            supervisor_adapter_digest: manifest.supervisor_adapter_digest.clone(),
            supervisor_key_class: manifest.supervisor_key_class,
            trials: vec![candidate.clone(), baseline.clone()],
        };
        let scenarios = vec![scenario.clone()];
        let attestations = vec![
            attestation(&manifest, &scenario, EvaluationArm::Candidate, 1, 35, 55),
            attestation(&manifest, &scenario, EvaluationArm::Baseline, 2, 55, 75),
        ];
        (manifest, manifest_commitment, freeze, ledger, scenarios, attestations)
    }

    fn attestation(
        manifest: &RunManifest,
        scenario: &ScenarioReceipt,
        arm: EvaluationArm,
        sequence: u64,
        start: u64,
        end: u64,
    ) -> SandboxTrialAttestation {
        let arm_identity_digest = match arm {
            EvaluationArm::Candidate => manifest.agent_identity_digest.clone(),
            EvaluationArm::Baseline => manifest.baseline_identity_digest.clone(),
        };
        let (mechanism, seccomp, namespace, cgroup, image, mount, network, tool,
            hypervisor, guest, cpu_set) = match manifest.sandbox_tier {
            SandboxTier::Research => (
                IsolationMechanism::ResearchUnisolated, None, None, None, None, None,
                None, None, None, None, None,
            ),
            SandboxTier::Process => (
                IsolationMechanism::SeccompProcess, Some(d(31)), Some(d(32)), Some(d(33)),
                None, None, None, None, None, None, None,
            ),
            SandboxTier::Container => (
                IsolationMechanism::ContainerRuntime, None, None, Some(d(34)), Some(d(35)),
                Some(d(36)), Some(d(37)), Some(d(38)), None, None, None,
            ),
            SandboxTier::MicroVm => (
                IsolationMechanism::Firecracker, None, None, Some(d(39)), None,
                Some(d(40)), Some(d(41)), Some(d(42)), Some(d(43)), Some(d(44)), Some(d(45)),
            ),
        };
        let (physical_coverage, energy) = if manifest.sandbox_tier == SandboxTier::MicroVm {
            (
                TelemetryCoverage::Complete,
                EnergyTelemetry {
                    cpu_package_microjoules: Some(100),
                    gpu_microjoules: None,
                    sample_start_ms: Some(30),
                    sample_end_ms: Some(80),
                    uncertainty_ppm: Some(5_000),
                    calibration_profile_digest: Some(d(46)),
                    hardware_source_id: Some("rapl".into()),
                    device_identity_digest: Some(d(47)),
                    unavailable_reason: None,
                },
            )
        } else {
            (
                TelemetryCoverage::NotAvailable,
                EnergyTelemetry {
                    cpu_package_microjoules: None,
                    gpu_microjoules: None,
                    sample_start_ms: None,
                    sample_end_ms: None,
                    uncertainty_ppm: None,
                    calibration_profile_digest: None,
                    hardware_source_id: None,
                    device_identity_digest: None,
                    unavailable_reason: Some("sensor unavailable on this test host".into()),
                },
            )
        };
        SandboxTrialAttestation {
            schema: SANDBOX_ATTESTATION_SCHEMA.into(),
            run_id: manifest.run_id.clone(),
            trial_id: scenario.trial_id.clone(),
            arm,
            arm_identity_digest,
            scenario_commitment: scenario.case_payload_commitment.clone(),
            manifest_commitment: scenario.manifest_commitment.clone(),
            execution_environment_digest: manifest.execution_environment_digest.clone(),
            resource_budget_digest: manifest.resource_budget_digest.clone(),
            sandbox_tier: manifest.sandbox_tier,
            sandbox_profile_digest: manifest.sandbox_profile_digest.clone(),
            supervisor_id: manifest.supervisor_id.clone(),
            supervisor_source_id: manifest.supervisor_source_id.clone(),
            supervisor_control_domain: manifest.supervisor_control_domain.clone(),
            supervisor_adapter_digest: manifest.supervisor_adapter_digest.clone(),
            supervisor_key_class: manifest.supervisor_key_class,
            receipt_sequence: sequence,
            runtime_instance_digest: d(50 + sequence as u8),
            writable_state_namespace_digest: d(60 + sequence as u8),
            issued_at_ms: end + 1,
            measurement_start_ms: start,
            measurement_end_ms: end,
            clock_uncertainty_ms: 2,
            host_os: "linux".into(),
            host_architecture: "x86_64".into(),
            host_kernel_digest: d(70),
            hardware_identity_digest: d(71),
            isolation_mechanism: mechanism,
            runtime_version: match manifest.sandbox_tier {
                SandboxTier::Research => "research/no-isolation",
                SandboxTier::Process => "gvisor-or-seccomp/1",
                SandboxTier::Container => "podman/5",
                SandboxTier::MicroVm => "firecracker/1",
            }.into(),
            seccomp_profile_digest: seccomp,
            namespace_profile_digest: namespace,
            cgroup_profile_digest: cgroup,
            container_image_digest: image,
            mount_policy_digest: mount,
            network_namespace_digest: network,
            tool_subsandbox_digest: tool,
            hypervisor_digest: hypervisor,
            guest_kernel_digest: guest,
            dedicated_cpu_set_digest: cpu_set,
            accelerator_assignment_digest: None,
            logical_coverage: TelemetryCoverage::Complete,
            session_identity_digest: d(80 + sequence as u8),
            harness_identity_digest: d(81),
            transcript_commitment: d(82 + sequence as u8),
            input_seed_commitment: d(83),
            tool_call_count: 3,
            input_tokens: 200,
            output_tokens: 50,
            resource_coverage: TelemetryCoverage::Complete,
            wallclock_ms: end - start,
            cpu_time_ns: 1_000_000,
            peak_memory_bytes: 4 * 1024 * 1024,
            disk_read_bytes: 1_000,
            disk_write_bytes: 2_000,
            network_rx_bytes: 3_000,
            network_tx_bytes: 4_000,
            accelerator_utilization_ppm: None,
            accelerator_memory_bytes: None,
            retry_count: 0,
            physical_coverage,
            energy,
        }
    }

    fn validate(
        fixture: &(
            RunManifest, Digest, FreezeReceipt, RunLedgerSummary,
            Vec<ScenarioReceipt>, Vec<SandboxTrialAttestation>
        ),
        attestations: &[SandboxTrialAttestation],
    ) -> Result<SandboxCertificationSummary, SandboxAttestationError> {
        validate_sandbox_attestations(
            &fixture.0,
            &fixture.1,
            &fixture.2,
            &fixture.3,
            &fixture.4,
            attestations,
        )
    }

    #[test]
    fn research_process_container_and_microvm_positive_controls_are_tier_stratified() {
        for tier in [
            SandboxTier::Research,
            SandboxTier::Process,
            SandboxTier::Container,
            SandboxTier::MicroVm,
        ] {
            let fixture = fixture(tier);
            let result = validate(&fixture, &fixture.5).unwrap();
            assert_eq!(result.tier, tier);
            assert_eq!(result.planned_arm_trials, 2);
            assert_eq!(result.complete_attestations, 2);
            assert_eq!(result.supervisor_id, "external-supervisor");
            assert_eq!(result.all_physical_energy_measured, tier == SandboxTier::MicroVm);
            assert_eq!(
                result.total_cpu_package_microjoules,
                if tier == SandboxTier::MicroVm { Some(200) } else { None }
            );
        }
    }

    #[test]
    fn supervisor_receipt_repeats_exact_environment_and_budget_digests() {
        let fixture = fixture(SandboxTier::Process);
        let mut receipts = fixture.5.clone();
        receipts[0].execution_environment_digest = d(99);
        assert_eq!(
            validate(&fixture, &receipts),
            Err(SandboxAttestationError::AttestationBindingMismatch(
                "trial-1".into(), EvaluationArm::Candidate
            ))
        );
        let mut receipts = fixture.5.clone();
        receipts[0].resource_budget_digest = d(98);
        assert_eq!(
            validate(&fixture, &receipts),
            Err(SandboxAttestationError::AttestationBindingMismatch(
                "trial-1".into(), EvaluationArm::Candidate
            ))
        );
    }

    #[test]
    fn candidate_baseline_cannot_share_runtime_or_writable_namespace() {
        let fixture = fixture(SandboxTier::Process);
        let mut receipts = fixture.5.clone();
        let duplicate_runtime = receipts[0].runtime_instance_digest.clone();
        receipts[1].runtime_instance_digest = duplicate_runtime;
        assert_eq!(
            validate(&fixture, &receipts),
            Err(SandboxAttestationError::DuplicateRuntimeInstance)
        );
        let mut receipts = fixture.5.clone();
        let duplicate_namespace = receipts[0].writable_state_namespace_digest.clone();
        receipts[1].writable_state_namespace_digest = duplicate_namespace;
        assert_eq!(
            validate(&fixture, &receipts),
            Err(SandboxAttestationError::DuplicateStateNamespace)
        );
    }

    #[test]
    fn missing_duplicate_and_unplanned_attestations_are_rejected() {
        let fixture = fixture(SandboxTier::Process);
        assert_eq!(
            validate(&fixture, &fixture.5[..1]),
            Err(SandboxAttestationError::MissingAttestation(
                "trial-1".into(), EvaluationArm::Baseline
            ))
        );
        let mut duplicate = fixture.5.clone();
        let extra_duplicate = duplicate[0].clone();
        duplicate.push(extra_duplicate);
        assert_eq!(
            validate(&fixture, &duplicate),
            Err(SandboxAttestationError::DuplicateAttestation(
                "trial-1".into(), EvaluationArm::Candidate
            ))
        );
        let mut extra = fixture.5.clone();
        extra[0].trial_id = "unplanned".into();
        assert!(validate(&fixture, &extra).is_err());
    }

    #[test]
    fn process_tier_requires_seccomp_namespaces_and_cgroups() {
        let fixture = fixture(SandboxTier::Process);
        let mut receipts = fixture.5.clone();
        receipts[0].seccomp_profile_digest = None;
        assert_eq!(
            validate(&fixture, &receipts),
            Err(SandboxAttestationError::TierEvidenceMissing(
                "trial-1".into(), "seccomp_profile"
            ))
        );
        let mut receipts = fixture.5.clone();
        receipts[0].namespace_profile_digest = None;
        assert!(validate(&fixture, &receipts).is_err());
        let mut receipts = fixture.5.clone();
        receipts[0].cgroup_profile_digest = None;
        assert!(validate(&fixture, &receipts).is_err());
    }

    #[test]
    fn container_tier_requires_content_addressed_image_mount_network_and_tool_sandbox() {
        let fixture = fixture(SandboxTier::Container);
        for mutation in 0..5 {
            let mut receipts = fixture.5.clone();
            match mutation {
                0 => receipts[0].container_image_digest = None,
                1 => receipts[0].mount_policy_digest = None,
                2 => receipts[0].network_namespace_digest = None,
                3 => receipts[0].cgroup_profile_digest = None,
                _ => receipts[0].tool_subsandbox_digest = None,
            }
            assert!(validate(&fixture, &receipts).is_err());
        }
    }

    #[test]
    fn microvm_tier_requires_hypervisor_guest_kernel_cpu_set_and_tool_isolation() {
        let fixture = fixture(SandboxTier::MicroVm);
        for mutation in 0..6 {
            let mut receipts = fixture.5.clone();
            match mutation {
                0 => receipts[0].hypervisor_digest = None,
                1 => receipts[0].guest_kernel_digest = None,
                2 => receipts[0].dedicated_cpu_set_digest = None,
                3 => receipts[0].mount_policy_digest = None,
                4 => receipts[0].network_namespace_digest = None,
                _ => receipts[0].tool_subsandbox_digest = None,
            }
            assert!(validate(&fixture, &receipts).is_err());
        }
    }

    #[test]
    fn microvm_requires_hardware_protected_supervisor_and_measured_energy() {
        let fixture = fixture(SandboxTier::MicroVm);
        let mut manifest = fixture.0.clone();
        manifest.supervisor_key_class = SupervisorKeyClass::SoftwareTestOnly;
        let mut freeze = fixture.2.clone();
        freeze.supervisor_key_class = SupervisorKeyClass::SoftwareTestOnly;
        let mut ledger = fixture.3.clone();
        ledger.supervisor_key_class = SupervisorKeyClass::SoftwareTestOnly;
        let altered = (
            manifest, fixture.1.clone(), freeze, ledger, fixture.4.clone(), fixture.5.clone()
        );
        assert!(validate(&altered, &altered.5).is_err());
        let mut receipts = fixture.5.clone();
        receipts[0].physical_coverage = TelemetryCoverage::NotAvailable;
        receipts[0].energy = EnergyTelemetry {
            cpu_package_microjoules: None,
            gpu_microjoules: None,
            sample_start_ms: None,
            sample_end_ms: None,
            uncertainty_ppm: None,
            calibration_profile_digest: None,
            hardware_source_id: None,
            device_identity_digest: None,
            unavailable_reason: Some("no physical sensor".into()),
        };
        assert!(validate(&fixture, &receipts).is_err());
    }

    #[test]
    fn no_partial_or_unknown_logical_or_resource_telemetry_is_accepted() {
        let fixture = fixture(SandboxTier::Process);
        let mut receipts = fixture.5.clone();
        receipts[0].logical_coverage = TelemetryCoverage::Partial;
        assert_eq!(
            validate(&fixture, &receipts),
            Err(SandboxAttestationError::IncompleteLogicalTelemetry(
                "trial-1".into(), EvaluationArm::Candidate
            ))
        );
        let mut receipts = fixture.5.clone();
        receipts[0].resource_coverage = TelemetryCoverage::Unknown;
        assert!(validate(&fixture, &receipts).is_err());
    }

    #[test]
    fn physical_energy_requires_interval_calibration_source_and_bounded_uncertainty() {
        let fixture = fixture(SandboxTier::MicroVm);
        let mut receipts = fixture.5.clone();
        receipts[0].energy.calibration_profile_digest = Some(d(0));
        assert!(validate(&fixture, &receipts).is_err());
        let mut receipts = fixture.5.clone();
        receipts[0].energy.sample_start_ms = Some(36);
        assert!(validate(&fixture, &receipts).is_err());
        let mut receipts = fixture.5.clone();
        receipts[0].energy.uncertainty_ppm = Some(MAX_ENERGY_UNCERTAINTY_PPM + 1);
        assert!(validate(&fixture, &receipts).is_err());
        let mut receipts = fixture.5.clone();
        receipts[0].energy.hardware_source_id = Some(String::new());
        assert!(validate(&fixture, &receipts).is_err());
    }

    #[test]
    fn full_trial_measurement_interval_and_clock_uncertainty_are_enforced() {
        let fixture = fixture(SandboxTier::Process);
        let mut receipts = fixture.5.clone();
        receipts[0].measurement_start_ms = 39;
        assert!(validate(&fixture, &receipts).is_err());
        let mut receipts = fixture.5.clone();
        receipts[0].measurement_end_ms = 49;
        receipts[0].issued_at_ms = 50;
        assert!(validate(&fixture, &receipts).is_err());
        let mut receipts = fixture.5.clone();
        receipts[0].clock_uncertainty_ms = 1_001;
        assert!(validate(&fixture, &receipts).is_err());
    }

    #[test]
    fn supervisor_identity_key_and_control_domain_must_match_frozen_manifest() {
        let fixture = fixture(SandboxTier::Process);
        let mut receipts = fixture.5.clone();
        receipts[0].supervisor_source_id = "other-source".into();
        assert_eq!(
            validate(&fixture, &receipts),
            Err(SandboxAttestationError::AttestationBindingMismatch(
                "trial-1".into(), EvaluationArm::Candidate
            ))
        );
        let mut receipts = fixture.5.clone();
        receipts[0].supervisor_control_domain = "candidate-domain".into();
        assert_eq!(
            validate(&fixture, &receipts),
            Err(SandboxAttestationError::AttestationBindingMismatch(
                "trial-1".into(), EvaluationArm::Candidate
            ))
        );
    }

    #[test]
    fn observer_sequence_must_start_at_one_and_be_contiguous_in_issue_order() {
        let fixture = fixture(SandboxTier::Process);
        let mut receipts = fixture.5.clone();
        receipts[0].receipt_sequence = 2;
        receipts[1].receipt_sequence = 3;
        assert_eq!(
            validate(&fixture, &receipts),
            Err(SandboxAttestationError::SupervisorSequenceNotContiguous)
        );
        let mut receipts = fixture.5.clone();
        receipts[0].issued_at_ms = 90;
        assert_eq!(
            validate(&fixture, &receipts),
            Err(SandboxAttestationError::SupervisorSequenceNotContiguous)
        );
    }

    #[test]
    fn hardware_kernel_and_architecture_must_match_the_frozen_environment() {
        let fixture = fixture(SandboxTier::Process);
        let mut receipts = fixture.5.clone();
        receipts[1].hardware_identity_digest = d(99);
        assert!(validate(&fixture, &receipts).is_err());
        let mut receipts = fixture.5.clone();
        receipts[1].host_kernel_digest = d(98);
        assert!(validate(&fixture, &receipts).is_err());
        let mut receipts = fixture.5.clone();
        receipts[1].host_architecture = "aarch64".into();
        assert!(validate(&fixture, &receipts).is_err());
    }

    #[test]
    fn resource_totals_are_reported_only_when_physical_energy_is_measured() {
        let vm_fixture = fixture(SandboxTier::MicroVm);
        let result = validate(&vm_fixture, &vm_fixture.5).unwrap();
        assert_eq!(result.total_cpu_package_microjoules, Some(200));
        assert_eq!(result.total_gpu_microjoules, None);
        assert!(result.all_physical_energy_measured);
        let process = fixture(SandboxTier::Process);
        let result = validate(&process, &process.5).unwrap();
        assert_eq!(result.total_cpu_package_microjoules, None);
        assert!(!result.all_physical_energy_measured);
    }

    #[test]
    fn sixty_four_adversarial_sandbox_mutations_never_pass() {
        for vector in 0..64 {
            let tier = if (38..=54).contains(&vector) {
                SandboxTier::MicroVm
            } else {
                SandboxTier::Process
            };
            let (mut manifest, commitment, mut freeze, mut ledger, mut scenarios, mut receipts) =
                fixture(tier);
            match vector {
                0 => manifest.run_id = "other-run".into(),
                1 => manifest.sandbox_profile_digest = d(99),
                2 => manifest.supervisor_id = "wrong-supervisor".into(),
                3 => manifest.supervisor_source_id = "wrong-source".into(),
                4 => manifest.supervisor_control_domain = manifest.agent.control_domain.clone(),
                5 => manifest.supervisor_adapter_digest = d(0),
                6 => manifest.sandbox_tier = SandboxTier::Research,
                7 => freeze.sandbox_profile_digest = d(99),
                8 => freeze.supervisor_id = "wrong-supervisor".into(),
                9 => freeze.supervisor_key_class = SupervisorKeyClass::SoftwareTestOnly,
                10 => ledger.manifest_commitment = d(99),
                11 => ledger.sandbox_profile_digest = d(99),
                12 => ledger.supervisor_source_id = "wrong".into(),
                13 => scenarios[0].manifest_commitment = d(99),
                14 => scenarios[0].case_payload_commitment = d(99),
                15 => receipts[0].schema = "unknown-schema".into(),
                16 => receipts[0].trial_id = "unplanned".into(),
                17 => receipts[0].arm_identity_digest = d(99),
                18 => receipts[0].scenario_commitment = d(99),
                19 => receipts[0].manifest_commitment = d(99),
                20 => receipts[0].sandbox_tier = SandboxTier::Container,
                21 => receipts[0].sandbox_profile_digest = d(99),
                22 => receipts[0].supervisor_id = "attacker".into(),
                23 => receipts[0].supervisor_source_id = "attacker".into(),
                24 => receipts[0].supervisor_control_domain = "candidate-domain".into(),
                25 => receipts[0].supervisor_adapter_digest = d(0),
                26 => receipts[0].supervisor_key_class = SupervisorKeyClass::SoftwareTestOnly,
                27 => receipts[0].receipt_sequence = 0,
                28 => { let duplicate_sequence = receipts[0].receipt_sequence; receipts[1].receipt_sequence = duplicate_sequence; },
                29 => receipts[0].issued_at_ms = receipts[0].measurement_end_ms - 1,
                30 => receipts[0].measurement_start_ms = 39,
                31 => receipts[0].measurement_end_ms = 49,
                32 => receipts[0].clock_uncertainty_ms = 5_000,
                33 => receipts[0].host_os.clear(),
                34 => receipts[0].host_architecture.clear(),
                35 => receipts[0].host_kernel_digest = d(0),
                36 => receipts[0].hardware_identity_digest = d(0),
                37 => receipts[0].isolation_mechanism = IsolationMechanism::ResearchUnisolated,
                38 => receipts[0].hypervisor_digest = None,
                39 => receipts[0].guest_kernel_digest = None,
                40 => receipts[0].dedicated_cpu_set_digest = None,
                41 => receipts[0].mount_policy_digest = None,
                42 => receipts[0].network_namespace_digest = None,
                43 => receipts[0].tool_subsandbox_digest = None,
                44 => receipts[0].physical_coverage = TelemetryCoverage::Partial,
                45 => receipts[0].energy.cpu_package_microjoules = None,
                46 => receipts[0].energy.calibration_profile_digest = Some(d(0)),
                47 => receipts[0].energy.sample_end_ms = Some(40),
                48 => receipts[0].energy.uncertainty_ppm = Some(200_000),
                49 => receipts[0].logical_coverage = TelemetryCoverage::Unknown,
                50 => receipts[0].transcript_commitment = d(0),
                51 => receipts[0].resource_coverage = TelemetryCoverage::Tampered,
                52 => receipts[0].cpu_time_ns = 0,
                53 => receipts[0].peak_memory_bytes = 0,
                54 => receipts[0].writable_state_namespace_digest = d(0),
                55 => { let duplicate_runtime = receipts[0].runtime_instance_digest.clone(); receipts[1].runtime_instance_digest = duplicate_runtime; },
                56 => { let duplicate_namespace = receipts[0].writable_state_namespace_digest.clone(); receipts[1].writable_state_namespace_digest = duplicate_namespace; },
                57 => { let extra = receipts[0].clone(); receipts.push(extra); },
                58 => { receipts.pop(); },
                59 => receipts[0].hardware_identity_digest = d(88),
                60 => receipts[1].host_kernel_digest = d(89),
                61 => receipts[1].host_architecture = "aarch64".into(),
                62 => receipts[0].runtime_version.clear(),
                _ => {
                    receipts[0].energy.cpu_package_microjoules = Some(u128::MAX);
                    receipts[1].energy.cpu_package_microjoules = Some(u128::MAX);
                },
            }
            let result = validate_sandbox_attestations(
                &manifest, &commitment, &freeze, &ledger, &scenarios, &receipts
            );
            assert!(
                result.is_err(),
                "sandbox mutation vector {vector} was accepted"
            );
        }
    }

    #[test]
    fn sixty_four_adversarial_vectors_run_in_each_of_the_four_sandbox_tiers() {
        for tier in [
            SandboxTier::Research,
            SandboxTier::Process,
            SandboxTier::Container,
            SandboxTier::MicroVm,
        ] {
            for vector in 0..64 {
                let (manifest, commitment, mut freeze, mut ledger, mut scenarios, mut receipts) =
                    fixture(tier);
                let other_tier = if tier == SandboxTier::Research {
                    SandboxTier::Process
                } else {
                    SandboxTier::Research
                };
                match vector {
                    0 => receipts[0].schema = "wrong-schema".into(),
                    1 => receipts[0].run_id = "another-run".into(),
                    2 => receipts[0].trial_id = "unknown-trial".into(),
                    3 => receipts[0].arm_identity_digest = d(99),
                    4 => receipts[0].scenario_commitment = d(99),
                    5 => receipts[0].manifest_commitment = d(99),
                    6 => receipts[0].sandbox_tier = other_tier,
                    7 => receipts[0].sandbox_profile_digest = d(0),
                    8 => receipts[0].supervisor_id = "substituted-supervisor".into(),
                    9 => receipts[0].supervisor_source_id = "substituted-source".into(),
                    10 => receipts[0].supervisor_control_domain = manifest.agent.control_domain.clone(),
                    11 => receipts[0].supervisor_adapter_digest = d(0),
                    12 => receipts[0].receipt_sequence = 0,
                    13 => { let seq = receipts[0].receipt_sequence; receipts[1].receipt_sequence = seq; },
                    14 => receipts[0].issued_at_ms = receipts[0].measurement_end_ms - 1,
                    15 => receipts[0].measurement_start_ms = 39,
                    16 => receipts[0].measurement_end_ms = 49,
                    17 => receipts[0].clock_uncertainty_ms = 1_001,
                    18 => receipts[0].host_os.clear(),
                    19 => receipts[0].host_architecture.clear(),
                    20 => receipts[0].host_kernel_digest = d(0),
                    21 => receipts[0].hardware_identity_digest = d(0),
                    22 => receipts[0].isolation_mechanism = match tier {
                        SandboxTier::Research => IsolationMechanism::Firecracker,
                        _ => IsolationMechanism::ResearchUnisolated,
                    },
                    23 => receipts[0].runtime_version.clear(),
                    24 => receipts[0].session_identity_digest = d(0),
                    25 => receipts[0].harness_identity_digest = d(0),
                    26 => receipts[0].transcript_commitment = d(0),
                    27 => receipts[0].input_seed_commitment = d(0),
                    28 => receipts[0].logical_coverage = TelemetryCoverage::Partial,
                    29 => receipts[0].logical_coverage = TelemetryCoverage::Unknown,
                    30 => receipts[0].resource_coverage = TelemetryCoverage::Partial,
                    31 => receipts[0].resource_coverage = TelemetryCoverage::Unknown,
                    32 => receipts[0].wallclock_ms = 0,
                    33 => receipts[0].cpu_time_ns = 0,
                    34 => receipts[0].peak_memory_bytes = 0,
                    35 => receipts[0].runtime_instance_digest = d(0),
                    36 => receipts[0].writable_state_namespace_digest = d(0),
                    37 => { let duplicate = receipts[0].runtime_instance_digest.clone(); receipts[1].runtime_instance_digest = duplicate; },
                    38 => { let duplicate = receipts[0].writable_state_namespace_digest.clone(); receipts[1].writable_state_namespace_digest = duplicate; },
                    39 => receipts[0].physical_coverage = TelemetryCoverage::Partial,
                    40 => receipts[0].physical_coverage = TelemetryCoverage::Unknown,
                    41 => receipts[0].physical_coverage = TelemetryCoverage::Tampered,
                    42 => { let duplicate = receipts[0].clone(); receipts.push(duplicate); },
                    43 => { receipts.pop(); },
                    44 => receipts[0].trial_id = "unplanned-trial".into(),
                    45 => receipts[1].receipt_sequence = 4,
                    46 => receipts[0].issued_at_ms = 90,
                    47 => { scenarios.clear(); },
                    48 => { let duplicate = scenarios[0].clone(); scenarios.push(duplicate); },
                    49 => receipts[0].arm = EvaluationArm::Baseline,
                    50 => receipts[0].run_id = "different-run".into(),
                    51 => scenarios[0].case_payload_commitment = d(99),
                    52 => ledger.run_id = "different-ledger-run".into(),
                    53 => ledger.manifest_commitment = d(99),
                    54 => ledger.sandbox_profile_digest = d(99),
                    55 => ledger.supervisor_id = "wrong-supervisor".into(),
                    56 => ledger.planned_arm_trials = 3,
                    57 => freeze.supervisor_source_id = "other-source".into(),
                    58 => freeze.supervisor_control_domain = "other-domain".into(),
                    59 => freeze.sandbox_profile_digest = d(99),
                    60 => freeze.supervisor_adapter_digest = d(0),
                    61 => freeze.sandbox_tier = other_tier,
                    62 => {},
                    _ => receipts[0].measurement_end_ms = 0,
                }
                let expected_commitment = if vector == 62 { d(99) } else { commitment.clone() };
                let result = validate_sandbox_attestations(
                    &manifest,
                    &expected_commitment,
                    &freeze,
                    &ledger,
                    &scenarios,
                    &receipts,
                );
                assert!(
                    result.is_err(),
                    "sandbox tier {tier:?} mutation vector {vector} was accepted"
                );
            }
        }
    }

}
