use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const VERIFIER_CONTRACT: &str = "omsi-independent-provenance-binding-verifier/0.1";
const MANIFEST_CONTRACT: &str = "omsi-manufacturing-provenance-manifest/0.2";
const PROVENANCE_MODEL: &str = "w3c-prov-dm-aligned-subset/0.2";
const COVERAGE_CONTRACT: &str = "omsi-manufacturing-adapter-schema-coverage/0.1";

const ARTIFACTS: [(&str, &str); 8] = [
    ("source-record", "source_record"),
    ("source-schema", "source_schema"),
    ("adapter-map", "adapter_map"),
    ("schema-coverage-report", "schema_coverage_report"),
    ("projection-report", "projection_report"),
    ("semantic-input", "semantic_input"),
    ("semantic-report", "semantic_report"),
    ("replay-envelope", "replay_envelope"),
];

const LIMITATIONS: &[&str] = &[
    "This tool independently checks exact-byte artifact digests, cross-artifact references, and a fixed provenance graph; it does not re-execute the main validator.",
    "Schema coverage is independently recomputed for a restricted closed-object JSON Schema field-inventory subset; this is not full JSON Schema validation, and array item values are not recursively validated.",
    "The verifier does not execute adapter transformations or independently recalculate the manufacturing semantic decision.",
    "A well-bound manifest does not authenticate the source, producer, tenant, calibration issuer, instrument, or physical measurement.",
    "The manifest and this report are unsigned; successful verification is not authorization, regulatory certification, quality release, or machine-control permission.",
    "The implementation is a separate Rust crate but shares serde_json and sha2 dependencies with the producer. A second implementation in a different language/library and an authenticated attestation layer remain further independence steps.",
];

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<u8, String> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("verify-bundle") if args.len() == 11 => {
            let manifest = read_bounded(&args[2], MAX_INPUT_BYTES)?;
            let source_record = read_bounded(&args[3], MAX_INPUT_BYTES)?;
            let source_schema = read_bounded(&args[4], MAX_INPUT_BYTES)?;
            let adapter_map = read_bounded(&args[5], MAX_INPUT_BYTES)?;
            let coverage_report = read_bounded(&args[6], MAX_INPUT_BYTES)?;
            let projection_report = read_bounded(&args[7], MAX_INPUT_BYTES)?;
            let semantic_input = read_bounded(&args[8], MAX_INPUT_BYTES)?;
            let semantic_report = read_bounded(&args[9], MAX_INPUT_BYTES)?;
            let replay_envelope = read_bounded(&args[10], MAX_INPUT_BYTES)?;

            let report = verify_bundle(
                &manifest,
                [
                    &source_record,
                    &source_schema,
                    &adapter_map,
                    &coverage_report,
                    &projection_report,
                    &semantic_input,
                    &semantic_report,
                    &replay_envelope,
                ],
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|error| format!("failed to serialize independent verification report: {error}"))?
            );
            if report.get("status").and_then(Value::as_str) == Some("artifact_bindings_verified") {
                Ok(0)
            } else {
                Ok(1)
            }
        }
        _ => Err(
            "usage: omsi-provenance-verifier verify-bundle <manifest.json> <source-record.json> <source-schema.json> <adapter-map.json> <schema-coverage-report.json> <projection-report.json> <semantic-input.json> <semantic-report.json> <replay-envelope.json>".into(),
        ),
    }
}

fn read_bounded(path: &str, maximum_bytes: usize) -> Result<Vec<u8>, String> {
    let bytes = if path == "-" {
        let mut bytes = Vec::with_capacity(maximum_bytes.min(64 * 1024));
        io::stdin()
            .take(maximum_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("failed to read stdin: {error}"))?;
        bytes
    } else {
        let file = fs::File::open(path).map_err(|error| format!("failed to open {path}: {error}"))?;
        let metadata = file
            .metadata()
            .map_err(|error| format!("failed to inspect {path}: {error}"))?;
        if metadata.len() > maximum_bytes as u64 {
            return Err(format!(
                "input {path} is {} bytes; maximum is {maximum_bytes} bytes",
                metadata.len()
            ));
        }
        let mut bytes = Vec::with_capacity(maximum_bytes.min(64 * 1024));
        file.take(maximum_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("failed to read {path}: {error}"))?;
        bytes
    };
    if bytes.len() > maximum_bytes {
        return Err(format!(
            "input {path} exceeds the {maximum_bytes}-byte limit"
        ));
    }
    Ok(bytes)
}

