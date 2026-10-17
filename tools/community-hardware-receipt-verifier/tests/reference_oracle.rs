//! Independent, test-only outcome oracle.
//!
//! This deliberately re-states a small set of critical invariants without calling production
//! semantic helpers. Expected dispositions are authored per mutation case below.

use community_hardware_receipt_verifier::verify_bytes;
use serde_json::{json, Value};
use std::collections::BTreeSet;

const SCHEMA: &[u8] = include_bytes!("../../../schemas/community-hardware-test-receipt-v1.schema.json");
const FIXTURE: &[u8] = include_bytes!("../../../docs/fixtures/community-hardware-test-receipt-v1.json");

fn oracle_accepts(r: &Value) -> bool {
    if r.get("deployment_authorization").and_then(Value::as_str) != Some("not_conveyed") {
        return false;
    }
    if r.get("maturity_promotion").and_then(Value::as_str) != Some("none") {
        return false;
    }

    let record_status = r.get("record_status").and_then(Value::as_str).unwrap_or("");
    let overall = r.get("overall_disposition").and_then(Value::as_str).unwrap_or("");
    let class = r.get("evidence_class").and_then(Value::as_str).unwrap_or("");
    let mode = r.pointer("/environment/execution_mode").and_then(Value::as_str).unwrap_or("");

    if record_status == "synthetic_example" && overall != "incomplete" {
        return false;
    }
    if record_status == "synthetic_example" && (class != "vm_simulated" || mode != "vm_simulated") {
        return false;
    }
    if record_status == "community_submission"
        && r.pointer("/subject/unit_identity_method").and_then(Value::as_str) == Some("synthetic_fixture")
    {
        return false;
    }
    if class == "bench_measured" && !matches!(mode, "physical_bench" | "independent_lab") {
        return false;
    }
    if class == "field_observed" && !matches!(mode, "field_observation" | "independent_lab") {
        return false;
    }
    if class == "vm_simulated" && mode != "vm_simulated" {
        return false;
    }
    if class == "unit_contract_tested" && mode != "unit_test" {
        return false;
    }
    if class == "static_checked" && mode != "static" {
        return false;
    }

    let artifacts: BTreeSet<&str> = r.get("evidence_artifacts")
        .and_then(Value::as_array).into_iter().flatten()
        .filter_map(|a| a.get("digest_sha256").and_then(Value::as_str))
        .collect();
    let results = match r.get("results").and_then(Value::as_array) {
        Some(results) => results,
        None => return false,
    };
    let mut test_ids = BTreeSet::new();
    for result in results {
        let id = result.get("test_id").and_then(Value::as_str).unwrap_or("");
        if id.is_empty() || !test_ids.insert(id) {
            return false;
        }
        let status = result.get("status").and_then(Value::as_str).unwrap_or("");
        let refs = match result.get("evidence_refs").and_then(Value::as_array) {
            Some(refs) => refs,
            None => return false,
        };
        if refs.iter().any(|v| v.as_str().is_none_or(|digest| !artifacts.contains(digest))) {
            return false;
        }
        if overall == "pass" {
            if status != "pass" || refs.is_empty() {
                return false;
            }
            if mode == "vm_simulated"
                && ["thermal", "electrical", "radio", "rf-", "acoustic", "physical-boot", "power-consumption", "power-draw"]
                    .iter().any(|token| id.to_ascii_lowercase().contains(token))
            {
                return false;
            }
        }
    }

    if overall == "incomplete"
        && !results.iter().any(|v| matches!(
            v.get("status").and_then(Value::as_str),
            Some("not_run" | "skipped")
        ))
    {
        return false;
    }
    if overall == "fail"
        && !results.iter().any(|v| matches!(
            v.get("status").and_then(Value::as_str),
            Some("fail" | "unsafe_to_continue")
        ))
    {
        return false;
    }

    if record_status == "community_submission" && overall == "pass" {
        for pointer in ["/campaign/suite_source_revision", "/campaign/runner_source_revision"] {
            let revision = r.pointer(pointer).and_then(Value::as_str).unwrap_or("");
            if !matches!(revision.len(), 40 | 64)
                || !revision.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                return false;
            }
        }
    }
    true
}

