use crate::adapter::apply_adapter_map;
use crate::replay::verify_replay_envelope;
use omsi_manufacturing_validator::evaluate_input_value;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const MANIFEST_CONTRACT: &str = "omsi-manufacturing-provenance-manifest/0.1";
const MAX_TRANSFORMS: usize = 100_000;
const LIMITATIONS: &[&str] = &[
    "The manifest records consistency and SHA-256 commitments over exact supplied bytes; it does not authenticate a producer, tenant, device, calibration issuer, or physical measurement.",
    "The manifest is unsigned. A party able to replace artifacts and recompute the manifest can create a new internally consistent bundle.",
    "JSON canonicalization is not performed. Whitespace or property-order changes alter artifact digests even when parsed values are equivalent.",
    "Projection re-execution, semantic evaluation, and replay-envelope verification use this validator implementation; this is not independent cross-implementation verification.",
    "A consistent evidence chain is not a measurement-uncertainty analysis, vendor-schema conformance claim, quality-release decision, regulatory certificate, or machine-control authorization.",
    "Artifact bytes are not embedded in the manifest. Verifiers must obtain each artifact and independently recompute its digest before relying on the references.",
];

/// Builds a local-consistency manifest only after every supplied link is rechecked.
/// All digests cover exact bytes; this function does not sign or authenticate them.
pub fn create_provenance_manifest(
    source_record_bytes: &[u8],
    adapter_map_bytes: &[u8],
    projection_report_bytes: &[u8],
    semantic_input_bytes: &[u8],
    semantic_report_bytes: &[u8],
    replay_envelope_bytes: &[u8],
) -> Result<Value, String> {
    let source_record = parse_json(source_record_bytes, "source record")?;
    let adapter_map = parse_json(adapter_map_bytes, "adapter map")?;
    let supplied_projection = parse_json(projection_report_bytes, "projection report")?;
    let semantic_input = parse_json(semantic_input_bytes, "semantic input")?;
    let supplied_semantic_report = parse_json(semantic_report_bytes, "semantic report")?;
    let replay_envelope = parse_json(replay_envelope_bytes, "replay envelope")?;

    let regenerated_projection = apply_adapter_map(
        &adapter_map,
        adapter_map_bytes,
        &source_record,
        source_record_bytes,
    );
    if regenerated_projection.get("status").and_then(Value::as_str) != Some("projection_created") {
        return Err("adapter re-execution did not create a projection; refusing manifest".into());
    }
    if regenerated_projection != supplied_projection {
        return Err("supplied projection report differs from a fresh adapter re-execution".into());
    }
    if supplied_projection.get("projection") != Some(&semantic_input) {
        return Err("semantic input is not exactly the projection payload".into());
    }

    let expected_semantic_report = evaluate_input_value(&semantic_input);
    if expected_semantic_report != supplied_semantic_report {
        return Err("supplied semantic report differs from fresh semantic evaluation".into());
    }
    let replay_check = verify_replay_envelope(&replay_envelope, semantic_input_bytes);
    if replay_check.get("status").and_then(Value::as_str) != Some("replay_consistent") {
        return Err("replay envelope does not verify against the exact semantic-input bytes".into());
    }

    let mapping_list = adapter_map
        .get("mappings")
        .and_then(Value::as_array)
        .ok_or_else(|| "adapter map is missing its mappings array".to_owned())?;
    if mapping_list.len() > MAX_TRANSFORMS {
        return Err(format!("adapter map exceeds the {MAX_TRANSFORMS}-mapping limit"));
    }
    let mut transform_ids = BTreeSet::new();
    for (index, mapping) in mapping_list.iter().enumerate() {
        if mapping.get("disposition").and_then(Value::as_str) == Some("transform") {
            let id = mapping
                .get("transform_id")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| format!("transform mapping {index} has no transform_id"))?;
            transform_ids.insert(id.to_owned());
        }
    }

    let profile = nonempty_string(&supplied_semantic_report, "profile")?;
    let comparison_rule_id = nonempty_string(&supplied_semantic_report, "comparison_rule_id")?;
    let validator_version = nonempty_string(&supplied_semantic_report, "validator_version")?;

    Ok(json!({
        "manifest_contract": MANIFEST_CONTRACT,
        "status": "local_chain_consistent",
        "authority": "local_consistency_only",
        "digest_profile": {
            "algorithm": "sha256",
            "scope": "exact_input_bytes",
            "canonicalization": "none"
        },
        "profile": profile,
        "comparison_rule_id": comparison_rule_id,
        "validator_version": validator_version,
        "transform_ids": transform_ids.into_iter().collect::<Vec<_>>(),
        "artifacts": [
            artifact("source-record", "source_record", source_record_bytes),
            artifact("adapter-map", "adapter_map", adapter_map_bytes),
            artifact("projection-report", "projection_report", projection_report_bytes),
            artifact("semantic-input", "semantic_input", semantic_input_bytes),
            artifact("semantic-report", "semantic_report", semantic_report_bytes),
            artifact("replay-envelope", "replay_envelope", replay_envelope_bytes)
        ],
        "edges": [
            {
                "from": "source-record",
                "to": "adapter-map",
                "relationship": "consumed_by"
            },
            {
                "from": "source-record",
                "to": "projection-report",
                "relationship": "projected_to",
                "evidence": ["adapter-map"]
            },
            {
                "from": "projection-report",
                "to": "semantic-input",
                "relationship": "payload_matches"
            },
            {
                "from": "semantic-input",
                "to": "semantic-report",
                "relationship": "evaluated_to"
            },
            {
                "from": "semantic-input",
                "to": "replay-envelope",
                "relationship": "bound_by_exact_bytes"
            }
        ],
        "limitations": LIMITATIONS
    }))
}

