use jsonschema::draft202012;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::PathBuf,
    process,
};

fn default_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn read(path: &PathBuf) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| {
        eprintln!("ERROR: cannot read {}: {error}", path.display());
        process::exit(2);
    })
}

fn validate_schema(schema_bytes: &[u8], stream_bytes: &[u8]) -> Result<Value, String> {
    let schema: Value = serde_json::from_slice(schema_bytes)
        .map_err(|error| format!("invalid schema JSON: {error}"))?;
    let stream: Value = serde_json::from_slice(stream_bytes)
        .map_err(|error| format!("invalid stream JSON: {error}"))?;

    draft202012::meta::validate(&schema)
        .map_err(|error| format!("schema fails Draft 2020-12 meta-validation: {error}"))?;
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("cannot compile Draft 2020-12 schema: {error}"))?;
    let errors = validator
        .iter_errors(&stream)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(stream)
    } else {
        Err(format!("schema validation failed: {}", errors.join("; ")))
    }
}

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn u64_at(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn array_at<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    value.get(key).and_then(Value::as_array)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ToolState {
    version: u64,
    schema_digest: String,
    authority_scope_digest: String,
    lifecycle: String,
    authorization_state: String,
    grant_binding_digest: Option<String>,
}

fn tool_state(value: &Value) -> Result<ToolState, String> {
    let grant = value.get("grant_binding_digest")
        .filter(|v| !v.is_null())
        .and_then(Value::as_str)
        .map(str::to_string);

    Ok(ToolState {
        version: u64_at(value, "version").ok_or("tool.version missing")?,
        schema_digest: str_at(value, "schema_digest").ok_or("tool.schema_digest missing")?.into(),
        authority_scope_digest: str_at(value, "authority_scope_digest")
            .ok_or("tool.authority_scope_digest missing")?.into(),
        lifecycle: str_at(value, "lifecycle").ok_or("tool.lifecycle missing")?.into(),
        authorization_state: str_at(value, "authorization_state")
            .ok_or("tool.authorization_state missing")?.into(),
        grant_binding_digest: grant,
    })
}

fn tools_by_id(stage: &Value) -> Result<BTreeMap<String, ToolState>, String> {
    let tools = array_at(stage, "tools").ok_or("stage.tools must be an array")?;
    let mut result = BTreeMap::new();
    for value in tools {
        let id = str_at(value, "capability_id").ok_or("tool.capability_id missing")?.to_string();
        let state = tool_state(value)?;
        if result.insert(id.clone(), state).is_some() {
            return Err(format!("duplicate capability_id {id:?} in stage"));
        }
    }
    Ok(result)
}

fn find_event<'a>(
    stage: &'a Value,
    kind: &str,
    capability_id: &str,
) -> Option<&'a Value> {
    array_at(stage, "capability_events")?.iter().find(|event| {
        str_at(event, "kind") == Some(kind)
            && str_at(event, "capability_id") == Some(capability_id)
    })
}

fn event_versions_match(
    event: &Value,
    from: Option<u64>,
    to: Option<u64>,
) -> bool {
    let actual_from = event.get("from_version").and_then(Value::as_u64);
    let actual_to = event.get("to_version").and_then(Value::as_u64);
    actual_from == from && actual_to == to
}

fn require_event(
    stage: &Value,
    kind: &str,
    id: &str,
    from: Option<u64>,
    to: Option<u64>,
) -> Result<(), String> {
    let event = find_event(stage, kind, id)
        .ok_or_else(|| format!("stage {} missing {kind} event for capability {id:?}",
            str_at(stage, "stage_id").unwrap_or("<unknown>")))?;
    if !event_versions_match(event, from, to) {
        return Err(format!(
            "stage {} has incorrect {kind} event versions for {id:?}",
            str_at(stage, "stage_id").unwrap_or("<unknown>")
        ));
    }
    Ok(())
}

