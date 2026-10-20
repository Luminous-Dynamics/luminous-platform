//! Reconcile submitted policy observations against the frozen benchmark corpus.
//!
//! This crate does not generate traffic or configure hosts. Hashes and source
//! declarations are not authenticated merely because this validator accepts them.

#![forbid(unsafe_code)]

pub mod adapter_capability;
pub mod promotion_journal;
pub mod replay_ledger;
pub mod rollout_gate;

pub mod freshness;

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use jsonschema::draft202012;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MAX_JSON_BYTES: usize = 33_554_432;
const MAX_CASES: usize = 20_000;

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

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

fn validate_document(
    schema_source: &str,
    document_source: &str,
    label: &str,
) -> Result<Value, Vec<String>> {
    let schema = parse_document(schema_source, &format!("{label} schema"))?;
    let document = parse_document(document_source, label)?;
    let mut errors = Vec::new();

    reject_remote_references(&schema, "$", &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }

    let meta = jsonschema::meta::validator_for(&schema).map_err(|error| {
        vec![format!(
            "{label}: cannot build schema meta-validator: {error}"
        )]
    })?;
    if !meta.evaluate(&schema).flag().valid {
        return Err(vec![format!("{label}: schema failed meta-validation")]);
    }

    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| {
            vec![format!(
                "{label}: cannot build Draft 2020-12 validator: {error}"
            )]
        })?;
    errors.extend(
        validator
            .iter_errors(&document)
            .map(|error| format!("{label} schema: {error}")),
    );
    if errors.is_empty() {
        Ok(document)
    } else {
        Err(errors)
    }
}

fn value_string<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn evidence_problem(receipt: &Value, observation: &Value, case: &Value) -> Option<String> {
    let evidence_class = value_string(receipt, "/evidence_class").unwrap_or("");
    let source = value_string(observation, "/evidence_source").unwrap_or("");
    let outcome = value_string(observation, "/observed_outcome").unwrap_or("");

    if evidence_class == "synthetic_fixture" {
        if value_string(receipt, "/time_basis/kind") != Some("synthetic")
            || source != "synthetic_fixture"
            || !observation["observed_at_unix_ms"].is_null()
            || !observation["monotonic_elapsed_ms"].is_null()
            || !observation["duration_ms"].is_null()
            || !observation["evidence_sha256"].is_null()
        {
            return Some(
                "synthetic fixture has lab-only fields or a non-synthetic source".to_owned(),
            );
        }
        return None;
    }

    if !matches!(
        evidence_class,
        "lab_observation_claim" | "namespace_lab_observation"
    ) {
        return Some("unsupported evidence class".to_owned());
    }
    if source != "independent_dataplane_probe" {
        return Some(format!(
            "source {source} is not an independent data-plane probe"
        ));
    }
    if value_string(receipt, "/time_basis/kind") != Some("monotonic_with_wall_anchor")
        || receipt
            .pointer("/time_basis/wall_anchor_unix_ms")
            .is_none_or(Value::is_null)
        || receipt
            .pointer("/time_basis/monotonic_origin_ms")
            .is_none_or(Value::is_null)
        || observation["observed_at_unix_ms"].is_null()
        || observation["monotonic_elapsed_ms"].is_null()
        || observation["evidence_sha256"].is_null()
        || observation["effective_policy_sha256"].is_null()
    {
        return Some(
            "lab observation lacks its required time anchor or evidence digest".to_owned(),
        );
    }
    if observation["monotonic_elapsed_ms"].as_u64().is_none() {
        return Some("monotonic elapsed time is missing".to_owned());
    }

    if case["target_response_ms"].as_u64().is_some()
        && (observation["duration_ms"].is_null() || outcome == "not_run")
    {
        return Some("case has a response-time target but no measured duration".to_owned());
    }
    None
}

