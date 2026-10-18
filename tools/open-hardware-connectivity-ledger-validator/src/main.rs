//! Offline, fail-closed validator for the open hardware connectivity ledger.
//!
//! Schema validation and policy invariants guard against self-promotion. This
//! tool does not prove design-file provenance/licensing, physical operation,
//! type approval, service permission or actual hardware openness.

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
const SCHEMA_VERSION: &str = "luminous.open-hardware-connectivity-candidate-ledger.v1";
const PROFILE_ID: &str = "open-hardware-connectivity-candidate-ledger-2026-10-18";

const EXPECTED_CANDIDATES: &[&str] = &[
    "GLINET_GL_MT6000_FLINT2_GATEWAY_CANDIDATE",
    "BANANAPI_BPI_R4_HARDWARE_FILE_CANDIDATE",
];

const REQUIRED_NEXT_ARTIFACTS: &[&str] = &[
    "Immutable exact model/revision inventory for every selected pilot component",
    "Reviewed licence/file inventory for board designs, firmware, drivers, calibration and manufacturing files",
    "Pinned firmware image and source/configuration digests",
    "Clean install, failed-update rollback and recovery receipts on each exact physical revision",
    "Measured idle/typical/peak power, thermal behaviour, WAN failover and local-only continuity",
    "Exact AP/client compatibility matrix and regional radio compliance evidence",
    "Provider sharing/resale and applicable ICASA/equipment-approval review",
    "Independent review before any qualified or fully-open claim",
];

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

fn reject_remote_references(value: &Value, path: &str, errors: &mut Vec<String>) {
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
                reject_remote_references(child, &format!("{path}/{key}"), errors);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                reject_remote_references(child, &format!("{path}/{index}"), errors);
            }
        }
        _ => {}
    }
}

fn contains_text(value: Option<&Value>, expected: &str) -> bool {
    value.and_then(Value::as_array).is_some_and(|items| {
        items.iter().any(|item| item.as_str() == Some(expected))
    })
}

