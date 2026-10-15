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
