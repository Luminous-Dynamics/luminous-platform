use jsonschema::validator_for;
use serde_json::{Value, json};

const SCORECARD_SCHEMA: &str =
    include_str!("../../../docs/assurance/infrastructure-assurance-scorecard-0.1.schema.json");
const SCORECARD_TEMPLATE: &str =
    include_str!("../fixtures/dependency-hypergraph-scorecard-template-0.1.json");
const REGISTRY: &str =
    include_str!("../../../docs/assurance/infrastructure-assurance-benchmark-registry-0.1.json");

fn schema_validator() -> jsonschema::Validator {
    let schema: Value = serde_json::from_str(SCORECARD_SCHEMA).expect("schema must be valid JSON");
    validator_for(&schema).expect("schema must compile as Draft 2020-12")
}

#[test]
fn not_run_template_is_valid_and_is_not_a_zero_score() {
    let scorecard: Value =
        serde_json::from_str(SCORECARD_TEMPLATE).expect("template must be valid JSON");
    assert_eq!(scorecard["run"]["status"], "not_run");
    assert!(schema_validator().is_valid(&scorecard));
}

#[test]
fn composite_trust_score_is_rejected() {
    let mut scorecard: Value =
        serde_json::from_str(SCORECARD_TEMPLATE).expect("template must be valid JSON");
    scorecard["overall_trust_score"] = json!(0.99);
    assert!(!schema_validator().is_valid(&scorecard));
}

#[test]
fn an_unmeasured_zero_cannot_be_relabelled_as_a_complete_run() {
    let mut scorecard: Value =
        serde_json::from_str(SCORECARD_TEMPLATE).expect("template must be valid JSON");
    scorecard["run"]["status"] = json!("complete");
    assert!(!schema_validator().is_valid(&scorecard));
}

#[test]
fn metric_denominator_is_required_when_numerator_is_present() {
    let mut scorecard: Value =
        serde_json::from_str(SCORECARD_TEMPLATE).expect("template must be valid JSON");
    scorecard["results"]["metric_families"] = json!({
        "dependency_assurance": [{
            "metric_id": "false_independent_rate",
            "value": 0.0,
            "unit": "fraction",
            "aggregation": "rate",
            "sample_count": 10,
            "population_id": "negative-controls-v0.1",
            "numerator": 0,
            "uncertainty": { "method": "exact-binomial" }
        }]
    });
    assert!(!schema_validator().is_valid(&scorecard));
}

#[test]
fn registry_track_ids_are_unique_and_have_explicit_claim_ceilings() {
    let registry: Value = serde_json::from_str(REGISTRY).expect("registry must be valid JSON");
    assert_eq!(registry["schema"], "luminous.infrastructure-assurance-registry/0.1");
    let tracks = registry["tracks"].as_array().expect("tracks array");
    assert!(tracks.len() >= 7);
    let mut ids = std::collections::BTreeSet::new();
    for track in tracks {
        let id = track["track_id"].as_str().expect("track id");
        assert!(ids.insert(id.to_owned()), "duplicate track id: {id}");
        assert!(
            track["claim_ceiling"].as_str().is_some_and(|s| !s.trim().is_empty()),
            "track {id} must declare a claim ceiling"
        );
        assert!(track["score_families"].is_array(), "track {id} must declare score families");
    }
    assert!(registry["external_comparators"].is_array());
    assert!(registry["standards_snapshot"].is_array());
}
