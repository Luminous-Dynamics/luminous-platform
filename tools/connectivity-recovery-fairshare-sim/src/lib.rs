//! Synthetic one-window recovery comparison: first-come-first-served versus
//! weighted max-min byte grants. This is not a radio airtime model.

#![forbid(unsafe_code)]

use community_bandwidth_fairshare_core::{
    AllocationPolicy, PeerDemand, allocate as fair_share_allocate,
};
use connectivity_task_outcome_core::{sha256_hex, validate_and_report};
use jsonschema::draft202012;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const SCENARIO_VERSION: &str = "luminous.connectivity-recovery-fairshare-scenario.v1";
const RUN_VERSION: &str = "luminous.connectivity-task-outcome-run.v1";
const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;
const REQUIRED_MODES: [&str; 3] = ["online", "wan_partition", "route_recovery"];
const REQUIRED_PRIORITIES: [&str; 3] = ["essential", "interactive", "bulk"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TaskResult {
    succeeded: bool,
    elapsed_ms: u64,
    bytes_tx: u64,
    bytes_rx: u64,
    energy_millijoules: u64,
    charge_minor: u64,
}

fn str_field<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn u64_field(value: &Value, key: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{key} must be a non-negative integer"))
}

fn u16_field(value: &Value, key: &str) -> Result<u16, String> {
    let raw = u64_field(value, key)?;
    u16::try_from(raw).map_err(|_| format!("{key} must fit in u16"))
}

fn reject_remote_refs(value: &Value, path: &str, errors: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if key == "$ref" || key == "$dynamicRef" {
                    match child.as_str() {
                        Some(reference) if reference.starts_with('#') => {}
                        Some(_) => errors.push(format!(
                            "{path}/{key}: only local schema references are allowed"
                        )),
                        None => errors.push(format!("{path}/{key}: reference must be a string")),
                    }
                }
                reject_remote_refs(child, &format!("{path}/{key}"), errors);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                reject_remote_refs(child, &format!("{path}/{index}"), errors);
            }
        }
        _ => {}
    }
}

fn schema_errors(schema: &Value, value: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    reject_remote_refs(schema, "$scenario_schema", &mut errors);
    if !errors.is_empty() {
        return errors;
    }
    let meta = match jsonschema::meta::validator_for(schema) {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot build schema meta-validator: {error}")],
    };
    if !meta.evaluate(schema).flag().valid {
        return vec!["recovery scenario schema failed meta-validation".to_owned()];
    }
    let validator = match draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot build scenario validator: {error}")],
    };
    errors.extend(
        validator
            .iter_errors(value)
            .map(|error| format!("scenario: {error}")),
    );
    errors
}

fn priority_rank(priority: &str) -> u8 {
    match priority {
        "essential" => 3,
        "interactive" => 2,
        "bulk" => 1,
        _ => 0,
    }
}

fn ceil_kib(bytes: u64) -> u64 {
    let kib = bytes / 1024;
    if bytes.is_multiple_of(1024) {
        kib
    } else {
        kib + 1
    }
}

fn energy_cost(base: u64, transferred_bytes: u64, per_kib: u64) -> Result<u64, String> {
    ceil_kib(transferred_bytes)
        .checked_mul(per_kib)
        .and_then(|variable| base.checked_add(variable))
        .ok_or_else(|| "synthetic energy arithmetic overflowed".to_owned())
}

fn charge_cost(transferred_bytes: u64, per_kib: u64) -> Result<u64, String> {
    ceil_kib(transferred_bytes)
        .checked_mul(per_kib)
        .ok_or_else(|| "synthetic charge arithmetic overflowed".to_owned())
}

fn used_bytes_for_task(service_bytes: u64, grant_bytes: u64) -> u64 {
    service_bytes.min(grant_bytes)
}

