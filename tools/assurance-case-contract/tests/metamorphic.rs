use assurance_case_contract::{
    AssuranceCaseInput, AssumptionSpec, AssumptionStatus, CaseEdge, CaseNode,
    ClaimSpec, ContextSpec, DefeaterSpec, DependencyAssessment, DependencyDisposition,
    DependencyRequirement, EdgeKind, EvidenceClass, EvidenceSpec, EvidenceStatus, NodeKind,
    RebuttalSpec, CaseOutcome, evaluate_bytes,
};
use jsonschema::draft202012;
use serde_json::json;

const INPUT_SCHEMA: &str =
    include_str!("../../../docs/assurance/assurance-case/assurance-case-input-0.1.schema.json");
const REPORT_SCHEMA: &str = include_str!("../schema/assurance-case-report-0.1.schema.json");

fn node(
    id: &str,
    kind: NodeKind,
    claim: Option<ClaimSpec>,
    argument: Option<&str>,
    context: Option<ContextSpec>,
    assumption: Option<AssumptionSpec>,
    evidence: Option<EvidenceSpec>,
    defeater: Option<DefeaterSpec>,
    rebuttal: Option<RebuttalSpec>,
) -> CaseNode {
    CaseNode {
        id: id.to_owned(),
        kind,
        claim,
        argument: argument.map(|rationale| {
            assurance_case_contract::ArgumentSpec {
                rationale: rationale.to_owned(),
            }
        }),
        context,
        assumption,
        evidence,
        defeater,
        rebuttal,
    }
}

fn evidence(status: EvidenceStatus, observed_at: i64) -> EvidenceSpec {
    EvidenceSpec {
        artifact_sha256: Some("a".repeat(64)),
        evidence_class: EvidenceClass::Measured,
        source_id: Some("external-reference-01".to_owned()),
        provenance_ref: Some("receipt:instrument-01".to_owned()),
        decision_scope: "industrial-lab".to_owned(),
        status,
        observed_at,
        valid_from: Some(0),
        valid_until: Some(100),
        dependency_assessment: None,
    }
}

fn no_common_dependency() -> DependencyAssessment {
    DependencyAssessment {
        domain: "clock".to_owned(),
        disposition: DependencyDisposition::NoDeclaredCommonDomain,
        source_ids: vec!["pm-ref-01".to_owned(), "ptp-clock-01".to_owned()],
        snapshot_id: "snapshot-001".to_owned(),
        inventory_complete: true,
        evidence_frontier: 100,
        claim_ceiling: "No declared common dependency in this complete inventory only".to_owned(),
    }
}