fn audit_stage_chain(stream: &Value) -> Result<(usize, usize), String> {
    let stages = array_at(stream, "stages").ok_or("stages missing")?;
    let mut stage_ids = BTreeSet::new();
    let mut prior_tools: Option<BTreeMap<String, ToolState>> = None;
    let mut prior_stage_id: Option<String> = None;
    let mut prior_retention_digest: Option<String> = None;
    let mut prior_authority_digest: Option<String> = None;
    let mut prior_tool_manifest_digest: Option<String> = None;
    let mut prior_adaptation_digests = BTreeSet::new();
    let mut prior_heldout_digests = BTreeSet::new();
    let mut total_events = 0usize;

    for (index, stage) in stages.iter().enumerate() {
        let actual_index = u64_at(stage, "stage_index").ok_or("stage_index missing")?;
        if actual_index != index as u64 {
            return Err(format!("stage index must be contiguous: position {index}, found {actual_index}"));
        }
        let stage_id = str_at(stage, "stage_id").ok_or("stage_id missing")?.to_string();
        if !stage_ids.insert(stage_id.clone()) {
            return Err(format!("duplicate stage_id {stage_id:?}"));
        }
        let parent = stage.get("parent_stage_id").filter(|v| !v.is_null()).and_then(Value::as_str);
        if index == 0 {
            if parent.is_some() {
                return Err("first stage must have null parent_stage_id".into());
            }
        } else if parent != prior_stage_id.as_deref() {
            return Err(format!("stage {stage_id:?} parent does not match previous stage"));
        }

        let component_digests = stage.get("component_digests").ok_or("component_digests missing")?;
        let authority_digest = str_at(component_digests, "authority_policy")
            .ok_or("authority_policy digest missing")?.to_string();
        let tool_manifest_digest = str_at(component_digests, "tool_manifest")
            .ok_or("tool_manifest digest missing")?.to_string();
        let retention_digest = str_at(
            stage.get("task_sets").ok_or("task_sets missing")?,
            "retention_task_manifest_digest"
        ).ok_or("retention_task_manifest_digest missing")?.to_string();
        let adaptation_digest = str_at(stage.get("task_sets").unwrap(), "adaptation_task_manifest_digest")
            .ok_or("adaptation_task_manifest_digest missing")?.to_string();
        let heldout_digest = str_at(stage.get("task_sets").unwrap(), "heldout_task_manifest_digest")
            .ok_or("heldout_task_manifest_digest missing")?.to_string();

        if let Some(previous) = &prior_retention_digest {
            if previous != &retention_digest {
                return Err(format!("retention task manifest changed at stage {stage_id:?}"));
            }
        }
        if prior_adaptation_digests.contains(&adaptation_digest)
            || prior_heldout_digests.contains(&heldout_digest)
            || prior_adaptation_digests.contains(&heldout_digest)
            || prior_heldout_digests.contains(&adaptation_digest)
        {
            return Err(format!("adaptation/held-out task manifest was reused at stage {stage_id:?}"));
        }
        prior_adaptation_digests.insert(adaptation_digest);
        prior_heldout_digests.insert(heldout_digest);

        let transition = str_at(stage, "authority_transition").ok_or("authority_transition missing")?;
        if let Some(previous) = &prior_authority_digest {
            let changed = previous != &authority_digest;
            if (transition == "unchanged") == changed {
                return Err(format!(
                    "authority policy digest and authority_transition disagree at stage {stage_id:?}"
                ));
            }
        }

        let current_tools = tools_by_id(stage)?;
        let events = array_at(stage, "capability_events").ok_or("capability_events missing")?;
        total_events += events.len();
        let mut event_keys = BTreeSet::new();
        for event in events {
            let kind = str_at(event, "kind").ok_or("event.kind missing")?;
            let id = str_at(event, "capability_id").ok_or("event.capability_id missing")?;
            if !event_keys.insert((kind.to_string(), id.to_string())) {
                return Err(format!("duplicate {kind} event for {id:?} at stage {stage_id:?}"));
            }
        }

        if let Some(previous_tools) = prior_tools.as_ref() {
            let mut any_tool_manifest_change = previous_tools.len() != current_tools.len();
            let mut authority_expanded = false;
            let mut authority_narrowed = false;
            let mut authority_scope_changed = false;
            for (id, current) in &current_tools {
                match previous_tools.get(id) {
                    None => {
                        any_tool_manifest_change = true;
                        require_event(stage, "added", id, None, Some(current.version))?;
                        if current.authorization_state == "explicit_grant" {
                            if transition != "expanded_with_explicit_grants"
                                || stage.get("explicit_grant_manifest_digest").is_none_or(Value::is_null)
                            {
                                return Err(format!(
                                    "new capability {id:?} has a grant without an explicit authority-expansion manifest"
                                ));
                            }
                        }
                    }
                    Some(previous) => {
                        if current.schema_digest != previous.schema_digest {
                            any_tool_manifest_change = true;
                            if current.version != previous.version + 1 {
                                return Err(format!(
                                    "schema change for {id:?} must increment version exactly once"
                                ));
                            }
                            require_event(
                                stage, "schema_changed", id,
                                Some(previous.version), Some(current.version)
                            )?;
                        } else if current.version != previous.version {
                            return Err(format!(
                                "version changed without schema digest change for capability {id:?}"
                            ));
                        }

                        let authority_changed =
                            current.authority_scope_digest != previous.authority_scope_digest
                            || current.authorization_state != previous.authorization_state
                            || current.grant_binding_digest != previous.grant_binding_digest;
                        if authority_changed {
                            require_event(
                                stage, "authority_changed", id,
                                Some(previous.version), Some(current.version)
                            )?;
                            if current.authority_scope_digest != previous.authority_scope_digest {
                                authority_scope_changed = true;
                            }
                            match (
                                previous.authorization_state.as_str(),
                                current.authorization_state.as_str()
                            ) {
                                ("no_grant", "explicit_grant") => authority_expanded = true,
                                ("explicit_grant", "no_grant") => authority_narrowed = true,
                                _ => {}
                            }
                        }
                        if current.lifecycle == "deprecated"
                            && current.authorization_state == "explicit_grant"
                        {
                            return Err(format!(
                                "deprecated capability {id:?} must not retain an active explicit grant"
                            ));
                        }

                        if previous.lifecycle == "active" && current.lifecycle == "deprecated" {
                            any_tool_manifest_change = true;
                            require_event(
                                stage, "deprecated", id,
                                Some(previous.version), Some(current.version)
                            )?;
                        } else if previous.lifecycle == "deprecated" && current.lifecycle == "active" {
                            return Err(format!("deprecated capability {id:?} was silently reactivated"));
                        }
                    }
                }
            }

            for (id, previous) in previous_tools {
                if !current_tools.contains_key(id) {
                    any_tool_manifest_change = true;
                    require_event(stage, "removed", id, Some(previous.version), None)?;
                }
            }

            if authority_expanded && transition != "expanded_with_explicit_grants" {
                return Err(format!(
                    "grant addition at stage {stage_id:?} requires expanded_with_explicit_grants"
                ));
            }
            if authority_narrowed && !matches!(transition, "narrowed" | "revoked") {
                return Err(format!(
                    "grant removal at stage {stage_id:?} requires narrowed or revoked transition"
                ));
            }
            if authority_scope_changed && transition == "unchanged" {
                return Err(format!(
                    "authority scope digest changed at stage {stage_id:?} under unchanged transition"
                ));
            }
            if transition == "expanded_with_explicit_grants"
                && stage.get("explicit_grant_manifest_digest").is_none_or(Value::is_null)
            {
                return Err(format!(
                    "authority expansion at stage {stage_id:?} requires an explicit grant manifest digest"
                ));
            }

            if any_tool_manifest_change
                && prior_tool_manifest_digest.as_deref() == Some(tool_manifest_digest.as_str())
            {
                return Err(format!("tool manifest digest did not change despite capability changes at stage {stage_id:?}"));
            }

            for event in events {
                let kind = str_at(event, "kind").unwrap_or_default();
                let id = str_at(event, "capability_id").unwrap_or_default();
                let old = previous_tools.get(id);
                let new = current_tools.get(id);
                let valid = match kind {
                    "added" => old.is_none() && new.is_some(),
                    "schema_changed" => old.zip(new).is_some_and(|(a,b)| {
                        a.schema_digest != b.schema_digest
                            && event_versions_match(event, Some(a.version), Some(b.version))
                    }),
                    "authority_changed" => old.zip(new).is_some_and(|(a,b)| {
                        (a.authority_scope_digest != b.authority_scope_digest
                            || a.authorization_state != b.authorization_state
                            || a.grant_binding_digest != b.grant_binding_digest)
                            && event_versions_match(event, Some(a.version), Some(b.version))
                    }),
                    "deprecated" => old.zip(new).is_some_and(|(a,b)| {
                        a.lifecycle == "active" && b.lifecycle == "deprecated"
                            && event_versions_match(event, Some(a.version), Some(b.version))
                    }),
                    "removed" => old.is_some() && new.is_none()
                        && event_versions_match(event, old.map(|v| v.version), None),
                    _ => false,
                };
                if !valid {
                    return Err(format!("orphan or inconsistent {kind} event for capability {id:?} at stage {stage_id:?}"));
                }
            }
        } else {
            for (id, tool) in &current_tools {
                require_event(stage, "added", id, None, Some(tool.version))?;
                if tool.authorization_state == "explicit_grant"
                    && tool.grant_binding_digest.is_none()
                {
                    return Err(format!("baseline grant for {id:?} is missing grant_binding_digest"));
                }
            }
            if events.len() != current_tools.len() {
                return Err("initial stage events must contain exactly one added event per baseline capability".into());
            }
        }

        prior_tools = Some(current_tools);
        prior_stage_id = Some(stage_id);
        prior_retention_digest = Some(retention_digest);
        prior_authority_digest = Some(authority_digest);
        prior_tool_manifest_digest = Some(tool_manifest_digest);
    }

    Ok((stages.len(), total_events))
}

