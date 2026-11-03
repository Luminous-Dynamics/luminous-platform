use jsonschema::draft202012;
use serde_json::Value;
use std::{env, fs, path::PathBuf, process};

fn default_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn read(path: &PathBuf) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| {
        eprintln!("ERROR: cannot read {}: {error}", path.display());
        process::exit(2);
    })
}

fn validate(schema_bytes: &[u8], instance_bytes: &[u8]) -> Result<Value, String> {
    let schema: Value = serde_json::from_slice(schema_bytes)
        .map_err(|error| format!("invalid schema JSON: {error}"))?;
    let instance: Value = serde_json::from_slice(instance_bytes)
        .map_err(|error| format!("invalid plan JSON: {error}"))?;

    draft202012::meta::validate(&schema)
        .map_err(|error| format!("schema fails Draft 2020-12 meta-validation: {error}"))?;
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("cannot compile Draft 2020-12 schema: {error}"))?;
    let errors = validator
        .iter_errors(&instance)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(instance)
    } else {
        Err(format!("schema validation failed: {}", errors.join("; ")))
    }
}

fn value_at<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(root, |current, key| current.get(key))
}

fn string_at<'a>(root: &'a Value, path: &str) -> Option<&'a str> {
    value_at(root, path).and_then(Value::as_str)
}

fn is_null_or_missing(root: &Value, path: &str) -> bool {
    value_at(root, path).is_none_or(Value::is_null)
}

fn unresolved(plan: &Value) -> Vec<&'static str> {
    let mut missing = Vec::new();
    let required = [
        ("dataset.revision", "dataset.revision"),
        ("dataset.task_manifest_digest", "dataset.task_manifest_digest"),
        ("task_selection.task_ids_digest", "task_selection.task_ids_digest"),
        ("task_selection.domain_inclusion_manifest_digest", "task_selection.domain_inclusion_manifest_digest"),
        ("native_scoring.scorer_path", "native_scoring.scorer_path"),
        ("native_scoring.scorer_commit", "native_scoring.scorer_commit"),
        ("native_scoring.reference_fixture_digest", "native_scoring.reference_fixture_digest"),
        ("system_under_test.provider", "system_under_test.provider"),
        ("system_under_test.model_name", "system_under_test.model_name"),
        ("system_under_test.immutable_model_version", "system_under_test.immutable_model_version"),
        ("system_under_test.endpoint_or_artifact", "system_under_test.endpoint_or_artifact"),
        ("system_under_test.inference_config_digest", "system_under_test.inference_config_digest"),
        ("system_under_test.system_prompt_digest", "system_under_test.system_prompt_digest"),
        ("system_under_test.developer_prompt_digest", "system_under_test.developer_prompt_digest"),
        ("system_under_test.agent_framework", "system_under_test.agent_framework"),
        ("system_under_test.orchestrator_commit", "system_under_test.orchestrator_commit"),
        ("system_under_test.tool_manifest_digest", "system_under_test.tool_manifest_digest"),
        ("runner.run_command_digest", "runner.run_command_digest"),
        ("reporting.raw_receipt_digest", "reporting.raw_receipt_digest"),
        ("reporting.analysis_code_digest", "reporting.analysis_code_digest"),
        ("reference_reproduction.system_profile_digest", "reference_reproduction.system_profile_digest"),
        ("reference_reproduction.profile_artifact_uri", "reference_reproduction.profile_artifact_uri"),
    ];
    for (path, label) in required {
        if is_null_or_missing(plan, path) {
            missing.push(label);
        }
    }

    if let Some(images) = value_at(plan, "environment.container_images").and_then(Value::as_array) {
        if images.iter().any(|image| image.get("image_digest").is_none_or(Value::is_null)) {
            missing.push("environment.container_images[].image_digest");
        }
    } else {
        missing.push("environment.container_images");
    }
    if let Some(snapshots) = value_at(plan, "environment.database_snapshots").and_then(Value::as_array) {
        if snapshots.iter().any(|snapshot| snapshot.get("snapshot_digest").is_none_or(Value::is_null)) {
            missing.push("environment.database_snapshots[].snapshot_digest");
        }
    } else {
        missing.push("environment.database_snapshots");
    }
    if string_at(plan, "dataset.revision_status") != Some("verified") {
        missing.push("dataset.revision_status=verified");
    }
    if value_at(plan, "source.repository_commit_verified").and_then(Value::as_bool) != Some(true) {
        missing.push("source.repository_commit_verified=true");
    }
    if value_at(plan, "reference_reproduction.reproduced").and_then(Value::as_bool) != Some(true) {
        missing.push("reference_reproduction.reproduced=true");
    }
    missing
}

