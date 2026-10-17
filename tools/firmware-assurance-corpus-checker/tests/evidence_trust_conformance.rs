use firmware_assurance_corpus_checker::evidence_trust::{
    Decision, EvidenceTrustInput, EVIDENCE_FIELDS,
};
use jsonschema::{meta, validator_for};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const CORPUS_TEXT: &str =
    include_str!("../../../docs/fixtures/firmware-evidence-trust-corpus-v1.json");
const SCHEMA_TEXT: &str =
    include_str!("../../../docs/schemas/firmware-evidence-trust-corpus-v1.schema.json");

fn corpus() -> Value {
    serde_json::from_str(CORPUS_TEXT).expect("evidence trust corpus JSON parses")
}

#[test]
fn corpus_matches_strict_schema_and_all_expected_decisions() {
    let value = corpus();
    let schema: Value = serde_json::from_str(SCHEMA_TEXT).expect("schema JSON parses");
    meta::validate(&schema).expect("schema meta-validates");
    let validator = validator_for(&schema).expect("schema compiles");
    let errors = validator
        .iter_errors(&value)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "schema errors: {errors:#?}");

    let cases = value["cases"].as_array().expect("cases array exists");
    assert_eq!(cases.len(), 32);
    assert_eq!(value["case_count"].as_u64(), Some(32));
    let mut seen = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let id = case["case_id"].as_str().expect("case ID is a string");
        assert_eq!(id, format!("EV-{:03}", index + 1));
        assert!(seen.insert(id.to_string()), "duplicate case {id}");
        let input = EvidenceTrustInput::from_json(case).expect("all trust inputs present");
        assert_eq!(
            input.evaluate().as_str(),
            case["expected_decision"].as_str().unwrap(),
            "{id}"
        );
    }
}

#[test]
fn every_explicit_failure_rejects_and_every_unknown_stays_non_authorizing() {
    let valid = EvidenceTrustInput::from_json(&corpus()["cases"][20]).unwrap();
    // EV-021 is intentionally the all-positive baseline.
    assert_eq!(valid.evaluate(), Decision::Accepted);
    for field in EVIDENCE_FIELDS {
        let mut changed = corpus()["cases"][20].clone();
        let negative = match field {
            "signature_status" => "invalid",
            "signer_status" => "unauthorized",
            "subject_status" => "mismatch",
            "artifact_status" => "mismatch",
            "policy_status" => "mismatch",
            "revocation_status" => "revoked",
            "freshness_status" => "expired",
            "nonce_status" => "replay",
            "counter_status" => "rollback",
            "provenance_status" => "unauthenticated",
            _ => unreachable!("field list is exhaustive"),
        };
        changed[field] = json!(negative);
        let input = EvidenceTrustInput::from_json(&changed).unwrap();
        assert_eq!(input.evaluate(), Decision::Rejected, "{field} negative control");

        changed[field] = json!("unknown");
        let input = EvidenceTrustInput::from_json(&changed).unwrap();
        assert_eq!(input.evaluate(), Decision::Indeterminate, "{field} unknown control");
    }
}

#[test]
fn known_violation_dominates_unknown_and_missing_fields_are_errors() {
    let mut changed = corpus()["cases"][20].clone();
    changed["signature_status"] = json!("invalid");
    changed["revocation_status"] = json!("unknown");
    assert_eq!(
        EvidenceTrustInput::from_json(&changed).unwrap().evaluate(),
        Decision::Rejected
    );
    changed.as_object_mut().unwrap().remove("nonce_status");
    assert!(EvidenceTrustInput::from_json(&changed).is_err());
    changed["nonce_status"] = json!("unknown-sounding-but-invalid");
    assert!(EvidenceTrustInput::from_json(&changed).is_err());
}