fn audit_arms(stream: &Value) -> Result<usize, String> {
    let arms = array_at(stream, "arms").ok_or("arms missing")?;
    let mut roles = BTreeSet::new();
    let mut profile_digests = BTreeSet::new();
    let mut arm_ids = BTreeSet::new();
    for arm in arms {
        let id = str_at(arm, "arm_id").ok_or("arm_id missing")?;
        let role = str_at(arm, "role").ok_or("arm.role missing")?;
        let digest = str_at(arm, "system_profile_digest").ok_or("system_profile_digest missing")?;
        if !arm_ids.insert(id.to_string()) {
            return Err(format!("duplicate arm_id {id:?}"));
        }
        if !roles.insert(role.to_string()) {
            return Err(format!("duplicate arm role {role:?}"));
        }
        if !profile_digests.insert(digest.to_string()) {
            return Err("every arm must pin a distinct system profile digest".into());
        }
        if str_at(arm, "environment_boundary") != Some("synthetic_only") {
            return Err(format!("arm {id:?} is not synthetic_only"));
        }
        let weights = str_at(arm, "model_weights_policy").ok_or("model_weights_policy missing")?;
        let scope = str_at(arm, "adaptation_scope").ok_or("adaptation_scope missing")?;
        match role {
            "static_harness_control" | "updated_harness_no_adaptation"
                if weights != "fixed" || scope != "none" =>
            {
                return Err(format!("arm {role:?} must have fixed weights and no adaptation"));
            }
            "continual_adaptation" if scope == "none" => {
                return Err("continual_adaptation arm must adapt at least one harness component".into());
            }
            "frozen_model_evolving_harness" if weights != "fixed" || scope != "full_harness" => {
                return Err("frozen_model_evolving_harness must hold model weights fixed and adapt the harness".into());
            }
            "frozen_model_evolving_harness" | "continual_adaptation" |
            "static_harness_control" | "updated_harness_no_adaptation" => {}
            _ => return Err(format!("unsupported arm role {role:?}")),
        }
    }

    for required in [
        "static_harness_control",
        "updated_harness_no_adaptation",
        "continual_adaptation",
    ] {
        if !roles.contains(required) {
            return Err(format!("missing required arm role {required:?}"));
        }
    }
    Ok(arms.len())
}