/// Construct a deterministic synthetic receipt from the corpus. The output is
/// useful for end-to-end smoke testing but is never a lab result.
pub fn build_synthetic_receipt(corpus_source: &str) -> Result<Value, Vec<String>> {
    let corpus = parse_document(corpus_source, "corpus")?;
    let cases = corpus
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| vec!["corpus.cases must be an array".to_owned()])?;
    if cases.len() > MAX_CASES {
        return Err(vec![format!("corpus exceeds {MAX_CASES} cases")]);
    }

    let mut observations = Vec::with_capacity(cases.len());
    for (index, case) in cases.iter().enumerate() {
        let case_id = case
            .get("case_id")
            .and_then(Value::as_str)
            .ok_or_else(|| vec![format!("corpus.cases[{index}].case_id must be a string")])?;
        let expected = case
            .get("expected_probe_outcome")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                vec![format!(
                    "corpus.cases[{index}].expected_probe_outcome must be a string"
                )]
            })?;
        if !matches!(expected, "permitted" | "blocked") {
            return Err(vec![format!(
                "corpus.cases[{index}] has unsupported expected outcome {expected}"
            )]);
        }
        observations.push(json!({
            "observation_id": format!("OBS-SYN-{:05}", index + 1),
            "case_id": case_id,
            "observed_outcome": expected,
            "evidence_source": "synthetic_fixture",
            "effective_policy_sha256": "b".repeat(64),
            "observed_at_unix_ms": null,
            "monotonic_elapsed_ms": null,
            "duration_ms": null,
            "evidence_sha256": null
        }));
    }

    Ok(json!({
        "schema_version": "1.0.0",
        "run_id": "synthetic-smoke-v1",
        "evidence_class": "synthetic_fixture",
        "corpus_sha256": sha256_hex(corpus_source.as_bytes()),
        "approved_policy_sha256": "a".repeat(64),
        "effective_config_sha256": "b".repeat(64),
        "target_token": "target-synthetic-lab",
        "time_basis": {
            "kind": "synthetic",
            "clock_id": "synthetic-clock",
            "wall_anchor_unix_ms": null,
            "monotonic_origin_ms": null
        },
        "observations": observations
    }))
}

