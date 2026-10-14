use public_source_catalogue_v1::{validate_catalogue_json, validate_observation_json};
use serde_json::{json, Value};

const CATALOGUE: &str = include_str!("../../examples/catalogue.json");
const OBSERVATION: &str = include_str!("../../examples/observation-v1.json");

fn parse_catalogue_fixture() -> Value {
    serde_json::from_str(CATALOGUE).expect("catalogue fixture must be valid JSON")
}

fn parse_observation_fixture() -> Value {
    serde_json::from_str(OBSERVATION).expect("observation fixture must be valid JSON")
}

fn encoded(value: &Value) -> String {
    serde_json::to_string(value).expect("test JSON must serialize")
}

#[test]
fn canonical_source_catalogue_fixture_is_accepted() {
    let catalogue = validate_catalogue_json(CATALOGUE).expect("catalogue should validate");
    assert_eq!(catalogue.schema_version, "public-source-catalogue-v1");
    assert_eq!(catalogue.sources.len(), 10);
    assert!(catalogue.sources.iter().all(|source| !source.ingestion.enabled));

    for expected_id in [
        "za-treasury-municipal-finance",
        "za-treasury-etender-ocds",
        "za-gp-jhb-johannesburg-water-public",
        "za-gp-jhb-city-plans-service-standards",
        "za-gp-jhb-city-power-public-notices",
        "za-gp-jhb-jra-operation-restore",
        "za-gp-jhb-pikitup-public-service-notices",
        "za-gp-jhb-transport-masterplan-updates",
        "za-gp-jhb-food-resilience-policy",
        "za-gov-jhb-power-water-coordination",
    ] {
        assert!(
            catalogue
                .sources
                .iter()
                .any(|source| source.source_id.as_str() == expected_id),
            "missing catalogued source {expected_id}"
        );
    }
}

#[test]
fn catalogue_rejects_unknown_root_fields() {
    let mut value = parse_catalogue_fixture();
    value["unreviewed_extension"] = json!(true);
    assert!(validate_catalogue_json(&encoded(&value)).is_err());
}

#[test]
fn catalogue_rejects_unknown_nested_fields() {
    let mut value = parse_catalogue_fixture();
    value["sources"][0]["publisher"]["authority"] = json!("invented");
    assert!(validate_catalogue_json(&encoded(&value)).is_err());
}

#[test]
fn catalogue_rejects_duplicate_source_ids() {
    let mut value = parse_catalogue_fixture();
    let first_source_id = value["sources"][0]["source_id"].clone();
    value["sources"][1]["source_id"] = first_source_id;
    let error = validate_catalogue_json(&encoded(&value)).expect_err("duplicate IDs must fail");
    assert!(error.to_string().contains("duplicate source_id"));
}

#[test]
fn catalogue_rejects_http_source_urls() {
    let mut value = parse_catalogue_fixture();
    value["sources"][0]["source_url"] = json!("http://municipaldata.treasury.gov.za/api");
    let error = validate_catalogue_json(&encoded(&value)).expect_err("HTTP must fail");
    assert!(error.to_string().contains("absolute HTTPS URL"));
}

#[test]
fn catalogue_rejects_malformed_https_url_and_userinfo() {
    for url in [
        "https://",
        "https://user:password@example.gov.za/private",
        "https:// not-a-host.example/",
    ] {
        let mut value = parse_catalogue_fixture();
        value["sources"][0]["source_url"] = json!(url);
        assert!(validate_catalogue_json(&encoded(&value)).is_err(), "accepted {url}");
    }
}

#[test]
fn catalogue_rejects_enabled_ingestion() {
    let mut value = parse_catalogue_fixture();
    value["sources"][0]["ingestion"]["enabled"] = json!(true);
    let error = validate_catalogue_json(&encoded(&value)).expect_err("ingestion must stay disabled");
    assert!(error.to_string().contains("must remain false"));
}

#[test]
fn catalogue_rejects_empty_catalogue_and_bad_review_date() {
    let mut empty = parse_catalogue_fixture();
    empty["sources"] = json!([]);
    assert!(validate_catalogue_json(&encoded(&empty)).is_err());

    let mut bad_date = parse_catalogue_fixture();
    bad_date["reviewed_on"] = json!("2026-02-30");
    assert!(validate_catalogue_json(&encoded(&bad_date)).is_err());
}

#[test]
fn catalogue_rejects_duplicate_access_methods() {
    let mut value = parse_catalogue_fixture();
    value["sources"][0]["access"]["documented_methods"] = json!(["GET", "GET"]);
    let error = validate_catalogue_json(&encoded(&value)).expect_err("duplicate methods must fail");
    assert!(error.to_string().contains("duplicate method"));
}

#[test]
fn canonical_synthetic_observation_is_accepted() {
    let observation =
        validate_observation_json(OBSERVATION).expect("synthetic observation should validate");
    assert!(observation.fixture_only);
    assert_eq!(observation.source_id, "za-treasury-municipal-finance");
}

#[test]
fn observation_rejects_unknown_fields_and_malformed_json() {
    assert!(validate_observation_json("{not json").is_err());

    let mut value = parse_observation_fixture();
    value["surprise"] = json!("ignore me");
    assert!(validate_observation_json(&encoded(&value)).is_err());
}

