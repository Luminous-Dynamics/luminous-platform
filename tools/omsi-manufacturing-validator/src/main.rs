use omsi_manufacturing_validator::{evaluate_input_value, validate_vector_document};
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
        _ => Err(
            "usage: omsi-manufacturing-validator validate-vectors <vectors.json>\n       omsi-manufacturing-validator evaluate-input <input.json | ->".into(),
        ),
    }
}

fn read_json(path: &str) -> Result<Value, String> {
    let mut source = String::new();
    if path == "-" {
        io::stdin()
            .read_to_string(&mut source)
            .map_err(|error| format!("failed to read stdin: {error}"))?;
    } else {
        source =
            fs::read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
    }
    serde_json::from_str(&source).map_err(|error| format!("invalid JSON in {path}: {error}"))
}
