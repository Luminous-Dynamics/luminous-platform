use civic_evaluation_pack_v1::validate_evaluation_pack_json;
use serde_json::Value;

const FIXTURE: &str = include_str!("../../examples/joburg-water-synthetic-v1.json");
const SCHEMA: &str = include_str!("../../schema.json");

fn fixture_value() -> Value {
    serde_json::from_str(FIXTURE).expect("checked-in fixture is valid JSON")
}

fn scenario_mut<'a>(value: &'a mut Value, id: &str) -> &'a mut Value {
    value["scenarios"]
        .as_array_mut()
        .expect("scenarios array")
        .iter_mut()
        .find(|scenario| scenario["scenario_id"] == id)
        .unwrap_or_else(|| panic!("missing scenario {id}"))
}

#[test]
fn schema_is_parseable_versioned_and_strict() {
    let schema: Value = serde_json::from_str(SCHEMA).expect("schema must be valid JSON");
    assert_eq!(schema["$schema"], "https://json-schema.org/draft/2020-12/schema");
    assert_eq!(
        schema["$id"],
        "https://luminousdynamics.io/schemas/civic-evaluation-pack-v1.json"
    );
    assert_eq!(
        schema["properties"]["schema_version"]["const"],
        "civic-evaluation-pack-v1"
    );
    assert_eq!(schema["additionalProperties"], false);

    for name in [
        "sourcePolicy",
        "reproducibility",
        "scenario",
        "input",
        "observation",
        "expected",
    ] {
        assert_eq!(
            schema["$defs"][name]["additionalProperties"],
            false,
            "{name} must reject unknown fields"
        );
    }

    let mut classes = schema["$defs"]["scenario"]["properties"]["scenario_class"]["enum"]
        .as_array()
        .expect("scenario_class enum must be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    classes.sort_unstable();
    assert_eq!(
        classes,
        vec![
            "baseline",
            "conflicting_sources",
            "cross_utility_context_not_causality",
            "independent_verification_positive_control",
            "recurrence",
            "repair_claim_without_independent_verification",
            "stale_source",
        ]
    );
}

#[test]
fn accepts_frozen_synthetic_pack_and_all_negative_controls() {
    let pack = validate_evaluation_pack_json(FIXTURE).expect("fixture must validate");
    assert_eq!(pack.pack_id, "joburg-water-synthetic-v1");
    assert_eq!(pack.scenarios.len(), 7);
}

#[test]
fn rejects_live_ingestion_personal_data_and_claims_as_ground_truth() {
    for (field, invalid) in [
        ("live_ingestion_enabled", true),
        ("personal_data_included", true),
        ("published_claims_count_as_ground_truth", true),
    ] {
        let mut value = fixture_value();
        value["source_policy"][field] = serde_json::json!(invalid);
        assert!(
            validate_evaluation_pack_json(&value.to_string()).is_err(),
            "must reject {field}=true"
        );
    }
}

#[test]
fn stale_and_conflicting_evidence_require_human_review_and_abstention() {
    for id in ["scenario-stale-public-notice", "scenario-conflicting-notices"] {
        let mut value = fixture_value();
        scenario_mut(&mut value, id)["expected"]["must_abstain"] = serde_json::json!(false);
        assert!(validate_evaluation_pack_json(&value.to_string()).is_err());
    }
}

#[test]
fn repair_claim_alone_cannot_become_verified() {
    let mut value = fixture_value();
    let scenario = scenario_mut(&mut value, "scenario-repair-claim-is-not-verification");
    scenario["expected"]["outcome"] = serde_json::json!("independently_verified");
    scenario["expected"]["must_abstain"] = serde_json::json!(false);
    assert!(validate_evaluation_pack_json(&value.to_string()).is_err());
}

#[test]
fn cross_utility_publication_cannot_become_incident_causality() {
    let mut value = fixture_value();
    let scenario = scenario_mut(&mut value, "scenario-context-is-not-causality");
    scenario["expected"]["causal_claim_permitted"] = serde_json::json!(true);
    assert!(validate_evaluation_pack_json(&value.to_string()).is_err());
}

#[test]
fn a_repairer_cannot_independently_verify_their_own_claim() {
    let mut value = fixture_value();
    let scenario = scenario_mut(&mut value, "scenario-independent-verification-positive-control");
    scenario["input"]["observations"][1]["actor_ref"] =
        serde_json::json!("synthetic-repair-crew-02");
    assert!(validate_evaluation_pack_json(&value.to_string()).is_err());
}

#[test]
fn recurrence_after_verified_resolution_must_reopen() {
    let mut value = fixture_value();
    let scenario = scenario_mut(&mut value, "scenario-repeat-fault-reopens");
    scenario["expected"]["outcome"] = serde_json::json!("independently_verified");
    assert!(validate_evaluation_pack_json(&value.to_string()).is_err());
}

#[test]
fn event_cannot_be_recorded_before_it_occurs() {
    let mut value = fixture_value();
    let scenario = scenario_mut(&mut value, "scenario-baseline-report");
    scenario["input"]["observations"][0]["event_minute"] = serde_json::json!(20);
    scenario["input"]["observations"][0]["recorded_minute"] = serde_json::json!(5);
    assert!(validate_evaluation_pack_json(&value.to_string()).is_err());
}

#[test]
fn unknown_fields_and_impossible_dates_fail_closed() {
    let mut value = fixture_value();
    value["created_on"] = serde_json::json!("2026-02-30");
    assert!(validate_evaluation_pack_json(&value.to_string()).is_err());

    let mut value = fixture_value();
    value["unreviewed_extra"] = serde_json::json!(true);
    assert!(validate_evaluation_pack_json(&value.to_string()).is_err());
}

#[test]
fn causal_claims_are_outside_this_synthetic_contract() {
    let mut value = fixture_value();
    scenario_mut(&mut value, "scenario-baseline-report")["expected"]["causal_claim_permitted"] =
        serde_json::json!(true);
    assert!(validate_evaluation_pack_json(&value.to_string()).is_err());
}

#[test]
fn scenario_class_must_match_the_actual_evidence_state() {
    let mut value = fixture_value();
    let scenario = scenario_mut(&mut value, "scenario-stale-public-notice");
    scenario["input"]["observations"][0]["evidence_state"] = serde_json::json!("current");
    assert!(validate_evaluation_pack_json(&value.to_string()).is_err());
}

#[test]
fn uncertain_evidence_cannot_verify_a_repair() {
    let mut value = fixture_value();
    let scenario = scenario_mut(&mut value, "scenario-independent-verification-positive-control");
    scenario["input"]["source_coverage"] = serde_json::json!("incomplete");
    assert!(validate_evaluation_pack_json(&value.to_string()).is_err());
}

#[test]
fn pack_id_must_match_the_schema_pattern() {
    let mut value = fixture_value();
    value["pack_id"] = serde_json::json!("---");
    assert!(validate_evaluation_pack_json(&value.to_string()).is_err());
}

#[test]
fn incomplete_source_coverage_requires_abstention_and_human_review() {
    let mut value = fixture_value();
    let scenario = scenario_mut(&mut value, "scenario-baseline-report");
    scenario["input"]["source_coverage"] = serde_json::json!("incomplete");
    assert!(validate_evaluation_pack_json(&value.to_string()).is_err());
}
