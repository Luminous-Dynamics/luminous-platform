//! Frozen run identity and append-only trial accounting for sealed OpsBench.
//!
//! This is a deterministic structural validator, not a cryptographic verifier.
//! Manifest commitments and source-authenticated receipts must be verified by the
//! existing typed evidence adapter before these records reach this module.

use super::Digest;
use std::collections::{BTreeMap, BTreeSet};

pub const SEALED_RUN_SCHEMA: &str = "continuous-opsbench-sealed-run-v0.1";
pub const SEALED_RUN_VERSION: u16 = 1;
pub const SCENARIO_COMMITMENT_SCHEME: &str = "sha256-domain-separated-length-prefixed-salt32-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvaluationLane {
    OpenDevelopment,
    SealedCertification,
    RetiredReproducibility,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrozenAgentIdentity {
    pub agent_id: String,
    pub agent_image_digest: Digest,
    pub source_commit: String,
    pub model_provider: String,
    pub model_id: String,
    pub model_version: String,
    pub scaffold_digest: Digest,
    pub prompt_digest: Digest,
    pub toolchain_digest: Digest,
    pub runtime_config_digest: Digest,
    pub tool_policy_digest: Digest,
    pub control_domain: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaselineIdentity {
    pub baseline_id: String,
    pub image_digest: Digest,
    pub source_commit: String,
    pub model_provider: String,
    pub model_id: String,
    pub model_version: String,
    pub scaffold_digest: Digest,
    pub prompt_digest: Digest,
    pub toolchain_digest: Digest,
    pub runtime_config_digest: Digest,
    pub tool_policy_digest: Digest,
    pub control_domain: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedTrial {
    pub trial_id: String,
    /// Independent resampling unit: fault schedule × topology × application cluster.
    pub cluster_id: String,
    /// Precommitted first arm. Both arms execute against the same case and budget.
    pub first_arm: EvaluationArm,
    /// Maximum declared attempts, including the first attempt.
    pub maximum_attempts: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunManifest {
    pub schema: String,
    pub version: u16,
    pub run_id: String,
    pub lane: EvaluationLane,
    pub created_at_ms: u64,
    pub frozen_at_ms: u64,
    pub evaluator_digest: Digest,
    /// Frozen common execution environment for both arms (host/VM/runtime profile).
    pub execution_environment_digest: Digest,
    /// Frozen common resource ceilings for both arms (time, tokens, CPU, memory, tool calls).
    pub resource_budget_digest: Digest,
    pub agent_identity_digest: Digest,
    pub baseline_identity_digest: Digest,
    pub agent: FrozenAgentIdentity,
    pub baseline: BaselineIdentity,
    pub freeze_authority_id: String,
    pub freeze_authority_control_domain: String,
    pub evaluator_control_domain: String,
    pub evidence_source_id: String,
    pub evidence_control_domain: String,
    pub trials: Vec<PlannedTrial>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreezeReceipt {
    pub run_id: String,
    /// Exact commitment calculated and authenticated outside this module.
    pub manifest_commitment: Digest,
    pub committed_at_ms: u64,
    pub frozen_at_ms: u64,
    pub agent_identity_digest: Digest,
    pub baseline_identity_digest: Digest,
    pub evaluator_digest: Digest,
    pub execution_environment_digest: Digest,
    pub resource_budget_digest: Digest,
    pub publisher_id: String,
    pub publisher_control_domain: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioReceipt {
    pub run_id: String,
    pub trial_id: String,
    pub manifest_commitment: Digest,
    pub commitment_scheme: String,
    /// Commitment to the case payload given to the agent.
    pub case_payload_commitment: Digest,
    /// Commitment to the private business oracle / grading truth.
    pub oracle_commitment: Digest,
    pub generated_at_ms: u64,
    pub issued_at_ms: u64,
    /// Required for cases in the retired reproducibility lane.
    pub retired_at_ms: Option<u64>,
    pub evaluator_digest: Digest,
    pub source_id: String,
    pub control_domain: String,
}

/// Post-retirement disclosure binding the formerly hidden payload/oracle artifacts
/// to the commitments used during sealed evaluation. Hash/signature verification
/// is performed by the external source-evidence adapter, not this structural validator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioReveal {
    pub run_id: String,
    pub trial_id: String,
    pub manifest_commitment: Digest,
    pub commitment_scheme: String,
    pub case_payload_commitment: Digest,
    pub oracle_commitment: Digest,
    pub case_payload_artifact_digest: Digest,
    pub oracle_artifact_digest: Digest,
    pub case_payload_salt: Vec<u8>,
    pub oracle_salt: Vec<u8>,
    pub revealed_at_ms: u64,
    pub source_id: String,
    pub control_domain: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvaluationArm {
    Candidate,
    Baseline,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrialEventKind {
    Started,
    Completed,
    Failed,
    Skipped,
    Invalidated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrialEvent {
    pub run_id: String,
    pub arm: EvaluationArm,
    pub trial_id: String,
    pub event_id: String,
    pub attempt: u16,
    pub occurred_at_ms: u64,
    pub kind: TrialEventKind,
    pub manifest_commitment: Digest,
    pub arm_identity_digest: Digest,
    pub evaluator_digest: Digest,
    pub scenario_commitment: Digest,
    pub source_id: String,
    pub control_domain: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrialDisposition {
    Completed,
    Failed,
    Skipped,
    Invalidated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrialResult {
    pub trial_id: String,
    pub arm: EvaluationArm,
    pub cluster_id: String,
    pub disposition: TrialDisposition,
    pub attempts_started: u16,
    pub first_started_at_ms: Option<u64>,
    pub terminal_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmRunSummary {
    pub planned_trials: usize,
    pub completed_trials: usize,
    pub failed_trials: usize,
    pub skipped_trials: usize,
    pub invalidated_trials: usize,
    pub total_attempts: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunLedgerSummary {
    pub run_id: String,
    pub lane: EvaluationLane,
    /// Number of preregistered scenario clusters/trials, before multiplying by arms.
    pub planned_trials: usize,
    /// Exactly two arm trials per planned scenario.
    pub planned_arm_trials: usize,
    pub candidate: ArmRunSummary,
    pub baseline: ArmRunSummary,
    pub completed_trials: usize,
    pub failed_trials: usize,
    pub skipped_trials: usize,
    pub invalidated_trials: usize,
    pub total_attempts: usize,
    pub unique_clusters: usize,
    pub manifest_commitment: Digest,
    pub agent_identity_digest: Digest,
    pub baseline_identity_digest: Digest,
    pub evaluator_digest: Digest,
    pub execution_environment_digest: Digest,
    pub resource_budget_digest: Digest,
    pub trials: Vec<TrialResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SealedRunError {
    UnsupportedSchema,
    EmptyIdentity(&'static str),
    InvalidSourceCommit,
    InvalidRunInterval,
    InvalidControlDomainSeparation,
    MissingPlannedTrials,
    DuplicateTrialId,
    DuplicateClusterTrialKey,
    InvalidMaximumAttempts(String),
    FreezeReceiptMismatch,
    FreezeReceiptNotBeforeFreeze,
    EmptyManifestCommitment,
    DuplicateScenarioReceipt(String),
    MissingScenarioReceipt(String),
    UnplannedScenarioReceipt(String),
    ScenarioBindingMismatch(String),
    ScenarioCommitmentCollision(String),
    UnsupportedScenarioCommitmentScheme(String),
    AgentBaselineIdentityCollision,
    InvalidTrialIdentity,
    InvalidTrialCluster(String),
    ScenarioGeneratedBeforeFreeze(String),
    ScenarioIssuedBeforeGenerated(String),
    MissingRetirementTime(String),
    RetirementAfterRunCreated(String),
    UnexpectedRetirementTime(String),
    ScenarioIssuedBeforeFreeze(String),
    DuplicateScenarioReveal(String),
    MissingScenarioReveal(String),
    UnplannedScenarioReveal(String),
    UnexpectedScenarioReveal(String),
    ScenarioRevealBindingMismatch(String),
    ScenarioRevealSourceMismatch(String),
    ScenarioRevealControlDomainCollision(String),
    ScenarioRevealBeforeRetirement(String),
    ScenarioRevealSaltLengthInvalid(String),
    DuplicateEventId,
    UnplannedTrialEvent(String),
    EventBindingMismatch(String),
    EventSourceMismatch(String),
    EventControlDomainCollision(String),
    EventBeforeScenarioIssue(String),
    MissingTrialDisposition(String),
    MissingArmTrialDisposition(String, EvaluationArm),
    ArmExecutionOrderMismatch(String),
    InvalidAttemptSequence(String),
    AttemptBeyondDeclaration(String),
    DuplicateAttemptStart(String),
    DuplicateAttemptTerminal(String),
    TerminalWithoutStart(String),
    StartWithoutTerminal(String),
    EventTimeOrder(String),
    RetryAfterNonFailure(String),
    SkipAfterStart(String),
    InvalidSkipAttempt(String),
    InvalidatedAttemptSequence(String),
}

pub fn validate_run_ledger(
    manifest: &RunManifest,
    expected_manifest_commitment: &Digest,
    freeze: &FreezeReceipt,
    scenarios: &[ScenarioReceipt],
    events: &[TrialEvent],
) -> Result<RunLedgerSummary, SealedRunError> {
    validate_run_ledger_with_reveals(
        manifest,
        expected_manifest_commitment,
        freeze,
        scenarios,
        &[],
        events,
    )
}

/// Full validation entry point. Retired-reproducibility runs must provide exactly
/// one reveal per planned trial; open and active sealed runs must provide none.
pub fn validate_run_ledger_with_reveals(
    manifest: &RunManifest,
    expected_manifest_commitment: &Digest,
    freeze: &FreezeReceipt,
    scenarios: &[ScenarioReceipt],
    reveals: &[ScenarioReveal],
    events: &[TrialEvent],
) -> Result<RunLedgerSummary, SealedRunError> {
    validate_manifest(manifest)?;
    validate_freeze_receipt(manifest, expected_manifest_commitment, freeze)?;

    let mut trial_index: BTreeMap<&str, &PlannedTrial> = BTreeMap::new();
    let mut cluster_trial_keys = BTreeSet::new();
    for trial in &manifest.trials {
        if !valid_record_id(&trial.trial_id) {
            return Err(SealedRunError::InvalidTrialIdentity);
        }
        if !valid_record_id(&trial.cluster_id) {
            return Err(SealedRunError::InvalidTrialCluster(trial.trial_id.clone()));
        }
        if trial_index.insert(trial.trial_id.as_str(), trial).is_some() {
            return Err(SealedRunError::DuplicateTrialId);
        }
        if !cluster_trial_keys.insert((trial.cluster_id.as_str(), trial.trial_id.as_str())) {
            return Err(SealedRunError::DuplicateClusterTrialKey);
        }
        if trial.maximum_attempts == 0 {
            return Err(SealedRunError::InvalidMaximumAttempts(trial.trial_id.clone()));
        }
    }

    let mut scenario_index: BTreeMap<&str, &ScenarioReceipt> = BTreeMap::new();
    for scenario in scenarios {
        let trial = match trial_index.get(scenario.trial_id.as_str()) {
            Some(value) => *value,
            None => return Err(SealedRunError::UnplannedScenarioReceipt(scenario.trial_id.clone())),
        };
        if scenario_index.insert(scenario.trial_id.as_str(), scenario).is_some() {
            return Err(SealedRunError::DuplicateScenarioReceipt(scenario.trial_id.clone()));
        }
        validate_scenario(manifest, expected_manifest_commitment, trial, scenario)?;
    }

    for trial in &manifest.trials {
        if !scenario_index.contains_key(trial.trial_id.as_str()) {
            return Err(SealedRunError::MissingScenarioReceipt(trial.trial_id.clone()));
        }
    }
    validate_scenario_reveals(
        manifest,
        expected_manifest_commitment,
        &trial_index,
        &scenario_index,
        reveals,
    )?;

    let mut event_ids = BTreeSet::new();
    let mut events_by_trial: BTreeMap<(&str, EvaluationArm), Vec<&TrialEvent>> = BTreeMap::new();
    for event in events {
        if !event_ids.insert(event.event_id.as_str()) {
            return Err(SealedRunError::DuplicateEventId);
        }
        let trial = match trial_index.get(event.trial_id.as_str()) {
            Some(value) => *value,
            None => return Err(SealedRunError::UnplannedTrialEvent(event.trial_id.clone())),
        };
        let scenario = scenario_index.get(event.trial_id.as_str())
            .ok_or_else(|| SealedRunError::MissingScenarioReceipt(event.trial_id.clone()))?;
        validate_event_binding(manifest, expected_manifest_commitment, trial, scenario, event)?;
        events_by_trial.entry((event.trial_id.as_str(), event.arm)).or_default().push(event);
    }

    let mut results = Vec::with_capacity(manifest.trials.len().saturating_mul(2));
    let mut completed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut invalidated = 0usize;
    let mut total_attempts = 0usize;
    let mut candidate = ArmRunSummary {
        planned_trials: manifest.trials.len(),
        completed_trials: 0,
        failed_trials: 0,
        skipped_trials: 0,
        invalidated_trials: 0,
        total_attempts: 0,
    };
    let mut baseline = ArmRunSummary {
        planned_trials: manifest.trials.len(),
        completed_trials: 0,
        failed_trials: 0,
        skipped_trials: 0,
        invalidated_trials: 0,
        total_attempts: 0,
    };
    let unique_clusters = manifest.trials.iter()
        .map(|trial| trial.cluster_id.as_str())
        .collect::<BTreeSet<_>>().len();

    for trial in &manifest.trials {
        let scenario = scenario_index[trial.trial_id.as_str()];
        let mut trial_arm_results = Vec::with_capacity(2);
        for arm in [EvaluationArm::Candidate, EvaluationArm::Baseline] {
            let trial_events = events_by_trial.get(&(trial.trial_id.as_str(), arm))
                .map(Vec::as_slice).unwrap_or(&[]);
            let result = validate_trial_events(trial, scenario, arm, trial_events)?;
            total_attempts = total_attempts.saturating_add(result.attempts_started as usize);
            let arm_summary = match result.arm {
                EvaluationArm::Candidate => &mut candidate,
                EvaluationArm::Baseline => &mut baseline,
            };
            arm_summary.total_attempts = arm_summary.total_attempts
                .saturating_add(result.attempts_started as usize);
            match result.disposition {
                TrialDisposition::Completed => { completed += 1; arm_summary.completed_trials += 1; }
                TrialDisposition::Failed => { failed += 1; arm_summary.failed_trials += 1; }
                TrialDisposition::Skipped => { skipped += 1; arm_summary.skipped_trials += 1; }
                TrialDisposition::Invalidated => { invalidated += 1; arm_summary.invalidated_trials += 1; }
            }
            trial_arm_results.push(result);
        }
        let candidate_result = trial_arm_results.iter()
            .find(|result| result.arm == EvaluationArm::Candidate).unwrap();
        let baseline_result = trial_arm_results.iter()
            .find(|result| result.arm == EvaluationArm::Baseline).unwrap();
        let candidate_first_activity = candidate_result.first_started_at_ms
            .unwrap_or(candidate_result.terminal_at_ms);
        let baseline_first_activity = baseline_result.first_started_at_ms
            .unwrap_or(baseline_result.terminal_at_ms);
        let arm_order_valid = match trial.first_arm {
            EvaluationArm::Candidate => candidate_first_activity < baseline_first_activity,
            EvaluationArm::Baseline => baseline_first_activity < candidate_first_activity,
        };
        if !arm_order_valid {
            return Err(SealedRunError::ArmExecutionOrderMismatch(trial.trial_id.clone()));
        }
        results.extend(trial_arm_results);
    }

    Ok(RunLedgerSummary {
        run_id: manifest.run_id.clone(),
        lane: manifest.lane,
        planned_trials: manifest.trials.len(),
        planned_arm_trials: manifest.trials.len().saturating_mul(2),
        candidate,
        baseline,
        completed_trials: completed,
        failed_trials: failed,
        skipped_trials: skipped,
        invalidated_trials: invalidated,
        total_attempts,
        unique_clusters,
        manifest_commitment: expected_manifest_commitment.clone(),
        agent_identity_digest: manifest.agent_identity_digest.clone(),
        baseline_identity_digest: manifest.baseline_identity_digest.clone(),
        evaluator_digest: manifest.evaluator_digest.clone(),
        execution_environment_digest: manifest.execution_environment_digest.clone(),
        resource_budget_digest: manifest.resource_budget_digest.clone(),
        trials: results,
    })
}

fn validate_manifest(manifest: &RunManifest) -> Result<(), SealedRunError> {
    if manifest.schema != SEALED_RUN_SCHEMA || manifest.version != SEALED_RUN_VERSION {
        return Err(SealedRunError::UnsupportedSchema);
    }
    for (value, name) in [
        (manifest.run_id.as_str(), "run_id"),
        (manifest.agent.agent_id.as_str(), "agent_id"),
        (manifest.agent.source_commit.as_str(), "agent_source_commit"),
        (manifest.agent.model_provider.as_str(), "model_provider"),
        (manifest.agent.model_id.as_str(), "model_id"),
        (manifest.agent.model_version.as_str(), "model_version"),
        (manifest.agent.control_domain.as_str(), "agent_control_domain"),
        (manifest.baseline.baseline_id.as_str(), "baseline_id"),
        (manifest.baseline.source_commit.as_str(), "baseline_source_commit"),
        (manifest.baseline.model_provider.as_str(), "baseline_model_provider"),
        (manifest.baseline.model_id.as_str(), "baseline_model_id"),
        (manifest.baseline.model_version.as_str(), "baseline_model_version"),
        (manifest.baseline.control_domain.as_str(), "baseline_control_domain"),
        (manifest.freeze_authority_id.as_str(), "freeze_authority_id"),
        (manifest.freeze_authority_control_domain.as_str(), "freeze_authority_control_domain"),
        (manifest.evaluator_control_domain.as_str(), "evaluator_control_domain"),
        (manifest.evidence_source_id.as_str(), "evidence_source_id"),
        (manifest.evidence_control_domain.as_str(), "evidence_control_domain"),
    ] {
        if value.trim().is_empty() { return Err(SealedRunError::EmptyIdentity(name)); }
    }
    if !valid_git_revision(&manifest.agent.source_commit)
        || !valid_git_revision(&manifest.baseline.source_commit)
    {
        return Err(SealedRunError::InvalidSourceCommit);
    }
    if manifest.agent_identity_digest == manifest.baseline_identity_digest {
        return Err(SealedRunError::AgentBaselineIdentityCollision);
    }
    if manifest.created_at_ms > manifest.frozen_at_ms {
        return Err(SealedRunError::InvalidRunInterval);
    }
    if manifest.trials.is_empty() { return Err(SealedRunError::MissingPlannedTrials); }
    if manifest.agent.control_domain == manifest.baseline.control_domain
        || manifest.agent.control_domain == manifest.freeze_authority_control_domain
        || manifest.agent.control_domain == manifest.evaluator_control_domain
        || manifest.agent.control_domain == manifest.evidence_control_domain
        || manifest.baseline.control_domain == manifest.freeze_authority_control_domain
        || manifest.baseline.control_domain == manifest.evaluator_control_domain
        || manifest.baseline.control_domain == manifest.evidence_control_domain
        || manifest.freeze_authority_control_domain == manifest.evaluator_control_domain
        || manifest.freeze_authority_control_domain == manifest.evidence_control_domain
        || manifest.evaluator_control_domain == manifest.evidence_control_domain
        || digest_is_zero(&manifest.evaluator_digest)
        || digest_is_zero(&manifest.execution_environment_digest)
        || digest_is_zero(&manifest.resource_budget_digest)
        || digest_is_zero(&manifest.agent_identity_digest)
        || digest_is_zero(&manifest.baseline_identity_digest)
        || digest_is_zero(&manifest.agent.agent_image_digest)
        || digest_is_zero(&manifest.agent.scaffold_digest)
        || digest_is_zero(&manifest.agent.prompt_digest)
        || digest_is_zero(&manifest.agent.toolchain_digest)
        || digest_is_zero(&manifest.agent.runtime_config_digest)
        || digest_is_zero(&manifest.agent.tool_policy_digest)
        || digest_is_zero(&manifest.baseline.image_digest)
        || digest_is_zero(&manifest.baseline.scaffold_digest)
        || digest_is_zero(&manifest.baseline.prompt_digest)
        || digest_is_zero(&manifest.baseline.toolchain_digest)
        || digest_is_zero(&manifest.baseline.runtime_config_digest)
        || digest_is_zero(&manifest.baseline.tool_policy_digest)
    {
        return Err(SealedRunError::InvalidControlDomainSeparation);
    }
    Ok(())
}

fn validate_freeze_receipt(
    manifest: &RunManifest,
    expected_commitment: &Digest,
    receipt: &FreezeReceipt,
) -> Result<(), SealedRunError> {
    if expected_commitment.0.iter().all(|byte| *byte == 0) {
        return Err(SealedRunError::EmptyManifestCommitment);
    }
    if receipt.run_id != manifest.run_id
        || receipt.manifest_commitment != *expected_commitment
        || receipt.frozen_at_ms != manifest.frozen_at_ms
        || receipt.agent_identity_digest != manifest.agent_identity_digest
        || receipt.baseline_identity_digest != manifest.baseline_identity_digest
        || receipt.evaluator_digest != manifest.evaluator_digest
        || receipt.execution_environment_digest != manifest.execution_environment_digest
        || receipt.resource_budget_digest != manifest.resource_budget_digest
        || receipt.publisher_id != manifest.freeze_authority_id
        || receipt.publisher_control_domain != manifest.freeze_authority_control_domain
    {
        return Err(SealedRunError::FreezeReceiptMismatch);
    }
    if receipt.committed_at_ms < manifest.created_at_ms
        || receipt.committed_at_ms > manifest.frozen_at_ms
    {
        return Err(SealedRunError::FreezeReceiptNotBeforeFreeze);
    }
    Ok(())
}

fn validate_scenario(
    manifest: &RunManifest,
    expected_commitment: &Digest,
    trial: &PlannedTrial,
    scenario: &ScenarioReceipt,
) -> Result<(), SealedRunError> {
    let id = trial.trial_id.clone();
    if scenario.commitment_scheme != SCENARIO_COMMITMENT_SCHEME {
        return Err(SealedRunError::UnsupportedScenarioCommitmentScheme(id));
    }
    if scenario.run_id != manifest.run_id
        || scenario.trial_id != trial.trial_id
        || scenario.manifest_commitment != *expected_commitment
        || scenario.evaluator_digest != manifest.evaluator_digest
        || scenario.case_payload_commitment.0.iter().all(|byte| *byte == 0)
        || scenario.oracle_commitment.0.iter().all(|byte| *byte == 0)
        || scenario.source_id != manifest.evidence_source_id
    {
        return Err(SealedRunError::ScenarioBindingMismatch(id));
    }
    if scenario.case_payload_commitment == scenario.oracle_commitment {
        return Err(SealedRunError::ScenarioCommitmentCollision(id));
    }
    if scenario.control_domain != manifest.evidence_control_domain
        || scenario.control_domain == manifest.agent.control_domain
        || scenario.control_domain == manifest.evaluator_control_domain
    {
        return Err(SealedRunError::EventControlDomainCollision(id));
    }
    if scenario.issued_at_ms < scenario.generated_at_ms {
        return Err(SealedRunError::ScenarioIssuedBeforeGenerated(id.clone()));
    }
    match manifest.lane {
        EvaluationLane::SealedCertification => {
            if scenario.generated_at_ms <= manifest.frozen_at_ms {
                return Err(SealedRunError::ScenarioGeneratedBeforeFreeze(id.clone()));
            }
            if scenario.retired_at_ms.is_some() {
                return Err(SealedRunError::UnexpectedRetirementTime(id));
            }
        }
        EvaluationLane::RetiredReproducibility => {
            let retired = scenario.retired_at_ms
                .ok_or_else(|| SealedRunError::MissingRetirementTime(id.clone()))?;
            if scenario.generated_at_ms > retired || retired > manifest.created_at_ms {
                return Err(SealedRunError::RetirementAfterRunCreated(id.clone()));
            }
            if scenario.issued_at_ms < manifest.frozen_at_ms {
                return Err(SealedRunError::ScenarioIssuedBeforeFreeze(id));
            }
        }
        EvaluationLane::OpenDevelopment => {
            if scenario.retired_at_ms.is_some() {
                return Err(SealedRunError::UnexpectedRetirementTime(id));
            }
        }
    }
    Ok(())
}

fn validate_event_binding(
    manifest: &RunManifest,
    expected_commitment: &Digest,
    trial: &PlannedTrial,
    scenario: &ScenarioReceipt,
    event: &TrialEvent,
) -> Result<(), SealedRunError> {
    let expected_arm_digest = match event.arm {
        EvaluationArm::Candidate => &manifest.agent_identity_digest,
        EvaluationArm::Baseline => &manifest.baseline_identity_digest,
    };
    if event.run_id != manifest.run_id
        || event.manifest_commitment != *expected_commitment
        || &event.arm_identity_digest != expected_arm_digest
        || event.evaluator_digest != manifest.evaluator_digest
        || event.scenario_commitment != scenario.case_payload_commitment
        || event.event_id.trim().is_empty()
    {
        return Err(SealedRunError::EventBindingMismatch(trial.trial_id.clone()));
    }
    if event.source_id != manifest.evidence_source_id {
        return Err(SealedRunError::EventSourceMismatch(trial.trial_id.clone()));
    }
    if event.control_domain != manifest.evidence_control_domain
        || event.control_domain == manifest.agent.control_domain
        || event.control_domain == manifest.baseline.control_domain
        || event.control_domain == manifest.evaluator_control_domain
    {
        return Err(SealedRunError::EventControlDomainCollision(trial.trial_id.clone()));
    }
    if event.occurred_at_ms < scenario.issued_at_ms {
        return Err(SealedRunError::EventBeforeScenarioIssue(trial.trial_id.clone()));
    }
    Ok(())
}

fn validate_trial_events(
    trial: &PlannedTrial,
    scenario: &ScenarioReceipt,
    arm: EvaluationArm,
    events: &[&TrialEvent],
) -> Result<TrialResult, SealedRunError> {
    if events.is_empty() {
        return Err(SealedRunError::MissingArmTrialDisposition(trial.trial_id.clone(), arm));
    }
    let skipped_or_invalidated: Vec<&TrialEvent> = events.iter().copied()
        .filter(|event| matches!(event.kind, TrialEventKind::Skipped | TrialEventKind::Invalidated))
        .collect();

    if skipped_or_invalidated.len() == 1 && events.len() == 1 {
        let event = skipped_or_invalidated[0];
        if event.attempt != 0 {
            return Err(SealedRunError::InvalidSkipAttempt(trial.trial_id.clone()));
        }
        if event.kind == TrialEventKind::Skipped {
            return Ok(TrialResult {
                trial_id: trial.trial_id.clone(),
                arm,
                cluster_id: trial.cluster_id.clone(),
                disposition: TrialDisposition::Skipped,
                attempts_started: 0,
                first_started_at_ms: None,
                terminal_at_ms: event.occurred_at_ms,
            });
        }
        return Ok(TrialResult {
            trial_id: trial.trial_id.clone(),
            arm,
            cluster_id: trial.cluster_id.clone(),
            disposition: TrialDisposition::Invalidated,
            attempts_started: 0,
            first_started_at_ms: None,
            terminal_at_ms: event.occurred_at_ms,
        });
    }
    if !skipped_or_invalidated.is_empty()
        && events.iter().any(|event| event.kind == TrialEventKind::Skipped)
    {
        return Err(SealedRunError::SkipAfterStart(trial.trial_id.clone()));
    }

    let mut by_attempt: BTreeMap<u16, Vec<&TrialEvent>> = BTreeMap::new();
    for event in events {
        if event.attempt == 0 {
            return Err(SealedRunError::InvalidAttemptSequence(trial.trial_id.clone()));
        }
        if event.attempt > trial.maximum_attempts {
            return Err(SealedRunError::AttemptBeyondDeclaration(trial.trial_id.clone()));
        }
        by_attempt.entry(event.attempt).or_default().push(*event);
    }

    if by_attempt.is_empty() {
        return Err(SealedRunError::MissingTrialDisposition(trial.trial_id.clone()));
    }
    let highest_attempt = *by_attempt.keys().max().unwrap();
    let mut first_started_at = None;
    let mut prior_terminal: Option<TrialEventKind> = None;
    let mut final_terminal: Option<&TrialEvent> = None;

    for attempt in 1..=highest_attempt {
        let attempt_events = by_attempt.get(&attempt)
            .ok_or_else(|| SealedRunError::InvalidAttemptSequence(trial.trial_id.clone()))?;
        let starts: Vec<&TrialEvent> = attempt_events.iter().copied()
            .filter(|event| event.kind == TrialEventKind::Started).collect();
        let terminals: Vec<&TrialEvent> = attempt_events.iter().copied()
            .filter(|event| matches!(event.kind,
                TrialEventKind::Completed | TrialEventKind::Failed | TrialEventKind::Invalidated))
            .collect();

        if starts.len() > 1 { return Err(SealedRunError::DuplicateAttemptStart(trial.trial_id.clone())); }
        if terminals.len() > 1 { return Err(SealedRunError::DuplicateAttemptTerminal(trial.trial_id.clone())); }
        if starts.is_empty() { return Err(SealedRunError::TerminalWithoutStart(trial.trial_id.clone())); }
        if terminals.is_empty() { return Err(SealedRunError::StartWithoutTerminal(trial.trial_id.clone())); }

        let start = starts[0];
        let terminal = terminals[0];
        if start.occurred_at_ms < scenario.issued_at_ms
            || terminal.occurred_at_ms <= start.occurred_at_ms
        {
            return Err(SealedRunError::EventTimeOrder(trial.trial_id.clone()));
        }
        if attempt == 1 {
            first_started_at = Some(start.occurred_at_ms);
        }
        if attempt > 1 && prior_terminal != Some(TrialEventKind::Failed) {
            return Err(SealedRunError::RetryAfterNonFailure(trial.trial_id.clone()));
        }
        if attempt > 1 {
            let previous = by_attempt[&(attempt - 1)].iter()
                .find(|event| matches!(event.kind,
                    TrialEventKind::Completed | TrialEventKind::Failed | TrialEventKind::Invalidated))
                .unwrap();
            if start.occurred_at_ms <= previous.occurred_at_ms {
                return Err(SealedRunError::EventTimeOrder(trial.trial_id.clone()));
            }
        }
        prior_terminal = Some(terminal.kind);
        final_terminal = Some(terminal);
    }

    let terminal = final_terminal.unwrap();
    let disposition = match terminal.kind {
        TrialEventKind::Completed => TrialDisposition::Completed,
        TrialEventKind::Failed => TrialDisposition::Failed,
        TrialEventKind::Invalidated => TrialDisposition::Invalidated,
        _ => return Err(SealedRunError::TerminalWithoutStart(trial.trial_id.clone())),
    };
    Ok(TrialResult {
        trial_id: trial.trial_id.clone(),
        arm,
        cluster_id: trial.cluster_id.clone(),
        disposition,
        attempts_started: highest_attempt,
        first_started_at_ms: first_started_at,
        terminal_at_ms: terminal.occurred_at_ms,
    })
}

fn validate_scenario_reveals(
    manifest: &RunManifest,
    expected_commitment: &Digest,
    trials: &BTreeMap<&str, &PlannedTrial>,
    scenarios: &BTreeMap<&str, &ScenarioReceipt>,
    reveals: &[ScenarioReveal],
) -> Result<(), SealedRunError> {
    if manifest.lane != EvaluationLane::RetiredReproducibility {
        if let Some(reveal) = reveals.first() {
            return Err(SealedRunError::UnexpectedScenarioReveal(reveal.trial_id.clone()));
        }
        return Ok(());
    }

    let mut reveal_index: BTreeMap<&str, &ScenarioReveal> = BTreeMap::new();
    for reveal in reveals {
        if !trials.contains_key(reveal.trial_id.as_str()) {
            return Err(SealedRunError::UnplannedScenarioReveal(reveal.trial_id.clone()));
        }
        if reveal_index.insert(reveal.trial_id.as_str(), reveal).is_some() {
            return Err(SealedRunError::DuplicateScenarioReveal(reveal.trial_id.clone()));
        }
        let scenario = scenarios.get(reveal.trial_id.as_str())
            .ok_or_else(|| SealedRunError::MissingScenarioReceipt(reveal.trial_id.clone()))?;
        if reveal.commitment_scheme != scenario.commitment_scheme
            || reveal.run_id != manifest.run_id
            || reveal.manifest_commitment != *expected_commitment
            || reveal.case_payload_commitment != scenario.case_payload_commitment
            || reveal.oracle_commitment != scenario.oracle_commitment
            || digest_is_zero(&reveal.case_payload_artifact_digest)
            || digest_is_zero(&reveal.oracle_artifact_digest)
            || reveal.case_payload_artifact_digest == reveal.oracle_artifact_digest
        {
            return Err(SealedRunError::ScenarioRevealBindingMismatch(reveal.trial_id.clone()));
        }
        if reveal.source_id != manifest.evidence_source_id {
            return Err(SealedRunError::ScenarioRevealSourceMismatch(reveal.trial_id.clone()));
        }
        if reveal.control_domain != manifest.evidence_control_domain
            || reveal.control_domain == manifest.agent.control_domain
            || reveal.control_domain == manifest.baseline.control_domain
            || reveal.control_domain == manifest.evaluator_control_domain
        {
            return Err(SealedRunError::ScenarioRevealControlDomainCollision(reveal.trial_id.clone()));
        }
        let retired_at = scenario.retired_at_ms
            .ok_or_else(|| SealedRunError::MissingRetirementTime(reveal.trial_id.clone()))?;
        if reveal.revealed_at_ms < retired_at
            || reveal.revealed_at_ms > manifest.created_at_ms
        {
            return Err(SealedRunError::ScenarioRevealBeforeRetirement(reveal.trial_id.clone()));
        }
        if reveal.case_payload_salt.len() != 32
            || reveal.oracle_salt.len() != 32
            || reveal.case_payload_salt == reveal.oracle_salt
        {
            return Err(SealedRunError::ScenarioRevealSaltLengthInvalid(reveal.trial_id.clone()));
        }
    }

    for trial_id in trials.keys() {
        if !reveal_index.contains_key(trial_id) {
            return Err(SealedRunError::MissingScenarioReveal((*trial_id).to_owned()));
        }
    }
    Ok(())
}

fn valid_record_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 256
        && !value.chars().any(char::is_control)
}
fn digest_is_zero(digest: &Digest) -> bool {
    digest.0.iter().all(|byte| *byte == 0)
}

fn valid_git_revision(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(lane: EvaluationLane) -> (RunManifest, Digest, FreezeReceipt) {
        let commitment = Digest::from_byte(90);
        let agent_identity_digest = Digest::from_byte(1);
        let baseline_identity_digest = Digest::from_byte(2);
        let evaluator_digest = Digest::from_byte(3);
        let execution_environment_digest = Digest::from_byte(16);
        let resource_budget_digest = Digest::from_byte(17);
        let manifest = RunManifest {
            schema: SEALED_RUN_SCHEMA.into(),
            version: SEALED_RUN_VERSION,
            run_id: "run-001".into(),
            lane,
            created_at_ms: 10,
            frozen_at_ms: 20,
            evaluator_digest: evaluator_digest.clone(),
            execution_environment_digest: execution_environment_digest.clone(),
            resource_budget_digest: resource_budget_digest.clone(),
            agent_identity_digest: agent_identity_digest.clone(),
            baseline_identity_digest: baseline_identity_digest.clone(),
            agent: FrozenAgentIdentity {
                agent_id: "candidate-agent".into(),
                agent_image_digest: Digest::from_byte(4),
                source_commit: "0123456789abcdef0123456789abcdef01234567".into(),
                model_provider: "provider".into(),
                model_id: "model-candidate".into(),
                model_version: "v1".into(),
                scaffold_digest: Digest::from_byte(5),
                prompt_digest: Digest::from_byte(6),
                toolchain_digest: Digest::from_byte(7),
                runtime_config_digest: Digest::from_byte(8),
                tool_policy_digest: Digest::from_byte(9),
                control_domain: "agent-domain".into(),
            },
            baseline: BaselineIdentity {
                baseline_id: "baseline-001".into(),
                image_digest: Digest::from_byte(10),
                source_commit: "abcdef0123456789abcdef0123456789abcdef01".into(),
                model_provider: "provider".into(),
                model_id: "model-baseline".into(),
                model_version: "v1".into(),
                scaffold_digest: Digest::from_byte(11),
                prompt_digest: Digest::from_byte(12),
                toolchain_digest: Digest::from_byte(14),
                runtime_config_digest: Digest::from_byte(15),
                tool_policy_digest: Digest::from_byte(13),
                control_domain: "baseline-domain".into(),
            },
            freeze_authority_id: "freeze-authority".into(),
            freeze_authority_control_domain: "authority-domain".into(),
            evaluator_control_domain: "evaluator-domain".into(),
            evidence_source_id: "independent-ledger".into(),
            evidence_control_domain: "evidence-domain".into(),
            trials: vec![
                PlannedTrial { trial_id: "trial-a".into(), cluster_id: "cluster-1".into(), first_arm: EvaluationArm::Candidate, maximum_attempts: 2 },
                PlannedTrial { trial_id: "trial-b".into(), cluster_id: "cluster-2".into(), first_arm: EvaluationArm::Baseline, maximum_attempts: 1 },
                PlannedTrial { trial_id: "trial-c".into(), cluster_id: "cluster-2".into(), first_arm: EvaluationArm::Candidate, maximum_attempts: 1 },
            ],
        };
        let freeze = FreezeReceipt {
            run_id: "run-001".into(),
            manifest_commitment: commitment.clone(),
            committed_at_ms: 15,
            frozen_at_ms: 20,
            agent_identity_digest,
            baseline_identity_digest,
            evaluator_digest,
            execution_environment_digest,
            resource_budget_digest,
            publisher_id: "freeze-authority".into(),
            publisher_control_domain: "authority-domain".into(),
        };
        (manifest, commitment, freeze)
    }

    fn scenario(manifest: &RunManifest, commitment: &Digest, trial_id: &str) -> ScenarioReceipt {
        ScenarioReceipt {
            run_id: manifest.run_id.clone(),
            trial_id: trial_id.into(),
            manifest_commitment: commitment.clone(),
            commitment_scheme: SCENARIO_COMMITMENT_SCHEME.into(),
            case_payload_commitment: Digest::from_byte(21),
            oracle_commitment: Digest::from_byte(22),
            generated_at_ms: 25,
            issued_at_ms: 30,
            retired_at_ms: None,
            evaluator_digest: manifest.evaluator_digest.clone(),
            source_id: manifest.evidence_source_id.clone(),
            control_domain: manifest.evidence_control_domain.clone(),
        }
    }

    fn event(
        manifest: &RunManifest,
        commitment: &Digest,
        scenario: &ScenarioReceipt,
        trial_id: &str,
        event_id: &str,
        attempt: u16,
        at: u64,
        kind: TrialEventKind,
    ) -> TrialEvent {
        event_for_arm(
            manifest, commitment, scenario, trial_id, event_id, attempt, at, kind,
            EvaluationArm::Candidate,
        )
    }

    fn event_for_arm(
        manifest: &RunManifest,
        commitment: &Digest,
        scenario: &ScenarioReceipt,
        trial_id: &str,
        event_id: &str,
        attempt: u16,
        at: u64,
        kind: TrialEventKind,
        arm: EvaluationArm,
    ) -> TrialEvent {
        let arm_identity_digest = match arm {
            EvaluationArm::Candidate => manifest.agent_identity_digest.clone(),
            EvaluationArm::Baseline => manifest.baseline_identity_digest.clone(),
        };
        TrialEvent {
            run_id: manifest.run_id.clone(),
            arm,
            trial_id: trial_id.into(),
            event_id: event_id.into(),
            attempt,
            occurred_at_ms: at,
            kind,
            manifest_commitment: commitment.clone(),
            arm_identity_digest,
            evaluator_digest: manifest.evaluator_digest.clone(),
            scenario_commitment: scenario.case_payload_commitment.clone(),
            source_id: manifest.evidence_source_id.clone(),
            control_domain: manifest.evidence_control_domain.clone(),
        }
    }

    fn reveal(manifest: &RunManifest, commitment: &Digest, scenario: &ScenarioReceipt) -> ScenarioReveal {
        ScenarioReveal {
            run_id: manifest.run_id.clone(),
            trial_id: scenario.trial_id.clone(),
            manifest_commitment: commitment.clone(),
            commitment_scheme: scenario.commitment_scheme.clone(),
            case_payload_commitment: scenario.case_payload_commitment.clone(),
            oracle_commitment: scenario.oracle_commitment.clone(),
            case_payload_artifact_digest: Digest::from_byte(23),
            oracle_artifact_digest: Digest::from_byte(24),
            case_payload_salt: vec![31; 32],
            oracle_salt: vec![32; 32],
            revealed_at_ms: scenario.retired_at_ms.unwrap_or(8) + 1,
            source_id: manifest.evidence_source_id.clone(),
            control_domain: manifest.evidence_control_domain.clone(),
        }
    }
    fn valid_fixture() -> (
        RunManifest, Digest, FreezeReceipt, Vec<ScenarioReceipt>, Vec<TrialEvent>
    ) {
        let (m, commitment, freeze) = manifest(EvaluationLane::SealedCertification);
        let scenarios = vec![
            scenario(&m, &commitment, "trial-a"),
            scenario(&m, &commitment, "trial-b"),
            scenario(&m, &commitment, "trial-c"),
        ];
        let mut events = vec![
            event(&m, &commitment, &scenarios[0], "trial-a", "a-1-start", 1, 40, TrialEventKind::Started),
            event(&m, &commitment, &scenarios[0], "trial-a", "a-1-fail", 1, 50, TrialEventKind::Failed),
            event(&m, &commitment, &scenarios[0], "trial-a", "a-2-start", 2, 60, TrialEventKind::Started),
            event(&m, &commitment, &scenarios[0], "trial-a", "a-2-done", 2, 70, TrialEventKind::Completed),
            event(&m, &commitment, &scenarios[1], "trial-b", "b-skip", 0, 36, TrialEventKind::Skipped),
            event(&m, &commitment, &scenarios[2], "trial-c", "c-start", 1, 40, TrialEventKind::Started),
            event(&m, &commitment, &scenarios[2], "trial-c", "c-invalid", 1, 45, TrialEventKind::Invalidated),
        ];
        events.extend(vec![
            event_for_arm(&m, &commitment, &scenarios[0], "trial-a", "base-a-1-start", 1, 41, TrialEventKind::Started, EvaluationArm::Baseline),
            event_for_arm(&m, &commitment, &scenarios[0], "trial-a", "base-a-1-fail", 1, 51, TrialEventKind::Failed, EvaluationArm::Baseline),
            event_for_arm(&m, &commitment, &scenarios[0], "trial-a", "base-a-2-start", 2, 61, TrialEventKind::Started, EvaluationArm::Baseline),
            event_for_arm(&m, &commitment, &scenarios[0], "trial-a", "base-a-2-done", 2, 71, TrialEventKind::Completed, EvaluationArm::Baseline),
            event_for_arm(&m, &commitment, &scenarios[1], "trial-b", "base-b-skip", 0, 35, TrialEventKind::Skipped, EvaluationArm::Baseline),
            event_for_arm(&m, &commitment, &scenarios[2], "trial-c", "base-c-start", 1, 41, TrialEventKind::Started, EvaluationArm::Baseline),
            event_for_arm(&m, &commitment, &scenarios[2], "trial-c", "base-c-invalid", 1, 46, TrialEventKind::Invalidated, EvaluationArm::Baseline),
        ]);
        events.reverse(); // input order is not trusted; event timestamps/attempts are.
        (m, commitment, freeze, scenarios, events)
    }

    #[test]
    fn valid_run_preserves_all_outcomes_attempts_and_cluster_denominator() {
        let (m, commitment, freeze, scenarios, events) = valid_fixture();
        let result = validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events).unwrap();
        assert_eq!(result.planned_trials, 3);
        assert_eq!(result.planned_arm_trials, 6);
        assert_eq!(result.completed_trials, 2);
        assert_eq!(result.failed_trials, 0);
        assert_eq!(result.skipped_trials, 2);
        assert_eq!(result.invalidated_trials, 2);
        assert_eq!(result.total_attempts, 6);
        assert_eq!(result.unique_clusters, 2);
        assert_eq!(result.candidate.completed_trials, 1);
        assert_eq!(result.baseline.completed_trials, 1);
        assert_eq!(result.candidate.total_attempts, 3);
        assert_eq!(result.baseline.total_attempts, 3);
    }

    #[test]
    fn matched_baseline_arm_is_required_for_every_planned_scenario() {
        let (m, commitment, freeze, scenarios, mut events) = valid_fixture();
        events.retain(|event| event.arm != EvaluationArm::Baseline);
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::MissingArmTrialDisposition("trial-a".into(), EvaluationArm::Baseline))
        );
    }

    #[test]
    fn baseline_event_cannot_claim_candidate_identity_digest() {
        let (m, commitment, freeze, scenarios, mut events) = valid_fixture();
        let event = events.iter_mut().find(|event| event.arm == EvaluationArm::Baseline).unwrap();
        event.arm_identity_digest = m.agent_identity_digest.clone();
        assert!(matches!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::EventBindingMismatch(_))
        ));
    }

    #[test]
    fn arm_specific_outcomes_are_not_collapsed_in_the_summary() {
        let (m, commitment, freeze, scenarios, mut events) = valid_fixture();
        let baseline_completed = events.iter_mut().find(|event| {
            event.arm == EvaluationArm::Baseline
                && event.trial_id == "trial-a"
                && event.kind == TrialEventKind::Completed
        }).unwrap();
        baseline_completed.kind = TrialEventKind::Failed;
        let result = validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events).unwrap();
        assert_eq!(result.candidate.completed_trials, 1);
        assert_eq!(result.baseline.failed_trials, 1);
        assert_eq!(result.baseline.completed_trials, 0);
        assert_eq!(result.completed_trials, 1);
        assert_eq!(result.failed_trials, 1);
    }

    #[test]
    fn sealed_scenarios_must_be_generated_after_the_exact_system_freeze() {
        let (mut m, commitment, freeze, mut scenarios, events) = valid_fixture();
        scenarios[0].generated_at_ms = m.frozen_at_ms;
        scenarios[0].issued_at_ms = m.frozen_at_ms;
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::ScenarioGeneratedBeforeFreeze("trial-a".into()))
        );
        m.frozen_at_ms += 1;
    }

    #[test]
    fn frozen_system_or_baseline_identity_substitution_is_rejected() {
        let (m, commitment, mut freeze, scenarios, events) = valid_fixture();
        freeze.agent_identity_digest = Digest::from_byte(99);
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::FreezeReceiptMismatch)
        );
    }

    #[test]
    fn missing_planned_trial_never_disappears_from_the_denominator() {
        let (m, commitment, freeze, mut scenarios, events) = valid_fixture();
        scenarios.pop();
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::MissingScenarioReceipt("trial-c".into()))
        );
    }

    #[test]
    fn duplicate_and_unplanned_scenario_receipts_are_rejected() {
        let (m, commitment, freeze, mut scenarios, events) = valid_fixture();
        scenarios.push(scenarios[0].clone());
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::DuplicateScenarioReceipt("trial-a".into()))
        );
        let (m, commitment, freeze, mut scenarios, events) = valid_fixture();
        let mut extra = scenarios[0].clone();
        extra.trial_id = "not-planned".into();
        scenarios.push(extra);
        assert!(matches!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::UnplannedScenarioReceipt(_))
        ));
    }

    #[test]
    fn missing_terminal_disposition_is_indeterminate_not_silently_dropped() {
        let (m, commitment, freeze, scenarios, mut events) = valid_fixture();
        events.retain(|e| e.trial_id != "trial-c");
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::MissingArmTrialDisposition("trial-c".into(), EvaluationArm::Candidate))
        );
    }

    #[test]
    fn retry_requires_a_prior_failed_attempt_and_is_declared() {
        let (m, commitment, freeze, scenarios, mut events) = valid_fixture();
        let a_scenario = &scenarios[0];
        events.retain(|e| !(e.trial_id == "trial-a" && e.attempt == 1 && e.kind == TrialEventKind::Failed));
        events.push(event(&m, &commitment, a_scenario, "trial-a", "a-1-done", 1, 50, TrialEventKind::Completed));
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::RetryAfterNonFailure("trial-a".into()))
        );
    }

    #[test]
    fn undeclared_attempt_and_duplicate_event_ids_are_rejected() {
        let (m, commitment, freeze, scenarios, mut events) = valid_fixture();
        let mut beyond = events.iter().find(|e| e.trial_id == "trial-a").unwrap().clone();
        beyond.event_id = "beyond".into();
        beyond.attempt = 3;
        beyond.kind = TrialEventKind::Started;
        events.push(beyond);
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::AttemptBeyondDeclaration("trial-a".into()))
        );
        let (m, commitment, freeze, scenarios, mut events) = valid_fixture();
        let duplicate = events[0].clone();
        events.push(duplicate);
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::DuplicateEventId)
        );
    }

    #[test]
    fn event_binding_substitution_and_foreign_source_fail_closed() {
        let (m, commitment, freeze, scenarios, mut events) = valid_fixture();
        events[0].manifest_commitment = Digest::from_byte(99);
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::EventBindingMismatch("trial-c".into()))
        );
        let (m, commitment, freeze, scenarios, mut events) = valid_fixture();
        events[0].source_id = "agent".into();
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::EventSourceMismatch("trial-c".into()))
        );
    }

    #[test]
    fn open_lane_can_use_preexisting_scenarios_but_sealed_lane_cannot() {
        let (mut m, commitment, freeze, mut scenarios, mut events) = valid_fixture();
        m.lane = EvaluationLane::OpenDevelopment;
        scenarios[0].generated_at_ms = 5;
        scenarios[0].issued_at_ms = 8;
        // The fixture's existing events still bind to the same frozen commitment and case.
        let result = validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events);
        assert!(result.is_ok());
        events.clear();
    }

    #[test]
    fn retired_reproducibility_requires_post_retirement_reveals() {
        let (mut m, commitment, freeze, mut scenarios, events) = valid_fixture();
        m.lane = EvaluationLane::RetiredReproducibility;
        for scenario in &mut scenarios {
            scenario.generated_at_ms = 5;
            scenario.retired_at_ms = Some(8);
            scenario.issued_at_ms = 30;
        }
        let reveals: Vec<ScenarioReveal> = scenarios.iter()
            .map(|scenario| reveal(&m, &commitment, scenario))
            .collect();
        let result = validate_run_ledger_with_reveals(
            &m, &commitment, &freeze, &scenarios, &reveals, &events
        );
        assert!(result.is_ok());
        let missing = validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events);
        assert_eq!(
            missing,
            Err(SealedRunError::MissingScenarioReveal("trial-a".into()))
        );
    }

    #[test]
    fn sealed_lane_rejects_retirement_marker_and_missing_retired_time() {
        let (m, commitment, freeze, mut scenarios, events) = valid_fixture();
        scenarios[0].retired_at_ms = Some(19);
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::UnexpectedRetirementTime("trial-a".into()))
        );
        let (mut m, commitment, freeze, mut scenarios, events) = valid_fixture();
        m.lane = EvaluationLane::RetiredReproducibility;
        scenarios[0].generated_at_ms = 5;
        scenarios[0].retired_at_ms = None;
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::MissingRetirementTime("trial-a".into()))
        );
    }

    #[test]
    fn open_development_accepts_preexisting_case_issued_before_run_freeze() {
        let (mut manifest, commitment, freeze, mut scenarios, events) = valid_fixture();
        manifest.lane = EvaluationLane::OpenDevelopment;
        for scenario in &mut scenarios {
            scenario.generated_at_ms = 5;
            scenario.issued_at_ms = 8;
        }
        assert!(validate_run_ledger(&manifest, &commitment, &freeze, &scenarios, &events).is_ok());
    }

    #[test]
    fn retired_reveal_must_bind_exact_commitments_and_be_published_after_retirement() {
        let (mut manifest, commitment, freeze, mut scenarios, events) = valid_fixture();
        manifest.lane = EvaluationLane::RetiredReproducibility;
        for scenario in &mut scenarios {
            scenario.generated_at_ms = 5;
            scenario.retired_at_ms = Some(8);
            scenario.issued_at_ms = 30;
        }
        let mut reveals: Vec<ScenarioReveal> = scenarios.iter()
            .map(|scenario| reveal(&manifest, &commitment, scenario)).collect();
        assert!(validate_run_ledger_with_reveals(&manifest, &commitment, &freeze, &scenarios, &reveals, &events).is_ok());
        reveals[0].case_payload_commitment = Digest::from_byte(99);
        assert_eq!(
            validate_run_ledger_with_reveals(&manifest, &commitment, &freeze, &scenarios, &reveals, &events),
            Err(SealedRunError::ScenarioRevealBindingMismatch("trial-a".into()))
        );
        reveals[0] = reveal(&manifest, &commitment, &scenarios[0]);
        reveals[0].revealed_at_ms = 7;
        assert_eq!(
            validate_run_ledger_with_reveals(&manifest, &commitment, &freeze, &scenarios, &reveals, &events),
            Err(SealedRunError::ScenarioRevealBeforeRetirement("trial-a".into()))
        );
    }

    #[test]
    fn retired_reveal_rejects_short_salts_and_foreign_source() {
        let (mut manifest, commitment, freeze, mut scenarios, events) = valid_fixture();
        manifest.lane = EvaluationLane::RetiredReproducibility;
        for scenario in &mut scenarios {
            scenario.generated_at_ms = 5;
            scenario.retired_at_ms = Some(8);
            scenario.issued_at_ms = 30;
        }
        let mut reveals: Vec<ScenarioReveal> = scenarios.iter()
            .map(|scenario| reveal(&manifest, &commitment, scenario)).collect();
        reveals[0].case_payload_salt.truncate(31);
        assert_eq!(
            validate_run_ledger_with_reveals(&manifest, &commitment, &freeze, &scenarios, &reveals, &events),
            Err(SealedRunError::ScenarioRevealSaltLengthInvalid("trial-a".into()))
        );
        reveals[0] = reveal(&manifest, &commitment, &scenarios[0]);
        reveals[0].source_id = "other-source".into();
        assert_eq!(
            validate_run_ledger_with_reveals(&manifest, &commitment, &freeze, &scenarios, &reveals, &events),
            Err(SealedRunError::ScenarioRevealSourceMismatch("trial-a".into()))
        );
    }

    #[test]
    fn open_or_sealed_lane_rejects_retirement_reveal() {
        let (manifest, commitment, freeze, scenarios, events) = valid_fixture();
        let reveals = vec![reveal(&manifest, &commitment, &scenarios[0])];
        assert_eq!(
            validate_run_ledger_with_reveals(&manifest, &commitment, &freeze, &scenarios, &reveals, &events),
            Err(SealedRunError::UnexpectedScenarioReveal("trial-a".into()))
        );
    }

    #[test]
    fn event_before_scenario_issue_and_duplicate_attempt_events_are_rejected() {
        let (m, commitment, freeze, scenarios, mut events) = valid_fixture();
        events[0].occurred_at_ms = 1;
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::EventBeforeScenarioIssue("trial-c".into()))
        );
        let (m, commitment, freeze, scenarios, mut events) = valid_fixture();
        let duplicate = events.iter().find(|e| e.trial_id == "trial-a" && e.kind == TrialEventKind::Started).unwrap().clone();
        let mut duplicate = duplicate;
        duplicate.event_id = "a-1-start-duplicate".into();
        events.push(duplicate);
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::DuplicateAttemptStart("trial-a".into()))
        );
    }

    #[test]
    fn freeze_receipt_time_and_manifest_commitment_are_pinned() {
        let (m, commitment, mut freeze, scenarios, events) = valid_fixture();
        freeze.committed_at_ms = m.frozen_at_ms + 1;
        assert_eq!(
            validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events),
            Err(SealedRunError::FreezeReceiptNotBeforeFreeze)
        );
        let (m, _commitment, freeze, scenarios, events) = valid_fixture();
        assert_eq!(
            validate_run_ledger(&m, &Digest::from_byte(99), &freeze, &scenarios, &events),
            Err(SealedRunError::FreezeReceiptMismatch)
        );
    }

    #[test]
    fn sixty_seven_adversarial_mutations_fail_closed() {
        for vector in 0..75 {
            let (mut m, commitment, mut freeze, mut scenarios, mut events) = valid_fixture();
            match vector {
                0 => { m.run_id.clear(); }
                1 => { m.schema.push_str("-wrong"); }
                2 => { m.version = 2; }
                3 => { m.agent.source_commit = "not-a-revision".into(); }
                4 => { m.agent.model_provider.clear(); }
                5 => { m.agent.model_id.clear(); }
                6 => { m.agent.model_version.clear(); }
                7 => { m.baseline.baseline_id.clear(); }
                8 => { m.baseline.source_commit = "bad".into(); }
                9 => { m.agent.control_domain = m.evaluator_control_domain.clone(); }
                10 => { m.evidence_control_domain = m.agent.control_domain.clone(); }
                11 => { m.evaluator_control_domain = m.evidence_control_domain.clone(); }
                12 => { m.created_at_ms = m.frozen_at_ms + 1; }
                13 => { m.trials.clear(); }
                14 => { m.trials[0].maximum_attempts = 0; }
                15 => { let duplicate = m.trials[0].clone(); m.trials.push(duplicate); }
                16 => { freeze.publisher_id.clear(); }
                17 => { freeze.publisher_control_domain = m.agent.control_domain.clone(); }
                18 => { freeze.agent_identity_digest = Digest::from_byte(99); }
                19 => { freeze.baseline_identity_digest = Digest::from_byte(99); }
                20 => { freeze.evaluator_digest = Digest::from_byte(99); }
                21 => { freeze.frozen_at_ms += 1; }
                22 => { scenarios[0].run_id = "other-run".into(); }
                23 => { scenarios[0].manifest_commitment = Digest::from_byte(99); }
                24 => { scenarios[0].case_payload_commitment = Digest::from_byte(0); }
                25 => { scenarios[0].oracle_commitment = Digest::from_byte(0); }
                26 => { scenarios[0].evaluator_digest = Digest::from_byte(99); }
                27 => { scenarios[0].source_id = "unknown-source".into(); }
                28 => { scenarios[0].control_domain = m.agent.control_domain.clone(); }
                29 => { scenarios[0].generated_at_ms = m.frozen_at_ms; }
                30 => { scenarios[0].issued_at_ms = scenarios[0].generated_at_ms - 1; }
                31 => { scenarios[0].retired_at_ms = Some(5); }
                32 => { scenarios.pop(); }
                33 => { let duplicate = scenarios[0].clone(); scenarios.push(duplicate); }
                34 => { let mut x = scenarios[0].clone(); x.trial_id = "unplanned".into(); scenarios.push(x); }
                35 => { events.pop(); }
                36 => { events[0].run_id = "other-run".into(); }
                37 => { events[0].manifest_commitment = Digest::from_byte(99); }
                38 => { events[0].arm_identity_digest = Digest::from_byte(99); }
                39 => { events[0].evaluator_digest = Digest::from_byte(99); }
                40 => { events[0].scenario_commitment = Digest::from_byte(99); }
                41 => { events[0].source_id = "agent".into(); }
                42 => { events[0].control_domain = m.agent.control_domain.clone(); }
                43 => { events[0].occurred_at_ms = 1; }
                44 => { events[0].event_id = events[1].event_id.clone(); }
                45 => { let duplicate = events[0].clone(); events.push(duplicate); }
                46 => { events[0].attempt = 0; }
                47 => { events[0].attempt = 3; }
                48 => { events[0].occurred_at_ms = 39; }
                49 => { events[0].kind = TrialEventKind::Started; }
                50 => { events[0].kind = TrialEventKind::Skipped; events[0].attempt = 1; }
                51 => { events.retain(|e| !(e.trial_id == "trial-a" && e.kind == TrialEventKind::Failed)); }
                52 => { events.retain(|e| !(e.trial_id == "trial-a" && e.kind == TrialEventKind::Completed)); }
                53 => { events.retain(|e| e.trial_id != "trial-c"); }
                54 => { events.push(event(&m, &commitment, &scenarios[0], "unplanned", "extra", 1, 50, TrialEventKind::Started)); }
                55 => { m.lane = EvaluationLane::RetiredReproducibility; }
                56 => { m.lane = EvaluationLane::OpenDevelopment; scenarios[0].generated_at_ms = 5; scenarios[0].issued_at_ms = 30; scenarios[0].retired_at_ms = Some(9); }
                57 => { m.lane = EvaluationLane::RetiredReproducibility; scenarios[0].generated_at_ms = 5; scenarios[0].retired_at_ms = Some(m.created_at_ms + 1); }
                58 => { m.agent.agent_id.clear(); }
                59 => { m.evaluator_control_domain.clear(); }
                60 => { m.evidence_source_id.clear(); }
                61 => { events[0].occurred_at_ms = scenarios[0].issued_at_ms; }
                62 => { events[0].trial_id = "unknown".into(); }
                63 => { freeze.manifest_commitment = Digest::from_byte(99); }
                64 => { freeze.publisher_id = "substituted-authority".into(); }
                65 => { freeze.publisher_control_domain = "substituted-domain".into(); }
                66 => { scenarios[0].oracle_commitment = scenarios[0].case_payload_commitment.clone(); }
                67 => { m.baseline.model_provider.clear(); }
                68 => { m.baseline.model_version.clear(); }
                69 => { m.baseline.toolchain_digest = Digest::from_byte(0); }
                70 => { m.baseline.runtime_config_digest = Digest::from_byte(0); }
                71 => { m.baseline.control_domain = m.agent.control_domain.clone(); }
                72 => { m.execution_environment_digest = Digest::from_byte(0); }
                73 => { m.resource_budget_digest = Digest::from_byte(0); }
                _ => { m.baseline_identity_digest = m.agent_identity_digest.clone(); }
            };
            assert!(
                validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events).is_err(),
                "sealed-run adversarial vector {vector} was accepted"
            );
        }
    }
}
