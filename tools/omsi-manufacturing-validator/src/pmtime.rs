use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const FIXTURE_CONTRACT: &str = "omsi.pmtime.integrity.fixture-pack";
pub const REPORT_CONTRACT: &str = "omsi-pmtime-integrity-report/0.1";

const EXPECTED_CASE_IDS: [&str; 14] = [
    "PMT-001-fully-qualified",
    "PMT-002-shared-clock",
    "PMT-003-shared-oscillator",
    "PMT-004-shared-gnss-reference",
    "PMT-005-missing-required-domain",
    "PMT-006-stale-calibration",
    "PMT-007-source-switch-without-receipt",
    "PMT-008-pmtime-rollback",
    "PMT-009-period-overlap",
    "PMT-010-incomplete-dependency-inventory",
    "PMT-011-valid-signature-stale-lineage",
    "PMT-012-unsupported-time-scale",
    "PMT-013-shared-clock-instance",
    "PMT-014-shared-network-delivery",
    "PMT-015-delayed-ingest-preserves-event-time",
    "PMT-016-period-gap-unknown",
    "PMT-017-source-switch-bound-to-new-config",
];

const REQUIRED_VERIFIER_CHECKS: [&str; 5] = [
    "input-byte-binding",
    "query-snapshot-binding",
    "independent-closure",
    "derived-disposition",
    "authority-and-claim-ceiling",
];

const CLAIM_CEILING: &str = "Synthetic PMTime source/record-time integrity and structural composition of declared Mycelix dependency-verification outputs. It does not authenticate upstream reports, prove absolute physical time accuracy, qualify a PTP implementation, or establish service availability.";

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn sorted_unique_strings(value: &Value) -> Option<Vec<&str>> {
    let values = value.as_array()?;
    let mut parsed = Vec::with_capacity(values.len());
    for item in values {
        parsed.push(item.as_str()?);
    }
    if parsed.iter().any(|item| item.trim().is_empty())
        || parsed.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return None;
    }
    Some(parsed)
}

fn check_dependency_assessment(
    assessment: &Value,
    expected_domain: &str,
    expected_sources: &[&str],
    expected_event_time: u64,
) -> Result<String, &'static str> {
    if string(assessment, "domain") != Some(expected_domain) {
        return Err("dependency_domain_mismatch");
    }

    let Some(sources) = sorted_unique_strings(
        assessment.get("source_ids").unwrap_or(&Value::Null),
    ) else {
        return Err("dependency_source_set_invalid");
    };
    if sources != expected_sources {
        return Err("dependency_source_set_mismatch");
    }
    if number(assessment, "event_time_tai_ns") != Some(expected_event_time) {
        return Err("dependency_event_time_mismatch");
    }

    let Some(snapshot_id) = string(assessment, "snapshot_id") else {
        return Err("dependency_snapshot_missing");
    };
    if snapshot_id.trim().is_empty() {
        return Err("dependency_snapshot_missing");
    }
    let Some(frontier) = number(assessment, "evidence_frontier_tai_ns") else {
        return Err("dependency_frontier_missing");
    };
    if frontier < expected_event_time {
        return Err("dependency_frontier_before_event");
    }

    for key in [
        "input_sha256",
        "dependency_report_artifact_sha256",
        "verification_report_artifact_sha256",
    ] {
        if !string(assessment, key).is_some_and(is_sha256) {
            return Err("dependency_artifact_digest_missing_or_invalid");
        }
    }

    let Some(report) = assessment.get("dependency_report") else {
        return Err("dependency_report_missing");
    };
    if string(report, "report_contract")
        != Some("emsr-dependency-hypergraph-report/0.1")
        || string(report, "input_sha256") != string(assessment, "input_sha256")
        || string(report, "snapshot_id") != Some(snapshot_id)
        || number(report, "evidence_frontier") != Some(frontier)
        || number(report, "event_time") != Some(expected_event_time)
        || string(report, "domain") != Some(expected_domain)
        || string(report, "authority") != Some("declared_dependency_assessment_only")
    {
        return Err("dependency_report_binding_mismatch");
    }

    let Some(report_sources) = sorted_unique_strings(
        report.get("source_ids").unwrap_or(&Value::Null),
    ) else {
        return Err("dependency_report_source_set_invalid");
    };
    if report_sources != expected_sources {
        return Err("dependency_report_source_set_mismatch");
    }

    let Some(disposition) = string(assessment, "disposition") else {
        return Err("dependency_disposition_missing");
    };
    if string(report, "disposition") != Some(disposition) {
        return Err("dependency_report_disposition_mismatch");
    }
    if !matches!(
        disposition,
        "SHARED_DEPENDENCY" | "UNKNOWN_DEPENDENCY" | "NO_DECLARED_COMMON_DOMAIN"
    ) {
        return Err("dependency_disposition_unsupported");
    }

    let Some(verification) = assessment.get("reference_verification") else {
        return Err("dependency_reference_verification_missing");
    };
    if string(verification, "verifier_contract")
        != Some("emsr-dependency-hypergraph-reference-verifier/0.1")
        || string(verification, "status") != Some("verified")
        || string(verification, "authority")
            != Some("independent_reference_algorithm_agreement_only")
    {
        return Err("dependency_reference_verification_not_passed");
    }

    let Some(checks) = verification.get("checks").and_then(Value::as_array) else {
        return Err("dependency_reference_checks_missing");
    };
    let mut check_map = BTreeMap::new();
    for check in checks {
        let (Some(id), Some(result)) = (string(check, "id"), string(check, "result")) else {
            return Err("dependency_reference_check_malformed");
        };
        if check_map.insert(id, result).is_some() {
            return Err("dependency_reference_check_duplicated");
        }
    }
    if check_map.len() != REQUIRED_VERIFIER_CHECKS.len()
        || REQUIRED_VERIFIER_CHECKS
            .iter()
            .any(|id| check_map.get(id) != Some(&"pass"))
    {
        return Err("dependency_reference_checks_incomplete_or_failed");
    }

    Ok(disposition.to_owned())
}

