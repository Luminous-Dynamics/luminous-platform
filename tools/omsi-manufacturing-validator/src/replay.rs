use omsi_manufacturing_validator::evaluate_input_value;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const ENVELOPE_CONTRACT: &str = "omsi-manufacturing-replay-envelope/0.1";
const VERIFIER_CONTRACT: &str = "omsi-manufacturing-replay-verifier/0.1";
const ENVELOPE_LIMITATIONS: &[&str] = &[
    "The envelope is unsigned and does not authenticate the source, producer, tenant binding, calibration issuer, or physical measurement.",
    "The digest binds exact input bytes; whitespace or property-order changes produce a different digest.",
    "The digest is not an RFC 8785 canonical JSON digest and must not be described as one.",
    "Replay consistency means this validator version reproduced the embedded JSON report; it is not cross-implementation equivalence, quality conformity, product-release authority, or equipment-control authority.",
    "An attacker able to replace both the input artifact and envelope can recompute the digest; preserve or authenticate the envelope separately when adversarial tampering is in scope.",
];

pub fn create_replay_envelope(source: &[u8]) -> Result<Value, String> {
    let input: Value = serde_json::from_slice(source)
        .map_err(|error| format!("invalid JSON input: {error}"))?;
    let report = evaluate_input_value(&input);

    Ok(json!({
        "envelope_contract": ENVELOPE_CONTRACT,
        "artifact_binding": {
            "digest_algorithm": "sha256",
            "digest": sha256_hex(source),
            "digest_scope": "exact_input_bytes",
            "canonicalization": "none",
            "byte_length": source.len()
        },
        "report": report,
        "limitations": ENVELOPE_LIMITATIONS
    }))
}

pub fn verify_replay_envelope(envelope: &Value, source: &[u8]) -> Value {
    let envelope_contract_ok = envelope.get("envelope_contract").and_then(Value::as_str)
        == Some(ENVELOPE_CONTRACT);
    let envelope_shape_ok = envelope.as_object().is_some_and(|object| {
        object.len() == 4
            && object.contains_key("envelope_contract")
            && object.contains_key("artifact_binding")
            && object.contains_key("report")
            && object.contains_key("limitations")
    });

    let binding = envelope.get("artifact_binding").and_then(Value::as_object);
    let binding_contract_ok = binding.is_some_and(|object| {
        object.len() == 5
            && object.get("digest_algorithm").and_then(Value::as_str) == Some("sha256")
            && object.get("digest").and_then(Value::as_str).is_some_and(is_sha256_hex)
            && object.get("digest_scope").and_then(Value::as_str)
                == Some("exact_input_bytes")
            && object.get("canonicalization").and_then(Value::as_str) == Some("none")
            && object.get("byte_length").and_then(Value::as_u64).is_some()
    });
    let length_matches = binding
        .and_then(|object| object.get("byte_length"))
        .and_then(Value::as_u64)
        == Some(source.len() as u64);
    let digest_matches = binding
        .and_then(|object| object.get("digest"))
        .and_then(Value::as_str)
        .is_some_and(|digest| digest == sha256_hex(source));

    let parsed_input = serde_json::from_slice::<Value>(source);
    let input_json_valid = parsed_input.is_ok();
    let report_replays = match parsed_input.as_ref() {
        Ok(input) => envelope.get("report") == Some(&evaluate_input_value(input)),
        Err(_) => false,
    };
    let expected_limitations = json!(ENVELOPE_LIMITATIONS);
    let limitations_match = envelope.get("limitations") == Some(&expected_limitations);

    let checks = vec![
        check(
            "envelope-contract",
            envelope_contract_ok,
            "supported replay-envelope contract",
            "unsupported or missing replay-envelope contract",
        ),
        check(
            "envelope-shape",
            envelope_shape_ok,
            "envelope contains exactly the contract fields",
            "envelope fields are missing or unrecognized",
        ),
        check(
            "artifact-binding-contract",
            binding_contract_ok,
            "artifact binding uses the declared raw-byte SHA-256 profile",
            "artifact binding is malformed or declares unsupported digest semantics",
        ),
        check(
            "input-byte-length",
            length_matches,
            "input byte length matches the envelope",
            "input byte length differs from the bound artifact length",
        ),
        check(
            "input-sha256",
            digest_matches,
            "SHA-256 matches the exact supplied input bytes",
            "SHA-256 does not match the exact supplied input bytes",
        ),
        check(
            "input-json",
            input_json_valid,
            "input parses as JSON for replay",
            "input bytes are not valid JSON",
        ),
        check(
            "semantic-report-replay",
            report_replays,
            "embedded report matches a fresh evaluation of the supplied input",
            "embedded report differs from a fresh evaluation or cannot be replayed",
        ),
        check(
            "envelope-limitations",
            limitations_match,
            "required security limitations are preserved",
            "security limitations were removed or altered",
        ),
    ];
    let replay_consistent = checks
        .iter()
        .all(|entry| entry.get("result").and_then(Value::as_str) == Some("pass"));

    json!({
        "verifier_contract": VERIFIER_CONTRACT,
        "status": if replay_consistent { "replay_consistent" } else { "rejected" },
        "authority": "replay_consistency_only",
        "checks": checks,
        "limitations": ENVELOPE_LIMITATIONS
    })
}

