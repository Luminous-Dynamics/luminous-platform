use omsi_manufacturing_validator::{
    evaluate_input_value, validate_vector_document, COMPARISON_RULE_ID,
};
use serde_json::{json, Value};

const VECTORS_JSON: &str = include_str!(
    "../../../docs/standards/omsi/profiles/manufacturing-asset-operations/0.1/CONFORMANCE-VECTORS.json"
);

fn document() -> Value {
    serde_json::from_str(VECTORS_JSON).expect("committed conformance vectors must be valid JSON")
}

fn vector(id: &str) -> Value {
    document()["vectors"]
        .as_array()
        .expect("vectors array")
        .iter()
        .find(|vector| vector["id"] == id)
        .unwrap_or_else(|| panic!("missing committed vector {id}"))
        .clone()
}

fn base_input() -> Value {
    vector("MFG-OBS-001")["input"].clone()
}

fn evaluate(input: &Value) -> Value {
    evaluate_input_value(input)
}

#[test]
fn committed_machine_readable_vectors_all_match_semantic_evaluator() {
    let report = validate_vector_document(&document());
    assert_eq!(report["status"], "passed", "{report:#}");
    assert_eq!(report["vector_count"], 8);
    assert_eq!(report["passed_count"], 8);
    assert_eq!(report["failed_count"], 0);
}

#[test]
fn comparison_rule_is_stable_and_never_claims_quality_release() {
    let report = evaluate(&base_input());
    assert_eq!(report["comparison_rule_id"], COMPARISON_RULE_ID);
    assert_eq!(report["outcome"], "accepted_projection");
    assert!(report["assurance_limitations"].as_array().unwrap().len() >= 4);
    assert!(!report
        .to_string()
        .contains("\"independent_quality_release\":true"));
}

#[test]
fn decimal_comparison_preserves_precision_beyond_f64_resolution() {
    let mut input = base_input();
    input["measurement"]["value_decimal"] = json!("70.0000000000000001");
    input["specification"]["lower_bound_decimal"] = json!("70.0000000000000000");
    input["specification"]["upper_bound_decimal"] = json!("70.0000000000000000");

    let report = evaluate(&input);
    assert_eq!(
        report["measurement_evaluation"],
        "outside_declared_bounds_point_estimate_only"
    );
    assert_eq!(report["outcome"], "preserve_with_conflict");
    assert_eq!(
        report["dispositions"][0]["reason"],
        "source_acceptance_disagrees_with_profile_comparison"
    );
}

#[test]
fn offset_equivalent_calibration_instants_compare_in_utc() {
    let mut input = base_input();
    input["calibration"]["valid_from"] = json!("2026-10-15T08:00:00Z");
    input["calibration"]["valid_until"] = json!("2026-10-15T08:01:00Z");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "accepted_projection");
    assert_eq!(
        report["measurement_evaluation"],
        "inside_declared_bounds_point_estimate_only"
    );
    assert!(report["dispositions"].as_array().unwrap().is_empty());
}

#[test]
fn invalid_calendar_date_is_rejected() {
    let mut input = base_input();
    input["observed_time"] = json!("2026-02-30T10:00:00+02:00");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "reject");
    assert_eq!(report["dispositions"][0]["field"], "observed_time");
    assert_eq!(
        report["dispositions"][0]["reason"],
        "timestamp_invalid_or_out_of_profile"
    );
    assert!(!report["projection_emitted"].as_bool().unwrap());
}

#[test]
fn naive_source_modified_timestamp_is_rejected() {
    let mut input = base_input();
    input["source_modified_time"] = json!("2026-10-15T10:02:00");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "reject");
    assert_eq!(report["dispositions"][0]["field"], "source_modified_time");
    assert_eq!(
        report["dispositions"][0]["reason"],
        "timestamp_has_no_explicit_offset"
    );
}

