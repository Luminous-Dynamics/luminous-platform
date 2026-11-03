use std::{env, fs, path::PathBuf, process};
use itops_qualification_core::{load_catalog, validate_draft2020_12};

fn default_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn read(path: &PathBuf) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| {
        eprintln!("ERROR: cannot read {}: {error}", path.display());
        process::exit(2);
    })
}

fn main() {
    let mut args = env::args_os().skip(1);
    let catalog_path = args.next().map(PathBuf::from)
        .unwrap_or_else(|| default_path("../case-catalog.v0.1.json"));
    let catalog_schema_path = args.next().map(PathBuf::from)
        .unwrap_or_else(|| default_path("../case-catalog.schema.json"));
    let result_example_path = args.next().map(PathBuf::from)
        .unwrap_or_else(|| default_path("../decision-result.example.json"));
    let result_schema_path = args.next().map(PathBuf::from)
        .unwrap_or_else(|| default_path("../decision-result.schema.json"));
    if args.next().is_some() {
        eprintln!("usage: itops-qualification-core [CATALOG_JSON] [CATALOG_SCHEMA_JSON] [RESULT_EXAMPLE_JSON] [RESULT_SCHEMA_JSON]");
        process::exit(2);
    }

    let catalog_bytes = read(&catalog_path);
    let catalog_schema_bytes = read(&catalog_schema_path);
    let result_example_bytes = read(&result_example_path);
    let result_schema_bytes = read(&result_schema_path);

    validate_draft2020_12(&catalog_bytes, &catalog_schema_bytes)
        .unwrap_or_else(|error| {
            eprintln!("ERROR: candidate catalog schema validation: {error}");
            process::exit(2);
        });
    validate_draft2020_12(&result_example_bytes, &result_schema_bytes)
        .unwrap_or_else(|error| {
            eprintln!("ERROR: result example schema validation: {error}");
            process::exit(2);
        });

    let catalog = load_catalog(&catalog_bytes).unwrap_or_else(|error| {
        eprintln!("ERROR: catalog qualification: {error}");
        process::exit(2);
    });

    println!("ITOPS_QUALIFICATION_CORE=PASS");
    println!("SCHEMA_DRAFT=2020-12");
    println!("FORMAT_ASSERTIONS=ENABLED");
    println!("CASE_COUNT={}", catalog.case_count);
    println!("CATALOG_SHA256={}", catalog.catalog_sha256);
    println!("STATUS=candidate_not_frozen");
    println!("RESULT_EXAMPLE_SCHEMA=PASS");
    for workflow in catalog.workflows {
        println!(
            "WORKFLOW={} STEPS={} MUTATING_STEPS={} PLAN_SHA256={}",
            workflow.case_id,
            workflow.step_count,
            workflow.mutating_step_count,
            workflow.plan_sha256
        );
    }
    println!("NOTE=Schema and deterministic core validation only; this does not execute an agent or provide independent process isolation.");
}