fn check(id: &str, passed: bool, pass_reason: &str, fail_reason: &str) -> Value {
    json!({
        "id": id,
        "result": if passed { "pass" } else { "fail" },
        "explanation": if passed { pass_reason } else { fail_reason }
    })
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(source: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(source);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::{create_replay_envelope, sha256_hex, verify_replay_envelope};
    use serde_json::{json, Value};

    const VECTORS_JSON: &str = include_str!(
        "../../../docs/standards/omsi/profiles/manufacturing-asset-operations/0.1/CONFORMANCE-VECTORS.json"
    );

    fn sample_input_bytes() -> Vec<u8> {
        let document: Value =
            serde_json::from_str(VECTORS_JSON).expect("committed vectors must parse");
        serde_json::to_vec(&document["vectors"][0]["input"])
            .expect("sample input must serialize")
    }

    #[test]
    fn sha256_implementation_matches_standard_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn generated_envelope_replays_with_identical_input_bytes() {
        let source = sample_input_bytes();
        let envelope = create_replay_envelope(&source).expect("valid source");
        let result = verify_replay_envelope(&envelope, &source);
        assert_eq!(result["status"], "replay_consistent", "{result:#}");
        assert_eq!(result["authority"], "replay_consistency_only");
        assert!(result["checks"].as_array().unwrap().iter()
            .all(|entry| entry["result"] == "pass"));
    }

    #[test]
    fn whitespace_change_fails_exact_byte_binding_even_if_json_is_equivalent() {
        let source = sample_input_bytes();
        let envelope = create_replay_envelope(&source).expect("valid source");
        let mut changed = b" \n".to_vec();
        changed.extend_from_slice(&source);
        let result = verify_replay_envelope(&envelope, &changed);
        assert_eq!(result["status"], "rejected");
        assert!(result["checks"].as_array().unwrap().iter()
            .any(|entry| entry["id"] == "input-sha256" && entry["result"] == "fail"));
        assert!(result["checks"].as_array().unwrap().iter()
            .any(|entry| entry["id"] == "semantic-report-replay" && entry["result"] == "pass"));
    }

    #[test]
    fn tampered_report_is_rejected_even_when_input_digest_matches() {
        let source = sample_input_bytes();
        let mut envelope = create_replay_envelope(&source).expect("valid source");
        envelope["report"]["decision_trace"][0]["interpretation"] =
            json!("forged explanation");
        let result = verify_replay_envelope(&envelope, &source);
        assert_eq!(result["status"], "rejected");
        assert!(result["checks"].as_array().unwrap().iter()
            .any(|entry| entry["id"] == "semantic-report-replay" && entry["result"] == "fail"));
    }

    #[test]
    fn tampered_digest_is_rejected() {
        let source = sample_input_bytes();
        let mut envelope = create_replay_envelope(&source).expect("valid source");
        envelope["artifact_binding"]["digest"] = json!("0".repeat(64));
        let result = verify_replay_envelope(&envelope, &source);
        assert_eq!(result["status"], "rejected");
        assert!(result["checks"].as_array().unwrap().iter()
            .any(|entry| entry["id"] == "input-sha256" && entry["result"] == "fail"));
    }

    #[test]
    fn unsupported_contract_and_modified_limitations_are_rejected() {
        let source = sample_input_bytes();
        let mut envelope = create_replay_envelope(&source).expect("valid source");
        envelope["envelope_contract"] = json!("omsi-manufacturing-replay-envelope/9.0");
        envelope["limitations"] = json!([]);
        let result = verify_replay_envelope(&envelope, &source);
        assert_eq!(result["status"], "rejected");
        assert!(result["checks"].as_array().unwrap().iter()
            .any(|entry| entry["id"] == "envelope-contract" && entry["result"] == "fail"));
        assert!(result["checks"].as_array().unwrap().iter()
            .any(|entry| entry["id"] == "envelope-limitations" && entry["result"] == "fail"));
    }

    #[test]
    fn malformed_input_cannot_be_enveloped_or_replay_consistent() {
        assert!(create_replay_envelope(b"{not-json").is_err());
        let result = verify_replay_envelope(&json!({}), b"{not-json");
        assert_eq!(result["status"], "rejected");
        assert!(result["checks"].as_array().unwrap().iter()
            .any(|entry| entry["id"] == "input-json" && entry["result"] == "fail"));
    }

    #[test]
    fn envelope_does_not_claim_signer_or_release_authority() {
        let source = sample_input_bytes();
        let envelope = create_replay_envelope(&source).expect("valid source");
        assert!(envelope.get("signature").is_none());
        assert_eq!(envelope["report"]["decision_authority"], "diagnostic_only");
        assert_eq!(envelope["report"]["quality_release_authority"], "none");
        assert!(envelope["limitations"].as_array().unwrap().len() >= 4);
    }
}