/// Rechecks artifact-byte bindings and the fixed v0.2 derivation graph without
/// importing or invoking the OMSI producer, adapter, schema auditor, or evaluator.
pub fn verify_bundle(manifest_bytes: &[u8], artifact_bytes: [&[u8]; 8]) -> Value {
    let manifest_parse = parse_json(manifest_bytes);
    let manifest_json_ok = manifest_parse.is_ok();
    let manifest = manifest_parse.ok();
    let artifacts: Vec<Option<Value>> = artifact_bytes.iter().map(|bytes| parse_json(bytes).ok()).collect();

    let contract_ok = manifest.as_ref().is_some_and(|value| {
        value.get("manifest_contract").and_then(Value::as_str) == Some(MANIFEST_CONTRACT)
            && value.get("provenance_model").and_then(Value::as_str) == Some(PROVENANCE_MODEL)
            && value.get("status").and_then(Value::as_str) == Some("local_chain_consistent")
            && value.get("authority").and_then(Value::as_str) == Some("local_consistency_only")
    });

    let manifest_shape_ok = manifest.as_ref().is_some_and(|value| {
        let Some(object) = value.as_object() else { return false; };
        let expected: BTreeSet<&str> = [
            "manifest_contract",
            "provenance_model",
            "status",
            "authority",
            "digest_profile",
            "profile",
            "comparison_rule_id",
            "validator_version",
            "schema_coverage",
            "declared_transform_ids",
            "artifacts",
            "activities",
            "edges",
            "limitations",
        ]
        .into_iter()
        .collect();
        let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
        let digest_profile = value.get("digest_profile");
        actual == expected
            && digest_profile.and_then(|item| item.get("algorithm")).and_then(Value::as_str) == Some("sha256")
            && digest_profile.and_then(|item| item.get("scope")).and_then(Value::as_str) == Some("exact_input_bytes")
            && digest_profile.and_then(|item| item.get("canonicalization")).and_then(Value::as_str) == Some("none")
    });

    let artifact_matches: Vec<bool> = ARTIFACTS
        .iter()
        .enumerate()
        .map(|(index, (artifact_id, role))| {
            let Some(manifest) = manifest.as_ref() else { return false; };
            let Some(items) = manifest.get("artifacts").and_then(Value::as_array) else { return false; };
            if items.len() != ARTIFACTS.len() { return false; }
            let item = &items[index];
            let exact_shape = item.as_object().is_some_and(|object| {
                object.len() == 4
                    && object.contains_key("artifact_id")
                    && object.contains_key("role")
                    && object.contains_key("sha256")
                    && object.contains_key("byte_length")
            });
            exact_shape
                && item.get("artifact_id").and_then(Value::as_str) == Some(*artifact_id)
                && item.get("role").and_then(Value::as_str) == Some(*role)
                && item.get("sha256").and_then(Value::as_str) == Some(sha256_hex(artifact_bytes[index]).as_str())
                && item.get("byte_length").and_then(Value::as_u64) == Some(artifact_bytes[index].len() as u64)
                && artifacts[index].is_some()
        })
        .collect();

    let schema_coverage_ok = match (
        manifest.as_ref(),
        artifacts.get(0).and_then(Option::as_ref),
        artifacts.get(1).and_then(Option::as_ref),
        artifacts.get(2).and_then(Option::as_ref),
        artifacts.get(3).and_then(Option::as_ref),
    ) {
        (Some(manifest), Some(source_record), Some(_source_schema), Some(_adapter_map), Some(coverage)) => {
            let coverage_digest = sha256_hex(artifact_bytes[3]);
            let source_digest = sha256_hex(artifact_bytes[1]);
            let record_digest = sha256_hex(artifact_bytes[0]);
            let map_digest = sha256_hex(artifact_bytes[2]);
            let section = manifest.get("schema_coverage");
            let schema_count = coverage.get("schema_field_count").and_then(Value::as_u64);
            let covered_count = coverage.get("covered_schema_field_count").and_then(Value::as_u64);
            let no_coverage_gaps = ["schema_paths_not_mapped", "mapping_paths_outside_schema", "presence_mismatches", "duplicate_source_mappings", "required_schema_paths_not_observed_in_record", "schema_errors", "record_structure_errors"]
                .iter()
                .all(|field| coverage.get(*field).and_then(Value::as_array).is_some_and(Vec::is_empty));
            let observed_audit_clean = coverage.get("observed_field_accounting_status").and_then(Value::as_str)
                == Some("complete_accounting")
                && coverage.get("observed_silent_loss_count").and_then(Value::as_u64) == Some(0);
            let status_and_contract = coverage.get("coverage_contract").and_then(Value::as_str) == Some(COVERAGE_CONTRACT)
                && coverage.get("status").and_then(Value::as_str) == Some("schema_coverage_complete")
                && coverage.get("authority").and_then(Value::as_str) == Some("schema_field_path_coverage_only");
            let hashes_match = coverage.get("source_schema_sha256").and_then(Value::as_str) == Some(source_digest.as_str())
                && coverage.get("source_record_sha256").and_then(Value::as_str) == Some(record_digest.as_str())
                && coverage.get("adapter_map_sha256").and_then(Value::as_str) == Some(map_digest.as_str());
            let manifest_section_matches = section.is_some_and(|section| {
                section.get("contract").and_then(Value::as_str) == Some(COVERAGE_CONTRACT)
                    && section.get("status").and_then(Value::as_str) == Some("schema_coverage_complete")
                    && section.get("source_schema_sha256").and_then(Value::as_str) == Some(source_digest.as_str())
                    && section.get("coverage_report_sha256").and_then(Value::as_str) == Some(coverage_digest.as_str())
                    && section.get("schema_field_count").and_then(Value::as_u64) == schema_count
                    && section.get("covered_schema_field_count").and_then(Value::as_u64) == covered_count
                    && schema_count.is_some_and(|count| count > 0)
                    && schema_count == covered_count
            });
            let source_record_shape_ok = source_record.is_object();
            status_and_contract && hashes_match && no_coverage_gaps && observed_audit_clean && manifest_section_matches && source_record_shape_ok
        }
        _ => false,
    };

    let independent_schema_coverage_ok = match (
        artifacts.get(0).and_then(Option::as_ref),
        artifacts.get(1).and_then(Option::as_ref),
        artifacts.get(2).and_then(Option::as_ref),
        artifacts.get(3).and_then(Option::as_ref),
    ) {
        (Some(source_record), Some(source_schema), Some(adapter_map), Some(coverage)) => {
            independently_audit_schema_coverage(source_schema, source_record, adapter_map, coverage)
        }
        _ => false,
    };

    let transform_declarations_ok = match (manifest.as_ref(), artifacts.get(2).and_then(Option::as_ref)) {
        (Some(manifest), Some(adapter_map)) => {
            let mut found = BTreeSet::new();
            let mappings = adapter_map.get("mappings").and_then(Value::as_array);
            let mut valid = mappings.is_some();
            if let Some(mappings) = mappings {
                for mapping in mappings {
                    if mapping.get("disposition").and_then(Value::as_str) == Some("transform") {
                        if let Some(id) = mapping.get("transform_id").and_then(Value::as_str).filter(|id| !id.trim().is_empty()) {
                            found.insert(id.to_owned());
                        } else {
                            valid = false;
                        }
                    }
                }
            }
            let expected: Vec<Value> = found.into_iter().map(Value::String).collect();
            valid && manifest.get("declared_transform_ids") == Some(&Value::Array(expected))
        }
        _ => false,
    };

    let projection_input_link_ok = match (
        artifacts.get(4).and_then(Option::as_ref),
        artifacts.get(5).and_then(Option::as_ref),
    ) {
        (Some(projection), Some(input)) => {
            projection.get("status").and_then(Value::as_str) == Some("projection_created")
                && projection.get("projection") == Some(input)
        }
        _ => false,
    };

    let semantic_metadata_ok = match (manifest.as_ref(), artifacts.get(6).and_then(Option::as_ref)) {
        (Some(manifest), Some(report)) => {
            manifest.get("profile") == report.get("profile")
                && manifest.get("comparison_rule_id") == report.get("comparison_rule_id")
                && manifest.get("validator_version") == report.get("validator_version")
                && report.get("decision_authority").and_then(Value::as_str) == Some("diagnostic_only")
                && report.get("quality_release_authority").and_then(Value::as_str) == Some("none")
                && report.get("equipment_control_authority").and_then(Value::as_str) == Some("none")
                && report.get("origin_authentication").and_then(Value::as_str) == Some("not_performed")
        }
        _ => false,
    };

    let replay_binding_ok = match (
        artifacts.get(5).and_then(Option::as_ref),
        artifacts.get(6).and_then(Option::as_ref),
        artifacts.get(7).and_then(Option::as_ref),
    ) {
        (Some(input_value), Some(report_value), Some(envelope)) => {
            let binding = envelope.get("artifact_binding");
            let exact_envelope_shape = envelope.as_object().is_some_and(|object| {
                object.len() == 4
                    && object.contains_key("envelope_contract")
                    && object.contains_key("artifact_binding")
                    && object.contains_key("report")
                    && object.contains_key("limitations")
            });
            let exact_binding_shape = binding.and_then(Value::as_object).is_some_and(|object| {
                object.len() == 5
                    && object.contains_key("digest_algorithm")
                    && object.contains_key("digest")
                    && object.contains_key("digest_scope")
                    && object.contains_key("canonicalization")
                    && object.contains_key("byte_length")
            });
            exact_envelope_shape
                && exact_binding_shape
                && envelope.get("envelope_contract").and_then(Value::as_str) == Some("omsi-manufacturing-replay-envelope/0.1")
                && binding.and_then(|item| item.get("digest_algorithm")).and_then(Value::as_str) == Some("sha256")
                && binding.and_then(|item| item.get("digest_scope")).and_then(Value::as_str) == Some("exact_input_bytes")
                && binding.and_then(|item| item.get("canonicalization")).and_then(Value::as_str) == Some("none")
                && binding.and_then(|item| item.get("digest")).and_then(Value::as_str) == Some(sha256_hex(artifact_bytes[5]).as_str())
                && binding.and_then(|item| item.get("byte_length")).and_then(Value::as_u64) == Some(artifact_bytes[5].len() as u64)
                && envelope.get("report") == Some(report_value)
                && input_value.is_object()
                && envelope.get("limitations").and_then(Value::as_array).is_some_and(|items| items.len() >= 5)
        }
        _ => false,
    };

    let graph_ok = manifest.as_ref().is_some_and(|manifest| {
        manifest.get("activities") == Some(&expected_activities())
            && manifest.get("edges") == Some(&expected_edges())
            && manifest.get("limitations").and_then(Value::as_array).is_some_and(|items| items.len() >= 7)
    });

    let mut checks = Vec::new();
    checks.push(check(
        "manifest-json",
        manifest_json_ok,
        "manifest parses as JSON",
        "manifest is not valid JSON",
    ));
    checks.push(check(
        "manifest-contract",
        contract_ok,
        "manifest uses the supported unsigned v0.2 contract",
        "contract, graph profile, status or authority is unsupported",
    ));
    checks.push(check(
        "manifest-shape",
        manifest_shape_ok,
        "manifest has the exact expected top-level key set and raw-byte digest profile",
        "manifest shape contains missing/extra fields or unsupported digest semantics",
    ));

    for (index, (artifact_id, _role)) in ARTIFACTS.iter().enumerate() {
        checks.push(check(
            &format!("artifact-bytes-{artifact_id}"),
            artifact_matches[index],
            "artifact identity, role, SHA-256 and byte length match the supplied exact bytes",
            "artifact is missing, malformed, reordered, or its exact-byte digest/length mismatches",
        ));
    }
    checks.push(check(
        "schema-coverage-bindings",
        schema_coverage_ok,
        "coverage report is bound to the exact source record/schema/map and its manifest references match",
        "coverage report, schema/record/map digests, coverage counts or gap lists are inconsistent",
    ));
    checks.push(check(
        "independent-source-schema-coverage",
        independent_schema_coverage_ok,
        "a separate implementation derives the closed-schema leaf inventory, requiredness, record structure, and mapping coverage",
        "independently derived source-schema coverage differs from the report, or the schema uses an unsupported construct",
    ));
    checks.push(check(
        "declared-transform-ids",
        transform_declarations_ok,
        "manifest transform declarations match the adapter-map declarations",
        "manifest transform IDs differ from the adapter map or a transform lacks an ID",
    ));
    checks.push(check(
        "projection-input-link",
        projection_input_link_ok,
        "semantic input equals the projection payload",
        "projection is not created or semantic input differs from projection payload",
    ));
    checks.push(check(
        "semantic-report-metadata",
        semantic_metadata_ok,
        "manifest profile/rule/build metadata matches a diagnostic-only report",
        "report metadata differs or diagnostic-only authority fields are missing/changed",
    ));
    checks.push(check(
        "replay-envelope-binding",
        replay_binding_ok,
        "envelope binds exact semantic-input bytes and carries the supplied semantic report",
        "envelope structure, byte digest/length, embedded report or limitations mismatch",
    ));
    checks.push(check(
        "provenance-graph",
        graph_ok,
        "five expected activities and fourteen expected used/generated edges match the v0.2 graph",
        "activity graph or disclosed limitation set differs from the expected v0.2 profile",
    ));

    let all_pass = checks
        .iter()
        .all(|entry| entry.get("result").and_then(Value::as_str) == Some("pass"));

    json!({
        "verifier_contract": VERIFIER_CONTRACT,
        "status": if all_pass { "artifact_bindings_verified" } else { "rejected" },
        "authority": "artifact_binding_and_graph_consistency_only",
        "manifest_sha256": sha256_hex(manifest_bytes),
        "checks": checks,
        "limitations": LIMITATIONS
    })
}