#[test]
fn specification_with_inverted_bounds_is_rejected() {
    let mut input = base_input();
    input["specification"]["lower_bound_decimal"] = json!("90");
    input["specification"]["upper_bound_decimal"] = json!("80");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "reject");
    assert_eq!(
        report["dispositions"][0]["field"],
        "specification.upper_bound_decimal"
    );
    assert_eq!(
        report["dispositions"][0]["reason"],
        "specification_bounds_inverted"
    );
}

#[test]
fn snapshot_token_mode_without_a_token_is_rejected() {
    let mut input = base_input();
    input["snapshot"]["consistency_mode"] = json!("source_snapshot_token");
    input["snapshot"]["source_watermark"] = Value::Null;

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "reject");
    assert_eq!(
        report["dispositions"][0]["field"],
        "snapshot.consistency_mode"
    );
    assert_eq!(
        report["dispositions"][0]["reason"],
        "snapshot_token_missing"
    );
}

#[test]
fn unknown_input_field_is_rejected_instead_of_ignored() {
    let mut input = base_input();
    input["source"]["unreviewed_field"] = json!("must-not-be-silently-accepted");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "reject");
    assert_eq!(
        report["dispositions"][0]["field"],
        "source.unreviewed_field"
    );
    assert_eq!(report["dispositions"][0]["reason"], "unexpected_field");
}

#[test]
fn cross_tenant_asset_reference_emits_no_projection() {
    let input = vector("MFG-OBS-004")["input"].clone();
    let report = evaluate(&input);

    assert_eq!(report["outcome"], "reject");
    assert_eq!(report["projection_emitted"], false);
    assert_eq!(
        report["dispositions"][0]["reason"],
        "trusted_scope_mismatch"
    );
}

#[test]
fn missing_revision_is_a_loss_not_a_freshness_inference() {
    let input = vector("MFG-OBS-007")["input"].clone();
    let report = evaluate(&input);

    assert_eq!(report["outcome"], "accepted_projection_with_loss");
    assert_eq!(report["source_revision_metadata"], "not_provided");
    assert_eq!(report["dispositions"][0]["type"], "loss");
    assert_eq!(report["dispositions"][0]["field"], "source.record_revision");
}

#[test]
fn dimensionally_incompatible_unit_is_not_converted_or_evaluated() {
    let input = vector("MFG-OBS-008")["input"].clone();
    let report = evaluate(&input);

    assert_eq!(report["outcome"], "preserve_with_conflict");
    assert_eq!(report["measurement_evaluation"], "not_evaluable");
    assert_eq!(
        report["dispositions"][0]["reason"],
        "measurement_unit_incompatible_with_specification"
    );
}

#[test]
fn expired_calibration_preserves_both_calibration_and_quality_claims() {
    let input = vector("MFG-OBS-003")["input"].clone();
    let report = evaluate(&input);

    assert_eq!(report["outcome"], "preserve_with_conflict");
    assert_eq!(
        report["measurement_evaluation"],
        "inside_declared_bounds_point_estimate_only_calibration_assurance_limited"
    );
    assert_eq!(report["dispositions"][0]["field"], "calibration");
    assert_eq!(
        report["dispositions"][0]["reason"],
        "calibration_validity_interval_does_not_cover_observation"
    );
}

#[test]
fn cannot_promote_a_source_claim_to_independent_quality_release() {
    let mut input = base_input();
    input["source_quality_claim"]["independent_quality_release"] = json!(true);

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "reject");
    assert_eq!(
        report["dispositions"][0]["reason"],
        "quality_claim_assurance_upgrade_not_permitted"
    );
}

#[test]
fn exponent_notation_is_rejected_for_decimal_strings() {
    let mut input = base_input();
    input["measurement"]["value_decimal"] = json!("7.24e1");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "reject");
    assert_eq!(
        report["dispositions"][0]["field"],
        "measurement.value_decimal"
    );
    assert_eq!(
        report["dispositions"][0]["reason"],
        "decimal_not_in_profile"
    );
}