fn base_case() -> AssuranceCaseInput {
    let claim = ClaimSpec {
        decision_scope: "industrial-lab".to_owned(),
        required_evidence_ids: vec!["e-direct".to_owned()],
        required_subclaim_ids: Vec::new(),
        required_argument_ids: vec!["arg-main".to_owned()],
        dependency_requirement: DependencyRequirement::None,
    };

    let nodes = vec![
        node(
            "claim-root",
            NodeKind::Claim,
            Some(claim),
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        node(
            "arg-main",
            NodeKind::Argument,
            None,
            Some("The measurements and their provenance support the declared claim."),
            None,
            None,
            None,
            None,
            None,
        ),
        node(
            "ctx-lab",
            NodeKind::Context,
            None,
            None,
            Some(ContextSpec {
                description: "Controlled industrial lab test environment".to_owned(),
                decision_scope: "industrial-lab".to_owned(),
            }),
            None,
            None,
            None,
            None,
        ),
        node(
            "e-direct",
            NodeKind::Evidence,
            None,
            None,
            None,
            None,
            Some(evidence(EvidenceStatus::Current, 30)),
            None,
            None,
        ),
        node(
            "e-argument",
            NodeKind::Evidence,
            None,
            None,
            None,
            None,
            Some(evidence(EvidenceStatus::Current, 30)),
            None,
            None,
        ),
    ];

    let edges = vec![
        CaseEdge {
            id: "edge-direct".to_owned(),
            kind: EdgeKind::Supports,
            from: "e-direct".to_owned(),
            to: "claim-root".to_owned(),
        },
        CaseEdge {
            id: "edge-arg-evidence".to_owned(),
            kind: EdgeKind::Supports,
            from: "e-argument".to_owned(),
            to: "arg-main".to_owned(),
        },
        CaseEdge {
            id: "edge-arg-claim".to_owned(),
            kind: EdgeKind::Supports,
            from: "arg-main".to_owned(),
            to: "claim-root".to_owned(),
        },
        CaseEdge {
            id: "edge-context".to_owned(),
            kind: EdgeKind::Contextualizes,
            from: "ctx-lab".to_owned(),
            to: "claim-root".to_owned(),
        },
    ];

    AssuranceCaseInput {
        contract: assurance_case_contract::INPUT_CONTRACT.to_owned(),
        case_id: "AC-BASE-001".to_owned(),
        decision_scope: "industrial-lab".to_owned(),
        as_of: 50,
        root_claim_id: "claim-root".to_owned(),
        nodes,
        edges,
    }
}

fn run(input: &AssuranceCaseInput) -> assurance_case_contract::AssuranceCaseReport {
    let bytes = serde_json::to_vec(input).expect("serialize assurance case");
    evaluate_bytes(&bytes)
}

fn add_assumption(input: &mut AssuranceCaseInput, status: AssumptionStatus, critical: bool) {
    let evidence_ids = if matches!(status, AssumptionStatus::Supported | AssumptionStatus::Bounded | AssumptionStatus::Refuted) {
        vec!["e-assumption".to_owned()]
    } else {
        Vec::new()
    };
    input.nodes.push(node(
        "assumption-a",
        NodeKind::Assumption,
        None,
        None,
        None,
        Some(AssumptionSpec {
            status,
            critical,
            evidence_ids,
        }),
        None,
        None,
        None,
    ));
    if !matches!(status, AssumptionStatus::Open) {
        input.nodes.push(node(
            "e-assumption",
            NodeKind::Evidence,
            None,
            None,
            None,
            None,
            Some(evidence(EvidenceStatus::Current, 35)),
            None,
            None,
        ));
        input.edges.push(CaseEdge {
            id: "edge-assumption-evidence".to_owned(),
            kind: EdgeKind::Supports,
            from: "e-assumption".to_owned(),
            to: "assumption-a".to_owned(),
        });
    }
    input.edges.push(CaseEdge {
        id: "edge-assumption".to_owned(),
        kind: EdgeKind::Assumes,
        from: "assumption-a".to_owned(),
        to: "claim-root".to_owned(),
    });
}

fn add_defeater(input: &mut AssuranceCaseInput, raised_at: i64) {
    input.nodes.push(node(
        "defeater-a",
        NodeKind::Defeater,
        None,
        None,
        None,
        None,
        None,
        Some(DefeaterSpec {
            applicable: true,
            critical: true,
            description: "The reference source may share a clock failure domain.".to_owned(),
            raised_at,
        }),
        None,
    ));
    input.edges.push(CaseEdge {
        id: "edge-defeater".to_owned(),
        kind: EdgeKind::Defeats,
        from: "defeater-a".to_owned(),
        to: "claim-root".to_owned(),
    });
}

fn add_rebuttal(input: &mut AssuranceCaseInput, observed_at: i64, target: &str) {
    input.nodes.push(node(
        "e-rebuttal",
        NodeKind::Evidence,
        None,
        None,
        None,
        None,
        Some(evidence(EvidenceStatus::Current, observed_at)),
        None,
        None,
    ));
    input.nodes.push(node(
        "rebuttal-a",
        NodeKind::Rebuttal,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(RebuttalSpec {
            target_defeater_id: target.to_owned(),
            evidence_ids: vec!["e-rebuttal".to_owned()],
        }),
    ));
    input.edges.push(CaseEdge {
        id: "edge-rebuttal-evidence".to_owned(),
        kind: EdgeKind::Supports,
        from: "e-rebuttal".to_owned(),
        to: "rebuttal-a".to_owned(),
    });
    input.edges.push(CaseEdge {
        id: "edge-rebuttal-target".to_owned(),
        kind: EdgeKind::Rebuts,
        from: "rebuttal-a".to_owned(),
        to: target.to_owned(),
    });
}

#[test]
fn complete_claim_with_direct_and_argument_support_is_supported() {
    let report = run(&base_case());
    assert_eq!(report.outcome, CaseOutcome::SupportedForDeclaredScope);
    assert_eq!(report.input_sha256.len(), 64);
    assert!(report.invalid_reasons.is_empty());
}

#[test]
fn missing_required_evidence_cannot_support_a_claim() {
    let mut input = base_case();
    let direct = input.nodes.iter_mut().find(|node| node.id == "e-direct").unwrap();
    direct.evidence.as_mut().unwrap().status = EvidenceStatus::Missing;
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::Indeterminate);
}

