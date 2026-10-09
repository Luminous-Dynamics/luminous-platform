use std::collections::BTreeMap;

use luminous_operations_event_contract::{
    TrustedConnectionContext, ValidationIssue, validate_event, validate_event_for_connection,
};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../examples/incident.snapshot.json"))
        .expect("checked-in synthetic fixture must be valid JSON")
}

fn trusted_context() -> TrustedConnectionContext {
    let mut tenant_by_company = BTreeMap::new();
    tenant_by_company.insert("company-42".to_owned(), "tenant-demo-001".to_owned());

    let mut incident_by_company_resource = BTreeMap::new();
    incident_by_company_resource.insert(
        ("company-42".to_owned(), "7301".to_owned()),
        "incident-local-7301".to_owned(),
    );

    TrustedConnectionContext {
        source_uri: "urn:luminousdynamics:integration:connectwise-connection-demo".to_owned(),
        connection_id: "connectwise-connection-demo".to_owned(),
        producer_system: "connectwise-psa".to_owned(),
        tenant_by_company,
        incident_by_company_resource,
    }
}

fn issues_for(event: &Value) -> Vec<ValidationIssue> {
    validate_event(event).expect_err("mutated invalid event must be rejected")
}

fn has_rule(issues: &[ValidationIssue], rule: &str) -> bool {
    issues.iter().any(|issue| issue.rule == rule)
}

fn assert_invalid(event: &Value) {
    assert!(
        validate_event(event).is_err(),
        "invalid event must be rejected"
    );
}

#[test]
fn checked_in_fixture_validates() {
    assert!(validate_event(&fixture()).is_ok());
}

#[test]
fn fixture_matches_explicit_trusted_connection_context() {
    assert!(validate_event_for_connection(&fixture(), &trusted_context()).is_ok());
}

#[test]
fn rejects_wrong_cloudevents_specversion() {
    let mut event = fixture();
    event["specversion"] = json!("2.0");
    assert_invalid(&event);
}

#[test]
fn rejects_missing_tenant_id() {
    let mut event = fixture();
    event.as_object_mut().unwrap().remove("tenantid");
    assert_invalid(&event);
}

#[test]
fn rejects_unknown_top_level_fields() {
    let mut event = fixture();
    event["execute_command"] = json!("reboot");
    assert_invalid(&event);
}

#[test]
fn rejects_unversioned_or_malformed_event_type() {
    let mut event = fixture();
    event["type"] = json!("command.exec.v1");
    assert_invalid(&event);
}

#[test]
fn rejects_unknown_producer_system() {
    let mut event = fixture();
    event["producersystem"] = json!("unknown-trust-root");
    assert_invalid(&event);
}

#[test]
fn rejects_unknown_actor_kind() {
    let mut event = fixture();
    event["actorkind"] = json!("ticket");
    assert_invalid(&event);
}

#[test]
fn rejects_malformed_source_uri() {
    let mut event = fixture();
    event["source"] = json!("not a uri");
    assert_invalid(&event);
}

#[test]
fn rejects_malformed_observed_timestamp() {
    let mut event = fixture();
    event["observedat"] = json!("yesterday");
    assert_invalid(&event);
}

#[test]
fn rejects_invalid_opaque_tenant_id() {
    let mut event = fixture();
    event["tenantid"] = json!("tenant/invalid");
    assert_invalid(&event);
}

#[test]
fn rejects_wrong_content_type() {
    let mut event = fixture();
    event["datacontenttype"] = json!("text/plain");
    assert_invalid(&event);
}

#[test]
fn rejects_malformed_correlation_id() {
    let mut event = fixture();
    event["correlationid"] = json!("not an opaque id");
    assert_invalid(&event);
}

#[test]
fn rejects_unknown_but_well_formed_payload_schema_uri() {
    let mut event = fixture();
    event["dataschema"] = json!(
        "https://luminousdynamics.io/contracts/operations-event-v1/payloads/other-v1.schema.json"
    );
    let issues = issues_for(&event);
    assert!(has_rule(&issues, "unsupported_dataschema"));
}

#[test]
fn rejects_payload_with_unknown_execution_field() {
    let mut event = fixture();
    event["data"]["execute_command"] = json!("reboot");
    assert_invalid(&event);
}

#[test]
fn rejects_unrecognized_incident_status() {
    let mut event = fixture();
    event["data"]["status"] = json!("maintenance");
    assert_invalid(&event);
}

#[test]
fn rejects_empty_incident_summary() {
    let mut event = fixture();
    event["data"]["summary"] = json!("");
    assert_invalid(&event);
}

#[test]
fn rejects_priority_outside_normalized_range() {
    let mut event = fixture();
    event["data"]["priority"] = json!(6);
    assert_invalid(&event);
}

