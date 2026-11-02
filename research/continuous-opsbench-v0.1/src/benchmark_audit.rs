//! Component-level benchmark validity and scorer audit.
//!
//! This structural validator checks the declared cohort, task, environment,
//! ground-truth and evaluator contracts plus known-answer, mutation and interaction
//! controls. It produces a component-wise admissibility vector, never a scalar score.
//! Signatures, publisher authentication and production representativeness remain
//! responsibilities of the existing typed source-evidence infrastructure.

use crate::sealed::{
    validate_freeze_receipt, validate_manifest, EvaluationLane, FreezeReceipt, RunManifest,
};
use crate::Digest;
use std::collections::{BTreeMap, BTreeSet};

pub const BENCHMARK_AUDIT_SCHEMA: &str = "continuous-opsbench-benchmark-audit-v0.1";
pub const BENCHMARK_AUDIT_VERSION: u16 = 1;
pub const REQUIRED_COMPONENTS: usize = 4;
pub const MIN_KNOWN_ANSWER_CONTROLS: usize = 40;
pub const MIN_KNOWN_ANSWER_CONTROLS_PER_COMPONENT: usize = 5;
pub const MIN_POSITIVE_CONTROLS_PER_COMPONENT: usize = 2;
pub const MIN_NEGATIVE_CONTROLS_PER_COMPONENT: usize = 2;
pub const MIN_EXPECTED_REJECTION_MUTATIONS_PER_COMPONENT: usize = 5;
pub const MIN_POSITIVE_CONTROLS: usize = 10;
pub const MIN_NEGATIVE_CONTROLS: usize = 10;
pub const MIN_MUTATION_CONTROLS: usize = 60;
pub const MIN_MUTATION_CONTROLS_PER_COMPONENT: usize = 10;
pub const MIN_RESET_REPETITIONS: usize = 3;
pub const COMPONENT_PAIR_COUNT: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BenchmarkComponent {
    UserTask,
    EnvironmentTools,
    GroundTruth,
    Evaluation,
}

