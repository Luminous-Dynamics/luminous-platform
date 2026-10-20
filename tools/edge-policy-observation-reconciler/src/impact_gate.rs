//! Counterfactual policy-impact gate for candidate changes.
//!
//! This module evaluates predictions from a trusted preflight adapter against
//! the exact frozen intent corpus. A prediction is not live evidence; promotion
//! still requires the separate freshness-bound independent-probe path.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};

const MAX_CASES: usize = 20_000;

/// Outcome predicted by a policy simulator or offline differential analyzer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredictedOutcome {
    Permitted,
    Blocked,
    Unknown,
}

impl PredictedOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Permitted => "permitted",
            Self::Blocked => "blocked",
            Self::Unknown => "unknown",
        }
    }

    fn from_expected(value: &str) -> Result<Self, String> {
        match value {
            "permitted" => Ok(Self::Permitted),
            "blocked" => Ok(Self::Blocked),
            _ => Err(format!("unsupported expected probe outcome {value:?}")),
        }
    }
}

/// Baseline and candidate predictions for one frozen corpus case.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredictedPolicyCase {
    pub case_id: String,
    pub baseline: PredictedOutcome,
    pub candidate: PredictedOutcome,
}

/// Output contract for an offline simulator, differential analyzer, or vendor
/// preflight adapter. The gate verifies identity, coverage and consistency; it
/// does not authenticate the simulator or claim these predictions were measured.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyPreflightVector {
    pub target_token: String,
    pub baseline_policy_sha256: String,
    pub candidate_policy_sha256: String,
    pub corpus_sha256: String,
    pub simulator_id: String,
    pub simulator_version: String,
    pub decisions: Vec<PredictedPolicyCase>,
}

/// A joined per-case impact record, preserving original corpus order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseImpact {
    pub case_id: String,
    pub severity: String,
    pub high_risk: bool,
    pub expected: PredictedOutcome,
    pub baseline: PredictedOutcome,
    pub candidate: PredictedOutcome,
    pub baseline_mismatch: bool,
    pub candidate_mismatch: bool,
    pub newly_permitted: bool,
    pub newly_blocked: bool,
}

/// Deterministic counterfactual report. Its digest covers metadata and every
/// joined case result, not merely the headline counts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyImpactReport {
    pub target_token: String,
    pub baseline_policy_sha256: String,
    pub candidate_policy_sha256: String,
    pub corpus_sha256: String,
    pub simulator_id: String,
    pub simulator_version: String,
    pub cases_evaluated: usize,
    pub baseline_mismatch_case_ids: Vec<String>,
    pub candidate_mismatch_case_ids: Vec<String>,
    pub high_risk_false_permit_case_ids: Vec<String>,
    pub unknown_case_ids: Vec<String>,
    pub newly_permitted_case_ids: Vec<String>,
    pub newly_blocked_case_ids: Vec<String>,
    pub promotable: bool,
    pub report_sha256: String,
    pub cases: Vec<CaseImpact>,
}

