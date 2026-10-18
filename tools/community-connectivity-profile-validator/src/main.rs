//! Offline, fail-closed validation for the community connectivity pilot profile.
//!
//! This validates document structure and draft policy invariants only. It does not
//! establish that field measurements are true, hardware is qualified, a provider
//! permits resale, or a deployment is legally authorized.

use jsonschema::draft202012;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process;

const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;
const EXPECTED_PROFILE_ID: &str = "community-connectivity-basic-access-v1";
const REQUIRED_NON_CLAIMS: &[&str] = &[
    "hardware qualification",
    "regulatory authorization",
    "Starlink service availability",
    "provider permission to share or resell",
    "production readiness",
    "performance guarantee",
];

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

/// Reject remote references before constructing a validator. This avoids
/// accidental HTTP/file retrieval through untrusted schema references.
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
                        None => errors.push(format!(
                            "{path}/{key}: reference must be a string"
                        )),
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

fn policy_errors(profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if profile.get("profile_id").and_then(Value::as_str) != Some(EXPECTED_PROFILE_ID) {
        errors.push(format!("profile_id must be {EXPECTED_PROFILE_ID}"));
    }
    if profile.get("status").and_then(Value::as_str) != Some("draft_research_only") {
        errors.push("profile status must remain draft_research_only".to_owned());
    }
    if profile.get("source_revalidation_required").and_then(Value::as_bool) != Some(true) {
        errors.push("source_revalidation_required must remain true".to_owned());
    }

    let claims = profile
        .get("claims_not_conveyed")
        .and_then(Value::as_array);
    for required in REQUIRED_NON_CLAIMS {
        let present = claims
            .map(|items| items.iter().any(|item| item.as_str() == Some(required)))
            .unwrap_or(false);
        if !present {
            errors.push(format!(
                "claims_not_conveyed must include {required:?}"
            ));
        }
    }

    let topology = profile.get("topology");
    if topology
        .and_then(|value| value.get("local_services_must_not_require_wan"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        errors.push("local essential services must not require WAN access".to_owned());
    }
    if topology
        .and_then(|value| value.get("cognitive_components_must_not_be_required_for_packet_forwarding"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        errors.push("cognitive components must not be required for packet forwarding".to_owned());
    }

    let starlink = profile
        .get("wan_adapters")
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("id").and_then(Value::as_str) == Some("starlink-ethernet-wan-v1"))
        });
    match starlink {
        None => errors.push("Starlink WAN adapter contract is missing".to_owned()),
        Some(adapter) => {
            for field in [
                "local_service_authorization_status",
                "service_plan_sharing_permission_status",
            ] {
                if adapter.get(field).and_then(Value::as_str) != Some("not_verified") {
                    errors.push(format!(
                        "Starlink {field} must remain not_verified until separate evidence is recorded"
                    ));
                }
            }
            if adapter
                .get("deployment_blocked_until_authorization_verified")
                .and_then(Value::as_bool)
                != Some(true)
            {
                errors.push(
                    "Starlink deployment must remain blocked until authorization is verified"
                        .to_owned(),
                );
            }
            if adapter
                .get("vendor_private_api_required")
                .and_then(Value::as_bool)
                != Some(false)
            {
                errors.push("normal WAN operation must not require a private vendor API".to_owned());
            }
        }
    }

    let benchmarks = profile.get("benchmarks").and_then(Value::as_array);
    match benchmarks {
        Some(items) if items.len() == 10 => {
            for (index, item) in items.iter().enumerate() {
                if item.get("status").and_then(Value::as_str) != Some("proposed_target") {
                    errors.push(format!(
                        "benchmarks/{index}/status must remain proposed_target until measured"
                    ));
                }
            }
        }
        Some(items) => errors.push(format!(
            "expected exactly 10 versioned benchmark hypotheses; found {}",
            items.len()
        )),
        None => errors.push("benchmarks must be an array".to_owned()),
    }

    errors
}

