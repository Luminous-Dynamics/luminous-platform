use omsi_manufacturing_validator::evaluate_input_value;
use serde_json::{json, Value};

const VECTORS_JSON: &str = include_str!(
    "../../../docs/standards/omsi/profiles/manufacturing-asset-operations/0.1/CONFORMANCE-VECTORS.json"
);

fn input_for(vector_id: &str) -> Value {
    let document: Value =
        serde_json::from_str(VECTORS_JSON).expect("committed conformance vectors must parse");
    document["vectors"]
        .as_array()
        .expect("conformance vector document must contain vectors")
        .iter()
        .find(|vector| vector["id"] == vector_id)
        .unwrap_or_else(|| panic!("missing conformance vector {vector_id}"))["input"]
        .clone()
}

fn report_for(input: &Value) -> Value {
    evaluate_input_value(input)
}

fn has_disposition(report: &Value, kind: &str, field: &str) -> bool {
    report["dispositions"]
        .as_array()
        .is_some_and(|dispositions| {
            dispositions
                .iter()
                .any(|item| item["type"] == kind && item["field"] == field)
        })
}

#[test]
fn trailing_zero_decimal_rewrites_preserve_the_semantic_report() {
    let input = input_for("MFG-OBS-001");
    let expected = report_for(&input);

    // Deterministically exercise equivalent textual forms without changing the
    // represented decimal values or depending on randomness / external crates.
    for extra_zeroes in 1..=6 {
        let zeros = "0".repeat(extra_zeroes);
        let mut candidate = input.clone();
        candidate["measurement"]["value_decimal"] = json!(format!("72.4{zeros}"));
        candidate["specification"]["lower_bound_decimal"] = json!(format!("70.0{zeros}"));
        candidate["specification"]["upper_bound_decimal"] = json!(format!("75.0{zeros}"));

        assert_eq!(
            expected,
            report_for(&candidate),
            "equivalent decimal spellings changed the report with {extra_zeroes} extra zeroes"
        );
    }
}

#[test]
fn equivalent_explicit_time_offsets_preserve_the_semantic_report() {
    let input = input_for("MFG-OBS-001");
    let expected = report_for(&input);
    let mut equivalent = input;

    // 10:00 and 10:02 at UTC+02:00 are the same instants as 08:00 and 08:02Z.
    equivalent["observed_time"] = json!("2026-10-15T08:00:00Z");
    equivalent["source_modified_time"] = json!("2026-10-15T08:02:00Z");

    assert_eq!(
        expected,
        report_for(&equivalent),
        "equivalent explicit offsets changed the semantic result"
    );
}

#[test]
fn missing_unit_stays_not_evaluable_instead_of_defaulting() {
    let mut input = input_for("MFG-OBS-001");
    input["measurement"]["unit_code"] = Value::Null;

    let report = report_for(&input);
    assert_eq!(report["measurement_evaluation"], "not_evaluable");
    assert_eq!(report["outcome"], "preserve_with_conflict");
    assert!(has_disposition(
        &report,
        "conflict",
        "measurement.unit_code"
    ));
    assert_eq!(report["quality_release_authority"], "none");
}

#[test]
fn export_time_never_substitutes_for_missing_source_revision() {
    let mut input = input_for("MFG-OBS-007");
    assert!(input["source"]["record_revision"].is_null());

    let before = report_for(&input);
    input["exported_at"] = json!("2026-10-16T10:00:00+02:00");
    let after = report_for(&input);

    assert_eq!(before["source_revision_metadata"], "not_provided");
    assert_eq!(after["source_revision_metadata"], "not_provided");
    assert!(has_disposition(
        &after,
        "loss",
        "source.record_revision"
    ));
    assert_eq!(
        after["decision_trace"]
            .as_array()
            .and_then(|trace| trace.iter().find(|entry| {
                entry["rule_id"] == "omsi.manufacturing.revision-provenance/0.1"
            }))
            .map(|entry| entry["result"].clone()),
        Some(json!("revision_token_not_provided"))
    );
}

#[test]
fn tenant_mismatch_is_a_hard_rejection_for_every_scoped_reference() {
    let base = input_for("MFG-OBS-001");
    let scoped_references = [
        "/asset_ref",
        "/work_order_ref",
        "/measurement/instrument_ref",
        "/measurement/calibration_ref",
        "/specification/spec_ref",
    ];

    for pointer in scoped_references {
        let mut candidate = base.clone();
        candidate
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("missing scoped reference at {pointer}"))["tenant_ref"] =
            json!("tenant:synthetic-b");

        let report = report_for(&candidate);
        assert_eq!(
            report["outcome"], "reject",
            "tenant mismatch was not rejected at {pointer}: {report}"
        );
        assert_eq!(report["quality_release_authority"], "none");
        assert_eq!(report["equipment_control_authority"], "none");
        assert_eq!(report["projection_emitted"], false);
    }
}

#[test]
fn source_acceptance_cannot_override_an_out_of_bounds_measurement() {
    let input = input_for("MFG-OBS-006");
    assert_eq!(input["source_quality_claim"]["raw_status"], "Accepted");

    let report = report_for(&input);
    assert_eq!(
        report["measurement_evaluation"],
        "outside_declared_bounds_point_estimate_only"
    );
    assert_eq!(report["outcome"], "preserve_with_conflict");
    assert!(has_disposition(
        &report,
        "conflict",
        "source_quality_claim.raw_status"
    ));
    assert_eq!(report["quality_release_authority"], "none");
}

#[test]
fn uncertainty_does_not_silently_change_the_legacy_nominal_rule() {
    let mut input = input_for("MFG-OBS-001");
    input["measurement"]["uncertainty"]["value_decimal"] = json!("4.5");

    let report = report_for(&input);
    assert_eq!(
        report["measurement_evaluation"],
        "inside_declared_bounds_point_estimate_only"
    );
    assert!(report["assurance_limitations"]
        .as_array()
        .is_some_and(|items| items.iter().any(|item| {
            item.as_str()
                .is_some_and(|text| text.contains("nominal decimal value only"))
        })));
    assert_eq!(report["quality_release_authority"], "none");
}

#[test]
fn asserting_calibration_authenticity_from_this_validator_is_rejected() {
    let mut input = input_for("MFG-OBS-001");
    input["calibration"]["authenticity_verified_by_adapter"] = json!(true);

    let report = report_for(&input);
    assert_eq!(report["outcome"], "reject");
    assert!(has_disposition(&report, "reject", "calibration.authenticity_verified_by_adapter"));
    assert_eq!(report["quality_release_authority"], "none");
}

#[test]
fn unknown_root_fields_fail_closed_instead_of_being_silently_ignored() {
    let mut input = input_for("MFG-OBS-001");
    input["release_authority"] = json!(true);

    let report = report_for(&input);
    assert_eq!(report["outcome"], "reject");
    assert_eq!(report["projection_emitted"], false);
    assert_eq!(report["quality_release_authority"], "none");
    assert_eq!(report["equipment_control_authority"], "none");
}
