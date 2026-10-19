//! Deterministic synthetic WAN-outage and recovery simulator.
//!
//! Compares a WAN-only baseline with a local-first candidate that can use a
//! bounded store-and-forward queue. This is not a radio model or measurement.

#![forbid(unsafe_code)]

use connectivity_task_outcome_core::{sha256_hex, validate_and_report};
use jsonschema::draft202012;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const SCENARIO_VERSION: &str = "luminous.connectivity-outage-scenario.v1";
const RUN_VERSION: &str = "luminous.connectivity-task-outcome-run.v1";
const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;
const MAX_TASKS: usize = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TrialOutcome {
    outcome: &'static str,
    elapsed_ms: u64,
    bytes_tx: u64,
    bytes_rx: u64,
    energy_millijoules: u64,
    charge_minor: u64,
}

fn string_field<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
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

fn bool_field(value: &Value, key: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{key} must be a boolean"))
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
                        None => {
                            errors.push(format!("{path}/{key}: schema reference must be a string"))
                        }
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
        Err(error) => {
            return vec![format!(
                "cannot create scenario schema meta-validator: {error}"
            )];
        }
    };
    if !meta.evaluate(schema).flag().valid {
        return vec!["scenario schema failed JSON Schema meta-validation".to_owned()];
    }
    let validator = match draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot create scenario validator: {error}")],
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

fn ceil_div(numerator: u128, denominator: u128) -> Result<u64, String> {
    if denominator == 0 {
        return Err("simulated link rate cannot be zero".to_owned());
    }
    let quotient = numerator / denominator;
    let result = if numerator.is_multiple_of(denominator) {
        quotient
    } else {
        quotient + 1
    };
    u64::try_from(result).map_err(|_| "simulated value overflows u64".to_owned())
}

fn path_duration_ms(
    payload_bytes: u64,
    latency_ms: u64,
    bytes_per_second: u64,
) -> Result<u64, String> {
    let transfer_ms = ceil_div(
        u128::from(payload_bytes) * 1000,
        u128::from(bytes_per_second),
    )?;
    latency_ms
        .checked_add(transfer_ms)
        .ok_or_else(|| "path duration overflows u64".to_owned())
}

fn payload_kib_ceil(payload_bytes: u64) -> u64 {
    let kib = payload_bytes / 1024;
    if payload_bytes.is_multiple_of(1024) {
        kib
    } else {
        kib + 1
    }
}

fn energy_cost(base: u64, payload_bytes: u64, per_kib: u64) -> Result<u64, String> {
    payload_kib_ceil(payload_bytes)
        .checked_mul(per_kib)
        .and_then(|variable| base.checked_add(variable))
        .ok_or_else(|| "synthetic energy arithmetic overflows u64".to_owned())
}

fn charge_cost(payload_bytes: u64, per_kib: u64) -> Result<u64, String> {
    payload_kib_ceil(payload_bytes)
        .checked_mul(per_kib)
        .ok_or_else(|| "synthetic charge arithmetic overflows u64".to_owned())
}

#[derive(Clone, Copy)]
struct LinkPath {
    latency_ms: u64,
    bytes_per_second: u64,
    energy_millijoules_per_kib: u64,
    charge_minor_per_kib: u64,
}

fn link_path(link: &Value, kind: &str) -> Result<LinkPath, String> {
    Ok(LinkPath {
        latency_ms: u64_field(link, &format!("{kind}_latency_ms"))?,
        bytes_per_second: u64_field(link, &format!("{kind}_bytes_per_second"))?,
        energy_millijoules_per_kib: u64_field(link, &format!("{kind}_energy_millijoules_per_kib"))?,
        charge_minor_per_kib: u64_field(link, &format!("{kind}_charge_minor_per_kib"))?,
    })
}

fn blocked_attempt(
    deadline_ms: u64,
    ttl_ms: u64,
    detection_ms: u64,
    base_energy: u64,
) -> TrialOutcome {
    TrialOutcome {
        outcome: "failed",
        elapsed_ms: deadline_ms.min(ttl_ms).min(detection_ms),
        bytes_tx: 0,
        bytes_rx: 0,
        energy_millijoules: base_energy,
        charge_minor: 0,
    }
}

