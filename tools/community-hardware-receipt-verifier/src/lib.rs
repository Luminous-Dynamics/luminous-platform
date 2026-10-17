//! Offline semantic and JSON Schema verifier for community hardware receipts.
//!
//! This verifier validates a submitted record. It never declares hardware qualified,
//! grants deployment authority, executes commands, or retrieves evidence over a network.

use jsonschema::draft202012;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_SCHEMA_BYTES: usize = 4 * 1024 * 1024;
const MAX_RECEIPT_BYTES: usize = 16 * 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const PHYSICAL_CLAIM_TOKENS: &[&str] = &[
    "thermal", "electrical", "radio", "rf-", "acoustic", "physical-boot",
    "power-consumption", "power-draw", "temperature-soak",
];

/// Machine-readable diagnostic that avoids echoing untrusted receipt values.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub path: String,
    pub message: String,
}

/// Local evidence byte-verification summary.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ArtifactVerification {
    /// `not_requested` or `local_digest_files`; remote retrieval is never attempted.
    pub mode: String,
    pub checked: usize,
    pub skipped_non_public: usize,
    pub missing: usize,
    pub mismatched: usize,
}

/// Result of schema and semantic receipt validation. This is not a hardware qualification.
#[derive(Clone, Debug, Serialize)]
pub struct ValidationReport {
    pub report_version: &'static str,
    pub valid: bool,
    pub schema_sha256: String,
    pub receipt_sha256: String,
    pub schema_meta_valid: bool,
    pub schema_validated: bool,
    pub schema_errors: Vec<Diagnostic>,
    pub semantic_errors: Vec<Diagnostic>,
    pub artifact_verification: ArtifactVerification,
    pub non_claims: Vec<&'static str>,
}

/// Validate schema bytes and receipt bytes. If `artifact_root` is supplied, public artifacts
/// must exist as regular, non-symlink files named exactly by their lowercase SHA-256 digest.
///
/// No URL is fetched; no host state is inspected; no command or device action is executed.
pub fn verify_bytes(
    schema_bytes: &[u8],
    receipt_bytes: &[u8],
    artifact_root: Option<&Path>,
) -> ValidationReport {
    let schema_sha256 = sha256_hex(schema_bytes);
    let receipt_sha256 = sha256_hex(receipt_bytes);
    let mut schema_errors = Vec::new();
    let mut semantic_errors = Vec::new();
    let mut schema_meta_valid = false;
    let mut schema_validated = false;
    let mut artifact_verification = ArtifactVerification {
        mode: if artifact_root.is_some() {
            "local_digest_files".to_owned()
        } else {
            "not_requested".to_owned()
        },
        checked: 0,
        skipped_non_public: 0,
        missing: 0,
        mismatched: 0,
    };

    if schema_bytes.len() > MAX_SCHEMA_BYTES {
        push(
            &mut schema_errors,
            "schema_too_large",
            "$",
            "Schema exceeds the configured 4 MiB input limit.",
        );
    }
    if receipt_bytes.len() > MAX_RECEIPT_BYTES {
        push(
            &mut schema_errors,
            "receipt_too_large",
            "$",
            "Receipt exceeds the configured 16 MiB input limit.",
        );
    }

    let schema: Option<Value> = if schema_bytes.len() <= MAX_SCHEMA_BYTES {
        match serde_json::from_slice(schema_bytes) {
            Ok(value) => Some(value),
            Err(_) => {
                push(
                    &mut schema_errors,
                    "schema_json_invalid",
                    "$",
                    "Schema is not valid JSON; raw source is not included in this report.",
                );
                None
            }
        }
    } else {
        None
    };

    let receipt: Option<Value> = if receipt_bytes.len() <= MAX_RECEIPT_BYTES {
        match serde_json::from_slice(receipt_bytes) {
            Ok(value) => Some(value),
            Err(_) => {
                push(
                    &mut schema_errors,
                    "receipt_json_invalid",
                    "$",
                    "Receipt is not valid JSON; raw source is not included in this report.",
                );
                None
            }
        }
    } else {
        None
    };

    if let Some(schema_value) = schema.as_ref() {
        match draft202012::meta::validate(schema_value) {
            Ok(()) => schema_meta_valid = true,
            Err(_) => push(
                &mut schema_errors,
                "schema_meta_invalid",
                "$",
                "Schema does not validate against the JSON Schema Draft 2020-12 meta-schema.",
            ),
        }

        reject_remote_references(schema_value, "$", &mut schema_errors);

        if schema_meta_valid && schema_errors.is_empty() {
            match draft202012::options()
                .should_validate_formats(true)
                .build(schema_value)
            {
                Ok(validator) => {
                    if let Some(receipt_value) = receipt.as_ref() {
                        for error in validator.iter_errors(receipt_value) {
                            push(
                                &mut schema_errors,
                                "receipt_schema_mismatch",
                                &error.instance_path().to_string(),
                                "Receipt value does not satisfy the schema at this instance path.",
                            );
                        }
                        schema_validated = schema_errors.is_empty();
                    }
                }
                Err(_) => push(
                    &mut schema_errors,
                    "schema_compile_failed",
                    "$",
                    "Schema could not be compiled by the Draft 2020-12 validator.",
                ),
            }
        }
    }

    if let Some(receipt_value) = receipt.as_ref() {
        if schema_validated {
            validate_semantics(receipt_value, &mut semantic_errors);
            if let Some(root) = artifact_root {
                verify_local_artifacts(
                    receipt_value,
                    root,
                    &mut artifact_verification,
                    &mut semantic_errors,
                );
            }
        }
    }

    let valid = schema_validated && schema_errors.is_empty() && semantic_errors.is_empty();

    ValidationReport {
        report_version: "1.0.0",
        valid,
        schema_sha256,
        receipt_sha256,
        schema_meta_valid,
        schema_validated,
        schema_errors,
        semantic_errors,
        artifact_verification,
        non_claims: vec![
            "A valid receipt is not proof that the test was honestly executed.",
            "A signature or digest binds an issuer or byte sequence; it does not prove the physical observation is true.",
            "This verifier never promotes hardware maturity or grants deployment authorization.",
            "A VM or simulated result cannot establish physical thermals, electrical/RF behavior, or physical boot behavior.",
            "No remote artifact retrieval, host mutation, firmware flashing, or device actuation is performed.",
        ],
    }
}