#[test]
fn rejects_invalid_evidence_digest() {
    let mut event = fixture();
    event["data"]["evidence_refs"] = json!([{
        "artifact_id": "evidence-1",
        "sha256": "not-a-digest",
        "classification": "internal"
    }]);
    assert_invalid(&event);
}

#[test]
fn rejects_missing_evidence_refs_collection() {
    let mut event = fixture();
    event["data"].as_object_mut().unwrap().remove("evidence_refs");
    assert_invalid(&event);
}

#[test]
fn rejects_non_object_payload() {
    let mut event = fixture();
    event["data"] = json!("not an object");
    assert_invalid(&event);
}

#[test]
fn validation_diagnostics_do_not_echo_untrusted_values() {
    let mut event = fixture();
    event["unexpected_private_value"] = json!("SYNTHETIC_SECRET_SENTINEL");
    let issues = issues_for(&event);
    assert!(!format!("{issues:?}").contains("SYNTHETIC_SECRET_SENTINEL"));
    assert!(issues.iter().all(|issue| issue.rule == "schema_violation"));
}

#[test]
fn trusted_binding_rejects_different_source_uri() {
    let mut event = fixture();
    event["source"] = json!("urn:luminousdynamics:integration:attacker");
    let issues = validate_event_for_connection(&event, &trusted_context()).unwrap_err();
    assert!(has_rule(&issues, "source_binding_mismatch"));
}

#[test]
fn trusted_binding_rejects_different_actor_identity() {
    let mut event = fixture();
    event["actorid"] = json!("attacker-connection");
    let issues = validate_event_for_connection(&event, &trusted_context()).unwrap_err();
    assert!(has_rule(&issues, "actor_binding_mismatch"));
}

#[test]
fn trusted_binding_rejects_different_producer() {
    let mut event = fixture();
    event["producersystem"] = json!("nixward");
    let issues = validate_event_for_connection(&event, &trusted_context()).unwrap_err();
    assert!(has_rule(&issues, "producer_binding_mismatch"));
}

#[test]
fn trusted_binding_rejects_tenant_mismatch() {
    let mut event = fixture();
    event["tenantid"] = json!("tenant-other-001");
    let issues = validate_event_for_connection(&event, &trusted_context()).unwrap_err();
    assert!(has_rule(&issues, "tenant_binding_mismatch"));
}

#[test]
fn trusted_binding_rejects_unmapped_company() {
    let mut event = fixture();
    event["data"]["source_reference"]["company_id"] = json!("company-unknown");
    let issues = validate_event_for_connection(&event, &trusted_context()).unwrap_err();
    assert!(has_rule(&issues, "unmapped_source_company"));
}

#[test]
fn trusted_binding_rejects_unmapped_incident() {
    let mut event = fixture();
    event["data"]["source_reference"]["resource_id"] = json!("9999");
    let issues = validate_event_for_connection(&event, &trusted_context()).unwrap_err();
    assert!(has_rule(&issues, "unmapped_source_incident"));
}

#[test]
fn trusted_binding_rejects_subject_not_matching_mapping() {
    let mut event = fixture();
    event["subject"] = json!("incident/another-local-id");
    let issues = validate_event_for_connection(&event, &trusted_context()).unwrap_err();
    assert!(has_rule(&issues, "incident_subject_mismatch"));
}

#[test]
fn trusted_binding_rejects_source_reference_system_mismatch() {
    let mut event = fixture();
    event["data"]["source_reference"]["system"] = json!("nixward");
    let issues = validate_event_for_connection(&event, &trusted_context()).unwrap_err();
    assert!(has_rule(&issues, "source_reference_system_mismatch"));
}

#[test]
fn trusted_binding_rejects_unsupported_resource_type() {
    let mut event = fixture();
    event["data"]["source_reference"]["resource_type"] = json!("service_asset");
    let issues = validate_event_for_connection(&event, &trusted_context()).unwrap_err();
    assert!(has_rule(&issues, "unsupported_resource_type"));
}

#[test]
fn incomplete_trusted_context_fails_closed() {
    let context = TrustedConnectionContext::default();
    let issues = validate_event_for_connection(&fixture(), &context).unwrap_err();
    assert!(has_rule(&issues, "trusted_context_invalid"));
}

#[test]
fn valid_optional_occurrence_time_is_accepted() {
    let mut event = fixture();
    event["time"] = json!("2026-10-09T08:00:00Z");
    assert!(validate_event(&event).is_ok());
}

#[test]
fn invalid_optional_occurrence_time_is_rejected() {
    let mut event = fixture();
    event["time"] = json!("tomorrow morning");
    assert_invalid(&event);
}
