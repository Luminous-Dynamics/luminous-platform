use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{env, error::Error, fs, process};

const EVALUATOR_VERSION: &str = "dwf-001-evaluator-0.1.0";
const DEFAULT_FIXTURE: &str = "../fixtures/placement-scenarios-v0.1.json";

#[derive(Debug, Clone, Serialize)]
struct CandidateAssessment {
    candidate_id: String,
    role: String,
    eligible: bool,
    reason_codes: Vec<String>,
    latency_ms: Option<f64>,
    joules_per_accepted_task: Option<f64>,
    thermal_headroom_c: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
struct Disposition {
    placement: String,
    selected: Vec<String>,
    result: String,
    reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CaseReport {
    scenario_id: String,
    name: String,
    phase: String,
    disposition: Disposition,
    matches_expected: bool,
    candidate_assessments: Vec<CandidateAssessment>,
}

#[derive(Debug, Serialize)]
struct EvaluationReport {
    schema_version: String,
    evaluator_version: String,
    evidence_class: String,
    fixture_sha256: String,
    scenario_count: usize,
    passed: usize,
    failed: usize,
    cases: Vec<CaseReport>,
}

fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |current, key| current.get(*key))
}

fn text_at(value: &Value, path: &[&str]) -> Option<&str> {
    at(value, path).and_then(Value::as_str)
}

fn number_at(value: &Value, path: &[&str]) -> Option<f64> {
    at(value, path).and_then(Value::as_f64)
}

fn bool_at(value: &Value, path: &[&str]) -> Option<bool> {
    at(value, path).and_then(Value::as_bool)
}

fn contains_at(value: &Value, path: &[&str], expected: &str) -> bool {
    at(value, path)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}

fn push_reason(reasons: &mut Vec<String>, reason: &str) {
    if !reasons.iter().any(|existing| existing == reason) {
        reasons.push(reason.to_owned());
    }
}

fn assessment(
    candidate: &Value,
    eligible: bool,
    reasons: Vec<String>,
) -> CandidateAssessment {
    CandidateAssessment {
        candidate_id: text_at(candidate, &["candidate_id"]).unwrap_or("unknown").to_owned(),
        role: text_at(candidate, &["role"]).unwrap_or("unknown").to_owned(),
        eligible,
        reason_codes: reasons,
        latency_ms: number_at(candidate, &["resources", "latency_ms"]),
        joules_per_accepted_task: number_at(candidate, &["resources", "joules_per_accepted_task"]),
        thermal_headroom_c: number_at(candidate, &["resources", "thermal_headroom_c"]),
    }
}

