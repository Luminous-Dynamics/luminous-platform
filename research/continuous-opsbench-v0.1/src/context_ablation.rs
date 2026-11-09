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
    pub answer: DiagnosisAnswer,
    pub usage: ReasonerUsage,
}

pub trait ContextReasoner {
    /// The reasoner receives only the compiled packet and equal per-arm budgets.
    /// It receives no evaluator-owned truth, expected root, or required evidence IDs.
    /// The adapter must use a fresh conversation/session per call while keeping
    /// the packet's model, prompt, reasoning-effort and seed manifest unchanged.
    fn diagnose(&mut self, packet: &ContextPacket, budget: ContextBudget) -> ReasonerResponse;
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
    pub paired_block_id: String,
    pub attempt_id: u32,
    pub profile: ContextExperimentProfile,
    pub arm: ContextArm,
    pub status: ArmStatus,
    pub compiler_time_micros: u128,
    pub reasoner_time_micros: Option<u128>,
    pub context_bytes: usize,
    pub estimated_input_tokens: u64,
    pub usage: Option<ReasonerUsage>,
    pub answer: Option<DiagnosisAnswer>,
    pub score: Option<ContextScore>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextExperimentError {
    EmptyScenarioId,
    EmptyTargetObject,
    EmptyPairedBlockId,
    InvalidExperimentProfile,
    EmptyEvidence,
    DuplicateEvidenceId,
    InvalidEvidenceRecord,
    DuplicateRelationId,
    InvalidRelation,
    RelationEvidenceMissing,
    InvalidBudget,
    InvalidGroundTruth,
    GroundTruthEvidenceMissing,
    InvalidArmOrder,
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
                .then_some(input.audited_at_ms - item.observed_at_ms),
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
    let _ = writeln!(
        &mut rendered,
        "scenario={} scenario_revision={} block={} attempt={} arm={:?} target={} model={} prompt={} permissions={} tools={} scorer={} effort={} seed={}",
        escape_field(&packet.scenario_id),
        digest_hex(&packet.profile.scenario_revision_digest),
        escape_field(&packet.paired_block_id),
        packet.attempt_id,
        packet.arm,
        escape_field(&packet.target_object_ref),
        digest_hex(&packet.profile.model_snapshot_digest),
        digest_hex(&packet.profile.prompt_digest),
        digest_hex(&packet.profile.permission_profile_digest),
        digest_hex(&packet.profile.tool_catalog_digest),
        digest_hex(&packet.profile.scorer_digest),
        escape_field(&packet.profile.reasoning_effort),
        packet.profile.sampling_seed
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
                supported.insert(*evidence_id);
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
                .get(evidence_id)
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
    for arm in arm_order {
        let (packet, compiler_time_micros) = compile_context_packet(input, *arm)?;
        if packet.context_bytes > budget.maximum_context_bytes
            || packet.estimated_input_tokens > budget.maximum_input_tokens
        {
            results.push(ContextArmResult {
                scenario_id: packet.scenario_id.clone(),
                paired_block_id: packet.paired_block_id.clone(),
                attempt_id: packet.attempt_id,
                profile: packet.profile.clone(),
                arm: *arm,
                status: ArmStatus::BudgetExceeded,
                compiler_time_micros,
                reasoner_time_micros: None,
                context_bytes: packet.context_bytes,
                estimated_input_tokens: packet.estimated_input_tokens,
                usage: None,
                answer: None,
                score: None,
            });
            continue;
        }

        let started = Instant::now();
        let response = reasoner.diagnose(&packet, budget);
        let reasoner_time_micros = started.elapsed().as_micros();
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
            paired_block_id: packet.paired_block_id.clone(),
            attempt_id: packet.attempt_id,
            profile: packet.profile.clone(),
            arm: *arm,
            status,
            compiler_time_micros,
            reasoner_time_micros: Some(reasoner_time_micros),
            context_bytes: packet.context_bytes,
            estimated_input_tokens: packet.estimated_input_tokens,
            usage: Some(response.usage),
            answer: Some(response.answer),
            score,
        });
    }
    Ok(results)
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
        calls: Vec<ContextArm>,
    }

    impl ContextReasoner for ScriptedReasoner {
        fn diagnose(&mut self, packet: &ContextPacket, _budget: ContextBudget) -> ReasonerResponse {
            self.calls.push(packet.arm);
            let selected = packet
                .ranked_candidates
                .first()
                .map(|candidate| candidate.object_ref.clone())
                .or_else(|| match &packet.evidence {
                    ContextPresentation::Raw(records) => {
                        records.first().map(|item| item.object_ref.clone())
                    }
                    ContextPresentation::Normalized(records) => {
                        records.first().map(|item| item.object_ref.clone())
                    }
                });
            let supporting_evidence_ids = match &packet.evidence {
                ContextPresentation::Raw(records) => records
                    .iter()
                    .filter(|item| selected.as_deref() == Some(item.object_ref.as_str()))
                    .map(|item| item.evidence_id.clone())
                    .collect(),
                ContextPresentation::Normalized(records) => records
                    .iter()
                    .filter(|item| selected.as_deref() == Some(item.object_ref.as_str()))
                    .map(|item| item.evidence_id.clone())
                    .collect(),
            };
            ReasonerResponse {
                answer: DiagnosisAnswer {
                    root_object_ref: selected,
                    supporting_evidence_ids,
                    alternative_object_refs: packet
                        .ranked_candidates
                        .iter()
                        .skip(1)
                        .take(2)
                        .map(|candidate| candidate.object_ref.clone())
                        .collect(),
                    abstained: false,
                },
                usage: ReasonerUsage {
                    input_tokens: packet.estimated_input_tokens,
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
        let mut reasoner = ScriptedReasoner { calls: Vec::new() };
        let results = run_paired_context_ablation(
            &input,
            &truth(),
            budget(),
            &CONTEXT_COMPILATION_ARMS,
            &mut reasoner,
        )
        .unwrap();
        assert_eq!(reasoner.calls, CONTEXT_COMPILATION_ARMS);
        assert_eq!(results.len(), 5);
        assert!(results
            .iter()
            .all(|result| result.status == ArmStatus::Scored));
        assert!(results.iter().all(|result| result.profile == input.profile));
        assert!(results
            .iter()
            .all(|result| result.paired_block_id == "fault-kafka-01-seed-7"));
        assert!(results.iter().all(|result| result.attempt_id == 1));
        assert!(results[0].score.as_ref().unwrap().required_evidence_total == 1);
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
                &mut ScriptedReasoner { calls: Vec::new() },
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
            &mut ScriptedReasoner { calls: Vec::new() },
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
                &mut ScriptedReasoner { calls: Vec::new() },
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
                &mut ScriptedReasoner { calls: Vec::new() },
            ),
            Err(ContextExperimentError::DuplicateEvidenceId)
        );
    }
}
