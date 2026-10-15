use serde_json::Value;
use verified_operations_conformance::{classify_for_test, validate_corpus, validate_document, validate_schema};

const SCHEMA: &str = include_str!("../../../profiles/verified-operations/pilot-v1.schema.json");
const CORPUS: &str = include_str!("../../../profiles/verified-operations/synthetic-pilot-v1.json");

fn parsed_schema() -> Value {
    serde_json::from_str(SCHEMA).expect("schema JSON must parse")
}

fn parsed_corpus() -> Value {
    serde_json::from_str(CORPUS).expect("corpus JSON must parse")
}

fn scenario<'a>(corpus: &'a Value, id: &str) -> &'a Value {
    corpus["scenarios"]
        .as_array()
        .expect("scenarios array")
        .iter()
        .find(|item| item["id"].as_str() == Some(id))
        .unwrap_or_else(|| panic!("missing scenario {id}"))
}

fn scenario_mut<'a>(corpus: &'a mut Value, id: &str) -> &'a mut Value {
    corpus["scenarios"]
        .as_array_mut()
        .expect("scenarios array")
        .iter_mut()
        .find(|item| item["id"].as_str() == Some(id))
        .unwrap_or_else(|| panic!("missing scenario {id}"))
}

#[test]
fn schema_is_valid_draft_2020_12_and_formats_are_checked() {
    let schema = parsed_schema();
    validate_schema(&schema).expect("schema should compile as Draft 2020-12");
    let corpus = parsed_corpus();
    validate_document(&schema, &corpus).expect("valid corpus should match the schema");

    let mut invalid_time = corpus;
    invalid_time["evaluation_time"] = Value::String("not-a-date".into());
    assert!(
        validate_document(&schema, &invalid_time).is_err(),
        "date-time format assertion must be enabled"
    );
}

#[test]
fn all_eighteen_scenarios_match_the_rust_classifier() {
    let schema = parsed_schema();
    let corpus = parsed_corpus();
    let count = validate_corpus(&schema, &corpus).expect("corpus must conform");
    assert_eq!(count, 18);
}

#[test]
fn missing_stale_invalid_or_conflicting_observations_never_become_healthy() {
    let mut corpus = parsed_corpus();
    let cases = [
        ("fresh-in-scope-observation", "observation_fresh"),
        ("missing-required-source", "source_present"),
        ("contradictory-sources", "sources_agree"),
        ("invalid-source-payload", "source_valid"),
    ];

    for (id, fact) in cases {
        let changed = scenario_mut(&mut corpus, id);
        match fact {
            "observation_fresh" | "source_present" | "source_valid" => {
                changed["facts"][fact] = Value::Bool(false);
            }
            "sources_agree" => changed["facts"][fact] = Value::Bool(false),
            _ => unreachable!("case list is closed"),
        }
        let (classification, _) = classify_for_test(changed).expect("scenario classifies");
        assert_ne!(
            classification, "healthy_within_observed_scope",
            "mutation of {id}.{fact} must not leave a healthy classification"
        );
    }
}

#[test]
fn successful_backup_job_without_restore_evidence_is_not_recoverable_by_assumption() {
    let corpus = parsed_corpus();
    let item = scenario(&corpus, "backup-success-no-restore-evidence");
    let (classification, reason) = classify_for_test(item).expect("scenario classifies");
    assert_eq!(classification, "recovery_not_demonstrated");
    assert_eq!(reason, "RESTORE_NOT_DEMONSTRATED");
}

#[test]
fn failed_restore_test_is_actionable_even_when_backup_job_succeeded() {
    let corpus = parsed_corpus();
    let item = scenario(&corpus, "restore-test-failed");
    let (classification, reason) = classify_for_test(item).expect("scenario classifies");
    assert_eq!(classification, "actionable_finding");
    assert_eq!(reason, "RESTORE_TEST_FAILED");
}

#[test]
fn executor_assertion_is_not_independent_verification() {
    let mut corpus = parsed_corpus();
    let item = scenario_mut(&mut corpus, "executor-claim-is-not-verification");
    let (classification, reason) = classify_for_test(item).expect("scenario classifies");
    assert_eq!(classification, "effect_unknown");
    assert_eq!(reason, "INDEPENDENT_VERIFIER_MISSING");

    item["facts"]["postcondition"] = Value::String("independently_verified".into());
    let (promoted, promoted_reason) = classify_for_test(item).expect("scenario classifies");
    assert_eq!(promoted, "verified");
    assert_eq!(promoted_reason, "POSTCONDITION_VERIFIED");
}

