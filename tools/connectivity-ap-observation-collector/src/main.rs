use std::env;
use std::fs;
use std::io;
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use connectivity_ap_observation_collector::{
    build_snapshot, validate_observation_manifest, CapturedOutputs,
};
use serde_json::{Value, json};

const MAX_INPUT_BYTES: u64 = 1_048_576;

struct Capture {
    status: Value,
    stdout: Option<String>,
}

fn read_limited(path: &str) -> Result<Vec<u8>, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("{path}: {error}"))?;
    if metadata.len() > MAX_INPUT_BYTES {
        return Err(format!("{path}: file exceeds {MAX_INPUT_BYTES} bytes"));
    }
    fs::read(path).map_err(|error| format!("{path}: {error}"))
}

fn capture(program: &str, args: &[&str]) -> Capture {
    match Command::new(program).args(args).output() {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Capture {
            status: json!({"status":"unavailable","exit_code":null}),
            stdout: None,
        },
        Err(_) => Capture {
            status: json!({"status":"error","exit_code":null}),
            stdout: None,
        },
        Ok(output) => {
            let exit_code = output.status.code();
            if output.status.success() {
                Capture {
                    status: json!({"status":"available","exit_code":exit_code}),
                    stdout: String::from_utf8(output.stdout).ok(),
                }
            } else {
                Capture {
                    status: json!({"status":"error","exit_code":exit_code}),
                    stdout: None,
                }
            }
        }
    }
}

fn timestamp_ms() -> Result<u64, String> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock precedes Unix epoch: {error}"))?;
    u64::try_from(elapsed.as_millis()).map_err(|_| "timestamp is outside u64 range".to_owned())
}

fn write_json(path: &str, value: &Value) -> Result<(), String> {
    let data = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("cannot encode observation snapshot: {error}"))?;
    fs::write(path, data).map_err(|error| format!("cannot write {path}: {error}"))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 6 {
        return Err(format!(
            "usage: {} <manifest-schema.json> <measured-testbed-manifest.json> <interface> <run-id> <output.json>",
            args.first().map(String::as_str).unwrap_or("connectivity-ap-observe")
        ));
    }
    let manifest_schema = read_limited(&args[1])?;
    let manifest_bytes = read_limited(&args[2])?;
    let schema_text = std::str::from_utf8(&manifest_schema)
        .map_err(|error| format!("{}: schema is not UTF-8: {error}", args[1]))?;
    let _: Value = validate_observation_manifest(schema_text, &manifest_bytes)
        .map_err(|errors| format!("manifest rejected:\n{}", errors.join("\n")))?;

    let interface = args[3].as_str();
    // Validate the interface before passing it to fixed-argument, read-only
    // commands. No shell is invoked and no traffic/configuration is generated.
    if interface.is_empty()
        || interface.len() > 15
        || !interface
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
    {
        return Err("interface must match [A-Za-z0-9_.-] and be 1..=15 bytes".to_owned());
    }
    let run_id = args[4].as_str();
    if run_id.is_empty()
        || run_id.len() > 128
        || !run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err("run ID must be a non-empty ASCII token of at most 128 bytes".to_owned());
    }

    // Allow-listed read-only observations only:
    // iw station/survey dumps, tc qdisc stats, and ip link JSON. No shell.
    let station = capture("iw", &["dev", interface, "station", "dump"]);
    let survey = capture("iw", &["dev", interface, "survey", "dump"]);
    let qdisc = capture("tc", &["-s", "qdisc", "show", "dev", interface]);
    let link = capture("ip", &["-j", "link", "show", "dev", interface]);
    let statuses = json!({
        "iw_station": station.status,
        "iw_survey": survey.status,
        "tc_qdisc": qdisc.status,
        "ip_link": link.status
    });

    let snapshot = build_snapshot(
        run_id,
        interface,
        &manifest_bytes,
        timestamp_ms()?,
        CapturedOutputs {
            station: station.stdout.as_deref(),
            survey: survey.stdout.as_deref(),
            qdisc: qdisc.stdout.as_deref(),
            link: link.stdout.as_deref(),
        },
        statuses,
    )
    .map_err(|errors| errors.join("\n"))?;

    write_json(&args[5], &snapshot)?;
    println!("Observation snapshot written; no configuration changes or test traffic were initiated.");
    println!("Counters are a single read-only snapshot. Unavailable values remain null.");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("AP observation refused or failed:\n{error}");
            ExitCode::FAILURE
        }
    }
}