fn validate_values(schema: &Value, profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    collect_nonlocal_references(schema, "$", &mut errors);
    if !errors.is_empty() {
        // Crucially, do not construct a validator against a schema with a
        // remote reference: invalid input must never trigger retrieval.
        return errors;
    }

    let meta_validator = match jsonschema::meta::validator_for(schema) {
        Ok(validator) => validator,
        Err(error) => {
            errors.push(format!("cannot build bundled JSON Schema meta-validator: {error}"));
            return errors;
        }
    };
    let meta_evaluation = meta_validator.evaluate(schema);
    if !meta_evaluation.flag().valid {
        errors.push("schema failed JSON Schema meta-schema validation".to_owned());
        return errors;
    }

    let validator = match draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => validator,
        Err(error) => {
            errors.push(format!("cannot build Draft 2020-12 profile validator: {error}"));
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
            "Usage: community-connectivity-profile-verify <schema.json> <profile.json>\n\
             Validates local JSON Schema structure and draft policy only; it does not\n\
             qualify hardware, prove measurements, authorize radio use, or grant provider permission."
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
        "report_version": "luminous.community-connectivity-validation-report.v1",
        "profile_id": profile.get("profile_id").and_then(Value::as_str),
        "schema_sha256": digest_hex(&schema_bytes),
        "profile_sha256": digest_hex(&profile_bytes),
        "valid": valid,
        "validation_scope": "json_schema_plus_draft_policy",
        "deployment_authorization_conveyed": false,
        "hardware_qualification_conveyed": false,
        "errors": errors,
    });
    let mut stdout = std::io::stdout().lock();
    if let Err(error) = serde_json::to_writer_pretty(&mut stdout, &report)
        .and_then(|_| stdout.write_all(b"\n").map_err(serde_json::Error::io))
    {
        eprintln!("cannot write validation report: {error}");
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

    fn fixture() -> (Value, Value) {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/community-connectivity-profile-v1.schema.json"
        ))
        .expect("checked-in schema must parse as JSON");
        let profile: Value = serde_json::from_str(include_str!(
            "../../../profiles/community-connectivity-reference-v1.json"
        ))
        .expect("checked-in profile must parse as JSON");
        (schema, profile)
    }

    #[test]
    fn checked_in_profile_passes_schema_and_policy() {
        let (schema, profile) = fixture();
        let errors = validate_values(&schema, &profile);
        assert!(errors.is_empty(), "validation errors: {errors:#?}");
    }

    #[test]
    fn production_status_is_rejected() {
        let (schema, mut profile) = fixture();
        profile["status"] = json!("production");
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn starlink_cannot_be_marked_authorized_by_profile_edit() {
        let (schema, mut profile) = fixture();
        profile["wan_adapters"][1]["local_service_authorization_status"] =
            json!("authorized");
        profile["wan_adapters"][1]["service_plan_sharing_permission_status"] =
            json!("authorized");
        profile["wan_adapters"][1]["deployment_blocked_until_authorization_verified"] =
            json!(false);
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn benchmark_targets_cannot_be_silently_weakened() {
        let (schema, mut profile) = fixture();
        profile["benchmarks"][0]["target"] = json!(0.50);
        assert!(!validate_values(&schema, &profile).is_empty());
    }

    #[test]
    fn external_schema_references_are_rejected_before_resolution() {
        let (mut schema, profile) = fixture();
        schema["properties"]["profile_id"]["$ref"] =
            json!("https://example.invalid/remote-schema.json");
        let errors = validate_values(&schema, &profile);
        assert!(errors.iter().any(|error| error.contains("only local fragment references")));
    }

    #[test]
    fn missing_authorization_non_claim_is_rejected() {
        let (schema, mut profile) = fixture();
        profile["claims_not_conveyed"] = json!([
            "hardware qualification",
            "regulatory authorization",
            "production readiness",
            "performance guarantee",
            "Starlink service availability"
        ]);
        assert!(!validate_values(&schema, &profile).is_empty());
    }
}
