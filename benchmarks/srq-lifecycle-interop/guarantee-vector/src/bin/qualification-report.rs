use srq_guarantee_vector::{evaluate, exhaustive_invariant_summary, fixture_matrix, Decision};
use std::process;

fn main() {
    let cases = fixture_matrix();
    let mut unexpected_decision_count = 0usize;
    let mut false_release_count = 0usize;
    let mut false_block_count = 0usize;
    let mut rows = Vec::with_capacity(cases.len());
    let mut blocked_count = 0usize;
    let mut operator_required_count = 0usize;
    let mut completed_count = 0usize;
    let mut permitted_count = 0usize;

    for case in cases {
        let observed = evaluate(&case.vector);
        let decision_matches = observed.decision == case.expected;
        if !decision_matches {
            unexpected_decision_count += 1;
        }
        if observed.action_permitted() && !case.expected_action_permitted {
            false_release_count += 1;
        }
        if !observed.action_permitted() && case.expected_action_permitted {
            false_block_count += 1;
        }
        if observed.decision == Decision::Block {
            blocked_count += 1;
        }
        if observed.decision == Decision::OperatorRequired {
            operator_required_count += 1;
        }
        if observed.decision == Decision::CompletedNoRedispatch {
            completed_count += 1;
        }
        if observed.action_permitted() {
            permitted_count += 1;
        }
        rows.push(format!(
            "{{\"name\":\"{}\",\"expected\":\"{}\",\"observed\":\"{}\",\"expected_action_permitted\":{},\"observed_action_permitted\":{},\"passed\":{}}}",
            case.name,
            case.expected.as_str(),
            observed.decision.as_str(),
            case.expected_action_permitted,
            observed.action_permitted(),
            decision_matches && observed.action_permitted() == case.expected_action_permitted
        ));
    }

    let case_count = rows.len();
    let exhaustive = exhaustive_invariant_summary();
    let passed = unexpected_decision_count == 0
        && false_release_count == 0
        && false_block_count == 0
        && case_count == 22
        && exhaustive.combinations_checked == 2_000
        && exhaustive.false_release_count == 0
        && exhaustive.false_block_count == 0;
    let status = if passed { "PASS" } else { "FAIL" };

    println!(
        concat!(
            "{{",
            "\"status\":\"{}\",",
            "\"qualification_status\":\"BLOCKED\",",
            "\"claim_ceiling\":\"none\",",
            "\"scope\":\"typed_evidence_evaluator_only\",",
            "\"expected_case_count\":22,",
            "\"observed_case_count\":{},",
            "\"unexpected_decision_count\":{},",
            "\"false_release_count\":{},",
            "\"false_block_count\":{},",
            "\"blocked_count\":{},",
            "\"operator_required_count\":{},",
            "\"completed_no_redispatch_count\":{},",
            "\"action_permitted_count\":{},",
            "\"exhaustive_combinations_checked\":{},",
            "\"exhaustive_false_release_count\":{},",
            "\"exhaustive_false_block_count\":{},",
            "\"exhaustive_action_permitted_count\":{},",
            "\"cases\":[{}]",
            "}}"
        ),
        status,
        case_count,
        unexpected_decision_count,
        false_release_count,
        false_block_count,
        blocked_count,
        operator_required_count,
        completed_count,
        permitted_count,
        exhaustive.combinations_checked,
        exhaustive.false_release_count,
        exhaustive.false_block_count,
        exhaustive.observed_action_permitted_count,
        rows.join(",")
    );

    if !passed {
        process::exit(1);
    }
}
