use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, error::Error, fs, process};

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
    let mut assessments: Vec<CandidateAssessment> = candidates
        .iter()
        .map(|candidate| assess_candidate(scenario, candidate))
        .collect();
    // Canonical candidate order makes the report stable under fixture-array
    // permutations. Candidate order is not an execution preference.
    assessments.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));

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
    let request = &scenario["request"];
    let mut reasons = Vec::new();

    // Every claim needed to bind the result to the request, selected peer,
    // execution attempt and independent correctness verifier is mandatory.
    // This is fixture-level semantic validation; it is not cryptographic
    // signature verification or execution of the oracle.
    let required_claims = [
        "candidate_id",
        "receipt_device_profile_digest",
        "receipt_posture_digest",
        "receipt_board_revision",
        "dispatch_state",
        "completion_state",
        "receipt_posture_freshness",
        "observed_backend",
        "receipt_runtime_digest",
        "receipt_workload_digest",
        "input_commitment",
        "signature_valid",
        "output_digest",
        "oracle_id",
        "oracle_verifier_id",
        "oracle_verifier_signature_valid",
        "oracle_input_commitment",
        "oracle_output_digest",
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

    let required_backend = text_at(request, &["requirements", "accelerator"]);
    if text_at(execution, &["observed_backend"]) != required_backend {
        push_reason(&mut reasons, "REQUIRED_BACKEND_NOT_OBSERVED");
        push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
    }

    // Expected runtime/workload/input commitments and the required oracle
    // identity are owned by the frozen request, never by the executor receipt.
    let expected_runtime = text_at(request, &["requirements", "runtime_digest"])
        .or_else(|| text_at(request, &["workload", "runtime_digest"]));
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

    let expected_workload = text_at(request, &["requirements", "workload_digest"])
        .or_else(|| text_at(request, &["workload", "workload_digest"]));
    match text_at(execution, &["receipt_workload_digest"]) {
        Some(receipt_workload) if Some(receipt_workload) == expected_workload => {}
        Some(_) => {
            push_reason(&mut reasons, "WORKLOAD_DIGEST_MISMATCH");
            push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
        }
        None => {
            push_reason(&mut reasons, "WORKLOAD_DIGEST_MISSING");
            push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
        }
    }

    let expected_input = text_at(request, &["input", "commitment"]);
    if text_at(execution, &["input_commitment"]) != expected_input {
        push_reason(&mut reasons, "INPUT_COMMITMENT_MISMATCH");
        push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
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

    let output_digest = text_at(execution, &["output_digest"]);
    if output_digest.is_none_or(|digest| digest.trim().is_empty()) {
        push_reason(&mut reasons, "OUTPUT_DIGEST_MISSING");
        push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
    }

    let expected_oracle = text_at(request, &["workload", "correctness_oracle"]);
    if text_at(execution, &["oracle_id"]) != expected_oracle {
        push_reason(&mut reasons, "ORACLE_ID_MISMATCH");
        push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
    }

    let expected_oracle_verifier = text_at(request, &["requirements", "oracle_verifier_id"]);
    if text_at(execution, &["oracle_verifier_id"]) != expected_oracle_verifier {
        push_reason(&mut reasons, "ORACLE_VERIFIER_ID_MISMATCH");
        push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
    }

    if bool_at(execution, &["oracle_verifier_signature_valid"]) != Some(true) {
        push_reason(&mut reasons, "ORACLE_VERIFIER_SIGNATURE_INVALID");
        push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
    }

    if text_at(execution, &["oracle_input_commitment"]) != expected_input {
        push_reason(&mut reasons, "ORACLE_INPUT_COMMITMENT_MISMATCH");
        push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
    }

    match text_at(execution, &["oracle_output_digest"]) {
        Some(oracle_output) if Some(oracle_output) == output_digest => {}
        Some(_) => {
            push_reason(&mut reasons, "ORACLE_OUTPUT_DIGEST_MISMATCH");
            push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
        }
        None => {
            push_reason(&mut reasons, "ORACLE_OUTPUT_DIGEST_MISSING");
            push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
        }
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

    let executor_id = text_at(execution, &["candidate_id"]);
    let selected_peer = scenario["candidates"]
        .as_array()
        .and_then(|items| items.iter().find(|candidate| {
            text_at(candidate, &["role"]) == Some("peer")
                && text_at(candidate, &["candidate_id"]) == executor_id
        }));
    if executor_id.is_none() || selected_peer.is_none() {
        push_reason(&mut reasons, "EXECUTOR_CANDIDATE_NOT_SELECTED");
        push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
    }

    if let Some(peer) = selected_peer {
        let expected_profile = text_at(peer, &["evidence", "device_profile_digest"]);
        match text_at(execution, &["receipt_device_profile_digest"]) {
            Some(receipt_profile) if Some(receipt_profile) == expected_profile => {}
            Some(_) => {
                push_reason(&mut reasons, "EXECUTOR_DEVICE_PROFILE_DIGEST_MISMATCH");
                push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
            }
            None => {
                push_reason(&mut reasons, "EXECUTOR_DEVICE_PROFILE_DIGEST_MISSING");
                push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
            }
        }

        let expected_posture = text_at(peer, &["evidence", "posture_digest"]);
        match text_at(execution, &["receipt_posture_digest"]) {
            Some(receipt_posture) if Some(receipt_posture) == expected_posture => {}
            Some(_) => {
                push_reason(&mut reasons, "EXECUTOR_POSTURE_DIGEST_MISMATCH");
                push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
            }
            None => {
                push_reason(&mut reasons, "EXECUTOR_POSTURE_DIGEST_MISSING");
                push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
            }
        }

        let expected_board = text_at(peer, &["device", "board_revision"]);
        match text_at(execution, &["receipt_board_revision"]) {
            Some(receipt_board) if Some(receipt_board) == expected_board => {}
            Some(_) => {
                push_reason(&mut reasons, "EXECUTOR_BOARD_REVISION_MISMATCH");
                push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
            }
            None => {
                push_reason(&mut reasons, "EXECUTOR_BOARD_REVISION_MISSING");
                push_reason(&mut reasons, "RESULT_NOT_ACCEPTED");
            }
        }
    }

    let selected = selected_peer
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


fn require_nonempty_text(value: &Value, path: &[&str], label: &str) -> Result<(), String> {
    if text_at(value, path).is_none_or(|text| text.trim().is_empty()) {
        return Err(format!("{label} must be a non-empty string"));
    }
    Ok(())
}

fn require_positive_number(value: &Value, path: &[&str], label: &str) -> Result<(), String> {
    let number = number_at(value, path)
        .ok_or_else(|| format!("{label} must be a finite positive number"))?;
    if !number.is_finite() || number <= 0.0 {
        return Err(format!("{label} must be a finite positive number"));
    }
    Ok(())
}

fn require_string_array(value: &Value, path: &[&str], label: &str, nonempty: bool) -> Result<(), String> {
    let items = at(value, path)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label} must be an array of strings"))?;
    if nonempty && items.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if items.iter().any(|item| item.as_str().is_none_or(|text| text.trim().is_empty())) {
        return Err(format!("{label} must contain only non-empty strings"));
    }
    Ok(())
}

/// Validate the scenario/request envelope before evaluating any disposition.
/// Negative controls may omit execution evidence deliberately; mandatory request
/// constraints and candidate identity structure may never be omitted.
fn validate_fixture_contract(fixture: &Value) -> Result<(), String> {
    if text_at(fixture, &["schema_version"]) != Some("0.1") {
        return Err("fixture schema_version must be 0.1".to_owned());
    }
    let scenarios = fixture.get("scenarios")
        .and_then(Value::as_array)
        .ok_or_else(|| "fixture scenarios must be an array".to_owned())?;
    if scenarios.is_empty() {
        return Err("fixture scenarios must not be empty".to_owned());
    }

    let mut scenario_ids = BTreeSet::new();
    for (index, scenario) in scenarios.iter().enumerate() {
        let label = format!("scenario[{index}]");
        require_nonempty_text(scenario, &["id"], &format!("{label}.id"))?;
        let scenario_id = text_at(scenario, &["id"]).expect("validated scenario id");
        if !scenario_ids.insert(scenario_id.to_owned()) {
            return Err(format!("duplicate scenario id: {scenario_id}"));
        }
        require_nonempty_text(scenario, &["name"], &format!("{label}.name"))?;
        let phase = text_at(scenario, &["phase"]).unwrap_or("placement");
        if !matches!(phase, "placement" | "pre_dispatch_recheck" | "result_validation" | "reconciliation" | "recovery") {
            return Err(format!("{label}.phase has unsupported value {phase:?}"));
        }

        let expected = scenario.get("expected")
            .ok_or_else(|| format!("{label}.expected is required"))?;
        for field in ["placement", "result"] {
            require_nonempty_text(expected, &[field], &format!("{label}.expected.{field}"))?;
        }
        require_string_array(expected, &["selected"], &format!("{label}.expected.selected"), false)?;
        require_string_array(expected, &["reason_codes"], &format!("{label}.expected.reason_codes"), false)?;

        let request = scenario.get("request")
            .ok_or_else(|| format!("{label}.request is required"))?;
        require_nonempty_text(request, &["task_id"], &format!("{label}.request.task_id"))?;
        for field in ["semantic_id", "semantic_version", "workload_digest", "runtime_digest", "correctness_oracle"] {
            require_nonempty_text(request, &["workload", field], &format!("{label}.request.workload.{field}"))?;
        }
        if text_at(request, &["workload", "operation_class"]) != Some("pure_read_only") {
            return Err(format!("{label}.request.workload.operation_class must be pure_read_only"));
        }
        require_nonempty_text(request, &["input", "commitment"], &format!("{label}.request.input.commitment"))?;
        require_nonempty_text(request, &["input", "data_classification"], &format!("{label}.request.input.data_classification"))?;
        if at(request, &["input", "bytes"]).and_then(Value::as_u64).is_none() {
            return Err(format!("{label}.request.input.bytes must be an unsigned integer"));
        }
        if bool_at(request, &["policy", "local_only"]).is_none() {
            return Err(format!("{label}.request.policy.local_only must be boolean"));
        }
        require_string_array(request, &["policy", "permitted_destinations"], &format!("{label}.request.policy.permitted_destinations"), false)?;
        if let Some(transfer_limit) = at(request, &["policy", "maximum_transfer_bytes"]) {
            if transfer_limit.as_u64().is_none() {
                return Err(format!("{label}.request.policy.maximum_transfer_bytes must be an unsigned integer when present"));
            }
        }
        require_string_array(request, &["requirements", "isa"], &format!("{label}.request.requirements.isa"), true)?;
        require_positive_number(request, &["requirements", "minimum_memory_mib"], &format!("{label}.request.requirements.minimum_memory_mib"))?;
        require_nonempty_text(request, &["requirements", "accelerator"], &format!("{label}.request.requirements.accelerator"))?;
        require_positive_number(request, &["requirements", "deadline_ms"], &format!("{label}.request.requirements.deadline_ms"))?;
        require_positive_number(request, &["requirements", "maximum_joules_per_accepted_task"], &format!("{label}.request.requirements.maximum_joules_per_accepted_task"))?;
        require_nonempty_text(request, &["requirements", "runtime_digest"], &format!("{label}.request.requirements.runtime_digest"))?;
        require_nonempty_text(request, &["requirements", "workload_digest"], &format!("{label}.request.requirements.workload_digest"))?;
        if bool_at(request, &["replay", "external_effects"]) != Some(false) {
            return Err(format!("{label}.request.replay.external_effects must be false"));
        }
        if bool_at(request, &["replay", "deterministic"]).is_none()
            || bool_at(request, &["replay", "duplicate_execution_safe"]).is_none()
        {
            return Err(format!("{label}.request.replay must declare deterministic and duplicate_execution_safe booleans"));
        }
        let network_state = text_at(scenario, &["network", "state"])
            .ok_or_else(|| format!("{label}.network.state is required"))?;
        if !matches!(network_state, "connected" | "disconnected" | "partitioned_before_dispatch" | "partitioned_after_dispatch" | "connected_after_partition") {
            return Err(format!("{label}.network.state has unsupported value {network_state:?}"));
        }

        let candidates = scenario.get("candidates")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{label}.candidates must be an array"))?;
        if candidates.is_empty() {
            return Err(format!("{label}.candidates must not be empty"));
        }
        let mut candidate_ids = BTreeSet::new();
        let mut origins = 0usize;
        for (candidate_index, candidate) in candidates.iter().enumerate() {
            let candidate_label = format!("{label}.candidates[{candidate_index}]");
            require_nonempty_text(candidate, &["candidate_id"], &format!("{candidate_label}.candidate_id"))?;
            let candidate_id = text_at(candidate, &["candidate_id"]).expect("validated candidate id");
            if !candidate_ids.insert(candidate_id.to_owned()) {
                return Err(format!("{candidate_label} duplicates candidate_id {candidate_id}"));
            }
            match text_at(candidate, &["role"]) {
                Some("origin") => origins += 1,
                Some("peer") => {}
                Some(other) => return Err(format!("{candidate_label}.role has unsupported value {other:?}")),
                None => return Err(format!("{candidate_label}.role is required")),
            }
            for (path, metric_label, strictly_positive) in [
                (&["device", "memory_mib"][..], "memory_mib", false),
                (&["resources", "latency_ms"][..], "latency_ms", true),
                (&["resources", "joules_per_accepted_task"][..], "joules_per_accepted_task", true),
                (&["resources", "thermal_headroom_c"][..], "thermal_headroom_c", false),
            ] {
                if let Some(value) = at(candidate, path) {
                    if !value.is_null() {
                        let number = value.as_f64()
                            .ok_or_else(|| format!("{candidate_label}.{metric_label} must be numeric when present"))?;
                        if !number.is_finite() || (strictly_positive && number <= 0.0) || (!strictly_positive && metric_label == "memory_mib" && number < 0.0) {
                            return Err(format!("{candidate_label}.{metric_label} has an invalid numeric value"));
                        }
                    }
                }
            }
        }
        if origins != 1 {
            return Err(format!("{label}.candidates must contain exactly one origin candidate"));
        }
        if phase == "result_validation" {
            require_nonempty_text(request, &["requirements", "oracle_verifier_id"], &format!("{label}.request.requirements.oracle_verifier_id"))?;
        }
    }
    Ok(())
}

fn evaluate_fixture(fixture: &Value, fixture_bytes: &[u8]) -> Result<EvaluationReport, Box<dyn Error>> {
    validate_fixture_contract(fixture)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
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

    // Fixture order is editorial, not semantic. Canonicalize reports by
    // stable scenario identity so harmless input permutations cannot reorder
    // the evaluator's output records.
    cases.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));

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
    fn result_receipt_must_bind_to_the_selected_device_profile() {
        let fixture = fixture_value();
        let (actual, _) = evaluate_case(case(&fixture, "DWF-001-29"));
        assert_eq!(actual.result, "REJECTED");
        assert!(actual.reason_codes.contains(&"EXECUTOR_DEVICE_PROFILE_DIGEST_MISMATCH".to_owned()));
    }

    #[test]
    fn result_receipt_must_bind_to_the_selected_posture_snapshot() {
        let fixture = fixture_value();
        let (actual, _) = evaluate_case(case(&fixture, "DWF-001-30"));
        assert_eq!(actual.result, "REJECTED");
        assert!(actual.reason_codes.contains(&"EXECUTOR_POSTURE_DIGEST_MISMATCH".to_owned()));
    }

    #[test]
    fn disconnected_peer_is_ineligible_while_local_execution_remains_available() {
        let fixture = fixture_value();
        let mut scenario = case(&fixture, "DWF-001-01").clone();
        scenario["network"]["state"] = Value::String("disconnected".to_owned());

        validate_fixture_contract(&json!({
            "schema_version": "0.1",
            "scenarios": [scenario.clone()]
        })).expect("disconnected is a supported network state");

        let (actual, assessments) = evaluate_placement(&scenario);
        assert_eq!(actual.placement, "LOCAL");
        assert!(assessments.iter().any(|candidate| {
            candidate.role == "peer"
                && !candidate.eligible
                && candidate.reason_codes.contains(&"REMOTE_UNREACHABLE_BEFORE_DISPATCH".to_owned())
        }));
    }

    #[test]
    fn malformed_candidate_role_is_rejected_before_placement() {
        let mut fixture = fixture_value();
        fixture["scenarios"][0]["candidates"][1]["role"] = Value::String("mystery".to_owned());
        let error = validate_fixture_contract(&fixture).unwrap_err();
        assert!(error.contains("unsupported value"), "{error}");
    }

    #[test]
    fn missing_deadline_is_rejected_instead_of_becoming_unbounded() {
        let mut fixture = fixture_value();
        fixture["scenarios"][0]["request"]["requirements"]
            .as_object_mut().unwrap().remove("deadline_ms");
        let error = validate_fixture_contract(&fixture).unwrap_err();
        assert!(error.contains("deadline_ms"), "{error}");
    }

    #[test]
    fn missing_energy_ceiling_is_rejected_instead_of_becoming_unbounded() {
        let mut fixture = fixture_value();
        fixture["scenarios"][0]["request"]["requirements"]
            .as_object_mut().unwrap().remove("maximum_joules_per_accepted_task");
        let error = validate_fixture_contract(&fixture).unwrap_err();
        assert!(error.contains("maximum_joules_per_accepted_task"), "{error}");
    }

    #[test]
    fn duplicate_candidate_identity_is_rejected() {
        let mut fixture = fixture_value();
        let duplicate_id = fixture["scenarios"][0]["candidates"][0]["candidate_id"].clone();
        fixture["scenarios"][0]["candidates"][1]["candidate_id"] = duplicate_id;
        let error = validate_fixture_contract(&fixture).unwrap_err();
        assert!(error.contains("duplicates candidate_id"), "{error}");
    }

    #[test]
    fn duplicate_scenario_ids_are_rejected() {
        let mut fixture = fixture_value();
        let duplicate_id = fixture["scenarios"][0]["id"].clone();
        fixture["scenarios"][1]["id"] = duplicate_id;
        let error = validate_fixture_contract(&fixture).unwrap_err();
        assert!(error.contains("duplicate scenario id"), "{error}");
    }

    #[test]
    fn every_preregistered_scenario_matches_the_independent_evaluator() {
        let fixture = fixture_value();
        let report = evaluate_fixture(&fixture, FIXTURE.as_bytes()).expect("evaluation succeeds");
        assert_eq!(report.scenario_count, 30);
        assert_eq!(report.failed, 0, "{:#?}", report.cases);
        assert_eq!(report.passed, 30);
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
    fn runtime_receipt_is_compared_to_request_owned_requirement() {
        let fixture = fixture_value();
        let mut scenario = case(&fixture, "DWF-001-24").clone();
        scenario["execution"]
            .as_object_mut()
            .expect("execution object")
            .remove("expected_runtime_digest");

        let (actual, _) = evaluate_case(&scenario);
        assert_eq!(actual.result, "ACCEPTED", "{:#?}", actual.reason_codes);
        assert!(actual.reason_codes.is_empty());
    }

    #[test]
    fn result_receipt_must_bind_to_the_selected_peer() {
        let fixture = fixture_value();
        let (actual, _) = evaluate_case(case(&fixture, "DWF-001-27"));
        assert_eq!(actual.result, "REJECTED");
        assert!(actual.reason_codes.contains(&"EXECUTOR_CANDIDATE_NOT_SELECTED".to_owned()));
    }

    #[test]
    fn independent_oracle_receipt_must_bind_to_the_exact_output() {
        let fixture = fixture_value();
        let (actual, _) = evaluate_case(case(&fixture, "DWF-001-28"));
        assert_eq!(actual.result, "REJECTED");
        assert!(actual.reason_codes.contains(&"ORACLE_OUTPUT_DIGEST_MISMATCH".to_owned()));
    }

    #[test]
    fn complete_receipt_accepts_only_verified_oracle_result() {
        let fixture = fixture_value();
        let (accepted, _) = evaluate_case(case(&fixture, "DWF-001-24"));
        assert_eq!(accepted.result, "ACCEPTED");
        assert!(accepted.reason_codes.is_empty());

        let (rejected, _) = evaluate_case(case(&fixture, "DWF-001-25"));
        assert_eq!(rejected.result, "REJECTED");
        assert!(rejected.reason_codes.contains(&"RESULT_SIGNATURE_INVALID".to_owned()));
    }


    fn mutate_peer(scenario: &mut Value, mutate: impl FnOnce(&mut Value)) {
        let candidates = scenario["candidates"]
            .as_array_mut()
            .expect("candidate array");
        let peer = candidates
            .iter_mut()
            .find(|candidate| text_at(candidate, &["role"]) == Some("peer"))
            .expect("peer candidate");
        mutate(peer);
    }

    #[test]
    fn hard_eligibility_is_monotone_under_single_field_policy_tightening() {
        let fixture = fixture_value();
        let baseline = case(&fixture, "DWF-001-01").clone();
        let baseline_peer = baseline["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| text_at(candidate, &["role"]) == Some("peer"))
            .unwrap();
        assert!(assess_candidate(&baseline, baseline_peer).eligible);

        let mutations: Vec<(&str, Box<dyn Fn(&mut Value)>)> = vec![
            ("local_only", Box::new(|s| s["request"]["policy"]["local_only"] = Value::Bool(true))),
            ("destination_not_permitted", Box::new(|s| s["request"]["policy"]["permitted_destinations"] = serde_json::json!([]))),
            ("private_data_class", Box::new(|s| s["request"]["input"]["data_classification"] = Value::String("private".to_owned()))),
            ("missing_data_scope", Box::new(|s| mutate_peer(s, |p| p["session"]["scopes"] = serde_json::json!(["compute:execute:read-only"])))),
            ("revoked_session", Box::new(|s| mutate_peer(s, |p| p["session"]["status"] = Value::String("revoked".to_owned())))),
            ("missing_compute_scope", Box::new(|s| mutate_peer(s, |p| p["session"]["scopes"] = serde_json::json!(["data:public"])))),
            ("expired_lease", Box::new(|s| mutate_peer(s, |p| p["authorization"]["local_lease"] = Value::String("expired".to_owned())))),
            ("denied_local_policy", Box::new(|s| mutate_peer(s, |p| p["authorization"]["local_policy"] = Value::String("deny".to_owned())))),
            ("denied_destination_policy", Box::new(|s| mutate_peer(s, |p| p["authorization"]["destination_policy"] = Value::String("denied".to_owned())))),
            ("stale_posture", Box::new(|s| mutate_peer(s, |p| p["evidence"]["posture_status"] = Value::String("stale".to_owned())))),
            ("unknown_device_profile", Box::new(|s| mutate_peer(s, |p| p["evidence"]["device_profile_status"] = Value::String("unknown".to_owned())))),
            ("unsupported_isa", Box::new(|s| mutate_peer(s, |p| p["device"]["isa"] = Value::String("riscv64".to_owned())))),
            ("insufficient_memory", Box::new(|s| mutate_peer(s, |p| p["device"]["memory_mib"] = serde_json::json!(1)))),
            ("runtime_digest_mismatch", Box::new(|s| mutate_peer(s, |p| p["runtime"]["runtime_digest"] = Value::String("runtime:other".to_owned())))),
            ("workload_digest_mismatch", Box::new(|s| mutate_peer(s, |p| p["runtime"]["workload_digest"] = Value::String("workload:other".to_owned())))),
            ("required_backend_missing", Box::new(|s| mutate_peer(s, |p| p["runtime"]["backend_support_claims"] = serde_json::json!([])))),
            ("deadline_exceeded", Box::new(|s| mutate_peer(s, |p| p["resources"]["latency_ms"] = serde_json::json!(9999)))),
            ("energy_budget_exceeded", Box::new(|s| mutate_peer(s, |p| p["resources"]["joules_per_accepted_task"] = serde_json::json!(9999)))),
            ("transfer_budget_exceeded", Box::new(|s| s["request"]["policy"]["maximum_transfer_bytes"] = serde_json::json!(1))),
            ("transfer_budget_unknown", Box::new(|s| s["request"]["policy"]["maximum_transfer_bytes"] = Value::Null)),
            ("network_disconnected", Box::new(|s| s["network"]["state"] = Value::String("disconnected".to_owned()))),
            ("non_read_only_workload", Box::new(|s| s["request"]["workload"]["operation_class"] = Value::String("stateful".to_owned()))),
            ("external_effects", Box::new(|s| s["request"]["replay"]["external_effects"] = Value::Bool(true))),
        ];

        for (name, mutate) in mutations {
            let mut scenario = baseline.clone();
            mutate(&mut scenario);
            let candidates = scenario["candidates"].as_array().unwrap();
            let peer = candidates
                .iter()
                .find(|candidate| text_at(candidate, &["role"]) == Some("peer"))
                .unwrap();
            let assessment = assess_candidate(&scenario, peer);
            assert!(
                !assessment.eligible,
                "hard constraint mutation {name} unexpectedly left peer eligible: {:?}",
                assessment.reason_codes
            );
        }
    }

    #[test]
    fn privacy_rejection_cannot_be_compensated_by_arbitrarily_better_performance() {
        let fixture = fixture_value();
        let mut scenario = case(&fixture, "DWF-001-01").clone();
        mutate_peer(&mut scenario, |peer| {
            peer["session"]["scopes"] = serde_json::json!(["compute:execute:read-only"]);
            peer["resources"]["latency_ms"] = serde_json::json!(0.001);
            peer["resources"]["joules_per_accepted_task"] = serde_json::json!(0.000001);
            peer["resources"]["thermal_headroom_c"] = serde_json::json!(1000.0);
        });

        let (actual, assessments) = evaluate_placement(&scenario);
        assert_eq!(actual.placement, "LOCAL");
        assert_eq!(actual.selected, vec!["origin-book".to_owned()]);
        assert!(assessments.iter().any(|candidate| {
            candidate.role == "peer"
                && !candidate.eligible
                && candidate.reason_codes.contains(&"REMOTE_DATA_SCOPE_MISSING".to_owned())
        }));
    }

    #[test]
    fn candidate_array_permutation_does_not_change_the_selected_frontier() {
        let fixture = fixture_value();
        let original = case(&fixture, "DWF-001-20").clone();
        let mut permuted = original.clone();
        permuted["candidates"].as_array_mut().unwrap().reverse();

        let (a, assessments_a) = evaluate_placement(&original);
        let (b, assessments_b) = evaluate_placement(&permuted);
        assert_eq!(a.placement, "PARETO_SET");
        assert_eq!(a.placement, b.placement);
        assert_eq!(a.selected, b.selected);
        assert_eq!(
            assessments_a.iter().map(|item| item.candidate_id.as_str()).collect::<Vec<_>>(),
            assessments_b.iter().map(|item| item.candidate_id.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn scenario_array_permutation_preserves_canonical_case_reports() {
        let fixture = fixture_value();
        let mut permuted = fixture.clone();
        permuted["scenarios"].as_array_mut().unwrap().reverse();

        let original_bytes = serde_json::to_vec(&fixture).expect("serialize original fixture");
        let permuted_bytes = serde_json::to_vec(&permuted).expect("serialize permuted fixture");
        let original = evaluate_fixture(&fixture, &original_bytes).expect("original evaluation");
        let reordered = evaluate_fixture(&permuted, &permuted_bytes).expect("permuted evaluation");

        // Exact fixture identity remains bound to the supplied bytes, so a
        // reordered fixture must have a different digest even though its
        // semantic per-case report is unchanged.
        assert_ne!(original.fixture_sha256, reordered.fixture_sha256);
        assert_eq!(
            serde_json::to_value(original.cases).expect("serialize original case reports"),
            serde_json::to_value(reordered.cases).expect("serialize reordered case reports")
        );
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
