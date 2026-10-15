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
        "b5407b885115ab4ef30c118a78a0ea424ae1ab6d"
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
        contract
            .notes
            .contains("fixture-only observations require null retrieved_at"),
        "the pinned observation contract must document truthful capture provenance"
    );
}

#[test]
fn sol_atlas_civic_case_pin_tracks_current_source_and_stays_unqualified() {
    let input = include_str!("../../crosswalk.json");
    let crosswalk = parse_and_validate_json(input).expect("valid crosswalk");
    let contract = crosswalk
        .contracts
        .iter()
        .find(|contract| contract.schema_id == "sol-atlas.civic-service-case-v1")
        .expect("Sol-Atlas civic case source is pinned");

    assert_eq!(
        contract.snapshot_commit_sha,
        "e5b68e0fd30667ac9712a839faadc951e939c42c"
    );
    assert_eq!(
        contract.git_blob_sha1.as_deref(),
        Some("d8897ac47fc78685b4f7e7fa3b06d6ea0724191a")
    );
    assert_eq!(contract.version.as_deref(), Some("1"));
    assert_eq!(
        contract.qualification_state,
        civic_record_crosswalk_v1::QualificationState::UnqualifiedDraft
    );
    assert!(
        contract.notes.contains("public_projection"),
        "public serialization must pass through the explicit projection boundary"
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
        "e78717f569d7aa5956e2f70c891f257417c2429f"
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
        "227e0b174c58044c75f1a56f6b78730fdf7fa4d6"
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
            "227e0b174c58044c75f1a56f6b78730fdf7fa4d6"
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
    for (schema_id, expected_sha, expected_blob) in [
        (
            "platform.work-case-v1",
            "7ec5588d9844b10006b3264e34f30bd4aeba48ec",
            "6be2f6498630d76287034020d9e89eca76ee2711",
        ),
        (
            "platform.operations-event-v1",
            "c43a68adc9543fee67d67debfd7bcf6e1387395d",
            "aa7364e5661125e04cc5e36b12269ec08b611448",
        ),
    ] {
        let contract = crosswalk
            .contracts
            .iter()
            .find(|entry| entry.schema_id == schema_id)
            .unwrap_or_else(|| panic!("missing pinned Platform contract {schema_id}"));
        assert_eq!(contract.snapshot_commit_sha, expected_sha);
        assert_eq!(contract.git_blob_sha1.as_deref(), Some(expected_blob));
        assert_eq!(
            contract.qualification_state,
            civic_record_crosswalk_v1::QualificationState::UnqualifiedDraft
        );
    }

    let work_state = crosswalk
        .mappings
        .iter()
        .find(|rule| rule.mapping_id == "MYCELIX-CASE-STATE-TO-WORK-CASE-STATE")
        .expect("Work Case mapping is explicit");
    assert_eq!(
        work_state.disposition,
        civic_record_crosswalk_v1::Disposition::Blocked
    );
    assert_eq!(
        work_state.conversion_policy,
        civic_record_crosswalk_v1::ConversionPolicy::NoAutomaticMapping
    );

    let event = crosswalk
        .mappings
        .iter()
        .find(|rule| rule.mapping_id == "SOL-EVENT-TO-OPERATIONS-EVENT")
        .expect("Operations Event mapping is explicit");
    assert_eq!(
        event.disposition,
        civic_record_crosswalk_v1::Disposition::Blocked
    );
    assert_eq!(
        event.conversion_policy,
        civic_record_crosswalk_v1::ConversionPolicy::ExplicitAdapterRequired
    );
}

#[test]
fn crosswalk_rejects_impossible_review_dates_and_accepts_leap_days() {
    let fixture = include_str!("../../crosswalk.json");

    let mut leap_day: serde_json::Value =
        serde_json::from_str(fixture).expect("checked-in crosswalk is valid JSON");
    leap_day["review_snapshot"] = serde_json::json!("2024-02-29");
    let encoded = serde_json::to_string(&leap_day).expect("crosswalk serializes");
    assert!(
        parse_and_validate_json(&encoded).is_ok(),
        "a valid leap day must be accepted"
    );

    for invalid_date in [
        "2026-02-30",
        "2026-04-31",
        "2025-02-29",
        "2026-13-01",
        "2026-00-10",
        "1899-12-31",
        "2201-01-01",
        "2026/10/15",
    ] {
        let mut value: serde_json::Value =
            serde_json::from_str(fixture).expect("checked-in crosswalk is valid JSON");
        value["review_snapshot"] = serde_json::json!(invalid_date);
        let encoded = serde_json::to_string(&value).expect("crosswalk serializes");
        let error = parse_and_validate_json(&encoded)
            .expect_err("invalid review snapshot dates must fail closed");
        assert!(
            error
                .to_string()
                .contains("review_snapshot must be a real calendar date"),
            "unexpected error for {invalid_date}: {error}"
        );
    }
}
