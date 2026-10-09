use luminous_holochain_attestation_contract::{validate_share_package, ValidationIssue};
use serde_json::{json, Value};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../examples/evidence-attestation.synthetic.json"))
        .expect("checked-in synthetic fixture must be valid JSON")
}

fn validate(value: &Value) -> Result<(), Vec<ValidationIssue>> {
    validate_share_package(value)
}

fn invalid(value: &Value) -> Vec<ValidationIssue> {
    validate(value).expect_err("mutated invalid instance must be rejected")
}

fn has_rule(issues: &[ValidationIssue], rule: &str) -> bool {
    issues.iter().any(|issue| issue.rule == rule)
}

#[test]
fn schema_is_valid_draft_2020_12() {
    assert!(validate(&fixture()).is_ok(), "validating the fixture compiles the pinned Draft 2020-12 schema");
}

#[test]
fn synthetic_fixture_is_valid() {
    assert_eq!(Ok(()), validate(&fixture()));
}

#[test]
fn malformed_record_type_is_rejected_without_validator_crash() {
    let mut event = fixture();
    event["recordType"] = json!(["dispute"]);
    assert!(!invalid(&event).is_empty());
}

#[test]
fn rejects_unrecognized_free_text_or_ticket_fields() {
    let mut event = fixture();
    event["ticketDescription"] = json!("synthetic customer details");
    assert!(!invalid(&event).is_empty());
}

#[test]
fn rejects_raw_sensitive_data_declaration() {
    let mut event = fixture();
    event["privacyReview"]["rawSensitiveDataIncluded"] = json!(true);
    assert!(!invalid(&event).is_empty());
}

#[test]
fn rejects_unapproved_share_classification() {
    let mut event = fixture();
    event["privacyReview"]["dataClass"] = json!("tenant-private");
    assert!(!invalid(&event).is_empty());
}

#[test]
fn rejects_any_execution_authority() {
    let mut event = fixture();
    event["authorityEffect"] = json!("remote-execution");
    assert!(!invalid(&event).is_empty());
}

#[test]
fn dispute_requires_target_and_cannot_target_itself() {
    let mut event = fixture();
    event["recordType"] = json!("dispute");
    event["statementCode"] = json!("disputed");
    assert!(!invalid(&event).is_empty(), "dispute without a target must fail");

    let own_id = event["shareId"].as_str().expect("fixture shareId").to_uppercase();
    event["targetShareId"] = json!(own_id);
    assert!(has_rule(&invalid(&event), "target_self_reference"), "UUID case normalization must not permit self-reference");

    event["targetShareId"] = json!("d7a5cf2a-7598-4c5c-b65e-f19f4a765901");
    assert!(validate(&event).is_ok());
}

#[test]
fn supersession_requires_target_and_cannot_target_itself() {
    let mut event = fixture();
    event["recordType"] = json!("supersession");
    event["statementCode"] = json!("supersedes");
    assert!(!invalid(&event).is_empty(), "supersession without a target must fail");

    let own_id = event["shareId"].as_str().expect("fixture shareId").to_uppercase();
    event["targetShareId"] = json!(own_id);
    assert!(has_rule(&invalid(&event), "target_self_reference"), "UUID case normalization must not permit self-reference");

    event["targetShareId"] = json!("d7a5cf2a-7598-4c5c-b65e-f19f4a765901");
    assert!(validate(&event).is_ok());
}

#[test]
fn target_reference_is_only_allowed_for_dispute_or_supersession() {
    let mut event = fixture();
    event["targetShareId"] = json!("d7a5cf2a-7598-4c5c-b65e-f19f4a765901");
    assert!(has_rule(&invalid(&event), "target_reference_not_allowed"));
}

#[test]
fn evidence_attestation_requires_a_nonempty_manifest() {
    let mut event = fixture();
    event.as_object_mut().expect("fixture object").remove("evidenceManifest");
    assert!(!invalid(&event).is_empty(), "missing evidence manifest must fail");

    let mut event = fixture();
    event["evidenceManifest"] = json!([]);
    assert!(!invalid(&event).is_empty(), "empty evidence manifest must fail");
}

#[test]
fn incident_acknowledgement_may_omit_evidence_manifest() {
    let mut event = fixture();
    event["recordType"] = json!("incident-acknowledgement");
    event["statementCode"] = json!("received");
    event.as_object_mut().expect("fixture object").remove("evidenceManifest");
    assert!(validate(&event).is_ok());
}

#[test]
fn record_type_and_statement_code_cannot_disagree() {
    let mut event = fixture();
    event["recordType"] = json!("incident-acknowledgement");
    event["statementCode"] = json!("reviewed");
    assert!(!invalid(&event).is_empty());
}

#[test]
fn rejects_malformed_evidence_digest() {
    let mut event = fixture();
    event["evidenceManifest"][0]["digest"] = json!("not-a-sha256-digest");
    assert!(!invalid(&event).is_empty());
}

#[test]
fn rejects_unknown_issuer_role_claim() {
    let mut event = fixture();
    event["issuerRoleClaim"] = json!("platform-root-authority");
    assert!(!invalid(&event).is_empty());
}

#[test]
fn rejects_invalid_timestamp() {
    let mut event = fixture();
    event["issuedAt"] = json!("yesterday");
    assert!(!invalid(&event).is_empty());
}

#[test]
fn rejects_duplicate_manifest_references_when_other_fields_differ() {
    let mut event = fixture();
    let mut duplicate = event["evidenceManifest"][0].clone();
    duplicate["assessmentClaim"] = json!("signature-check-reported");
    event["evidenceManifest"].as_array_mut().expect("manifest list").push(duplicate);
    assert!(has_rule(&invalid(&event), "duplicate_manifest_reference"));
}

#[test]
fn limits_evidence_manifest_references() {
    let mut event = fixture();
    let first = event["evidenceManifest"][0].clone();
    event["evidenceManifest"] = Value::Array(vec![first; 17]);
    assert!(!invalid(&event).is_empty());
}

#[test]
fn validation_errors_do_not_echo_untrusted_values() {
    let mut event = fixture();
    let secret_like_value = "synthetic-sensitive-issuer-value";
    event["issuerRoleClaim"] = json!(secret_like_value);
    let issues = invalid(&event);
    assert!(!issues.is_empty());
    assert!(issues.iter().all(|issue| !issue.path.contains(secret_like_value) && issue.rule != secret_like_value));

    let mut event = fixture();
    let manifest_ref = event["evidenceManifest"][0]["manifestRef"].as_str().expect("manifestRef").to_owned();
    let mut duplicate = event["evidenceManifest"][0].clone();
    duplicate["assessmentClaim"] = json!("signature-check-reported");
    event["evidenceManifest"].as_array_mut().expect("manifest list").push(duplicate);
    let issues = invalid(&event);
    assert!(has_rule(&issues, "duplicate_manifest_reference"));
    assert!(issues.iter().all(|issue| !issue.path.contains(&manifest_ref)));
}

#[test]
fn issuer_role_claim_never_promotes_authority() {
    let event = fixture();
    assert_eq!(json!("none"), event["authorityEffect"]);
    assert_eq!(json!("service-provider"), event["issuerRoleClaim"]);
}
