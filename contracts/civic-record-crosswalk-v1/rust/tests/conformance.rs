use civic_record_crosswalk_v1::parse_and_validate_json;

#[test]
fn checked_in_crosswalk_conforms_to_strict_rust_contract() {
    let input = include_str!("../../crosswalk.json");
    let result = parse_and_validate_json(input);
    assert!(result.is_ok(), "crosswalk rejected: {result:?}");
}

#[test]
fn public_observation_contract_pin_tracks_truthful_retrieval_provenance() {
    let input = include_str!("../../crosswalk.json");
    let crosswalk = parse_and_validate_json(input).expect("valid crosswalk");
    let contract = crosswalk
        .contracts
        .iter()
        .find(|contract| contract.schema_id == "platform.public-observation-v1")
        .expect("public observation schema is pinned");

    assert_eq!(
        contract.snapshot_commit_sha,
        "9316cc917b13a395a62da4b719bcdb76f0ef224a"
    );
    assert_eq!(
        contract.git_blob_sha1.as_deref(),
        Some("4a50b82a8ed32deeb245e11b7e4d542860a6c2a2")
    );
    assert_eq!(contract.version.as_deref(), Some("public-observation-v1"));
    assert_eq!(
        contract.qualification_state,
        civic_record_crosswalk_v1::QualificationState::UnqualifiedDraft
    );
    assert!(
        contract.notes.contains("fixture-only observations require null retrieved_at"),
        "the pinned observation contract must document truthful capture provenance"
    );
}

#[test]
fn symthaea_life_civic_corpus_pin_tracks_v0_2_0_and_stays_unqualified() {
    let input = include_str!("../../crosswalk.json");
    let crosswalk = parse_and_validate_json(input).expect("valid crosswalk");
    let contract = crosswalk
        .contracts
        .iter()
        .find(|contract| contract.schema_id == "symthaea.life-civic-reference-corpus")
        .expect("Symthaea life/civic corpus is pinned");

    assert_eq!(
        contract.snapshot_commit_sha,
        "42dbd6de258cf6d1b97e4b863c010c628802433d"
    );
    assert_eq!(
        contract.git_blob_sha1.as_deref(),
        Some("df29b96497a4a1dfc0042d9853ee10eb79f962c0")
    );
    assert_eq!(contract.version.as_deref(), Some("0.2.0"));
    assert_eq!(
        contract.qualification_state,
        civic_record_crosswalk_v1::QualificationState::UnqualifiedDraft
    );
}

#[test]
fn mycelix_service_corpus_pin_and_cross_corpus_mapping_stay_non_runtime() {
    let input = include_str!("../../crosswalk.json");
    let crosswalk = parse_and_validate_json(input).expect("valid crosswalk");
    let contract = crosswalk
        .contracts
        .iter()
        .find(|contract| contract.schema_id == "mycelix.life-civic-service-reference-corpus")
        .expect("Mycelix service-journey corpus is pinned");

    assert_eq!(
        contract.snapshot_commit_sha,
        "1eec5ed1e046eae71b0e376f327016a0998e0a92"
    );
    assert_eq!(
        contract.git_blob_sha1.as_deref(),
        Some("8674042a1d4d9fe4d8b27cfca759657ace4f07f5")
    );
    assert_eq!(contract.version.as_deref(), Some("0.4.0"));
    assert_eq!(
        contract.qualification_state,
        civic_record_crosswalk_v1::QualificationState::UnqualifiedDraft
    );

    for (schema_id, expected_blob) in [
        (
            "mycelix.service-journey-reference-v0.3.0",
            "7e2b8b3d963c9c5598c78927121fcf60203591f9",
        ),
        (
            "mycelix.applicability-assessment",
            "241d4c4abbd765c05e56ddfae3391cead0f26dbc",
        ),
    ] {
        let source_schema = crosswalk
            .contracts
            .iter()
            .find(|entry| entry.schema_id == schema_id)
            .unwrap_or_else(|| panic!("missing pinned Mycelix schema {schema_id}"));
        assert_eq!(
            source_schema.snapshot_commit_sha,
            "1eec5ed1e046eae71b0e376f327016a0998e0a92"
        );
        assert_eq!(source_schema.git_blob_sha1.as_deref(), Some(expected_blob));
    }

    let mapping = crosswalk
        .mappings
        .iter()
        .find(|rule| rule.mapping_id == "MYCELIX-SYMTHAEA-CAUSALITY-CORPORA")
        .expect("cross-corpus semantic relationship is explicit");
    assert_eq!(
        mapping.from.schema_id,
        "mycelix.life-civic-service-reference-corpus"
    );
    assert_eq!(mapping.to.schema_id, "symthaea.life-civic-reference-corpus");
    assert_eq!(
        mapping.relationship,
        civic_record_crosswalk_v1::SemanticRelationship::PartialSemanticOverlap
    );
    assert_eq!(
        mapping.conversion_policy,
        civic_record_crosswalk_v1::ConversionPolicy::PreserveSeparately
    );
    assert_eq!(
        mapping.disposition,
        civic_record_crosswalk_v1::Disposition::PreserveSeparate
    );
    assert!(
        mapping
            .required_guards
            .iter()
            .any(|guard| guard.contains("incident-level cause")),
        "the cross-corpus mapping must preserve its causal-inference guard"
    );
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
