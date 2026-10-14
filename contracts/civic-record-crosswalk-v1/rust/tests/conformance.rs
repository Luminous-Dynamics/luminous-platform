use civic_record_crosswalk_v1::parse_and_validate_json;

#[test]
fn checked_in_crosswalk_conforms_to_strict_rust_contract() {
    let input = include_str!("../../crosswalk.json");
    let result = parse_and_validate_json(input);
    assert!(result.is_ok(), "crosswalk rejected: {result:?}");
}

#[test]
fn existing_draft_work_case_and_operations_event_contracts_remain_blocked_until_qualified() {
    let input = include_str!("../../crosswalk.json");
    let crosswalk = parse_and_validate_json(input).expect("valid mapping register");
    let work_state = crosswalk
        .mappings
        .iter()
        .find(|rule| rule.mapping_id == "MYCELIX-CASE-STATE-TO-WORK-CASE-STATE")
        .expect("Work Case mapping is explicit");
    assert_eq!(work_state.disposition, civic_record_crosswalk_v1::Disposition::Blocked);
    assert_eq!(
        work_state.conversion_policy,
        civic_record_crosswalk_v1::ConversionPolicy::NoAutomaticMapping
    );

    let event = crosswalk
        .mappings
        .iter()
        .find(|rule| rule.mapping_id == "SOL-EVENT-TO-OPERATIONS-EVENT")
        .expect("Operations Event mapping is explicit");
    assert_eq!(event.disposition, civic_record_crosswalk_v1::Disposition::Blocked);
    assert_eq!(
        event.conversion_policy,
        civic_record_crosswalk_v1::ConversionPolicy::ExplicitAdapterRequired
    );
}
