use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const CONFIG_BYTES: &[u8] = b"synthetic lab config snapshot only\n";

fn digest_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct Fixture {
    root: PathBuf,
    schema_path: PathBuf,
    profile_path: PathBuf,
    receipt_path: PathBuf,
    evidence_root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "local-first-receipt-cli-{}-{nonce}",
            std::process::id()
        ));
        let evidence_root = root.join("evidence");
        fs::create_dir_all(evidence_root.join("config")).expect("create evidence root");

        let schema_path = root.join("receipt-schema.json");
        let profile_path = root.join("benchmark-profile.json");
        let receipt_path = root.join("receipt.json");
        let schema_bytes = include_bytes!("../../../schemas/local-first-measurement-receipt-v1.schema.json");
        let profile_bytes = include_bytes!("../../../profiles/local-first-outage-resilience-benchmark-profile-v1.json");
        fs::write(&schema_path, schema_bytes).expect("write receipt schema");
        fs::write(&profile_path, profile_bytes).expect("write profile");
        fs::write(evidence_root.join("config/config.json"), CONFIG_BYTES).expect("write config artifact");

        let config_digest = digest_hex(CONFIG_BYTES);
        let receipt = json!({
            "receipt_version":"luminous.local-first-measurement-receipt.v1",
            "receipt_id":"local-first-cli-test-positive-001",
            "status":"draft_unreviewed",
            "benchmark_profile_sha256":digest_hex(profile_bytes),
            "benchmark_id":"QUEUE_BOUNDS_001",
            "target_operator":"==",
            "target":0,
            "target_status":"proposed_target",
            "evidence_class":"vm_simulated",
            "execution_environment":"vm_simulated",
            "source":{
                "repository":"Luminous-Dynamics/luminous-platform",
                "commit_sha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "config_sha256":config_digest,
                "software_build":"synthetic-cli-fixture-only",
                "firmware_image_sha256":null
            },
            "run_started_at":"2026-10-18T10:00:00Z",
            "run_finished_at":"2026-10-18T10:01:00Z",
            "environment":{
                "topology_id":"two-node-offline-test-v1",
                "gateway_model":null,
                "gateway_revision":null,
                "device_count":1,
                "site_token":null
            },
            "metric":{
                "name":"messages_admitted_over_any_bound",
                "unit":"count",
                "measurement_method":"Synthetic integration fixture only; no live queue or network measured.",
                "aggregation_method":"count invalid synthetic envelopes accepted"
            },
            "trials":[
                {"trial_id":"case-001","outcome":"succeeded","value":0,"elapsed_ms":1,"failure_code":null},
                {"trial_id":"case-002","outcome":"succeeded","value":0,"elapsed_ms":1,"failure_code":null}
            ],
            "summary":{"attempted":2,"succeeded":2,"failed":0,"inconclusive":0},
            "artifacts":[{
                "relative_path":"config/config.json",
                "sha256":config_digest,
                "byte_length":CONFIG_BYTES.len(),
                "role":"config_snapshot"
            }],
            "privacy":{
                "raw_payloads_collected":false,
                "personal_data_collected":false,
                "identifiers_pseudonymized":true
            },
            "limitations":[
                "Synthetic integration fixture only; no network, device or field experiment occurred.",
                "This receipt is an unreviewed evidence record and does not prove measurement truth or deployment authorization."
            ]
        });
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).expect("serialize receipt"))
            .expect("write receipt");

        Self { root, schema_path, profile_path, receipt_path, evidence_root }
    }

    fn invoke(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_local-first-measurement-receipt-verify"))
            .arg(&self.schema_path)
            .arg(&self.receipt_path)
            .arg(&self.profile_path)
            .arg(&self.evidence_root)
            .output()
            .expect("invoke verifier CLI")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn cli_accepts_synthetic_receipt_and_emits_non_claims() {
    let fixture = Fixture::new();
    let output = fixture.invoke();
    assert!(output.status.success(), "CLI failed: {}", String::from_utf8_lossy(&output.stdout));
    let report: Value = serde_json::from_slice(&output.stdout).expect("validation report JSON");
    assert_eq!(report.get("valid").and_then(Value::as_bool), Some(true));
    assert_eq!(report.get("measurement_truth_conveyed").and_then(Value::as_bool), Some(false));
    assert_eq!(report.get("independent_review_conveyed").and_then(Value::as_bool), Some(false));
    assert_eq!(report.get("deployment_authorization_conveyed").and_then(Value::as_bool), Some(false));
    assert_eq!(
        report.get("benchmark_profile_sha256").and_then(Value::as_str),
        Some(digest_hex(include_bytes!("../../../profiles/local-first-outage-resilience-benchmark-profile-v1.json")).as_str())
    );
}

#[test]
fn cli_rejects_mutated_trial_denominators() {
    let fixture = Fixture::new();
    let mut receipt: Value = serde_json::from_slice(&fs::read(&fixture.receipt_path).unwrap()).unwrap();
    receipt["summary"]["succeeded"] = json!(1);
    fs::write(&fixture.receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    let output = fixture.invoke();
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).expect("validation report JSON");
    assert_eq!(report.get("valid").and_then(Value::as_bool), Some(false));
    assert!(report.get("errors").and_then(Value::as_array).is_some_and(|items| {
        items.iter().any(|item| item.as_str().unwrap_or("").contains("summary.succeeded"))
    }));
}

#[test]
fn cli_rejects_tampered_artifact_bytes() {
    let fixture = Fixture::new();
    fs::write(fixture.evidence_root.join("config/config.json"), b"changed bytes").expect("tamper artifact");
    let output = fixture.invoke();
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).expect("validation report JSON");
    assert_eq!(report.get("valid").and_then(Value::as_bool), Some(false));
    assert!(report.get("errors").and_then(Value::as_array).is_some_and(|items| {
        items.iter().any(|item| item.as_str().unwrap_or("").contains("SHA-256 mismatch"))
    }));
}
