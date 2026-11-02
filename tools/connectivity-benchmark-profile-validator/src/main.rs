//! Offline validation of the provider-neutral connectivity benchmark profile.
//!
//! This checks versioned structure and policy invariants only. It does not
//! validate measurement truth, hardware, radio conditions, or legal/provider rights.

use jsonschema::draft202012;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process;

const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;
const EXPECTED_PROFILE_ID: &str = "provider-neutral-connectivity-broker-v1";
const EXPECTED_SCHEMA_VERSION: &str = "luminous.connectivity-broker-benchmark-profile.v1";

const REQUIRED_NON_CLAIMS: &[&str] = &[
    "No claim of Google Fi coverage parity",
    "No claim of operator or eSIM provisioning access without an agreement",
    "No claim of authorized service resale",
    "No claim that simulation proves field performance",
    "No claim of satellite phone support without exact device/service/country evidence",
    "No claim that a successful policy decision itself changed a live route",
];

const REQUIRED_ARTIFACTS: &[&str] = &[
    "Strict JSON Schema for this profile",
    "Independent Rust validator with negative mutation controls",
    "Rust-enabled cargo test, cargo clippy, reviewed Cargo.lock and exact commit evidence",
    "Pinned provider-neutral lab fixture set",
    "OpenWrt adapter acceptance profile",
    "Provider/device/country matrix for any eSIM or satellite claim",
];

struct ExpectedBenchmark {
    id: &'static str,
    domain: &'static str,
    metric: &'static str,
    operator: &'static str,
    target_status: &'static str,
    target: Value,
}

