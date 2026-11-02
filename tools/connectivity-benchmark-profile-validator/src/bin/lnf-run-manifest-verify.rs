//! Fail-closed validation of the LNF end-to-end experiment manifest.
//!
//! This validates a planned experiment and its pinned source artifacts. It does
//! not execute a network, verify measurement truth, qualify hardware, or grant
//! deployment authorization.

use jsonschema::draft202012;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;
const SCHEMA_VERSION: &str = "luminous.lnf-e2e-run-manifest.v1";
const PROFILE_ID: &str = "lnf-edge-dtn-e2e-v1";
const BROKER_COMMIT: &str = "85f1d9aa93457dab5b4c4aa372353799ee0984f8";
const MYCELIX_COMMIT: &str = "006f7da146b4d1c781967c3ee8a6161b411b633c";

#[derive(Clone, Copy)]
struct SourcePin {
    id: &'static str,
    repository: &'static str,
    path: &'static str,
    commit: &'static str,
    blob: &'static str,
    verification: &'static str,
}

const SOURCE_PINS: [SourcePin; 4] = [
    SourcePin {
        id: "broker-profile",
        repository: "Luminous-Dynamics/luminous-platform",
        path: "profiles/connectivity-broker-benchmark-profile-v1.json",
        commit: BROKER_COMMIT,
        blob: "2693cc12985d8612d2f86c049862fec4ca800aaf",
        verification: "local_git_blob_at_pinned_commit",
    },
    SourcePin {
        id: "broker-profile-schema",
        repository: "Luminous-Dynamics/luminous-platform",
        path: "schemas/connectivity-broker-benchmark-profile-v1.schema.json",
        commit: BROKER_COMMIT,
        blob: "a3887217b4505fe38e5b25aba8967ed2e79af415",
        verification: "local_git_blob_at_pinned_commit",
    },
    SourcePin {
        id: "measurement-receipt-schema",
        repository: "Luminous-Dynamics/luminous-platform",
        path: "schemas/connectivity-measurement-receipt-v1.schema.json",
        commit: BROKER_COMMIT,
        blob: "1e81ae82d1d9082e4d19ff0ed69f6595a62f55b2",
        verification: "local_git_blob_at_pinned_commit",
    },
    SourcePin {
        id: "mycelix-f0-topology",
        repository: "Luminous-Dynamics/mycelix",
        path: "docs/interoperability/fixtures/MYC_INT_007F_F0_FEDERATION.json",
        commit: MYCELIX_COMMIT,
        blob: "537c6f299be47565c26d30f294c1518b5e8debb1",
        verification: "remote_git_blob_pinned_not_materialized",
    },
];

const REQUIRED_SCENARIOS: [&str; 12] = [
    "E2E-00", "E2E-01", "E2E-02", "E2E-03", "E2E-04", "E2E-05",
    "E2E-06", "E2E-07", "E2E-08", "E2E-09", "E2E-10", "E2E-11",
];

const REQUIRED_NODES: [&str; 6] = [
    "client", "gateway", "local_service", "dtn_sender", "dtn_receiver", "observer",
];

const REQUIRED_LINKS: [&str; 4] = [
    "lan-client-gateway", "wan-a", "wan-b", "dtn-delay-path",
];

const REQUIRED_BENCHMARK_IDS: [&str; 8] = [
    "POLICY_HARD_CONSTRAINTS_001",
    "POLICY_EXPLANATION_001",
    "POLICY_REPLAY_001",
    "POLICY_DATA_MINIMIZATION_001",
    "GATEWAY_FAILOVER_001",
    "LOCAL_CONTINUITY_001",
    "STORE_FORWARD_001",
    "ENERGY_PER_USEFUL_TASK_001",
];

fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path)
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

/// Do not allow schema validation to retrieve untrusted remote references.
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

