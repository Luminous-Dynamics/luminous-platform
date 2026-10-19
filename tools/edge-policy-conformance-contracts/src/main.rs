use std::env;
use std::fs;
use std::process::ExitCode;

use edge_policy_conformance_contracts::validate_corpus;

const MAX_FILE_BYTES: u64 = 2_097_152;

fn read_utf8(path: &str) -> Result<String, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("{path}: {error}"))?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err(format!("{path}: file exceeds {MAX_FILE_BYTES} bytes"));
    }
    let bytes = fs::read(path).map_err(|error| format!("{path}: {error}"))?;
    String::from_utf8(bytes).map_err(|error| format!("{path}: file is not UTF-8: {error}"))
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() != 3 {
        return Err(format!(
            "usage: {} <corpus-schema.json> <corpus.json>",
            arguments.first().map(String::as_str).unwrap_or("edge-policy-conformance-verify")
        ));
    }

    let schema = read_utf8(&arguments[1])?;
    let corpus_source = read_utf8(&arguments[2])?;
    match validate_corpus(&schema, &corpus_source) {
        Ok(corpus) => {
            let id = corpus.get("corpus_id").and_then(serde_json::Value::as_str).unwrap_or("unknown");
            let count = corpus.get("cases").and_then(serde_json::Value::as_array).map_or(0, Vec::len);
            println!("VALID synthetic benchmark corpus: id={id} cases={count}");
            println!("This validates benchmark intent only; it does not prove policy enforcement or a measured result.");
            Ok(())
        }
        Err(errors) => Err(errors.join("\n")),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("INVALID edge policy conformance corpus:\n{error}");
            ExitCode::FAILURE
        }
    }
}
