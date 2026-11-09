use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const INPUT_CONTRACT: &str = "luminous.executable-assurance-case/0.1";
pub const REPORT_CONTRACT: &str = "luminous.executable-assurance-case-report/0.1";

const CLAIM_CEILING: &str = "Evaluates the declared structure, scope, evidence obligations, temporal validity, dependency admissibility, assumptions and defeaters in a synthetic assurance case. It does not prove physical truth, authenticate source signatures, establish organizational independence, or certify safety or regulatory conformity.";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Claim,
    Argument,
    Context,
    Assumption,
    Evidence,
    Defeater,
    Rebuttal,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    DecomposesInto,
    Supports,
    Contextualizes,
    Assumes,
    Defeats,
    Rebuts,
    DependsOn,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyRequirement {
    None,
    NoDeclaredCommonDomain,
    PhysicalOrStatisticalIndependence,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Measured,
    Reported,
    Derived,
    Inferred,
    Predicted,
    Simulated,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Current,
    Missing,
    Stale,
    Revoked,
    Conflicted,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssumptionStatus {
    Supported,
    Bounded,
    Open,
    Refuted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyDisposition {
    SharedDependency,
    UnknownDependency,
    NoDeclaredCommonDomain,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimSpec {
    pub decision_scope: String,
    pub required_evidence_ids: Vec<String>,
    pub required_subclaim_ids: Vec<String>,
    pub required_argument_ids: Vec<String>,
    pub dependency_requirement: DependencyRequirement,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArgumentSpec {
    pub rationale: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextSpec {
    pub description: String,
    pub decision_scope: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssumptionSpec {
    pub status: AssumptionStatus,
    pub critical: bool,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyAssessment {
    pub domain: String,
    pub disposition: DependencyDisposition,
    pub source_ids: Vec<String>,
    pub snapshot_id: String,
    pub inventory_complete: bool,
    pub evidence_frontier: i64,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSpec {
    pub artifact_sha256: Option<String>,
    pub evidence_class: EvidenceClass,
    pub source_id: Option<String>,
    pub provenance_ref: Option<String>,
    pub decision_scope: String,
    pub status: EvidenceStatus,
    pub observed_at: i64,
    pub valid_from: Option<i64>,
    pub valid_until: Option<i64>,
    pub dependency_assessment: Option<DependencyAssessment>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DefeaterSpec {
    pub applicable: bool,
    pub critical: bool,
    pub description: String,
    pub raised_at: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RebuttalSpec {
    pub target_defeater_id: String,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaseNode {
    pub id: String,
    pub kind: NodeKind,
    pub claim: Option<ClaimSpec>,
    pub argument: Option<ArgumentSpec>,
    pub context: Option<ContextSpec>,
    pub assumption: Option<AssumptionSpec>,
    pub evidence: Option<EvidenceSpec>,
    pub defeater: Option<DefeaterSpec>,
    pub rebuttal: Option<RebuttalSpec>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaseEdge {
    pub id: String,
    pub kind: EdgeKind,
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssuranceCaseInput {
    pub contract: String,
    pub case_id: String,
    pub decision_scope: String,
    pub as_of: i64,
    pub root_claim_id: String,
    pub nodes: Vec<CaseNode>,
    pub edges: Vec<CaseEdge>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClaimOutcome {
    Supported,
    Defeated,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CaseOutcome {
    SupportedForDeclaredScope,
    Defeated,
    Indeterminate,
    InvalidArgumentGraph,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimAssessment {
    pub claim_id: String,
    pub outcome: ClaimOutcome,
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssuranceCaseReport {
    pub report_contract: String,
    pub case_id: String,
    pub outcome: CaseOutcome,
    pub input_sha256: String,
    pub root_claim_id: String,
    pub decision_scope: String,
    pub claim_assessments: Vec<ClaimAssessment>,
    pub open_defeater_ids: Vec<String>,
    pub unresolved_assumption_ids: Vec<String>,
    pub invalid_reasons: Vec<String>,
    pub claim_ceiling: String,
}

impl CaseNode {
    fn kind_data_count(&self) -> usize {
        [
            self.claim.is_some(),
            self.argument.is_some(),
            self.context.is_some(),
            self.assumption.is_some(),
            self.evidence.is_some(),
            self.defeater.is_some(),
            self.rebuttal.is_some(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count()
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub fn evaluate_bytes(bytes: &[u8]) -> AssuranceCaseReport {
    let input_sha256 = sha256_hex(bytes);
    match serde_json::from_slice::<AssuranceCaseInput>(bytes) {
        Ok(input) => evaluate(&input, input_sha256),
        Err(_) => invalid_report(
            input_sha256,
            String::new(),
            String::new(),
            "case_input_parse_failed",
        ),
    }
}

pub fn evaluate(input: &AssuranceCaseInput, input_sha256: String) -> AssuranceCaseReport {
    let invalid = validate_structure(input);
    if !invalid.is_empty() {
        let mut report = invalid_report(
            input_sha256,
            input.case_id.clone(),
            input.root_claim_id.clone(),
            "invalid_argument_graph",
        );
        report.decision_scope = input.decision_scope.clone();
        report.invalid_reasons = invalid;
        return report;
    }

    let by_id: BTreeMap<&str, &CaseNode> =
        input.nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    let mut claim_assessments = BTreeMap::new();
    let mut open_defeaters = BTreeSet::new();
    let mut unresolved_assumptions = BTreeSet::new();
    let root = evaluate_claim(
        &input.root_claim_id,
        input,
        &by_id,
        &mut claim_assessments,
        &mut open_defeaters,
        &mut unresolved_assumptions,
        &mut BTreeSet::new(),
    );

    let outcome = match root {
        ClaimOutcome::Supported => CaseOutcome::SupportedForDeclaredScope,
        ClaimOutcome::Defeated => CaseOutcome::Defeated,
        ClaimOutcome::Indeterminate => CaseOutcome::Indeterminate,
    };
    AssuranceCaseReport {
        report_contract: REPORT_CONTRACT.to_owned(),
        case_id: input.case_id.clone(),
        outcome,
        input_sha256,
        root_claim_id: input.root_claim_id.clone(),
        decision_scope: input.decision_scope.clone(),
        claim_assessments: claim_assessments.into_values().collect(),
        open_defeater_ids: open_defeaters.into_iter().collect(),
        unresolved_assumption_ids: unresolved_assumptions.into_iter().collect(),
        invalid_reasons: Vec::new(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
    }
}

fn validate_structure(input: &AssuranceCaseInput) -> Vec<String> {
    let mut errors = Vec::new();
    if input.contract != INPUT_CONTRACT {
        errors.push("unsupported_case_contract".to_owned());
    }
    if input.case_id.trim().is_empty() || input.decision_scope.trim().is_empty() {
        errors.push("case_identity_or_scope_empty".to_owned());
    }
    if input.nodes.is_empty() {
        errors.push("case_has_no_nodes".to_owned());
    }

    let mut nodes = BTreeMap::new();
    for node in &input.nodes {
        if node.id.trim().is_empty() || nodes.insert(node.id.as_str(), node).is_some() {
            errors.push("empty_or_duplicate_node_id".to_owned());
        }
        if node.kind_data_count() != 1 || !kind_matches_data(node) {
            errors.push(format!("node_{}_kind_data_mismatch", node.id));
        }
    }
    let Some(root) = nodes.get(input.root_claim_id.as_str()) else {
        errors.push("root_claim_missing".to_owned());
        return sort_unique(errors);
    };
    if root.kind != NodeKind::Claim {
        errors.push("root_node_is_not_claim".to_owned());
    } else if root.claim.as_ref().is_some_and(|spec| {
        spec.decision_scope != input.decision_scope
    }) {
        errors.push("root_claim_scope_mismatch".to_owned());
    }

    let mut edge_ids = BTreeSet::new();
    for edge in &input.edges {
        if edge.id.trim().is_empty() || !edge_ids.insert(edge.id.as_str()) {
            errors.push("empty_or_duplicate_edge_id".to_owned());
            continue;
        }
        let (Some(from), Some(to)) =
            (nodes.get(edge.from.as_str()), nodes.get(edge.to.as_str()))
        else {
            errors.push(format!("edge_{}_dangling_endpoint", edge.id));
            continue;
        };
        if !edge_types_valid(edge.kind, from.kind, to.kind) {
            errors.push(format!("edge_{}_endpoint_type_invalid", edge.id));
        }
    }

    for node in &input.nodes {
        match node.kind {
            NodeKind::Claim => {
                if let Some(spec) = &node.claim {
                    if spec.decision_scope.trim().is_empty() {
                        errors.push(format!("claim_{}_scope_empty", node.id));
                    }
                    if !unique_ids(&spec.required_evidence_ids)
                        || !unique_ids(&spec.required_subclaim_ids)
                        || !unique_ids(&spec.required_argument_ids)
                    {
                        errors.push(format!("claim_{}_required_ids_invalid", node.id));
                    }
                    if spec.required_evidence_ids.is_empty()
                        && spec.required_subclaim_ids.is_empty()
                        && spec.required_argument_ids.is_empty()
                    {
                        errors.push(format!("claim_{}_has_no_evidence_or_subclaim_obligation", node.id));
                    }
                    for evidence_id in &spec.required_evidence_ids {
                        if !has_node_kind(&nodes, evidence_id, NodeKind::Evidence)
                            || !has_edge(&input.edges, EdgeKind::Supports, evidence_id, &node.id)
                        {
                            errors.push(format!("claim_{}_required_evidence_unbound", node.id));
                        }
                    }
                    for subclaim_id in &spec.required_subclaim_ids {
                        if !has_node_kind(&nodes, subclaim_id, NodeKind::Claim)
                            || !has_edge(
                                &input.edges,
                                EdgeKind::DecomposesInto,
                                &node.id,
                                subclaim_id,
                            )
                        {
                            errors.push(format!("claim_{}_required_subclaim_unbound", node.id));
                        }
                    }
                    for argument_id in &spec.required_argument_ids {
                        if !has_node_kind(&nodes, argument_id, NodeKind::Argument)
                            || !has_edge(
                                &input.edges,
                                EdgeKind::Supports,
                                argument_id,
                                &node.id,
                            )
                        {
                            errors.push(format!("claim_{}_required_argument_unbound", node.id));
                        }
                    }
                    if !input.edges.iter().any(|edge| {
                        edge.kind == EdgeKind::Contextualizes
                            && edge.to == node.id
                            && nodes.get(edge.from.as_str()).is_some_and(|n| n.kind == NodeKind::Context)
                    }) {
                        errors.push(format!("claim_{}_context_missing", node.id));
                    }
                }
            }
            NodeKind::Argument => {
                if node.argument.as_ref().is_none_or(|spec| spec.rationale.trim().is_empty()) {
                    errors.push(format!("argument_{}_rationale_empty", node.id));
                }
                let has_claim_support = input.edges.iter().any(|edge| {
                    edge.kind == EdgeKind::Supports
                        && edge.from == node.id
                        && nodes.get(edge.to.as_str()).is_some_and(|n| n.kind == NodeKind::Claim)
                });
                let has_evidence_support = input.edges.iter().any(|edge| {
                    edge.kind == EdgeKind::Supports
                        && edge.to == node.id
                        && nodes.get(edge.from.as_str()).is_some_and(|n| n.kind == NodeKind::Evidence)
                });
                if !has_claim_support || !has_evidence_support {
                    errors.push(format!("argument_{}_support_chain_incomplete", node.id));
                }
            }
            NodeKind::Context => {
                if node.context.as_ref().is_none_or(|spec| {
                    spec.description.trim().is_empty()
                        || spec.decision_scope != input.decision_scope
                }) {
                    errors.push(format!("context_{}_scope_or_description_invalid", node.id));
                }
                if !input.edges.iter().any(|edge| {
                    edge.kind == EdgeKind::Contextualizes && edge.from == node.id
                }) {
                    errors.push(format!("context_{}_not_attached", node.id));
                }
            }
            NodeKind::Assumption => {
                let assumption = node.assumption.as_ref().expect("kind data validated");
                if !input.edges.iter().any(|edge| {
                    edge.kind == EdgeKind::Assumes && edge.from == node.id
                }) {
                    errors.push(format!("assumption_{}_not_attached", node.id));
                }
                if matches!(
                    assumption.status,
                    AssumptionStatus::Supported | AssumptionStatus::Bounded | AssumptionStatus::Refuted
                ) && assumption.evidence_ids.is_empty()
                {
                    errors.push(format!("assumption_{}_evidence_required", node.id));
                }
                if !unique_ids(&assumption.evidence_ids) {
                    errors.push(format!("assumption_{}_evidence_ids_invalid", node.id));
                }
                for evidence_id in &assumption.evidence_ids {
                    if !has_node_kind(&nodes, evidence_id, NodeKind::Evidence)
                        || !has_edge(&input.edges, EdgeKind::Supports, evidence_id, &node.id)
                    {
                        errors.push(format!("assumption_{}_evidence_unbound", node.id));
                    }
                }
            }
            NodeKind::Evidence => {
                if !input.edges.iter().any(|edge| {
                    edge.kind == EdgeKind::Supports && edge.from == node.id
                }) {
                    errors.push(format!("evidence_{}_disconnected", node.id));
                }
            }
            NodeKind::Defeater => {
                let defeater = node.defeater.as_ref().expect("kind data validated");
                if defeater.description.trim().is_empty() {
                    errors.push(format!("defeater_{}_description_empty", node.id));
                }
                if defeater.raised_at > input.as_of {
                    errors.push(format!("defeater_{}_raised_after_case_as_of", node.id));
                }
                if !input.edges.iter().any(|edge| {
                    edge.kind == EdgeKind::Defeats && edge.from == node.id
                }) {
                    errors.push(format!("defeater_{}_not_attached", node.id));
                }
            }
            NodeKind::Rebuttal => {
                let rebuttal = node.rebuttal.as_ref().expect("kind data validated");
                let matching = input.edges.iter().any(|edge| {
                    edge.kind == EdgeKind::Rebuts
                        && edge.from == node.id
                        && edge.to == rebuttal.target_defeater_id
                });
                if !matching
                    || !has_node_kind(&nodes, &rebuttal.target_defeater_id, NodeKind::Defeater)
                    || rebuttal.evidence_ids.is_empty()
                    || !unique_ids(&rebuttal.evidence_ids)
                {
                    errors.push(format!("rebuttal_{}_target_binding_invalid", node.id));
                }
                for evidence_id in &rebuttal.evidence_ids {
                    if !has_node_kind(&nodes, evidence_id, NodeKind::Evidence)
                        || !has_edge(&input.edges, EdgeKind::Supports, evidence_id, &node.id)
                    {
                        errors.push(format!("rebuttal_{}_evidence_unbound", node.id));
                    }
                }
            }
        }
    }

    let reachable_claims = reachable_claim_ids(&input.root_claim_id, &input.edges);
    for node in &input.nodes {
        if node.kind == NodeKind::Claim && !reachable_claims.contains(node.id.as_str()) {
            errors.push(format!("claim_{}_disconnected_from_root", node.id));
        }
    }
    if has_cycle(input) {
        errors.push("argument_graph_cycle_detected".to_owned());
    }
    sort_unique(errors)
}

fn kind_matches_data(node: &CaseNode) -> bool {
    match node.kind {
        NodeKind::Claim => node.claim.is_some() && node.argument.is_none() && node.context.is_none()
            && node.assumption.is_none() && node.evidence.is_none() && node.defeater.is_none()
            && node.rebuttal.is_none(),
        NodeKind::Argument => node.argument.is_some() && node.claim.is_none() && node.context.is_none()
            && node.assumption.is_none() && node.evidence.is_none() && node.defeater.is_none()
            && node.rebuttal.is_none(),
        NodeKind::Context => node.context.is_some() && node.claim.is_none() && node.argument.is_none()
            && node.assumption.is_none() && node.evidence.is_none() && node.defeater.is_none()
            && node.rebuttal.is_none(),
        NodeKind::Assumption => node.assumption.is_some() && node.claim.is_none() && node.argument.is_none()
            && node.context.is_none() && node.evidence.is_none() && node.defeater.is_none()
            && node.rebuttal.is_none(),
        NodeKind::Evidence => node.evidence.is_some() && node.claim.is_none() && node.argument.is_none()
            && node.context.is_none() && node.assumption.is_none() && node.defeater.is_none()
            && node.rebuttal.is_none(),
        NodeKind::Defeater => node.defeater.is_some() && node.claim.is_none() && node.argument.is_none()
            && node.context.is_none() && node.assumption.is_none() && node.evidence.is_none()
            && node.rebuttal.is_none(),
        NodeKind::Rebuttal => node.rebuttal.is_some() && node.claim.is_none() && node.argument.is_none()
            && node.context.is_none() && node.assumption.is_none() && node.evidence.is_none()
            && node.defeater.is_none(),
    }
}

fn edge_types_valid(kind: EdgeKind, from: NodeKind, to: NodeKind) -> bool {
    match kind {
        EdgeKind::DecomposesInto => from == NodeKind::Claim && to == NodeKind::Claim,
        EdgeKind::Supports => matches!(from, NodeKind::Evidence | NodeKind::Claim | NodeKind::Argument)
            && matches!(to, NodeKind::Claim | NodeKind::Argument | NodeKind::Rebuttal | NodeKind::Assumption),
        EdgeKind::Contextualizes => from == NodeKind::Context
            && matches!(to, NodeKind::Claim | NodeKind::Argument | NodeKind::Evidence),
        EdgeKind::Assumes => from == NodeKind::Assumption
            && matches!(to, NodeKind::Claim | NodeKind::Argument),
        EdgeKind::Defeats => from == NodeKind::Defeater
            && matches!(to, NodeKind::Claim | NodeKind::Argument),
        EdgeKind::Rebuts => from == NodeKind::Rebuttal && to == NodeKind::Defeater,
        EdgeKind::DependsOn => matches!(from, NodeKind::Claim | NodeKind::Argument | NodeKind::Evidence)
            && matches!(to, NodeKind::Claim | NodeKind::Evidence),
    }
}

fn evaluate_claim(
    claim_id: &str,
    input: &AssuranceCaseInput,
    by_id: &BTreeMap<&str, &CaseNode>,
    assessments: &mut BTreeMap<String, ClaimAssessment>,
    open_defeaters: &mut BTreeSet<String>,
    unresolved_assumptions: &mut BTreeSet<String>,
    visiting: &mut BTreeSet<String>,
) -> ClaimOutcome {
    if let Some(existing) = assessments.get(claim_id) {
        return existing.outcome;
    }
    if !visiting.insert(claim_id.to_owned()) {
        return ClaimOutcome::Indeterminate;
    }

    let node = by_id.get(claim_id).expect("structure validated");
    let spec = node.claim.as_ref().expect("claim data validated");
    let mut outcomes = Vec::new();
    let mut reasons = Vec::new();

    for evidence_id in &spec.required_evidence_ids {
        let evidence_node = by_id.get(evidence_id.as_str()).expect("structure validated");
        let evidence = evidence_node.evidence.as_ref().expect("evidence data validated");
        let (outcome, mut evidence_reasons) = evaluate_evidence(
            evidence,
            &input.decision_scope,
            input.as_of,
            spec.dependency_requirement,
        );
        outcomes.push(outcome);
        reasons.append(&mut evidence_reasons);
    }

    for subclaim_id in &spec.required_subclaim_ids {
        outcomes.push(evaluate_claim(
            subclaim_id,
            input,
            by_id,
            assessments,
            open_defeaters,
            unresolved_assumptions,
            visiting,
        ));
        if let Some(child) = assessments.get(subclaim_id) {
            reasons.extend(child.reason_codes.iter().cloned());
        }
    }

    for argument_id in &spec.required_argument_ids {
        let (argument_outcome, mut argument_reasons) = evaluate_argument(
            argument_id,
            input,
            by_id,
            spec.dependency_requirement,
            open_defeaters,
            unresolved_assumptions,
        );
        outcomes.push(argument_outcome);
        reasons.append(&mut argument_reasons);
    }

    for edge in input.edges.iter().filter(|edge| {
        edge.kind == EdgeKind::Assumes && edge.to == claim_id
    }) {
        let assumption_node = by_id.get(edge.from.as_str()).expect("edge endpoint validated");
        let (outcome, mut why) = evaluate_assumption(
            assumption_node,
            input,
            by_id,
            unresolved_assumptions,
        );
        outcomes.push(outcome);
        reasons.append(&mut why);
    }

    for edge in input.edges.iter().filter(|edge| {
        edge.kind == EdgeKind::Defeats && edge.to == claim_id
    }) {
        let defeater_node = by_id.get(edge.from.as_str()).expect("edge endpoint validated");
        let (outcome, mut why) = evaluate_defeater(
            defeater_node,
            input,
            by_id,
            open_defeaters,
        );
        outcomes.push(outcome);
        reasons.append(&mut why);
    }

    visiting.remove(claim_id);
    let outcome = outcomes.into_iter().fold(ClaimOutcome::Supported, combine);
    let assessment = ClaimAssessment {
        claim_id: claim_id.to_owned(),
        outcome,
        reason_codes: sort_unique(reasons),
    };
    assessments.insert(claim_id.to_owned(), assessment);
    outcome
}

fn evaluate_argument(
    argument_id: &str,
    input: &AssuranceCaseInput,
    by_id: &BTreeMap<&str, &CaseNode>,
    dependency_requirement: DependencyRequirement,
    open_defeaters: &mut BTreeSet<String>,
    unresolved_assumptions: &mut BTreeSet<String>,
) -> (ClaimOutcome, Vec<String>) {
    let mut outcome = ClaimOutcome::Supported;
    let mut reasons = Vec::new();

    for edge in input.edges.iter().filter(|edge| {
        edge.kind == EdgeKind::Supports && edge.to == argument_id
    }) {
        let evidence_node = by_id.get(edge.from.as_str()).expect("edge endpoint validated");
        if evidence_node.kind != NodeKind::Evidence {
            continue;
        }
        let evidence = evidence_node.evidence.as_ref().expect("evidence data validated");
        let (evidence_outcome, mut why) = evaluate_evidence(
            evidence,
            &input.decision_scope,
            input.as_of,
            dependency_requirement,
        );
        outcome = combine(outcome, evidence_outcome);
        reasons.append(&mut why);
    }

    for edge in input.edges.iter().filter(|edge| {
        edge.kind == EdgeKind::Assumes && edge.to == argument_id
    }) {
        let assumption_node = by_id.get(edge.from.as_str()).expect("edge endpoint validated");
        let (assumption_outcome, mut why) = evaluate_assumption(
            assumption_node,
            input,
            by_id,
            unresolved_assumptions,
        );
        outcome = combine(outcome, assumption_outcome);
        reasons.append(&mut why);
    }

    for edge in input.edges.iter().filter(|edge| {
        edge.kind == EdgeKind::Defeats && edge.to == argument_id
    }) {
        let defeater_node = by_id.get(edge.from.as_str()).expect("edge endpoint validated");
        let (defeater_outcome, mut why) = evaluate_defeater(
            defeater_node,
            input,
            by_id,
            open_defeaters,
        );
        outcome = combine(outcome, defeater_outcome);
        reasons.append(&mut why);
    }

    (outcome, reasons)
}

fn evaluate_assumption(
    assumption_node: &CaseNode,
    input: &AssuranceCaseInput,
    by_id: &BTreeMap<&str, &CaseNode>,
    unresolved_assumptions: &mut BTreeSet<String>,
) -> (ClaimOutcome, Vec<String>) {
    let assumption = assumption_node.assumption.as_ref().expect("assumption data validated");
    let mut outcome = ClaimOutcome::Supported;
    let mut reasons = Vec::new();

    match assumption.status {
        AssumptionStatus::Open => {
            unresolved_assumptions.insert(assumption_node.id.clone());
            reasons.push(format!("assumption_{}_open", assumption_node.id));
            return (ClaimOutcome::Indeterminate, reasons);
        }
        AssumptionStatus::Refuted => {
            unresolved_assumptions.insert(assumption_node.id.clone());
            reasons.push(format!("assumption_{}_refuted", assumption_node.id));
            return (
                if assumption.critical {
                    ClaimOutcome::Defeated
                } else {
                    ClaimOutcome::Indeterminate
                },
                reasons,
            );
        }
        AssumptionStatus::Supported | AssumptionStatus::Bounded => {}
    }

    if assumption.evidence_ids.is_empty() {
        unresolved_assumptions.insert(assumption_node.id.clone());
        return (
            ClaimOutcome::Indeterminate,
            vec![format!("assumption_{}_has_no_support_evidence", assumption_node.id)],
        );
    }

    for evidence_id in &assumption.evidence_ids {
        // Structural validation establishes the type and edge before evaluation.
        let evidence_node = by_id.get(evidence_id.as_str()).expect("assumption evidence validated");
        let evidence = evidence_node.evidence.as_ref().expect("assumption evidence type validated");
        let (evidence_outcome, mut why) = evaluate_evidence(
            evidence,
            &input.decision_scope,
            input.as_of,
            DependencyRequirement::None,
        );
        outcome = combine(outcome, evidence_outcome);
        reasons.append(&mut why);
    }

    if outcome != ClaimOutcome::Supported {
        unresolved_assumptions.insert(assumption_node.id.clone());
        reasons.push(format!("assumption_{}_support_not_established", assumption_node.id));
    }
    (outcome, reasons)
}

fn evaluate_defeater(
    defeater_node: &CaseNode,
    input: &AssuranceCaseInput,
    by_id: &BTreeMap<&str, &CaseNode>,
    open_defeaters: &mut BTreeSet<String>,
) -> (ClaimOutcome, Vec<String>) {
    let defeater = defeater_node.defeater.as_ref().expect("defeater data validated");
    if !defeater.applicable || rebuttal_closes(&defeater_node.id, input, by_id) {
        return (ClaimOutcome::Supported, Vec::new());
    }

    open_defeaters.insert(defeater_node.id.clone());
    let outcome = if defeater.critical {
        ClaimOutcome::Defeated
    } else {
        ClaimOutcome::Indeterminate
    };
    (
        outcome,
        vec![format!("defeater_{}_open", defeater_node.id)],
    )
}

fn rebuttal_closes(
    defeater_id: &str,
    input: &AssuranceCaseInput,
    by_id: &BTreeMap<&str, &CaseNode>,
) -> bool {
    let Some(defeater_node) = by_id.get(defeater_id) else {
        return false;
    };
    let Some(defeater) = defeater_node.defeater.as_ref() else {
        return false;
    };

    for edge in input.edges.iter().filter(|edge| {
        edge.kind == EdgeKind::Rebuts && edge.to == defeater_id
    }) {
        let Some(rebuttal_node) = by_id.get(edge.from.as_str()) else {
            continue;
        };
        let Some(spec) = rebuttal_node.rebuttal.as_ref() else {
            continue;
        };
        if spec.target_defeater_id != defeater_id || spec.evidence_ids.is_empty() {
            continue;
        }

        let all_valid_and_fresh = spec.evidence_ids.iter().all(|evidence_id| {
            if !has_edge(&input.edges, EdgeKind::Supports, evidence_id, &rebuttal_node.id) {
                return false;
            }
            by_id
                .get(evidence_id.as_str())
                .and_then(|evidence_node| evidence_node.evidence.as_ref())
                .is_some_and(|evidence| {
                    evidence.observed_at >= defeater.raised_at
                        && evaluate_evidence(
                            evidence,
                            &input.decision_scope,
                            input.as_of,
                            DependencyRequirement::None,
                        )
                        .0 == ClaimOutcome::Supported
                })
        });
        if all_valid_and_fresh {
            return true;
        }
    }
    false
}

fn evaluate_evidence(
    evidence: &EvidenceSpec,
    decision_scope: &str,
    as_of: i64,
    requirement: DependencyRequirement,
) -> (ClaimOutcome, Vec<String>) {
    let mut reasons = Vec::new();
    if evidence.status != EvidenceStatus::Current {
        reasons.push(format!("evidence_status_{:?}_not_current", evidence.status).to_lowercase());
        return (ClaimOutcome::Indeterminate, reasons);
    }
    let Some(digest) = evidence.artifact_sha256.as_deref().filter(|value| valid_digest(value)) else {
        return (ClaimOutcome::Indeterminate, vec!["evidence_digest_missing_or_invalid".to_owned()]);
    };
    let _ = digest;
    if evidence.evidence_class == EvidenceClass::Unknown {
        reasons.push("evidence_class_unknown".to_owned());
    }
    if evidence.source_id.as_deref().is_none_or(|value| value.trim().is_empty())
        || evidence.provenance_ref.as_deref().is_none_or(|value| value.trim().is_empty())
    {
        reasons.push("evidence_source_or_provenance_missing".to_owned());
    }
    if evidence.decision_scope != decision_scope {
        reasons.push("evidence_outside_decision_scope".to_owned());
    }
    if evidence.observed_at > as_of
        || evidence.valid_from.is_some_and(|start| start > as_of)
        || evidence.valid_until.is_some_and(|end| as_of >= end)
    {
        reasons.push("evidence_not_temporally_valid_at_case_as_of".to_owned());
    }

    match requirement {
        DependencyRequirement::None => {}
        DependencyRequirement::NoDeclaredCommonDomain
        | DependencyRequirement::PhysicalOrStatisticalIndependence => {
            match &evidence.dependency_assessment {
                None => reasons.push("required_dependency_assessment_missing".to_owned()),
                Some(assessment) => match assessment.disposition {
                    DependencyDisposition::SharedDependency => {
                        return (ClaimOutcome::Defeated, vec!["required_independence_defeated_by_shared_dependency".to_owned()]);
                    }
                    DependencyDisposition::UnknownDependency => {
                        reasons.push("required_dependency_assessment_unknown".to_owned());
                    }
                    DependencyDisposition::NoDeclaredCommonDomain => {
                        let sources_sorted = assessment.source_ids.windows(2).all(|pair| pair[0] < pair[1]);
                        if !assessment.inventory_complete
                            || assessment.snapshot_id.trim().is_empty()
                            || assessment.evidence_frontier < as_of
                            || assessment.source_ids.is_empty()
                            || !sources_sorted
                        {
                            reasons.push("no_declared_common_domain_evidence_incomplete".to_owned());
                        } else if requirement == DependencyRequirement::PhysicalOrStatisticalIndependence {
                            reasons.push("no_declared_common_domain_does_not_prove_physical_or_statistical_independence".to_owned());
                        }
                    }
                },
            }
        }
    }

    if reasons.is_empty() {
        (ClaimOutcome::Supported, reasons)
    } else {
        (ClaimOutcome::Indeterminate, reasons)
    }
}

fn combine(left: ClaimOutcome, right: ClaimOutcome) -> ClaimOutcome {
    match (left, right) {
        (ClaimOutcome::Defeated, _) | (_, ClaimOutcome::Defeated) => ClaimOutcome::Defeated,
        (ClaimOutcome::Indeterminate, _) | (_, ClaimOutcome::Indeterminate) => ClaimOutcome::Indeterminate,
        _ => ClaimOutcome::Supported,
    }
}


fn reachable_claim_ids(root: &str, edges: &[CaseEdge]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![root.to_owned()];
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        for edge in edges.iter().filter(|edge| {
            edge.kind == EdgeKind::DecomposesInto && edge.from == id
        }) {
            stack.push(edge.to.clone());
        }
    }
    seen
}

fn has_cycle(input: &AssuranceCaseInput) -> bool {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in &input.edges {
        if matches!(edge.kind, EdgeKind::DecomposesInto | EdgeKind::DependsOn) {
            adjacency.entry(&edge.from).or_default().push(&edge.to);
        }
    }
    fn visit<'a>(
        node: &'a str,
        adjacency: &BTreeMap<&'a str, Vec<&'a str>>,
        visiting: &mut BTreeSet<&'a str>,
        visited: &mut BTreeSet<&'a str>,
    ) -> bool {
        if visited.contains(node) {
            return false;
        }
        if !visiting.insert(node) {
            return true;
        }
        if let Some(children) = adjacency.get(node) {
            for child in children {
                if visit(child, adjacency, visiting, visited) {
                    return true;
                }
            }
        }
        visiting.remove(node);
        visited.insert(node);
        false
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    adjacency
        .keys()
        .any(|node| visit(node, &adjacency, &mut visiting, &mut visited))
}

fn has_node_kind(nodes: &BTreeMap<&str, &CaseNode>, id: &str, kind: NodeKind) -> bool {
    nodes.get(id).is_some_and(|node| node.kind == kind)
}

fn has_edge(edges: &[CaseEdge], kind: EdgeKind, from: &str, to: &str) -> bool {
    edges
        .iter()
        .any(|edge| edge.kind == kind && edge.from == from && edge.to == to)
}

fn unique_ids(ids: &[String]) -> bool {
    let set: BTreeSet<&str> = ids.iter().map(String::as_str).collect();
    set.len() == ids.len() && ids.iter().all(|id| !id.trim().is_empty())
}

fn sort_unique(mut items: Vec<String>) -> Vec<String> {
    items.sort();
    items.dedup();
    items
}

fn invalid_report(
    input_sha256: String,
    case_id: String,
    root_claim_id: String,
    reason: &str,
) -> AssuranceCaseReport {
    AssuranceCaseReport {
        report_contract: REPORT_CONTRACT.to_owned(),
        case_id,
        outcome: CaseOutcome::InvalidArgumentGraph,
        input_sha256,
        root_claim_id,
        decision_scope: String::new(),
        claim_assessments: Vec::new(),
        open_defeater_ids: Vec::new(),
        unresolved_assumption_ids: Vec::new(),
        invalid_reasons: vec![reason.to_owned()],
        claim_ceiling: CLAIM_CEILING.to_owned(),
    }
}
