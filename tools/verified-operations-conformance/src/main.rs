use serde_json::Value;
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process;

use verified_operations_conformance::{validate_corpus, validate_evidence_bundle, validate_schema};

fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    let raw = fs::read_to_string(path)?;
    let value = serde_json::from_str(&raw)?;
    Ok(value)
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 {
        return Err(
            "usage: verified-operations-conformance <schema.json> <synthetic-corpus-or-evidence-bundle.json>".into(),
        );
    }

    let schema_path = Path::new(&args[0]);
    let corpus_path = Path::new(&args[1]);
    let schema = read_json(schema_path)?;
    let corpus = read_json(corpus_path)?;

    validate_schema(&schema).map_err(|errors| -> Box<dyn Error> {
        format!(
            "{} is not a valid JSON Schema Draft 2020-12 schema: {}",
            schema_path.display(),
            errors.join("; ")
        )
        .into()
    })?;

    let is_evidence_bundle = corpus.get("evidence").and_then(Value::as_array).is_some();
    let count = if is_evidence_bundle {
        validate_evidence_bundle(&schema, &corpus)
    } else {
        validate_corpus(&schema, &corpus)
    }
    .map_err(|errors| -> Box<dyn Error> {
        format!(
            "{} failed conformance: {}",
            corpus_path.display(),
            errors.join("; ")
        )
        .into()
    })?;

    if is_evidence_bundle {
        println!(
            "VERIFIED OPERATIONS EVIDENCE BUNDLE PASS: {count} evidence records validated; no runtime actions executed"
        );
    } else {
        println!(
            "VERIFIED OPERATIONS CONTRACT PASS: {count} synthetic scenarios validated; no runtime actions executed"
        );
    }
    if let Ok(source_sha) = env::var("SOURCE_HEAD_SHA") {
        println!("SOURCE_HEAD_SHA={source_sha}");
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("VERIFIED OPERATIONS CONTRACT FAIL: {error}");
        process::exit(1);
    }
}
