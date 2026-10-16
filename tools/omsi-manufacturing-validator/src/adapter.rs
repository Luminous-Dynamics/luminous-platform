use omsi_manufacturing_validator::normalize_decimal_literal;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const ADAPTER_MAP_CONTRACT: &str = "omsi-manufacturing-adapter-map/0.1";
const AUDIT_REPORT_CONTRACT: &str = "omsi-manufacturing-adapter-audit/0.1";
const MAX_ADAPTER_FIELDS: usize = 100_000;

#[derive(Clone, Debug)]
struct Mapping {
    disposition: String,
    target_path: Option<String>,
    transform_id: Option<String>,
    reason: Option<String>,
}

/// Audit that every observed source-record leaf has exactly one explicit
/// adapter disposition. This is a mapping-accounting check, not a proof that
/// a transform is semantically correct or that the resulting OMSI input passes.
pub fn audit_adapter_map(
    contract: &Value,
    contract_bytes: &[u8],
    source_record: &Value,
    source_record_bytes: &[u8],
) -> Value {
    let mut errors: Vec<Value> = Vec::new();
    let Some(root) = contract.as_object() else {
        return invalid_report(
            contract_bytes,
            source_record_bytes,
            vec![json!({"field":"$", "reason":"expected_object"})],
        );
    };

    check_object_shape(
        root,
        "$",
        &["adapter_contract", "adapter_id", "source_format", "mappings"],
        &["description"],
        &mut errors,
    );
    if root.contains_key("description")
        && !root.get("description").is_some_and(Value::is_string)
    {
        errors.push(json!({
            "field":"description",
            "reason":"expected_string"
        }));
    }
    if root.get("adapter_contract").and_then(Value::as_str) != Some(ADAPTER_MAP_CONTRACT) {
        errors.push(json!({
            "field":"adapter_contract",
            "reason":"unsupported_adapter_map_contract",
            "expected": ADAPTER_MAP_CONTRACT
        }));
    }
    for field in ["adapter_id", "source_format"] {
        if !root
            .get(field)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
        {
            errors.push(json!({
                "field":field,
                "reason":"expected_nonempty_string"
            }));
        }
    }

    let mapping_values = root.get("mappings").and_then(Value::as_array);
    if mapping_values.map_or(true, |items| items.is_empty()) {
        errors.push(json!({
            "field":"mappings",
            "reason":"expected_nonempty_array"
        }));
    }
    if mapping_values.is_some_and(|items| items.len() > MAX_ADAPTER_FIELDS) {
        errors.push(json!({
            "field":"mappings",
            "reason":"mapping_count_exceeds_limit",
            "maximum":MAX_ADAPTER_FIELDS
        }));
    }

    let mut mappings: BTreeMap<String, Mapping> = BTreeMap::new();

    if let Some(items) = mapping_values.filter(|items| items.len() <= MAX_ADAPTER_FIELDS) {
        for (index, item) in items.iter().enumerate() {
            let path = format!("mappings[{index}]");
            let Some(object) = item.as_object() else {
                errors.push(json!({"field":path,"reason":"expected_object"}));
                continue;
            };
            check_object_shape(
                object,
                &path,
                &["source_path", "disposition"],
                &["target_path", "transform_id", "reason"],
                &mut errors,
            );

            let source_path = object.get("source_path").and_then(Value::as_str);
            let disposition = object.get("disposition").and_then(Value::as_str);
            if !source_path.is_some_and(valid_pointer) {
                errors.push(json!({
                    "field":format!("{path}.source_path"),
                    "reason":"expected_valid_absolute_json_pointer"
                }));
                continue;
            }
            let source_path = source_path.unwrap_or_default().to_owned();
            if mappings.contains_key(&source_path) {
                errors.push(json!({
                    "field":format!("{path}.source_path"),
                    "source_path":source_path,
                    "reason":"duplicate_source_mapping"
                }));
                continue;
            }

            let target_path = optional_nonempty_string(
                object,
                "target_path",
                &format!("{path}.target_path"),
                &mut errors,
            );
            let transform_id = optional_nonempty_string(
                object,
                "transform_id",
                &format!("{path}.transform_id"),
                &mut errors,
            );
            let reason = optional_nonempty_string(
                object,
                "reason",
                &format!("{path}.reason"),
                &mut errors,
            );
            match disposition {
                Some("preserve") => {
                    if target_path.as_deref().map_or(true, |value| !valid_pointer(value)) {
                        errors.push(json!({
                            "field":format!("{path}.target_path"),
                            "reason":"preserve_requires_valid_target_path"
                        }));
                    }
                    if transform_id.is_some() {
                        errors.push(json!({
                            "field":format!("{path}.transform_id"),
                            "reason":"preserve_must_not_declare_transform_id"
                        }));
                    }
                }
                Some("transform") => {
                    if target_path.as_deref().map_or(true, |value| !valid_pointer(value)) {
                        errors.push(json!({
                            "field":format!("{path}.target_path"),
                            "reason":"transform_requires_valid_target_path"
                        }));
                    }
                    if transform_id.is_none() {
                        errors.push(json!({
                            "field":format!("{path}.transform_id"),
                            "reason":"transform_requires_named_transform_id"
                        }));
                    }
                }
                Some("drop" | "unmapped") => {
                    if object.contains_key("target_path") || object.contains_key("transform_id") {
                        errors.push(json!({
                            "field":path,
                            "reason":"drop_or_unmapped_must_not_declare_target_or_transform"
                        }));
                    }
                    if reason.is_none() {
                        errors.push(json!({
                            "field":format!("{path}.reason"),
                            "reason":"drop_or_unmapped_requires_reason"
                        }));
                    }
                }
                _ => {
                    errors.push(json!({
                        "field":format!("{path}.disposition"),
                        "reason":"unknown_disposition",
                        "allowed":["preserve","transform","drop","unmapped"]
                    }));
                    continue;
                }
            }
            mappings.insert(
                source_path,
                Mapping {
                    disposition: disposition.unwrap_or_default().to_owned(),
                    target_path,
                    transform_id,
                    reason,
                },
            );
        }
    }

    let Some(source_object) = source_record.as_object() else {
        errors.push(json!({"field":"source_record","reason":"expected_object"}));
        return invalid_report(contract_bytes, source_record_bytes, errors);
    };
    if source_object.is_empty() {
        errors.push(json!({"field":"source_record","reason":"empty_source_record"}));
    }

    let mut source_paths = Vec::new();
    for (key, value) in source_object {
        if !collect_leaf_paths(
            value,
            &format!("/{}", pointer_escape(key)),
            &mut source_paths,
            MAX_ADAPTER_FIELDS,
        ) {
            errors.push(json!({
                "field":"source_record",
                "reason":"source_field_count_exceeds_limit",
                "maximum":MAX_ADAPTER_FIELDS
            }));
            return invalid_report(contract_bytes, source_record_bytes, errors);
        }
    }
    source_paths.sort();

    if !errors.is_empty() {
        let mut report = invalid_report(contract_bytes, source_record_bytes, errors);
        report["adapter_id"] = json!(root.get("adapter_id").and_then(Value::as_str));
        report["source_format"] = json!(root.get("source_format").and_then(Value::as_str));
        report["source_field_count"] = json!(source_paths.len());
        return report;
    }

    let source_path_set: BTreeSet<&str> = source_paths.iter().map(String::as_str).collect();
    let mapping_paths_not_observed: Vec<String> = mappings
        .keys()
        .filter(|path| !source_path_set.contains(path.as_str()))
        .cloned()
        .collect();

    let mut fields = Vec::with_capacity(source_paths.len());
    let mut counts = BTreeMap::from([
        ("preserve".to_owned(), 0usize),
        ("transform".to_owned(), 0usize),
        ("drop".to_owned(), 0usize),
        ("unmapped".to_owned(), 0usize),
    ]);
    let mut unclassified_paths = Vec::new();

    for source_path in &source_paths {
        match mappings.get(source_path) {
            Some(mapping) => {
                *counts.entry(mapping.disposition.clone()).or_insert(0) += 1;
                let mut field = Map::new();
                field.insert("source_path".into(), json!(source_path));
                field.insert("disposition".into(), json!(mapping.disposition));
                insert_optional(&mut field, "target_path", mapping.target_path.as_deref());
                insert_optional(&mut field, "transform_id", mapping.transform_id.as_deref());
                insert_optional(&mut field, "reason", mapping.reason.as_deref());
                fields.push(Value::Object(field));
            }
            None => {
                unclassified_paths.push(source_path.clone());
                fields.push(json!({
                    "source_path":source_path,
                    "disposition":"unclassified",
                    "reason":"no_mapping_for_observed_source_field"
                }));
            }
        }
    }

    let complete = unclassified_paths.is_empty();
    json!({
        "audit_contract": AUDIT_REPORT_CONTRACT,
        "status": if complete { "complete_accounting" } else { "incomplete_accounting" },
        "authority": "adapter_mapping_accounting_only",
        "adapter_id": root.get("adapter_id"),
        "source_format": root.get("source_format"),
        "binding": {
            "digest_algorithm":"sha256",
            "contract_digest":sha256_hex(contract_bytes),
            "contract_digest_scope":"exact_contract_bytes",
            "source_record_digest":sha256_hex(source_record_bytes),
            "source_record_digest_scope":"exact_source_record_bytes",
            "canonicalization":"none"
        },
        "source_field_count": source_paths.len(),
        "accounted_field_count": source_paths.len() - unclassified_paths.len(),
        "preserved_count": counts["preserve"],
        "transformed_count": counts["transform"],
        "dropped_count": counts["drop"],
        "explicitly_unmapped_count": counts["unmapped"],
        "unclassified_field_count": unclassified_paths.len(),
        "silent_loss_count": unclassified_paths.len(),
        "unclassified_paths": unclassified_paths,
        "mapping_paths_not_observed_in_record": mapping_paths_not_observed,
        "fields": fields,
        "limitations": [
            "This report checks declared mapping coverage for one exact source-record artifact; audit all representative records and source-schema fields to find optional paths not present in this sample.",
            "A transform identifier documents an intended transformation but does not execute or prove that transformation.",
            "Complete mapping accounting is not proof of semantic equivalence, source authenticity, manufacturing correctness, privacy authorization, or quality-release authority.",
            "The source record and adapter-map digests bind exact bytes, not canonicalized JSON."
        ]
    })
}