/// Independently derives the source schema's field inventory and checks
/// that the coverage report's summary agrees with the actual schema/map/record.
/// This deliberately supports a closed-object subset rather than all JSON Schema.
fn independently_audit_schema_coverage(
    source_schema: &Value,
    source_record: &Value,
    adapter_map: &Value,
    coverage: &Value,
) -> bool {
    let mut fields = BTreeMap::new();
    if !derive_schema_fields(source_schema, "", true, &mut fields) || fields.is_empty() {
        return false;
    }
    if fields.len() > 100_000 || !record_matches_closed_schema(source_schema, source_record) {
        return false;
    }

    if adapter_map.get("adapter_contract").and_then(Value::as_str)
        != Some("omsi-manufacturing-adapter-map/0.1")
        || adapter_map.get("adapter_id").and_then(Value::as_str)
            .is_none_or(|value| value.trim().is_empty())
        || adapter_map.get("source_format").and_then(Value::as_str)
            .is_none_or(|value| value.trim().is_empty())
    {
        return false;
    }
    let Some(mappings) = adapter_map.get("mappings").and_then(Value::as_array) else {
        return false;
    };
    if mappings.len() > 100_000 {
        return false;
    }
    let mut declarations: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for mapping in mappings {
        let Some(path) = mapping.get("source_path").and_then(Value::as_str) else {
            return false;
        };
        let presence = match mapping.get("presence").and_then(Value::as_str) {
            None => "required",
            Some("required") => "required",
            Some("optional") => "optional",
            Some(_) => return false,
        };
        if !fields.contains_key(path) || !is_json_pointer(path) {
            return false;
        }
        let disposition = match mapping.get("disposition").and_then(Value::as_str) {
            Some("preserve") => {
                mapping.get("target_path").and_then(Value::as_str)
                    .is_some_and(is_json_pointer)
            }
            Some("transform") => {
                mapping.get("target_path").and_then(Value::as_str)
                    .is_some_and(is_json_pointer)
                    && mapping.get("transform_id").and_then(Value::as_str)
                        .is_some_and(|value| !value.trim().is_empty())
            }
            Some("drop" | "unmapped") => {
                mapping.get("reason").and_then(Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty())
            }
            _ => false,
        };
        if !disposition {
            return false;
        }
        declarations.entry(path.to_owned()).or_default().push(presence.to_owned());
    }

    for (path, schema_required) in &fields {
        let expected = if *schema_required { "required" } else { "optional" };
        match declarations.get(path) {
            Some(values) if values.len() == 1 && values[0] == expected => {}
            _ => return false,
        }
    }
    if declarations.len() != fields.len() {
        return false;
    }

    let schema_count = coverage.get("schema_field_count").and_then(Value::as_u64);
    let covered_count = coverage.get("covered_schema_field_count").and_then(Value::as_u64);
    let expected_unobserved: Vec<Value> = fields
        .iter()
        .filter(|(path, _)| source_record.pointer(path).is_none())
        .map(|(path, required)| json!({
            "source_path": path,
            "schema_presence": if *required { "required" } else { "optional" }
        }))
        .collect();
    let unobserved_fields_match =
        coverage.get("schema_paths_not_observed_in_record") == Some(&Value::Array(expected_unobserved));
    let clean_lists = [
        "schema_paths_not_mapped",
        "mapping_paths_outside_schema",
        "presence_mismatches",
        "duplicate_source_mappings",
        "required_schema_paths_not_observed_in_record",
        "schema_errors",
        "record_structure_errors",
    ]
    .iter()
    .all(|key| coverage.get(*key).and_then(Value::as_array).is_some_and(Vec::is_empty));

    coverage.get("coverage_contract").and_then(Value::as_str)
        == Some(COVERAGE_CONTRACT)
        && coverage.get("status").and_then(Value::as_str) == Some("schema_coverage_complete")
        && coverage.get("authority").and_then(Value::as_str) == Some("schema_field_path_coverage_only")
        && schema_count == Some(fields.len() as u64)
        && covered_count == Some(fields.len() as u64)
        && coverage.get("observed_field_accounting_status").and_then(Value::as_str)
            == Some("complete_accounting")
        && coverage.get("observed_silent_loss_count").and_then(Value::as_u64) == Some(0)
        && unobserved_fields_match
        && clean_lists
}

