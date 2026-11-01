//! Independent runtime enforcement evidence for sealed OpsBench.
//!
//! This module validates the structure and binding of runtime observer receipts.
//! It does not authenticate signatures or prove that a kernel/hypervisor enforced
//! a policy. The existing typed source-evidence adapter must authenticate records
//! before they are passed here.

use crate::sealed::{
    validate_freeze_receipt, validate_manifest, validate_scenario, ArmRunSummary,
    EvaluationArm, FreezeReceipt,
    RunLedgerSummary, RunManifest, ScenarioReceipt, TrialDisposition, TrialResult,
};
use crate::Digest;
use std::collections::{BTreeMap, BTreeSet};

pub const RUNTIME_ATTESTATION_SCHEMA: &str = "continuous-opsbench-runtime-attestation-v0.1";
pub const MAX_CLOCK_UNCERTAINTY_MS: u64 = 1_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeCoverage {
    Complete,
    Partial,
    Unknown,
    Tampered,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeTrialAttestation {
    pub schema: String,
    pub run_id: String,
    pub trial_id: String,
    pub arm: EvaluationArm,
    pub arm_identity_digest: Digest,
    pub scenario_commitment: Digest,
    pub manifest_commitment: Digest,
    pub evaluator_digest: Digest,
    pub execution_environment_digest: Digest,
    pub resource_budget_digest: Digest,
    pub state_isolation_policy_digest: Digest,
    pub artifact_access_policy_digest: Digest,
    pub network_egress_policy_digest: Digest,
    pub runtime_instance_digest: Digest,
    pub writable_state_namespace_digest: Digest,
    pub observer_source_id: String,
    pub observer_control_domain: String,
    pub observer_adapter_digest: Digest,
    /// Fresh nonce issued by the frozen runtime challenge authority.
    pub challenge_nonce_digest: Digest,
    pub challenge_authority_id: String,
    pub challenge_authority_control_domain: String,
    pub challenge_issued_at_ms: u64,
    pub challenge_expires_at_ms: u64,
    pub receipt_sequence: u64,
    pub issued_at_ms: u64,
    pub measurement_start_ms: u64,
    pub measurement_end_ms: u64,
    pub clock_uncertainty_ms: u64,
    pub process_tree_commitment: Digest,
    pub mount_namespace_commitment: Digest,
    pub filesystem_access_log_commitment: Digest,
    pub network_egress_log_commitment: Digest,
    pub runtime_event_stream_commitment: Digest,
    pub coverage: RuntimeCoverage,
    pub sequence_gap_count: u64,
    pub dropped_event_count: u64,
    pub oracle_read_attempts: u64,
    pub oracle_read_successes: u64,
    pub unauthorized_artifact_read_attempts: u64,
    pub unauthorized_artifact_read_successes: u64,
    pub disallowed_egress_attempts: u64,
    pub disallowed_egress_successes: u64,
    pub cross_trial_state_observations: u64,
    pub shared_writable_namespace_observations: u64,
    /// Allowed egress is reported separately and must be checked against the
    /// exact allowlist by the authenticated runtime adapter.
    pub allowlisted_egress_events: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeTrialEvidence {
    pub trial_id: String,
    pub arm: EvaluationArm,
    pub runtime_instance_digest: Digest,
    pub writable_state_namespace_digest: Digest,
    pub challenge_nonce_digest: Digest,
    pub challenge_issued_at_ms: u64,
    pub challenge_expires_at_ms: u64,
    pub receipt_sequence: u64,
    pub measurement_start_ms: u64,
    pub measurement_end_ms: u64,
    pub allowlisted_egress_events: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeAttestationSummary {
    pub run_id: String,
    pub planned_arm_trials: usize,
    pub complete_attestations: usize,
    pub allowlisted_egress_events: u128,
    pub runtime_observer_source_id: String,
    pub runtime_observer_control_domain: String,
    pub runtime_observer_adapter_digest: Digest,
    pub runtime_challenge_authority_id: String,
    pub runtime_challenge_authority_control_domain: String,
    pub evidence: Vec<RuntimeTrialEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeAttestationError {
    UnsupportedSchema(String),
    LedgerManifestMismatch,
    LedgerTrialDenominatorMismatch,
    LedgerTrialDuplicate(String, EvaluationArm),
    LedgerTrialMissing(String, EvaluationArm),
    LedgerTrialUnplanned(String, EvaluationArm),
    LedgerTrialClusterMismatch(String, EvaluationArm),
    LedgerTrialSummaryMismatch,
    LedgerArmExecutionOrderMismatch(String),
    ScenarioDuplicate(String),
    ScenarioManifestMismatch(String),
    ScenarioMissing(String),
    ScenarioBindingMismatch(String),
    DuplicateAttestation(String, EvaluationArm),
    MissingAttestation(String, EvaluationArm),
    UnplannedAttestation(String, EvaluationArm),
    AttestationBindingMismatch(String, EvaluationArm),
    ObserverIdentityMismatch,
    ObserverControlDomainCollision,
    ChallengeAuthorityMismatch,
    ChallengeNonceInvalid,
    ChallengeNonceCollision,
    ChallengeFreshnessInvalid(String, EvaluationArm),
    InvalidObserverSequence,
    EmptyOrInvalidIdentity,
    RuntimeInstanceCollision,
    StateNamespaceCollision,
    RuntimeIdentityDigestInvalid(String, EvaluationArm),
    InvalidMeasurementInterval(String, EvaluationArm),
    MeasurementDoesNotCoverTrial(String, EvaluationArm),
    ClockUncertaintyExceeded(String, EvaluationArm),
    IncompleteCoverage(String, EvaluationArm, RuntimeCoverage),
    TelemetryGap(String, EvaluationArm),
    OracleAccessObserved(String, EvaluationArm),
    UnauthorizedArtifactAccessObserved(String, EvaluationArm),
    DisallowedEgressObserved(String, EvaluationArm),
    CrossTrialStateObserved(String, EvaluationArm),
    SharedWritableNamespaceObserved(String, EvaluationArm),
    DuplicateObserverSequence,
    ObserverSequenceOrderInvalid,
    ObserverSequenceNotContiguous,
    ArithmeticOverflow,
}

pub fn validate_runtime_attestations(
    manifest: &RunManifest,
    freeze: &FreezeReceipt,
    ledger: &RunLedgerSummary,
    scenarios: &[ScenarioReceipt],
    attestations: &[RuntimeTrialAttestation],
) -> Result<RuntimeAttestationSummary, RuntimeAttestationError> {
    validate_manifest(manifest)
        .map_err(|_| RuntimeAttestationError::LedgerManifestMismatch)?;
    validate_freeze_receipt(manifest, &freeze.manifest_commitment, freeze)
        .map_err(|_| RuntimeAttestationError::LedgerManifestMismatch)?;
    if ledger.manifest_commitment != freeze.manifest_commitment {
        return Err(RuntimeAttestationError::LedgerManifestMismatch);
    }
    validate_ledger_binding(manifest, ledger)?;

    let mut scenario_index: BTreeMap<&str, &ScenarioReceipt> = BTreeMap::new();
    for scenario in scenarios {
        if !manifest.trials.iter().any(|trial| trial.trial_id == scenario.trial_id) {
            return Err(RuntimeAttestationError::ScenarioManifestMismatch(
                scenario.trial_id.clone(),
            ));
        }
        let planned = manifest.trials.iter()
            .find(|trial| trial.trial_id == scenario.trial_id)
            .ok_or_else(|| RuntimeAttestationError::ScenarioManifestMismatch(
                scenario.trial_id.clone()
            ))?;
        validate_scenario(
            manifest,
            &ledger.manifest_commitment,
            planned,
            scenario,
        ).map_err(|_| RuntimeAttestationError::ScenarioManifestMismatch(
            scenario.trial_id.clone()
        ))?;
        if scenario_index.insert(scenario.trial_id.as_str(), scenario).is_some() {
            return Err(RuntimeAttestationError::ScenarioDuplicate(
                scenario.trial_id.clone(),
            ));
        }
    }

    let mut ledger_index: BTreeMap<(&str, EvaluationArm), &TrialResult> = BTreeMap::new();
    for result in &ledger.trials {
        if ledger_index
            .insert((result.trial_id.as_str(), result.arm), result)
            .is_some()
        {
            return Err(RuntimeAttestationError::LedgerTrialDuplicate(
                result.trial_id.clone(),
                result.arm,
            ));
        }
    }

    let mut receipt_index: BTreeMap<(&str, EvaluationArm), &RuntimeTrialAttestation> =
        BTreeMap::new();
    let mut instances = BTreeSet::new();
    let mut namespaces = BTreeSet::new();
    let mut sequences = BTreeSet::new();
    let mut challenge_nonces = BTreeSet::new();
    let mut sequence_order = Vec::with_capacity(attestations.len());
    let mut evidence = Vec::with_capacity(attestations.len());
    let mut allowlisted_egress_events = 0u128;

    for receipt in attestations {
        if receipt.schema != RUNTIME_ATTESTATION_SCHEMA {
            return Err(RuntimeAttestationError::UnsupportedSchema(
                receipt.trial_id.clone(),
            ));
        }
        if !manifest.trials.iter().any(|trial| trial.trial_id == receipt.trial_id) {
            return Err(RuntimeAttestationError::UnplannedAttestation(
                receipt.trial_id.clone(),
                receipt.arm,
            ));
        }
        let result = ledger_index
            .get(&(receipt.trial_id.as_str(), receipt.arm))
            .copied()
            .ok_or_else(|| {
                RuntimeAttestationError::LedgerTrialMissing(
                    receipt.trial_id.clone(),
                    receipt.arm,
                )
            })?;
        let scenario = scenario_index
            .get(receipt.trial_id.as_str())
            .copied()
            .ok_or_else(|| RuntimeAttestationError::ScenarioMissing(receipt.trial_id.clone()))?;

        if receipt_index
            .insert((receipt.trial_id.as_str(), receipt.arm), receipt)
            .is_some()
        {
            return Err(RuntimeAttestationError::DuplicateAttestation(
                receipt.trial_id.clone(),
                receipt.arm,
            ));
        }

        validate_receipt_binding(
            manifest,
            &ledger.manifest_commitment,
            scenario,
            receipt,
        )?;
        if is_zero(&receipt.challenge_nonce_digest) {
            return Err(RuntimeAttestationError::ChallengeNonceInvalid);
        }
        if !challenge_nonces.insert(receipt.challenge_nonce_digest.clone()) {
            return Err(RuntimeAttestationError::ChallengeNonceCollision);
        }
        if receipt.challenge_issued_at_ms <= manifest.frozen_at_ms
            || receipt.challenge_issued_at_ms >= receipt.measurement_start_ms
            || receipt.challenge_expires_at_ms < receipt.measurement_end_ms
            || receipt.challenge_expires_at_ms < receipt.issued_at_ms
            || receipt.challenge_expires_at_ms <= receipt.challenge_issued_at_ms
        {
            return Err(RuntimeAttestationError::ChallengeFreshnessInvalid(
                receipt.trial_id.clone(),
                receipt.arm,
            ));
        }

        if receipt.runtime_instance_digest == receipt.writable_state_namespace_digest
            || is_zero(&receipt.runtime_instance_digest)
            || is_zero(&receipt.writable_state_namespace_digest)
        {
            return Err(RuntimeAttestationError::RuntimeIdentityDigestInvalid(
                receipt.trial_id.clone(),
                receipt.arm,
            ));
        }
        if !instances.insert(receipt.runtime_instance_digest.clone()) {
            return Err(RuntimeAttestationError::RuntimeInstanceCollision);
        }
        if !namespaces.insert(receipt.writable_state_namespace_digest.clone()) {
            return Err(RuntimeAttestationError::StateNamespaceCollision);
        }

        if receipt.measurement_start_ms >= receipt.measurement_end_ms
            || receipt.issued_at_ms < receipt.measurement_end_ms
        {
            return Err(RuntimeAttestationError::InvalidMeasurementInterval(
                receipt.trial_id.clone(),
                receipt.arm,
            ));
        }
        if receipt.clock_uncertainty_ms > MAX_CLOCK_UNCERTAINTY_MS {
            return Err(RuntimeAttestationError::ClockUncertaintyExceeded(
                receipt.trial_id.clone(),
                receipt.arm,
            ));
        }
        let first_activity = result
            .first_started_at_ms
            .unwrap_or(result.terminal_at_ms);
        if receipt.measurement_start_ms.saturating_add(receipt.clock_uncertainty_ms)
                > first_activity
            || receipt.measurement_end_ms.saturating_sub(receipt.clock_uncertainty_ms)
                < result.terminal_at_ms
        {
            return Err(RuntimeAttestationError::MeasurementDoesNotCoverTrial(
                receipt.trial_id.clone(),
                receipt.arm,
            ));
        }

        if receipt.coverage != RuntimeCoverage::Complete {
            return Err(RuntimeAttestationError::IncompleteCoverage(
                receipt.trial_id.clone(),
                receipt.arm,
                receipt.coverage,
            ));
        }
        if receipt.sequence_gap_count != 0 || receipt.dropped_event_count != 0 {
            return Err(RuntimeAttestationError::TelemetryGap(
                receipt.trial_id.clone(),
                receipt.arm,
            ));
        }
        if receipt.oracle_read_attempts != 0 || receipt.oracle_read_successes != 0 {
            return Err(RuntimeAttestationError::OracleAccessObserved(
                receipt.trial_id.clone(),
                receipt.arm,
            ));
        }
        if receipt.unauthorized_artifact_read_attempts != 0
            || receipt.unauthorized_artifact_read_successes != 0
        {
            return Err(RuntimeAttestationError::UnauthorizedArtifactAccessObserved(
                receipt.trial_id.clone(),
                receipt.arm,
            ));
        }
        if receipt.disallowed_egress_attempts != 0
            || receipt.disallowed_egress_successes != 0
        {
            return Err(RuntimeAttestationError::DisallowedEgressObserved(
                receipt.trial_id.clone(),
                receipt.arm,
            ));
        }
        if receipt.cross_trial_state_observations != 0 {
            return Err(RuntimeAttestationError::CrossTrialStateObserved(
                receipt.trial_id.clone(),
                receipt.arm,
            ));
        }
        if receipt.shared_writable_namespace_observations != 0 {
            return Err(RuntimeAttestationError::SharedWritableNamespaceObserved(
                receipt.trial_id.clone(),
                receipt.arm,
            ));
        }

        validate_log_commitments(receipt)?;

        allowlisted_egress_events = allowlisted_egress_events
            .checked_add(receipt.allowlisted_egress_events as u128)
            .ok_or(RuntimeAttestationError::ArithmeticOverflow)?;
        sequences.insert(receipt.receipt_sequence);
        sequence_order.push((receipt.issued_at_ms, receipt.receipt_sequence));
        evidence.push(RuntimeTrialEvidence {
            trial_id: receipt.trial_id.clone(),
            arm: receipt.arm,
            runtime_instance_digest: receipt.runtime_instance_digest.clone(),
            writable_state_namespace_digest: receipt.writable_state_namespace_digest.clone(),
            challenge_nonce_digest: receipt.challenge_nonce_digest.clone(),
            challenge_issued_at_ms: receipt.challenge_issued_at_ms,
            challenge_expires_at_ms: receipt.challenge_expires_at_ms,
            receipt_sequence: receipt.receipt_sequence,
            measurement_start_ms: receipt.measurement_start_ms,
            measurement_end_ms: receipt.measurement_end_ms,
            allowlisted_egress_events: receipt.allowlisted_egress_events,
        });
    }

    for trial in &manifest.trials {
        if !scenario_index.contains_key(trial.trial_id.as_str()) {
            return Err(RuntimeAttestationError::ScenarioMissing(trial.trial_id.clone()));
        }
        for arm in [EvaluationArm::Candidate, EvaluationArm::Baseline] {
            if !ledger_index.contains_key(&(trial.trial_id.as_str(), arm)) {
                return Err(RuntimeAttestationError::LedgerTrialMissing(
                    trial.trial_id.clone(),
                    arm,
                ));
            }
            if !receipt_index.contains_key(&(trial.trial_id.as_str(), arm)) {
                return Err(RuntimeAttestationError::MissingAttestation(
                    trial.trial_id.clone(),
                    arm,
                ));
            }
        }
    }

    if sequences.len() != attestations.len() {
        return Err(RuntimeAttestationError::DuplicateObserverSequence);
    }
    sequence_order.sort_unstable_by_key(|(issued, sequence)| (*issued, *sequence));
    if sequence_order
        .windows(2)
        .any(|pair| pair[0].1 >= pair[1].1)
    {
        return Err(RuntimeAttestationError::ObserverSequenceOrderInvalid);
    }
    if sequence_order.first().map(|entry| entry.1) != Some(1)
        || sequence_order.windows(2).any(|pair| pair[1].1 != pair[0].1.saturating_add(1))
    {
        return Err(RuntimeAttestationError::ObserverSequenceNotContiguous);
    }

    Ok(RuntimeAttestationSummary {
        run_id: manifest.run_id.clone(),
        planned_arm_trials: manifest.trials.len().saturating_mul(2),
        complete_attestations: attestations.len(),
        allowlisted_egress_events,
        runtime_observer_source_id: manifest.runtime_observer_source_id.clone(),
        runtime_observer_control_domain: manifest.runtime_observer_control_domain.clone(),
        runtime_observer_adapter_digest: manifest.runtime_observer_adapter_digest.clone(),
        runtime_challenge_authority_id: manifest.runtime_challenge_authority_id.clone(),
        runtime_challenge_authority_control_domain: manifest.runtime_challenge_authority_control_domain.clone(),
        evidence,
    })
}

pub(crate) pub(crate) fn validate_ledger_binding(
    manifest: &RunManifest,
    ledger: &RunLedgerSummary,
) -> Result<(), RuntimeAttestationError> {
    if ledger.run_id != manifest.run_id
        || ledger.lane != manifest.lane
        || ledger.manifest_commitment.0.iter().all(|b| *b == 0)
        || ledger.agent_identity_digest != manifest.agent_identity_digest
        || ledger.baseline_identity_digest != manifest.baseline_identity_digest
        || ledger.evaluator_digest != manifest.evaluator_digest
        || ledger.execution_environment_digest != manifest.execution_environment_digest
        || ledger.resource_budget_digest != manifest.resource_budget_digest
        || ledger.arm_order_schedule_digest != manifest.arm_order_schedule_digest
        || ledger.state_isolation_policy_digest != manifest.state_isolation_policy_digest
        || ledger.artifact_access_policy_digest != manifest.artifact_access_policy_digest
        || ledger.network_egress_policy_digest != manifest.network_egress_policy_digest
        || ledger.runtime_observer_source_id != manifest.runtime_observer_source_id
        || ledger.runtime_observer_control_domain != manifest.runtime_observer_control_domain
        || ledger.runtime_observer_adapter_digest != manifest.runtime_observer_adapter_digest
        || ledger.runtime_challenge_authority_id != manifest.runtime_challenge_authority_id
        || ledger.runtime_challenge_authority_control_domain != manifest.runtime_challenge_authority_control_domain
        || ledger.planned_trials != manifest.trials.len()
        || ledger.planned_arm_trials != manifest.trials.len().saturating_mul(2)
        || ledger.trials.len() != manifest.trials.len().saturating_mul(2)
        || ledger.candidate.planned_trials != manifest.trials.len()
        || ledger.baseline.planned_trials != manifest.trials.len()
        || ledger.unique_clusters
            != manifest.trials.iter().map(|trial| trial.cluster_id.as_str())
                .collect::<BTreeSet<_>>().len()
    {
        return Err(RuntimeAttestationError::LedgerManifestMismatch);
    }

    let mut candidate = ArmRunSummary {
        planned_trials: manifest.trials.len(),
        completed_trials: 0,
        failed_trials: 0,
        skipped_trials: 0,
        invalidated_trials: 0,
        total_attempts: 0,
    };
    let mut baseline = candidate.clone();
    let mut total_attempts = 0usize;
    let mut result_index: BTreeMap<(&str, EvaluationArm), &TrialResult> = BTreeMap::new();
    for result in &ledger.trials {
        if result_index.insert((result.trial_id.as_str(), result.arm), result).is_some() {
            return Err(RuntimeAttestationError::LedgerTrialDuplicate(
                result.trial_id.clone(), result.arm
            ));
        }
    }
    for trial in &manifest.trials {
        let candidate_result = result_index.get(&(trial.trial_id.as_str(), EvaluationArm::Candidate))
            .ok_or_else(|| RuntimeAttestationError::LedgerTrialMissing(
                trial.trial_id.clone(), EvaluationArm::Candidate
            ))?;
        let baseline_result = result_index.get(&(trial.trial_id.as_str(), EvaluationArm::Baseline))
            .ok_or_else(|| RuntimeAttestationError::LedgerTrialMissing(
                trial.trial_id.clone(), EvaluationArm::Baseline
            ))?;
        let candidate_activity = candidate_result.first_started_at_ms
            .unwrap_or(candidate_result.terminal_at_ms);
        let baseline_activity = baseline_result.first_started_at_ms
            .unwrap_or(baseline_result.terminal_at_ms);
        let arm_order_valid = match trial.first_arm {
            EvaluationArm::Candidate => candidate_activity < baseline_activity,
            EvaluationArm::Baseline => baseline_activity < candidate_activity,
        };
        if !arm_order_valid {
            return Err(RuntimeAttestationError::LedgerArmExecutionOrderMismatch(
                trial.trial_id.clone()
            ));
        }
        for result in [*candidate_result, *baseline_result] {
            let timeline_valid = match (result.attempts_started, result.first_started_at_ms) {
                (0, None) => matches!(
                    result.disposition,
                    TrialDisposition::Skipped | TrialDisposition::Invalidated
                ),
                (0, Some(_)) => false,
                (_, None) => false,
                (_, Some(started_at)) => started_at < result.terminal_at_ms,
            };
            if result.attempts_started > trial.maximum_attempts
                || !timeline_valid
                || (result.attempts_started > 0
                    && matches!(result.disposition, TrialDisposition::Skipped))
            {
                return Err(RuntimeAttestationError::LedgerTrialSummaryMismatch);
            }
        }
    }
    for result in &ledger.trials {
        let planned = manifest.trials.iter()
            .find(|trial| trial.trial_id == result.trial_id)
            .ok_or_else(|| RuntimeAttestationError::LedgerTrialUnplanned(
                result.trial_id.clone(), result.arm
            ))?;
        if planned.cluster_id != result.cluster_id {
            return Err(RuntimeAttestationError::LedgerTrialClusterMismatch(
                result.trial_id.clone(), result.arm
            ));
        }
        let arm_summary = match result.arm {
            EvaluationArm::Candidate => &mut candidate,
            EvaluationArm::Baseline => &mut baseline,
        };
        arm_summary.total_attempts = arm_summary.total_attempts
            .checked_add(result.attempts_started as usize)
            .ok_or(RuntimeAttestationError::ArithmeticOverflow)?;
        total_attempts = total_attempts.checked_add(result.attempts_started as usize)
            .ok_or(RuntimeAttestationError::ArithmeticOverflow)?;
        match result.disposition {
            crate::sealed::TrialDisposition::Completed => arm_summary.completed_trials += 1,
            crate::sealed::TrialDisposition::Failed => arm_summary.failed_trials += 1,
            crate::sealed::TrialDisposition::Skipped => arm_summary.skipped_trials += 1,
            crate::sealed::TrialDisposition::Invalidated => arm_summary.invalidated_trials += 1,
        }
    }
    if candidate != ledger.candidate
        || baseline != ledger.baseline
        || total_attempts != ledger.total_attempts
        || candidate.completed_trials + baseline.completed_trials != ledger.completed_trials
        || candidate.failed_trials + baseline.failed_trials != ledger.failed_trials
        || candidate.skipped_trials + baseline.skipped_trials != ledger.skipped_trials
        || candidate.invalidated_trials + baseline.invalidated_trials != ledger.invalidated_trials
    {
        return Err(RuntimeAttestationError::LedgerTrialSummaryMismatch);
    }
    Ok(())
}

fn validate_receipt_binding(
    manifest: &RunManifest,
    expected_manifest_commitment: &Digest,
    scenario: &ScenarioReceipt,
    receipt: &RuntimeTrialAttestation,
) -> Result<(), RuntimeAttestationError> {
    let expected_identity = match receipt.arm {
        EvaluationArm::Candidate => &manifest.agent_identity_digest,
        EvaluationArm::Baseline => &manifest.baseline_identity_digest,
    };
    if receipt.run_id != manifest.run_id
        || receipt.trial_id != scenario.trial_id
        || receipt.arm_identity_digest != *expected_identity
        || receipt.scenario_commitment != scenario.case_payload_commitment
        || receipt.manifest_commitment != *expected_manifest_commitment
        || scenario.manifest_commitment != *expected_manifest_commitment
        || receipt.evaluator_digest != manifest.evaluator_digest
        || receipt.execution_environment_digest != manifest.execution_environment_digest
        || receipt.resource_budget_digest != manifest.resource_budget_digest
        || receipt.state_isolation_policy_digest != manifest.state_isolation_policy_digest
        || receipt.artifact_access_policy_digest != manifest.artifact_access_policy_digest
        || receipt.network_egress_policy_digest != manifest.network_egress_policy_digest
        || receipt.observer_adapter_digest != manifest.runtime_observer_adapter_digest
    {
        return Err(RuntimeAttestationError::AttestationBindingMismatch(
            receipt.trial_id.clone(),
            receipt.arm,
        ));
    }
    if receipt.observer_source_id != manifest.runtime_observer_source_id {
        return Err(RuntimeAttestationError::ObserverIdentityMismatch);
    }
    if receipt.observer_control_domain != manifest.runtime_observer_control_domain {
        return Err(RuntimeAttestationError::ObserverControlDomainCollision);
    }
    if receipt.observer_control_domain == manifest.agent.control_domain
        || receipt.observer_control_domain == manifest.baseline.control_domain
        || receipt.observer_control_domain == manifest.evaluator_control_domain
        || receipt.observer_control_domain == manifest.freeze_authority_control_domain
        || receipt.observer_control_domain == manifest.evidence_control_domain
        || receipt.observer_control_domain == manifest.runtime_challenge_authority_control_domain
    {
        return Err(RuntimeAttestationError::ObserverControlDomainCollision);
    }
    if receipt.observer_source_id.trim().is_empty() {
        return Err(RuntimeAttestationError::EmptyOrInvalidIdentity);
    }
    if receipt.challenge_authority_id != manifest.runtime_challenge_authority_id
        || receipt.challenge_authority_control_domain
            != manifest.runtime_challenge_authority_control_domain
        || receipt.challenge_authority_control_domain == manifest.agent.control_domain
        || receipt.challenge_authority_control_domain == manifest.baseline.control_domain
        || receipt.challenge_authority_control_domain == manifest.evaluator_control_domain
        || receipt.challenge_authority_control_domain == manifest.freeze_authority_control_domain
        || receipt.challenge_authority_control_domain == manifest.evidence_control_domain
        || receipt.challenge_authority_control_domain == manifest.runtime_observer_control_domain
    {
        return Err(RuntimeAttestationError::ChallengeAuthorityMismatch);
    }
    Ok(())
}

fn validate_log_commitments(
    receipt: &RuntimeTrialAttestation,
) -> Result<(), RuntimeAttestationError> {
    let commitments = [
        &receipt.process_tree_commitment,
        &receipt.mount_namespace_commitment,
        &receipt.filesystem_access_log_commitment,
        &receipt.network_egress_log_commitment,
        &receipt.runtime_event_stream_commitment,
    ];
    if commitments.iter().any(|digest| is_zero(digest)) {
        return Err(RuntimeAttestationError::AttestationBindingMismatch(
            receipt.trial_id.clone(),
            receipt.arm,
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
    use crate::sealed::{BaselineIdentity, EvaluationLane, FrozenAgentIdentity};

    fn digest(byte: u8) -> Digest {
        Digest::from_byte(byte)
    }

    fn fixture() -> (
        RunManifest,
        FreezeReceipt,
        RunLedgerSummary,
        Vec<ScenarioReceipt>,
        Vec<RuntimeTrialAttestation>,
    ) {
        let agent_digest = digest(1);
        let baseline_digest = digest(2);
        let evaluator_digest = digest(3);
        let environment_digest = digest(4);
        let budget_digest = digest(5);
        let arm_order_digest = digest(6);
        let state_policy_digest = digest(7);
        let artifact_policy_digest = digest(8);
        let egress_policy_digest = digest(9);
        let adapter_digest = digest(10);
        let manifest_commitment = digest(11);

        let agent = FrozenAgentIdentity {
            agent_id: "candidate".into(),
            agent_image_digest: digest(12),
            source_commit: "0123456789abcdef0123456789abcdef01234567".into(),
            model_provider: "provider".into(),
            model_id: "candidate-model".into(),
            model_version: "v1".into(),
            scaffold_digest: digest(13),
            prompt_digest: digest(14),
            toolchain_digest: digest(15),
            runtime_config_digest: digest(16),
            tool_policy_digest: digest(17),
            control_domain: "candidate-domain".into(),
        };
        let baseline = BaselineIdentity {
            baseline_id: "baseline".into(),
            image_digest: digest(18),
            source_commit: "abcdef0123456789abcdef0123456789abcdef01".into(),
            model_provider: "provider".into(),
            model_id: "baseline-model".into(),
            model_version: "v1".into(),
            scaffold_digest: digest(19),
            prompt_digest: digest(20),
            toolchain_digest: digest(21),
            runtime_config_digest: digest(22),
            tool_policy_digest: digest(23),
            control_domain: "baseline-domain".into(),
        };
        let trials = vec![
            crate::sealed::PlannedTrial {
                trial_id: "trial-a".into(),
                cluster_id: "cluster-a".into(),
                first_arm: EvaluationArm::Candidate,
                maximum_attempts: 1,
            },
            crate::sealed::PlannedTrial {
                trial_id: "trial-b".into(),
                cluster_id: "cluster-b".into(),
                first_arm: EvaluationArm::Baseline,
                maximum_attempts: 1,
            },
        ];
        let manifest = RunManifest {
            schema: crate::sealed::SEALED_RUN_SCHEMA.into(),
            version: crate::sealed::SEALED_RUN_VERSION,
            run_id: "runtime-run".into(),
            lane: EvaluationLane::SealedCertification,
            created_at_ms: 10,
            frozen_at_ms: 20,
            evaluator_digest: evaluator_digest.clone(),
            execution_environment_digest: environment_digest.clone(),
            resource_budget_digest: budget_digest.clone(),
            arm_order_schedule_digest: arm_order_digest.clone(),
            state_isolation_policy_digest: state_policy_digest.clone(),
            artifact_access_policy_digest: artifact_policy_digest.clone(),
            network_egress_policy_digest: egress_policy_digest.clone(),
            runtime_observer_source_id: "runtime-observer".into(),
            runtime_observer_control_domain: "runtime-observer-domain".into(),
            runtime_observer_adapter_digest: adapter_digest.clone(),
            runtime_challenge_authority_id: "challenge-authority".into(),
            runtime_challenge_authority_control_domain: "challenge-domain".into(),
            agent_identity_digest: agent_digest.clone(),
            baseline_identity_digest: baseline_digest.clone(),
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
            manifest_commitment: manifest_commitment.clone(),
            committed_at_ms: 15,
            frozen_at_ms: manifest.frozen_at_ms,
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
            publisher_id: manifest.freeze_authority_id.clone(),
            publisher_control_domain: manifest.freeze_authority_control_domain.clone(),
        };

        let scenarios = vec![
            ScenarioReceipt {
                run_id: manifest.run_id.clone(),
                trial_id: "trial-a".into(),
                manifest_commitment: manifest_commitment.clone(),
                commitment_scheme: crate::sealed::SCENARIO_COMMITMENT_SCHEME.into(),
                case_payload_commitment: digest(24),
                oracle_commitment: digest(25),
                generated_at_ms: 25,
                issued_at_ms: 30,
                retired_at_ms: None,
                evaluator_digest: evaluator_digest.clone(),
                source_id: manifest.evidence_source_id.clone(),
                control_domain: manifest.evidence_control_domain.clone(),
            },
            ScenarioReceipt {
                run_id: manifest.run_id.clone(),
                trial_id: "trial-b".into(),
                manifest_commitment: manifest_commitment.clone(),
                commitment_scheme: crate::sealed::SCENARIO_COMMITMENT_SCHEME.into(),
                case_payload_commitment: digest(26),
                oracle_commitment: digest(27),
                generated_at_ms: 25,
                issued_at_ms: 30,
                retired_at_ms: None,
                evaluator_digest: evaluator_digest.clone(),
                source_id: manifest.evidence_source_id.clone(),
                control_domain: manifest.evidence_control_domain.clone(),
            },
        ];
        let ledger_trials = vec![
            TrialResult {
                trial_id: "trial-a".into(), arm: EvaluationArm::Candidate,
                cluster_id: "cluster-a".into(), disposition: TrialDisposition::Completed,
                attempts_started: 1, first_started_at_ms: Some(40), terminal_at_ms: 50,
            },
            TrialResult {
                trial_id: "trial-a".into(), arm: EvaluationArm::Baseline,
                cluster_id: "cluster-a".into(), disposition: TrialDisposition::Completed,
                attempts_started: 1, first_started_at_ms: Some(60), terminal_at_ms: 70,
            },
            TrialResult {
                trial_id: "trial-b".into(), arm: EvaluationArm::Baseline,
                cluster_id: "cluster-b".into(), disposition: TrialDisposition::Completed,
                attempts_started: 1, first_started_at_ms: Some(40), terminal_at_ms: 50,
            },
            TrialResult {
                trial_id: "trial-b".into(), arm: EvaluationArm::Candidate,
                cluster_id: "cluster-b".into(), disposition: TrialDisposition::Completed,
                attempts_started: 1, first_started_at_ms: Some(60), terminal_at_ms: 70,
            },
        ];
        let arm_summary = ArmRunSummary {
            planned_trials: 2, completed_trials: 2, failed_trials: 0, skipped_trials: 0,
            invalidated_trials: 0, total_attempts: 2,
        };
        let ledger = RunLedgerSummary {
            run_id: manifest.run_id.clone(),
            lane: manifest.lane,
            planned_trials: 2,
            planned_arm_trials: 4,
            candidate: arm_summary.clone(),
            baseline: arm_summary,
            completed_trials: 4,
            failed_trials: 0,
            skipped_trials: 0,
            invalidated_trials: 0,
            total_attempts: 4,
            unique_clusters: 2,
            manifest_commitment: manifest_commitment.clone(),
            agent_identity_digest: agent_digest,
            baseline_identity_digest: baseline_digest,
            evaluator_digest: evaluator_digest.clone(),
            execution_environment_digest: environment_digest,
            resource_budget_digest: budget_digest,
            arm_order_schedule_digest: arm_order_digest,
            state_isolation_policy_digest: state_policy_digest,
            artifact_access_policy_digest: artifact_policy_digest,
            network_egress_policy_digest: egress_policy_digest,
            runtime_observer_source_id: manifest.runtime_observer_source_id.clone(),
            runtime_observer_control_domain: manifest.runtime_observer_control_domain.clone(),
            runtime_observer_adapter_digest: adapter_digest.clone(),
            runtime_challenge_authority_id: manifest.runtime_challenge_authority_id.clone(),
            runtime_challenge_authority_control_domain: manifest.runtime_challenge_authority_control_domain.clone(),
            trials: ledger_trials,
        };
        let attestations = vec![
            attestation(&manifest, &scenarios[0], EvaluationArm::Candidate, digest(40), digest(41), 1, 30, 60),
            attestation(&manifest, &scenarios[0], EvaluationArm::Baseline, digest(42), digest(43), 2, 30, 80),
            attestation(&manifest, &scenarios[1], EvaluationArm::Baseline, digest(44), digest(45), 3, 30, 90),
            attestation(&manifest, &scenarios[1], EvaluationArm::Candidate, digest(46), digest(47), 4, 30, 100),
        ];
        (manifest, freeze, ledger, scenarios, attestations)
    }

    fn attestation(
        manifest: &RunManifest,
        scenario: &ScenarioReceipt,
        arm: EvaluationArm,
        runtime_instance_digest: Digest,
        writable_state_namespace_digest: Digest,
        sequence: u64,
        measurement_start_ms: u64,
        measurement_end_ms: u64,
    ) -> RuntimeTrialAttestation {
        let arm_identity_digest = match arm {
            EvaluationArm::Candidate => manifest.agent_identity_digest.clone(),
            EvaluationArm::Baseline => manifest.baseline_identity_digest.clone(),
        };
        RuntimeTrialAttestation {
            schema: RUNTIME_ATTESTATION_SCHEMA.into(),
            run_id: manifest.run_id.clone(),
            trial_id: scenario.trial_id.clone(),
            arm,
            arm_identity_digest,
            scenario_commitment: scenario.case_payload_commitment.clone(),
            manifest_commitment: scenario.manifest_commitment.clone(),
            evaluator_digest: manifest.evaluator_digest.clone(),
            execution_environment_digest: manifest.execution_environment_digest.clone(),
            resource_budget_digest: manifest.resource_budget_digest.clone(),
            state_isolation_policy_digest: manifest.state_isolation_policy_digest.clone(),
            artifact_access_policy_digest: manifest.artifact_access_policy_digest.clone(),
            network_egress_policy_digest: manifest.network_egress_policy_digest.clone(),
            runtime_instance_digest,
            writable_state_namespace_digest,
            observer_source_id: manifest.runtime_observer_source_id.clone(),
            observer_control_domain: manifest.runtime_observer_control_domain.clone(),
            observer_adapter_digest: manifest.runtime_observer_adapter_digest.clone(),
            challenge_nonce_digest: digest(60u8.saturating_add(sequence as u8)),
            challenge_authority_id: manifest.runtime_challenge_authority_id.clone(),
            challenge_authority_control_domain: manifest.runtime_challenge_authority_control_domain.clone(),
            challenge_issued_at_ms: 25,
            challenge_expires_at_ms: measurement_end_ms + 100,
            receipt_sequence: sequence,
            issued_at_ms: measurement_end_ms + 1,
            measurement_start_ms,
            measurement_end_ms,
            clock_uncertainty_ms: 10,
            process_tree_commitment: digest(50),
            mount_namespace_commitment: digest(51),
            filesystem_access_log_commitment: digest(52),
            network_egress_log_commitment: digest(53),
            runtime_event_stream_commitment: digest(54),
            coverage: RuntimeCoverage::Complete,
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
            allowlisted_egress_events: 2,
        }
    }

    #[test]
    fn complete_clean_runtime_receipts_cover_every_arm_and_trial() {
        let (manifest, freeze, ledger, scenarios, attestations) = fixture();
        let result =
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations).unwrap();
        assert_eq!(result.planned_arm_trials, 4);
        assert_eq!(result.complete_attestations, 4);
        assert_eq!(result.allowlisted_egress_events, 8);
        assert_eq!(result.evidence.len(), 4);
    }

    #[test]
    fn forged_ledger_cluster_count_is_rejected_before_runtime_receipts_are_credited() {
        let (manifest, freeze, mut ledger, scenarios, attestations) = fixture();
        ledger.unique_clusters = 1;
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::LedgerManifestMismatch)
        );
    }

    #[test]
    fn missing_first_attempt_timestamp_cannot_shrink_runtime_coverage() {
        let (manifest, freeze, mut ledger, scenarios, attestations) = fixture();
        ledger.trials[0].first_started_at_ms = None;
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::LedgerTrialSummaryMismatch)
        );
    }

    #[test]
    fn challenge_nonce_is_unique_and_must_be_fresh_after_freeze() {
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[1].challenge_nonce_digest = attestations[0].challenge_nonce_digest.clone();
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::ChallengeNonceCollision)
        );
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[0].challenge_issued_at_ms = manifest.frozen_at_ms;
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::ChallengeFreshnessInvalid(
                "trial-a".into(), EvaluationArm::Candidate
            ))
        );
    }

    #[test]
    fn runtime_challenge_must_enclose_measurement_and_receipt_issue() {
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[0].challenge_expires_at_ms = 59;
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::ChallengeFreshnessInvalid(
                "trial-a".into(), EvaluationArm::Candidate
            ))
        );
    }

    #[test]
    fn challenge_authority_cannot_be_the_runtime_observer_or_agent() {
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[0].challenge_authority_control_domain =
            manifest.runtime_observer_control_domain.clone();
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::ChallengeAuthorityMismatch)
        );
    }

    #[test]
    fn missing_runtime_receipt_is_not_zero_access() {
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations.pop();
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::MissingAttestation(
                "trial-b".into(),
                EvaluationArm::Candidate,
            ))
        );
    }

    #[test]
    fn oracle_attempt_is_a_hard_sealed_lane_violation_even_when_denied() {
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[0].oracle_read_attempts = 1;
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::OracleAccessObserved(
                "trial-a".into(),
                EvaluationArm::Candidate,
            ))
        );
    }

    #[test]
    fn partial_or_unknown_coverage_never_means_zero_access() {
        for coverage in [
            RuntimeCoverage::Partial,
            RuntimeCoverage::Unknown,
            RuntimeCoverage::Tampered,
        ] {
            let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
            attestations[0].coverage = coverage;
            assert_eq!(
                validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
                Err(RuntimeAttestationError::IncompleteCoverage(
                    "trial-a".into(),
                    EvaluationArm::Candidate,
                    coverage,
                ))
            );
        }
    }

    #[test]
    fn runtime_and_namespace_identifiers_must_be_unique() {
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[1].runtime_instance_digest = attestations[0].runtime_instance_digest.clone();
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::RuntimeInstanceCollision)
        );
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[1].writable_state_namespace_digest =
            attestations[0].writable_state_namespace_digest.clone();
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::StateNamespaceCollision)
        );
    }

    #[test]
    fn runtime_measurement_must_cover_all_agent_activity() {
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[0].measurement_start_ms = 41;
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::MeasurementDoesNotCoverTrial(
                "trial-a".into(),
                EvaluationArm::Candidate,
            ))
        );
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[0].measurement_end_ms = 49;
        attestations[0].issued_at_ms = 50;
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::MeasurementDoesNotCoverTrial(
                "trial-a".into(),
                EvaluationArm::Candidate,
            ))
        );
    }

    #[test]
    fn mismatched_policy_or_source_is_rejected() {
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[0].artifact_access_policy_digest = digest(99);
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::AttestationBindingMismatch(
                "trial-a".into(),
                EvaluationArm::Candidate,
            ))
        );
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[0].observer_source_id = "agent".into();
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::ObserverIdentityMismatch)
        );
    }

    #[test]
    fn state_carryover_egress_and_telemetry_loss_are_non_compensatory() {
        let mutations: [fn(&mut RuntimeTrialAttestation); 7] = [
            |r| r.cross_trial_state_observations = 1,
            |r| r.shared_writable_namespace_observations = 1,
            |r| r.disallowed_egress_attempts = 1,
            |r| r.disallowed_egress_successes = 1,
            |r| r.unauthorized_artifact_read_attempts = 1,
            |r| r.unauthorized_artifact_read_successes = 1,
            |r| r.dropped_event_count = 1,
        ];
        for mutate in mutations {
            let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
            mutate(&mut attestations[0]);
            assert!(validate_runtime_attestations(
                &manifest, &freeze, &ledger, &scenarios, &attestations
            ).is_err());
        }
    }

    #[test]
    fn duplicate_ordered_and_gapped_observer_sequences_fail_closed() {
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[1].receipt_sequence = attestations[0].receipt_sequence;
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::DuplicateObserverSequence)
        );
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[0].receipt_sequence = 10;
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::ObserverSequenceOrderInvalid)
        );
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[3].receipt_sequence = 6;
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::ObserverSequenceNotContiguous)
        );
    }

    #[test]
    fn clock_uncertainty_and_zero_log_commitment_are_rejected() {
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[0].clock_uncertainty_ms = MAX_CLOCK_UNCERTAINTY_MS + 1;
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::ClockUncertaintyExceeded(
                "trial-a".into(),
                EvaluationArm::Candidate,
            ))
        );
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[0].filesystem_access_log_commitment = digest(0);
        assert!(validate_runtime_attestations(
            &manifest, &freeze, &ledger, &scenarios, &attestations
        ).is_err());
    }

    #[test]
    fn sixty_adversarial_runtime_mutations_fail_closed() {
        for vector in 0..60 {
            let (mut manifest, freeze, mut ledger, mut scenarios, mut attestations) = fixture();
            match vector {
                0 => attestations[0].schema.push_str("-unknown"),
                1 => attestations[0].run_id = "other-run".into(),
                2 => attestations[0].trial_id = "unplanned".into(),
                3 => attestations[0].arm_identity_digest = digest(99),
                4 => attestations[0].scenario_commitment = digest(99),
                5 => attestations[0].manifest_commitment = digest(99),
                6 => attestations[0].evaluator_digest = digest(99),
                7 => attestations[0].execution_environment_digest = digest(99),
                8 => attestations[0].resource_budget_digest = digest(99),
                9 => attestations[0].state_isolation_policy_digest = digest(99),
                10 => attestations[0].artifact_access_policy_digest = digest(99),
                11 => attestations[0].network_egress_policy_digest = digest(99),
                12 => attestations[0].runtime_instance_digest = digest(0),
                13 => attestations[0].writable_state_namespace_digest = digest(0),
                14 => attestations[0].writable_state_namespace_digest = attestations[0].runtime_instance_digest.clone(),
                15 => attestations[1].runtime_instance_digest = attestations[0].runtime_instance_digest.clone(),
                16 => attestations[1].writable_state_namespace_digest = attestations[0].writable_state_namespace_digest.clone(),
                17 => attestations[0].observer_source_id = "candidate".into(),
                18 => attestations[0].observer_control_domain = "candidate-domain".into(),
                19 => attestations[0].observer_adapter_digest = digest(99),
                20 => attestations[0].receipt_sequence = 0,
                21 => attestations[1].receipt_sequence = attestations[0].receipt_sequence,
                22 => attestations[0].receipt_sequence = 10,
                23 => attestations[0].issued_at_ms = attestations[0].measurement_end_ms - 1,
                24 => attestations[0].measurement_start_ms = 41,
                25 => { attestations[0].measurement_end_ms = 49; attestations[0].issued_at_ms = 50; },
                26 => attestations[0].measurement_end_ms = attestations[0].measurement_start_ms,
                27 => attestations[0].clock_uncertainty_ms = MAX_CLOCK_UNCERTAINTY_MS + 1,
                28 => attestations[0].process_tree_commitment = digest(0),
                29 => attestations[0].mount_namespace_commitment = digest(0),
                30 => attestations[0].filesystem_access_log_commitment = digest(0),
                31 => attestations[0].network_egress_log_commitment = digest(0),
                32 => attestations[0].runtime_event_stream_commitment = digest(0),
                33 => attestations[0].coverage = RuntimeCoverage::Partial,
                34 => attestations[0].coverage = RuntimeCoverage::Unknown,
                35 => attestations[0].coverage = RuntimeCoverage::Tampered,
                36 => attestations[0].sequence_gap_count = 1,
                37 => attestations[0].dropped_event_count = 1,
                38 => attestations[0].oracle_read_attempts = 1,
                39 => attestations[0].oracle_read_successes = 1,
                40 => attestations[0].unauthorized_artifact_read_attempts = 1,
                41 => attestations[0].unauthorized_artifact_read_successes = 1,
                42 => attestations[0].disallowed_egress_attempts = 1,
                43 => attestations[0].disallowed_egress_successes = 1,
                44 => attestations[0].cross_trial_state_observations = 1,
                45 => attestations[0].shared_writable_namespace_observations = 1,
                46 => { attestations.pop(); },
                47 => { scenarios.pop(); },
                48 => { let mut extra = attestations[0].clone(); extra.trial_id = "not-planned".into(); attestations.push(extra); },
                49 => { let duplicate = attestations[0].clone(); attestations.push(duplicate); },
                50 => { let duplicate = scenarios[0].clone(); scenarios.push(duplicate); },
                51 => ledger.manifest_commitment = digest(99),
                52 => ledger.agent_identity_digest = digest(99),
                53 => ledger.baseline_identity_digest = digest(99),
                54 => ledger.execution_environment_digest = digest(99),
                55 => ledger.state_isolation_policy_digest = digest(99),
                56 => ledger.trials[0].cluster_id = "wrong-cluster".into(),
                57 => { ledger.trials.pop(); },
                58 => { manifest.runtime_observer_control_domain = manifest.agent.control_domain.clone(); },
                _ => manifest.runtime_observer_adapter_digest = digest(0),
            }
            assert!(
                validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations).is_err(),
                "runtime attestation adversarial vector {vector} was accepted"
            );
        }
    }
    #[test]
    fn observer_control_domain_must_be_distinct_from_the_evaluated_systems() {
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[0].observer_control_domain = manifest.agent.control_domain.clone();
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::ObserverControlDomainCollision)
        );
    }

    #[test]
    fn invalid_oracle_event_count_and_missing_scenario_are_rejected() {
        let (manifest, freeze, ledger, scenarios, mut attestations) = fixture();
        attestations[0].oracle_read_successes = 1;
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::OracleAccessObserved(
                "trial-a".into(),
                EvaluationArm::Candidate,
            ))
        );
        let (manifest, freeze, ledger, mut scenarios, attestations) = fixture();
        scenarios.pop();
        assert_eq!(
            validate_runtime_attestations(&manifest, &freeze, &ledger, &scenarios, &attestations),
            Err(RuntimeAttestationError::ScenarioMissing("trial-b".into()))
        );
    }
}
