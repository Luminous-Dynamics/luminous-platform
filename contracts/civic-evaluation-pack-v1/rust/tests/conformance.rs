use civic_evaluation_pack_v1::validate_evaluation_pack_json;
use serde_json::Value;

const FIXTURE: &str = include_str!("../../examples/joburg-water-synthetic-v1.json");

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
fn incomplete_source_coverage_requires_abstention_and_human_review() {
    let mut value = fixture_value();
    let scenario = scenario_mut(&mut value, "scenario-baseline-report");
    scenario["input"]["source_coverage"] = serde_json::json!("incomplete");
    assert!(validate_evaluation_pack_json(&value.to_string()).is_err());
}
