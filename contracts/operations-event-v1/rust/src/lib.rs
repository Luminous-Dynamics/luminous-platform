//! Rust-native validation for Operations Event Contract V1.
//!
//! This crate validates the checked-in envelope and the one explicitly supported
//! incident-snapshot payload schema. The trusted-connection API checks that
//! claims in an otherwise valid event match trusted configuration supplied by
//! the caller. It does not authenticate that caller, persist events, verify
//! signatures, grant authority, or authorize execution.
//!
//! Diagnostics are deliberately bounded: stable rule identifiers and JSON
//! Pointer paths only. Untrusted values and validator-rendered messages are
//! never returned.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use jsonschema::{Draft, Validator};
use serde_json::Value;

const EVENT_SCHEMA_JSON: &str = include_str!("../../schema.json");
const INCIDENT_SCHEMA_JSON: &str =
    include_str!("../../payloads/incident-snapshot-v1.schema.json");
const INCIDENT_SCHEMA_ID: &str =
    "https://luminousdynamics.io/contracts/operations-event-v1/payloads/incident-snapshot-v1.schema.json";

static EVENT_VALIDATOR: OnceLock<Result<Validator, ()>> = OnceLock::new();
static INCIDENT_VALIDATOR: OnceLock<Result<Validator, ()>> = OnceLock::new();

/// A non-sensitive validation diagnostic.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ValidationIssue {
    /// RFC 6901-style instance pointer; `/` denotes the root.
    pub path: String,
    /// Stable machine-readable rule code.
    pub rule: &'static str,
}

fn issue(path: impl Into<String>, rule: &'static str) -> ValidationIssue {
    let path = path.into();
    ValidationIssue {
        path: if path.is_empty() { "/".to_owned() } else { path },
        rule,
    }
}

fn validator_from_embedded_schema(schema_json: &str) -> Result<Validator, ()> {
    let schema: Value = serde_json::from_str(schema_json).map_err(|_| ())?;
    jsonschema::options()
        .with_draft(Draft::Draft202012)
        .should_validate_formats(true)
        .offline()
        .build(&schema)
        .map_err(|_| ())
}

fn event_validator() -> Result<&'static Validator, ()> {
    match EVENT_VALIDATOR.get_or_init(|| validator_from_embedded_schema(EVENT_SCHEMA_JSON)) {
        Ok(validator) => Ok(validator),
        Err(()) => Err(()),
    }
}

fn incident_validator() -> Result<&'static Validator, ()> {
    match INCIDENT_VALIDATOR.get_or_init(|| validator_from_embedded_schema(INCIDENT_SCHEMA_JSON)) {
        Ok(validator) => Ok(validator),
        Err(()) => Err(()),
    }
}

fn schema_issues(validator: &Validator, instance: &Value, prefix: &str) -> Vec<ValidationIssue> {
    validator
        .iter_errors(instance)
        .map(|error| {
            let path = error.instance_path().to_string();
            let path = if prefix.is_empty() {
                path
            } else if path.is_empty() {
                prefix.to_owned()
            } else {
                format!("{prefix}{path}")
            };
            issue(path, "schema_violation")
        })
        .collect()
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str()
}

/// Validate the envelope and its exact, supported domain payload schema.
///
/// This never resolves a schema URI over the network. A syntactically valid but
/// unknown `dataschema` is rejected rather than treated as an unvalidated
/// payload or fetched dynamically.
pub fn validate_event(event: &Value) -> Result<(), Vec<ValidationIssue>> {
    let mut issues = match event_validator() {
        Ok(validator) => schema_issues(validator, event, ""),
        Err(()) => vec![issue("/", "validator_unavailable")],
    };

    match string_at(event, &["dataschema"]) {
        Some(INCIDENT_SCHEMA_ID) => match incident_validator() {
            Ok(validator) => issues.extend(schema_issues(
                validator,
                event.get("data").unwrap_or(&Value::Null),
                "/data",
            )),
            Err(()) => issues.push(issue("/", "validator_unavailable")),
        },
        Some(_) => issues.push(issue("/dataschema", "unsupported_dataschema")),
        None => {}
    }

    issues.sort_unstable();
    issues.dedup();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

/// Trusted configuration supplied by an authenticated connector boundary.
///
/// Callers must populate this from connection configuration and verified
/// authentication context, never from the incoming event itself.
#[derive(Clone, Debug, Default)]
pub struct TrustedConnectionContext {
    pub source_uri: String,
    pub connection_id: String,
    pub producer_system: String,
    /// Explicit external company ID -> canonical local tenant ID mapping.
    pub tenant_by_company: BTreeMap<String, String>,
    /// Explicit (external company ID, provider resource ID) -> local incident ID.
    pub incident_by_company_resource: BTreeMap<(String, String), String>,
}

/// Validate schema and compare event claims to trusted connector configuration.
///
/// The only supported mapping in this V1 contract is a ConnectWise-shaped
/// service-ticket incident snapshot. Successful validation means the event
/// matches the provided configuration; it does not authenticate the connection
/// or authorize any downstream side effect.
pub fn validate_event_for_connection(
    event: &Value,
    context: &TrustedConnectionContext,
) -> Result<(), Vec<ValidationIssue>> {
    validate_event(event)?;

    let mut issues = Vec::new();
    if context.source_uri.is_empty()
        || context.connection_id.is_empty()
        || context.producer_system.is_empty()
    {
        issues.push(issue("/", "trusted_context_invalid"));
    }

    if string_at(event, &["source"]) != Some(context.source_uri.as_str()) {
        issues.push(issue("/source", "source_binding_mismatch"));
    }
    if string_at(event, &["actorid"]) != Some(context.connection_id.as_str()) {
        issues.push(issue("/actorid", "actor_binding_mismatch"));
    }
    if string_at(event, &["producersystem"]) != Some(context.producer_system.as_str()) {
        issues.push(issue("/producersystem", "producer_binding_mismatch"));
    }

    let company_id = string_at(event, &["data", "source_reference", "company_id"])
        .expect("payload schema requires company_id");
    let resource_id = string_at(event, &["data", "source_reference", "resource_id"])
        .expect("payload schema requires resource_id");
    let resource_system = string_at(event, &["data", "source_reference", "system"])
        .expect("payload schema requires system");
    let resource_type = string_at(event, &["data", "source_reference", "resource_type"])
        .expect("payload schema requires resource_type");

    if resource_system != context.producer_system {
        issues.push(issue(
            "/data/source_reference/system",
            "source_reference_system_mismatch",
        ));
    }
    if resource_type != "service_ticket" {
        issues.push(issue(
            "/data/source_reference/resource_type",
            "unsupported_resource_type",
        ));
    }

    match context.tenant_by_company.get(company_id) {
        Some(expected_tenant) => {
            if string_at(event, &["tenantid"]) != Some(expected_tenant.as_str()) {
                issues.push(issue("/tenantid", "tenant_binding_mismatch"));
            }
        }
        None => issues.push(issue(
            "/data/source_reference/company_id",
            "unmapped_source_company",
        )),
    }

    match context
        .incident_by_company_resource
        .get(&(company_id.to_owned(), resource_id.to_owned()))
    {
        Some(expected_incident) => {
            let expected_subject = format!("incident/{expected_incident}");
            if string_at(event, &["subject"]) != Some(expected_subject.as_str()) {
                issues.push(issue("/subject", "incident_subject_mismatch"));
            }
        }
        None => issues.push(issue(
            "/data/source_reference/resource_id",
            "unmapped_source_incident",
        )),
    }

    issues.sort_unstable();
    issues.dedup();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}