fn expected_benchmarks() -> Vec<ExpectedBenchmark> {
    vec![
        ExpectedBenchmark {
            id: "POLICY_HARD_CONSTRAINTS_001",
            domain: "decision_core",
            metric: "hard_constraint_violations",
            operator: "==",
            target_status: "proposed_target",
            target: json!(0),
        },
        ExpectedBenchmark {
            id: "POLICY_EXPLANATION_001",
            domain: "decision_core",
            metric: "decisions_with_selected_path_or_explicit_no_route_and_per_candidate_reasons",
            operator: "==",
            target_status: "proposed_target",
            target: json!(1),
        },
        ExpectedBenchmark {
            id: "POLICY_REPLAY_001",
            domain: "decision_core",
            metric: "same_inputs_producing_identical_decision_structure",
            operator: "==",
            target_status: "proposed_target",
            target: json!(1),
        },
        ExpectedBenchmark {
            id: "POLICY_DATA_MINIMIZATION_001",
            domain: "decision_core",
            metric: "decisions_requiring_packet_or_message_content_or_browsing_history",
            operator: "==",
            target_status: "proposed_target",
            target: json!(0),
        },
        ExpectedBenchmark {
            id: "COST_PER_USEFUL_TASK_001",
            domain: "economics",
            metric: "fully_burdened_cost_per_successful_task_reduction_vs_matched_baseline",
            operator: ">=",
            target_status: "proposed_target",
            target: json!(0.3),
        },
        ExpectedBenchmark {
            id: "GATEWAY_FAILOVER_001",
            domain: "multi_wan_gateway",
            metric: "primary_link_failure_trials_restoring_new_eligible_sessions_within_seconds",
            operator: "within_threshold_success_ratio",
            target_status: "proposed_target",
            target: json!({
                "threshold_seconds": 30,
                "successful_fraction": 0.99,
                "minimum_trials": 100
            }),
        },
        ExpectedBenchmark {
            id: "LOCAL_CONTINUITY_001",
            domain: "local_first",
            metric: "hours_essential_local_services_work_with_all_wan_links_disconnected",
            operator: ">=",
            target_status: "proposed_target",
            target: json!(24),
        },
        ExpectedBenchmark {
            id: "SERVICE_AVAILABILITY_001",
            domain: "service_quality",
            metric: "monthly_service_availability",
            operator: ">=",
            target_status: "proposed_target",
            target: json!(0.995),
        },
        ExpectedBenchmark {
            id: "WIFI_ROAMING_BASELINE_001",
            domain: "wifi_roaming",
            metric: "arrival_to_usable_authenticated_ip_connectivity",
            operator: "baseline_then_set_target",
            target_status: "baseline_first",
            target: Value::Null,
        },
        ExpectedBenchmark {
            id: "VOICE_VIDEO_RESILIENCE_001",
            domain: "voice_video",
            metric: "connection_quality_and_interruption_under_link_failure",
            operator: "baseline_then_set_target",
            target_status: "baseline_first",
            target: Value::Null,
        },
        ExpectedBenchmark {
            id: "STORE_FORWARD_001",
            domain: "resilient_messaging",
            metric: "unexpired_test_messages_delivered_after_connectivity_returns",
            operator: ">=",
            target_status: "proposed_target",
            target: json!(0.99),
        },
        ExpectedBenchmark {
            id: "ENERGY_PER_USEFUL_TASK_001",
            domain: "energy",
            metric: "whole_site_energy_per_completed_useful_task",
            operator: "baseline_then_minimize",
            target_status: "baseline_first",
            target: Value::Null,
        },
        ExpectedBenchmark {
            id: "CRITICAL_ARTIFACT_TRACEABILITY_001",
            domain: "open_hardware",
            metric: "critical_components_with_revision_license_firmware_and_bom_status_recorded",
            operator: "==",
            target_status: "proposed_target",
            target: json!(1),
        },
        ExpectedBenchmark {
            id: "PRIVATE_CELLULAR_QUALIFICATION_001",
            domain: "private_lte_5g",
            metric: "measured_coverage_capacity_mobility_and_recovery_for_named_authorized_profile",
            operator: "baseline_then_set_target",
            target_status: "blocked_on_authorization_and_profile",
            target: Value::Null,
        },
        ExpectedBenchmark {
            id: "ESIM_PORTABILITY_001",
            domain: "cellular_esim",
            metric: "successful_authorized_profile_activation_and_rollback_by_supported_device_provider_country",
            operator: "baseline_then_set_target",
            target_status: "blocked_on_provider_agreement_and_entitlement",
            target: Value::Null,
        },
        ExpectedBenchmark {
            id: "SATELLITE_PHONE_SERVICE_001",
            domain: "direct_to_phone_satellite",
            metric: "supported_device_country_service_and_sky_visibility_success_rate",
            operator: "baseline_then_set_target",
            target_status: "blocked_on_provider_device_and_country_support",
            target: Value::Null,
        },
    ]
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let file = File::open(path)
        .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut limited = file.take((MAX_INPUT_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    limited
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(format!(
            "{} exceeds the {} byte input limit",
            path.display(),
            MAX_INPUT_BYTES
        ));
    }
    Ok(bytes)
}

fn digest_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Reject remote schema references before constructing a validator so validation
/// never causes retrieval of untrusted HTTP, HTTPS, or file resources.
fn collect_nonlocal_references(value: &Value, path: &str, errors: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if key == "$ref" || key == "$dynamicRef" {
                    match child.as_str() {
                        Some(reference) if reference.starts_with('#') => {}
                        Some(_) => errors.push(format!(
                            "{path}/{key}: only local fragment references are allowed"
                        )),
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

fn contains_string(value: Option<&Value>, required: &str) -> bool {
    value
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(required)))
}

fn policy_errors(profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if profile.get("schema_version").and_then(Value::as_str) != Some(EXPECTED_SCHEMA_VERSION) {
        errors.push(format!("schema_version must be {EXPECTED_SCHEMA_VERSION}"));
    }
    if profile.get("profile_id").and_then(Value::as_str) != Some(EXPECTED_PROFILE_ID) {
        errors.push(format!("profile_id must be {EXPECTED_PROFILE_ID}"));
    }
    if profile.get("status").and_then(Value::as_str) != Some("draft_targets_not_results") {
        errors.push("profile status must remain draft_targets_not_results".to_owned());
    }

    let contract = profile.get("comparison_contract");
    for key in [
        "baseline_must_be_named",
        "matched_quality_security_and_geographic_scope_required",
        "include_hardware_energy_transit_paid_labor_support_and_taxes_where_applicable",
        "show_failed_trials_and_denominators",
        "simulation_must_not_be_reported_as_field_evidence",
        "cost_currency_must_be_explicit",
        "source_and_test_revisions_must_be_immutable",
    ] {
        if contract.and_then(|value| value.get(key)).and_then(Value::as_bool) != Some(true) {
            errors.push(format!("comparison_contract/{key} must remain true"));
        }
    }

    let items = profile.get("benchmarks").and_then(Value::as_array);
    let expected = expected_benchmarks();
    match items {
        None => errors.push("benchmarks must be an array".to_owned()),
        Some(actual) => {
            if actual.len() != expected.len() {
                errors.push(format!(
                    "expected exactly {} benchmark records, found {}",
                    expected.len(),
                    actual.len()
                ));
            }
            for spec in &expected {
                let matches: Vec<&Value> = actual
                    .iter()
                    .filter(|item| item.get("id").and_then(Value::as_str) == Some(spec.id))
                    .collect();
                if matches.len() != 1 {
                    errors.push(format!(
                        "benchmark {} must occur exactly once; found {}",
                        spec.id,
                        matches.len()
                    ));
                    continue;
                }
                let item = matches[0];
                for (field, wanted) in [
                    ("domain", json!(spec.domain)),
                    ("metric", json!(spec.metric)),
                    ("operator", json!(spec.operator)),
                    ("target_status", json!(spec.target_status)),
                    ("target", spec.target.clone()),
                ] {
                    if item.get(field) != Some(&wanted) {
                        errors.push(format!(
                            "benchmarks/{}/{} must remain {}",
                            spec.id, field, wanted
                        ));
                    }
                }
            }

            let mut seen = std::collections::BTreeSet::new();
            for (index, item) in actual.iter().enumerate() {
                match item.get("id").and_then(Value::as_str) {
                    Some(id) if !seen.insert(id) => {
                        errors.push(format!("benchmarks/{index}/id duplicates {id}"));
                    }
                    Some(_) => {}
                    None => errors.push(format!("benchmarks/{index}/id must be a string")),
                }
            }
        }
    }

    for required in REQUIRED_NON_CLAIMS {
        if !contains_string(profile.get("explicit_non_claims"), required) {
            errors.push(format!("explicit_non_claims must include {required:?}"));
        }
    }
    for required in REQUIRED_ARTIFACTS {
        if !contains_string(profile.get("next_required_artifacts"), required) {
            errors.push(format!("next_required_artifacts must include {required:?}"));
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

fn validate_values(schema: &Value, profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    collect_nonlocal_references(schema, "$", &mut errors);
    if !errors.is_empty() {
        return errors;
    }

    let meta_validator = match jsonschema::meta::validator_for(schema) {
        Ok(validator) => validator,
        Err(error) => {
            errors.push(format!("cannot build JSON Schema meta-validator: {error}"));
            return errors;
        }
    };
    if !meta_validator.evaluate(schema).flag().valid {
        errors.push("schema failed JSON Schema meta-schema validation".to_owned());
        return errors;
    }

    let validator = match draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => validator,
        Err(error) => {
            errors.push(format!("cannot build Draft 2020-12 validator: {error}"));
            return errors;
        }
    };

    errors.extend(
        validator
            .iter_errors(profile)
            .map(|error| format!("schema: {error}")),
    );
    errors.extend(policy_errors(profile));
    errors.sort();
    errors.dedup();
    errors
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] == "--help" || args[1] == "-h" {
        eprintln!(
            "Usage: connectivity-benchmark-profile-verify <schema.json> <profile.json>\n             Validates schema and draft benchmark policy only; it does not prove measurements,\n             production readiness, hardware qualification, or deployment authorization."
        );
        process::exit(if args.len() == 2 { 0 } else { 64 });
    }

    let schema_path = Path::new(&args[1]);
    let profile_path = Path::new(&args[2]);
    let schema_bytes = match read_bounded(schema_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    let profile_bytes = match read_bounded(profile_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    let schema: Value = match serde_json::from_slice(&schema_bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("schema is not valid JSON: {error}");
            process::exit(2);
        }
    };
    let profile: Value = match serde_json::from_slice(&profile_bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("profile is not valid JSON: {error}");
            process::exit(2);
        }
    };

    let errors = validate_values(&schema, &profile);
    let valid = errors.is_empty();
    let report = json!({
        "report_version": "luminous.connectivity-benchmark-validation-report.v1",
        "profile_id": profile.get("profile_id").and_then(Value::as_str),
        "schema_sha256": digest_hex(&schema_bytes),
        "profile_sha256": digest_hex(&profile_bytes),
        "valid": valid,
        "validation_scope": "json_schema_plus_versioned_draft_policy",
        "measurement_truth_conveyed": false,
        "production_readiness_conveyed": false,
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
    if !valid {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixtures() -> (Value, Value) {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/connectivity-broker-benchmark-profile-v1.schema.json"
        ))
        .expect("checked-in schema must parse as JSON");
        let profile: Value = serde_json::from_str(include_str!(
            "../../../profiles/connectivity-broker-benchmark-profile-v1.json"
        ))
        .expect("checked-in profile must parse as JSON");
        (schema, profile)
    }

    #[test]
    fn checked_in_profile_passes_schema_and_policy() {
        let (schema, profile) = fixtures();
        let errors = validate_values(&schema, &profile);
        assert!(errors.is_empty(), "validation errors: {errors:#?}");
    }

    #[test]
    fn weakening_economic_target_is_rejected() {
        let (schema, mut profile) = fixtures();
        profile["benchmarks"][4]["target"] = json!(0.05);
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn weakening_hard_constraint_target_is_rejected() {
        let (schema, mut profile) = fixtures();
        profile["benchmarks"][0]["target"] = json!(1);
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn measured_or_production_status_cannot_be_self_declared() {
        let (schema, mut profile) = fixtures();
        profile["status"] = json!("production_and_measured");
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn comparison_contract_cannot_drop_failed_trials() {
        let (schema, mut profile) = fixtures();
        profile["comparison_contract"]["show_failed_trials_and_denominators"] = json!(false);
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn duplicated_benchmark_identity_is_rejected() {
        let (schema, mut profile) = fixtures();
        profile["benchmarks"][1]["id"] = json!("POLICY_HARD_CONSTRAINTS_001");
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn required_non_claim_cannot_be_removed() {
        let (schema, mut profile) = fixtures();
        let claims = profile["explicit_non_claims"].as_array_mut().unwrap();
        claims.retain(|item| item.as_str() != Some("No claim of authorized service resale"));
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn external_schema_references_are_rejected_without_resolution() {
        let (mut schema, profile) = fixtures();
        schema["properties"]["profile_id"]["$ref"] =
            json!("https://example.invalid/remote-schema.json");
        let errors = validate_values(&schema, &profile);
        assert!(errors.iter().any(|error| error.contains("only local fragment references")));
    }

    #[test]
    fn missing_benchmark_is_rejected() {
        let (schema, mut profile) = fixtures();
        profile["benchmarks"].as_array_mut().unwrap().pop();
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn blocked_capabilities_cannot_be_given_targets_without_policy_change() {
        let (schema, mut profile) = fixtures();
        profile["benchmarks"][13]["target"] = json!(1);
        profile["benchmarks"][13]["target_status"] = json!("proposed_target");
        assert!(!validate_values(&schema, &profile).is_empty());
    }
}