fn is_json_pointer(pointer: &str) -> bool {
    if pointer.is_empty() || !pointer.starts_with('/') {
        return false;
    }
    let bytes = pointer.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'~' {
            if index + 1 >= bytes.len() || !matches!(bytes[index + 1], b'0' | b'1') {
                return false;
            }
            index += 2;
        } else {
            index += 1;
        }
    }
    true
}

fn derive_schema_fields(
    schema: &Value,
    path: &str,
    ancestors_required: bool,
    fields: &mut BTreeMap<String, bool>,
) -> bool {
    let Some(object) = schema.as_object() else {
        return false;
    };
    if path.is_empty() && object.get("type").and_then(Value::as_str) != Some("object") {
        return false;
    }
    const UNSUPPORTED: &[&str] = &[
        "$ref",
        "$dynamicRef",
        "allOf",
        "anyOf",
        "oneOf",
        "not",
        "if",
        "patternProperties",
        "propertyNames",
        "dependentSchemas",
        "unevaluatedProperties",
        "unevaluatedItems",
    ];
    if UNSUPPORTED.iter().any(|key| object.contains_key(*key)) {
        return false;
    }
    let Some(type_name) = object.get("type").and_then(Value::as_str) else {
        return false;
    };

    match type_name {
        "object" => {
            if object.get("additionalProperties") != Some(&Value::Bool(false)) {
                return false;
            }
            let Some(properties) = object.get("properties").and_then(Value::as_object) else {
                return false;
            };
            if properties.is_empty() {
                return false;
            }
            let Some(required_names) = declared_required_names(object, properties) else {
                return false;
            };
            for (name, child_schema) in properties {
                let escaped = name.replace('~', "~0").replace('/', "~1");
                let child_path = format!("{path}/{escaped}");
                let child_required =
                    ancestors_required && required_names.contains(name.as_str());
                if !derive_schema_fields(child_schema, &child_path, child_required, fields) {
                    return false;
                }
                if fields.len() > 100_000 {
                    return false;
                }
            }
            true
        }
        "array" => {
            if !object.contains_key("items") {
                return false;
            }
            fields.insert(path.to_owned(), ancestors_required).is_none()
        }
        "string" | "number" | "integer" | "boolean" | "null" => {
            fields.insert(path.to_owned(), ancestors_required).is_none()
        }
        _ => false,
    }
}