/// Apply a fully accounted adapter map using the small, versioned transform
/// registry below. Any unrecognized transform, target collision, or unmapped
/// source field blocks projection instead of silently producing a partial record.
pub fn apply_adapter_map(
    contract: &Value,
    contract_bytes: &[u8],
    source_record: &Value,
    source_record_bytes: &[u8],
) -> Value {
    let audit = audit_adapter_map(contract, contract_bytes, source_record, source_record_bytes);
    if audit.get("status").and_then(Value::as_str) != Some("complete_accounting") {
        return projection_blocked(
            audit,
            vec![json!({"field":"adapter_map","reason":"adapter_map_accounting_not_complete"})],
        );
    }
    if audit.get("explicitly_unmapped_count").and_then(Value::as_u64).unwrap_or(0) > 0 {
        return projection_blocked(
            audit,
            vec![json!({"field":"mappings","reason":"explicitly_unmapped_source_fields_block_projection"})],
        );
    }
    if audit
        .get("mapping_paths_not_observed_in_record")
        .and_then(Value::as_array)
        .is_some_and(|paths| !paths.is_empty())
    {
        return projection_blocked(
            audit,
            vec![json!({
                "field":"mappings",
                "reason":"declared_mapping_path_absent_from_this_source_record"
            })],
        );
    }

    let Some(root) = contract.as_object() else {
        return projection_blocked(
            audit,
            vec![json!({"field":"adapter_map","reason":"expected_object"})],
        );
    };
    let Some(mappings) = root.get("mappings").and_then(Value::as_array) else {
        return projection_blocked(
            audit,
            vec![json!({"field":"mappings","reason":"expected_array"})],
        );
    };

    let mut target_paths: Vec<String> = Vec::new();
    for (index, mapping) in mappings.iter().enumerate() {
        let Some(object) = mapping.as_object() else {
            return projection_blocked(
                audit,
                vec![json!({"field":format!("mappings[{index}]"),"reason":"expected_object"})],
            );
        };
        let disposition = object.get("disposition").and_then(Value::as_str).unwrap_or("");
        if !matches!(disposition, "preserve" | "transform") {
            continue;
        }
        let target = object.get("target_path").and_then(Value::as_str).unwrap_or("");
        if target_paths.iter().any(|existing| target_paths_conflict(existing, target)) {
            return projection_blocked(
                audit,
                vec![json!({
                    "field":format!("mappings[{index}].target_path"),
                    "target_path":target,
                    "reason":"target_path_collision_or_prefix_overlap"
                })],
            );
        }
        target_paths.push(target.to_owned());
    }

    let mut projection = Value::Object(Map::new());
    let mut errors: Vec<Value> = Vec::new();
    let mut applied_count = 0usize;
    let mut dropped_count = 0usize;

    for (index, mapping) in mappings.iter().enumerate() {
        let Some(object) = mapping.as_object() else {
            errors.push(json!({"field":format!("mappings[{index}]"),"reason":"expected_object"}));
            continue;
        };
        let source_path = object.get("source_path").and_then(Value::as_str).unwrap_or("");
        let disposition = object.get("disposition").and_then(Value::as_str).unwrap_or("");
        match disposition {
            "drop" => {
                dropped_count += 1;
            }
            "unmapped" => {
                errors.push(json!({
                    "field":source_path,
                    "reason":"explicitly_unmapped_source_field"
                }));
            }
            "preserve" | "transform" => {
                let Some(value) = get_source_pointer(source_record, source_path) else {
                    errors.push(json!({
                        "field":source_path,
                        "reason":"source_pointer_not_found_at_application_time"
                    }));
                    continue;
                };
                let mapped_value = if disposition == "preserve" {
                    Ok(value.clone())
                } else {
                    let transform_id = object.get("transform_id").and_then(Value::as_str).unwrap_or("");
                    run_transform(transform_id, value).map_err(|reason| {
                        json!({
                            "field":source_path,
                            "transform_id":transform_id,
                            "reason":reason
                        })
                    })
                };
                match mapped_value {
                    Ok(value) => {
                        let target_path = object.get("target_path").and_then(Value::as_str).unwrap_or("");
                        match write_target_pointer(&mut projection, target_path, value) {
                            Ok(()) => applied_count += 1,
                            Err(reason) => errors.push(json!({
                                "field":target_path,
                                "source_path":source_path,
                                "reason":reason
                            })),
                        }
                    }
                    Err(error) => errors.push(error),
                }
            }
            _ => errors.push(json!({
                "field":format!("mappings[{index}].disposition"),
                "reason":"unknown_disposition"
            })),
        }
    }

    if !errors.is_empty() {
        return projection_blocked(audit, errors);
    }

    json!({
        "projection_contract":"omsi-manufacturing-adapter-projection/0.1",
        "status":"projection_created",
        "authority":"adapter_projection_only",
        "adapter_id":root.get("adapter_id").and_then(Value::as_str),
        "source_format":root.get("source_format").and_then(Value::as_str),
        "applied_mapping_count":applied_count,
        "explicitly_dropped_count":dropped_count,
        "audit":audit,
        "projection":projection,
        "limitations":[
            "Projection applies only the small versioned transformation registry; it does not certify conformance with an external vendor schema.",
            "A created projection is not evidence of source authenticity, physical measurement truth, calibration validity, product conformity, or release authority.",
            "Any source fields that were intentionally dropped are listed by the nested audit; their omission still requires data-owner authorization and privacy review.",
            "The emitted projection is not signed and has no public-receipt or equipment-control authority."
        ]
    })
}

