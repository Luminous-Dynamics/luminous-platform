use civic_record_crosswalk_v1::parse_and_validate_json;

#[test]
fn checked_in_crosswalk_conforms_to_strict_rust_contract() {
    let input = include_str!("../../crosswalk.json");
    let result = parse_and_validate_json(input);
    assert!(result.is_ok(), "crosswalk rejected: {result:?}");
}

#[test]
fn work_case_and_operations_event_dependencies_are_explicitly_blocked() {
    let input = include_str!("../../crosswalk.json");
    let crosswalk = parse_and_validate_json(input).expect("valid mapping register");
    for id in [
        "PLATFORM-WORK-CASE-CONTRACT-DEPENDENCY",
        "PLATFORM-OPERATIONS-EVENT-CONTRACT-DEPENDENCY",
    ] {
        let rule = crosswalk
            .mappings
            .iter()
            .find(|rule| rule.mapping_id == id)
            .expect("dependency rule is present");
        assert_eq!(rule.disposition, civic_record_crosswalk_v1::Disposition::Blocked);
        assert_eq!(
            rule.conversion_policy,
            civic_record_crosswalk_v1::ConversionPolicy::BlockedUntilContractExists
        );
    }
}
