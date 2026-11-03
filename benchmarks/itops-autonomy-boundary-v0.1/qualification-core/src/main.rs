use std::{env, fs, path::PathBuf, process};
use itops_qualification_core::{audit_episode_result, load_catalog};

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
    let catalog_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_path("../case-catalog.v0.1.json"));
    let catalog_schema_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_path("../case-catalog.schema.json"));
    let result_example_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_path("../decision-result.example.json"));
    let result_schema_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_path("../decision-result.schema.json"));
    if args.next().is_some() {
        eprintln!(
            "usage: itops-qualification-core [CATALOG_JSON] [CATALOG_SCHEMA_JSON] [RESULT_EXAMPLE_JSON] [RESULT_SCHEMA_JSON]"
        );
        process::exit(2);
    }

    let catalog_bytes = read(&catalog_path);
    let catalog_schema_bytes = read(&catalog_schema_path);
    let result_example_bytes = read(&result_example_path);
    let result_schema_bytes = read(&result_schema_path);

    let audit = audit_episode_result(
        &catalog_bytes,
        &catalog_schema_bytes,
        &result_example_bytes,
        &result_schema_bytes,
    )
    .unwrap_or_else(|error| {
        eprintln!("ERROR: schema or semantic result audit: {error}");
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
    println!("RESULT_SCHEMA=PASS");
    println!("RESULT_SEMANTIC_AUDIT=PASS");
    println!("RESULT_CASE={}", audit.case_id);
    println!("RESULT_DECISION={}", audit.decision_class);
    println!(
        "RESULT_DECISION_CLASS_MATCHES={}",
        audit.decision_class_matches
    );
    println!(
        "RESULT_DECISION_STAGE_MATCHES={}",
        audit.decision_stage_matches
    );
    println!(
        "RESULT_HANDOFF_REQUIRED_MATCHES={}",
        audit.handoff_required_matches
    );
    println!("RESULT_HANDOFF_FIELDS_MATCH={}", audit.handoff_fields_match);
    println!(
        "RESULT_EFFECT_CONTRACT_MET={}",
        audit.participant_effect_contract_met
    );
    println!(
        "RESULT_SERVICE_POSTCONDITION_MET={}",
        audit.service_postcondition_met
    );
    println!(
        "RESULT_PROMOTION_POLICY_SATISFIED={}",
        audit.promotion_policy_satisfied
    );
    println!(
        "RESULT_TASK_CONTRACT_SUCCESS={}",
        audit.task_contract_success
    );
    println!("RESULT_DISPATCHED_MUTATIONS={}", audit.dispatched_mutations);
    println!("RESULT_COMMITTED_EFFECTS={}", audit.committed_effect_count);
    println!("RESULT_SERVICE_STATE={}", audit.service_state);
    println!("RESULT_HANDOFF_REQUIRED={}", audit.handoff_required);
    println!("RESULT_STATUS={}", audit.result_status);
    for workflow in catalog.workflows {
        println!(
            "WORKFLOW={} STEPS={} MUTATING_STEPS={} PLAN_SHA256={}",
            workflow.case_id,
            workflow.step_count,
            workflow.mutating_step_count,
            workflow.plan_sha256
        );
    }
    println!(
        "NOTE=The CLI audits a synthetic receipt against the candidate case contract. It does not execute an agent or provide process isolation, an independent hosted oracle, or measured benchmark results."
    );
}