#[test]
fn open_assumption_is_indeterminate() {
    let mut input = base_case();
    add_assumption(&mut input, AssumptionStatus::Open, true);
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::Indeterminate);
    assert_eq!(report.unresolved_assumption_ids, vec!["assumption-a"]);
}

#[test]
fn refuted_critical_assumption_defeats_the_claim() {
    let mut input = base_case();
    add_assumption(&mut input, AssumptionStatus::Refuted, true);
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::Defeated);
}

#[test]
fn bounded_assumption_with_current_evidence_is_supported() {
    let mut input = base_case();
    add_assumption(&mut input, AssumptionStatus::Bounded, false);
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::SupportedForDeclaredScope);
}

#[test]
fn critical_defeater_attached_to_claim_defeats_it() {
    let mut input = base_case();
    add_defeater(&mut input, 20);
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::Defeated);
    assert_eq!(report.open_defeater_ids, vec!["defeater-a"]);
}

#[test]
fn evidence_backed_fresh_rebuttal_closes_only_the_targeted_defeater() {
    let mut input = base_case();
    add_defeater(&mut input, 20);
    add_rebuttal(&mut input, 21, "defeater-a");
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::SupportedForDeclaredScope);
    assert!(report.open_defeater_ids.is_empty());
}

#[test]
fn rebuttal_evidence_older_than_defeater_does_not_close_it() {
    let mut input = base_case();
    add_defeater(&mut input, 40);
    add_rebuttal(&mut input, 30, "defeater-a");
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::Defeated);
    assert_eq!(report.open_defeater_ids, vec!["defeater-a"]);
}

#[test]
fn wrong_rebuttal_target_is_an_invalid_graph() {
    let mut input = base_case();
    add_defeater(&mut input, 20);
    add_rebuttal(&mut input, 30, "missing-defeater");
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::InvalidArgumentGraph);
}

#[test]
fn missing_assumption_evidence_reference_is_invalid_not_a_panic() {
    let mut input = base_case();
    input.nodes.push(node(
        "assumption-a",
        NodeKind::Assumption,
        None,
        None,
        None,
        Some(AssumptionSpec {
            status: AssumptionStatus::Supported,
            critical: true,
            evidence_ids: vec!["does-not-exist".to_owned()],
        }),
        None,
        None,
        None,
    ));
    input.edges.push(CaseEdge {
        id: "edge-assumption".to_owned(),
        kind: EdgeKind::Assumes,
        from: "assumption-a".to_owned(),
        to: "claim-root".to_owned(),
    });
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::InvalidArgumentGraph);
}

#[test]
fn shared_dependency_defeats_independent_corroboration_claim() {
    let mut input = base_case();
    input.nodes[0].claim.as_mut().unwrap().dependency_requirement =
        DependencyRequirement::NoDeclaredCommonDomain;
    for id in ["e-direct", "e-argument"] {
        let evidence = input.nodes.iter_mut().find(|node| node.id == id).unwrap();
        evidence.evidence.as_mut().unwrap().dependency_assessment = Some(DependencyAssessment {
            disposition: DependencyDisposition::SharedDependency,
            ..no_common_dependency()
        });
    }
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::Defeated);
}