#[derive(Debug, PartialEq)]
struct Summary {
    stream_id: String,
    stages: usize,
    arms: usize,
    capability_events: usize,
    status: String,
}

fn audit(bytes: &[u8], schema_bytes: &[u8]) -> Result<Summary, String> {
    let stream = validate_schema(schema_bytes, bytes)?;
    let (stages, capability_events) = audit_stage_chain(&stream)?;
    let arms = audit_arms(&stream)?;
    let evaluation = stream.get("evaluation").ok_or("evaluation missing")?;
    let minimum_retention = evaluation.get("min_retention_rate").and_then(Value::as_f64)
        .ok_or("min_retention_rate missing")?;
    let minimum_adaptation = evaluation.get("min_adaptation_rate").and_then(Value::as_f64)
        .ok_or("min_adaptation_rate missing")?;
    if minimum_retention < 0.90 {
        return Err("minimum retention gate must be at least 0.90".into());
    }
    if minimum_adaptation < 0.50 {
        return Err("minimum adaptation gate must be at least 0.50".into());
    }
    Ok(Summary {
        stream_id: str_at(&stream, "stream_id").ok_or("stream_id missing")?.into(),
        stages,
        arms,
        capability_events,
        status: str_at(&stream, "status").ok_or("status missing")?.into(),
    })
}