fn evaluate_case(
    case: &Value,
    default_required_domains: &[String],
    default_max_uncertainty: u64,
) -> (String, Vec<String>) {
    let mut invalid = Vec::new();
    let mut unknown = Vec::new();
    let mut shared = false;

    let Some(records) = case.get("records").and_then(Value::as_array) else {
        return (
            "TIMESTAMP_INVALID".to_owned(),
            vec!["records_missing_or_not_array".to_owned()],
        );
    };
    if records.is_empty() {
        return (
            "TIMESTAMP_INVALID".to_owned(),
            vec!["records_empty".to_owned()],
        );
    }

    let max_uncertainty = number(case, "max_uncertainty_ns").unwrap_or(default_max_uncertainty);
    let mut previous: Option<(u64, u64, u64, &str, &str, &str, &str, &str, &str, &str)> = None;
    let mut expected_queries: Vec<(u64, Vec<&str>)> = Vec::with_capacity(records.len());

    for (index, record) in records.iter().enumerate() {
        let required_numeric = [
            "record_index",
            "pmtime_tai_ns",
            "period_start_tai_ns",
            "period_end_tai_ns",
            "timestamp_uncertainty_ns",
            "record_received_tai_ns",
        ];
        if required_numeric
            .iter()
            .any(|key| number(record, key).is_none())
        {
            invalid.push(format!("record_{index}_numeric_field_missing"));
            continue;
        }

        let record_index = number(record, "record_index").unwrap_or(0);
        let timestamp = number(record, "pmtime_tai_ns").unwrap_or(0);
        let period_start = number(record, "period_start_tai_ns").unwrap_or(0);
        let period_end = number(record, "period_end_tai_ns").unwrap_or(0);
        let uncertainty = number(record, "timestamp_uncertainty_ns").unwrap_or(u64::MAX);
        let record_received = number(record, "record_received_tai_ns").unwrap_or(0);

        let (
            Some(source_id),
            Some(source_type),
            Some(clock_domain_id),
            Some(ptp_id),
            Some(time_scale),
            Some(method),
            Some(config_revision),
            Some(adapter_version),
        ) = (
            string(record, "timestamp_source_id"),
            string(record, "timestamp_source_type"),
            string(record, "clock_domain_id"),
            string(record, "ptp_clock_instance_id"),
            string(record, "time_scale"),
            string(record, "timestamp_method"),
            string(record, "configuration_revision"),
            string(record, "adapter_mapping_version"),
        )
        else {
            invalid.push(format!("record_{index}_identity_or_time_basis_missing"));
            continue;
        };

        if source_id.trim().is_empty()
            || ptp_id.trim().is_empty()
            || source_type.trim().is_empty()
            || clock_domain_id.trim().is_empty()
            || method.trim().is_empty()
            || config_revision.trim().is_empty()
            || adapter_version.trim().is_empty()
        {
            invalid.push(format!("record_{index}_identity_or_method_empty"));
        }

        let source_type_supported = matches!(
            source_type,
            "external_reference_clock"
                | "gnss_reference"
                | "ptp_clock"
                | "local_oscillator"
                | "remote_timing_service"
                | "unknown"
        );
        if !source_type_supported || source_type == "unknown" {
            unknown.push(format!("record_{index}_timestamp_source_type_not_qualified"));
        }

        if period_start >= period_end || timestamp < period_start || timestamp >= period_end {
            invalid.push(format!("record_{index}_pmtime_outside_valid_period"));
        }
        if record_received < timestamp {
            invalid.push(format!("record_{index}_received_before_event_time"));
        }
        if !string(record, "record_content_sha256").is_some_and(is_sha256)
            || !string(record, "timestamp_uncertainty_budget_sha256").is_some_and(is_sha256)
        {
            unknown.push(format!("record_{index}_source_or_uncertainty_artifact_missing"));
        }
        if time_scale != "TAI" {
            unknown.push(format!("record_{index}_unsupported_or_unmapped_time_scale"));
            if !string(record, "time_scale_mapping_sha256").is_some_and(is_sha256) {
                unknown.push(format!("record_{index}_time_scale_mapping_evidence_missing"));
            }
        }
        if uncertainty > max_uncertainty {
            unknown.push(format!(
                "record_{index}_uncertainty_exceeds_declared_profile_limit"
            ));
        }
        if source_id == ptp_id {
            shared = true;
        }

        let Some(calibration) = record.get("calibration") else {
            unknown.push(format!("record_{index}_calibration_evidence_missing"));
            // Continue sequence validation even when calibration evidence is absent.
            let mut pair = vec![source_id, ptp_id];
            pair.sort();
            expected_queries.push((timestamp, pair));
            previous = Some((
                record_index,
                timestamp,
                period_end,
                source_id,
                source_type,
                clock_domain_id,
                ptp_id,
                config_revision,
                adapter_version,
                time_scale,
            ));
            continue;
        };
        if string(calibration, "status") != Some("current") {
            unknown.push(format!("record_{index}_calibration_not_current"));
        }
        let valid_from = number(calibration, "valid_from_tai_ns");
        let valid_until = number(calibration, "valid_until_tai_ns");
        if valid_from.is_none_or(|value| value > period_start)
            || valid_until.is_none_or(|value| value < period_end)
        {
            unknown.push(format!("record_{index}_calibration_does_not_cover_period"));
        }
        if !string(calibration, "certificate_sha256").is_some_and(is_sha256)
            || string(calibration, "traceability_ref")
                .is_none_or(|value| value.trim().is_empty())
        {
            unknown.push(format!("record_{index}_calibration_traceability_missing"));
        }

        let mut pair = vec![source_id, ptp_id];
        pair.sort();
        expected_queries.push((timestamp, pair));

        if let Some((
            prior_index,
            prior_time,
            prior_period_end,
            prior_source,
            prior_source_type,
            prior_clock_domain,
            prior_ptp,
            prior_config,
            prior_adapter_version,
            prior_scale,
        )) = previous
        {
            if record_index <= prior_index {
                invalid.push(format!("record_{index}_index_not_strictly_increasing"));
            } else if record_index != prior_index.saturating_add(1) {
                unknown.push(format!("record_{index}_record_index_gap"));
            }
            if timestamp <= prior_time {
                invalid.push(format!("record_{index}_pmtime_not_strictly_increasing"));
            }
            if period_start < prior_period_end {
                invalid.push(format!("record_{index}_period_overlaps_previous"));
            } else if period_start > prior_period_end {
                unknown.push(format!("record_{index}_period_gap_detected"));
            }

            let source_changed = source_id != prior_source
                || source_type != prior_source_type
                || clock_domain_id != prior_clock_domain
                || ptp_id != prior_ptp;
            let config_changed = config_revision != prior_config
                || adapter_version != prior_adapter_version
                || time_scale != prior_scale;
            if (source_changed || config_changed)
                && !string(record, "configuration_change_receipt_sha256").is_some_and(is_sha256)
            {
                unknown.push(format!("record_{index}_source_or_configuration_switch_unbound"));
            }
        }
        previous = Some((
            record_index,
            timestamp,
            period_end,
            source_id,
            source_type,
            clock_domain_id,
            ptp_id,
            config_revision,
            adapter_version,
            time_scale,
        ));
    }

    let required_domains = case
        .get("required_dependency_domains")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| default_required_domains.to_vec());

    if required_domains.is_empty() {
        invalid.push("required_dependency_domain_list_empty".to_owned());
    }
    let mut seen_queries = BTreeSet::new();
    let mut seen_by_event: BTreeMap<u64, BTreeSet<String>> = BTreeMap::new();
    let mut snapshot_by_event: BTreeMap<u64, (String, u64)> = BTreeMap::new();

    let assessments = case.get("dependency_assessments").and_then(Value::as_array);
    if assessments.is_none() {
        unknown.push("dependency_assessments_missing".to_owned());
    }

    for assessment in assessments.into_iter().flatten() {
        let Some(domain) = string(assessment, "domain") else {
            unknown.push("dependency_assessment_domain_missing".to_owned());
            continue;
        };
        let Some(event_time) = number(assessment, "event_time_tai_ns") else {
            unknown.push("dependency_assessment_event_time_missing".to_owned());
            continue;
        };
        let Some((_, sources)) = expected_queries
            .iter()
            .find(|(expected_time, _)| *expected_time == event_time)
        else {
            unknown.push(format!("dependency_{domain}_event_time_has_no_record"));
            continue;
        };

        if !seen_queries.insert((event_time, domain.to_owned())) {
            invalid.push(format!(
                "dependency_domain_{domain}_duplicated_for_event_{event_time}"
            ));
            continue;
        }

        match check_dependency_assessment(assessment, domain, sources, event_time) {
            Ok(disposition) => {
                if disposition == "SHARED_DEPENDENCY" {
                    shared = true;
                } else if disposition == "UNKNOWN_DEPENDENCY" {
                    unknown.push(format!("dependency_{domain}_unknown_at_event_{event_time}"));
                }
            }
            Err(reason) => unknown.push(format!(
                "dependency_{domain}_{reason}_at_event_{event_time}"
            )),
        }

        let snapshot_id = string(assessment, "snapshot_id").unwrap_or_default().to_owned();
        let frontier = number(assessment, "evidence_frontier_tai_ns").unwrap_or(0);
        match snapshot_by_event.get(&event_time) {
            Some((known_snapshot, known_frontier))
                if known_snapshot != &snapshot_id || *known_frontier != frontier =>
            {
                unknown.push(format!(
                    "dependency_snapshot_or_frontier_mismatch_across_domains_at_event_{event_time}"
                ));
            }
            None => {
                snapshot_by_event.insert(event_time, (snapshot_id, frontier));
            }
            _ => {}
        }

        let supported = matches!(
            domain,
            "clock"
                | "oscillator"
                | "gnss_reference"
                | "power"
                | "network"
                | "correction_provider"
                | "firmware_lineage"
                | "collector"
                | "data_pipeline"
                | "antenna_site"
        );
        if !supported {
            invalid.push(format!("dependency_domain_{domain}_unsupported"));
        }

        let required_for_case = case
            .get("required_dependency_domains")
            .and_then(Value::as_array)
            .map(|items| items.iter().any(|value| value.as_str() == Some(domain)))
            .unwrap_or_else(|| default_required_domains.iter().any(|value| value == domain));
        if !required_for_case && !supported {
            invalid.push(format!("dependency_domain_{domain}_unsupported"));
        }
        seen_by_event
            .entry(event_time)
            .or_default()
            .insert(domain.to_owned());
    }

    for (event_time, _) in &expected_queries {
        let seen = seen_by_event.get(event_time);
        for domain in &required_domains {
            if !seen.is_some_and(|domains| domains.contains(domain)) {
                unknown.push(format!(
                    "required_dependency_domain_{domain}_missing_at_event_{event_time}"
                ));
            }
        }
    }

    let outcome = if !invalid.is_empty() {
        "TIMESTAMP_INVALID"
    } else if shared {
        "TIMESTAMP_SHARED_FAILURE_DOMAIN"
    } else if !unknown.is_empty() {
        "TIMESTAMP_INDETERMINATE"
    } else {
        "TIMESTAMP_QUALIFIED_FOR_DECLARED_SCOPE"
    };

    let mut reasons: Vec<String> = invalid.into_iter().chain(unknown).collect();
    if shared {
        reasons.push("declared_common_failure_domain_detected".to_owned());
    }
    reasons.sort();
    reasons.dedup();
    (outcome.to_owned(), reasons)
}

