//! Offline validation of exact connectivity benchmark measurement receipts.
//!
//! A passing receipt proves only structural conformity and matching local artifact
//! bytes. It does not prove that the trials happened as described or that a
//! provider/network deployment was authorized.

use jsonschema::draft202012;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process;

const MAX_SCHEMA_BYTES: usize = 2 * 1024 * 1024;
const MAX_RECEIPT_BYTES: usize = 8 * 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const RECEIPT_VERSION: &str = "luminous.connectivity-measurement-receipt.v1";
const PROFILE_VERSION: &str = "luminous.connectivity-broker-benchmark-profile.v1";

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let file = File::open(path)
        .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut limited = file.take((limit + 1) as u64);
    let mut bytes = Vec::new();
    limited
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.len() > limit {
        return Err(format!("{} exceeds the {limit} byte input limit", path.display()));
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

fn array_contains_text(value: Option<&Value>, required: &str) -> bool {
    value
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(required)))
}

fn validate_contract(schema: &Value, receipt: &Value, profile: &Value, profile_sha: &str) -> Vec<String> {
    let mut errors = Vec::new();
    collect_nonlocal_references(schema, "$schema", &mut errors);
    if !errors.is_empty() {
        return errors;
    }

    let meta_validator = match jsonschema::meta::validator_for(schema) {
        Ok(validator) => validator,
        Err(error) => {
            return vec![format!("cannot build JSON Schema meta-validator: {error}")];
        }
    };
    if !meta_validator.evaluate(schema).flag().valid {
        return vec!["measurement receipt schema failed JSON Schema meta-validation".to_owned()];
    }

    let validator = match draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot build Draft 2020-12 validator: {error}")],
    };
    errors.extend(
        validator
            .iter_errors(receipt)
            .map(|error| format!("schema: {error}")),
    );

    if receipt.get("receipt_version").and_then(Value::as_str) != Some(RECEIPT_VERSION) {
        errors.push(format!("receipt_version must remain {RECEIPT_VERSION}"));
    }
    if receipt.get("status").and_then(Value::as_str) != Some("draft_unreviewed") {
        errors.push("receipt status must remain draft_unreviewed".to_owned());
    }
    if profile.get("schema_version").and_then(Value::as_str) != Some(PROFILE_VERSION) {
        errors.push(format!("benchmark profile schema_version must be {PROFILE_VERSION}"));
    }
    if profile.get("status").and_then(Value::as_str) != Some("draft_targets_not_results") {
        errors.push("benchmark profile must remain draft_targets_not_results".to_owned());
    }
    if receipt.get("benchmark_profile_sha256").and_then(Value::as_str) != Some(profile_sha) {
        errors.push("benchmark_profile_sha256 does not match the exact profile file bytes".to_owned());
    }

    let benchmark_id = receipt.get("benchmark_id").and_then(Value::as_str);
    let profile_benchmarks = profile.get("benchmarks").and_then(Value::as_array);
    let matching: Vec<&Value> = profile_benchmarks
        .into_iter()
        .flatten()
        .filter(|benchmark| benchmark.get("id").and_then(Value::as_str) == benchmark_id)
        .collect();

    if matching.len() != 1 {
        errors.push("benchmark_id must match exactly one record in the bound benchmark profile".to_owned());
    } else {
        let benchmark = matching[0];
        for (receipt_field, profile_field) in [
            ("target_operator", "operator"),
            ("target", "target"),
            ("target_status", "target_status"),
        ] {
            if receipt.get(receipt_field) != benchmark.get(profile_field) {
                errors.push(format!(
                    "{receipt_field} differs from the exact benchmark profile record"
                ));
            }
        }
    }

    let evidence_class = receipt.get("evidence_class").and_then(Value::as_str);
    let execution_environment = receipt.get("execution_environment").and_then(Value::as_str);
    if evidence_class != execution_environment {
        errors.push("evidence_class must exactly match execution_environment".to_owned());
    }

    let trials = receipt.get("trials").and_then(Value::as_array);
    let summary = receipt.get("summary");
    if let (Some(trials), Some(summary)) = (trials, summary) {
        let attempted = summary.get("attempted").and_then(Value::as_u64);
        let succeeded = summary.get("succeeded").and_then(Value::as_u64);
        let failed = summary.get("failed").and_then(Value::as_u64);
        let inconclusive = summary.get("inconclusive").and_then(Value::as_u64);

        if attempted != Some(trials.len() as u64) {
            errors.push("summary.attempted must equal the number of raw trial records".to_owned());
        }

        let mut ids = BTreeSet::new();
        let mut counts = [0_u64; 3];
        for (index, trial) in trials.iter().enumerate() {
            match trial.get("trial_id").and_then(Value::as_str) {
                Some(id) if !ids.insert(id.to_owned()) => {
                    errors.push(format!("trials/{index}/trial_id duplicates {id}"));
                }
                Some(_) => {}
                None => {}
            }
            match trial.get("outcome").and_then(Value::as_str) {
                Some("succeeded") => counts[0] += 1,
                Some("failed") => counts[1] += 1,
                Some("inconclusive") => counts[2] += 1,
                _ => {}
            }
            if trial.get("outcome").and_then(Value::as_str) == Some("succeeded")
                && trial.get("failure_code").is_some_and(|value| !value.is_null())
            {
                errors.push(format!("trials/{index}: succeeded outcome cannot have failure_code"));
            }
            if trial.get("outcome").and_then(Value::as_str) == Some("failed")
                && trial.get("failure_code").is_none_or(Value::is_null)
            {
                errors.push(format!("trials/{index}: failed outcome must include failure_code"));
            }
        }

        if succeeded != Some(counts[0]) {
            errors.push("summary.succeeded must match raw trial outcomes".to_owned());
        }
        if failed != Some(counts[1]) {
            errors.push("summary.failed must match raw trial outcomes".to_owned());
        }
        if inconclusive != Some(counts[2]) {
            errors.push("summary.inconclusive must match raw trial outcomes".to_owned());
        }
        if attempted != succeeded.map(|v| v + failed.unwrap_or(0) + inconclusive.unwrap_or(0)) {
            errors.push("attempted must equal succeeded + failed + inconclusive".to_owned());
        }
    }

    if evidence_class == Some("bench_measured") || evidence_class == Some("field_observed") {
        for (path, value) in [
            ("source.firmware_image_sha256", receipt.pointer("/source/firmware_image_sha256")),
            ("environment.gateway_model", receipt.pointer("/environment/gateway_model")),
            ("environment.gateway_revision", receipt.pointer("/environment/gateway_revision")),
        ] {
            if value.is_none_or(Value::is_null) {
                errors.push(format!("{path} is required for bench or field evidence"));
            }
        }
    }
    if evidence_class == Some("field_observed")
        && receipt.pointer("/environment/site_token").is_none_or(Value::is_null)
    {
        errors.push("field_observed evidence requires a pseudonymous site_token".to_owned());
    }

    let config_sha = receipt.pointer("/source/config_sha256").and_then(Value::as_str);
    let artifacts = receipt.get("artifacts").and_then(Value::as_array);
    if let (Some(config_sha), Some(artifacts)) = (config_sha, artifacts) {
        let bound_config = artifacts.iter().any(|artifact| {
            artifact.get("role").and_then(Value::as_str) == Some("config_snapshot")
                && artifact.get("sha256").and_then(Value::as_str) == Some(config_sha)
        });
        if !bound_config {
            errors.push("source.config_sha256 must match a config_snapshot artifact digest".to_owned());
        }
    }

    if !array_contains_text(
        receipt.pointer("/limitations"),
        "This receipt is an unreviewed evidence record and does not prove measurement truth or deployment authorization.",
    ) {
        errors.push("limitations must retain the required unreviewed-evidence non-claim".to_owned());
    }

    errors.sort();
    errors.dedup();
    errors
}

