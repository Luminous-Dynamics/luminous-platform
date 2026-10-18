//! Offline validation for the community Wi-Fi roaming benchmark profile and matrix.
//!
//! A valid profile records contract shape/policy only. A valid matrix proves
//! schema and count consistency, not that a radio/authentication trial occurred.

use jsonschema::draft202012;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process;

const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;
const PROFILE_VERSION: &str = "luminous.community-wifi-roaming-benchmark-profile.v1";
const PROFILE_ID: &str = "community-wifi-roaming-openroaming-v1";
const MATRIX_VERSION: &str = "luminous.community-wifi-roaming-interoperability-matrix.v1";

const REQUIRED_PROFILE_NON_CLAIMS: &[&str] = &[
    "No claim of OpenRoaming federation membership",
    "No claim of live roaming credential or identity-provider access",
    "No claim that an open-source portal conveys federation eligibility",
    "No claim of free upstream transit or zero-cost internet",
    "No claim of global coverage or universal handset compatibility",
    "No claim of seamless roaming until measured for the named flow and device",
    "No claim of authorized upstream sharing or resale",
    "No claim of measured field performance from a schema, simulation or CI result",
];

const REQUIRED_PROFILE_ARTIFACTS: &[&str] = &[
    "Named client device, OS build, AP firmware and credential-profile capability matrix",
    "Controlled two-AP local AAA lab with invalid-certificate and wrong-realm negative controls",
    "Privacy-minimizing telemetry and public receipt specification",
    "Provider/federation eligibility and upstream permission evidence for any roaming claim",
    "Baseline-first timing, failure and economic measurements with raw denominators",
    "Deployment-specific ICASA and equipment type-approval review",
    "Independent evidence review and repeatable rollback procedure",
];

const REQUIRED_MATRIX_NON_CLAIMS: &[&str] = &[
    "No device or OS compatibility is established by this template alone.",
    "No OpenRoaming federation membership or credential entitlement is conveyed.",
    "No upstream sharing, resale, spectrum or deployment authorization is conveyed.",
    "A passing row applies only to the exact recorded client, OS, profile, AP, AAA, federation and site combination.",
];

struct ExpectedBenchmark {
    id: &'static str,
    operator: &'static str,
    target: Value,
    target_status: &'static str,
}

fn expected_benchmarks() -> Vec<ExpectedBenchmark> {
    vec![
        ExpectedBenchmark { id: "WIFI_AUTH_INVALID_CERT_REJECT_001", operator: "==", target: json!(0), target_status: "proposed_target" },
        ExpectedBenchmark { id: "WIFI_AUTH_REALM_REJECT_001", operator: "==", target: json!(0), target_status: "proposed_target" },
        ExpectedBenchmark { id: "WIFI_SECRET_EXPOSURE_001", operator: "==", target: json!(0), target_status: "proposed_target" },
        ExpectedBenchmark { id: "WIFI_TIME_TO_USABLE_IP_001", operator: "baseline_then_set_target", target: Value::Null, target_status: "baseline_first" },
        ExpectedBenchmark { id: "WIFI_FIRST_ATTEMPT_AUTH_001", operator: "baseline_then_set_target", target: Value::Null, target_status: "baseline_first" },
        ExpectedBenchmark { id: "WIFI_HANDOFF_INTERRUPTION_001", operator: "baseline_then_set_target", target: Value::Null, target_status: "baseline_first" },
        ExpectedBenchmark { id: "WIFI_FAILURE_RECOVERY_001", operator: "baseline_then_set_target", target: Value::Null, target_status: "baseline_first" },
        ExpectedBenchmark { id: "WIFI_LOCAL_CONTINUITY_001", operator: ">=", target: json!(24), target_status: "proposed_target" },
        ExpectedBenchmark { id: "WIFI_COST_PER_USEFUL_TASK_001", operator: "baseline_then_set_target", target: Value::Null, target_status: "baseline_first" },
        ExpectedBenchmark { id: "WIFI_ENERGY_PER_TASK_001", operator: "baseline_then_minimize", target: Value::Null, target_status: "baseline_first" },
        ExpectedBenchmark { id: "WIFI_INTEROP_EVIDENCE_001", operator: "==", target: json!(1), target_status: "proposed_target" },
        ExpectedBenchmark { id: "WIFI_FEDERATION_ELIGIBILITY_001", operator: "baseline_then_set_target", target: Value::Null, target_status: "blocked_on_provider_agreement_and_entitlement" },
        ExpectedBenchmark { id: "WIFI_CLIENT_ISOLATION_001", operator: "==", target: json!(0), target_status: "proposed_target" },
    ]
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut limited = file.take((MAX_INPUT_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    limited.read_to_end(&mut bytes).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(format!("{} exceeds the {MAX_INPUT_BYTES} byte input limit", path.display()));
    }
    Ok(bytes)
}

fn digest_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn collect_nonlocal_references(value: &Value, path: &str, errors: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if key == "$ref" || key == "$dynamicRef" {
                    match child.as_str() {
                        Some(reference) if reference.starts_with('#') => {}
                        Some(_) => errors.push(format!("{path}/{key}: only local fragment references are allowed")),
                        None => errors.push(format!("{path}/{key}: reference must be a string")),
                    }
                }
                collect_nonlocal_references(child, &format!("{path}/{key}"), errors);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_nonlocal_references(child, &format!("{path}/{index}"), errors);
            }
        }
        _ => {}
    }
}