fn declared_required_names<'a>(
    schema: &'a serde_json::Map<String, Value>,
    properties: &'a serde_json::Map<String, Value>,
) -> Option<BTreeSet<&'a str>> {
    match schema.get("required") {
        None => Some(BTreeSet::new()),
        Some(Value::Array(items)) => {
            let mut required = BTreeSet::new();
            for item in items {
                let name = item.as_str()?;
                if !properties.contains_key(name) || !required.insert(name) {
                    return None;
                }
            }
            Some(required)
        }
        Some(_) => None,
    }
}

fn record_matches_closed_schema(schema: &Value, record: &Value) -> bool {
    let Some(object) = schema.as_object() else {
        return false;
    };
    let Some(type_name) = object.get("type").and_then(Value::as_str) else {
        return false;
    };
    match type_name {
        "object" => {
            if object.get("additionalProperties") != Some(&Value::Bool(false)) {
                return false;
            }
            let Some(properties) = object.get("properties").and_then(Value::as_object) else {
                return false;
            };
            let Some(record_object) = record.as_object() else {
                return false;
            };
            let Some(required) = declared_required_names(object, properties) else {
                return false;
            };
            if required.iter().any(|name| !record_object.contains_key(*name))
                || record_object.keys().any(|name| !properties.contains_key(name))
            {
                return false;
            }
            properties.iter().all(|(name, child_schema)| match record_object.get(name) {
                Some(value) => record_matches_closed_schema(child_schema, value),
                None => true,
            })
        }
        "array" => record.is_array() && object.contains_key("items"),
        "string" => record.is_string(),
        "integer" => record.as_number().is_some_and(|number| {
            number.as_i64().is_some() || number.as_u64().is_some()
        }),
        "number" => record.is_number(),
        "boolean" => record.is_boolean(),
        "null" => record.is_null(),
        _ => false,
    }
}

