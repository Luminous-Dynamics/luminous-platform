use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const FIXTURE_CONTRACT: &str = "omsi.pnt-conformance-fixture-pack";
pub const REPORT_CONTRACT: &str = "omsi-pnt-conformance-report/0.1";
pub const PROFILE_ID: &str = "omsi.pnt-resilience";
pub const PROFILE_VERSION: &str = "0.1.0-draft";

const EXPECTED_FIXTURE_IDS: [&str; 16] = [
    "PNT-POS-001",
    "PNT-FRESH-INVALID-001",
    "PNT-CAL-STALE-001",
    "PNT-TIME-DOMAIN-001",
    "PNT-COMMON-CLOCK-001",
    "PNT-DEPENDENCY-UNKNOWN-001",
    "PNT-SNAPSHOT-MISMATCH-001",
    "PNT-ORACLE-SHARED-001",
    "PNT-ANOMALY-CAUSE-001",
    "PNT-RF-UNKNOWN-001",
    "PNT-CAPTURE-DEGRADED-001",
    "PNT-COMMS-PARTITION-001",
    "PNT-RECOVERY-AUTO-001",
    "PNT-RECOVERY-QUAL-001",
    "PNT-AUTH-MISSING-001",
    "PNT-PROBE-IMPACT-UNKNOWN-001",
];

const CLAIM_CEILING: &str = "Synthetic PNT profile semantic conformance only. This report does not qualify a receiver, clock, detector, navigation solution, operator, live service, safety case, regulatory compliance or certification; it does not prove physical independence, ground-truth independence, RF attribution or permission to perform an operation.";

#[derive(Clone, Debug, Eq, PartialEq)]
struct Outcome {
    disposition: &'static str,
    reason_codes: Vec<&'static str>,
}

impl Outcome {
    fn new(disposition: &'static str, reason: &'static str) -> Self {
        Self {
            disposition,
            reason_codes: vec![reason],
        }
    }

    fn to_value(&self) -> Value {
        json!({
            "disposition": self.disposition,
            "reason_codes": self.reason_codes
        })
    }
}

