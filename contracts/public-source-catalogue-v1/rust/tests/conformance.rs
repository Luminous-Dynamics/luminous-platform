use public_source_catalogue_v1::{validate_catalogue_json, validate_observation_json};
use serde_json::{json, Value};

const CATALOGUE: &str = include_str!("../../examples/catalogue.json");
const OBSERVATION: &str = include_str!("../../examples/observation-v1.json");
const OBSERVATION_SCHEMA: &str = include_str!("../../observation.schema.json");
const JHB_POWER_WATER_STATEMENT: &str =
    include_str!("../../examples/johannesburg-power-water-statement-v1.synthetic.json");
const JHB_WATER_DEPOT: &str =
    include_str!("../../examples/johannesburg-water-depot-upgrade-2026-10-12.synthetic.json");
const JHB_BRAAMFONTEIN_TRAFFIC: &str =
    include_str!("../../examples/braamfontein-traffic-signal-upgrade-2026-10-02.synthetic.json");
const JHB_MALVERN_CLINIC: &str =
    include_str!("../../examples/malvern-clinic-reopening-2026-10-14.synthetic.json");

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
    assert_eq!(catalogue.sources.len(), 13);
    assert!(catalogue
        .sources
        .iter()
        .all(|source| !source.ingestion.enabled));

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
        "za-gp-jhb-city-project-updates",
        "za-gp-jhb-johannesburg-water-emergency-notices",
        "za-gov-water-sanitation-compulsory-standards",
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
fn water_clock_semantics_and_national_standards_limits_are_preserved() {
    let catalogue = validate_catalogue_json(CATALOGUE).expect("catalogue should validate");
    let emergency = catalogue
        .sources
        .iter()
        .find(|source| source.source_id == "za-gp-jhb-johannesburg-water-emergency-notices")
        .expect("daily water notices must be catalogued");
    assert!(emergency.known_limitations.iter().any(|limitation| {
        limitation.contains("clock start/stop rule")
    }));
    assert!(emergency.known_limitations.iter().any(|limitation| {
        limitation.contains("do not infer a contradiction")
    }));
    assert!(!emergency.ingestion.enabled);

    let standards = catalogue
        .sources
        .iter()
        .find(|source| source.source_id == "za-gov-water-sanitation-compulsory-standards")
        .expect("official national standards landing page must be catalogued");
    assert!(standards.known_limitations.iter().any(|limitation| {
        limitation.contains("current legal status")
    }));
    assert!(standards.known_limitations.iter().any(|limitation| {
        limitation.contains("qualified human/legal review")
    }));
    assert!(!standards.ingestion.enabled);
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
        assert!(
            validate_catalogue_json(&encoded(&value)).is_err(),
            "accepted {url}"
        );
    }
}
#[test]
fn catalogue_rejects_enabled_ingestion() {
    let mut value = parse_catalogue_fixture();
    value["sources"][0]["ingestion"]["enabled"] = json!(true);
    let error =
        validate_catalogue_json(&encoded(&value)).expect_err("ingestion must stay disabled");
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
fn observation_json_schema_binds_fixture_and_capture_provenance_consistently() {
    let schema: Value =
        serde_json::from_str(OBSERVATION_SCHEMA).expect("observation schema must be valid JSON");
    let required = schema["required"]
        .as_array()
        .expect("schema must require an array of fields");
    for key in ["retrieved_at", "source_snapshot_sha256", "fixture_only"] {
        assert!(
            required.iter().any(|value| value.as_str() == Some(key)),
            "{key} must be present even when its value is nullable"
        );
    }

    let guards = schema["allOf"]
        .as_array()
        .expect("schema must express source-provenance conditional rules");
    let fixture_guard = guards
        .iter()
        .find(|guard| guard["if"]["properties"]["fixture_only"]["const"] == true)
        .expect("fixture-only observations must have a dedicated schema guard");
    assert_eq!(
        fixture_guard["then"]["properties"]["retrieved_at"]["type"],
        "null"
    );
    assert_eq!(
        fixture_guard["then"]["properties"]["source_snapshot_sha256"]["type"],
        "null"
    );
    assert_eq!(
        fixture_guard["else"]["properties"]["retrieved_at"]["type"],
        "string"
    );
    assert_eq!(
        fixture_guard["else"]["properties"]["retrieved_at"]["format"],
        "date-time"
    );
    assert_eq!(
        fixture_guard["else"]["properties"]["source_snapshot_sha256"]["pattern"],
        "^sha256:[a-f0-9]{64}$"
    );
}

#[test]
fn canonical_synthetic_observation_is_accepted() {
    let observation =
        validate_observation_json(OBSERVATION).expect("synthetic observation should validate");
    assert!(observation.fixture_only);
    assert!(observation.retrieved_at.is_none());
    assert!(observation.source_snapshot_sha256.is_null());
    assert_eq!(observation.source_id, "za-treasury-municipal-finance");
}

#[test]
fn synthetic_johannesburg_power_water_statement_is_not_live_or_causal_evidence() {
    let observation = validate_observation_json(JHB_POWER_WATER_STATEMENT)
        .expect("synthetic source-claim fixture should validate");
    assert!(observation.fixture_only);
    assert!(observation.retrieved_at.is_none());
    assert_eq!(observation.source_id, "za-gov-jhb-power-water-coordination");
    assert_eq!(observation.raw_value["is_live_service_status"], false);
    assert_eq!(observation.raw_value["is_case_level_causal_proof"], false);
    assert_eq!(
        observation.raw_value["is_independently_verified_outcome"],
        false
    );
    assert!(observation.source_snapshot_sha256.is_none());

    let catalogue = validate_catalogue_json(CATALOGUE).expect("catalogue should validate");
    let source = catalogue
        .sources
        .iter()
        .find(|source| source.source_id == observation.source_id)
        .expect("fixture source must exist in the catalogue");
    assert!(!source.ingestion.enabled);
}

#[test]
fn synthetic_city_project_announcements_do_not_prove_live_or_verified_outcomes() {
    let fixtures = [
        (JHB_WATER_DEPOT, "2026-10-12"),
        (JHB_BRAAMFONTEIN_TRAFFIC, "2026-10-02"),
        (JHB_MALVERN_CLINIC, "2026-10-14"),
    ];

    let catalogue = validate_catalogue_json(CATALOGUE).expect("catalogue should validate");
    let source = catalogue
        .sources
        .iter()
        .find(|source| source.source_id == "za-gp-jhb-city-project-updates")
        .expect("City project announcement source must be catalogued");
    assert!(!source.ingestion.enabled);

    for (fixture, expected_date) in fixtures {
        let observation = validate_observation_json(fixture)
            .expect("synthetic City project statement should validate");
        assert_eq!(observation.source_id, "za-gp-jhb-city-project-updates");
        assert!(observation.fixture_only);
        assert!(observation.retrieved_at.is_none());
        assert!(observation.source_snapshot_sha256.is_none());
        assert_eq!(observation.raw_value["claim_kind"], "publisher_statement");
        assert_eq!(observation.raw_value["publication_date"], expected_date);
        assert_eq!(observation.raw_value["is_live_service_status"], false);
        assert_eq!(
            observation.raw_value["is_current_operational_status_independently_verified"],
            false
        );
        assert_eq!(
            observation.raw_value["is_independently_verified_outcome"],
            false
        );
        assert_eq!(observation.normalization.applied, false);
        assert_eq!(
            observation.normalization.normalized_value, observation.raw_value,
            "no-op normalization must preserve the raw publisher claim"
        );
    }

    let depot: Value = serde_json::from_str(JHB_WATER_DEPOT).expect("valid depot fixture");
    assert_eq!(
        depot["raw_value"]["reported_progress"]["relation"],
        "greater_than"
    );
    assert_eq!(depot["raw_value"]["reported_progress"]["percent"], 89);
    assert_eq!(
        depot["raw_value"]["reported_project_state"],
        "final_close_out_in_progress"
    );

    let traffic: Value =
        serde_json::from_str(JHB_BRAAMFONTEIN_TRAFFIC).expect("valid traffic fixture");
    assert_eq!(traffic["raw_value"]["reported_intersection_count"], 8);
    assert_eq!(
        traffic["raw_value"]["reported_project_value_zar"],
        19_000_000
    );

    let clinic: Value = serde_json::from_str(JHB_MALVERN_CLINIC).expect("valid clinic fixture");
    assert_eq!(
        clinic["raw_value"]["city_reported_service_state"],
        "reopened_after_refurbishment"
    );
    assert_eq!(
        clinic["raw_value"]["is_current_operational_status_independently_verified"],
        false
    );
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
    no_timezone["fixture_only"] = json!(false);
    no_timezone["retrieved_at"] = json!("2026-10-12T12:00:00");
    no_timezone["source_snapshot_sha256"] = json!(format!("sha256:{}", "a".repeat(64)));
    let error = validate_observation_json(&encoded(&no_timezone))
        .expect_err("captured observation timestamps must include an explicit timezone");
    assert!(error.to_string().contains("valid RFC3339 timestamp"));
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
    value["retrieved_at"] = json!("2026-10-12T12:00:00Z");
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
fn fixture_only_observations_reject_synthetic_retrieval_timestamps() {
    let fixtures = [
        parse_observation_fixture(),
        serde_json::from_str::<Value>(JHB_POWER_WATER_STATEMENT)
            .expect("Johannesburg fixture must be valid JSON"),
    ];
    for fixture in fixtures {
        let mut claimed_capture = fixture;
        claimed_capture["retrieved_at"] = json!("2026-10-14T00:00:00Z");
        let error = validate_observation_json(&encoded(&claimed_capture))
            .expect_err("fixture-only observations must not claim a retrieval timestamp");
        assert!(error
            .to_string()
            .contains("retrieved_at must be null for fixture-only observations"));
    }
}
#[test]
fn observations_reject_a_missing_retrieved_at_field_even_for_fixtures() {
    let mut value = parse_observation_fixture();
    value
        .as_object_mut()
        .expect("observation must be an object")
        .remove("retrieved_at");
    assert!(
        validate_observation_json(&encoded(&value)).is_err(),
        "nullable retrieved_at must still be a required key"
    );
}

#[test]
fn non_fixture_observation_requires_retrieval_timestamp_and_snapshot_digest() {
    let mut no_retrieval_time = parse_observation_fixture();
    no_retrieval_time["fixture_only"] = json!(false);
    no_retrieval_time["retrieved_at"] = Value::Null;
    no_retrieval_time["source_snapshot_sha256"] = json!(format!("sha256:{}", "a".repeat(64)));
    let error = validate_observation_json(&encoded(&no_retrieval_time))
        .expect_err("captured observation must have a retrieval timestamp");
    assert!(error
        .to_string()
        .contains("retrieved_at is required for non-fixture observations"));

    let mut no_digest = parse_observation_fixture();
    no_digest["fixture_only"] = json!(false);
    no_digest["retrieved_at"] = json!("2026-10-12T12:00:00Z");
    no_digest["source_snapshot_sha256"] = Value::Null;
    let error = validate_observation_json(&encoded(&no_digest))
        .expect_err("captured observation must have a source digest");
    assert!(error
        .to_string()
        .contains("source_snapshot_sha256 must be sha256:"));
}

#[test]
fn non_fixture_observation_requires_snapshot_digest() {
    let mut value = parse_observation_fixture();
    value["fixture_only"] = json!(false);
    value["retrieved_at"] = json!("2026-10-12T12:00:00Z");
    value["source_snapshot_sha256"] = Value::Null;
    let error = validate_observation_json(&encoded(&value))
        .expect_err("captured observations must bind a source snapshot digest");
    assert!(error
        .to_string()
        .contains("source_snapshot_sha256 must be sha256:"));
}
