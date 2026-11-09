use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const COMMITMENT_SCHEMA: &str = "luminous.sealed-challenge-commitment/0.1";
pub const REVEAL_SCHEMA: &str = "luminous.sealed-challenge-reveal/0.1";
pub const ORDER_SCHEMA: &str = "luminous.sealed-challenge-order-receipt/0.1";
pub const PUBLIC_MANIFEST_SCHEMA: &str = "luminous.sealed-challenge-public-manifest/0.1";
pub const VERIFIER_SCHEMA: &str = "luminous.sealed-challenge-verification-report/0.1";

const HASH_DOMAIN: &[u8] = b"LUMINOUS-SEALED-ASSURANCE-COMMITMENT\0v0.1\0";
const SALT_BYTES: usize = 32;
const CLAIM_CEILING: &str = "Verifies exact-byte consistency of a revealed challenge against a prior commitment and checks ordering/access assertions supplied by referenced monitor and custody receipts. It does not authenticate those receipts, prove evaluator secrecy, prove runtime isolation, or establish physical measurement truth.";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommitmentReceipt {
    pub schema: String,
    pub run_id: String,
    pub suite_id: String,
    pub protocol_version: String,
    pub public_input_manifest_sha256: String,
    pub scoring_policy_sha256: String,
    pub evaluator_build_sha256: String,
    pub commitment_sha256: String,
    pub commitment_sequence: u64,
    pub custody_receipt_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RevealReceipt {
    pub schema: String,
    pub run_id: String,
    pub suite_id: String,
    pub protocol_version: String,
    pub public_input_manifest_sha256: String,
    pub scoring_policy_sha256: String,
    pub evaluator_build_sha256: String,
    pub private_bundle_sha256: String,
    pub private_bundle_length: u64,
    pub salt_hex: String,
    pub candidate_output_bundle_sha256: String,
    pub reveal_sequence: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunOrderReceipt {
    pub schema: String,
    pub run_id: String,
    pub monitor_source_id: Option<String>,
    pub monitor_control_domain: Option<String>,
    pub monitor_receipt_sha256: Option<String>,
    pub custody_receipt_sha256: Option<String>,
    pub commitment_sequence: u64,
    pub first_execution_sequence: u64,
    pub last_execution_sequence: u64,
    pub output_lock_sequence: u64,
    pub candidate_output_bundle_sha256: String,
    pub coverage_complete: bool,
    pub sequence_gap_count: u64,
    pub dropped_event_count: u64,
    pub private_access_attempts: u64,
    pub private_access_successes: u64,
    pub grader_access_attempts: u64,
    pub grader_access_successes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicRunManifest {
    pub schema: String,
    pub run_id: String,
    pub suite_id: String,
    pub protocol_version: String,
    pub public_input_manifest_sha256: String,
    pub scoring_policy_sha256: String,
    pub candidate_visible_bundle_sha256: String,
    pub allowed_resource_classes: Vec<String>,
    pub claim_ceiling: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    Verified,
    Indeterminate,
    Rejected,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationCheck {
    pub id: String,
    pub result: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationReport {
    pub schema: String,
    pub status: VerificationStatus,
    pub checks: Vec<VerificationCheck>,
    pub reason_codes: Vec<String>,
    pub claim_ceiling: String,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn update_field(hasher: &mut Sha256, field: &[u8]) {
    hasher.update((field.len() as u64).to_be_bytes());
    hasher.update(field);
}

pub fn compute_commitment_sha256(
    receipt: &CommitmentReceipt,
    salt: &[u8],
    private_bundle_bytes: &[u8],
) -> Result<String, String> {
    if salt.len() != SALT_BYTES {
        return Err("salt_must_be_exactly_32_bytes".to_owned());
    }

    let sequence_bytes = receipt.commitment_sequence.to_be_bytes();
    let custody_digest = receipt.custody_receipt_sha256.as_deref().unwrap_or("");
    let mut hasher = Sha256::new();
    hasher.update(HASH_DOMAIN);

    for field in [
        receipt.schema.as_bytes(),
        receipt.run_id.as_bytes(),
        receipt.suite_id.as_bytes(),
        receipt.protocol_version.as_bytes(),
        receipt.public_input_manifest_sha256.as_bytes(),
        receipt.scoring_policy_sha256.as_bytes(),
        receipt.evaluator_build_sha256.as_bytes(),
        sequence_bytes.as_slice(),
        custody_digest.as_bytes(),
        salt,
        private_bundle_bytes,
    ] {
        update_field(&mut hasher, field);
    }

    let digest = hasher.finalize();
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    Ok(output)
}

pub fn verify_bundle(
    commitment: &CommitmentReceipt,
    reveal: &RevealReceipt,
    order: &RunOrderReceipt,
    private_bundle_bytes: &[u8],
) -> VerificationReport {
    let mut checks = Vec::new();
    let mut failures = Vec::new();
    let mut unknowns = Vec::new();

    check_equal(
        &mut checks,
        &mut failures,
        "commitment_schema",
        commitment.schema == COMMITMENT_SCHEMA,
    );
    check_equal(
        &mut checks,
        &mut failures,
        "reveal_schema",
        reveal.schema == REVEAL_SCHEMA,
    );
    check_equal(
        &mut checks,
        &mut failures,
        "order_receipt_schema",
        order.schema == ORDER_SCHEMA,
    );

    let identifiers_match = commitment.run_id == reveal.run_id
        && commitment.run_id == order.run_id
        && commitment.suite_id == reveal.suite_id
        && commitment.protocol_version == reveal.protocol_version;
    check_equal(
        &mut checks,
        &mut failures,
        "run_suite_and_protocol_binding",
        identifiers_match,
    );

    let public_policy_evaluator_match = commitment.public_input_manifest_sha256
        == reveal.public_input_manifest_sha256
        && commitment.scoring_policy_sha256 == reveal.scoring_policy_sha256
        && commitment.evaluator_build_sha256 == reveal.evaluator_build_sha256;
    check_equal(
        &mut checks,
        &mut failures,
        "public_input_policy_and_evaluator_binding",
        public_policy_evaluator_match,
    );

    let digest_fields_valid = [
        commitment.public_input_manifest_sha256.as_str(),
        commitment.scoring_policy_sha256.as_str(),
        commitment.evaluator_build_sha256.as_str(),
        commitment.commitment_sha256.as_str(),
        reveal.public_input_manifest_sha256.as_str(),
        reveal.scoring_policy_sha256.as_str(),
        reveal.evaluator_build_sha256.as_str(),
        reveal.private_bundle_sha256.as_str(),
        reveal.candidate_output_bundle_sha256.as_str(),
        order.candidate_output_bundle_sha256.as_str(),
    ]
    .iter()
    .all(|value| is_sha256_hex(value));
    check_equal(
        &mut checks,
        &mut failures,
        "digest_field_shapes",
        digest_fields_valid,
    );

    check_equal(
        &mut checks,
        &mut failures,
        "private_bundle_exact_byte_digest",
        is_sha256_hex(&reveal.private_bundle_sha256)
            && sha256_hex(private_bundle_bytes) == reveal.private_bundle_sha256,
    );
    check_equal(
        &mut checks,
        &mut failures,
        "private_bundle_exact_byte_length",
        reveal.private_bundle_length == private_bundle_bytes.len() as u64,
    );

    match decode_salt(&reveal.salt_hex) {
        Ok(salt_bytes) => {
            checks.push(VerificationCheck {
                id: "secret_salt_shape".to_owned(),
                result: "pass".to_owned(),
            });
            match compute_commitment_sha256(commitment, &salt_bytes, private_bundle_bytes) {
                Ok(computed) => check_equal(
                    &mut checks,
                    &mut failures,
                    "pre_run_commitment_reconstruction",
                    computed == commitment.commitment_sha256,
                ),
                Err(_) => check_equal(
                    &mut checks,
                    &mut failures,
                    "pre_run_commitment_reconstruction",
                    false,
                ),
            }
        }
        Err(_) => {
            checks.push(VerificationCheck {
                id: "secret_salt_shape".to_owned(),
                result: "fail".to_owned(),
            });
            failures.push("secret_salt_invalid".to_owned());
        }
    }

    check_equal(
        &mut checks,
        &mut failures,
        "commitment_order_binding",
        commitment.commitment_sequence == order.commitment_sequence,
    );

    let ordered = order.commitment_sequence < order.first_execution_sequence
        && order.first_execution_sequence <= order.last_execution_sequence
        && order.last_execution_sequence < order.output_lock_sequence
        && order.output_lock_sequence < reveal.reveal_sequence;
    check_equal(
        &mut checks,
        &mut failures,
        "commit_before_execution_and_reveal_after_output_lock",
        ordered,
    );

    check_equal(
        &mut checks,
        &mut failures,
        "candidate_output_lock_binding",
        reveal.candidate_output_bundle_sha256 == order.candidate_output_bundle_sha256,
    );

    let custody_binding_matches = commitment.custody_receipt_sha256 == order.custody_receipt_sha256
        && commitment
            .custody_receipt_sha256
            .as_deref()
            .is_some_and(is_sha256_hex);
    if !custody_binding_matches {
        unknowns.push("custody_receipt_missing_or_mismatched".to_owned());
    }
    checks.push(VerificationCheck {
        id: "custody_receipt_binding".to_owned(),
        result: if custody_binding_matches {
            "pass".to_owned()
        } else {
            "indeterminate".to_owned()
        },
    });

    let monitor_receipt_valid = order
        .monitor_receipt_sha256
        .as_deref()
        .is_some_and(is_sha256_hex)
        && order
            .monitor_source_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && order
            .monitor_control_domain
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
    if !monitor_receipt_valid {
        unknowns.push("independent_monitor_receipt_missing".to_owned());
    }
    checks.push(VerificationCheck {
        id: "independent_monitor_receipt".to_owned(),
        result: if monitor_receipt_valid {
            "pass".to_owned()
        } else {
            "indeterminate".to_owned()
        },
    });

    let coverage_complete =
        order.coverage_complete && order.sequence_gap_count == 0 && order.dropped_event_count == 0;
    if !coverage_complete {
        unknowns.push("monitor_coverage_incomplete_or_discontinuous".to_owned());
    }
    checks.push(VerificationCheck {
        id: "monitor_coverage".to_owned(),
        result: if coverage_complete {
            "pass".to_owned()
        } else {
            "indeterminate".to_owned()
        },
    });

    let counts_consistent = order.private_access_successes <= order.private_access_attempts
        && order.grader_access_successes <= order.grader_access_attempts;
    check_equal(
        &mut checks,
        &mut failures,
        "access_count_consistency",
        counts_consistent,
    );

    let no_private_access = order.private_access_attempts == 0
        && order.private_access_successes == 0
        && order.grader_access_attempts == 0
        && order.grader_access_successes == 0;
    check_equal(
        &mut checks,
        &mut failures,
        "no_private_or_grader_access_attempts",
        no_private_access,
    );

    if !no_private_access {
        failures.push("private_or_grader_access_attempted".to_owned());
    }

    failures.sort();
    failures.dedup();
    unknowns.sort();
    unknowns.dedup();

    let status = if !failures.is_empty() {
        VerificationStatus::Rejected
    } else if !unknowns.is_empty() {
        VerificationStatus::Indeterminate
    } else {
        VerificationStatus::Verified
    };

    let mut reason_codes = failures;
    reason_codes.extend(unknowns);
    reason_codes.sort();
    reason_codes.dedup();

    VerificationReport {
        schema: VERIFIER_SCHEMA.to_owned(),
        status,
        checks,
        reason_codes,
        claim_ceiling: CLAIM_CEILING.to_owned(),
    }
}

pub fn parse_public_manifest(bytes: &[u8]) -> Result<PublicRunManifest, String> {
    let manifest: PublicRunManifest =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;

    if manifest.schema != PUBLIC_MANIFEST_SCHEMA {
        return Err("unsupported_public_manifest_schema".to_owned());
    }
    if manifest.run_id.trim().is_empty()
        || manifest.suite_id.trim().is_empty()
        || manifest.protocol_version.trim().is_empty()
    {
        return Err("public_manifest_identity_fields_empty".to_owned());
    }
    for digest in [
        manifest.public_input_manifest_sha256.as_str(),
        manifest.scoring_policy_sha256.as_str(),
        manifest.candidate_visible_bundle_sha256.as_str(),
    ] {
        if !is_sha256_hex(digest) {
            return Err("public_manifest_digest_invalid".to_owned());
        }
    }
    if manifest.allowed_resource_classes.is_empty()
        || manifest
            .allowed_resource_classes
            .iter()
            .any(|value| value.trim().is_empty())
    {
        return Err("public_manifest_resource_allowlist_empty_or_invalid".to_owned());
    }
    if manifest.claim_ceiling.trim().is_empty() {
        return Err("public_manifest_claim_ceiling_empty".to_owned());
    }
    Ok(manifest)
}

fn decode_salt(value: &str) -> Result<Vec<u8>, String> {
    if value.len() != SALT_BYTES * 2 {
        return Err("salt_hex_must_encode_32_bytes".to_owned());
    }
    let mut output = Vec::with_capacity(SALT_BYTES);
    for index in (0..value.len()).step_by(2) {
        let byte = u8::from_str_radix(&value[index..index + 2], 16)
            .map_err(|_| "salt_hex_invalid".to_owned())?;
        output.push(byte);
    }
    Ok(output)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn check_equal(
    checks: &mut Vec<VerificationCheck>,
    failures: &mut Vec<String>,
    id: &str,
    passed: bool,
) {
    checks.push(VerificationCheck {
        id: id.to_owned(),
        result: if passed {
            "pass".to_owned()
        } else {
            "fail".to_owned()
        },
    });
    if !passed {
        failures.push(format!("{id}_failed"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const PRIVATE_BYTES: &[u8] =
        br#"{"expected":"SHARED_DEPENDENCY","hidden_edge":"grandmaster-a"}"#;

    fn digest(byte: u8) -> String {
        [byte; 32]
            .iter()
            .map(|value| format!("{value:02x}"))
            .collect()
    }

    fn fixture() -> (CommitmentReceipt, RevealReceipt, RunOrderReceipt, Vec<u8>) {
        let private = PRIVATE_BYTES.to_vec();
        let salt = vec![0x5a; SALT_BYTES];
        let mut commitment = CommitmentReceipt {
            schema: COMMITMENT_SCHEMA.to_owned(),
            run_id: "blind-run-001".to_owned(),
            suite_id: "dependency-hidden-001".to_owned(),
            protocol_version: "0.1.0".to_owned(),
            public_input_manifest_sha256: digest(b'a'),
            scoring_policy_sha256: digest(b'b'),
            evaluator_build_sha256: digest(b'c'),
            commitment_sha256: String::new(),
            commitment_sequence: 10,
            custody_receipt_sha256: Some(digest(b'd')),
        };
        commitment.commitment_sha256 =
            compute_commitment_sha256(&commitment, &salt, &private).expect("commitment");

        let reveal = RevealReceipt {
            schema: REVEAL_SCHEMA.to_owned(),
            run_id: commitment.run_id.clone(),
            suite_id: commitment.suite_id.clone(),
            protocol_version: commitment.protocol_version.clone(),
            public_input_manifest_sha256: commitment.public_input_manifest_sha256.clone(),
            scoring_policy_sha256: commitment.scoring_policy_sha256.clone(),
            evaluator_build_sha256: commitment.evaluator_build_sha256.clone(),
            private_bundle_sha256: sha256_hex(&private),
            private_bundle_length: private.len() as u64,
            salt_hex: salt.iter().map(|value| format!("{value:02x}")).collect(),
            candidate_output_bundle_sha256: digest(b'e'),
            reveal_sequence: 16,
        };

        let order = RunOrderReceipt {
            schema: ORDER_SCHEMA.to_owned(),
            run_id: commitment.run_id.clone(),
            monitor_source_id: Some("external-monitor-01".to_owned()),
            monitor_control_domain: Some("isolated-evaluator-domain".to_owned()),
            monitor_receipt_sha256: Some(digest(b'f')),
            custody_receipt_sha256: commitment.custody_receipt_sha256.clone(),
            commitment_sequence: 10,
            first_execution_sequence: 11,
            last_execution_sequence: 13,
            output_lock_sequence: 15,
            candidate_output_bundle_sha256: digest(b'e'),
            coverage_complete: true,
            sequence_gap_count: 0,
            dropped_event_count: 0,
            private_access_attempts: 0,
            private_access_successes: 0,
            grader_access_attempts: 0,
            grader_access_successes: 0,
        };

        (commitment, reveal, order, private)
    }

    #[test]
    fn valid_commitment_and_post_lock_reveal_verify() {
        let (commitment, reveal, order, private) = fixture();
        let report = verify_bundle(&commitment, &reveal, &order, &private);
        assert_eq!(report.status, VerificationStatus::Verified);
        assert!(report.reason_codes.is_empty());
    }

    #[test]
    fn private_bundle_byte_mutation_is_rejected() {
        let (commitment, reveal, order, mut private) = fixture();
        private.push(b' ');
        let report = verify_bundle(&commitment, &reveal, &order, &private);
        assert_eq!(report.status, VerificationStatus::Rejected);
    }

    #[test]
    fn salt_mutation_is_rejected() {
        let (commitment, mut reveal, order, private) = fixture();
        reveal.salt_hex = "00".repeat(SALT_BYTES);
        let report = verify_bundle(&commitment, &reveal, &order, &private);
        assert_eq!(report.status, VerificationStatus::Rejected);
    }

    #[test]
    fn policy_substitution_is_rejected() {
        let (commitment, mut reveal, order, private) = fixture();
        reveal.scoring_policy_sha256 = digest(b'9');
        let report = verify_bundle(&commitment, &reveal, &order, &private);
        assert_eq!(report.status, VerificationStatus::Rejected);
    }

    #[test]
    fn suite_substitution_is_rejected() {
        let (commitment, mut reveal, order, private) = fixture();
        reveal.suite_id = "another-suite".to_owned();
        let report = verify_bundle(&commitment, &reveal, &order, &private);
        assert_eq!(report.status, VerificationStatus::Rejected);
    }

    #[test]
    fn commit_after_execution_start_is_rejected() {
        let (commitment, reveal, mut order, private) = fixture();
        order.first_execution_sequence = order.commitment_sequence;
        let report = verify_bundle(&commitment, &reveal, &order, &private);
        assert_eq!(report.status, VerificationStatus::Rejected);
    }

    #[test]
    fn reveal_before_output_lock_is_rejected() {
        let (commitment, mut reveal, order, private) = fixture();
        reveal.reveal_sequence = order.output_lock_sequence;
        let report = verify_bundle(&commitment, &reveal, &order, &private);
        assert_eq!(report.status, VerificationStatus::Rejected);
    }

    #[test]
    fn missing_monitor_receipt_is_indeterminate() {
        let (commitment, reveal, mut order, private) = fixture();
        order.monitor_receipt_sha256 = None;
        let report = verify_bundle(&commitment, &reveal, &order, &private);
        assert_eq!(report.status, VerificationStatus::Indeterminate);
    }

    #[test]
    fn incomplete_monitor_coverage_is_indeterminate() {
        let (commitment, reveal, mut order, private) = fixture();
        order.coverage_complete = false;
        let report = verify_bundle(&commitment, &reveal, &order, &private);
        assert_eq!(report.status, VerificationStatus::Indeterminate);
    }

    #[test]
    fn sequence_gap_is_indeterminate() {
        let (commitment, reveal, mut order, private) = fixture();
        order.sequence_gap_count = 1;
        let report = verify_bundle(&commitment, &reveal, &order, &private);
        assert_eq!(report.status, VerificationStatus::Indeterminate);
    }

    #[test]
    fn blocked_private_access_attempt_is_rejected() {
        let (commitment, reveal, mut order, private) = fixture();
        order.private_access_attempts = 1;
        let report = verify_bundle(&commitment, &reveal, &order, &private);
        assert_eq!(report.status, VerificationStatus::Rejected);
    }

    #[test]
    fn successful_grader_access_is_rejected() {
        let (commitment, reveal, mut order, private) = fixture();
        order.grader_access_attempts = 1;
        order.grader_access_successes = 1;
        let report = verify_bundle(&commitment, &reveal, &order, &private);
        assert_eq!(report.status, VerificationStatus::Rejected);
    }

    #[test]
    fn public_manifest_rejects_private_truth_fields() {
        let manifest = json!({
            "schema": PUBLIC_MANIFEST_SCHEMA,
            "run_id": "run-1",
            "suite_id": "suite-1",
            "protocol_version": "0.1.0",
            "public_input_manifest_sha256": digest(b'a'),
            "scoring_policy_sha256": digest(b'b'),
            "candidate_visible_bundle_sha256": digest(b'c'),
            "allowed_resource_classes": ["public_fixture", "instrument_metadata"],
            "claim_ceiling": "synthetic_only",
            "private_bundle_sha256": digest(b'd'),
            "expected_outcomes": ["SHARED_DEPENDENCY"]
        });
        assert!(parse_public_manifest(manifest.to_string().as_bytes()).is_err());
    }

    #[test]
    fn public_manifest_accepts_only_public_contract_fields() {
        let manifest = json!({
            "schema": PUBLIC_MANIFEST_SCHEMA,
            "run_id": "run-1",
            "suite_id": "suite-1",
            "protocol_version": "0.1.0",
            "public_input_manifest_sha256": digest(b'a'),
            "scoring_policy_sha256": digest(b'b'),
            "candidate_visible_bundle_sha256": digest(b'c'),
            "allowed_resource_classes": ["public_fixture", "instrument_metadata"],
            "claim_ceiling": "synthetic_only"
        });
        assert!(parse_public_manifest(manifest.to_string().as_bytes()).is_ok());
    }

    #[test]
    fn private_bundle_commitment_is_bound_to_exact_byte_representation() {
        let (commitment, _, _, private) = fixture();
        let mut changed = private.clone();
        changed.push(b'\n');
        let salt = vec![0x5a; SALT_BYTES];
        let original = compute_commitment_sha256(&commitment, &salt, &private).expect("original");
        let changed_hash =
            compute_commitment_sha256(&commitment, &salt, &changed).expect("changed");
        assert_ne!(original, changed_hash);
    }

    #[test]
    fn non_32_byte_salt_is_rejected() {
        let (commitment, _, _, private) = fixture();
        assert!(compute_commitment_sha256(&commitment, &[0u8; 31], &private).is_err());
    }
}
