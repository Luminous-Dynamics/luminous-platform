#!/usr/bin/env python3
"""Structural guard for Service Episode Attestation v0.1 synthetic conformance vectors."""
from __future__ import annotations
import json
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
PREDICATE = ROOT / "benchmarks" / "service-episode-attestation-v0.1" / "predicate.schema.json"
BUNDLE = ROOT / "benchmarks" / "service-episode-attestation-v0.1" / "bundle.schema.json"
VECTORS = ROOT / "benchmarks" / "service-episode-attestation-v0.1" / "conformance-vectors.json"

EXPECTED_ROLES = {"coordinator", "participant", "independent_observer", "authority"}
EXPECTED_IDS = {
 "VSA-VALID-001",
 "VSA-SIG-001",
 "VSA-SIG-002",
 "VSA-SIG-003",
 "VSA-SIG-004",
 "VSA-ISS-001",
 "VSA-ISS-002",
 "VSA-ISS-003",
 "VSA-ROLE-001",
 "VSA-ROLE-002",
 "VSA-ROLE-003",
 "VSA-ROLE-004",
 "VSA-ROLE-005",
 "VSA-BIND-001",
 "VSA-BIND-002",
 "VSA-BIND-003",
 "VSA-BIND-004",
 "VSA-BIND-005",
 "VSA-BIND-006",
 "VSA-BIND-007",
 "VSA-FRESH-001",
 "VSA-FRESH-002",
 "VSA-FRESH-003",
 "VSA-FRESH-004",
 "VSA-SEM-001",
 "VSA-SEM-002",
 "VSA-SEM-003",
 "VSA-SEM-004",
 "VSA-SCHEMA-001",
 "VSA-PRIV-001",
 "VSA-LOG-001",
 "VSA-ROLE-006",
 "VSA-SEM-005"
}
EXPECTED_DECISIONS = {
 "VSA-VALID-001": "accept",
 "VSA-SIG-001": "reject",
 "VSA-SIG-002": "reject",
 "VSA-SIG-003": "reject",
 "VSA-SIG-004": "reject",
 "VSA-ISS-001": "reject",
 "VSA-ISS-002": "reject",
 "VSA-ISS-003": "reject",
 "VSA-ROLE-001": "reject",
 "VSA-ROLE-002": "reject",
 "VSA-ROLE-003": "reject",
 "VSA-ROLE-004": "reject",
 "VSA-ROLE-005": "reject",
 "VSA-BIND-001": "reject",
 "VSA-BIND-002": "reject",
 "VSA-BIND-003": "reject",
 "VSA-BIND-004": "reject",
 "VSA-BIND-005": "reject",
 "VSA-BIND-006": "reject",
 "VSA-BIND-007": "reject",
 "VSA-FRESH-001": "reject",
 "VSA-FRESH-002": "reject",
 "VSA-FRESH-003": "reject",
 "VSA-FRESH-004": "reject",
 "VSA-SEM-001": "indeterminate",
 "VSA-SEM-002": "indeterminate",
 "VSA-SEM-003": "indeterminate",
 "VSA-SEM-004": "reject",
 "VSA-SCHEMA-001": "reject",
 "VSA-PRIV-001": "reject",
 "VSA-LOG-001": "reject",
 "VSA-ROLE-006": "reject",
 "VSA-SEM-005": "reject"
}
TOP_KEYS = {"schema","version","status","predicate_type","envelope_profile","claim_ceiling","trust_policy","vectors"}
INPUT_KEYS = set(["signature_integrity","payload_type_integrity","pae_valid","issuer_key_status","issuer_registry_status","trust_bundle_status","required_roles","claimed_role","role_claim_alignment","coordinator_observer_control_domains","tenant_binding","plan_binding","target_binding","operation_binding","request_binding","attempt_binding","idempotency_binding","freshness","nonce_state","issuer_sequence_state","effect","consistency","observer_evidence","postcondition_binding","predicate_version","transparency_requirement","transparency_proof","privacy_commitment","service_claim_signer_role","extra_signatures_from_same_role"])
VECTOR_KEYS = {"id","category","visibility","input","expected"}

def fail(message: str) -> None:
    raise ValueError(message)

def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)

def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate JSON key: {key}")
        result[key] = value
    return result