fn validate_semantics(receipt: &Value, errors: &mut Vec<Diagnostic>) {
    let record_status = string_at(receipt, "/record_status").unwrap_or_default();
    let disposition = string_at(receipt, "/overall_disposition").unwrap_or_default();
    let evidence_class = string_at(receipt, "/evidence_class").unwrap_or_default();
    let execution_mode = string_at(receipt, "/environment/execution_mode").unwrap_or_default();
    let deployment_authorization =
        string_at(receipt, "/deployment_authorization").unwrap_or_default();
    let maturity_promotion = string_at(receipt, "/maturity_promotion").unwrap_or_default();

    if deployment_authorization != "not_conveyed" {
        push(errors, "authorization_promotion_forbidden", "/deployment_authorization",
            "A test receipt cannot grant deployment authorization.");
    }
    if maturity_promotion != "none" {
        push(errors, "maturity_promotion_forbidden", "/maturity_promotion",
            "A receipt cannot promote its own hardware maturity.");
    }

    if record_status == "synthetic_example" {
        if disposition != "incomplete" {
            push(errors, "synthetic_must_remain_incomplete", "/overall_disposition",
                "A synthetic example must remain incomplete and cannot claim a real campaign pass.");
        }
        if evidence_class != "vm_simulated" || execution_mode != "vm_simulated" {
            push(errors, "synthetic_scope_mismatch", "/evidence_class",
                "This synthetic fixture profile must be labelled vm_simulated.");
        }
    }

    if record_status == "community_submission"
        && string_at(receipt, "/subject/unit_identity_method") == Some("synthetic_fixture")
    {
        push(errors, "synthetic_identity_in_submission", "/subject/unit_identity_method",
            "A community submission cannot identify a synthetic fixture as a real device.");
    }

    let allowed_modes: &[&str] = match evidence_class {
        "designed" | "static_checked" => &["static"],
        "unit_contract_tested" => &["unit_test"],
        "vm_simulated" => &["vm_simulated"],
        "bench_measured" => &["physical_bench", "independent_lab"],
        "field_observed" => &["field_observation", "independent_lab"],
        "independently_reviewed_qualified" => {
            if !array_is_nonempty(receipt, "/review_refs") {
                push(errors, "independent_review_missing", "/review_refs",
                    "Independently reviewed evidence requires explicit review references.");
            }
            &["physical_bench", "field_observation", "independent_lab"]
        }
        _ => &[],
    };

    if !allowed_modes.contains(&execution_mode) {
        push(errors, "evidence_mode_mismatch", "/environment/execution_mode",
            "Evidence class is inconsistent with the declared execution mode.");
    }

    if disposition == "pass" {
        if record_status == "synthetic_example" {
            push(errors, "synthetic_pass_forbidden", "/overall_disposition",
                "Synthetic examples cannot claim an overall pass.");
        }
        let results = array_at(receipt, "/results").unwrap_or(&[]);
        for (index, result) in results.iter().enumerate() {
            let path = format!("/results/{index}");
            let status = result.get("status").and_then(Value::as_str).unwrap_or_default();
            if status != "pass" {
                push(errors, "pass_contains_nonpassing_test", &format!("{path}/status"),
                    "Overall pass requires every listed test result to be pass.");
            }
            if result.get("evidence_refs").and_then(Value::as_array).is_none_or(Vec::is_empty) {
                push(errors, "pass_missing_test_evidence", &format!("{path}/evidence_refs"),
                    "Each passing result must reference at least one evidence artifact digest.");
            }

            let test_id = result.get("test_id").and_then(Value::as_str).unwrap_or_default().to_ascii_lowercase();
            if execution_mode == "vm_simulated"
                && PHYSICAL_CLAIM_TOKENS.iter().any(|token| test_id.contains(token))
            {
                push(errors, "vm_cannot_prove_physical_claim", &format!("{path}/status"),
                    "A VM/simulator result cannot pass a physical-behavior claim.");
            }
        }
        if record_status != "community_submission" && record_status != "synthetic_example" {
            push(errors, "unknown_record_status", "/record_status",
                "Receipt status is not recognized for a pass claim.");
        }
        if record_status == "community_submission"
            && string_at(receipt, "/subject/unit_identity_method") == Some("unknown")
        {
            push(errors, "unknown_subject_identity", "/subject/unit_identity_method",
                "A passing community submission requires an explicit device identity method.");
        }
        if matches!(evidence_class, "bench_measured" | "field_observed") {
            let measured = array_at(receipt, "/results").is_some_and(|results| {
                results.iter().any(|r| r.get("measurements").and_then(Value::as_array).is_some_and(|m| !m.is_empty()))
            });
            if !measured {
                push(errors, "measured_evidence_without_measurements", "/results",
                    "Bench/field pass requires at least one reported measurement.");
            }
        }
    }

    validate_disposition_consistency(receipt, disposition, errors);
    validate_references(receipt, errors);
    validate_calibration(receipt, disposition, evidence_class, errors);
}

