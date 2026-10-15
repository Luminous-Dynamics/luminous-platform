//! Strict offline validation of synthetic civic evaluation packs.
//!
//! This validator checks fixture structure and safety-oriented expected
//! outcomes. It is not a simulator, source authenticator, or action authority.

use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationErrors {
    pub messages: Vec<String>,
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, message) in self.messages.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "- {message}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationPack {
    pub schema_version: String,
    pub pack_id: String,
    pub created_on: String,
    pub evidence_mode: EvidenceMode,
    pub source_policy: SourcePolicy,
    pub reproducibility: Reproducibility,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceMode {
    SyntheticOnly,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePolicy {
    pub live_ingestion_enabled: bool,
    pub personal_data_included: bool,
    pub published_claims_count_as_ground_truth: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reproducibility {
    pub seed: u64,
    pub simulator_version: String,
    pub expected_outcomes_are_fixture_oracles: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub scenario_id: String,
    pub title: String,
    pub scenario_class: ScenarioClass,
    pub input: ScenarioInput,
    pub expected: ExpectedOutcome,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioClass {
    Baseline,
    StaleSource,
    ConflictingSources,
    RepairClaimWithoutIndependentVerification,
    CrossUtilityContextNotCausality,
    Recurrence,
    IndependentVerificationPositiveControl,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioInput {
    pub source_coverage: SourceCoverage,
    pub service_context: ServiceContext,
    pub prior_outcome: PriorOutcome,
    pub available_inspectors: u16,
    pub available_crews: u16,
    pub observations: Vec<Observation>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceCoverage {
    Complete,
    Incomplete,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceContext {
    LocalPublicService,
    CrossUtilityPublication,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PriorOutcome {
    None,
    RepairClaimed,
    IndependentlyVerified,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub observation_id: String,
    pub kind: ObservationKind,
    pub evidence_state: EvidenceState,
    pub event_minute: u32,
    pub recorded_minute: u32,
    pub actor_ref: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKind {
    ResidentReport,
    PublicNotice,
    RepairClaim,
    IndependentInspection,
    ContextualStatement,
    RepeatFault,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    Current,
    Stale,
    Conflicting,
    ClaimOnly,
    IndependentlyVerified,
    RepeatFault,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedOutcome {
    pub triage: TriageOutcome,
    pub outcome: OutcomeState,
    pub causal_claim_permitted: bool,
    pub must_abstain: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TriageOutcome {
    Inspect,
    HumanReview,
    DoNotVerify,
    Reopen,
    NoAction,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeState {
    Unresolved,
    Unknown,
    Unverified,
    IndependentlyVerified,
    Reopened,
}

/// Parse and semantically validate a synthetic evaluation pack.
pub fn validate_evaluation_pack_json(input: &str) -> Result<EvaluationPack, ValidationErrors> {
    let pack: EvaluationPack = serde_json::from_str(input).map_err(|error| ValidationErrors {
        messages: vec![format!("invalid evaluation-pack JSON: {error}")],
    })?;
    let errors = validate(&pack);
    if errors.is_empty() {
        Ok(pack)
    } else {
        Err(ValidationErrors { messages: errors })
    }
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_id(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix)
        && value.len() > prefix.len()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if !value.is_ascii()
        || bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        value[0..4].parse::<i32>(),
        value[5..7].parse::<u32>(),
        value[8..10].parse::<u32>(),
    ) else {
        return false;
    };
    if !(1900..=2200).contains(&year) || !(1..=12).contains(&month) {
        return false;
    }
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

fn valid_pack_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (3..=96).contains(&bytes.len())
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-'
        })
}

fn validate(pack: &EvaluationPack) -> Vec<String> {
    let mut errors = Vec::new();
    if pack.schema_version != "civic-evaluation-pack-v1" {
        errors.push("schema_version must be civic-evaluation-pack-v1".to_owned());
    }
    if !valid_pack_id(&pack.pack_id) {
        errors.push(
            "pack_id must contain only lowercase ASCII letters, digits, and hyphens".to_owned(),
        );
    }
    if !valid_iso_date(&pack.created_on) {
        errors.push("created_on must be a real calendar date in YYYY-MM-DD format".to_owned());
    }
    if pack.source_policy.live_ingestion_enabled {
        errors.push("live_ingestion_enabled must remain false for this contract".to_owned());
    }
    if pack.source_policy.personal_data_included {
        errors.push("personal_data_included must remain false for this contract".to_owned());
    }
    if pack.source_policy.published_claims_count_as_ground_truth {
        errors.push("published claims must not count as ground truth".to_owned());
    }
    if !pack.reproducibility.expected_outcomes_are_fixture_oracles {
        errors.push("expected_outcomes_are_fixture_oracles must be true".to_owned());
    }
    if !nonempty(&pack.reproducibility.simulator_version) {
        errors.push("simulator_version must not be empty".to_owned());
    }
    if pack.scenarios.is_empty() {
        errors.push("scenarios must contain at least one scenario".to_owned());
    }

    let mut scenario_ids = HashSet::new();
    let mut observation_ids = HashSet::new();
    for (index, scenario) in pack.scenarios.iter().enumerate() {
        let prefix = format!("scenarios[{index}]");
        if !valid_id(&scenario.scenario_id, "scenario-")
            || !scenario_ids.insert(scenario.scenario_id.as_str())
        {
            errors.push(format!("{prefix}.scenario_id is invalid or duplicated"));
        }
        if !nonempty(&scenario.title) {
            errors.push(format!("{prefix}.title must not be empty"));
        }
        if scenario.input.observations.is_empty() {
            errors.push(format!("{prefix}.input.observations must not be empty"));
        }
        for (observation_index, observation) in scenario.input.observations.iter().enumerate() {
            let observation_prefix = format!("{prefix}.input.observations[{observation_index}]");
            if !valid_id(&observation.observation_id, "obs-")
                || !observation_ids.insert(observation.observation_id.as_str())
            {
                errors.push(format!(
                    "{observation_prefix}.observation_id is invalid or duplicated across the pack"
                ));
            }
            if !valid_id(&observation.actor_ref, "synthetic-") {
                errors.push(format!(
                    "{observation_prefix}.actor_ref must be a synthetic pseudonymous identifier"
                ));
            }
            if observation.recorded_minute < observation.event_minute {
                errors.push(format!(
                    "{observation_prefix}.recorded_minute cannot precede event_minute"
                ));
            }
        }

        let uncertain = scenario.input.source_coverage == SourceCoverage::Incomplete
            || scenario.input.observations.iter().any(|observation| {
                matches!(
                    observation.evidence_state,
                    EvidenceState::Stale | EvidenceState::Conflicting
                )
            });
        if uncertain
            && (!scenario.expected.must_abstain
                || scenario.expected.triage != TriageOutcome::HumanReview)
        {
            errors.push(format!("{prefix}: uncertain evidence requires abstention and review"));
        }

        let has_repair_claim = scenario.input.observations.iter().any(|observation| {
            observation.kind == ObservationKind::RepairClaim
        });
        let valid_independent_inspection = scenario.input.observations.iter().any(|inspection| {
            if inspection.kind != ObservationKind::IndependentInspection
                || inspection.evidence_state != EvidenceState::IndependentlyVerified
            {
                return false;
            }
            scenario.input.observations.iter().any(|claim| {
                claim.kind == ObservationKind::RepairClaim
                    && claim.actor_ref != inspection.actor_ref
                    && scenario.input.prior_outcome == PriorOutcome::RepairClaimed
            })
        });
        if scenario.expected.outcome == OutcomeState::IndependentlyVerified
            && !valid_independent_inspection
        {
            errors.push(format!("{prefix}: missing independent verification evidence"));
        }
        if has_repair_claim && !valid_independent_inspection
            && scenario.expected.outcome == OutcomeState::IndependentlyVerified
        {
            errors.push(format!("{prefix}: a repair claim is not verification"));
        }

        if scenario.expected.causal_claim_permitted {
            errors.push(format!("{prefix}: causal claims are outside this contract"));
        }

        let has_contextual_statement = scenario.input.observations.iter().any(|observation| {
            observation.kind == ObservationKind::ContextualStatement
        });
        if scenario.input.service_context == ServiceContext::CrossUtilityPublication
            && has_contextual_statement
            && scenario.expected.causal_claim_permitted
        {
            errors.push(format!("{prefix}: contextual evidence does not establish causality"));
        }

        let has_repeat_fault = scenario.input.observations.iter().any(|observation| {
            observation.kind == ObservationKind::RepeatFault
                || observation.evidence_state == EvidenceState::RepeatFault
        });
        if uncertain && scenario.expected.outcome == OutcomeState::IndependentlyVerified {
            errors.push(format!("{prefix}: uncertainty cannot verify an outcome"));
        }

        if scenario.input.prior_outcome == PriorOutcome::IndependentlyVerified
            && has_repeat_fault
            && scenario.expected.outcome != OutcomeState::Reopened
        {
            errors.push(format!("{prefix}: repeat fault must reopen verified outcome"));
        }
        if scenario.expected.outcome == OutcomeState::Reopened && !has_repeat_fault {
            errors.push(format!("{prefix}: reopened outcome requires a repeat-fault observation"));
        }
        match scenario.scenario_class {
            ScenarioClass::StaleSource
                if !scenario
                    .input
                    .observations
                    .iter()
                    .any(|observation| observation.evidence_state == EvidenceState::Stale) =>
            {
                errors.push(format!("{prefix}: stale_source class requires a stale observation"));
            }
            ScenarioClass::ConflictingSources
                if !scenario
                    .input
                    .observations
                    .iter()
                    .any(|observation| observation.evidence_state == EvidenceState::Conflicting) =>
            {
                errors.push(format!("{prefix}: conflict class needs conflicting evidence"));
            }
            ScenarioClass::RepairClaimWithoutIndependentVerification
                if !has_repair_claim || valid_independent_inspection =>
            {
                errors.push(format!("{prefix}: claim needs no qualifying inspection"));
            }
            ScenarioClass::CrossUtilityContextNotCausality
                if scenario.input.service_context != ServiceContext::CrossUtilityPublication
                    || !has_contextual_statement
                    || scenario.expected.causal_claim_permitted
                    || !scenario.expected.must_abstain =>
            {
                errors.push(format!("{prefix}: causal control invalid"));
            }
            ScenarioClass::Recurrence
                if scenario.input.prior_outcome != PriorOutcome::IndependentlyVerified
                    || !has_repeat_fault
                    || scenario.expected.outcome != OutcomeState::Reopened
                    || scenario.expected.triage != TriageOutcome::Reopen =>
            {
                errors.push(format!("{prefix}: recurrence control invalid"));
            }
            ScenarioClass::IndependentVerificationPositiveControl
                if scenario.expected.outcome != OutcomeState::IndependentlyVerified
                    || !valid_independent_inspection
                    || scenario.expected.must_abstain
                    || scenario.expected.triage != TriageOutcome::NoAction =>
            {
                errors.push(format!("{prefix}: positive control invalid"));
            }
            _ => {}
        }
    }
    errors
}
