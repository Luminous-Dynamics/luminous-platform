use serde_json::{json, Map, Value};
use std::cmp::Ordering;
use std::collections::HashSet;

pub const COMPARISON_RULE_ID: &str = "nominal-point-estimate-range/0.1";

const LIMITATIONS: &[&str] = &[
    "Trusted tenant binding is assumed input; this validator does not authenticate its issuer or origin.",
    "Source status and calibration issuer claims are not authenticated by this validator.",
    "The comparison rule evaluates the nominal decimal value only; it does not apply uncertainty or authorize quality release.",
    "Export time does not establish source revision, source-clock trust, or snapshot atomicity.",
    "No machine-control, product-release, or other consequential authority is granted.",
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct Issue {
    field: String,
    reason: &'static str,
}

impl Issue {
    fn new(field: impl Into<String>, reason: &'static str) -> Self {
        Self {
            field: field.into(),
            reason,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Disposition {
    kind: &'static str,
    field: String,
    reason: &'static str,
}

impl Disposition {
    fn new(kind: &'static str, field: impl Into<String>, reason: &'static str) -> Self {
        Self {
            kind,
            field: field.into(),
            reason,
        }
    }

    fn to_value(&self) -> Value {
        json!({"type": self.kind, "field": self.field, "reason": self.reason})
    }
}

#[derive(Clone, Debug)]
pub struct Report {
    outcome: &'static str,
    dispositions: Vec<Disposition>,
    measurement_evaluation: Option<&'static str>,
    projection_emitted: Option<bool>,
    source_revision_metadata: Option<&'static str>,
}

impl Report {
    fn rejected(issue: Issue, source_revision_metadata: Option<&'static str>) -> Self {
        Self {
            outcome: "reject",
            dispositions: vec![Disposition::new("reject", issue.field, issue.reason)],
            measurement_evaluation: None,
            projection_emitted: Some(false),
            source_revision_metadata,
        }
    }

    fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("report_contract".into(), json!("omsi-manufacturing-semantic-report/0.1"));
        object.insert("profile".into(), json!("manufacturing-asset-operations/0.1-incubation"));
        object.insert("validator_version".into(), json!(env!("CARGO_PKG_VERSION")));
        object.insert("decision_authority".into(), json!("diagnostic_only"));
        object.insert("quality_release_authority".into(), json!("none"));
        object.insert("equipment_control_authority".into(), json!("none"));
        object.insert("origin_authentication".into(), json!("not_performed"));
        object.insert("outcome".into(), json!(self.outcome));
        object.insert(
            "dispositions".into(),
            Value::Array(
                self.dispositions
                    .iter()
                    .map(Disposition::to_value)
                    .collect(),
            ),
        );
        object.insert("comparison_rule_id".into(), json!(COMPARISON_RULE_ID));
        object.insert("assurance_limitations".into(), json!(LIMITATIONS));
        if let Some(value) = self.measurement_evaluation {
            object.insert("measurement_evaluation".into(), json!(value));
        }
        if let Some(value) = self.projection_emitted {
            object.insert("projection_emitted".into(), json!(value));
        }
        if let Some(value) = self.source_revision_metadata {
            object.insert("source_revision_metadata".into(), json!(value));
        }
        Value::Object(object)
    }
}

/// Evaluate one manufacturing-profile input. Invalid inputs return a stable
/// reject report; malformed JSON must be rejected by the caller.
pub fn evaluate_input(input: &Value) -> Report {
    match evaluate_inner(input) {
        Ok(report) => report,
        Err(issue) => Report::rejected(issue, raw_source_revision_metadata(input)),
    }
}

fn raw_source_revision_metadata(input: &Value) -> Option<&'static str> {
    match input.pointer("/source/record_revision") {
        Some(Value::String(value)) if !value.trim().is_empty() => Some("provided"),
        Some(Value::Null) => Some("not_provided"),
        _ => None,
    }
}

/// Public JSON representation of the semantic evaluation result.
pub fn evaluate_input_value(input: &Value) -> Value {
    let mut report = evaluate_input(input).to_value();
    let trace = decision_trace(input, &report);
    report["decision_trace"] = Value::Array(trace);
    report
}

fn decision_trace(input: &Value, report: &Value) -> Vec<Value> {
    let outcome = report.get("outcome").and_then(Value::as_str).unwrap_or("reject");
    let rejected = outcome == "reject";
    let dispositions = report.get("dispositions").and_then(Value::as_array);
    let rejected_field = dispositions
        .and_then(|items| items.iter().find(|item| item["type"] == "reject"))
        .and_then(|item| item["field"].as_str());
    let has_disposition = |field: &str, kind: &str| {
        dispositions.is_some_and(|items| {
            items.iter().any(|item| item["field"] == field && item["type"] == kind)
        })
    };
    let field_in = |prefix: &str| rejected_field.is_some_and(|field| field.starts_with(prefix));
    let mut trace = Vec::new();

    trace.push(json!({
        "rule_id": "omsi.manufacturing.input-contract/0.1",
        "result": if rejected { "rejected" } else { "satisfied" },
        "inputs": ["input"],
        "interpretation": "Only fields understood by this pinned profile are accepted."
    }));

    trace.push(json!({
        "rule_id": "omsi.manufacturing.tenant-scope/0.1",
        "result": if field_in("asset_ref.tenant_ref") || field_in("measurement.") && rejected_field.is_some_and(|field| field.ends_with(".tenant_ref")) {
            "rejected"
        } else if rejected {
            "not_established_due_to_rejection"
        } else {
            "matched_declared_binding"
        },
        "inputs": ["scope_binding.trusted_tenant_id", "asset_ref.tenant_ref", "measurement.instrument_ref.tenant_ref", "measurement.calibration_ref.tenant_ref", "specification.spec_ref.tenant_ref"],
        "interpretation": "The validator compares declarations; it does not authenticate the binding or source."
    }));

    trace.push(json!({
        "rule_id": "omsi.manufacturing.source-instance-scope/0.1",
        "result": if field_in("asset_ref.source_instance") || field_in("work_order_ref.source_instance") || field_in("measurement.instrument_ref.source_instance") || field_in("measurement.calibration_ref.source_instance") || field_in("specification.spec_ref.source_instance") {
            "rejected"
        } else if rejected {
            "not_established_due_to_rejection"
        } else {
            "all_checked_references_in_namespace"
        },
        "inputs": ["source.instance_namespace", "asset_ref.source_instance", "work_order_ref.source_instance", "measurement.instrument_ref.source_instance", "measurement.calibration_ref.source_instance", "specification.spec_ref.source_instance"],
        "interpretation": "Matching a namespace is not proof of physical identity."
    }));

    let time_field_rejected = rejected_field.is_some_and(|field| {
        field.ends_with("_time")
            || field == "observed_time"
            || field == "exported_at"
            || field.starts_with("snapshot.extraction_")
            || field.starts_with("calibration.valid_")
    });
    trace.push(json!({
        "rule_id": "omsi.manufacturing.time-parse/0.1",
        "result": if time_field_rejected { "rejected" } else if rejected { "not_established_due_to_rejection" } else { "offsets_parsed_and_normalized" },
        "inputs": ["observed_time", "source_modified_time", "exported_at", "snapshot.extraction_started_at", "snapshot.extraction_completed_at", "calibration.valid_from", "calibration.valid_until"],
        "interpretation": "Timestamp syntax and supplied offsets are checked; source clock accuracy and causality are not established."
    }));

    let snapshot_mode = input
        .pointer("/snapshot/consistency_mode")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let snapshot_result = if has_disposition("snapshot.consistency_mode", "reject") {
        "rejected"
    } else if has_disposition("snapshot.source_watermark", "loss") {
        "declared_token_not_independently_verified"
    } else if snapshot_mode == "unknown" {
        "unknown"
    } else if snapshot_mode == "per_record_non_atomic" {
        "explicitly_non_atomic"
    } else if rejected {
        "not_established_due_to_rejection"
    } else {
        "declaration_present_not_independently_verified"
    };
    trace.push(json!({
        "rule_id": "omsi.manufacturing.snapshot-consistency/0.1",
        "result": snapshot_result,
        "inputs": ["snapshot.consistency_mode", "snapshot.source_watermark"],
        "interpretation": "A source-declared token is preserved but is not treated as independent proof of atomic consistency."
    }));

    let measurement = report.get("measurement_evaluation")
        .and_then(Value::as_str)
        .unwrap_or("not_evaluated");
    trace.push(json!({
        "rule_id": COMPARISON_RULE_ID,
        "result": measurement,
        "inputs": ["measurement.value_decimal", "measurement.unit_system", "measurement.unit_code", "specification.lower_bound_decimal", "specification.upper_bound_decimal", "specification.unit_system", "specification.unit_code"],
        "interpretation": "Nominal-point comparison only; no uncertainty calculation, unit conversion, or release decision."
    }));

    let calibration_result = if has_disposition("calibration", "conflict") {
        "validity_or_source_status_conflict"
    } else if rejected && rejected_field.is_some_and(|field| field.starts_with("calibration.")) {
        "rejected"
    } else if rejected {
        "not_established_due_to_rejection"
    } else {
        "interval_checked_source_claim_unauthenticated"
    };
    trace.push(json!({
        "rule_id": "omsi.manufacturing.calibration-window/0.1",
        "result": calibration_result,
        "inputs": ["calibration.source_status_claim", "calibration.valid_from", "calibration.valid_until", "observed_time", "calibration.authenticity_verified_by_adapter"],
        "interpretation": "Interval coverage is checked; certificate origin, procedure and instrument accuracy are not authenticated."
    }));

    let quality_result = if rejected_field == Some("source_quality_claim") {
        "rejected_assurance_upgrade"
    } else if rejected {
        "not_established_due_to_rejection"
    } else if has_disposition("source_quality_claim.raw_status", "conflict") {
        "source_claim_preserved_with_conflict"
    } else {
        "source_claim_preserved_unverified"
    };
    trace.push(json!({
        "rule_id": "omsi.manufacturing.quality-claim-nonescalation/0.1",
        "result": quality_result,
        "inputs": ["source_quality_claim.raw_status", "source_quality_claim.source_claim_only", "source_quality_claim.independent_quality_release"],
        "interpretation": "A source status is never promoted to independently verified quality acceptance or release authority."
    }));

    let revision_result = match report.get("source_revision_metadata").and_then(Value::as_str) {
        Some("provided") if rejected => "revision_token_supplied_projection_rejected",
        Some("provided") => "revision_token_present",
        Some("not_provided") if rejected => "revision_token_absent_projection_rejected",
        Some("not_provided") => "revision_token_not_provided",
        _ => "not_established_due_to_rejection",
    };
    trace.push(json!({
        "rule_id": "omsi.manufacturing.revision-provenance/0.1",
        "result": revision_result,
        "inputs": ["source.record_revision", "source_modified_time", "exported_at"],
        "interpretation": "Export time is not a substitute for source revision or freshness."
    }));

    if let Some(items) = dispositions {
        for item in items {
            trace.push(json!({
                "rule_id": "omsi.manufacturing.disposition/0.1",
                "result": item["type"],
                "inputs": [item["field"]],
                "reason": item["reason"],
                "interpretation": "This disposition records the bounded rule outcome; it does not grant operational authority."
            }));
        }
    }
    trace
}

/// Execute the committed machine-readable expectations against the real
/// semantic evaluator. Free-text assertions are informative only and are not
/// treated as executable rules.
pub fn validate_vector_document(document: &Value) -> Value {
    let mut failures: Vec<Value> = Vec::new();
    if let Err(issue) = shape(
        document,
        "$",
        &[
            "format",
            "format_version",
            "status",
            "profile",
            "common_rules",
            "vectors",
        ],
        &[],
    ) {
        failures.push(issue_value(issue));
        return summary_value(0, failures);
    }
    let Some(root) = document.as_object() else {
        failures.push(issue_value(Issue::new("$", "expected_object")));
        return summary_value(0, failures);
    };

    for (field, expected) in [
        ("format", "omsi-manufacturing-conformance-vectors"),
        ("format_version", "0.1"),
        (
            "status",
            "synthetic research fixture set; not a normative OMSI wire schema",
        ),
        ("profile", "manufacturing-asset-operations/0.1-incubation"),
    ] {
        if root.get(field).and_then(Value::as_str) != Some(expected) {
            failures.push(json!({
                "field": field,
                "reason": "unexpected_document_metadata",
                "expected": expected
            }));
        }
    }

    match root.get("common_rules").and_then(Value::as_array) {
        Some(rules) if !rules.is_empty() && rules.iter().all(Value::is_string) => {}
        _ => failures.push(json!({
            "field": "common_rules",
            "reason": "expected_nonempty_string_array"
        })),
    }

    let Some(vectors) = root.get("vectors").and_then(Value::as_array) else {
        failures.push(json!({"field":"vectors","reason":"expected_array"}));
        return summary_value(0, failures);
    };
    if vectors.is_empty() {
        failures.push(json!({"field":"vectors","reason":"empty_vector_set"}));
        return summary_value(0, failures);
    }

    let mut ids = HashSet::new();
    let mut passed = 0usize;

    for (index, vector) in vectors.iter().enumerate() {
        let base = format!("vectors[{index}]");
        if let Err(issue) = shape(
            vector,
            &base,
            &["id", "label", "classification", "input", "expected"],
            &[],
        ) {
            failures.push(issue_value(issue));
            continue;
        }
        let Some(vector_object) = vector.as_object() else {
            failures.push(json!({"field":base,"reason":"expected_object"}));
            continue;
        };
        let Some(id) = vector_object.get("id").and_then(Value::as_str) else {
            failures.push(json!({"field":format!("{base}.id"),"reason":"expected_string"}));
            continue;
        };
        if vector_object
            .get("label")
            .and_then(Value::as_str)
            .map_or(true, |label| label.trim().is_empty())
        {
            failures.push(json!({
                "vector_id": id,
                "field": format!("{base}.label"),
                "reason": "expected_nonempty_string"
            }));
            continue;
        }
        if id.trim().is_empty() || !ids.insert(id.to_owned()) {
            failures.push(json!({
                "vector_id": id,
                "field": format!("{base}.id"),
                "reason": "empty_or_duplicate_vector_id"
            }));
            continue;
        }

        let classification = vector_object.get("classification").and_then(Value::as_str);
        if !matches!(classification, Some("positive" | "negative_control")) {
            failures.push(json!({
                "vector_id": id,
                "field": format!("{base}.classification"),
                "reason": "unknown_classification"
            }));
            continue;
        }

        let expected = vector_object.get("expected").expect("shape checked");
        if let Err(issue) = shape(
            expected,
            &format!("{base}.expected"),
            &[
                "outcome",
                "dispositions",
                "assertions",
                "comparison_rule_id",
                "source_revision_metadata",
            ],
            &["measurement_evaluation", "projection_emitted"],
        ) {
            failures.push(json!({"vector_id":id,"issue":issue_value(issue)}));
            continue;
        }
        let Some(expected_object) = expected.as_object() else {
            failures.push(json!({"vector_id":id,"reason":"expected_result_not_object"}));
            continue;
        };

        let assertions_valid = expected_object
            .get("assertions")
            .and_then(Value::as_array)
            .is_some_and(|items| {
                !items.is_empty()
                    && items.iter().all(|item| {
                        item.as_str()
                            .is_some_and(|assertion| !assertion.trim().is_empty())
                    })
            });
        if !assertions_valid {
            failures.push(json!({
                "vector_id": id,
                "field": format!("{base}.expected.assertions"),
                "reason": "expected_nonempty_string_array"
            }));
            continue;
        }

        if expected_object
            .get("comparison_rule_id")
            .and_then(Value::as_str)
            != Some(COMPARISON_RULE_ID)
        {
            failures.push(json!({
                "vector_id": id,
                "field": format!("{base}.expected.comparison_rule_id"),
                "reason": "comparison_rule_id_mismatch",
                "expected_rule": COMPARISON_RULE_ID
            }));
            continue;
        }

        let report = evaluate_input_value(vector_object.get("input").expect("shape checked"));
        let mut case_errors: Vec<Value> = Vec::new();

        for field in ["outcome", "dispositions", "comparison_rule_id"] {
            if report.get(field) != expected_object.get(field) {
                case_errors.push(json!({
                    "field": field,
                    "expected": expected_object.get(field),
                    "actual": report.get(field)
                }));
            }
        }

        for field in ["measurement_evaluation", "projection_emitted", "source_revision_metadata"] {
            if let Some(expected_value) = expected_object.get(field) {
                if report.get(field) != Some(expected_value) {
                    case_errors.push(json!({
                        "field": field,
                        "expected": expected_value,
                        "actual": report.get(field)
                    }));
                }
            }
        }

        if classification == Some("positive")
            && expected_object.get("outcome").and_then(Value::as_str) != Some("accepted_projection")
        {
            case_errors.push(json!({
                "field": "classification",
                "reason": "positive_vector_must_expect_unqualified_projection"
            }));
        }
        if classification == Some("negative_control")
            && expected_object.get("outcome").and_then(Value::as_str) == Some("accepted_projection")
        {
            case_errors.push(json!({
                "field": "classification",
                "reason": "negative_control_cannot_expect_clean_projection"
            }));
        }

        if case_errors.is_empty() {
            passed += 1;
        } else {
            failures.push(json!({
                "vector_id": id,
                "computed": report,
                "mismatches": case_errors
            }));
        }
    }

    let failed_count = vectors.len().saturating_sub(passed);
    json!({
        "status": if failures.is_empty() { "passed" } else { "failed" },
        "vector_count": vectors.len(),
        "passed_count": passed,
        "failed_count": failed_count,
        "failures": failures
    })
}

fn summary_value(vector_count: usize, failures: Vec<Value>) -> Value {
    let failed_count = failures.len();
    json!({
        "status": if failed_count == 0 { "passed" } else { "failed" },
        "vector_count": vector_count,
        "passed_count": vector_count.saturating_sub(failed_count),
        "failed_count": failed_count,
        "failures": failures
    })
}

fn issue_value(issue: Issue) -> Value {
    json!({"field": issue.field, "reason": issue.reason})
}

fn evaluate_inner(input: &Value) -> Result<Report, Issue> {
    let root = shape(
        input,
        "input",
        &[
            "source",
            "scope_binding",
            "asset_ref",
            "work_order_ref",
            "observed_time",
            "source_modified_time",
            "exported_at",
            "snapshot",
            "measurement",
            "calibration",
            "specification",
            "source_quality_claim",
        ],
        &[],
    )?;

    let source = child_object(
        root,
        "source",
        "source",
        &[
            "system",
            "instance_namespace",
            "release_pin",
            "object_type",
            "external_id",
            "record_revision",
        ],
        &[],
    )?;
    let source_namespace =
        required_string(source, "instance_namespace", "source.instance_namespace")?;
    for field in ["system", "release_pin", "object_type", "external_id"] {
        required_string(source, field, &format!("source.{field}"))?;
    }
    let record_revision = nullable_string(source, "record_revision", "source.record_revision")?;

    let binding = child_object(
        root,
        "scope_binding",
        "scope_binding",
        &["binding_id", "trusted_tenant_id", "source_site_ref"],
        &[],
    )?;
    required_string(binding, "binding_id", "scope_binding.binding_id")?;
    let trusted_tenant = required_string(
        binding,
        "trusted_tenant_id",
        "scope_binding.trusted_tenant_id",
    )?;
    required_string(binding, "source_site_ref", "scope_binding.source_site_ref")?;

    let asset = child_object(
        root,
        "asset_ref",
        "asset_ref",
        &["source_instance", "object_type", "external_id"],
        &["tenant_ref"],
    )?;
    let asset_tenant = match asset.get("tenant_ref") {
        Some(Value::String(value)) if !value.is_empty() => value.as_str(),
        Some(Value::String(_)) => {
            return Err(Issue::new(
                "asset_ref.tenant_ref",
                "empty_string_not_allowed",
            ))
        }
        Some(_) => return Err(Issue::new("asset_ref.tenant_ref", "expected_string")),
        None => return Err(Issue::new("asset_ref.tenant_ref", "trusted_scope_missing")),
    };
    if asset_tenant.trim().is_empty() {
        return Err(Issue::new("asset_ref.tenant_ref", "trusted_scope_missing"));
    }
    if asset_tenant != trusted_tenant {
        return Err(Issue::new("asset_ref.tenant_ref", "trusted_scope_mismatch"));
    }

    let work_order = child_object(
        root,
        "work_order_ref",
        "work_order_ref",
        &["source_instance", "object_type", "external_id"],
        &["tenant_ref"],
    )?;
    for (name, reference) in [("asset_ref", asset), ("work_order_ref", work_order)] {
        validate_source_reference(reference, name, source_namespace, trusted_tenant)?;
    }

    let observed_text = required_string(root, "observed_time", "observed_time")?;
    let observed =
        parse_timestamp(observed_text).map_err(|reason| Issue::new("observed_time", reason))?;
    if root
        .get("source_modified_time")
        .and_then(Value::as_str)
        .is_some()
    {
        let text = required_string(root, "source_modified_time", "source_modified_time")?;
        parse_timestamp(text).map_err(|reason| Issue::new("source_modified_time", reason))?;
    } else if !root.get("source_modified_time").is_some_and(Value::is_null) {
        return Err(Issue::new(
            "source_modified_time",
            "expected_timestamp_or_null",
        ));
    }
    let exported_text = required_string(root, "exported_at", "exported_at")?;
    parse_timestamp(exported_text).map_err(|reason| Issue::new("exported_at", reason))?;

    let snapshot = child_object(
        root,
        "snapshot",
        "snapshot",
        &[
            "export_id",
            "consistency_mode",
            "extraction_started_at",
            "extraction_completed_at",
            "source_watermark",
        ],
        &[],
    )?;
    required_string(snapshot, "export_id", "snapshot.export_id")?;
    let consistency = required_string(snapshot, "consistency_mode", "snapshot.consistency_mode")?;
    if !matches!(
        consistency,
        "per_record_non_atomic" | "source_snapshot_token" | "transactional_snapshot" | "unknown"
    ) {
        return Err(Issue::new(
            "snapshot.consistency_mode",
            "unknown_consistency_mode",
        ));
    }
    let watermark = nullable_string(snapshot, "source_watermark", "snapshot.source_watermark")?;
    if matches!(
        consistency,
        "source_snapshot_token" | "transactional_snapshot"
    ) && watermark.is_none()
    {
        return Err(Issue::new(
            "snapshot.consistency_mode",
            "snapshot_token_missing",
        ));
    }
    let snapshot_consistency_loss = match consistency {
        "source_snapshot_token" | "transactional_snapshot" => Some((
            "snapshot.source_watermark",
            "snapshot_consistency_not_independently_verified",
        )),
        "unknown" => Some((
            "snapshot.consistency_mode",
            "snapshot_consistency_mode_unknown",
        )),
        _ => None,
    };
    let extraction_start_text = required_string(
        snapshot,
        "extraction_started_at",
        "snapshot.extraction_started_at",
    )?;
    let extraction_start = parse_timestamp(extraction_start_text)
        .map_err(|reason| Issue::new("snapshot.extraction_started_at", reason))?;
    let extraction_end_text = required_string(
        snapshot,
        "extraction_completed_at",
        "snapshot.extraction_completed_at",
    )?;
    let extraction_end = parse_timestamp(extraction_end_text)
        .map_err(|reason| Issue::new("snapshot.extraction_completed_at", reason))?;
    if extraction_end < extraction_start {
        return Err(Issue::new(
            "snapshot.extraction_completed_at",
            "snapshot_extraction_interval_inverted",
        ));
    }

    let measurement = child_object(
        root,
        "measurement",
        "measurement",
        &[
            "quantity_kind",
            "value_decimal",
            "unit_system",
            "unit_code",
            "instrument_ref",
            "calibration_ref",
            "method_ref",
            "uncertainty",
        ],
        &[],
    )?;
    for field in ["quantity_kind", "unit_system", "method_ref"] {
        required_string(measurement, field, &format!("measurement.{field}"))?;
    }
    let value_text = required_string(measurement, "value_decimal", "measurement.value_decimal")?;
    let value = Decimal::parse(value_text)
        .map_err(|reason| Issue::new("measurement.value_decimal", reason))?;
    let unit_system = required_string(measurement, "unit_system", "measurement.unit_system")?;
    let unit_code = nullable_string(measurement, "unit_code", "measurement.unit_code")?;

    let instrument = child_object(
        measurement,
        "instrument_ref",
        "measurement.instrument_ref",
        &["source_instance", "object_type", "external_id"],
        &["tenant_ref"],
    )?;
    let calibration_ref = child_object(
        measurement,
        "calibration_ref",
        "measurement.calibration_ref",
        &["source_instance", "object_type", "external_id"],
        &["tenant_ref"],
    )?;
    validate_source_reference(
        instrument,
        "measurement.instrument_ref",
        source_namespace,
        trusted_tenant,
    )?;
    validate_source_reference(
        calibration_ref,
        "measurement.calibration_ref",
        source_namespace,
        trusted_tenant,
    )?;

    let uncertainty = child_object(
        measurement,
        "uncertainty",
        "measurement.uncertainty",
        &["value_decimal", "unit_code", "meaning"],
        &[],
    )?;
    let uncertainty_text = required_string(
        uncertainty,
        "value_decimal",
        "measurement.uncertainty.value_decimal",
    )?;
    let uncertainty_value = Decimal::parse(uncertainty_text)
        .map_err(|reason| Issue::new("measurement.uncertainty.value_decimal", reason))?;
    if uncertainty_value.is_negative() {
        return Err(Issue::new(
            "measurement.uncertainty.value_decimal",
            "uncertainty_must_not_be_negative",
        ));
    }
    required_string(
        uncertainty,
        "unit_code",
        "measurement.uncertainty.unit_code",
    )?;
    required_string(uncertainty, "meaning", "measurement.uncertainty.meaning")?;

    let calibration = child_object(
        root,
        "calibration",
        "calibration",
        &[
            "source_status_claim",
            "valid_from",
            "valid_until",
            "issuer_ref",
            "authenticity_verified_by_adapter",
        ],
        &[],
    )?;
    let calibration_status = required_string(
        calibration,
        "source_status_claim",
        "calibration.source_status_claim",
    )?;
    let valid_from_text = required_string(calibration, "valid_from", "calibration.valid_from")?;
    let valid_from = parse_timestamp(valid_from_text)
        .map_err(|reason| Issue::new("calibration.valid_from", reason))?;
    let valid_until_text = required_string(calibration, "valid_until", "calibration.valid_until")?;
    let valid_until = parse_timestamp(valid_until_text)
        .map_err(|reason| Issue::new("calibration.valid_until", reason))?;
    required_string(calibration, "issuer_ref", "calibration.issuer_ref")?;
    if valid_until <= valid_from {
        return Err(Issue::new(
            "calibration.valid_until",
            "calibration_interval_empty_or_inverted",
        ));
    }
    if calibration
        .get("authenticity_verified_by_adapter")
        .and_then(Value::as_bool)
        != Some(false)
    {
        return Err(Issue::new(
            "calibration.authenticity_verified_by_adapter",
            "calibration_authenticity_cannot_be_asserted_by_this_validator",
        ));
    }

    let specification = child_object(
        root,
        "specification",
        "specification",
        &[
            "spec_ref",
            "revision",
            "lower_bound_decimal",
            "upper_bound_decimal",
            "unit_system",
            "unit_code",
        ],
        &[],
    )?;
    let spec_ref = child_object(
        specification,
        "spec_ref",
        "specification.spec_ref",
        &["source_instance", "object_type", "external_id"],
        &["tenant_ref"],
    )?;
    validate_source_reference(
        spec_ref,
        "specification.spec_ref",
        source_namespace,
        trusted_tenant,
    )?;
    required_string(specification, "revision", "specification.revision")?;
    let spec_unit_system =
        required_string(specification, "unit_system", "specification.unit_system")?;
    let spec_unit_code = required_string(specification, "unit_code", "specification.unit_code")?;
    let lower_text = required_string(
        specification,
        "lower_bound_decimal",
        "specification.lower_bound_decimal",
    )?;
    let upper_text = required_string(
        specification,
        "upper_bound_decimal",
        "specification.upper_bound_decimal",
    )?;
    let lower = Decimal::parse(lower_text)
        .map_err(|reason| Issue::new("specification.lower_bound_decimal", reason))?;
    let upper = Decimal::parse(upper_text)
        .map_err(|reason| Issue::new("specification.upper_bound_decimal", reason))?;
    if upper.cmp(&lower) == Ordering::Less {
        return Err(Issue::new(
            "specification.upper_bound_decimal",
            "specification_bounds_inverted",
        ));
    }

    let quality = child_object(
        root,
        "source_quality_claim",
        "source_quality_claim",
        &[
            "raw_status",
            "source_claim_only",
            "independent_quality_release",
        ],
        &[],
    )?;
    let raw_status = required_string(quality, "raw_status", "source_quality_claim.raw_status")?;
    if quality.get("source_claim_only").and_then(Value::as_bool) != Some(true)
        || quality
            .get("independent_quality_release")
            .and_then(Value::as_bool)
            != Some(false)
    {
        return Err(Issue::new(
            "source_quality_claim",
            "quality_claim_assurance_upgrade_not_permitted",
        ));
    }

    let mut dispositions = Vec::new();
    if let Some((field, reason)) = snapshot_consistency_loss {
        dispositions.push(Disposition::new("loss", field, reason));
    }
    let measurement_evaluation = match unit_code {
        None => {
            dispositions.push(Disposition::new(
                "conflict",
                "measurement.unit_code",
                "measurement_unit_missing",
            ));
            "not_evaluable"
        }
        Some(code) if unit_system != spec_unit_system || code != spec_unit_code => {
            dispositions.push(Disposition::new(
                "conflict",
                "measurement.unit_code",
                "measurement_unit_incompatible_with_specification",
            ));
            "not_evaluable"
        }
        Some(_) => match (value.cmp(&lower), value.cmp(&upper)) {
            (Ordering::Less, _) | (_, Ordering::Greater) => {
                "outside_declared_bounds_point_estimate_only"
            }
            _ => "inside_declared_bounds_point_estimate_only",
        },
    };

    let calibration_interval_covers_observation = valid_from <= observed && observed < valid_until;
    let calibration_conflict = if calibration_status != "valid" {
        Some("calibration_source_status_not_valid")
    } else if !calibration_interval_covers_observation {
        Some("calibration_validity_interval_does_not_cover_observation")
    } else {
        None
    };
    if let Some(reason) = calibration_conflict {
        dispositions.push(Disposition::new("conflict", "calibration", reason));
    }

    let mut final_measurement_evaluation = measurement_evaluation;
    if calibration_conflict.is_some()
        && measurement_evaluation == "inside_declared_bounds_point_estimate_only"
    {
        final_measurement_evaluation =
            "inside_declared_bounds_point_estimate_only_calibration_assurance_limited";
    }

    if measurement_evaluation == "outside_declared_bounds_point_estimate_only"
        && raw_status == "Accepted"
    {
        dispositions.push(Disposition::new(
            "conflict",
            "source_quality_claim.raw_status",
            "source_acceptance_disagrees_with_profile_comparison",
        ));
    }

    let source_revision_metadata = if record_revision.is_some() {
        Some("provided")
    } else {
        dispositions.push(Disposition::new(
            "loss",
            "source.record_revision",
            "source_record_revision_not_provided",
        ));
        Some("not_provided")
    };

    let has_conflict = dispositions
        .iter()
        .any(|disposition| disposition.kind == "conflict");
    let has_loss = dispositions
        .iter()
        .any(|disposition| disposition.kind == "loss");
    let outcome = if has_conflict {
        "preserve_with_conflict"
    } else if has_loss {
        "accepted_projection_with_loss"
    } else {
        "accepted_projection"
    };

    Ok(Report {
        outcome,
        dispositions,
        measurement_evaluation: Some(final_measurement_evaluation),
        projection_emitted: None,
        source_revision_metadata,
    })
}

fn validate_source_reference(
    reference: &Map<String, Value>,
    path: &str,
    expected_namespace: &str,
    expected_tenant: &str,
) -> Result<(), Issue> {
    let namespace = required_string(
        reference,
        "source_instance",
        &format!("{path}.source_instance"),
    )?;
    required_string(reference, "object_type", &format!("{path}.object_type"))?;
    required_string(reference, "external_id", &format!("{path}.external_id"))?;
    if namespace != expected_namespace {
        return Err(Issue::new(
            format!("{path}.source_instance"),
            "source_instance_mismatch",
        ));
    }
    if let Some(tenant) = reference.get("tenant_ref") {
        match tenant {
            Value::String(value) if !value.trim().is_empty() && value == expected_tenant => {}
            Value::String(value) if value.trim().is_empty() => {
                return Err(Issue::new(
                    format!("{path}.tenant_ref"),
                    "expected_nonempty_string",
                ));
            }
            Value::String(_) => {
                return Err(Issue::new(
                    format!("{path}.tenant_ref"),
                    "trusted_scope_mismatch",
                ));
            }
            _ => {
                return Err(Issue::new(
                    format!("{path}.tenant_ref"),
                    "expected_nonempty_string",
                ))
            }
        }
    }
    Ok(())
}

fn child_object<'a>(
    parent: &'a Map<String, Value>,
    key: &str,
    path: &str,
    required: &[&str],
    optional: &[&str],
) -> Result<&'a Map<String, Value>, Issue> {
    let value = parent
        .get(key)
        .ok_or(Issue::new(path, "required_field_missing"))?;
    shape(value, path, required, optional)
}

