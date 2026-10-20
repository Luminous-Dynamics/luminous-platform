use std::env;
use std::fs;
use std::process::ExitCode;

use connectivity_ap_testbed_contracts::validate_manifest;

const MAX_FILE_BYTES: u64 = 1_048_576;

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
            "usage: {} <manifest-schema.json> <testbed-manifest.json>",
            arguments
                .first()
                .map(String::as_str)
                .unwrap_or("connectivity-ap-testbed-verify")
        ));
    }

    let schema = read_utf8(&arguments[1])?;
    let manifest_source = read_utf8(&arguments[2])?;
    match validate_manifest(&schema, &manifest_source) {
        Ok(manifest) => {
            let id = manifest
                .get("manifest_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            let stage = manifest
                .get("evidence_stage")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            println!("VALID configuration manifest: id={id} stage={stage}");
            println!(
                "This result validates configuration metadata only; it does not certify hardware or prove a measurement occurred."
            );
            Ok(())
        }
        Err(errors) => Err(errors.join("\n")),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("INVALID connectivity AP testbed manifest:\n{error}");
            ExitCode::FAILURE
        }
    }
}