fn expected_activities() -> Value {
    json!([
        {
            "activity_id": "schema-coverage-audit",
            "activity_type": "schema_coverage_audit",
            "used": ["source-record", "source-schema", "adapter-map"],
            "generated": ["schema-coverage-report"]
        },
        {
            "activity_id": "adapter-projection",
            "activity_type": "adapter_projection",
            "used": ["source-record", "adapter-map", "schema-coverage-report"],
            "generated": ["projection-report"]
        },
        {
            "activity_id": "semantic-input-extraction",
            "activity_type": "semantic_input_extraction",
            "used": ["projection-report"],
            "generated": ["semantic-input"]
        },
        {
            "activity_id": "semantic-evaluation",
            "activity_type": "semantic_evaluation",
            "used": ["semantic-input"],
            "generated": ["semantic-report"]
        },
        {
            "activity_id": "replay-envelope-creation",
            "activity_type": "replay_envelope_creation",
            "used": ["semantic-input"],
            "generated": ["replay-envelope"]
        }
    ])
}

fn expected_edges() -> Value {
    json!([
        {"from": "schema-coverage-audit", "to": "source-record", "relationship": "used"},
        {"from": "schema-coverage-audit", "to": "source-schema", "relationship": "used"},
        {"from": "schema-coverage-audit", "to": "adapter-map", "relationship": "used"},
        {"from": "schema-coverage-report", "to": "schema-coverage-audit", "relationship": "wasGeneratedBy"},
        {"from": "adapter-projection", "to": "source-record", "relationship": "used"},
        {"from": "adapter-projection", "to": "adapter-map", "relationship": "used"},
        {"from": "adapter-projection", "to": "schema-coverage-report", "relationship": "used"},
        {"from": "projection-report", "to": "adapter-projection", "relationship": "wasGeneratedBy"},
        {"from": "semantic-input-extraction", "to": "projection-report", "relationship": "used"},
        {"from": "semantic-input", "to": "semantic-input-extraction", "relationship": "wasGeneratedBy"},
        {"from": "semantic-evaluation", "to": "semantic-input", "relationship": "used"},
        {"from": "semantic-report", "to": "semantic-evaluation", "relationship": "wasGeneratedBy"},
        {"from": "replay-envelope-creation", "to": "semantic-input", "relationship": "used"},
        {"from": "replay-envelope", "to": "replay-envelope-creation", "relationship": "wasGeneratedBy"}
    ])
}

fn check(id: &str, passed: bool, pass_reason: &str, fail_reason: &str) -> Value {
    json!({
        "id": id,
        "result": if passed { "pass" } else { "fail" },
        "explanation": if passed { pass_reason } else { fail_reason }
    })
}

fn parse_json(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    serde_json::from_slice(bytes)
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}


#[cfg(test)]
mod tests {
    use super::{sha256_hex, verify_bundle, ARTIFACTS, COVERAGE_CONTRACT, LIMITATIONS};
    use serde_json::{json, Value};
    use std::collections::BTreeSet;

    fn fixture_bundle() -> (Vec<u8>, [Vec<u8>; 8]) {
        let source_record = br#"{"value":"ok"}"#.to_vec();
        let source_schema = br#"{"type":"object","required":["value"],"properties":{"value":{"type":"string"},"optional_note":{"type":"string"}},"additionalProperties":false}"#.to_vec();
        let adapter_map = br#"{"adapter_contract":"omsi-manufacturing-adapter-map/0.1","adapter_id":"independent-fixture/0.1","source_format":"independent-fixture-json/0.1","mappings":[{"source_path":"/value","disposition":"transform","target_path":"/value","transform_id":"qms-decimal-normalization/0.1"},{"source_path":"/optional_note","presence":"optional","disposition":"preserve","target_path":"/optional_note"}]}"#.to_vec();
        let coverage = json!({
            "coverage_contract": COVERAGE_CONTRACT,
            "status": "schema_coverage_complete",
            "authority": "schema_field_path_coverage_only",
            "adapter_map_sha256": sha256_hex(&adapter_map),
            "source_schema_sha256": sha256_hex(&source_schema),
            "source_record_sha256": sha256_hex(&source_record),
            "schema_field_count": 2,
            "covered_schema_field_count": 2,
            "schema_paths_not_mapped": [],
            "mapping_paths_outside_schema": [],
            "presence_mismatches": [],
            "duplicate_source_mappings": [],
            "schema_paths_not_observed_in_record": [
                {"source_path":"/optional_note","schema_presence":"optional"}
            ],
            "required_schema_paths_not_observed_in_record": [],
            "observed_field_accounting_status": "complete_accounting",
            "observed_silent_loss_count": 0,
            "schema_errors": [],
            "record_structure_errors": [],
            "limitations": [
                "not full JSON Schema",
                "not vendor conformance",
                "does not prove semantic transform correctness",
                "does not authenticate source",
                "does not authorize release"
            ]
        });
        let coverage_bytes = serde_json::to_vec(&coverage).unwrap();

        let projection = br#"{"projection_contract":"omsi-manufacturing-adapter-projection/0.1","status":"projection_created","authority":"adapter_projection_only","projection":{"value":"ok"},"limitations":["synthetic fixture"]}"#.to_vec();
        let semantic_input = br#"{"value":"ok"}"#.to_vec();
        let semantic_report = br#"{"profile":"manufacturing-asset-operations/0.1-incubation","comparison_rule_id":"nominal-point-estimate-range/0.1","validator_version":"0.1.0","decision_authority":"diagnostic_only","quality_release_authority":"none","equipment_control_authority":"none","origin_authentication":"not_performed","outcome":"reject"}"#.to_vec();
        let replay_envelope = serde_json::to_vec(&json!({
            "envelope_contract":"omsi-manufacturing-replay-envelope/0.1",
            "artifact_binding":{
                "digest_algorithm":"sha256",
                "digest":sha256_hex(&semantic_input),
                "digest_scope":"exact_input_bytes",
                "canonicalization":"none",
                "byte_length":semantic_input.len()
            },
            "report":serde_json::from_slice::<Value>(&semantic_report).unwrap(),
            "limitations":[
                "unsigned",
                "does not authenticate source",
                "exact byte binding only",
                "does not prove measurement truth",
                "does not authorize release"
            ]
        })).unwrap();

        let artifact_bytes = [
            source_record,
            source_schema,
            adapter_map,
            coverage_bytes,
            projection,
            semantic_input,
            semantic_report,
            replay_envelope,
        ];
        let mut declared_transforms = BTreeSet::new();
        declared_transforms.insert("qms-decimal-normalization/0.1".to_owned());
        let ids: Vec<Value> = declared_transforms.into_iter().map(Value::String).collect();
        let profile_report: Value = serde_json::from_slice(&artifact_bytes[6]).unwrap();
        let manifest = json!({
            "manifest_contract":"omsi-manufacturing-provenance-manifest/0.2",
            "provenance_model":"w3c-prov-dm-aligned-subset/0.2",
            "status":"local_chain_consistent",
            "authority":"local_consistency_only",
            "digest_profile":{"algorithm":"sha256","scope":"exact_input_bytes","canonicalization":"none"},
            "profile":profile_report["profile"],
            "comparison_rule_id":profile_report["comparison_rule_id"],
            "validator_version":profile_report["validator_version"],
            "schema_coverage":{
                "contract":COVERAGE_CONTRACT,
                "status":"schema_coverage_complete",
                "schema_field_count":2,
                "covered_schema_field_count":2,
                "source_schema_sha256":sha256_hex(&artifact_bytes[1]),
                "coverage_report_sha256":sha256_hex(&artifact_bytes[3])
            },
            "declared_transform_ids":ids,
            "artifacts": ARTIFACTS.iter().enumerate().map(|(index,(artifact_id,role))| {
                json!({
                    "artifact_id":artifact_id,
                    "role":role,
                    "sha256":sha256_hex(&artifact_bytes[index]),
                    "byte_length":artifact_bytes[index].len()
                })
            }).collect::<Vec<_>>(),
            "activities":super::expected_activities(),
            "edges":super::expected_edges(),
            "limitations":LIMITATIONS
        });
        (serde_json::to_vec(&manifest).unwrap(), artifact_bytes)
    }