fn validate_disposition_consistency(receipt: &Value, disposition: &str, errors: &mut Vec<Diagnostic>) {
    let results = array_at(receipt, "/results").unwrap_or(&[]);
    let has_status = |wanted: &str| results.iter().any(|r| r.get("status").and_then(Value::as_str) == Some(wanted));
    match disposition {
        "fail" if !has_status("fail") && !has_status("unsafe_to_continue") => push(
            errors, "failure_disposition_without_failed_test", "/overall_disposition",
            "Overall fail requires at least one failed or unsafe-to-continue test.",
        ),
        "unsafe_to_continue" if !has_status("unsafe_to_continue") => push(
            errors, "unsafe_disposition_without_unsafe_test", "/overall_disposition",
            "unsafe_to_continue requires at least one result marked unsafe_to_continue.",
        ),
        "indeterminate" if !has_status("indeterminate") => push(
            errors, "indeterminate_disposition_without_unknown_test", "/overall_disposition",
            "Overall indeterminate requires an indeterminate test result.",
        ),
        "incomplete" if !has_status("not_run") && !has_status("skipped") => push(
            errors, "incomplete_without_unrun_test", "/overall_disposition",
            "The listed results must identify at least one skipped or not-run test for an incomplete campaign.",
        ),
        _ => {}
    }
}