fn parse_json(bytes: &[u8], label: &str) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|error| format!("{label} is not valid JSON: {error}"))
}

fn nonempty_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("semantic report is missing non-empty {field}"))
}

fn artifact(artifact_id: &str, role: &str, bytes: &[u8]) -> Value {
    json!({
        "artifact_id": artifact_id,
        "role": role,
        "sha256": sha256_hex(bytes),
        "byte_length": bytes.len()
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::create_provenance_manifest;
    use crate::adapter::apply_adapter_map;
    use crate::replay::create_replay_envelope;
    use jsonschema::draft202012;
    use omsi_manufacturing_validator::evaluate_input_value;
    use serde_json::{json, Value};

    const MAP: &str = include_str!(
        "../../../docs/standards/omsi/adapters/0.1/examples/qms-nested-map.json"
    );
    const SOURCE: &str = include_str!(
        "../../../docs/standards/omsi/adapters/0.1/examples/qms-nested-record.json"
    );
    const SCHEMA: &str = include_str!("../schema/provenance-manifest.schema.json");

    fn fixture_chain() -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
        let map: Value = serde_json::from_str(MAP).expect("map fixture parses");
        let source: Value = serde_json::from_str(SOURCE).expect("source fixture parses");
        let projection = apply_adapter_map(&map, MAP.as_bytes(), &source, SOURCE.as_bytes());
        assert_eq!(projection["status"], "projection_created");

        let projection_bytes = serde_json::to_vec_pretty(&projection).expect("projection serializes");
        let semantic_input_bytes = serde_json::to_vec_pretty(&projection["projection"])
            .expect("semantic input serializes");
        let semantic_input: Value =
            serde_json::from_slice(&semantic_input_bytes).expect("semantic input parses");
        let semantic_report = evaluate_input_value(&semantic_input);
        let semantic_report_bytes =
            serde_json::to_vec_pretty(&semantic_report).expect("semantic report serializes");
        let replay_envelope =
            create_replay_envelope(&semantic_input_bytes).expect("envelope is created");
        let replay_envelope_bytes =
            serde_json::to_vec_pretty(&replay_envelope).expect("envelope serializes");

        (
            SOURCE.as_bytes().to_vec(),
            MAP.as_bytes().to_vec(),
            projection_bytes,
            semantic_input_bytes,
            semantic_report_bytes,
            replay_envelope_bytes,
        )
    }

    #[test]
    fn manifest_rechecks_each_edge_and_matches_the_independent_schema() {
        let (source, map, projection, input, report, envelope) = fixture_chain();
        let manifest = create_provenance_manifest(&source, &map, &projection, &input, &report, &envelope)
            .expect("consistent evidence chain emits a manifest");
        let schema: Value = serde_json::from_str(SCHEMA).expect("manifest schema parses");
        assert!(draft202012::meta::is_valid(&schema), "schema meta-validation failed");
        let validator = draft202012::new(&schema).expect("manifest schema compiles");
        assert!(validator.is_valid(&manifest), "manifest does not match schema: {manifest:#}");
        assert_eq!(manifest["status"], "local_chain_consistent");
        assert_eq!(manifest["authority"], "local_consistency_only");
        assert_eq!(manifest["artifacts"].as_array().unwrap().len(), 6);
        assert_eq!(manifest["edges"].as_array().unwrap().len(), 5);
        assert_eq!(manifest["transform_ids"].as_array().unwrap().len(), 2);
        assert!(manifest.get("signature").is_none());
    }

    #[test]
    fn manifest_rejects_projection_report_drift() {
        let (source, map, mut projection, input, report, envelope) = fixture_chain();
        let mut parsed: Value =
            serde_json::from_slice(&projection).expect("projection report parses");
        parsed["projection"]["source_quality_claim"]["raw_status"] = json!("ACCEPTED");
        projection = serde_json::to_vec_pretty(&parsed).expect("mutated projection serializes");
        let result = create_provenance_manifest(&source, &map, &projection, &input, &report, &envelope);
        assert!(result.unwrap_err().contains("differs from a fresh adapter re-execution"));
    }

    #[test]
    fn manifest_rejects_replay_envelope_bound_to_different_bytes() {
        let (source, map, projection, input, report, mut envelope) = fixture_chain();
        let mut parsed: Value =
            serde_json::from_slice(&envelope).expect("replay envelope parses");
        parsed["artifact_binding"]["digest"] = json!("0".repeat(64));
        envelope = serde_json::to_vec_pretty(&parsed).expect("tampered envelope serializes");
        let result = create_provenance_manifest(&source, &map, &projection, &input, &report, &envelope);
        assert!(result.unwrap_err().contains("replay envelope does not verify"));
    }

    #[test]
    fn byte_level_change_to_source_is_not_silently_ignored() {
        let (mut source, map, projection, input, report, envelope) = fixture_chain();
        source.push(b' ');
        let result = create_provenance_manifest(&source, &map, &projection, &input, &report, &envelope);
        assert!(result.unwrap_err().contains("differs from a fresh adapter re-execution"));
    }
}
