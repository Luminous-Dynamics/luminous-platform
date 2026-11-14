use firmware_assurance_corpus_checker::{validate_corpus_bytes, MAX_DOCUMENT_BYTES};
use std::env;
use std::fs::File;
use std::io::{Read, Take};
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
    let args = env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str) {
        Some("validate-corpus") if args.len() == 4 => {
            let corpus = read_bounded(&args[2])?;
            let schema = read_bounded(&args[3])?;
            let report = validate_corpus_bytes(&corpus, &schema)?;
            let passed = report.get("status").and_then(serde_json::Value::as_str)
                == Some("passed");
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|error| format!("failed to serialize report: {error}"))?
            );
            Ok(if passed { 0 } else { 1 })
        }
        _ => {
            eprintln!(
                "usage: firmware-assurance-corpus-checker validate-corpus <corpus.json> <schema.json>"
            );
            Ok(2)
        }
    }
}

fn read_bounded(path: &str) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|error| format!("cannot open {path}: {error}"))?;
    let limit = (MAX_DOCUMENT_BYTES as u64) + 1;
    let mut bounded: Take<File> = file.take(limit);
    let mut bytes = Vec::new();
    bounded
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read {path}: {error}"))?;
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(format!(
            "{path} exceeds the {}-byte input limit",
            MAX_DOCUMENT_BYTES
        ));
    }
    Ok(bytes)
}