#[test]
fn negative_measurement_uncertainty_is_rejected() {
    let mut input = base_input();
    input["measurement"]["uncertainty"]["value_decimal"] = json!("-0.1");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "reject");
    assert_eq!(
        report["dispositions"][0]["reason"],
        "uncertainty_must_not_be_negative"
    );
}

#[test]
fn empty_record_revision_is_not_canonical_absence() {
    let mut input = base_input();
    input["source"]["record_revision"] = json!("");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "reject");
    assert_eq!(
        report["dispositions"][0]["reason"],
        "empty_string_not_canonical"
    );
}

#[test]
fn duplicate_vector_ids_fail_the_vector_runner() {
    let mut doc = document();
    let first_id = doc["vectors"][0]["id"].clone();
    doc["vectors"][1]["id"] = first_id;

    let report = validate_vector_document(&doc);
    assert_eq!(report["status"], "failed");
    assert!(report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .any(|failure| failure["reason"] == "empty_or_duplicate_vector_id"));
}

#[test]
fn mutated_expected_result_fails_instead_of_self_certifying() {
    let mut doc = document();
    doc["vectors"][0]["expected"]["outcome"] = json!("reject");

    let report = validate_vector_document(&doc);
    assert_eq!(report["status"], "failed");
    assert!(report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .any(|failure| failure["vector_id"] == "MFG-OBS-001"));
}


#[test]
fn tenant_scope_is_enforced_for_measurement_instrument_references() {
    let mut input = base_input();
    input["measurement"]["instrument_ref"]["tenant_ref"] = json!("tenant:synthetic-b");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "reject");
    assert_eq!(
        report["dispositions"][0]["field"],
        "measurement.instrument_ref.tenant_ref"
    );
    assert_eq!(
        report["dispositions"][0]["reason"],
        "trusted_scope_mismatch"
    );
}

#[test]
fn source_instance_scope_is_enforced_for_linked_references() {
    let mut input = base_input();
    input["specification"]["spec_ref"]["source_instance"] = json!("synthetic-mes://plant-b");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "reject");
    assert_eq!(
        report["dispositions"][0]["field"],
        "specification.spec_ref.source_instance"
    );
    assert_eq!(
        report["dispositions"][0]["reason"],
        "source_instance_mismatch"
    );
}

#[test]
fn an_empty_calibration_interval_is_rejected() {
    let mut input = base_input();
    input["calibration"]["valid_from"] = json!("2026-10-15T08:00:00Z");
    input["calibration"]["valid_until"] = json!("2026-10-15T08:00:00Z");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "reject");
    assert_eq!(
        report["dispositions"][0]["field"],
        "calibration.valid_until"
    );
    assert_eq!(
        report["dispositions"][0]["reason"],
        "calibration_interval_empty_or_inverted"
    );
}

#[test]
fn equal_specification_boundaries_are_inclusive() {
    let mut input = base_input();
    input["measurement"]["value_decimal"] = json!("70.000");
    input["specification"]["lower_bound_decimal"] = json!("70");
    input["specification"]["upper_bound_decimal"] = json!("70.0");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "accepted_projection");
    assert_eq!(
        report["measurement_evaluation"],
        "inside_declared_bounds_point_estimate_only"
    );
}

#[test]
fn snapshot_export_does_not_upgrade_record_revision_or_atomicity() {
    let input = vector("MFG-OBS-007")["input"].clone();
    let report = evaluate(&input);
    assert_eq!(report["source_revision_metadata"], "not_provided");
    let limitations = report["assurance_limitations"].as_array().unwrap();
    assert!(limitations.iter().any(|limitation| {
        limitation
            .as_str()
            .unwrap_or_default()
            .contains("snapshot atomicity")
    }));
}