fn policy_errors(profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if profile.get("schema_version").and_then(Value::as_str) != Some(SCHEMA_VERSION) {
        errors.push(format!("schema_version must remain {SCHEMA_VERSION}"));
    }
    if profile.get("profile_id").and_then(Value::as_str) != Some(PROFILE_ID) {
        errors.push(format!("profile_id must remain {PROFILE_ID}"));
    }
    if profile.get("status").and_then(Value::as_str) != Some("draft_research_only") {
        errors.push("profile status must remain draft_research_only".to_owned());
    }

    let gates = profile.get("deployment_gates");
    for (key, expected) in [
        ("public_site_deployment_authorized", json!(false)),
        ("south_africa_type_approval_status", json!("not_verified")),
        ("upstream_sharing_permission_status", json!("not_verified")),
        ("spectrum_permission_status", json!("not_verified")),
        ("installation_safety_review_status", json!("not_verified")),
        ("live_radio_measurement_available", json!(false)),
    ] {
        if gates.and_then(|value| value.get(key)) != Some(&expected) {
            errors.push(format!("deployment_gates/{key} must remain {expected}"));
        }
    }

    let claim_contract = profile.get("claim_contract");
    for (key, expected) in [
        ("overall_open_hardware_claim", json!(false)),
        ("firmware_availability_alone_proves_hardware_open", json!(false)),
        ("design_file_link_alone_proves_design_license", json!(false)),
        ("openwrt_support_alone_proves_open_radio_stack", json!(false)),
        ("ci_pass_alone_proves_field_qualification", json!(false)),
        ("starlink_ip_compatibility_proves_terminal_open", json!(false)),
        ("unknown_critical_dimension_blocks_full_open_claim", json!(true)),
    ] {
        if claim_contract.and_then(|value| value.get(key)) != Some(&expected) {
            errors.push(format!("claim_contract/{key} must remain {expected}"));
        }
    }

    match profile.get("candidate_profiles").and_then(Value::as_array) {
        Some(candidates) => {
            if candidates.len() != EXPECTED_CANDIDATES.len() {
                errors.push(format!("expected exactly {} versioned candidates", EXPECTED_CANDIDATES.len()));
            }
            let mut seen = BTreeSet::new();
            for (index, candidate) in candidates.iter().enumerate() {
                match candidate.get("candidate_id").and_then(Value::as_str) {
                    Some(id) if !seen.insert(id.to_owned()) => errors.push(format!("candidate_profiles/{index}/candidate_id duplicates {id}")),
                    Some(_) => {}
                    None => errors.push(format!("candidate_profiles/{index}/candidate_id must be a string")),
                }
                if candidate.get("qualification_status").and_then(Value::as_str) != Some("design_candidate_only") {
                    errors.push(format!("candidate_profiles/{index}/qualification_status must remain design_candidate_only"));
                }
                if candidate.get("critical_artifacts").and_then(Value::as_array).is_some_and(|artifacts| {
                    artifacts.iter().any(|artifact| {
                        artifact.get("artifact_id").and_then(Value::as_str) == Some("south_africa_equipment_approval")
                            && artifact.get("status").and_then(Value::as_str) != Some("not_verified")
                    })
                }) {
                    errors.push(format!("candidate_profiles/{index}: South African approval must remain not_verified until separate evidence is recorded"));
                }
            }
            for expected in EXPECTED_CANDIDATES {
                if seen.get(*expected).is_none() {
                    errors.push(format!("required candidate {expected} is missing"));
                }
            }
        }
        None => errors.push("candidate_profiles must be an array".to_owned()),
    }

    for required in REQUIRED_NEXT_ARTIFACTS {
        if !contains_text(profile.get("next_required_artifacts"), required) {
            errors.push(format!("next_required_artifacts must include {required:?}"));
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

fn validate_values(schema: &Value, profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    reject_remote_references(schema, "$schema", &mut errors);
    if !errors.is_empty() {
        return errors;
    }

    let meta = match jsonschema::meta::validator_for(schema) {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot build JSON Schema meta-validator: {error}")],
    };
    if !meta.evaluate(schema).flag().valid {
        return vec!["candidate ledger schema failed JSON Schema meta-validation".to_owned()];
    }

    let validator = match draft202012::options().should_validate_formats(true).build(schema) {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot build Draft 2020-12 validator: {error}")],
    };
    errors.extend(validator.iter_errors(profile).map(|error| format!("schema: {error}")));
    errors.extend(policy_errors(profile));
    errors.sort();
    errors.dedup();
    errors
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] == "--help" || args[1] == "-h" {
        eprintln!(
            "Usage: open-hardware-connectivity-ledger-verify <schema.json> <profile.json>\n\
             Validates ledger shape and draft claim gates only; no design licence, hardware operation or authorization is proved."
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

    let errors = validate_values(&schema, &profile);
    let valid = errors.is_empty();
    let report = json!({
        "report_version": "luminous.open-hardware-ledger-validation-report.v1",
        "profile_id": profile.get("profile_id").and_then(Value::as_str),
        "schema_sha256": digest_hex(&schema_bytes),
        "profile_sha256": digest_hex(&profile_bytes),
        "candidate_count": profile.get("candidate_profiles").and_then(Value::as_array).map_or(0, |items| items.len()),
        "valid": valid,
        "validation_scope": "json_schema_plus_locked_draft_claim_policy",
        "hardware_openness_conveyed": false,
        "device_qualification_conveyed": false,
        "deployment_authorization_conveyed": false,
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

    fn fixtures() -> (Value, Value) {
        let schema: Value = serde_json::from_str(include_str!("../../../schemas/open-hardware-connectivity-candidate-ledger-v1.schema.json")).expect("ledger schema parses");
        let profile: Value = serde_json::from_str(include_str!("../../../profiles/open-hardware-connectivity-candidate-ledger-v1.json")).expect("ledger profile parses");
        (schema, profile)
    }

    #[test]
    fn checked_in_ledger_passes_schema_and_policy() {
        let (schema, profile) = fixtures();
        let errors = validate_values(&schema, &profile);
        assert!(errors.is_empty(), "ledger errors: {errors:#?}");
    }

    #[test]
    fn overall_fully_open_claim_cannot_be_self_declared() {
        let (schema, mut profile) = fixtures();
        profile["claim_contract"]["overall_open_hardware_claim"] = json!(true);
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn firmware_availability_does_not_prove_open_hardware() {
        let (schema, mut profile) = fixtures();
        profile["claim_contract"]["firmware_availability_alone_proves_hardware_open"] = json!(true);
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn candidate_cannot_self_promote_to_qualified() {
        let (schema, mut profile) = fixtures();
        profile["candidate_profiles"][0]["qualification_status"] = json!("field_qualified");
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn type_approval_cannot_be_self_declared() {
        let (schema, mut profile) = fixtures();
        profile["deployment_gates"]["south_africa_type_approval_status"] = json!("verified");
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn candidate_revision_and_recovery_claims_remain_unverified() {
        let (schema, mut profile) = fixtures();
        profile["candidate_profiles"][0]["critical_artifacts"][5]["status"] = json!("physically_tested");
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn unknown_radio_firmware_cannot_be_marked_verified_without_contract_edit() {
        let (schema, mut profile) = fixtures();
        profile["candidate_profiles"][0]["critical_artifacts"][4]["status"] = json!("fully_open");
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn dropping_candidate_artifact_evidence_is_rejected() {
        let (schema, mut profile) = fixtures();
        profile["candidate_profiles"][0]["critical_artifacts"].as_array_mut().unwrap().pop();
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn missing_candidate_is_rejected() {
        let (schema, mut profile) = fixtures();
        profile["candidate_profiles"].as_array_mut().unwrap().pop();
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn remote_schema_references_are_rejected_without_resolution() {
        let (mut schema, profile) = fixtures();
        schema["properties"]["profile_id"]["$ref"] = json!("https://example.invalid/remote.json");
        assert!(!validate_values(&schema, &profile).is_empty());
    }
}
