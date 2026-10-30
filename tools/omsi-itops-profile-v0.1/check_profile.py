#!/usr/bin/env python3
"""Dependency-free structural guard for OMSI ITOPS synthetic profile vectors.

This validates profile-level semantic fixtures and frozen upstream references.
It does not run connectors, service tasks, Saga transitions or an OMSI adapter.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
VECTOR_PATH = ROOT / "benchmarks" / "omsi-itops-profile-v0.1" / "conformance-vectors.json"
SCHEMA_PATH = ROOT / "benchmarks" / "omsi-itops-profile-v0.1" / "schema.json"
REFERENCES_PATH = ROOT / "benchmarks" / "omsi-itops-profile-v0.1" / "upstream-crosswalk.json"

EXPECTED_PROFILE = "omsi:technology-services-operations:0.1-incubation"
EXPECTED_OPSBENCH_COMMIT = "c3a746afd56f36b791783f49855cbf7736024f6f"
EXPECTED_OPSBENCH_BLOB = "1de8c499abb8ca995223bae8ef42037cc04fa34e"
EXPECTED_SAGA_COMMIT = "d445116bc86c607f1f782c10869138a1cbcc076c"
EXPECTED_SAGA_BLOB = "ee0ed9c3e8e4d10bd165d3a2bde816f146aaa977"

EXPECTED_OPSBENCH_IDS = {
    "INC-001", "INC-002", "INC-003", "CHG-001", "CHG-002",
    "PATCH-001", "BACKUP-001", "IAM-001", "IAM-002", "NET-001",
    "DNS-001", "CERT-001", "DB-001", "QUEUE-001", "ALERT-001",
    "TENANT-001", "PROMPT-001", "CHG-003", "CONN-001", "SLA-001",
}
EXPECTED_SAGA_IDS = {
    "CAS-001", "PLAN-001", "AUTH-001", "DISPATCH-001",
    "NOEFFECT-001", "NOEFFECT-002", "CANCEL-001", "OBS-001",
    "COMP-001", "COMP-002", "SERVICE-001", "FLEET-001",
}
EXPECTED_VECTOR_CONTRACTS = {
    "ITOPS-OBS-001": ("negative_control", "conflicting", "none"),
    "ITOPS-HYP-001": ("negative_control", "hypothesis_only", "none"),
    "ITOPS-AUTH-001": ("negative_control", "rejected_before_dispatch", "none_for_this_target"),
    "ITOPS-TENANT-001": ("negative_control", "blocked_and_audited", "none"),
    "ITOPS-DISPATCH-001": ("negative_control", "outcome_unknown", "no_automatic_retry"),
    "ITOPS-NOEFFECT-001": ("negative_control", "indeterminate", "retry_blocked"),
    "ITOPS-NOEFFECT-002": ("positive_with_gate", "retry_eligible_after_new_grant", "one_new_exact_attempt"),
    "ITOPS-CANCEL-001": ("negative_control", "partially_cancelled_with_inflight_unknown", "no_new_dispatch"),
    "ITOPS-COMP-001": ("negative_control", "compensation_rejected_without_distinct_grant", "none"),
    "ITOPS-SERVICE-001": ("negative_control", "service_not_verified", "none"),
    "ITOPS-SERVICE-002": ("positive_with_gate", "service_verified", "none_after_terminal_resolution"),
    "ITOPS-COMMS-001": ("negative_control", "communication_outcome_unknown", "no_automatic_resend_or_success_claim"),
}
TOP_KEYS = {
    "schema", "profile_id", "profile_version", "status", "purpose",
    "visibility", "upstream_crosswalk", "common_rules", "vectors",
}
VECTOR_KEYS = {
    "id", "category", "classification", "maps_to", "input", "expected",
    "forbidden_claims",
}
MAP_KEYS = {"opsbench_case", "saga_fault"}
EXPECTED_COMMON_RULE_COUNT = 10
FORBIDDEN_SUT_INPUT_KEYS = {
    "answer", "answer_key", "expected_answer", "expected_outcome",
    "seeded_fault", "oracle_channels", "evaluator_private",
    "hidden_state", "target_final_state", "judge_packet", "ground_truth",
}


def fail(message: str) -> None:
    raise ValueError(message)


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        fail(f"cannot load valid UTF-8 JSON from {path}: {exc}")


def reject_evaluator_leakage(value: Any, path: str = "input") -> None:
    if isinstance(value, dict):
        for key, nested in value.items():
            require(key not in FORBIDDEN_SUT_INPUT_KEYS, f"{path}: evaluator-only field leaked: {key}")
            reject_evaluator_leakage(nested, f"{path}.{key}")
    elif isinstance(value, list):
        for index, nested in enumerate(value):
            reject_evaluator_leakage(nested, f"{path}[{index}]")


def validate_references(references: Any) -> dict[str, int]:
    require(isinstance(references, dict), "upstream crosswalk root must be an object")
    required = {
        "schema", "version", "status", "profile_version", "upstreams",
        "opsbench_case_ids", "saga_fault_ids", "comparability_rules",
    }
    require(set(references) == required, "upstream crosswalk top-level fields drifted")
    require(references["schema"] == "omsi-itops-upstream-crosswalk-v0.1", "unexpected crosswalk schema")
    require(type(references["version"]) is int and references["version"] == 1, "crosswalk version drifted")
    require(references["status"] == "pinned_reference_inventory_not_runtime_input", "crosswalk must remain non-runtime input")
    require(references["profile_version"] == "0.1", "crosswalk profile version mismatch")

    upstreams = references["upstreams"]
    require(isinstance(upstreams, list) and len(upstreams) == 2, "exactly two upstream references are required")
    upstream_by_id = {item.get("id"): item for item in upstreams if isinstance(item, dict)}
    require(set(upstream_by_id) == {"opsbench_v0_1", "service_episode_saga_v1"}, "upstream reference inventory changed")

    ops = upstream_by_id["opsbench_v0_1"]
    require(ops.get("pull_request") == 149
            and ops.get("commit") == EXPECTED_OPSBENCH_COMMIT
            and ops.get("blob_sha") == EXPECTED_OPSBENCH_BLOB
            and ops.get("path") == "benchmarks/technology-services-ops-v0.1/corpus.json",
            "OpsBench source pin changed")
    saga = upstream_by_id["service_episode_saga_v1"]
    require(saga.get("pull_request") == 153
            and saga.get("commit") == EXPECTED_SAGA_COMMIT
            and saga.get("blob_sha") == EXPECTED_SAGA_BLOB
            and saga.get("path") == "benchmarks/service-episode-saga-v1/fault-matrix.json",
            "Saga fault-matrix source pin changed")

    ops_ids = references["opsbench_case_ids"]
    saga_ids = references["saga_fault_ids"]
    require(isinstance(ops_ids, list) and set(ops_ids) == EXPECTED_OPSBENCH_IDS
            and len(ops_ids) == len(set(ops_ids)), "OpsBench ID inventory changed")
    require(isinstance(saga_ids, list) and set(saga_ids) == EXPECTED_SAGA_IDS
            and len(saga_ids) == len(set(saga_ids)), "Saga fault ID inventory changed")
    rules = references["comparability_rules"]
    require(isinstance(rules, list) and len(rules) == 4 and len(set(rules)) == 4,
            "benchmark comparability rules are missing or duplicated")
    require(any("not OpsBench task success scores" in item for item in rules),
            "profile scores must remain separate from OpsBench task success")
    require(any("not live connector or service restoration results" in item for item in rules),
            "Saga vectors must not be presented as live connector results")
    return {"opsbench_case_count": len(ops_ids), "saga_fault_count": len(saga_ids)}


def validate(vectors: Any, schema: Any, references: Any) -> dict[str, int]:
    require(isinstance(vectors, dict), "vector root must be an object")
    require(set(vectors) == TOP_KEYS, "vector top-level keys drifted")
    require(vectors.get("schema") == "omsi-itops-conformance-vectors", "unexpected vector schema ID")
    require(vectors.get("profile_id") == EXPECTED_PROFILE, "unexpected profile ID")
    require(vectors.get("profile_version") == "0.1", "profile version drifted")
    require(vectors.get("status") == "synthetic_research_fixture_not_normative_wire_schema",
            "synthetic/non-normative status must remain explicit")
    require(vectors.get("visibility") == "profile_evaluator_only_not_sut_input",
            "profile conformance vectors must not be labelled as runtime/SUT input")
    require(vectors.get("upstream_crosswalk") == "upstream-crosswalk.json",
            "profile must reference its pinned upstream crosswalk")
    require(isinstance(vectors.get("purpose"), str) and len(vectors["purpose"]) >= 30,
            "profile vector purpose is missing")

    common_rules = vectors.get("common_rules")
    require(isinstance(common_rules, list) and len(common_rules) == EXPECTED_COMMON_RULE_COUNT
            and len(common_rules) == len(set(common_rules)), "common profile rules drifted")
    joined_rules = " ".join(common_rules).lower()
    for required_phrase in (
        "read-only", "recommendation", "never accepted", "compensation",
        "service-level evidence", "tenant boundaries", "synthetic",
    ):
        require(required_phrase in joined_rules, f"common rules omit mandatory phrase: {required_phrase}")

    ref_summary = validate_references(references)
    require(isinstance(schema, dict), "schema root must be an object")
    require(schema.get("$schema") == "https://json-schema.org/draft/2020-12/schema",
            "schema must declare JSON Schema Draft 2020-12")
    require(schema.get("properties", {}).get("profile_id", {}).get("const") == EXPECTED_PROFILE,
            "schema profile ID drifted")
    require(schema.get("properties", {}).get("visibility", {}).get("const") == "profile_evaluator_only_not_sut_input",
            "schema does not enforce evaluator-only visibility")
    require(schema.get("properties", {}).get("vectors", {}).get("minItems") == 12
            and schema.get("properties", {}).get("vectors", {}).get("maxItems") == 12,
            "schema must freeze exactly twelve vectors")

    items = vectors.get("vectors")
    require(isinstance(items, list) and len(items) == 12, "exactly twelve ITOPS vectors are required")
    seen: set[str] = set()
    ops_ids = set(references["opsbench_case_ids"])
    saga_ids = set(references["saga_fault_ids"])

    for index, item in enumerate(items):
        require(isinstance(item, dict) and set(item) == VECTOR_KEYS,
                f"vector {index} has unknown or missing fields")
        vector_id = item.get("id")
        require(isinstance(vector_id, str) and re.fullmatch(r"ITOPS-[A-Z0-9]+-[0-9]{3}", vector_id),
                f"vector {index} has invalid ID")
        require(vector_id not in seen, f"duplicate vector ID: {vector_id}")
        seen.add(vector_id)
        require(vector_id in EXPECTED_VECTOR_CONTRACTS, f"unexpected vector ID: {vector_id}")

        classification, disposition, authority = EXPECTED_VECTOR_CONTRACTS[vector_id]
        require(item.get("classification") == classification, f"{vector_id}: classification changed")
        expected = item.get("expected")
        require(isinstance(expected, dict), f"{vector_id}: expected projection is missing")
        require(expected.get("profile_disposition") == disposition, f"{vector_id}: disposition changed")
        require(expected.get("action_authority") == authority, f"{vector_id}: authority boundary changed")
        require(isinstance(expected.get("assertions"), list) and len(expected["assertions"]) >= 2,
                f"{vector_id}: at least two semantic assertions are required")
        require(isinstance(item.get("input"), dict) and item["input"], f"{vector_id}: synthetic input is empty")
        reject_evaluator_leakage(item["input"], f"{vector_id}.input")

        mapping = item.get("maps_to")
        require(isinstance(mapping, dict) and set(mapping) == MAP_KEYS,
                f"{vector_id}: invalid benchmark crosswalk shape")
        ops_case = mapping.get("opsbench_case")
        saga_fault = mapping.get("saga_fault")
        require(ops_case is None or ops_case in ops_ids, f"{vector_id}: unknown OpsBench case {ops_case}")
        require(saga_fault is None or saga_fault in saga_ids, f"{vector_id}: unknown Saga fault {saga_fault}")
        require(ops_case is not None or saga_fault is not None, f"{vector_id}: vector has no upstream mapping")

        forbidden = item.get("forbidden_claims")
        require(isinstance(forbidden, list) and forbidden and len(forbidden) == len(set(forbidden)),
                f"{vector_id}: forbidden claim list missing or duplicated")

    require(seen == set(EXPECTED_VECTOR_CONTRACTS), "frozen ITOPS vector inventory changed")
    return {
        "vector_count": len(seen),
        "opsbench_case_count": ref_summary["opsbench_case_count"],
        "saga_fault_count": ref_summary["saga_fault_count"],
    }


def main() -> int:
    try:
        result = validate(
            load_json(VECTOR_PATH),
            load_json(SCHEMA_PATH),
            load_json(REFERENCES_PATH),
        )
    except ValueError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    print("PASS: OMSI ITOPS profile structural contract")
    for key, value in result.items():
        print(f"{key}={value}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