fn validate_references(receipt: &Value, errors: &mut Vec<Diagnostic>) {
    let artifacts = array_at(receipt, "/evidence_artifacts").unwrap_or(&[]);
    let mut known = BTreeSet::new();
    for (index, artifact) in artifacts.iter().enumerate() {
        let digest = artifact.get("digest_sha256").and_then(Value::as_str).unwrap_or_default();
        if !known.insert(digest.to_owned()) {
            push(errors, "duplicate_artifact_digest", &format!("/evidence_artifacts/{index}/digest_sha256"),
                "Evidence artifact digests must be unique within a receipt.");
        }
        if artifact.get("visibility").and_then(Value::as_str) == Some("public") {
            let safe_redaction = matches!(
                artifact.get("redaction_state").and_then(Value::as_str),
                Some("reviewed_redacted" | "not_sensitive")
            );
            if !safe_redaction {
                push(errors, "unsafe_public_artifact", &format!("/evidence_artifacts/{index}/redaction_state"),
                    "Public evidence must be reviewed-redacted or explicitly not sensitive.");
            }
        }
    }

    let results = array_at(receipt, "/results").unwrap_or(&[]);
    for (result_index, result) in results.iter().enumerate() {
        if let Some(refs) = result.get("evidence_refs").and_then(Value::as_array) {
            for (ref_index, reference) in refs.iter().enumerate() {
                let digest = reference.as_str().unwrap_or_default();
                if !known.contains(digest) {
                    push(errors, "unknown_evidence_reference",
                        &format!("/results/{result_index}/evidence_refs/{ref_index}"),
                        "Test result references a digest not present in evidence_artifacts.");
                }
            }
        }
    }
}

fn validate_calibration(receipt: &Value, disposition: &str, evidence_class: &str, errors: &mut Vec<Diagnostic>) {
    let instruments = array_at(receipt, "/environment/instrumentation").unwrap_or(&[]);
    let mut by_ref = BTreeMap::new();
    for instrument in instruments {
        if let (Some(id), Some(calibration)) = (
            instrument.get("instrument_ref").and_then(Value::as_str),
            instrument.get("calibration_state").and_then(Value::as_str),
        ) {
            by_ref.insert(id.to_owned(), (calibration, instrument.get("calibration_evidence_digest").and_then(Value::as_str)));
        }
    }

    let results = array_at(receipt, "/results").unwrap_or(&[]);
    for (ri, result) in results.iter().enumerate() {
        for (mi, measurement) in array_at_value(result, "measurements").iter().enumerate() {
            let path = format!("/results/{ri}/measurements/{mi}");
            let instrument_ref = measurement.get("instrument_ref").and_then(Value::as_str);
            let calibration = measurement.get("calibration_state").and_then(Value::as_str).unwrap_or("unknown");

            if let Some(instrument_ref) = instrument_ref {
                match by_ref.get(instrument_ref) {
                    None => push(errors, "measurement_instrument_missing", &format!("{path}/instrument_ref"),
                        "Measurement instrument must match an environment instrumentation record."),
                    Some((instrument_state, digest)) => {
                        if *instrument_state != calibration {
                            push(errors, "calibration_state_mismatch", &format!("{path}/calibration_state"),
                                "Measurement and instrument calibration states must agree.");
                        }
                        if *instrument_state == "current" && digest.is_none_or(|d| !is_sha256(d)) {
                            push(errors, "calibration_evidence_missing", "/environment/instrumentation",
                                "Current calibration must reference a SHA-256 calibration evidence artifact.");
                        }
                    }
                }
            }

            if matches!(evidence_class, "bench_measured" | "field_observed" | "independently_reviewed_qualified")
                && disposition == "pass"
                && matches!(calibration, "expired" | "unknown")
            {
                push(errors, "passing_measurement_calibration_unknown", &format!("{path}/calibration_state"),
                    "A passing measured claim cannot depend on expired or unknown instrument calibration.");
            }
        }
    }
}