fn projection_blocked(audit: Value, errors: Vec<Value>) -> Value {
    json!({
        "projection_contract":"omsi-manufacturing-adapter-projection/0.1",
        "status":"projection_blocked",
        "authority":"adapter_projection_only",
        "projection":null,
        "audit":audit,
        "errors":errors,
        "limitations":[
            "No partial projection is emitted when adapter accounting, transform execution, scope of mapping, or target insertion fails.",
            "The adapter audit and projection do not authenticate the source record or establish manufacturing correctness."
        ]
    })
}

fn run_transform(transform_id: &str, value: &Value) -> Result<Value, Value> {
    match transform_id {
        "qms-decimal-normalization/0.1" => {
            let text = value.as_str().ok_or_else(|| json!({
                "reason":"decimal_transform_requires_string_value"
            }))?;
            normalize_decimal_literal(text)
                .map(Value::String)
                .map_err(|reason| json!({"reason":"decimal_transform_rejected_input","detail":reason}))
        }
        "qms-unit-code-crosswalk/0.1" => {
            let code = value.as_str().ok_or_else(|| json!({
                "reason":"unit_crosswalk_requires_string_value"
            }))?;
            let normalized = match code {
                "millimeter" | "millimetre" | "mm" => "mm",
                "meter" | "metre" | "m" => "m",
                "micrometer" | "micrometre" | "um" => "um",
                "degree Celsius" | "degrees Celsius" | "degC" | "Cel" => "Cel",
                _ => return Err(json!({
                    "reason":"unit_code_not_in_crosswalk",
                    "source_unit_code":code,
                    "crosswalk_id":transform_id
                })),
            };
            Ok(Value::String(normalized.to_owned()))
        }
        _ => Err(json!({
            "reason":"transform_id_not_registered",
            "transform_id":transform_id
        })),
    }
}

