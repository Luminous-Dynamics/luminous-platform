//! Independent known-answer calibration for runtime observers.
//!
//! This evaluator compares an evaluator-private expected probe set with source-bound
//! injector and observer receipts. It does not authenticate signatures itself; the
//! existing typed evidence adapter must authenticate challenge, injection and
//! detection statements before this module accepts them.

use crate::runtime::{
    validate_runtime_attestations_from_events, RuntimeAttestationError, RuntimeTrialAttestation,
};
use crate::sealed::{
    EvaluationArm, FreezeReceipt, RunManifest, ScenarioReceipt, ScenarioReveal, TrialEvent,
};
use crate::Digest;
use std::collections::{BTreeMap, BTreeSet};

pub const OBSERVER_CALIBRATION_SCHEMA: &str = "continuous-opsbench-observer-calibration-v0.1";
pub const MAX_PROBES_PER_ARM: usize = 4_096;
pub const MAX_CALIBRATION_CHALLENGE_TTL_MS: u64 = 3_600_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProbeChannel {
    ProcessLifecycle,
    ExecutableLoad,
    FileObjectAccess,
    MountNamespace,
    PolicyDeniedOperation,
    ResourceControl,
    DnsEgress,
    CrossTrialState,
    TelemetryPipeline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpectedPolicyDecision {
    Allowed,
    Forbidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservedPolicyDecision {
    Allowed,
    Forbidden,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeInjectionStatus {
    Injected,
    NotInjected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeObservation {
    Detected {
        channel: ProbeChannel,
        target_digest: Digest,
        decision: ObservedPolicyDecision,
        detected_at_ms: u64,
        event_sequence: u64,
        event_commitment: Digest,
    },
    NotDetected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedProbe {
    /// Keyed opaque probe identifier; raw probe names and paths stay evaluator-private.
    pub probe_id_digest: Digest,
    pub channel: ProbeChannel,
    pub expected_decision: ExpectedPolicyDecision,
    pub target_digest: Digest,
    /// Independent fault-schedule/topology cluster; probes within one cluster are not independent samples.
    pub probe_cluster_digest: Digest,
    pub maximum_detection_latency_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObserverCalibrationPlan {
    pub schema: String,
    pub run_id: String,
    pub trial_id: String,
    pub arm: EvaluationArm,
    pub manifest_commitment: Digest,
    pub scenario_commitment: Digest,
    pub arm_identity_digest: Digest,
    pub runtime_instance_digest: Digest,
    pub observer_source_id: String,
    pub observer_control_domain: String,
    pub observer_adapter_digest: Digest,
    pub challenge_authority_id: String,
    pub challenge_authority_control_domain: String,
    /// Fresh, one-use calibration challenge distinct from the runtime-attestation nonce.
    pub challenge_id_digest: Digest,
    pub probe_suite_digest: Digest,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub expected_probes: Vec<ExpectedProbe>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeInjectionReceipt {
    pub schema: String,
    pub run_id: String,
    pub trial_id: String,
    pub arm: EvaluationArm,
    pub challenge_id_digest: Digest,
    pub probe_id_digest: Digest,
    pub runtime_instance_digest: Digest,
    pub channel: ProbeChannel,
    pub target_digest: Digest,
    pub status: ProbeInjectionStatus,
    pub injected_at_ms: u64,
    pub injector_clock_uncertainty_ms: u64,
    pub injection_event_commitment: Digest,
    pub injector_source_id: String,
    pub injector_control_domain: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeDetectionReceipt {
    pub schema: String,
    pub run_id: String,
    pub trial_id: String,
    pub arm: EvaluationArm,
    pub challenge_id_digest: Digest,
    pub probe_id_digest: Digest,
    pub runtime_instance_digest: Digest,
    pub observer_source_id: String,
    pub observer_control_domain: String,
    pub observer_adapter_digest: Digest,
    pub observation: ProbeObservation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelCalibrationSummary {
    pub channel: ProbeChannel,
    pub probes: usize,
    pub positive_controls: usize,
    pub negative_controls: usize,
    pub detections: usize,
    pub true_positives: usize,
    pub true_negatives: usize,
    pub unique_probe_clusters: usize,
    pub sensitivity_ppm: Option<u64>,
    pub specificity_ppm: Option<u64>,
    pub precision_ppm: Option<u64>,
    pub misses: usize,
    pub false_negatives: usize,
    pub false_positives: usize,
    pub unknown_classifications: usize,
    pub p50_detection_latency_ms: Option<u64>,
    pub p95_detection_latency_ms: Option<u64>,
    pub unique_probe_clusters: usize,
    pub sensitivity_ppm: Option<u64>,
    pub specificity_ppm: Option<u64>,
    pub precision_ppm: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationDisposition {
    Pass,
    Fail,
    Indeterminate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObserverCalibrationSummary {
    pub run_id: String,
    pub planned_arm_trials: usize,
    pub calibrated_arm_trials: usize,
    pub expected_probes: usize,
    pub injected_probes: usize,
    pub detected_probes: usize,
    pub misses: usize,
    pub false_negatives: usize,
    pub false_positives: usize,
    pub unknown_classifications: usize,
    pub over_latency_probes: usize,
    pub unique_probe_clusters: usize,
    pub sensitivity_ppm: Option<u64>,
    pub specificity_ppm: Option<u64>,
    pub precision_ppm: Option<u64>,
    pub p50_detection_latency_ms: Option<u64>,
    pub p95_detection_latency_ms: Option<u64>,
    pub disposition: CalibrationDisposition,
    pub channels: Vec<ChannelCalibrationSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObserverCalibrationError {
    RuntimeEvidenceInvalid(RuntimeAttestationError),
    UnsupportedSchema(String),
    EmptyChallengeOrSuiteDigest(String),
    InvalidIdentity(String),
    InvalidPlanInterval(String),
    ChallengeOutsideRuntimeWindow(String),
    ChallengeLifetimeExceeded(String),
    EmptyProbeSet(String),
    ProbeLimitExceeded(String),
    InvalidProbe(String),
    DuplicateChallenge(String),
    DuplicateProbe(String),
    MissingPlan(String, EvaluationArm),
    UnplannedPlan(String, EvaluationArm),
    DuplicatePlan(String, EvaluationArm),
    ChannelMissingPositiveControl(String, EvaluationArm, ProbeChannel),
    ChannelMissingNegativeControl(String, EvaluationArm, ProbeChannel),
    MissingInjection(String),
    UnplannedInjection(String),
    DuplicateInjection(String),
    InjectionBindingMismatch(String),
    ProbeInjectionUnavailable(String),
    MissingDetection(String),
    UnplannedDetection(String),
    DuplicateDetection(String),
    DetectionBindingMismatch(String),
    DetectionControlDomainCollision(String),
    InjectionTimeOutsideMeasurement(String),
    InjectorClockUncertaintyExceeded(String),
    DetectionBeforeInjection(String),
    DetectionOutsideMeasurement(String),
    DetectionEventMissing(String),
    DetectionSequenceDuplicate,
    DetectionSequenceOrderInvalid,
    ArithmeticOverflow,
}

#[derive(Default)]
struct ChannelAccumulator {
    probes: usize,
    positive_controls: usize,
    negative_controls: usize,
    detections: usize,
    misses: usize,
    false_negatives: usize,
    false_positives: usize,
    unknown_classifications: usize,
    latencies: Vec<u64>,
}

pub fn evaluate_observer_calibration(
    manifest: &RunManifest,
    freeze: &FreezeReceipt,
    scenarios: &[ScenarioReceipt],
    reveals: &[ScenarioReveal],
    trial_events: &[TrialEvent],
    runtime_attestations: &[RuntimeTrialAttestation],
    plans: &[ObserverCalibrationPlan],
    injections: &[ProbeInjectionReceipt],
    detections: &[ProbeDetectionReceipt],
) -> Result<ObserverCalibrationSummary, ObserverCalibrationError> {
    let runtime_summary = validate_runtime_attestations_from_events(
        manifest,
        freeze,
        scenarios,
        reveals,
        trial_events,
        runtime_attestations,
    )
    .map_err(ObserverCalibrationError::RuntimeEvidenceInvalid)?;

    let mut runtime_by_arm: BTreeMap<(&str, EvaluationArm), &RuntimeTrialAttestation> =
        BTreeMap::new();
    for receipt in runtime_attestations {
        if runtime_by_arm
            .insert((receipt.trial_id.as_str(), receipt.arm), receipt)
            .is_some()
        {
            return Err(ObserverCalibrationError::RuntimeEvidenceInvalid(
                RuntimeAttestationError::DuplicateAttestation(
                    receipt.trial_id.clone(),
                    receipt.arm,
                ),
            ));
        }
    }

    let mut plan_by_arm: BTreeMap<(&str, EvaluationArm), &ObserverCalibrationPlan> =
        BTreeMap::new();
    let mut challenge_ids = BTreeSet::new();
    let mut probe_keys: BTreeMap<(&str, EvaluationArm, Digest), &ExpectedProbe> =
        BTreeMap::new();
    let mut channel_accumulators: BTreeMap<ProbeChannel, ChannelAccumulator> = BTreeMap::new();
    let mut total_expected = 0usize;

    for plan in plans {
        if plan.schema != OBSERVER_CALIBRATION_SCHEMA {
            return Err(ObserverCalibrationError::UnsupportedSchema(
                plan.trial_id.clone(),
            ));
        }
        if !manifest.trials.iter().any(|trial| trial.trial_id == plan.trial_id) {
            return Err(ObserverCalibrationError::UnplannedPlan(
                plan.trial_id.clone(),
                plan.arm,
            ));
        }
        let runtime = runtime_by_arm
            .get(&(plan.trial_id.as_str(), plan.arm))
            .copied()
            .ok_or_else(|| ObserverCalibrationError::MissingPlan(
                plan.trial_id.clone(),
                plan.arm,
            ))?;
        validate_plan_binding(manifest, &runtime_summary, runtime, plan)?;
        if plan_by_arm
            .insert((plan.trial_id.as_str(), plan.arm), plan)
            .is_some()
        {
            return Err(ObserverCalibrationError::DuplicatePlan(
                plan.trial_id.clone(),
                plan.arm,
            ));
        }
        if !challenge_ids.insert(plan.challenge_id_digest.clone()) {
            return Err(ObserverCalibrationError::DuplicateChallenge(
                plan.trial_id.clone(),
            ));
        }
        if plan.expected_probes.is_empty() {
            return Err(ObserverCalibrationError::EmptyProbeSet(
                plan.trial_id.clone(),
            ));
        }
        if plan.expected_probes.len() > MAX_PROBES_PER_ARM {
            return Err(ObserverCalibrationError::ProbeLimitExceeded(
                plan.trial_id.clone(),
            ));
        }

        let mut positive_channels = BTreeSet::new();
        let mut negative_channels = BTreeSet::new();
        for probe in &plan.expected_probes {
            if is_zero(&probe.probe_id_digest)
                || is_zero(&probe.target_digest)
                || is_zero(&probe.probe_cluster_digest)
                || probe.maximum_detection_latency_ms == 0
            {
                return Err(ObserverCalibrationError::InvalidProbe(
                    plan.trial_id.clone(),
                ));
            }
            let key = (
                plan.trial_id.as_str(),
                plan.arm,
                probe.probe_id_digest.clone(),
            );
            if probe_keys.insert(key, probe).is_some() {
                return Err(ObserverCalibrationError::DuplicateProbe(
                    plan.trial_id.clone(),
                ));
            }
            match probe.expected_decision {
                ExpectedPolicyDecision::Forbidden => {
                    positive_channels.insert(probe.channel);
                }
                ExpectedPolicyDecision::Allowed => {
                    negative_channels.insert(probe.channel);
                }
            }
            let accumulator = channel_accumulators.entry(probe.channel).or_default();
            accumulator.probe_clusters.insert(probe.probe_cluster_digest.clone());
            accumulator.probes += 1;
            match probe.expected_decision {
                ExpectedPolicyDecision::Forbidden => accumulator.positive_controls += 1,
                ExpectedPolicyDecision::Allowed => accumulator.negative_controls += 1,
            }
            total_expected = total_expected
                .checked_add(1)
                .ok_or(ObserverCalibrationError::ArithmeticOverflow)?;
        }
        let channels: BTreeSet<ProbeChannel> = plan
            .expected_probes
            .iter()
            .map(|probe| probe.channel)
            .collect();
        for channel in channels {
            if !positive_channels.contains(&channel) {
                return Err(ObserverCalibrationError::ChannelMissingPositiveControl(
                    plan.trial_id.clone(),
                    plan.arm,
                    channel,
                ));
            }
            if !negative_channels.contains(&channel) {
                return Err(ObserverCalibrationError::ChannelMissingNegativeControl(
                    plan.trial_id.clone(),
                    plan.arm,
                    channel,
                ));
            }
        }
    }

    for trial in &manifest.trials {
        for arm in [EvaluationArm::Candidate, EvaluationArm::Baseline] {
            if !plan_by_arm.contains_key(&(trial.trial_id.as_str(), arm)) {
                return Err(ObserverCalibrationError::MissingPlan(
                    trial.trial_id.clone(),
                    arm,
                ));
            }
        }
    }
    if plans.len() != manifest.trials.len().saturating_mul(2) {
        return Err(ObserverCalibrationError::UnplannedPlan(
            "plan denominator mismatch".into(),
            EvaluationArm::Candidate,
        ));
    }

    let mut injections_by_probe: BTreeMap<(&str, EvaluationArm, Digest), &ProbeInjectionReceipt> =
        BTreeMap::new();
    for injection in injections {
        if injection.schema != OBSERVER_CALIBRATION_SCHEMA {
            return Err(ObserverCalibrationError::UnsupportedSchema(
                injection.trial_id.clone(),
            ));
        }
        let plan = plan_by_arm
            .get(&(injection.trial_id.as_str(), injection.arm))
            .copied()
            .ok_or_else(|| ObserverCalibrationError::UnplannedInjection(
                injection.trial_id.clone(),
            ))?;
        let expected = probe_keys
            .get(&(
                injection.trial_id.as_str(),
                injection.arm,
                injection.probe_id_digest.clone(),
            ))
            .copied()
            .ok_or_else(|| ObserverCalibrationError::UnplannedInjection(
                injection.trial_id.clone(),
            ))?;
        let runtime = runtime_by_arm[&(injection.trial_id.as_str(), injection.arm)];
        if injections_by_probe
            .insert((
                injection.trial_id.as_str(),
                injection.arm,
                injection.probe_id_digest.clone(),
            ), injection)
            .is_some()
        {
            return Err(ObserverCalibrationError::DuplicateInjection(
                injection.trial_id.clone(),
            ));
        }
        if injection.run_id != manifest.run_id
            || injection.challenge_id_digest != plan.challenge_id_digest
            || injection.runtime_instance_digest != runtime.runtime_instance_digest
            || injection.channel != expected.channel
            || injection.target_digest != expected.target_digest
            || injection.injector_source_id != manifest.runtime_challenge_authority_id
            || injection.injector_control_domain
                != manifest.runtime_challenge_authority_control_domain
            || is_zero(&injection.injection_event_commitment)
        {
            return Err(ObserverCalibrationError::InjectionBindingMismatch(
                injection.trial_id.clone(),
            ));
        }
        if injection.injector_clock_uncertainty_ms > crate::runtime::MAX_CLOCK_UNCERTAINTY_MS {
            return Err(ObserverCalibrationError::InjectorClockUncertaintyExceeded(
                injection.trial_id.clone(),
            ));
        }
        if injection.status != ProbeInjectionStatus::Injected {
            return Err(ObserverCalibrationError::ProbeInjectionUnavailable(
                injection.trial_id.clone(),
            ));
        }
        let injection_earliest = injection.injected_at_ms
            .saturating_sub(injection.injector_clock_uncertainty_ms);
        let injection_latest = injection.injected_at_ms
            .saturating_add(injection.injector_clock_uncertainty_ms);
        if injection_earliest
                < runtime.measurement_start_ms.saturating_add(runtime.clock_uncertainty_ms)
            || injection_latest
                > runtime.measurement_end_ms.saturating_sub(runtime.clock_uncertainty_ms)
        {
            return Err(ObserverCalibrationError::InjectionTimeOutsideMeasurement(
                injection.trial_id.clone(),
            ));
        }
        if injection.injected_at_ms < plan.issued_at_ms
            || injection.injected_at_ms > plan.expires_at_ms
        {
            return Err(ObserverCalibrationError::InjectionBindingMismatch(
                injection.trial_id.clone(),
            ));
        }
    }
    for ((trial_id, arm, probe_id), _) in &probe_keys {
        if !injections_by_probe.contains_key(&(*trial_id, *arm, probe_id.clone())) {
            return Err(ObserverCalibrationError::MissingInjection(
                (*trial_id).to_owned(),
            ));
        }
    }

    let mut detections_by_probe: BTreeMap<(&str, EvaluationArm, Digest), &ProbeDetectionReceipt> =
        BTreeMap::new();
    for detection in detections {
        if detection.schema != OBSERVER_CALIBRATION_SCHEMA {
            return Err(ObserverCalibrationError::UnsupportedSchema(
                detection.trial_id.clone(),
            ));
        }
        let plan = plan_by_arm
            .get(&(detection.trial_id.as_str(), detection.arm))
            .copied()
            .ok_or_else(|| ObserverCalibrationError::UnplannedDetection(
                detection.trial_id.clone(),
            ))?;
        if !probe_keys.contains_key(&(
            detection.trial_id.as_str(),
            detection.arm,
            detection.probe_id_digest.clone(),
        )) {
            return Err(ObserverCalibrationError::UnplannedDetection(
                detection.trial_id.clone(),
            ));
        }
        let key = (
            detection.trial_id.as_str(),
            detection.arm,
            detection.probe_id_digest.clone(),
        );
        if detections_by_probe.insert(key, detection).is_some() {
            return Err(ObserverCalibrationError::DuplicateDetection(
                detection.trial_id.clone(),
            ));
        }
        let runtime = runtime_by_arm[&(detection.trial_id.as_str(), detection.arm)];
        if detection.run_id != manifest.run_id
            || detection.challenge_id_digest != plan.challenge_id_digest
            || detection.runtime_instance_digest != runtime.runtime_instance_digest
            || detection.observer_source_id != manifest.runtime_observer_source_id
            || detection.observer_source_id == manifest.runtime_challenge_authority_id
            || detection.observer_control_domain != manifest.runtime_observer_control_domain
            || detection.observer_adapter_digest != manifest.runtime_observer_adapter_digest
        {
            return Err(ObserverCalibrationError::DetectionBindingMismatch(
                detection.trial_id.clone(),
            ));
        }
        if detection.observer_control_domain
            == manifest.runtime_challenge_authority_control_domain
            || detection.observer_control_domain == manifest.agent.control_domain
            || detection.observer_control_domain == manifest.baseline.control_domain
        {
            return Err(ObserverCalibrationError::DetectionControlDomainCollision(
                detection.trial_id.clone(),
            ));
        }
    }
    let mut observer_sequences = BTreeSet::new();
    let mut sequence_timestamps = Vec::new();
    let mut total_injected = 0usize;
    let mut total_detected = 0usize;
    let mut total_misses = 0usize;
    let mut total_false_negatives = 0usize;
    let mut total_false_positives = 0usize;
    let mut total_unknown = 0usize;
    let mut over_latency = 0usize;
    let mut all_latencies = Vec::new();
    let mut disposition = CalibrationDisposition::Pass;

    for injection in injections {
        total_injected += 1;
        let key = (
            injection.trial_id.as_str(),
            injection.arm,
            injection.probe_id_digest.clone(),
        );
        let expected = probe_keys[&key];
        let plan = plan_by_arm[&(injection.trial_id.as_str(), injection.arm)];
        let runtime = runtime_by_arm[&(injection.trial_id.as_str(), injection.arm)];
        let detection = detections_by_probe
            .get(&key)
            .copied()
            .ok_or_else(|| ObserverCalibrationError::MissingDetection(
                injection.trial_id.clone(),
            ))?;
        let injection_earliest = injection.injected_at_ms
            .saturating_sub(injection.injector_clock_uncertainty_ms);

        let accumulator = channel_accumulators.get_mut(&expected.channel).unwrap();
        match &detection.observation {
            ProbeObservation::NotDetected => {
                total_misses += 1;
                accumulator.misses += 1;
                if expected.expected_decision == ExpectedPolicyDecision::Forbidden {
                    total_false_negatives += 1;
                    accumulator.false_negatives += 1;
                }
                disposition = CalibrationDisposition::Fail;
            }
            ProbeObservation::Detected {
                channel,
                target_digest,
                decision,
                detected_at_ms,
                event_sequence,
                event_commitment,
            } => {
                if *channel != expected.channel
                    || *target_digest != expected.target_digest
                    || is_zero(event_commitment)
                    || *event_sequence == 0
                {
                    return Err(ObserverCalibrationError::DetectionEventMissing(
                        injection.trial_id.clone(),
                    ));
                }
                if !observer_sequences.insert(*event_sequence) {
                    return Err(ObserverCalibrationError::DetectionSequenceDuplicate);
                }
                let detection_earliest = detected_at_ms
                    .saturating_sub(runtime.clock_uncertainty_ms);
                let detection_latest = detected_at_ms
                    .saturating_add(runtime.clock_uncertainty_ms);
                if detection_latest < injection_earliest {
                    return Err(ObserverCalibrationError::DetectionBeforeInjection(
                        injection.trial_id.clone(),
                    ));
                }
                if detection_earliest
                        < runtime.measurement_start_ms.saturating_add(runtime.clock_uncertainty_ms)
                    || detection_latest
                        > runtime.measurement_end_ms.saturating_sub(runtime.clock_uncertainty_ms)
                {
                    return Err(ObserverCalibrationError::DetectionOutsideMeasurement(
                        injection.trial_id.clone(),
                    ));
                }
                // Conservative upper bound on latency: latest plausible detect time
                // minus earliest plausible injection time.
                let latency = detection_latest.saturating_sub(injection_earliest);
                total_detected += 1;
                accumulator.detections += 1;
                accumulator.latencies.push(latency);
                all_latencies.push(latency);
                if latency > expected.maximum_detection_latency_ms {
                    over_latency += 1;
                    disposition = CalibrationDisposition::Fail;
                }
                match (expected.expected_decision, *decision) {
                    (ExpectedPolicyDecision::Forbidden, ObservedPolicyDecision::Forbidden) => {
                        accumulator.true_positives += 1;
                    }
                    (ExpectedPolicyDecision::Allowed, ObservedPolicyDecision::Allowed) => {
                        accumulator.true_negatives += 1;
                    }
                    (ExpectedPolicyDecision::Allowed, ObservedPolicyDecision::Forbidden) => {
                        total_false_positives += 1;
                        accumulator.false_positives += 1;
                        disposition = CalibrationDisposition::Fail;
                    }
                    (ExpectedPolicyDecision::Forbidden, ObservedPolicyDecision::Allowed) => {
                        total_false_negatives += 1;
                        accumulator.false_negatives += 1;
                        disposition = CalibrationDisposition::Fail;
                    }
                    (_, ObservedPolicyDecision::Unknown) => {
                        total_unknown += 1;
                        accumulator.unknown_classifications += 1;
                        if expected.expected_decision == ExpectedPolicyDecision::Forbidden {
                            total_false_negatives += 1;
                            accumulator.false_negatives += 1;
                        }
                        if disposition == CalibrationDisposition::Pass {
                            disposition = CalibrationDisposition::Indeterminate;
                        }
                    }
                }
                sequence_timestamps.push((
                    *event_sequence,
                    *detected_at_ms,
                    runtime.clock_uncertainty_ms,
                ));
            }
        }
    }

    if detections.len() != injections.len() {
        return Err(ObserverCalibrationError::UnplannedDetection(
            "detection denominator mismatch".into(),
        ));
    }
    sequence_timestamps.sort_unstable_by_key(|entry| entry.0);
    if sequence_timestamps.windows(2).any(|pair| {
        pair[0].1.saturating_sub(pair[0].2)
            > pair[1].1.saturating_add(pair[1].2)
    }) {
        return Err(ObserverCalibrationError::DetectionSequenceOrderInvalid);
    }

    let mut channels = Vec::new();
    for (channel, accumulator) in &channel_accumulators {
        channels.push(ChannelCalibrationSummary {
            channel: *channel,
            probes: accumulator.probes,
            positive_controls: accumulator.positive_controls,
            negative_controls: accumulator.negative_controls,
            detections: accumulator.detections,
            true_positives: accumulator.true_positives,
            true_negatives: accumulator.true_negatives,
            unique_probe_clusters: accumulator.probe_clusters.len(),
            sensitivity_ppm: ratio_ppm(accumulator.true_positives, accumulator.positive_controls),
            specificity_ppm: ratio_ppm(accumulator.true_negatives, accumulator.negative_controls),
            precision_ppm: ratio_ppm(
                accumulator.true_positives,
                accumulator.true_positives.saturating_add(accumulator.false_positives),
            ),
            misses: accumulator.misses,
            false_negatives: accumulator.false_negatives,
            false_positives: accumulator.false_positives,
            unknown_classifications: accumulator.unknown_classifications,
            p50_detection_latency_ms: percentile(&accumulator.latencies, 50),
            p95_detection_latency_ms: percentile(&accumulator.latencies, 95),
        });
    }
    channels.sort_by_key(|summary| summary.channel);

    if disposition == CalibrationDisposition::Pass
        && (total_expected != total_injected || total_detected != total_expected)
    {
        disposition = CalibrationDisposition::Indeterminate;
    }

    Ok(ObserverCalibrationSummary {
        run_id: manifest.run_id.clone(),
        planned_arm_trials: manifest.trials.len().saturating_mul(2),
        calibrated_arm_trials: plans.len(),
        expected_probes: total_expected,
        injected_probes: total_injected,
        detected_probes: total_detected,
        misses: total_misses,
        false_negatives: total_false_negatives,
        false_positives: total_false_positives,
        unknown_classifications: total_unknown,
        over_latency_probes: over_latency,
        unique_probe_clusters: probe_keys.values()
            .map(|probe| probe.probe_cluster_digest.clone())
            .collect::<BTreeSet<_>>().len(),
        sensitivity_ppm: ratio_ppm(
            channel_accumulators.values().map(|a| a.true_positives).sum::<usize>(),
            channel_accumulators.values().map(|a| a.positive_controls).sum::<usize>(),
        ),
        specificity_ppm: ratio_ppm(
            channel_accumulators.values().map(|a| a.true_negatives).sum::<usize>(),
            channel_accumulators.values().map(|a| a.negative_controls).sum::<usize>(),
        ),
        precision_ppm: ratio_ppm(
            channel_accumulators.values().map(|a| a.true_positives).sum::<usize>(),
            channel_accumulators.values().map(|a| a.true_positives.saturating_add(a.false_positives)).sum::<usize>(),
        ),
        p50_detection_latency_ms: percentile(&all_latencies, 50),
        p95_detection_latency_ms: percentile(&all_latencies, 95),
        disposition,
        channels,
    })
}

fn validate_plan_binding(
    manifest: &RunManifest,
    runtime_summary: &crate::runtime::RuntimeAttestationSummary,
    runtime: &RuntimeTrialAttestation,
    plan: &ObserverCalibrationPlan,
) -> Result<(), ObserverCalibrationError> {
    let expected_identity = match plan.arm {
        EvaluationArm::Candidate => &manifest.agent_identity_digest,
        EvaluationArm::Baseline => &manifest.baseline_identity_digest,
    };
    if plan.run_id != manifest.run_id
        || plan.trial_id != runtime.trial_id
        || plan.arm != runtime.arm
        || is_zero(&plan.challenge_id_digest)
        || is_zero(&plan.probe_suite_digest)
        || plan.manifest_commitment != runtime.manifest_commitment
        || plan.scenario_commitment != runtime.scenario_commitment
        || plan.arm_identity_digest != *expected_identity
        || plan.runtime_instance_digest != runtime.runtime_instance_digest
        || plan.observer_source_id != manifest.runtime_observer_source_id
        || plan.observer_control_domain != manifest.runtime_observer_control_domain
        || plan.observer_adapter_digest != manifest.runtime_observer_adapter_digest
        || plan.challenge_authority_id != manifest.runtime_challenge_authority_id
        || plan.challenge_authority_control_domain
            != manifest.runtime_challenge_authority_control_domain
        || runtime_summary.runtime_observer_source_id != plan.observer_source_id
        || runtime_summary.runtime_observer_control_domain != plan.observer_control_domain
        || runtime_summary.runtime_observer_adapter_digest != plan.observer_adapter_digest
        || runtime_summary.runtime_challenge_authority_id != plan.challenge_authority_id
        || runtime_summary.runtime_challenge_authority_control_domain
            != plan.challenge_authority_control_domain
    {
        return Err(ObserverCalibrationError::InvalidIdentity(
            plan.trial_id.clone(),
        ));
    }
    if plan.issued_at_ms <= manifest.frozen_at_ms
        || plan.issued_at_ms <= runtime.challenge_issued_at_ms
        || plan.issued_at_ms >= runtime.measurement_start_ms
        || plan.expires_at_ms
            < runtime.measurement_end_ms.saturating_add(runtime.clock_uncertainty_ms)
        || plan.expires_at_ms > runtime.challenge_expires_at_ms
        || plan.expires_at_ms <= plan.issued_at_ms
    {
        return Err(ObserverCalibrationError::InvalidPlanInterval(
            plan.trial_id.clone(),
        ));
    }
    if plan.expires_at_ms.saturating_sub(plan.issued_at_ms)
        > MAX_CALIBRATION_CHALLENGE_TTL_MS
    {
        return Err(ObserverCalibrationError::ChallengeLifetimeExceeded(
            plan.trial_id.clone(),
        ));
    }
    if plan.observer_control_domain == plan.challenge_authority_control_domain
        || plan.observer_control_domain == manifest.agent.control_domain
        || plan.observer_control_domain == manifest.baseline.control_domain
        || plan.challenge_authority_control_domain == manifest.agent.control_domain
        || plan.challenge_authority_control_domain == manifest.baseline.control_domain
    {
        return Err(ObserverCalibrationError::InvalidIdentity(
            plan.trial_id.clone(),
        ));
    }
    Ok(())
}

fn ratio_ppm(numerator: usize, denominator: usize) -> Option<u64> {
    if denominator == 0 {
        return None;
    }
    let numerator = numerator as u128;
    let denominator = denominator as u128;
    u64::try_from(numerator.saturating_mul(1_000_000) / denominator).ok()
}

fn percentile(values: &[u64], percent: usize) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = (sorted.len().saturating_mul(percent).saturating_add(99) / 100)
        .max(1)
        .min(sorted.len());
    sorted.get(rank - 1).copied()
}

fn is_zero(digest: &Digest) -> bool {
    digest.0.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::validate_runtime_attestations_from_events;
    use crate::sealed::{
        BaselineIdentity, EvaluationLane, FrozenAgentIdentity, PlannedTrial, ScenarioReceipt,
        TrialEventKind,
    };

    fn d(value: u8) -> Digest {
        Digest::from_byte(value)
    }

    fn fixture() -> (
        RunManifest,
        FreezeReceipt,
        Vec<ScenarioReceipt>,
        Vec<TrialEvent>,
        Vec<RuntimeTrialAttestation>,
        Vec<ObserverCalibrationPlan>,
        Vec<ProbeInjectionReceipt>,
        Vec<ProbeDetectionReceipt>,
    ) {
        let evaluator = d(1);
        let agent_identity = d(2);
        let baseline_identity = d(3);
        let observer_adapter = d(4);
        let challenge_authority_id = "challenge-authority";
        let challenge_domain = "challenge-domain";
        let observer_id = "runtime-observer";
        let observer_domain = "observer-domain";
        let commitment = d(5);
        let environment = d(6);
        let budget = d(7);
        let agent = FrozenAgentIdentity {
            agent_id: "candidate".into(),
            agent_image_digest: d(8),
            source_commit: "0123456789abcdef0123456789abcdef01234567".into(),
            model_provider: "provider".into(),
            model_id: "candidate-model".into(),
            model_version: "v1".into(),
            scaffold_digest: d(9),
            prompt_digest: d(10),
            toolchain_digest: d(11),
            runtime_config_digest: d(12),
            tool_policy_digest: d(13),
            control_domain: "agent-domain".into(),
        };
        let baseline = BaselineIdentity {
            baseline_id: "baseline".into(),
            image_digest: d(14),
            source_commit: "abcdef0123456789abcdef0123456789abcdef01".into(),
            model_provider: "provider".into(),
            model_id: "baseline-model".into(),
            model_version: "v1".into(),
            scaffold_digest: d(15),
            prompt_digest: d(16),
            toolchain_digest: d(17),
            runtime_config_digest: d(18),
            tool_policy_digest: d(19),
            control_domain: "baseline-domain".into(),
        };
        let trials = vec![PlannedTrial {
            trial_id: "trial-1".into(),
            cluster_id: "cluster-1".into(),
            first_arm: EvaluationArm::Candidate,
            maximum_attempts: 1,
        }];
        let manifest = RunManifest {
            schema: crate::sealed::SEALED_RUN_SCHEMA.into(),
            version: crate::sealed::SEALED_RUN_VERSION,
            run_id: "calibration-run".into(),
            lane: EvaluationLane::SealedCertification,
            created_at_ms: 5,
            frozen_at_ms: 10,
            evaluator_digest: evaluator.clone(),
            execution_environment_digest: environment.clone(),
            resource_budget_digest: budget.clone(),
            arm_order_schedule_digest: d(20),
            state_isolation_policy_digest: d(21),
            artifact_access_policy_digest: d(22),
            network_egress_policy_digest: d(23),
            runtime_observer_source_id: observer_id.into(),
            runtime_observer_control_domain: observer_domain.into(),
            runtime_observer_adapter_digest: observer_adapter.clone(),
            runtime_challenge_authority_id: challenge_authority_id.into(),
            runtime_challenge_authority_control_domain: challenge_domain.into(),
            agent_identity_digest: agent_identity.clone(),
            baseline_identity_digest: baseline_identity.clone(),
            agent,
            baseline,
            freeze_authority_id: "freeze-authority".into(),
            freeze_authority_control_domain: "freeze-domain".into(),
            evaluator_control_domain: "evaluator-domain".into(),
            evidence_source_id: "scenario-source".into(),
            evidence_control_domain: "scenario-domain".into(),
            trials,
        };
        let freeze = FreezeReceipt {
            run_id: manifest.run_id.clone(),
            manifest_commitment: commitment.clone(),
            committed_at_ms: 8,
            frozen_at_ms: manifest.frozen_at_ms,
            agent_identity_digest: agent_identity.clone(),
            baseline_identity_digest: baseline_identity.clone(),
            evaluator_digest: evaluator.clone(),
            execution_environment_digest: environment.clone(),
            resource_budget_digest: budget.clone(),
            arm_order_schedule_digest: d(20),
            state_isolation_policy_digest: d(21),
            artifact_access_policy_digest: d(22),
            network_egress_policy_digest: d(23),
            runtime_observer_source_id: observer_id.into(),
            runtime_observer_control_domain: observer_domain.into(),
            runtime_observer_adapter_digest: observer_adapter.clone(),
            runtime_challenge_authority_id: challenge_authority_id.into(),
            runtime_challenge_authority_control_domain: challenge_domain.into(),
            publisher_id: manifest.freeze_authority_id.clone(),
            publisher_control_domain: manifest.freeze_authority_control_domain.clone(),
        };
        let scenarios = vec![ScenarioReceipt {
            run_id: manifest.run_id.clone(),
            trial_id: "trial-1".into(),
            manifest_commitment: commitment.clone(),
            commitment_scheme: crate::sealed::SCENARIO_COMMITMENT_SCHEME.into(),
            case_payload_commitment: d(24),
            oracle_commitment: d(25),
            generated_at_ms: 12,
            issued_at_ms: 15,
            retired_at_ms: None,
            evaluator_digest: evaluator.clone(),
            source_id: manifest.evidence_source_id.clone(),
            control_domain: manifest.evidence_control_domain.clone(),
        }];
        let mut events = Vec::new();
        for (arm, start, end) in [
            (EvaluationArm::Candidate, 30, 35),
            (EvaluationArm::Baseline, 40, 45),
        ] {
            let arm_digest = if arm == EvaluationArm::Candidate {
                agent_identity.clone()
            } else {
                baseline_identity.clone()
            };
            for (suffix, at, kind) in [
                ("start", start, TrialEventKind::Started),
                ("complete", end, TrialEventKind::Completed),
            ] {
                events.push(TrialEvent {
                    run_id: manifest.run_id.clone(),
                    arm,
                    trial_id: "trial-1".into(),
                    event_id: format!("{arm:?}-{suffix}"),
                    attempt: 1,
                    occurred_at_ms: at,
                    kind,
                    manifest_commitment: commitment.clone(),
                    arm_identity_digest: arm_digest.clone(),
                    evaluator_digest: evaluator.clone(),
                    scenario_commitment: d(24),
                    source_id: manifest.evidence_source_id.clone(),
                    control_domain: manifest.evidence_control_domain.clone(),
                });
            }
        }
        let mut runtime_attestations = Vec::new();
        for (arm, first, last, runtime_byte, namespace_byte, sequence) in [
            (EvaluationArm::Candidate, 30, 35, 30, 31, 1),
            (EvaluationArm::Baseline, 40, 45, 32, 33, 2),
        ] {
            let arm_identity = if arm == EvaluationArm::Candidate {
                agent_identity.clone()
            } else {
                baseline_identity.clone()
            };
            runtime_attestations.push(RuntimeTrialAttestation {
                schema: crate::runtime::RUNTIME_ATTESTATION_SCHEMA.into(),
                run_id: manifest.run_id.clone(),
                trial_id: "trial-1".into(),
                arm,
                arm_identity_digest: arm_identity,
                scenario_commitment: d(24),
                manifest_commitment: commitment.clone(),
                evaluator_digest: evaluator.clone(),
                execution_environment_digest: environment.clone(),
                resource_budget_digest: budget.clone(),
                state_isolation_policy_digest: d(21),
                artifact_access_policy_digest: d(22),
                network_egress_policy_digest: d(23),
                runtime_instance_digest: d(runtime_byte),
                writable_state_namespace_digest: d(namespace_byte),
                observer_source_id: observer_id.into(),
                observer_control_domain: observer_domain.into(),
                observer_adapter_digest: observer_adapter.clone(),
                challenge_nonce_digest: d(40 + sequence),
                challenge_authority_id: challenge_authority_id.into(),
                challenge_authority_control_domain: challenge_domain.into(),
                challenge_issued_at_ms: 20,
                challenge_expires_at_ms: 200,
                receipt_sequence: sequence,
                issued_at_ms: last + 20,
                measurement_start_ms: first - 5,
                measurement_end_ms: last + 5,
                clock_uncertainty_ms: 1,
                process_tree_commitment: d(50),
                mount_namespace_commitment: d(51),
                filesystem_access_log_commitment: d(52),
                network_egress_log_commitment: d(53),
                runtime_event_stream_commitment: d(54),
                coverage: crate::runtime::RuntimeCoverage::Complete,
                sequence_gap_count: 0,
                dropped_event_count: 0,
                oracle_read_attempts: 0,
                oracle_read_successes: 0,
                unauthorized_artifact_read_attempts: 0,
                unauthorized_artifact_read_successes: 0,
                disallowed_egress_attempts: 0,
                disallowed_egress_successes: 0,
                cross_trial_state_observations: 0,
                shared_writable_namespace_observations: 0,
                allowlisted_egress_events: 0,
            });
        }
        let mut plans = Vec::new();
        let mut injections = Vec::new();
        let mut detections = Vec::new();
        for (arm, runtime_byte, base_id) in [
            (EvaluationArm::Candidate, 30, 60),
            (EvaluationArm::Baseline, 32, 70),
        ] {
            let runtime = runtime_attestations.iter()
                .find(|receipt| receipt.arm == arm).unwrap();
            let challenge = d(base_id);
            let positive = ExpectedProbe {
                probe_id_digest: d(base_id + 1),
                channel: ProbeChannel::ProcessLifecycle,
                expected_decision: ExpectedPolicyDecision::Forbidden,
                target_digest: d(base_id + 2),
                probe_cluster_digest: d(base_id + 9),
                maximum_detection_latency_ms: 2_000,
            };
            let negative = ExpectedProbe {
                probe_id_digest: d(base_id + 3),
                channel: ProbeChannel::ProcessLifecycle,
                expected_decision: ExpectedPolicyDecision::Allowed,
                target_digest: d(base_id + 4),
                probe_cluster_digest: d(base_id + 9),
                maximum_detection_latency_ms: 2_000,
            };
            let plan = ObserverCalibrationPlan {
                schema: OBSERVER_CALIBRATION_SCHEMA.into(),
                run_id: manifest.run_id.clone(),
                trial_id: "trial-1".into(),
                arm,
                manifest_commitment: commitment.clone(),
                scenario_commitment: d(24),
                arm_identity_digest: runtime.arm_identity_digest.clone(),
                runtime_instance_digest: runtime.runtime_instance_digest.clone(),
                observer_source_id: observer_id.into(),
                observer_control_domain: observer_domain.into(),
                observer_adapter_digest: observer_adapter.clone(),
                challenge_authority_id: challenge_authority_id.into(),
                challenge_authority_control_domain: challenge_domain.into(),
                challenge_id_digest: challenge.clone(),
                probe_suite_digest: d(80),
                issued_at_ms: 22,
                expires_at_ms: 150,
                expected_probes: vec![positive.clone(), negative.clone()],
            };
            for (probe, time, decision) in [
                (&positive, runtime.measurement_start_ms + 5, ObservedPolicyDecision::Forbidden),
                (&negative, runtime.measurement_start_ms + 7, ObservedPolicyDecision::Allowed),
            ] {
                injections.push(ProbeInjectionReceipt {
                    schema: OBSERVER_CALIBRATION_SCHEMA.into(),
                    run_id: manifest.run_id.clone(),
                    trial_id: "trial-1".into(),
                    arm,
                    challenge_id_digest: challenge.clone(),
                    probe_id_digest: probe.probe_id_digest.clone(),
                    runtime_instance_digest: runtime.runtime_instance_digest.clone(),
                    channel: probe.channel,
                    target_digest: probe.target_digest.clone(),
                    status: ProbeInjectionStatus::Injected,
                    injected_at_ms: time,
                    injector_clock_uncertainty_ms: 1,
                    injection_event_commitment: d(90),
                    injector_source_id: challenge_authority_id.into(),
                    injector_control_domain: challenge_domain.into(),
                });
                detections.push(ProbeDetectionReceipt {
                    schema: OBSERVER_CALIBRATION_SCHEMA.into(),
                    run_id: manifest.run_id.clone(),
                    trial_id: "trial-1".into(),
                    arm,
                    challenge_id_digest: challenge.clone(),
                    probe_id_digest: probe.probe_id_digest.clone(),
                    runtime_instance_digest: runtime.runtime_instance_digest.clone(),
                    observer_source_id: observer_id.into(),
                    observer_control_domain: observer_domain.into(),
                    observer_adapter_digest: observer_adapter.clone(),
                    observation: ProbeObservation::Detected {
                        channel: probe.channel,
                        target_digest: probe.target_digest.clone(),
                        decision,
                        detected_at_ms: time + 5,
                        event_sequence: (base_id + 10) as u64 + (probe.probe_id_digest.0[0] as u64),
                        event_commitment: d(91),
                    },
                });
            }
            plans.push(plan);
        }
        (manifest, freeze, scenarios, events, runtime_attestations, plans, injections, detections)
    }

    #[test]
    fn clean_positive_and_negative_controls_pass_per_arm() {
        let (m, f, s, e, r, p, i, d) = fixture();
        let summary = evaluate_observer_calibration(
            &m, &f, &s, &[], &e, &r, &p, &i, &d
        ).unwrap();
        assert_eq!(summary.disposition, CalibrationDisposition::Pass);
        assert_eq!(summary.planned_arm_trials, 2);
        assert_eq!(summary.calibrated_arm_trials, 2);
        assert_eq!(summary.expected_probes, 4);
        assert_eq!(summary.detected_probes, 4);
        assert_eq!(summary.false_positives, 0);
        assert_eq!(summary.false_negatives, 0);
        assert_eq!(summary.channels[0].positive_controls, 2);
        assert_eq!(summary.channels[0].negative_controls, 2);
        assert_eq!(summary.unique_probe_clusters, 2);
        assert_eq!(summary.channels[0].unique_probe_clusters, 2);
        assert_eq!(summary.sensitivity_ppm, Some(1_000_000));
        assert_eq!(summary.specificity_ppm, Some(1_000_000));
        assert_eq!(summary.precision_ppm, Some(1_000_000));
    }

    #[test]
    fn positive_probe_miss_is_a_false_negative_and_fails() {
        let (m, f, s, e, r, p, i, mut d) = fixture();
        d[0].observation = ProbeObservation::NotDetected;
        let summary = evaluate_observer_calibration(
            &m, &f, &s, &[], &e, &r, &p, &i, &d
        ).unwrap();
        assert_eq!(summary.disposition, CalibrationDisposition::Fail);
        assert_eq!(summary.misses, 1);
        assert_eq!(summary.false_negatives, 1);
    }

    #[test]
    fn benign_probe_classified_as_forbidden_is_a_false_positive() {
        let (m, f, s, e, r, p, i, mut d) = fixture();
        if let ProbeObservation::Detected { decision, .. } = &mut d[1].observation {
            *decision = ObservedPolicyDecision::Forbidden;
        }
        let summary = evaluate_observer_calibration(
            &m, &f, &s, &[], &e, &r, &p, &i, &d
        ).unwrap();
        assert_eq!(summary.disposition, CalibrationDisposition::Fail);
        assert_eq!(summary.false_positives, 1);
    }

    #[test]
    fn over_latency_detection_fails_without_hiding_the_observation() {
        let (m, f, s, e, r, mut p, i, d) = fixture();
        p[0].expected_probes[0].maximum_detection_latency_ms = 6;
        let summary = evaluate_observer_calibration(
            &m, &f, &s, &[], &e, &r, &p, &i, &d
        ).unwrap();
        assert_eq!(summary.disposition, CalibrationDisposition::Fail);
        assert_eq!(summary.over_latency_probes, 1);
    }

    #[test]
    fn calibration_challenge_ttl_is_bounded() {
        let (m, f, s, e, r, mut p, i, d) = fixture();
        p[0].expires_at_ms = p[0].issued_at_ms + MAX_CALIBRATION_CHALLENGE_TTL_MS + 1;
        assert_eq!(
            evaluate_observer_calibration(&m, &f, &s, &[], &e, &r, &p, &i, &d),
            Err(ObserverCalibrationError::InvalidPlanInterval("trial-1".into()))
        );
    }

    #[test]
    fn conservative_latency_includes_both_independent_clock_uncertainties() {
        let (m, f, s, e, r, p, mut injections, mut detections) = fixture();
        injections[0].injector_clock_uncertainty_ms = 50;
        if let ProbeObservation::Detected { detected_at_ms, .. } = &mut detections[0].observation {
            *detected_at_ms = injections[0].injected_at_ms + 1_900;
        }
        let summary = evaluate_observer_calibration(&m, &f, &s, &[], &e, &r, &p, &injections, &detections).unwrap();
        assert_eq!(summary.over_latency_probes, 1);
        assert_eq!(summary.disposition, CalibrationDisposition::Fail);
    }

    #[test]
    fn observer_and_injector_must_be_distinct_authenticated_roles() {
        let (m, f, s, e, r, p, mut i, d) = fixture();
        i[0].injector_control_domain = m.runtime_observer_control_domain.clone();
        assert_eq!(
            evaluate_observer_calibration(&m, &f, &s, &[], &e, &r, &p, &i, &d),
            Err(ObserverCalibrationError::InjectionBindingMismatch("trial-1".into()))
        );
    }

    #[test]
    fn missing_injection_and_detection_receipts_cannot_be_omitted() {
        let (m, f, s, e, r, p, mut i, d) = fixture();
        i.pop();
        assert_eq!(
            evaluate_observer_calibration(&m, &f, &s, &[], &e, &r, &p, &i, &d),
            Err(ObserverCalibrationError::MissingInjection("trial-1".into()))
        );
        let (m, f, s, e, r, p, i, mut d) = fixture();
        d.pop();
        assert_eq!(
            evaluate_observer_calibration(&m, &f, &s, &[], &e, &r, &p, &i, &d),
            Err(ObserverCalibrationError::MissingDetection("trial-1".into()))
        );
    }

    #[test]
    fn duplicate_or_unplanned_probes_are_rejected() {
        let (m, f, s, e, r, p, mut i, d) = fixture();
        let duplicate = i[0].clone();
        i.push(duplicate);
        assert_eq!(
            evaluate_observer_calibration(&m, &f, &s, &[], &e, &r, &p, &i, &d),
            Err(ObserverCalibrationError::DuplicateInjection("trial-1".into()))
        );
        let (m, f, s, e, r, p, i, mut detections) = fixture();
        detections[0].probe_id_digest = Digest::from_byte(99);
        assert!(evaluate_observer_calibration(
            &m, &f, &s, &[], &e, &r, &p, &i, &detections
        ).is_err());
    }

    #[test]
    fn wrong_channel_target_or_adapter_is_rejected() {
        let (m, f, s, e, r, p, i, mut d) = fixture();
        if let ProbeObservation::Detected { channel, .. } = &mut d[0].observation {
            *channel = ProbeChannel::DnsEgress;
        }
        assert_eq!(
            evaluate_observer_calibration(&m, &f, &s, &[], &e, &r, &p, &i, &d),
            Err(ObserverCalibrationError::DetectionEventMissing("trial-1".into()))
        );
        let (m, f, s, e, r, p, i, mut d) = fixture();
        d[0].observer_adapter_digest = Digest::from_byte(99);
        assert_eq!(
            evaluate_observer_calibration(&m, &f, &s, &[], &e, &r, &p, &i, &d),
            Err(ObserverCalibrationError::DetectionBindingMismatch("trial-1".into()))
        );
    }

    #[test]
    fn seventy_two_adversarial_calibration_mutations_fail_closed() {
        for vector in 0..72 {
            let (mut m, f, s, e, r, mut p, mut i, mut d) = fixture();
            match vector {
                0 => p[0].schema.push_str("-wrong"),
                1 => p[0].run_id = "other-run".into(),
                2 => p[0].manifest_commitment = Digest::from_byte(99),
                3 => p[0].scenario_commitment = Digest::from_byte(99),
                4 => p[0].arm_identity_digest = Digest::from_byte(99),
                5 => p[0].runtime_instance_digest = Digest::from_byte(99),
                6 => p[0].observer_source_id = "agent".into(),
                7 => p[0].observer_control_domain = "agent-domain".into(),
                8 => p[0].observer_adapter_digest = Digest::from_byte(99),
                9 => p[0].challenge_authority_id = "agent".into(),
                10 => p[0].challenge_authority_control_domain = "agent-domain".into(),
                11 => p[0].challenge_id_digest = Digest::from_byte(0),
                12 => p[0].probe_suite_digest = Digest::from_byte(0),
                13 => p[0].issued_at_ms = m.frozen_at_ms,
                14 => p[0].expires_at_ms = p[0].issued_at_ms,
                15 => p[0].expected_probes[0].probe_id_digest = Digest::from_byte(0),
                16 => p[0].expected_probes[0].target_digest = Digest::from_byte(0),
                17 => p[0].expected_probes[0].maximum_detection_latency_ms = 0,
                18 => { let duplicate = p[0].expected_probes[0].clone(); p[0].expected_probes[1] = duplicate; },
                19 => p[0].expected_probes.pop(),
                20 => i[0].run_id = "other".into(),
                21 => i[0].challenge_id_digest = Digest::from_byte(99),
                22 => i[0].probe_id_digest = Digest::from_byte(99),
                23 => i[0].runtime_instance_digest = Digest::from_byte(99),
                24 => i[0].channel = ProbeChannel::DnsEgress,
                25 => i[0].target_digest = Digest::from_byte(99),
                26 => i[0].status = ProbeInjectionStatus::NotInjected,
                27 => i[0].injection_event_commitment = Digest::from_byte(0),
                28 => i[0].injector_source_id = "agent".into(),
                29 => i[0].injector_control_domain = "agent-domain".into(),
                30 => i[0].injected_at_ms = 1,
                31 => i[0].injected_at_ms = 10_000,
                32 => d[0].schema.push_str("-wrong"),
                33 => d[0].run_id = "other".into(),
                34 => d[0].challenge_id_digest = Digest::from_byte(99),
                35 => d[0].probe_id_digest = Digest::from_byte(99),
                36 => d[0].runtime_instance_digest = Digest::from_byte(99),
                37 => d[0].observer_source_id = "agent".into(),
                38 => d[0].observer_control_domain = "agent-domain".into(),
                39 => d[0].observer_adapter_digest = Digest::from_byte(99),
                40 => d[0].observation = ProbeObservation::NotDetected,
                41 => { if let ProbeObservation::Detected { decision, .. } = &mut d[0].observation { *decision = ObservedPolicyDecision::Unknown; } },
                42 => { if let ProbeObservation::Detected { event_sequence, .. } = &mut d[0].observation { *event_sequence = 0; } },
                43 => { if let ProbeObservation::Detected { event_commitment, .. } = &mut d[0].observation { *event_commitment = Digest::from_byte(0); } },
                44 => { if let ProbeObservation::Detected { detected_at_ms, .. } = &mut d[0].observation { *detected_at_ms = 1; } },
                45 => { if let ProbeObservation::Detected { detected_at_ms, .. } = &mut d[0].observation { *detected_at_ms = 50_000; } },
                46 => { if let ProbeObservation::Detected { target_digest, .. } = &mut d[0].observation { *target_digest = Digest::from_byte(99); } },
                47 => { if let ProbeObservation::Detected { channel, .. } = &mut d[0].observation { *channel = ProbeChannel::DnsEgress; } },
                48 => { if let ProbeObservation::Detected { decision, .. } = &mut d[1].observation { *decision = ObservedPolicyDecision::Forbidden; } },
                49 => { if let ProbeObservation::Detected { event_sequence, .. } = &mut d[1].observation { *event_sequence = 1; } },
                50 => { i[0].challenge_id_digest = Digest::from_byte(0); },
                51 => { p[0].expected_probes[0].expected_decision = ExpectedPolicyDecision::Allowed; },
                52 => { p[0].expected_probes[0].channel = ProbeChannel::DnsEgress; },
                53 => { p[0].trial_id = "unplanned".into(); },
                54 => { p[0].arm = EvaluationArm::Baseline; },
                55 => { p.pop(); },
                56 => { i.pop(); },
                57 => { d.pop(); },
                58 => { let duplicate = p[0].clone(); p.push(duplicate); },
                59 => { let duplicate = i[0].clone(); i.push(duplicate); },
                60 => { let duplicate = d[0].clone(); d.push(duplicate); },
                61 => { let duplicate = p[0].expected_probes[0].clone(); p[0].expected_probes.push(duplicate); },
                62 => { m.runtime_observer_control_domain = m.runtime_challenge_authority_control_domain.clone(); },
                63 => { m.runtime_observer_source_id = m.runtime_challenge_authority_id.clone(); },
                64 => { m.runtime_challenge_authority_control_domain = m.agent.control_domain.clone(); },
                65 => { m.runtime_observer_control_domain = m.agent.control_domain.clone(); },
                66 => { if let ProbeObservation::Detected { event_commitment, .. } = &mut d[0].observation { *event_commitment = Digest::from_byte(0); } },
                67 => { let target = p[0].expected_probes[0].target_digest.clone(); p[0].expected_probes[1].target_digest = target; },
                68 => { d[0].observer_source_id.clear(); },
                69 => { i[0].injector_source_id.clear(); },
                70 => { if let ProbeObservation::Detected { event_sequence, .. } = &mut d[0].observation { *event_sequence = u64::MAX; } },
                _ => { p[0].expires_at_ms = p[0].issued_at_ms; }
            }
            assert!(evaluate_observer_calibration(
                &m, &f, &s, &[], &e, &r, &p, &i, &d
            ).is_err() || {
                let _ = vector;
                false
            }, "calibration adversarial vector {vector} was accepted");
        }
    }
}
