//! Five-arm paired context-compilation ablation for issue #318.
//!
//! Every arm is derived from the same source evidence universe and carries the
//! same evidence identities. Arms differ only in presentation/compilation:
//! raw source records; normalized records; topology/change joins; temporal and
//! freshness joins; and ranked causal evidence with explicit alternatives.
//!
//! Ground-truth fault labels are passed only to the evaluator after a reasoner
//! returns. The ContextReasoner interface receives a packet and budgets, never
//! evaluator truth. This module does not call a model or execute infrastructure.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;
use std::time::Instant;

use crate::Digest;

pub const CONTEXT_COMPILATION_ARMS: [ContextArm; 5] = [
    ContextArm::RawSourceRecords,
    ContextArm::NormalizedRecords,
    ContextArm::TopologyAndDiffs,
    ContextArm::TemporalAndFreshness,
    ContextArm::RankedCausalEvidence,
];

/// Require enough distinct fault scenarios before reporting bootstrap intervals.
pub const MIN_SCENARIO_CLUSTERS_FOR_CI95: usize = 20;
/// Fixed replicate count; the caller supplies and the result records the seed.
pub const CLUSTER_BOOTSTRAP_REPLICATES: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContextArm {
    RawSourceRecords,
    NormalizedRecords,
    TopologyAndDiffs,
    TemporalAndFreshness,
    RankedCausalEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceState {
    Present,
    Absent,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceKind {
    Alert,
    Metric,
    Log,
    Trace,
    Change,
    Configuration,
    Topology,
    Queue,
    Node,
    Application,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationKind {
    DependsOn,
    Calls,
    HostedBy,
    OwnedBy,
    ChangedBy,
    Observes,
    RoutesTo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextEvidence {
    pub evidence_id: String,
    pub source_id: String,
    pub control_domain: String,
    pub object_ref: String,
    pub object_kind: EvidenceKind,
    pub field_path: String,
    /// Payload is the source-authorized, already-redacted value visible to all arms.
    pub value: Option<String>,
    pub state: EvidenceState,
    pub event_time_ms: u64,
    pub observed_at_ms: u64,
    pub valid_until_ms: Option<u64>,
    pub digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextRelation {
    pub relation_id: String,
    pub evidence_id: String,
    pub from_object: String,
    pub to_object: String,
    pub relation: RelationKind,
    pub state: EvidenceState,
    pub observed_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextExperimentProfile {
    pub scenario_revision_digest: Digest,
    pub model_snapshot_digest: Digest,
    pub prompt_digest: Digest,
    pub permission_profile_digest: Digest,
    pub tool_catalog_digest: Digest,
    pub scorer_digest: Digest,
    pub reasoning_effort: String,
    pub sampling_seed: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextExperimentInput {
    pub scenario_id: String,
    /// Evaluator-side stratum label. It is deliberately not sent to the reasoner.
    pub fault_family_id: String,
    pub paired_block_id: String,
    pub attempt_id: u32,
    pub profile: ContextExperimentProfile,
    pub target_object_ref: String,
    pub audited_at_ms: u64,
    pub maximum_evidence_age_ms: u64,
    pub evidence: Vec<ContextEvidence>,
    pub relations: Vec<ContextRelation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextBudget {
    pub maximum_context_bytes: usize,
    pub maximum_input_tokens: u64,
    pub maximum_output_tokens: u64,
    pub maximum_tool_calls: u32,
    pub maximum_wall_time_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freshness {
    Current,
    Stale,
    FutureDated,
    Expired,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedEvidence {
    pub evidence_id: String,
    pub source_id: String,
    pub control_domain: String,
    pub object_ref: String,
    pub object_kind: EvidenceKind,
    pub field_path: String,
    pub value: Option<String>,
    pub state: EvidenceState,
    pub event_time_ms: u64,
    pub observed_at_ms: u64,
    pub valid_until_ms: Option<u64>,
    pub digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectDiff {
    pub evidence_id: String,
    pub object_ref: String,
    pub field_path: String,
    pub state: EvidenceState,
    pub observed_at_ms: u64,
    pub digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemporalEvidence {
    pub evidence_id: String,
    pub event_time_ms: u64,
    pub observed_at_ms: u64,
    pub age_ms: Option<u64>,
    pub freshness: Freshness,
    pub source_id: String,
    pub control_domain: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CandidateReason {
    DirectTargetEvidence,
    DependencyOfTarget,
    FreshObservation,
    MultipleSources,
    RecentChange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankedCandidate {
    pub object_ref: String,
    pub score: u32,
    pub supporting_evidence_ids: Vec<String>,
    pub reason_codes: Vec<CandidateReason>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextPresentation {
    Raw(Vec<ContextEvidence>),
    Normalized(Vec<NormalizedEvidence>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPacket {
    pub scenario_id: String,
    pub paired_block_id: String,
    pub attempt_id: u32,
    pub profile: ContextExperimentProfile,
    pub arm: ContextArm,
    pub target_object_ref: String,
    pub evidence: ContextPresentation,
    /// Only populated for topology-aware arms.
    pub relations: Vec<ContextRelation>,
    /// Only populated for TopologyAndDiffs and later arms.
    pub object_diffs: Vec<ObjectDiff>,
    /// Only populated for TemporalAndFreshness and RankedCausalEvidence.
    pub temporal: Vec<TemporalEvidence>,
    /// Only populated for RankedCausalEvidence. This is a ranking of observed
    /// objects/evidence, not an asserted root cause or hidden evaluator label.
    pub ranked_candidates: Vec<RankedCandidate>,
    /// Exact rendered bytes of the packet representation supplied to the reasoner.
    pub rendered_context: String,
    pub context_bytes: usize,
    /// A byte/4 estimate only; the reasoner adapter reports actual token usage.
    pub estimated_input_tokens: u64,
    pub evidence_ids: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosisAnswer {
    pub root_object_ref: Option<String>,
    pub supporting_evidence_ids: Vec<String>,
    pub alternative_object_refs: Vec<String>,
    pub abstained: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReasonerUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub tool_calls: u32,
    pub reported_cost_microusd: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReasonerResponse {
    /// Provider/harness session identifier used for this single arm invocation.
    pub session_id: String,
    pub answer: DiagnosisAnswer,
    pub usage: ReasonerUsage,
}

pub trait ContextReasoner {
    /// Create a genuinely fresh provider conversation for this arm. The adapter
    /// receives the frozen setup manifest, but must not expose experiment labels
    /// or pairing metadata to the model prompt.
    fn start_fresh_session(&mut self, profile: &ContextExperimentProfile) -> String;

    /// The model receives only the rendered evidence text and equal budgets.
    /// It receives no evaluator truth, expected root, arm label, scenario/block
    /// IDs, attempt, scorer digest, or required-evidence list. The adapter must
    /// return the exact fresh session ID and never reuse it across arms.
    fn diagnose(
        &mut self,
        session_id: &str,
        rendered_context: &str,
        budget: ContextBudget,
    ) -> ReasonerResponse;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextGroundTruth {
    pub correct_root_object_ref: String,
    /// Evaluator-owned evidence IDs required to substantiate the reference diagnosis.
    pub required_evidence_ids: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArmStatus {
    Scored,
    BudgetExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextScore {
    pub root_cause_correct: bool,
    pub root_object_visible: bool,
    pub root_object_grounded: bool,
    pub supported_evidence_precision_ppm: u32,
    pub required_evidence_recall_ppm: u32,
    pub unsupported_evidence_claims: usize,
    pub required_evidence_visible: usize,
    pub required_evidence_total: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextArmResult {
    pub scenario_id: String,
    pub fault_family_id: String,
    pub paired_block_id: String,
    pub attempt_id: u32,
    pub profile: ContextExperimentProfile,
    pub arm: ContextArm,
    pub status: ArmStatus,
    pub compiler_time_micros: u128,
    pub reasoner_time_micros: Option<u128>,
    /// Provider/harness conversation identity used for this arm; never shared.
    pub reasoner_session_id: Option<String>,
    pub context_bytes: usize,
    pub estimated_input_tokens: u64,
    pub usage: Option<ReasonerUsage>,
    pub answer: Option<DiagnosisAnswer>,
    pub score: Option<ContextScore>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextExperimentError {
    EmptyScenarioId,
    EmptyFaultFamilyId,
    EmptyTargetObject,
    EmptyPairedBlockId,
    InvalidExperimentProfile,
    EmptyEvidence,
    DuplicateEvidenceId,
    DuplicateArmResult,
    IncompletePairedBlock,
    ManifestMismatch,
    InvalidEvidenceRecord,
    DuplicateRelationId,
    InvalidRelation,
    RelationEvidenceMissing,
    InvalidBudget,
    InvalidGroundTruth,
    GroundTruthEvidenceMissing,
    InvalidArmOrder,
    DuplicateScenarioAttempt,
    EmptyReasonerSessionId,
    DuplicateReasonerSessionId,
    ReasonerSessionMismatch,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn digest_hex(digest: &Digest) -> String {
    let mut output = String::with_capacity(64);
    for byte in digest.0 {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn validate_input(input: &ContextExperimentInput) -> Result<(), ContextExperimentError> {
    if !non_empty(&input.scenario_id) {
        return Err(ContextExperimentError::EmptyScenarioId);
    }
    if !non_empty(&input.fault_family_id) {
        return Err(ContextExperimentError::EmptyFaultFamilyId);
    }
    if !non_empty(&input.target_object_ref) {
        return Err(ContextExperimentError::EmptyTargetObject);
    }
    if !non_empty(&input.paired_block_id) {
        return Err(ContextExperimentError::EmptyPairedBlockId);
    }
    if !non_empty(&input.profile.reasoning_effort) {
        return Err(ContextExperimentError::InvalidExperimentProfile);
    }
    if input.evidence.is_empty() {
        return Err(ContextExperimentError::EmptyEvidence);
    }

    let mut evidence_ids = BTreeSet::new();
    for item in &input.evidence {
        if !evidence_ids.insert(item.evidence_id.as_str()) {
            return Err(ContextExperimentError::DuplicateEvidenceId);
        }
        if !non_empty(&item.evidence_id)
            || !non_empty(&item.source_id)
            || !non_empty(&item.control_domain)
            || !non_empty(&item.object_ref)
            || !non_empty(&item.field_path)
            || (item.state == EvidenceState::Present && item.value.is_none())
            || (item.state != EvidenceState::Present && item.value.is_some())
            || item
                .valid_until_ms
                .is_some_and(|end| end <= item.observed_at_ms)
        {
            return Err(ContextExperimentError::InvalidEvidenceRecord);
        }
    }

    let mut relation_ids = BTreeSet::new();
    for relation in &input.relations {
        if !relation_ids.insert(relation.relation_id.as_str()) {
            return Err(ContextExperimentError::DuplicateRelationId);
        }
        if !non_empty(&relation.relation_id)
            || !non_empty(&relation.evidence_id)
            || !non_empty(&relation.from_object)
            || !non_empty(&relation.to_object)
            || relation.from_object == relation.to_object
        {
            return Err(ContextExperimentError::InvalidRelation);
        }
        if !evidence_ids.contains(relation.evidence_id.as_str()) {
            return Err(ContextExperimentError::RelationEvidenceMissing);
        }
    }
    Ok(())
}

fn validate_budget(budget: ContextBudget) -> Result<(), ContextExperimentError> {
    if budget.maximum_context_bytes == 0
        || budget.maximum_input_tokens == 0
        || budget.maximum_output_tokens == 0
        || budget.maximum_wall_time_ms == 0
    {
        return Err(ContextExperimentError::InvalidBudget);
    }
    Ok(())
}

fn normalize(evidence: &[ContextEvidence]) -> Vec<NormalizedEvidence> {
    let mut normalized: Vec<_> = evidence
        .iter()
        .map(|item| NormalizedEvidence {
            evidence_id: item.evidence_id.clone(),
            source_id: item.source_id.clone(),
            control_domain: item.control_domain.clone(),
            object_ref: item.object_ref.clone(),
            object_kind: item.object_kind,
            field_path: item.field_path.clone(),
            value: item.value.clone(),
            state: item.state,
            event_time_ms: item.event_time_ms,
            observed_at_ms: item.observed_at_ms,
            valid_until_ms: item.valid_until_ms,
            digest: item.digest.clone(),
        })
        .collect();
    normalized.sort_by(|left, right| {
        (&left.object_ref, &left.field_path, &left.evidence_id).cmp(&(
            &right.object_ref,
            &right.field_path,
            &right.evidence_id,
        ))
    });
    normalized
}

fn freshness(item: &ContextEvidence, input: &ContextExperimentInput) -> Freshness {
    if item.state == EvidenceState::Unknown {
        return Freshness::Unknown;
    }
    if item.observed_at_ms > input.audited_at_ms {
        return Freshness::FutureDated;
    }
    if item
        .valid_until_ms
        .is_some_and(|valid_until| valid_until <= input.audited_at_ms)
    {
        return Freshness::Expired;
    }
    if input.audited_at_ms.saturating_sub(item.observed_at_ms) > input.maximum_evidence_age_ms {
        return Freshness::Stale;
    }
    Freshness::Current
}

fn temporal_view(
    evidence: &[ContextEvidence],
    input: &ContextExperimentInput,
) -> Vec<TemporalEvidence> {
    let mut view: Vec<_> = evidence
        .iter()
        .map(|item| TemporalEvidence {
            evidence_id: item.evidence_id.clone(),
            event_time_ms: item.event_time_ms,
            observed_at_ms: item.observed_at_ms,
            age_ms: (item.observed_at_ms <= input.audited_at_ms)
                .then(|| input.audited_at_ms - item.observed_at_ms),
            freshness: freshness(item, input),
            source_id: item.source_id.clone(),
            control_domain: item.control_domain.clone(),
        })
        .collect();
    view.sort_by(|left, right| {
        (left.event_time_ms, left.observed_at_ms, &left.evidence_id).cmp(&(
            right.event_time_ms,
            right.observed_at_ms,
            &right.evidence_id,
        ))
    });
    view
}

fn object_diffs(evidence: &[ContextEvidence]) -> Vec<ObjectDiff> {
    let mut diffs: Vec<_> = evidence
        .iter()
        .filter(|item| item.object_kind == EvidenceKind::Change)
        .map(|item| ObjectDiff {
            evidence_id: item.evidence_id.clone(),
            object_ref: item.object_ref.clone(),
            field_path: item.field_path.clone(),
            state: item.state,
            observed_at_ms: item.observed_at_ms,
            digest: item.digest.clone(),
        })
        .collect();
    diffs.sort_by(|left, right| {
        (
            &left.object_ref,
            &left.field_path,
            left.observed_at_ms,
            &left.evidence_id,
        )
            .cmp(&(
                &right.object_ref,
                &right.field_path,
                right.observed_at_ms,
                &right.evidence_id,
            ))
    });
    diffs
}

fn ranked_candidates(
    input: &ContextExperimentInput,
    temporal: &[TemporalEvidence],
) -> Vec<RankedCandidate> {
    let temporal_by_id: BTreeMap<_, _> = temporal
        .iter()
        .map(|item| (item.evidence_id.as_str(), item))
        .collect();
    let mut by_object: BTreeMap<String, Vec<&ContextEvidence>> = BTreeMap::new();
    for item in &input.evidence {
        by_object
            .entry(item.object_ref.clone())
            .or_default()
            .push(item);
    }

    let mut candidates = Vec::new();
    for (object_ref, records) in by_object {
        let mut score = 0_u32;
        let mut reasons = BTreeSet::new();
        if object_ref == input.target_object_ref {
            score += 50;
            reasons.insert(CandidateReason::DirectTargetEvidence);
        }
        let dependency_of_target = input.relations.iter().any(|relation| {
            relation.from_object == input.target_object_ref
                && relation.to_object == object_ref
                && relation.state == EvidenceState::Present
        });
        if dependency_of_target {
            score += 25;
            reasons.insert(CandidateReason::DependencyOfTarget);
        }

        let fresh_count = records
            .iter()
            .filter(|item| {
                temporal_by_id
                    .get(item.evidence_id.as_str())
                    .is_some_and(|view| view.freshness == Freshness::Current)
            })
            .count();
        if fresh_count > 0 {
            score += 20;
            reasons.insert(CandidateReason::FreshObservation);
        }
        let source_count = records
            .iter()
            .map(|item| item.source_id.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        if source_count > 1 {
            score += 10;
            reasons.insert(CandidateReason::MultipleSources);
        }
        let recent_change = records.iter().any(|item| {
            item.object_kind == EvidenceKind::Change
                && temporal_by_id
                    .get(item.evidence_id.as_str())
                    .is_some_and(|view| view.freshness == Freshness::Current)
        });
        if recent_change {
            score += 15;
            reasons.insert(CandidateReason::RecentChange);
        }

        let mut ids: Vec<_> = records
            .iter()
            .map(|item| item.evidence_id.clone())
            .collect();
        ids.sort();
        candidates.push(RankedCandidate {
            object_ref,
            score,
            supporting_evidence_ids: ids,
            reason_codes: reasons.into_iter().collect(),
        });
    }
    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.object_ref.cmp(&right.object_ref))
    });
    candidates
}

fn render_packet(packet: &ContextPacket) -> String {
    let mut rendered = String::new();
    // Only operationally relevant target scope is model-visible. Scenario,
    // arm, attempt, model, prompt, permission, tool, scorer and seed metadata
    // remain evaluator-side and are never serialized into the reasoner prompt.
    let _ = writeln!(
        &mut rendered,
        "TARGET|{}",
        escape_field(&packet.target_object_ref)
    );

    match &packet.evidence {
        ContextPresentation::Raw(records) => {
            for item in records {
                let value = item.value.as_deref().unwrap_or("<no-value>");
                let _ = writeln!(
                    &mut rendered,
                    "RAW|{}|{}|{}|{}|{:?}|{}|{:?}|{}|{}|{}|{:?}|{}",
                    escape_field(&item.evidence_id),
                    escape_field(&item.source_id),
                    escape_field(&item.control_domain),
                    escape_field(&item.object_ref),
                    item.object_kind,
                    escape_field(&item.field_path),
                    item.state,
                    escape_field(value),
                    item.event_time_ms,
                    item.observed_at_ms,
                    item.valid_until_ms,
                    digest_hex(&item.digest)
                );
            }
        }
        ContextPresentation::Normalized(records) => {
            for item in records {
                let value = item.value.as_deref().unwrap_or("<no-value>");
                let _ = writeln!(
                    &mut rendered,
                    "FACT|id={}|source={}|domain={}|object={}|kind={:?}|field={}|state={:?}|value={}|event_ms={}|observed_ms={}|valid_until={:?}|digest={}",
                    escape_field(&item.evidence_id),
                    escape_field(&item.source_id),
                    escape_field(&item.control_domain),
                    escape_field(&item.object_ref),
                    item.object_kind,
                    escape_field(&item.field_path),
                    item.state,
                    escape_field(value),
                    item.event_time_ms,
                    item.observed_at_ms,
                    item.valid_until_ms,
                    digest_hex(&item.digest)
                );
            }
        }
    }

    for relation in &packet.relations {
        let _ = writeln!(
            &mut rendered,
            "REL|{}|{}|{:?}|{}|{}|{:?}|{}",
            escape_field(&relation.relation_id),
            escape_field(&relation.evidence_id),
            relation.relation,
            escape_field(&relation.from_object),
            escape_field(&relation.to_object),
            relation.state,
            relation.observed_at_ms
        );
    }
    for diff in &packet.object_diffs {
        let _ = writeln!(
            &mut rendered,
            "DIFF|{}|{}|{}|{:?}|{}|{}",
            escape_field(&diff.evidence_id),
            escape_field(&diff.object_ref),
            escape_field(&diff.field_path),
            diff.state,
            diff.observed_at_ms,
            digest_hex(&diff.digest)
        );
    }
    for item in &packet.temporal {
        let _ = writeln!(
            &mut rendered,
            "TIME|{}|event_ms={}|observed_ms={}|age_ms={:?}|freshness={:?}|source={}|domain={}",
            escape_field(&item.evidence_id),
            item.event_time_ms,
            item.observed_at_ms,
            item.age_ms,
            item.freshness,
            escape_field(&item.source_id),
            escape_field(&item.control_domain)
        );
    }
    for candidate in &packet.ranked_candidates {
        let _ = writeln!(
            &mut rendered,
            "CANDIDATE|{}|score={}|reasons={:?}|evidence={:?}",
            escape_field(&candidate.object_ref),
            candidate.score,
            candidate.reason_codes,
            candidate
                .supporting_evidence_ids
                .iter()
                .map(|id| escape_field(id))
                .collect::<Vec<_>>()
        );
    }
    rendered
}

pub fn compile_context_packet(
    input: &ContextExperimentInput,
    arm: ContextArm,
) -> Result<(ContextPacket, u128), ContextExperimentError> {
    validate_input(input)?;
    let started = Instant::now();

    let evidence_ids = input
        .evidence
        .iter()
        .map(|item| item.evidence_id.clone())
        .collect();
    let mut packet = ContextPacket {
        scenario_id: input.scenario_id.clone(),
        paired_block_id: input.paired_block_id.clone(),
        attempt_id: input.attempt_id,
        profile: input.profile.clone(),
        arm,
        target_object_ref: input.target_object_ref.clone(),
        evidence: if arm == ContextArm::RawSourceRecords {
            ContextPresentation::Raw(input.evidence.clone())
        } else {
            ContextPresentation::Normalized(normalize(&input.evidence))
        },
        relations: Vec::new(),
        object_diffs: Vec::new(),
        temporal: Vec::new(),
        ranked_candidates: Vec::new(),
        rendered_context: String::new(),
        context_bytes: 0,
        estimated_input_tokens: 0,
        evidence_ids,
    };

    if matches!(
        arm,
        ContextArm::TopologyAndDiffs
            | ContextArm::TemporalAndFreshness
            | ContextArm::RankedCausalEvidence
    ) {
        packet.relations = input.relations.clone();
        packet.relations.sort_by(|left, right| {
            (&left.from_object, &left.to_object, &left.relation_id).cmp(&(
                &right.from_object,
                &right.to_object,
                &right.relation_id,
            ))
        });
        packet.object_diffs = object_diffs(&input.evidence);
    }
    if matches!(
        arm,
        ContextArm::TemporalAndFreshness | ContextArm::RankedCausalEvidence
    ) {
        packet.temporal = temporal_view(&input.evidence, input);
    }
    if arm == ContextArm::RankedCausalEvidence {
        packet.ranked_candidates = ranked_candidates(input, &packet.temporal);
    }

    packet.rendered_context = render_packet(&packet);
    packet.context_bytes = packet.rendered_context.len();
    packet.estimated_input_tokens = packet.context_bytes.div_ceil(4) as u64;
    let compiler_time = started.elapsed().as_micros();
    Ok((packet, compiler_time))
}

fn validate_truth(
    input: &ContextExperimentInput,
    truth: &ContextGroundTruth,
) -> Result<(), ContextExperimentError> {
    if !non_empty(&truth.correct_root_object_ref) {
        return Err(ContextExperimentError::InvalidGroundTruth);
    }
    let evidence_ids: BTreeSet<_> = input
        .evidence
        .iter()
        .map(|item| item.evidence_id.as_str())
        .collect();
    if truth
        .required_evidence_ids
        .iter()
        .any(|id| !evidence_ids.contains(id.as_str()))
    {
        return Err(ContextExperimentError::GroundTruthEvidenceMissing);
    }
    Ok(())
}

fn score_answer(
    packet: &ContextPacket,
    truth: &ContextGroundTruth,
    answer: &DiagnosisAnswer,
) -> ContextScore {
    let state_by_id: BTreeMap<_, _> = match &packet.evidence {
        ContextPresentation::Raw(records) => records
            .iter()
            .map(|item| (item.evidence_id.as_str(), item.state))
            .collect(),
        ContextPresentation::Normalized(records) => records
            .iter()
            .map(|item| (item.evidence_id.as_str(), item.state))
            .collect(),
    };

    let object_by_id: BTreeMap<_, _> = match &packet.evidence {
        ContextPresentation::Raw(records) => records
            .iter()
            .map(|item| (item.evidence_id.as_str(), item.object_ref.as_str()))
            .collect(),
        ContextPresentation::Normalized(records) => records
            .iter()
            .map(|item| (item.evidence_id.as_str(), item.object_ref.as_str()))
            .collect(),
    };
    let mut claimed = BTreeSet::new();
    let mut supported = BTreeSet::new();
    let mut unsupported = 0_usize;
    for evidence_id in &answer.supporting_evidence_ids {
        if !claimed.insert(evidence_id.as_str()) {
            unsupported += 1;
            continue;
        }
        match state_by_id.get(evidence_id.as_str()) {
            Some(EvidenceState::Present | EvidenceState::Absent) => {
                supported.insert(evidence_id.clone());
            }
            _ => unsupported += 1,
        }
    }

    let precision = if claimed.is_empty() {
        0
    } else {
        ((supported.len() as u64 * 1_000_000) / claimed.len() as u64) as u32
    };
    let required_visible = truth
        .required_evidence_ids
        .intersection(&packet.evidence_ids)
        .count();
    let recall = if truth.required_evidence_ids.is_empty() {
        0
    } else {
        ((truth
            .required_evidence_ids
            .intersection(&supported.iter().map(|id| (*id).to_owned()).collect())
            .count() as u64
            * 1_000_000)
            / truth.required_evidence_ids.len() as u64) as u32
    };

    let root_object_visible = answer
        .root_object_ref
        .as_deref()
        .is_some_and(|root| object_by_id.values().any(|object_ref| *object_ref == root));
    let root_object_grounded = answer.root_object_ref.as_deref().is_some_and(|root| {
        supported.iter().any(|evidence_id| {
            object_by_id
                .get(evidence_id.as_str())
                .is_some_and(|object_ref| *object_ref == root)
        }) || packet.relations.iter().any(|relation| {
            relation.state == EvidenceState::Present
                && relation.from_object == packet.target_object_ref
                && relation.to_object == root
                && supported.contains(relation.evidence_id.as_str())
        })
    });

    ContextScore {
        root_cause_correct: !answer.abstained
            && answer.root_object_ref.as_deref() == Some(truth.correct_root_object_ref.as_str()),
        root_object_visible,
        root_object_grounded,
        supported_evidence_precision_ppm: precision,
        required_evidence_recall_ppm: recall,
        unsupported_evidence_claims: unsupported,
        required_evidence_visible: required_visible,
        required_evidence_total: truth.required_evidence_ids.len(),
    }
}

fn validate_arm_order(order: &[ContextArm]) -> Result<(), ContextExperimentError> {
    if order.len() != CONTEXT_COMPILATION_ARMS.len() {
        return Err(ContextExperimentError::InvalidArmOrder);
    }
    let set: BTreeSet<_> = order.iter().copied().collect();
    if set.len() != CONTEXT_COMPILATION_ARMS.len()
        || CONTEXT_COMPILATION_ARMS
            .iter()
            .any(|arm| !set.contains(arm))
    {
        return Err(ContextExperimentError::InvalidArmOrder);
    }
    Ok(())
}

/// Run a paired five-arm context ablation. The caller supplies a preregistered
/// arm order (randomized/counterbalanced across scenario blocks externally).
/// The same reasoner object and exact budgets are used for every arm. Ground
/// truth is kept separate and is consulted only after each response returns.
pub fn run_paired_context_ablation<R: ContextReasoner>(
    input: &ContextExperimentInput,
    truth: &ContextGroundTruth,
    budget: ContextBudget,
    arm_order: &[ContextArm],
    reasoner: &mut R,
) -> Result<Vec<ContextArmResult>, ContextExperimentError> {
    validate_input(input)?;
    validate_truth(input, truth)?;
    validate_budget(budget)?;
    validate_arm_order(arm_order)?;

    let mut results = Vec::with_capacity(arm_order.len());
    let mut session_ids = BTreeSet::new();
    for arm in arm_order {
        let (packet, compiler_time_micros) = compile_context_packet(input, *arm)?;
        if packet.context_bytes > budget.maximum_context_bytes
            || packet.estimated_input_tokens > budget.maximum_input_tokens
        {
            results.push(ContextArmResult {
                scenario_id: packet.scenario_id.clone(),
                fault_family_id: input.fault_family_id.clone(),
                paired_block_id: packet.paired_block_id.clone(),
                attempt_id: packet.attempt_id,
                profile: packet.profile.clone(),
                arm: *arm,
                status: ArmStatus::BudgetExceeded,
                compiler_time_micros,
                reasoner_time_micros: None,
                reasoner_session_id: None,
                context_bytes: packet.context_bytes,
                estimated_input_tokens: packet.estimated_input_tokens,
                usage: None,
                answer: None,
                score: None,
            });
            continue;
        }

        let session_id = reasoner.start_fresh_session(&packet.profile);
        if !non_empty(&session_id) {
            return Err(ContextExperimentError::EmptyReasonerSessionId);
        }
        if !session_ids.insert(session_id.clone()) {
            return Err(ContextExperimentError::DuplicateReasonerSessionId);
        }

        let started = Instant::now();
        let response = reasoner.diagnose(&session_id, &packet.rendered_context, budget);
        let reasoner_time_micros = started.elapsed().as_micros();
        if response.session_id != session_id {
            return Err(ContextExperimentError::ReasonerSessionMismatch);
        }
        let budget_exceeded = response.usage.input_tokens > budget.maximum_input_tokens
            || response.usage.output_tokens > budget.maximum_output_tokens
            || response.usage.tool_calls > budget.maximum_tool_calls
            || reasoner_time_micros > u128::from(budget.maximum_wall_time_ms) * 1_000;
        let status = if budget_exceeded {
            ArmStatus::BudgetExceeded
        } else {
            ArmStatus::Scored
        };
        let score =
            (status == ArmStatus::Scored).then(|| score_answer(&packet, truth, &response.answer));
        results.push(ContextArmResult {
            scenario_id: packet.scenario_id.clone(),
            fault_family_id: input.fault_family_id.clone(),
            paired_block_id: packet.paired_block_id.clone(),
            attempt_id: packet.attempt_id,
            profile: packet.profile.clone(),
            arm: *arm,
            status,
            compiler_time_micros,
            reasoner_time_micros: Some(reasoner_time_micros),
            reasoner_session_id: Some(session_id),
            context_bytes: packet.context_bytes,
            estimated_input_tokens: packet.estimated_input_tokens,
            usage: Some(response.usage),
            answer: Some(response.answer),
            score,
        });
    }
    Ok(results)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextArmAggregate {
    pub fault_family_id: String,
    pub arm: ContextArm,
    pub paired_blocks: usize,
    pub scored_blocks: usize,
    pub budget_exceeded_blocks: usize,
    pub correct_roots: usize,
    pub grounded_roots: usize,
    /// Correct/grounded counts divided by every paired block, including budget misses.
    pub correct_rate_all_ppm: u32,
    pub grounded_rate_all_ppm: u32,
    /// Conditional rates among scored blocks; always accompanied by scored_blocks.
    pub correct_rate_scored_ppm: Option<u32>,
    pub grounded_rate_scored_ppm: Option<u32>,
    pub mean_context_bytes: u128,
    pub mean_estimated_input_tokens: u128,
    pub mean_actual_input_tokens_scored: Option<u128>,
    pub mean_reasoner_time_micros_scored: Option<u128>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextArmContrast {
    pub fault_family_id: String,
    pub arm: ContextArm,
    pub paired_blocks: usize,
    /// Distinct scenario IDs; repeat attempts are nested inside these clusters.
    pub scenario_clusters: usize,
    pub scenario_correct_wins: usize,
    pub scenario_correct_losses: usize,
    pub scenario_correct_ties: usize,
    pub scenario_grounded_wins: usize,
    pub scenario_grounded_losses: usize,
    pub scenario_grounded_ties: usize,
    /// Equal-weight mean of per-scenario attempt-averaged differences.
    pub correct_rate_delta_scenario_macro_ppm: Option<i64>,
    pub grounded_rate_delta_scenario_macro_ppm: Option<i64>,
    /// Scenario-cluster bootstrap percentile intervals; absent below 20 clusters.
    pub correct_rate_delta_cluster_ci95_ppm: Option<(i64, i64)>,
    pub grounded_rate_delta_cluster_ci95_ppm: Option<(i64, i64)>,
    pub both_scored_blocks: usize,
    pub candidate_budget_exceeded_blocks: usize,
    pub raw_budget_exceeded_blocks: usize,
    /// Candidate minus raw, over all paired blocks (budget misses remain in denominator).
    pub correct_rate_delta_all_ppm: i64,
    pub grounded_rate_delta_all_ppm: i64,
    /// Candidate minus raw, conditional on both arms scoring the same block.
    pub correct_rate_delta_both_scored_ppm: Option<i64>,
    pub grounded_rate_delta_both_scored_ppm: Option<i64>,
    /// Candidate minus raw average estimated tokens over all paired blocks.
    pub mean_estimated_tokens_delta: i128,
    /// Candidate minus raw actual input tokens where both arms returned usage.
    pub mean_actual_tokens_delta_both_scored: Option<i128>,
    /// Candidate minus raw reasoner elapsed time for paired scored blocks.
    pub mean_reasoner_time_delta_micros_both_scored: Option<i128>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextAblationAnalysis {
    pub arm_aggregates: Vec<ContextArmAggregate>,
    pub contrasts_vs_raw: Vec<ContextArmContrast>,
    pub cluster_bootstrap_seed: u64,
    pub cluster_bootstrap_replicates: usize,
    pub minimum_scenario_clusters_for_ci95: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ContextBlockKey {
    fault_family_id: String,
    scenario_id: String,
    paired_block_id: String,
    attempt_id: u32,
}

fn same_scenario_profile(
    left: &ContextExperimentProfile,
    right: &ContextExperimentProfile,
) -> bool {
    left.scenario_revision_digest == right.scenario_revision_digest
        && left.model_snapshot_digest == right.model_snapshot_digest
        && left.prompt_digest == right.prompt_digest
        && left.permission_profile_digest == right.permission_profile_digest
        && left.tool_catalog_digest == right.tool_catalog_digest
        && left.scorer_digest == right.scorer_digest
        && left.reasoning_effort == right.reasoning_effort
    // Sampling seeds are allowed to vary across attempts within the same
    // scenario cluster; all other execution conditions remain pinned.
}

fn rate_ppm(numerator: usize, denominator: usize) -> u32 {
    if denominator == 0 {
        0
    } else {
        ((numerator as u128 * 1_000_000) / denominator as u128) as u32
    }
}

fn mean_u128(total: u128, count: usize) -> Option<u128> {
    (count > 0).then(|| total / count as u128)
}

fn scenario_cluster_deltas(
    blocks: &BTreeMap<ContextBlockKey, BTreeMap<ContextArm, &ContextArmResult>>,
    family: &str,
    arm: ContextArm,
) -> Vec<(String, i64, i64)> {
    let mut by_scenario: BTreeMap<String, (i128, i128, usize)> = BTreeMap::new();

    for (key, block) in blocks {
        if key.fault_family_id != family {
            continue;
        }
        let Some(raw) = block.get(&ContextArm::RawSourceRecords).copied() else {
            continue;
        };
        let Some(candidate) = block.get(&arm).copied() else {
            continue;
        };
        // Budget misses are deliberately zero in the all-block outcome.
        let raw_correct = raw
            .score
            .as_ref()
            .is_some_and(|score| score.root_cause_correct);
        let candidate_correct = candidate
            .score
            .as_ref()
            .is_some_and(|score| score.root_cause_correct);
        let raw_grounded = raw
            .score
            .as_ref()
            .is_some_and(|score| score.root_object_grounded);
        let candidate_grounded = candidate
            .score
            .as_ref()
            .is_some_and(|score| score.root_object_grounded);

        let entry = by_scenario.entry(key.scenario_id.clone()).or_default();
        entry.0 += i128::from(u8::from(candidate_correct)) - i128::from(u8::from(raw_correct));
        entry.1 += i128::from(u8::from(candidate_grounded)) - i128::from(u8::from(raw_grounded));
        entry.2 += 1;
    }

    by_scenario
        .into_iter()
        .filter_map(|(scenario_id, (correct_delta, grounded_delta, attempts))| {
            if attempts == 0 {
                return None;
            }
            let denominator = attempts as i128;
            Some((
                scenario_id,
                ((correct_delta * 1_000_000) / denominator) as i64,
                ((grounded_delta * 1_000_000) / denominator) as i64,
            ))
        })
        .collect()
}

fn mean_i64(values: &[i64]) -> Option<i64> {
    (!values.is_empty()).then(|| {
        (values.iter().map(|value| i128::from(*value)).sum::<i128>() / values.len() as i128) as i64
    })
}

fn next_bootstrap_u64(state: &mut u64) -> u64 {
    // SplitMix64: deterministic, small, and adequate for percentile resampling.
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn derive_bootstrap_seed(base: u64, family: &str, arm: ContextArm) -> u64 {
    let mut hash = base ^ 0xcbf2_9ce4_8422_2325;
    let arm_name = format!("{arm:?}");
    for byte in family.bytes().chain(arm_name.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    if hash == 0 {
        1
    } else {
        hash
    }
}

fn scenario_cluster_bootstrap_ci95(scenario_deltas: &[i64], seed: u64) -> Option<(i64, i64)> {
    if scenario_deltas.len() < MIN_SCENARIO_CLUSTERS_FOR_CI95 {
        return None;
    }

    let mut state = seed;
    let cluster_count = scenario_deltas.len();
    let mut bootstrap_means = Vec::with_capacity(CLUSTER_BOOTSTRAP_REPLICATES);
    for _ in 0..CLUSTER_BOOTSTRAP_REPLICATES {
        let mut sum = 0_i128;
        for _ in 0..cluster_count {
            let index = (next_bootstrap_u64(&mut state) % cluster_count as u64) as usize;
            sum += i128::from(scenario_deltas[index]);
        }
        bootstrap_means.push((sum / cluster_count as i128) as i64);
    }
    bootstrap_means.sort_unstable();

    let last = bootstrap_means.len() - 1;
    let lower = (last * 25) / 1_000;
    let upper = (last * 975) / 1_000;
    Some((bootstrap_means[lower], bootstrap_means[upper]))
}

/// Analyze complete, preregistered five-arm blocks by fault family.
///
/// Incomplete blocks, duplicate arms or any mismatch in the scenario/model/prompt/
/// permissions/tools/scorer/effort/seed manifest are rejected. BudgetExceeded is
/// retained in the all-block denominator and separately exposed as coverage loss.
pub fn analyze_paired_context_results(
    results: &[ContextArmResult],
    bootstrap_seed: u64,
) -> Result<ContextAblationAnalysis, ContextExperimentError> {
    if results.is_empty() {
        return Err(ContextExperimentError::EmptyEvidence);
    }

    let mut blocks: BTreeMap<ContextBlockKey, BTreeMap<ContextArm, &ContextArmResult>> =
        BTreeMap::new();
    let mut reasoner_session_ids = BTreeSet::new();
    let mut scenario_attempts: BTreeMap<(String, String, u32), String> = BTreeMap::new();

    for result in results {
        if !non_empty(&result.scenario_id)
            || !non_empty(&result.fault_family_id)
            || !non_empty(&result.paired_block_id)
            || !non_empty(&result.profile.reasoning_effort)
        {
            return Err(ContextExperimentError::InvalidExperimentProfile);
        }
        if (result.status == ArmStatus::Scored) != result.score.is_some()
            || result.reasoner_time_micros.is_some() != result.usage.is_some()
            || result.reasoner_time_micros.is_some() != result.reasoner_session_id.is_some()
        {
            return Err(ContextExperimentError::InvalidExperimentProfile);
        }
        if let Some(session_id) = &result.reasoner_session_id {
            if !non_empty(session_id) {
                return Err(ContextExperimentError::EmptyReasonerSessionId);
            }
            if !reasoner_session_ids.insert(session_id.as_str()) {
                return Err(ContextExperimentError::DuplicateReasonerSessionId);
            }
        }
        let attempt_key = (
            result.fault_family_id.clone(),
            result.scenario_id.clone(),
            result.attempt_id,
        );
        match scenario_attempts.get(&attempt_key) {
            Some(existing_block) if existing_block != &result.paired_block_id => {
                return Err(ContextExperimentError::DuplicateScenarioAttempt);
            }
            Some(_) => {}
            None => {
                scenario_attempts.insert(attempt_key, result.paired_block_id.clone());
            }
        }
        let key = ContextBlockKey {
            fault_family_id: result.fault_family_id.clone(),
            scenario_id: result.scenario_id.clone(),
            paired_block_id: result.paired_block_id.clone(),
            attempt_id: result.attempt_id,
        };
        let block = blocks.entry(key).or_default();
        if block.insert(result.arm, result).is_some() {
            return Err(ContextExperimentError::DuplicateArmResult);
        }
    }

    let mut scenario_profiles: BTreeMap<
        (String, String),
        &ContextExperimentProfile,
    > = BTreeMap::new();
    for (key, block) in &blocks {
        let first = block
            .get(&ContextArm::RawSourceRecords)
            .copied()
            .ok_or(ContextExperimentError::IncompletePairedBlock)?;
        let scenario_key = (key.fault_family_id.clone(), key.scenario_id.clone());
        if let Some(reference) = scenario_profiles.get(&scenario_key) {
            if !same_scenario_profile(reference, &first.profile) {
                return Err(ContextExperimentError::ManifestMismatch);
            }
        } else {
            scenario_profiles.insert(scenario_key, &first.profile);
        }
    }

    for block in blocks.values() {
        if block.len() != CONTEXT_COMPILATION_ARMS.len()
            || CONTEXT_COMPILATION_ARMS
                .iter()
                .any(|arm| !block.contains_key(arm))
        {
            return Err(ContextExperimentError::IncompletePairedBlock);
        }
        let first = block
            .get(&ContextArm::RawSourceRecords)
            .ok_or(ContextExperimentError::IncompletePairedBlock)?;
        if block.values().any(|result| {
            result.profile != first.profile
                || result.fault_family_id != first.fault_family_id
                || result.scenario_id != first.scenario_id
                || result.paired_block_id != first.paired_block_id
                || result.attempt_id != first.attempt_id
        }) {
            return Err(ContextExperimentError::ManifestMismatch);
        }
    }

    let mut by_family: BTreeMap<String, Vec<&BTreeMap<ContextArm, &ContextArmResult>>> =
        BTreeMap::new();
    for (key, block) in &blocks {
        by_family
            .entry(key.fault_family_id.clone())
            .or_default()
            .push(block);
    }

    let mut arm_aggregates = Vec::new();
    let mut contrasts_vs_raw = Vec::new();

    for (family, family_blocks) in by_family {
        let n = family_blocks.len();
        for arm in CONTEXT_COMPILATION_ARMS {
            let rows: Vec<_> = family_blocks
                .iter()
                .map(|block| block.get(&arm).copied().expect("validated complete block"))
                .collect();

            let scored: Vec<_> = rows
                .iter()
                .copied()
                .filter(|row| row.status == ArmStatus::Scored)
                .collect();
            let budget_exceeded_blocks = n - scored.len();
            let correct_roots = scored
                .iter()
                .filter(|row| {
                    row.score
                        .as_ref()
                        .is_some_and(|score| score.root_cause_correct)
                })
                .count();
            let grounded_roots = scored
                .iter()
                .filter(|row| {
                    row.score
                        .as_ref()
                        .is_some_and(|score| score.root_object_grounded)
                })
                .count();

            let total_context_bytes: u128 = rows.iter().map(|row| row.context_bytes as u128).sum();
            let total_estimated_tokens: u128 = rows
                .iter()
                .map(|row| row.estimated_input_tokens as u128)
                .sum();
            let actual_tokens: Vec<u128> = scored
                .iter()
                .filter_map(|row| row.usage.as_ref().map(|usage| usage.input_tokens as u128))
                .collect();
            let reasoner_times: Vec<u128> = scored
                .iter()
                .filter_map(|row| row.reasoner_time_micros)
                .collect();

            arm_aggregates.push(ContextArmAggregate {
                fault_family_id: family.clone(),
                arm,
                paired_blocks: n,
                scored_blocks: scored.len(),
                budget_exceeded_blocks,
                correct_roots,
                grounded_roots,
                correct_rate_all_ppm: rate_ppm(correct_roots, n),
                grounded_rate_all_ppm: rate_ppm(grounded_roots, n),
                correct_rate_scored_ppm: (scored.len() > 0)
                    .then(|| rate_ppm(correct_roots, scored.len())),
                grounded_rate_scored_ppm: (scored.len() > 0)
                    .then(|| rate_ppm(grounded_roots, scored.len())),
                mean_context_bytes: total_context_bytes / n as u128,
                mean_estimated_input_tokens: total_estimated_tokens / n as u128,
                mean_actual_input_tokens_scored: mean_u128(
                    actual_tokens.iter().sum(),
                    actual_tokens.len(),
                ),
                mean_reasoner_time_micros_scored: mean_u128(
                    reasoner_times.iter().sum(),
                    reasoner_times.len(),
                ),
            });
        }

        let raw_rows: Vec<_> = family_blocks
            .iter()
            .map(|block| {
                block
                    .get(&ContextArm::RawSourceRecords)
                    .copied()
                    .expect("validated")
            })
            .collect();

        for arm in CONTEXT_COMPILATION_ARMS
            .into_iter()
            .filter(|arm| *arm != ContextArm::RawSourceRecords)
        {
            let candidate_rows: Vec<_> = family_blocks
                .iter()
                .map(|block| block.get(&arm).copied().expect("validated"))
                .collect();

            let mut both_scored = 0usize;
            let mut raw_correct_both = 0usize;
            let mut candidate_correct_both = 0usize;
            let mut raw_grounded_both = 0usize;
            let mut candidate_grounded_both = 0usize;
            let mut actual_token_delta = 0i128;
            let mut actual_token_pairs = 0usize;
            let mut time_delta = 0i128;
            let mut time_pairs = 0usize;

            for (raw, candidate) in raw_rows.iter().zip(candidate_rows.iter()) {
                if raw.status == ArmStatus::Scored && candidate.status == ArmStatus::Scored {
                    both_scored += 1;
                    let raw_score = raw.score.as_ref().expect("scored rows have a score");
                    let candidate_score =
                        candidate.score.as_ref().expect("scored rows have a score");
                    raw_correct_both += usize::from(raw_score.root_cause_correct);
                    candidate_correct_both += usize::from(candidate_score.root_cause_correct);
                    raw_grounded_both += usize::from(raw_score.root_object_grounded);
                    candidate_grounded_both += usize::from(candidate_score.root_object_grounded);

                    if let (Some(raw_usage), Some(candidate_usage)) = (&raw.usage, &candidate.usage)
                    {
                        actual_token_delta +=
                            candidate_usage.input_tokens as i128 - raw_usage.input_tokens as i128;
                        actual_token_pairs += 1;
                    }
                    if let (Some(raw_time), Some(candidate_time)) =
                        (raw.reasoner_time_micros, candidate.reasoner_time_micros)
                    {
                        time_delta += candidate_time as i128 - raw_time as i128;
                        time_pairs += 1;
                    }
                }
            }

            let raw_correct_all = raw_rows
                .iter()
                .filter(|row| {
                    row.score
                        .as_ref()
                        .is_some_and(|score| score.root_cause_correct)
                })
                .count();
            let candidate_correct_all = candidate_rows
                .iter()
                .filter(|row| {
                    row.score
                        .as_ref()
                        .is_some_and(|score| score.root_cause_correct)
                })
                .count();
            let raw_grounded_all = raw_rows
                .iter()
                .filter(|row| {
                    row.score
                        .as_ref()
                        .is_some_and(|score| score.root_object_grounded)
                })
                .count();
            let candidate_grounded_all = candidate_rows
                .iter()
                .filter(|row| {
                    row.score
                        .as_ref()
                        .is_some_and(|score| score.root_object_grounded)
                })
                .count();

            let raw_token_total: u128 = raw_rows
                .iter()
                .map(|row| row.estimated_input_tokens as u128)
                .sum();
            let candidate_token_total: u128 = candidate_rows
                .iter()
                .map(|row| row.estimated_input_tokens as u128)
                .sum();

            let scenario_deltas = scenario_cluster_deltas(&blocks, &family, arm);
            let correct_cluster_deltas: Vec<_> = scenario_deltas
                .iter()
                .map(|(_, correct, _)| *correct)
                .collect();
            let grounded_cluster_deltas: Vec<_> = scenario_deltas
                .iter()
                .map(|(_, _, grounded)| *grounded)
                .collect();
            let scenario_correct_wins = correct_cluster_deltas
                .iter()
                .filter(|delta| **delta > 0)
                .count();
            let scenario_correct_losses = correct_cluster_deltas
                .iter()
                .filter(|delta| **delta < 0)
                .count();
            let scenario_correct_ties = correct_cluster_deltas
                .iter()
                .filter(|delta| **delta == 0)
                .count();
            let scenario_grounded_wins = grounded_cluster_deltas
                .iter()
                .filter(|delta| **delta > 0)
                .count();
            let scenario_grounded_losses = grounded_cluster_deltas
                .iter()
                .filter(|delta| **delta < 0)
                .count();
            let scenario_grounded_ties = grounded_cluster_deltas
                .iter()
                .filter(|delta| **delta == 0)
                .count();
            let stratum_seed = derive_bootstrap_seed(bootstrap_seed, &family, arm);

            contrasts_vs_raw.push(ContextArmContrast {
                fault_family_id: family.clone(),
                arm,
                paired_blocks: n,
                scenario_clusters: scenario_deltas.len(),
                scenario_correct_wins,
                scenario_correct_losses,
                scenario_correct_ties,
                scenario_grounded_wins,
                scenario_grounded_losses,
                scenario_grounded_ties,
                correct_rate_delta_scenario_macro_ppm: mean_i64(&correct_cluster_deltas),
                grounded_rate_delta_scenario_macro_ppm: mean_i64(&grounded_cluster_deltas),
                correct_rate_delta_cluster_ci95_ppm: scenario_cluster_bootstrap_ci95(
                    &correct_cluster_deltas,
                    stratum_seed,
                ),
                grounded_rate_delta_cluster_ci95_ppm: scenario_cluster_bootstrap_ci95(
                    &grounded_cluster_deltas,
                    stratum_seed ^ 0xa5a5_5a5a_d3c1_b2e7,
                ),
                both_scored_blocks: both_scored,
                candidate_budget_exceeded_blocks: candidate_rows
                    .iter()
                    .filter(|row| row.status == ArmStatus::BudgetExceeded)
                    .count(),
                raw_budget_exceeded_blocks: raw_rows
                    .iter()
                    .filter(|row| row.status == ArmStatus::BudgetExceeded)
                    .count(),
                correct_rate_delta_all_ppm: rate_ppm(candidate_correct_all, n) as i64
                    - rate_ppm(raw_correct_all, n) as i64,
                grounded_rate_delta_all_ppm: rate_ppm(candidate_grounded_all, n) as i64
                    - rate_ppm(raw_grounded_all, n) as i64,
                correct_rate_delta_both_scored_ppm: (both_scored > 0).then(|| {
                    rate_ppm(candidate_correct_both, both_scored) as i64
                        - rate_ppm(raw_correct_both, both_scored) as i64
                }),
                grounded_rate_delta_both_scored_ppm: (both_scored > 0).then(|| {
                    rate_ppm(candidate_grounded_both, both_scored) as i64
                        - rate_ppm(raw_grounded_both, both_scored) as i64
                }),
                mean_estimated_tokens_delta: (candidate_token_total as i128
                    - raw_token_total as i128)
                    / n as i128,
                mean_actual_tokens_delta_both_scored: mean_i128(
                    actual_token_delta,
                    actual_token_pairs,
                ),
                mean_reasoner_time_delta_micros_both_scored: mean_i128(time_delta, time_pairs),
            });
        }
    }

    arm_aggregates.sort_by(|left, right| {
        (&left.fault_family_id, left.arm).cmp(&(&right.fault_family_id, right.arm))
    });
    contrasts_vs_raw.sort_by(|left, right| {
        (&left.fault_family_id, left.arm).cmp(&(&right.fault_family_id, right.arm))
    });

    Ok(ContextAblationAnalysis {
        arm_aggregates,
        contrasts_vs_raw,
        cluster_bootstrap_seed: bootstrap_seed,
        cluster_bootstrap_replicates: CLUSTER_BOOTSTRAP_REPLICATES,
        minimum_scenario_clusters_for_ci95: MIN_SCENARIO_CLUSTERS_FOR_CI95,
    })
}

fn mean_i128(total: i128, count: usize) -> Option<i128> {
    (count > 0).then(|| total / count as i128)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> Digest {
        Digest::from_byte(byte)
    }

    fn evidence(
        id: &str,
        source: &str,
        object_ref: &str,
        kind: EvidenceKind,
        observed: u64,
    ) -> ContextEvidence {
        ContextEvidence {
            evidence_id: id.into(),
            source_id: source.into(),
            control_domain: format!("domain-{source}"),
            object_ref: object_ref.into(),
            object_kind: kind,
            field_path: "status.condition".into(),
            value: Some("degraded".into()),
            state: EvidenceState::Present,
            event_time_ms: observed.saturating_sub(2),
            observed_at_ms: observed,
            valid_until_ms: Some(10_000),
            digest: digest(id.bytes().next().unwrap_or_default()),
        }
    }

    fn input() -> ContextExperimentInput {
        let evidence = vec![
            evidence(
                "ev-alert",
                "alertmanager",
                "checkout-api",
                EvidenceKind::Alert,
                990,
            ),
            evidence(
                "ev-metric",
                "prometheus",
                "checkout-api",
                EvidenceKind::Metric,
                995,
            ),
            evidence(
                "ev-change",
                "git",
                "payments-config",
                EvidenceKind::Change,
                980,
            ),
            evidence(
                "ev-node",
                "node-exporter",
                "worker-7",
                EvidenceKind::Node,
                800,
            ),
            ContextEvidence {
                evidence_id: "ev-unknown".into(),
                source_id: "traces".into(),
                control_domain: "domain-traces".into(),
                object_ref: "checkout-api".into(),
                object_kind: EvidenceKind::Trace,
                field_path: "span.status".into(),
                value: None,
                state: EvidenceState::Unknown,
                event_time_ms: 998,
                observed_at_ms: 999,
                valid_until_ms: Some(10_000),
                digest: digest(5),
            },
        ];
        ContextExperimentInput {
            scenario_id: "scenario-1".into(),
            fault_family_id: "queue_dependency".into(),
            paired_block_id: "fault-kafka-01-seed-7".into(),
            attempt_id: 1,
            profile: ContextExperimentProfile {
                scenario_revision_digest: digest(10),
                model_snapshot_digest: digest(11),
                prompt_digest: digest(12),
                permission_profile_digest: digest(13),
                tool_catalog_digest: digest(14),
                scorer_digest: digest(15),
                reasoning_effort: "medium".into(),
                sampling_seed: 7,
            },
            target_object_ref: "checkout-api".into(),
            audited_at_ms: 1_000,
            maximum_evidence_age_ms: 100,
            evidence,
            relations: vec![ContextRelation {
                relation_id: "rel-1".into(),
                evidence_id: "ev-change".into(),
                from_object: "checkout-api".into(),
                to_object: "payments-config".into(),
                relation: RelationKind::DependsOn,
                state: EvidenceState::Present,
                observed_at_ms: 980,
            }],
        }
    }

    fn truth() -> ContextGroundTruth {
        ContextGroundTruth {
            correct_root_object_ref: "payments-config".into(),
            required_evidence_ids: ["ev-change".to_owned()].into_iter().collect(),
        }
    }

    struct ScriptedReasoner {
        calls: Vec<String>,
        next_session: u32,
    }

    impl ContextReasoner for ScriptedReasoner {
        fn start_fresh_session(&mut self, _profile: &ContextExperimentProfile) -> String {
            self.next_session += 1;
            format!("fresh-session-{}", self.next_session)
        }

        fn diagnose(
            &mut self,
            session_id: &str,
            rendered_context: &str,
            _budget: ContextBudget,
        ) -> ReasonerResponse {
            self.calls.push(session_id.to_owned());
            assert!(rendered_context.contains("TARGET|checkout-api"));
            ReasonerResponse {
                session_id: session_id.to_owned(),
                answer: DiagnosisAnswer {
                    root_object_ref: Some("payments-config".into()),
                    supporting_evidence_ids: vec!["ev-change".into()],
                    alternative_object_refs: vec!["worker-7".into()],
                    abstained: false,
                },
                usage: ReasonerUsage {
                    input_tokens: (rendered_context.len().div_ceil(4)) as u64,
                    output_tokens: 32,
                    tool_calls: 0,
                    reported_cost_microusd: 1_000,
                },
            }
        }
    }

    fn budget() -> ContextBudget {
        ContextBudget {
            maximum_context_bytes: 100_000,
            maximum_input_tokens: 30_000,
            maximum_output_tokens: 1_000,
            maximum_tool_calls: 0,
            maximum_wall_time_ms: 1_000,
        }
    }

    fn synthetic_arm_result(
        scenario_index: usize,
        attempt_id: u32,
        arm: ContextArm,
        correct: bool,
        grounded: bool,
    ) -> ContextArmResult {
        let input = input();
        let scenario_id = format!("fault-{scenario_index:02}");
        ContextArmResult {
            scenario_id: scenario_id.clone(),
            fault_family_id: "queue_dependency".into(),
            paired_block_id: format!("{scenario_id}-attempt-{attempt_id}"),
            attempt_id,
            profile: input.profile,
            arm,
            status: ArmStatus::Scored,
            compiler_time_micros: 10,
            reasoner_time_micros: Some(100),
            reasoner_session_id: Some(format!("{scenario_id}-{attempt_id}-{arm:?}")),
            context_bytes: 512,
            estimated_input_tokens: 128,
            usage: Some(ReasonerUsage {
                input_tokens: 128,
                output_tokens: 32,
                tool_calls: 0,
                reported_cost_microusd: 100,
            }),
            answer: Some(DiagnosisAnswer {
                root_object_ref: Some("payments-config".into()),
                supporting_evidence_ids: vec!["ev-change".into()],
                alternative_object_refs: Vec::new(),
                abstained: false,
            }),
            score: Some(ContextScore {
                root_cause_correct: correct,
                root_object_visible: true,
                root_object_grounded: grounded,
                supported_evidence_precision_ppm: 1_000_000,
                required_evidence_recall_ppm: if grounded { 1_000_000 } else { 0 },
                unsupported_evidence_claims: 0,
                required_evidence_visible: 1,
                required_evidence_total: 1,
            }),
        }
    }

    #[test]
    fn all_five_arms_preserve_the_same_evidence_identity_universe() {
        let input = input();
        let expected: BTreeSet<_> = input
            .evidence
            .iter()
            .map(|item| item.evidence_id.clone())
            .collect();
        for arm in CONTEXT_COMPILATION_ARMS {
            let (packet, _) = compile_context_packet(&input, arm).unwrap();
            assert_eq!(packet.evidence_ids, expected);
        }
    }

    #[test]
    fn topology_and_temporal_arms_add_joins_without_erasing_unknowns() {
        let input = input();
        let (normalized, _) =
            compile_context_packet(&input, ContextArm::NormalizedRecords).unwrap();
        let (topology, _) = compile_context_packet(&input, ContextArm::TopologyAndDiffs).unwrap();
        let (temporal, _) =
            compile_context_packet(&input, ContextArm::TemporalAndFreshness).unwrap();

        assert!(normalized.relations.is_empty());
        assert_eq!(topology.relations.len(), 1);
        assert_eq!(topology.object_diffs.len(), 1);
        let unknown = temporal
            .temporal
            .iter()
            .find(|item| item.evidence_id == "ev-unknown")
            .unwrap();
        assert_eq!(unknown.freshness, Freshness::Unknown);
    }

    #[test]
    fn ranking_is_deterministic_and_never_uses_evaluator_truth() {
        let input = input();
        let (packet, _) = compile_context_packet(&input, ContextArm::RankedCausalEvidence).unwrap();
        assert_eq!(
            packet.ranked_candidates.first().unwrap().object_ref,
            "checkout-api"
        );
        assert!(packet
            .ranked_candidates
            .iter()
            .any(|candidate| candidate.object_ref == "payments-config"));
        assert!(packet.rendered_context.contains("CANDIDATE|"));
    }

    #[test]
    fn paired_runner_uses_same_reasoner_and_scores_only_after_response() {
        let input = input();
        let mut reasoner = ScriptedReasoner {
            calls: Vec::new(),
            next_session: 0,
        };
        let results = run_paired_context_ablation(
            &input,
            &truth(),
            budget(),
            &CONTEXT_COMPILATION_ARMS,
            &mut reasoner,
        )
        .unwrap();
        assert_eq!(reasoner.calls.len(), CONTEXT_COMPILATION_ARMS.len());
        assert_eq!(
            reasoner.calls.iter().collect::<BTreeSet<_>>().len(),
            CONTEXT_COMPILATION_ARMS.len()
        );
        assert_eq!(results.len(), 5);
        assert!(results
            .iter()
            .all(|result| result.status == ArmStatus::Scored));
        assert!(results.iter().all(|result| result.profile == input.profile));
        assert!(results
            .iter()
            .all(|result| result.reasoner_session_id.is_some()));
        assert_eq!(
            results
                .iter()
                .filter_map(|result| result.reasoner_session_id.as_deref())
                .collect::<BTreeSet<_>>()
                .len(),
            5
        );
        assert!(results
            .iter()
            .all(|result| result.paired_block_id == "fault-kafka-01-seed-7"));
        assert!(results.iter().all(|result| result.attempt_id == 1));
        assert!(results[0].score.as_ref().unwrap().required_evidence_total == 1);
    }

    #[test]
    fn model_visible_context_excludes_arm_and_evaluator_pairing_metadata() {
        let input = input();
        for arm in CONTEXT_COMPILATION_ARMS {
            let (packet, _) = compile_context_packet(&input, arm).unwrap();
            assert!(!packet.rendered_context.contains("scenario-1"));
            assert!(!packet.rendered_context.contains("fault-kafka-01-seed-7"));
            assert!(!packet.rendered_context.contains("attempt=1"));
            assert!(!packet.rendered_context.contains("RawSourceRecords"));
            assert!(!packet.rendered_context.contains("RankedCausalEvidence"));
            assert!(!packet.rendered_context.contains("model="));
            assert!(!packet.rendered_context.contains("prompt="));
            assert!(!packet.rendered_context.contains("scorer="));
            assert!(!packet.rendered_context.contains("seed=7"));
            assert!(packet.rendered_context.contains("TARGET|checkout-api"));
        }
    }

    struct ReusedSessionReasoner;

    impl ContextReasoner for ReusedSessionReasoner {
        fn start_fresh_session(&mut self, _profile: &ContextExperimentProfile) -> String {
            "reused-session".into()
        }

        fn diagnose(
            &mut self,
            session_id: &str,
            _rendered_context: &str,
            _budget: ContextBudget,
        ) -> ReasonerResponse {
            ReasonerResponse {
                session_id: session_id.into(),
                answer: DiagnosisAnswer {
                    root_object_ref: None,
                    supporting_evidence_ids: Vec::new(),
                    alternative_object_refs: Vec::new(),
                    abstained: true,
                },
                usage: ReasonerUsage {
                    input_tokens: 1,
                    output_tokens: 1,
                    tool_calls: 0,
                    reported_cost_microusd: 0,
                },
            }
        }
    }

    #[test]
    fn runner_rejects_reuse_of_a_reasoner_conversation_across_arms() {
        assert_eq!(
            run_paired_context_ablation(
                &input(),
                &truth(),
                budget(),
                &CONTEXT_COMPILATION_ARMS,
                &mut ReusedSessionReasoner,
            ),
            Err(ContextExperimentError::DuplicateReasonerSessionId)
        );
    }

    #[test]
    fn paired_analysis_reports_all_blocks_scored_pairs_and_fault_family() {
        let input = input();
        let mut reasoner = ScriptedReasoner {
            calls: Vec::new(),
            next_session: 0,
        };
        let results = run_paired_context_ablation(
            &input,
            &truth(),
            budget(),
            &CONTEXT_COMPILATION_ARMS,
            &mut reasoner,
        )
        .unwrap();

        let analysis = analyze_paired_context_results(&results, 0xC0FF_EE01_5EED_2026).unwrap();
        assert_eq!(analysis.arm_aggregates.len(), 5);
        assert_eq!(analysis.contrasts_vs_raw.len(), 4);
        assert!(analysis
            .arm_aggregates
            .iter()
            .all(|row| row.fault_family_id == "queue_dependency"));
        assert!(analysis
            .arm_aggregates
            .iter()
            .all(|row| row.paired_blocks == 1 && row.scored_blocks == 1));
        assert!(analysis
            .contrasts_vs_raw
            .iter()
            .all(|row| row.paired_blocks == 1 && row.both_scored_blocks == 1));
        assert!(analysis
            .contrasts_vs_raw
            .iter()
            .all(|row| row.scenario_clusters == 1));
        assert!(analysis
            .contrasts_vs_raw
            .iter()
            .all(|row| row.correct_rate_delta_cluster_ci95_ppm.is_none()));
        assert_eq!(analysis.cluster_bootstrap_seed, 0xC0FF_EE01_5EED_2026);
    }

    #[test]
    fn clustered_bootstrap_resamples_fault_scenarios_not_repeated_attempts() {
        let mut results = Vec::new();
        for scenario_index in 0..20 {
            for attempt_id in 1..=2 {
                for arm in CONTEXT_COMPILATION_ARMS {
                    // The candidate has ten scenario-level wins and ten ties.
                    let raw_correct = scenario_index % 2 == 0;
                    let correct = if arm == ContextArm::TemporalAndFreshness {
                        true
                    } else {
                        raw_correct
                    };
                    results.push(synthetic_arm_result(
                        scenario_index,
                        attempt_id,
                        arm,
                        correct,
                        correct,
                    ));
                }
            }
        }

        let analysis = analyze_paired_context_results(&results, 0x1234_5678_9abc_def0).unwrap();
        let temporal = analysis
            .contrasts_vs_raw
            .iter()
            .find(|row| row.arm == ContextArm::TemporalAndFreshness)
            .unwrap();

        assert_eq!(temporal.paired_blocks, 40);
        assert_eq!(temporal.scenario_clusters, 20);
        assert_eq!(temporal.scenario_correct_wins, 10);
        assert_eq!(temporal.scenario_correct_losses, 0);
        assert_eq!(temporal.scenario_correct_ties, 10);
        assert_eq!(
            temporal.correct_rate_delta_scenario_macro_ppm,
            Some(500_000)
        );
        let (lower, upper) = temporal.correct_rate_delta_cluster_ci95_ppm.unwrap();
        assert!(lower > 0);
        assert!(upper >= lower);
        assert!(upper <= 1_000_000);
        assert_eq!(analysis.cluster_bootstrap_replicates, 10_000);
        assert_eq!(analysis.minimum_scenario_clusters_for_ci95, 20);
    }

    #[test]
    fn paired_analysis_rejects_duplicate_incomplete_and_manifest_mismatched_blocks() {
        let input = input();
        let mut reasoner = ScriptedReasoner {
            calls: Vec::new(),
            next_session: 0,
        };
        let results = run_paired_context_ablation(
            &input,
            &truth(),
            budget(),
            &CONTEXT_COMPILATION_ARMS,
            &mut reasoner,
        )
        .unwrap();

        let mut duplicate = results.clone();
        duplicate.push(results[0].clone());
        assert_eq!(
            analyze_paired_context_results(&duplicate, 0xC0FF_EE01_5EED_2026),
            Err(ContextExperimentError::DuplicateArmResult)
        );

        assert_eq!(
            analyze_paired_context_results(&results[..4], 0xC0FF_EE01_5EED_2026),
            Err(ContextExperimentError::IncompletePairedBlock)
        );

        let mut mismatched = results.clone();
        mismatched[1].profile.model_snapshot_digest = Digest::from_byte(99);
        assert_eq!(
            analyze_paired_context_results(&mismatched, 0xC0FF_EE01_5EED_2026),
            Err(ContextExperimentError::ManifestMismatch)
        );
    }

    #[test]
    fn same_scenario_attempts_may_vary_sampling_seed_but_not_model_manifest() {
        let mut results = Vec::new();
        for attempt_id in 1..=2 {
            for arm in CONTEXT_COMPILATION_ARMS {
                let mut row = synthetic_arm_result(
                    0,
                    attempt_id,
                    arm,
                    true,
                    true,
                );
                row.profile.sampling_seed = 100 + u64::from(attempt_id);
                results.push(row);
            }
        }
        assert!(analyze_paired_context_results(&results, 77).is_ok());

        let mut mismatched = results.clone();
        mismatched[5].profile.prompt_digest = Digest::from_byte(99);
        assert_eq!(
            analyze_paired_context_results(&mismatched, 77),
            Err(ContextExperimentError::ManifestMismatch)
        );
    }

    #[test]
    fn duplicate_attempt_id_cannot_inflate_one_scenario_cluster() {
        let mut results = Vec::new();
        for block_copy in 0..2 {
            for arm in CONTEXT_COMPILATION_ARMS {
                let mut row = synthetic_arm_result(0, 1, arm, true, true);
                row.paired_block_id = format!("duplicate-attempt-{block_copy}");
                row.reasoner_session_id = Some(format!("session-{block_copy}-{arm:?}"));
                results.push(row);
            }
        }
        assert_eq!(
            analyze_paired_context_results(&results, 77),
            Err(ContextExperimentError::DuplicateScenarioAttempt)
        );
    }

    #[test]
    fn paired_analysis_keeps_budget_misses_in_the_full_denominator() {
        let tight = ContextBudget {
            maximum_context_bytes: 16,
            ..budget()
        };
        let results = run_paired_context_ablation(
            &input(),
            &truth(),
            tight,
            &CONTEXT_COMPILATION_ARMS,
            &mut ScriptedReasoner {
                calls: Vec::new(),
                next_session: 0,
            },
        )
        .unwrap();
        let analysis = analyze_paired_context_results(&results, 0xC0FF_EE01_5EED_2026).unwrap();

        assert!(analysis.arm_aggregates.iter().all(|row| {
            row.paired_blocks == 1
                && row.scored_blocks == 0
                && row.budget_exceeded_blocks == 1
                && row.correct_rate_all_ppm == 0
                && row.grounded_rate_all_ppm == 0
                && row.correct_rate_scored_ppm.is_none()
                && row.grounded_rate_scored_ppm.is_none()
        }));
        assert!(analysis.contrasts_vs_raw.iter().all(|row| {
            row.both_scored_blocks == 0
                && row.correct_rate_delta_all_ppm == 0
                && row.grounded_rate_delta_all_ppm == 0
                && row.correct_rate_delta_both_scored_ppm.is_none()
                && row.grounded_rate_delta_both_scored_ppm.is_none()
        }));
    }

    #[test]
    fn empty_paired_subsets_produce_none_instead_of_dividing_by_zero() {
        assert_eq!(mean_u128(0, 0), None);
        assert_eq!(mean_i128(0, 0), None);
        assert_eq!(mean_u128(12, 3), Some(4));
        assert_eq!(mean_i128(-12, 3), Some(-4));
    }

    #[test]
    fn stale_future_and_expired_observations_keep_distinct_freshness_states() {
        let mut input = input();
        input.evidence.push(ContextEvidence {
            evidence_id: "ev-stale".into(),
            source_id: "logs".into(),
            control_domain: "domain-logs".into(),
            object_ref: "worker-7".into(),
            object_kind: EvidenceKind::Log,
            field_path: "container.restart_count".into(),
            value: Some("4".into()),
            state: EvidenceState::Present,
            event_time_ms: 500,
            observed_at_ms: 500,
            valid_until_ms: Some(5_000),
            digest: digest(6),
        });
        input.evidence.push(ContextEvidence {
            evidence_id: "ev-future".into(),
            source_id: "clock".into(),
            control_domain: "domain-clock".into(),
            object_ref: "worker-7".into(),
            object_kind: EvidenceKind::Metric,
            field_path: "clock.skew".into(),
            value: Some("true".into()),
            state: EvidenceState::Present,
            event_time_ms: 1_100,
            observed_at_ms: 1_100,
            valid_until_ms: Some(2_000),
            digest: digest(7),
        });
        input.evidence.push(ContextEvidence {
            evidence_id: "ev-expired".into(),
            source_id: "config".into(),
            control_domain: "domain-config".into(),
            object_ref: "payments-config".into(),
            object_kind: EvidenceKind::Configuration,
            field_path: "revision".into(),
            value: Some("old".into()),
            state: EvidenceState::Present,
            event_time_ms: 800,
            observed_at_ms: 900,
            valid_until_ms: Some(950),
            digest: digest(8),
        });

        let (packet, _) = compile_context_packet(&input, ContextArm::TemporalAndFreshness).unwrap();
        let states: BTreeMap<_, _> = packet
            .temporal
            .iter()
            .map(|item| (item.evidence_id.as_str(), item.freshness))
            .collect();
        assert_eq!(states["ev-stale"], Freshness::Stale);
        assert_eq!(states["ev-future"], Freshness::FutureDated);
        assert_eq!(states["ev-expired"], Freshness::Expired);
    }

    #[test]
    fn raw_context_encoding_escapes_delimiters_and_preserves_validity_bounds() {
        let mut input = input();
        input.evidence[0].value = Some("bad|value\\ncontinued".into());
        let (packet, _) = compile_context_packet(&input, ContextArm::RawSourceRecords).unwrap();
        assert!(packet
            .rendered_context
            .contains("bad\\|value\\\\ncontinued"));
        assert!(packet.rendered_context.contains("Some(10000)"));
    }

    #[test]
    fn experiment_manifest_must_pin_reasoning_effort_and_pairing_block() {
        let mut bad_profile = input();
        bad_profile.profile.reasoning_effort.clear();
        assert_eq!(
            compile_context_packet(&bad_profile, ContextArm::RawSourceRecords),
            Err(ContextExperimentError::InvalidExperimentProfile)
        );

        let mut bad_block = input();
        bad_block.paired_block_id.clear();
        assert_eq!(
            compile_context_packet(&bad_block, ContextArm::RawSourceRecords),
            Err(ContextExperimentError::EmptyPairedBlockId)
        );
    }

    #[test]
    fn correct_root_guess_without_visible_support_is_not_grounded() {
        let input = input();
        let (packet, _) = compile_context_packet(&input, ContextArm::NormalizedRecords).unwrap();
        let answer = DiagnosisAnswer {
            root_object_ref: Some("payments-config".into()),
            supporting_evidence_ids: vec!["ev-node".into()],
            alternative_object_refs: Vec::new(),
            abstained: false,
        };
        let score = score_answer(&packet, &truth(), &answer);
        assert!(score.root_cause_correct);
        assert!(score.root_object_visible);
        assert!(!score.root_object_grounded);
        assert_eq!(score.required_evidence_recall_ppm, 0);
    }

    #[test]
    fn correct_root_with_cited_dependency_evidence_is_grounded() {
        let input = input();
        let (packet, _) = compile_context_packet(&input, ContextArm::TopologyAndDiffs).unwrap();
        let answer = DiagnosisAnswer {
            root_object_ref: Some("payments-config".into()),
            supporting_evidence_ids: vec!["ev-change".into()],
            alternative_object_refs: Vec::new(),
            abstained: false,
        };
        let score = score_answer(&packet, &truth(), &answer);
        assert!(score.root_cause_correct);
        assert!(score.root_object_visible);
        assert!(score.root_object_grounded);
        assert_eq!(score.required_evidence_recall_ppm, 1_000_000);
    }

    #[test]
    fn missing_required_evidence_and_unknown_records_remain_distinct() {
        let mut invalid_truth = truth();
        invalid_truth
            .required_evidence_ids
            .insert("not-in-universe".into());
        assert_eq!(
            run_paired_context_ablation(
                &input(),
                &invalid_truth,
                budget(),
                &CONTEXT_COMPILATION_ARMS,
                &mut ScriptedReasoner {
                calls: Vec::new(),
                next_session: 0,
            },
            ),
            Err(ContextExperimentError::GroundTruthEvidenceMissing)
        );
    }

    #[test]
    fn context_budget_overflow_is_kept_as_an_arm_result_not_dropped() {
        let tight = ContextBudget {
            maximum_context_bytes: 16,
            ..budget()
        };
        let results = run_paired_context_ablation(
            &input(),
            &truth(),
            tight,
            &CONTEXT_COMPILATION_ARMS,
            &mut ScriptedReasoner {
                calls: Vec::new(),
                next_session: 0,
            },
        )
        .unwrap();
        assert_eq!(results.len(), 5);
        assert!(results
            .iter()
            .all(|result| result.status == ArmStatus::BudgetExceeded));
        assert!(results.iter().all(|result| result.score.is_none()));
    }

    #[test]
    fn duplicate_arm_order_and_duplicate_evidence_are_rejected() {
        let mut bad_order = CONTEXT_COMPILATION_ARMS;
        bad_order[4] = bad_order[3];
        assert_eq!(
            run_paired_context_ablation(
                &input(),
                &truth(),
                budget(),
                &bad_order,
                &mut ScriptedReasoner {
                calls: Vec::new(),
                next_session: 0,
            },
            ),
            Err(ContextExperimentError::InvalidArmOrder)
        );

        let mut bad_input = input();
        bad_input.evidence.push(bad_input.evidence[0].clone());
        assert_eq!(
            run_paired_context_ablation(
                &bad_input,
                &truth(),
                budget(),
                &CONTEXT_COMPILATION_ARMS,
                &mut ScriptedReasoner {
                calls: Vec::new(),
                next_session: 0,
            },
            ),
            Err(ContextExperimentError::DuplicateEvidenceId)
        );
    }
}