pub fn validate_fixture_pack(document: &Value, source_bytes: &[u8]) -> Value {
    let standard = document.get("standard");
    let policy = document.get("profile_policy");
    let cases = document.get("cases").and_then(Value::as_array);
    let required_domains: Vec<String> = document
        .get("required_dependency_domains")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let max_uncertainty = policy
        .and_then(|value| number(value, "max_uncertainty_ns"))
        .unwrap_or(0);

    let metadata_valid = string(document, "contract") == Some(FIXTURE_CONTRACT)
        && string(document, "version") == Some("0.1.0")
        && string(document, "state") == Some("SYNTHETIC_CONFORMANCE_FIXTURES")
        && standard.is_some_and(|value| {
            string(value, "recommendation") == Some("ITU-T G.8275")
                && string(value, "edition") == Some("01/2026")
                && string(value, "annex") == Some("F")
        })
        && !required_domains.is_empty()
        && max_uncertainty > 0
        && cases.is_some_and(|items| items.len() == EXPECTED_CASE_IDS.len());

    let discovered: Vec<String> = cases
        .into_iter()
        .flatten()
        .filter_map(|case| string(case, "id").map(str::to_owned))
        .collect();
    let discovery_complete = discovered.len() == EXPECTED_CASE_IDS.len()
        && discovered
            .iter()
            .map(String::as_str)
            .eq(EXPECTED_CASE_IDS.iter().copied());

    let mut case_reports = Vec::new();
    let mut failure_count = 0usize;
    for case in cases.into_iter().flatten() {
        let id = string(case, "id").unwrap_or("MISSING_ID");
        let expected = string(case, "expected").unwrap_or("MISSING_EXPECTED");
        let (actual, reasons) = evaluate_case(case, &required_domains, max_uncertainty);
        let passed = actual == expected;
        if !passed {
            failure_count += 1;
        }
        case_reports.push(json!({
            "id": id,
            "expected": expected,
            "actual": actual,
            "passed": passed,
            "reason_codes": reasons
        }));
    }

    let passed = metadata_valid && discovery_complete && failure_count == 0;
    json!({
        "report_contract": REPORT_CONTRACT,
        "status": if passed { "passed" } else { "failed" },
        "authority": "synthetic_pmtime_record_integrity_and_declared_dependency_binding_only",
        "input_sha256": sha256_hex(source_bytes),
        "standard": {
            "recommendation": "ITU-T G.8275",
            "edition": "01/2026",
            "annex": "F",
            "amendment_2026_08": "NOT_APPLIED"
        },
        "required_dependency_domains": required_domains,
        "case_count_expected": EXPECTED_CASE_IDS.len(),
        "case_count_discovered": discovered.len(),
        "case_ids": discovered,
        "discovery_complete": discovery_complete,
        "metadata_valid": metadata_valid,
        "passed_count": case_reports.iter().filter(|item| item["passed"] == true).count(),
        "failure_count": failure_count,
        "per_case": case_reports,
        "claim_ceiling": CLAIM_CEILING,
        "non_claims": [
            "not_absolute_physical_time_accuracy",
            "not_ptp_device_or_network_certification",
            "not_syncaas_availability_reconstruction",
            "not_authentication_of_mycelix_verifier_or_custody_receipts",
            "not_proof_of_physical_independence",
            "not_itu_t_2026_amendment_1_conformance"
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::validate_fixture_pack;
    use jsonschema::draft202012;
    use serde_json::Value;

    const FIXTURES: &str = include_str!("../../../docs/standards/omsi/pmtime-integrity-fixtures-0.1.json");
    const FIXTURE_SCHEMA: &str = include_str!("../../../docs/standards/omsi/pmtime-integrity-fixtures-0.1.schema.json");
    const REPORT_SCHEMA: &str = include_str!("../schema/pmtime-integrity-report.schema.json");

    #[test]
    fn frozen_pmtime_pack_executes_all_cases() {
        let document: Value = serde_json::from_str(FIXTURES).expect("fixture JSON");
        let input_schema: Value = serde_json::from_str(FIXTURE_SCHEMA).expect("input schema JSON");
        assert!(draft202012::meta::is_valid(&input_schema));
        let input_validator = draft202012::new(&input_schema).expect("compile input schema");
        assert!(input_validator.is_valid(&document), "fixture pack schema validation failed");

        let report = validate_fixture_pack(&document, FIXTURES.as_bytes());
        assert_eq!(report["status"], "passed", "{report:#}");
        assert_eq!(report["case_count_expected"], 17);
        assert_eq!(report["case_count_discovered"], 17);
        assert_eq!(report["passed_count"], 17);
        assert_eq!(report["failure_count"], 0);

        let report_schema: Value = serde_json::from_str(REPORT_SCHEMA).expect("report schema JSON");
        assert!(draft202012::meta::is_valid(&report_schema));
        let report_validator = draft202012::new(&report_schema).expect("compile report schema");
        assert!(report_validator.is_valid(&report), "report schema validation failed: {report:#}");
    }

    #[test]
    fn exact_case_discovery_fails_closed() {
        let mut document: Value = serde_json::from_str(FIXTURES).expect("fixture JSON");
        document["cases"].as_array_mut().expect("cases").pop();
        let report = validate_fixture_pack(&document, b"mutated");
        assert_eq!(report["status"], "failed");
        assert_eq!(report["discovery_complete"], false);
    }
}
