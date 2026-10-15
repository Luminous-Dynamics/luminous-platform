#![forbid(unsafe_code)]

use jsonschema::{draft202012, Validator};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

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
            if window_start < 0 || window_end <= window_start || interval <= 0 || maximum_gap < interval {
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

 
/// Validate a portable evidence bundle, including payload/normalized digests,
/// source-event uniqueness, forward-only lineage, derived arithmetic, and the
/// classification implied by the referenced objective measurement.
///
/// SHA-256 detects content mismatch; it does not authenticate the source,
/// collector, signer, clock, or adapter.
pub fn validate_evidence_bundle(schema: &Value, bundle: &Value) -> Result<usize, Vec<String>> {
    validate_document(schema, bundle)?;

    let evidence = bundle
        .get("evidence")
        .and_then(Value::as_array)
        .ok_or_else(|| vec!["evidence must be an array".to_string()])?;
    let mut errors = Vec::new();
    let mut seen_ids = HashSet::with_capacity(evidence.len());
    let mut seen_source_events = HashSet::with_capacity(evidence.len());
    let mut positions = HashMap::with_capacity(evidence.len());

    for (index, record) in evidence.iter().enumerate() {
        let id = record
            .get("evidence_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing-evidence-id>");
        if !seen_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate evidence_id {id}"));
        }
        positions.entry(id.to_owned()).or_insert(index);

        let source_id = record
            .get("source")
            .and_then(|source| source.get("source_id"))
            .and_then(Value::as_str)
            .unwrap_or("<missing-source-id>");
        let source_event_id = record
            .get("source")
            .and_then(|source| source.get("source_event_id"))
            .and_then(Value::as_str)
            .unwrap_or("<missing-source-event-id>");
        let source_event_key = (source_id.to_owned(), source_event_id.to_owned());
        if !seen_source_events.insert(source_event_key) {
            errors.push(format!(
                "repeated source event identity for evidence {id}; adjudicate replay/conflict before bundling"
            ));
        }

        let payload = record.get("payload");
        let raw_utf8 = payload
            .and_then(|payload| payload.get("raw_utf8"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let declared_payload_digest = payload
            .and_then(|payload| payload.get("sha256_of_raw_utf8"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if sha256_hex(raw_utf8.as_bytes()) != declared_payload_digest {
            errors.push(format!("evidence {id} raw payload SHA-256 mismatch"));
        }

        let measurement = record.get("measurement");
        match normalized_measurement_digest(measurement) {
            Some(actual) => {
                let expected = measurement
                    .and_then(|value| value.get("normalized_sha256"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if actual != expected {
                    errors.push(format!("evidence {id} normalized measurement SHA-256 mismatch"));
                }
            }
            None => errors.push(format!(
                "evidence {id} cannot be encoded under normalized-measurement-v1"
            )),
        }

        validate_source_normalization(record, id, &mut errors);
    }

    // Require each lineage edge to point backward in the bundle. This yields a
    // deterministic topological order and rules out cycles and forward refs.
    for (index, record) in evidence.iter().enumerate() {
        let id = record
            .get("evidence_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing-evidence-id>");
        let parents = record
            .get("derived_from")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for parent in &parents {
            let Some(parent_id) = parent.as_str() else {
                errors.push(format!("evidence {id} has a non-string derived_from reference"));
                continue;
            };
            match positions.get(parent_id) {
                None => errors.push(format!(
                    "evidence {id} derived_from references unknown evidence {parent_id}"
                )),
                Some(parent_index) if *parent_index >= index => errors.push(format!(
                    "evidence {id} derived_from must point to an earlier record: {parent_id}"
                )),
                Some(_) => {}
            }
        }

        if record.get("kind").and_then(Value::as_str) == Some("derived_measurement") {
            validate_derived_measurement(record, evidence, &parents, id, &mut errors);
        }
    }

    let decision = bundle.get("decision");
    let raw_refs = decision
        .and_then(|decision| decision.get("evidence_refs"))
        .and_then(Value::as_array)
        .ok_or_else(|| vec!["decision.evidence_refs must be an array".to_string()])?;
    let mut decision_refs = HashSet::with_capacity(raw_refs.len());
    for reference in raw_refs {
        let Some(reference_id) = reference.as_str() else {
            errors.push("decision.evidence_refs contains a non-string reference".into());
            continue;
        };
        decision_refs.insert(reference_id.to_owned());
        if !seen_ids.contains(reference_id) {
            errors.push(format!("decision references unknown evidence {reference_id}"));
        }
    }

    let classification = decision
        .and_then(|decision| decision.get("classification"))
        .and_then(Value::as_str)
        .unwrap_or("indeterminate");
    if classification != "indeterminate" {
        let objective = decision.and_then(|decision| decision.get("objective"));
        let metric_name = objective
            .and_then(|objective| objective.get("metric_name"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let target_unit = objective
            .and_then(|objective| objective.get("target_unit"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let target_decimal = objective
            .and_then(|objective| objective.get("target_value_decimal"))
            .and_then(Value::as_str)
            .and_then(parse_decimal);
        let comparator = objective
            .and_then(|objective| objective.get("comparator"))
            .and_then(Value::as_str);
        let candidates: Vec<&Value> = evidence
            .iter()
            .filter(|record| {
                let id = record.get("evidence_id").and_then(Value::as_str).unwrap_or("");
                let measurement = record.get("measurement");
                decision_refs.contains(id)
                    && measurement
                        .and_then(|measurement| measurement.get("metric_name"))
                        .and_then(Value::as_str)
                        == Some(metric_name)
            })
            .collect();

        if candidates.len() != 1 {
            errors.push(
                "non-indeterminate decision requires exactly one referenced objective measurement"
                    .into(),
            );
        } else {
            let measurement = candidates[0].get("measurement");
            let measured_unit = measurement
                .and_then(|measurement| measurement.get("unit"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let validity = measurement
                .and_then(|measurement| measurement.get("validity"))
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let measured_decimal = measurement
                .and_then(|measurement| measurement.get("value_decimal"))
                .and_then(Value::as_str)
                .and_then(parse_decimal);
            if measured_unit != target_unit {
                errors.push("objective target unit does not match referenced measurement".into());
            }
            if validity != "valid" {
                errors.push("non-indeterminate decision references a non-valid measurement".into());
            }
            match (measured_decimal, target_decimal, comparator) {
                (Some(measured), Some(target), Some("lte" | "gte"))
                    if measured_unit == target_unit && validity == "valid" =>
                {
                    let measured_vs_target = compare_decimals(measured, target);
                    let met = match comparator {
                        Some("lte") => measured_vs_target.is_some_and(|order| order.is_le()),
                        Some("gte") => measured_vs_target.is_some_and(|order| order.is_ge()),
                        _ => false,
                    };
                    let expected_classification = if met {
                        "objective_met_within_observed_scope"
                    } else {
                        "objective_target_missed"
                    };
                    let expected_reason = if met {
                        "OBJECTIVE_TARGET_MET"
                    } else {
                        "OBJECTIVE_TARGET_MISSED"
                    };
                    if classification != expected_classification {
                        errors.push(format!(
                            "decision classification does not match measurement and objective: expected {expected_classification}"
                        ));
                    }
                    let actual_reason = decision
                        .and_then(|decision| decision.get("reason_code"))
                        .and_then(Value::as_str);
                    if actual_reason != Some(expected_reason) {
                        errors.push(format!(
                            "decision reason_code does not match measurement and objective: expected {expected_reason}"
                        ));
                    }
                }
                _ => errors.push(
                    "objective measurement, decimal target, or comparator is invalid".into(),
                ),
            }
        }
    }

    if errors.is_empty() {
        Ok(evidence.len())
    } else {
        Err(errors)
    }
}

fn validate_source_normalization(record: &Value, id: &str, errors: &mut Vec<String>) {
    let kind = record.get("kind").and_then(Value::as_str).unwrap_or("");
    let profile = record
        .get("normalization")
        .and_then(|value| value.get("profile_id"))
        .and_then(Value::as_str)
        .unwrap_or("");
    match profile {
        "home-assistant-energy-state-v1" => {
            if kind != "telemetry_sample" {
                errors.push(format!(
                    "evidence {id} uses home-assistant-energy-state-v1 for unsupported kind {kind}"
                ));
                return;
            }
            let adapter_id = record
                .get("source")
                .and_then(|source| source.get("adapter_id"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if !matches!(adapter_id, "synthetic-home-assistant" | "home-assistant") {
                errors.push(format!(
                    "evidence {id} adapter {adapter_id} is not allowed by home-assistant-energy-state-v1"
                ));
                return;
            }
            let raw_utf8 = record
                .get("payload")
                .and_then(|payload| payload.get("raw_utf8"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let raw: Value = match serde_json::from_str(raw_utf8) {
                Ok(value) => value,
                Err(_) => {
                    errors.push(format!("evidence {id} source payload is not valid JSON"));
                    return;
                }
            };
            let entity_id = raw.get("entity_id").and_then(Value::as_str).unwrap_or("");
            let raw_state = raw.get("state").and_then(Value::as_str).unwrap_or("");
            let raw_unit = raw
                .get("unit_of_measurement")
                .and_then(Value::as_str)
                .unwrap_or("");
            let measurement = record.get("measurement");
            let normalized_metric = measurement
                .and_then(|value| value.get("metric_name"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let normalized_value = measurement
                .and_then(|value| value.get("value_decimal"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let normalized_unit = measurement
                .and_then(|value| value.get("unit"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let normalized_validity = measurement
                .and_then(|value| value.get("validity"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if entity_id.is_empty()
                || parse_decimal(raw_state).is_none()
                || raw_state != normalized_value
                || raw_unit != normalized_unit
                || normalized_metric != "energy.total"
                || normalized_validity != "valid"
            {
                errors.push(format!(
                    "evidence {id} raw source fields do not support the normalized energy measurement"
                ));
            }
        }
        "derived-subtract-v1" => {
            if kind != "derived_measurement" {
                errors.push(format!(
                    "evidence {id} uses derived-subtract-v1 for unsupported kind {kind}"
                ));
            }
        }
        _ => errors.push(format!(
            "evidence {id} has unsupported normalization profile {profile}"
        )),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Digest framing v1 is UTF-8 with newline-delimited fields:
/// luminous-operations-measurement-v1, metric_name, value_decimal, unit, validity.
/// Schema patterns prohibit newlines inside each field.
fn normalized_measurement_digest(measurement: Option<&Value>) -> Option<String> {
    let measurement = measurement?;
    let metric_name = measurement.get("metric_name")?.as_str()?;
    let value_decimal = measurement.get("value_decimal")?.as_str()?;
    let unit = measurement.get("unit")?.as_str()?;
    let validity = measurement.get("validity")?.as_str()?;
    let framed = format!(
        "luminous-operations-measurement-v1\n{metric_name}\n{value_decimal}\n{unit}\n{validity}"
    );
    Some(sha256_hex(framed.as_bytes()))
}

fn parse_decimal(value: &str) -> Option<(i128, u32)> {
    if value.len() > 48 {
        return None;
    }
    let (negative, unsigned) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value),
    };
    let (whole, fractional) = match unsigned.split_once('.') {
        Some((whole, fractional)) => (whole, fractional),
        None => (unsigned, ""),
    };
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fractional.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let digits = format!("{whole}{fractional}");
    let mut coefficient = digits.parse::<i128>().ok()?;
    if negative {
        coefficient = coefficient.checked_neg()?;
    }
    Some((coefficient, fractional.len() as u32))
}

fn decimal_power_of_ten(power: u32) -> Option<i128> {
    if power > 18 {
        return None;
    }
    (0..power).try_fold(1_i128, |acc, _| acc.checked_mul(10))
}

fn compare_decimals(left: (i128, u32), right: (i128, u32)) -> Option<std::cmp::Ordering> {
    let scale = left.1.max(right.1);
    let left_scaled = left.0.checked_mul(decimal_power_of_ten(scale.checked_sub(left.1)?)?)?;
    let right_scaled = right.0.checked_mul(decimal_power_of_ten(scale.checked_sub(right.1)?)?)?;
    Some(left_scaled.cmp(&right_scaled))
}

fn subtract_decimals_matches(end: &str, start: &str, result: &str) -> Option<bool> {
    let end = parse_decimal(end)?;
    let start = parse_decimal(start)?;
    let result = parse_decimal(result)?;
    let scale = end.1.max(start.1).max(result.1);
    let end_scaled = end.0.checked_mul(decimal_power_of_ten(scale.checked_sub(end.1)?)?)?;
    let start_scaled = start.0.checked_mul(decimal_power_of_ten(scale.checked_sub(start.1)?)?)?;
    let result_scaled =
        result.0.checked_mul(decimal_power_of_ten(scale.checked_sub(result.1)?)?)?;
    Some(end_scaled.checked_sub(start_scaled)? == result_scaled)
}

fn validate_derived_measurement(
    record: &Value,
    evidence: &[Value],
    parents: &[Value],
    id: &str,
    errors: &mut Vec<String>,
) {
    let raw_utf8 = record
        .get("payload")
        .and_then(|payload| payload.get("raw_utf8"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let derived_payload: Value = match serde_json::from_str(raw_utf8) {
        Ok(value) => value,
        Err(_) => {
            errors.push(format!("derived evidence {id} raw payload is not JSON"));
            return;
        }
    };
    let measurement = record.get("measurement");
    for field in ["metric_name", "value_decimal", "unit", "validity"] {
        if derived_payload.get(field) != measurement.and_then(|measurement| measurement.get(field)) {
            errors.push(format!(
                "derived evidence {id} raw payload field {field} differs from normalized measurement"
            ));
        }
    }
    if derived_payload
        .get("derived_from")
        .and_then(Value::as_array)
        .map(|values| values.as_slice())
        != Some(parents)
    {
        errors.push(format!(
            "derived evidence {id} raw payload lineage differs from the evidence manifest"
        ));
    }
    if derived_payload.get("operation").and_then(Value::as_str) != Some("subtract") {
        errors.push(format!("derived evidence {id} uses an unsupported operation"));
        return;
    }
    let start_id = derived_payload
        .get("start_evidence_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let end_id = derived_payload
        .get("end_evidence_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    if parents.len() != 2
        || parents[0].as_str() != Some(start_id)
        || parents[1].as_str() != Some(end_id)
        || start_id == end_id
    {
        errors.push(format!(
            "derived evidence {id} must bind ordered start/end evidence references"
        ));
        return;
    }
    let start = evidence
        .iter()
        .find(|candidate| candidate.get("evidence_id").and_then(Value::as_str) == Some(start_id));
    let end = evidence
        .iter()
        .find(|candidate| candidate.get("evidence_id").and_then(Value::as_str) == Some(end_id));
    let (Some(start), Some(end)) = (start, end) else {
        errors.push(format!("derived evidence {id} has a missing input record"));
        return;
    };
    let start_measurement = start.get("measurement");
    let end_measurement = end.get("measurement");
    let output_measurement = measurement;
    let start_metric = start_measurement.and_then(|value| value.get("metric_name")).and_then(Value::as_str);
    let end_metric = end_measurement.and_then(|value| value.get("metric_name")).and_then(Value::as_str);
    let output_unit = output_measurement.and_then(|value| value.get("unit")).and_then(Value::as_str);
    let start_unit = start_measurement.and_then(|value| value.get("unit")).and_then(Value::as_str);
    let end_unit = end_measurement.and_then(|value| value.get("unit")).and_then(Value::as_str);
    let start_validity = start_measurement.and_then(|value| value.get("validity")).and_then(Value::as_str);
    let end_validity = end_measurement.and_then(|value| value.get("validity")).and_then(Value::as_str);
    let start_value = start_measurement.and_then(|value| value.get("value_decimal")).and_then(Value::as_str);
    let end_value = end_measurement.and_then(|value| value.get("value_decimal")).and_then(Value::as_str);
    let output_value = output_measurement.and_then(|value| value.get("value_decimal")).and_then(Value::as_str);
    if start_metric != end_metric
        || start_unit != output_unit
        || end_unit != output_unit
        || start_validity != Some("valid")
        || end_validity != Some("valid")
    {
        errors.push(format!("derived evidence {id} input measurements are incompatible or invalid"));
        return;
    }
    match (end_value, start_value, output_value) {
        (Some(end_value), Some(start_value), Some(output_value))
            if subtract_decimals_matches(end_value, start_value, output_value) == Some(true) => {}
        _ => errors.push(format!(
            "derived evidence {id} does not equal end minus start under exact decimal arithmetic"
        )),
    }
}