#[test]
fn a_snapshot_token_is_reported_as_a_source_claim_not_verified_atomicity() {
    let mut input = base_input();
    input["snapshot"]["consistency_mode"] = json!("source_snapshot_token");
    input["snapshot"]["source_watermark"] = json!("source-watermark-123");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "accepted_projection_with_loss");
    assert_eq!(
        report["dispositions"][0]["field"],
        "snapshot.source_watermark"
    );
    assert_eq!(
        report["dispositions"][0]["reason"],
        "snapshot_consistency_not_independently_verified"
    );
    assert!(report["assurance_limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item.as_str()
                .unwrap_or_default()
                .contains("snapshot atomicity")
        }));
}

#[test]
fn unknown_snapshot_consistency_is_an_explicit_loss() {
    let mut input = base_input();
    input["snapshot"]["consistency_mode"] = json!("unknown");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "accepted_projection_with_loss");
    assert_eq!(
        report["dispositions"][0]["reason"],
        "snapshot_consistency_mode_unknown"
    );
}

#[test]
fn lowercase_timezone_designator_is_not_silently_normalized() {
    let mut input = base_input();
    input["observed_time"] = json!("2026-10-15T10:00:00z");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "reject");
    assert_eq!(
        report["dispositions"][0]["reason"],
        "timestamp_has_no_explicit_offset"
    );
}


#[test]
fn reports_an_auditable_rule_bound_decision_trace() {
    let report = evaluate(&base_input());
    assert_eq!(report["report_contract"], "omsi-manufacturing-semantic-report/0.1");
    assert_eq!(report["decision_authority"], "diagnostic_only");
    assert_eq!(report["quality_release_authority"], "none");
    assert_eq!(report["equipment_control_authority"], "none");
    assert_eq!(report["origin_authentication"], "not_performed");
    let trace = report["decision_trace"].as_array().expect("decision trace");

    for rule_id in [
        "omsi.manufacturing.input-contract/0.1",
        "omsi.manufacturing.tenant-scope/0.1",
        "omsi.manufacturing.source-instance-scope/0.1",
        "omsi.manufacturing.time-parse/0.1",
        "omsi.manufacturing.snapshot-consistency/0.1",
        COMPARISON_RULE_ID,
        "omsi.manufacturing.calibration-window/0.1",
        "omsi.manufacturing.quality-claim-nonescalation/0.1",
        "omsi.manufacturing.revision-provenance/0.1",
    ] {
        assert!(
            trace.iter().any(|check| check["rule_id"] == rule_id),
            "missing decision trace rule {rule_id}"
        );
    }

    assert!(trace.iter().all(|check| {
        check["result"].is_string()
            && check["inputs"].as_array().is_some_and(|items| !items.is_empty())
            && check["interpretation"].is_string()
    }));
    assert_eq!(
        trace
            .iter()
            .find(|check| check["rule_id"] == "omsi.manufacturing.quality-claim-nonescalation/0.1")
            .unwrap()["result"],
        "source_claim_preserved_unverified"
    );
}

#[test]
fn decimal_trailing_zero_metamorphism_preserves_the_semantic_result() {
    let mut baseline = base_input();
    baseline["measurement"]["value_decimal"] = json!("72.4");
    baseline["specification"]["lower_bound_decimal"] = json!("70");
    baseline["specification"]["upper_bound_decimal"] = json!("75");
    let baseline_report = evaluate(&baseline);

    for (value, lower, upper) in [
        ("72.4000", "70.000", "75.0000"),
        ("72.400000000", "70.000000", "75.000000000"),
        ("72.400000000000", "70.0", "75.000"),
    ] {
        let mut equivalent = base_input();
        equivalent["measurement"]["value_decimal"] = json!(value);
        equivalent["specification"]["lower_bound_decimal"] = json!(lower);
        equivalent["specification"]["upper_bound_decimal"] = json!(upper);
        assert_eq!(
            evaluate(&equivalent),
            baseline_report,
            "equivalent decimal spelling must not change semantic output: {value}"
        );
    }
}