/// Energy is fixed attempt energy plus a declared synthetic cost per payload
/// KiB. It is not calibrated meter data and does not model both radio directions
/// separately. A failed post-transmission task records request bytes, no reply.
fn transfer_attempt(
    payload_bytes: u64,
    wait_ms: u64,
    path: LinkPath,
    deadline_ms: u64,
    ttl_ms: u64,
    base_energy: u64,
) -> Result<TrialOutcome, String> {
    let allowed = deadline_ms.min(ttl_ms);
    if wait_ms >= allowed {
        return Ok(TrialOutcome {
            outcome: "failed",
            elapsed_ms: allowed,
            bytes_tx: 0,
            bytes_rx: 0,
            energy_millijoules: base_energy,
            charge_minor: 0,
        });
    }
    let path_ms = path_duration_ms(payload_bytes, path.latency_ms, path.bytes_per_second)?;
    let completion_ms = wait_ms
        .checked_add(path_ms)
        .ok_or_else(|| "wait plus path duration overflows u64".to_owned())?;
    let energy = energy_cost(base_energy, payload_bytes, path.energy_millijoules_per_kib)?;
    let charge = charge_cost(payload_bytes, path.charge_minor_per_kib)?;
    if completion_ms <= allowed {
        Ok(TrialOutcome {
            outcome: "succeeded",
            elapsed_ms: completion_ms,
            bytes_tx: payload_bytes,
            bytes_rx: payload_bytes,
            energy_millijoules: energy,
            charge_minor: charge,
        })
    } else {
        Ok(TrialOutcome {
            outcome: "failed",
            elapsed_ms: allowed,
            bytes_tx: payload_bytes,
            bytes_rx: 0,
            energy_millijoules: energy,
            charge_minor: charge,
        })
    }
}

fn blocked_for_outage(
    task: &Value,
    scenario: &Value,
    base_energy: u64,
) -> Result<TrialOutcome, String> {
    let arrival = u64_field(task, "arrival_ms")?;
    let start = u64_field(scenario, "outage_start_ms")?;
    let end = u64_field(scenario, "outage_end_ms")?;
    if arrival < start || arrival >= end {
        return Err("internal simulation error: expected WAN outage at task arrival".to_owned());
    }
    let queue_policy = scenario
        .get("queue_policy")
        .ok_or("queue_policy is missing")?;
    Ok(blocked_attempt(
        u64_field(task, "deadline_ms")?,
        u64_field(task, "expires_after_ms")?,
        u64_field(queue_policy, "failure_detection_ms")?,
        base_energy,
    ))
}

fn simulate_baseline(task: &Value, scenario: &Value) -> Result<TrialOutcome, String> {
    let arrival = u64_field(task, "arrival_ms")?;
    let outage_start = u64_field(scenario, "outage_start_ms")?;
    let outage_end = u64_field(scenario, "outage_end_ms")?;
    let link = scenario.get("link_model").ok_or("link_model is missing")?;
    let base = u64_field(link, "base_attempt_energy_millijoules")?;
    if arrival >= outage_start && arrival < outage_end {
        return blocked_for_outage(task, scenario, base);
    }
    transfer_attempt(
        u64_field(task, "payload_bytes")?,
        0,
        link_path(link, "wan")?,
        u64_field(task, "deadline_ms")?,
        u64_field(task, "expires_after_ms")?,
        base,
    )
}

