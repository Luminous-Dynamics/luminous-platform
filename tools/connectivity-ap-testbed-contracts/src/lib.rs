//! Offline contract validation for physical Wi-Fi testbed manifests.
//!
//! A valid manifest describes a test configuration. It is not evidence that a
//! radio measurement happened, a task succeeded, or a device is certified.

#![forbid(unsafe_code)]

use jsonschema::draft202012;
use serde_json::Value;

const MAX_JSON_BYTES: usize = 1_048_576;
const PLANNED_STAGE: &str = "planned";

fn reject_remote_references(value: &Value, path: &str, errors: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if key == "$ref" || key == "$dynamicRef" {
                    match child.as_str() {
                        Some(reference) if reference.starts_with('#') => {}
                        Some(_) => errors.push(format!(
                            "{path}/{key}: only local JSON Schema references are allowed"
                        )),
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

fn parse_document(source: &str, label: &str) -> Result<Value, Vec<String>> {
    if source.len() > MAX_JSON_BYTES {
        return Err(vec![format!(
            "{label}: input exceeds {MAX_JSON_BYTES} bytes"
        )]);
    }
    serde_json::from_str(source).map_err(|error| vec![format!("{label}: invalid JSON: {error}")])
}

fn non_null_at(value: &Value, pointer: &str) -> bool {
    value.pointer(pointer).is_some_and(|item| !item.is_null())
}

fn non_empty_string_at(value: &Value, pointer: &str) -> bool {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .is_some_and(|text| !text.trim().is_empty())
}

fn stage_policy(manifest: &Value, errors: &mut Vec<String>) {
    let stage = manifest
        .get("evidence_stage")
        .and_then(Value::as_str)
        .unwrap_or("");
    if stage == PLANNED_STAGE {
        return;
    }

    for pointer in [
        "/access_point/hardware_revision",
        "/access_point/firmware_version",
        "/access_point/firmware_sha256",
        "/access_point/radio_config_sha256",
        "/run_controls/test_profile_sha256",
        "/software/harness_revision",
        "/software/profile_sha256",
        "/wired_peer/link_speed_mbps",
        "/wired_peer/nic_driver",
    ] {
        if !non_null_at(manifest, pointer) {
            errors.push(format!("{pointer} is required for measured evidence"));
        }
    }

    if manifest.pointer("/run_controls/test_profile_sha256")
        != manifest.pointer("/software/profile_sha256")
    {
        errors.push(
            "run_controls.test_profile_sha256 must match software.profile_sha256".to_owned(),
        );
    }

    for pointer in [
        "/access_point/vendor_model",
        "/wired_peer/device_model",
        "/wired_peer/nic_model",
        "/software/host_os",
    ] {
        if !non_empty_string_at(manifest, pointer) {
            errors.push(format!("{pointer} must be a recorded non-empty value"));
        }
    }

    let clients = manifest.get("clients").and_then(Value::as_array);
    if clients.is_none_or(|items| items.len() < 3) {
        errors.push("measured evidence requires at least three declared Wi-Fi clients".to_owned());
    }
    if let Some(clients) = clients {
        for (index, client) in clients.iter().enumerate() {
            for key in [
                "hardware_revision",
                "kernel_release",
                "wifi_driver",
                "wifi_firmware_revision",
            ] {
                if client.get(key).is_none_or(Value::is_null) {
                    errors.push(format!(
                        "/clients/{index}/{key} must be recorded for measured evidence; use a documented explicit value if the device does not expose it"
                    ));
                }
            }
        }
    }

    for pointer in [
        "/instrumentation/iperf3_version",
        "/instrumentation/ping_tool_version",
        "/instrumentation/tc_version",
    ] {
        if !non_null_at(manifest, pointer) {
            errors.push(format!("{pointer} must be recorded for measured evidence"));
        }
    }

    for pointer in [
        "/instrumentation/station_retry_stats",
        "/instrumentation/airtime_stats",
        "/instrumentation/qdisc_stats",
    ] {
        if manifest.pointer(pointer).and_then(Value::as_str) == Some("not_attempted") {
            errors.push(format!(
                "{pointer} must be checked and set to available or unavailable; unsupported is not zero"
            ));
        }
    }

    let power = manifest.pointer("/instrumentation/power_meter");
    if power
        .and_then(|item| item.get("available"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        errors.push("measured evidence requires an available power meter".to_owned());
    }
    for key in [
        "meter_model",
        "calibration_date",
        "sampling_hz",
        "resolution_mw",
    ] {
        if power
            .and_then(|item| item.get(key))
            .is_none_or(Value::is_null)
        {
            errors.push(format!(
                "/instrumentation/power_meter/{key} is required for measured energy claims"
            ));
        }
    }

    if manifest
        .pointer("/run_controls/network_authorization")
        .and_then(Value::as_str)
        != Some("owner_authorized_lab")
    {
        errors.push("measured runs require explicit owner_authorized_lab permission".to_owned());
    }
    if !matches!(
        manifest
            .pointer("/run_controls/pair_order")
            .and_then(Value::as_str),
        Some("randomized" | "blocked_randomized")
    ) {
        errors.push(
            "measured comparisons require randomized or blocked_randomized pair order".to_owned(),
        );
    }

    if matches!(stage, "field_pilot" | "reliability_qualification") {
        if manifest
            .pointer("/run_controls/interference_protocol")
            .and_then(Value::as_str)
            == Some("uncontrolled_declared")
        {
            errors.push(
                "field evidence requires a documented interference-control protocol".to_owned(),
            );
        }
        if !non_null_at(manifest, "/run_controls/field_site_token")
            || !non_null_at(manifest, "/run_controls/consent_record_sha256")
        {
            errors.push(
                "field evidence requires a pseudonymous site token and consent-record digest"
                    .to_owned(),
            );
        }
    }
}

/// Validate a testbed manifest against a supplied local Draft 2020-12 schema
/// and conservative evidence-stage rules.
pub fn validate_manifest(schema_source: &str, manifest_source: &str) -> Result<Value, Vec<String>> {
    let schema = parse_document(schema_source, "schema")?;
    let manifest = parse_document(manifest_source, "manifest")?;
    let mut errors = Vec::new();

    reject_remote_references(&schema, "$", &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }

    let meta = jsonschema::meta::validator_for(&schema)
        .map_err(|error| vec![format!("cannot build schema meta-validator: {error}")])?;
    if !meta.evaluate(&schema).flag().valid {
        return Err(vec!["schema failed JSON Schema meta-validation".to_owned()]);
    }

    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| vec![format!("cannot build Draft 2020-12 validator: {error}")])?;
    errors.extend(
        validator
            .iter_errors(&manifest)
            .map(|error| format!("schema: {error}")),
    );
    if !errors.is_empty() {
        return Err(errors);
    }

    stage_policy(&manifest, &mut errors);
    if errors.is_empty() {
        Ok(manifest)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_JSON_BYTES, validate_manifest};
    use serde_json::Value;

    const SCHEMA: &str =
        include_str!("../../../schemas/connectivity-ap-testbed-manifest-v1.schema.json");
    const PLANNED: &str =
        include_str!("../../../examples/connectivity-ap-testbed-manifest-planned-v1.json");

    #[test]
    fn planned_configuration_is_valid_without_claiming_measurements() {
        let manifest =
            validate_manifest(SCHEMA, PLANNED).expect("planned manifest should validate");
        assert_eq!(manifest["evidence_stage"], "planned");
        assert_eq!(manifest["privacy_contract"]["payloads_exported"], false);
    }

    #[test]
    fn unknown_fields_fail_closed() {
        let mut manifest: Value = serde_json::from_str(PLANNED).expect("fixture JSON");
        manifest["access_point"]["mac_address"] = Value::String("must-not-be-added".to_owned());
        let error = validate_manifest(SCHEMA, &manifest.to_string()).expect_err("unknown field");
        assert!(error.iter().any(|message| message.contains("mac_address")));
    }

    #[test]
    fn measured_stage_cannot_promote_planned_unknowns() {
        let mut manifest: Value = serde_json::from_str(PLANNED).expect("fixture JSON");
        manifest["evidence_stage"] = Value::String("lab_screening".to_owned());
        manifest["clients"].as_array_mut().expect("client array").truncate(2);
        let error = validate_manifest(SCHEMA, &manifest.to_string())
            .expect_err("measured evidence must have recorded instrumentation and hashes");
        assert!(
            error
                .iter()
                .any(|message| message.contains("firmware_sha256"))
        );
        assert!(
            error
                .iter()
                .any(|message| message.contains("three declared Wi-Fi clients"))
        );
        assert!(error.iter().any(|message| message.contains("power meter")));
        assert!(error.iter().any(|message| message.contains("randomized")));
    }

    #[test]
    fn runtime_profile_digest_must_match_test_profile_digest() {
        let mut manifest: Value = serde_json::from_str(PLANNED).expect("fixture JSON");
        manifest["evidence_stage"] = Value::String("bench_smoke".to_owned());
        manifest["run_controls"]["test_profile_sha256"] =
            Value::String("a".repeat(64));
        manifest["software"]["profile_sha256"] = Value::String("b".repeat(64));
        let error = validate_manifest(SCHEMA, &manifest.to_string())
            .expect_err("conflicting profile digests must fail");
        assert!(
            error
                .iter()
                .any(|message| message.contains("must match software.profile_sha256"))
        );
    }

    #[test]
    fn unsupported_counters_must_be_explicitly_checked() {
        let mut manifest: Value = serde_json::from_str(PLANNED).expect("fixture JSON");
        manifest["evidence_stage"] = Value::String("bench_smoke".to_owned());
        manifest["instrumentation"]["airtime_stats"] = Value::String("not_attempted".to_owned());
        let error = validate_manifest(SCHEMA, &manifest.to_string()).expect_err("stage gate");
        assert!(
            error
                .iter()
                .any(|message| message.contains("airtime_stats"))
        );
    }

    #[test]
    fn remote_schema_references_are_rejected() {
        let mut schema: Value = serde_json::from_str(SCHEMA).expect("schema JSON");
        schema["properties"]["manifest_id"] =
            serde_json::json!({ "$ref": "https://example.invalid/remote-schema.json" });
        let error =
            validate_manifest(&schema.to_string(), PLANNED).expect_err("remote ref must fail");
        assert!(
            error
                .iter()
                .any(|message| message.contains("only local JSON Schema references"))
        );
    }

    #[test]
    fn oversized_documents_are_rejected() {
        let oversized = " ".repeat(MAX_JSON_BYTES + 1);
        assert!(validate_manifest(SCHEMA, &oversized).is_err());
    }

    #[test]
    fn field_stage_requires_site_and_consent_evidence() {
        let mut manifest: Value = serde_json::from_str(PLANNED).expect("fixture JSON");
        manifest["evidence_stage"] = Value::String("field_pilot".to_owned());
        let error =
            validate_manifest(SCHEMA, &manifest.to_string()).expect_err("field evidence gate");
        assert!(
            error
                .iter()
                .any(|message| message.contains("site token and consent-record digest"))
        );
    }
}