fn shape<'a>(
    value: &'a Value,
    path: &str,
    required: &[&str],
    optional: &[&str],
) -> Result<&'a Map<String, Value>, Issue> {
    let object = value
        .as_object()
        .ok_or(Issue::new(path, "expected_object"))?;
    for key in required {
        if !object.contains_key(*key) {
            return Err(Issue::new(
                format!("{path}.{key}"),
                "required_field_missing",
            ));
        }
    }
    for key in object.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(Issue::new(format!("{path}.{key}"), "unexpected_field"));
        }
    }
    Ok(object)
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<&'a str, Issue> {
    match object.get(key) {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(value),
        Some(Value::String(_)) => Err(Issue::new(path, "empty_string_not_allowed")),
        Some(_) => Err(Issue::new(path, "expected_string")),
        None => Err(Issue::new(path, "required_field_missing")),
    }
}

fn nullable_string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<Option<&'a str>, Issue> {
    match object.get(key) {
        Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(Some(value)),
        Some(Value::String(_)) => Err(Issue::new(path, "empty_string_not_canonical")),
        Some(_) => Err(Issue::new(path, "expected_string_or_null")),
        None => Err(Issue::new(path, "required_field_missing")),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Decimal {
    coefficient: i128,
    scale: u32,
}

impl Decimal {
    fn parse(text: &str) -> Result<Self, &'static str> {
        if text.is_empty() || !text.is_ascii() || text.starts_with('+') {
            return Err("decimal_not_in_profile");
        }
        let (negative, unsigned) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        if unsigned.is_empty() {
            return Err("decimal_not_in_profile");
        }
        let mut parts = unsigned.split('.');
        let whole = parts.next().ok_or("decimal_not_in_profile")?;
        let fraction = parts.next();
        if parts.next().is_some()
            || whole.is_empty()
            || !whole.bytes().all(|byte| byte.is_ascii_digit())
            || (whole.len() > 1 && whole.starts_with('0'))
        {
            return Err("decimal_not_in_profile");
        }
        if whole.len() > 18 {
            return Err("decimal_out_of_supported_precision");
        }
        let (fraction, scale) = match fraction {
            None => ("", 0u32),
            Some(value)
                if !value.is_empty()
                    && value.bytes().all(|byte| byte.is_ascii_digit())
                    && value.len() <= 18 =>
            {
                (value, value.len() as u32)
            }
            Some(_) => return Err("decimal_not_in_profile"),
        };

        let digits = format!("{whole}{fraction}");
        let mut coefficient = digits
            .parse::<i128>()
            .map_err(|_| "decimal_out_of_supported_precision")?;
        if negative {
            coefficient = -coefficient;
        }
        let mut value = Self { coefficient, scale };
        while value.scale > 0 && value.coefficient % 10 == 0 {
            value.coefficient /= 10;
            value.scale -= 1;
        }
        Ok(value)
    }

    fn is_negative(self) -> bool {
        self.coefficient < 0
    }

}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        let scale = self.scale.max(other.scale);
        let left = self.coefficient * pow10(scale - self.scale);
        let right = other.coefficient * pow10(scale - other.scale);
        left.cmp(&right)
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn pow10(exponent: u32) -> i128 {
    let mut value = 1i128;
    for _ in 0..exponent {
        value *= 10;
    }
    value
}

