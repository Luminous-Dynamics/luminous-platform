use home_profile_validator::{check_directory, read_json, validate_schema};
use std::env;
use std::error::Error;
use std::path::Path;
use std::process;

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(
            "usage: home-profile-validator <schema.json> <valid-fixture-directory> <negative-fixture-directory>"
                .into(),
        );
    }

    let schema_path = Path::new(&args[0]);
    let valid_root = Path::new(&args[1]);
    let negative_root = Path::new(&args[2]);
    let schema = read_json(schema_path)?;

    validate_schema(&schema).map_err(|errors| -> Box<dyn Error> {
        format!("{} is not a valid JSON Schema Draft 2020-12 document: {}", schema_path.display(), errors.join("; ")).into()
    })?;

    let valid_count = check_directory(&schema, valid_root, true)?;
    let negative_count = check_directory(&schema, negative_root, false)?;
    println!("PROFILE CONTRACT PASS: {valid_count} valid fixture(s), {negative_count} negative fixture(s) rejected");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("PROFILE CONTRACT FAIL: {error}");
        process::exit(1);
    }
}