fn main() {
    let mut args = env::args_os().skip(1);
    let plan_path = args.next().map(PathBuf::from)
        .unwrap_or_else(|| default_path("../enterpriseops-replication.example.json"));
    let schema_path = args.next().map(PathBuf::from)
        .unwrap_or_else(|| default_path("../enterpriseops-replication.schema.json"));
    if args.next().is_some() {
        eprintln!("usage: enterpriseops-parity-lint [PLAN_JSON] [SCHEMA_JSON]");
        process::exit(2);
    }

    let plan = validate(&read(&schema_path), &read(&plan_path)).unwrap_or_else(|error| {
        eprintln!("ENTERPRISEOPS_PARITY_PLAN=INVALID: {error}");
        process::exit(2);
    });
    let plan_id = string_at(&plan, "plan_id").unwrap_or_default();
    let status = string_at(&plan, "status").unwrap_or_default();
    let source_commit = string_at(&plan, "source.repository_commit").unwrap_or_default();
    let missing = unresolved(&plan);

    println!("ENTERPRISEOPS_PARITY_SCHEMA=PASS");
    println!("PLAN_ID={plan_id}");
    println!("PLAN_STATUS={status}");
    println!("SOURCE_COMMIT={source_commit}");
    println!("PUBLISHED_REFERENCE_MODEL={}", string_at(&plan, "published_reference.model_name").unwrap_or_default());
    println!("PUBLISHED_REFERENCE_SCORE={:.3}%", value_at(&plan, "published_reference.published_score_fraction").and_then(Value::as_f64).unwrap_or_default() * 100.0);
    if missing.is_empty() {
        println!("EXECUTION_READINESS=READY");
    } else {
        println!("EXECUTION_READINESS=BLOCKED_UNPINNED");
        for path in missing {
            println!("UNRESOLVED={path}");
        }
    }
    println!("NOTE=Schema/readiness audit only; no EnterpriseOps-Gym container or model benchmark was executed.");
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAN: &[u8] = include_bytes!("../../enterpriseops-replication.example.json");
    const SCHEMA: &[u8] = include_bytes!("../../enterpriseops-replication.schema.json");

    fn parse_plan() -> Value {
        serde_json::from_slice(PLAN).expect("valid example plan")
    }

    #[test]
    fn planning_example_is_valid_but_correctly_blocked() {
        let plan = validate(SCHEMA, PLAN).expect("planning profile should be schema-valid");
        assert_eq!(string_at(&plan, "status"), Some("planning_unpinned"));
        let missing = unresolved(&plan);
        assert!(missing.contains(&"dataset.revision"));
        assert!(missing.contains(&"environment.container_images[].image_digest"));
        assert!(missing.contains(&"reference_reproduction.reproduced=true"));
        assert!(!missing.is_empty());
    }

    #[test]
    fn changing_status_without_pinning_all_inputs_is_rejected() {
        let mut plan = parse_plan();
        plan["status"] = serde_json::json!("ready_to_execute");
        let bytes = serde_json::to_vec(&plan).unwrap();
        assert!(validate(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn direct_comparison_requires_reference_arm_reproduction() {
        let mut plan = parse_plan();
        plan["reference_reproduction"]["reproduced"] = serde_json::json!(true);
        let missing = unresolved(&plan);
        assert!(missing.contains(&"reference_reproduction.system_profile_digest"));
        assert!(missing.contains(&"reference_reproduction.profile_artifact_uri"));
    }
}
