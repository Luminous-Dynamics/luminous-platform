mod adapter;
mod benchmark;
mod replay;

use omsi_manufacturing_validator::{evaluate_input_value, validate_vector_document};
use adapter::audit_adapter_map;
use replay::{create_replay_envelope, verify_replay_envelope};
use serde_json::Value;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

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
        Some("validate-vectors") if args.len() == 3 => {
            let document = read_json(&args[2])?;
            let report = validate_vector_document(&document);
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|error| format!("failed to serialize report: {error}"))?
            );
            if report.get("status").and_then(Value::as_str) == Some("passed") {
                Ok(0)
            } else {
                Ok(1)
            }
        }
        Some("evaluate-input") if args.len() == 3 => {
            let input = read_json(&args[2])?;
            let report = evaluate_input_value(&input);
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|error| format!("failed to serialize report: {error}"))?
            );
            match report.get("outcome").and_then(Value::as_str) {
                Some("accepted_projection" | "accepted_projection_with_loss") => Ok(0),
                _ => Ok(1),
            }
        }
        Some("create-envelope") if args.len() == 3 => {
            let source = read_bytes(&args[2])?;
            let envelope = create_replay_envelope(&source)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&envelope)
                    .map_err(|error| format!("failed to serialize envelope: {error}"))?
            );
            Ok(0)
        }
        Some("verify-envelope") if args.len() == 4 => {
            let envelope = read_json(&args[2])?;
            let source = read_bytes(&args[3])?;
            let verification = verify_replay_envelope(&envelope, &source);
            println!(
                "{}",
                serde_json::to_string_pretty(&verification)
                    .map_err(|error| format!("failed to serialize verification: {error}"))?
            );
            if verification.get("status").and_then(Value::as_str) == Some("replay_consistent") {
                Ok(0)
            } else {
                Ok(1)
            }
        }
        Some("benchmark-vectors") if args.len() == 3 || args.len() == 4 => {
            let source = read_bytes(&args[2])?;
            let document: Value = serde_json::from_slice(&source)
                .map_err(|error| format!("invalid JSON in {}: {error}", args[2]))?;
            let iterations = args
                .get(3)
                .map(|value| {
                    value
                        .parse::<usize>()
                        .map_err(|error| format!("invalid iterations_per_case {value:?}: {error}"))
                })
                .transpose()?
                .unwrap_or(1_000);
            let report = benchmark::run(&document, &source, iterations)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|error| format!("failed to serialize benchmark: {error}"))?
            );
            Ok(0)
        }
        Some("audit-adapter-map") if args.len() == 4 => {
            let contract_bytes = read_bytes(&args[2])?;
            let source_bytes = read_bytes(&args[3])?;
            let contract: Value = serde_json::from_slice(&contract_bytes)
                .map_err(|error| format!("invalid JSON in {}: {error}", args[2]))?;
            let source_record: Value = serde_json::from_slice(&source_bytes)
                .map_err(|error| format!("invalid JSON in {}: {error}", args[3]))?;
            let report = audit_adapter_map(
                &contract,
                &contract_bytes,
                &source_record,
                &source_bytes,
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|error| format!("failed to serialize adapter audit: {error}"))?
            );
            if report.get("status").and_then(Value::as_str) == Some("complete_accounting") {
                Ok(0)
            } else {
                Ok(1)
            }
        }
        _ => Err(
            "usage: omsi-manufacturing-validator validate-vectors <vectors.json>\n       omsi-manufacturing-validator evaluate-input <input.json | ->\n       omsi-manufacturing-validator create-envelope <input.json | ->\n       omsi-manufacturing-validator verify-envelope <envelope.json> <input.json | ->\n       omsi-manufacturing-validator benchmark-vectors <vectors.json> [iterations_per_case>\n       omsi-manufacturing-validator audit-adapter-map <adapter-map.json> <source-record.json>".into(),
        ),
    }
}

fn read_json(path: &str) -> Result<Value, String> {
    let source = read_bytes(path)?;
    serde_json::from_slice(&source).map_err(|error| format!("invalid JSON in {path}: {error}"))
}

fn read_bytes(path: &str) -> Result<Vec<u8>, String> {
    if path == "-" {
        let mut source = Vec::new();
        io::stdin()
            .read_to_end(&mut source)
            .map_err(|error| format!("failed to read stdin: {error}"))?;
        Ok(source)
    } else {
        fs::read(path).map_err(|error| format!("failed to read {path}: {error}"))
    }
}