fn simulate_candidate(
    task: &Value,
    scenario: &Value,
    pending: &mut Vec<(u64, u64)>,
) -> Result<TrialOutcome, String> {
    let arrival = u64_field(task, "arrival_ms")?;
    let start = u64_field(scenario, "outage_start_ms")?;
    let end = u64_field(scenario, "outage_end_ms")?;
    let payload = u64_field(task, "payload_bytes")?;
    let deadline = u64_field(task, "deadline_ms")?;
    let ttl = u64_field(task, "expires_after_ms")?;
    let queue = scenario
        .get("queue_policy")
        .ok_or("queue_policy is missing")?;
    let link = scenario.get("link_model").ok_or("link_model is missing")?;
    let base = u64_field(link, "base_attempt_energy_millijoules")?;
    pending.retain(|(release_at, _)| *release_at > arrival);

    if bool_field(task, "local_execution_allowed")? && bool_field(queue, "local_link_available")? {
        return transfer_attempt(payload, 0, link_path(link, "local")?, deadline, ttl, base);
    }

    if arrival < start || arrival >= end {
        return transfer_attempt(payload, 0, link_path(link, "wan")?, deadline, ttl, base);
    }

    let failure_detection = u64_field(queue, "failure_detection_ms")?;
    if !bool_field(task, "allow_store_forward")? {
        return blocked_for_outage(task, scenario, base);
    }

    let wait_ms = end.saturating_sub(arrival);
    // If the application deadline arrives before route recovery, there is no
    // reason to reserve queue space for a result that can no longer succeed.
    if deadline <= wait_ms {
        return Ok(TrialOutcome {
            outcome: "failed",
            elapsed_ms: deadline.min(ttl),
            bytes_tx: 0,
            bytes_rx: 0,
            energy_millijoules: base,
            charge_minor: 0,
        });
    }

    let expiry_at = arrival
        .checked_add(ttl)
        .ok_or_else(|| "expiry timestamp overflows u64".to_owned())?;
    let deadline_at = arrival
        .checked_add(deadline)
        .ok_or_else(|| "deadline timestamp overflows u64".to_owned())?;
    let release_at = expiry_at.min(deadline_at).min(end);
    let max_items = u64_field(queue, "max_items")? as usize;
    let max_bytes = u64_field(queue, "max_bytes")?;
    let pending_bytes = pending
        .iter()
        .try_fold(0_u64, |sum, (_, bytes)| sum.checked_add(*bytes))
        .ok_or_else(|| "pending queue-byte counter overflows u64".to_owned())?;
    let next_bytes = pending_bytes.checked_add(payload);
    if pending.len() >= max_items || next_bytes.is_none_or(|bytes| bytes > max_bytes) {
        return Ok(blocked_attempt(deadline, ttl, failure_detection, base));
    }
    pending.push((release_at, payload));

    if ttl <= wait_ms {
        return Ok(TrialOutcome {
            outcome: "failed",
            elapsed_ms: ttl.min(deadline),
            bytes_tx: 0,
            bytes_rx: 0,
            energy_millijoules: base,
            charge_minor: 0,
        });
    }

    transfer_attempt(
        payload,
        wait_ms,
        link_path(link, "wan")?,
        deadline,
        ttl,
        base,
    )
}