    #[test]
    fn valid_bundle_has_all_byte_bindings_and_graph_checks() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let (manifest, artifacts) = fixture_bundle();
        let result = verify_bundle(
            &manifest,
            [
                artifacts[0].as_slice(),
                artifacts[1].as_slice(),
                artifacts[2].as_slice(),
                artifacts[3].as_slice(),
                artifacts[4].as_slice(),
                artifacts[5].as_slice(),
                artifacts[6].as_slice(),
                artifacts[7].as_slice(),
            ],
        );
        assert_eq!(result["status"], "artifact_bindings_verified", "{result:#}");
        assert_eq!(result["authority"], "artifact_binding_and_graph_consistency_only");
        assert_eq!(result["checks"].as_array().unwrap().len(), 18);
        assert!(result["checks"].as_array().unwrap().iter().all(|check| check["result"] == "pass"));
    }

    #[test]
    fn forged_coverage_counts_fail_independent_recomputation_even_with_updated_hashes() {
        let (mut manifest_bytes, artifacts) = fixture_bundle();
        let mut manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        let mut coverage: Value = serde_json::from_slice(&artifacts[3]).unwrap();
        coverage["schema_field_count"] = json!(17);
        coverage["covered_schema_field_count"] = json!(17);
        let forged_coverage_bytes = serde_json::to_vec(&coverage).unwrap();

        manifest["artifacts"][3]["sha256"] = json!(sha256_hex(&forged_coverage_bytes));
        manifest["artifacts"][3]["byte_length"] = json!(forged_coverage_bytes.len());
        manifest["schema_coverage"]["coverage_report_sha256"] =
            json!(sha256_hex(&forged_coverage_bytes));
        manifest["schema_coverage"]["schema_field_count"] = json!(17);
        manifest["schema_coverage"]["covered_schema_field_count"] = json!(17);
        manifest_bytes = serde_json::to_vec(&manifest).unwrap();

        let mut changed = artifacts.clone();
        changed[3] = forged_coverage_bytes;
        let result = verify_bundle(
            &manifest_bytes,
            [
                changed[0].as_slice(),
                changed[1].as_slice(),
                changed[2].as_slice(),
                changed[3].as_slice(),
                changed[4].as_slice(),
                changed[5].as_slice(),
                changed[6].as_slice(),
                changed[7].as_slice(),
            ],
        );

        assert_eq!(result["status"], "rejected", "{result:#}");
        assert!(result["checks"].as_array().unwrap().iter().any(|check| {
            check["id"] == "schema-coverage-bindings" && check["result"] == "pass"
        }), "the forged report should remain byte-bound and internally self-consistent");
        assert!(result["checks"].as_array().unwrap().iter().any(|check| {
            check["id"] == "independent-source-schema-coverage" && check["result"] == "fail"
        }), "independent recomputation must detect the forged field count");
    }

    #[test]
    fn invalid_mapping_disposition_fails_independent_schema_coverage() {
        let (manifest, artifacts) = fixture_bundle();
        let _ = manifest;
        let source_record: Value = serde_json::from_slice(&artifacts[0]).unwrap();
        let source_schema: Value = serde_json::from_slice(&artifacts[1]).unwrap();
        let mut adapter_map: Value = serde_json::from_slice(&artifacts[2]).unwrap();
        let coverage: Value = serde_json::from_slice(&artifacts[3]).unwrap();
        adapter_map["adapter_contract"] = json!("omsi-manufacturing-adapter-map/9.0");
        assert!(!super::independently_audit_schema_coverage(
            &source_schema,
            &source_record,
            &adapter_map,
            &coverage,
        ));
        adapter_map["adapter_contract"] = json!("omsi-manufacturing-adapter-map/0.1");
        adapter_map["mappings"][0]["disposition"] = json!("preserve-or-guess");
        assert!(!super::independently_audit_schema_coverage(
            &source_schema,
            &source_record,
            &adapter_map,
            &coverage,
        ));
        adapter_map["mappings"][0]["disposition"] = json!("transform");
        adapter_map["mappings"][0]["target_path"] = json!("/invalid~2pointer");
        assert!(!super::independently_audit_schema_coverage(
            &source_schema,
            &source_record,
            &adapter_map,
            &coverage,
        ));
    }

    #[test]
    fn omitted_optional_path_inventory_is_rejected_even_when_report_hashes_match() {
        let (mut manifest_bytes, artifacts) = fixture_bundle();
        let mut manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        let mut coverage: Value = serde_json::from_slice(&artifacts[3]).unwrap();
        coverage["schema_paths_not_observed_in_record"] = json!([]);
        let forged_coverage_bytes = serde_json::to_vec(&coverage).unwrap();

        manifest["artifacts"][3]["sha256"] = json!(sha256_hex(&forged_coverage_bytes));
        manifest["artifacts"][3]["byte_length"] = json!(forged_coverage_bytes.len());
        manifest["schema_coverage"]["coverage_report_sha256"] =
            json!(sha256_hex(&forged_coverage_bytes));
        manifest_bytes = serde_json::to_vec(&manifest).unwrap();

        let mut changed = artifacts.clone();
        changed[3] = forged_coverage_bytes;
        let result = verify_bundle(
            &manifest_bytes,
            [
                changed[0].as_slice(),
                changed[1].as_slice(),
                changed[2].as_slice(),
                changed[3].as_slice(),
                changed[4].as_slice(),
                changed[5].as_slice(),
                changed[6].as_slice(),
                changed[7].as_slice(),
            ],
        );

        assert_eq!(result["status"], "rejected", "{result:#}");
        assert!(result["checks"].as_array().unwrap().iter().any(|check| {
            check["id"] == "schema-coverage-bindings" && check["result"] == "pass"
        }), "byte bindings should pass when hashes are recalculated");
        assert!(result["checks"].as_array().unwrap().iter().any(|check| {
            check["id"] == "independent-source-schema-coverage" && check["result"] == "fail"
        }), "independent recomputation must restore the absent optional path");
    }

    #[test]
    fn every_artifact_byte_mutation_is_rejected() {
        let (manifest, artifacts) = fixture_bundle();
        for index in 0..ARTIFACTS.len() {
            let mut changed = artifacts.clone();
            changed[index].push(b' ');
            let result = verify_bundle(
                &manifest,
                [
                    changed[0].as_slice(),
                    changed[1].as_slice(),
                    changed[2].as_slice(),
                    changed[3].as_slice(),
                    changed[4].as_slice(),
                    changed[5].as_slice(),
                    changed[6].as_slice(),
                    changed[7].as_slice(),
                ],
            );
            assert_eq!(result["status"], "rejected", "artifact {index} unexpectedly verified: {result:#}");
        }
    }

    #[test]
    fn altered_graph_or_coverage_reference_is_rejected() {
        let (mut manifest_bytes, artifacts) = fixture_bundle();
        let mut manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        manifest["edges"][0]["to"] = json!("semantic-input");
        manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let result = verify_bundle(
            &manifest_bytes,
            [
                artifacts[0].as_slice(),
                artifacts[1].as_slice(),
                artifacts[2].as_slice(),
                artifacts[3].as_slice(),
                artifacts[4].as_slice(),
                artifacts[5].as_slice(),
                artifacts[6].as_slice(),
                artifacts[7].as_slice(),
            ],
        );
        assert_eq!(result["status"], "rejected");
        assert!(result["checks"].as_array().unwrap().iter()
            .any(|check| check["id"] == "provenance-graph" && check["result"] == "fail"));
    }

    #[test]
    fn manifest_without_supported_contract_is_rejected() {
        let (mut manifest_bytes, artifacts) = fixture_bundle();
        let mut manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        manifest["manifest_contract"] = json!("omsi-manufacturing-provenance-manifest/9.0");
        manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let result = verify_bundle(
            &manifest_bytes,
            [
                artifacts[0].as_slice(),
                artifacts[1].as_slice(),
                artifacts[2].as_slice(),
                artifacts[3].as_slice(),
                artifacts[4].as_slice(),
                artifacts[5].as_slice(),
                artifacts[6].as_slice(),
                artifacts[7].as_slice(),
            ],
        );
        assert_eq!(result["status"], "rejected");
    }
}
