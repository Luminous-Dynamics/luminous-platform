#!/usr/bin/env python3
"""Test-only Origin/Intent Provenance schema and semantic-vector checker.

This deterministic oracle operates only on synthetic fixtures. It is not a
live authorization engine, policy decision point, connector, broker, or agent.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "research" / "auth-core-002-v0.1"


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot load {path.relative_to(ROOT)}: {exc}") from exc


def source_disposition(source: dict[str, Any] | None) -> str:
    if source is None:
        return "indeterminate"
    if source.get("lineage_state") in {"unknown", "incomplete"}:
        return "indeterminate"
    status = source.get("attestation_status")
    if status == "unknown":
        return "indeterminate"
    if status in {"stale", "revoked", "invalid"}:
        return "denied"
    if status != "valid":
        return "indeterminate"
    if source.get("attester_role") in {"model", "none", None}:
        return "denied"
    return "eligible"


def evaluate_origin(case: dict[str, Any]) -> str:
    task = case["task"]
    candidate = case["candidate"]
    sources = {source["source_ref"]: source for source in case.get("sources", [])}

    if not candidate.get("manifest_binding_matches", False):
        return "denied"

    # A transform must preserve the union of source lineage. Missing lineage
    # is not repaired by a digest, signature, summary, or child-agent receipt.
    for transform in candidate.get("transformations", []):
        inputs = set(transform.get("input_source_refs", []))
        propagated = set(transform.get("propagated_source_refs", []))
        if not inputs.issubset(propagated):
            return "denied"
        if not transform.get("lineage_complete", False):
            return "indeterminate"

    precision = task["precision"]
    if precision in {"fully_specified", "parameter_open"}:
        if candidate.get("selection_source_ref") != "principal":
            return "denied"
        if candidate.get("action_id") != task.get("fixed_action"):
            return "denied"
        if candidate.get("target_ref") != task.get("fixed_target"):
            return "denied"
    elif precision == "action_open":
        allowed_actions = task.get("preauthorized_actions", [])
        delegation = task.get("action_selection_delegation")
        if not allowed_actions or not delegation or not delegation.get("allowed_source_refs"):
            return "clarification_required"
        if not candidate.get("action_id"):
            return "clarification_required"
        exact_member = any(
            item.get("action_id") == candidate.get("action_id")
            and item.get("target_ref") == candidate.get("target_ref")
            and item.get("operation_ref") == candidate.get("operation_ref")
            for item in allowed_actions
        )
        if not exact_member:
            return "denied"
        source_ref = candidate.get("selection_source_ref")
        if source_ref not in delegation.get("allowed_source_refs", []):
            return "denied"
        disposition = source_disposition(sources.get(source_ref))
        if disposition != "eligible":
            return disposition
    else:
        return "indeterminate"

    delegations = {
        grant["parameter_name"]: grant
        for grant in task.get("parameter_delegations", [])
    }
    for parameter in candidate.get("parameters", []):
        name = parameter["parameter_name"]
        refs = parameter.get("origin_source_refs", [])
        if not refs:
            return "indeterminate"
        if parameter.get("lineage_state") in {"unknown", "incomplete", "mixed"}:
            return "indeterminate"
        if parameter.get("lineage_state") != "complete":
            return "indeterminate"
        status = parameter.get("receipt_status")
        if status == "unknown":
            return "indeterminate"
        if status in {"stale", "revoked", "invalid"}:
            return "denied"
        if status != "valid":
            return "indeterminate"
        if parameter.get("attester_role") in {"model", "none", None}:
            return "denied"
        if not parameter.get("constraints_satisfied", False):
            return "denied"

        if refs == ["principal"]:
            continue
        grant = delegations.get(name)
        if grant is None:
            return "denied"
        allowed = set(grant.get("allowed_source_refs", []))
        for source_ref in refs:
            if source_ref == "principal":
                continue
            if source_ref not in allowed:
                return "denied"
            disposition = source_disposition(sources.get(source_ref))
            if disposition != "eligible":
                return disposition

    return "eligible"


def evaluate_authority(case: dict[str, Any]) -> str:
    state = case["candidate"].get("authority_state")
    if state == "current":
        return "current"
    if state == "revoked":
        return "revoked"
    return "indeterminate"


def evaluate_effect_gate(origin: str, authority: str) -> str:
    # This is deliberately not a final authorization decision. Eligible
    # means only "may proceed to the canonical participant checks".
    if origin != "eligible" or authority != "current":
        return "no_dispatch"
    return "participant_check_required"


def main() -> int:
    schema = load_json(BASE / "origin-intent-manifest-v0.1.schema.json")
    example = load_json(BASE / "origin-intent-manifest.example.v0.1.json")
    suite = load_json(BASE / "origin-intent-conformance.v0.1.json")

    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise SystemExit("schema must declare Draft 2020-12")
    if schema.get("$id") != "urn:luminous:auth-core-002:origin-intent:0.1":
        raise SystemExit("unexpected schema identifier")

    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(
        validator.iter_errors(example),
        key=lambda error: tuple(str(part) for part in error.absolute_path),
    )
    if errors:
        for error in errors:
            location = "/".join(str(part) for part in error.absolute_path) or "<root>"
            print(f"example {location}: {error.message}", file=sys.stderr)
        raise SystemExit(f"example failed schema validation: {len(errors)} error(s)")

    if example["record_disposition"] != "not_claimed" or example["manifest_digest"] is not None:
        raise SystemExit("design-only example must remain not_claimed and uncommitted")
    if example["claim_ceiling"] != "non_authoritative_provenance_only":
        raise SystemExit("manifest must remain non-authoritative")

    if suite.get("profile") != "luminous-origin-intent-conformance":
        raise SystemExit("invalid conformance profile")
    if suite.get("status") != "synthetic_not_executed":
        raise SystemExit("synthetic vectors must not be represented as executed runtime evidence")

    cases = suite.get("cases")
    if not isinstance(cases, list) or len(cases) != 20:
        raise SystemExit("expected exactly 20 frozen synthetic cases")

    counts: dict[str, int] = {}
    seen: set[str] = set()
    failures = 0
    for case in cases:
        vector_id = case.get("vector_id", "<missing>")
        if vector_id in seen:
            print(f"{vector_id}: duplicate vector ID", file=sys.stderr)
            failures += 1
            continue
        seen.add(vector_id)
        actual = {
            "origin_decision": evaluate_origin(case),
            "authority_disposition": evaluate_authority(case),
        }
        actual["effect_gate"] = evaluate_effect_gate(
            actual["origin_decision"], actual["authority_disposition"]
        )
        expected = case["expected"]
        if actual != expected:
            print(f"{vector_id}: expected {expected}, got {actual}", file=sys.stderr)
            failures += 1
        else:
            state = actual["origin_decision"]
            counts[state] = counts.get(state, 0) + 1

    required = {"eligible", "denied", "indeterminate", "clarification_required"}
    if not required.issubset(counts):
        raise SystemExit(f"missing outcome strata: {sorted(required - set(counts))}")
    if not any(case["expected"]["effect_gate"] == "participant_check_required" for case in cases):
        raise SystemExit("positive controls are required")
    if not any(case["expected"]["effect_gate"] == "no_dispatch" for case in cases):
        raise SystemExit("negative controls are required")

    if failures:
        raise SystemExit(f"origin-intent conformance failed: {failures}/{len(cases)} mismatches")

    print(
        "ORIGIN_INTENT_SCHEMA=Draft2020-12_CHECKED "
        f"EXAMPLE=VALID VECTORS={len(cases)} MATCHED "
        f"OUTCOMES={json.dumps(counts, sort_keys=True)} "
        "AUTHORITY=SEPARATE EFFECT=PARTICIPANT_CHECK_REQUIRED CLAIM=NON_AUTHORITY"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