fn verify_local_artifacts(
    receipt: &Value,
    root: &Path,
    summary: &mut ArtifactVerification,
    errors: &mut Vec<Diagnostic>,
) {
    let canonical_root = match root.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            push(errors, "artifact_root_unavailable", "$",
                "Artifact root cannot be resolved as a local directory.");
            return;
        }
    };
    if !canonical_root.is_dir() {
        push(errors, "artifact_root_not_directory", "$",
            "Artifact root must be a directory.");
        return;
    }

    for (index, artifact) in array_at(receipt, "/evidence_artifacts").unwrap_or(&[]).iter().enumerate() {
        let visibility = artifact.get("visibility").and_then(Value::as_str).unwrap_or_default();
        if visibility != "public" {
            summary.skipped_non_public += 1;
            continue;
        }
        let digest = artifact.get("digest_sha256").and_then(Value::as_str).unwrap_or_default();
        if !is_sha256(digest) {
            push(errors, "invalid_artifact_digest", &format!("/evidence_artifacts/{index}/digest_sha256"),
                "Local artifact filename must be a lowercase SHA-256 digest.");
            summary.mismatched += 1;
            continue;
        }
        let candidate: PathBuf = canonical_root.join(digest);
        let metadata = match fs::symlink_metadata(&candidate) {
            Ok(metadata) => metadata,
            Err(_) => {
                summary.missing += 1;
                push(errors, "public_artifact_missing", &format!("/evidence_artifacts/{index}"),
                    "Public artifact is not available as a digest-named local file.");
                continue;
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            summary.mismatched += 1;
            push(errors, "artifact_not_regular_file", &format!("/evidence_artifacts/{index}"),
                "Artifact must be a regular non-symlink file.");
            continue;
        }
        if metadata.len() > MAX_ARTIFACT_BYTES {
            summary.mismatched += 1;
            push(errors, "artifact_too_large", &format!("/evidence_artifacts/{index}"),
                "Artifact exceeds the configured 64 MiB per-file limit.");
            continue;
        }
        let canonical_file = match candidate.canonicalize() {
            Ok(path) if path.starts_with(&canonical_root) => path,
            _ => {
                summary.mismatched += 1;
                push(errors, "artifact_path_escape", &format!("/evidence_artifacts/{index}"),
                    "Artifact resolves outside the selected artifact root.");
                continue;
            }
        };
        let bytes = match fs::read(canonical_file) {
            Ok(bytes) => bytes,
            Err(_) => {
                summary.missing += 1;
                push(errors, "artifact_unreadable", &format!("/evidence_artifacts/{index}"),
                    "Artifact bytes could not be read.");
                continue;
            }
        };
        summary.checked += 1;
        if sha256_hex(&bytes) != digest {
            summary.mismatched += 1;
            push(errors, "artifact_digest_mismatch", &format!("/evidence_artifacts/{index}/digest_sha256"),
                "Local artifact bytes do not match the declared SHA-256 digest.");
        }
    }
}