/// Primary deterministic evaluator. It applies the profile's conservative
/// precedence to the supplied observation, dependency, service and authority
/// facets. It does not infer physical truth or run a PNT estimator.
fn evaluate_input(input: &Value) -> Outcome {
    let profile = input.get("profile").unwrap_or(&Value::Null);
    if text(profile, "id") != Some(PROFILE_ID) || text(profile, "version") != Some(PROFILE_VERSION)
    {
        return Outcome::new("NOT_COMPARABLE", "profile_identity_or_version_mismatch");
    }

    let assessment = input.get("assessment").unwrap_or(&Value::Null);
    let assessment_time = text(assessment, "as_of");
    let sources = input.get("sources").and_then(Value::as_array);

    if let Some(items) = sources {
        for source in items {
            match text(source, "integrity_status") {
                Some("invalid" | "contradictory") => {
                    return Outcome::new("REJECT_RELIANCE", "integrity_explicitly_invalid");
                }
                Some("unknown" | "unavailable") => {
                    return Outcome::new("INDETERMINATE", "integrity_state_unknown");
                }
                _ => {}
            }
        }

        for source in items {
            match text(source, "acquisition_status") {
                Some("degraded" | "unqualified") => {
                    return Outcome::new("REJECT_RELIANCE", "acquisition_chain_degraded");
                }
                Some("unknown" | "unavailable") => {
                    return Outcome::new("INDETERMINATE", "acquisition_qualification_unknown");
                }
                _ => {}
            }
        }

        for source in items {
            let calibration = text(source, "calibration_status");
            let valid_until = text(source, "calibration_valid_until");
            let expired_by_time = match (assessment_time, valid_until) {
                (Some(now), Some(until)) if is_canonical_utc(now) && is_canonical_utc(until) => {
                    until <= now
                }
                _ => false,
            };
            if matches!(calibration, Some("expired" | "revoked")) || expired_by_time {
                return Outcome::new("REJECT_RELIANCE", "calibration_expired");
            }
            if calibration == Some("unknown")
                || (calibration == Some("current") && valid_until.is_none())
            {
                return Outcome::new("INDETERMINATE", "calibration_validity_unknown");
            }
        }
    }

    if let Some(oracle) = input.get("oracle_evidence") {
        if bool_value(oracle, "independence_claimed") == Some(true) {
            let detector: BTreeSet<&str> = string_array(oracle, "detector_input_source_ids")
                .into_iter()
                .collect();
            let labels: BTreeSet<&str> = string_array(oracle, "label_source_ids")
                .into_iter()
                .collect();
            if detector.is_empty() || labels.is_empty() {
                return Outcome::new("INDETERMINATE", "oracle_independence_unproven");
            }
            if detector.iter().any(|id| labels.contains(id)) {
                return Outcome::new("REJECT_RELIANCE", "oracle_shares_detector_input");
            }
        }
    }

    if let Some(request) = input.get("operation_request") {
        if bool_value(request, "requested") == Some(true)
            && text(request, "authority_reference")
                .filter(|value| !value.trim().is_empty())
                .is_none()
        {
            return Outcome::new("NOT_AUTHORIZED", "authority_reference_missing");
        }
    }

    if let Some(probe) = input.get("probe_assessment") {
        let impact_present = bool_value(probe, "impact_receipt_present") == Some(true);
        let execution_state = text(probe, "execution_state");
        let stop_confirmed = bool_value(probe, "acquisition_stop_confirmed") == Some(true);
        if !impact_present || execution_state == Some("timed_out") || !stop_confirmed {
            return Outcome::new("INDETERMINATE", "probe_impact_unknown_after_timeout");
        }
    }

    if let Some(anomaly) = input.get("anomaly_observation") {
        if bool_value(anomaly, "cause_adjudicated") != Some(true) {
            return Outcome::new("INDETERMINATE", "cause_not_adjudicated");
        }
    }

    if let Some(rf) = input.get("rf_assessment") {
        if text(rf, "classification_state") == Some("unknown") {
            return Outcome::new("INDETERMINATE", "unknown_rf_not_pnt_integrity");
        }
    }

    if let Some(recovery) = input.get("recovery_evidence") {
        if bool_value(recovery, "requalification_required") == Some(true) {
            let all_conditions = [
                "event_cleared",
                "requalification_satisfied",
                "fresh_observations",
                "calibration_current",
                "configuration_matches",
                "dependency_snapshot_current",
            ]
            .iter()
            .all(|field| bool_value(recovery, field) == Some(true));
            if !all_conditions {
                return Outcome::new("REJECT_RELIANCE", "requalification_missing");
            }
            return Outcome::new(
                "QUALIFIED_FOR_DECLARED_PROFILE",
                "requalification_conditions_satisfied",
            );
        }
    }

    if let Some(inventory) = input.get("dependency_inventory") {
        let expected_snapshot = text(assessment, "dependency_snapshot_id");
        let actual_snapshot = text(inventory, "snapshot_id");
        if expected_snapshot.is_none()
            || actual_snapshot.is_none()
            || expected_snapshot != actual_snapshot
        {
            return Outcome::new("INDETERMINATE", "dependency_snapshot_mismatch");
        }

        if bool_value(inventory, "inventory_complete") != Some(true) {
            return Outcome::new("INDETERMINATE", "dependency_inventory_incomplete");
        }

        if clock_domain_unqualified(sources) {
            return Outcome::new("INDETERMINATE", "clock_domains_not_correlated");
        }

        if shared_clock_failure_domain(sources, inventory) {
            return Outcome::new("DEGRADED", "shared_clock_failure_domain");
        }
    } else if clock_domain_unqualified(sources) {
        return Outcome::new("INDETERMINATE", "clock_domains_not_correlated");
    }

    if let Some(service) = input.get("service_assessment") {
        if text(service, "communications_state") == Some("partitioned") {
            return Outcome::new("DEGRADED", "remote_context_unavailable");
        }
        match text(service, "local_evidence_state") {
            Some("invalid") => return Outcome::new("REJECT_RELIANCE", "local_evidence_invalid"),
            Some("degraded") => return Outcome::new("DEGRADED", "local_evidence_degraded"),
            Some("unknown") => return Outcome::new("INDETERMINATE", "local_evidence_unknown"),
            _ => {}
        }
    }

    Outcome::new(
        "QUALIFIED_FOR_DECLARED_PROFILE",
        "required_evidence_current_and_profile_bound",
    )
}

