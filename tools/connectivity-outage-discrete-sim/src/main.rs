use connectivity_outage_discrete_sim::validate_and_simulate;
use serde_json::Value;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process;

const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;

fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut reader = file.take((MAX_INPUT_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(format!("{} exceeds the {MAX_INPUT_BYTES}-byte input limit", path.display()));
    }
    Ok(bytes)
}

fn parse_json(bytes: &[u8], label: &str) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|error| format!("{label} JSON error: {error}"))
}

fn fatal(message: impl std::fmt::Display, code: i32) -> ! {
    eprintln!("{message}");
    process::exit(code)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 7 || args[1] == "--help" || args[1] == "-h" {
        eprintln!(
            "Usage: connectivity-outage-discrete-sim <scenario-schema.json> <scenario.json> <task-run-schema.json> <task-profile-schema.json> <task-profile.json> <output-run.json>\n\
             Emits a synthetic paired run and a validated report; it does not model RF behavior or establish field performance."
        );
        process::exit(if args.len() == 2 { 0 } else { 64 });
    }

    let scenario_schema_bytes = read_bounded(Path::new(&args[1])).unwrap_or_else(|error| fatal(error, 2));
    let scenario_bytes = read_bounded(Path::new(&args[2])).unwrap_or_else(|error| fatal(error, 2));
    let run_schema_bytes = read_bounded(Path::new(&args[3])).unwrap_or_else(|error| fatal(error, 2));
    let profile_schema_bytes = read_bounded(Path::new(&args[4])).unwrap_or_else(|error| fatal(error, 2));
    let profile_bytes = read_bounded(Path::new(&args[5])).unwrap_or_else(|error| fatal(error, 2));

    let scenario_schema = parse_json(&scenario_schema_bytes, "scenario schema").unwrap_or_else(|error| fatal(error, 2));
    let scenario = parse_json(&scenario_bytes, "scenario").unwrap_or_else(|error| fatal(error, 2));
    let run_schema = parse_json(&run_schema_bytes, "task-outcome run schema").unwrap_or_else(|error| fatal(error, 2));
    let profile_schema = parse_json(&profile_schema_bytes, "task-outcome profile schema").unwrap_or_else(|error| fatal(error, 2));
    let profile = parse_json(&profile_bytes, "task-outcome profile").unwrap_or_else(|error| fatal(error, 2));

    let (run, report) = validate_and_simulate(
        &scenario_schema, &scenario, &run_schema, &profile_schema, &profile_bytes, &profile,
    ).unwrap_or_else(|errors| fatal(errors.join("\n"), 1));

    let serialized_run = serde_json::to_vec_pretty(&run).unwrap_or_else(|error| fatal(error, 2));
    fs::write(Path::new(&args[6]), serialized_run).unwrap_or_else(|error| fatal(error, 2));
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer_pretty(&mut output, &report).unwrap_or_else(|error| fatal(error, 2));
    use std::io::Write;
    output.write_all(b"\n").unwrap_or_else(|error| fatal(error, 2));
}