fn reject_remote_references(value: &Value, path: &str, errors: &mut Vec<Diagnostic>) {
    match value {
        Value::Object(map) => {
            for key in ["$ref", "$dynamicRef"] {
                if let Some(reference) = map.get(key) {
                    match reference.as_str() {
                        Some(reference) if reference.starts_with('#') => {}
                        _ => push(errors, "remote_schema_reference_forbidden", path,
                            "Offline verifier accepts only local fragment schema references."),
                    }
                }
            }
            for (key, child) in map {
                let escaped = key.replace('~', "~0").replace('/', "~1");
                reject_remote_references(child, &format!("{path}/{escaped}"), errors);
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

fn array_at<'a>(value: &'a Value, pointer: &str) -> Option<&'a Vec<Value>> {
    value.pointer(pointer).and_then(Value::as_array)
}

fn array_at_value(value: &Value, key: &str) -> &[Value] {
    value.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn array_is_nonempty(value: &Value, pointer: &str) -> bool {
    array_at(value, pointer).is_some_and(|items| !items.is_empty())
}

fn string_at<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn push(errors: &mut Vec<Diagnostic>, code: &str, path: &str, message: &str) {
    errors.push(Diagnostic {
        code: code.to_owned(),
        path: path.to_owned(),
        message: message.to_owned(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    const SCHEMA: &[u8] = include_bytes!("../../../schemas/community-hardware-test-receipt-v1.schema.json");
    const FIXTURE: &[u8] = include_bytes!("../../../docs/fixtures/community-hardware-test-receipt-v1.json");

    fn parsed_fixture() -> Value {
        serde_json::from_slice(FIXTURE).expect("checked-in fixture is valid JSON")
    }

    fn checked(receipt: &Value) -> ValidationReport {
        verify_bytes(SCHEMA, &serde_json::to_vec(receipt).expect("serialize fixture"), None)
    }

    fn has_code(report: &ValidationReport, code: &str) -> bool {
        report.schema_errors.iter().chain(report.semantic_errors.iter()).any(|d| d.code == code)
    }

    #[test]
    fn synthetic_incomplete_fixture_validates_without_promoting_hardware() {
        let report = verify_bytes(SCHEMA, FIXTURE, None);
        assert!(report.valid, "unexpected diagnostics: {report:?}");
        assert_eq!(report.schema_meta_valid, true);
        assert_eq!(report.artifact_verification.mode, "not_requested");
    }

    #[test]
    fn overall_pass_cannot_include_not_run_test_or_synthetic_example() {
        let mut fixture = parsed_fixture();
        fixture["overall_disposition"] = json!("pass");
        let report = checked(&fixture);
        assert!(!report.valid);
        assert!(has_code(&report, "synthetic_must_remain_incomplete"));
        assert!(has_code(&report, "pass_contains_nonpassing_test"));
    }

    #[test]
    fn test_cannot_reference_unlisted_evidence_digest() {
        let mut fixture = parsed_fixture();
        fixture["results"][0]["evidence_refs"][0] = json!("a".repeat(64));
        let report = checked(&fixture);
        assert!(!report.valid);
        assert!(has_code(&report, "unknown_evidence_reference"));
    }

    #[test]
    fn vm_cannot_pass_a_physical_thermal_claim() {
        let mut fixture = parsed_fixture();
        fixture["overall_disposition"] = json!("pass");
        fixture["record_status"] = json!("community_submission");
        fixture["subject"]["unit_identity_method"] = json!("label_and_visual_inspection");
        fixture["campaign"]["suite_source_revision"] = json!("0123456789abcdef0123456789abcdef01234567");
        fixture["campaign"]["runner_source_revision"] = json!("fedcba9876543210fedcba9876543210fedcba98");
        fixture["results"][0]["status"] = json!("pass");
        fixture["results"][1]["status"] = json!("pass");
        fixture["results"][1]["evidence_refs"] = json!(["ec343cc7a760e3ba328ff381409b91858ef77a4e6c09fa2f8c2de74401271563"]);
        let report = checked(&fixture);
        assert!(!report.valid);
        assert!(has_code(&report, "vm_cannot_prove_physical_claim"));
        assert!(has_code(&report, "evidence_mode_mismatch"));
    }

    #[test]
    fn community_submission_cannot_use_synthetic_subject_identity() {
        let mut fixture = parsed_fixture();
        fixture["record_status"] = json!("community_submission");
        let report = checked(&fixture);
        assert!(!report.valid);
        assert!(has_code(&report, "synthetic_identity_in_submission"));
    }

    #[test]
    fn schema_remote_references_are_rejected_for_offline_safety() {
        let mut schema: Value = serde_json::from_slice(SCHEMA).expect("schema JSON");
        schema["properties"]["$ref"] = json!("https://example.invalid/remote.json");
        let receipt = parsed_fixture();
        let report = verify_bytes(&serde_json::to_vec(&schema).unwrap(), &serde_json::to_vec(&receipt).unwrap(), None);
        assert!(!report.valid);
        assert!(has_code(&report, "remote_schema_reference_forbidden"));
    }

    #[test]
    fn public_artifact_bytes_are_checked_by_digest_when_root_is_supplied() {
        let mut fixture = parsed_fixture();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock after epoch").as_nanos();
        let root = std::env::temp_dir().join(format!("luminous-hw-receipt-{}-{now}", std::process::id()));
        fs::create_dir_all(&root).expect("create test artifact root");
        let bytes = b"synthetic artifact bytes for digest verification";
        let digest = sha256_hex(bytes);
        fs::write(root.join(&digest), bytes).expect("write digest-named artifact");
        fixture["evidence_artifacts"][0]["digest_sha256"] = json!(digest);
        fixture["results"][0]["evidence_refs"][0] = json!(digest);

        let report = verify_bytes(SCHEMA, &serde_json::to_vec(&fixture).expect("serialize fixture"), Some(&root));
        assert!(report.valid, "unexpected diagnostics: {report:?}");
        assert_eq!(report.artifact_verification.checked, 1);
        assert_eq!(report.artifact_verification.missing, 0);
        fs::remove_dir_all(root).expect("cleanup test artifact root");
    }
}
