use std::env;
use std::fs;
use std::process::ExitCode;

use connectivity_recovery_multiwindow_sim::validate_and_simulate;
use serde_json::Value;

const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;

fn read_json(path: &str) -> Result<Value, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("{path}: {error}"))?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err(format!("{path}: file exceeds {MAX_FILE_BYTES} bytes"));
    }
    let bytes = fs::read(path).map_err(|error| format!("{path}: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{path}: invalid JSON: {error}"))
}

fn read_bytes(path: &str) -> Result<Vec<u8>, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("{path}: {error}"))?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err(format!("{path}: file exceeds {MAX_FILE_BYTES} bytes"));
    }
    fs::read(path).map_err(|error| format!("{path}: {error}"))
}

fn write_json(path: &str, value: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("cannot encode {path}: {error}"))?;
    fs::write(path, bytes).map_err(|error| format!("cannot write {path}: {error}"))
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() != 8 {
        return Err(format!(
            "usage: {} <scenario-schema.json> <scenario.json> <run-schema.json> <profile-schema.json> <profile.json> <output-run.json> <output-report.json>",
            arguments.first().map(String::as_str).unwrap_or("connectivity-recovery-multiwindow-sim")
        ));
    }

    let scenario_schema = read_json(&arguments[1])?;
    let scenario = read_json(&arguments[2])?;
    let run_schema = read_json(&arguments[3])?;
    let profile_schema = read_json(&arguments[4])?;
    let profile_bytes = read_bytes(&arguments[5])?;
    let profile: Value = serde_json::from_slice(&profile_bytes)
        .map_err(|error| format!("{}: invalid profile JSON: {error}", arguments[5]))?;

    let (run, report) = validate_and_simulate(
        &scenario_schema,
        &scenario,
        &run_schema,
        &profile_schema,
        &profile_bytes,
        &profile,
    )
    .map_err(|errors| errors.join("\n"))?;

    write_json(&arguments[6], &run)?;
    write_json(&arguments[7], &report)?;
    let mode_count = report
        .pointer("/multiwindow_recovery/per_mode")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    println!(
        "VALID synthetic multi-window recovery receipt: modes={mode_count}; raw trial receipt and aggregate report written"
    );
    println!(
        "Synthetic simulation only: configuration validation is not a radio measurement or field-performance claim."
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("INVALID multi-window recovery scenario:\n{error}");
            ExitCode::FAILURE
        }
    }
}
