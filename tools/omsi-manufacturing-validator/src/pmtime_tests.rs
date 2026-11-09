use crate::pmtime::validate_fixture_pack;
use jsonschema::draft202012;
use serde_json::Value;

const FIXTURES: &str =
    include_str!("../../../docs/standards/omsi/pmtime-integrity-fixtures-0.1.json");
const FIXTURE_SCHEMA: &str =
    include_str!("../../../docs/standards/omsi/pmtime-integrity-fixtures-0.1.schema.json");
const REPORT_SCHEMA: &str = include_str!("../schema/pmtime-integrity-report.schema.json");

#[test]
fn frozen_pmtime_pack_executes_all_fourteen_cases() {
    let document: Value = serde_json::from_str(FIXTURES).expect("fixture JSON");
    let input_schema: Value = serde_json::from_str(FIXTURE_SCHEMA).expect("fixture schema JSON");
    assert!(draft202012::meta::is_valid(&input_schema));
    let input_validator = draft202012::new(&input_schema).expect("compile fixture schema");
    assert!(
        input_validator.is_valid(&document),
        "fixture schema validation failed"
    );

    let report = validate_fixture_pack(&document, FIXTURES.as_bytes());
    assert_eq!(report["status"], "passed", "{report:#}");
    assert_eq!(report["case_count_expected"], 20);
    assert_eq!(report["case_count_discovered"], 20);
    assert_eq!(report["passed_count"], 20);
    assert_eq!(report["failure_count"], 0);

    let report_schema: Value = serde_json::from_str(REPORT_SCHEMA).expect("report schema JSON");
    assert!(draft202012::meta::is_valid(&report_schema));
    let report_validator = draft202012::new(&report_schema).expect("compile report schema");
    assert!(
        report_validator.is_valid(&report),
        "report schema validation failed: {report:#}"
    );
}

#[test]
fn exact_case_discovery_fails_closed() {
    let mut document: Value = serde_json::from_str(FIXTURES).expect("fixture JSON");
    document["cases"].as_array_mut().expect("cases").pop();
    let report = validate_fixture_pack(&document, b"mutated");
    assert_eq!(report["status"], "failed");
    assert_eq!(report["discovery_complete"], false);
}