/// Reconcile a simulator vector to an exact corpus, target, baseline and candidate.
///
/// Candidate predictions must match every expected corpus outcome. Unknown
/// predictions fail closed. Baseline mismatches are reported but can be repaired
/// by the candidate; the candidate, not the baseline, must satisfy the intended
/// decisions. Changes to intent require a new versioned/approved corpus.
pub fn evaluate_policy_impact(
    corpus_schema_source: &str,
    corpus_source: &str,
    expected_target_token: &str,
    expected_baseline_policy_sha256: &str,
    expected_candidate_policy_sha256: &str,
    vector: &PolicyPreflightVector,
) -> Result<PolicyImpactReport, Vec<String>> {
    let corpus = super::validate_document(
        corpus_schema_source,
        corpus_source,
        "policy-impact corpus",
    )?;
    let cases = corpus
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| vec!["policy-impact corpus.cases must be an array".to_owned()])?;
    if cases.is_empty() || cases.len() > MAX_CASES {
        return Err(vec![format!(
            "policy-impact corpus must contain 1..={MAX_CASES} cases"
        )]);
    }

    validate_target_token(expected_target_token).map_err(|error| vec![error])?;
    validate_digest(expected_baseline_policy_sha256, "expected baseline digest")
        .map_err(|error| vec![error])?;
    validate_digest(expected_candidate_policy_sha256, "expected candidate digest")
        .map_err(|error| vec![error])?;
    if expected_baseline_policy_sha256 == expected_candidate_policy_sha256 {
        return Err(vec![
            "candidate policy digest must differ from the baseline".to_owned(),
        ]);
    }
    if vector.target_token != expected_target_token {
        return Err(vec![
            "preflight vector target token does not match the target being changed".to_owned(),
        ]);
    }
    if vector.baseline_policy_sha256 != expected_baseline_policy_sha256
        || vector.candidate_policy_sha256 != expected_candidate_policy_sha256
    {
        return Err(vec![
            "preflight vector baseline/candidate digest binding does not match the requested change"
                .to_owned(),
        ]);
    }
    let actual_corpus_sha256 = super::sha256_hex(corpus_source.as_bytes());
    if vector.corpus_sha256 != actual_corpus_sha256 {
        return Err(vec![
            "preflight vector corpus digest does not match the exact corpus bytes".to_owned(),
        ]);
    }
    validate_identifier(&vector.simulator_id, "simulator_id")
        .map_err(|error| vec![error])?;
    validate_identifier(&vector.simulator_version, "simulator_version")
        .map_err(|error| vec![error])?;

    if vector.decisions.len() != cases.len() {
        return Err(vec![format!(
            "preflight vector covers {} cases but the frozen corpus contains {}",
            vector.decisions.len(),
            cases.len()
        )]);
    }

    let mut decision_by_id = HashMap::with_capacity(vector.decisions.len());
    for decision in &vector.decisions {
        validate_case_id(&decision.case_id).map_err(|error| vec![error])?;
        if decision_by_id
            .insert(decision.case_id.as_str(), decision)
            .is_some()
        {
            return Err(vec![format!(
                "preflight vector contains duplicate case ID {:?}",
                decision.case_id
            )]);
        }
    }

    let expected_ids: HashSet<&str> = cases
        .iter()
        .filter_map(|case| case.get("case_id").and_then(Value::as_str))
        .collect();
    if expected_ids.len() != cases.len() {
        return Err(vec![
            "frozen corpus contains duplicate or missing case IDs".to_owned(),
        ]);
    }
    for case_id in decision_by_id.keys() {
        if !expected_ids.contains(case_id) {
            return Err(vec![format!(
                "preflight vector contains unknown case ID {case_id:?}"
            )]);
        }
    }
    for case_id in &expected_ids {
        if !decision_by_id.contains_key(case_id) {
            return Err(vec![format!(
                "preflight vector omits required case ID {case_id:?}"
            )]);
        }
    }

    let mut cases_report = Vec::with_capacity(cases.len());
    let mut baseline_mismatch_case_ids = Vec::new();
    let mut candidate_mismatch_case_ids = Vec::new();
    let mut high_risk_false_permit_case_ids = Vec::new();
    let mut unknown_case_ids = Vec::new();
    let mut newly_permitted_case_ids = Vec::new();
    let mut newly_blocked_case_ids = Vec::new();

    for corpus_case in cases {
        let case_id = corpus_case["case_id"]
            .as_str()
            .ok_or_else(|| vec!["corpus case is missing case_id".to_owned()])?;
        let expected_str = corpus_case["expected_probe_outcome"]
            .as_str()
            .ok_or_else(|| vec![format!("corpus case {case_id:?} is missing expected outcome")])?;
        let expected = PredictedOutcome::from_expected(expected_str)
            .map_err(|error| vec![format!("corpus case {case_id:?}: {error}")])?;
        let severity = corpus_case["severity"]
            .as_str()
            .ok_or_else(|| vec![format!("corpus case {case_id:?} is missing severity")])?
            .to_owned();
        let high_risk = corpus_case["high_risk"]
            .as_bool()
            .ok_or_else(|| vec![format!("corpus case {case_id:?} is missing high_risk")])?;
        let predicted = decision_by_id
            .get(case_id)
            .copied()
            .ok_or_else(|| vec![format!("preflight omitted case {case_id:?}")])?;

        let baseline_mismatch = predicted.baseline != expected;
        let candidate_mismatch = predicted.candidate != expected;
        let newly_permitted = predicted.baseline == PredictedOutcome::Blocked
            && predicted.candidate == PredictedOutcome::Permitted;
        let newly_blocked = predicted.baseline == PredictedOutcome::Permitted
            && predicted.candidate == PredictedOutcome::Blocked;
        let high_risk_false_permit = high_risk
            && expected == PredictedOutcome::Blocked
            && predicted.candidate == PredictedOutcome::Permitted;
        let unknown = predicted.baseline == PredictedOutcome::Unknown
            || predicted.candidate == PredictedOutcome::Unknown;

        if baseline_mismatch {
            baseline_mismatch_case_ids.push(case_id.to_owned());
        }
        if candidate_mismatch {
            candidate_mismatch_case_ids.push(case_id.to_owned());
        }
        if high_risk_false_permit {
            high_risk_false_permit_case_ids.push(case_id.to_owned());
        }
        if unknown {
            unknown_case_ids.push(case_id.to_owned());
        }
        if newly_permitted {
            newly_permitted_case_ids.push(case_id.to_owned());
        }
        if newly_blocked {
            newly_blocked_case_ids.push(case_id.to_owned());
        }

        cases_report.push(CaseImpact {
            case_id: case_id.to_owned(),
            severity,
            high_risk,
            expected,
            baseline: predicted.baseline,
            candidate: predicted.candidate,
            baseline_mismatch,
            candidate_mismatch,
            newly_permitted,
            newly_blocked,
        });
    }

    let promotable = candidate_mismatch_case_ids.is_empty() && unknown_case_ids.is_empty();
    let report_payload = json!({
        "schema_version": "1.0.0",
        "target_token": expected_target_token,
        "baseline_policy_sha256": expected_baseline_policy_sha256,
        "candidate_policy_sha256": expected_candidate_policy_sha256,
        "corpus_sha256": actual_corpus_sha256,
        "simulator_id": vector.simulator_id,
        "simulator_version": vector.simulator_version,
        "cases_evaluated": cases_report.len(),
        "baseline_mismatch_case_ids": &baseline_mismatch_case_ids,
        "candidate_mismatch_case_ids": &candidate_mismatch_case_ids,
        "high_risk_false_permit_case_ids": &high_risk_false_permit_case_ids,
        "unknown_case_ids": &unknown_case_ids,
        "newly_permitted_case_ids": &newly_permitted_case_ids,
        "newly_blocked_case_ids": &newly_blocked_case_ids,
        "promotable": promotable,
        "cases": cases_report.iter().map(case_impact_json).collect::<Vec<_>>()
    });
    let report_bytes = serde_json::to_vec(&report_payload)
        .map_err(|error| vec![format!("cannot serialize policy-impact report: {error}")])?;
    let report_sha256 = super::sha256_hex(&report_bytes);

    Ok(PolicyImpactReport {
        target_token: expected_target_token.to_owned(),
        baseline_policy_sha256: expected_baseline_policy_sha256.to_owned(),
        candidate_policy_sha256: expected_candidate_policy_sha256.to_owned(),
        corpus_sha256: actual_corpus_sha256,
        simulator_id: vector.simulator_id.clone(),
        simulator_version: vector.simulator_version.clone(),
        cases_evaluated: cases_report.len(),
        baseline_mismatch_case_ids,
        candidate_mismatch_case_ids,
        high_risk_false_permit_case_ids,
        unknown_case_ids,
        newly_permitted_case_ids,
        newly_blocked_case_ids,
        promotable,
        report_sha256,
        cases: cases_report,
    })
}

