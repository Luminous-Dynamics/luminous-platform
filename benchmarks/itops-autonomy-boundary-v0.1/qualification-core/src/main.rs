use std::{env, fs, path::PathBuf, process};
use itops_qualification_core::load_catalog;

fn main() {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../case-catalog.v0.1.json")
        });

    let bytes = fs::read(&path).unwrap_or_else(|error| {
        eprintln!("ERROR: cannot read {}: {error}", path.display());
        process::exit(2);
    });
    let catalog = load_catalog(&bytes).unwrap_or_else(|error| {
        eprintln!("ERROR: {error}");
        process::exit(2);
    });

    println!("ITOPS_QUALIFICATION_CORE=PASS");
    println!("CASE_COUNT={}", catalog.case_count);
    println!("CATALOG_SHA256={}", catalog.catalog_sha256);
    println!("STATUS=candidate_not_frozen");
    for workflow in catalog.workflows {
        println!(
            "WORKFLOW={} STEPS={} MUTATING_STEPS={} PLAN_SHA256={}",
            workflow.case_id,
            workflow.step_count,
            workflow.mutating_step_count,
            workflow.plan_sha256
        );
    }
    println!("NOTE=This validates catalog/workflow and core invariants only; it does not execute an agent or provide independent process isolation.");
}
