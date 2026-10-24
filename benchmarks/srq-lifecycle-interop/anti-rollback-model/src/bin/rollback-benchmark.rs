use srq_anti_rollback_model::{fixture_matrix, reconcile, ExpectedOutcome};
use std::process;

fn main() {
    let fixtures = fixture_matrix();
    let expected_count = fixtures.len();
    let expected_positive_count = fixtures
        .iter()
        .filter(|case| matches!(case.expected, ExpectedOutcome::Accept(_)))
        .count();
    let expected_attack_count = expected_count - expected_positive_count;

    let mut accepted_positive_controls = 0usize;
    let mut rejected_attack_cases = 0usize;
    let mut false_accept_count = 0usize;
    let mut false_reject_count = 0usize;
    let mut unexpected_error_count = 0usize;
    let mut rows = Vec::with_capacity(expected_count);

    for fixture in fixtures {
        let expected = fixture.expected.clone();
        let mut journal = fixture.journal;
        let observed = reconcile(&mut journal, &fixture.anchor);
        let passed = match (&expected, &observed) {
            (ExpectedOutcome::Accept(expected_action), Ok(actual_action)) => {
                if expected_action == actual_action {
                    accepted_positive_controls += 1;
                    true
                } else {
                    false_reject_count += 1;
                    unexpected_error_count += 1;
                    false
                }
            }
            (ExpectedOutcome::Reject(expected_error), Err(actual_error)) => {
                rejected_attack_cases += 1;
                if expected_error == actual_error {
                    true
                } else {
                    unexpected_error_count += 1;
                    false
                }
            }
            (ExpectedOutcome::Reject(_), Ok(_)) => {
                false_accept_count += 1;
                false
            }
            (ExpectedOutcome::Accept(_), Err(_)) => {
                false_reject_count += 1;
                false
            }
        };
        rows.push(format!(
            "{{\"name\":\"{}\",\"passed\":{}}}",
            fixture.name, passed
        ));
    }

    let passed = accepted_positive_controls == expected_positive_count
        && rejected_attack_cases == expected_attack_count
        && false_accept_count == 0
        && false_reject_count == 0
        && unexpected_error_count == 0;
    let status = if passed { "PASS" } else { "FAIL" };

    println!(
        concat!(
            "{{",
            "\"status\":\"{}\",",
            "\"qualification_status\":\"BLOCKED\",",
            "\"claim_ceiling\":\"none\",",
            "\"model_scope\":\"abstract_checkpoint_state_machine_only\",",
            "\"expected_case_count\":{},",
            "\"observed_case_count\":{},",
            "\"expected_positive_control_count\":{},",
            "\"accepted_positive_controls\":{},",
            "\"expected_attack_case_count\":{},",
            "\"rejected_attack_cases\":{},",
            "\"false_accept_count\":{},",
            "\"false_reject_count\":{},",
            "\"unexpected_error_count\":{},",
            "\"anchor_backend_count\":0,",
            "\"cases\":[{}]",
            "}}"
        ),
        status,
        expected_count,
        rows.len(),
        expected_positive_count,
        accepted_positive_controls,
        expected_attack_count,
        rejected_attack_cases,
        false_accept_count,
        false_reject_count,
        unexpected_error_count,
        rows.join(",")
    );

    if !passed {
        process::exit(1);
    }
}
