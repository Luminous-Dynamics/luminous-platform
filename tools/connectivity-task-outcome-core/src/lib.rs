//! Offline validation and deterministic aggregation for task-first connectivity
//! experiment receipts. The raw outcomes are supplied claims; this crate does
//! not prove that a real user task succeeded or authenticate the reporter.

#![forbid(unsafe_code)]

use jsonschema::draft202012;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const RUN_SCHEMA_VERSION: &str = "luminous.connectivity-task-outcome-run.v1";
const PROFILE_SCHEMA_VERSION: &str = "luminous.connectivity-task-outcome-profile.v1";
const MAX_RUN_BYTES: usize = 32 * 1024 * 1024;

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn reject_remote_references(value: &Value, path: &str, errors: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if key == "$ref" || key == "$dynamicRef" {
                    match child.as_str() {
                        Some(reference) if reference.starts_with('#') => {}
                        Some(_) => {
                            errors.push(format!("{path}/{key}: only local references are allowed"))
                        }
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

fn schema_errors(schema: &Value, value: &Value, label: &str) -> Vec<String> {
    let mut errors = Vec::new();
    reject_remote_references(schema, label, &mut errors);
    if !errors.is_empty() {
        return errors;
    }
    let meta = match jsonschema::meta::validator_for(schema) {
        Ok(validator) => validator,
        Err(error) => {
            return vec![format!(
                "{label}: cannot build JSON Schema meta-validator: {error}"
            )];
        }
    };
    if !meta.evaluate(schema).flag().valid {
        return vec![format!(
            "{label}: schema failed JSON Schema meta-validation"
        )];
    }
    let validator = match draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => validator,
        Err(error) => {
            return vec![format!(
                "{label}: cannot build Draft 2020-12 validator: {error}"
            )];
        }
    };
    errors.extend(
        validator
            .iter_errors(value)
            .map(|error| format!("{label}: {error}")),
    );
    errors
}

fn required_string<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn checked_u64_sum<I>(values: I) -> Option<u64>
where
    I: IntoIterator<Item = u64>,
{
    values.into_iter().try_fold(0_u64, u64::checked_add)
}

fn percentile_nearest_rank(sorted: &[u64], numerator: usize, denominator: usize) -> Option<u64> {
    if sorted.is_empty() || denominator == 0 || numerator == 0 || numerator > denominator {
        return None;
    }
    let rank = sorted
        .len()
        .saturating_mul(numerator)
        .div_ceil(denominator)
        .max(1);
    sorted.get(rank - 1).copied()
}

#[derive(Default)]
struct ArmStats {
    succeeded: u64,
    failed: u64,
    inconclusive: u64,
    successful_elapsed_ms: Vec<u64>,
    successful_bytes: Vec<u64>,
    successful_energy_mj: Vec<Option<u64>>,
    successful_charge_minor: Vec<Option<u64>>,
    bytes_all_trials: Vec<u64>,
}

impl ArmStats {
    fn attempted(&self) -> u64 {
        self.succeeded + self.failed + self.inconclusive
    }

    fn report(&self, currency: &str) -> Value {
        let attempted = self.attempted();
        let bytes_all = checked_u64_sum(self.bytes_all_trials.iter().copied());
        let mut latencies = self.successful_elapsed_ms.clone();
        latencies.sort_unstable();
        let successful_bytes = checked_u64_sum(self.successful_bytes.iter().copied());
        let all_energy_present = !self.successful_energy_mj.is_empty()
            && self.successful_energy_mj.iter().all(Option::is_some);
        let all_charge_present = !self.successful_charge_minor.is_empty()
            && self.successful_charge_minor.iter().all(Option::is_some);
        let energy_sum = all_energy_present
            .then(|| checked_u64_sum(self.successful_energy_mj.iter().flatten().copied()))
            .flatten();
        let charge_sum = all_charge_present
            .then(|| checked_u64_sum(self.successful_charge_minor.iter().flatten().copied()))
            .flatten();
        let energy_per_success = if self.succeeded > 0 {
            energy_sum.map(|sum| sum / self.succeeded)
        } else {
            None
        };
        let charge_per_success = if self.succeeded > 0 {
            charge_sum.map(|sum| sum / self.succeeded)
        } else {
            None
        };
        let success_rate_ppm = if attempted > 0 {
            Some(((u128::from(self.succeeded) * 1_000_000) / u128::from(attempted)) as u64)
        } else {
            None
        };

        json!({
            "attempted": attempted,
            "succeeded": self.succeeded,
            "failed": self.failed,
            "inconclusive": self.inconclusive,
            "success_rate_numerator": self.succeeded,
            "success_rate_denominator": attempted,
            "success_rate_ppm": success_rate_ppm,
            "successful_latency_sample_count": latencies.len(),
            "latency_p50_ms": percentile_nearest_rank(&latencies, 1, 2),
            "latency_p95_ms": percentile_nearest_rank(&latencies, 95, 100),
            "latency_p99_ms": percentile_nearest_rank(&latencies, 99, 100),
            "bytes_all_trials": bytes_all,
            "bytes_per_successful_task": if self.succeeded > 0 && self.successful_bytes.len() as u64 == self.succeeded {
                successful_bytes.map(|sum| sum / self.succeeded)
            } else {
                None
            },
            "energy_per_successful_task_millijoules": energy_per_success,
            "user_charge_per_successful_task_minor": charge_per_success,
            "currency": currency,
            "zero_success_denominator": self.succeeded == 0
        })
    }
}

fn validate_environment(run: &Value, evidence: &str, errors: &mut Vec<String>) {
    let environment = run.get("environment");
    let present = |key: &str| {
        environment
            .and_then(|value| value.get(key))
            .is_some_and(|value| !value.is_null())
    };
    if evidence != "synthetic_simulation" {
        for key in [
            "gateway_model",
            "hardware_revision",
            "firmware_image_sha256",
            "measurement_method",
            "power_meter_token",
        ] {
            if !present(key) {
                errors.push(format!(
                    "environment.{key} is required for {evidence} evidence"
                ));
            }
        }
    }
    if evidence == "field_observed" {
        for key in ["site_token", "consent_record_sha256"] {
            if !present(key) {
                errors.push(format!(
                    "environment.{key} is required for field_observed evidence"
                ));
            }
        }
    }
}

fn aggregate_policy(profile: &Value, run: &Value) -> Result<Value, Vec<String>> {
    let mut errors = Vec::new();
    if required_string(profile, "/schema_version") != Some(PROFILE_SCHEMA_VERSION) {
        errors.push(format!(
            "profile.schema_version must be {PROFILE_SCHEMA_VERSION}"
        ));
    }
    if required_string(run, "/schema_version") != Some(RUN_SCHEMA_VERSION) {
        errors.push(format!("run.schema_version must be {RUN_SCHEMA_VERSION}"));
    }
    let evidence = required_string(run, "/evidence_class").unwrap_or("");
    validate_environment(run, evidence, &mut errors);

    let profile_id = required_string(profile, "/profile_id").unwrap_or("invalid-profile");
    let profile_sha = required_string(run, "/source/profile_sha256").unwrap_or("");
    if !is_sha256(profile_sha) {
        errors.push("run.source.profile_sha256 must be a lowercase SHA-256 digest".to_owned());
    }

    let required_priorities: BTreeSet<String> = profile
        .get("required_priorities")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let required_modes: BTreeSet<String> = profile
        .get("required_modes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let minimum_pairs = profile
        .get("minimum_pairs")
        .and_then(Value::as_u64)
        .unwrap_or(u64::MAX) as usize;

    let trials = match run.get("trials").and_then(Value::as_array) {
        Some(trials) => trials,
        None => return Err(vec!["run.trials must be an array".to_owned()]),
    };
    let mut ids = BTreeSet::new();
    let mut pairs: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    let mut priorities = BTreeSet::new();
    let mut modes = BTreeSet::new();
    let mut stats: BTreeMap<String, ArmStats> = BTreeMap::new();
    stats.insert("baseline".to_owned(), ArmStats::default());
    stats.insert("candidate".to_owned(), ArmStats::default());
    let mut stratified_counts: BTreeMap<(String, String), BTreeMap<String, [u64; 3]>> =
        BTreeMap::new();
    let mut stratified_latencies: BTreeMap<(String, String, String), Vec<u64>> = BTreeMap::new();

    for (index, trial) in trials.iter().enumerate() {
        let trial_id = required_string(trial, "/trial_id").unwrap_or("");
        if !ids.insert(trial_id.to_owned()) {
            errors.push(format!("trials/{index}/trial_id duplicates {trial_id}"));
        }
        let pair_id = required_string(trial, "/pair_id").unwrap_or("");
        pairs.entry(pair_id.to_owned()).or_default().push(trial);
        if let Some(priority) = required_string(trial, "/priority") {
            priorities.insert(priority.to_owned());
        }
        if let Some(mode) = required_string(trial, "/mode") {
            modes.insert(mode.to_owned());
        }

        let arm = required_string(trial, "/arm").unwrap_or("");
        let Some(arm_stats) = stats.get_mut(arm) else {
            errors.push(format!("trials/{index}/arm must be baseline or candidate"));
            continue;
        };
        let outcome = required_string(trial, "/outcome").unwrap_or("");
        let priority = required_string(trial, "/priority")
            .unwrap_or("unknown")
            .to_owned();
        let mode = required_string(trial, "/mode")
            .unwrap_or("unknown")
            .to_owned();
        let counts = stratified_counts
            .entry((priority.clone(), mode.clone()))
            .or_default()
            .entry(arm.to_owned())
            .or_insert([0_u64; 3]);
        match outcome {
            "succeeded" => counts[0] += 1,
            "failed" => counts[1] += 1,
            "inconclusive" => counts[2] += 1,
            _ => {}
        }
        match outcome {
            "succeeded" => {
                arm_stats.succeeded += 1;
                match trial.get("elapsed_ms").and_then(Value::as_u64) {
                    Some(elapsed) => {
                        arm_stats.successful_elapsed_ms.push(elapsed);
                        stratified_latencies
                            .entry((priority.clone(), mode.clone(), arm.to_owned()))
                            .or_default()
                            .push(elapsed);
                    }
                    None => errors.push(format!(
                        "trials/{index}: succeeded task requires elapsed_ms"
                    )),
                }
                let bytes_tx = trial
                    .get("bytes_tx")
                    .and_then(Value::as_u64)
                    .unwrap_or(u64::MAX);
                let bytes_rx = trial
                    .get("bytes_rx")
                    .and_then(Value::as_u64)
                    .unwrap_or(u64::MAX);
                match bytes_tx.checked_add(bytes_rx) {
                    Some(bytes) => arm_stats.successful_bytes.push(bytes),
                    None => errors.push(format!(
                        "trials/{index}: transmitted+received bytes overflow"
                    )),
                }
                arm_stats
                    .successful_energy_mj
                    .push(trial.get("energy_millijoules").and_then(Value::as_u64));
                arm_stats
                    .successful_charge_minor
                    .push(trial.get("charge_minor").and_then(Value::as_u64));
            }
            "failed" => {
                arm_stats.failed += 1;
                if trial.get("elapsed_ms").and_then(Value::as_u64).is_none() {
                    errors.push(format!("trials/{index}: failed task requires elapsed_ms"));
                }
            }
            "inconclusive" => {
                arm_stats.inconclusive += 1;
                if trial
                    .get("elapsed_ms")
                    .is_some_and(|value| !value.is_null())
                {
                    errors.push(format!(
                        "trials/{index}: inconclusive trial must not assert elapsed_ms"
                    ));
                }
            }
            _ => errors.push(format!("trials/{index}: outcome is invalid")),
        }

        let bytes_tx = trial
            .get("bytes_tx")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX);
        let bytes_rx = trial
            .get("bytes_rx")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX);
        if let Some(bytes) = bytes_tx.checked_add(bytes_rx) {
            arm_stats.bytes_all_trials.push(bytes);
        } else {
            errors.push(format!(
                "trials/{index}: transmitted+received bytes overflow"
            ));
        }
    }

    for (pair_id, pair_trials) in &pairs {
        if pair_trials.len() != 2 {
            errors.push(format!(
                "pair {pair_id} must contain exactly one baseline and one candidate trial"
            ));
            continue;
        }
        let baseline: Vec<&Value> = pair_trials
            .iter()
            .copied()
            .filter(|trial| required_string(trial, "/arm") == Some("baseline"))
            .collect();
        let candidate: Vec<&Value> = pair_trials
            .iter()
            .copied()
            .filter(|trial| required_string(trial, "/arm") == Some("candidate"))
            .collect();
        if baseline.len() != 1 || candidate.len() != 1 {
            errors.push(format!(
                "pair {pair_id} must contain one baseline and one candidate"
            ));
            continue;
        }
        for key in ["/scenario_id", "/priority", "/mode"] {
            if baseline[0].pointer(key) != candidate[0].pointer(key) {
                errors.push(format!("pair {pair_id} mismatches {key} between arms"));
            }
        }
    }

    if pairs.len() < minimum_pairs {
        errors.push(format!(
            "run has {} pair IDs; profile requires at least {minimum_pairs}",
            pairs.len()
        ));
    }
    if profile
        .get("require_each_priority")
        .and_then(Value::as_bool)
        == Some(true)
    {
        for priority in &required_priorities {
            if !priorities.contains(priority) {
                errors.push(format!("required priority {priority} is absent"));
            }
        }
    }
    if profile.get("require_each_mode").and_then(Value::as_bool) == Some(true) {
        for mode in &required_modes {
            if !modes.contains(mode) {
                errors.push(format!("required mode {mode} is absent"));
            }
        }
    }

    for (arm, arm_stats) in &stats {
        if checked_u64_sum(arm_stats.bytes_all_trials.iter().copied()).is_none() {
            errors.push(format!("arm {arm} total transferred bytes overflow u64"));
        }
        if !arm_stats.successful_energy_mj.is_empty()
            && arm_stats.successful_energy_mj.iter().all(Option::is_some)
            && checked_u64_sum(arm_stats.successful_energy_mj.iter().flatten().copied()).is_none()
        {
            errors.push(format!(
                "arm {arm} total successful-task energy overflow u64"
            ));
        }
        if !arm_stats.successful_charge_minor.is_empty()
            && arm_stats
                .successful_charge_minor
                .iter()
                .all(Option::is_some)
            && checked_u64_sum(arm_stats.successful_charge_minor.iter().flatten().copied())
                .is_none()
        {
            errors.push(format!(
                "arm {arm} total successful-task charge overflow u64"
            ));
        }
        if arm_stats.attempted() == 0 {
            errors.push(format!("arm {arm} has no trials"));
        }
        if arm_stats.successful_elapsed_ms.len() as u64 != arm_stats.succeeded {
            errors.push(format!(
                "arm {arm} latency samples must match successful task count"
            ));
        }
        if arm_stats.successful_bytes.len() as u64 != arm_stats.succeeded {
            errors.push(format!(
                "arm {arm} successful byte samples must match successful task count"
            ));
        }
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let currency = required_string(run, "/currency").unwrap_or("XXX");
    let reports: BTreeMap<String, Value> = stats
        .iter()
        .map(|(arm, arm_stats)| (arm.clone(), arm_stats.report(currency)))
        .collect();
    let by_priority_and_mode: Vec<Value> = stratified_counts
        .iter()
        .map(|((priority, mode), arms)| {
            let arm_report = |arm: &str| {
                let counts = arms.get(arm).copied().unwrap_or([0, 0, 0]);
                let attempted = counts[0] + counts[1] + counts[2];
                let success_rate_ppm = if attempted > 0 {
                    Some(((u128::from(counts[0]) * 1_000_000) / u128::from(attempted)) as u64)
                } else {
                    None
                };
                let key = (priority.clone(), mode.clone(), arm.to_owned());
                let mut latencies = stratified_latencies.get(&key).cloned().unwrap_or_default();
                latencies.sort_unstable();
                json!({
                    "attempted": attempted,
                    "succeeded": counts[0],
                    "failed": counts[1],
                    "inconclusive": counts[2],
                    "success_rate_numerator": counts[0],
                    "success_rate_denominator": attempted,
                    "success_rate_ppm": success_rate_ppm,
                    "successful_latency_sample_count": latencies.len(),
                    "latency_p50_ms": percentile_nearest_rank(&latencies, 1, 2),
                    "latency_p95_ms": percentile_nearest_rank(&latencies, 95, 100),
                    "latency_p99_ms": percentile_nearest_rank(&latencies, 99, 100)
                })
            };
            json!({
                "priority": priority,
                "mode": mode,
                "baseline": arm_report("baseline"),
                "candidate": arm_report("candidate")
            })
        })
        .collect();
    let baseline_success = stats.get("baseline").map(|arm| arm.succeeded).unwrap_or(0);
    let candidate_success = stats.get("candidate").map(|arm| arm.succeeded).unwrap_or(0);
    let baseline_attempts = stats.get("baseline").map(ArmStats::attempted).unwrap_or(0);
    let candidate_attempts = stats.get("candidate").map(ArmStats::attempted).unwrap_or(0);
    let successful_delta_ppm = if baseline_attempts > 0 && candidate_attempts > 0 {
        let base =
            (u128::from(baseline_success) * 1_000_000 / u128::from(baseline_attempts)) as i64;
        let cand =
            (u128::from(candidate_success) * 1_000_000 / u128::from(candidate_attempts)) as i64;
        Some(cand - base)
    } else {
        None
    };

    Ok(json!({
        "report_version": "luminous.connectivity-task-outcome-report.v1",
        "profile_id": profile_id,
        "profile_sha256": profile_sha,
        "run_id": required_string(run, "/run_id"),
        "evidence_class": evidence,
        "trial_count": trials.len(),
        "pair_count": pairs.len(),
        "priority_classes_observed": priorities,
        "modes_observed": modes,
        "baseline": reports.get("baseline"),
        "candidate": reports.get("candidate"),
        "by_priority_and_mode": by_priority_and_mode,
        "candidate_minus_baseline_success_rate_ppm": successful_delta_ppm,
        "interpretation_boundary": {
            "task_outcomes_are_reporter_assertions": true,
            "field_performance_proven": false,
            "identity_authenticated": false,
            "currency_conversion_performed": false
        }
    }))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Validates schema/profile/run content and aggregates outcomes. The profile
/// SHA is computed from the exact profile bytes supplied by the caller.
pub fn validate_and_report(
    run_schema: &Value,
    profile_schema: &Value,
    profile_bytes: &[u8],
    profile: &Value,
    run: &Value,
) -> Result<Value, Vec<String>> {
    let mut errors = Vec::new();
    if profile_bytes.len() > MAX_RUN_BYTES {
        return Err(vec!["profile exceeds input size limit".to_owned()]);
    }
    errors.extend(schema_errors(profile_schema, profile, "profile"));
    errors.extend(schema_errors(run_schema, run, "run"));
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }
    let profile_digest = sha256_hex(profile_bytes);
    if required_string(run, "/source/profile_sha256") != Some(profile_digest.as_str()) {
        errors.push("run.source.profile_sha256 does not match exact profile bytes".to_owned());
        return Err(errors);
    }
    aggregate_policy(profile, run)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> (Value, Value, Vec<u8>, Value) {
        let run_schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/connectivity-task-outcome-run-v1.schema.json"
        ))
        .expect("run schema parses");
        let profile_schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/connectivity-task-outcome-profile-v1.schema.json"
        ))
        .expect("profile schema parses");
        let profile_bytes =
            include_bytes!("../../../profiles/connectivity-task-outcome-benchmark-profile-v1.json")
                .to_vec();
        let run: Value = serde_json::from_str(include_str!(
            "../../../examples/connectivity-task-outcome-run-synthetic-v1.json"
        ))
        .expect("run parses");
        (run_schema, profile_schema, profile_bytes, run)
    }

    fn ready_run(mut run: Value, profile_bytes: &[u8]) -> Value {
        run["source"]["profile_sha256"] = json!(sha256_hex(profile_bytes));
        run
    }

    #[test]
    fn checked_in_schemas_and_profile_validate() {
        let (run_schema, profile_schema, profile_bytes, run) = fixtures();
        let run = ready_run(run, &profile_bytes);
        let profile: Value = serde_json::from_slice(&profile_bytes).unwrap();
        let result =
            validate_and_report(&run_schema, &profile_schema, &profile_bytes, &profile, &run);
        assert!(result.is_ok(), "report errors: {result:#?}");
    }

    #[test]
    fn report_keeps_failed_and_inconclusive_trials_in_denominator() {
        let (run_schema, profile_schema, profile_bytes, run) = fixtures();
        let run = ready_run(run, &profile_bytes);
        let profile: Value = serde_json::from_slice(&profile_bytes).unwrap();
        let report =
            validate_and_report(&run_schema, &profile_schema, &profile_bytes, &profile, &run)
                .unwrap();
        assert_eq!(report["baseline"]["attempted"], 3);
        assert_eq!(report["baseline"]["succeeded"], 1);
        assert_eq!(report["baseline"]["failed"], 1);
        assert_eq!(report["baseline"]["inconclusive"], 1);
        assert_eq!(report["baseline"]["success_rate_denominator"], 3);
        assert_eq!(report["baseline"]["success_rate_ppm"], 333333);
        assert_eq!(report["candidate"]["succeeded"], 3);
        let strata = report["by_priority_and_mode"].as_array().unwrap();
        let interactive = strata
            .iter()
            .find(|item| item["priority"] == "interactive")
            .unwrap();
        assert_eq!(interactive["baseline"]["success_rate_ppm"], 0);
        assert_eq!(interactive["candidate"]["success_rate_ppm"], 1_000_000);
        assert_eq!(interactive["candidate"]["latency_p50_ms"], 250);
        assert_eq!(interactive["candidate"]["latency_p99_ms"], 250);
        assert_eq!(interactive["baseline"]["latency_p50_ms"], Value::Null);
    }

    #[test]
    fn missing_energy_on_any_success_makes_energy_per_success_null() {
        let (run_schema, profile_schema, profile_bytes, run) = fixtures();
        let profile: Value = serde_json::from_slice(&profile_bytes).unwrap();
        let mut run = ready_run(run, &profile_bytes);
        run["trials"][1]["energy_millijoules"] = Value::Null;
        let report =
            validate_and_report(&run_schema, &profile_schema, &profile_bytes, &profile, &run)
                .unwrap();
        assert_eq!(
            report["candidate"]["energy_per_successful_task_millijoules"],
            Value::Null
        );
    }

    #[test]
    fn per_trial_byte_sum_overflow_is_rejected() {
        let (run_schema, profile_schema, profile_bytes, run) = fixtures();
        let profile: Value = serde_json::from_slice(&profile_bytes).unwrap();
        let mut run = ready_run(run, &profile_bytes);
        run["trials"][0]["bytes_tx"] = json!(u64::MAX);
        run["trials"][0]["bytes_rx"] = json!(1);
        assert!(
            validate_and_report(&run_schema, &profile_schema, &profile_bytes, &profile, &run)
                .is_err()
        );
    }

    #[test]
    fn paired_scenario_metadata_mismatch_fails_closed() {
        let (run_schema, profile_schema, profile_bytes, run) = fixtures();
        let profile: Value = serde_json::from_slice(&profile_bytes).unwrap();
        let mut run = ready_run(run, &profile_bytes);
        run["trials"][1]["scenario_id"] = json!("other-scenario");
        let result =
            validate_and_report(&run_schema, &profile_schema, &profile_bytes, &profile, &run);
        assert!(result.is_err());
    }

    #[test]
    fn duplicate_trial_ids_are_rejected() {
        let (run_schema, profile_schema, profile_bytes, run) = fixtures();
        let profile: Value = serde_json::from_slice(&profile_bytes).unwrap();
        let mut run = ready_run(run, &profile_bytes);
        run["trials"][1]["trial_id"] = run["trials"][0]["trial_id"].clone();
        let result =
            validate_and_report(&run_schema, &profile_schema, &profile_bytes, &profile, &run);
        assert!(result.is_err());
    }

    #[test]
    fn wrong_profile_digest_is_rejected() {
        let (run_schema, profile_schema, profile_bytes, mut run) = fixtures();
        run["source"]["profile_sha256"] = json!("a".repeat(64));
        let profile: Value = serde_json::from_slice(&profile_bytes).unwrap();
        assert!(
            validate_and_report(&run_schema, &profile_schema, &profile_bytes, &profile, &run)
                .is_err()
        );
    }

    #[test]
    fn measured_evidence_requires_device_firmware_and_instrument_metadata() {
        let (run_schema, profile_schema, profile_bytes, run) = fixtures();
        let profile: Value = serde_json::from_slice(&profile_bytes).unwrap();
        let mut run = ready_run(run, &profile_bytes);
        run["evidence_class"] = json!("bench_measured");
        let result =
            validate_and_report(&run_schema, &profile_schema, &profile_bytes, &profile, &run);
        assert!(result.is_err());
    }

    #[test]
    fn field_evidence_requires_site_token_and_consent_digest() {
        let (run_schema, profile_schema, profile_bytes, run) = fixtures();
        let profile: Value = serde_json::from_slice(&profile_bytes).unwrap();
        let mut run = ready_run(run, &profile_bytes);
        run["evidence_class"] = json!("field_observed");
        let result =
            validate_and_report(&run_schema, &profile_schema, &profile_bytes, &profile, &run);
        assert!(result.is_err());
    }

    #[test]
    fn no_success_makes_per_task_cost_and_energy_null() {
        let (run_schema, profile_schema, profile_bytes, run) = fixtures();
        let profile: Value = serde_json::from_slice(&profile_bytes).unwrap();
        let mut run = ready_run(run, &profile_bytes);
        for trial in run["trials"].as_array_mut().unwrap() {
            trial["outcome"] = json!("failed");
            trial["elapsed_ms"] = json!(1);
        }
        let result =
            validate_and_report(&run_schema, &profile_schema, &profile_bytes, &profile, &run);
        assert!(
            result.is_ok(),
            "zero-success run should be valid: {result:#?}"
        );
        let report = result.unwrap();
        assert_eq!(report["baseline"]["success_rate_ppm"], 0);
        assert_eq!(
            report["baseline"]["energy_per_successful_task_millijoules"],
            Value::Null
        );
        assert_eq!(
            report["baseline"]["user_charge_per_successful_task_minor"],
            Value::Null
        );
    }

    #[test]
    fn percentiles_use_successful_tasks_only() {
        let (run_schema, profile_schema, profile_bytes, run) = fixtures();
        let profile: Value = serde_json::from_slice(&profile_bytes).unwrap();
        let mut run = ready_run(run, &profile_bytes);
        run["trials"][2]["outcome"] = json!("succeeded");
        run["trials"][2]["elapsed_ms"] = json!(6000);
        run["trials"][2]["energy_millijoules"] = json!(2000);
        run["trials"][2]["charge_minor"] = json!(0);
        let report =
            validate_and_report(&run_schema, &profile_schema, &profile_bytes, &profile, &run)
                .unwrap();
        assert_eq!(report["baseline"]["latency_p50_ms"], 110);
        assert_eq!(report["baseline"]["latency_p95_ms"], 6000);
    }

    #[test]
    fn remote_schema_refs_are_rejected_without_resolution() {
        let (mut run_schema, profile_schema, profile_bytes, run) = fixtures();
        let profile: Value = serde_json::from_slice(&profile_bytes).unwrap();
        run_schema["properties"]["run_id"]["$ref"] =
            json!("https://example.invalid/remote-schema.json");
        let run = ready_run(run, &profile_bytes);
        assert!(
            validate_and_report(&run_schema, &profile_schema, &profile_bytes, &profile, &run)
                .is_err()
        );
    }
}
