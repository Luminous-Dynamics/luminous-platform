use crate::adapter::audit_adapter_map;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const COVERAGE_CONTRACT: &str = "omsi-manufacturing-adapter-schema-coverage/0.1";
const MAX_SCHEMA_FIELDS: usize = 100_000;
const LIMITATIONS: &[&str] = &[
    "This report checks field-path coverage against a deliberately restricted JSON Schema subset; it is not full JSON Schema validation or a claim of source-system conformance.",
    "Arrays are atomic field paths. This audit does not enumerate or validate object fields within array items.",
    "The source record is audited for observed leaf paths, but this command does not claim that every source-record value satisfies all schema value constraints.",
    "A complete field-path inventory proves neither that target mappings are semantically correct nor that a transform preserves physical meaning.",
    "A schema field being mapped does not prove that the source producer or physical measurement is authentic.",
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct SchemaField {
    path: String,
    required: bool,
}

/// Compare adapter mappings with all scalar/array leaf paths declared by a
/// closed-object JSON Schema subset. Unknown schema constructs fail closed so
/// the report cannot imply complete coverage over an unenumerated field space.
pub fn audit_adapter_schema_coverage(
    adapter_map: &Value,
    adapter_map_bytes: &[u8],
    source_schema: &Value,
    source_schema_bytes: &[u8],
    source_record: &Value,
    source_record_bytes: &[u8],
) -> Value {
    let observation_audit = audit_adapter_map(
        adapter_map,
        adapter_map_bytes,
        source_record,
        source_record_bytes,
    );
    let inventory = schema_inventory(source_schema);

    let Some(fields) = inventory.fields else {
        return json!({
            "coverage_contract": COVERAGE_CONTRACT,
            "status": "unsupported_source_schema",
            "authority": "schema_field_path_coverage_only",
            "adapter_map_sha256": sha256_hex(adapter_map_bytes),
            "source_schema_sha256": sha256_hex(source_schema_bytes),
            "source_record_sha256": sha256_hex(source_record_bytes),
            "schema_field_count": 0,
            "covered_schema_field_count": 0,
            "schema_paths_not_mapped": [],
            "mapping_paths_outside_schema": [],
            "presence_mismatches": [],
            "schema_paths_not_observed_in_record": [],
            "required_schema_paths_not_observed_in_record": [],
            "observed_field_accounting_status": observation_audit.get("status").cloned().unwrap_or(Value::Null),
            "observed_silent_loss_count": observation_audit.get("silent_loss_count").cloned().unwrap_or(Value::Null),
            "schema_errors": inventory.errors,
            "limitations": LIMITATIONS
        });
    };

    let fields_by_path: BTreeMap<String, bool> = fields
        .iter()
        .map(|field| (field.path.clone(), field.required))
        .collect();
    let mut mapping_paths: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut presence_mismatches = Vec::new();

    let mappings = adapter_map.get("mappings").and_then(Value::as_array);
    if let Some(mappings) = mappings {
        for (index, mapping) in mappings.iter().enumerate() {
            let Some(object) = mapping.as_object() else {
                continue;
            };
            let Some(path) = object.get("source_path").and_then(Value::as_str) else {
                continue;
            };
            let declared_presence = match object.get("presence").and_then(Value::as_str) {
                None => "required",
                Some(value) => value,
            };
            mapping_paths
                .entry(path.to_owned())
                .or_default()
                .push(declared_presence.to_owned());

            if let Some(schema_required) = fields_by_path.get(path) {
                let expected = if *schema_required { "required" } else { "optional" };
                if declared_presence != expected {
                    presence_mismatches.push(json!({
                        "mapping_index": index,
                        "source_path": path,
                        "declared_presence": declared_presence,
                        "schema_presence": expected,
                        "reason": "mapping_presence_differs_from_source_schema"
                    }));
                }
            }
        }
    }

    let schema_paths_not_mapped: Vec<Value> = fields_by_path
        .iter()
        .filter(|(path, _)| !mapping_paths.contains_key(*path))
        .map(|(path, required)| {
            json!({
                "source_path": path,
                "schema_presence": if *required { "required" } else { "optional" }
            })
        })
        .collect();

    let mapping_paths_outside_schema: Vec<Value> = mapping_paths
        .iter()
        .filter(|(path, _)| !fields_by_path.contains_key(*path))
        .map(|(path, presences)| {
            json!({
                "source_path": path,
                "mapping_declaration_count": presences.len(),
                "reason": "mapping_path_not_declared_by_source_schema"
            })
        })
        .collect();

    let schema_paths_not_observed_in_record: Vec<Value> = fields_by_path
        .iter()
        .filter(|(path, _)| source_record.pointer(path).is_none())
        .map(|(path, required)| {
            json!({
                "source_path": path,
                "schema_presence": if *required { "required" } else { "optional" }
            })
        })
        .collect();

    let required_schema_paths_not_observed_in_record: Vec<String> = fields_by_path
        .iter()
        .filter(|(path, required)| **required && source_record.pointer(path).is_none())
        .map(|(path, _)| path.clone())
        .collect();

    let duplicate_source_mappings: Vec<Value> = mapping_paths
        .iter()
        .filter(|(_, presences)| presences.len() != 1)
        .map(|(path, presences)| {
            json!({
                "source_path": path,
                "mapping_declaration_count": presences.len(),
                "reason": "source_path_must_have_exactly_one_mapping"
            })
        })
        .collect();

    let schema_coverage_complete = schema_paths_not_mapped.is_empty()
        && mapping_paths_outside_schema.is_empty()
        && presence_mismatches.is_empty()
        && duplicate_source_mappings.is_empty()
        && observation_audit.get("status").and_then(Value::as_str) == Some("complete_accounting");

    let schema_field_count = fields_by_path.len();
    let covered_schema_field_count = fields_by_path
        .iter()
        .filter(|(path, schema_required)| {
            let expected_presence = if **schema_required { "required" } else { "optional" };
            mapping_paths.get(*path).is_some_and(|declared| {
                declared.len() == 1 && declared[0] == expected_presence
            })
        })
        .count();
    let status = if schema_coverage_complete {
        "schema_coverage_complete"
    } else if observation_audit.get("status").and_then(Value::as_str) == Some("invalid_contract") {
        "invalid_contract"
    } else {
        "schema_coverage_incomplete"
    };

    json!({
        "coverage_contract": COVERAGE_CONTRACT,
        "status": status,
        "authority": "schema_field_path_coverage_only",
        "adapter_id": adapter_map.get("adapter_id").cloned().unwrap_or(Value::Null),
        "source_format": adapter_map.get("source_format").cloned().unwrap_or(Value::Null),
        "adapter_map_sha256": sha256_hex(adapter_map_bytes),
        "source_schema_sha256": sha256_hex(source_schema_bytes),
        "source_record_sha256": sha256_hex(source_record_bytes),
        "schema_field_count": schema_field_count,
        "covered_schema_field_count": covered_schema_field_count,
        "schema_paths_not_mapped": schema_paths_not_mapped,
        "mapping_paths_outside_schema": mapping_paths_outside_schema,
        "presence_mismatches": presence_mismatches,
        "duplicate_source_mappings": duplicate_source_mappings,
        "schema_paths_not_observed_in_record": schema_paths_not_observed_in_record,
        "required_schema_paths_not_observed_in_record": required_schema_paths_not_observed_in_record,
        "observed_field_accounting_status": observation_audit.get("status").cloned().unwrap_or(Value::Null),
        "observed_silent_loss_count": observation_audit.get("silent_loss_count").cloned().unwrap_or(Value::Null),
        "schema_errors": inventory.errors,
        "limitations": LIMITATIONS
    })
}

struct SchemaInventory {
    fields: Option<Vec<SchemaField>>,
    errors: Vec<Value>,
}

fn schema_inventory(schema: &Value) -> SchemaInventory {
    let mut fields = Vec::new();
    let mut errors = Vec::new();
    if let Err(error) = walk_schema(schema, "", true, &mut fields, &mut errors) {
        errors.push(error);
    }
    if fields.len() > MAX_SCHEMA_FIELDS {
        errors.push(json!({
            "field": "$",
            "reason": "schema_field_count_exceeds_limit",
            "maximum": MAX_SCHEMA_FIELDS
        }));
    }
    fields.sort_by(|left, right| left.path.cmp(&right.path));
    if errors.is_empty() {
        SchemaInventory {
            fields: Some(fields),
            errors,
        }
    } else {
        SchemaInventory {
            fields: None,
            errors,
        }
    }
}

fn walk_schema(
    schema: &Value,
    path: &str,
    required: bool,
    fields: &mut Vec<SchemaField>,
    errors: &mut Vec<Value>,
) -> Result<(), Value> {
    let Some(object) = schema.as_object() else {
        return Err(schema_error(path, "schema_node_must_be_an_object"));
    };

    if path.is_empty() && object.get("type").and_then(Value::as_str) != Some("object") {
        return Err(schema_error(path, "schema_root_must_be_an_object"));
    }

    for unsupported in [
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
    ] {
        if object.contains_key(unsupported) {
            return Err(schema_error(path, "unsupported_schema_composition_or_dynamic_fields"));
        }
    }

    let Some(type_name) = object.get("type").and_then(Value::as_str) else {
        return Err(schema_error(path, "schema_type_must_be_one_string"));
    };

    match type_name {
        "object" => {
            if object.get("additionalProperties") != Some(&Value::Bool(false)) {
                return Err(schema_error(path, "object_must_set_additionalProperties_false"));
            }
            let Some(properties) = object.get("properties").and_then(Value::as_object) else {
                return Err(schema_error(path, "object_must_declare_properties"));
            };
            if properties.is_empty() {
                return Err(schema_error(path, "empty_object_has_no_auditable_leaf_inventory"));
            }
            let required_names: BTreeSet<&str> = match object.get("required") {
                None => BTreeSet::new(),
                Some(Value::Array(items)) => {
                    let mut names = BTreeSet::new();
                    for item in items {
                        let Some(name) = item.as_str() else {
                            return Err(schema_error(path, "required_must_contain_only_strings"));
                        };
                        if !properties.contains_key(name) {
                            return Err(schema_error(path, "required_name_not_declared_in_properties"));
                        }
                        if !names.insert(name) {
                            return Err(schema_error(path, "duplicate_required_name"));
                        }
                    }
                    names
                }
                Some(_) => return Err(schema_error(path, "required_must_be_an_array")),
            };

            for (name, child) in properties {
                let escaped = name.replace('~', "~0").replace('/', "~1");
                let child_path = format!("{path}/{escaped}");
                let child_required = required && required_names.contains(name.as_str());
                walk_schema(child, &child_path, child_required, fields, errors)?;
                if fields.len() > MAX_SCHEMA_FIELDS {
                    return Err(json!({
                        "field": child_path,
                        "reason": "schema_field_count_exceeds_limit",
                        "maximum": MAX_SCHEMA_FIELDS
                    }));
                }
            }
            Ok(())
        }
        "array" => {
            if !object.contains_key("items") {
                return Err(schema_error(path, "array_must_declare_items"));
            }
            fields.push(SchemaField {
                path: path.to_owned(),
                required,
            });
            Ok(())
        }
        "string" | "number" | "integer" | "boolean" | "null" => {
            fields.push(SchemaField {
                path: path.to_owned(),
                required,
            });
            Ok(())
        }
        _ => Err(schema_error(path, "unsupported_schema_type")),
    }
}

fn schema_error(path: &str, reason: &str) -> Value {
    json!({
        "field": if path.is_empty() { "/" } else { path },
        "reason": reason
    })
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
    use super::audit_adapter_schema_coverage;
    use jsonschema::draft202012;
    use serde_json::{json, Value};

    const MAP: &str = include_str!(
        "../../../docs/standards/omsi/adapters/0.1/examples/mes-optional-map.json"
    );
    const RECORD: &str = include_str!(
        "../../../docs/standards/omsi/adapters/0.1/examples/mes-optional-record.json"
    );
    const SCHEMA: &str = include_str!(
        "../../../docs/standards/omsi/adapters/0.1/examples/mes-optional-record.schema.json"
    );

    fn fixture() -> (Value, Value, Value) {
        (
            serde_json::from_str(MAP).expect("map fixture parses"),
            serde_json::from_str(SCHEMA).expect("source schema parses"),
            serde_json::from_str(RECORD).expect("source record parses"),
        )
    }

    #[test]
    fn source_schema_fixture_is_meta_valid_and_sample_conforms() {
        let (_, schema, record) = fixture();
        assert!(draft202012::meta::is_valid(&schema));
        let validator = draft202012::new(&schema).expect("source schema compiles");
        assert!(validator.is_valid(&record), "sample should conform to schema: {record:#}");
    }

    #[test]
    fn optional_absent_fields_are_covered_from_schema_not_only_sample_data() {
        let (map, schema, record) = fixture();
        let report = audit_adapter_schema_coverage(
            &map, MAP.as_bytes(), &schema, SCHEMA.as_bytes(), &record, RECORD.as_bytes(),
        );
        assert_eq!(report["status"], "schema_coverage_complete", "{report:#}");
        assert_eq!(report["schema_field_count"], 3);
        assert_eq!(report["covered_schema_field_count"], 3);
        let report_schema: Value = serde_json::from_str(include_str!(
            "../schema/adapter-schema-coverage-report.schema.json"
        ))
        .expect("coverage report schema parses");
        assert!(draft202012::meta::is_valid(&report_schema));
        let report_validator = draft202012::new(&report_schema)
            .expect("coverage report schema compiles");
        assert!(report_validator.is_valid(&report), "report failed schema: {report:#}");
        assert_eq!(report["schema_paths_not_mapped"], json!([]));
        assert_eq!(report["presence_mismatches"], json!([]));
        assert_eq!(report["schema_paths_not_observed_in_record"].as_array().unwrap().len(), 2);
        assert_eq!(report["required_schema_paths_not_observed_in_record"], json!([]));
    }

    #[test]
    fn unmapped_optional_schema_field_blocks_coverage_even_when_absent() {
        let (mut map, schema, record) = fixture();
        map["mappings"].as_array_mut().unwrap().pop();
        let report = audit_adapter_schema_coverage(
            &map, &serde_json::to_vec(&map).unwrap(), &schema, SCHEMA.as_bytes(), &record, RECORD.as_bytes(),
        );
        assert_eq!(report["status"], "schema_coverage_incomplete");
        assert_eq!(report["schema_paths_not_mapped"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn presence_contract_must_match_schema_requiredness() {
        let (mut map, schema, record) = fixture();
        map["mappings"][1]["presence"] = json!("required");
        let map_bytes = serde_json::to_vec(&map).unwrap();
        let report = audit_adapter_schema_coverage(
            &map, &map_bytes, &schema, SCHEMA.as_bytes(), &record, RECORD.as_bytes(),
        );
        assert_eq!(report["status"], "schema_coverage_incomplete");
        assert_eq!(report["presence_mismatches"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn open_objects_and_schema_composition_fail_closed() {
        let (map, mut schema, record) = fixture();
        schema.as_object_mut().unwrap().remove("additionalProperties");
        let map_bytes = MAP.as_bytes();
        let report = audit_adapter_schema_coverage(
            &map, map_bytes, &schema, &serde_json::to_vec(&schema).unwrap(), &record, RECORD.as_bytes(),
        );
        assert_eq!(report["status"], "unsupported_source_schema");
        assert_eq!(report["schema_errors"][0]["reason"], "object_must_set_additionalProperties_false");

        let (_, mut schema, _) = fixture();
        schema["$ref"] = json!("#/$defs/fields");
        let schema_bytes = serde_json::to_vec(&schema).unwrap();
        let report = audit_adapter_schema_coverage(
            &map, map_bytes, &schema, &schema_bytes, &record, RECORD.as_bytes(),
        );
        assert_eq!(report["status"], "unsupported_source_schema");
    }

    #[test]
    fn pointer_escaping_preserves_slash_and_tilde_keys() {
        let map = json!({
            "adapter_contract":"omsi-manufacturing-adapter-map/0.1",
            "adapter_id":"pointer-test/0.1",
            "source_format":"pointer-test-json/0.1",
            "mappings":[
                {"source_path":"/a~1b","disposition":"preserve","target_path":"/target"},
                {"source_path":"/a~0b","disposition":"preserve","target_path":"/target2"}
            ]
        });
        let schema = json!({
            "type":"object","additionalProperties":false,
            "required":["a/b","a~b"],
            "properties":{
                "a/b":{"type":"string"},
                "a~b":{"type":"string"}
            }
        });
        let record = json!({"a/b":"slash","a~b":"tilde"});
        let map_bytes = serde_json::to_vec(&map).unwrap();
        let schema_bytes = serde_json::to_vec(&schema).unwrap();
        let record_bytes = serde_json::to_vec(&record).unwrap();
        let report = audit_adapter_schema_coverage(
            &map, &map_bytes, &schema, &schema_bytes, &record, &record_bytes,
        );
        assert_eq!(report["status"], "schema_coverage_complete", "{report:#}");
        assert_eq!(report["schema_field_count"], 2);
    }
}