#[test]
fn deterministic_ten_thousand_assignment_sweep_never_accepts_bad_or_unknown_evidence() {
    let base = corpus()["cases"][20].clone();
    let allowed: [&[&str]; 10] = [
        &["valid", "invalid", "unknown"],
        &["authorized", "unauthorized", "unknown"],
        &["match", "mismatch", "unknown"],
        &["match", "mismatch", "unknown"],
        &["match", "mismatch", "unknown"],
        &["clear", "revoked", "unknown"],
        &["fresh", "expired", "unknown"],
        &["current", "replay", "unknown"],
        &["monotonic", "rollback", "unknown"],
        &["authenticated", "unauthenticated", "unknown"],
    ];
    let mut seed = 0x4556_3031_u32;
    let mut distinct = BTreeSet::new();
    for trial in 0..10_000 {
        let mut changed = base.clone();
        for (index, field) in EVIDENCE_FIELDS.iter().enumerate() {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            changed[*field] = json!(allowed[index][((seed >> 16) as usize) % 3]);
        }
        let key = EVIDENCE_FIELDS
            .iter()
            .map(|field| changed[*field].as_str().unwrap())
            .collect::<Vec<_>>()
            .join("|");
        distinct.insert(key);
        let input = EvidenceTrustInput::from_json(&changed).unwrap();
        let decision = input.evaluate();
        let has_bad = EVIDENCE_FIELDS.iter().any(|field| {
            let actual = changed[*field].as_str().unwrap();
            match *field {
                "signature_status" => actual == "invalid",
                "signer_status" => actual == "unauthorized",
                "subject_status" => actual == "mismatch",
                "artifact_status" => actual == "mismatch",
                "policy_status" => actual == "mismatch",
                "revocation_status" => actual == "revoked",
                "freshness_status" => actual == "expired",
                "nonce_status" => actual == "replay",
                "counter_status" => actual == "rollback",
                "provenance_status" => actual == "unauthenticated",
                _ => false,
            }
        });
        let has_unknown = EVIDENCE_FIELDS.iter().any(|field| changed[*field] == "unknown");
        if has_bad {
            assert_eq!(decision, Decision::Rejected, "trial {trial}");
        } else if has_unknown {
            assert_eq!(decision, Decision::Indeterminate, "trial {trial}");
        } else {
            assert_eq!(decision, Decision::Accepted, "trial {trial}");
        }
    }
    assert!(distinct.len() > 9_000, "only {} distinct assignments", distinct.len());
}


#[test]
fn exhaustively_checks_the_complete_three_valued_state_space() {
    const EXPECTED_COMBINATIONS: usize = 59_049; // 3^10
    let base = corpus()["cases"][20].clone();
    let choices: [&[&str]; 10] = [
        &["valid", "invalid", "unknown"],
        &["authorized", "unauthorized", "unknown"],
        &["match", "mismatch", "unknown"],
        &["match", "mismatch", "unknown"],
        &["match", "mismatch", "unknown"],
        &["clear", "revoked", "unknown"],
        &["fresh", "expired", "unknown"],
        &["current", "replay", "unknown"],
        &["monotonic", "rollback", "unknown"],
        &["authenticated", "unauthenticated", "unknown"],
    ];
    let mut counts = [0_usize; 3]; // accepted, rejected, indeterminate
    let mut invariant_violations = 0_usize;

    // Base-3 decoding is a bijection over every possible input tuple. Unlike
    // random sampling, this establishes policy coverage for the full finite model.
    for encoded in 0..EXPECTED_COMBINATIONS {
        let mut ordinal = encoded;
        let mut changed = base.clone();
        for (index, field) in EVIDENCE_FIELDS.iter().enumerate() {
            changed[*field] = json!(choices[index][ordinal % 3][..]);
            ordinal /= 3;
        }

        let has_bad = EVIDENCE_FIELDS.iter().any(|field| {
            let actual = changed[*field].as_str().unwrap();
            match *field {
                "signature_status" => actual == "invalid",
                "signer_status" => actual == "unauthorized",
                "subject_status" => actual == "mismatch",
                "artifact_status" => actual == "mismatch",
                "policy_status" => actual == "mismatch",
                "revocation_status" => actual == "revoked",
                "freshness_status" => actual == "expired",
                "nonce_status" => actual == "replay",
                "counter_status" => actual == "rollback",
                "provenance_status" => actual == "unauthenticated",
                _ => false,
            }
        });
        let has_unknown = EVIDENCE_FIELDS
            .iter()
            .any(|field| changed[*field] == "unknown");
        let expected = if has_bad {
            Decision::Rejected
        } else if has_unknown {
            Decision::Indeterminate
        } else {
            Decision::Accepted
        };
        let actual = EvidenceTrustInput::from_json(&changed)
            .expect("exhaustive tuple uses only supported states")
            .evaluate();

        if actual != expected || (actual == Decision::Accepted && (has_bad || has_unknown)) {
            invariant_violations += 1;
        }
        assert_eq!(actual, expected, "state tuple {encoded}");
        match actual {
            Decision::Accepted => counts[0] += 1,
            Decision::Rejected => counts[1] += 1,
            Decision::Indeterminate => counts[2] += 1,
        }
    }

    assert_eq!(invariant_violations, 0);
    assert_eq!(counts, [1, 58_025, 1_023]);
    assert_eq!(counts.iter().sum::<usize>(), EXPECTED_COMBINATIONS);
}
