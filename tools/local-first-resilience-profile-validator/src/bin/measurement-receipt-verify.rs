//! Offline verifier for local-first outage resilience measurement receipts.
//!
//! It checks schema, benchmark-profile binding, raw-trial denominators and
//! local artifact hashes. It does not prove that a trial happened or that the
//! observed result is truthful, secure, representative or authorized.

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
const MAX_JSON_BYTES: usize = 8 * 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const PROFILE_VERSION: &str = "luminous.local-first-outage-resilience-benchmark-profile.v1";
const RECEIPT_VERSION: &str = "luminous.local-first-measurement-receipt.v1";

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut limited = file.take((limit + 1) as u64);
    let mut bytes = Vec::new();
    limited.read_to_end(&mut bytes).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.len() > limit {
        return Err(format!("{} exceeds the {limit} byte input limit", path.display()));
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

fn validate_contract(schema: &Value, receipt: &Value, profile: &Value, profile_bytes: &[u8]) -> Vec<String> {
    let mut errors = Vec::new();
    reject_remote_references(schema, "$schema", &mut errors);
    if !errors.is_empty() {
        return errors;
    }

    let meta_validator = match jsonschema::meta::validator_for(schema) {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot build JSON Schema meta-validator: {error}")],
    };
    if !meta_validator.evaluate(schema).flag().valid {
        return vec!["receipt schema failed JSON Schema meta-validation".to_owned()];
    }

    let validator = match draft202012::options().should_validate_formats(true).build(schema) {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot build Draft 2020-12 validator: {error}")],
    };
    errors.extend(validator.iter_errors(receipt).map(|error| format!("schema: {error}")));

    if profile.get("schema_version").and_then(Value::as_str) != Some(PROFILE_VERSION) {
        errors.push(format!("profile schema_version must be {PROFILE_VERSION}"));
    }
    if profile.get("status").and_then(Value::as_str) != Some("draft_targets_not_results") {
        errors.push("profile must remain draft_targets_not_results".to_owned());
    }
    if receipt.get("receipt_version").and_then(Value::as_str) != Some(RECEIPT_VERSION) {
        errors.push(format!("receipt_version must be {RECEIPT_VERSION}"));
    }
    if receipt.get("status").and_then(Value::as_str) != Some("draft_unreviewed") {
        errors.push("receipt status must remain draft_unreviewed".to_owned());
    }
    if receipt.get("benchmark_profile_sha256").and_then(Value::as_str) != Some(digest_hex(profile_bytes).as_str()) {
        errors.push("benchmark_profile_sha256 differs from exact profile file bytes".to_owned());
    }

    let benchmark_id = receipt.get("benchmark_id").and_then(Value::as_str);
    let benchmarks = profile.get("benchmarks").and_then(Value::as_array);
    let matches: Vec<&Value> = benchmarks
        .into_iter()
        .flatten()
        .filter(|item| item.get("id").and_then(Value::as_str) == benchmark_id)
        .collect();
    if matches.len() != 1 {
        errors.push("benchmark_id must match exactly one record in the bound profile".to_owned());
    } else {
        let benchmark = matches[0];
        for (receipt_field, profile_field) in [
            ("target_operator", "operator"),
            ("target", "target"),
            ("target_status", "target_status"),
        ] {
            if receipt.get(receipt_field) != benchmark.get(profile_field) {
                errors.push(format!("{receipt_field} differs from exact profile benchmark record"));
            }
        }
    }

    let evidence = receipt.get("evidence_class").and_then(Value::as_str);
    if evidence != receipt.get("execution_environment").and_then(Value::as_str) {
        errors.push("evidence_class must equal execution_environment".to_owned());
    }

    if let (Some(trials), Some(summary)) = (
        receipt.get("trials").and_then(Value::as_array),
        receipt.get("summary"),
    ) {
        let attempted = summary.get("attempted").and_then(Value::as_u64);
        let succeeded = summary.get("succeeded").and_then(Value::as_u64);
        let failed = summary.get("failed").and_then(Value::as_u64);
        let inconclusive = summary.get("inconclusive").and_then(Value::as_u64);
        if attempted != Some(trials.len() as u64) {
            errors.push("summary.attempted must equal number of raw trial records".to_owned());
        }

        let mut ids = BTreeSet::new();
        let mut counts = [0_u64; 3];
        for (index, trial) in trials.iter().enumerate() {
            if let Some(id) = trial.get("trial_id").and_then(Value::as_str)
                && !ids.insert(id.to_owned())
            {
                errors.push(format!("trials/{index}/trial_id duplicates {id}"));
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
                errors.push(format!("trials/{index}: succeeded trial cannot carry failure_code"));
            }
            if trial.get("outcome").and_then(Value::as_str) == Some("failed")
                && trial.get("failure_code").is_none_or(Value::is_null)
            {
                errors.push(format!("trials/{index}: failed trial requires failure_code"));
            }
        }
        if succeeded != Some(counts[0]) {
            errors.push("summary.succeeded differs from raw trial outcomes".to_owned());
        }
        if failed != Some(counts[1]) {
            errors.push("summary.failed differs from raw trial outcomes".to_owned());
        }
        if inconclusive != Some(counts[2]) {
            errors.push("summary.inconclusive differs from raw trial outcomes".to_owned());
        }
        if attempted != succeeded.map(|value| value + failed.unwrap_or(0) + inconclusive.unwrap_or(0)) {
            errors.push("attempted must equal succeeded + failed + inconclusive".to_owned());
        }
    }

    let config_digest = receipt.pointer("/source/config_sha256").and_then(Value::as_str);
    let artifacts = receipt.get("artifacts").and_then(Value::as_array);
    if let (Some(config_digest), Some(artifacts)) = (config_digest, artifacts)
        && !artifacts.iter().any(|artifact| {
            artifact.get("role").and_then(Value::as_str) == Some("config_snapshot")
                && artifact.get("sha256").and_then(Value::as_str) == Some(config_digest)
        })
    {
        errors.push("source.config_sha256 must match a config_snapshot artifact digest".to_owned());
    }

    if evidence == Some("bench_measured") || evidence == Some("field_observed") {
        for (name, value) in [
            ("source.firmware_image_sha256", receipt.pointer("/source/firmware_image_sha256")),
            ("environment.gateway_model", receipt.pointer("/environment/gateway_model")),
            ("environment.gateway_revision", receipt.pointer("/environment/gateway_revision")),
        ] {
            if value.is_none_or(Value::is_null) {
                errors.push(format!("{name} is required for bench/field evidence"));
            }
        }
    }
    if evidence == Some("field_observed")
        && receipt.pointer("/environment/site_token").is_none_or(Value::is_null)
    {
        errors.push("field_observed evidence requires pseudonymous site_token".to_owned());
    }

    errors.sort();
    errors.dedup();
    errors
}

fn checked_artifact_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.is_empty() || relative.contains('\\') {
        return Err("artifact path must be non-empty and slash-separated".to_owned());
    }
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("artifact path must be a safe relative path without traversal components".to_owned());
    }

    let canonical_root = root.canonicalize().map_err(|error| format!("cannot resolve evidence root: {error}"))?;
    let candidate = canonical_root.join(relative_path);
    let metadata = fs::symlink_metadata(&candidate).map_err(|error| format!("cannot stat artifact {relative:?}: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("artifact must be a regular non-symlink file".to_owned());
    }
    if metadata.len() > MAX_ARTIFACT_BYTES {
        return Err(format!("artifact exceeds {MAX_ARTIFACT_BYTES} byte limit"));
    }
    let canonical = candidate.canonicalize().map_err(|error| format!("cannot resolve artifact: {error}"))?;
    if !canonical.starts_with(&canonical_root) {
        return Err("artifact resolves outside evidence root".to_owned());
    }
    Ok(canonical)
}