fn checked_artifact_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.is_empty() || relative.contains('\\') {
        return Err("artifact path must be a non-empty, slash-separated relative path".to_owned());
    }
    let relative_path = Path::new(relative);
    if relative_path.is_absolute() {
        return Err("artifact path must not be absolute".to_owned());
    }
    for component in relative_path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err("artifact path must not contain '.', '..', root or prefix components".to_owned());
        }
    }

    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve evidence root {}: {error}", root.display()))?;
    let candidate = canonical_root.join(relative_path);
    let metadata = fs::symlink_metadata(&candidate)
        .map_err(|error| format!("cannot stat evidence artifact {relative:?}: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Err("artifact must not be a symlink".to_owned());
    }
    if !metadata.is_file() {
        return Err("artifact must be a regular file".to_owned());
    }
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|error| format!("cannot resolve evidence artifact {relative:?}: {error}"))?;
    if !canonical_candidate.starts_with(&canonical_root) {
        return Err("artifact resolves outside the evidence root".to_owned());
    }
    if metadata.len() > MAX_ARTIFACT_BYTES {
        return Err(format!("artifact exceeds {MAX_ARTIFACT_BYTES} byte limit"));
    }
    Ok(canonical_candidate)
}

fn verify_artifacts(root: &Path, receipt: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let artifacts = match receipt.get("artifacts").and_then(Value::as_array) {
        Some(items) => items,
        None => return vec!["artifacts must be an array".to_owned()],
    };
    let mut seen_paths = BTreeSet::new();

    for (index, artifact) in artifacts.iter().enumerate() {
        let relative = match artifact.get("relative_path").and_then(Value::as_str) {
            Some(path) => path,
            None => {
                errors.push(format!("artifacts/{index}/relative_path must be a string"));
                continue;
            }
        };
        if !seen_paths.insert(relative.to_owned()) {
            errors.push(format!("artifacts/{index}/relative_path duplicates {relative:?}"));
            continue;
        }

        let expected_sha = artifact.get("sha256").and_then(Value::as_str);
        let expected_length = artifact.get("byte_length").and_then(Value::as_u64);
        let path = match checked_artifact_path(root, relative) {
            Ok(path) => path,
            Err(error) => {
                errors.push(format!("artifacts/{index}/{relative:?}: {error}"));
                continue;
            }
        };
        let bytes = match read_bounded(&path, MAX_ARTIFACT_BYTES as usize) {
            Ok(bytes) => bytes,
            Err(error) => {
                errors.push(format!("artifacts/{index}/{relative:?}: {error}"));
                continue;
            }
        };

        if expected_length != Some(bytes.len() as u64) {
            errors.push(format!("artifacts/{index}/{relative:?}: byte_length mismatch"));
        }
        let actual_sha = digest_hex(&bytes);
        if expected_sha != Some(actual_sha.as_str()) {
            errors.push(format!("artifacts/{index}/{relative:?}: SHA-256 mismatch"));
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

fn make_validation_report(
    schema_bytes: &[u8],
    receipt_bytes: &[u8],
    schema: &Value,
    receipt: &Value,
    profile: &Value,
    profile_bytes: &[u8],
    evidence_root: &Path,
) -> Value {
    let mut errors = validate_contract(schema, receipt, profile, &digest_hex(profile_bytes));
    errors.extend(verify_artifacts(evidence_root, receipt));
    errors.sort();
    errors.dedup();
    json!({
        "report_version": "luminous.connectivity-measurement-verification-report.v1",
        "receipt_id": receipt.get("receipt_id").and_then(Value::as_str),
        "benchmark_id": receipt.get("benchmark_id").and_then(Value::as_str),
        "schema_sha256": digest_hex(schema_bytes),
        "receipt_sha256": digest_hex(receipt_bytes),
        "benchmark_profile_sha256": digest_hex(profile_bytes),
        "artifact_count": receipt.get("artifacts").and_then(Value::as_array).map_or(0, |items| items.len()),
        "valid": errors.is_empty(),
        "validation_scope": "schema_profile_binding_trial_denominators_local_artifact_hashes",
        "measurement_truth_conveyed": false,
        "independent_review_conveyed": false,
        "deployment_authorization_conveyed": false,
        "errors": errors,
    })
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 || args[1] == "--help" || args[1] == "-h" {
        eprintln!(
            "Usage: connectivity-measurement-receipt-verify <schema.json> <receipt.json> <benchmark-profile.json> <evidence-root>\n\
             Validates draft receipt structure, exact benchmark-profile binding, trial denominators and local artifact bytes.\n\
             This does not prove measurement truth, independent review or deployment authorization."
        );
        process::exit(if args.len() == 2 { 0 } else { 64 });
    }

    let schema_path = Path::new(&args[1]);
    let receipt_path = Path::new(&args[2]);
    let profile_path = Path::new(&args[3]);
    let evidence_root = Path::new(&args[4]);

    let schema_bytes = match read_bounded(schema_path, MAX_SCHEMA_BYTES) {
        Ok(bytes) => bytes,
        Err(error) => { eprintln!("{error}"); process::exit(2); }
    };
    let receipt_bytes = match read_bounded(receipt_path, MAX_RECEIPT_BYTES) {
        Ok(bytes) => bytes,
        Err(error) => { eprintln!("{error}"); process::exit(2); }
    };
    let profile_bytes = match read_bounded(profile_path, MAX_RECEIPT_BYTES) {
        Ok(bytes) => bytes,
        Err(error) => { eprintln!("{error}"); process::exit(2); }
    };

    let schema: Value = match serde_json::from_slice(&schema_bytes) {
        Ok(value) => value,
        Err(error) => { eprintln!("schema JSON error: {error}"); process::exit(2); }
    };
    let receipt: Value = match serde_json::from_slice(&receipt_bytes) {
        Ok(value) => value,
        Err(error) => { eprintln!("receipt JSON error: {error}"); process::exit(2); }
    };
    let profile: Value = match serde_json::from_slice(&profile_bytes) {
        Ok(value) => value,
        Err(error) => { eprintln!("profile JSON error: {error}"); process::exit(2); }
    };

    let report = make_validation_report(
        &schema_bytes, &receipt_bytes, &schema, &receipt, &profile, &profile_bytes, evidence_root
    );
    let report_valid = report.get("valid").and_then(Value::as_bool) == Some(true);
    let mut stdout = std::io::stdout().lock();
    if let Err(error) = serde_json::to_writer_pretty(&mut stdout, &report) {
        eprintln!("cannot write validation report: {error}");
        process::exit(2);
    }
    if let Err(error) = stdout.write_all(b"\n") {
        eprintln!("cannot terminate validation report: {error}");
        process::exit(2);
    }
    if !report_valid {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    const CONFIG_BYTES: &[u8] = b"{\"wan_a\":\"lab-only\",\"wan_b\":\"lab-only\"}\n";

    fn fixtures() -> (Value, Value, Value, Vec<u8>) {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../../schemas/connectivity-measurement-receipt-v1.schema.json"
        )).expect("receipt schema parses");
        let profile_bytes = include_bytes!(
            "../../../../profiles/connectivity-broker-benchmark-profile-v1.json"
        ).to_vec();
        let profile: Value = serde_json::from_slice(&profile_bytes).expect("profile parses");
        let profile_sha = digest_hex(&profile_bytes);
        let config_sha = digest_hex(CONFIG_BYTES);

        let receipt = json!({
            "receipt_version": RECEIPT_VERSION,
            "receipt_id": "connectivity-unit-test-positive-001",
            "status": "draft_unreviewed",
            "benchmark_id": "POLICY_HARD_CONSTRAINTS_001",
            "benchmark_profile_sha256": profile_sha,
            "target_operator": "==",
            "target": 0,
            "target_status": "proposed_target",
            "evidence_class": "vm_simulated",
            "execution_environment": "vm_simulated",
            "source": {
                "repository": "Luminous-Dynamics/luminous-platform",
                "commit_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "config_sha256": config_sha,
                "software_build": "synthetic-test-build-not-product",
                "firmware_image_sha256": null
            },
            "run_started_at": "2026-10-18T10:00:00Z",
            "run_finished_at": "2026-10-18T10:01:00Z",
            "environment": {
                "topology_id": "isolated-two-wan-test-v1",
                "gateway_model": null,
                "gateway_revision": null,
                "device_count": 1,
                "region_code": null,
                "site_token": null
            },
            "metric": {
                "name": "hard_constraint_violations",
                "unit": "count",
                "measurement_method": "Synthetic unit-test fixture only; no live network measured.",
                "aggregation_method": "count failed policy assertions from enumerated test cases"
            },
            "trials": [
                {"trial_id": "case-001", "outcome": "succeeded", "value": 0, "elapsed_ms": 1, "failure_code": null},
                {"trial_id": "case-002", "outcome": "succeeded", "value": 0, "elapsed_ms": 1, "failure_code": null}
            ],
            "summary": {"attempted": 2, "succeeded": 2, "failed": 0, "inconclusive": 0},
            "artifacts": [{
                "relative_path": "config/config.json",
                "sha256": config_sha,
                "byte_length": CONFIG_BYTES.len(),
                "role": "config_snapshot"
            }],
            "privacy": {
                "raw_payloads_collected": false,
                "personal_data_collected": false,
                "identifiers_pseudonymized": true
            },
            "limitations": [
                "Synthetic fixture only; no network, hardware or field experiment occurred.",
                "This receipt is an unreviewed evidence record and does not prove measurement truth or deployment authorization."
            ]
        });
        (schema, receipt, profile, profile_bytes)
    }

    fn temp_root() -> PathBuf {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = env::temp_dir().join(format!("connectivity-receipt-test-{}-{nonce}", process::id()));
        fs::create_dir_all(path.join("config")).unwrap();
        fs::write(path.join("config/config.json"), CONFIG_BYTES).unwrap();
        path
    }

    #[test]
    fn synthetic_receipt_fixture_passes_contract_and_artifact_hash() {
        let (schema, receipt, profile, profile_bytes) = fixtures();
        let root = temp_root();
        let errors = validate_contract(&schema, &receipt, &profile, &digest_hex(&profile_bytes));
        assert!(errors.is_empty(), "contract errors: {errors:#?}");
        let artifact_errors = verify_artifacts(&root, &receipt);
        assert!(artifact_errors.is_empty(), "artifact errors: {artifact_errors:#?}");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn target_cannot_drift_from_bound_profile() {
        let (schema, mut receipt, profile, profile_bytes) = fixtures();
        receipt["target"] = json!(1);
        assert!(!validate_contract(&schema, &receipt, &profile, &digest_hex(&profile_bytes)).is_empty());
    }

    #[test]
    fn benchmark_profile_byte_digest_is_binding() {
        let (schema, receipt, profile, _) = fixtures();
        assert!(!validate_contract(&schema, &receipt, &profile, &"0".repeat(64)).is_empty());
    }

    #[test]
    fn summary_denominators_must_match_raw_trials() {
        let (schema, mut receipt, profile, profile_bytes) = fixtures();
        receipt["summary"]["succeeded"] = json!(1);
        assert!(!validate_contract(&schema, &receipt, &profile, &digest_hex(&profile_bytes)).is_empty());
    }

    #[test]
    fn duplicate_trial_ids_are_rejected() {
        let (schema, mut receipt, profile, profile_bytes) = fixtures();
        receipt["trials"][1]["trial_id"] = json!("case-001");
        assert!(!validate_contract(&schema, &receipt, &profile, &digest_hex(&profile_bytes)).is_empty());
    }

    #[test]
    fn evidence_class_must_match_execution_environment() {
        let (schema, mut receipt, profile, profile_bytes) = fixtures();
        receipt["evidence_class"] = json!("field_observed");
        assert!(!validate_contract(&schema, &receipt, &profile, &digest_hex(&profile_bytes)).is_empty());
    }

    #[test]
    fn failed_trial_requires_failure_code() {
        let (schema, mut receipt, profile, profile_bytes) = fixtures();
        receipt["trials"][0]["outcome"] = json!("failed");
        receipt["trials"][0]["failure_code"] = Value::Null;
        receipt["summary"]["succeeded"] = json!(1);
        receipt["summary"]["failed"] = json!(1);
        assert!(!validate_contract(&schema, &receipt, &profile, &digest_hex(&profile_bytes)).is_empty());
    }

    #[test]
    fn path_traversal_is_rejected() {
        let root = temp_root();
        assert!(checked_artifact_path(&root, "../escape.txt").is_err());
        assert!(checked_artifact_path(&root, "/etc/passwd").is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn altered_artifact_bytes_are_rejected() {
        let (schema, receipt, profile, profile_bytes) = fixtures();
        let root = temp_root();
        fs::write(root.join("config/config.json"), b"altered").unwrap();
        assert!(validate_contract(&schema, &receipt, &profile, &digest_hex(&profile_bytes)).is_empty());
        assert!(!verify_artifacts(&root, &receipt).is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn remote_schema_reference_is_rejected_before_resolution() {
        let (mut schema, receipt, profile, profile_bytes) = fixtures();
        schema["properties"]["status"]["$ref"] = json!("https://example.invalid/remote.json");
        assert!(!validate_contract(&schema, &receipt, &profile, &digest_hex(&profile_bytes)).is_empty());
    }
}
