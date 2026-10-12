use public_source_catalogue_v1::{validate_catalogue_json, validate_observation_json};
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: validate-public-source <catalogue|observation> <json-file>");
        return ExitCode::from(2);
    }

    let input = match fs::read_to_string(&args[2]) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("cannot read {}: {error}", args[2]);
            return ExitCode::from(2);
        }
    };

    match args[1].as_str() {
        "catalogue" => match validate_catalogue_json(&input) {
            Ok(catalogue) => {
                println!(
                    "VALID catalogue revision={} source_count={}",
                    catalogue.catalogue_revision,
                    catalogue.sources.len()
                );
                ExitCode::SUCCESS
            }
            Err(errors) => {
                eprintln!("INVALID catalogue\n{errors}");
                ExitCode::FAILURE
            }
        },
        "observation" => match validate_observation_json(&input) {
            Ok(observation) => {
                println!(
                    "VALID observation id={} source={} state={:?} fixture_only={}",
                    observation.observation_id,
                    observation.source_id,
                    observation.raw_value_state,
                    observation.fixture_only
                );
                ExitCode::SUCCESS
            }
            Err(errors) => {
                eprintln!("INVALID observation\n{errors}");
                ExitCode::FAILURE
            }
        },
        other => {
            eprintln!("unknown document type {other:?}; expected catalogue or observation");
            ExitCode::from(2)
        }
    }
}