fn main() {
    let mut args = env::args_os().skip(1);
    let stream_path = args.next().map(PathBuf::from)
        .unwrap_or_else(|| default_path("../evoitops-harness-stream.example.json"));
    let schema_path = args.next().map(PathBuf::from)
        .unwrap_or_else(|| default_path("../evoitops-harness-stream.schema.json"));
    if args.next().is_some() {
        eprintln!("usage: evoitops-stream-lint [STREAM_JSON] [SCHEMA_JSON]");
        process::exit(2);
    }

    let summary = audit(&read(&stream_path), &read(&schema_path)).unwrap_or_else(|error| {
        eprintln!("EVOITOPS_STREAM=FAIL: {error}");
        process::exit(2);
    });
    println!("EVOITOPS_STREAM=PASS");
    println!("STREAM_ID={}", summary.stream_id);
    println!("STATUS={}", summary.status);
    println!("STAGES={}", summary.stages);
    println!("ARMS={}", summary.arms);
    println!("CAPABILITY_EVENTS={}", summary.capability_events);
    println!("SAFETY=authority_is_separate_from_capability_discovery");
    println!("NOTE=Schema and semantic stage audit only; no agent campaign was executed.");
}

#[cfg(test)]
mod tests {
    use super::*;

    const STREAM: &[u8] = include_bytes!("../../evoitops-harness-stream.example.json");
    const SCHEMA: &[u8] = include_bytes!("../../evoitops-harness-stream.schema.json");

    fn parsed() -> Value {
        serde_json::from_slice(STREAM).expect("valid example stream")
    }

    fn encode(value: &Value) -> Vec<u8> {
        serde_json::to_vec(value).expect("serialize stream")
    }