#[test]
fn observation_rejects_http_and_timezone_free_timestamps() {
    let mut bad_url = parse_observation_fixture();
    bad_url["source_uri"] = json!("http://example.org/data");
    assert!(validate_observation_json(&encoded(&bad_url)).is_err());

    let mut no_timezone = parse_observation_fixture();
    no_timezone["retrieved_at"] = json!("2026-10-12T12:00:00");
    assert!(validate_observation_json(&encoded(&no_timezone)).is_err());
}

#[test]
fn raw_zero_null_and_omitted_row_are_distinct_states() {
    let mut zero = parse_observation_fixture();
    zero["raw_value"] = json!(0);
    zero["raw_value_state"] = json!("reported_zero");
    zero["normalization"]["normalized_value"] = json!(0);
    assert!(validate_observation_json(&encoded(&zero)).is_ok());

    let mut null = parse_observation_fixture();
    null["raw_value"] = Value::Null;
    null["raw_value_state"] = json!("reported_null");
    null["normalization"]["normalized_value"] = Value::Null;
    assert!(validate_observation_json(&encoded(&null)).is_ok());

    let mut omitted = parse_observation_fixture();
    omitted["raw_value"] = Value::Null;
    omitted["raw_value_state"] = json!("row_omitted");
    omitted["normalization"]["normalized_value"] = Value::Null;
    assert!(validate_observation_json(&encoded(&omitted)).is_ok());

    assert_ne!(zero["raw_value_state"], null["raw_value_state"]);
    assert_ne!(null["raw_value_state"], omitted["raw_value_state"]);
}

#[test]
fn source_specific_null_to_zero_normalization_keeps_raw_null() {
    // Treasury's guide says a returned null may be treated as zero. Preserve
    // that raw null, and record the explicit source-specific rule and version.
    let mut value = parse_observation_fixture();
    value["raw_value"] = Value::Null;
    value["raw_value_state"] = json!("reported_null");
    value["normalization"] = json!({
        "applied": true,
        "rule_id": "treasury-null-as-zero",
        "rule_version": "1",
        "normalized_value": 0,
        "reason": "Source-specific Treasury guidance; raw null retained for audit."
    });
    let observation = validate_observation_json(&encoded(&value))
        .expect("versioned source-specific normalization should validate");
    assert_eq!(observation.raw_value, Value::Null);
    assert_eq!(observation.normalization.normalized_value, json!(0));
}

#[test]
fn observation_rejects_normalization_without_rule_identity() {
    let mut value = parse_observation_fixture();
    value["normalization"]["applied"] = json!(true);
    value["normalization"]["normalized_value"] = json!(0);
    assert!(validate_observation_json(&encoded(&value)).is_err());
}

#[test]
fn omitted_or_not_retrieved_rows_cannot_be_normalized() {
    for state in ["row_omitted", "not_retrieved"] {
        let mut value = parse_observation_fixture();
        value["raw_value"] = Value::Null;
        value["raw_value_state"] = json!(state);
        value["normalization"] = json!({
            "applied": true,
            "rule_id": "incorrect-null-conversion",
            "rule_version": "1",
            "normalized_value": 0,
            "reason": "A missing observation is not a reported value."
        });
        let error = validate_observation_json(&encoded(&value))
            .expect_err("missing observations must not be normalized");
        assert!(error.to_string().contains("cannot be applied"));
    }
}

#[test]
fn non_fixture_observation_accepts_only_a_sha256_digest() {
    let mut value = parse_observation_fixture();
    value["fixture_only"] = json!(false);
    value["source_snapshot_sha256"] = json!(format!("sha256:{}", "a".repeat(64)));
    assert!(validate_observation_json(&encoded(&value)).is_ok());

    value["source_snapshot_sha256"] = json!(format!("sha256:{}", "G".repeat(64)));
    assert!(validate_observation_json(&encoded(&value)).is_err());
}

#[test]
fn observation_rejects_rewriting_values_without_a_rule() {
    let mut value = parse_observation_fixture();
    value["raw_value"] = json!(8);
    value["raw_value_state"] = json!("reported_value");
    value["normalization"]["normalized_value"] = json!(0);
    assert!(validate_observation_json(&encoded(&value)).is_err());
}

#[test]
fn observation_rejects_invalid_zero_claim_and_digest() {
    let mut bad_zero = parse_observation_fixture();
    bad_zero["raw_value"] = json!(1);
    bad_zero["raw_value_state"] = json!("reported_zero");
    bad_zero["normalization"]["normalized_value"] = json!(1);
    assert!(validate_observation_json(&encoded(&bad_zero)).is_err());

    let mut bad_digest = parse_observation_fixture();
    bad_digest["source_snapshot_sha256"] = json!("sha256:xyz");
    assert!(validate_observation_json(&encoded(&bad_digest)).is_err());
}

#[test]
fn non_fixture_observation_requires_snapshot_digest() {
    let mut value = parse_observation_fixture();
    value["fixture_only"] = json!(false);
    value["source_snapshot_sha256"] = Value::Null;
    assert!(validate_observation_json(&encoded(&value)).is_err());
}
