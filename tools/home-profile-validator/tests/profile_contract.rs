use home_profile_validator::validate_document;
use serde_json::Value;

const SCHEMA: &str = include_str!("../../../schemas/home-system-profile.schema.json");
const VALID: &[(&str, &str)] = &[
    ("hearth-node", include_str!("../../../profiles/home-systems/valid/hearth-node-design-only.json")),
    ("arc-console", include_str!("../../../profiles/home-systems/valid/arc-console-design-only.json")),
    ("commons-compute", include_str!("../../../profiles/home-systems/valid/commons-compute-design-only.json")),
    ("robotics-joint-lab", include_str!("../../../profiles/home-systems/valid/robotics-joint-lab-design-only.json")),
];
const INVALID: &[(&str, &str)] = &[
    ("design-only-qualified-capability", include_str!("../../../profiles/home-systems/invalid/invalid-design-only-qualified-capability.json")),
    ("secrets-in-model-context", include_str!("../../../profiles/home-systems/invalid/invalid-secrets-in-model-context.json")),
    ("robotics-no-physical-stop", include_str!("../../../profiles/home-systems/invalid/invalid-robotics-no-stop.json")),
    ("compute-not-opt-in", include_str!("../../../profiles/home-systems/invalid/invalid-compute-not-opt-in.json")),
    ("qualified-without-evidence", include_str!("../../../profiles/home-systems/invalid/invalid-qualified-without-evidence.json")),
    ("qualified-with-fabricated-receipt", include_str!("../../../profiles/home-systems/invalid/invalid-qualified-with-fabricated-receipt.json")),
];

fn schema() -> Value {
    serde_json::from_str(SCHEMA).expect("schema JSON must parse")
}

#[test]
fn schema_is_valid_draft_2020_12() {
    let schema = schema();
    assert!(jsonschema::draft202012::meta::is_valid(&schema), "schema must validate against its Draft 2020-12 meta-schema");
}

#[test]
fn all_declared_valid_profiles_pass_shared_validator() {
    let schema = schema();
    for (name, raw) in VALID {
        let profile: Value = serde_json::from_str(raw).unwrap_or_else(|error| panic!("{name}: {error}"));
        validate_document(&schema, &profile)
            .unwrap_or_else(|errors| panic!("{name} rejected by production validator: {errors:#?}"));
    }
}

#[test]
fn all_negative_fixtures_are_rejected_by_shared_validator() {
    let schema = schema();
    for (name, raw) in INVALID {
        let profile: Value = serde_json::from_str(raw).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            validate_document(&schema, &profile).is_err(),
            "{name} must be rejected by the production validator"
        );
    }
}

#[test]
fn qualified_profile_requires_receipt_bound_to_exact_profile_and_subject() {
    let schema = schema();
    let mut profile: Value = serde_json::from_str(VALID[0].1).expect("fixture parses");
    let profile_id = profile["profile_id"].as_str().unwrap().to_owned();
    let digest = "a".repeat(64);

    profile["readiness"] = Value::String("qualified".into());
    profile["subject"]["sha256"] = Value::String(digest);
    profile["hardware"]["prototype_exists"] = Value::Bool(true);
    profile["qualification"]["records"] = serde_json::json!([{
        "profile_id": profile_id,
        "subject_sha256": "b".repeat(64),
        "result": "pass",
        "receipt_ref": "test-fixture://qualification/receipt-1",
        "verifier_profile": "synthetic-verifier-v1",
        "observed_at": "2026-10-15T00:00:00Z"
    }]);

    let errors = validate_document(&schema, &profile)
        .expect_err("a receipt bound to another subject must be rejected");
    assert!(
        errors.iter().any(|error| error.contains("requires a passing receipt bound to this profile_id")),
        "expected profile/subject-binding error, got: {errors:#?}"
    );
}

#[test]
fn matching_fabricated_receipt_still_fails_closed() {
    let schema = schema();
    let mut profile: Value = serde_json::from_str(VALID[0].1).expect("fixture parses");
    let profile_id = profile["profile_id"].as_str().unwrap().to_owned();
    let digest = "a".repeat(64);

    profile["readiness"] = Value::String("qualified".into());
    profile["subject"]["sha256"] = Value::String(digest.clone());
    profile["hardware"]["prototype_exists"] = Value::Bool(true);
    profile["qualification"]["records"] = serde_json::json!([{
        "profile_id": profile_id,
        "subject_sha256": digest,
        "result": "pass",
        "receipt_ref": "test-fixture://qualification/fabricated-receipt",
        "verifier_profile": "synthetic-verifier-v1",
        "observed_at": "2026-10-15T00:00:00Z"
    }]);

    let errors = validate_document(&schema, &profile)
        .expect_err("matching metadata and a fabricated receipt reference must not qualify");
    assert!(
        errors.iter().any(|error| error.contains("qualified readiness is fail-closed")),
        "expected explicit fail-closed qualification error, got: {errors:#?}"
    );
}