/// Reconcile a receipt with exact corpus bytes and return a deterministic report.
///
/// A clean report for a submitted lab observation is still unauthenticated
/// because this function does not verify device attestation or signer identity.
pub fn reconcile(
    corpus_schema_source: &str,
    corpus_source: &str,
    receipt_schema_source: &str,
    receipt_source: &str,
) -> Result<Value, Vec<String>> {
    let corpus = validate_document(corpus_schema_source, corpus_source, "corpus")?;
    let receipt = validate_document(receipt_schema_source, receipt_source, "receipt")?;

    let actual_corpus_sha256 = sha256_hex(corpus_source.as_bytes());
    if receipt["corpus_sha256"].as_str() != Some(actual_corpus_sha256.as_str()) {
        return Err(vec![
            "receipt.corpus_sha256 does not match the exact corpus bytes".to_owned(),
        ]);
    }

    let cases = corpus["cases"]
        .as_array()
        .ok_or_else(|| vec!["corpus.cases must be an array".to_owned()])?;
    if cases.len() > MAX_CASES {
        return Err(vec![format!("corpus exceeds {MAX_CASES} cases")]);
    }
    let observations = receipt["observations"]
        .as_array()
        .ok_or_else(|| vec!["receipt.observations must be an array".to_owned()])?;

    let mut corpus_cases: HashMap<&str, &Value> = HashMap::with_capacity(cases.len());
    for (index, case) in cases.iter().enumerate() {
        let id = case["case_id"]
            .as_str()
            .ok_or_else(|| vec![format!("corpus.cases[{index}].case_id must be a string")])?;
        if corpus_cases.insert(id, case).is_some() {
            return Err(vec![format!("corpus contains duplicate case ID {id}")]);
        }
    }

    let mut observation_ids = HashSet::with_capacity(observations.len());
    let mut observed_cases: HashMap<&str, &Value> = HashMap::with_capacity(observations.len());
    for (index, observation) in observations.iter().enumerate() {
        let observation_id = observation["observation_id"].as_str().ok_or_else(|| {
            vec![format!(
                "receipt.observations[{index}].observation_id must be a string"
            )]
        })?;
        if !observation_ids.insert(observation_id) {
            return Err(vec![format!("duplicate observation ID {observation_id}")]);
        }
        let case_id = observation["case_id"].as_str().ok_or_else(|| {
            vec![format!(
                "receipt.observations[{index}].case_id must be a string"
            )]
        })?;
        if !corpus_cases.contains_key(case_id) {
            return Err(vec![format!(
                "observation references unknown case ID {case_id}"
            )]);
        }
        if observed_cases.insert(case_id, observation).is_some() {
            return Err(vec![format!("duplicate observation for case ID {case_id}")]);
        }
    }

    // Validate monotonic time in submitted trial order, not corpus order:
    // workloads may be randomized and the corpus is only an expectation set.
    let mut reversed_clock_cases = HashSet::new();
    if matches!(
        receipt["evidence_class"].as_str(),
        Some("lab_observation_claim" | "namespace_lab_observation")
    ) {
        let mut previous_elapsed = None;
        for observation in observations {
            let case_id = observation["case_id"].as_str().unwrap_or("");
            if let Some(current_elapsed) = observation["monotonic_elapsed_ms"].as_u64() {
                if previous_elapsed.is_some_and(|previous| current_elapsed < previous) {
                    reversed_clock_cases.insert(case_id.to_owned());
                } else {
                    previous_elapsed = Some(current_elapsed);
                }
            }
        }
    }

    let mut summaries = Vec::with_capacity(cases.len());
    let mut mismatches = 0_u64;
    let mut incomplete = 0_u64;
    let mut insufficient = 0_u64;
    let mut missing = 0_u64;
    let mut critical_false_permits = 0_u64;
    let mut target_violations = 0_u64;

    for case in cases {
        let case_id = case["case_id"].as_str().unwrap_or("invalid");
        let expected = case["expected_probe_outcome"].as_str().unwrap_or("");
        let severity = case["severity"].as_str().unwrap_or("");
        let high_risk = case["high_risk"].as_bool().unwrap_or(false);

        let Some(observation) = observed_cases.get(case_id).copied() else {
            missing += 1;
            summaries.push(json!({
                "case_id": case_id,
                "expected": expected,
                "observed": null,
                "status": "missing",
                "high_risk": high_risk,
                "detail": "no observation was supplied"
            }));
            continue;
        };

        if reversed_clock_cases.contains(case_id) {
            insufficient += 1;
            summaries.push(json!({
                "case_id": case_id,
                "expected": expected,
                "observed": observation["observed_outcome"],
                "status": "insufficient_evidence",
                "high_risk": high_risk,
                "detail": "monotonic time moved backwards in submitted observation order"
            }));
            continue;
        }

        if let Some(problem) = evidence_problem(&receipt, observation, case) {
            insufficient += 1;
            summaries.push(json!({
                "case_id": case_id,
                "expected": expected,
                "observed": observation["observed_outcome"],
                "status": "insufficient_evidence",
                "high_risk": high_risk,
                "detail": problem
            }));
            continue;
        }

        let observed = observation["observed_outcome"].as_str().unwrap_or("");
        if matches!(observed, "inconclusive" | "not_run") {
            incomplete += 1;
            summaries.push(json!({
                "case_id": case_id,
                "expected": expected,
                "observed": observed,
                "status": "incomplete",
                "high_risk": high_risk,
                "detail": "outcome does not establish the expected behavior"
            }));
            continue;
        }

        if observed != expected {
            mismatches += 1;
            let false_permit = expected == "blocked" && observed == "permitted";
            if false_permit && (high_risk || severity == "critical") {
                critical_false_permits += 1;
            }
            summaries.push(json!({
                "case_id": case_id,
                "expected": expected,
                "observed": observed,
                "status": "mismatch",
                "high_risk": high_risk,
                "detail": if false_permit {
                    "unauthorized path was reported as permitted"
                } else {
                    "observed outcome did not match the corpus expectation"
                }
            }));
            continue;
        }

        if let (Some(target), Some(duration)) = (
            case["target_response_ms"].as_u64(),
            observation["duration_ms"].as_u64(),
        ) && duration > target
        {
            target_violations += 1;
            summaries.push(json!({
                "case_id": case_id,
                "expected": expected,
                "observed": observed,
                "status": "target_violation",
                "high_risk": high_risk,
                "detail": format!("duration {duration} ms exceeded target {target} ms")
            }));
            continue;
        }

        summaries.push(json!({
            "case_id": case_id,
            "expected": expected,
            "observed": observed,
            "status": "match",
            "high_risk": high_risk,
            "detail": "declared observation matches expected outcome"
        }));
    }

    let evidence_class = receipt["evidence_class"].as_str().unwrap_or("");
    let disposition = if critical_false_permits > 0 || mismatches > 0 {
        "nonconformant"
    } else if insufficient > 0 {
        "insufficient_evidence"
    } else if missing > 0 || incomplete > 0 {
        "incomplete"
    } else if target_violations > 0 {
        "target_violations"
    } else if evidence_class == "synthetic_fixture" {
        "synthetic_fixture_consistent_not_qualification"
    } else {
        "reported_observations_consistent_not_authenticated"
    };

    Ok(json!({
        "schema_version": "1.0.0",
        "run_id": receipt["run_id"],
        "evidence_class": evidence_class,
        "claim_ceiling": match evidence_class {
            "synthetic_fixture" => "synthetic_only_not_a_measurement",
            "namespace_lab_observation" => {
                "isolated_linux_namespace_observation_not_physical_or_authenticated"
            }
            _ => "submitted_lab_observation_claim_not_authenticated",
        },
        "corpus_sha256": receipt["corpus_sha256"],
        "approved_policy_sha256": receipt["approved_policy_sha256"],
        "effective_config_sha256": receipt["effective_config_sha256"],
        "disposition": disposition,
        "counts": {
            "cases_total": cases.len(),
            "observations_supplied": observations.len(),
            "matches": summaries.iter().filter(|item| item["status"] == "match").count(),
            "mismatches": mismatches,
            "critical_false_permits": critical_false_permits,
            "missing": missing,
            "incomplete": incomplete,
            "insufficient_evidence": insufficient,
            "target_violations": target_violations
        },
        "cases": summaries
    }))
}