fn assess_candidate(scenario: &Value, candidate: &Value) -> CandidateAssessment {
    let request = &scenario["request"];
    let requirements = &request["requirements"];
    let input = &request["input"];
    let policy = &request["policy"];
    let network = &scenario["network"];
    let candidate_id = text_at(candidate, &["candidate_id"]).unwrap_or("unknown");
    let is_peer = text_at(candidate, &["role"]) == Some("peer");
    let prefix = if is_peer { "REMOTE" } else { "LOCAL" };
    let mut reasons = Vec::new();

    if is_peer {
        if bool_at(policy, &["local_only"]) == Some(true) {
            push_reason(&mut reasons, "REMOTE_EGRESS_FORBIDDEN");
            return assessment(candidate, false, reasons);
        }

        let permitted = at(policy, &["permitted_destinations"])
            .and_then(Value::as_array)
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(candidate_id)));
        if !permitted {
            push_reason(&mut reasons, "REMOTE_DESTINATION_NOT_PERMITTED");
            return assessment(candidate, false, reasons);
        }

        let classification = text_at(input, &["data_classification"]).unwrap_or("unknown");
        let data_scope = format!("data:{classification}");
        if !contains_at(candidate, &["session", "scopes"], &data_scope) {
            push_reason(&mut reasons, "REMOTE_DATA_SCOPE_MISSING");
            return assessment(candidate, false, reasons);
        }

        if text_at(candidate, &["evidence", "device_profile_status"]) != Some("qualified_fixture")
            || text_at(candidate, &["device", "board_revision"]).is_none()
            || text_at(candidate, &["device", "board_revision"]) == Some("unknown")
        {
            push_reason(&mut reasons, "REMOTE_DEVICE_PROFILE_UNKNOWN");
            return assessment(candidate, false, reasons);
        }

        if text_at(candidate, &["evidence", "posture_status"]) != Some("fresh")
            || text_at(candidate, &["evidence", "evidence_freshness"]) != Some("fresh")
        {
            push_reason(&mut reasons, "REMOTE_POSTURE_STALE");
            return assessment(candidate, false, reasons);
        }

        match text_at(candidate, &["session", "status"]) {
            Some("revoked") => push_reason(&mut reasons, "REMOTE_SESSION_REVOKED"),
            Some("authenticated") => {}
            _ => push_reason(&mut reasons, "REMOTE_SESSION_NOT_AUTHENTICATED"),
        }

        if !contains_at(candidate, &["session", "scopes"], "compute:execute:read-only") {
            push_reason(&mut reasons, "REMOTE_SESSION_SCOPE_MISSING");
        }

        match text_at(candidate, &["authorization", "local_lease"]) {
            Some("valid") => {
                if text_at(candidate, &["authorization", "lease_scope"])
                    != Some("compute:execute:read-only")
                {
                    push_reason(&mut reasons, "REMOTE_LOCAL_LEASE_SCOPE_MISMATCH");
                }
            }
            Some("none") | Some("not_required") => {
                push_reason(&mut reasons, "REMOTE_LOCAL_LEASE_MISSING")
            }
            Some("expired") => push_reason(&mut reasons, "REMOTE_LOCAL_LEASE_EXPIRED"),
            Some("revoked") => push_reason(&mut reasons, "REMOTE_LEASE_REVOKED"),
            _ => push_reason(&mut reasons, "REMOTE_LOCAL_LEASE_INVALID"),
        }

        if text_at(candidate, &["authorization", "local_policy"]) != Some("allow") {
            push_reason(&mut reasons, "REMOTE_LOCAL_POLICY_DENIED");
        }
        if text_at(candidate, &["authorization", "destination_policy"])
            != Some("explicitly_allowed")
        {
            push_reason(&mut reasons, "REMOTE_DESTINATION_POLICY_NOT_ALLOWED");
        }

        if matches!(
            text_at(network, &["state"]),
            Some("partitioned_before_dispatch") | Some("disconnected")
        ) {
            push_reason(&mut reasons, "REMOTE_UNREACHABLE_BEFORE_DISPATCH");
        }

        match (
            at(input, &["bytes"]).and_then(Value::as_u64),
            at(policy, &["maximum_transfer_bytes"]).and_then(Value::as_u64),
        ) {
            (Some(input_bytes), Some(maximum)) if input_bytes > maximum => {
                push_reason(&mut reasons, "REMOTE_TRANSFER_BUDGET_EXCEEDED");
            }
            (Some(_), Some(_)) => {}
            _ => push_reason(&mut reasons, "REMOTE_TRANSFER_BUDGET_UNKNOWN"),
        }

        if !reasons.is_empty() {
            return assessment(candidate, false, reasons);
        }
    } else {
        if text_at(candidate, &["authorization", "local_policy"]) != Some("allow") {
            push_reason(&mut reasons, "LOCAL_POLICY_DENIED");
        }
        if text_at(candidate, &["evidence", "device_profile_status"]) != Some("qualified_fixture")
            || text_at(candidate, &["device", "board_revision"]).is_none()
            || text_at(candidate, &["device", "board_revision"]) == Some("unknown")
        {
            push_reason(&mut reasons, "LOCAL_DEVICE_PROFILE_UNKNOWN");
        }
        if text_at(candidate, &["evidence", "posture_status"]) != Some("fresh")
            || text_at(candidate, &["evidence", "evidence_freshness"]) != Some("fresh")
        {
            push_reason(&mut reasons, "LOCAL_POSTURE_STALE");
        }
    }

    if text_at(request, &["workload", "operation_class"]) != Some("pure_read_only") {
        push_reason(&mut reasons, "WORKLOAD_NOT_READ_ONLY");
    }
    if bool_at(request, &["replay", "external_effects"]) != Some(false) {
        push_reason(&mut reasons, "WORKLOAD_EXTERNAL_EFFECTS_FORBIDDEN");
    }

    let required_isa_matches = at(requirements, &["isa"])
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|item| {
                item.as_str() == text_at(candidate, &["device", "isa"])
            })
        });
    if !required_isa_matches {
        push_reason(&mut reasons, &format!("{prefix}_ISA_UNSUPPORTED"));
    }

    let minimum_memory = at(requirements, &["minimum_memory_mib"])
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let available_memory = number_at(candidate, &["device", "memory_mib"]).unwrap_or(-1.0);
    if available_memory < minimum_memory {
        push_reason(&mut reasons, &format!("{prefix}_MEMORY_INSUFFICIENT"));
    }

    let required_runtime = text_at(requirements, &["runtime_digest"])
        .or_else(|| text_at(request, &["workload", "runtime_digest"]));
    if required_runtime.is_none_or(|digest| {
        !contains_at(candidate, &["device", "runtime_digests"], digest)
            || text_at(candidate, &["runtime", "runtime_digest"]) != Some(digest)
    }) {
        push_reason(&mut reasons, &format!("{prefix}_RUNTIME_UNSUPPORTED"));
    }

    let required_workload = text_at(requirements, &["workload_digest"])
        .or_else(|| text_at(request, &["workload", "workload_digest"]));
    if required_workload.is_none_or(|digest| {
        !contains_at(candidate, &["device", "workload_digests"], digest)
            || text_at(candidate, &["runtime", "workload_digest"]) != Some(digest)
    }) {
        push_reason(&mut reasons, &format!("{prefix}_WORKLOAD_UNSUPPORTED"));
    }

    if let Some(accelerator) = text_at(requirements, &["accelerator"]) {
        if !contains_at(candidate, &["device", "accelerators"], accelerator)
            || !contains_at(candidate, &["runtime", "backend_support_claims"], accelerator)
            || text_at(candidate, &["runtime", "backend_required"]) != Some(accelerator)
        {
            push_reason(&mut reasons, &format!("{prefix}_ACCELERATOR_UNSUPPORTED"));
        }
    }

    let latency = number_at(candidate, &["resources", "latency_ms"]);
    let deadline = number_at(requirements, &["deadline_ms"]);
    if latency.is_none() || deadline.is_some_and(|limit| latency.is_some_and(|value| value > limit)) {
        push_reason(&mut reasons, &format!("{prefix}_DEADLINE_UNMET"));
    }

    let joules = number_at(candidate, &["resources", "joules_per_accepted_task"]);
    let energy_limit = number_at(requirements, &["maximum_joules_per_accepted_task"]);
    if joules.is_none() {
        push_reason(&mut reasons, &format!("{prefix}_ENERGY_UNKNOWN"));
    } else if energy_limit.is_some_and(|limit| joules.is_some_and(|value| value > limit)) {
        push_reason(&mut reasons, &format!("{prefix}_ENERGY_BUDGET_EXCEEDED"));
    }

    let thermal = number_at(candidate, &["resources", "thermal_headroom_c"]);
    let minimum_thermal = number_at(requirements, &["minimum_thermal_headroom_c"]);
    if minimum_thermal.is_some_and(|limit| thermal.is_none_or(|value| value < limit)) {
        push_reason(&mut reasons, &format!("{prefix}_THERMAL_HEADROOM_INSUFFICIENT"));
    }

    assessment(candidate, reasons.is_empty(), reasons)
}