#[test]
fn equivalent_timezone_offsets_preserve_the_semantic_result() {
    let baseline = evaluate(&base_input());
    for timestamp in [
        "2026-10-15T08:00:00Z",
        "2026-10-15T03:00:00-05:00",
        "2026-10-15T13:30:00+05:30",
        "2026-10-15T04:30:00-03:30",
        "2026-10-15T09:00:00+01:00",
    ] {
        let mut equivalent = base_input();
        equivalent["observed_time"] = json!(timestamp);
        assert_eq!(
            evaluate(&equivalent),
            baseline,
            "same UTC instant must produce same semantic result: {timestamp}"
        );
    }
}

#[test]
fn generated_nominal_values_respect_inclusive_bounds_and_never_escalate() {
    for hundredths in 6_950..=7_550 {
        let value = format!("{}.{:02}", hundredths / 100, hundredths % 100);
        let mut input = base_input();
        input["measurement"]["value_decimal"] = json!(value);
        input["specification"]["lower_bound_decimal"] = json!("70.000");
        input["specification"]["upper_bound_decimal"] = json!("75.000");

        let report = evaluate(&input);
        let expected_inside = (7_000..=7_500).contains(&hundredths);
        assert_eq!(
            report["measurement_evaluation"],
            if expected_inside {
                "inside_declared_bounds_point_estimate_only"
            } else {
                "outside_declared_bounds_point_estimate_only"
            },
            "value {value} and inclusive [70, 75]"
        );
        assert_eq!(
            report["outcome"],
            if expected_inside {
                "accepted_projection"
            } else {
                "preserve_with_conflict"
            },
            "value {value}"
        );
        assert_eq!(report["quality_release_authority"], "none");
        assert_eq!(report["equipment_control_authority"], "none");
    }
}

#[test]
fn malformed_json_value_shapes_never_panic_and_always_reject() {
    for malformed in [
        Value::Null,
        json!(true),
        json!(42),
        json!("not-an-object"),
        json!([]),
        json!({"source": []}),
        json!({"source": {}, "scope_binding": null}),
    ] {
        let report = evaluate(&malformed);
        assert_eq!(report["outcome"], "reject", "input: {malformed}");
        assert_eq!(report["projection_emitted"], false);
        assert!(report["decision_trace"].as_array().is_some());
    }
}

#[test]
fn decision_trace_exposes_unverified_source_snapshot_token() {
    let mut input = base_input();
    input["snapshot"]["consistency_mode"] = json!("source_snapshot_token");
    input["snapshot"]["source_watermark"] = json!("token-not-independent-proof");

    let report = evaluate(&input);
    let check = report["decision_trace"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["rule_id"] == "omsi.manufacturing.snapshot-consistency/0.1")
        .unwrap();

    assert_eq!(check["result"], "declared_token_not_independently_verified");
    assert!(check["interpretation"]
        .as_str()
        .unwrap()
        .contains("not treated as independent proof"));
}

#[test]
fn tenant_scope_rejection_is_visible_in_the_decision_trace() {
    let mut input = base_input();
    input["asset_ref"]["tenant_ref"] = json!("tenant:other");

    let report = evaluate(&input);
    assert_eq!(report["outcome"], "reject");
    let check = report["decision_trace"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["rule_id"] == "omsi.manufacturing.tenant-scope/0.1")
        .unwrap();
    assert_eq!(check["result"], "rejected");
}


const REPORT_SCHEMA_JSON: &str =
    include_str!("../schema/semantic-report.schema.json");

#[test]
fn semantic_report_contract_is_versioned_and_explicitly_non_authoritative() {
    let schema: Value =
        serde_json::from_str(REPORT_SCHEMA_JSON).expect("report schema must parse");
    assert_eq!(
        schema["$id"],
        "urn:luminous-dynamics:omsi:manufacturing-semantic-report:0.1"
    );
    assert_eq!(
        schema["properties"]["report_contract"]["const"],
        "omsi-manufacturing-semantic-report/0.1"
    );
    assert_eq!(schema["properties"]["decision_authority"]["const"], "diagnostic_only");
    assert_eq!(schema["properties"]["quality_release_authority"]["const"], "none");
    assert_eq!(schema["properties"]["equipment_control_authority"]["const"], "none");
    assert_eq!(schema["properties"]["origin_authentication"]["const"], "not_performed");
    assert!(schema["required"].as_array().unwrap().contains(&json!("decision_trace")));
}

