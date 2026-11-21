#!/usr/bin/env python3
"""Validate resource-enforced commit-frontier fencing and late-writer controls."""
from __future__ import annotations
import json
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "benchmarks" / "agentic-endpoint-security-v1"
DIGEST = re.compile(r"^sha256:[a-f0-9]{64}$")
CONTROL_COUNTS = {"positive_control": 7, "negative_control": 17, "indeterminate_control": 4}


def load(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot load {path}: {exc}") from exc


def evaluate(case: dict[str, Any]) -> dict[str, Any]:
    state, cert = case["resource_state"], case["frontier_certificate"]
    operation, fence, events = case["operation"], case["fence"], case["target_events"]
    owner_unknown = (
        not state["owner_available"] or not state["high_water_persisted"]
        or not state["external_witness_independent"]
    )
    high_regression = int(state["external_witness_generation"] > state["high_water_generation"])
    if owner_unknown or not cert["complete"] or not cert["independent"] or not cert["witness_qualified"] or cert["read_consistency"] == "unknown":
        frontier = "unknown"
    elif cert["resource_id"] != state["resource_id"] or cert["observed_owner_revision"] != state["owner_revision"] or cert["read_consistency"] != "linearizable":
        frontier = "stale"
    elif cert["committed_through_epoch"] < operation["epoch"]:
        frontier = "behind"
    elif not cert["source_closed"] and cert["late_data_policy"] != "target_fence":
        frontier = "late_data_open"
    else:
        frontier = "ready"

    stale_token = (
        fence["generation"] < state["current_generation"]
        or fence["generation"] < state["high_water_generation"]
        or fence["generation"] < state["external_witness_generation"]
    )
    wrong_binding = (
        fence["resource_id"] != operation["resource_id"] or fence["tx_id"] != operation["tx_id"]
        or fence["branch_id"] != operation["branch_id"] or fence["grant_id"] != operation["grant_id"]
        or fence["policy_epoch"] != operation["policy_epoch"] or fence["operation_digest"] != operation["operation_digest"]
        or fence["frontier_certificate_id"] != cert["certificate_id"] or not fence["authenticated"]
    )
    policy_mismatch = operation["policy_epoch"] != state["policy_epoch"] or fence["policy_epoch"] != state["policy_epoch"]
    op_version_mismatch = operation["resource_version_precondition"] != state["resource_version_at_grant"]
    late_epoch = operation["epoch"] < state["sealed_through_epoch"]
    accepted = [e for e in events if e["decision"] == "accepted"]
    rejected = [e for e in events if e["decision"] == "rejected"]
    stale_token_acceptance = int(stale_token and bool(accepted))
    late_acceptance = int(late_epoch and bool(accepted))
    wrong_binding_acceptance = int(wrong_binding and bool(accepted)) + sum(
        1 for e in accepted if (
            e["resource_id"] != operation["resource_id"] or e["tx_id"] != operation["tx_id"]
            or e["branch_id"] != operation["branch_id"] or e["effect_id"] != operation["effect_id"]
            or e["token_id"] != fence["token_id"] or e["token_generation"] != fence["generation"]
            or e["grant_id"] != operation["grant_id"] or e["operation_digest"] != operation["operation_digest"]
        )
    )
    policy_acceptance = int(policy_mismatch and bool(accepted)) + sum(
        1 for e in accepted if e["policy_epoch"] != state["policy_epoch"]
    )
    expected_version = state["current_resource_version"]
    per_event_version = 0
    for event in accepted:
        if event["resource_version_before"] != expected_version:
            per_event_version += 1
        expected_version = event["resource_version_after"]
    version_acceptance = int(op_version_mismatch and bool(accepted)) + per_event_version
    toctou = sum(1 for e in accepted if (
        not e["atomic_compare_and_write"]
        or e["observed_generation_at_check"] != e["observed_highwater_at_commit"]
    ))
    atomic_failure = sum(1 for e in accepted if not e["atomic_compare_and_write"])
    high_mismatch = sum(1 for e in accepted if e["observed_highwater_at_commit"] != state["high_water_generation"])
    revision_mismatch = sum(1 for e in accepted if e["observed_owner_revision_at_commit"] != state["owner_revision"])
    invalid_observation = any(
        (e["decision"] == "accepted" and not e["effect_observed"])
        or (e["decision"] == "rejected" and e["effect_observed"]) for e in events
    )
    stale_frontier_acceptance = int(frontier != "ready" and bool(accepted))
    witness_correlation_acceptance = int(witness_domain_correlation_count > 0 and bool(accepted))
    valid = (
        not owner_unknown and frontier == "ready" and not stale_token and not wrong_binding
        and not policy_mismatch and not op_version_mismatch and not late_epoch and not high_regression
        and fence["generation"] == state["current_generation"]
        and fence["generation"] >= state["high_water_generation"]
    )
    false_reject = int(valid and bool(rejected))
    violations = (
        stale_token_acceptance + late_acceptance + wrong_binding_acceptance + policy_acceptance
        + version_acceptance + toctou + high_regression + atomic_failure + high_mismatch
        + revision_mismatch + stale_frontier_acceptance + witness_correlation_acceptance
        + int(invalid_observation)
    )
    if violations:
        disposition = "violation"
    elif owner_unknown or frontier in {"unknown", "behind", "late_data_open"} or not events:
        disposition = "indeterminate"
    elif frontier == "stale" and rejected and len(rejected) == len(events):
        disposition = "qualified_deny"
    elif accepted and len(accepted) == len(events) and valid:
        disposition = "qualified_allow"
    elif rejected and len(rejected) == len(events) and not valid:
        disposition = "qualified_deny"
    elif false_reject:
        disposition = "false_reject"
    else:
        disposition = "indeterminate"
    return {
        "frontier_state": frontier,
        "oic_unknown_domain_count": oic_unknown_domain_count,
        "witness_domain_correlation_count": witness_domain_correlation_count,
        "witness_correlation_acceptance_count": witness_correlation_acceptance,
        "stale_token_acceptance_count": stale_token_acceptance,
        "late_epoch_acceptance_count": late_acceptance,
        "wrong_binding_acceptance_count": wrong_binding_acceptance,
        "policy_epoch_acceptance_count": policy_acceptance,
        "stale_resource_version_acceptance_count": version_acceptance,
        "stale_frontier_acceptance_count": stale_frontier_acceptance,
        "toctou_race_count": toctou,
        "high_water_regression_count": high_regression,
        "atomic_fence_failure_count": atomic_failure,
        "target_owner_revision_mismatch_count": revision_mismatch,
        "target_high_water_mismatch_count": high_mismatch,
        "false_reject_count": false_reject,
        "target_effect_count": sum(1 for e in accepted if e["effect_observed"]),
        "disposition": disposition,
    }


def validate_case(case: dict[str, Any], catalog_ids: set[str]) -> None:
    case_id = case.get("case_id", "<unknown>")
    if set(case) != {"case_id", "control_class", "resource_state", "frontier_certificate", "operation", "fence", "target_events", "reconciliation", "expected"}:
        raise ValueError(f"{case_id}: case fields differ")
    state, cert = case["resource_state"], case["frontier_certificate"]
    operation, fence = case["operation"], case["fence"]
    oic = case["oic_binding"]
    if set(oic) != {"profile_id", "oic_case_id", "required_disjoint_domains", "subject_domains", "resource_owner_domains", "witness"}:
        raise ValueError(f"{case_id}: OIC binding fields differ")
    if oic["profile_id"] != "critical_effect_truth_v0" or oic["required_disjoint_domains"] != list(CRITICAL_DOMAINS):
        raise ValueError(f"{case_id}: OIC critical effect-truth profile cannot be weakened")
    for domain_map_name in ("subject_domains", "resource_owner_domains"):
        domain_map = oic[domain_map_name]
        if set(domain_map) != set(DOMAIN_FIELDS):
            raise ValueError(f"{case_id}: {domain_map_name} must carry the OIC domain inventory")
    witness = oic["witness"]
    if set(witness) != {"witness_id", "role", "domains", "evidence_ref", "attestation_ref"}:
        raise ValueError(f"{case_id}: frontier witness identity fields differ")
    if witness["role"] not in {"resource_observer", "host_observer", "independent_oracle", "supervisor", "source_inventory"}:
        raise ValueError(f"{case_id}: invalid OIC witness role")
    if set(witness["domains"]) != set(DOMAIN_FIELDS):
        raise ValueError(f"{case_id}: witness must carry the OIC domain inventory")
    if not witness["witness_id"] or not witness["evidence_ref"] or not witness["attestation_ref"]:
        raise ValueError(f"{case_id}: OIC witness identity/evidence references must be nonempty")
    if set(state) != {"resource_id", "current_generation", "high_water_generation", "sealed_through_epoch", "owner_revision", "resource_version_at_grant", "current_resource_version", "policy_epoch", "owner_available", "high_water_persisted", "external_witness_generation", "external_witness_independent"}:
        raise ValueError(f"{case_id}: resource-state fields differ")
    for key in ("current_generation", "high_water_generation", "sealed_through_epoch", "owner_revision", "external_witness_generation"):
        if type(state[key]) is not int or state[key] < 0:
            raise ValueError(f"{case_id}: invalid resource high-water field")
    if set(cert) != {"certificate_id", "resource_id", "committed_through_epoch", "observed_owner_revision", "read_consistency", "complete", "independent", "witness_qualified", "allowed_lateness_epochs", "source_closed", "late_data_policy", "witness_ref"}:
        raise ValueError(f"{case_id}: certificate fields differ")
    if cert["read_consistency"] not in {"linearizable", "stale", "unknown"} or cert["late_data_policy"] not in {"closed", "target_fence", "unknown"}:
        raise ValueError(f"{case_id}: invalid certificate consistency or late-data policy")
    if set(operation) != {"tx_id", "branch_id", "semantic_operation_id", "effect_id", "resource_id", "epoch", "operation_digest", "idempotency_key", "grant_id", "policy_epoch", "resource_version_precondition"}:
        raise ValueError(f"{case_id}: operation fields differ")
    if operation["effect_id"] not in catalog_ids or not DIGEST.fullmatch(operation["operation_digest"]):
        raise ValueError(f"{case_id}: malformed operation/effect identity")
    if type(operation["epoch"]) is not int or operation["epoch"] < 0:
        raise ValueError(f"{case_id}: invalid epoch")
    if set(fence) != {"token_id", "resource_id", "generation", "tx_id", "branch_id", "grant_id", "policy_epoch", "operation_digest", "frontier_certificate_id", "owner_revision_at_issue", "authenticated"}:
        raise ValueError(f"{case_id}: fence fields differ")
    if type(fence["generation"]) is not int or fence["generation"] < 0 or not DIGEST.fullmatch(fence["operation_digest"]):
        raise ValueError(f"{case_id}: malformed fence generation/digest")
    event_ids = set()
    event_fields = {"event_id", "decision", "decision_reason", "resource_id", "tx_id", "branch_id", "effect_id", "token_id", "token_generation", "grant_id", "policy_epoch", "operation_digest", "idempotency_key", "observed_highwater_at_commit", "observed_owner_revision_at_commit", "observed_generation_at_check", "resource_version_before", "resource_version_after", "atomic_compare_and_write", "effect_observed"}
    for event in case["target_events"]:
        if set(event) != event_fields:
            raise ValueError(f"{case_id}: target-event fields differ")
        if event["event_id"] in event_ids:
            raise ValueError(f"{case_id}: duplicate target-event id")
        event_ids.add(event["event_id"])
        if event["decision"] not in {"accepted", "rejected"} or event["effect_id"] not in catalog_ids or not DIGEST.fullmatch(event["operation_digest"]):
            raise ValueError(f"{case_id}: invalid target decision/effect/digest")
        for key in ("token_generation", "observed_highwater_at_commit", "observed_owner_revision_at_commit", "observed_generation_at_check"):
            if type(event[key]) is not int or event[key] < 0:
                raise ValueError(f"{case_id}: invalid event generation/revision")
    reconciliation = case["reconciliation"]
    if set(reconciliation) != {"complete", "independent", "observed_event_ids"}:
        raise ValueError(f"{case_id}: reconciliation fields differ")
    observed_ids = {e["event_id"] for e in case["target_events"] if e["effect_observed"]}
    if len(reconciliation["observed_event_ids"]) != len(set(reconciliation["observed_event_ids"])) or not set(reconciliation["observed_event_ids"]).issubset(event_ids):
        raise ValueError(f"{case_id}: invalid reconciliation event ids")
    if reconciliation["complete"] and reconciliation["independent"] and set(reconciliation["observed_event_ids"]) != observed_ids:
        raise ValueError(f"{case_id}: target effect observations disagree with reconciliation")
    computed = evaluate(case)
    if case["expected"] != computed:
        raise ValueError(f"{case_id}: expected {case['expected']!r}, computed {computed!r}")


def validate_suite(suite: dict[str, Any], schema: dict[str, Any]) -> dict[str, int]:
    if suite.get("profile") != "luminous-commit-frontier-fencing" or suite.get("version") != "0.1":
        raise ValueError("unexpected CFF profile/version")
    if suite.get("status") != "synthetic_resource_fence_contract_not_executed" or suite.get("no_aggregate_score") is not True:
        raise ValueError("CFF claim ceiling/no-score invariant changed")
    constraints = {"disposable_resource_owner_required": True, "external_network_access": False, "production_endpoints": False, "real_credentials_or_customer_data": False, "synthetic_canaries_only": True, "atomic_compare_and_write_required": True, "owner_enforced_monotonic_generation_required": True, "unknown_owner_highwater_blocks_mutation": True}
    if suite.get("constraints") != constraints or suite.get("canonical_effect_owner") != "CAPC-v0.1":
        raise ValueError("unsafe CFF constraints or CAPC effect owner changed")
    if schema.get("$id") != "urn:luminous:commit-frontier-fencing:0.1":
        raise ValueError("CFF schema id changed")
    catalog = suite.get("effect_catalog", [])
    catalog_ids = {item["effect_id"] for item in catalog}
    if len(catalog_ids) != len(catalog) or not catalog_ids:
        raise ValueError("invalid CAPC effect catalog")
    cases = suite.get("cases", [])
    if len(cases) != 28:
        raise ValueError("CFF requires 26 frozen cases")
    counts = {"positive_control": 0, "negative_control": 0, "indeterminate_control": 0}
    seen = set()
    for case in cases:
        if case["case_id"] in seen:
            raise ValueError("duplicate CFF case id")
        seen.add(case["case_id"])
        if case["control_class"] not in counts:
            raise ValueError("invalid CFF control class")
        counts[case["control_class"]] += 1
        validate_case(case, catalog_ids)
    if counts != CONTROL_COUNTS:
        raise ValueError(f"CFF control distribution changed: {counts}")
    return {"cases": len(cases), **counts}


def main() -> int:
    suite = load(BASE / "commit-frontier-fencing.fixtures.json")
    schema = load(BASE / "commit-frontier-fencing.schema.json")
    summary = validate_suite(suite, schema)
    print(f"CFF=STRUCTURE_CHECKED SUMMARY={json.dumps(summary, sort_keys=True)} NO_AGGREGATE_SCORE=true CLAIM=SYNTHETIC_ONLY")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