#[test]
fn unknown_dependency_cannot_discharge_independence_obligation() {
    let mut input = base_case();
    input.nodes[0].claim.as_mut().unwrap().dependency_requirement =
        DependencyRequirement::NoDeclaredCommonDomain;
    for id in ["e-direct", "e-argument"] {
        let evidence = input.nodes.iter_mut().find(|node| node.id == id).unwrap();
        evidence.evidence.as_mut().unwrap().dependency_assessment = Some(DependencyAssessment {
            disposition: DependencyDisposition::UnknownDependency,
            ..no_common_dependency()
        });
    }
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::Indeterminate);
}

#[test]
fn complete_no_declared_common_domain_supports_only_the_scoped_claim() {
    let mut input = base_case();
    input.nodes[0].claim.as_mut().unwrap().dependency_requirement =
        DependencyRequirement::NoDeclaredCommonDomain;
    for id in ["e-direct", "e-argument"] {
        let evidence = input.nodes.iter_mut().find(|node| node.id == id).unwrap();
        evidence.evidence.as_mut().unwrap().dependency_assessment = Some(no_common_dependency());
    }
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::SupportedForDeclaredScope);
}

#[test]
fn no_declared_common_domain_never_proves_physical_independence() {
    let mut input = base_case();
    input.nodes[0].claim.as_mut().unwrap().dependency_requirement =
        DependencyRequirement::PhysicalOrStatisticalIndependence;
    for id in ["e-direct", "e-argument"] {
        let evidence = input.nodes.iter_mut().find(|node| node.id == id).unwrap();
        evidence.evidence.as_mut().unwrap().dependency_assessment = Some(no_common_dependency());
    }
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::Indeterminate);
}

#[test]
fn incomplete_inventory_cannot_satisfy_no_common_dependency_obligation() {
    let mut input = base_case();
    input.nodes[0].claim.as_mut().unwrap().dependency_requirement =
        DependencyRequirement::NoDeclaredCommonDomain;
    for id in ["e-direct", "e-argument"] {
        let evidence = input.nodes.iter_mut().find(|node| node.id == id).unwrap();
        evidence.evidence.as_mut().unwrap().dependency_assessment = Some(DependencyAssessment {
            inventory_complete: false,
            ..no_common_dependency()
        });
    }
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::Indeterminate);
}

#[test]
fn expired_evidence_is_indeterminate() {
    let mut input = base_case();
    let direct = input.nodes.iter_mut().find(|node| node.id == "e-direct").unwrap();
    direct.evidence.as_mut().unwrap().valid_until = Some(50);
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::Indeterminate);
}

#[test]
fn evidence_outside_decision_scope_is_indeterminate() {
    let mut input = base_case();
    let direct = input.nodes.iter_mut().find(|node| node.id == "e-direct").unwrap();
    direct.evidence.as_mut().unwrap().decision_scope = "different-system".to_owned();
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::Indeterminate);
}

#[test]
fn dangling_edges_produce_invalid_argument_graph() {
    let mut input = base_case();
    input.edges.push(CaseEdge {
        id: "edge-dangling".to_owned(),
        kind: EdgeKind::Supports,
        from: "unknown-evidence".to_owned(),
        to: "claim-root".to_owned(),
    });
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::InvalidArgumentGraph);
}

#[test]
fn wrong_edge_endpoint_type_is_invalid() {
    let mut input = base_case();
    input.edges.push(CaseEdge {
        id: "edge-wrong-type".to_owned(),
        kind: EdgeKind::Defeats,
        from: "e-direct".to_owned(),
        to: "claim-root".to_owned(),
    });
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::InvalidArgumentGraph);
}