fn contains_text(value: Option<&Value>, required: &str) -> bool {
    value.and_then(Value::as_array).is_some_and(|items| {
        items.iter().any(|item| item.as_str() == Some(required))
    })
}

fn build_validator(schema: &Value) -> Result<impl Fn(&Value) -> Vec<String>, Vec<String>> {
    let mut reference_errors = Vec::new();
    collect_nonlocal_references(schema, "$schema", &mut reference_errors);
    if !reference_errors.is_empty() {
        return Err(reference_errors);
    }

    let meta = jsonschema::meta::validator_for(schema)
        .map_err(|error| vec![format!("cannot build JSON Schema meta-validator: {error}")])?;
    if !meta.evaluate(schema).flag().valid {
        return Err(vec!["schema failed JSON Schema meta-validation".to_owned()]);
    }

    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| vec![format!("cannot build Draft 2020-12 validator: {error}")])?;
    Ok(move |instance: &Value| {
        validator
            .iter_errors(instance)
            .map(|error| format!("schema: {error}"))
            .collect()
    })
}

fn validate_profile(schema: &Value, profile: &Value) -> Vec<String> {
    let mut errors = match build_validator(schema) {
        Ok(validate) => validate(profile),
        Err(errors) => return errors,
    };

    if profile.get("schema_version").and_then(Value::as_str) != Some(PROFILE_VERSION) {
        errors.push(format!("schema_version must be {PROFILE_VERSION}"));
    }
    if profile.get("profile_id").and_then(Value::as_str) != Some(PROFILE_ID) {
        errors.push(format!("profile_id must be {PROFILE_ID}"));
    }
    if profile.get("status").and_then(Value::as_str) != Some("draft_research_only") {
        errors.push("profile status must remain draft_research_only".to_owned());
    }

    let gates = profile.get("deployment_gates");
    for (key, expected) in [
        ("federation_membership_status", json!("not_verified")),
        ("identity_provider_credentials_status", json!("not_obtained")),
        ("upstream_sharing_permission_status", json!("not_verified")),
        ("regulatory_review_status", json!("not_verified")),
        ("equipment_type_approval_status", json!("not_verified")),
        ("deployment_blocked_until_gates_verified", json!(true)),
        ("local_only_access_must_not_require_wan", json!(true)),
        ("payload_inspection_for_routing", json!(false)),
        ("credential_validation_must_fail_closed", json!(true)),
    ] {
        if gates.and_then(|value| value.get(key)) != Some(&expected) {
            errors.push(format!("deployment_gates/{key} must remain {expected}"));
        }
    }

    let contract = profile.get("comparison_contract");
    for key in [
        "device_os_profile_and_site_scope_must_be_named",
        "unsupported_and_failed_attempts_remain_in_denominator",
        "simulation_must_not_be_reported_as_field_evidence",
        "raw_credentials_and_payloads_must_not_be_in_public_receipts",
        "targets_without_field_baselines_remain_baseline_first",
        "source_and_configuration_revisions_must_be_immutable",
    ] {
        if contract.and_then(|value| value.get(key)).and_then(Value::as_bool) != Some(true) {
            errors.push(format!("comparison_contract/{key} must remain true"));
        }
    }

    let actual = profile.get("benchmarks").and_then(Value::as_array);
    let expected = expected_benchmarks();
    match actual {
        Some(actual) => {
            if actual.len() != expected.len() {
                errors.push(format!("expected exactly {} versioned benchmarks; found {}", expected.len(), actual.len()));
            }
            let mut ids = BTreeSet::new();
            for (index, item) in actual.iter().enumerate() {
                match item.get("id").and_then(Value::as_str) {
                    Some(id) if !ids.insert(id.to_owned()) => errors.push(format!("benchmarks/{index}/id duplicates {id}")),
                    Some(_) => {}
                    None => errors.push(format!("benchmarks/{index}/id must be a string")),
                }
            }
            for spec in expected {
                let matches: Vec<&Value> = actual.iter().filter(|item| item.get("id").and_then(Value::as_str) == Some(spec.id)).collect();
                if matches.len() != 1 {
                    errors.push(format!("benchmark {} must occur exactly once; found {}", spec.id, matches.len()));
                    continue;
                }
                let item = matches[0];
                for (field, expected_value) in [
                    ("operator", json!(spec.operator)),
                    ("target", spec.target.clone()),
                    ("target_status", json!(spec.target_status)),
                ] {
                    if item.get(field) != Some(&expected_value) {
                        errors.push(format!("benchmarks/{}/{} must remain {}", spec.id, field, expected_value));
                    }
                }
            }
        }
        None => errors.push("benchmarks must be an array".to_owned()),
    }

    for required in REQUIRED_PROFILE_NON_CLAIMS {
        if !contains_text(profile.get("explicit_non_claims"), required) {
            errors.push(format!("explicit_non_claims must include {required:?}"));
        }
    }
    for required in REQUIRED_PROFILE_ARTIFACTS {
        if !contains_text(profile.get("next_required_artifacts"), required) {
            errors.push(format!("next_required_artifacts must include {required:?}"));
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

fn validate_matrix(schema: &Value, matrix: &Value) -> Vec<String> {
    let mut errors = match build_validator(schema) {
        Ok(validate) => validate(matrix),
        Err(errors) => return errors,
    };

    if matrix.get("schema_version").and_then(Value::as_str) != Some(MATRIX_VERSION) {
        errors.push(format!("schema_version must be {MATRIX_VERSION}"));
    }
    let rows = matrix.get("rows").and_then(Value::as_array);
    match rows {
        Some(rows) => {
            let expected_status = if rows.is_empty() { "empty_template_no_observations" } else { "draft_evidence_matrix" };
            if matrix.get("status").and_then(Value::as_str) != Some(expected_status) {
                errors.push(format!("matrix status must be {expected_status} for {} row(s)", rows.len()));
            }

            let mut row_ids = BTreeSet::new();
            for (index, row) in rows.iter().enumerate() {
                match row.get("row_id").and_then(Value::as_str) {
                    Some(id) if !row_ids.insert(id.to_owned()) => errors.push(format!("rows/{index}/row_id duplicates {id}")),
                    Some(_) => {}
                    None => {}
                }
                let run = row.get("test_run");
                let counts = [
                    run.and_then(|v| v.get("attempted")).and_then(Value::as_u64),
                    run.and_then(|v| v.get("succeeded")).and_then(Value::as_u64),
                    run.and_then(|v| v.get("failed")).and_then(Value::as_u64),
                    run.and_then(|v| v.get("inconclusive")).and_then(Value::as_u64),
                ];
                if counts.iter().any(Option::is_none) {
                    errors.push(format!("rows/{index}/test_run counts must be unsigned integers"));
                } else {
                    let [attempted, succeeded, failed, inconclusive] = counts.map(Option::unwrap);
                    if attempted != succeeded + failed + inconclusive {
                        errors.push(format!("rows/{index}/test_run denominator mismatch: attempted must equal succeeded + failed + inconclusive"));
                    }
                    let claim = row.get("claim_status").and_then(Value::as_str);
                    if claim == Some("not_tested") && attempted != 0 {
                        errors.push(format!("rows/{index}: not_tested must have zero attempts"));
                    }
                    if matches!(claim, Some("supported" | "unsupported" | "inconclusive")) && attempted == 0 {
                        errors.push(format!("rows/{index}: tested claim status requires attempts"));
                    }
                    if matches!(claim, Some("supported" | "unsupported")) && inconclusive != 0 {
                        errors.push(format!("rows/{index}: supported/unsupported claim cannot include inconclusive trials"));
                    }
                }
            }
        }
        None => errors.push("rows must be an array".to_owned()),
    }

    for required in REQUIRED_MATRIX_NON_CLAIMS {
        if !contains_text(matrix.get("explicit_non_claims"), required) {
            errors.push(format!("explicit_non_claims must include {required:?}"));
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

fn load_json(path: &Path) -> Result<(Vec<u8>, Value), String> {
    let bytes = read_bounded(path)?;
    let value = serde_json::from_slice(&bytes).map_err(|error| format!("{} is not valid JSON: {error}", path.display()))?;
    Ok((bytes, value))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 || args[1] == "--help" || args[1] == "-h" {
        eprintln!(
            "Usage: community-wifi-roaming-verify <benchmark-schema.json> <benchmark-profile.json> <matrix-schema.json> <matrix.json>\n\
             Validates draft profile/matrix structure and locked policy invariants only; no compatibility or federation membership is conveyed."
        );
        process::exit(if args.len() == 2 { 0 } else { 64 });
    }

    let (profile_schema_bytes, profile_schema) = match load_json(Path::new(&args[1])) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); process::exit(2); }
    };
    let (profile_bytes, profile) = match load_json(Path::new(&args[2])) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); process::exit(2); }
    };
    let (matrix_schema_bytes, matrix_schema) = match load_json(Path::new(&args[3])) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); process::exit(2); }
    };
    let (matrix_bytes, matrix) = match load_json(Path::new(&args[4])) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); process::exit(2); }
    };

    let mut errors = validate_profile(&profile_schema, &profile);
    errors.extend(validate_matrix(&matrix_schema, &matrix));
    errors.sort();
    errors.dedup();
    let valid = errors.is_empty();
    let report = json!({
        "report_version": "luminous.community-wifi-roaming-validation-report.v1",
        "profile_id": profile.get("profile_id").and_then(Value::as_str),
        "matrix_id": matrix.get("matrix_id").and_then(Value::as_str),
        "profile_sha256": digest_hex(&profile_bytes),
        "profile_schema_sha256": digest_hex(&profile_schema_bytes),
        "matrix_sha256": digest_hex(&matrix_bytes),
        "matrix_schema_sha256": digest_hex(&matrix_schema_bytes),
        "benchmark_count": profile.get("benchmarks").and_then(Value::as_array).map_or(0, |items| items.len()),
        "matrix_row_count": matrix.get("rows").and_then(Value::as_array).map_or(0, |items| items.len()),
        "valid": valid,
        "validation_scope": "schema_plus_locked_policy_and_matrix_denominators",
        "device_compatibility_conveyed": false,
        "federation_membership_conveyed": false,
        "deployment_authorization_conveyed": false,
        "field_performance_conveyed": false,
        "errors": errors
    });

    let mut stdout = std::io::stdout().lock();
    if let Err(error) = serde_json::to_writer_pretty(&mut stdout, &report) {
        eprintln!("cannot write validation report: {error}");
        process::exit(2);
    }
    if let Err(error) = stdout.write_all(b"\n") {
        eprintln!("cannot terminate validation report: {error}");
        process::exit(2);
    }
    if !valid { process::exit(1); }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> (Value, Value, Value, Value) {
        let profile_schema: Value = serde_json::from_str(include_str!("../../../schemas/community-wifi-roaming-benchmark-profile-v1.schema.json")).expect("profile schema parses");
        let profile: Value = serde_json::from_str(include_str!("../../../profiles/community-wifi-roaming-benchmark-profile-v1.json")).expect("profile parses");
        let matrix_schema: Value = serde_json::from_str(include_str!("../../../schemas/community-wifi-roaming-interoperability-matrix-v1.schema.json")).expect("matrix schema parses");
        let matrix: Value = serde_json::from_str(include_str!("../../../profiles/community-wifi-roaming-interoperability-matrix-template-v1.json")).expect("matrix template parses");
        (profile_schema, profile, matrix_schema, matrix)
    }

    fn not_tested_row() -> Value {
        json!({
            "row_id":"row-test-001",
            "claim_scope":"local_passpoint",
            "claim_status":"not_tested",
            "client":{"manufacturer":"Synthetic","model":"Fixture","os_build":"fixture-only","wifi_chipset":null},
            "credential":{"identity_provider_label":"synthetic-local-lab","profile_revision":"fixture-v1","eap_method":"not_applicable","credential_state":"not_obtained"},
            "access_network":{"ap_manufacturer":"Synthetic","ap_model":"Fixture AP","firmware_revision":"fixture-only","aaa_provider_token":"aaaaaaaa","visited_network_token":"bbbbbbbb","federation_state":"not_applicable","region_code":null},
            "test_run":{"tested_at":null,"evidence_class":"none","attempted":0,"succeeded":0,"failed":0,"inconclusive":0,"receipt_path":null,"receipt_sha256":null},
            "limitations":["Synthetic row used by unit tests only; no compatibility or field testing occurred."]
        })
    }

    #[test]
    fn checked_in_profile_and_empty_matrix_pass() {
        let (ps, p, ms, m) = fixtures();
        assert!(validate_profile(&ps, &p).is_empty(), "profile errors: {:#?}", validate_profile(&ps, &p));
        assert!(validate_matrix(&ms, &m).is_empty(), "matrix errors: {:#?}", validate_matrix(&ms, &m));
    }

    #[test]
    fn weakening_invalid_certificate_target_is_rejected() {
        let (ps, mut p, _, _) = fixtures();
        p["benchmarks"][0]["target"] = json!(1);
        assert!(!validate_profile(&ps, &p).is_empty());
    }

    #[test]
    fn federation_membership_cannot_be_self_declared() {
        let (ps, mut p, _, _) = fixtures();
        p["deployment_gates"]["federation_membership_status"] = json!("verified");
        p["deployment_gates"]["deployment_blocked_until_gates_verified"] = json!(false);
        assert!(!validate_profile(&ps, &p).is_empty());
    }

    #[test]
    fn credential_validation_cannot_be_changed_to_fail_open() {
        let (ps, mut p, _, _) = fixtures();
        p["deployment_gates"]["credential_validation_must_fail_closed"] = json!(false);
        assert!(!validate_profile(&ps, &p).is_empty());
    }

    #[test]
    fn missing_baseline_first_status_cannot_be_promoted() {
        let (ps, mut p, _, _) = fixtures();
        p["benchmarks"][3]["target_status"] = json!("proposed_target");
        assert!(!validate_profile(&ps, &p).is_empty());
    }

    #[test]
    fn weakening_cost_comparison_denominator_contract_is_rejected() {
        let (ps, mut p, _, _) = fixtures();
        p["comparison_contract"]["unsupported_and_failed_attempts_remain_in_denominator"] = json!(false);
        assert!(!validate_profile(&ps, &p).is_empty());
    }

    #[test]
    fn duplicate_benchmark_id_is_rejected() {
        let (ps, mut p, _, _) = fixtures();
        p["benchmarks"][1]["id"] = json!("WIFI_AUTH_INVALID_CERT_REJECT_001");
        assert!(!validate_profile(&ps, &p).is_empty());
    }

    #[test]
    fn removed_federation_non_claim_is_rejected() {
        let (ps, mut p, _, _) = fixtures();
        p["explicit_non_claims"].as_array_mut().unwrap().retain(|v| v.as_str() != Some("No claim of OpenRoaming federation membership"));
        assert!(!validate_profile(&ps, &p).is_empty());
    }

    #[test]
    fn remote_schema_reference_is_rejected_before_resolution() {
        let (mut ps, p, _, _) = fixtures();
        ps["properties"]["profile_id"]["$ref"] = json!("https://example.invalid/remote.json");
        assert!(!validate_profile(&ps, &p).is_empty());
    }

    #[test]
    fn blank_template_cannot_be_marked_as_evidence_matrix() {
        let (_, _, ms, mut m) = fixtures();
        m["status"] = json!("draft_evidence_matrix");
        assert!(!validate_matrix(&ms, &m).is_empty());
    }

    #[test]
    fn matrix_denominators_must_equal_raw_outcome_counts() {
        let (_, _, ms, mut m) = fixtures();
        m["status"] = json!("draft_evidence_matrix");
        m["rows"] = json!([not_tested_row()]);
        m["rows"][0]["test_run"]["attempted"] = json!(1);
        assert!(!validate_matrix(&ms, &m).is_empty());
    }

    #[test]
    fn not_tested_row_must_have_zero_attempts() {
        let (_, _, ms, mut m) = fixtures();
        m["status"] = json!("draft_evidence_matrix");
        m["rows"] = json!([not_tested_row()]);
        m["rows"][0]["test_run"]["attempted"] = json!(1);
        m["rows"][0]["test_run"]["succeeded"] = json!(1);
        assert!(!validate_matrix(&ms, &m).is_empty());
    }

    #[test]
    fn matrix_cannot_self_promote_field_roaming_without_receipt() {
        let (_, _, ms, mut m) = fixtures();
        m["status"] = json!("draft_evidence_matrix");
        m["rows"] = json!([not_tested_row()]);
        m["rows"][0]["claim_status"] = json!("supported");
        m["rows"][0]["test_run"]["tested_at"] = json!("2026-10-18T10:00:00Z");
        m["rows"][0]["test_run"]["evidence_class"] = json!("field_observed");
        m["rows"][0]["test_run"]["attempted"] = json!(1);
        m["rows"][0]["test_run"]["succeeded"] = json!(1);
        m["rows"][0]["test_run"]["receipt_path"] = Value::Null;
        assert!(!validate_matrix(&ms, &m).is_empty());
    }

    #[test]
    fn local_passpoint_test_is_not_mistaken_for_federation_enrollment() {
        let (_, _, ms, mut m) = fixtures();
        m["status"] = json!("draft_evidence_matrix");
        let mut row = not_tested_row();
        row["claim_scope"] = json!("openroaming_federated");
        row["claim_status"] = json!("supported");
        row["credential"]["credential_state"] = json!("verified_for_named_test");
        row["access_network"]["federation_state"] = json!("not_verified");
        row["test_run"]["tested_at"] = json!("2026-10-18T10:00:00Z");
        row["test_run"]["evidence_class"] = json!("bench_measured");
        row["test_run"]["attempted"] = json!(1);
        row["test_run"]["succeeded"] = json!(1);
        row["test_run"]["receipt_path"] = json!("receipts/test.json");
        row["test_run"]["receipt_sha256"] = json!("a".repeat(64));
        m["rows"] = json!([row]);
        assert!(!validate_matrix(&ms, &m).is_empty());
    }

    #[test]
    fn matrix_ids_must_be_unique() {
        let (_, _, ms, mut m) = fixtures();
        let row = not_tested_row();
        let mut second = row.clone();
        second["claim_scope"] = json!("ordinary_wifi_fallback");
        m["status"] = json!("draft_evidence_matrix");
        m["rows"] = json!([row, second]);
        assert!(!validate_matrix(&ms, &m).is_empty());
    }
}