fn pointer_tokens(pointer: &str) -> Option<Vec<String>> {
    if !valid_pointer(pointer) {
        return None;
    }
    Some(
        pointer[1..]
            .split('/')
            .map(|token| token.replace("~1", "/").replace("~0", "~"))
            .collect(),
    )
}

fn get_source_pointer<'a>(source: &'a Value, pointer: &str) -> Option<&'a Value> {
    let tokens = pointer_tokens(pointer)?;
    let mut current = source;
    for token in tokens {
        current = current.as_object()?.get(&token)?;
    }
    Some(current)
}

fn target_paths_conflict(left: &str, right: &str) -> bool {
    let Some(left_tokens) = pointer_tokens(left) else {
        return true;
    };
    let Some(right_tokens) = pointer_tokens(right) else {
        return true;
    };
    let shared = left_tokens.len().min(right_tokens.len());
    left_tokens[..shared] == right_tokens[..shared]
}

fn write_target_pointer(root: &mut Value, pointer: &str, value: Value) -> Result<(), String> {
    let tokens = pointer_tokens(pointer).ok_or_else(|| "invalid_target_pointer".to_owned())?;
    let Some(root_object) = root.as_object_mut() else {
        return Err("projection_root_is_not_object".to_owned());
    };
    insert_nested_object(root_object, &tokens, value)
}