#[cfg(test)]
mod tests {
    use super::{build_synthetic_receipt, reconcile};
    use serde_json::Value;

    const CORPUS_SCHEMA: &str =
        include_str!("../../../schemas/edge-policy-conformance-corpus-v1.schema.json");
    const CORPUS: &str =
        include_str!("../../../examples/edge-policy-conformance-corpus-synthetic-v1.json");
    const RECEIPT_SCHEMA: &str =
        include_str!("../../../schemas/edge-policy-observation-receipt-v1.schema.json");

    fn synthetic_receipt() -> Value {
        build_synthetic_receipt(CORPUS).expect("synthetic receipt")
    }

    fn reconcile_receipt(receipt: &Value) -> Result<Value, Vec<String>> {
        reconcile(CORPUS_SCHEMA, CORPUS, RECEIPT_SCHEMA, &receipt.to_string())
    }

    #[test]
    fn synthetic_smoke_is_consistent_but_never_qualifies_hardware() {
        let report = reconcile_receipt(&synthetic_receipt()).expect("report");
        assert_eq!(
            report["disposition"],
            "synthetic_fixture_consistent_not_qualification"
        );
        assert_eq!(report["counts"]["cases_total"], 13);
        assert_eq!(report["counts"]["matches"], 13);
        assert_eq!(report["counts"]["critical_false_permits"], 0);
        assert_eq!(report["claim_ceiling"], "synthetic_only_not_a_measurement");
    }

    #[test]
    fn exact_corpus_bytes_are_digest_bound() {
        let mut receipt = synthetic_receipt();
        receipt["corpus_sha256"] = Value::String("0".repeat(64));
        let error = reconcile_receipt(&receipt).expect_err("digest mismatch");
        assert!(error.iter().any(|item| item.contains("exact corpus bytes")));
    }

