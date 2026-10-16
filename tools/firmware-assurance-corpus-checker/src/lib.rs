//! Structural and contract validation for the synthetic benchmark corpus.
//!
//! This crate does not verify firmware artifacts, signer trust, device identity,
//! vendor compatibility, or any physical-world claim. It checks that the test
//! corpus is well-formed and safe to hand to a later, separately qualified
//! firmware-assurance verifier.

use jsonschema::validator_for;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_CASES: usize = 10_000;
pub const REQUIRED_PROFILE: &str = "synthetic-uefi-uki-v1";
pub const REQUIRED_SCHEMA_VERSION: &str = "firmware-assurance-corpus/1.0.0";

const REQUIRED_READ_ONLY_ASSERTIONS: [&str; 3] = [
    "no_firmware_write",
    "no_key_enrollment",
    "no_boot_configuration_change",
];

/// Validate the exact corpus/schema byte inputs and return a deterministic JSON report.
///
/// A passed status only means the corpus contract passed. It is deliberately not
/// a firmware, cryptographic, hardware, or production-assurance verdict.
pub fn validate_corpus_bytes(corpus_bytes: &[u8], schema_bytes: &[u8]) -> Result<Value, String> {
    if corpus_bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(format!(
            "corpus exceeds the {}-byte input limit",
            MAX_DOCUMENT_BYTES
        ));
    }
    if schema_bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(format!(
            "schema exceeds the {}-byte input limit",
            MAX_DOCUMENT_BYTES
        ));
    }

    let corpus: Value = serde_json::from_slice(corpus_bytes)
        .map_err(|error| format!("invalid corpus JSON: {error}"))?;
    let schema: Value = serde_json::from_slice(schema_bytes)
        .map_err(|error| format!("invalid schema JSON: {error}"))?;

    let validator = validator_for(&schema)
        .map_err(|error| format!("schema compilation failed: {error}"))?;

    let mut schema_errors = validator
        .iter_errors(&corpus)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    schema_errors.sort();
    schema_errors.truncate(100);

    let contract_errors = validate_contract(&corpus);
    let passed = schema_errors.is_empty() && contract_errors.is_empty();

    let case_count = corpus
        .get("cases")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

    let mut kind_counts = Map::new();
    for kind in [
        "positive_control",
        "negative_control",
        "indeterminate_control",
    ] {
        let count = corpus
            .get("cases")
            .and_then(Value::as_array)
            .map(|cases| {
                cases
                    .iter()
                    .filter(|case| case.get("case_kind").and_then(Value::as_str) == Some(kind))
                    .count()
            })
            .unwrap_or(0);
        kind_counts.insert(kind.to_string(), json!(count));
    }

    Ok(json!({
        "report_version": "firmware-assurance-corpus-check/1.0.0",
        "status": if passed { "passed" } else { "blocked" },
        "scope": {
            "profile_id": REQUIRED_PROFILE,
            "physical_target": "none",
            "mutation_enabled": false,
            "claim_ceiling": "corpus_structure_only_not_firmware_verification"
        },
        "inputs": {
            "corpus_sha256": sha256_hex(corpus_bytes),
            "corpus_bytes": corpus_bytes.len(),
            "schema_sha256": sha256_hex(schema_bytes),
            "schema_bytes": schema_bytes.len()
        },
        "case_count": case_count,
        "case_kind_counts": Value::Object(kind_counts),
        "checks": {
            "json_parse": "passed",
            "json_schema": if schema_errors.is_empty() { "passed" } else { "failed" },
            "semantic_contract": if contract_errors.is_empty() { "passed" } else { "failed" }
        },
        "schema_errors": schema_errors,
        "contract_errors": contract_errors,
        "executed_cases": 0,
        "physical_device_operations": 0
    }))
}