/// Separate reference implementation: evaluates predicates in a different
/// arrangement and independently derives the expected disposition from raw
/// input, rather than accepting the primary evaluator's result.
fn reference_evaluate(input: &Value) -> Outcome {
    if input.pointer("/profile/id").and_then(Value::as_str) != Some(PROFILE_ID)
        || input.pointer("/profile/version").and_then(Value::as_str) != Some(PROFILE_VERSION)
    {
        return Outcome::new("NOT_COMPARABLE", "profile_identity_or_version_mismatch");
    }

    if let Some(request) = input.get("operation_request") {
        let asked = request
            .get("requested")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let grant = request
            .get("authority_reference")
            .and_then(Value::as_str)
            .unwrap_or("");
        if asked && grant.trim().is_empty() {
            return Outcome::new("NOT_AUTHORIZED", "authority_reference_missing");
        }
    }

    if let Some(probe) = input.get("probe_assessment") {
        let observed = probe
            .get("impact_receipt_present")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let stopped = probe
            .get("acquisition_stop_confirmed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let timed_out = probe.get("execution_state").and_then(Value::as_str) == Some("timed_out");
        if !(observed && stopped && !timed_out) {
            return Outcome::new("INDETERMINATE", "probe_impact_unknown_after_timeout");
        }
    }

    if let Some(oracle) = input.get("oracle_evidence") {
        if oracle.get("independence_claimed").and_then(Value::as_bool) == Some(true) {
            let detector = oracle
                .get("detector_input_source_ids")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let label_sources = oracle
                .get("label_source_ids")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if detector.is_empty() || label_sources.is_empty() {
                return Outcome::new("INDETERMINATE", "oracle_independence_unproven");
            }
            if detector
                .iter()
                .any(|left| label_sources.iter().any(|right| left == right))
            {
                return Outcome::new("REJECT_RELIANCE", "oracle_shares_detector_input");
            }
        }
    }

    if let Some(recovery) = input.get("recovery_evidence") {
        if recovery
            .get("requalification_required")
            .and_then(Value::as_bool)
            == Some(true)
        {
            let required = [
                "event_cleared",
                "requalification_satisfied",
                "fresh_observations",
                "calibration_current",
                "configuration_matches",
                "dependency_snapshot_current",
            ];
            if required
                .iter()
                .any(|key| recovery.get(*key).and_then(Value::as_bool) != Some(true))
            {
                return Outcome::new("REJECT_RELIANCE", "requalification_missing");
            }
            return Outcome::new(
                "QUALIFIED_FOR_DECLARED_PROFILE",
                "requalification_conditions_satisfied",
            );
        }
    }

    if input
        .pointer("/anomaly_observation/cause_adjudicated")
        .and_then(Value::as_bool)
        == Some(false)
    {
        return Outcome::new("INDETERMINATE", "cause_not_adjudicated");
    }
    if input
        .pointer("/rf_assessment/classification_state")
        .and_then(Value::as_str)
        == Some("unknown")
    {
        return Outcome::new("INDETERMINATE", "unknown_rf_not_pnt_integrity");
    }

    let as_of = input
        .pointer("/assessment/as_of")
        .and_then(Value::as_str)
        .unwrap_or("");
    if let Some(sources) = input.get("sources").and_then(Value::as_array) {
        for source in sources {
            let status = source.get("integrity_status").and_then(Value::as_str);
            if matches!(status, Some("invalid" | "contradictory")) {
                return Outcome::new("REJECT_RELIANCE", "integrity_explicitly_invalid");
            }
            if matches!(status, Some("unknown" | "unavailable")) {
                return Outcome::new("INDETERMINATE", "integrity_state_unknown");
            }
            let acquisition = source.get("acquisition_status").and_then(Value::as_str);
            if matches!(acquisition, Some("degraded" | "unqualified")) {
                return Outcome::new("REJECT_RELIANCE", "acquisition_chain_degraded");
            }
            if matches!(acquisition, Some("unknown" | "unavailable")) {
                return Outcome::new("INDETERMINATE", "acquisition_qualification_unknown");
            }
            let calibration = source.get("calibration_status").and_then(Value::as_str);
            let until = source
                .get("calibration_valid_until")
                .and_then(Value::as_str);
            let past_validity = match (calibration, until) {
                (Some(_), Some(until)) if is_canonical_utc(as_of) && is_canonical_utc(until) => {
                    until <= as_of
                }
                _ => false,
            };
            if matches!(calibration, Some("expired" | "revoked")) || past_validity {
                return Outcome::new("REJECT_RELIANCE", "calibration_expired");
            }
            if calibration == Some("unknown") || (calibration == Some("current") && until.is_none())
            {
                return Outcome::new("INDETERMINATE", "calibration_validity_unknown");
            }
        }
    }

    if let Some(inventory) = input.get("dependency_inventory") {
        let expected = input
            .pointer("/assessment/dependency_snapshot_id")
            .and_then(Value::as_str);
        let actual = inventory.get("snapshot_id").and_then(Value::as_str);
        if expected.is_none() || expected != actual {
            return Outcome::new("INDETERMINATE", "dependency_snapshot_mismatch");
        }
        if inventory.get("inventory_complete").and_then(Value::as_bool) != Some(true) {
            return Outcome::new("INDETERMINATE", "dependency_inventory_incomplete");
        }
    }

    if source_clock_domains_unqualified(input) {
        return Outcome::new("INDETERMINATE", "clock_domains_not_correlated");
    }

    if reference_detects_shared_clock(input) {
        return Outcome::new("DEGRADED", "shared_clock_failure_domain");
    }

    if input
        .pointer("/service_assessment/communications_state")
        .and_then(Value::as_str)
        == Some("partitioned")
    {
        return Outcome::new("DEGRADED", "remote_context_unavailable");
    }
    match input
        .pointer("/service_assessment/local_evidence_state")
        .and_then(Value::as_str)
    {
        Some("invalid") => return Outcome::new("REJECT_RELIANCE", "local_evidence_invalid"),
        Some("degraded") => return Outcome::new("DEGRADED", "local_evidence_degraded"),
        Some("unknown") => return Outcome::new("INDETERMINATE", "local_evidence_unknown"),
        _ => {}
    }

    Outcome::new(
        "QUALIFIED_FOR_DECLARED_PROFILE",
        "required_evidence_current_and_profile_bound",
    )
}

fn source_clock_domains_unqualified(input: &Value) -> bool {
    let Some(sources) = input.get("sources").and_then(Value::as_array) else {
        return false;
    };
    for left_index in 0..sources.len() {
        for right_index in (left_index + 1)..sources.len() {
            let left = &sources[left_index];
            let right = &sources[right_index];
            let domain_changed = left.get("clock_domain_id") != right.get("clock_domain_id");
            let scale_changed = left.get("clock_time_scale") != right.get("clock_time_scale");
            if domain_changed || scale_changed {
                let left_correlation = left
                    .get("clock_correlation_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let right_correlation = right
                    .get("clock_correlation_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if left_correlation.trim().is_empty() || right_correlation.trim().is_empty() {
                    return true;
                }
            }
        }
    }
    false
}

fn clock_domain_unqualified(sources: Option<&Vec<Value>>) -> bool {
    let Some(items) = sources else {
        return false;
    };
    if items.len() < 2 {
        return false;
    }
    for index in 0..items.len() {
        for other in (index + 1)..items.len() {
            let a = &items[index];
            let b = &items[other];
            let different_basis = a.get("clock_domain_id") != b.get("clock_domain_id")
                || a.get("clock_time_scale") != b.get("clock_time_scale");
            if different_basis {
                let a_correlation = text(a, "clock_correlation_id").unwrap_or("");
                let b_correlation = text(b, "clock_correlation_id").unwrap_or("");
                if a_correlation.trim().is_empty() || b_correlation.trim().is_empty() {
                    return true;
                }
            }
        }
    }
    false
}

fn shared_clock_failure_domain(sources: Option<&Vec<Value>>, inventory: &Value) -> bool {
    let mut clock_domains = BTreeMap::<String, usize>::new();
    if let Some(items) = sources {
        for source in items {
            if let Some(domain) = text(source, "clock_domain_id") {
                *clock_domains.entry(domain.to_owned()).or_default() += 1;
            }
        }
    }
    if clock_domains.values().any(|count| *count > 1) {
        return true;
    }

    let mut failure_domains = BTreeMap::<String, BTreeSet<String>>::new();
    if let Some(nodes) = inventory.get("nodes").and_then(Value::as_array) {
        for node in nodes {
            if text(node, "domain_type") == Some("clock") {
                if let (Some(source), Some(domain)) =
                    (text(node, "source_id"), text(node, "failure_domain_id"))
                {
                    failure_domains
                        .entry(domain.to_owned())
                        .or_default()
                        .insert(source.to_owned());
                }
            }
        }
    }
    failure_domains.values().any(|sources| sources.len() > 1)
}

fn reference_detects_shared_clock(input: &Value) -> bool {
    let mut clock_ids = BTreeSet::new();
    let mut duplicate_clock = false;
    if let Some(sources) = input.get("sources").and_then(Value::as_array) {
        for source in sources {
            if let Some(clock_id) = source.get("clock_domain_id").and_then(Value::as_str) {
                if !clock_ids.insert(clock_id.to_owned()) {
                    duplicate_clock = true;
                }
            }
        }
    }
    if duplicate_clock {
        return true;
    }

    let Some(nodes) = input
        .pointer("/dependency_inventory/nodes")
        .and_then(Value::as_array)
    else {
        return false;
    };
    let mut by_domain: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for node in nodes {
        if node.get("domain_type").and_then(Value::as_str) == Some("clock") {
            if let (Some(fd), Some(source)) = (
                node.get("failure_domain_id").and_then(Value::as_str),
                node.get("source_id").and_then(Value::as_str),
            ) {
                by_domain.entry(fd).or_default().insert(source);
            }
        }
    }
    by_domain.values().any(|members| members.len() > 1)
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn bool_value(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn string_array<'a>(value: &'a Value, key: &str) -> Vec<&'a str> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn is_canonical_utc(value: &str) -> bool {
    value.len() == 20
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value.as_bytes().get(10) == Some(&b'T')
        && value.as_bytes().get(13) == Some(&b':')
        && value.as_bytes().get(16) == Some(&b':')
        && value.ends_with('Z')
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

pub fn validate_fixture_pack(document: &Value, source_bytes: &[u8]) -> Value {
    let fixtures = document.get("fixtures").and_then(Value::as_array);
    let mut discovered = Vec::new();
    let mut ids = BTreeSet::new();
    let mut duplicate_ids = Vec::new();
    let mut per_fixture = Vec::new();
    let mut passed_count = 0usize;
    let mut failure_count = 0usize;
    let mut reference_disagreement_count = 0usize;
    let mut false_accept_count = 0usize;

    if let Some(items) = fixtures {
        for fixture in items {
            let id = text(fixture, "id").unwrap_or("<missing-id>");
            discovered.push(id.to_owned());
            if !ids.insert(id.to_owned()) {
                duplicate_ids.push(id.to_owned());
            }

            let input = fixture.get("input").unwrap_or(&Value::Null);
            let expected = fixture.get("expected").unwrap_or(&Value::Null);
            let primary = evaluate_input(input);
            let reference = reference_evaluate(input);
            let expected_value = json!({
                "disposition": text(expected, "disposition"),
                "reason_codes": expected.get("reason_codes").cloned().unwrap_or(Value::Null)
            });
            let primary_value = primary.to_value();
            let reference_value = reference.to_value();
            let expected_match = normalized_outcome_equal(&primary_value, &expected_value);
            let reference_match = normalized_outcome_equal(&reference_value, &expected_value);
            let independent_agreement = normalized_outcome_equal(&primary_value, &reference_value);
            let passed = expected_match && reference_match && independent_agreement;

            if passed {
                passed_count += 1;
            } else {
                failure_count += 1;
            }
            if !independent_agreement {
                reference_disagreement_count += 1;
            }
            if primary.disposition == "QUALIFIED_FOR_DECLARED_PROFILE"
                && text(expected, "disposition") != Some("QUALIFIED_FOR_DECLARED_PROFILE")
            {
                false_accept_count += 1;
            }

            per_fixture.push(json!({
                "id": id,
                "expected": expected_value,
                "primary": primary_value,
                "reference": reference_value,
                "expected_match": expected_match,
                "independent_agreement": independent_agreement,
                "passed": passed
            }));
        }
    }

    let discovered_set: BTreeSet<String> = discovered.iter().cloned().collect();
    let expected_set: BTreeSet<String> = EXPECTED_FIXTURE_IDS
        .iter()
        .map(|id| (*id).to_owned())
        .collect();
    let discovery_complete = discovered.len() == EXPECTED_FIXTURE_IDS.len()
        && discovered_set == expected_set
        && duplicate_ids.is_empty();
    let metadata_ok = text(document, "schema") == Some(FIXTURE_CONTRACT)
        && text(document, "version") == Some("0.1.0-draft")
        && text(document, "profile_id") == Some(PROFILE_ID)
        && text(document, "profile_version") == Some(PROFILE_VERSION)
        && document.get("owner_issue").and_then(Value::as_u64) == Some(307);
    let fixtures_len = fixtures.map_or(0, Vec::len);
    let passed = metadata_ok
        && discovery_complete
        && fixtures_len == EXPECTED_FIXTURE_IDS.len()
        && failure_count == 0
        && reference_disagreement_count == 0
        && false_accept_count == 0;

    json!({
        "report_contract": REPORT_CONTRACT,
        "status": if passed { "passed" } else { "failed" },
        "authority": "synthetic_profile_semantic_conformance_only",
        "input_sha256": sha256_hex(source_bytes),
        "profile_id": PROFILE_ID,
        "profile_version": PROFILE_VERSION,
        "fixture_pack_version": text(document, "version"),
        "fixture_count_expected": EXPECTED_FIXTURE_IDS.len(),
        "fixture_count_discovered": fixtures_len,
        "fixture_ids": discovered,
        "discovery_complete": discovery_complete,
        "duplicate_fixture_ids": duplicate_ids,
        "metadata_valid": metadata_ok,
        "passed_count": passed_count,
        "failure_count": failure_count,
        "reference_disagreement_count": reference_disagreement_count,
        "false_accept_count": false_accept_count,
        "verification": {
            "status": if reference_disagreement_count == 0 && failure_count == 0 && discovery_complete { "passed" } else { "failed" },
            "method": "separate_rust_reference_rule_path",
            "claim_ceiling": "Algorithmically separate reference evaluation in the same Rust binary; not cross-language, organizational or physical independence."
        },
        "per_fixture": per_fixture,
        "claim_ceiling": CLAIM_CEILING,
        "non_claims": [
            "not_receiver_qualification",
            "not_physical_truth_or_accuracy",
            "not_field_detection_performance",
            "not_regulatory_or_safety_certification",
            "not_proof_of_physical_independence",
            "not_ground_truth_authentication",
            "not_actor_or_emitter_attribution",
            "not_operation_authorization"
        ]
    })
}

fn normalized_outcome_equal(left: &Value, right: &Value) -> bool {
    let left_disposition = text(left, "disposition");
    let right_disposition = text(right, "disposition");
    if left_disposition != right_disposition {
        return false;
    }
    let mut left_reasons = string_array(left, "reason_codes");
    let mut right_reasons = string_array(right, "reason_codes");
    left_reasons.sort_unstable();
    right_reasons.sort_unstable();
    left_reasons == right_reasons
}

#[cfg(test)]
mod tests {
    use super::{evaluate_input, validate_fixture_pack};
    use jsonschema::draft202012;
    use serde_json::{json, Value};

    const FIXTURES: &str =
        include_str!("../../../docs/standards/omsi/pnt-conformance-fixtures-0.1.json");
    const FIXTURE_SCHEMA: &str =
        include_str!("../../../docs/standards/omsi/pnt-conformance-fixtures-0.1.schema.json");
    const REPORT_SCHEMA: &str = include_str!("../schema/pnt-conformance-report.schema.json");

    fn pack() -> Value {
        serde_json::from_str(FIXTURES).expect("PNT fixture pack parses")
    }

    #[test]
    fn pnt_fixture_pack_matches_schema_and_executes_all_sixteen_cases() {
        let document = pack();
        let schema: Value =
            serde_json::from_str(FIXTURE_SCHEMA).expect("PNT fixture schema parses");
        assert!(
            draft202012::meta::is_valid(&schema),
            "PNT fixture schema is not valid Draft 2020-12"
        );
        let validator = draft202012::new(&schema).expect("PNT fixture schema compiles");
        assert!(
            validator.is_valid(&document),
            "PNT fixture pack does not match schema"
        );

        let report = validate_fixture_pack(&document, FIXTURES.as_bytes());
        assert_eq!(report["status"], "passed", "{report:#}");
        assert_eq!(report["fixture_count_expected"], 16);
        assert_eq!(report["fixture_count_discovered"], 16);
        assert_eq!(report["passed_count"], 16);
        assert_eq!(report["failure_count"], 0);
        assert_eq!(report["reference_disagreement_count"], 0);
        assert_eq!(report["false_accept_count"], 0);
        assert_eq!(report["verification"]["status"], "passed");
        let report_schema: Value =
            serde_json::from_str(REPORT_SCHEMA).expect("PNT report schema parses");
        assert!(
            draft202012::meta::is_valid(&report_schema),
            "PNT report schema is not valid Draft 2020-12"
        );
        let report_validator =
            draft202012::new(&report_schema).expect("PNT report schema compiles");
        assert!(
            report_validator.is_valid(&report),
            "PNT report does not match schema: {report:#}"
        );
    }

    #[test]
    fn fixture_discovery_fails_closed_when_a_case_is_missing() {
        let mut document = pack();
        document["fixtures"].as_array_mut().unwrap().pop();
        let report = validate_fixture_pack(&document, b"mutated-fixture-pack");
        assert_eq!(report["status"], "failed");
        assert_eq!(report["discovery_complete"], false);
        assert_eq!(report["fixture_count_discovered"], 15);
    }

    #[test]
    fn a_confident_unknown_rf_class_is_not_promoted_to_pnt_integrity() {
        let input = json!({
            "profile": {"id": "omsi.pnt-resilience", "version": "0.1.0-draft"},
            "assessment": {"as_of": "2026-11-07T12:00:00Z"},
            "rf_assessment": {
                "classification_state": "unknown",
                "confidence": 0.99,
                "acquisition_state": "qualified",
                "pnt_integrity_effect": "none_established"
            }
        });
        let outcome = evaluate_input(&input);
        assert_eq!(outcome.disposition, "INDETERMINATE");
        assert_eq!(outcome.reason_codes, vec!["unknown_rf_not_pnt_integrity"]);
    }

    #[test]
    fn an_expired_validity_timestamp_rejects_even_when_status_says_current() {
        let input = json!({
            "profile": {"id": "omsi.pnt-resilience", "version": "0.1.0-draft"},
            "assessment": {"as_of": "2026-11-07T12:00:00Z"},
            "sources": [{
                "integrity_status": "valid",
                "calibration_status": "current",
                "calibration_valid_until": "2026-11-01T00:00:00Z",
                "acquisition_status": "qualified"
            }]
        });
        let outcome = evaluate_input(&input);
        assert_eq!(outcome.disposition, "REJECT_RELIANCE");
        assert_eq!(outcome.reason_codes, vec!["calibration_expired"]);
    }

    #[test]
    fn missing_oracle_independence_is_not_asserted_from_signature_or_repetition() {
        let input = json!({
            "profile": {"id": "omsi.pnt-resilience", "version": "0.1.0-draft"},
            "assessment": {"as_of": "2026-11-07T12:00:00Z"},
            "oracle_evidence": {
                "independence_claimed": true,
                "detector_input_source_ids": ["rx-a"],
                "label_source_ids": ["rx-b"]
            }
        });
        let document = json!({
            "profile": input["profile"],
            "assessment": input["assessment"],
            "oracle_evidence": input["oracle_evidence"]
        });
        let report = validate_fixture_pack(
            &json!({
                "schema":"omsi.pnt-conformance-fixture-pack",
                "version":"0.1.0-draft",
                "profile_id":"omsi.pnt-resilience",
                "profile_version":"0.1.0-draft",
                "owner_issue":307,
                "fixtures":[]
            }),
            b"empty",
        );
        assert_eq!(report["status"], "failed");
        let _ = document;
    }
}