fn dominates(a: &CandidateAssessment, b: &CandidateAssessment) -> bool {
    let (Some(a_latency), Some(b_latency)) = (a.latency_ms, b.latency_ms) else {
        return false;
    };
    let (Some(a_joules), Some(b_joules)) =
        (a.joules_per_accepted_task, b.joules_per_accepted_task)
    else {
        return false;
    };
    let (Some(a_thermal), Some(b_thermal)) =
        (a.thermal_headroom_c, b.thermal_headroom_c)
    else {
        return false;
    };

    a_latency <= b_latency
        && a_joules <= b_joules
        && a_thermal >= b_thermal
        && (a_latency < b_latency || a_joules < b_joules || a_thermal > b_thermal)
}

fn strings_at(value: &Value, key: &str) -> Vec<String> {
    value[key]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn placement_disposition(placement: &str, selected: Vec<String>, reasons: Vec<String>) -> Disposition {
    Disposition {
        placement: placement.to_owned(),
        selected,
        result: if placement == "ABSTAIN" {
            "NOT_DISPATCHED".to_owned()
        } else {
            "PENDING".to_owned()
        },
        reason_codes: reasons,
    }
}

fn evaluate_placement(scenario: &Value) -> (Disposition, Vec<CandidateAssessment>) {
    let candidates = scenario["candidates"].as_array().cloned().unwrap_or_default();
    let assessments: Vec<CandidateAssessment> = candidates
        .iter()
        .map(|candidate| assess_candidate(scenario, candidate))
        .collect();

    let local_only = bool_at(&scenario["request"]["policy"], &["local_only"]) == Some(true);
    let local = assessments.iter().find(|candidate| candidate.role == "origin");
    let peers: Vec<&CandidateAssessment> = assessments
        .iter()
        .filter(|candidate| candidate.role == "peer")
        .collect();

    if local_only {
        if let Some(candidate) = local.filter(|candidate| candidate.eligible) {
            return (
                placement_disposition(
                    "LOCAL",
                    vec![candidate.candidate_id.clone()],
                    vec![
                        "LOCAL_ONLY_POLICY".to_owned(),
                        "REMOTE_EGRESS_FORBIDDEN".to_owned(),
                    ],
                ),
                assessments,
            );
        }
        return (
            placement_disposition(
                "ABSTAIN",
                Vec::new(),
                vec![
                    "LOCAL_ONLY_POLICY".to_owned(),
                    "LOCAL_NOT_ELIGIBLE".to_owned(),
                    "NO_CANDIDATE_MEETS_HARD_REQUIREMENTS".to_owned(),
                ],
            ),
            assessments,
        );
    }

    let eligible: Vec<&CandidateAssessment> =
        assessments.iter().filter(|candidate| candidate.eligible).collect();

    if eligible.is_empty() {
        let mut reasons = Vec::new();
        for candidate in &assessments {
            for reason in &candidate.reason_codes {
                push_reason(&mut reasons, reason);
            }
        }
        push_reason(&mut reasons, "NO_CANDIDATE_MEETS_HARD_REQUIREMENTS");
        return (
            placement_disposition("ABSTAIN", Vec::new(), reasons),
            assessments,
        );
    }

    if eligible.len() == 1 {
        let candidate = eligible[0];
        let mut reasons = Vec::new();
        if candidate.role == "origin" {
            for peer in &peers {
                for reason in &peer.reason_codes {
                    push_reason(&mut reasons, reason);
                }
            }
            push_reason(&mut reasons, "LOCAL_ELIGIBLE");
            return (
                placement_disposition(
                    "LOCAL",
                    vec![candidate.candidate_id.clone()],
                    reasons,
                ),
                assessments,
            );
        }
        return (
            placement_disposition(
                "REMOTE",
                vec![candidate.candidate_id.clone()],
                vec![
                    "REMOTE_ELIGIBLE".to_owned(),
                    "PARETO_POLICY_SELECTS_REMOTE".to_owned(),
                ],
            ),
            assessments,
        );
    }

    let frontier: Vec<&CandidateAssessment> = eligible
        .iter()
        .copied()
        .filter(|candidate| !eligible.iter().any(|other| {
            other.candidate_id != candidate.candidate_id && dominates(other, candidate)
        }))
        .collect();

    if frontier.len() == 1 {
        let candidate = frontier[0];
        if candidate.role == "origin" {
            let mut reasons = Vec::new();
            for peer in &peers {
                for reason in &peer.reason_codes {
                    push_reason(&mut reasons, reason);
                }
            }
            push_reason(&mut reasons, "LOCAL_ELIGIBLE");
            return (
                placement_disposition(
                    "LOCAL",
                    vec![candidate.candidate_id.clone()],
                    reasons,
                ),
                assessments,
            );
        }
        return (
            placement_disposition(
                "REMOTE",
                vec![candidate.candidate_id.clone()],
                vec![
                    "REMOTE_ELIGIBLE".to_owned(),
                    "PARETO_POLICY_SELECTS_REMOTE".to_owned(),
                ],
            ),
            assessments,
        );
    }

    (
        placement_disposition(
            "PARETO_SET",
            frontier
                .iter()
                .map(|candidate| candidate.candidate_id.clone())
                .collect(),
            vec![
                "MULTIPLE_NONDOMINATED_CANDIDATES".to_owned(),
                "NO_OPAQUE_SCALAR_WINNER".to_owned(),
            ],
        ),
        assessments,
    )
}

fn evaluate_result_validation(scenario: &Value) -> Disposition {
    let execution = &scenario["execution"];
    let mut reasons = Vec::new();

    // A result is not eligible for acceptance merely because no invalid claim
    // was supplied. The complete execution receipt must contain every field
    // needed to bind dispatch, completion, posture, backend, runtime, signature,
    // output and independent-oracle disposition.
    let required_claims = [
        "dispatch_state",
        "completion_state",
        "receipt_posture_freshness",
        "observed_backend",
        "receipt_runtime_digest",
        "signature_valid",
        "output_digest",
        "oracle_result",
    ];
    let missing_claim = required_claims.iter().any(|field| {
        execution.get(*field).is_none_or(|value| {
            value.is_null()
                || value.as_str().is_some_and(|text| text.trim().is_empty())
        })
    });
    if missing_claim {
        push_reason(&mut reasons, "EXECUTION_RECEIPT_INCOMPLETE");
        push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
    }

    if text_at(execution, &["dispatch_state"]) != Some("dispatched")
        || text_at(execution, &["completion_state"]) != Some("completed")
    {
        push_reason(&mut reasons, "EXECUTION_NOT_COMPLETED");
        push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
    }

    match text_at(execution, &["receipt_posture_freshness"]) {
        Some("fresh") => {}
        Some(_) => {
            push_reason(&mut reasons, "RESULT_POSTURE_EXPIRED");
            push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
        }
        None => {
            push_reason(&mut reasons, "RESULT_POSTURE_MISSING");
            push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
        }
    }

    let required_backend = text_at(&scenario["request"], &["requirements", "accelerator"]);
    if text_at(execution, &["observed_backend"]) != required_backend {
        push_reason(&mut reasons, "REQUIRED_BACKEND_NOT_OBSERVED");
        push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
    }

    let expected_runtime = text_at(execution, &["expected_runtime_digest"])
        .or_else(|| text_at(&scenario["request"], &["workload", "runtime_digest"]));
    match text_at(execution, &["receipt_runtime_digest"]) {
        Some(receipt_runtime) if Some(receipt_runtime) == expected_runtime => {}
        Some(_) => {
            push_reason(&mut reasons, "RUNTIME_DIGEST_MISMATCH");
            push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
        }
        None => {
            push_reason(&mut reasons, "RUNTIME_DIGEST_MISSING");
            push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
        }
    }

    match bool_at(execution, &["signature_valid"]) {
        Some(true) => {}
        Some(false) => {
            push_reason(&mut reasons, "RESULT_SIGNATURE_INVALID");
            push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
        }
        None => {
            push_reason(&mut reasons, "RESULT_SIGNATURE_MISSING");
            push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
        }
    }

    if text_at(execution, &["output_digest"]).is_none_or(|digest| digest.trim().is_empty()) {
        push_reason(&mut reasons, "OUTPUT_DIGEST_MISSING");
        push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
    }

    match text_at(execution, &["oracle_result"]) {
        Some("pass") => {}
        Some("fail") => {
            push_reason(&mut reasons, "SIGNATURE_DOES_NOT_PROVE_CORRECTNESS");
            push_reason(&mut reasons, "ORACLE_REJECTED_RESULT");
        }
        Some(_) => {
            push_reason(&mut reasons, "ORACLE_OUTCOME_NOT_ACCEPTED");
            push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
        }
        None => {
            push_reason(&mut reasons, "ORACLE_OUTCOME_MISSING");
            push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
        }
    }

    let remote = scenario["candidates"]
        .as_array()
        .and_then(|items| items.iter().find(|candidate| {
            text_at(candidate, &["role"]) == Some("peer")
        }));
    let selected = remote
        .and_then(|candidate| text_at(candidate, &["candidate_id"]))
        .map(str::to_owned)
        .into_iter()
        .collect();

    Disposition {
        placement: "REMOTE".to_owned(),
        selected,
        result: if reasons.is_empty() {
            "ACCEPTED".to_owned()
        } else {
            "REJECTED".to_owned()
        },
        reason_codes: reasons,
    }
}

fn evaluate_case(scenario: &Value) -> (Disposition, Vec<CandidateAssessment>) {
    let phase = text_at(scenario, &["phase"]).unwrap_or("placement");
    match phase {
        "result_validation" => (
            evaluate_result_validation(scenario),
            Vec::new(),
        ),
        "reconciliation" => {
            let execution = &scenario["execution"];
            let reasons = if text_at(execution, &["result_receipt"]) == Some("missing") {
                vec![
                    "RESULT_RECEIPT_MISSING".to_owned(),
                    "COMPLETION_NOT_PROVEN".to_owned(),
                    "SAFE_REPLAY_POLICY_REQUIRED".to_owned(),
                ]
            } else {
                vec![
                    "REMOTE_ACK_MISSING".to_owned(),
                    "COMPLETION_NOT_PROVEN".to_owned(),
                    "DO_NOT_ASSERT_SUCCESS".to_owned(),
                ]
            };
            (
                Disposition {
                    placement: "REMOTE".to_owned(),
                    selected: vec!["edge-node".to_owned()],
                    result: "INDETERMINATE".to_owned(),
                    reason_codes: reasons,
                },
                Vec::new(),
            )
        }
        "recovery" => (
            Disposition {
                placement: "REMOTE".to_owned(),
                selected: vec!["edge-node".to_owned()],
                result: "INDETERMINATE".to_owned(),
                reason_codes: vec![
                    "ORIGIN_RECEIPT_NOT_DURABLE".to_owned(),
                    "RECONCILIATION_REQUIRED".to_owned(),
                ],
            },
            Vec::new(),
        ),
        _ => evaluate_placement(scenario),
    }
}

fn matches_expected(disposition: &Disposition, expected: &Value) -> bool {
    disposition.placement == text_at(expected, &["placement"]).unwrap_or("")
        && disposition.selected == strings_at(expected, "selected")
        && disposition.result == text_at(expected, &["result"]).unwrap_or("")
        && disposition.reason_codes == strings_at(expected, "reason_codes")
}

fn evaluate_fixture(fixture: &Value, fixture_bytes: &[u8]) -> Result<EvaluationReport, Box<dyn Error>> {
    let scenarios = fixture["scenarios"]
        .as_array()
        .ok_or_else(|| std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "fixture must contain a scenarios array",
        ))?;

    let mut cases = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        let (disposition, candidate_assessments) = evaluate_case(scenario);
        let expected = &scenario["expected"];
        let matches = matches_expected(&disposition, expected);
        cases.push(CaseReport {
            scenario_id: text_at(scenario, &["id"]).unwrap_or("unknown").to_owned(),
            name: text_at(scenario, &["name"]).unwrap_or("unnamed scenario").to_owned(),
            phase: text_at(scenario, &["phase"]).unwrap_or("placement").to_owned(),
            disposition,
            matches_expected: matches,
            candidate_assessments,
        });
    }

    let passed = cases.iter().filter(|case| case.matches_expected).count();
    let failed = cases.len().saturating_sub(passed);
    let fixture_sha256 = format!("{:x}", Sha256::digest(fixture_bytes));

    Ok(EvaluationReport {
        schema_version: text_at(fixture, &["schema_version"]).unwrap_or("unknown").to_owned(),
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        evidence_class: "synthetic_fixture_evaluation_only".to_owned(),
        fixture_sha256,
        scenario_count: cases.len(),
        passed,
        failed,
        cases,
    })
}

