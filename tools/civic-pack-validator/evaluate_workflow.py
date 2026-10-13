#!/usr/bin/env python3
"""Exact-scope evaluator for jurisdiction packs.

This module selects candidate workflow guidance. It does not determine legal
applicability from free text, make authority decisions, or record user progress.
"""
from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from validate_pack import (
    VALIDATOR_VERSION,
    load_json,
    parse_datetime,
    sha256_hex,
    validate_pack,
)

ENGINE_VERSION = "0.1.0"
AUTHORITATIVE_OUTCOMES = {"approved", "refused", "legally_compliant", "satisfied"}


def _iso_utc(value: datetime) -> str:
    return value.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")


def evaluate_workflow(
    schema: dict[str, Any],
    pack: dict[str, Any],
    *,
    schema_bytes: bytes,
    pack_bytes: bytes,
    as_of: datetime,
    jurisdiction_id: str,
    workflow_id: str,
    business_type: str,
) -> dict[str, Any]:
    """Evaluate exact-scope candidates while refusing to infer legal outcomes."""
    if as_of.tzinfo is None or as_of.utcoffset() is None:
        raise ValueError("as_of must be timezone-aware")
    as_of = as_of.astimezone(timezone.utc)
    evaluated_at = _iso_utc(as_of)
    validation = validate_pack(
        schema, pack, schema_bytes=schema_bytes, pack_bytes=pack_bytes, as_of=as_of
    )

    report: dict[str, Any] = {
        "engine_version": ENGINE_VERSION,
        "validator_version": VALIDATOR_VERSION,
        "evaluated_at": evaluated_at,
        "input_sha256": sha256_hex(pack_bytes),
        "schema_sha256": sha256_hex(schema_bytes),
        "selection": {
            "jurisdiction_id": jurisdiction_id,
            "workflow_id": workflow_id,
            "business_type": business_type,
        },
        "pack_id": pack.get("pack_id") if isinstance(pack, dict) else None,
        "pack_version": pack.get("pack_version") if isinstance(pack, dict) else None,
        "validation_passed": validation["validation_passed"],
        "publication_eligible": validation["publication_eligible"],
        "execution_state": "not_executable",
        "scope_status": "unresolved",
        "notice": (
            "This evaluator selects source-linked guidance only. It does not "
            "establish legal applicability, certify compliance, or infer an "
            "official submission or decision."
        ),
        "requirements": [],
        "reasons": [],
    }

    if not validation["validation_passed"]:
        report["execution_state"] = "invalid_pack"
        report["reasons"].append("Pack failed structural or semantic validation.")
        report["validation_errors"] = [
            c for c in validation["checks"] if c["status"] == "FAIL"
        ]
        return report

    if not isinstance(pack, dict):
        report["execution_state"] = "invalid_pack"
        report["reasons"].append("Pack root is not an object.")
        return report

    jurisdiction = pack.get("jurisdiction", {})
    workflow = pack.get("workflow", {})
    if jurisdiction.get("jurisdiction_id") != jurisdiction_id:
        report["execution_state"] = "scope_confirmation_required"
        report["scope_status"] = "jurisdiction_mismatch"
        report["reasons"].append(
            "Selected jurisdiction does not exactly match this pack; no rules were selected."
        )
        return report
    if workflow.get("workflow_id") != workflow_id:
        report["execution_state"] = "scope_confirmation_required"
        report["scope_status"] = "workflow_mismatch"
        report["reasons"].append(
            "Selected workflow does not exactly match this pack; no rules were selected."
        )
        return report
    if business_type not in workflow.get("business_types", []):
        report["execution_state"] = "scope_confirmation_required"
        report["scope_status"] = "business_type_out_of_scope"
        report["reasons"].append(
            "Business type is not listed in this workflow's explicit scope; no rules were selected."
        )
        return report

    report["scope_status"] = "exact_pack_scope_match"
    if pack.get("publication_state") != "published":
        report["execution_state"] = "pack_not_published"
        report["reasons"].append(
            "Only published packs may provide current workflow guidance."
        )
        return report
    if not validation["publication_eligible"]:
        report["execution_state"] = "publication_blocked"
        report["reasons"].extend(validation.get("publication_blockers", []))
        return report

    source_by_id = {s["source_id"]: s for s in pack.get("sources", [])}
    candidates: list[dict[str, Any]] = []
    for requirement in pack.get("requirements", []):
        applicability = requirement.get("applicability", {})
        if business_type not in applicability.get("business_types", []):
            continue

        refs = requirement.get("source_refs", [])
        source_states = [
            {
                "source_id": source_id,
                "review_state": source_by_id[source_id]["review_state"],
                "source_url": source_by_id[source_id]["canonical_url"],
                "checked_at": source_by_id[source_id].get("checked_at"),
                "next_review_at": source_by_id[source_id].get("next_review_at"),
            }
            for source_id in refs
        ]

        # The pack schema stores conditions and unknowns as explanatory text.
        # The engine deliberately does not parse these strings as policy logic.
        conditions = applicability.get("conditions", [])
        unknowns = applicability.get("unknowns", [])
        if requirement.get("review_state") != "verified":
            state = "needs_human_review"
        elif any(s["review_state"] != "current" for s in source_states):
            state = "needs_source_verification"
        elif conditions or unknowns:
            state = "needs_scope_confirmation"
        elif requirement.get("completion_effect") in {
            "must_confirm_with_authority", "external_decision_required"
        }:
            state = "authority_confirmation_required"
        else:
            state = "guidance_available"

        candidate = {
            "requirement_id": requirement["requirement_id"],
            "title": requirement["title"],
            "claim_type": requirement["claim_type"],
            "display_state": state,
            "summary": requirement["summary"],
            "source_refs": refs,
            "sources": source_states,
            "user_actions": requirement.get("user_actions", []),
            "scope_conditions_to_review": conditions,
            "known_unknowns": unknowns,
            "completion_effect": requirement["completion_effect"],
            "user_checkbox_effect": "records_user_progress_only",
            "authority_outcome": "unknown",
        }
        # Fail closed if a future edit ever introduces an authority-like state.
        if candidate["display_state"] in AUTHORITATIVE_OUTCOMES:
            raise AssertionError("Evaluator emitted a prohibited authoritative outcome.")
        candidates.append(candidate)

    report["requirements"] = candidates
    report["execution_state"] = "guidance_ready" if candidates else "no_matching_requirements"
    report["reasons"].append(
        "Matched records are guidance candidates only; conditions and unknowns remain explicit."
    )
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--schema", required=True, type=Path)
    parser.add_argument("--pack", required=True, type=Path)
    parser.add_argument("--as-of", required=True, help="Frozen timezone-aware ISO-8601 timestamp.")
    parser.add_argument("--jurisdiction", required=True)
    parser.add_argument("--workflow", required=True)
    parser.add_argument("--business-type", required=True)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args(argv)

    as_of = parse_datetime(args.as_of)
    if as_of is None:
        parser.error("--as-of must be a timezone-aware ISO-8601 timestamp")
    try:
        schema, schema_bytes = load_json(args.schema)
        pack, pack_bytes = load_json(args.pack)
        report = evaluate_workflow(
            schema,
            pack,
            schema_bytes=schema_bytes,
            pack_bytes=pack_bytes,
            as_of=as_of,
            jurisdiction_id=args.jurisdiction,
            workflow_id=args.workflow,
            business_type=args.business_type,
        )
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
        print(json.dumps({"execution_state": "invalid_input", "error": str(exc)}, indent=2), file=sys.stderr)
        return 1

    rendered = json.dumps(report, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if report["execution_state"] != "invalid_pack" else 1


if __name__ == "__main__":
    raise SystemExit(main())
