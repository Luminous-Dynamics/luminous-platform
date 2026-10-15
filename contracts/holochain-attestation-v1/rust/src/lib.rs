//! Rust-native validation for the Holochain Attestation Share Package V1.
//!
//! Schema validation is necessary but not sufficient. This crate also rejects
//! duplicate manifest references and self-referential disputes/supersessions.
//! It does not authenticate issuers, authorize publication, verify evidence
//! signatures, validate DHT state, or prove the represented real-world claim.

use std::collections::HashSet;
use std::sync::OnceLock;

use jsonschema::{Draft, Validator};
use serde_json::Value;
use uuid::Uuid;

const SCHEMA_JSON: &str = include_str!("../../share-package.schema.json");

static VALIDATOR: OnceLock<Result<Validator, ()>> = OnceLock::new();

/// A non-sensitive validation diagnostic.
///
/// Diagnostics intentionally contain only a JSON Pointer and a stable rule
/// identifier. They never include rejected values, issuer-supplied strings,
/// ticket content, manifest identifiers, or validator-rendered input fragments.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ValidationIssue {
    /// RFC 6901-style instance pointer; `/` denotes the root.
    pub path: String,
    /// Stable machine-readable rule code.
    pub rule: &'static str,
}

fn schema_validator() -> Result<&'static Validator, ()> {
    match VALIDATOR.get_or_init(|| {
        let schema: Value = serde_json::from_str(SCHEMA_JSON).map_err(|_| ())?;
        jsonschema::options()
            .with_draft(Draft::Draft202012)
            .should_validate_formats(true)
            .offline()
            .build(&schema)
            .map_err(|_| ())
    }) {
        Ok(validator) => Ok(validator),
        Err(()) => Err(()),
    }
}

fn pointer_or_root(path: String) -> String {
    if path.is_empty() {
        "/".to_owned()
    } else {
        path
    }
}

fn same_uuid(left: Option<&Value>, right: Option<&Value>) -> bool {
    let (Some(left), Some(right)) = (left.and_then(Value::as_str), right.and_then(Value::as_str))
    else {
        return false;
    };
    match (Uuid::parse_str(left), Uuid::parse_str(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

/// Validate shape and local cross-field invariants for a share package.
///
/// Returns `Ok(())` only when the pinned Draft 2020-12 schema and semantic
/// invariants pass. If the embedded schema cannot be compiled, validation
/// fails closed with a generic `validator_unavailable` issue.
pub fn validate_share_package(instance: &Value) -> Result<(), Vec<ValidationIssue>> {
    let validator = schema_validator().map_err(|()| {
        vec![ValidationIssue {
            path: "/".to_owned(),
            rule: "validator_unavailable",
        }]
    })?;

    let mut issues: Vec<ValidationIssue> = validator
        .iter_errors(instance)
        .map(|error| ValidationIssue {
            path: pointer_or_root(error.instance_path().to_string()),
            rule: "schema_violation",
        })
        .collect();

    if let Some(object) = instance.as_object() {
        let record_type = object.get("recordType").and_then(Value::as_str);

        if matches!(record_type, Some("dispute" | "supersession")) {
            if same_uuid(object.get("targetShareId"), object.get("shareId")) {
                issues.push(ValidationIssue {
                    path: "/targetShareId".to_owned(),
                    rule: "target_self_reference",
                });
            }
        } else if object.contains_key("targetShareId") {
            issues.push(ValidationIssue {
                path: "/targetShareId".to_owned(),
                rule: "target_reference_not_allowed",
            });
        }

        if let Some(manifests) = object.get("evidenceManifest").and_then(Value::as_array) {
            let mut seen = HashSet::with_capacity(manifests.len());
            for (index, item) in manifests.iter().enumerate() {
                let Some(reference) = item.get("manifestRef").and_then(Value::as_str) else {
                    continue;
                };
                if !seen.insert(reference) {
                    issues.push(ValidationIssue {
                        path: format!("/evidenceManifest/{index}/manifestRef"),
                        rule: "duplicate_manifest_reference",
                    });
                }
            }
        }
    }

    issues.sort_unstable();
    issues.dedup();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}
