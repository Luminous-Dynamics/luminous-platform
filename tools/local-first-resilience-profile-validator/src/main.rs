//! Offline validation of the versioned local-first outage resilience profile.
//!
//! This validates schema and policy shape only. It is not a queue implementation,
//! cryptographic verifier, durable store, BPv7 implementation or deployment approval.

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
const PROFILE_VERSION: &str = "luminous.local-first-outage-resilience-benchmark-profile.v1";
const PROFILE_ID: &str = "local-first-outage-resilience-v1";

const REQUIRED_NON_CLAIMS: &[&str] = &[
    "No claim of guaranteed delivery or exactly-once network transport",
    "No claim that metadata validates payload encryption or authenticity",
    "No claim of BPv7/BPSec interoperability before a named implementation is tested",
    "No claim that a mesh provides global Internet without authorized upstream backhaul",
    "No claim of safe public relay, lawful resale or spectrum authorization",
    "No claim of emergency-grade telephony or emergency service",
    "No claim of field performance from schemas, unit tests, simulation or queued CI",
    "No claim of verified cache contents without source integrity evidence",
];

const REQUIRED_ARTIFACTS: &[&str] = &[
    "Strict profile JSON Schema and offline Rust validator",
    "Metadata-only Rust queue/expiry/quota reference model with negative tests",
    "Saved synthetic receipts for positive and mutated negative tests",
    "Local-only two-node VM/namespace test topology and DNS/cloud dependency negative controls",
    "BPv7/BPSec candidate inventory with license, maintenance, interoperability and threat-model review",
    "Named persistence adapter with crash-consistency tests before any durability claim",
    "Consent, abuse handling, privacy retention, owner opt-in and site authorization plan",
    "Independent review of test receipts and a staged authorized field pilot",
];

struct ExpectedBenchmark {
    id: &'static str,
    operator: &'static str,
    target: Value,
    target_status: &'static str,
}

