//! Tiered sandbox profile and logical/resource/physical telemetry validation.
//!
//! This module validates the attested sandbox boundary and telemetry contract.
//! It does not provision isolation or authenticate the Supervisor. The existing
//! typed evidence verifier must authenticate Supervisor receipts before use.

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
    pub sandbox_tier: SandboxTier,
    pub sandbox_profile_digest: Digest,
    pub supervisor_id: String,
    pub supervisor_source_id: String,
    pub supervisor_control_domain: String,
    pub supervisor_adapter_digest: Digest,
    pub supervisor_key_class: SupervisorKeyClass,
    pub receipt_sequence: u64,
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
    validate_sandbox_ledger_binding(manifest, expected_manifest_commitment, ledger)?;

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

        if receipt.receipt_sequence == 0 || !supervisor_sequences.insert(receipt.receipt_sequence) {
            return Err(SandboxAttestationError::DuplicateSupervisorSequence);
        }
        sequence_order.push((receipt.issued_at_ms, receipt.receipt_sequence));

        if !runtime_instances.insert(receipt.session_identity_digest.clone()) {
            // Session identity is a per-trial logical ID and must be unique.
            return Err(SandboxAttestationError::DuplicateRuntimeInstance);
        }
        if !state_namespaces.insert(receipt.namespace_digest()) {
            return Err(SandboxAttestationError::DuplicateStateNamespace);
        }

        if common_hardware.as_ref().map(|v| v != &receipt.hardware_identity_digest).unwrap_or(false)
            || common_kernel.as_ref().map(|v| v != &receipt.host_kernel_digest).unwrap_or(false)
            || common_architecture.map(|v| v != receipt.host_architecture).unwrap_or(false)
        {
            return Err(SandboxAttestationError::HardwareProfileMismatch(
                receipt.trial_id.clone(), receipt.arm
            ));
        }
        common_hardware.get_or_insert_with(|| receipt.hardware_identity_digest.clone());
        common_kernel.get_or_insert_with(|| receipt.host_kernel_digest.clone());
        common_architecture.get_or_insert(receipt.host_architecture.as_str());

        if let Some(energy) = receipt.energy.cpu_package_microjoules {
            total_cpu_energy = total_cpu_energy.and_then(|sum| sum.checked_add(energy));
        } else {
            total_cpu_energy = None;
        }
        if let Some(energy) = receipt.energy.gpu_microjoules {
            total_gpu_energy = total_gpu_energy.and_then(|sum| sum.checked_add(energy));
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
    if sequence_order.iter().enumerate().any(|(index, (_, sequence))| {
        *sequence != (index as u64) + 1
    }) {
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

impl SandboxTrialAttestation {
    fn namespace_digest(&self) -> Digest {
        // The writable namespace is represented by mount + network namespace + runtime
        // profile commitments. It is distinct from a process/session identity.
        combine_namespace_digests(
            &self.mount_policy_digest,
            &self.network_namespace_digest,
            &self.sandbox_profile_digest,
        )
    }
}

fn combine_namespace_digests(
    mount: &Option<Digest>,
    network: &Option<Digest>,
    profile: &Digest,
) -> Digest {
    // This is only an in-memory collision key. It is not a cryptographic commitment.
    let mut bytes = profile.0;
    if let Some(v) = mount { for (slot, value) in bytes.iter_mut().zip(v.0.iter()) { *slot ^= value; } }
    if let Some(v) = network { for (slot, value) in bytes.iter_mut().zip(v.0.iter()) { *slot ^= value; } }
    Digest(bytes)
}

fn validate_sandbox_ledger_binding(
    manifest: &RunManifest,
    expected_manifest_commitment: &Digest,
    ledger: &RunLedgerSummary,
) -> Result<(), SandboxAttestationError> {
    if ledger.run_id != manifest.run_id
        || ledger.lane != manifest.lane
        || ledger.manifest_commitment != *expected_manifest_commitment
        || ledger.agent_identity_digest != manifest.agent_identity_digest
        || ledger.baseline_identity_digest != manifest.baseline_identity_digest
        || ledger.evaluator_digest != manifest.evaluator_digest
        || ledger.execution_environment_digest != manifest.execution_environment_digest
        || ledger.resource_budget_digest != manifest.resource_budget_digest
        || ledger.sandbox_tier != manifest.sandbox_tier
        || ledger.sandbox_profile_digest != manifest.sandbox_profile_digest
        || ledger.supervisor_id != manifest.supervisor_id
        || ledger.supervisor_source_id != manifest.supervisor_source_id
        || ledger.supervisor_control_domain != manifest.supervisor_control_domain
        || ledger.supervisor_adapter_digest != manifest.supervisor_adapter_digest
        || ledger.supervisor_key_class != manifest.supervisor_key_class
        || ledger.planned_trials != manifest.trials.len()
        || ledger.planned_arm_trials != manifest.trials.len().saturating_mul(2)
    {
        return Err(SandboxAttestationError::LedgerManifestMismatch);
    }
    Ok(())
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
