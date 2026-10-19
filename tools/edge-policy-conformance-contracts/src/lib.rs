//! Offline validation for the synthetic edge policy-conformance corpus.
//!
//! A valid corpus specifies what must be tested. It is not evidence that a
//! data plane enforces policy, and it never authorizes a network change.

#![forbid(unsafe_code)]

use std::collections::HashSet;

use jsonschema::draft202012;
use serde_json::Value;

const MAX_JSON_BYTES: usize = 2_097_152;

fn reject_remote_references(value: &Value, path: &str, errors: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if key == "$ref" || key == "$dynamicRef" {
                    match child.as_str() {
                        Some(reference) if reference.starts_with('#') => {}
                        Some(_) => errors.push(format!(
                            "{path}/{key}: only local JSON Schema references are allowed"
                        )),
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

fn parse_document(source: &str, label: &str) -> Result<Value, Vec<String>> {
    if source.len() > MAX_JSON_BYTES {
        return Err(vec![format!(
            "{label}: input exceeds {MAX_JSON_BYTES} bytes"
        )]);
    }
    serde_json::from_str(source).map_err(|error| vec![format!("{label}: invalid JSON: {error}")])
}

fn semantic_checks(corpus: &Value, errors: &mut Vec<String>) {
    let required_invariants = [
        "zero_unauthorized_effects",
        "unknown_or_stale_authority_denies_high_risk",
        "independent_data_plane_probe_required",
        "no_identity_or_payload_export",
    ];
    let invariants = corpus
        .get("non_negotiable_invariants")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for invariant in required_invariants {
        if !invariants
            .iter()
            .any(|value| value.as_str() == Some(invariant))
        {
            errors.push(format!("missing non-negotiable invariant: {invariant}"));
        }
    }

    let cases = match corpus.get("cases").and_then(Value::as_array) {
        Some(cases) => cases,
        None => {
            errors.push("cases must be an array".to_owned());
            return;
        }
    };
    if cases.len() > 1000 {
        errors.push("case count exceeds 1000".to_owned());
        return;
    }

    let mut ids = HashSet::with_capacity(cases.len());
    for (index, case) in cases.iter().enumerate() {
        let case_id = case.get("case_id").and_then(Value::as_str).unwrap_or("");
        if !ids.insert(case_id.to_owned()) {
            errors.push(format!("/cases/{index}/case_id duplicates {case_id}"));
        }
        if case
            .get("independent_probe_required")
            .and_then(Value::as_bool)
            != Some(true)
        {
            errors.push(format!(
                "/cases/{index}/independent_probe_required must be true"
            ));
        }
        let evidence = case.get("evidence_required").and_then(Value::as_array);
        if !evidence.is_some_and(|items| {
            items
                .iter()
                .any(|item| item.as_str() == Some("independent_probe"))
        }) {
            errors.push(format!(
                "/cases/{index}/evidence_required must include independent_probe"
            ));
        }

        let authority = case
            .get("authority_state")
            .and_then(Value::as_str)
            .unwrap_or("");
        let decision = case
            .get("intended_decision")
            .and_then(Value::as_str)
            .unwrap_or("");
        let probe = case
            .get("expected_probe_outcome")
            .and_then(Value::as_str)
            .unwrap_or("");
        if authority != "valid" && (decision != "deny" || probe != "blocked") {
            errors.push(format!(
                "/cases/{index}: non-valid authority must deny and expect a blocked probe"
            ));
        }
        if decision == "deny" && probe != "blocked" {
            errors.push(format!(
                "/cases/{index}: deny decision cannot expect a permitted probe"
            ));
        }
        if decision == "allow" && (authority != "valid" || probe != "permitted") {
            errors.push(format!(
                "/cases/{index}: allow requires valid authority and a permitted positive-control probe"
            ));
        }
    }
}

/// Validate a synthetic conformance corpus against a local Draft 2020-12 schema
/// and semantic invariants. This validates benchmark intent, not runtime behavior.
pub fn validate_corpus(schema_source: &str, corpus_source: &str) -> Result<Value, Vec<String>> {
    let schema = parse_document(schema_source, "schema")?;
    let corpus = parse_document(corpus_source, "corpus")?;
    let mut errors = Vec::new();

    reject_remote_references(&schema, "$", &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }

    let meta = jsonschema::meta::validator_for(&schema)
        .map_err(|error| vec![format!("cannot build schema meta-validator: {error}")])?;
    if !meta.evaluate(&schema).flag().valid {
        return Err(vec!["schema failed JSON Schema meta-validation".to_owned()]);
    }

    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| vec![format!("cannot build Draft 2020-12 validator: {error}")])?;
    errors.extend(
        validator
            .iter_errors(&corpus)
            .map(|error| format!("schema: {error}")),
    );
    if !errors.is_empty() {
        return Err(errors);
    }

    semantic_checks(&corpus, &mut errors);
    if errors.is_empty() {
        Ok(corpus)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_JSON_BYTES, validate_corpus};
    use serde_json::Value;

    const SCHEMA: &str =
        include_str!("../../../schemas/edge-policy-conformance-corpus-v1.schema.json");
    const CORPUS: &str =
        include_str!("../../../examples/edge-policy-conformance-corpus-synthetic-v1.json");

    #[test]
    fn synthetic_corpus_validates_without_claiming_measurements() {
        let corpus = validate_corpus(SCHEMA, CORPUS).expect("synthetic corpus should validate");
        assert_eq!(corpus["evidence_class"], "synthetic_contract");
        assert_eq!(corpus["claim_ceiling"], "synthetic_only_not_a_measurement");
        assert_eq!(corpus["cases"].as_array().expect("cases").len(), 13);
    }

    #[test]
    fn stale_authority_cannot_be_changed_to_allow() {
        let mut corpus: Value = serde_json::from_str(CORPUS).expect("fixture JSON");
        corpus["cases"][2]["intended_decision"] = Value::String("allow".to_owned());
        corpus["cases"][2]["expected_probe_outcome"] = Value::String("permitted".to_owned());
        let errors = validate_corpus(SCHEMA, &corpus.to_string()).expect_err("stale allow");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("non-valid authority"))
        );
    }

    #[test]
    fn deny_cannot_expect_a_permitted_probe() {
        let mut corpus: Value = serde_json::from_str(CORPUS).expect("fixture JSON");
        corpus["cases"][1]["expected_probe_outcome"] = Value::String("permitted".to_owned());
        assert!(validate_corpus(SCHEMA, &corpus.to_string()).is_err());
    }

    #[test]
    fn duplicate_case_ids_are_rejected() {
        let mut corpus: Value = serde_json::from_str(CORPUS).expect("fixture JSON");
        corpus["cases"][1]["case_id"] = corpus["cases"][0]["case_id"].clone();
        let errors = validate_corpus(SCHEMA, &corpus.to_string()).expect_err("duplicate ID");
        assert!(errors.iter().any(|error| error.contains("duplicates")));
    }

    #[test]
    fn unknown_fields_fail_closed() {
        let mut corpus: Value = serde_json::from_str(CORPUS).expect("fixture JSON");
        corpus["cases"][0]["subscriber_mac"] = Value::String("must-not-be-exported".to_owned());
        let errors = validate_corpus(SCHEMA, &corpus.to_string()).expect_err("unknown field");
        assert!(errors.iter().any(|error| error.contains("subscriber_mac")));
    }

    #[test]
    fn remote_schema_references_are_rejected() {
        let mut schema: Value = serde_json::from_str(SCHEMA).expect("schema JSON");
        schema["properties"]["corpus_id"] =
            serde_json::json!({ "$ref": "https://example.invalid/remote-schema.json" });
        let errors =
            validate_corpus(&schema.to_string(), CORPUS).expect_err("remote ref must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("only local JSON Schema references"))
        );
    }

    #[test]
    fn oversized_documents_are_rejected() {
        let oversized = " ".repeat(MAX_JSON_BYTES + 1);
        assert!(validate_corpus(SCHEMA, &oversized).is_err());
    }

    #[test]
    fn positive_control_cannot_be_removed_to_make_deny_all_look_safe() {
        let mut corpus: Value = serde_json::from_str(CORPUS).expect("fixture JSON");
        for case in corpus["cases"].as_array_mut().expect("cases") {
            if case["intended_decision"] == "allow" {
                case["authority_state"] = Value::String("unknown".to_owned());
            }
        }
        let errors = validate_corpus(SCHEMA, &corpus.to_string()).expect_err("invalid positives");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("non-valid authority"))
        );
    }
}
