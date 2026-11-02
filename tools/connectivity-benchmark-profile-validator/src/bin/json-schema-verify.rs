//! Generic offline Draft 2020-12 JSON Schema verifier.
//!
//! Remote references are rejected to prevent validation from loading network or
//! filesystem resources. A valid document is structural evidence only.

use jsonschema::draft202012;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process;

const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;

fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let file = File::open(path)
        .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut limited = file.take((MAX_INPUT_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    limited
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(format!(
            "{} exceeds the {} byte input limit",
            path.display(),
            MAX_INPUT_BYTES
        ));
    }
    Ok(bytes)
}

fn digest_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn collect_nonlocal_references(value: &Value, path: &str, errors: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if key == "$ref" || key == "$dynamicRef" {
                    match child.as_str() {
                        Some(reference) if reference.starts_with('#') => {}
                        Some(_) => errors.push(format!(
                            "{path}/{key}: only local fragment references are allowed"
                        )),
                        None => errors.push(format!("{path}/{key}: reference must be a string")),
                    }
                }
                collect_nonlocal_references(child, &format!("{path}/{key}"), errors);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_nonlocal_references(child, &format!("{path}/{index}"), errors);
            }
        }
        _ => {}
    }
}

fn validate(schema: &Value, document: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    collect_nonlocal_references(schema, "$schema", &mut errors);
    if !errors.is_empty() {
        return errors;
    }

    let meta = match jsonschema::meta::validator_for(schema) {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot build schema meta-validator: {error}")],
    };
    if !meta.evaluate(schema).flag().valid {
        return vec!["schema failed JSON Schema meta-schema validation".to_owned()];
    }

    let validator = match draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot build Draft 2020-12 validator: {error}")],
    };

    let mut errors: Vec<String> = validator
        .iter_errors(document)
        .map(|error| format!("schema: {error}"))
        .collect();
    errors.sort();
    errors.dedup();
    errors
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] == "--help" || args[1] == "-h" {
        eprintln!(
            "Usage: json-schema-verify <schema.json> <document.json>\n\
             Validates Draft 2020-12 structure with local references only.\n\
             A valid document does not prove execution, measurement truth or authorization."
        );
        process::exit(if args.len() == 2 { 0 } else { 64 });
    }

    let schema_path = Path::new(&args[1]);
    let document_path = Path::new(&args[2]);
    let schema_bytes = match read_bounded(schema_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    let document_bytes = match read_bounded(document_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };

    let schema: Value = match serde_json::from_slice(&schema_bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("schema is not valid JSON: {error}");
            process::exit(2);
        }
    };
    let document: Value = match serde_json::from_slice(&document_bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("document is not valid JSON: {error}");
            process::exit(2);
        }
    };

    let errors = validate(&schema, &document);
    let valid = errors.is_empty();
    let report = json!({
        "report_version": "luminous.offline-json-schema-validation-report.v1",
        "schema_sha256": digest_hex(&schema_bytes),
        "document_sha256": digest_hex(&document_bytes),
        "valid": valid,
        "validation_scope": "offline_draft_2020_12_schema_only",
        "remote_references_allowed": false,
        "execution_conducted": false,
        "measurement_truth_conveyed": false,
        "authorization_conveyed": false,
        "errors": errors
    });

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    if let Err(error) = serde_json::to_writer_pretty(&mut lock, &report) {
        eprintln!("cannot write validation report: {error}");
        process::exit(2);
    }
    if let Err(error) = lock.write_all(b"\n") {
        eprintln!("cannot terminate validation report: {error}");
        process::exit(2);
    }
    if !valid {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn multipath_fixtures() -> (Value, Value) {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../../schemas/lnf-multipath-scheduling-profile-v1.schema.json"
        ))
        .expect("checked-in multipath schema parses");
        let profile: Value = serde_json::from_str(include_str!(
            "../../../../profiles/lnf-multipath-scheduling-benchmark-v1.json"
        ))
        .expect("checked-in multipath profile parses");
        (schema, profile)
    }

    #[test]
    fn multipath_profile_passes_its_strict_schema() {
        let (schema, profile) = multipath_fixtures();
        let errors = validate(&schema, &profile);
        assert!(errors.is_empty(), "validation errors: {errors:#?}");
    }

    #[test]
    fn remote_schema_references_are_rejected() {
        let (mut schema, profile) = multipath_fixtures();
        schema["properties"]["status"]["$ref"] =
            serde_json::json!("https://example.invalid/remote.json");
        let errors = validate(&schema, &profile);
        assert!(errors.iter().any(|error| error.contains("only local fragment references")));
    }

    #[test]
    fn missing_scenario_is_rejected() {
        let (schema, mut profile) = multipath_fixtures();
        profile["scenario_ids"]
            .as_array_mut()
            .expect("scenario array")
            .pop();
        let errors = validate(&schema, &profile);
        assert!(!errors.is_empty());
    }
}
