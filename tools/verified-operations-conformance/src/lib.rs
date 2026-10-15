#![forbid(unsafe_code)]

use jsonschema::{draft202012, Validator};
use serde_json::Value;
use std::collections::HashSet;

pub const SCHEMA_DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";
pub const PROFILE_ID: &str = "verified-operations-pilot-v1";
pub const CONTRACT_VERSION: &str = "1.0.0";

fn validator_for(schema: &Value) -> Result<Validator, Vec<String>> {
    if schema.get("$schema").and_then(Value::as_str) != Some(SCHEMA_DRAFT) {
        return Err(vec![format!(
            "schema must declare the exact Draft 2020-12 meta-schema URI {SCHEMA_DRAFT}"
        )]);
    }

    if !draft202012::meta::is_valid(schema) {
        return Err(vec![
            "schema does not validate against the Draft 2020-12 meta-schema".into(),
        ]);
    }

    draft202012::options()
        .offline()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| vec![format!("schema compilation failed: {error}")])
}

pub fn validate_schema(schema: &Value) -> Result<(), Vec<String>> {
    validator_for(schema).map(|_| ())
}

pub fn validate_document(schema: &Value, document: &Value) -> Result<(), Vec<String>> {
    let validator = validator_for(schema)?;
    let errors: Vec<String> = validator
        .iter_errors(document)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate the Draft 2020-12 shape and the semantic invariants that JSON Schema
/// cannot express alone (unique scenario IDs, fail-closed action flag, and the
/// deterministic expected-outcome classifier).
pub fn validate_corpus(schema: &Value, corpus: &Value) -> Result<usize, Vec<String>> {
    validate_document(schema, corpus)?;

    let scenarios = corpus
        .get("scenarios")
        .and_then(Value::as_array)
        .ok_or_else(|| vec!["scenarios must be an array".to_string()])?;

    let mut seen = HashSet::with_capacity(scenarios.len());
    let mut errors = Vec::new();

    for (index, scenario) in scenarios.iter().enumerate() {
        let label = scenario
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("<missing-id>");

        if !seen.insert(label.to_owned()) {
            errors.push(format!("scenarios[{index}] duplicates scenario id {label}"));
        }

        let Some(expected) = scenario.get("expected") else {
            errors.push(format!("scenario {label} is missing expected outcomes"));
            continue;
        };
        if expected
            .get("automatic_mutation_allowed")
            .and_then(Value::as_bool)
            != Some(false)
        {
            errors.push(format!(
                "scenario {label} must keep automatic_mutation_allowed=false"
            ));
        }
        if expected
            .get("must_preserve_provenance")
            .and_then(Value::as_bool)
            != Some(true)
        {
            errors.push(format!(
                "scenario {label} must keep must_preserve_provenance=true"
            ));
        }

        let declared_classification = expected
            .get("classification")
            .and_then(Value::as_str);
        let declared_reason = expected.get("reason_code").and_then(Value::as_str);
        match classify(scenario) {
            Some((classification, reason_code)) => {
                if declared_classification != Some(classification) {
                    errors.push(format!(
                        "scenario {label} classification mismatch: declared {:?}, calculated {}",
                        declared_classification, classification
                    ));
                }
                if declared_reason != Some(reason_code) {
                    errors.push(format!(
                        "scenario {label} reason-code mismatch: declared {:?}, calculated {}",
                        declared_reason, reason_code
                    ));
                }
            }
            None => errors.push(format!(
                "scenario {label} has an unsupported or incomplete semantic state"
            )),
        }
    }

    if errors.is_empty() {
        Ok(scenarios.len())
    } else {
        Err(errors)
    }
}

fn fact_str<'a>(scenario: &'a Value, name: &str) -> Option<&'a str> {
    scenario.get("facts")?.get(name)?.as_str()
}

fn fact_bool(scenario: &Value, name: &str) -> Option<bool> {
    scenario.get("facts")?.get(name)?.as_bool()
}