/// Parse the profile's narrow timestamp syntax and normalize it to UTC
/// nanoseconds. This function never consults the host timezone.
fn parse_timestamp(text: &str) -> Result<i128, &'static str> {
    if !text.is_ascii() {
        return Err("timestamp_invalid_or_out_of_profile");
    }
    let t_index = text.find('T').ok_or("timestamp_has_no_explicit_offset")?;
    if t_index != 10 || text.len() < 19 {
        return Err("timestamp_invalid_or_out_of_profile");
    }
    let date = &text[..t_index];
    let time_and_zone = &text[t_index + 1..];

    let (time, offset_minutes) = if let Some(time) = time_and_zone.strip_suffix('Z') {
        (time, 0i64)
    } else {
        let plus = time_and_zone.rfind('+');
        let minus = time_and_zone.rfind('-');
        let split = match (plus, minus) {
            (Some(p), Some(m)) => Some(p.max(m)),
            (Some(p), None) => Some(p),
            (None, Some(m)) => Some(m),
            (None, None) => None,
        }
        .ok_or("timestamp_has_no_explicit_offset")?;
        let offset = &time_and_zone[split..];
        if offset.len() != 6 || &offset[3..4] != ":" {
            return Err("timestamp_invalid_or_out_of_profile");
        }
        if !offset[1..3].bytes().all(|b| b.is_ascii_digit())
            || !offset[4..6].bytes().all(|b| b.is_ascii_digit())
        {
            return Err("timestamp_invalid_or_out_of_profile");
        }
        let hours = offset[1..3]
            .parse::<i64>()
            .map_err(|_| "timestamp_invalid_or_out_of_profile")?;
        let minutes = offset[4..6]
            .parse::<i64>()
            .map_err(|_| "timestamp_invalid_or_out_of_profile")?;
        if hours > 23 || minutes > 59 {
            return Err("timestamp_invalid_or_out_of_profile");
        }
        let magnitude = hours * 60 + minutes;
        let signed = if offset.starts_with('+') {
            magnitude
        } else {
            -magnitude
        };
        (&time_and_zone[..split], signed)
    };

    let date_parts: Vec<&str> = date.split('-').collect();
    if date_parts.len() != 3
        || date_parts[0].len() != 4
        || date_parts[1].len() != 2
        || date_parts[2].len() != 2
        || !date_parts
            .iter()
            .all(|part| part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err("timestamp_invalid_or_out_of_profile");
    }
    let year = date_parts[0]
        .parse::<i64>()
        .map_err(|_| "timestamp_invalid_or_out_of_profile")?;
    let month = date_parts[1]
        .parse::<u32>()
        .map_err(|_| "timestamp_invalid_or_out_of_profile")?;
    let day = date_parts[2]
        .parse::<u32>()
        .map_err(|_| "timestamp_invalid_or_out_of_profile")?;
    if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
        return Err("timestamp_invalid_or_out_of_profile");
    }
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => unreachable!(),
    };
    if day == 0 || day > days_in_month {
        return Err("timestamp_invalid_or_out_of_profile");
    }

    let time_parts: Vec<&str> = time.split(':').collect();
    if time_parts.len() != 3
        || time_parts[0].len() != 2
        || time_parts[1].len() != 2
        || time_parts[2].len() < 2
        || !time_parts[0].bytes().all(|b| b.is_ascii_digit())
        || !time_parts[1].bytes().all(|b| b.is_ascii_digit())
    {
        return Err("timestamp_invalid_or_out_of_profile");
    }
    let hour = time_parts[0]
        .parse::<i64>()
        .map_err(|_| "timestamp_invalid_or_out_of_profile")?;
    let minute = time_parts[1]
        .parse::<i64>()
        .map_err(|_| "timestamp_invalid_or_out_of_profile")?;
    let mut second_parts = time_parts[2].split('.');
    let seconds_text = second_parts
        .next()
        .ok_or("timestamp_invalid_or_out_of_profile")?;
    let fractional = second_parts.next();
    if second_parts.next().is_some()
        || seconds_text.len() != 2
        || !seconds_text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err("timestamp_invalid_or_out_of_profile");
    }
    let second = seconds_text
        .parse::<i64>()
        .map_err(|_| "timestamp_invalid_or_out_of_profile")?;
    if hour > 23 || minute > 59 || second > 59 {
        return Err("timestamp_invalid_or_out_of_profile");
    }
    let fractional_nanos = match fractional {
        None => 0i128,
        Some(digits)
            if !digits.is_empty()
                && digits.len() <= 9
                && digits.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            let parsed = digits
                .parse::<i128>()
                .map_err(|_| "timestamp_invalid_or_out_of_profile")?;
            parsed * pow10(9 - digits.len() as u32)
        }
        Some(_) => return Err("timestamp_invalid_or_out_of_profile"),
    };

    let days = days_from_civil(year, month, day);
    let local_seconds =
        days as i128 * 86_400 + hour as i128 * 3_600 + minute as i128 * 60 + second as i128;
    let utc_seconds = local_seconds - offset_minutes as i128 * 60;
    Ok(utc_seconds * 1_000_000_000 + fractional_nanos)
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let adjusted_month = month as i64 + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day as i64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}