#[test]
fn partial_effect_stays_partial_and_requires_reconciliation() {
    let mut corpus = parsed_corpus();
    let item = scenario_mut(&mut corpus, "partial-effect-requires-reconciliation");
    let (classification, _) = classify_for_test(item).expect("scenario classifies");
    assert_eq!(classification, "effect_partial");

    item["facts"]["postcondition"] = Value::String("missing".into());
    let (classification_after_missing_evidence, _) =
        classify_for_test(item).expect("scenario classifies");
    assert_eq!(classification_after_missing_evidence, "effect_partial");
}

#[test]
fn expired_authority_and_stale_prestate_do_not_dispatch_a_change() {
    let corpus = parsed_corpus();
    let (classification, reason) = classify_for_test(scenario(&corpus, "authorization-expired"))
        .expect("scenario classifies");
    assert_eq!(classification, "denied");
    assert_eq!(reason, "AUTHORIZATION_EXPIRED");

    let (classification, reason) = classify_for_test(scenario(&corpus, "prestate-became-stale"))
        .expect("scenario classifies");
    assert_eq!(classification, "requires_replan");
    assert_eq!(reason, "PRESTATE_STALE_REPLAN");
}

#[test]
fn conflicting_event_id_is_quarantined_and_identical_replay_is_deduplicated() {
    let corpus = parsed_corpus();
    let (classification, reason) =
        classify_for_test(scenario(&corpus, "duplicate-event-content-conflict"))
            .expect("scenario classifies");
    assert_eq!(classification, "quarantine_conflict");
    assert_eq!(reason, "EVENT_ID_CONTENT_CONFLICT");

    let (classification, reason) = classify_for_test(scenario(&corpus, "duplicate-identical-event"))
        .expect("scenario classifies");
    assert_eq!(classification, "deduplicated");
    assert_eq!(reason, "EVENT_DUPLICATE_IDENTICAL");
}

#[test]
fn semantic_gate_rejects_duplicate_ids_even_when_schema_allows_the_shape() {
    let schema = parsed_schema();
    let mut corpus = parsed_corpus();
    let duplicate = corpus["scenarios"][0].clone();
    corpus["scenarios"]
        .as_array_mut()
        .expect("scenarios")
        .push(duplicate);

    let errors = validate_corpus(&schema, &corpus).expect_err("duplicate IDs must fail");
    assert!(
        errors.iter().any(|error| error.contains("duplicates scenario id")),
        "expected duplicate-ID diagnostic, got {errors:#?}"
    );
}

#[test]
fn semantic_gate_rejects_a_safer_declared_classification_than_the_facts_support() {
    let schema = parsed_schema();
    let mut corpus = parsed_corpus();
    corpus["scenarios"][1]["expected"]["classification"] =
        Value::String("healthy_within_observed_scope".into());

    let errors = validate_corpus(&schema, &corpus).expect_err("false green must be rejected");
    assert!(
        errors.iter().any(|error| error.contains("classification mismatch")),
        "expected classifier mismatch, got {errors:#?}"
    );
}

#[test]
fn corpus_rejects_mutation_permission_or_provenance_invariant_breaks() {
    let schema = parsed_schema();
    let mut corpus = parsed_corpus();
    corpus["scenarios"][0]["expected"]["automatic_mutation_allowed"] = Value::Bool(true);

    let errors = validate_corpus(&schema, &corpus).expect_err("mutation permission must fail");
    assert!(
        errors.iter().any(|error| error.contains("automatic_mutation_allowed=false")),
        "expected mutation invariant diagnostic, got {errors:#?}"
    );

    let mut corpus = parsed_corpus();
    corpus["scenarios"][0]["expected"]["must_preserve_provenance"] = Value::Bool(false);
    let errors = validate_corpus(&schema, &corpus).expect_err("provenance invariant must fail");
    assert!(
        errors.iter().any(|error| error.contains("must_preserve_provenance=true")),
        "expected provenance invariant diagnostic, got {errors:#?}"
    );
}

#[test]
fn unsupported_contract_version_mode_and_extra_fields_are_rejected() {
    let schema = parsed_schema();

    let mut corpus = parsed_corpus();
    corpus["contract_version"] = Value::String("2.0.0".into());
    assert!(validate_corpus(&schema, &corpus).is_err());

    let mut corpus = parsed_corpus();
    corpus["mode"] = Value::String("live".into());
    assert!(validate_corpus(&schema, &corpus).is_err());

    let mut corpus = parsed_corpus();
    corpus["unrecognized"] = Value::Bool(true);
    assert!(validate_corpus(&schema, &corpus).is_err());
}