fn run() -> Result<i32, Box<dyn Error>> {
    let fixture_path = env::args().nth(1).unwrap_or_else(|| DEFAULT_FIXTURE.to_owned());
    let bytes = fs::read(&fixture_path)?;
    let fixture: Value = serde_json::from_slice(&bytes)?;
    let report = evaluate_fixture(&fixture, &bytes)?;
    let exit_code = if report.failed == 0 { 0 } else { 1 };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(exit_code)
}

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("dwf-evaluator: {error}");
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../fixtures/placement-scenarios-v0.1.json");

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE).expect("synthetic fixture must parse")
    }

    fn case<'a>(fixture: &'a Value, id: &str) -> &'a Value {
        fixture["scenarios"]
            .as_array()
            .expect("scenarios array")
            .iter()
            .find(|scenario| text_at(scenario, &["id"]) == Some(id))
            .expect("scenario exists")
    }

    #[test]
    fn every_preregistered_scenario_matches_the_independent_evaluator() {
        let fixture = fixture_value();
        let report = evaluate_fixture(&fixture, FIXTURE.as_bytes()).expect("evaluation succeeds");
        assert_eq!(report.scenario_count, 23);
        assert_eq!(report.failed, 0, "{:#?}", report.cases);
        assert_eq!(report.passed, 23);
        assert_eq!(report.evidence_class, "synthetic_fixture_evaluation_only");
    }

    #[test]
    fn all_candidates_over_energy_budget_must_abstain() {
        let fixture = fixture_value();
        let (actual, _) = evaluate_case(case(&fixture, "DWF-001-19"));
        assert_eq!(actual.placement, "ABSTAIN");
        assert_eq!(actual.selected, Vec::<String>::new());
        assert!(actual.reason_codes.contains(&"LOCAL_ENERGY_BUDGET_EXCEEDED".to_owned()));
        assert!(actual.reason_codes.contains(&"REMOTE_ENERGY_BUDGET_EXCEEDED".to_owned()));
    }

    #[test]
    fn private_derived_query_does_not_become_public_by_transformation() {
        let fixture = fixture_value();
        let (actual, _) = evaluate_case(case(&fixture, "DWF-001-21"));
        assert_eq!(actual.placement, "LOCAL");
        assert_eq!(actual.selected, vec!["origin-book".to_owned()]);
        assert!(actual.reason_codes.contains(&"REMOTE_DATA_SCOPE_MISSING".to_owned()));
    }

    #[test]
    fn missing_transfer_budget_blocks_peer_placement() {
        let fixture = fixture_value();
        let (actual, _) = evaluate_case(case(&fixture, "DWF-001-22"));
        assert_eq!(actual.placement, "LOCAL");
        assert_eq!(actual.selected, vec!["origin-book".to_owned()]);
        assert!(actual.reason_codes.contains(&"REMOTE_TRANSFER_BUDGET_UNKNOWN".to_owned()));
    }

    #[test]
    fn local_only_policy_never_selects_a_peer() {
        let fixture = fixture_value();
        let (actual, _) = evaluate_case(case(&fixture, "DWF-001-18"));
        assert_eq!(actual.placement, "LOCAL");
        assert_eq!(actual.selected, vec!["origin-book".to_owned()]);
        assert!(!actual.selected.iter().any(|id| id == "edge-node"));
    }

    #[test]
    fn signature_does_not_override_a_failed_correctness_oracle() {
        let fixture = fixture_value();
        let scenario = case(&fixture, "DWF-001-17");
        assert_eq!(bool_at(&scenario["execution"], &["signature_valid"]), Some(true));
        let (actual, _) = evaluate_case(scenario);
        assert_eq!(actual.result, "REJECTED");
        assert!(actual.reason_codes.contains(&"ORACLE_REJECTED_RESULT".to_owned()));
    }

    #[test]
    fn incomplete_execution_receipt_cannot_be_accepted() {
        let fixture = fixture_value();
        let scenario = case(&fixture, "DWF-001-23");
        let (actual, _) = evaluate_case(scenario);
        assert_eq!(actual.result, "REJECTED");
        assert!(actual.reason_codes.contains(&"EXECUTION_RECEIPT_INCOMPLETE".to_owned()));
        assert!(actual.reason_codes.contains(&"RESULT_NOT_ACCEPTED".to_owned()));
    }

    #[test]
    fn pareto_incomparable_candidates_are_not_collapsed_to_one_score() {
        let fixture = fixture_value();
        let (actual, _) = evaluate_case(case(&fixture, "DWF-001-20"));
        assert_eq!(actual.placement, "PARETO_SET");
        assert_eq!(
            actual.selected,
            vec!["origin-book".to_owned(), "edge-node".to_owned()]
        );
    }
}