/// Divides actual consumed service bytes into request/response accounting. A
/// partially funded task spends request bytes first; the remaining grant can
/// fund the response. This is an abstract atomic-task model, not packet replay.
fn split_transfer(service_bytes: u64, used_bytes: u64) -> (u64, u64) {
    let request_bytes = service_bytes / 2;
    let bytes_tx = used_bytes.min(request_bytes);
    let bytes_rx = used_bytes
        .saturating_sub(bytes_tx)
        .min(service_bytes - request_bytes);
    (bytes_tx, bytes_rx)
}

fn task_result(
    task: &Value,
    grant_bytes: u64,
    completion_latency_ms: u64,
    base_energy: u64,
    energy_per_kib: u64,
    charge_per_kib: u64,
) -> Result<TaskResult, String> {
    let service_bytes = u64_field(task, "service_bytes")?;
    let deadline = u64_field(task, "deadline_ms")?;
    let ttl = u64_field(task, "expires_after_ms")?;
    let used = used_bytes_for_task(service_bytes, grant_bytes);
    let succeeded = grant_bytes >= service_bytes
        && completion_latency_ms <= deadline
        && completion_latency_ms <= ttl;
    let (bytes_tx, bytes_rx) = split_transfer(service_bytes, used);
    let elapsed_ms = if succeeded {
        completion_latency_ms
    } else {
        deadline.min(ttl)
    };
    Ok(TaskResult {
        succeeded,
        elapsed_ms,
        bytes_tx,
        bytes_rx,
        energy_millijoules: energy_cost(base_energy, used, energy_per_kib)?,
        charge_minor: charge_cost(used, charge_per_kib)?,
    })
}

