use connectivity_task_outcome_core::validate_and_report;
use serde_json::Value;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process;

const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;

fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let file =
        File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut limited = file.take((MAX_INPUT_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    limited
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(format!(
            "{} exceeds the {MAX_INPUT_BYTES} byte input limit",
            path.display()
        ));
    }
    Ok(bytes)
}

fn parse_json(bytes: &[u8], name: &str) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|error| format!("{name} JSON error: {error}"))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 || args[1] == "--help" || args[1] == "-h" {
        eprintln!(
            "Usage: connectivity-task-outcome-verify <run-schema.json> <profile-schema.json> <profile.json> <run.json>. This report does not authenticate task success or field truth."
        );
        process::exit(if args.len() == 2 { 0 } else { 64 });
    }

    let run_schema_bytes = read_bounded(Path::new(&args[1])).unwrap_or_else(|error| {
        eprintln!("{error}");
        process::exit(2);
    });
    let profile_schema_bytes = read_bounded(Path::new(&args[2])).unwrap_or_else(|error| {
        eprintln!("{error}");
        process::exit(2);
    });
    let profile_bytes = read_bounded(Path::new(&args[3])).unwrap_or_else(|error| {
        eprintln!("{error}");
        process::exit(2);
    });
    let run_bytes = read_bounded(Path::new(&args[4])).unwrap_or_else(|error| {
        eprintln!("{error}");
        process::exit(2);
    });

    let run_schema = parse_json(&run_schema_bytes, "run schema").unwrap_or_else(|error| {
        eprintln!("{error}");
        process::exit(2);
    });
    let profile_schema =
        parse_json(&profile_schema_bytes, "profile schema").unwrap_or_else(|error| {
            eprintln!("{error}");
            process::exit(2);
        });
    let profile = parse_json(&profile_bytes, "profile").unwrap_or_else(|error| {
        eprintln!("{error}");
        process::exit(2);
    });
    let run = parse_json(&run_bytes, "run").unwrap_or_else(|error| {
        eprintln!("{error}");
        process::exit(2);
    });

    let result =
        match validate_and_report(&run_schema, &profile_schema, &profile_bytes, &profile, &run) {
            Ok(report) => report,
            Err(errors) => {
                eprintln!("task-outcome validation failed:");
                for error in errors {
                    eprintln!("- {error}");
                }
                process::exit(1);
            }
        };
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    if let Err(error) = serde_json::to_writer_pretty(&mut output, &result) {
        eprintln!("cannot write validation report: {error}");
        process::exit(2);
    }
    if let Err(error) = output.write_all(&[10]) {
        eprintln!("cannot terminate report: {error}");
        process::exit(2);
    }
}