    #[test]
    fn unexpected_permit_on_critical_deny_is_nonconformant() {
        let mut receipt = synthetic_receipt();
        receipt["observations"][1]["observed_outcome"] = Value::String("permitted".to_owned());
        let report = reconcile_receipt(&receipt).expect("report");
        assert_eq!(report["disposition"], "nonconformant");
        assert_eq!(report["counts"]["critical_false_permits"], 1);
        assert_eq!(report["counts"]["mismatches"], 1);
    }

    #[test]
    fn unexpected_block_on_positive_control_is_nonconformant() {
        let mut receipt = synthetic_receipt();
        receipt["observations"][0]["observed_outcome"] = Value::String("blocked".to_owned());
        let report = reconcile_receipt(&receipt).expect("report");
        assert_eq!(report["disposition"], "nonconformant");
        assert_eq!(report["counts"]["mismatches"], 1);
    }

    #[test]
    fn omitted_case_is_incomplete_not_a_pass() {
        let mut receipt = synthetic_receipt();
        receipt["observations"].as_array_mut().expect("array").pop();
        let report = reconcile_receipt(&receipt).expect("report");
        assert_eq!(report["disposition"], "incomplete");
        assert_eq!(report["counts"]["missing"], 1);
    }

    #[test]
    fn duplicate_observation_for_a_case_is_rejected() {
        let mut receipt = synthetic_receipt();
        let mut observation = receipt["observations"][0].clone();
        observation["observation_id"] = Value::String("OBS-SYN-DUPLICATE-CASE".to_owned());
        receipt["observations"]
            .as_array_mut()
            .expect("array")
            .push(observation);
        let error = reconcile_receipt(&receipt).expect_err("duplicate case");
        assert!(
            error
                .iter()
                .any(|item| item.contains("duplicate observation for case ID"))
        );
    }

    #[test]
    fn unknown_case_is_rejected() {
        let mut receipt = synthetic_receipt();
        receipt["observations"][0]["case_id"] = Value::String("EPC-999".to_owned());
        let error = reconcile_receipt(&receipt).expect_err("unknown case");
        assert!(error.iter().any(|item| item.contains("unknown case ID")));
    }

    #[test]
    fn control_plane_acknowledgement_cannot_qualify_a_lab_observation() {
        let mut receipt = synthetic_receipt();
        receipt["evidence_class"] = Value::String("lab_observation_claim".to_owned());
        receipt["time_basis"]["kind"] = Value::String("monotonic_with_wall_anchor".to_owned());
        receipt["time_basis"]["wall_anchor_unix_ms"] = Value::from(1_800_000_000_000_u64);
        receipt["time_basis"]["monotonic_origin_ms"] = Value::from(1_000_u64);
        for (index, observation) in receipt["observations"]
            .as_array_mut()
            .expect("array")
            .iter_mut()
            .enumerate()
        {
            observation["evidence_source"] = Value::String("control_plane_ack".to_owned());
            observation["observed_at_unix_ms"] =
                Value::from(1_800_000_000_000_u64 + index as u64 * 1000);
            observation["monotonic_elapsed_ms"] = Value::from(1_000_u64 + index as u64 * 1000);
            observation["duration_ms"] = Value::from(10_u64);
            observation["evidence_sha256"] = Value::String("c".repeat(64));
        }
        let report = reconcile_receipt(&receipt).expect("report");
        assert_eq!(report["disposition"], "insufficient_evidence");
        assert_eq!(report["counts"]["insufficient_evidence"], 13);
    }

    #[test]
    fn lab_claim_requires_timestamp_digest_and_independent_probe() {
        let mut receipt = synthetic_receipt();
        receipt["evidence_class"] = Value::String("lab_observation_claim".to_owned());
        receipt["time_basis"]["kind"] = Value::String("monotonic_with_wall_anchor".to_owned());
        receipt["time_basis"]["wall_anchor_unix_ms"] = Value::from(1_800_000_000_000_u64);
        receipt["time_basis"]["monotonic_origin_ms"] = Value::from(1_000_u64);
        let first = receipt["observations"][0]
            .as_object_mut()
            .expect("observation");
        first.insert(
            "evidence_source".to_owned(),
            Value::String("independent_dataplane_probe".to_owned()),
        );
        first.insert(
            "observed_at_unix_ms".to_owned(),
            Value::from(1_800_000_000_000_u64),
        );
        first.insert("monotonic_elapsed_ms".to_owned(), Value::from(1_000_u64));
        first.insert("duration_ms".to_owned(), Value::from(10_u64));
        first.insert("evidence_sha256".to_owned(), Value::String("d".repeat(64)));
        let report = reconcile_receipt(&receipt).expect("report");
        assert_eq!(report["disposition"], "insufficient_evidence");
    }