def load(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        fail(f"cannot load JSON {path}: {exc}")

def validate(vectors: Any, predicate: Any, bundle: Any) -> dict[str, int]:
    require(isinstance(vectors, dict) and set(vectors) == TOP_KEYS, "conformance root keys drifted")
    require(vectors["schema"] == "luminous-service-episode-attestation-conformance-v0.1"
            and vectors["version"] == 1, "conformance identity/version drifted")
    require(vectors["status"] == "synthetic_evaluator_only_not_runtime_input", "vectors must remain evaluator-only")
    require(vectors["predicate_type"] == "https://luminousdynamics.io/attestations/service-episode/v0.1",
            "predicate URI drifted")
    require(vectors["envelope_profile"] == "DSSE_v1.0_over_in_toto_Statement_v1", "envelope profile drifted")
    require(vectors["claim_ceiling"] == "E0_E1_schema_and_synthetic_vectors_only", "claim ceiling drifted")
    policy = vectors["trust_policy"]
    require(set(policy) == {"policy_version","required_signer_roles","independent_observer_requires_distinct_control_domain",
            "keyid_is_identity_proof","signed_effect_claim_requires_participant_role","service_resolution_requires_independent_observer_role",
            "unknown_effect_never_resolves","unknown_major_version_fails_closed","anti_replay","transparency_mode"},
            "trust policy field inventory drifted")
    require(set(policy["required_signer_roles"]) == EXPECTED_ROLES and len(policy["required_signer_roles"]) == 4,
            "four distinct role requirements are mandatory")
    require(policy["independent_observer_requires_distinct_control_domain"] is True
            and policy["keyid_is_identity_proof"] is False
            and policy["signed_effect_claim_requires_participant_role"] is True
            and policy["service_resolution_requires_independent_observer_role"] is True
            and policy["unknown_effect_never_resolves"] is True
            and policy["unknown_major_version_fails_closed"] is True,
            "trust policy safety invariant weakened")
    require(set(policy["anti_replay"]) == {"nonce","monotonic_issuer_sequence","signed_expiry"}
            and len(policy["anti_replay"]) == 3, "anti-replay controls drifted")

    items = vectors["vectors"]
    require(isinstance(items, list) and len(items) == 32, "exactly 32 vectors required")
    ids: set[str] = set()
    categories: set[str] = set()
    for item in items:
        require(isinstance(item, dict) and set(item) == VECTOR_KEYS, "vector envelope shape drifted")
        vid = item["id"]
        require(vid in EXPECTED_IDS and vid not in ids, f"unknown or duplicate vector ID: {vid}")
        ids.add(vid)
        require(item["visibility"] == "evaluator_only_not_sut_input", f"{vid}: not evaluator-only")
        require(isinstance(item["input"], dict) and set(item["input"]) == INPUT_KEYS,
                f"{vid}: input schema drifted")
        require(isinstance(item["expected"], dict) and set(item["expected"]) == {"decision","reason_code"},
                f"{vid}: expected result shape drifted")
        require(item["expected"]["decision"] == EXPECTED_DECISIONS[vid],
                f"{vid}: decision contract drifted")
        require(isinstance(item["expected"]["reason_code"], str) and item["expected"]["reason_code"],
                f"{vid}: reason code missing")
        categories.add(item["category"])
    require(ids == EXPECTED_IDS, "frozen vector inventory changed")
    require(len(categories) == 10, "all ten conformance categories are required")

    require(predicate.get("$schema") == "https://json-schema.org/draft/2020-12/schema",
            "predicate schema must be Draft 2020-12")
    pred_props = predicate.get("properties", {})
    require(predicate.get("additionalProperties") is False, "predicate schema must reject unknown fields")
    require(pred_props.get("claim_kind", {}).get("enum") ==
            ["coordinator_action","participant_effect","service_outcome","authority_grant"],
            "predicate claim vocabulary drifted")
    require(len(predicate.get("allOf", [])) == 4, "each claim kind requires a role binding rule")
    issuer_props = pred_props.get("issuer", {}).get("properties", {})
    require(set(issuer_props.get("claimed_role", {}).get("enum", [])) == EXPECTED_ROLES,
            "predicate role vocabulary drifted")
    binding_props = pred_props.get("binding", {}).get("properties", {})
    for digest_field in ("target_digest","request_digest"):
        require(binding_props.get(digest_field, {}).get("pattern") == "^sha256:[a-f0-9]{64}$",
                f"predicate {digest_field} must be SHA-256 bound")
    freshness_props = pred_props.get("freshness", {}).get("properties", {})
    require("nonce" in freshness_props and "issuer_sequence" in freshness_props,
            "signed nonce and issuer sequence are required")

    require(bundle.get("$schema") == "https://json-schema.org/draft/2020-12/schema",
            "bundle schema must be Draft 2020-12")
    bundle_props = bundle.get("properties", {})
    require(bundle.get("additionalProperties") is False, "bundle schema must reject unknown fields")
    require(bundle_props.get("payload_type", {}).get("const") == "application/vnd.in-toto+json",
            "DSSE payload type is not pinned")
    require(bundle_props.get("required_roles", {}).get("const") == [
        "coordinator","participant","independent_observer","authority"
    ], "bundle schema must require four roles")
    signature_props = bundle_props.get("attestations", {}).get("items", {}).get("properties", {}).get("signatures", {})
    require(signature_props.get("minItems") == 1, "DSSE envelope must require at least one signature")
    return {"vector_count":len(ids),"category_count":len(categories),"required_role_count":len(EXPECTED_ROLES)}

def main() -> int:
    try:
        result = validate(load(VECTORS), load(PREDICATE), load(BUNDLE))
    except (ValueError, KeyError, TypeError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    print("PASS: Service Episode Attestation v0.1 structural contract")
    for key, value in result.items():
        print(f"{key}={value}")
    print("SCOPE: structural schemas and synthetic decision vectors only; no cryptographic verifier.")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