fn run_case(name: &str, receipt: &Value, expected_accept: bool) {
    let oracle = oracle_accepts(receipt);
    assert_eq!(oracle, expected_accept, "independent expected disposition disagrees for {name}");
    let bytes = serde_json::to_vec(receipt).expect("serialize mutated fixture");
    let observed = verify_bytes(SCHEMA, &bytes, None);
    assert_eq!(
        observed.valid, expected_accept,
        "production verifier and independent oracle disagree for {name}: {observed:?}"
    );
}

fn baseline() -> Value {
    serde_json::from_slice(FIXTURE).expect("synthetic fixture parses")
}

#[test]
fn frozen_adversarial_cases_match_independent_reference_oracle() {
    run_case("synthetic-incomplete-baseline", &baseline(), true);

    let mut synthetic_pass = baseline();
    synthetic_pass["overall_disposition"] = json!("pass");
    run_case("synthetic-cannot-pass", &synthetic_pass, false);

    let mut unknown_digest = baseline();
    unknown_digest["results"][0]["evidence_refs"][0] = json!("a".repeat(64));
    run_case("unregistered-evidence-digest", &unknown_digest, false);

    let mut vm_physical_claim = baseline();
    vm_physical_claim["record_status"] = json!("community_submission");
    vm_physical_claim["subject"]["unit_identity_method"] = json!("label_and_visual_inspection");
    vm_physical_claim["campaign"]["suite_source_revision"] = json!("0123456789abcdef0123456789abcdef01234567");
    vm_physical_claim["campaign"]["runner_source_revision"] = json!("fedcba9876543210fedcba9876543210fedcba98");
    vm_physical_claim["overall_disposition"] = json!("pass");
    vm_physical_claim["results"][0]["status"] = json!("pass");
    vm_physical_claim["results"][1]["status"] = json!("pass");
    vm_physical_claim["results"][1]["evidence_refs"] = json!(["ec343cc7a760e3ba328ff381409b91858ef77a4e6c09fa2f8c2de74401271563"]);
    run_case("vm-cannot-pass-physical-thermal", &vm_physical_claim, false);

    let mut synthetic_identity = baseline();
    synthetic_identity["record_status"] = json!("community_submission");
    run_case("synthetic-unit-misrepresented-as-community-hardware", &synthetic_identity, false);

    let mut mismatched_basis = baseline();
    mismatched_basis["evidence_class"] = json!("bench_measured");
    run_case("bench-evidence-with-vm-environment", &mismatched_basis, false);

    let mut mutable_toolchain = baseline();
    mutable_toolchain["record_status"] = json!("community_submission");
    mutable_toolchain["subject"]["unit_identity_method"] = json!("label_and_visual_inspection");
    mutable_toolchain["campaign"]["suite_source_revision"] = json!("1234567");
    mutable_toolchain["campaign"]["runner_source_revision"] = json!("7654321");
    mutable_toolchain["overall_disposition"] = json!("pass");
    mutable_toolchain["results"][0]["status"] = json!("pass");
    mutable_toolchain["results"][1]["test_id"] = json!("network-basic-check");
    mutable_toolchain["results"][1]["status"] = json!("pass");
    mutable_toolchain["results"][1]["evidence_refs"] = json!(["ec343cc7a760e3ba328ff381409b91858ef77a4e6c09fa2f8c2de74401271563"]);
    run_case("mutable-suite-runner-identity", &mutable_toolchain, false);

    let mut duplicate_test_ids = baseline();
    duplicate_test_ids["results"][1]["test_id"] = duplicate_test_ids["results"][0]["test_id"].clone();
    run_case("duplicate-test-identities", &duplicate_test_ids, false);
}