fn classify(scenario: &Value) -> Option<(&'static str, &'static str)> {
    match scenario.get("domain")?.as_str()? {
        "asset_health" => {
            if fact_bool(scenario, "asset_in_scope") == Some(false) {
                return Some(("denied", "ASSET_OUT_OF_SCOPE"));
            }
            if fact_bool(scenario, "source_present") != Some(true) {
                return Some(("indeterminate", "REQUIRED_SOURCE_MISSING"));
            }
            if fact_bool(scenario, "source_valid") != Some(true) {
                return Some(("indeterminate", "SOURCE_PAYLOAD_INVALID"));
            }
            if fact_bool(scenario, "observation_fresh") != Some(true) {
                return Some(("indeterminate", "OBSERVATION_STALE"));
            }
            match fact_bool(scenario, "sources_agree") {
                Some(true) => Some(("healthy_within_observed_scope", "OBSERVATION_SET_CURRENT")),
                Some(false) => Some(("indeterminate", "OBSERVATION_CONFLICT")),
                None => Some(("indeterminate", "OBSERVATION_AGREEMENT_UNKNOWN")),
            }
        }
        "backup_recovery" => {
            if fact_bool(scenario, "source_present") != Some(true) {
                return Some(("indeterminate", "REQUIRED_SOURCE_MISSING"));
            }
            if fact_bool(scenario, "observation_fresh") != Some(true) {
                return Some(("indeterminate", "OBSERVATION_STALE"));
            }
            if fact_bool(scenario, "source_valid") != Some(true) {
                return Some(("indeterminate", "SOURCE_PAYLOAD_INVALID"));
            }
            if fact_str(scenario, "backup_job") == Some("failed") {
                return Some(("actionable_finding", "BACKUP_JOB_FAILED"));
            }
            match fact_str(scenario, "restore_test")? {
                "failed" => Some(("actionable_finding", "RESTORE_TEST_FAILED")),
                "passed" => Some(("verified", "RESTORE_TEST_PASSED")),
                "not_run" | "stale" | "unknown" | "not_applicable" => {
                    Some(("recovery_not_demonstrated", "RESTORE_NOT_DEMONSTRATED"))
                }
                _ => None,
            }
        }
        "connector_security" => match fact_str(scenario, "credential")? {
            "revoked" => Some(("denied", "CONNECTOR_CREDENTIAL_REVOKED")),
            "expired" => Some(("denied", "CONNECTOR_CREDENTIAL_EXPIRED")),
            "overprivileged" => Some(("denied", "CONNECTOR_CREDENTIAL_OVERPRIVILEGED")),
            "valid_scoped" => Some(("healthy_within_observed_scope", "CONNECTOR_CREDENTIAL_SCOPED")),
            "not_applicable" => Some(("indeterminate", "CONNECTOR_CREDENTIAL_UNKNOWN")),
            _ => None,
        },
        "event_processing" => match fact_str(scenario, "event_delivery")? {
            "duplicate_same_content" => Some(("deduplicated", "EVENT_DUPLICATE_IDENTICAL")),
            "duplicate_conflicting_content" => {
                Some(("quarantine_conflict", "EVENT_ID_CONTENT_CONFLICT"))
            }
            "out_of_order" => Some(("indeterminate", "EVENT_ORDER_UNRESOLVED")),
            "first_delivery" if fact_bool(scenario, "source_valid") == Some(true) => {
                Some(("healthy_within_observed_scope", "EVENT_FIRST_DELIVERY_VALID"))
            }
            "first_delivery" => Some(("indeterminate", "EVENT_SOURCE_INVALID")),
            "not_applicable" => Some(("indeterminate", "EVENT_STATE_NOT_APPLICABLE")),
            _ => None,
        },
        "outcome_objective" => {
            if fact_bool(scenario, "objective_scope_matches") != Some(true) {
                return Some(("indeterminate", "OBJECTIVE_SCOPE_MISMATCH"));
            }
            if fact_bool(scenario, "objective_defined") != Some(true) {
                return Some(("indeterminate", "OBJECTIVE_UNDEFINED"));
            }
            if fact_bool(scenario, "measurement_valid") != Some(true) {
                return Some(("indeterminate", "OBJECTIVE_MEASUREMENT_INVALID"));
            }

            // Derive coverage from observed sample positions and the declared schedule.
            // Never trust a caller-supplied coverage ratio or "window complete" boolean.
            let facts = scenario.get("facts")?;
            let window_start = facts
                .get("observation_window_start_epoch_seconds")
                .and_then(Value::as_i64);
            let window_end = facts
                .get("observation_window_end_epoch_seconds")
                .and_then(Value::as_i64);
            let interval = facts
                .get("expected_interval_seconds")
                .and_then(Value::as_i64);
            let maximum_gap = facts
                .get("maximum_allowed_gap_seconds")
                .and_then(Value::as_i64);
            let offsets_value = facts.get("valid_sample_offsets_seconds");
            let (Some(window_start), Some(window_end), Some(interval), Some(maximum_gap), Some(offsets_value)) =
                (window_start, window_end, interval, maximum_gap, offsets_value)
            else {
                return Some(("indeterminate", "OBJECTIVE_TIMELINE_UNKNOWN"));
            };
            if window_end <= window_start || interval <= 0 || maximum_gap < interval {
                return Some(("indeterminate", "OBJECTIVE_WINDOW_POLICY_INVALID"));
            }

            let duration = window_end - window_start;
            if duration % interval != 0 {
                return Some(("indeterminate", "OBJECTIVE_WINDOW_POLICY_INVALID"));
            }

            let Some(raw_offsets) = offsets_value.as_array() else {
                return Some(("indeterminate", "OBJECTIVE_TIMELINE_INVALID"));
            };
            if raw_offsets.len() < 2 {
                return Some(("indeterminate", "OBJECTIVE_TIMELINE_INVALID"));
            }
            let mut offsets = Vec::with_capacity(raw_offsets.len());
            for value in raw_offsets {
                let Some(offset) = value.as_i64() else {
                    return Some(("indeterminate", "OBJECTIVE_TIMELINE_INVALID"));
                };
                if offset < 0 || offset > duration || offset % interval != 0 {
                    return Some(("indeterminate", "OBJECTIVE_TIMELINE_INVALID"));
                }
                offsets.push(offset);
            }
            offsets.sort_unstable();
            if offsets.windows(2).any(|pair| pair[0] == pair[1]) {
                return Some(("indeterminate", "OBJECTIVE_TIMELINE_INVALID"));
            }

            if offsets.first() != Some(&0) || offsets.last() != Some(&duration) {
                return Some(("indeterminate", "OBJECTIVE_WINDOW_INCOMPLETE"));
            }

            let expected_count = (duration / interval + 1) as f64;
            let coverage = offsets.len() as f64 / expected_count;
            let minimum_coverage = scenario
                .get("facts")
                .and_then(|facts| facts.get("minimum_coverage_ratio"))
                .and_then(Value::as_f64);
            let Some(minimum_coverage) = minimum_coverage else {
                return Some(("indeterminate", "OBJECTIVE_COVERAGE_UNKNOWN"));
            };
            if !(0.0..=1.0).contains(&minimum_coverage) {
                return Some(("indeterminate", "OBJECTIVE_COVERAGE_POLICY_INVALID"));
            }
            if coverage < minimum_coverage {
                return Some(("indeterminate", "OBJECTIVE_COVERAGE_INSUFFICIENT"));
            }

            if offsets
                .windows(2)
                .any(|pair| pair[1] - pair[0] > maximum_gap)
            {
                return Some(("indeterminate", "OBJECTIVE_GAP_TOO_LONG"));
            }

            let measured_unit = fact_str(scenario, "measured_unit");
            let target_unit = fact_str(scenario, "target_unit");
            if measured_unit.is_none() || target_unit.is_none() || measured_unit != target_unit {
                return Some(("indeterminate", "OBJECTIVE_UNITS_INCOMPATIBLE"));
            }

            if fact_bool(scenario, "independent_measurement_present") != Some(true) {
                return Some(("indeterminate", "OBJECTIVE_INDEPENDENT_MEASUREMENT_MISSING"));
            }

            let measured_value = scenario
                .get("facts")
                .and_then(|facts| facts.get("measured_value"))
                .and_then(Value::as_f64);
            let target_value = scenario
                .get("facts")
                .and_then(|facts| facts.get("target_value"))
                .and_then(Value::as_f64);
            let (Some(measured_value), Some(target_value)) = (measured_value, target_value) else {
                return Some(("indeterminate", "OBJECTIVE_MEASUREMENT_INVALID"));
            };

            match fact_str(scenario, "comparator") {
                Some("lte") if measured_value <= target_value => {
                    Some(("objective_met_within_observed_scope", "OBJECTIVE_TARGET_MET"))
                }
                Some("lte") => Some(("objective_target_missed", "OBJECTIVE_TARGET_MISSED")),
                Some("gte") if measured_value >= target_value => {
                    Some(("objective_met_within_observed_scope", "OBJECTIVE_TARGET_MET"))
                }
                Some("gte") => Some(("objective_target_missed", "OBJECTIVE_TARGET_MISSED")),
                _ => Some(("indeterminate", "OBJECTIVE_COMPARATOR_UNKNOWN")),
            }
        }
        "change_execution" => {
            match fact_str(scenario, "authorization")? {
                "missing" => return Some(("denied", "AUTHORIZATION_MISSING")),
                "expired" => return Some(("denied", "AUTHORIZATION_EXPIRED")),
                "wrong_target" => return Some(("denied", "AUTHORIZATION_TARGET_MISMATCH")),
                "revoked" => return Some(("denied", "AUTHORIZATION_REVOKED")),
                "valid" => {}
                "not_applicable" => {
                    return Some(("denied", "AUTHORIZATION_NOT_APPLICABLE"))
                }
                _ => return None,
            }

            match fact_str(scenario, "pre_state")? {
                "stale" => return Some(("requires_replan", "PRESTATE_STALE_REPLAN")),
                "unknown" => return Some(("requires_replan", "PRESTATE_UNKNOWN_REPLAN")),
                "fresh" | "not_applicable" => {}
                _ => return None,
            }

            match fact_str(scenario, "dispatch")? {
                "partial" => Some(("effect_partial", "PARTIAL_EFFECT_RECONCILE")),
                "unknown" => Some(("effect_unknown", "EFFECT_OUTCOME_UNKNOWN")),
                "not_dispatched" => Some(("requires_replan", "EFFECT_NOT_DISPATCHED")),
                "dispatched" => match fact_str(scenario, "postcondition")? {
                    "independently_verified" => Some(("verified", "POSTCONDITION_VERIFIED")),
                    "executor_asserted_only" => {
                        Some(("effect_unknown", "INDEPENDENT_VERIFIER_MISSING"))
                    }
                    "failed" => Some(("effect_partial", "POSTCONDITION_FAILED_RECONCILE")),
                    "missing" => Some(("effect_unknown", "EFFECT_DISPATCHED_UNVERIFIED")),
                    "not_applicable" => Some(("effect_unknown", "POSTCONDITION_NOT_APPLICABLE")),
                    _ => None,
                },
                "not_applicable" => Some(("indeterminate", "DISPATCH_STATE_NOT_APPLICABLE")),
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn classify_for_test(scenario: &Value) -> Option<(&'static str, &'static str)> {
    classify(scenario)
}
