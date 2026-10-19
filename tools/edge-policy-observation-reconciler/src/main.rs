use std::env;
use std::fs;
use std::process::ExitCode;

use edge_policy_observation_reconciler::{build_synthetic_receipt, reconcile};

const MAX_FILE_BYTES: u64 = 33_554_432;

fn read_utf8(path: &str) -> Result<String, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("{path}: {error}"))?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err(format!("{path}: file exceeds {MAX_FILE_BYTES} bytes"));
    }
    let bytes = fs::read(path).map_err(|error| format!("{path}: {error}"))?;
    String::from_utf8(bytes).map_err(|error| format!("{path}: file is not UTF-8: {error}"))
}

fn run() -> Result<ExitCode, String> {
    let arguments: Vec<String> = env::args().collect();
    let (corpus_schema_path, corpus_path, receipt_schema_path, receipt_source) = if arguments.len()
        == 5
        && arguments[1] == "--smoke"
    {
        let corpus_source = read_utf8(&arguments[3])?;
        let receipt =
            build_synthetic_receipt(&corpus_source).map_err(|errors| errors.join("\n"))?;
        (
            arguments[2].as_str(),
            arguments[3].as_str(),
            arguments[4].as_str(),
            serde_json::to_string(&receipt)
                .map_err(|error| format!("cannot encode synthetic receipt: {error}"))?,
        )
    } else if arguments.len() == 5 {
        (
            arguments[1].as_str(),
            arguments[2].as_str(),
            arguments[3].as_str(),
            read_utf8(&arguments[4])?,
        )
    } else {
        return Err(format!(
            "usage: {} <corpus-schema.json> <corpus.json> <receipt-schema.json> <receipt.json>\n       {} --smoke <corpus-schema.json> <corpus.json> <receipt-schema.json>",
            arguments
                .first()
                .map(String::as_str)
                .unwrap_or("edge-policy-observation-reconcile"),
            arguments
                .first()
                .map(String::as_str)
                .unwrap_or("edge-policy-observation-reconcile")
        ));
    };

    let corpus_schema = read_utf8(corpus_schema_path)?;
    let corpus_source = read_utf8(corpus_path)?;
    let receipt_schema = read_utf8(receipt_schema_path)?;
    let report = reconcile(
        &corpus_schema,
        &corpus_source,
        &receipt_schema,
        &receipt_source,
    )
    .map_err(|errors| errors.join("\n"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("cannot serialize report: {error}"))?
    );

    let disposition = report["disposition"].as_str().unwrap_or("");
    match disposition {
        "synthetic_fixture_consistent_not_qualification"
        | "reported_observations_consistent_not_authenticated" => Ok(ExitCode::SUCCESS),
        _ => Ok(ExitCode::from(2)),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("edge policy observation reconciliation failed: {error}");
            ExitCode::FAILURE
        }
    }
}