fn expected_benchmarks() -> Vec<ExpectedBenchmark> {
    vec![
        ExpectedBenchmark { id: "LOCAL_CONTINUITY_001", operator: ">=", target: json!(24), target_status: "proposed_target" },
        ExpectedBenchmark { id: "QUEUE_BOUNDS_001", operator: "==", target: json!(0), target_status: "proposed_target" },
        ExpectedBenchmark { id: "EXPIRY_FORWARDING_001", operator: "==", target: json!(0), target_status: "proposed_target" },
        ExpectedBenchmark { id: "SENSITIVE_PROTECTION_GATE_001", operator: "==", target: json!(0), target_status: "proposed_target" },
        ExpectedBenchmark { id: "DUPLICATE_APPLICATION_EFFECT_001", operator: "==", target: json!(0), target_status: "proposed_target" },
        ExpectedBenchmark { id: "STORE_FORWARD_DELIVERY_001", operator: "within_threshold_success_ratio", target: json!({"success_fraction":0.99,"minimum_messages_per_scenario":100}), target_status: "proposed_target" },
        ExpectedBenchmark { id: "CRASH_RECOVERY_001", operator: "==", target: json!(0), target_status: "proposed_target" },
        ExpectedBenchmark { id: "QUOTA_FAIRNESS_001", operator: "==", target: json!(0), target_status: "proposed_target" },
        ExpectedBenchmark { id: "FAIRNESS_STARVATION_001", operator: "baseline_then_set_target", target: Value::Null, target_status: "baseline_first" },
        ExpectedBenchmark { id: "ENERGY_PER_USEFUL_TASK_001", operator: "baseline_then_minimize", target: Value::Null, target_status: "baseline_first" },
        ExpectedBenchmark { id: "PUBLIC_TELEMETRY_PRIVACY_001", operator: "==", target: json!(0), target_status: "proposed_target" },
        ExpectedBenchmark { id: "UNAUTHORIZED_RELAY_001", operator: "==", target: json!(0), target_status: "proposed_target" },
        ExpectedBenchmark { id: "CACHE_INTEGRITY_001", operator: "==", target: json!(0), target_status: "proposed_target" },
        ExpectedBenchmark { id: "EVIDENCE_TRACEABILITY_001", operator: "==", target: json!(1), target_status: "proposed_target" },
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

fn validate_profile(schema: &Value, profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    collect_nonlocal_references(schema, "$schema", &mut errors);
    if !errors.is_empty() {
        return errors;
    }

    let meta_validator = match jsonschema::meta::validator_for(schema) {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot build JSON Schema meta-validator: {error}")],
    };
    if !meta_validator.evaluate(schema).flag().valid {
        return vec!["schema failed JSON Schema meta-validation".to_owned()];
    }

    let validator = match draft202012::options().should_validate_formats(true).build(schema) {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot build Draft 2020-12 validator: {error}")],
    };
    errors.extend(validator.iter_errors(profile).map(|error| format!("schema: {error}")));

    if profile.get("schema_version").and_then(Value::as_str) != Some(PROFILE_VERSION) {
        errors.push(format!("schema_version must be {PROFILE_VERSION}"));
    }
    if profile.get("profile_id").and_then(Value::as_str) != Some(PROFILE_ID) {
        errors.push(format!("profile_id must be {PROFILE_ID}"));
    }
    if profile.get("status").and_then(Value::as_str) != Some("draft_targets_not_results") {
        errors.push("profile status must remain draft_targets_not_results".to_owned());
    }

    let gates = profile.get("deployment_gates");
    for (key, expected) in [
        ("bpv7_implementation_status", json!("not_selected")),
        ("bpsec_security_context_status", json!("not_selected")),
        ("end_to_end_crypto_adapter_status", json!("not_implemented")),
        ("durable_queue_adapter_status", json!("not_implemented")),
        ("upstream_relay_permission_status", json!("not_verified")),
        ("regulatory_review_status", json!("not_verified")),
        ("local_continuity_measured", json!(false)),
        ("deployment_blocked_until_gates_verified", json!(true)),
        ("local_service_must_not_require_wan", json!(true)),
        ("payload_inspection_for_routing", json!(false)),
        ("sensitive_admission_must_require_trusted_protection_verification", json!(true)),
    ] {
        if gates.and_then(|value| value.get(key)) != Some(&expected) {
            errors.push(format!("deployment_gates/{key} must remain {expected}"));
        }
    }

    let comparison = profile.get("comparison_contract");
    for key in [
        "source_code_and_config_revisions_must_be_immutable",
        "raw_trial_denominators_must_be_published",
        "failed_and_expired_messages_must_remain_in_denominator",
        "simulation_must_not_be_reported_as_field_evidence",
        "no_exactly_once_transport_claim",
        "cryptography_must_not_be_implemented_by_metadata_model",
        "costs_must_include_hardware_energy_maintenance_and_labor",
    ] {
        if comparison.and_then(|value| value.get(key)).and_then(Value::as_bool) != Some(true) {
            errors.push(format!("comparison_contract/{key} must remain true"));
        }
    }

    let actual = profile.get("benchmarks").and_then(Value::as_array);
    let expected = expected_benchmarks();
    match actual {
        Some(actual) => {
            if actual.len() != expected.len() {
                errors.push(format!("expected exactly {} benchmark records; found {}", expected.len(), actual.len()));
            }
            let mut ids = BTreeSet::new();
            for (index, benchmark) in actual.iter().enumerate() {
                match benchmark.get("id").and_then(Value::as_str) {
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
                for (field, wanted) in [
                    ("operator", json!(spec.operator)),
                    ("target", spec.target.clone()),
                    ("target_status", json!(spec.target_status)),
                ] {
                    if item.get(field) != Some(&wanted) {
                        errors.push(format!("benchmarks/{}/{} must remain {}", spec.id, field, wanted));
                    }
                }
            }
        }
        None => errors.push("benchmarks must be an array".to_owned()),
    }

    for required in REQUIRED_NON_CLAIMS {
        if !contains_text(profile.get("explicit_non_claims"), required) {
            errors.push(format!("explicit_non_claims must include {required:?}"));
        }
    }
    for required in REQUIRED_ARTIFACTS {
        if !contains_text(profile.get("next_required_artifacts"), required) {
            errors.push(format!("next_required_artifacts must include {required:?}"));
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] == "--help" || args[1] == "-h" {
        eprintln!(
            "Usage: local-first-resilience-profile-verify <schema.json> <profile.json>\n\
             Validates benchmark shape and locked draft policy only; does not prove delivery, security or authorization."
        );
        process::exit(if args.len() == 2 { 0 } else { 64 });
    }

    let schema_path = Path::new(&args[1]);
    let profile_path = Path::new(&args[2]);
    let schema_bytes = match read_bounded(schema_path) {
        Ok(bytes) => bytes,
        Err(error) => { eprintln!("{error}"); process::exit(2); }
    };
    let profile_bytes = match read_bounded(profile_path) {
        Ok(bytes) => bytes,
        Err(error) => { eprintln!("{error}"); process::exit(2); }
    };
    let schema: Value = match serde_json::from_slice(&schema_bytes) {
        Ok(value) => value,
        Err(error) => { eprintln!("schema JSON error: {error}"); process::exit(2); }
    };
    let profile: Value = match serde_json::from_slice(&profile_bytes) {
        Ok(value) => value,
        Err(error) => { eprintln!("profile JSON error: {error}"); process::exit(2); }
    };

    let errors = validate_profile(&schema, &profile);
    let valid = errors.is_empty();
    let report = json!({
        "report_version": "luminous.local-first-profile-validation-report.v1",
        "profile_id": profile.get("profile_id").and_then(Value::as_str),
        "schema_sha256": digest_hex(&schema_bytes),
        "profile_sha256": digest_hex(&profile_bytes),
        "benchmark_count": profile.get("benchmarks").and_then(Value::as_array).map_or(0, |items| items.len()),
        "valid": valid,
        "validation_scope": "json_schema_plus_locked_draft_policy",
        "delivery_truth_conveyed": false,
        "cryptographic_verification_conveyed": false,
        "deployment_authorization_conveyed": false,
        "errors": errors,
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

    fn fixtures() -> (Value, Value) {
        let schema: Value = serde_json::from_str(include_str!("../../../schemas/local-first-outage-resilience-benchmark-profile-v1.schema.json")).expect("profile schema parses");
        let profile: Value = serde_json::from_str(include_str!("../../../profiles/local-first-outage-resilience-benchmark-profile-v1.json")).expect("profile parses");
        (schema, profile)
    }

    #[test]
    fn checked_in_profile_passes_schema_and_policy() {
        let (schema, profile) = fixtures();
        let errors = validate_profile(&schema, &profile);
        assert!(errors.is_empty(), "errors: {errors:#?}");
    }

    #[test]
    fn weakening_local_continuity_target_is_rejected() {
        let (schema, mut profile) = fixtures();
        profile["benchmarks"][0]["target"] = json!(1);
        assert!(!validate_profile(&schema, &profile).is_empty());
    }

    #[test]
    fn delivery_target_cannot_be_weakened() {
        let (schema, mut profile) = fixtures();
        profile["benchmarks"][5]["target"]["success_fraction"] = json!(0.5);
        assert!(!validate_profile(&schema, &profile).is_empty());
    }

    #[test]
    fn production_status_cannot_be_self_declared() {
        let (schema, mut profile) = fixtures();
        profile["status"] = json!("production");
        assert!(!validate_profile(&schema, &profile).is_empty());
    }

    #[test]
    fn external_relay_permission_cannot_be_self_declared() {
        let (schema, mut profile) = fixtures();
        profile["deployment_gates"]["upstream_relay_permission_status"] = json!("authorized");
        profile["deployment_gates"]["deployment_blocked_until_gates_verified"] = json!(false);
        assert!(!validate_profile(&schema, &profile).is_empty());
    }

    #[test]
    fn cryptography_must_not_be_promoted_to_implemented() {
        let (schema, mut profile) = fixtures();
        profile["deployment_gates"]["end_to_end_crypto_adapter_status"] = json!("implemented");
        assert!(!validate_profile(&schema, &profile).is_empty());
    }

    #[test]
    fn payload_inspection_cannot_be_enabled() {
        let (schema, mut profile) = fixtures();
        profile["deployment_gates"]["payload_inspection_for_routing"] = json!(true);
        assert!(!validate_profile(&schema, &profile).is_empty());
    }

    #[test]
    fn removing_failure_denominators_is_rejected() {
        let (schema, mut profile) = fixtures();
        profile["comparison_contract"]["failed_and_expired_messages_must_remain_in_denominator"] = json!(false);
        assert!(!validate_profile(&schema, &profile).is_empty());
    }

    #[test]
    fn duplicate_benchmark_ids_are_rejected() {
        let (schema, mut profile) = fixtures();
        profile["benchmarks"][1]["id"] = json!("LOCAL_CONTINUITY_001");
        assert!(!validate_profile(&schema, &profile).is_empty());
    }

    #[test]
    fn baseline_first_metric_cannot_be_promoted_without_policy_change() {
        let (schema, mut profile) = fixtures();
        profile["benchmarks"][8]["target_status"] = json!("proposed_target");
        assert!(!validate_profile(&schema, &profile).is_empty());
    }

    #[test]
    fn required_non_claim_cannot_be_removed() {
        let (schema, mut profile) = fixtures();
        profile["explicit_non_claims"].as_array_mut().unwrap().retain(|item| {
            item.as_str() != Some("No claim of guaranteed delivery or exactly-once network transport")
        });
        assert!(!validate_profile(&schema, &profile).is_empty());
    }

    #[test]
    fn remote_schema_references_are_rejected_without_resolution() {
        let (mut schema, profile) = fixtures();
        schema["properties"]["profile_id"]["$ref"] = json!("https://example.invalid/remote.json");
        assert!(!validate_profile(&schema, &profile).is_empty());
    }

    #[test]
    fn missing_required_artifact_is_rejected() {
        let (schema, mut profile) = fixtures();
        profile["next_required_artifacts"].as_array_mut().unwrap().retain(|item| {
            item.as_str() != Some("Strict profile JSON Schema and offline Rust validator")
        });
        assert!(!validate_profile(&schema, &profile).is_empty());
    }
}
