use serde_json::Value;
use verified_operations_conformance::{
    classify_for_test, validate_corpus, validate_document, validate_schema,
};

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
fn all_twenty_eight_scenarios_match_the_rust_classifier() {
    let schema = parsed_schema();
    let corpus = parsed_corpus();
    let count = validate_corpus(&schema, &corpus).expect("corpus must conform");
    assert_eq!(count, 28);
}

#[test]
fn one_fact_at_a_time_never_promotes_uncertainty_to_health() {
    let baseline = scenario(&parsed_corpus(), "fresh-in-scope-observation").clone();
    let mutations = [
        ("source_present", Value::Bool(false)),
        ("source_valid", Value::Bool(false)),
        ("observation_fresh", Value::Bool(false)),
        ("sources_agree", Value::Bool(false)),
    ];

    for (fact, mutated_value) in mutations {
        let mut changed = baseline.clone();
        changed["facts"][fact] = mutated_value;
        let (classification, _) =
            classify_for_test(&changed).expect("mutated scenario remains classifiable");
        assert_ne!(
            classification, "healthy_within_observed_scope",
            "changing facts.{fact} to a negative state must not preserve a healthy classification"
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
fn revoked_connector_credentials_are_denied() {
    let corpus = parsed_corpus();
    let (classification, reason) =
        classify_for_test(scenario(&corpus, "revoked-connector-credential"))
            .expect("scenario classifies");
    assert_eq!(classification, "denied");
    assert_eq!(reason, "CONNECTOR_CREDENTIAL_REVOKED");
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

    let (classification, reason) =
        classify_for_test(scenario(&corpus, "duplicate-identical-event"))
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
fn schema_rejects_mutation_permission_or_provenance_invariant_breaks() {
    let schema = parsed_schema();
    let mut corpus = parsed_corpus();
    corpus["scenarios"][0]["expected"]["automatic_mutation_allowed"] = Value::Bool(true);
    assert!(
        validate_corpus(&schema, &corpus).is_err(),
        "schema must reject mutation authority in synthetic corpus"
    );

    let mut corpus = parsed_corpus();
    corpus["scenarios"][0]["expected"]["must_preserve_provenance"] = Value::Bool(false);
    assert!(
        validate_corpus(&schema, &corpus).is_err(),
        "schema must reject disabling provenance preservation"
    );
}


#[test]
fn outcome_objective_target_met_and_missed_are_distinct() {
    let corpus = parsed_corpus();

    let (classification, reason) = classify_for_test(
        scenario(&corpus, "objective-slo-met-with-adequate-evidence"),
    )
    .expect("scenario classifies");
    assert_eq!(classification, "objective_met_within_observed_scope");
    assert_eq!(reason, "OBJECTIVE_TARGET_MET");

    let (classification, reason) = classify_for_test(
        scenario(&corpus, "objective-slo-missed-with-adequate-evidence"),
    )
    .expect("scenario classifies");
    assert_eq!(classification, "objective_target_missed");
    assert_eq!(reason, "OBJECTIVE_TARGET_MISSED");
}

#[test]
fn objective_claim_requires_complete_window_and_sufficient_coverage() {
    let corpus = parsed_corpus();
    for (id, reason) in [
        ("objective-coverage-insufficient", "OBJECTIVE_COVERAGE_INSUFFICIENT"),
        ("objective-window-incomplete", "OBJECTIVE_WINDOW_INCOMPLETE"),
    ] {
        let (classification, actual_reason) =
            classify_for_test(scenario(&corpus, id)).expect("scenario classifies");
        assert_eq!(classification, "indeterminate", "{id}");
        assert_eq!(actual_reason, reason, "{id}");
    }
}

#[test]
fn objective_claim_fails_closed_on_scope_units_or_missing_independent_measurement() {
    let corpus = parsed_corpus();
    for (id, reason) in [
        ("objective-scope-mismatch", "OBJECTIVE_SCOPE_MISMATCH"),
        ("objective-unit-mismatch", "OBJECTIVE_UNITS_INCOMPATIBLE"),
        (
            "objective-independent-measurement-missing",
            "OBJECTIVE_INDEPENDENT_MEASUREMENT_MISSING",
        ),
        ("objective-measurement-invalid", "OBJECTIVE_MEASUREMENT_INVALID"),
        ("objective-definition-missing", "OBJECTIVE_UNDEFINED"),
    ] {
        let (classification, actual_reason) =
            classify_for_test(scenario(&corpus, id)).expect("scenario classifies");
        assert_eq!(classification, "indeterminate", "{id}");
        assert_eq!(actual_reason, reason, "{id}");
    }
}

#[test]
fn objective_gap_limit_rejects_clumped_samples_despite_adequate_sample_ratio() {
    let corpus = parsed_corpus();
    let (classification, reason) =
        classify_for_test(scenario(&corpus, "objective-sampling-gap-too-long"))
            .expect("scenario classifies");
    assert_eq!(classification, "indeterminate");
    assert_eq!(reason, "OBJECTIVE_GAP_TOO_LONG");
}

#[test]
fn objective_coverage_is_derived_and_caller_claimed_ratio_is_rejected() {
    let schema = parsed_schema();
    let mut corpus = parsed_corpus();
    let item = scenario_mut(&mut corpus, "objective-coverage-insufficient");
    // Inject optimistic caller claims. The classifier ignores them and derives
    // coverage from valid sample offsets and the declared sampling schedule.
    item["facts"]["coverage_ratio"] = serde_json::json!(1.0);
    item["facts"]["window_complete"] = Value::Bool(true);

    let (classification, reason) =
        classify_for_test(item).expect("sample timeline still classifies");
    assert_eq!(classification, "indeterminate");
    assert_eq!(reason, "OBJECTIVE_COVERAGE_INSUFFICIENT");

    assert!(
        validate_corpus(&schema, &corpus).is_err(),
        "unknown self-declared coverage fields must be rejected by strict schema"
    );
}

#[test]
fn objective_window_and_sample_schedule_are_validated() {
    let corpus = parsed_corpus();
    let mut misaligned_window =
        scenario(&corpus, "objective-slo-met-with-adequate-evidence").clone();
    misaligned_window["facts"]["observation_window_end_epoch_seconds"] =
        serde_json::json!(1792025999);
    let (classification, reason) =
        classify_for_test(&misaligned_window).expect("invalid window classifies fail-closed");
    assert_eq!(classification, "indeterminate");
    assert_eq!(reason, "OBJECTIVE_WINDOW_POLICY_INVALID");

    let mut duplicate_sample =
        scenario(&corpus, "objective-slo-met-with-adequate-evidence").clone();
    duplicate_sample["facts"]["valid_sample_offsets_seconds"][1] =
        serde_json::json!(0);
    let (classification, reason) =
        classify_for_test(&duplicate_sample).expect("invalid timeline classifies fail-closed");
    assert_eq!(classification, "indeterminate");
    assert_eq!(reason, "OBJECTIVE_TIMELINE_INVALID");
}

#[test]
fn objective_comparator_supports_bounded_energy_usage_goals() {
    let corpus = parsed_corpus();
    let mut energy = scenario(&corpus, "objective-slo-met-with-adequate-evidence").clone();
    energy["facts"]["measured_unit"] = Value::String("kWh".into());
    energy["facts"]["target_unit"] = Value::String("kWh".into());
    energy["facts"]["comparator"] = Value::String("lte".into());
    energy["facts"]["measured_value"] = serde_json::json!(7.0);
    energy["facts"]["target_value"] = serde_json::json!(8.0);

    let (classification, reason) =
        classify_for_test(&energy).expect("energy scenario classifies");
    assert_eq!(classification, "objective_met_within_observed_scope");
    assert_eq!(reason, "OBJECTIVE_TARGET_MET");

    energy["facts"]["measured_value"] = serde_json::json!(9.0);
    let (classification, reason) =
        classify_for_test(&energy).expect("energy scenario classifies");
    assert_eq!(classification, "objective_target_missed");
    assert_eq!(reason, "OBJECTIVE_TARGET_MISSED");
}

#[test]
fn objective_metric_values_do_not_override_insufficient_evidence() {
    let corpus = parsed_corpus();
    let mut item = scenario(&corpus, "objective-coverage-insufficient").clone();
    item["facts"]["measured_value"] = serde_json::json!(100.0);
    item["facts"]["target_value"] = serde_json::json!(99.0);

    let (classification, reason) =
        classify_for_test(&item).expect("scenario classifies");
    assert_eq!(classification, "indeterminate");
    assert_eq!(reason, "OBJECTIVE_COVERAGE_INSUFFICIENT");
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


const EVIDENCE_SCHEMA: &str =
    include_str!("../../../profiles/verified-operations/evidence-bundle-v1.schema.json");
const EVIDENCE_BUNDLE: &str =
    include_str!("../../../profiles/verified-operations/synthetic-evidence-bundle-v1.json");

fn parsed_evidence_schema() -> Value {
    serde_json::from_str(EVIDENCE_SCHEMA).expect("evidence bundle schema JSON must parse")
}

fn parsed_evidence_bundle() -> Value {
    serde_json::from_str(EVIDENCE_BUNDLE).expect("synthetic evidence bundle JSON must parse")
}

#[test]
fn evidence_bundle_schema_and_fixture_are_valid() {
    let schema = parsed_evidence_schema();
    let bundle = parsed_evidence_bundle();
    validate_schema(&schema).expect("evidence bundle schema should compile as Draft 2020-12");
    assert_eq!(
        verified_operations_conformance::validate_evidence_bundle(&schema, &bundle),
        Ok(3),
        "all source payload digests, normalized measurement digests, lineage and decision semantics should agree"
    );

    let mut premature_production = bundle;
    premature_production["mode"] = Value::String("production".into());
    assert!(
        verified_operations_conformance::validate_evidence_bundle(&schema, &premature_production)
            .is_err(),
        "v1 must fail closed rather than imply production source authentication"
    );
}

#[test]
fn evidence_bundle_rejects_payload_tampering() {
    let schema = parsed_evidence_schema();
    let mut bundle = parsed_evidence_bundle();
    bundle["evidence"][0]["payload"]["raw_utf8"] =
        Value::String("{\"entity_id\":\"sensor.demo_energy\",\"state\":\"999.0\",\"unit_of_measurement\":\"kWh\"}".into());

    let errors = verified_operations_conformance::validate_evidence_bundle(&schema, &bundle)
        .expect_err("changed bytes without a changed digest must fail");
    assert!(
        errors.iter().any(|error| error.contains("raw payload SHA-256 mismatch")),
        "expected raw-payload digest failure, got {errors:#?}"
    );
}

#[test]
fn evidence_bundle_rejects_dangling_decision_refs_and_replayed_source_identity() {
    let schema = parsed_evidence_schema();
    let mut bundle = parsed_evidence_bundle();
    bundle["decision"]["evidence_refs"][0] = Value::String("missing-evidence".into());
    let errors = verified_operations_conformance::validate_evidence_bundle(&schema, &bundle)
        .expect_err("decision evidence links must resolve");
    assert!(
        errors.iter().any(|error| error.contains("decision references unknown evidence")),
        "expected unresolved decision evidence diagnostic, got {errors:#?}"
    );

    let mut bundle = parsed_evidence_bundle();
    bundle["evidence"][1]["source"]["source_event_id"] =
        Value::String("synthetic-event-start".into());
    let errors = verified_operations_conformance::validate_evidence_bundle(&schema, &bundle)
        .expect_err("a source event identity may not appear twice in a normalized receipt");
    assert!(
        errors.iter().any(|error| error.contains("repeated source event identity")),
        "expected replay diagnostic, got {errors:#?}"
    );
}

#[test]
fn evidence_bundle_rejects_self_consistent_measurement_that_disagrees_with_inputs() {
    let schema = parsed_evidence_schema();
    let mut bundle = parsed_evidence_bundle();
    bundle["evidence"][2]["measurement"]["value_decimal"] = Value::String("9.0".into());
    bundle["evidence"][2]["measurement"]["normalized_sha256"] =
        Value::String("3f345e27cf319b4971f613d1d90fdc7a0d7ab2dbcaf57b4a7d51e726404c61d7".into());
    bundle["evidence"][2]["payload"]["raw_utf8"] = Value::String(
        "{\"operation\":\"subtract\",\"start_evidence_id\":\"meter-start\",\"end_evidence_id\":\"meter-end\",\"derived_from\":[\"meter-start\",\"meter-end\"],\"metric_name\":\"energy.delta_window\",\"value_decimal\":\"9.0\",\"unit\":\"kWh\",\"validity\":\"valid\"}".into()
    );
    bundle["evidence"][2]["payload"]["sha256_of_raw_utf8"] =
        Value::String("b5cdf1febf0d30ce5c704eb0dcee9b7488ca13230226d376e36dd0971931573b".into());

    let errors = verified_operations_conformance::validate_evidence_bundle(&schema, &bundle)
        .expect_err("9 kWh is inconsistent with source readings of 100 and 107 kWh");
    assert!(
        errors.iter().any(|error| error.contains("does not equal end minus start")),
        "expected derived arithmetic diagnostic, got {errors:#?}"
    );
}

#[test]
fn evidence_bundle_rejects_a_validly_derived_measurement_that_misses_its_objective() {
    let schema = parsed_evidence_schema();
    let mut bundle = parsed_evidence_bundle();
    bundle["decision"]["objective"]["target_value_decimal"] = Value::String("6.0".into());

    let errors = verified_operations_conformance::validate_evidence_bundle(&schema, &bundle)
        .expect_err("7.0 kWh must not meet a 6.0 kWh maximum");
    assert!(
        errors.iter().any(|error| error.contains("decision classification does not match")),
        "expected false-green decision diagnostic, got {errors:#?}"
    );
}