fn verify_artifacts(root: &Path, receipt: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(artifacts) = receipt.get("artifacts").and_then(Value::as_array) else {
        return vec!["artifacts must be an array".to_owned()];
    };
    let mut paths = BTreeSet::new();
    for (index, artifact) in artifacts.iter().enumerate() {
        let Some(relative) = artifact.get("relative_path").and_then(Value::as_str) else {
            errors.push(format!("artifacts/{index}/relative_path must be a string"));
            continue;
        };
        if !paths.insert(relative.to_owned()) {
            errors.push(format!("artifacts/{index}/relative_path duplicates {relative:?}"));
            continue;
        }
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
        let actual_digest = digest_hex(&bytes);
        if artifact.get("byte_length").and_then(Value::as_u64) != Some(bytes.len() as u64) {
            errors.push(format!("artifacts/{index}/{relative:?}: byte_length mismatch"));
        }
        if artifact.get("sha256").and_then(Value::as_str) != Some(actual_digest.as_str()) {
            errors.push(format!("artifacts/{index}/{relative:?}: SHA-256 mismatch"));
        }
    }
    errors.sort();
    errors.dedup();
    errors
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 || args[1] == "--help" || args[1] == "-h" {
        eprintln!(
            "Usage: local-first-measurement-receipt-verify <receipt-schema.json> <receipt.json> <benchmark-profile.json> <evidence-root>\n\
             Validates schema, exact profile binding, trial counts and local artifact digests only.\n\
             This does not prove measurement truth, cryptographic security, independent review or authorization."
        );
        process::exit(if args.len() == 2 { 0 } else { 64 });
    }

    let schema_bytes = match read_bounded(Path::new(&args[1]), MAX_SCHEMA_BYTES) {
        Ok(bytes) => bytes,
        Err(error) => { eprintln!("{error}"); process::exit(2); }
    };
    let receipt_bytes = match read_bounded(Path::new(&args[2]), MAX_JSON_BYTES) {
        Ok(bytes) => bytes,
        Err(error) => { eprintln!("{error}"); process::exit(2); }
    };
    let profile_bytes = match read_bounded(Path::new(&args[3]), MAX_JSON_BYTES) {
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

    let mut errors = validate_contract(&schema, &receipt, &profile, &profile_bytes);
    errors.extend(verify_artifacts(Path::new(&args[4]), &receipt));
    errors.sort();
    errors.dedup();
    let valid = errors.is_empty();

    let report = json!({
        "report_version": "luminous.local-first-measurement-verification-report.v1",
        "receipt_id": receipt.get("receipt_id").and_then(Value::as_str),
        "benchmark_id": receipt.get("benchmark_id").and_then(Value::as_str),
        "schema_sha256": digest_hex(&schema_bytes),
        "receipt_sha256": digest_hex(&receipt_bytes),
        "benchmark_profile_sha256": digest_hex(&profile_bytes),
        "artifact_count": receipt.get("artifacts").and_then(Value::as_array).map_or(0, |items| items.len()),
        "valid": valid,
        "validation_scope": "schema_profile_binding_trial_denominators_local_artifact_hashes",
        "measurement_truth_conveyed": false,
        "independent_review_conveyed": false,
        "deployment_authorization_conveyed": false,
        "errors": errors
    });

    let mut stdout = std::io::stdout().lock();
    if let Err(error) = serde_json::to_writer_pretty(&mut stdout, &report) {
        eprintln!("cannot write verification report: {error}");
        process::exit(2);
    }
    if let Err(error) = stdout.write_all(b"\n") {
        eprintln!("cannot terminate verification report: {error}");
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

    const CONFIG_BYTES: &[u8] = b"test-only config snapshot; no network tested\n";

    fn fixtures() -> (Value, Value, Value, Vec<u8>) {
        let schema: Value = serde_json::from_str(include_str!("../../../../schemas/local-first-measurement-receipt-v1.schema.json")).expect("receipt schema parses");
        let profile_bytes = include_bytes!("../../../../profiles/local-first-outage-resilience-benchmark-profile-v1.json").to_vec();
        let profile: Value = serde_json::from_slice(&profile_bytes).expect("profile parses");
        let config_digest = digest_hex(CONFIG_BYTES);
        let receipt = json!({
            "receipt_version": RECEIPT_VERSION,
            "receipt_id": "local-first-unit-test-positive-001",
            "status": "draft_unreviewed",
            "benchmark_profile_sha256": digest_hex(&profile_bytes),
            "benchmark_id": "QUEUE_BOUNDS_001",
            "target_operator": "==",
            "target": 0,
            "target_status": "proposed_target",
            "evidence_class": "vm_simulated",
            "execution_environment": "vm_simulated",
            "source": {
                "repository": "Luminous-Dynamics/luminous-platform",
                "commit_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "config_sha256": config_digest,
                "software_build": "synthetic-test-only",
                "firmware_image_sha256": null
            },
            "run_started_at": "2026-10-18T10:00:00Z",
            "run_finished_at": "2026-10-18T10:01:00Z",
            "environment": {
                "topology_id": "two-node-offline-test-v1",
                "gateway_model": null,
                "gateway_revision": null,
                "device_count": 1,
                "site_token": null
            },
            "metric": {
                "name": "messages_admitted_over_any_bound",
                "unit": "count",
                "measurement_method": "Synthetic fixture only; no live queue or network measured.",
                "aggregation_method": "count invalid test envelopes accepted"
            },
            "trials": [
                {"trial_id":"case-001","outcome":"succeeded","value":0,"elapsed_ms":1,"failure_code":null},
                {"trial_id":"case-002","outcome":"succeeded","value":0,"elapsed_ms":1,"failure_code":null}
            ],
            "summary": {"attempted":2,"succeeded":2,"failed":0,"inconclusive":0},
            "artifacts": [{
                "relative_path":"config/config.json",
                "sha256":config_digest,
                "byte_length":CONFIG_BYTES.len(),
                "role":"config_snapshot"
            }],
            "privacy": {
                "raw_payloads_collected":false,
                "personal_data_collected":false,
                "identifiers_pseudonymized":true
            },
            "limitations": [
                "Synthetic fixture only; no queue, device or field test occurred.",
                "This receipt is an unreviewed evidence record and does not prove measurement truth or deployment authorization."
            ]
        });
        (schema, receipt, profile, profile_bytes)
    }

    #[test]
    fn schema_and_profile_binding_accept_synthetic_receipt() {
        let (schema, receipt, profile, profile_bytes) = fixtures();
        let errors = validate_contract(&schema, &receipt, &profile, &profile_bytes);
        assert!(errors.is_empty(), "errors: {errors:#?}");
    }

    #[test]
    fn target_drift_is_rejected() {
        let (schema, mut receipt, profile, profile_bytes) = fixtures();
        receipt["target"] = json!(1);
        assert!(!validate_contract(&schema, &receipt, &profile, &profile_bytes).is_empty());
    }

    #[test]
    fn profile_byte_digest_mismatch_is_rejected() {
        let (schema, receipt, profile, _) = fixtures();
        assert!(!validate_contract(&schema, &receipt, &profile, &b"modified profile bytes"[..]).is_empty());
    }

    #[test]
    fn raw_trial_denominator_mismatch_is_rejected() {
        let (schema, mut receipt, profile, profile_bytes) = fixtures();
        receipt["summary"]["succeeded"] = json!(1);
        assert!(!validate_contract(&schema, &receipt, &profile, &profile_bytes).is_empty());
    }

    #[test]
    fn duplicate_trial_ids_are_rejected() {
        let (schema, mut receipt, profile, profile_bytes) = fixtures();
        receipt["trials"][1]["trial_id"] = json!("case-001");
        assert!(!validate_contract(&schema, &receipt, &profile, &profile_bytes).is_empty());
    }

    #[test]
    fn failed_trials_require_failure_codes() {
        let (schema, mut receipt, profile, profile_bytes) = fixtures();
        receipt["trials"][0]["outcome"] = json!("failed");
        receipt["trials"][0]["failure_code"] = Value::Null;
        receipt["summary"]["succeeded"] = json!(1);
        receipt["summary"]["failed"] = json!(1);
        assert!(!validate_contract(&schema, &receipt, &profile, &profile_bytes).is_empty());
    }

    #[test]
    fn remote_schema_reference_is_rejected() {
        let (mut schema, receipt, profile, profile_bytes) = fixtures();
        schema["properties"]["status"]["$ref"] = json!("https://example.invalid/remote.json");
        assert!(!validate_contract(&schema, &receipt, &profile, &profile_bytes).is_empty());
    }

    #[test]
    fn unsafe_artifact_paths_are_rejected() {
        assert!(checked_artifact_path(Path::new("."), "../escape").is_err());
        assert!(checked_artifact_path(Path::new("."), "/etc/passwd").is_err());
    }
}