fn insert_nested_object(
    object: &mut Map<String, Value>,
    tokens: &[String],
    value: Value,
) -> Result<(), String> {
    let Some((head, tail)) = tokens.split_first() else {
        return Err("empty_target_pointer".to_owned());
    };
    if tail.is_empty() {
        if object.contains_key(head) {
            return Err("target_key_already_exists".to_owned());
        }
        object.insert(head.clone(), value);
        return Ok(());
    }

    if !object.contains_key(head) {
        object.insert(head.clone(), json!({}));
    }
    let Some(child) = object.get_mut(head) else {
        return Err("target_parent_not_found".to_owned());
    };
    let Some(child_object) = child.as_object_mut() else {
        return Err("target_parent_is_not_object".to_owned());
    };
    insert_nested_object(child_object, tail, value)
}

fn invalid_report(contract_bytes: &[u8], source_record_bytes: &[u8], errors: Vec<Value>) -> Value {
    json!({
        "audit_contract":AUDIT_REPORT_CONTRACT,
        "status":"invalid_contract",
        "authority":"adapter_mapping_accounting_only",
        "binding":{
            "digest_algorithm":"sha256",
            "contract_digest":sha256_hex(contract_bytes),
            "contract_digest_scope":"exact_contract_bytes",
            "source_record_digest":sha256_hex(source_record_bytes),
            "source_record_digest_scope":"exact_source_record_bytes",
            "canonicalization":"none"
        },
        "errors":errors,
        "silent_loss_count":null,
        "limitations":[
            "A malformed adapter map cannot establish field accounting.",
            "Digest binding does not authenticate the adapter, source, or transformation."
        ]
    })
}

fn check_object_shape(
    object: &Map<String, Value>,
    path: &str,
    required: &[&str],
    optional: &[&str],
    errors: &mut Vec<Value>,
) {
    for key in required {
        if !object.contains_key(*key) {
            errors.push(json!({
                "field":format!("{path}.{key}"),
                "reason":"required_field_missing"
            }));
        }
    }
    for key in object.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            errors.push(json!({
                "field":format!("{path}.{key}"),
                "reason":"unrecognized_field"
            }));
        }
    }
}

fn optional_nonempty_string(
    object: &Map<String, Value>,
    key: &str,
    path: &str,
    errors: &mut Vec<Value>,
) -> Option<String> {
    match object.get(key) {
        None => None,
        Some(Value::String(value)) if !value.trim().is_empty() => {
            Some(value.trim().to_owned())
        }
        Some(Value::String(_)) => {
            errors.push(json!({"field":path,"reason":"empty_string_not_allowed"}));
            None
        }
        Some(_) => {
            errors.push(json!({"field":path,"reason":"expected_nonempty_string"}));
            None
        }
    }
}

fn insert_optional(object: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        object.insert(key.to_owned(), json!(value));
    }
}