    #[test]
    fn response_time_target_violations_are_reported_separately() {
        let mut receipt = synthetic_receipt();
        receipt["evidence_class"] = Value::String("lab_observation_claim".to_owned());
        receipt["time_basis"]["kind"] = Value::String("monotonic_with_wall_anchor".to_owned());
        receipt["time_basis"]["wall_anchor_unix_ms"] = Value::from(1_800_000_000_000_u64);
        receipt["time_basis"]["monotonic_origin_ms"] = Value::from(1_000_u64);
        for (index, observation) in receipt["observations"]
            .as_array_mut()
            .expect("array")
            .iter_mut()
            .enumerate()
        {
            observation["evidence_source"] =
                Value::String("independent_dataplane_probe".to_owned());
            observation["observed_at_unix_ms"] =
                Value::from(1_800_000_000_000_u64 + index as u64 * 1000);
            observation["monotonic_elapsed_ms"] = Value::from(1_000_u64 + index as u64 * 1000);
            observation["duration_ms"] = Value::from(10_u64);
            observation["evidence_sha256"] = Value::String("c".repeat(64));
        }
        receipt["observations"][2]["duration_ms"] = Value::from(6_000_u64);
        let report = reconcile_receipt(&receipt).expect("report");
        assert_eq!(report["disposition"], "target_violations");
        assert_eq!(report["counts"]["target_violations"], 1);
    }

    #[test]
    fn monotonic_clock_reversal_in_receipt_order_is_insufficient_evidence() {
        let mut receipt = synthetic_receipt();
        receipt["evidence_class"] = Value::String("namespace_lab_observation".to_owned());
        receipt["time_basis"]["kind"] = Value::String("monotonic_with_wall_anchor".to_owned());
        receipt["time_basis"]["wall_anchor_unix_ms"] = Value::from(1_800_000_000_000_u64);
        receipt["time_basis"]["monotonic_origin_ms"] = Value::from(1_000_u64);
        for (index, observation) in receipt["observations"]
            .as_array_mut()
            .expect("array")
            .iter_mut()
            .enumerate()
        {
            observation["evidence_source"] =
                Value::String("independent_dataplane_probe".to_owned());
            observation["observed_at_unix_ms"] =
                Value::from(1_800_000_000_000_u64 + index as u64 * 1000);
            observation["monotonic_elapsed_ms"] = Value::from(1_000_u64 + index as u64 * 1000);
            observation["duration_ms"] = Value::from(10_u64);
            observation["evidence_sha256"] = Value::String("e".repeat(64));
        }
        receipt["observations"]
            .as_array_mut()
            .expect("array")
            .swap(3, 4);
        let report = reconcile_receipt(&receipt).expect("report");
        assert_eq!(report["disposition"], "insufficient_evidence");
        assert_eq!(report["counts"]["insufficient_evidence"], 1);
    }

    #[test]
    fn remote_schema_references_are_rejected() {
        let mut bad_schema: Value = serde_json::from_str(RECEIPT_SCHEMA).expect("schema JSON");
        bad_schema["properties"]["schema_version"] =
            serde_json::json!({ "$ref": "https://example.invalid/remote.json" });
        let error = reconcile(
            CORPUS_SCHEMA,
            CORPUS,
            &bad_schema.to_string(),
            &synthetic_receipt().to_string(),
        )
        .expect_err("remote reference");
        assert!(
            error
                .iter()
                .any(|item| item.contains("only local JSON Schema references"))
        );
    }
}