fn validate_schema(schema: &Value, manifest: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    collect_nonlocal_references(schema, "$schema", &mut errors);
    if !errors.is_empty() {
        return errors;
    }

    let meta = match jsonschema::meta::validator_for(schema) {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot build schema meta-validator: {error}")],
    };
    if !meta.evaluate(schema).flag().valid {
        return vec!["LNF manifest schema failed JSON Schema meta-validation".to_owned()];
    }

    let validator = match draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot build Draft 2020-12 validator: {error}")],
    };

    validator
        .iter_errors(manifest)
        .map(|error| format!("schema: {error}"))
        .collect()
}

fn source_binding_by_id<'a>(manifest: &'a Value, id: &str) -> Option<&'a Value> {
    manifest
        .get("source_bindings")?
        .as_array()?
        .iter()
        .find(|binding| binding.get("id").and_then(Value::as_str) == Some(id))
}

fn git_blob_at(root: &Path, commit: &str, path: &str) -> Result<String, String> {
    let tree_path = format!("{commit}:{path}");
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("rev-parse")
        .arg("--verify")
        .arg(tree_path)
        .output()
        .map_err(|error| format!("cannot execute git rev-parse: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git cannot resolve pinned source {commit}:{path}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| format!("git returned a non-UTF-8 blob id: {error}"))
}

fn git_show(root: &Path, commit: &str, path: &str) -> Result<Vec<u8>, String> {
    let tree_path = format!("{commit}:{path}");
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("show")
        .arg(tree_path)
        .output()
        .map_err(|error| format!("cannot execute git show: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot read pinned source {commit}:{path}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    if output.stdout.len() > MAX_INPUT_BYTES {
        return Err(format!("pinned source {path} exceeds the input limit"));
    }
    Ok(output.stdout)
}

fn validate_source_pins(manifest: &Value, repo_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let bindings = manifest.get("source_bindings").and_then(Value::as_array);
    let Some(bindings) = bindings else {
        return vec!["source_bindings must be an array".to_owned()];
    };

    if bindings.len() != SOURCE_PINS.len() {
        errors.push(format!(
            "exactly {} source bindings are required",
            SOURCE_PINS.len()
        ));
    }

    let mut seen = BTreeSet::new();
    for binding in bindings {
        let id = binding.get("id").and_then(Value::as_str).unwrap_or("<missing>");
        if !seen.insert(id.to_owned()) {
            errors.push(format!("source binding {id} is duplicated"));
            continue;
        }

        let Some(pin) = SOURCE_PINS.iter().find(|pin| pin.id == id) else {
            errors.push(format!("unknown source binding {id}"));
            continue;
        };

        for (field, expected) in [
            ("repository", pin.repository),
            ("path", pin.path),
            ("commit_sha", pin.commit),
            ("git_blob_sha", pin.blob),
            ("source_state", "draft_pr"),
            ("verification_method", pin.verification),
        ] {
            if binding.get(field).and_then(Value::as_str) != Some(expected) {
                errors.push(format!("source_bindings/{id}/{field} does not match the pinned value"));
            }
        }

        if pin.verification == "local_git_blob_at_pinned_commit" {
            match git_blob_at(repo_root, pin.commit, pin.path) {
                Ok(actual) if actual == pin.blob => {}
                Ok(actual) => errors.push(format!(
                    "source_bindings/{id}: local Git blob mismatch; expected {}, found {actual}",
                    pin.blob
                )),
                Err(error) => errors.push(format!("source_bindings/{id}: {error}")),
            }
        }
    }

    for pin in SOURCE_PINS {
        if !seen.contains(pin.id) {
            errors.push(format!("required source binding {} is missing", pin.id));
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

fn validate_benchmarks(manifest: &Value, profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    if profile.get("schema_version").and_then(Value::as_str)
        != Some("luminous.connectivity-broker-benchmark-profile.v1")
    {
        errors.push("pinned broker profile has an unexpected schema_version".to_owned());
    }
    if profile.get("profile_id").and_then(Value::as_str)
        != Some("provider-neutral-connectivity-broker-v1")
    {
        errors.push("pinned broker profile has an unexpected profile_id".to_owned());
    }
    if profile.get("status").and_then(Value::as_str) != Some("draft_targets_not_results") {
        errors.push("pinned broker profile is not marked draft_targets_not_results".to_owned());
    }

    let profile_items = profile.get("benchmarks").and_then(Value::as_array);
    let Some(profile_items) = profile_items else {
        return vec!["pinned broker profile has no benchmark array".to_owned()];
    };
    let profile_ids: BTreeSet<&str> = profile_items
        .iter()
        .filter_map(|item| item.get("id").and_then(Value::as_str))
        .collect();

    let selected = manifest.get("selected_benchmarks").and_then(Value::as_array);
    let Some(selected) = selected else {
        return vec!["selected_benchmarks must be an array".to_owned()];
    };
    let mut selected_ids = BTreeSet::new();
    for item in selected {
        match item.as_str() {
            Some(id) => {
                if !selected_ids.insert(id) {
                    errors.push(format!("selected benchmark {id} is duplicated"));
                }
                if !profile_ids.contains(id) {
                    errors.push(format!("selected benchmark {id} is absent from the pinned profile"));
                }
            }
            None => errors.push("selected benchmark IDs must be strings".to_owned()),
        }
    }

    for required in REQUIRED_BENCHMARK_IDS {
        if !selected_ids.contains(required) {
            errors.push(format!("required campaign benchmark {required} is not selected"));
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

fn validate_exact_ids(
    value: Option<&Value>,
    field: &str,
    required: &[&str],
    errors: &mut Vec<String>,
) {
    let Some(items) = value.and_then(Value::as_array) else {
        errors.push(format!("{field} must be an array"));
        return;
    };
    let mut actual = BTreeSet::new();
    for item in items {
        let id = item
            .as_str()
            .or_else(|| item.get("id").and_then(Value::as_str));
        match id {
            Some(id) if !actual.insert(id.to_owned()) => {
                errors.push(format!("{field} contains duplicate ID {id}"));
            }
            Some(_) => {}
            None => errors.push(format!("{field} entries must be strings or objects with string IDs")),
        }
    }
    let expected: BTreeSet<String> = required.iter().map(|id| (*id).to_owned()).collect();
    if actual != expected {
        errors.push(format!("{field} must contain exactly the declared v1 IDs"));
    }
}

fn validate_topology(manifest: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let topology = manifest.get("topology");
    validate_exact_ids(
        topology.and_then(|value| value.get("nodes")),
        "topology.nodes",
        &REQUIRED_NODES,
        &mut errors,
    );
    validate_exact_ids(
        topology.and_then(|value| value.get("links")),
        "topology.links",
        &REQUIRED_LINKS,
        &mut errors,
    );

    if topology
        .and_then(|value| value.get("semantic_identity_is_separate_from_runtime_identity"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        errors.push("semantic identity must remain separate from runtime identity".to_owned());
    }

    if let Some(nodes) = topology.and_then(|value| value.get("nodes")).and_then(Value::as_array) {
        for node in nodes {
            let id = node.get("id").and_then(Value::as_str).unwrap_or("<unknown>");
            if node.get("provenance").and_then(Value::as_str) != Some("synthetic") {
                errors.push(format!("node {id} must remain synthetic in this fixture"));
            }
            if node.get("runtime_id_policy").and_then(Value::as_str)
                != Some("ephemeral_locator_not_semantic_identity")
            {
                errors.push(format!("node {id} has an unsafe runtime identity policy"));
            }
        }
    }

    if let Some(links) = topology.and_then(|value| value.get("links")).and_then(Value::as_array) {
        let mut wan_domains = BTreeSet::new();
        for link in links {
            let id = link.get("id").and_then(Value::as_str).unwrap_or("<unknown>");
            if link.get("physical_technology_claimed").and_then(Value::as_bool) != Some(false) {
                errors.push(format!("link {id} must not claim a physical technology"));
            }
            if matches!(id, "wan-a" | "wan-b") {
                if let Some(domain) = link.get("failure_domain").and_then(Value::as_str) {
                    wan_domains.insert(domain.to_owned());
                }
            }
        }
        if wan_domains.len() != 2 {
            errors.push("WAN-A and WAN-B must have separately declared failure domains".to_owned());
        }
    }

    errors
}

fn validate_workloads(manifest: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(workloads) = manifest.get("workloads").and_then(Value::as_array) else {
        return vec!["workloads must be an array".to_owned()];
    };

    let expected: BTreeMap<&str, (&str, &str)> = [
        ("local-cache-read", ("LocalOnly", "local_only")),
        ("remote-resource-fetch", ("GeneralBrowsing", "live_only")),
        ("durable-message", ("Messaging", "durable_store_forward")),
    ]
    .into_iter()
    .collect();

    let mut seen = BTreeSet::new();
    for workload in workloads {
        let id = workload.get("id").and_then(Value::as_str).unwrap_or("<missing>");
        if !seen.insert(id.to_owned()) {
            errors.push(format!("workload {id} is duplicated"));
            continue;
        }
        let Some((intent, semantics)) = expected.get(id) else {
            errors.push(format!("unknown workload {id}"));
            continue;
        };
        if workload.get("intent").and_then(Value::as_str) != Some(intent) {
            errors.push(format!("workload {id} has incorrect declared intent"));
        }
        if workload.get("path_semantics").and_then(Value::as_str) != Some(semantics) {
            errors.push(format!("workload {id} has incorrect path semantics"));
        }
        if workload.get("raw_payload_capture").and_then(Value::as_bool) != Some(false) {
            errors.push(format!("workload {id} must not capture raw payloads"));
        }
    }
    if seen.len() != expected.len() {
        errors.push("the three required workload classes must be present".to_owned());
    }
    errors
}

fn validate_claim_ceiling(manifest: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(ceiling) = manifest.get("claim_ceiling") else {
        return vec!["claim_ceiling is required".to_owned()];
    };
    for field in [
        "execution_conducted",
        "measurement_truth_established",
        "hardware_qualified",
        "field_observed",
        "production_ready",
        "deployment_authorized",
        "carrier_equivalence_established",
    ] {
        if ceiling.get(field).and_then(Value::as_bool) != Some(false) {
            errors.push(format!("claim_ceiling/{field} must remain false"));
        }
    }
    errors
}

fn validate_execution_profile(manifest: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(execution) = manifest.get("execution_profile") else {
        return vec!["execution_profile is required".to_owned()];
    };
    for field in [
        "external_network_access",
        "production_routes_present",
        "public_rf_transmission",
        "real_subscriber_data",
        "provider_credentials",
    ] {
        if execution.get(field).and_then(Value::as_bool) != Some(false) {
            errors.push(format!("execution_profile/{field} must remain false"));
        }
    }
    if execution
        .get("simulation_is_not_field_evidence")
        .and_then(Value::as_bool)
        != Some(true)
    {
        errors.push("execution_profile must preserve the simulation/field evidence boundary".to_owned());
    }
    errors
}

fn validate_manifest(
    schema: &Value,
    manifest: &Value,
    broker_profile: &Value,
    repo_root: &Path,
) -> Vec<String> {
    let mut errors = validate_schema(schema, manifest);
    errors.extend(validate_source_pins(manifest, repo_root));
    errors.extend(validate_benchmarks(manifest, broker_profile));
    errors.extend(validate_topology(manifest));
    errors.extend(validate_workloads(manifest));
    errors.extend(validate_claim_ceiling(manifest));
    errors.extend(validate_execution_profile(manifest));

    if manifest.get("schema_version").and_then(Value::as_str) != Some(SCHEMA_VERSION) {
        errors.push(format!("schema_version must be {SCHEMA_VERSION}"));
    }
    if manifest.get("profile_id").and_then(Value::as_str) != Some(PROFILE_ID) {
        errors.push(format!("profile_id must be {PROFILE_ID}"));
    }
    if manifest.get("status").and_then(Value::as_str) != Some("design_fixture_only") {
        errors.push("manifest status must remain design_fixture_only".to_owned());
    }
    if manifest.get("run_id").and_then(Value::as_str).is_none_or(|id| !id.starts_with("fixture-")) {
        errors.push("run_id must remain a non-executed fixture identifier".to_owned());
    }

    validate_exact_ids(
        manifest.get("scenario_ids"),
        "scenario_ids",
        &REQUIRED_SCENARIOS,
        &mut errors,
    );

    errors.sort();
    errors.dedup();
    errors
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 || args[1] == "--help" || args[1] == "-h" {
        eprintln!(
            "Usage: lnf-run-manifest-verify <schema.json> <manifest.json> <repository-root>\n\
             Validates a design-only LNF campaign manifest and exact pinned source objects.\n\
             It does not execute a network or prove measurement truth, hardware qualification,\n\
             production readiness, or deployment authorization."
        );
        process::exit(if args.len() == 2 { 0 } else { 64 });
    }

    let schema_path = Path::new(&args[1]);
    let manifest_path = Path::new(&args[2]);
    let repo_root = PathBuf::from(&args[3]);

    let schema_bytes = match read_bounded(schema_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    let manifest_bytes = match read_bounded(manifest_path) {
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
    let manifest: Value = match serde_json::from_slice(&manifest_bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("manifest is not valid JSON: {error}");
            process::exit(2);
        }
    };

    let broker_profile_bytes = match git_show(
        &repo_root,
        BROKER_COMMIT,
        "profiles/connectivity-broker-benchmark-profile-v1.json",
    ) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    let broker_profile: Value = match serde_json::from_slice(&broker_profile_bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("pinned broker profile is not valid JSON: {error}");
            process::exit(2);
        }
    };

    let errors = validate_manifest(&schema, &manifest, &broker_profile, &repo_root);
    let valid = errors.is_empty();
    let source_blob_verified = SOURCE_PINS
        .iter()
        .filter(|pin| pin.verification == "local_git_blob_at_pinned_commit")
        .count();
    let report = json!({
        "report_version": "luminous.lnf-e2e-manifest-validation-report.v1",
        "profile_id": manifest.get("profile_id").and_then(Value::as_str),
        "manifest_sha256": digest_hex(&manifest_bytes),
        "schema_sha256": digest_hex(&schema_bytes),
        "broker_profile_sha256": digest_hex(&broker_profile_bytes),
        "source_bindings": SOURCE_PINS.len(),
        "local_git_blob_sources_verified": source_blob_verified,
        "external_pins_materialized": false,
        "scenario_count": manifest.get("scenario_ids").and_then(Value::as_array).map_or(0, Vec::len),
        "valid": valid,
        "validation_scope": "draft_manifest_schema_plus_exact_source_pins_plus_campaign_invariants",
        "execution_conducted": false,
        "measurement_truth_conveyed": false,
        "hardware_qualification_conveyed": false,
        "field_evidence_conveyed": false,
        "production_readiness_conveyed": false,
        "deployment_authorization_conveyed": false,
        "errors": errors,
    });

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    if let Err(error) = serde_json::to_writer_pretty(&mut lock, &report) {
        eprintln!("cannot write validation report: {error}");
        process::exit(2);
    }
    if let Err(error) = lock.write_all(b"\n") {
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

    fn fixtures() -> (Value, Value, Value, PathBuf) {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let schema: Value = serde_json::from_str(include_str!(
            "../../../../schemas/lnf-e2e-run-manifest-v1.schema.json"
        ))
        .expect("checked-in schema parses");
        let manifest: Value = serde_json::from_str(include_str!(
            "../../../../profiles/lnf-e2e-run-manifest-v1.json"
        ))
        .expect("checked-in manifest parses");
        let profile_bytes = git_show(
            &repo_root,
            BROKER_COMMIT,
            "profiles/connectivity-broker-benchmark-profile-v1.json",
        )
        .expect("pinned broker profile is in Git history");
        let profile: Value = serde_json::from_slice(&profile_bytes).expect("pinned profile parses");
        (schema, manifest, profile, repo_root)
    }

    #[test]
    fn checked_in_manifest_and_all_local_source_pins_pass() {
        let (schema, manifest, profile, root) = fixtures();
        let errors = validate_manifest(&schema, &manifest, &profile, &root);
        assert!(errors.is_empty(), "validation errors: {errors:#?}");
    }

    #[test]
    fn broker_source_commit_drift_is_rejected() {
        let (schema, mut manifest, profile, root) = fixtures();
        manifest["source_bindings"][0]["commit_sha"] = json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let errors = validate_manifest(&schema, &manifest, &profile, &root);
        assert!(errors.iter().any(|error| error.contains("commit_sha")));
    }

    #[test]
    fn claim_ceiling_promotion_is_rejected() {
        let (schema, mut manifest, profile, root) = fixtures();
        manifest["claim_ceiling"]["field_observed"] = json!(true);
        let errors = validate_manifest(&schema, &manifest, &profile, &root);
        assert!(errors.iter().any(|error| error.contains("claim_ceiling/field_observed")));
    }

    #[test]
    fn required_outage_scenario_cannot_be_omitted() {
        let (schema, mut manifest, profile, root) = fixtures();
        manifest["scenario_ids"].as_array_mut().expect("array").pop();
        let errors = validate_manifest(&schema, &manifest, &profile, &root);
        assert!(errors.iter().any(|error| error.contains("scenario_ids")));
    }

    #[test]
    fn duplicate_wan_failure_domains_are_rejected() {
        let (schema, mut manifest, profile, root) = fixtures();
        manifest["topology"]["links"][2]["failure_domain"] =
            json!("wan-a-independent-netem");
        let errors = validate_manifest(&schema, &manifest, &profile, &root);
        assert!(errors.iter().any(|error| error.contains("failure domains")));
    }

    #[test]
    fn benchmark_id_not_present_in_pinned_profile_is_rejected() {
        let (schema, mut manifest, profile, root) = fixtures();
        manifest["selected_benchmarks"][0] = json!("NOT_A_REAL_BENCHMARK");
        let errors = validate_manifest(&schema, &manifest, &profile, &root);
        assert!(errors.iter().any(|error| error.contains("selected_benchmarks")));
    }

    #[test]
    fn external_mycelix_pin_is_not_misrepresented_as_locally_verified() {
        let (schema, mut manifest, profile, root) = fixtures();
        manifest["source_bindings"][3]["verification_method"] =
            json!("local_git_blob_at_pinned_commit");
        let errors = validate_manifest(&schema, &manifest, &profile, &root);
        assert!(errors.iter().any(|error| error.contains("verification_method")));
    }

    #[test]
    fn raw_payload_capture_is_rejected() {
        let (schema, mut manifest, profile, root) = fixtures();
        manifest["workloads"][0]["raw_payload_capture"] = json!(true);
        let errors = validate_manifest(&schema, &manifest, &profile, &root);
        assert!(!errors.is_empty());
    }

    #[test]
    fn remote_schema_references_are_rejected() {
        let (mut schema, manifest, profile, root) = fixtures();
        schema["properties"]["status"]["$ref"] = json!("https://example.invalid/remote.json");
        let errors = validate_manifest(&schema, &manifest, &profile, &root);
        assert!(errors.iter().any(|error| error.contains("only local fragment references")));
    }
}