fn valid_pointer(pointer: &str) -> bool {
    if !pointer.starts_with('/') {
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

fn pointer_escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// Objects recurse to leaves; arrays are atomic fields because adapting an
/// array's item shape is a separate explicit transformation.
fn collect_leaf_paths(
    value: &Value,
    path: &str,
    output: &mut Vec<String>,
    maximum_fields: usize,
) -> bool {
    match value {
        Value::Object(object) if !object.is_empty() => {
            for (key, child) in object {
                if !collect_leaf_paths(
                    child,
                    &format!("{path}/{}", pointer_escape(key)),
                    output,
                    maximum_fields,
                ) {
                    return false;
                }
            }
            true
        }
        _ => {
            if output.len() >= maximum_fields {
                return false;
            }
            output.push(path.to_owned());
            true
        }
    }
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
    use super::{apply_adapter_map, audit_adapter_map, ADAPTER_MAP_CONTRACT};
    use serde_json::{json, Value};

    fn bytes(value: &Value) -> Vec<u8> {
        serde_json::to_vec(value).expect("test JSON must serialize")
    }

    fn audit(contract: Value, source: Value) -> Value {
        let contract_bytes = bytes(&contract);
        let source_bytes = bytes(&source);
        audit_adapter_map(&contract, &contract_bytes, &source, &source_bytes)
    }

    fn map(disposition: &str, extras: Value) -> Value {
        let mut entry = json!({
            "source_path":"/temperature",
            "disposition":disposition
        });
        if let (Some(target), Some(object)) = (extras.get("target_path"), entry.as_object_mut()) {
            object.insert("target_path".into(), target.clone());
        }
        if let (Some(transform), Some(object)) = (extras.get("transform_id"), entry.as_object_mut()) {
            object.insert("transform_id".into(), transform.clone());
        }
        if let (Some(reason), Some(object)) = (extras.get("reason"), entry.as_object_mut()) {
            object.insert("reason".into(), reason.clone());
        }
        entry
    }

    fn contract(mappings: Vec<Value>) -> Value {
        json!({
            "adapter_contract":ADAPTER_MAP_CONTRACT,
            "adapter_id":"synthetic-mes-flat/0.1",
            "source_format":"synthetic-mes-flat-json/0.1",
            "mappings":mappings
        })
    }

    fn apply(contract: Value, source: Value) -> Value {
        let contract_bytes = bytes(&contract);
        let source_bytes = bytes(&source);
        apply_adapter_map(&contract, &contract_bytes, &source, &source_bytes)
    }

    #[test]
    fn all_observed_source_fields_require_and_accept_explicit_dispositions() {
        let contract = contract(vec![
            map("preserve", json!({"target_path":"/measurement/value_decimal"})),
            json!({
                "source_path":"/operator_note",
                "disposition":"drop",
                "reason":"operator free text is not part of this projection"
            }),
        ]);
        let source = json!({"temperature":"72.4","operator_note":"fixture note"});
        let report = audit(contract, source);
        assert_eq!(report["status"], "complete_accounting");
        assert_eq!(report["source_field_count"], 2);
        assert_eq!(report["accounted_field_count"], 2);
        assert_eq!(report["preserved_count"], 1);
        assert_eq!(report["dropped_count"], 1);
        assert_eq!(report["silent_loss_count"], 0);
        assert_eq!(report["authority"], "adapter_mapping_accounting_only");
        assert_eq!(report["binding"]["canonicalization"], "none");
    }

    #[test]
    fn unclassified_observed_field_is_a_hard_accounting_failure() {
        let report = audit(
            contract(vec![map("preserve", json!({"target_path":"/measurement/value_decimal"}))]),
            json!({"temperature":"72.4","unit":"Cel"}),
        );
        assert_eq!(report["status"], "incomplete_accounting");
        assert_eq!(report["silent_loss_count"], 1);
        assert_eq!(report["unclassified_paths"][0], "/unit");
        assert_eq!(report["unclassified_field_count"], 1);
    }

    #[test]
    fn explicit_unmapped_is_not_silent_but_remains_visible() {
        let report = audit(
            contract(vec![
                map("preserve", json!({"target_path":"/measurement/value_decimal"})),
                json!({
                    "source_path":"/unit",
                    "disposition":"unmapped",
                    "reason":"source unit code has no agreed OMSI crosswalk yet"
                }),
            ]),
            json!({"temperature":"72.4","unit":"legacy-degree"}),
        );
        assert_eq!(report["status"], "complete_accounting");
        assert_eq!(report["explicitly_unmapped_count"], 1);
        assert_eq!(report["silent_loss_count"], 0);
    }

    #[test]
    fn transform_requires_a_named_rule_and_target() {
        let invalid = audit(
            contract(vec![map("transform", json!({"target_path":"/measurement/value_decimal"}))]),
            json!({"temperature":"72.4"}),
        );
        assert_eq!(invalid["status"], "invalid_contract");
        assert!(invalid["errors"].as_array().unwrap().iter().any(|entry| {
            entry["reason"] == "transform_requires_named_transform_id"
        }));
    }

    #[test]
    fn duplicate_source_mapping_is_rejected() {
        let invalid = audit(
            contract(vec![
                map("preserve", json!({"target_path":"/one"})),
                map("preserve", json!({"target_path":"/two"})),
            ]),
            json!({"temperature":"72.4"}),
        );
        assert_eq!(invalid["status"], "invalid_contract");
        assert!(invalid["errors"].as_array().unwrap().iter().any(|entry| {
            entry["reason"] == "duplicate_source_mapping"
        }));
    }

    #[test]
    fn json_pointer_escaping_is_part_of_the_field_identity() {
        let report = audit(
            contract(vec![json!({
                "source_path":"/a~1b/~0key",
                "disposition":"preserve",
                "target_path":"/measurement/value_decimal"
            })]),
            json!({"a/b":{"~key":"72.4"}}),
        );
        assert_eq!(report["status"], "complete_accounting");
        assert_eq!(report["fields"][0]["source_path"], "/a~1b/~0key");
    }

    #[test]
    fn leaf_collection_stops_at_the_configured_field_limit() {
        let source = json!({"a":1,"b":2,"c":3});
        let mut paths = Vec::new();
        assert!(!super::collect_leaf_paths(&source, "", &mut paths, 2));
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn empty_json_object_keys_are_preserved_by_pointer_escaping() {
        let report = audit(
            contract(vec![json!({
                "source_path":"//",
                "disposition":"preserve",
                "target_path":"/neutral/empty-key"
            })]),
            json!({"":{"":"value"}}),
        );
        assert_eq!(report["status"], "complete_accounting");
        assert_eq!(report["fields"][0]["source_path"], "//");
    }

    #[test]
    fn malformed_optional_properties_are_rejected_not_treated_as_absent() {
        let invalid = audit(
            contract(vec![json!({
                "source_path":"/temperature",
                "disposition":"drop",
                "reason":"intentionally omitted",
                "target_path":17
            })]),
            json!({"temperature":"72.4"}),
        );
        assert_eq!(invalid["status"], "invalid_contract");
        assert!(invalid["errors"].as_array().unwrap().iter().any(|entry| {
            entry["reason"] == "expected_nonempty_string"
        }));
    }

    #[test]
    fn mapping_accounting_does_not_claim_transformation_correctness() {
        let report = audit(
            contract(vec![json!({
                "source_path":"/status",
                "disposition":"transform",
                "target_path":"/source_quality_claim/raw_status",
                "transform_id":"status-normalization/0.1"
            })]),
            json!({"status":"GOOD"}),
        );
        assert_eq!(report["status"], "complete_accounting");
        assert_eq!(report["transformed_count"], 1);
        assert!(report["limitations"].as_array().unwrap().iter().any(|item| {
            item.as_str().is_some_and(|text| text.contains("does not execute or prove"))
        }));
    }
    #[test]
    fn qms_fixture_executes_named_decimal_and_unit_transforms() {
        let contract: Value = serde_json::from_str(include_str!(
            "../../../docs/standards/omsi/adapters/0.1/examples/qms-nested-map.json"
        )).expect("QMS map fixture parses");
        let source: Value = serde_json::from_str(include_str!(
            "../../../docs/standards/omsi/adapters/0.1/examples/qms-nested-record.json"
        )).expect("QMS record fixture parses");
        let contract_bytes = include_str!(
            "../../../docs/standards/omsi/adapters/0.1/examples/qms-nested-map.json"
        ).as_bytes();
        let source_bytes = include_str!(
            "../../../docs/standards/omsi/adapters/0.1/examples/qms-nested-record.json"
        ).as_bytes();

        let report = apply_adapter_map(&contract, contract_bytes, &source, source_bytes);
        assert_eq!(report["status"], "projection_created");
        assert_eq!(report["authority"], "adapter_projection_only");
        assert_eq!(report["projection"]["measurement"]["value_decimal"], "0.0072");
        assert_eq!(report["projection"]["measurement"]["unit_code"], "mm");
        assert_eq!(report["projection"]["source_quality_claim"]["raw_status"], "REVIEW");
        assert_eq!(report["audit"]["silent_loss_count"], 0);
        assert_eq!(report["audit"]["transformed_count"], 2);
        assert_eq!(report["applied_mapping_count"], 8);
    }

    #[test]
    fn mes_fixture_preserves_raw_status_and_surfaces_intentional_drop() {
        let contract: Value = serde_json::from_str(include_str!(
            "../../../docs/standards/omsi/adapters/0.1/examples/mes-flat-map.json"
        )).expect("MES map fixture parses");
        let source: Value = serde_json::from_str(include_str!(
            "../../../docs/standards/omsi/adapters/0.1/examples/mes-flat-record.json"
        )).expect("MES record fixture parses");
        let report = apply(contract, source);

        assert_eq!(report["status"], "projection_created");
        assert_eq!(report["projection"]["source_quality_claim"]["raw_status"], "Accepted");
        assert_eq!(report["explicitly_dropped_count"], 1);
        assert_eq!(report["audit"]["dropped_count"], 1);
        assert!(report["projection"].pointer("/operator_note").is_none());
    }

    #[test]
    fn explicit_unmapped_fields_block_projection_even_with_complete_accounting() {
        let report = apply(
            contract(vec![json!({
                "source_path":"/temperature",
                "disposition":"unmapped",
                "reason":"no agreed target"
            })]),
            json!({"temperature":"72.4"}),
        );
        assert_eq!(report["status"], "projection_blocked");
        assert_eq!(report["projection"], Value::Null);
        assert_eq!(report["audit"]["status"], "complete_accounting");
        assert_eq!(report["audit"]["explicitly_unmapped_count"], 1);
    }

    #[test]
    fn unknown_transform_blocks_projection_without_emitting_partial_output() {
        let report = apply(
            contract(vec![json!({
                "source_path":"/temperature",
                "disposition":"transform",
                "target_path":"/measurement/value_decimal",
                "transform_id":"unregistered-transform/0.1"
            })]),
            json!({"temperature":"72.4"}),
        );
        assert_eq!(report["status"], "projection_blocked");
        assert_eq!(report["projection"], Value::Null);
        assert!(report["errors"].as_array().unwrap().iter().any(|error| {
            error["reason"] == "transform_id_not_registered"
        }));
    }

    #[test]
    fn unit_outside_the_explicit_crosswalk_blocks_projection() {
        let report = apply(
            contract(vec![json!({
                "source_path":"/unit",
                "disposition":"transform",
                "target_path":"/measurement/unit_code",
                "transform_id":"qms-unit-code-crosswalk/0.1"
            })]),
            json!({"unit":"parsec"}),
        );
        assert_eq!(report["status"], "projection_blocked");
        assert_eq!(report["projection"], Value::Null);
        assert!(report["errors"].as_array().unwrap().iter().any(|error| {
            error["reason"] == "unit_code_not_in_crosswalk"
        }));
    }

    #[test]
    fn absent_declared_source_path_blocks_projection() {
        let report = apply(
            contract(vec![
                json!({
                    "source_path":"/temperature",
                    "disposition":"preserve",
                    "target_path":"/measurement/value_decimal"
                }),
                json!({
                    "source_path":"/optional_unit",
                    "disposition":"preserve",
                    "target_path":"/measurement/unit_code"
                }),
            ]),
            json!({"temperature":"72.4"}),
        );
        assert_eq!(report["status"], "projection_blocked");
        assert_eq!(report["projection"], Value::Null);
        assert_eq!(
            report["audit"]["mapping_paths_not_observed_in_record"][0],
            "/optional_unit"
        );
    }

    #[test]
    fn overlapping_target_paths_block_projection_before_writes() {
        let report = apply(
            contract(vec![
                json!({
                    "source_path":"/temperature",
                    "disposition":"preserve",
                    "target_path":"/measurement"
                }),
                json!({
                    "source_path":"/unit",
                    "disposition":"preserve",
                    "target_path":"/measurement/unit_code"
                }),
            ]),
            json!({"temperature":"72.4","unit":"Cel"}),
        );
        assert_eq!(report["status"], "projection_blocked");
        assert_eq!(report["projection"], Value::Null);
        assert!(report["errors"].as_array().unwrap().iter().any(|error| {
            error["reason"] == "target_path_collision_or_prefix_overlap"
        }));
    }

}