    #[test]
    fn example_stream_passes_schema_and_semantic_audit() {
        let result = audit(STREAM, SCHEMA).expect("example stream should pass");
        assert_eq!(result.stages, 4);
        assert_eq!(result.arms, 4);
        assert_eq!(result.capability_events, 10);
        assert_eq!(result.status, "candidate_not_frozen");
    }

    #[test]
    fn new_tool_does_not_implicitly_receive_authority() {
        let mut stream = parsed();
        stream["stages"][1]["tools"][4]["authorization_state"] =
            serde_json::json!("explicit_grant");
        stream["stages"][1]["tools"][4]["grant_binding_digest"] =
            serde_json::json!("sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
        assert!(audit(&encode(&stream), SCHEMA).is_err());
    }

    #[test]
    fn schema_change_requires_version_increment_and_event() {
        let mut stream = parsed();
        stream["stages"][2]["tools"][2]["version"] = serde_json::json!(1);
        assert!(audit(&encode(&stream), SCHEMA).is_err());
    }

    #[test]
    fn retention_manifest_must_remain_constant() {
        let mut stream = parsed();
        stream["stages"][2]["task_sets"]["retention_task_manifest_digest"] =
            serde_json::json!("sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
        assert!(audit(&encode(&stream), SCHEMA).is_err());
    }

    #[test]
    fn stale_tool_removal_requires_explicit_event() {
        let mut stream = parsed();
        stream["stages"][3]["capability_events"] = serde_json::json!([]);
        assert!(audit(&encode(&stream), SCHEMA).is_err());
    }

    #[test]
    fn existing_tool_grant_cannot_expand_under_unchanged_transition() {
        let mut stream = parsed();
        stream["stages"][1]["tools"][0]["authorization_state"] =
            serde_json::json!("explicit_grant");
        stream["stages"][1]["tools"][0]["grant_binding_digest"] =
            serde_json::json!("sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
        stream["stages"][1]["capability_events"].as_array_mut().unwrap().push(
            serde_json::json!({
                "kind":"authority_changed",
                "capability_id":"read_health",
                "from_version":1,
                "to_version":1,
                "reason":"negative control"
            })
        );
        assert!(audit(&encode(&stream), SCHEMA).is_err());
    }

    #[test]
    fn deprecated_tool_cannot_keep_an_explicit_grant() {
        let mut stream = parsed();
        stream["stages"][2]["tools"][3]["authorization_state"] =
            serde_json::json!("explicit_grant");
        stream["stages"][2]["tools"][3]["grant_binding_digest"] =
            serde_json::json!("sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
        assert!(audit(&encode(&stream), SCHEMA).is_err());
    }

    #[test]
    fn authority_policy_cannot_change_under_unchanged_transition() {
        let mut stream = parsed();
        stream["stages"][1]["component_digests"]["authority_policy"] =
            serde_json::json!("sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
        assert!(audit(&encode(&stream), SCHEMA).is_err());
    }

    #[test]
    fn capability_grant_revocation_requires_event() {
        let mut stream = parsed();
        stream["stages"][2]["capability_events"] = serde_json::json!([
            {"kind":"schema_changed","capability_id":"restart_service","from_version":1,"to_version":2,"reason":"schema"}
        ]);
        assert!(audit(&encode(&stream), SCHEMA).is_err());
    }

    #[test]
    fn stage_parent_chain_must_be_contiguous() {
        let mut stream = parsed();
        stream["stages"][2]["parent_stage_id"] = serde_json::json!("H0-CORE");
        assert!(audit(&encode(&stream), SCHEMA).is_err());
    }

    #[test]
    fn arms_must_not_duplicate_profile_digests() {
        let mut stream = parsed();
        stream["arms"][3]["system_profile_digest"] =
            stream["arms"][2]["system_profile_digest"].clone();
        assert!(audit(&encode(&stream), SCHEMA).is_err());
    }
}
