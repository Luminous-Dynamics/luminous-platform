//! Stateful synthetic recovery simulation with partial progress retained across windows.
//!
//! This model compares FIFO byte service with per-peer weighted max-min service.
//! It is not an 802.11 airtime, queueing, TCP/QUIC, or physical energy model.

#![forbid(unsafe_code)]

use community_bandwidth_fairshare_core::{
    AllocationPolicy, PeerDemand, allocate as fair_share_allocate,
};
use connectivity_task_outcome_core::{sha256_hex, validate_and_report};
use jsonschema::draft202012;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const SCENARIO_VERSION: &str = "luminous.connectivity-recovery-multiwindow-scenario.v1";
const RUN_VERSION: &str = "luminous.connectivity-task-outcome-run.v1";
const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;
const REQUIRED_MODES: [&str; 3] = ["online", "wan_partition", "route_recovery"];
const REQUIRED_PRIORITIES: [&str; 3] = ["essential", "interactive", "bulk"];

#[derive(Clone, Debug, Eq, PartialEq)]
struct Task {
    task_id: String,
    pair_id: String,
    scenario_id: String,
    peer_id: String,
    weight: u16,
    priority: String,
    mode: String,
    arrival_window: u64,
    arrival_order: u64,
    deadline_window: u64,
    expires_after_window: u64,
    service_bytes: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Progress {
    delivered_bytes: u64,
    energy_millijoules: u64,
    charge_minor: u64,
    completion_window: Option<u64>,
    terminal_window: Option<u64>,
    failure_reason: Option<String>,
}

#[derive(Clone, Debug)]
struct WindowStats {
    window: u64,
    active_tasks: usize,
    transferred_bytes: u64,
    completed_now: usize,
    failed_now: usize,
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
        return vec!["scenario schema failed meta-validation".to_owned()];
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

fn parse_task(value: &Value) -> Result<Task, String> {
    Ok(Task {
        task_id: str_field(value, "task_id")?.to_owned(),
        pair_id: str_field(value, "pair_id")?.to_owned(),
        scenario_id: str_field(value, "scenario_id")?.to_owned(),
        peer_id: str_field(value, "peer_id")?.to_owned(),
        weight: u16_field(value, "weight")?,
        priority: str_field(value, "priority")?.to_owned(),
        mode: str_field(value, "mode")?.to_owned(),
        arrival_window: u64_field(value, "arrival_window")?,
        arrival_order: u64_field(value, "arrival_order")?,
        deadline_window: u64_field(value, "deadline_window")?,
        expires_after_window: u64_field(value, "expires_after_window")?,
        service_bytes: u64_field(value, "service_bytes")?,
    })
}

fn task_to_value(task: &Task) -> Value {
    json!({
        "task_id": task.task_id,
        "pair_id": task.pair_id,
        "scenario_id": task.scenario_id,
        "peer_id": task.peer_id,
        "weight": task.weight,
        "priority": task.priority,
        "mode": task.mode,
        "arrival_window": task.arrival_window,
        "arrival_order": task.arrival_order,
        "deadline_window": task.deadline_window,
        "expires_after_window": task.expires_after_window,
        "service_bytes": task.service_bytes
    })
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
    bytes / 1024 + u64::from(!bytes.is_multiple_of(1024))
}

fn checked_add(target: &mut u64, value: u64, label: &str) -> Result<(), String> {
    *target = target
        .checked_add(value)
        .ok_or_else(|| format!("{label} arithmetic overflowed"))?;
    Ok(())
}

fn checked_scaled(value: u64, factor: u64, label: &str) -> Result<u64, String> {
    value
        .checked_mul(factor)
        .ok_or_else(|| format!("{label} arithmetic overflowed"))
}

fn remaining(task: &Task, progress: &Progress) -> u64 {
    task.service_bytes.saturating_sub(progress.delivered_bytes)
}

fn is_active(task: &Task, progress: &Progress, window: u64) -> bool {
    progress.completion_window.is_none()
        && progress.failure_reason.is_none()
        && task.arrival_window <= window
        && window <= task.deadline_window
        && window <= task.expires_after_window
}

fn fifo_order(tasks: &[Task], left: usize, right: usize) -> std::cmp::Ordering {
    tasks[left]
        .arrival_window
        .cmp(&tasks[right].arrival_window)
        .then_with(|| tasks[left].arrival_order.cmp(&tasks[right].arrival_order))
        .then_with(|| tasks[left].task_id.cmp(&tasks[right].task_id))
}

fn deadline_order(tasks: &[Task], left: usize, right: usize) -> std::cmp::Ordering {
    tasks[left]
        .deadline_window
        .min(tasks[left].expires_after_window)
        .cmp(&tasks[right]
            .deadline_window
            .min(tasks[right].expires_after_window))
        .then_with(|| tasks[left].arrival_window.cmp(&tasks[right].arrival_window))
        .then_with(|| tasks[left].arrival_order.cmp(&tasks[right].arrival_order))
        .then_with(|| tasks[left].task_id.cmp(&tasks[right].task_id))
}

fn fcfs_grants(
    tasks: &[Task],
    progress: &[Progress],
    active: &[usize],
    capacity: u64,
) -> BTreeMap<usize, u64> {
    let mut order = active.to_vec();
    order.sort_by(|left, right| fifo_order(tasks, *left, *right));
    let mut left = capacity;
    let mut grants = BTreeMap::new();
    for index in order {
        let grant = remaining(&tasks[index], &progress[index]).min(left);
        left -= grant;
        grants.insert(index, grant);
    }
    grants
}

fn distribute(
    tasks: &[Task],
    progress: &[Progress],
    indices: &mut [usize],
    grant: u64,
    task_grants: &mut BTreeMap<usize, u64>,
) -> Result<(), String> {
    indices.sort_by(|left, right| deadline_order(tasks, *left, *right));
    let mut left = grant;
    for index in indices.iter().copied() {
        if left == 0 {
            break;
        }
        let amount = remaining(&tasks[index], &progress[index]).min(left);
        left -= amount;
        task_grants.insert(index, amount);
    }
    if left != 0 {
        return Err("allocator grant exceeds eligible task demand".to_owned());
    }
    Ok(())
}

fn fair_share_grants(
    tasks: &[Task],
    progress: &[Progress],
    active: &[usize],
    capacity: u64,
    essential_reserve: u64,
) -> Result<BTreeMap<usize, u64>, String> {
    let mut indices_by_peer: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for index in active {
        indices_by_peer
            .entry(tasks[*index].peer_id.clone())
            .or_default()
            .push(*index);
    }

    let mut demands = Vec::with_capacity(indices_by_peer.len());
    for (peer_id, indices) in &indices_by_peer {
        let weight = tasks[indices[0]].weight;
        let mut essential_demand_bytes = 0_u64;
        let mut ordinary_demand_bytes = 0_u64;
        for index in indices {
            let demand = remaining(&tasks[*index], &progress[*index]);
            if tasks[*index].priority == "essential" {
                checked_add(&mut essential_demand_bytes, demand, "essential demand")?;
            } else {
                checked_add(&mut ordinary_demand_bytes, demand, "ordinary demand")?;
            }
        }
        demands.push(PeerDemand {
            peer_id: peer_id.clone(),
            weight,
            essential_demand_bytes,
            ordinary_demand_bytes,
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

    let mut peer_grants = BTreeMap::new();
    for grant in allocation.grants {
        peer_grants.insert(
            grant.peer_id,
            (grant.essential_granted_bytes, grant.ordinary_granted_bytes),
        );
    }

    let mut task_grants = BTreeMap::new();
    for (peer, indices) in indices_by_peer {
        let (essential_grant, ordinary_grant) = peer_grants
            .get(&peer)
            .copied()
            .ok_or_else(|| format!("allocator omitted peer {peer}"))?;
        let mut essential_tasks: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|index| tasks[*index].priority == "essential")
            .collect();
        distribute(
            tasks,
            progress,
            &mut essential_tasks,
            essential_grant,
            &mut task_grants,
        )?;

        let mut ordinary_tasks: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|index| tasks[*index].priority != "essential")
            .collect();
        ordinary_tasks.sort_by(|left, right| {
            priority_rank(&tasks[*right].priority)
                .cmp(&priority_rank(&tasks[*left].priority))
                .then_with(|| deadline_order(tasks, *left, *right))
        });
        let mut ordinary_left = ordinary_grant;
        for index in ordinary_tasks {
            if ordinary_left == 0 {
                break;
            }
            let amount = remaining(&tasks[index], &progress[index]).min(ordinary_left);
            ordinary_left -= amount;
            task_grants.insert(index, amount);
        }
        if ordinary_left != 0 {
            return Err("ordinary allocator grant exceeds eligible task demand".to_owned());
        }
    }

    let total: u64 = task_grants.values().try_fold(0_u64, |sum, amount| {
        sum.checked_add(*amount)
            .ok_or_else(|| "task grant sum overflowed".to_owned())
    })?;
    if total > capacity {
        return Err("fair-share task grants exceed the window budget".to_owned());
    }
    Ok(task_grants)
}

fn apply_window(
    tasks: &[Task],
    progress: &mut [Progress],
    active: &[usize],
    grants: &BTreeMap<usize, u64>,
    window: u64,
    base_attempt_energy_millijoules: u64,
    energy_millijoules_per_kib: u64,
    charge_minor_per_kib: u64,
) -> Result<(u64, usize, usize), String> {
    let mut transferred_bytes = 0_u64;
    let mut completed_now = 0_usize;
    let mut failed_now = 0_usize;

    for index in active {
        let amount = grants.get(index).copied().unwrap_or(0);
        let item = &tasks[*index];
        let state = &mut progress[*index];

        // Each eligible task accrues a fixed synthetic active-window cost,
        // even when it receives no bytes. This is not measured radio energy.
        checked_add(
            &mut state.energy_millijoules,
            base_attempt_energy_millijoules,
            "base task-window energy",
        )?;
        let variable_energy = checked_scaled(
            ceil_kib(amount),
            energy_millijoules_per_kib,
            "variable energy",
        )?;
        checked_add(
            &mut state.energy_millijoules,
            variable_energy,
            "task energy",
        )?;
        let charge = checked_scaled(ceil_kib(amount), charge_minor_per_kib, "task charge")?;
        checked_add(&mut state.charge_minor, charge, "task charge total")?;
        checked_add(&mut state.delivered_bytes, amount, "task delivered bytes")?;
        checked_add(&mut transferred_bytes, amount, "window transfer")?;

        if state.delivered_bytes >= item.service_bytes {
            state.completion_window = Some(window);
            state.terminal_window = Some(window);
            completed_now += 1;
        } else if window >= item.deadline_window.min(item.expires_after_window) {
            state.terminal_window = Some(window);
            state.failure_reason = Some(if item.deadline_window <= item.expires_after_window {
                "deadline".to_owned()
            } else {
                "expired".to_owned()
            });
            failed_now += 1;
        }
    }
    Ok((transferred_bytes, completed_now, failed_now))
}

fn simulate_mode(
    tasks: &[Task],
    mode_indices: &[usize],
    mode: &str,
    progress: &mut [Progress],
    max_windows: u64,
    capacity: u64,
    essential_reserve: u64,
    base_energy: u64,
    energy_per_kib: u64,
    charge_per_kib: u64,
    fair_share: bool,
) -> Result<Vec<WindowStats>, String> {
    let mut window_stats = Vec::new();
    for window in 0..max_windows {
        let active: Vec<usize> = mode_indices
            .iter()
            .copied()
            .filter(|index| is_active(&tasks[*index], &progress[*index], window))
            .collect();
        if active.is_empty() {
            continue;
        }
        let grants = if fair_share {
            fair_share_grants(
                tasks,
                progress,
                &active,
                capacity,
                essential_reserve,
            )?
        } else {
            fcfs_grants(tasks, progress, &active, capacity)
        };
        let (transferred_bytes, completed_now, failed_now) = apply_window(
            tasks,
            progress,
            &active,
            &grants,
            window,
            base_energy,
            energy_per_kib,
            charge_per_kib,
        )?;
        if transferred_bytes > capacity {
            return Err(format!(
                "mode {mode} window {window}: transferred bytes exceed the declared budget"
            ));
        }
        window_stats.push(WindowStats {
            window,
            active_tasks: active.len(),
            transferred_bytes,
            completed_now,
            failed_now,
        });
    }

    if mode_indices.iter().any(|index| {
        progress[*index].completion_window.is_none()
            && progress[*index].failure_reason.is_none()
    }) {
        return Err(format!(
            "mode {mode}: at least one task was still non-terminal at the simulation horizon"
        ));
    }
    Ok(window_stats)
}

fn jain_fairness_ppm(tasks: &[Task], progress: &[Progress], indices: &[usize]) -> Option<u64> {
    let mut completions_by_peer: BTreeMap<&str, u64> = BTreeMap::new();
    for index in indices {
        let entry = completions_by_peer
            .entry(tasks[*index].peer_id.as_str())
            .or_insert(0);
        if progress[*index].completion_window.is_some() {
            *entry += 1;
        }
    }
    if completions_by_peer.is_empty() {
        return None;
    }
    let sum: u128 = completions_by_peer.values().map(|value| u128::from(*value)).sum();
    let sum_squares: u128 = completions_by_peer
        .values()
        .map(|value| u128::from(*value) * u128::from(*value))
        .sum();
    if sum_squares == 0 {
        return None;
    }
    Some(
        (sum * sum * 1_000_000
            / ((completions_by_peer.len() as u128) * sum_squares)) as u64,
    )
}

fn arm_summary(tasks: &[Task], progress: &[Progress], indices: &[usize]) -> Value {
    let mut completed = 0_u64;
    let mut delivered = 0_u64;
    let mut energy = 0_u64;
    let mut charge = 0_u64;
    for index in indices {
        completed += u64::from(progress[*index].completion_window.is_some());
        delivered = delivered.saturating_add(progress[*index].delivered_bytes);
        energy = energy.saturating_add(progress[*index].energy_millijoules);
        charge = charge.saturating_add(progress[*index].charge_minor);
    }
    json!({
        "task_count": indices.len(),
        "completed_tasks": completed,
        "failed_tasks": indices.len() as u64 - completed,
        "completed_task_jain_fairness_ppm": jain_fairness_ppm(tasks, progress, indices),
        "total_bytes_delivered": delivered,
        "total_task_energy_millijoules": energy,
        "total_task_charge_minor": charge
    })
}

fn window_values(stats: &[WindowStats], capacity: u64) -> Vec<Value> {
    stats.iter().map(|item| json!({
        "window": item.window,
        "active_tasks": item.active_tasks,
        "transferred_bytes": item.transferred_bytes,
        "unused_capacity_bytes": capacity - item.transferred_bytes,
        "completed_tasks_in_window": item.completed_now,
        "failed_tasks_in_window": item.failed_now
    })).collect()
}

fn task_results(tasks: &[Task], progress: &[Progress], indices: &[usize], arm: &str) -> Vec<Value> {
    let mut out: Vec<Value> = indices.iter().map(|index| {
        let task = &tasks[*index];
        let state = &progress[*index];
        json!({
            "task_id": task.task_id,
            "mode": task.mode,
            "priority": task.priority,
            "arm": arm,
            "service_bytes": task.service_bytes,
            "delivered_bytes": state.delivered_bytes,
            "outcome": if state.completion_window.is_some() { "succeeded" } else { "failed" },
            "completion_window": state.completion_window,
            "terminal_window": state.terminal_window,
            "failure_reason": state.failure_reason,
            "energy_millijoules": state.energy_millijoules,
            "charge_minor": state.charge_minor
        })
    }).collect();
    out.sort_by(|left, right| {
        left.get("task_id").and_then(Value::as_str).cmp(&right.get("task_id").and_then(Value::as_str))
            .then_with(|| left.get("arm").and_then(Value::as_str).cmp(&right.get("arm").and_then(Value::as_str)))
    });
    out
}

fn validate_semantics(scenario: &Value, tasks: &[Task]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if str_field(scenario, "schema_version").ok() != Some(SCENARIO_VERSION) {
        errors.push(format!("schema_version must be {SCENARIO_VERSION}"));
    }
    let capacity = u64_field(scenario, "window_capacity_bytes").unwrap_or(0);
    let reserve = u64_field(scenario, "essential_reserve_bytes").unwrap_or(u64::MAX);
    let max_windows = u64_field(scenario, "max_windows").unwrap_or(0);
    if capacity == 0 {
        errors.push("window_capacity_bytes must be positive".to_owned());
    }
    if reserve > capacity {
        errors.push("essential_reserve_bytes cannot exceed window_capacity_bytes".to_owned());
    }
    if max_windows == 0 || max_windows > 256 {
        errors.push("max_windows must be between 1 and 256".to_owned());
    }
    let mut task_ids = BTreeSet::new();
    let mut pair_ids = BTreeSet::new();
    let mut peer_weights = BTreeMap::new();
    let mut modes = BTreeSet::new();
    let mut priorities = BTreeSet::new();
    let mut cells = BTreeSet::new();

    for task in tasks {
        if !task_ids.insert(task.task_id.clone()) {
            errors.push(format!("duplicate task_id {}", task.task_id));
        }
        if !pair_ids.insert(task.pair_id.clone()) {
            errors.push(format!("duplicate pair_id {}", task.pair_id));
        }
        modes.insert(task.mode.clone());
        priorities.insert(task.priority.clone());
        cells.insert((task.mode.clone(), task.priority.clone()));
        if !REQUIRED_MODES.contains(&task.mode.as_str()) {
            errors.push(format!("unsupported mode {}", task.mode));
        }
        if !REQUIRED_PRIORITIES.contains(&task.priority.as_str()) {
            errors.push(format!("unsupported priority {}", task.priority));
        }
        if let Some(previous) = peer_weights.insert(task.peer_id.clone(), task.weight)
            && previous != task.weight
        {
            errors.push(format!("peer {} has inconsistent weights", task.peer_id));
        }
        if task.arrival_window > task.deadline_window {
            errors.push(format!("task {} arrives after its deadline", task.task_id));
        }
        if task.arrival_window > task.expires_after_window {
            errors.push(format!("task {} arrives after its expiry window", task.task_id));
        }
        for (label, window) in [
            ("arrival_window", task.arrival_window),
            ("deadline_window", task.deadline_window),
            ("expires_after_window", task.expires_after_window),
        ] {
            if window >= max_windows {
                errors.push(format!(
                    "task {} {label} {window} is outside max_windows {max_windows}",
                    task.task_id
                ));
            }
        }
        if task.service_bytes == 0 {
            errors.push(format!("task {} service_bytes must be positive", task.task_id));
        }
    }
    for mode in REQUIRED_MODES {
        if !modes.contains(mode) {
            errors.push(format!("required mode {mode} is absent"));
        }
        for priority in REQUIRED_PRIORITIES {
            if !cells.contains(&(mode.to_owned(), priority.to_owned())) {
                errors.push(format!("required mode/priority cell {mode}/{priority} is absent"));
            }
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

/// Validate scenario/schema contracts and emit a paired synthetic receipt plus
/// aggregate multi-window recovery trace. No peer identifiers are exported.
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
        return Err(vec!["scenario or profile exceeds the bounded input size".to_owned()]);
    }
    let mut errors = schema_errors(scenario_schema, scenario);
    let scenario_tasks = match scenario.get("tasks").and_then(Value::as_array) {
        Some(items) => items,
        None => return Err(vec!["tasks must be an array".to_owned()]),
    };
    let mut tasks = Vec::with_capacity(scenario_tasks.len());
    for (index, item) in scenario_tasks.iter().enumerate() {
        match parse_task(item) {
            Ok(task) => tasks.push(task),
            Err(error) => errors.push(format!("tasks/{index}: {error}")),
        }
    }
    if errors.is_empty()
        && let Err(semantic) = validate_semantics(scenario, &tasks)
    {
        errors.extend(semantic);
    }
    errors.extend(schema_errors(profile_schema, profile));
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    tasks.sort_by(|left, right| {
        left.mode
            .cmp(&right.mode)
            .then_with(|| left.arrival_window.cmp(&right.arrival_window))
            .then_with(|| left.arrival_order.cmp(&right.arrival_order))
            .then_with(|| priority_rank(&right.priority).cmp(&priority_rank(&left.priority)))
            .then_with(|| left.task_id.cmp(&right.task_id))
    });

    let capacity = u64_field(scenario, "window_capacity_bytes").map_err(|e| vec![e])?;
    let reserve = u64_field(scenario, "essential_reserve_bytes").map_err(|e| vec![e])?;
    let max_windows = u64_field(scenario, "max_windows").map_err(|e| vec![e])?;
    let window_duration_ms = u64_field(scenario, "window_duration_ms").map_err(|e| vec![e])?;
    let base_energy = u64_field(scenario, "base_attempt_energy_millijoules").map_err(|e| vec![e])?;
    let energy_per_kib = u64_field(scenario, "energy_millijoules_per_kib").map_err(|e| vec![e])?;
    let charge_per_kib = u64_field(scenario, "charge_minor_per_kib").map_err(|e| vec![e])?;

    let mut baseline_progress = vec![Progress::default(); tasks.len()];
    let mut candidate_progress = vec![Progress::default(); tasks.len()];
    let mut mode_reports = Vec::new();
    let mut trials = Vec::with_capacity(tasks.len() * 2);
    let mut all_baseline_windows = BTreeMap::new();
    let mut all_candidate_windows = BTreeMap::new();

    for mode in REQUIRED_MODES {
        let indices: Vec<usize> = tasks.iter().enumerate()
            .filter_map(|(index, task)| (task.mode == mode).then_some(index))
            .collect();
        let baseline_windows = simulate_mode(
            &tasks, &indices, mode, &mut baseline_progress, max_windows, capacity,
            reserve, base_energy, energy_per_kib, charge_per_kib, false,
        ).map_err(|error| vec![error])?;
        let candidate_windows = simulate_mode(
            &tasks, &indices, mode, &mut candidate_progress, max_windows, capacity,
            reserve, base_energy, energy_per_kib, charge_per_kib, true,
        ).map_err(|error| vec![error])?;

        all_baseline_windows.insert(mode.to_owned(), window_values(&baseline_windows, capacity));
        all_candidate_windows.insert(mode.to_owned(), window_values(&candidate_windows, capacity));

        mode_reports.push(json!({
            "mode": mode,
            "window_capacity_bytes": capacity,
            "essential_reserve_bytes": reserve,
            "baseline": arm_summary(&tasks, &baseline_progress, &indices),
            "candidate_fair_share": arm_summary(&tasks, &candidate_progress, &indices),
            "baseline_window_trace": window_values(&baseline_windows, capacity),
            "candidate_window_trace": window_values(&candidate_windows, capacity)
        }));

        for index in indices {
            for (arm, progress) in [
                ("baseline", &baseline_progress[index]),
                ("candidate", &candidate_progress[index]),
            ] {
                let task = &tasks[index];
                let terminal_window = progress.terminal_window.unwrap_or(task.deadline_window.min(task.expires_after_window));
                let latency_windows = terminal_window.saturating_sub(task.arrival_window).saturating_add(1);
                let elapsed_ms = latency_windows.saturating_mul(window_duration_ms);
                let request_bytes = task.service_bytes / 2;
                let bytes_tx = progress.delivered_bytes.min(request_bytes);
                let bytes_rx = progress.delivered_bytes.saturating_sub(bytes_tx).min(task.service_bytes - request_bytes);
                trials.push(json!({
                    "trial_id": format!("{}-{}", task.task_id, arm),
                    "pair_id": task.pair_id,
                    "scenario_id": task.scenario_id,
                    "arm": arm,
                    "priority": task.priority,
                    "mode": task.mode,
                    "outcome": if progress.completion_window.is_some() { "succeeded" } else { "failed" },
                    "elapsed_ms": elapsed_ms,
                    "bytes_tx": bytes_tx,
                    "bytes_rx": bytes_rx,
                    "energy_millijoules": progress.energy_millijoules,
                    "charge_minor": progress.charge_minor
                }));
            }
        }
    }

    trials.sort_by(|left, right| {
        left.get("pair_id").and_then(Value::as_str)
            .cmp(&right.get("pair_id").and_then(Value::as_str))
            .then_with(|| left.get("arm").and_then(Value::as_str).cmp(&right.get("arm").and_then(Value::as_str)))
    });

    let canonical_tasks: Vec<Value> = tasks.iter().map(task_to_value).collect();
    let mut canonical_scenario = scenario.clone();
    canonical_scenario["tasks"] = json!(canonical_tasks);
    let scenario_bytes = serde_json::to_vec(&canonical_scenario)
        .map_err(|error| vec![format!("cannot canonicalize scenario: {error}")])?;
    let scenario_sha = sha256_hex(&scenario_bytes);

    let run = json!({
        "schema_version": RUN_VERSION,
        "run_id": str_field(scenario, "run_id").map_err(|e| vec![e])?,
        "source": {
            "profile_sha256": sha256_hex(profile_bytes),
            "scenario_sha256": scenario_sha
        },
        "evidence_class": "synthetic_simulation",
        "currency": str_field(scenario, "currency").map_err(|e| vec![e])?,
        "environment": {},
        "trials": trials
    });

    let mut report = validate_and_report(run_schema, profile_schema, profile_bytes, profile, &run)?;
    report["simulation_id"] = scenario.get("simulation_id").cloned().unwrap_or(Value::Null);
    report["scenario_sha256"] = json!(scenario_sha);
    report["multiwindow_recovery"] = json!({
        "model": "stateful_partial_progress_per_window",
        "evidence_class": "synthetic_simulation",
        "window_duration_semantics": "discrete simulation unit; not measured radio or application latency",
        "capacity_semantics": "window capacity resets per independent mode episode; capacity is not shared across mode episodes",
        "partial_progress_persists": true,
        "peer_identifiers_exported": false,
        "per_mode": mode_reports,
        "task_results": [
            task_results(&tasks, &baseline_progress, &(0..tasks.len()).collect::<Vec<_>>(), "baseline"),
            task_results(&tasks, &candidate_progress, &(0..tasks.len()).collect::<Vec<_>>(), "candidate")
        ]
    });
    Ok((run, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

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
                "../../../schemas/connectivity-recovery-multiwindow-scenario-v1.schema.json"
            )).unwrap(),
            scenario: serde_json::from_str(include_str!(
                "../../../examples/connectivity-recovery-multiwindow-scenario-synthetic-v1.json"
            )).unwrap(),
            run_schema: serde_json::from_str(include_str!(
                "../../../schemas/connectivity-task-outcome-run-v1.schema.json"
            )).unwrap(),
            profile_schema: serde_json::from_str(include_str!(
                "../../../schemas/connectivity-task-outcome-profile-v1.schema.json"
            )).unwrap(),
            profile: serde_json::from_slice(&profile_bytes).unwrap(),
            profile_bytes,
        }
    }

    fn simulate(f: &Fixtures, scenario: &Value) -> Result<(Value, Value), Vec<String>> {
        validate_and_simulate(
            &f.schema, scenario, &f.run_schema, &f.profile_schema,
            &f.profile_bytes, &f.profile,
        )
    }

    #[test]
    fn scenario_produces_valid_paired_synthetic_receipt() {
        let f = fixtures();
        let (run, report) = simulate(&f, &f.scenario).unwrap();
        assert_eq!(run["evidence_class"], "synthetic_simulation");
        assert_eq!(run["trials"].as_array().unwrap().len(), 18);
        assert_eq!(report["multiwindow_recovery"]["partial_progress_persists"], true);
        assert_eq!(report["multiwindow_recovery"]["peer_identifiers_exported"], false);
    }

    #[test]
    fn fair_share_preserves_urgent_essential_completion_across_windows() {
        let f = fixtures();
        let (_, report) = simulate(&f, &f.scenario).unwrap();
        for mode in report["multiwindow_recovery"]["per_mode"].as_array().unwrap() {
            assert_eq!(mode["baseline"]["completed_tasks"], 1);
            assert_eq!(mode["candidate_fair_share"]["completed_tasks"], 3);
            assert_eq!(mode["baseline"]["completed_task_jain_fairness_ppm"], 333333);
            assert_eq!(mode["candidate_fair_share"]["completed_task_jain_fairness_ppm"], 1000000);
        }
    }

    #[test]
    fn partial_progress_is_retained_across_windows() {
        let f = fixtures();
        let (_, report) = simulate(&f, &f.scenario).unwrap();
        let online = &report["multiwindow_recovery"]["per_mode"][0];
        let baseline = online["baseline_window_trace"].as_array().unwrap();
        assert!(baseline[0]["transferred_bytes"].as_u64().unwrap() > 0);
        let candidate_results = report["multiwindow_recovery"]["task_results"].as_array().unwrap();
        let big = candidate_results[1].as_array().unwrap().iter()
            .find(|task| task["task_id"] == "online-bulk-large-a").unwrap();
        assert_eq!(big["outcome"], "succeeded");
        assert_eq!(big["completion_window"], 2);
        assert_eq!(big["delivered_bytes"], 20000);
    }

    #[test]
    fn output_is_deterministic_under_input_task_permutation() {
        let f = fixtures();
        let (run_a, report_a) = simulate(&f, &f.scenario).unwrap();
        let mut scenario = f.scenario.clone();
        let mut tasks = scenario["tasks"].as_array().unwrap().clone();
        tasks.reverse();
        scenario["tasks"] = json!(tasks);
        let (run_b, report_b) = simulate(&f, &scenario).unwrap();
        assert_eq!(run_a, run_b);
        assert_eq!(report_a, report_b);
    }

    #[test]
    fn duplicate_ids_and_missing_cells_fail_closed() {
        let f = fixtures();
        let mut scenario = f.scenario.clone();
        scenario["tasks"][1]["pair_id"] = scenario["tasks"][0]["pair_id"].clone();
        assert!(simulate(&f, &scenario).is_err());
        let mut scenario = f.scenario.clone();
        scenario["tasks"].as_array_mut().unwrap().retain(|task| task["mode"] != "route_recovery" || task["priority"] != "bulk");
        assert!(simulate(&f, &scenario).is_err());
    }

    #[test]
    fn deadlines_and_expiry_must_be_inside_horizon() {
        let f = fixtures();
        let mut scenario = f.scenario.clone();
        scenario["tasks"][0]["deadline_window"] = json!(5);
        assert!(simulate(&f, &scenario).is_err());
    }

    #[test]
    fn remote_schema_references_are_rejected() {
        let f = fixtures();
        let mut schema = f.schema.clone();
        schema["properties"]["simulation_id"]["$ref"] = json!("https://example.invalid/schema.json");
        assert!(!schema_errors(&schema, &f.scenario).is_empty());
    }
}
