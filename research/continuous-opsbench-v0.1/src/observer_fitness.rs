//! Known-answer observer fitness and telemetry-loss calibration.
//!
//! This validator measures whether the exact pinned runtime-observer profile
//! detects independently injected positive, negative and loss controls. It is a
//! structural evaluator only. The source-evidence adapter must authenticate the
//! injector, observer and verifier records and verify their canonical digests.

use crate::sealed::{
    validate_freeze_receipt, validate_manifest, RunManifest, FreezeReceipt,
};
use crate::Digest;
use std::collections::{BTreeMap, BTreeSet};

pub const FITNESS_PLAN_SCHEMA: &str = "continuous-opsbench-observer-fitness-plan-v0.1";
pub const FITNESS_INJECTION_SCHEMA: &str = "continuous-opsbench-observer-probe-injection-v0.1";
pub const FITNESS_RECEIPT_SCHEMA: &str = "continuous-opsbench-observer-fitness-receipt-v0.1";
pub const FITNESS_MAX_CLOCK_UNCERTAINTY_MS: u64 = 1_000;
pub const FITNESS_MAX_RECEIPT_TTL_MS: u64 = 3_600_000;
pub const FITNESS_MAX_PROBE_LATENCY_MS: u64 = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProbeSurface {
    ProcessExec,
    FileRead,
    ObjectRead,
    OracleAccess,
    NetworkConnect,
    DnsLookup,
    NamespaceChange,
    CgroupMembership,
    ObserverHeartbeat,
    EventStreamLoss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProbeExpectation {
    MustObserve,
    MustNotObserve,
    MustDetectLoss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeOperation {
    AllowedOperation,
    DeniedOperation,
    NegativeControl,
    InducedEventLoss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitnessCoverage {
    Complete,
    Partial,
    Unknown,
    Tampered,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeDefinition {
    pub probe_id: String,
    pub surface: ProbeSurface,
    pub expectation: ProbeExpectation,
    /// Exact event class expected from the observer for MustObserve.
    pub expected_event_class: String,
    pub maximum_detection_latency_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObserverFitnessPlan {
    pub schema: String,
    pub plan_id: String,
    pub run_id: String,
    pub manifest_commitment: Digest,
    pub plan_digest: Digest,
    pub created_at_ms: u64,
    pub runtime_observer_source_id: String,
    pub runtime_observer_control_domain: String,
    pub runtime_observer_adapter_digest: Digest,
    pub runtime_kernel_profile_digest: Digest,
    pub execution_environment_digest: Digest,
    pub state_isolation_policy_digest: Digest,
    pub artifact_access_policy_digest: Digest,
    pub network_egress_policy_digest: Digest,
    pub probe_injector_id: String,
    pub probe_injector_control_domain: String,
    pub fitness_verifier_id: String,
    pub fitness_verifier_control_domain: String,
    pub probes: Vec<ProbeDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeInjectionReceipt {
    pub schema: String,
    pub run_id: String,
    pub batch_id: String,
    pub plan_digest: Digest,
    pub probe_id: String,
    /// Unpredictable per-batch marker; it must not be reused across batches.
    pub marker_digest: Digest,
    pub surface: ProbeSurface,
    pub expectation: ProbeExpectation,
    pub operation: ProbeOperation,
    pub calibration_runtime_digest: Digest,
    pub calibration_namespace_digest: Digest,
    pub issued_at_ms: u64,
    pub executed_at_ms: u64,
    pub completed_at_ms: u64,
    pub injector_id: String,
    pub injector_control_domain: String,
    pub sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedProbeEvent {
    pub batch_id: String,
    pub probe_id: String,
    pub marker_digest: Digest,
    pub surface: ProbeSurface,
    pub event_class: String,
    pub calibration_runtime_digest: Digest,
    pub observed_at_ms: u64,
    pub observer_source_id: String,
    pub observer_control_domain: String,
    pub sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedProbeLoss {
    pub batch_id: String,
    pub probe_id: String,
    pub marker_digest: Digest,
    pub calibration_runtime_digest: Digest,
    pub observed_at_ms: u64,
    pub dropped_event_count: u64,
    pub sequence_gap_count: u64,
    pub observer_source_id: String,
    pub observer_control_domain: String,
    pub sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObserverFitnessReceipt {
    pub schema: String,
    pub receipt_digest: Digest,
    pub batch_nonce_digest: Digest,
    pub batch_id: String,
    pub run_id: String,
    pub manifest_commitment: Digest,
    pub plan_digest: Digest,
    pub runtime_observer_source_id: String,
    pub runtime_observer_control_domain: String,
    pub runtime_observer_adapter_digest: Digest,
    pub runtime_kernel_profile_digest: Digest,
    pub execution_environment_digest: Digest,
    pub state_isolation_policy_digest: Digest,
    pub artifact_access_policy_digest: Digest,
    pub network_egress_policy_digest: Digest,
    pub probe_injector_id: String,
    pub probe_injector_control_domain: String,
    pub fitness_verifier_id: String,
    pub fitness_verifier_control_domain: String,
    pub calibration_runtime_digest: Digest,
    pub calibration_namespace_digest: Digest,
    pub measurement_start_ms: u64,
    pub measurement_end_ms: u64,
    pub issued_at_ms: u64,
    /// Fitness is valid from verifier issue time, not from probe execution time.
    pub valid_from_ms: u64,
    pub expires_at_ms: u64,
    pub clock_uncertainty_ms: u64,
    pub coverage: FitnessCoverage,
    pub unexplained_drop_count: u64,
    pub unexplained_sequence_gap_count: u64,
    pub injection_bundle_digest: Digest,
    pub observation_bundle_digest: Digest,
    pub loss_bundle_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObserverFitnessBatch {
    pub receipt: ObserverFitnessReceipt,
    pub injections: Vec<ProbeInjectionReceipt>,
    pub observations: Vec<ObservedProbeEvent>,
    pub loss_observations: Vec<ObservedProbeLoss>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObserverFitnessBundle {
    pub plan: ObserverFitnessPlan,
    pub batches: Vec<ObserverFitnessBatch>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObserverFitnessBatchSummary {
    pub receipt_digest: Digest,
    pub batch_id: String,
    pub calibration_runtime_digest: Digest,
    pub calibration_namespace_digest: Digest,
    pub valid_from_ms: u64,
    pub expires_at_ms: u64,
    pub positive_controls: usize,
    pub negative_controls: usize,
    pub loss_controls: usize,
    pub maximum_detection_latency_ms: u64,
    pub observer_sequence_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObserverFitnessSummary {
    pub run_id: String,
    pub plan_digest: Digest,
    pub batches: Vec<ObserverFitnessBatchSummary>,
    pub total_positive_controls: usize,
    pub total_negative_controls: usize,
    pub total_loss_controls: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObserverFitnessError {
    UnsupportedPlanSchema,
    UnsupportedInjectionSchema,
    UnsupportedReceiptSchema,
    InvalidIdentity,
    InvalidTrustDomainSeparation,
    ManifestBindingMismatch,
    FreezeBindingMismatch,
    ZeroOrDuplicateDigest,
    EmptyProbePlan,
    DuplicateProbeId(String),
    MissingRequiredPositiveControl(ProbeSurface),
    MissingRequiredNegativeControl(ProbeSurface),
    MissingLossControl,
    InvalidProbeLatency(String),
    MissingBatch,
    DuplicateBatch(String),
    DuplicateInjection(String),
    MissingInjection(String),
    UnplannedInjection(String),
    InjectionBindingMismatch(String),
    InjectionOutsideMeasurement(String),
    InjectionBeforeFreeze(String),
    InvalidOperationForExpectation(String),
    DuplicateObservation(String),
    UnexpectedObservation(String),
    MissingExpectedObservation(String),
    WrongObservedEventClass(String),
    ObservationRuntimeMismatch(String),
    ObservationOutsideMeasurement(String),
    DetectionLatencyExceeded(String),
    DuplicateLossObservation(String),
    MissingLossObservation(String),
    UnexpectedLossObservation(String),
    LossEvidenceMissing(String),
    LossRuntimeMismatch(String),
    LossOutsideMeasurement(String),
    ObserverIdentityMismatch,
    InjectorIdentityMismatch,
    VerifierIdentityMismatch,
    RuntimeNamespaceCollision,
    IncompleteFitnessCoverage(FitnessCoverage),
    UnexplainedTelemetryLoss,
    InvalidFitnessInterval,
    FitnessReceiptIssuedBeforeMeasurement,
    FitnessReceiptFreshnessInvalid,
    FitnessReceiptLifetimeExceeded,
    FitnessReceiptBindingMismatch(String),
    DuplicateObserverSequence,
    ObserverSequenceNotContiguous,
    ObserverSequenceOrderInvalid,
    ArithmeticOverflow,
}

pub fn validate_observer_fitness_bundle(
    manifest: &RunManifest,
    freeze: &FreezeReceipt,
    bundle: &ObserverFitnessBundle,
) -> Result<ObserverFitnessSummary, ObserverFitnessError> {
    validate_manifest(manifest).map_err(|_| ObserverFitnessError::ManifestBindingMismatch)?;
    validate_freeze_receipt(manifest, &freeze.manifest_commitment, freeze)
        .map_err(|_| ObserverFitnessError::FreezeBindingMismatch)?;

    let plan = &bundle.plan;
    if plan.schema != FITNESS_PLAN_SCHEMA {
        return Err(ObserverFitnessError::UnsupportedPlanSchema);
    }
    if !valid_id(&plan.plan_id)
        || !valid_id(&plan.run_id)
        || !valid_id(&plan.runtime_observer_source_id)
        || !valid_id(&plan.runtime_observer_control_domain)
        || !valid_id(&plan.probe_injector_id)
        || !valid_id(&plan.probe_injector_control_domain)
        || !valid_id(&plan.fitness_verifier_id)
        || !valid_id(&plan.fitness_verifier_control_domain)
    {
        return Err(ObserverFitnessError::InvalidIdentity);
    }
    if plan.run_id != manifest.run_id
        || plan.manifest_commitment != freeze.manifest_commitment
        || plan.plan_digest != manifest.observer_fitness_plan_digest
        || plan.runtime_observer_source_id != manifest.runtime_observer_source_id
        || plan.runtime_observer_control_domain != manifest.runtime_observer_control_domain
        || plan.runtime_observer_adapter_digest != manifest.runtime_observer_adapter_digest
        || plan.runtime_kernel_profile_digest != manifest.runtime_kernel_profile_digest
        || plan.execution_environment_digest != manifest.execution_environment_digest
        || plan.state_isolation_policy_digest != manifest.state_isolation_policy_digest
        || plan.artifact_access_policy_digest != manifest.artifact_access_policy_digest
        || plan.network_egress_policy_digest != manifest.network_egress_policy_digest
        || plan.probe_injector_id != manifest.observer_probe_injector_id
        || plan.probe_injector_control_domain != manifest.observer_probe_injector_control_domain
        || plan.fitness_verifier_id != manifest.observer_fitness_verifier_id
        || plan.fitness_verifier_control_domain != manifest.observer_fitness_verifier_control_domain
    {
        return Err(ObserverFitnessError::ManifestBindingMismatch);
    }
    if plan.created_at_ms > manifest.frozen_at_ms
        || is_zero(&plan.plan_digest)
        || is_zero(&plan.runtime_kernel_profile_digest)
    {
        return Err(ObserverFitnessError::ManifestBindingMismatch);
    }
    validate_trust_domains(manifest, plan)?;

    let mut definition_index: BTreeMap<&str, &ProbeDefinition> = BTreeMap::new();
    let mut positive_surfaces = BTreeSet::new();
    let mut negative_surfaces = BTreeSet::new();
    let mut has_loss_probe = false;
    for definition in &plan.probes {
        if !valid_id(&definition.probe_id)
            || definition.expected_event_class.trim().is_empty()
            || definition.expected_event_class.len() > 256
        {
            return Err(ObserverFitnessError::InvalidIdentity);
        }
        if definition.maximum_detection_latency_ms == 0
            || definition.maximum_detection_latency_ms > FITNESS_MAX_PROBE_LATENCY_MS
        {
            return Err(ObserverFitnessError::InvalidProbeLatency(
                definition.probe_id.clone(),
            ));
        }
        if definition.expectation == ProbeExpectation::MustObserve {
            positive_surfaces.insert(definition.surface);
        }
        if definition.expectation == ProbeExpectation::MustNotObserve {
            negative_surfaces.insert(definition.surface);
        }
        if definition.expectation == ProbeExpectation::MustDetectLoss {
            if definition.surface != ProbeSurface::EventStreamLoss {
                return Err(ObserverFitnessError::InvalidOperationForExpectation(
                    definition.probe_id.clone(),
                ));
            }
            has_loss_probe = true;
        }
        if definition_index
            .insert(definition.probe_id.as_str(), definition)
            .is_some()
        {
            return Err(ObserverFitnessError::DuplicateProbeId(
                definition.probe_id.clone(),
            ));
        }
    }
    if definition_index.is_empty() {
        return Err(ObserverFitnessError::EmptyProbePlan);
    }
    for surface in [
        ProbeSurface::ProcessExec,
        ProbeSurface::FileRead,
        ProbeSurface::ObjectRead,
        ProbeSurface::OracleAccess,
        ProbeSurface::NetworkConnect,
        ProbeSurface::DnsLookup,
        ProbeSurface::NamespaceChange,
        ProbeSurface::CgroupMembership,
        ProbeSurface::ObserverHeartbeat,
    ] {
        if !positive_surfaces.contains(&surface) {
            return Err(ObserverFitnessError::MissingRequiredPositiveControl(surface));
        }
        if !negative_surfaces.contains(&surface) {
            return Err(ObserverFitnessError::MissingRequiredNegativeControl(surface));
        }
    }
    if !has_loss_probe {
        return Err(ObserverFitnessError::MissingLossControl);
    }
    if bundle.batches.is_empty() {
        return Err(ObserverFitnessError::MissingBatch);
    }

    let mut batch_ids = BTreeSet::new();
    let mut receipt_digests = BTreeSet::new();
    let mut batch_nonces = BTreeSet::new();
    let mut calibration_identities = BTreeSet::new();
    let mut all_markers = BTreeSet::new();
    let mut batches = Vec::with_capacity(bundle.batches.len());
    let mut total_positive = 0usize;
    let mut total_negative = 0usize;
    let mut total_loss = 0usize;

    for batch in &bundle.batches {
        let receipt = &batch.receipt;
        if receipt.schema != FITNESS_RECEIPT_SCHEMA {
            return Err(ObserverFitnessError::UnsupportedReceiptSchema);
        }
        if !valid_id(&receipt.batch_id)
            || receipt.run_id != manifest.run_id
            || receipt.manifest_commitment != freeze.manifest_commitment
            || receipt.plan_digest != plan.plan_digest
            || receipt.runtime_observer_source_id != manifest.runtime_observer_source_id
            || receipt.runtime_observer_control_domain != manifest.runtime_observer_control_domain
            || receipt.runtime_observer_adapter_digest != manifest.runtime_observer_adapter_digest
            || receipt.runtime_kernel_profile_digest != manifest.runtime_kernel_profile_digest
            || receipt.execution_environment_digest != manifest.execution_environment_digest
            || receipt.state_isolation_policy_digest != manifest.state_isolation_policy_digest
            || receipt.artifact_access_policy_digest != manifest.artifact_access_policy_digest
            || receipt.network_egress_policy_digest != manifest.network_egress_policy_digest
            || receipt.probe_injector_id != manifest.observer_probe_injector_id
            || receipt.probe_injector_control_domain != manifest.observer_probe_injector_control_domain
            || receipt.fitness_verifier_id != manifest.observer_fitness_verifier_id
            || receipt.fitness_verifier_control_domain != manifest.observer_fitness_verifier_control_domain
        {
            return Err(ObserverFitnessError::FitnessReceiptBindingMismatch(
                receipt.batch_id.clone(),
            ));
        }
        if is_zero(&receipt.receipt_digest)
            || !receipt_digests.insert(receipt.receipt_digest.clone())
            || is_zero(&receipt.batch_nonce_digest)
            || !batch_nonces.insert(receipt.batch_nonce_digest.clone())
            || !batch_ids.insert(receipt.batch_id.as_str())
        {
            return Err(ObserverFitnessError::ZeroOrDuplicateDigest);
        }
        if is_zero(&receipt.calibration_runtime_digest)
            || is_zero(&receipt.calibration_namespace_digest)
            || receipt.calibration_runtime_digest == receipt.calibration_namespace_digest
            || !calibration_identities.insert(receipt.calibration_runtime_digest.clone())
            || !calibration_identities.insert(receipt.calibration_namespace_digest.clone())
        {
            return Err(ObserverFitnessError::RuntimeNamespaceCollision);
        }
        if receipt.coverage != FitnessCoverage::Complete {
            return Err(ObserverFitnessError::IncompleteFitnessCoverage(receipt.coverage));
        }
        if receipt.unexplained_drop_count != 0 || receipt.unexplained_sequence_gap_count != 0 {
            return Err(ObserverFitnessError::UnexplainedTelemetryLoss);
        }
        if receipt.clock_uncertainty_ms > FITNESS_MAX_CLOCK_UNCERTAINTY_MS
            || receipt.measurement_start_ms >= receipt.measurement_end_ms
            || receipt.issued_at_ms < receipt.measurement_end_ms
            || receipt.valid_from_ms != receipt.issued_at_ms
            || receipt.expires_at_ms <= receipt.valid_from_ms
        {
            return Err(ObserverFitnessError::InvalidFitnessInterval);
        }
        if receipt.issued_at_ms <= manifest.frozen_at_ms {
            return Err(ObserverFitnessError::FitnessReceiptIssuedBeforeMeasurement);
        }
        if receipt.expires_at_ms.saturating_sub(receipt.valid_from_ms)
            > FITNESS_MAX_RECEIPT_TTL_MS
        {
            return Err(ObserverFitnessError::FitnessReceiptLifetimeExceeded);
        }

        let mut injection_index: BTreeMap<&str, &ProbeInjectionReceipt> = BTreeMap::new();
        let mut markers = BTreeSet::new();
        let mut injection_sequences = BTreeSet::new();
        for injection in &batch.injections {
            if injection.schema != FITNESS_INJECTION_SCHEMA {
                return Err(ObserverFitnessError::UnsupportedInjectionSchema);
            }
            let definition = definition_index
                .get(injection.probe_id.as_str())
                .copied()
                .ok_or_else(|| ObserverFitnessError::UnplannedInjection(
                    injection.probe_id.clone(),
                ))?;
            if injection.run_id != manifest.run_id
                || injection.batch_id != receipt.batch_id
                || injection.plan_digest != plan.plan_digest
                || injection.surface != definition.surface
                || injection.expectation != definition.expectation
                || injection.injector_id != manifest.observer_probe_injector_id
                || injection.injector_control_domain != manifest.observer_probe_injector_control_domain
                || injection.calibration_runtime_digest != receipt.calibration_runtime_digest
                || injection.calibration_namespace_digest != receipt.calibration_namespace_digest
                || injection.issued_at_ms <= manifest.frozen_at_ms
                || injection.issued_at_ms >= injection.executed_at_ms
                || injection.executed_at_ms > injection.completed_at_ms
                || injection.executed_at_ms < receipt.measurement_start_ms
                || injection.completed_at_ms > receipt.measurement_end_ms
                || !valid_id(&injection.injector_id)
                || is_zero(&injection.marker_digest)
                || !markers.insert(injection.marker_digest.clone())
                || !all_markers.insert(injection.marker_digest.clone())
            {
                return Err(ObserverFitnessError::InjectionBindingMismatch(
                    injection.probe_id.clone(),
                ));
            }
            let operation_valid = matches!(
                (definition.expectation, injection.operation),
                (ProbeExpectation::MustObserve, ProbeOperation::AllowedOperation)
                    | (ProbeExpectation::MustObserve, ProbeOperation::DeniedOperation)
                    | (ProbeExpectation::MustNotObserve, ProbeOperation::NegativeControl)
                    | (ProbeExpectation::MustDetectLoss, ProbeOperation::InducedEventLoss)
            );
            if !operation_valid {
                return Err(ObserverFitnessError::InvalidOperationForExpectation(
                    injection.probe_id.clone(),
                ));
            }
            if injection_sequences.insert(injection.sequence) == false || injection.sequence == 0 {
                return Err(ObserverFitnessError::DuplicateObserverSequence);
            }
            if injection_index
                .insert(injection.probe_id.as_str(), injection)
                .is_some()
            {
                return Err(ObserverFitnessError::DuplicateInjection(
                    injection.probe_id.clone(),
                ));
            }
        }
        for probe_id in definition_index.keys() {
            if !injection_index.contains_key(probe_id) {
                return Err(ObserverFitnessError::MissingInjection((*probe_id).to_owned()));
            }
        }
        if injection_index.len() != definition_index.len()
            || injection_sequences.len() != injection_index.len()
            || injection_sequences.iter().min() != Some(&1)
            || injection_sequences.iter().max() != Some(&(injection_sequences.len() as u64))
        {
            return Err(ObserverFitnessError::ObserverSequenceNotContiguous);
        }

        let mut observations_by_marker: BTreeMap<&Digest, Vec<&ObservedProbeEvent>> = BTreeMap::new();
        let mut losses_by_marker: BTreeMap<&Digest, Vec<&ObservedProbeLoss>> = BTreeMap::new();
        let mut observation_sequences = BTreeSet::new();
        let mut timed_events: Vec<(u64, u64)> = Vec::new();
        for observation in &batch.observations {
            let injection = injection_index
                .values()
                .find(|value| value.marker_digest == observation.marker_digest)
                .copied()
                .ok_or_else(|| ObserverFitnessError::UnexpectedObservation(
                    observation.probe_id.clone(),
                ))?;
            if observation.batch_id != receipt.batch_id
                || observation.probe_id != injection.probe_id
                || observation.surface != injection.surface
                || observation.calibration_runtime_digest != receipt.calibration_runtime_digest
                || observation.observer_source_id != manifest.runtime_observer_source_id
                || observation.observer_control_domain != manifest.runtime_observer_control_domain
                || observation.observed_at_ms < injection.completed_at_ms
                || observation.observed_at_ms > receipt.measurement_end_ms
            {
                return Err(ObserverFitnessError::ObservationRuntimeMismatch(
                    observation.probe_id.clone(),
                ));
            }
            if observation.sequence == 0 || !observation_sequences.insert(observation.sequence) {
                return Err(ObserverFitnessError::DuplicateObserverSequence);
            }
            observations_by_marker
                .entry(&observation.marker_digest)
                .or_default()
                .push(observation);
            timed_events.push((observation.sequence, observation.observed_at_ms));
        }
        for loss in &batch.loss_observations {
            let injection = injection_index
                .values()
                .find(|value| value.marker_digest == loss.marker_digest)
                .copied()
                .ok_or_else(|| ObserverFitnessError::UnexpectedLossObservation(
                    loss.probe_id.clone(),
                ))?;
            if loss.batch_id != receipt.batch_id
                || loss.probe_id != injection.probe_id
                || loss.calibration_runtime_digest != receipt.calibration_runtime_digest
                || loss.observer_source_id != manifest.runtime_observer_source_id
                || loss.observer_control_domain != manifest.runtime_observer_control_domain
                || loss.observed_at_ms < injection.completed_at_ms
                || loss.observed_at_ms > receipt.measurement_end_ms
            {
                return Err(ObserverFitnessError::LossRuntimeMismatch(loss.probe_id.clone()));
            }
            if loss.dropped_event_count == 0 && loss.sequence_gap_count == 0 {
                return Err(ObserverFitnessError::LossEvidenceMissing(loss.probe_id.clone()));
            }
            if loss.sequence == 0 || !observation_sequences.insert(loss.sequence) {
                return Err(ObserverFitnessError::DuplicateObserverSequence);
            }
            losses_by_marker
                .entry(&loss.marker_digest)
                .or_default()
                .push(loss);
            timed_events.push((loss.sequence, loss.observed_at_ms));
        }

        let mut positive = 0usize;
        let mut negative = 0usize;
        let mut loss_count = 0usize;
        let mut max_latency = 0u64;
        for definition in &plan.probes {
            let injection = injection_index[definition.probe_id.as_str()];
            let observed = observations_by_marker
                .get(&injection.marker_digest)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let losses = losses_by_marker
                .get(&injection.marker_digest)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            match definition.expectation {
                ProbeExpectation::MustObserve => {
                    if !losses.is_empty() {
                        return Err(ObserverFitnessError::UnexpectedLossObservation(
                            definition.probe_id.clone(),
                        ));
                    }
                    if observed.is_empty() {
                        return Err(ObserverFitnessError::MissingExpectedObservation(
                            definition.probe_id.clone(),
                        ));
                    }
                    if observed.len() != 1 {
                        return Err(ObserverFitnessError::DuplicateObservation(
                            definition.probe_id.clone(),
                        ));
                    }
                    let event = observed[0];
                    if event.event_class != definition.expected_event_class {
                        return Err(ObserverFitnessError::WrongObservedEventClass(
                            definition.probe_id.clone(),
                        ));
                    }
                    let latency = event.observed_at_ms.saturating_sub(injection.completed_at_ms);
                    if latency > definition.maximum_detection_latency_ms {
                        return Err(ObserverFitnessError::DetectionLatencyExceeded(
                            definition.probe_id.clone(),
                        ));
                    }
                    max_latency = max_latency.max(latency);
                    positive += 1;
                }
                ProbeExpectation::MustNotObserve => {
                    if !observed.is_empty() {
                        return Err(ObserverFitnessError::UnexpectedObservation(
                            definition.probe_id.clone(),
                        ));
                    }
                    if !losses.is_empty() {
                        return Err(ObserverFitnessError::UnexpectedLossObservation(
                            definition.probe_id.clone(),
                        ));
                    }
                    negative += 1;
                }
                ProbeExpectation::MustDetectLoss => {
                    if !observed.is_empty() {
                        return Err(ObserverFitnessError::UnexpectedObservation(
                            definition.probe_id.clone(),
                        ));
                    }
                    if losses.len() != 1 {
                        return Err(if losses.is_empty() {
                            ObserverFitnessError::MissingLossObservation(definition.probe_id.clone())
                        } else {
                            ObserverFitnessError::DuplicateLossObservation(definition.probe_id.clone())
                        });
                    }
                    let loss = losses[0];
                    let latency = loss.observed_at_ms.saturating_sub(injection.completed_at_ms);
                    if latency > definition.maximum_detection_latency_ms {
                        return Err(ObserverFitnessError::DetectionLatencyExceeded(
                            definition.probe_id.clone(),
                        ));
                    }
                    max_latency = max_latency.max(latency);
                    loss_count += 1;
                }
            }
        }

        if observation_sequences.len() != timed_events.len()
            || timed_events.len() != batch.observations.len() + batch.loss_observations.len()
        {
            return Err(ObserverFitnessError::DuplicateObserverSequence);
        }
        timed_events.sort_unstable_by_key(|item| item.0);
        if timed_events.first().map(|item| item.0) != Some(1)
            || timed_events.windows(2).any(|pair| pair[1].0 != pair[0].0.saturating_add(1))
        {
            return Err(ObserverFitnessError::ObserverSequenceNotContiguous);
        }
        if timed_events.windows(2).any(|pair| {
            pair[0].1.saturating_add(receipt.clock_uncertainty_ms)
                < pair[1].1.saturating_sub(receipt.clock_uncertainty_ms)
                && pair[0].0 >= pair[1].0
        }) {
            return Err(ObserverFitnessError::ObserverSequenceOrderInvalid);
        }
        total_positive = total_positive.checked_add(positive)
            .ok_or(ObserverFitnessError::ArithmeticOverflow)?;
        total_negative = total_negative.checked_add(negative)
            .ok_or(ObserverFitnessError::ArithmeticOverflow)?;
        total_loss = total_loss.checked_add(loss_count)
            .ok_or(ObserverFitnessError::ArithmeticOverflow)?;
        batches.push(ObserverFitnessBatchSummary {
            receipt_digest: receipt.receipt_digest.clone(),
            batch_id: receipt.batch_id.clone(),
            calibration_runtime_digest: receipt.calibration_runtime_digest.clone(),
            calibration_namespace_digest: receipt.calibration_namespace_digest.clone(),
            valid_from_ms: receipt.valid_from_ms,
            expires_at_ms: receipt.expires_at_ms,
            positive_controls: positive,
            negative_controls: negative,
            loss_controls: loss_count,
            maximum_detection_latency_ms: max_latency,
            observer_sequence_count: timed_events.len(),
        });
    }

    Ok(ObserverFitnessSummary {
        run_id: manifest.run_id.clone(),
        plan_digest: plan.plan_digest.clone(),
        batches,
        total_positive_controls: total_positive,
        total_negative_controls: total_negative,
        total_loss_controls: total_loss,
    })
}

fn validate_trust_domains(
    manifest: &RunManifest,
    plan: &ObserverFitnessPlan,
) -> Result<(), ObserverFitnessError> {
    let domains = [
        manifest.agent.control_domain.as_str(),
        manifest.baseline.control_domain.as_str(),
        manifest.freeze_authority_control_domain.as_str(),
        manifest.evaluator_control_domain.as_str(),
        manifest.evidence_control_domain.as_str(),
        manifest.runtime_observer_control_domain.as_str(),
        manifest.runtime_challenge_authority_control_domain.as_str(),
        manifest.observer_probe_injector_control_domain.as_str(),
        manifest.observer_fitness_verifier_control_domain.as_str(),
    ];
    let unique: BTreeSet<&str> = domains.iter().copied().collect();
    if unique.len() != domains.len()
        || plan.runtime_observer_source_id == plan.probe_injector_id
        || plan.runtime_observer_source_id == plan.fitness_verifier_id
        || plan.probe_injector_id == plan.fitness_verifier_id
    {
        return Err(ObserverFitnessError::InvalidTrustDomainSeparation);
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 256
        && !value.chars().any(char::is_control)
}

fn is_zero(digest: &Digest) -> bool {
    digest.0.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(value: u8) -> Digest {
        Digest::from_byte(value)
    }

    fn definitions() -> Vec<ProbeDefinition> {
        let surfaces = [
            ProbeSurface::ProcessExec,
            ProbeSurface::FileRead,
            ProbeSurface::ObjectRead,
            ProbeSurface::OracleAccess,
            ProbeSurface::NetworkConnect,
            ProbeSurface::DnsLookup,
            ProbeSurface::NamespaceChange,
            ProbeSurface::CgroupMembership,
            ProbeSurface::ObserverHeartbeat,
        ];
        let mut probes = Vec::new();
        let mut index = 0usize;
        for surface in surfaces {
            for expectation in [ProbeExpectation::MustObserve, ProbeExpectation::MustNotObserve] {
                index += 1;
                probes.push(ProbeDefinition {
                    probe_id: format!("probe-{index}"),
                    surface,
                    expectation,
                    expected_event_class: format!("event-{surface:?}"),
                    maximum_detection_latency_ms: 50,
                });
            }
        }
        index += 1;
        probes.push(ProbeDefinition {
            probe_id: format!("probe-{index}"),
            surface: ProbeSurface::EventStreamLoss,
            expectation: ProbeExpectation::MustDetectLoss,
            expected_event_class: "loss-detected".into(),
            maximum_detection_latency_ms: 50,
        });
        probes
    }

    pub fn fixture(
        manifest: &RunManifest,
        freeze: &FreezeReceipt,
    ) -> ObserverFitnessBundle {
        let probes = definitions();
        let plan = ObserverFitnessPlan {
            schema: FITNESS_PLAN_SCHEMA.into(),
            plan_id: "fitness-plan-001".into(),
            run_id: manifest.run_id.clone(),
            manifest_commitment: freeze.manifest_commitment.clone(),
            plan_digest: manifest.observer_fitness_plan_digest.clone(),
            created_at_ms: 15,
            runtime_observer_source_id: manifest.runtime_observer_source_id.clone(),
            runtime_observer_control_domain: manifest.runtime_observer_control_domain.clone(),
            runtime_observer_adapter_digest: manifest.runtime_observer_adapter_digest.clone(),
            runtime_kernel_profile_digest: manifest.runtime_kernel_profile_digest.clone(),
            execution_environment_digest: manifest.execution_environment_digest.clone(),
            state_isolation_policy_digest: manifest.state_isolation_policy_digest.clone(),
            artifact_access_policy_digest: manifest.artifact_access_policy_digest.clone(),
            network_egress_policy_digest: manifest.network_egress_policy_digest.clone(),
            probe_injector_id: manifest.observer_probe_injector_id.clone(),
            probe_injector_control_domain: manifest.observer_probe_injector_control_domain.clone(),
            fitness_verifier_id: manifest.observer_fitness_verifier_id.clone(),
            fitness_verifier_control_domain: manifest.observer_fitness_verifier_control_domain.clone(),
            probes: probes.clone(),
        };
        let batch_id = "fitness-batch-001".to_owned();
        let runtime = d(200);
        let namespace = d(201);
        let mut injections = Vec::new();
        let mut observations = Vec::new();
        let mut loss_observations = Vec::new();
        let mut seq = 1u64;
        for (index, probe) in probes.iter().enumerate() {
            let marker = d((index + 1) as u8);
            let operation = match probe.expectation {
                ProbeExpectation::MustObserve => ProbeOperation::AllowedOperation,
                ProbeExpectation::MustNotObserve => ProbeOperation::NegativeControl,
                ProbeExpectation::MustDetectLoss => ProbeOperation::InducedEventLoss,
            };
            injections.push(ProbeInjectionReceipt {
                schema: FITNESS_INJECTION_SCHEMA.into(),
                run_id: manifest.run_id.clone(),
                batch_id: batch_id.clone(),
                plan_digest: plan.plan_digest.clone(),
                probe_id: probe.probe_id.clone(),
                marker_digest: marker.clone(),
                surface: probe.surface,
                expectation: probe.expectation,
                operation,
                calibration_runtime_digest: runtime.clone(),
                calibration_namespace_digest: namespace.clone(),
                issued_at_ms: 21,
                executed_at_ms: 22,
                completed_at_ms: 23,
                injector_id: plan.probe_injector_id.clone(),
                injector_control_domain: plan.probe_injector_control_domain.clone(),
                sequence: index as u64 + 1,
            });
            match probe.expectation {
                ProbeExpectation::MustObserve => {
                    observations.push(ObservedProbeEvent {
                        batch_id: batch_id.clone(),
                        probe_id: probe.probe_id.clone(),
                        marker_digest: marker,
                        surface: probe.surface,
                        event_class: probe.expected_event_class.clone(),
                        calibration_runtime_digest: runtime.clone(),
                        observed_at_ms: 24,
                        observer_source_id: plan.runtime_observer_source_id.clone(),
                        observer_control_domain: plan.runtime_observer_control_domain.clone(),
                        sequence: seq,
                    });
                    seq += 1;
                }
                ProbeExpectation::MustNotObserve => {}
                ProbeExpectation::MustDetectLoss => {
                    loss_observations.push(ObservedProbeLoss {
                        batch_id: batch_id.clone(),
                        probe_id: probe.probe_id.clone(),
                        marker_digest: marker,
                        calibration_runtime_digest: runtime.clone(),
                        observed_at_ms: 24,
                        dropped_event_count: 1,
                        sequence_gap_count: 1,
                        observer_source_id: plan.runtime_observer_source_id.clone(),
                        observer_control_domain: plan.runtime_observer_control_domain.clone(),
                        sequence: seq,
                    });
                    seq += 1;
                }
            }
        }
        let receipt = ObserverFitnessReceipt {
            schema: FITNESS_RECEIPT_SCHEMA.into(),
            receipt_digest: d(220),
            batch_nonce_digest: d(221),
            batch_id,
            run_id: manifest.run_id.clone(),
            manifest_commitment: freeze.manifest_commitment.clone(),
            plan_digest: plan.plan_digest.clone(),
            runtime_observer_source_id: plan.runtime_observer_source_id.clone(),
            runtime_observer_control_domain: plan.runtime_observer_control_domain.clone(),
            runtime_observer_adapter_digest: plan.runtime_observer_adapter_digest.clone(),
            runtime_kernel_profile_digest: plan.runtime_kernel_profile_digest.clone(),
            execution_environment_digest: plan.execution_environment_digest.clone(),
            state_isolation_policy_digest: plan.state_isolation_policy_digest.clone(),
            artifact_access_policy_digest: plan.artifact_access_policy_digest.clone(),
            network_egress_policy_digest: plan.network_egress_policy_digest.clone(),
            probe_injector_id: plan.probe_injector_id.clone(),
            probe_injector_control_domain: plan.probe_injector_control_domain.clone(),
            fitness_verifier_id: plan.fitness_verifier_id.clone(),
            fitness_verifier_control_domain: plan.fitness_verifier_control_domain.clone(),
            calibration_runtime_digest: runtime,
            calibration_namespace_digest: namespace,
            measurement_start_ms: 20,
            measurement_end_ms: 25,
            issued_at_ms: 30,
            valid_from_ms: 30,
            expires_at_ms: 3_600_030,
            clock_uncertainty_ms: 5,
            coverage: FitnessCoverage::Complete,
            unexplained_drop_count: 0,
            unexplained_sequence_gap_count: 0,
            injection_bundle_digest: d(222),
            observation_bundle_digest: d(223),
            loss_bundle_digest: d(224),
        };
        ObserverFitnessBundle {
            plan,
            batches: vec![ObserverFitnessBatch {
                receipt,
                injections,
                observations,
                loss_observations,
            }],
        }
    }

    #[test]
    fn clean_positive_negative_and_loss_controls_qualify_observer() {
        let (manifest, freeze) = crate::sealed::tests::runtime_manifest_fixture();
        let bundle = fixture(&manifest, &freeze);
        let result = validate_observer_fitness_bundle(&manifest, &freeze, &bundle).unwrap();
        assert_eq!(result.total_positive_controls, 9);
        assert_eq!(result.total_negative_controls, 9);
        assert_eq!(result.total_loss_controls, 1);
        assert_eq!(result.batches.len(), 1);
    }

    #[test]
    fn missing_positive_marker_is_a_false_negative_and_fails() {
        let (manifest, freeze) = crate::sealed::tests::runtime_manifest_fixture();
        let mut bundle = fixture(&manifest, &freeze);
        bundle.batches[0].observations.remove(0);
        assert!(matches!(
            validate_observer_fitness_bundle(&manifest, &freeze, &bundle),
            Err(ObserverFitnessError::MissingExpectedObservation(_))
        ));
    }

    #[test]
    fn negative_control_marker_is_a_false_positive_and_fails() {
        let (manifest, freeze) = crate::sealed::tests::runtime_manifest_fixture();
        let mut bundle = fixture(&manifest, &freeze);
        let negative = bundle.batches[0].injections.iter()
            .find(|injection| injection.expectation == ProbeExpectation::MustNotObserve)
            .unwrap().clone();
        bundle.batches[0].observations.push(ObservedProbeEvent {
            batch_id: negative.batch_id,
            probe_id: negative.probe_id,
            marker_digest: negative.marker_digest,
            surface: negative.surface,
            event_class: "unexpected".into(),
            calibration_runtime_digest: negative.calibration_runtime_digest,
            observed_at_ms: 24,
            observer_source_id: manifest.runtime_observer_source_id.clone(),
            observer_control_domain: manifest.runtime_observer_control_domain.clone(),
            sequence: 20,
        });
        assert!(matches!(
            validate_observer_fitness_bundle(&manifest, &freeze, &bundle),
            Err(ObserverFitnessError::UnexpectedObservation(_))
        ));
    }

    #[test]
    fn event_loss_must_be_reported_as_loss_not_as_clean_absence() {
        let (manifest, freeze) = crate::sealed::tests::runtime_manifest_fixture();
        let mut bundle = fixture(&manifest, &freeze);
        bundle.batches[0].loss_observations.clear();
        assert!(matches!(
            validate_observer_fitness_bundle(&manifest, &freeze, &bundle),
            Err(ObserverFitnessError::MissingLossObservation(_))
        ));
    }

    #[test]
    fn event_class_marker_latency_and_identity_are_all_pinned() {
        let (manifest, freeze) = crate::sealed::tests::runtime_manifest_fixture();
        let mut bundle = fixture(&manifest, &freeze);
        bundle.batches[0].observations[0].event_class = "wrong-class".into();
        assert!(matches!(
            validate_observer_fitness_bundle(&manifest, &freeze, &bundle),
            Err(ObserverFitnessError::WrongObservedEventClass(_))
        ));
    }

    #[test]
    fn observer_fitness_expiry_is_short_and_receipt_is_post_measurement() {
        let (manifest, freeze) = crate::sealed::tests::runtime_manifest_fixture();
        let mut bundle = fixture(&manifest, &freeze);
        bundle.batches[0].receipt.expires_at_ms += FITNESS_MAX_RECEIPT_TTL_MS + 1;
        assert_eq!(
            validate_observer_fitness_bundle(&manifest, &freeze, &bundle),
            Err(ObserverFitnessError::FitnessReceiptLifetimeExceeded)
        );
    }

    #[test]
    fn cross_batch_marker_reuse_and_runtime_aliasing_are_rejected() {
        let (manifest, freeze) = crate::sealed::tests::runtime_manifest_fixture();
        let mut bundle = fixture(&manifest, &freeze);
        let mut duplicate = bundle.batches[0].clone();
        duplicate.receipt.batch_id = "batch-002".into();
        duplicate.receipt.receipt_digest = d(230);
        duplicate.receipt.batch_nonce_digest = d(231);
        duplicate.injections[0].batch_id = "batch-002".into();
        duplicate.injections[0].marker_digest = bundle.batches[0].injections[0].marker_digest.clone();
        bundle.batches.push(duplicate);
        assert!(matches!(
            validate_observer_fitness_bundle(&manifest, &freeze, &bundle),
            Err(ObserverFitnessError::ZeroOrDuplicateDigest)
        ));
    }

    #[test]
    fn verifier_injector_and_observer_domains_must_be_distinct() {
        let (manifest, freeze) = crate::sealed::tests::runtime_manifest_fixture();
        let mut bundle = fixture(&manifest, &freeze);
        bundle.plan.fitness_verifier_control_domain =
            bundle.plan.runtime_observer_control_domain.clone();
        assert_eq!(
            validate_observer_fitness_bundle(&manifest, &freeze, &bundle),
            Err(ObserverFitnessError::ManifestBindingMismatch)
        );
    }
}
