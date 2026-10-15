use civic_evaluation_pack_v1::validate_evaluation_pack_json;
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "validate" {
        eprintln!("usage: validate-evaluation-pack validate <json-file>");
        return ExitCode::from(2);
    }

    const MAX_INPUT_BYTES: u64 = 2 * 1024 * 1024;
    let metadata = match fs::metadata(&args[2]) {
        Ok(metadata) => metadata,
        Err(error) => {
            eprintln!("cannot stat {}: {error}", args[2]);
            return ExitCode::from(2);
        }
    };
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES {
        eprintln!("input must be a regular file no larger than {MAX_INPUT_BYTES} bytes");
        return ExitCode::from(2);
    }

    let input = match fs::read_to_string(&args[2]) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("cannot read {}: {error}", args[2]);
            return ExitCode::from(2);
        }
    };

    match validate_evaluation_pack_json(&input) {
        Ok(pack) => {
            println!(
                "VALID synthetic evaluation pack id={} scenarios={} seed={}",
                pack.pack_id,
                pack.scenarios.len(),
                pack.reproducibility.seed
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("INVALID synthetic evaluation pack\n{errors}");
            ExitCode::FAILURE
        }
    }
}