fn validate_semantics(scenario: &Value) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if str_field(scenario, "schema_version").ok() != Some(SCENARIO_VERSION) {
        errors.push(format!("schema_version must be {SCENARIO_VERSION}"));
    }
    let capacity = u64_field(scenario, "window_budget_bytes").unwrap_or(0);
    let reserve = u64_field(scenario, "essential_reserve_bytes").unwrap_or(u64::MAX);
    if capacity == 0 {
        errors.push("window_budget_bytes must be positive".to_owned());
    }
    if reserve > capacity {
        errors.push("essential_reserve_bytes cannot exceed window_budget_bytes".to_owned());
    }
    let completion_latency = u64_field(scenario, "completion_latency_ms").unwrap_or(u64::MAX);
    let tasks = match scenario.get("tasks").and_then(Value::as_array) {
        Some(items) => items,
        None => return Err(vec!["tasks must be an array".to_owned()]),
    };

    let mut task_ids = BTreeSet::new();
    let mut pair_ids = BTreeSet::new();
    let mut mode_priorities: BTreeMap<String, String> = BTreeMap::new();
    let mut mode_scenarios: BTreeMap<String, String> = BTreeMap::new();
    let mut peer_mode_pairs = BTreeSet::new();
    let mut peer_weights: BTreeMap<String, u16> = BTreeMap::new();
    let mut modes = BTreeSet::new();
    let mut priorities = BTreeSet::new();

    for (index, task) in tasks.iter().enumerate() {
        let task_id = str_field(task, "task_id").unwrap_or("");
        if !task_ids.insert(task_id.to_owned()) {
            errors.push(format!("tasks/{index}: duplicate task_id {task_id}"));
        }
        let pair_id = str_field(task, "pair_id").unwrap_or("");
        if !pair_ids.insert(pair_id.to_owned()) {
            errors.push(format!("tasks/{index}: duplicate pair_id {pair_id}"));
        }
        let mode = str_field(task, "mode").unwrap_or("");
        let priority = str_field(task, "priority").unwrap_or("");
        let peer = str_field(task, "peer_id").unwrap_or("");
        let scenario_id = str_field(task, "scenario_id").unwrap_or("");
        let weight = u16_field(task, "weight").unwrap_or(0);
        modes.insert(mode.to_owned());
        priorities.insert(priority.to_owned());

        if !REQUIRED_MODES.contains(&mode) {
            errors.push(format!("tasks/{index}: unsupported mode {mode}"));
        }
        if !REQUIRED_PRIORITIES.contains(&priority) {
            errors.push(format!("tasks/{index}: unsupported priority {priority}"));
        }
        if let Some(previous) = mode_priorities.insert(mode.to_owned(), priority.to_owned())
            && previous != priority
        {
            errors.push(format!(
                "mode {mode} mixes priorities; v1 requires one class per window"
            ));
        }
        if let Some(previous) = mode_scenarios.insert(mode.to_owned(), scenario_id.to_owned())
            && previous != scenario_id
        {
            errors.push(format!(
                "mode {mode} mixes scenario IDs in one shared recovery window"
            ));
        }
        if !peer_mode_pairs.insert((mode.to_owned(), peer.to_owned())) {
            errors.push(format!(
                "mode {mode} contains more than one task for peer {peer}"
            ));
        }
        if let Some(previous) = peer_weights.insert(peer.to_owned(), weight)
            && previous != weight
        {
            errors.push(format!(
                "peer {peer} has inconsistent weights across mode cells"
            ));
        }
        if u64_field(task, "service_bytes").unwrap_or(0) == 0 {
            errors.push(format!("tasks/{index}/service_bytes must be positive"));
        }
        if u64_field(task, "deadline_ms").unwrap_or(0) < completion_latency {
            errors.push(format!(
                "tasks/{index}: deadline_ms is shorter than the declared completion latency"
            ));
        }
        if u64_field(task, "expires_after_ms").unwrap_or(0) < completion_latency {
            errors.push(format!(
                "tasks/{index}: expires_after_ms is shorter than the declared completion latency"
            ));
        }
    }
    for mode in REQUIRED_MODES {
        if !modes.contains(mode) {
            errors.push(format!("required mode {mode} is absent"));
        }
    }
    for priority in REQUIRED_PRIORITIES {
        if !priorities.contains(priority) {
            errors.push(format!("required priority {priority} is absent"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        errors.sort();
        errors.dedup();
        Err(errors)
    }
}

fn sort_tasks(tasks: &[Value]) -> Vec<Value> {
    let mut ordered = tasks.to_vec();
    ordered.sort_by(|left, right| {
        str_field(left, "mode")
            .unwrap_or("")
            .cmp(str_field(right, "mode").unwrap_or(""))
            .then_with(|| {
                u64_field(left, "arrival_ms")
                    .unwrap_or(0)
                    .cmp(&u64_field(right, "arrival_ms").unwrap_or(0))
            })
            .then_with(|| {
                priority_rank(str_field(right, "priority").unwrap_or(""))
                    .cmp(&priority_rank(str_field(left, "priority").unwrap_or("")))
            })
            .then_with(|| {
                str_field(left, "task_id")
                    .unwrap_or("")
                    .cmp(str_field(right, "task_id").unwrap_or(""))
            })
    });
    ordered
}

fn fcfs_grants(tasks: &[Value], capacity: u64) -> Result<BTreeMap<String, u64>, String> {
    let mut left = capacity;
    let mut grants = BTreeMap::new();
    for task in tasks {
        let id = str_field(task, "task_id")?;
        let service = u64_field(task, "service_bytes")?;
        let grant = service.min(left);
        left -= grant;
        grants.insert(id.to_owned(), grant);
    }
    Ok(grants)
}

fn fair_share_grants(
    tasks: &[Value],
    capacity: u64,
    essential_reserve: u64,
) -> Result<(BTreeMap<String, u64>, u64, u64), String> {
    let mut demands = Vec::with_capacity(tasks.len());
    for task in tasks {
        let peer_id = str_field(task, "peer_id")?.to_owned();
        let weight = u16_field(task, "weight")?;
        let service = u64_field(task, "service_bytes")?;
        let priority = str_field(task, "priority")?;
        demands.push(PeerDemand {
            peer_id,
            weight,
            essential_demand_bytes: if priority == "essential" { service } else { 0 },
            ordinary_demand_bytes: if priority == "essential" { 0 } else { service },
        });
    }
    let allocation = fair_share_allocate(
        &AllocationPolicy {
            total_capacity_bytes: capacity,
            essential_reserve_bytes: essential_reserve,
        },
        &demands,
    )
    .map_err(|error| format!("fair-share allocation failed: {error}"))?;

    let by_peer: BTreeMap<String, (u64, u64)> = allocation
        .grants
        .iter()
        .map(|grant| {
            (
                grant.peer_id.clone(),
                (grant.essential_granted_bytes, grant.ordinary_granted_bytes),
            )
        })
        .collect();
    let mut per_task = BTreeMap::new();
    for task in tasks {
        let peer = str_field(task, "peer_id")?;
        let priority = str_field(task, "priority")?;
        let (essential, ordinary) = by_peer
            .get(peer)
            .copied()
            .ok_or_else(|| format!("allocator omitted peer {peer}"))?;
        per_task.insert(
            str_field(task, "task_id")?.to_owned(),
            if priority == "essential" {
                essential
            } else {
                ordinary
            },
        );
    }
    Ok((
        per_task,
        allocation.unallocated_bytes,
        allocation.total_essential_granted_bytes + allocation.total_ordinary_granted_bytes,
    ))
}

fn make_trial(task: &Value, arm: &str, result: TaskResult) -> Value {
    let task_id = str_field(task, "task_id").unwrap_or("invalid-task");
    json!({
        "trial_id": format!("{task_id}-{arm}"),
        "pair_id": task.get("pair_id"),
        "scenario_id": task.get("scenario_id"),
        "arm": arm,
        "priority": task.get("priority"),
        "mode": task.get("mode"),
        "outcome": if result.succeeded { "succeeded" } else { "failed" },
        "elapsed_ms": result.elapsed_ms,
        "bytes_tx": result.bytes_tx,
        "bytes_rx": result.bytes_rx,
        "energy_millijoules": result.energy_millijoules,
        "charge_minor": result.charge_minor
    })
}

fn jain_fairness_ppm(success_counts: &[u64]) -> Option<u64> {
    if success_counts.is_empty() {
        return None;
    }
    let sum: u128 = success_counts.iter().map(|value| u128::from(*value)).sum();
    let sum_squares: u128 = success_counts
        .iter()
        .map(|value| u128::from(*value) * u128::from(*value))
        .sum();
    if sum_squares == 0 {
        return None;
    }
    let numerator = sum * sum * 1_000_000;
    let denominator = (success_counts.len() as u128) * sum_squares;
    Some((numerator / denominator) as u64)
}

/// Validates the scenario and emits a task-outcome run comparing a deterministic
/// FCFS baseline with the weighted max-min fair-share candidate. Each mode is a
/// separate recovery-window batch, each with its own declared byte budget.
pub fn validate_and_simulate(
    scenario_schema: &Value,
    scenario: &Value,
    run_schema: &Value,
    profile_schema: &Value,
    profile_bytes: &[u8],
    profile: &Value,
) -> Result<(Value, Value), Vec<String>> {
    if profile_bytes.len() > MAX_INPUT_BYTES
        || serde_json::to_vec(scenario).map_or(true, |bytes| bytes.len() > MAX_INPUT_BYTES)
    {
        return Err(vec![
            "scenario or profile exceeds the bounded input size".to_owned(),
        ]);
    }
    let mut errors = schema_errors(scenario_schema, scenario);
    errors.extend(schema_errors(profile_schema, profile));
    if errors.is_empty() {
        if let Err(semantic) = validate_semantics(scenario) {
            errors.extend(semantic);
        }
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let tasks = scenario
        .get("tasks")
        .and_then(Value::as_array)
        .ok_or_else(|| vec!["tasks must be an array".to_owned()])?;
    let ordered = sort_tasks(tasks);
    let budget = u64_field(scenario, "window_budget_bytes").map_err(|error| vec![error])?;
    let reserve = u64_field(scenario, "essential_reserve_bytes").map_err(|error| vec![error])?;
    let latency = u64_field(scenario, "completion_latency_ms").map_err(|error| vec![error])?;
    let base_energy =
        u64_field(scenario, "base_attempt_energy_millijoules").map_err(|error| vec![error])?;
    let energy_per_kib =
        u64_field(scenario, "energy_millijoules_per_kib").map_err(|error| vec![error])?;
    let charge_per_kib =
        u64_field(scenario, "charge_minor_per_kib").map_err(|error| vec![error])?;

    let mut task_groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for task in &ordered {
        task_groups
            .entry(
                str_field(task, "mode")
                    .map_err(|error| vec![error])?
                    .to_owned(),
            )
            .or_default()
            .push(task.clone());
    }

    let mut trials = Vec::with_capacity(ordered.len() * 2);
    let mut fairness_summaries = Vec::new();
    for mode in REQUIRED_MODES {
        let group = task_groups
            .get(mode)
            .ok_or_else(|| vec![format!("mode {mode} has no task group")])?;
        let baseline_grants = fcfs_grants(group, budget).map_err(|error| vec![error])?;
        let (candidate_grants, candidate_unallocated, candidate_allocated) =
            fair_share_grants(group, budget, reserve).map_err(|error| vec![error])?;

        let mut baseline_peer_successes = BTreeMap::new();
        let mut candidate_peer_successes = BTreeMap::new();
        let mut baseline_actual_bytes = 0_u64;
        let mut candidate_actual_bytes = 0_u64;
        let mut baseline_successes = 0_u64;
        let mut candidate_successes = 0_u64;

        for task in group {
            let task_id = str_field(task, "task_id").map_err(|error| vec![error])?;
            let baseline_grant = baseline_grants.get(task_id).copied().unwrap_or(0);
            let candidate_grant = candidate_grants.get(task_id).copied().unwrap_or(0);
            let baseline_result = task_result(
                task,
                baseline_grant,
                latency,
                base_energy,
                energy_per_kib,
                charge_per_kib,
            )
            .map_err(|error| vec![error])?;
            let candidate_result = task_result(
                task,
                candidate_grant,
                latency,
                base_energy,
                energy_per_kib,
                charge_per_kib,
            )
            .map_err(|error| vec![error])?;

            baseline_successes += if baseline_result.succeeded { 1 } else { 0 };
            candidate_successes += if candidate_result.succeeded { 1 } else { 0 };
            let peer = str_field(task, "peer_id")
                .map_err(|error| vec![error])?
                .to_owned();
            baseline_peer_successes
                .insert(peer.clone(), if baseline_result.succeeded { 1 } else { 0 });
            candidate_peer_successes.insert(peer, if candidate_result.succeeded { 1 } else { 0 });
            baseline_actual_bytes = baseline_actual_bytes
                .checked_add(baseline_result.bytes_tx)
                .and_then(|sum| sum.checked_add(baseline_result.bytes_rx))
                .ok_or_else(|| vec!["baseline byte accounting overflows u64".to_owned()])?;
            candidate_actual_bytes = candidate_actual_bytes
                .checked_add(candidate_result.bytes_tx)
                .and_then(|sum| sum.checked_add(candidate_result.bytes_rx))
                .ok_or_else(|| vec!["candidate byte accounting overflows u64".to_owned()])?;

            trials.push(make_trial(task, "baseline", baseline_result));
            trials.push(make_trial(task, "candidate", candidate_result));
        }

        if baseline_actual_bytes > budget || candidate_actual_bytes > budget {
            return Err(vec![format!(
                "mode {mode}: actual transferred bytes exceed the recovery-window budget"
            )]);
        }
        let baseline_success_values: Vec<u64> = baseline_peer_successes.values().copied().collect();
        let candidate_success_values: Vec<u64> =
            candidate_peer_successes.values().copied().collect();
        fairness_summaries.push(json!({
            "mode": mode,
            "priority": group.first().and_then(|task| task.get("priority")),
            "peer_count": group.len(),
            "recovery_window_budget_bytes": budget,
            "essential_reserve_bytes": reserve,
            "baseline": {
                "successful_tasks": baseline_successes,
                "actual_transferred_bytes": baseline_actual_bytes,
                "unspent_window_bytes": budget - baseline_actual_bytes,
                "jain_peer_task_success_fairness_ppm": jain_fairness_ppm(&baseline_success_values)
            },
            "candidate_fair_share": {
                "successful_tasks": candidate_successes,
                "allocator_granted_bytes": candidate_allocated,
                "allocator_unallocated_bytes": candidate_unallocated,
                "actual_transferred_bytes": candidate_actual_bytes,
                "unspent_window_bytes": budget - candidate_actual_bytes,
                "jain_peer_task_success_fairness_ppm": jain_fairness_ppm(&candidate_success_values)
            }
        }));
    }

    trials.sort_by(|left, right| {
        str_field(left, "pair_id")
            .unwrap_or("")
            .cmp(str_field(right, "pair_id").unwrap_or(""))
            .then_with(|| {
                str_field(left, "arm")
                    .unwrap_or("")
                    .cmp(str_field(right, "arm").unwrap_or(""))
            })
    });

    let mut canonical_scenario = scenario.clone();
    canonical_scenario["tasks"] = json!(ordered);
    let scenario_bytes = serde_json::to_vec(&canonical_scenario)
        .map_err(|error| vec![format!("cannot canonicalize recovery scenario: {error}")])?;
    let scenario_sha = sha256_hex(&scenario_bytes);
    let run = json!({
        "schema_version": RUN_VERSION,
        "run_id": str_field(scenario, "run_id").map_err(|error| vec![error])?,
        "source": {
            "profile_sha256": sha256_hex(profile_bytes),
            "scenario_sha256": scenario_sha
        },
        "evidence_class": "synthetic_simulation",
        "currency": str_field(scenario, "currency").map_err(|error| vec![error])?,
        "environment": {},
        "trials": trials
    });
    let mut report = validate_and_report(run_schema, profile_schema, profile_bytes, profile, &run)?;
    report["simulation_id"] = scenario
        .get("simulation_id")
        .cloned()
        .unwrap_or(Value::Null);
    report["scenario_sha256"] = json!(scenario_sha);
    report["fair_share_recovery_windows"] = json!(fairness_summaries);
    Ok((run, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixtures {
        schema: Value,
        scenario: Value,
        run_schema: Value,
        profile_schema: Value,
        profile_bytes: Vec<u8>,
        profile: Value,
    }

    fn fixtures() -> Fixtures {
        let profile_bytes =
            include_bytes!("../../../profiles/connectivity-task-outcome-benchmark-profile-v1.json")
                .to_vec();
        Fixtures {
            schema: serde_json::from_str(include_str!(
                "../../../schemas/connectivity-recovery-fairshare-scenario-v1.schema.json"
            ))
            .unwrap(),
            scenario: serde_json::from_str(include_str!(
                "../../../examples/connectivity-recovery-fairshare-scenario-synthetic-v1.json"
            ))
            .unwrap(),
            run_schema: serde_json::from_str(include_str!(
                "../../../schemas/connectivity-task-outcome-run-v1.schema.json"
            ))
            .unwrap(),
            profile_schema: serde_json::from_str(include_str!(
                "../../../schemas/connectivity-task-outcome-profile-v1.schema.json"
            ))
            .unwrap(),
            profile: serde_json::from_slice(&profile_bytes).unwrap(),
            profile_bytes,
        }
    }

    fn simulate(f: &Fixtures, scenario: &Value) -> Result<(Value, Value), Vec<String>> {
        validate_and_simulate(
            &f.schema,
            scenario,
            &f.run_schema,
            &f.profile_schema,
            &f.profile_bytes,
            &f.profile,
        )
    }

    #[test]
    fn fixture_generates_valid_paired_receipt_and_fairness_report() {
        let f = fixtures();
        let (run, report) = simulate(&f, &f.scenario).unwrap();
        assert_eq!(run["evidence_class"], "synthetic_simulation");
        assert_eq!(run["source"]["scenario_sha256"], report["scenario_sha256"]);
        assert_eq!(report["trial_count"], 18);
        assert_eq!(report["pair_count"], 9);
        assert_eq!(report["paired_comparison"]["candidate_only_succeeded"], 6);
        assert_eq!(report["paired_comparison"]["baseline_only_succeeded"], 3);
    }

    #[test]
    fn fair_share_improves_synthetic_peer_completion_fairness() {
        let f = fixtures();
        let (_, report) = simulate(&f, &f.scenario).unwrap();
        for window in report["fair_share_recovery_windows"].as_array().unwrap() {
            assert_eq!(window["baseline"]["successful_tasks"], 1);
            assert_eq!(window["candidate_fair_share"]["successful_tasks"], 2);
            assert_eq!(
                window["baseline"]["jain_peer_task_success_fairness_ppm"],
                333333
            );
            assert_eq!(
                window["candidate_fair_share"]["jain_peer_task_success_fairness_ppm"],
                666666
            );
            assert!(
                window["candidate_fair_share"]["actual_transferred_bytes"]
                    .as_u64()
                    .unwrap()
                    <= 9000
            );
            assert_eq!(
                window["candidate_fair_share"]["allocator_granted_bytes"]
                    .as_u64()
                    .unwrap()
                    + window["candidate_fair_share"]["allocator_unallocated_bytes"]
                        .as_u64()
                        .unwrap(),
                9000
            );
            assert!(
                window["baseline"]["actual_transferred_bytes"]
                    .as_u64()
                    .unwrap()
                    <= 9000
            );
        }
    }

    #[test]
    fn invalid_window_or_reserve_is_rejected() {
        let f = fixtures();
        let mut scenario = f.scenario.clone();
        scenario["essential_reserve_bytes"] = json!(9001);
        assert!(simulate(&f, &scenario).is_err());
    }

    #[test]
    fn duplicate_peer_task_in_same_window_is_rejected() {
        let f = fixtures();
        let mut scenario = f.scenario.clone();
        scenario["tasks"][1]["peer_id"] = json!("peer-a");
        assert!(simulate(&f, &scenario).is_err());
    }

    #[test]
    fn output_is_deterministic_under_task_order_permutation() {
        let f = fixtures();
        let (run1, report1) = simulate(&f, &f.scenario).unwrap();
        let mut scenario = f.scenario.clone();
        let mut tasks = scenario["tasks"].as_array().unwrap().clone();
        tasks.reverse();
        scenario["tasks"] = json!(tasks);
        let (run2, report2) = simulate(&f, &scenario).unwrap();
        assert_eq!(run1, run2);
        assert_eq!(report1, report2);
    }

    #[test]
    fn no_peer_id_is_emitted_in_public_task_receipt() {
        let f = fixtures();
        let (run, _) = simulate(&f, &f.scenario).unwrap();
        let serialized = serde_json::to_string(&run).unwrap();
        assert!(!serialized.contains("peer-a"));
        assert!(!serialized.contains("peer-b"));
        assert!(!serialized.contains("peer-c"));
    }

    #[test]
    fn remote_schema_references_are_rejected() {
        let f = fixtures();
        let mut schema = f.schema.clone();
        schema["properties"]["simulation_id"]["$ref"] =
            json!("https://example.invalid/schema.json");
        assert!(!schema_errors(&schema, &f.scenario).is_empty());
    }
}