fn validate_semantics(scenario: &Value) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if string_field(scenario, "schema_version").ok() != Some(SCENARIO_VERSION) {
        errors.push(format!("schema_version must be {SCENARIO_VERSION}"));
    }
    let start = u64_field(scenario, "outage_start_ms").unwrap_or(0);
    let end = u64_field(scenario, "outage_end_ms").unwrap_or(0);
    if start >= end {
        errors.push("outage_start_ms must be less than outage_end_ms".to_owned());
    }
    let tasks = match scenario.get("tasks").and_then(Value::as_array) {
        Some(tasks) => tasks,
        None => return Err(vec!["tasks must be an array".to_owned()]),
    };
    if tasks.len() > MAX_TASKS {
        errors.push(format!("task count exceeds hard limit {MAX_TASKS}"));
    }
    let mut task_ids = BTreeSet::new();
    let mut pair_ids = BTreeSet::new();
    for (index, task) in tasks.iter().enumerate() {
        let id = string_field(task, "task_id").unwrap_or("");
        if !task_ids.insert(id.to_owned()) {
            errors.push(format!("tasks/{index}: duplicate task_id {id}"));
        }
        let pair = string_field(task, "pair_id").unwrap_or("");
        if !pair_ids.insert(pair.to_owned()) {
            errors.push(format!("tasks/{index}: duplicate pair_id {pair}"));
        }
        let arrival = u64_field(task, "arrival_ms").unwrap_or(0);
        let mode = string_field(task, "mode").unwrap_or("");
        let in_outage = arrival >= start && arrival < end;
        match mode {
            "online" if in_outage => errors.push(format!(
                "tasks/{index}: online task arrives during WAN outage"
            )),
            "wan_partition" | "route_recovery" if !in_outage => errors.push(format!(
                "tasks/{index}: {mode} task must arrive during WAN outage"
            )),
            "online" | "wan_partition" | "route_recovery" => {}
            _ => errors.push(format!("tasks/{index}: unsupported mode {mode}")),
        }
        let priority = string_field(task, "priority").unwrap_or("");
        if !["essential", "interactive", "bulk"].contains(&priority) {
            errors.push(format!("tasks/{index}: unsupported priority {priority}"));
        }
        if u64_field(task, "payload_bytes").unwrap_or(0) == 0
            || u64_field(task, "deadline_ms").unwrap_or(0) == 0
            || u64_field(task, "expires_after_ms").unwrap_or(0) == 0
        {
            errors.push(format!(
                "tasks/{index}: payload, deadline, and TTL must be positive"
            ));
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

fn make_trial(task: &Value, arm: &str, result: TrialOutcome) -> Value {
    json!({
        "trial_id": format!("{}-{arm}", task["task_id"].as_str().unwrap_or("invalid-task")),
        "pair_id": task["pair_id"],
        "scenario_id": task["scenario_id"],
        "arm": arm,
        "priority": task["priority"],
        "mode": task["mode"],
        "outcome": result.outcome,
        "elapsed_ms": result.elapsed_ms,
        "bytes_tx": result.bytes_tx,
        "bytes_rx": result.bytes_rx,
        "energy_millijoules": result.energy_millijoules,
        "charge_minor": result.charge_minor
    })
}

/// Validates the scenario, generates paired synthetic trials, then validates the
/// exact output with the canonical task-outcome contract and profile digest.
pub fn validate_and_simulate(
    scenario_schema: &Value,
    scenario: &Value,
    run_schema: &Value,
    profile_schema: &Value,
    profile_bytes: &[u8],
    profile: &Value,
) -> Result<(Value, Value), Vec<String>> {
    if profile_bytes.len() > MAX_INPUT_BYTES {
        return Err(vec!["profile exceeds input-size limit".to_owned()]);
    }
    if serde_json::to_vec(scenario).map_or(true, |bytes| bytes.len() > MAX_INPUT_BYTES) {
        return Err(vec!["scenario exceeds input-size limit".to_owned()]);
    }
    let schema_errors = schema_errors(scenario_schema, scenario);
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }
    validate_semantics(scenario)?;

    let tasks = scenario
        .get("tasks")
        .and_then(Value::as_array)
        .ok_or_else(|| vec!["tasks must be an array".to_owned()])?;
    let mut ordered = tasks.clone();
    ordered.sort_by(|left, right| {
        u64_field(left, "arrival_ms")
            .unwrap_or(0)
            .cmp(&u64_field(right, "arrival_ms").unwrap_or(0))
            .then_with(|| {
                priority_rank(string_field(right, "priority").unwrap_or(""))
                    .cmp(&priority_rank(string_field(left, "priority").unwrap_or("")))
            })
            .then_with(|| {
                string_field(left, "task_id")
                    .unwrap_or("")
                    .cmp(string_field(right, "task_id").unwrap_or(""))
            })
    });

    let mut pending: Vec<(u64, u64)> = Vec::new();
    let mut output: Vec<(String, Value, TrialOutcome, TrialOutcome)> =
        Vec::with_capacity(ordered.len());
    for task in &ordered {
        let baseline = simulate_baseline(task, scenario).map_err(|error| vec![error])?;
        let candidate =
            simulate_candidate(task, scenario, &mut pending).map_err(|error| vec![error])?;
        output.push((
            string_field(task, "task_id")
                .map_err(|error| vec![error])?
                .to_owned(),
            task.clone(),
            baseline,
            candidate,
        ));
    }
    output.sort_by(|left, right| {
        string_field(&left.1, "pair_id")
            .unwrap_or("")
            .cmp(string_field(&right.1, "pair_id").unwrap_or(""))
    });

    let mut trials = Vec::with_capacity(output.len() * 2);
    for (_, task, baseline, candidate) in output {
        trials.push(make_trial(&task, "baseline", baseline));
        trials.push(make_trial(&task, "candidate", candidate));
    }

    let mut canonical_scenario = scenario.clone();
    canonical_scenario["tasks"] = json!(ordered);
    let canonical_scenario_bytes = serde_json::to_vec(&canonical_scenario)
        .map_err(|error| vec![format!("cannot canonicalize scenario: {error}")])?;
    let scenario_digest = sha256_hex(&canonical_scenario_bytes);
    let run = json!({
        "schema_version": RUN_VERSION,
        "run_id": string_field(scenario, "run_id").map_err(|error| vec![error])?,
        "source": {
            "profile_sha256": sha256_hex(profile_bytes),
            "run_artifact_sha256": scenario_digest
        },
        "evidence_class": "synthetic_simulation",
        "currency": string_field(scenario, "currency").map_err(|error| vec![error])?,
        "environment": {},
        "trials": trials
    });
    let mut report = validate_and_report(run_schema, profile_schema, profile_bytes, profile, &run)?;
    report["simulation_id"] = scenario
        .get("simulation_id")
        .cloned()
        .unwrap_or(Value::Null);
    report["scenario_canonical_sha256"] = json!(scenario_digest);
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
                "../../../schemas/connectivity-outage-scenario-v1.schema.json"
            ))
            .unwrap(),
            scenario: serde_json::from_str(include_str!(
                "../../../examples/connectivity-outage-scenario-synthetic-v1.json"
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
    fn fixture_generates_a_valid_paired_synthetic_run() {
        let f = fixtures();
        let (run, report) = simulate(&f, &f.scenario).unwrap();
        assert_eq!(run["evidence_class"], "synthetic_simulation");
        assert_eq!(run["trials"].as_array().unwrap().len(), 18);
        assert_eq!(report["trial_count"], 18);
        assert_eq!(report["pair_count"], 9);
        assert_eq!(
            report["interpretation_boundary"]["field_performance_proven"],
            false
        );
        assert_eq!(
            run["source"]["run_artifact_sha256"],
            report["scenario_canonical_sha256"]
        );
        assert_eq!(report["simulation_id"], "outage-scenario-smoke-v1");
    }

    #[test]
    fn local_essential_task_survives_a_wan_partition() {
        let f = fixtures();
        let (_, report) = simulate(&f, &f.scenario).unwrap();
        let row = report["by_priority_and_mode"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["priority"] == "essential" && item["mode"] == "wan_partition")
            .unwrap();
        assert_eq!(row["baseline"]["success_rate_ppm"], 0);
        assert_eq!(row["candidate"]["success_rate_ppm"], 1_000_000);
    }

    #[test]
    fn message_expired_before_recovery_is_not_forwarded() {
        let f = fixtures();
        let (run, _) = simulate(&f, &f.scenario).unwrap();
        let trial = run["trials"]
            .as_array()
            .unwrap()
            .iter()
            .find(|trial| trial["trial_id"] == "task-partition-bulk-candidate")
            .unwrap();
        assert_eq!(trial["outcome"], "failed");
        assert_eq!(trial["bytes_tx"], 0);
        assert_eq!(trial["bytes_rx"], 0);
    }

    #[test]
    fn bounded_queue_rejects_the_next_eligible_message_when_full() {
        let f = fixtures();
        let mut scenario = f.scenario.clone();
        scenario["queue_policy"]["max_items"] = json!(1);
        let (run, _) = simulate(&f, &scenario).unwrap();
        let trial = run["trials"]
            .as_array()
            .unwrap()
            .iter()
            .find(|trial| trial["trial_id"] == "task-recovery-interactive-candidate")
            .unwrap();
        assert_eq!(trial["outcome"], "failed");
        assert_eq!(trial["bytes_tx"], 0);
    }

    #[test]
    fn task_mode_must_match_the_declared_outage_window() {
        let f = fixtures();
        let mut scenario = f.scenario.clone();
        scenario["tasks"][3]["arrival_ms"] = json!(1000);
        assert!(simulate(&f, &scenario).is_err());
    }

    #[test]
    fn output_is_deterministic_when_input_tasks_are_reordered() {
        let f = fixtures();
        let (first, first_report) = simulate(&f, &f.scenario).unwrap();
        let mut scenario = f.scenario.clone();
        let mut tasks = scenario["tasks"].as_array().unwrap().clone();
        tasks.reverse();
        scenario["tasks"] = json!(tasks);
        let (reversed, reversed_report) = simulate(&f, &scenario).unwrap();
        assert_eq!(first, reversed);
        assert_eq!(first_report, reversed_report);
    }

    #[test]
    fn generated_run_binds_exact_profile_bytes() {
        let f = fixtures();
        let (run, _) = simulate(&f, &f.scenario).unwrap();
        assert_eq!(
            run["source"]["profile_sha256"],
            sha256_hex(&f.profile_bytes)
        );
    }

    #[test]
    fn scenario_schema_with_remote_reference_is_rejected() {
        let f = fixtures();
        let mut schema = f.schema.clone();
        schema["properties"]["simulation_id"]["$ref"] =
            json!("https://example.invalid/schema.json");
        let err = schema_errors(&schema, &f.scenario);
        assert!(!err.is_empty());
    }
}
