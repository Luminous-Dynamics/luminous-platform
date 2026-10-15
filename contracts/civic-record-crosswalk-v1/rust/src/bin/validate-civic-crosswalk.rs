use civic_record_crosswalk_v1::parse_and_validate_json;
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let command = args.next();
    let path = args.next();

    if command.as_deref() != Some("validate") {
        eprintln!("usage: validate-civic-crosswalk validate <crosswalk.json>");
        return ExitCode::from(2);
    }
    let Some(path) = path else {
        eprintln!("missing crosswalk path");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("unexpected extra argument");
        return ExitCode::from(2);
    }

    let input = match fs::read_to_string(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            return ExitCode::from(2);
        }
    };
    match parse_and_validate_json(&input) {
        Ok(crosswalk) => {
            println!(
                "PASS: {} contracts, {} mapping rules, snapshot {}",
                crosswalk.contracts.len(),
                crosswalk.mappings.len(),
                crosswalk.review_snapshot
            );
            println!(
                "BOUNDARY: mapping register only; no adapter executed and no action authorized"
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("{errors}");
            ExitCode::FAILURE
        }
    }
}
