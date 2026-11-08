use crate::pmtime::validate_fixture_pack;
use serde_json::Value;

const FIXTURES: &str =
    include_str!("../../../docs/standards/omsi/pmtime-integrity-fixtures-0.1.json");

#[test]
fn frozen_pmtime_pack_executes_all_fourteen_cases() {
    let document: Value = serde_json::from_str(FIXTURES).expect("fixture JSON");
    let report = validate_fixture_pack(&document, FIXTURES.as_bytes());
    assert_eq!(report["status"], "passed", "{report:#}");
    assert_eq!(report["case_count_expected"], 14);
    assert_eq!(report["case_count_discovered"], 14);
    assert_eq!(report["passed_count"], 14);
    assert_eq!(report["failure_count"], 0);
}

#[test]
fn exact_case_discovery_fails_closed() {
    let mut document: Value = serde_json::from_str(FIXTURES).expect("fixture JSON");
    document["cases"].as_array_mut().expect("cases").pop();
    let report = validate_fixture_pack(&document, b"mutated");
    assert_eq!(report["status"], "failed");
    assert_eq!(report["discovery_complete"], false);
}
