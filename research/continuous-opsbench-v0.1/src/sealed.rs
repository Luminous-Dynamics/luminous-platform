//! Frozen run identity and append-only trial accounting for sealed OpsBench.
//!
//! This is a deterministic structural validator, not a cryptographic verifier.
//! Manifest commitments and source-authenticated receipts must be verified by the
//! existing typed evidence adapter before these records reach this module.

use super::Digest;
use std::collections::{BTreeMap, BTreeSet};

pub const SEALED_RUN_SCHEMA: &str = "continuous-opsbench-sealed-run-v0.1";
pub const SEALED_RUN_VERSION: u16 = 1;

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
    pub model_id: String,
    pub scaffold_digest: Digest,
    pub prompt_digest: Digest,
    pub tool_policy_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedTrial {
    pub trial_id: String,
    /// Independent resampling unit: fault schedule × topology × application cluster.
    pub cluster_id: String,
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
    pub agent_identity_digest: Digest,
    pub baseline_identity_digest: Digest,
    pub agent: FrozenAgentIdentity,
    pub baseline: BaselineIdentity,
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
    pub publisher_id: String,
    pub publisher_control_domain: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioReceipt {
    pub run_id: String,
    pub trial_id: String,
    pub manifest_commitment: Digest,
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
    pub trial_id: String,
    pub event_id: String,
    pub attempt: u16,
    pub occurred_at_ms: u64,
    pub kind: TrialEventKind,
    pub manifest_commitment: Digest,
    pub agent_identity_digest: Digest,
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
    pub cluster_id: String,
    pub disposition: TrialDisposition,
    pub attempts_started: u16,
    pub first_started_at_ms: Option<u64>,
    pub terminal_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunLedgerSummary {
    pub run_id: String,
    pub lane: EvaluationLane,
    pub planned_trials: usize,
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
    ScenarioGeneratedBeforeFreeze(String),
    ScenarioIssuedBeforeGenerated(String),
    MissingRetirementTime(String),
    RetirementAfterRunCreated(String),
    UnexpectedRetirementTime(String),
    DuplicateEventId,
    UnplannedTrialEvent(String),
    EventBindingMismatch(String),
    EventSourceMismatch(String),
    EventControlDomainCollision(String),
    EventBeforeScenarioIssue(String),
    MissingTrialDisposition(String),
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
    validate_manifest(manifest)?;
    validate_freeze_receipt(manifest, expected_manifest_commitment, freeze)?;

    let mut trial_index: BTreeMap<&str, &PlannedTrial> = BTreeMap::new();
    let mut cluster_trial_keys = BTreeSet::new();
    for trial in &manifest.trials {
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

    let mut event_ids = BTreeSet::new();
    let mut events_by_trial: BTreeMap<&str, Vec<&TrialEvent>> = BTreeMap::new();
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
        events_by_trial.entry(event.trial_id.as_str()).or_default().push(event);
    }

    let mut results = Vec::with_capacity(manifest.trials.len());
    let mut completed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut invalidated = 0usize;
    let mut total_attempts = 0usize;
    let unique_clusters = manifest.trials.iter()
        .map(|trial| trial.cluster_id.as_str())
        .collect::<BTreeSet<_>>().len();

    for trial in &manifest.trials {
        let scenario = scenario_index[trial.trial_id.as_str()];
        let trial_events = events_by_trial.get(trial.trial_id.as_str())
            .map(Vec::as_slice).unwrap_or(&[]);
        let result = validate_trial_events(manifest, trial, scenario, trial_events)?;
        total_attempts = total_attempts.saturating_add(result.attempts_started as usize);
        match result.disposition {
            TrialDisposition::Completed => completed += 1,
            TrialDisposition::Failed => failed += 1,
            TrialDisposition::Skipped => skipped += 1,
            TrialDisposition::Invalidated => invalidated += 1,
        }
        results.push(result);
    }

    Ok(RunLedgerSummary {
        run_id: manifest.run_id.clone(),
        lane: manifest.lane,
        planned_trials: manifest.trials.len(),
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
        (manifest.baseline.model_id.as_str(), "baseline_model_id"),
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
    if manifest.created_at_ms > manifest.frozen_at_ms {
        return Err(SealedRunError::InvalidRunInterval);
    }
    if manifest.trials.is_empty() { return Err(SealedRunError::MissingPlannedTrials); }
    if manifest.agent.control_domain == manifest.evaluator_control_domain
        || manifest.agent.control_domain == manifest.evidence_control_domain
        || manifest.agent.control_domain == manifest.agent.control_domain.trim()
            && manifest.agent.control_domain.is_empty()
        || manifest.evaluator_control_domain == manifest.evidence_control_domain
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
        || receipt.publisher_id.trim().is_empty()
        || receipt.publisher_control_domain.trim().is_empty()
        || receipt.publisher_control_domain == manifest.agent.control_domain
        || receipt.publisher_control_domain == manifest.evaluator_control_domain
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
    if scenario.control_domain != manifest.evidence_control_domain
        || scenario.control_domain == manifest.agent.control_domain
        || scenario.control_domain == manifest.evaluator_control_domain
    {
        return Err(SealedRunError::EventControlDomainCollision(id));
    }
    if scenario.generated_at_ms < manifest.created_at_ms {
        return Err(SealedRunError::ScenarioGeneratedBeforeFreeze(id));
    }
    if scenario.issued_at_ms < scenario.generated_at_ms
        || scenario.issued_at_ms < manifest.frozen_at_ms
    {
        return Err(SealedRunError::ScenarioIssuedBeforeGenerated(id));
    }
    match manifest.lane {
        EvaluationLane::SealedCertification => {
            if scenario.generated_at_ms <= manifest.frozen_at_ms {
                return Err(SealedRunError::ScenarioGeneratedBeforeFreeze(id));
            }
            if scenario.retired_at_ms.is_some() {
                return Err(SealedRunError::UnexpectedRetirementTime(id));
            }
        }
        EvaluationLane::RetiredReproducibility => {
            let retired = scenario.retired_at_ms
                .ok_or_else(|| SealedRunError::MissingRetirementTime(id.clone()))?;
            if scenario.generated_at_ms > retired || retired > manifest.created_at_ms {
                return Err(SealedRunError::RetirementAfterRunCreated(id));
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
    if event.run_id != manifest.run_id
        || event.manifest_commitment != *expected_commitment
        || event.agent_identity_digest != manifest.agent_identity_digest
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
    manifest: &RunManifest,
    trial: &PlannedTrial,
    scenario: &ScenarioReceipt,
    events: &[&TrialEvent],
) -> Result<TrialResult, SealedRunError> {
    if events.is_empty() {
        return Err(SealedRunError::MissingTrialDisposition(trial.trial_id.clone()));
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
                cluster_id: trial.cluster_id.clone(),
                disposition: TrialDisposition::Skipped,
                attempts_started: 0,
                first_started_at_ms: None,
                terminal_at_ms: event.occurred_at_ms,
            });
        }
        return Ok(TrialResult {
            trial_id: trial.trial_id.clone(),
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
        cluster_id: trial.cluster_id.clone(),
        disposition,
        attempts_started: highest_attempt,
        first_started_at_ms: first_started_at,
        terminal_at_ms: terminal.occurred_at_ms,
    })
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
        let manifest = RunManifest {
            schema: SEALED_RUN_SCHEMA.into(),
            version: SEALED_RUN_VERSION,
            run_id: "run-001".into(),
            lane,
            created_at_ms: 10,
            frozen_at_ms: 20,
            evaluator_digest: evaluator_digest.clone(),
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
                model_id: "model-baseline".into(),
                scaffold_digest: Digest::from_byte(11),
                prompt_digest: Digest::from_byte(12),
                tool_policy_digest: Digest::from_byte(13),
            },
            evaluator_control_domain: "evaluator-domain".into(),
            evidence_source_id: "independent-ledger".into(),
            evidence_control_domain: "evidence-domain".into(),
            trials: vec![
                PlannedTrial { trial_id: "trial-a".into(), cluster_id: "cluster-1".into(), maximum_attempts: 2 },
                PlannedTrial { trial_id: "trial-b".into(), cluster_id: "cluster-2".into(), maximum_attempts: 1 },
                PlannedTrial { trial_id: "trial-c".into(), cluster_id: "cluster-2".into(), maximum_attempts: 1 },
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
        TrialEvent {
            run_id: manifest.run_id.clone(),
            trial_id: trial_id.into(),
            event_id: event_id.into(),
            attempt,
            occurred_at_ms: at,
            kind,
            manifest_commitment: commitment.clone(),
            agent_identity_digest: manifest.agent_identity_digest.clone(),
            evaluator_digest: manifest.evaluator_digest.clone(),
            scenario_commitment: scenario.case_payload_commitment.clone(),
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
            event(&m, &commitment, &scenarios[1], "trial-b", "b-skip", 0, 35, TrialEventKind::Skipped),
            event(&m, &commitment, &scenarios[2], "trial-c", "c-start", 1, 40, TrialEventKind::Started),
            event(&m, &commitment, &scenarios[2], "trial-c", "c-invalid", 1, 45, TrialEventKind::Invalidated),
        ];
        events.reverse(); // input order is not trusted; event timestamps/attempts are.
        (m, commitment, freeze, scenarios, events)
    }

    #[test]
    fn valid_run_preserves_all_outcomes_attempts_and_cluster_denominator() {
        let (m, commitment, freeze, scenarios, events) = valid_fixture();
        let result = validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events).unwrap();
        assert_eq!(result.planned_trials, 3);
        assert_eq!(result.completed_trials, 1);
        assert_eq!(result.failed_trials, 0);
        assert_eq!(result.skipped_trials, 1);
        assert_eq!(result.invalidated_trials, 1);
        assert_eq!(result.total_attempts, 3);
        assert_eq!(result.unique_clusters, 2);
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
            Err(SealedRunError::MissingTrialDisposition("trial-c".into()))
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
        scenarios[0].generated_at_ms = 12;
        scenarios[0].issued_at_ms = 30;
        // The fixture's existing events still bind to the same frozen commitment and case.
        let result = validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events);
        assert!(result.is_ok());
        events.clear();
    }

    #[test]
    fn retired_reproducibility_cases_must_be_retired_before_run_creation() {
        let (mut m, commitment, freeze, mut scenarios, events) = valid_fixture();
        m.lane = EvaluationLane::RetiredReproducibility;
        for scenario in &mut scenarios {
            scenario.generated_at_ms = 5;
            scenario.retired_at_ms = Some(8);
            scenario.issued_at_ms = 30;
        }
        let result = validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events);
        assert!(result.is_ok());
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
        let (m, commitment, freeze, scenarios, events) = valid_fixture();
        assert_eq!(
            validate_run_ledger(&m, &Digest::from_byte(99), &freeze, &scenarios, &events),
            Err(SealedRunError::FreezeReceiptMismatch)
        );
    }

    #[test]
    fn sixty_four_adversarial_mutations_fail_closed() {
        for vector in 0..64 {
            let (mut m, commitment, mut freeze, mut scenarios, mut events) = valid_fixture();
            let outcome = match vector {
                0 => { m.run_id.clear(); None }
                1 => { m.schema.push_str("-wrong"); None }
                2 => { m.version = 2; None }
                3 => { m.agent.source_commit = "not-a-revision".into(); None }
                4 => { m.agent.model_provider.clear(); None }
                5 => { m.agent.model_id.clear(); None }
                6 => { m.agent.model_version.clear(); None }
                7 => { m.baseline.baseline_id.clear(); None }
                8 => { m.baseline.source_commit = "bad".into(); None }
                9 => { m.agent.control_domain = m.evaluator_control_domain.clone(); None }
                10 => { m.evidence_control_domain = m.agent.control_domain.clone(); None }
                11 => { m.evaluator_control_domain = m.evidence_control_domain.clone(); None }
                12 => { m.created_at_ms = m.frozen_at_ms + 1; None }
                13 => { m.trials.clear(); None }
                14 => { m.trials[0].maximum_attempts = 0; None }
                15 => { m.trials.push(m.trials[0].clone()); None }
                16 => { freeze.publisher_id.clear(); None }
                17 => { freeze.publisher_control_domain = m.agent.control_domain.clone(); None }
                18 => { freeze.agent_identity_digest = Digest::from_byte(99); None }
                19 => { freeze.baseline_identity_digest = Digest::from_byte(99); None }
                20 => { freeze.evaluator_digest = Digest::from_byte(99); None }
                21 => { freeze.frozen_at_ms += 1; None }
                22 => { scenarios[0].run_id = "other-run".into(); None }
                23 => { scenarios[0].manifest_commitment = Digest::from_byte(99); None }
                24 => { scenarios[0].case_payload_commitment = Digest::from_byte(0); None }
                25 => { scenarios[0].oracle_commitment = Digest::from_byte(0); None }
                26 => { scenarios[0].evaluator_digest = Digest::from_byte(99); None }
                27 => { scenarios[0].source_id = "unknown-source".into(); None }
                28 => { scenarios[0].control_domain = m.agent.control_domain.clone(); None }
                29 => { scenarios[0].generated_at_ms = m.frozen_at_ms; None }
                30 => { scenarios[0].issued_at_ms = scenarios[0].generated_at_ms - 1; None }
                31 => { scenarios[0].retired_at_ms = Some(5); None }
                32 => { scenarios.pop(); None }
                33 => { scenarios.push(scenarios[0].clone()); None }
                34 => { let mut x = scenarios[0].clone(); x.trial_id = "unplanned".into(); scenarios.push(x); None }
                35 => { events.pop(); None }
                36 => { events[0].run_id = "other-run".into(); None }
                37 => { events[0].manifest_commitment = Digest::from_byte(99); None }
                38 => { events[0].agent_identity_digest = Digest::from_byte(99); None }
                39 => { events[0].evaluator_digest = Digest::from_byte(99); None }
                40 => { events[0].scenario_commitment = Digest::from_byte(99); None }
                41 => { events[0].source_id = "agent".into(); None }
                42 => { events[0].control_domain = m.agent.control_domain.clone(); None }
                43 => { events[0].occurred_at_ms = 1; None }
                44 => { events[0].event_id = events[1].event_id.clone(); None }
                45 => { events.push(events[0].clone()); None }
                46 => { events[0].attempt = 0; None }
                47 => { events[0].attempt = 3; None }
                48 => { events[0].kind = TrialEventKind::Completed; None }
                49 => { events[0].kind = TrialEventKind::Started; None }
                50 => { events[0].kind = TrialEventKind::Skipped; events[0].attempt = 1; None }
                51 => { events.retain(|e| !(e.trial_id == "trial-a" && e.kind == TrialEventKind::Failed)); None }
                52 => { events.retain(|e| !(e.trial_id == "trial-a" && e.kind == TrialEventKind::Completed)); None }
                53 => { events.retain(|e| e.trial_id != "trial-c"); None }
                54 => { events.push(event(&m, &commitment, &scenarios[0], "unplanned", "extra", 1, 50, TrialEventKind::Started)); None }
                55 => { m.lane = EvaluationLane::RetiredReproducibility; None }
                56 => { m.lane = EvaluationLane::OpenDevelopment; scenarios[0].generated_at_ms = 5; scenarios[0].issued_at_ms = 30; None }
                57 => { m.lane = EvaluationLane::RetiredReproducibility; scenarios[0].generated_at_ms = 5; scenarios[0].retired_at_ms = Some(m.created_at_ms + 1); None }
                58 => { m.agent.agent_id.clear(); None }
                59 => { m.evaluator_control_domain.clear(); None }
                60 => { m.evidence_source_id.clear(); None }
                61 => { events[0].occurred_at_ms = scenarios[0].issued_at_ms; None }
                62 => { events[0].trial_id = "unknown".into(); None }
                _ => { freeze.manifest_commitment = Digest::from_byte(99); None }
            };
            let _ = outcome;
            assert!(
                validate_run_ledger(&m, &commitment, &freeze, &scenarios, &events).is_err(),
                "sealed-run adversarial vector {vector} was accepted"
            );
        }
    }
}
