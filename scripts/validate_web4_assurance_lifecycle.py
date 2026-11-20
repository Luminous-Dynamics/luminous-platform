#!/usr/bin/env python3
"""Test-only WALP v0.1 schema and semantic projection validator.

This validates synthetic lifecycle projection fixtures. It does not implement
ASSURE-004, Mycelix applicability, WAEP qualification, or deployment policy.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "docs" / "security"


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot load {path.relative_to(ROOT)}: {exc}") from exc


def main() -> int:
    schema = load_json(BASE / "web4-assurance-lifecycle.schema.v0.1.json")
    suite = load_json(BASE / "web4-assurance-lifecycle.fixtures.v0.1.json")
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(
        validator.iter_errors(suite),
        key=lambda error: tuple(str(part) for part in error.absolute_path),
    )
    if errors:
        for error in errors:
            where = "/".join(str(part) for part in error.absolute_path) or "<root>"
            print(f"{where}: {error.message}", file=sys.stderr)
        raise SystemExit(f"WALP schema validation failed with {len(errors)} error(s)")

    if suite["status"] != "synthetic_fixture_suite_not_executed":
        raise SystemExit("WALP fixtures must remain explicitly unexecuted")
    if suite["claim_ceiling"] != "synthetic_projection_conformance_only":
        raise SystemExit("WALP claim ceiling was weakened")
    if suite["no_aggregate_score"] is not True:
        raise SystemExit("WALP must not introduce an aggregate assurance score")

    records = suite["records"]
    if len(records) != 24:
        raise SystemExit(f"expected 24 WALP fixtures, found {len(records)}")

    seen: set[str] = set()
    lifecycle_counts: dict[str, int] = {}
    applicability_counts: dict[str, int] = {}
    for record in records:
        rid = record["record_id"]
        if rid in seen:
            raise SystemExit(f"duplicate WALP record: {rid}")
        seen.add(rid)

        lifecycle = record["lifecycle"]
        applicability = record["applicability"]
        expected = record["expected"]
        consumer = record["consumer_requirement"]
        state = lifecycle["state"]

        lifecycle_counts[state] = lifecycle_counts.get(state, 0) + 1
        applicability_counts[applicability["status"]] = applicability_counts.get(applicability["status"], 0) + 1

        if state != expected["lifecycle_disposition"]:
            raise SystemExit(f"{rid}: expected lifecycle {expected['lifecycle_disposition']}, got {state}")

        if lifecycle["transition_kind"] in {"invalidated", "withdrawn", "expired", "superseded", "requalified"}:
            if lifecycle["transition_evidence_ref"] is None:
                raise SystemExit(f"{rid}: {lifecycle['transition_kind']} transition lacks event evidence")

        if state == "superseded" and lifecycle["successor_result_ref"] is None:
            raise SystemExit(f"{rid}: superseded result has no successor")
        if state == "forked":
            if lifecycle["lineage_state"] != "forked" or len(lifecycle["competing_successor_refs"]) < 2:
                raise SystemExit(f"{rid}: forked state lacks competing successors")
        if state == "indeterminate" and lifecycle["lineage_state"] == "complete" and lifecycle["currentness_class"] != "TerminalInSuppliedView":
            if expected["reason_code"] != "checkpoint_currentness_not_established":
                raise SystemExit(f"{rid}: indeterminate lifecycle has no explicit currentness/lineage boundary")

        for dependency in record["dependencies"]:
            baseline = dependency["baseline_digest"]
            observed = dependency["observed_digest"]
            change = dependency["change_state"]
            evidence_ref = dependency["change_evidence_ref"]
            if change == "unchanged" and baseline != observed:
                raise SystemExit(f"{rid}/{dependency['dependency_id']}: unchanged dependency has differing digests")
            if change in {"controlled_non_material_change", "material_change"}:
                if baseline == observed:
                    raise SystemExit(f"{rid}/{dependency['dependency_id']}: changed dependency has identical digests")
                if evidence_ref is None:
                    raise SystemExit(f"{rid}/{dependency['dependency_id']}: changed dependency has no evidence reference")
            if change == "unknown" and expected["lifecycle_disposition"] == "current":
                raise SystemExit(f"{rid}/{dependency['dependency_id']}: unknown dependency cannot silently retain currentness")

        if applicability["status"] == "controlled_equivalent" and applicability["bridge_evidence_ref"] is None:
            raise SystemExit(f"{rid}: controlled equivalent lacks bridge evidence")
        if applicability["status"] == "unknown" and state == "current":
            raise SystemExit(f"{rid}: unknown applicability cannot be current")

        if lifecycle["transition_kind"] == "requalified":
            if state != "current":
                raise SystemExit(f"{rid}: requalified successor must be evaluated independently")
            if lifecycle["predecessor_result_ref"] is None or lifecycle["successor_result_ref"] != record["result_ref"]:
                raise SystemExit(f"{rid}: requalification must bind exact predecessor and this successor")
        if any(record["carry_forward"].values()):
            raise SystemExit(f"{rid}: prior conclusions were carried forward")

        applicable = applicability["status"] in consumer["acceptable_applicability"]
        currentness_ok = lifecycle["currentness_class"] in consumer["required_currentness_classes"]
        should_be_eligible = state == "current" and applicable and currentness_ok
        actual_eligible = expected["consumer_handling"] == "eligible_for_policy_evaluation"
        if should_be_eligible != actual_eligible:
            raise SystemExit(f"{rid}: consumer handling does not match lifecycle/applicability/currentness inputs")

    required_states = {"current","expired","invalidated","withdrawn","superseded","requires_requalification","indeterminate","forked"}
    if not required_states.issubset(lifecycle_counts):
        raise SystemExit(f"missing lifecycle states: {sorted(required_states - set(lifecycle_counts))}")
    required_applicability = {"exact_subject","controlled_equivalent","requires_targeted_revalidation","unknown"}
    if not required_applicability.issubset(applicability_counts):
        raise SystemExit(f"missing applicability strata: {sorted(required_applicability - set(applicability_counts))}")

    print(
        "WALP_SCHEMA=Draft2020-12_CHECKED "
        f"FIXTURES={len(records)} "
        f"LIFECYCLE={json.dumps(lifecycle_counts, sort_keys=True)} "
        f"APPLICABILITY={json.dumps(applicability_counts, sort_keys=True)} "
        "NO_AGGREGATE_SCORE=true CARRY_FORWARD=FALSE CLAIM=SYNTHETIC_ONLY"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