#[test]
fn unsupported_argument_dependency_cycle_is_invalid() {
    let mut input = base_case();
    input.edges.push(CaseEdge {
        id: "cycle-a".to_owned(),
        kind: EdgeKind::DependsOn,
        from: "claim-root".to_owned(),
        to: "e-direct".to_owned(),
    });
    input.edges.push(CaseEdge {
        id: "cycle-b".to_owned(),
        kind: EdgeKind::DependsOn,
        from: "e-direct".to_owned(),
        to: "claim-root".to_owned(),
    });
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::InvalidArgumentGraph);
}

#[test]
fn all_required_argument_support_can_satisfy_a_claim_without_direct_evidence() {
    let mut input = base_case();
    input.nodes[0].claim.as_mut().unwrap().required_evidence_ids.clear();
    input.nodes.retain(|node| node.id != "e-direct");
    input.edges.retain(|edge| edge.id != "edge-direct");
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::SupportedForDeclaredScope);
}

#[test]
fn defeater_attached_to_argument_is_propagated_to_root_claim() {
    let mut input = base_case();
    input.nodes.push(node(
        "argument-defeater",
        NodeKind::Defeater,
        None,
        None,
        None,
        None,
        None,
        Some(DefeaterSpec {
            applicable: true,
            critical: true,
            description: "The argument's measurement evidence is compromised.".to_owned(),
            raised_at: 20,
        }),
        None,
    ));
    input.edges.push(CaseEdge {
        id: "edge-argument-defeater".to_owned(),
        kind: EdgeKind::Defeats,
        from: "argument-defeater".to_owned(),
        to: "arg-main".to_owned(),
    });
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::Defeated);
    assert_eq!(report.open_defeater_ids, vec!["argument-defeater"]);
}

#[test]
fn assumptions_attached_to_arguments_are_not_ignored() {
    let mut input = base_case();
    add_assumption(&mut input, AssumptionStatus::Open, true);
    let assumption_edge = input.edges.iter_mut().find(|edge| edge.id == "edge-assumption").unwrap();
    assumption_edge.to = "arg-main".to_owned();
    let report = run(&input);
    assert_eq!(report.outcome, CaseOutcome::Indeterminate);
}

#[test]
fn report_is_bound_to_exact_input_bytes_not_json_equivalence() {
    let input = base_case();
    let bytes = serde_json::to_vec(&input).expect("serialize case");
    let pretty = serde_json::to_vec_pretty(&input).expect("serialize pretty case");
    assert_ne!(bytes, pretty);
    let compact_report = evaluate_bytes(&bytes);
    let pretty_report = evaluate_bytes(&pretty);
    assert_eq!(compact_report.outcome, pretty_report.outcome);
    assert_ne!(compact_report.input_sha256, pretty_report.input_sha256);
}

#[test]
fn unknown_node_fields_are_rejected_by_the_closed_contract() {
    let input = base_case();
    let mut value = serde_json::to_value(input).expect("serialize case");
    value["private_unreviewed_field"] = json!({"force_supported": true});
    let bytes = serde_json::to_vec(&value).expect("serialize malformed case");
    let report = evaluate_bytes(&bytes);
    assert_eq!(report.outcome, CaseOutcome::InvalidArgumentGraph);
}

#[test]
fn input_and_report_are_valid_against_closed_json_schemas() {
    let input = base_case();
    let input_value = serde_json::to_value(&input).expect("serialize input");
    let input_schema: serde_json::Value =
        serde_json::from_str(INPUT_SCHEMA).expect("input schema JSON");
    assert!(draft202012::meta::is_valid(&input_schema));
    let input_validator = draft202012::new(&input_schema).expect("compile input schema");
    assert!(
        input_validator.is_valid(&input_value),
        "input schema rejected valid input: {input_value:#}"
    );

    let report = run(&input);
    let report_value = serde_json::to_value(&report).expect("serialize report");
    let report_schema: serde_json::Value =
        serde_json::from_str(REPORT_SCHEMA).expect("report schema JSON");
    assert!(draft202012::meta::is_valid(&report_schema));
    let report_validator = draft202012::new(&report_schema).expect("compile report schema");
    assert!(
        report_validator.is_valid(&report_value),
        "report schema rejected valid report: {report_value:#}"
    );
}