/// Validate cross-field invariants that are intentionally stronger than JSON Schema.
///
/// This function checks corpus metadata and test-case bookkeeping. It does not
/// interpret prose setup/mutation strings as executable test procedures.
pub fn validate_contract(corpus: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if !corpus.is_object() {
        return vec!["corpus root must be a JSON object".to_string()];
    }

    expect_string(corpus, "schema_version", REQUIRED_SCHEMA_VERSION, &mut errors);
    expect_string(corpus, "status", "synthetic_only", &mut errors);
    expect_string(corpus, "physical_target", "none", &mut errors);
    expect_string(corpus, "profile_id", REQUIRED_PROFILE, &mut errors);

    if corpus.get("mutation_enabled").and_then(Value::as_bool) != Some(false) {
        errors.push("mutation_enabled must be false for this corpus".to_string());
    }

    if corpus
        .get("benchmark_goal")
        .and_then(Value::as_str)
        .map_or(true, str::is_empty)
    {
        errors.push("benchmark_goal must be a non-empty string".to_string());
    }

    let Some(cases) = corpus.get("cases").and_then(Value::as_array) else {
        errors.push("cases must be an array".to_string());
        return errors;
    };

    if cases.len() < 32 {
        errors.push("cases must contain at least 32 entries".to_string());
    }
    if cases.len() > MAX_CASES {
        errors.push(format!("cases exceeds the {MAX_CASES}-case limit"));
        return errors;
    }

    match corpus.get("case_count").and_then(Value::as_u64) {
        Some(count) if count == cases.len() as u64 => {}
        _ => errors.push("case_count must equal cases.len()".to_string()),
    }

    let root_profile = corpus.get("profile_id").and_then(Value::as_str);
    let mut seen_ids = BTreeSet::new();
    let mut kind_counts = [0_usize; 3];

    for (index, case) in cases.iter().enumerate() {
        let label = case
            .get("case_id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("case at index {index}"));

        let expected_id = format!("FWA-{:03}", index + 1);
        match case.get("case_id").and_then(Value::as_str) {
            Some(id) if id == expected_id => {}
            Some(id) => errors.push(format!(
                "{label}: case_id must be sequential {expected_id}, found {id}"
            )),
            None => errors.push(format!("{label}: case_id must be a string")),
        }

        if let Some(id) = case.get("case_id").and_then(Value::as_str) {
            if !seen_ids.insert(id.to_string()) {
                errors.push(format!("{label}: duplicate case_id"));
            }
        }

        if case.get("profile_id").and_then(Value::as_str) != root_profile {
            errors.push(format!("{label}: profile_id must match the corpus profile"));
        }

        let case_kind = case.get("case_kind").and_then(Value::as_str);
        let disposition = case
            .get("expected_disposition")
            .and_then(Value::as_str);
        let expected_disposition = match case_kind {
            Some("positive_control") => {
                kind_counts[0] += 1;
                Some("eligible_for_lab_staging")
            }
            Some("negative_control") => {
                kind_counts[1] += 1;
                Some("blocked")
            }
            Some("indeterminate_control") => {
                kind_counts[2] += 1;
                Some("indeterminate")
            }
            Some(other) => {
                errors.push(format!("{label}: unsupported case_kind {other}"));
                None
            }
            None => {
                errors.push(format!("{label}: case_kind must be a string"));
                None
            }
        };

        if expected_disposition != disposition {
            errors.push(format!(
                "{label}: expected_disposition is inconsistent with case_kind"
            ));
        }

        let Some(claims) = case.get("expected_claims").and_then(Value::as_object) else {
            errors.push(format!("{label}: expected_claims must be an object"));
            continue;
        };

        if claims.is_empty() {
            errors.push(format!("{label}: expected_claims must not be empty"));
        }

        let mut has_verified = false;
        let mut has_refuted = false;
        let mut has_unknown = false;
        for (claim_name, state) in claims {
            match state.as_str() {
                Some("verified") => has_verified = true,
                Some("refuted") => has_refuted = true,
                Some("unknown") => has_unknown = true,
                Some("not_applicable") => {}
                Some(other) => errors.push(format!(
                    "{label}: claim {claim_name} has unsupported state {other}"
                )),
                None => errors.push(format!(
                    "{label}: claim {claim_name} state must be a string"
                )),
            }
        }

        match case_kind {
            Some("positive_control") if !has_verified || has_refuted => errors.push(format!(
                "{label}: positive_control must include verified evidence and no refuted claims"
            )),
            Some("negative_control") if !has_refuted => errors.push(format!(
                "{label}: negative_control must contain at least one refuted claim"
            )),
            Some("indeterminate_control") if !has_unknown || has_refuted => errors.push(format!(
                "{label}: indeterminate_control must include unknown evidence and no refuted claims"
            )),
            _ => {}
        }

        let Some(assertions) = case.get("required_assertions").and_then(Value::as_array) else {
            errors.push(format!("{label}: required_assertions must be an array"));
            continue;
        };

        let mut unique_assertions = BTreeSet::new();
        for assertion in assertions {
            match assertion.as_str() {
                Some(value) if !unique_assertions.insert(value.to_string()) => {
                    errors.push(format!("{label}: duplicate required_assertion {value}"));
                }
                Some("firmware_write_allowed") => {
                    errors.push(format!("{label}: corpus must never allow firmware writes"));
                }
                Some(_) => {}
                None => errors.push(format!(
                    "{label}: every required_assertion must be a string"
                )),
            }
        }

        for required in REQUIRED_READ_ONLY_ASSERTIONS {
            if !unique_assertions.contains(required) {
                errors.push(format!(
                    "{label}: required_assertions must include {required}"
                ));
            }
        }
    }

    for (index, kind) in ["positive_control", "negative_control", "indeterminate_control"]
        .iter()
        .enumerate()
    {
        if kind_counts[index] == 0 {
            errors.push(format!("corpus must include at least one {kind}"));
        }
    }

    errors
}

fn expect_string(corpus: &Value, key: &str, expected: &str, errors: &mut Vec<String>) {
    if corpus.get(key).and_then(Value::as_str) != Some(expected) {
        errors.push(format!("{key} must equal {expected:?}"));
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{validate_contract, validate_corpus_bytes, MAX_DOCUMENT_BYTES};
    use serde_json::{json, Value};

    const CORPUS: &str = include_str!("../../../docs/fixtures/firmware-assurance-corpus-v1.json");
    const SCHEMA: &str = include_str!("../../../docs/schemas/firmware-assurance-corpus-v1.schema.json");

    fn parsed_corpus() -> Value {
        serde_json::from_str(CORPUS).expect("checked-in corpus JSON parses")
    }

    #[test]
    fn baseline_corpus_satisfies_contract_and_schema() {
        let report = validate_corpus_bytes(CORPUS.as_bytes(), SCHEMA.as_bytes())
            .expect("validator can compile schema and parse inputs");
        assert_eq!(report["status"], "passed", "{report:#}");
        assert_eq!(report["case_count"], json!(44));
        assert_eq!(report["scope"]["claim_ceiling"], "corpus_structure_only_not_firmware_verification");
    }

    #[test]
    fn corpus_digest_is_bound_to_exact_source_bytes() {
        let report = validate_corpus_bytes(CORPUS.as_bytes(), SCHEMA.as_bytes()).unwrap();
        let digest = report["inputs"]["corpus_sha256"].as_str().unwrap();
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|character| character.is_ascii_hexdigit()));
        let changed = format!("{CORPUS}\n");
        let changed_report =
            validate_corpus_bytes(changed.as_bytes(), SCHEMA.as_bytes()).unwrap();
        assert_ne!(digest, changed_report["inputs"]["corpus_sha256"].as_str().unwrap());
    }

    #[test]
    fn duplicate_or_nonsequential_case_ids_are_rejected() {
        let mut corpus = parsed_corpus();
        corpus["cases"][1]["case_id"] = json!("FWA-001");
        assert!(validate_contract(&corpus).iter().any(|error| error.contains("sequential")));
    }

    #[test]
    fn case_count_mismatch_is_rejected() {
        let mut corpus = parsed_corpus();
        corpus["case_count"] = json!(43);
        assert!(validate_contract(&corpus).iter().any(|error| error.contains("case_count")));
    }

    #[test]
    fn kind_disposition_mismatch_is_rejected() {
        let mut corpus = parsed_corpus();
        corpus["cases"][0]["expected_disposition"] = json!("blocked");
        assert!(validate_contract(&corpus).iter().any(|error| error.contains("inconsistent")));
    }

    #[test]
    fn mutation_enabled_is_rejected() {
        let mut corpus = parsed_corpus();
        corpus["mutation_enabled"] = json!(true);
        assert!(validate_contract(&corpus).iter().any(|error| error.contains("mutation_enabled")));
    }

    #[test]
    fn missing_read_only_assertion_is_rejected() {
        let mut corpus = parsed_corpus();
        corpus["cases"][0]["required_assertions"] = json!(["no_firmware_write"]);
        assert!(validate_contract(&corpus).iter().any(|error| error.contains("no_key_enrollment")));
    }

    #[test]
    fn unknown_case_fields_fail_json_schema_validation() {
        let mut corpus = parsed_corpus();
        corpus["cases"][0]["unexpected_security_override"] = json!(true);
        let altered = serde_json::to_vec(&corpus).unwrap();
        let report = validate_corpus_bytes(&altered, SCHEMA.as_bytes()).unwrap();
        assert_eq!(report["status"], "blocked");
        assert_eq!(report["checks"]["json_schema"], "failed");
    }

    #[test]
    fn malformed_json_is_a_parse_error_not_a_pass() {
        assert!(validate_corpus_bytes(b"{bad json", SCHEMA.as_bytes()).is_err());
    }

    #[test]
    fn oversized_input_is_rejected_before_parsing() {
        let oversized = vec![b' '; MAX_DOCUMENT_BYTES + 1];
        assert!(validate_corpus_bytes(&oversized, SCHEMA.as_bytes())
            .unwrap_err()
            .contains("input limit"));
    }
}
