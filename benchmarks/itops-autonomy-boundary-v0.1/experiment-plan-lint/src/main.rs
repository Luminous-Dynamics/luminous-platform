use jsonschema::draft202012;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    env,
    fs,
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

fn validate_schema(instance_bytes: &[u8], schema_bytes: &[u8]) -> Result<Value, String> {
    let instance: Value = serde_json::from_slice(instance_bytes)
        .map_err(|error| format!("invalid instance JSON: {error}"))?;
    let schema: Value = serde_json::from_slice(schema_bytes)
        .map_err(|error| format!("invalid schema JSON: {error}"))?;

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
    if !errors.is_empty() {
        return Err(format!("schema validation failed: {}", errors.join("; ")));
    }
    Ok(instance)
}

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn array_at<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    value.get(key).and_then(Value::as_array)
}

#[derive(Debug, Clone, PartialEq)]
struct PlanSummary {
    plan_id: String,
    strata: usize,
    target_episodes: u64,
    weight_sum: f64,
    arms: usize,
    analysis_unit: String,
    zero_event_method: String,
}

fn audit_plan(plan_bytes: &[u8], schema_bytes: &[u8]) -> Result<PlanSummary, String> {
    let plan = validate_schema(plan_bytes, schema_bytes)?;

    let plan_id = str_at(&plan, "plan_id")
        .ok_or_else(|| "plan_id missing after schema validation".to_string())?
        .to_string();
    let sampling = plan.get("sampling").ok_or("sampling missing")?;
    let target_episodes = sampling
        .get("target_episode_count_per_arm")
        .and_then(Value::as_u64)
        .ok_or("sampling.target_episode_count_per_arm is not an unsigned integer")?;

    let strata = array_at(&plan, "strata").ok_or("strata must be an array")?;
    let mut stratum_ids = BTreeSet::new();
    let mut minimum_total = 0_u64;
    let mut weight_sum = 0.0_f64;
    for stratum in strata {
        let id = str_at(stratum, "stratum_id")
            .ok_or("stratum_id missing")?
            .to_string();
        if !stratum_ids.insert(id.clone()) {
            return Err(format!("duplicate stratum_id {id:?}"));
        }
        let minimum = stratum
            .get("minimum_episodes_per_arm")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("stratum {id:?} has invalid minimum_episodes"))?;
        minimum_total = minimum_total
            .checked_add(minimum)
            .ok_or("minimum episode total overflow")?;
        let weight = stratum
            .get("population_weight")
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("stratum {id:?} has invalid population_weight"))?;
        weight_sum += weight;
    }
    if minimum_total != target_episodes {
        return Err(format!(
            "stratum minimum_episodes_per_arm sum {minimum_total} does not equal target_episode_count_per_arm {target_episodes}"
        ));
    }
    if (weight_sum - 1.0).abs() > 1.0e-9 {
        return Err(format!("stratum population weights sum to {weight_sum}, expected 1.0"));
    }

    let arms = array_at(&plan, "arms").ok_or("arms must be an array")?;
    let roles = arms
        .iter()
        .map(|arm| {
            str_at(arm, "role")
                .ok_or_else(|| "arm role missing".to_string())
                .map(str::to_string)
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let expected_roles = BTreeSet::from([
        "runbook_baseline".to_string(),
        "ungated_synthetic_only_agent".to_string(),
        "exact_scope_gated_agent".to_string(),
    ]);
    if roles != expected_roles {
        return Err("arms must contain exactly one runbook, ungated-synthetic, and exact-scope-gated role".into());
    }
    let mut arm_profile_digests = BTreeSet::new();
    for arm in arms {
        let role = str_at(arm, "role").ok_or("arm role missing")?;
        let boundary = str_at(arm, "environment_boundary")
            .ok_or("arm environment_boundary missing")?;
        let profile_digest = str_at(arm, "system_profile_digest")
            .ok_or("arm system_profile_digest missing")?;
        if !arm_profile_digests.insert(profile_digest) {
            return Err("each experiment arm must pin a distinct system profile digest".into());
        }
        match role {
            "runbook_baseline" if boundary != "runbook_fixture" => {
                return Err("runbook baseline arm must use runbook_fixture".into());
            }
            "ungated_synthetic_only_agent" | "exact_scope_gated_agent"
                if boundary != "synthetic_only" =>
            {
                return Err(format!("{role} must remain synthetic_only"));
            }
            _ => {}
        }
    }

    let dependence = plan.get("dependence").ok_or("dependence missing")?;
    let analysis_unit = str_at(dependence, "analysis_unit")
        .ok_or("dependence.analysis_unit missing")?;
    let inference = plan.get("inference").ok_or("inference missing")?;
    let independent_unit = str_at(inference, "independent_unit")
        .ok_or("inference.independent_unit missing")?;
    if independent_unit != analysis_unit {
        return Err(format!(
            "inference.independent_unit {independent_unit:?} does not match dependence.analysis_unit {analysis_unit:?}"
        ));
    }
    let zero_event_method = str_at(inference, "zero_event_method")
        .ok_or("inference.zero_event_method missing")?;
    let expected_method = match analysis_unit {
        "episode" => "exact_clopper_pearson_one_sided",
        "cluster" => "cluster_conservative_one_sided",
        _ => return Err(format!("unsupported analysis unit {analysis_unit:?}")),
    };
    if zero_event_method != expected_method {
        return Err(format!(
            "analysis unit {analysis_unit:?} requires zero-event method {expected_method:?}, got {zero_event_method:?}"
        ));
    }

    let reset = plan.get("reset_contract").ok_or("reset_contract missing")?;
    let agent_state = str_at(reset, "agent_state").ok_or("reset agent_state missing")?;
    let memory_policy = str_at(dependence, "cross_episode_memory")
        .ok_or("dependence.cross_episode_memory missing")?;
    match (agent_state, memory_policy, analysis_unit) {
        ("reset_each_episode", "disabled", "episode") => {}
        ("cluster_and_report_carryover", "treated_as_shared_cluster", "cluster") => {}
        _ => {
            return Err(
                "agent-state reset, cross-episode memory, and analysis unit are inconsistent".into()
            );
        }
    }

    let comparison = plan.get("comparison").ok_or("comparison missing")?;
    let block_keys = array_at(comparison, "block_keys").ok_or("comparison.block_keys missing")?;
    let block_keys = block_keys.iter().filter_map(Value::as_str).collect::<BTreeSet<_>>();
    if !block_keys.contains("scenario_variant_digest") || !block_keys.contains("stratum_id") {
        return Err("paired block keys must include scenario_variant_digest and stratum_id".into());
    }

    Ok(PlanSummary {
        plan_id,
        strata: strata.len(),
        target_episodes,
        weight_sum,
        arms: arms.len(),
        analysis_unit: analysis_unit.to_string(),
        zero_event_method: zero_event_method.to_string(),
    })
}

fn main() {
    let mut args = env::args_os().skip(1);
    let plan_path = args.next().map(PathBuf::from)
        .unwrap_or_else(|| default_path("../experiment-plan.example.json"));
    let schema_path = args.next().map(PathBuf::from)
        .unwrap_or_else(|| default_path("../experiment-plan.schema.json"));
    if args.next().is_some() {
        eprintln!("usage: itops-experiment-plan-lint [PLAN_JSON] [SCHEMA_JSON]");
        process::exit(2);
    }

    let plan_bytes = read(&plan_path);
    let schema_bytes = read(&schema_path);
    let summary = audit_plan(&plan_bytes, &schema_bytes).unwrap_or_else(|error| {
        eprintln!("ITOPS_EXPERIMENT_PLAN=FAIL: {error}");
        process::exit(2);
    });

    println!("ITOPS_EXPERIMENT_PLAN=PASS");
    println!("PLAN_ID={}", summary.plan_id);
    println!("STRATA={}", summary.strata);
    println!("TARGET_EPISODES={}", summary.target_episodes);
    println!("POPULATION_WEIGHT_SUM={:.12}", summary.weight_sum);
    println!("ARMS={}", summary.arms);
    println!("ANALYSIS_UNIT={}", summary.analysis_unit);
    println!("ZERO_EVENT_METHOD={}", summary.zero_event_method);
    println!("NOTE=Schema and cross-field preregistration checks only; no experiment was executed.");
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAN: &[u8] = include_bytes!("../../experiment-plan.example.json");
    const SCHEMA: &[u8] = include_bytes!("../../experiment-plan.schema.json");

    fn parsed_plan() -> Value {
        serde_json::from_slice(PLAN).expect("valid example plan")
    }

    fn encode(value: &Value) -> Vec<u8> {
        serde_json::to_vec(value).expect("serialize plan")
    }

    #[test]
    fn preregistered_example_passes_schema_and_semantic_audit() {
        let result = audit_plan(PLAN, SCHEMA).expect("example plan should pass");
        assert_eq!(result.target_episodes, 3000);
        assert_eq!(result.strata, 3);
        assert_eq!(result.arms, 3);
        assert_eq!(result.analysis_unit, "episode");
        assert_eq!(result.zero_event_method, "exact_clopper_pearson_one_sided");
    }

    #[test]
    fn rejects_stratum_weights_that_do_not_sum_to_one() {
        let mut plan = parsed_plan();
        plan["strata"][0]["population_weight"] = serde_json::json!(0.6);
        assert!(audit_plan(&encode(&plan), SCHEMA).is_err());
    }

    #[test]
    fn rejects_stratum_episode_total_that_differs_from_campaign_target() {
        let mut plan = parsed_plan();
        plan["strata"][0]["minimum_episodes_per_arm"] = serde_json::json!(1499);
        assert!(audit_plan(&encode(&plan), SCHEMA).is_err());
    }

    #[test]
    fn rejects_episode_independence_claim_with_cluster_interval_method() {
        let mut plan = parsed_plan();
        plan["inference"]["zero_event_method"] =
            serde_json::json!("cluster_conservative_one_sided");
        assert!(audit_plan(&encode(&plan), SCHEMA).is_err());
    }

    #[test]
    fn rejects_cross_episode_memory_without_cluster_analysis() {
        let mut plan = parsed_plan();
        plan["reset_contract"]["agent_state"] =
            serde_json::json!("cluster_and_report_carryover");
        plan["dependence"]["cross_episode_memory"] = serde_json::json!("treated_as_shared_cluster");
        assert!(audit_plan(&encode(&plan), SCHEMA).is_err());
    }

    #[test]
    fn rejects_duplicate_arm_roles() {
        let mut plan = parsed_plan();
        plan["arms"][2]["role"] = serde_json::json!("runbook_baseline");
        assert!(audit_plan(&encode(&plan), SCHEMA).is_err());
    }

    #[test]
    fn rejects_non_synthetic_gated_agent_arm() {
        let mut plan = parsed_plan();
        plan["arms"][2]["environment_boundary"] = serde_json::json!("runbook_fixture");
        assert!(audit_plan(&encode(&plan), SCHEMA).is_err());
    }

    #[test]
    fn rejects_duplicate_system_profile_digests_across_arms() {
        let mut plan = parsed_plan();
        plan["arms"][2]["system_profile_digest"] =
            plan["arms"][1]["system_profile_digest"].clone();
        assert!(audit_plan(&encode(&plan), SCHEMA).is_err());
    }

    #[test]
    fn rejects_paired_comparison_missing_scenario_block_key() {
        let mut plan = parsed_plan();
        plan["comparison"]["block_keys"] = serde_json::json!(["stratum_id", "seed_lineage_id"]);
        assert!(audit_plan(&encode(&plan), SCHEMA).is_err());
    }
}