fn case_impact_json(case: &CaseImpact) -> Value {
    json!({
        "case_id": case.case_id,
        "severity": case.severity,
        "high_risk": case.high_risk,
        "expected": case.expected.as_str(),
        "baseline": case.baseline.as_str(),
        "candidate": case.candidate.as_str(),
        "baseline_mismatch": case.baseline_mismatch,
        "candidate_mismatch": case.candidate_mismatch,
        "newly_permitted": case.newly_permitted,
        "newly_blocked": case.newly_blocked
    })
}

fn validate_case_id(value: &str) -> Result<(), String> {
    if value.len() < 7
        || value.len() > 9
        || !value.starts_with("EPC-")
        || !value[4..].bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(format!("invalid policy-impact case ID {value:?}"));
    }
    Ok(())
}

fn validate_identifier(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 80
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(format!(
            "{label} must be a non-empty ASCII identifier of at most 80 characters"
        ));
    }
    Ok(())
}

fn validate_target_token(value: &str) -> Result<(), String> {
    if !value.starts_with("target-")
        || value.len() < 8
        || value.len() > 55
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err("target token is outside the supported target- identifier grammar".to_owned());
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{label} must be a 64-character lowercase SHA-256 digest"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        PolicyPreflightVector, PredictedOutcome, PredictedPolicyCase, evaluate_policy_impact,
    };
    use serde_json::Value;

    const CORPUS_SCHEMA: &str =
        include_str!("../../../schemas/edge-policy-conformance-corpus-v1.schema.json");
    const CORPUS: &str =
        include_str!("../../../examples/edge-policy-conformance-corpus-synthetic-v1.json");
    const TARGET: &str = "target-impact-test";
    const BASELINE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const CANDIDATE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn vector() -> PolicyPreflightVector {
        let corpus: Value = serde_json::from_str(CORPUS).expect("corpus JSON");
        let decisions = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .map(|case| {
                let expected = match case["expected_probe_outcome"].as_str().expect("outcome") {
                    "permitted" => PredictedOutcome::Permitted,
                    "blocked" => PredictedOutcome::Blocked,
                    unexpected => panic!("unexpected expected outcome {unexpected:?}"),
                };
                PredictedPolicyCase {
                    case_id: case["case_id"].as_str().expect("case ID").to_owned(),
                    baseline: expected,
                    candidate: expected,
                }
            })
            .collect();
        PolicyPreflightVector {
            target_token: TARGET.to_owned(),
            baseline_policy_sha256: BASELINE.to_owned(),
            candidate_policy_sha256: CANDIDATE.to_owned(),
            corpus_sha256: crate::sha256_hex(CORPUS.as_bytes()),
            simulator_id: "batfish-diff-adapter".to_owned(),
            simulator_version: "1.0.0".to_owned(),
            decisions,
        }
    }

    fn evaluate(vector: &PolicyPreflightVector) -> super::PolicyImpactReport {
        evaluate_policy_impact(
            CORPUS_SCHEMA,
            CORPUS,
            TARGET,
            BASELINE,
            CANDIDATE,
            vector,
        )
        .expect("impact report")
    }

    #[test]
    fn conformant_candidate_is_promotable_and_report_digest_is_deterministic() {
        let vector = vector();
        let first = evaluate(&vector);
        let second = evaluate(&vector);
        assert!(first.promotable);
        assert!(first.candidate_mismatch_case_ids.is_empty());
        assert_eq!(first.cases_evaluated, vector.decisions.len());
        assert_eq!(first.report_sha256, second.report_sha256);
    }

    #[test]
    fn candidate_high_risk_false_permit_blocks_preflight() {
        let mut vector = vector();
        let corpus: Value = serde_json::from_str(CORPUS).expect("corpus JSON");
        let case = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|case| {
                case["high_risk"].as_bool() == Some(true)
                    && case["expected_probe_outcome"] == "blocked"
            })
            .expect("corpus includes a high-risk deny case");
        let id = case["case_id"].as_str().expect("case ID");
        let decision = vector
            .decisions
            .iter_mut()
            .find(|decision| decision.case_id == id)
            .expect("matching decision");
        decision.candidate = PredictedOutcome::Permitted;

        let report = evaluate(&vector);
        assert!(!report.promotable);
        assert_eq!(report.high_risk_false_permit_case_ids, vec![id]);
        assert!(report.candidate_mismatch_case_ids.contains(&id.to_owned()));
    }

    #[test]
    fn candidate_unexpected_block_is_reported_as_a_service_regression() {
        let mut vector = vector();
        let corpus: Value = serde_json::from_str(CORPUS).expect("corpus JSON");
        let case = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|case| case["expected_probe_outcome"] == "permitted")
            .expect("corpus includes a permitted service case");
        let id = case["case_id"].as_str().expect("case ID");
        let decision = vector
            .decisions
            .iter_mut()
            .find(|decision| decision.case_id == id)
            .expect("matching decision");
        decision.candidate = PredictedOutcome::Blocked;

        let report = evaluate(&vector);
        assert!(!report.promotable);
        assert!(report.candidate_mismatch_case_ids.contains(&id.to_owned()));
        assert!(report.newly_blocked_case_ids.contains(&id.to_owned()));
    }

    #[test]
    fn existing_baseline_mismatch_is_reported_when_candidate_repairs_it() {
        let mut vector = vector();
        let corpus: Value = serde_json::from_str(CORPUS).expect("corpus JSON");
        let case = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|case| case["expected_probe_outcome"] == "blocked")
            .expect("corpus includes a deny case");
        let id = case["case_id"].as_str().expect("case ID");
        let decision = vector
            .decisions
            .iter_mut()
            .find(|decision| decision.case_id == id)
            .expect("matching decision");
        decision.baseline = PredictedOutcome::Permitted;

        let report = evaluate(&vector);
        assert!(report.promotable);
        assert!(report.baseline_mismatch_case_ids.contains(&id.to_owned()));
        assert!(report.candidate_mismatch_case_ids.is_empty());
    }

    #[test]
    fn unknown_candidate_prediction_fails_closed() {
        let mut vector = vector();
        vector.decisions[0].candidate = PredictedOutcome::Unknown;
        let report = evaluate(&vector);
        assert!(!report.promotable);
        assert!(report.unknown_case_ids.contains(&vector.decisions[0].case_id));
    }

    #[test]
    fn rejects_wrong_target_digest_and_corpus_bindings() {
        let mut vector = vector();
        vector.target_token = "target-another-device".to_owned();
        assert!(evaluate_policy_impact(
            CORPUS_SCHEMA,
            CORPUS,
            TARGET,
            BASELINE,
            CANDIDATE,
            &vector,
        )
        .expect_err("wrong target rejected")
        .iter()
        .any(|error| error.contains("target token")));

        let mut vector = vector();
        vector.corpus_sha256 = "e".repeat(64);
        assert!(evaluate_policy_impact(
            CORPUS_SCHEMA,
            CORPUS,
            TARGET,
            BASELINE,
            CANDIDATE,
            &vector,
        )
        .expect_err("wrong corpus rejected")
        .iter()
        .any(|error| error.contains("corpus digest")));
    }

    #[test]
    fn duplicate_missing_and_unknown_case_ids_are_rejected() {
        let mut vector = vector();
        vector.decisions[1].case_id = vector.decisions[0].case_id.clone();
        assert!(evaluate_policy_impact(
            CORPUS_SCHEMA,
            CORPUS,
            TARGET,
            BASELINE,
            CANDIDATE,
            &vector,
        )
        .is_err());

        let mut vector = vector();
        vector.decisions.pop();
        assert!(evaluate_policy_impact(
            CORPUS_SCHEMA,
            CORPUS,
            TARGET,
            BASELINE,
            CANDIDATE,
            &vector,
        )
        .is_err());

        let mut vector = vector();
        vector.decisions[0].case_id = "EPC-99999".to_owned();
        assert!(evaluate_policy_impact(
            CORPUS_SCHEMA,
            CORPUS,
            TARGET,
            BASELINE,
            CANDIDATE,
            &vector,
        )
        .is_err());
    }
}