#[test]
fn every_vector_report_has_required_trace_fields_and_bounded_authority() {
    let schema: Value =
        serde_json::from_str(REPORT_SCHEMA_JSON).expect("report schema must parse");
    let required: Vec<&str> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();

    for item in document()["vectors"].as_array().unwrap() {
        let report = evaluate(&item["input"]);
        for key in &required {
            assert!(report.get(*key).is_some(), "{} missing {key}", item["id"]);
        }
        assert_eq!(
            report["report_contract"],
            schema["properties"]["report_contract"]["const"]
        );
        assert_eq!(
            report["comparison_rule_id"],
            schema["properties"]["comparison_rule_id"]["const"]
        );
        assert!(schema["properties"]["outcome"]["enum"]
            .as_array()
            .unwrap()
            .contains(&report["outcome"]));
        assert!(report["decision_trace"].as_array().is_some_and(|trace| {
            !trace.is_empty()
                && trace.iter().all(|check| {
                    check["rule_id"].is_string()
                        && check["result"].is_string()
                        && check["interpretation"].is_string()
                        && check["inputs"].as_array().is_some_and(|inputs| {
                            !inputs.is_empty() && inputs.iter().all(Value::is_string)
                        })
                })
        }));
        assert_eq!(report["quality_release_authority"], "none");
        assert_eq!(report["equipment_control_authority"], "none");
        assert_eq!(report["origin_authentication"], "not_performed");
    }
}


#[test]
fn weakening_evidence_never_improves_the_report_outcome() {
    fn rank(outcome: &str) -> u8 {
        match outcome {
            "accepted_projection" => 0,
            "accepted_projection_with_loss" => 1,
            "preserve_with_conflict" => 2,
            "reject" => 3,
            other => panic!("unknown outcome: {other}"),
        }
    }

    let baseline = evaluate(&base_input());
    assert_eq!(baseline["outcome"], "accepted_projection");
    let baseline_rank = rank(baseline["outcome"].as_str().unwrap());

    let mut mutations = Vec::new();

    let mut missing_revision = base_input();
    missing_revision["source"]["record_revision"] = Value::Null;
    missing_revision["source_modified_time"] = Value::Null;
    mutations.push(("missing revision metadata", missing_revision));

    let mut unknown_snapshot = base_input();
    unknown_snapshot["snapshot"]["consistency_mode"] = json!("unknown");
    mutations.push(("unknown snapshot consistency", unknown_snapshot));

    let mut missing_unit = base_input();
    missing_unit["measurement"]["unit_code"] = Value::Null;
    mutations.push(("missing unit", missing_unit));

    let mut invalid_calibration = base_input();
    invalid_calibration["calibration"]["valid_until"] = json!("2026-01-01T00:00:00Z");
    mutations.push(("expired calibration", invalid_calibration));

    let mut wrong_tenant = base_input();
    wrong_tenant["asset_ref"]["tenant_ref"] = json!("tenant:other");
    mutations.push(("tenant mismatch", wrong_tenant));

    for (name, mutated) in mutations {
        let report = evaluate(&mutated);
        let actual_rank = rank(report["outcome"].as_str().unwrap());
        assert!(
            actual_rank >= baseline_rank,
            "weakening evidence via {name} must not improve outcome: {report:#}"
        );
    }
}


#[test]
fn raw_revision_metadata_is_not_lost_when_another_check_rejects_the_projection() {
    for id in ["MFG-OBS-004", "MFG-OBS-005"] {
        let input = vector(id)["input"].clone();
        let report = evaluate(&input);
        assert_eq!(report["outcome"], "reject");
        assert_eq!(
            report["source_revision_metadata"],
            "provided",
            "revision token presence is separate from projection eligibility for {id}"
        );
    }
}