impl BenchmarkComponent {
    pub const ALL: [Self; REQUIRED_COMPONENTS] = [
        Self::UserTask,
        Self::EnvironmentTools,
        Self::GroundTruth,
        Self::Evaluation,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditDisposition {
    Pass,
    Fail,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnownAnswerClass {
    Positive,
    Negative,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentEvidence {
    pub component: BenchmarkComponent,
    pub version: String,
    pub artifact_digest: Digest,
    pub schema_digest: Digest,
    pub evidence_digest: Digest,
    pub source_id: String,
    pub control_domain: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserTaskContract {
    pub task_id: String,
    pub instruction_digest: Digest,
    pub capability_id: String,
    pub ambiguity_class: String,
    pub precondition_digest: Digest,
    pub postcondition_digest: Digest,
    pub required_predicate_ids: Vec<String>,
    pub prohibited_predicate_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResetReceipt {
    pub reset_id: String,
    pub repetition: u32,
    pub observed_initial_state_digest: Digest,
    pub fault_activation_control_passed: bool,
    pub reset_completed_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvironmentToolsContract {
    pub environment_image_digest: Digest,
    pub tool_schema_digest: Digest,
    pub dependency_lock_digest: Digest,
    pub fault_injector_digest: Digest,
    pub reset_procedure_digest: Digest,
    pub expected_initial_state_digest: Digest,
    pub minimum_reset_repetitions: usize,
    pub reset_receipts: Vec<ResetReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundTruthContract {
    pub oracle_version: String,
    pub oracle_program_digest: Digest,
    pub oracle_coverage_digest: Digest,
    pub ambiguity_policy_digest: Digest,
    pub oracle_control_domain: String,
    pub oracle_predicate_ids: Vec<String>,
    pub private_oracle_commitment: Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricDirection {
    HigherIsBetter,
    LowerIsBetter,
    TargetRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricDefinition {
    pub metric_id: String,
    pub numerator_predicate_ids: Vec<String>,
    pub denominator_id: String,
    pub unit: String,
    pub direction: MetricDirection,
    pub exclusion_policy_digest: Digest,
    pub estimand_semantics_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationContract {
    pub evaluator_binary_digest: Digest,
    pub metric_semantics_digest: Digest,
    pub statistical_plan_digest: Digest,
    pub metrics: Vec<MetricDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkCohortContract {
    pub schema: String,
    pub version: u16,
    pub cohort_id: String,
    pub run_id: String,
    pub manifest_commitment: Digest,
    pub cohort_version: String,
    pub user_task_set_digest: Digest,
    pub issued_at_ms: u64,
    pub auditor_id: String,
    pub auditor_control_domain: String,
    pub evaluator_control_domain: String,
    pub minimum_controls_per_component: usize,
    pub minimum_mutations_per_component: usize,
    pub components: Vec<ComponentEvidence>,
    pub tasks: Vec<UserTaskContract>,
    pub environment: EnvironmentToolsContract,
    pub ground_truth: GroundTruthContract,
    pub evaluation: EvaluationContract,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownAnswerControl {
    pub control_id: String,
    pub component: BenchmarkComponent,
    pub class: KnownAnswerClass,
    /// Expected verdict from an independently authored known-answer fixture.
    pub expected_acceptance: bool,
    /// Actual result produced by the frozen component under audit.
    pub observed_acceptance: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutationControl {
    pub mutation_id: String,
    pub component: BenchmarkComponent,
    pub expected_rejection: bool,
    pub observed_rejection: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComponentPair {
    pub first: BenchmarkComponent,
    pub second: BenchmarkComponent,
}

impl ComponentPair {
    pub fn new(a: BenchmarkComponent, b: BenchmarkComponent) -> Option<Self> {
        if a == b {
            return None;
        }
        Some(if a < b {
            Self { first: a, second: b }
        } else {
            Self { first: b, second: a }
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskMetamorphicControl {
    pub control_id: String,
    pub task_id: String,
    pub base_instruction_digest: Digest,
    pub variant_instruction_digest: Digest,
    pub shared_precondition_digest: Digest,
    pub shared_postcondition_digest: Digest,
    pub equivalence_contract_digest: Digest,
    pub expected_semantic_equivalence: bool,
    pub observed_semantic_equivalence: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InteractionControl {
    pub control_id: String,
    pub pair: ComponentPair,
    pub expected_acceptance: bool,
    pub observed_acceptance: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentAuditResult {
    pub component: BenchmarkComponent,
    pub disposition: AuditDisposition,
    pub known_answer_controls: usize,
    pub positive_controls: usize,
    pub negative_controls: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub false_positive_rate_ppm: Option<u64>,
    pub false_negative_rate_ppm: Option<u64>,
    pub mutation_controls: usize,
    pub expected_rejection_controls: usize,
    pub expected_non_rejection_controls: usize,
    pub missed_mutations: usize,
    pub unexpected_rejections: usize,
    pub mutation_miss_rate_ppm: Option<u64>,
    pub mutation_overreject_rate_ppm: Option<u64>,
    pub reset_repetitions: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkAuditResult {
    pub cohort_id: String,
    pub run_id: String,
    pub lane: EvaluationLane,
    pub manifest_commitment: Digest,
    pub components: Vec<ComponentAuditResult>,
    pub interaction_controls: usize,
    pub failed_interaction_controls: usize,
    pub metamorphic_controls: usize,
    pub failed_metamorphic_controls: usize,
    /// No scalar quality score is emitted. A claim is admissible only when all
    /// component and cross-component controls pass.
    pub claim_admissible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BenchmarkAuditError {
    UnsupportedSchema,
    InvalidCohortIdentity,
    ManifestBindingMismatch,
    AuditIssuedAfterRunFreeze,
    AuditorControlDomainCollision,
    EvaluatorControlDomainCollision,
    MissingComponent(BenchmarkComponent),
    DuplicateComponent(BenchmarkComponent),
    InvalidComponentEvidence(BenchmarkComponent),
    ComponentControlDomainCollision(BenchmarkComponent),
    InvalidTask(String),
    DuplicateTask(String),
    TaskPredicateNotInOracle(String),
    RequiredAndProhibitedOverlap(String),
    InvalidEnvironmentContract,
    InsufficientResetRepetitions,
    DuplicateResetReceipt(String),
    ResetStateMismatch(String),
    FaultActivationControlFailed(String),
    InvalidGroundTruthContract,
    OracleEvaluatorDomainCollision,
    OracleEvaluatorIndependenceCollision,
    InvalidEvaluationContract,
    DuplicateMetric(String),
    InvalidMetric(String),
    InsufficientKnownAnswerControls,
    InsufficientPositiveControls,
    InsufficientNegativeControls,
    DuplicateKnownAnswerControl(String),
    KnownAnswerClassMismatch(String),
    KnownAnswerControlFailure(String),
    InsufficientMutationControls,
    DuplicateMutationControl(String),
    MutationControlFailure(String),
    InsufficientInteractionControls,
    DuplicateInteractionControl(String),
    InsufficientMetamorphicControls,
    DuplicateMetamorphicControl(String),
    MetamorphicControlBindingMismatch(String),
    MetamorphicControlFailure(String),
    InvalidInteractionPair(String),
    InteractionControlFailure(String),
    ArithmeticOverflow,
}

pub fn validate_benchmark_component_audit(
    manifest: &RunManifest,
    freeze: &FreezeReceipt,
    cohort: &BenchmarkCohortContract,
    known_answers: &[KnownAnswerControl],
    mutations: &[MutationControl],
    interactions: &[InteractionControl],
    metamorphic_controls: &[TaskMetamorphicControl],
) -> Result<BenchmarkAuditResult, BenchmarkAuditError> {
    validate_manifest(manifest)
        .map_err(|_| BenchmarkAuditError::ManifestBindingMismatch)?;
    validate_freeze_receipt(manifest, &freeze.manifest_commitment, freeze)
        .map_err(|_| BenchmarkAuditError::ManifestBindingMismatch)?;

    if cohort.schema != BENCHMARK_AUDIT_SCHEMA || cohort.version != BENCHMARK_AUDIT_VERSION {
        return Err(BenchmarkAuditError::UnsupportedSchema);
    }
    if !valid_id(&cohort.cohort_id)
        || !valid_id(&cohort.run_id)
        || !valid_id(&cohort.cohort_version)
        || !valid_id(&cohort.auditor_id)
        || !valid_id(&cohort.auditor_control_domain)
        || !valid_id(&cohort.evaluator_control_domain)
        || cohort.minimum_controls_per_component < MIN_KNOWN_ANSWER_CONTROLS_PER_COMPONENT
        || cohort.minimum_mutations_per_component < MIN_MUTATION_CONTROLS_PER_COMPONENT
    {
        return Err(BenchmarkAuditError::InvalidCohortIdentity);
    }
    if cohort.run_id != manifest.run_id
        || cohort.manifest_commitment != freeze.manifest_commitment
        || digest_is_zero(&cohort.manifest_commitment)
        || cohort.evaluator_control_domain != manifest.evaluator_control_domain
    {
        return Err(BenchmarkAuditError::ManifestBindingMismatch);
    }
    if cohort.issued_at_ms > manifest.frozen_at_ms {
        return Err(BenchmarkAuditError::AuditIssuedAfterRunFreeze);
    }
    if cohort.auditor_control_domain == manifest.agent.control_domain
        || cohort.auditor_control_domain == manifest.baseline.control_domain
        || cohort.auditor_control_domain == manifest.evaluator_control_domain
        || cohort.auditor_control_domain == manifest.freeze_authority_control_domain
        || cohort.auditor_control_domain == manifest.evidence_control_domain
        || cohort.auditor_control_domain == manifest.runtime_observer_control_domain
        || cohort.auditor_control_domain == manifest.runtime_challenge_authority_control_domain
    {
        return Err(BenchmarkAuditError::AuditorControlDomainCollision);
    }
    if cohort.evaluator_control_domain == manifest.agent.control_domain
        || cohort.evaluator_control_domain == manifest.baseline.control_domain
        || cohort.evaluator_control_domain == manifest.freeze_authority_control_domain
        || cohort.evaluator_control_domain == manifest.evidence_control_domain
    {
        return Err(BenchmarkAuditError::EvaluatorControlDomainCollision);
    }

    let component_index = validate_components(manifest, cohort)?;
    if component_index.values().any(|component|
        component.control_domain == cohort.auditor_control_domain)
    {
        return Err(BenchmarkAuditError::AuditorControlDomainCollision);
    }
    validate_tasks(cohort)?;
    let reset_repetitions = validate_environment(&cohort.environment, cohort.issued_at_ms)?;
    let ground_truth_component = component_index.get(&BenchmarkComponent::GroundTruth)
        .ok_or(BenchmarkAuditError::MissingComponent(BenchmarkComponent::GroundTruth))?;
    let evaluation_component = component_index.get(&BenchmarkComponent::Evaluation)
        .ok_or(BenchmarkAuditError::MissingComponent(BenchmarkComponent::Evaluation))?;
    if cohort.ground_truth.oracle_control_domain != ground_truth_component.control_domain
        || cohort.ground_truth.oracle_control_domain == evaluation_component.control_domain
    {
        return Err(BenchmarkAuditError::OracleEvaluatorDomainCollision);
    }
    if cohort.ground_truth.oracle_program_digest == cohort.evaluation.evaluator_binary_digest {
        return Err(BenchmarkAuditError::OracleEvaluatorIndependenceCollision);
    }
    validate_ground_truth(&cohort.ground_truth)?;
    validate_evaluation(&cohort.evaluation, &cohort.ground_truth)?;

    let mut known_ids = BTreeSet::new();
    let mut mutations_ids = BTreeSet::new();
    let mut interactions_ids = BTreeSet::new();

    let mut component_results: BTreeMap<BenchmarkComponent, ComponentAuditResult> =
        BenchmarkComponent::ALL
            .into_iter()
            .map(|component| {
                (
                    component,
                    ComponentAuditResult {
                        component,
                        disposition: AuditDisposition::Pass,
                        known_answer_controls: 0,
                        positive_controls: 0,
                        negative_controls: 0,
                        false_positives: 0,
                        false_negatives: 0,
                        false_positive_rate_ppm: None,
                        false_negative_rate_ppm: None,
                        mutation_controls: 0,
                        expected_rejection_controls: 0,
                        expected_non_rejection_controls: 0,
                        missed_mutations: 0,
                        unexpected_rejections: 0,
                        mutation_miss_rate_ppm: None,
                        mutation_overreject_rate_ppm: None,
                        reset_repetitions: if component == BenchmarkComponent::EnvironmentTools {
                            reset_repetitions
                        } else {
                            0
                        },
                    },
                )
            })
            .collect();

    for control in known_answers {
        if !valid_id(&control.control_id) || !known_ids.insert(control.control_id.as_str()) {
            return Err(BenchmarkAuditError::DuplicateKnownAnswerControl(
                control.control_id.clone(),
            ));
        }
        let result = component_results.get_mut(&control.component).unwrap();
        result.known_answer_controls += 1;
        match control.class {
            KnownAnswerClass::Positive => {
                result.positive_controls += 1;
                if !control.expected_acceptance {
                    return Err(BenchmarkAuditError::KnownAnswerClassMismatch(
                        control.control_id.clone(),
                    ));
                }
            }
            KnownAnswerClass::Negative => {
                result.negative_controls += 1;
                if control.expected_acceptance {
                    return Err(BenchmarkAuditError::KnownAnswerClassMismatch(
                        control.control_id.clone(),
                    ));
                }
            }
        }
        if control.observed_acceptance != control.expected_acceptance {
            match control.class {
                KnownAnswerClass::Positive => result.false_negatives += 1,
                KnownAnswerClass::Negative => result.false_positives += 1,
            }
            result.disposition = AuditDisposition::Fail;
        }
    }

    let total_positive = known_answers
        .iter()
        .filter(|control| control.class == KnownAnswerClass::Positive)
        .count();
    let total_negative = known_answers
        .iter()
        .filter(|control| control.class == KnownAnswerClass::Negative)
        .count();
    if known_answers.len() < MIN_KNOWN_ANSWER_CONTROLS {
        return Err(BenchmarkAuditError::InsufficientKnownAnswerControls);
    }
    if total_positive < MIN_POSITIVE_CONTROLS {
        return Err(BenchmarkAuditError::InsufficientPositiveControls);
    }
    if total_negative < MIN_NEGATIVE_CONTROLS {
        return Err(BenchmarkAuditError::InsufficientNegativeControls);
    }

    for mutation in mutations {
        if !valid_id(&mutation.mutation_id) || !mutations_ids.insert(mutation.mutation_id.as_str()) {
            return Err(BenchmarkAuditError::DuplicateMutationControl(
                mutation.mutation_id.clone(),
            ));
        }
        let result = component_results.get_mut(&mutation.component).unwrap();
        result.mutation_controls += 1;
        if mutation.expected_rejection {
            result.expected_rejection_controls += 1;
        } else {
            result.expected_non_rejection_controls += 1;
        }
        if mutation.expected_rejection && !mutation.observed_rejection {
            result.missed_mutations += 1;
            result.disposition = AuditDisposition::Fail;
        } else if !mutation.expected_rejection && mutation.observed_rejection {
            result.unexpected_rejections += 1;
            result.disposition = AuditDisposition::Fail;
        }
    }
    if mutations.len() < MIN_MUTATION_CONTROLS {
        return Err(BenchmarkAuditError::InsufficientMutationControls);
    }

    for (component, result) in &mut component_results {
        if result.disposition == AuditDisposition::Pass
            && (result.known_answer_controls < cohort.minimum_controls_per_component
                || result.positive_controls < MIN_POSITIVE_CONTROLS_PER_COMPONENT
                || result.negative_controls < MIN_NEGATIVE_CONTROLS_PER_COMPONENT
                || result.mutation_controls < cohort.minimum_mutations_per_component
                || result.expected_rejection_controls < MIN_EXPECTED_REJECTION_MUTATIONS_PER_COMPONENT
                || result.expected_non_rejection_controls == 0)
        {
            result.disposition = AuditDisposition::Indeterminate;
        }
        result.false_positive_rate_ppm = (result.negative_controls > 0)
            .then(|| rate_ppm(result.false_positives, result.negative_controls));
        result.false_negative_rate_ppm = (result.positive_controls > 0)
            .then(|| rate_ppm(result.false_negatives, result.positive_controls));
        result.mutation_miss_rate_ppm = (result.expected_rejection_controls > 0)
            .then(|| rate_ppm(result.missed_mutations, result.expected_rejection_controls));
        result.mutation_overreject_rate_ppm = (result.expected_non_rejection_controls > 0)
            .then(|| rate_ppm(result.unexpected_rejections, result.expected_non_rejection_controls));
        if *component == BenchmarkComponent::EnvironmentTools
            && result.disposition == AuditDisposition::Pass
            && result.reset_repetitions < cohort.environment.minimum_reset_repetitions
        {
            result.disposition = AuditDisposition::Indeterminate;
        }
    }

    let mut metamorphic_ids = BTreeSet::new();
    let task_index: BTreeMap<&str, &UserTaskContract> =
        cohort.tasks.iter().map(|task| (task.task_id.as_str(), task)).collect();
    let mut failed_metamorphic_controls = 0usize;
    for control in metamorphic_controls {
        if !valid_id(&control.control_id)
            || !metamorphic_ids.insert(control.control_id.as_str())
        {
            return Err(BenchmarkAuditError::DuplicateMetamorphicControl(
                control.control_id.clone(),
            ));
        }
        let task = task_index.get(control.task_id.as_str())
            .ok_or_else(|| BenchmarkAuditError::MetamorphicControlBindingMismatch(
                control.control_id.clone(),
            ))?;
        if is_zero(&control.base_instruction_digest)
            || is_zero(&control.variant_instruction_digest)
            || control.base_instruction_digest == control.variant_instruction_digest
            || control.base_instruction_digest != task.instruction_digest
            || is_zero(&control.shared_precondition_digest)
            || control.shared_precondition_digest != task.precondition_digest
            || is_zero(&control.shared_postcondition_digest)
            || control.shared_postcondition_digest != task.postcondition_digest
            || is_zero(&control.equivalence_contract_digest)
        {
            return Err(BenchmarkAuditError::MetamorphicControlBindingMismatch(
                control.control_id.clone(),
            ));
        }
        if control.expected_semantic_equivalence != control.observed_semantic_equivalence {
            failed_metamorphic_controls += 1;
            component_results.get_mut(&BenchmarkComponent::UserTask).unwrap().disposition =
                AuditDisposition::Fail;
            component_results.get_mut(&BenchmarkComponent::Evaluation).unwrap().disposition =
                AuditDisposition::Fail;
        }
    }
    if metamorphic_controls.len() < 10 {
        return Err(BenchmarkAuditError::InsufficientMetamorphicControls);
    }

    let mut interaction_pairs = BTreeSet::new();
    let mut failed_interaction_controls = 0usize;
    for interaction in interactions {
        if !valid_id(&interaction.control_id) {
            return Err(BenchmarkAuditError::InvalidInteractionPair(
                interaction.control_id.clone(),
            ));
        }
        if !interactions_ids.insert(interaction.control_id.as_str()) {
            return Err(BenchmarkAuditError::DuplicateInteractionControl(
                interaction.control_id.clone(),
            ));
        }
        if interaction.pair.first == interaction.pair.second
            || !BenchmarkComponent::ALL.contains(&interaction.pair.first)
            || !BenchmarkComponent::ALL.contains(&interaction.pair.second)
        {
            return Err(BenchmarkAuditError::InvalidInteractionPair(
                interaction.control_id.clone(),
            ));
        }
        let pair = ComponentPair::new(interaction.pair.first, interaction.pair.second)
            .ok_or_else(|| BenchmarkAuditError::InvalidInteractionPair(
                interaction.control_id.clone(),
            ))?;
        interaction_pairs.insert(pair);
        if interaction.expected_acceptance != interaction.observed_acceptance {
            failed_interaction_controls += 1;
            component_results.get_mut(&pair.first).unwrap().disposition = AuditDisposition::Fail;
            component_results.get_mut(&pair.second).unwrap().disposition = AuditDisposition::Fail;
        }
    }
    if interactions.len() < COMPONENT_PAIR_COUNT || interaction_pairs.len() < COMPONENT_PAIR_COUNT {
        return Err(BenchmarkAuditError::InsufficientInteractionControls);
    }

    for component in BenchmarkComponent::ALL {
        if !component_index.contains_key(&component) {
            return Err(BenchmarkAuditError::MissingComponent(component));
        }
    }
    let components: Vec<ComponentAuditResult> = BenchmarkComponent::ALL
        .iter()
        .map(|component| component_results[component].clone())
        .collect();
    let claim_admissible = failed_interaction_controls == 0
        && failed_metamorphic_controls == 0
        && components
            .iter()
            .all(|component| component.disposition == AuditDisposition::Pass);

    Ok(BenchmarkAuditResult {
        cohort_id: cohort.cohort_id.clone(),
        run_id: manifest.run_id.clone(),
        lane: manifest.lane,
        manifest_commitment: freeze.manifest_commitment.clone(),
        components,
        interaction_controls: interactions.len(),
        failed_interaction_controls,
        metamorphic_controls: metamorphic_controls.len(),
        failed_metamorphic_controls,
        claim_admissible,
    })
}

fn validate_components<'a>(
    manifest: &RunManifest,
    cohort: &'a BenchmarkCohortContract,
) -> Result<BTreeMap<BenchmarkComponent, &'a ComponentEvidence>, BenchmarkAuditError> {
    let mut index = BTreeMap::new();
    let mut sources = BTreeSet::new();
    let mut domains = BTreeSet::new();
    for component in &cohort.components {
        if index.insert(component.component, component).is_some() {
            return Err(BenchmarkAuditError::DuplicateComponent(component.component));
        }
        if !valid_id(&component.version)
            || !valid_id(&component.source_id)
            || !valid_id(&component.control_domain)
            || digest_is_zero(&component.artifact_digest)
            || digest_is_zero(&component.schema_digest)
            || digest_is_zero(&component.evidence_digest)
        {
            return Err(BenchmarkAuditError::InvalidComponentEvidence(component.component));
        }
        if !sources.insert(component.source_id.as_str())
            || !domains.insert(component.control_domain.as_str())
        {
            return Err(BenchmarkAuditError::InvalidComponentEvidence(component.component));
        }
        let evaluation_component = component.component == BenchmarkComponent::Evaluation;
        if component.control_domain == manifest.evaluator_control_domain && !evaluation_component {
            return Err(BenchmarkAuditError::ComponentControlDomainCollision(component.component));
        }
        if evaluation_component && component.control_domain != manifest.evaluator_control_domain {
            return Err(BenchmarkAuditError::ComponentControlDomainCollision(component.component));
        }
        if component.control_domain == manifest.agent.control_domain
            || component.control_domain == manifest.baseline.control_domain
            || (component.control_domain == manifest.evaluator_control_domain && !evaluation_component)
            || component.control_domain == manifest.freeze_authority_control_domain
            || component.control_domain == manifest.evidence_control_domain
            || component.control_domain == manifest.runtime_observer_control_domain
            || component.control_domain == manifest.runtime_challenge_authority_control_domain
        {
            return Err(BenchmarkAuditError::ComponentControlDomainCollision(
                component.component,
            ));
        }
        if index.len() > REQUIRED_COMPONENTS {
            return Err(BenchmarkAuditError::InvalidComponentEvidence(component.component));
        }
    }
    for component in BenchmarkComponent::ALL {
        if !index.contains_key(&component) {
            return Err(BenchmarkAuditError::MissingComponent(component));
        }
    }
    let expected_task_component = index.get(&BenchmarkComponent::UserTask)
        .map(|component| &component.artifact_digest);
    let expected_environment_component = index.get(&BenchmarkComponent::EnvironmentTools)
        .map(|component| &component.artifact_digest);
    let expected_ground_truth_component = index.get(&BenchmarkComponent::GroundTruth)
        .map(|component| &component.artifact_digest);
    let expected_evaluation_component = index.get(&BenchmarkComponent::Evaluation)
        .map(|component| &component.artifact_digest);
    if expected_task_component != Some(&cohort.user_task_set_digest)
        || expected_environment_component != Some(&cohort.environment.environment_image_digest)
        || expected_ground_truth_component != Some(&cohort.ground_truth.oracle_program_digest)
        || expected_evaluation_component != Some(&cohort.evaluation.evaluator_binary_digest)
    {
        return Err(BenchmarkAuditError::InvalidComponentEvidence(BenchmarkComponent::Evaluation));
    }
    Ok(index)
}

fn validate_tasks(cohort: &BenchmarkCohortContract) -> Result<(), BenchmarkAuditError> {
    if cohort.tasks.is_empty() || digest_is_zero(&cohort.ground_truth.oracle_coverage_digest) {
        return Err(BenchmarkAuditError::InvalidTask("empty task set or oracle coverage".into()));
    }
    let oracle_predicates: BTreeSet<&str> = cohort.ground_truth.oracle_predicate_ids
        .iter().map(String::as_str).collect();
    if oracle_predicates.is_empty() {
        return Err(BenchmarkAuditError::InvalidGroundTruthContract);
    }
    let mut ids = BTreeSet::new();
    for task in &cohort.tasks {
        if !valid_id(&task.task_id) || !ids.insert(task.task_id.as_str()) {
            return Err(BenchmarkAuditError::DuplicateTask(task.task_id.clone()));
        }
        if !valid_id(&task.capability_id)
            || !valid_id(&task.ambiguity_class)
            || digest_is_zero(&task.instruction_digest)
            || digest_is_zero(&task.precondition_digest)
            || digest_is_zero(&task.postcondition_digest)
            || task.required_predicate_ids.is_empty()
        {
            return Err(BenchmarkAuditError::InvalidTask(task.task_id.clone()));
        }
        let required: BTreeSet<&str> = task.required_predicate_ids.iter().map(String::as_str).collect();
        let prohibited: BTreeSet<&str> = task.prohibited_predicate_ids.iter().map(String::as_str).collect();
        if required.len() != task.required_predicate_ids.len()
            || prohibited.len() != task.prohibited_predicate_ids.len()
        {
            return Err(BenchmarkAuditError::InvalidTask(task.task_id.clone()));
        }
        if !required.is_disjoint(&prohibited) {
            return Err(BenchmarkAuditError::RequiredAndProhibitedOverlap(task.task_id.clone()));
        }
        if required.iter().any(|predicate| !oracle_predicates.contains(predicate)) {
            return Err(BenchmarkAuditError::TaskPredicateNotInOracle(task.task_id.clone()));
        }
        if prohibited.iter().any(|predicate| !oracle_predicates.contains(predicate)) {
            return Err(BenchmarkAuditError::TaskPredicateNotInOracle(task.task_id.clone()));
        }
    }
    Ok(())
}

fn validate_environment(
    environment: &EnvironmentToolsContract,
    audit_issued_at_ms: u64,
) -> Result<usize, BenchmarkAuditError> {
    if digest_is_zero(&environment.environment_image_digest)
        || digest_is_zero(&environment.tool_schema_digest)
        || digest_is_zero(&environment.dependency_lock_digest)
        || digest_is_zero(&environment.fault_injector_digest)
        || digest_is_zero(&environment.reset_procedure_digest)
        || digest_is_zero(&environment.expected_initial_state_digest)
        || environment.minimum_reset_repetitions < MIN_RESET_REPETITIONS
    {
        return Err(BenchmarkAuditError::InvalidEnvironmentContract);
    }
    if environment.reset_receipts.len() < environment.minimum_reset_repetitions {
        return Err(BenchmarkAuditError::InsufficientResetRepetitions);
    }
    let mut ids = BTreeSet::new();
    let mut repetition_ids = BTreeSet::new();
    let mut reset_times = BTreeMap::new();
    for receipt in &environment.reset_receipts {
        if !valid_id(&receipt.reset_id) || !ids.insert(receipt.reset_id.as_str())
            || receipt.repetition == 0
            || !repetition_ids.insert(receipt.repetition)
        {
            return Err(BenchmarkAuditError::DuplicateResetReceipt(receipt.reset_id.clone()));
        }
        if receipt.reset_completed_at_ms == 0
            || receipt.reset_completed_at_ms > audit_issued_at_ms
            || reset_times.insert(receipt.repetition, receipt.reset_completed_at_ms).is_some()
        {
            return Err(BenchmarkAuditError::InvalidEnvironmentContract);
        }
        if receipt.observed_initial_state_digest != environment.expected_initial_state_digest {
            return Err(BenchmarkAuditError::ResetStateMismatch(receipt.reset_id.clone()));
        }
        if !receipt.fault_activation_control_passed {
            return Err(BenchmarkAuditError::FaultActivationControlFailed(receipt.reset_id.clone()));
        }
    }
    if repetition_ids.len() != environment.reset_receipts.len()
        || repetition_ids.iter().enumerate().any(|(index, repetition)|
            *repetition != (index as u32 + 1))
        || reset_times.values().copied().collect::<BTreeSet<_>>().len()
            != reset_times.len()
    {
        return Err(BenchmarkAuditError::InvalidEnvironmentContract);
    }
    let ordered_times: Vec<u64> = reset_times.values().copied().collect();
    if ordered_times.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(BenchmarkAuditError::InvalidEnvironmentContract);
    }
    Ok(environment.reset_receipts.len())
}

fn validate_ground_truth(
    ground_truth: &GroundTruthContract,
) -> Result<(), BenchmarkAuditError> {
    if !valid_id(&ground_truth.oracle_version)
        || digest_is_zero(&ground_truth.oracle_program_digest)
        || digest_is_zero(&ground_truth.oracle_coverage_digest)
        || digest_is_zero(&ground_truth.ambiguity_policy_digest)
        || !valid_id(&ground_truth.oracle_control_domain)
        || digest_is_zero(&ground_truth.private_oracle_commitment)
        || ground_truth.oracle_predicate_ids.is_empty()
    {
        return Err(BenchmarkAuditError::InvalidGroundTruthContract);
    }
    let ids: BTreeSet<&str> = ground_truth.oracle_predicate_ids.iter().map(String::as_str).collect();
    if ids.len() != ground_truth.oracle_predicate_ids.len() {
        return Err(BenchmarkAuditError::InvalidGroundTruthContract);
    }
    Ok(())
}

fn validate_evaluation(
    evaluation: &EvaluationContract,
    ground_truth: &GroundTruthContract,
) -> Result<(), BenchmarkAuditError> {
    if digest_is_zero(&evaluation.evaluator_binary_digest)
        || digest_is_zero(&evaluation.metric_semantics_digest)
        || digest_is_zero(&evaluation.statistical_plan_digest)
        || evaluation.metrics.is_empty()
    {
        return Err(BenchmarkAuditError::InvalidEvaluationContract);
    }
    let mut ids = BTreeSet::new();
    for metric in &evaluation.metrics {
        if !valid_id(&metric.metric_id) || !ids.insert(metric.metric_id.as_str()) {
            return Err(BenchmarkAuditError::DuplicateMetric(metric.metric_id.clone()));
        }
        let numerator_ids: BTreeSet<&str> =
            metric.numerator_predicate_ids.iter().map(String::as_str).collect();
        if metric.numerator_predicate_ids.is_empty()
            || numerator_ids.len() != metric.numerator_predicate_ids.len()
            || numerator_ids.iter().any(|id|
                !ground_truth.oracle_predicate_ids.iter().any(|predicate| predicate == id))
            || !valid_id(&metric.denominator_id)
            || !valid_id(&metric.unit)
            || digest_is_zero(&metric.exclusion_policy_digest)
            || digest_is_zero(&metric.estimand_semantics_digest)
        {
            return Err(BenchmarkAuditError::InvalidMetric(metric.metric_id.clone()));
        }
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 256
        && !value.chars().any(char::is_control)
}

fn rate_ppm(numerator: usize, denominator: usize) -> u64 {
    if denominator == 0 {
        return 0;
    }
    ((numerator as u128).saturating_mul(1_000_000) / denominator as u128) as u64
}

fn is_zero(digest: &Digest) -> bool {
    digest_is_zero(digest)
}

fn digest_is_zero(digest: &Digest) -> bool {
    digest.0.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sealed::{BaselineIdentity, EvaluationArm, EvaluationLane, FrozenAgentIdentity};

    fn d(byte: u8) -> Digest {
        Digest::from_byte(byte)
    }

    fn fixture() -> (
        RunManifest,
        FreezeReceipt,
        BenchmarkCohortContract,
        Vec<KnownAnswerControl>,
        Vec<MutationControl>,
        Vec<InteractionControl>,
        Vec<TaskMetamorphicControl>,
    ) {
        let agent_identity_digest = d(1);
        let baseline_identity_digest = d(2);
        let evaluator_digest = d(3);
        let environment_digest = d(4);
        let budget_digest = d(5);
        let arm_order_digest = d(6);
        let state_policy_digest = d(7);
        let artifact_policy_digest = d(8);
        let egress_policy_digest = d(9);
        let runtime_observer_digest = d(10);
        let manifest_commitment = d(12);
        let agent = FrozenAgentIdentity {
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
        };
        let baseline = BaselineIdentity {
            baseline_id: "baseline".into(),
            image_digest: d(19),
            source_commit: "abcdef0123456789abcdef0123456789abcdef01".into(),
            model_provider: "provider".into(),
            model_id: "baseline-model".into(),
            model_version: "v1".into(),
            scaffold_digest: d(20),
            prompt_digest: d(21),
            toolchain_digest: d(22),
            runtime_config_digest: d(23),
            tool_policy_digest: d(24),
            control_domain: "baseline-domain".into(),
        };
        let manifest = RunManifest {
            schema: crate::sealed::SEALED_RUN_SCHEMA.into(),
            version: crate::sealed::SEALED_RUN_VERSION,
            run_id: "audit-run".into(),
            lane: EvaluationLane::SealedCertification,
            created_at_ms: 1,
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
            runtime_observer_adapter_digest: runtime_observer_digest.clone(),
            runtime_challenge_authority_id: "challenge-authority".into(),
            runtime_challenge_authority_control_domain: "challenge-domain".into(),
            agent_identity_digest: agent_identity_digest.clone(),
            baseline_identity_digest: baseline_identity_digest.clone(),
            agent,
            baseline,
            freeze_authority_id: "freeze-authority".into(),
            freeze_authority_control_domain: "freeze-domain".into(),
            evaluator_control_domain: "evaluator-domain".into(),
            evidence_source_id: "scenario-source".into(),
            evidence_control_domain: "scenario-domain".into(),
            trials: vec![
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
            ],
        };
        let freeze = FreezeReceipt {
            run_id: manifest.run_id.clone(),
            manifest_commitment: manifest_commitment.clone(),
            committed_at_ms: 10,
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
        let components = vec![
            ComponentEvidence {
                component: BenchmarkComponent::UserTask,
                version: "task-v1".into(),
                artifact_digest: d(60),
                schema_digest: d(31),
                evidence_digest: d(32),
                source_id: "task-source".into(),
                control_domain: "task-domain".into(),
            },
            ComponentEvidence {
                component: BenchmarkComponent::EnvironmentTools,
                version: "env-v1".into(),
                artifact_digest: d(45),
                schema_digest: d(34),
                evidence_digest: d(35),
                source_id: "env-source".into(),
                control_domain: "env-domain".into(),
            },
            ComponentEvidence {
                component: BenchmarkComponent::GroundTruth,
                version: "oracle-v1".into(),
                artifact_digest: d(51),
                schema_digest: d(37),
                evidence_digest: d(38),
                source_id: "oracle-source".into(),
                control_domain: "oracle-domain".into(),
            },
            ComponentEvidence {
                component: BenchmarkComponent::Evaluation,
                version: "eval-v1".into(),
                artifact_digest: d(55),
                schema_digest: d(40),
                evidence_digest: d(41),
                source_id: "eval-source".into(),
                control_domain: "evaluator-domain".into(),
            },
        ];
        let tasks = vec![
            UserTaskContract {
                task_id: "task-a".into(),
                instruction_digest: d(42),
                capability_id: "repair-service".into(),
                ambiguity_class: "unambiguous".into(),
                precondition_digest: d(43),
                postcondition_digest: d(44),
                required_predicate_ids: vec!["service-healthy".into()],
                prohibited_predicate_ids: vec!["oracle-read".into()],
            },
        ];
        let environment = EnvironmentToolsContract {
            environment_image_digest: d(45),
            tool_schema_digest: d(46),
            dependency_lock_digest: d(47),
            fault_injector_digest: d(48),
            reset_procedure_digest: d(49),
            expected_initial_state_digest: d(50),
            minimum_reset_repetitions: MIN_RESET_REPETITIONS,
            reset_receipts: (0..MIN_RESET_REPETITIONS)
                .map(|index| ResetReceipt {
                    reset_id: format!("reset-{index}"),
                    repetition: index as u32 + 1,
                    observed_initial_state_digest: d(50),
                    fault_activation_control_passed: true,
                    reset_completed_at_ms: index as u64 + 1,
                })
                .collect(),
        };
        let ground_truth = GroundTruthContract {
            oracle_version: "oracle-v1".into(),
            oracle_program_digest: d(51),
            oracle_coverage_digest: d(52),
            ambiguity_policy_digest: d(53),
            oracle_control_domain: "oracle-domain".into(),
            oracle_predicate_ids: vec!["service-healthy".into(), "oracle-read".into()],
            private_oracle_commitment: d(54),
        };
        let evaluation = EvaluationContract {
            evaluator_binary_digest: d(55),
            metric_semantics_digest: d(56),
            statistical_plan_digest: d(57),
            metrics: vec![MetricDefinition {
                metric_id: "verified-recovery-rate".into(),
                numerator_predicate_ids: vec!["service-healthy".into()],
                denominator_id: "all-injected-faults".into(),
                unit: "proportion".into(),
                direction: MetricDirection::HigherIsBetter,
                exclusion_policy_digest: d(58),
                estimand_semantics_digest: d(59),
            }],
        };
        let cohort = BenchmarkCohortContract {
            schema: BENCHMARK_AUDIT_SCHEMA.into(),
            version: BENCHMARK_AUDIT_VERSION,
            cohort_id: "cohort-001".into(),
            run_id: manifest.run_id.clone(),
            manifest_commitment: manifest_commitment.clone(),
            cohort_version: "cohort-v1".into(),
            user_task_set_digest: d(60),
            issued_at_ms: 15,
            auditor_id: "benchmark-auditor".into(),
            auditor_control_domain: "auditor-domain".into(),
            evaluator_control_domain: manifest.evaluator_control_domain.clone(),
            minimum_controls_per_component: MIN_KNOWN_ANSWER_CONTROLS_PER_COMPONENT,
            minimum_mutations_per_component: MIN_MUTATION_CONTROLS_PER_COMPONENT,
            components,
            tasks,
            environment,
            ground_truth,
            evaluation,
        };
        let mut known = Vec::new();
        for component in BenchmarkComponent::ALL {
            for i in 0..10 {
                let class = if i < 5 { KnownAnswerClass::Positive } else { KnownAnswerClass::Negative };
                known.push(KnownAnswerControl {
                    control_id: format!("{component:?}-known-{i}"),
                    component,
                    class,
                    expected_acceptance: class == KnownAnswerClass::Positive,
                    observed_acceptance: class == KnownAnswerClass::Positive,
                });
            }
        }
        let mut mutation = Vec::new();
        for component in BenchmarkComponent::ALL {
            for i in 0..15 {
                mutation.push(MutationControl {
                    mutation_id: format!("{component:?}-mutation-{i}"),
                    component,
                    expected_rejection: i < 10,
                    observed_rejection: i < 10,
                });
            }
        }
        let interactions = vec![
            (BenchmarkComponent::UserTask, BenchmarkComponent::EnvironmentTools),
            (BenchmarkComponent::UserTask, BenchmarkComponent::GroundTruth),
            (BenchmarkComponent::UserTask, BenchmarkComponent::Evaluation),
            (BenchmarkComponent::EnvironmentTools, BenchmarkComponent::GroundTruth),
            (BenchmarkComponent::EnvironmentTools, BenchmarkComponent::Evaluation),
            (BenchmarkComponent::GroundTruth, BenchmarkComponent::Evaluation),
        ].into_iter().enumerate().map(|(index, (first, second))| InteractionControl {
            control_id: format!("interaction-{index}"),
            pair: ComponentPair::new(first, second).unwrap(),
            expected_acceptance: true,
            observed_acceptance: true,
        }).collect();
        let metamorphic = (0..10).map(|index| TaskMetamorphicControl {
            control_id: format!("metamorphic-{index}"),
            task_id: "task-a".into(),
            base_instruction_digest: d(42),
            variant_instruction_digest: d(70 + index as u8),
            shared_precondition_digest: d(43),
            shared_postcondition_digest: d(44),
            equivalence_contract_digest: d(80 + index as u8),
            expected_semantic_equivalence: true,
            observed_semantic_equivalence: true,
        }).collect();
        (manifest, freeze, cohort, known, mutation, interactions, metamorphic)
    }

    #[test]
    fn fully_valid_cohort_produces_component_vector_without_scalar_score() {
        let (manifest, freeze, cohort, known, mutations, interactions, metamorphic) = fixture();
        let result = validate_benchmark_component_audit(
            &manifest, &freeze, &cohort, &known, &mutations, &interactions, &metamorphic
        ).unwrap();
        assert!(result.claim_admissible);
        assert_eq!(result.components.len(), REQUIRED_COMPONENTS);
        assert!(
            result.components.iter()
                .all(|component| component.disposition == AuditDisposition::Pass)
        );
    }

    #[test]
    fn component_rates_report_false_positive_false_negative_and_mutation_miss_separately() {
        let (manifest, freeze, cohort, mut known, mut mutations, interactions, metamorphic) = fixture();
        known[0].observed_acceptance = false;
        known[5].observed_acceptance = true;
        mutations[0].observed_rejection = false;
        mutations[10].observed_rejection = true;
        let result = validate_benchmark_component_audit(
            &manifest, &freeze, &cohort, &known, &mutations, &interactions, &metamorphic
        ).unwrap();
        let user = result.components.iter()
            .find(|component| component.component == BenchmarkComponent::UserTask).unwrap();
        assert_eq!(user.false_positive_rate_ppm, Some(200_000));
        assert_eq!(user.false_negative_rate_ppm, Some(200_000));
        assert_eq!(user.mutation_miss_rate_ppm, Some(100_000));
        assert_eq!(user.mutation_overreject_rate_ppm, Some(200_000));
        assert_eq!(user.missed_mutations, 1);
        assert_eq!(user.unexpected_rejections, 1);
        assert_eq!(user.expected_rejection_controls, 10);
        assert_eq!(user.expected_non_rejection_controls, 5);
    }

    #[test]
    fn known_answer_false_positive_and_false_negative_block_claim() {
        let (manifest, freeze, cohort, mut known, mutations, interactions, metamorphic) = fixture();
        known[0].observed_acceptance = false;
        known[5].observed_acceptance = true;
        let result = validate_benchmark_component_audit(
            &manifest, &freeze, &cohort, &known, &mutations, &interactions, &metamorphic
        ).unwrap();
        assert!(!result.claim_admissible);
        let user = result.components.iter()
            .find(|component| component.component == BenchmarkComponent::UserTask).unwrap();
        assert_eq!(user.false_negatives, 1);
        assert_eq!(user.false_positives, 1);
        assert_eq!(user.disposition, AuditDisposition::Fail);
    }

    #[test]
    fn equivalent_task_variants_must_preserve_the_scored_semantics() {
        let (manifest, freeze, cohort, known, mutations, interactions, mut metamorphic) = fixture();
        metamorphic[0].observed_semantic_equivalence = false;
        let result = validate_benchmark_component_audit(
            &manifest, &freeze, &cohort, &known, &mutations, &interactions, &metamorphic
        ).unwrap();
        assert!(!result.claim_admissible);
        assert_eq!(result.failed_metamorphic_controls, 1);
        assert_eq!(result.components.iter()
            .find(|component| component.component == BenchmarkComponent::UserTask)
            .unwrap().disposition, AuditDisposition::Fail);
    }

    #[test]
    fn missing_component_is_rejected_instead_of_defaulting_to_pass() {
        let (manifest, freeze, mut cohort, known, mutations, interactions, metamorphic) = fixture();
        cohort.components.pop();
        assert_eq!(
            validate_benchmark_component_audit(
                &manifest, &freeze, &cohort, &known, &mutations, &interactions, &metamorphic
            ),
            Err(BenchmarkAuditError::MissingComponent(BenchmarkComponent::Evaluation))
        );
    }

    #[test]
    fn reset_drift_or_inactive_fault_injector_invalidates_environment_component() {
        let (manifest, freeze, mut cohort, known, mutations, interactions, metamorphic) = fixture();
        cohort.environment.reset_receipts[1].observed_initial_state_digest = d(99);
        assert_eq!(
            validate_benchmark_component_audit(
                &manifest, &freeze, &cohort, &known, &mutations, &interactions, &metamorphic
            ),
            Err(BenchmarkAuditError::ResetStateMismatch("reset-1".into()))
        );
        let (manifest, freeze, mut cohort, known, mutations, interactions, metamorphic) = fixture();
        cohort.environment.reset_receipts[1].fault_activation_control_passed = false;
        assert_eq!(
            validate_benchmark_component_audit(
                &manifest, &freeze, &cohort, &known, &mutations, &interactions, &metamorphic
            ),
            Err(BenchmarkAuditError::FaultActivationControlFailed("reset-1".into()))
        );
    }

    #[test]
    fn task_requirements_must_map_to_oracle_predicates_without_overlap() {
        let (manifest, freeze, mut cohort, known, mutations, interactions, metamorphic) = fixture();
        cohort.tasks[0].required_predicate_ids.push("not-in-oracle".into());
        assert_eq!(
            validate_benchmark_component_audit(
                &manifest, &freeze, &cohort, &known, &mutations, &interactions, &metamorphic
            ),
            Err(BenchmarkAuditError::TaskPredicateNotInOracle("task-a".into()))
        );
        let (manifest, freeze, mut cohort, known, mutations, interactions, metamorphic) = fixture();
        cohort.tasks[0].prohibited_predicate_ids.push("service-healthy".into());
        assert_eq!(
            validate_benchmark_component_audit(
                &manifest, &freeze, &cohort, &known, &mutations, &interactions, &metamorphic
            ),
            Err(BenchmarkAuditError::RequiredAndProhibitedOverlap("task-a".into()))
        );
    }

    #[test]
    fn all_six_component_pairs_require_interaction_controls() {
        let (manifest, freeze, cohort, known, mutations, mut interactions, mut metamorphic) = fixture();
        interactions.pop();
        assert_eq!(
            validate_benchmark_component_audit(
                &manifest, &freeze, &cohort, &known, &mutations, &interactions, &metamorphic
            ),
            Err(BenchmarkAuditError::InsufficientInteractionControls)
        );
    }

    #[test]
    fn oracle_and_evaluator_must_not_share_a_control_domain() {
        let (manifest, freeze, mut cohort, known, mutations, interactions, metamorphic) = fixture();
        cohort.ground_truth.oracle_control_domain = "evaluator-domain".into();
        assert_eq!(
            validate_benchmark_component_audit(
                &manifest, &freeze, &cohort, &known, &mutations, &interactions, &metamorphic
            ),
            Err(BenchmarkAuditError::OracleEvaluatorDomainCollision)
        );
    }

    #[test]
    fn ninety_adversarial_mutations_fail_closed() {
        for vector in 0..95 {
            let (manifest, freeze, mut cohort, mut known, mut mutations, mut interactions, metamorphic) = fixture();
            match vector {
                0 => { cohort.schema.push_str("-invalid"); }
                1 => { cohort.version = 2; }
                2 => { cohort.cohort_id.clear(); }
                3 => { cohort.run_id = "other-run".into(); }
                4 => { cohort.manifest_commitment = d(99); }
                5 => { cohort.issued_at_ms = manifest.frozen_at_ms + 1; }
                6 => { cohort.auditor_control_domain = manifest.agent.control_domain.clone(); }
                7 => { cohort.evaluator_control_domain = manifest.agent.control_domain.clone(); }
                8 => { cohort.components.clear(); }
                9 => { cohort.components[0].artifact_digest = d(0); }
                10 => { cohort.components[0].source_id.clear(); }
                11 => { cohort.components[0].control_domain = manifest.evaluator_control_domain.clone(); }
                12 => { cohort.components.push(cohort.components[0].clone()); }
                13 => { cohort.tasks.clear(); }
                14 => { cohort.tasks[0].instruction_digest = d(0); }
                15 => { cohort.tasks[0].required_predicate_ids.clear(); }
                16 => { cohort.tasks[0].required_predicate_ids.push("not-in-oracle".into()); }
                17 => { cohort.tasks[0].prohibited_predicate_ids.push("service-healthy".into()); }
                18 => { cohort.environment.environment_image_digest = d(0); }
                19 => { cohort.environment.tool_schema_digest = d(0); }
                20 => { cohort.environment.dependency_lock_digest = d(0); }
                21 => { cohort.environment.fault_injector_digest = d(0); }
                22 => { cohort.environment.reset_procedure_digest = d(0); }
                23 => { cohort.environment.expected_initial_state_digest = d(99); }
                24 => { cohort.environment.minimum_reset_repetitions = 1; }
                25 => { cohort.environment.reset_receipts.pop(); }
                26 => { cohort.environment.reset_receipts[0].reset_id = "reset-1".into(); }
                27 => { cohort.environment.reset_receipts[0].fault_activation_control_passed = false; }
                28 => { cohort.ground_truth.oracle_program_digest = d(0); }
                29 => { cohort.ground_truth.oracle_coverage_digest = d(0); }
                30 => { cohort.ground_truth.ambiguity_policy_digest = d(0); }
                31 => { cohort.ground_truth.oracle_control_domain.clear(); }
                32 => { cohort.ground_truth.oracle_predicate_ids.clear(); }
                33 => { cohort.ground_truth.private_oracle_commitment = d(0); }
                34 => { cohort.evaluation.evaluator_binary_digest = d(0); }
                35 => { cohort.evaluation.metric_semantics_digest = d(0); }
                36 => { cohort.evaluation.statistical_plan_digest = d(0); }
                37 => { cohort.evaluation.metrics.clear(); }
                38 => { cohort.evaluation.metrics[0].metric_id.clear(); }
                39 => { cohort.evaluation.metrics[0].numerator_predicate_ids.clear(); }
                40 => { cohort.evaluation.metrics[0].denominator_id.clear(); }
                41 => { cohort.evaluation.metrics[0].unit.clear(); }
                42 => { cohort.evaluation.metrics[0].exclusion_policy_digest = d(0); }
                43 => { cohort.evaluation.metrics[0].estimand_semantics_digest = d(0); }
                44 => { known.clear(); }
                45 => { known[0].control_id = "duplicate".into(); known[1].control_id = "duplicate".into(); }
                46 => { known[0].expected_acceptance = false; }
                47 => { known[0].observed_acceptance = false; }
                48 => { known[5].observed_acceptance = true; }
                49 => { known.pop(); }
                50 => { mutations.clear(); }
                51 => { mutations[0].mutation_id = "duplicate".into(); mutations[1].mutation_id = "duplicate".into(); }
                52 => { mutations[0].observed_rejection = false; }
                53 => { mutations.pop(); }
                54 => { interactions.clear(); }
                55 => { interactions[0].pair = ComponentPair { first: BenchmarkComponent::UserTask, second: BenchmarkComponent::UserTask }; }
                56 => { interactions[0].observed_acceptance = false; }
                57 => { interactions[0].control_id = "duplicate".into(); interactions[1].control_id = "duplicate".into(); }
                58 => { cohort.auditor_control_domain = manifest.evaluator_control_domain.clone(); }
                59 => { cohort.evaluator_control_domain = manifest.freeze_authority_control_domain.clone(); }
                60 => { cohort.tasks[0].precondition_digest = d(0); }
                61 => { cohort.tasks[0].postcondition_digest = d(0); }
                62 => { cohort.tasks[0].capability_id.clear(); }
                63 => { cohort.tasks[0].ambiguity_class.clear(); }
                64 => { cohort.components[0].version.clear(); }
                65 => { cohort.components[0].schema_digest = d(0); }
                66 => { cohort.components[0].evidence_digest = d(0); }
                67 => { cohort.components[1].control_domain = cohort.components[0].control_domain.clone(); }
                68 => { cohort.environment.reset_receipts[0].observed_initial_state_digest = d(99); }
                69 => { cohort.ground_truth.oracle_version.clear(); }
                70 => { cohort.ground_truth.oracle_control_domain = cohort.evaluation.metrics[0].metric_id.clone(); }
                71 => { cohort.evaluation.metrics[0].numerator_predicate_ids.push("not-in-oracle".into()); }
                72 => { cohort.evaluation.metrics.push(cohort.evaluation.metrics[0].clone()); }
                73 => { known[0].class = KnownAnswerClass::Negative; }
                74 => { known[5].class = KnownAnswerClass::Positive; }
                75 => { interactions[0].expected_acceptance = false; }
                76 => { interactions[0].pair = ComponentPair { first: BenchmarkComponent::EnvironmentTools, second: BenchmarkComponent::UserTask }; }
                77 => { cohort.components[0].control_domain = manifest.baseline.control_domain.clone(); }
                78 => { cohort.components[0].source_id = cohort.components[1].source_id.clone(); }
                79 => { cohort.environment.reset_receipts[0].repetition = cohort.environment.reset_receipts[1].repetition; }
                80 => { cohort.environment.reset_receipts[0].reset_completed_at_ms = 0; }
                81 => { cohort.ground_truth.oracle_coverage_digest = d(0); }
                82 => { cohort.ground_truth.private_oracle_commitment = d(0); }
                83 => { cohort.environment.reset_receipts.clear(); }
                84 => { cohort.minimum_controls_per_component = 0; }
                85 => { cohort.minimum_mutations_per_component = 0; }
                86 => { known[0].control_id.clear(); }
                87 => { mutations[0].mutation_id.clear(); }
                88 => { interactions[0].control_id.clear(); }
                89 => { cohort.cohort_version.clear(); }
                90 => { metamorphic[0].task_id = "unknown-task".into(); }
                91 => { metamorphic[0].variant_instruction_digest = metamorphic[0].base_instruction_digest.clone(); }
                92 => { metamorphic[0].shared_postcondition_digest = d(99); }
                93 => { metamorphic[0].observed_semantic_equivalence = false; }
                _ => { metamorphic[0].control_id = metamorphic[1].control_id.clone(); }
            }
            match validate_benchmark_component_audit(
                &manifest, &freeze, &cohort, &known, &mutations, &interactions, &metamorphic
            ) {
                Err(_) => {}
                Ok(result) => assert!(
                    !result.claim_admissible,
                    "benchmark audit adversarial vector {vector} was incorrectly admissible"
                ),
            }
        }
    }
}
