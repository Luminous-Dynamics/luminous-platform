#!/usr/bin/env python3
"""Audit AgentCanary's Entry × Impact coverage contract without inventing runs."""
from __future__ import annotations

import json
from pathlib import Path
from jsonschema import Draft202012Validator

ROOT = Path("research/agentic-threat-resilience-v0.1")
SCHEMA = ROOT / "schemas/agentcanary-entry-impact-coverage-v0.1.schema.json"
MATRIX = ROOT / "scenarios/agentcanary-entry-impact-coverage-v0.1.json"


def main() -> None:
    schema = json.loads(SCHEMA.read_text())
    matrix = json.loads(MATRIX.read_text())
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(matrix),
        key=lambda error: list(error.absolute_path),
    )
    if errors:
        for error in errors:
            location = "/".join(str(part) for part in error.absolute_path)
            print(f"schema {location}: {error.message}")
        raise SystemExit(f"{len(errors)} schema error(s)")

    entries = {item["id"] for item in matrix["semantics"]["entries"]}
    impacts = {item["id"] for item in matrix["semantics"]["impacts"]}
    expected = {f"{entry}-{impact}" for entry in entries for impact in impacts}
    actual_ids = [cell["cell_id"] for cell in matrix["cells"]]
    actual = set(actual_ids)
    if len(actual_ids) != len(actual):
        raise SystemExit("duplicate Entry × Impact cell")
    if actual != expected:
        raise SystemExit(
            f"coverage matrix mismatch: missing={sorted(expected-actual)} "
            f"unexpected={sorted(actual-expected)}"
        )

    for cell in matrix["cells"]:
        state = cell["coverage_state"]
        if state == "uncovered":
            if cell["upstream_task_ids"] or cell["atr_observer_profile"] is not None:
                raise SystemExit(f"{cell['cell_id']}: uncovered cell cannot claim task/observer evidence")
        elif state == "task_pinned":
            if not cell["upstream_task_ids"]:
                raise SystemExit(f"{cell['cell_id']}: task_pinned requires upstream task IDs")
        elif state in ("executed_native", "atr_observed", "independently_verified"):
            if not cell["upstream_task_ids"]:
                raise SystemExit(f"{cell['cell_id']}: execution state requires upstream task IDs")
        if state in ("atr_observed", "independently_verified") and not cell["atr_observer_profile"]:
            raise SystemExit(f"{cell['cell_id']}: ATR state requires a named observer profile")
        if state == "independently_verified" and not cell["atr_observer_profile"]:
            raise SystemExit(f"{cell['cell_id']}: independent verification requires a separate observer")

    counts = {
        state: sum(cell["coverage_state"] == state for cell in matrix["cells"])
        for state in matrix["semantics"]["coverage_states"]
    }
    print(
        f"AgentCanary coverage contract valid: {len(actual)}/35 unique cells; "
        f"coverage={counts}; no task execution implied by schema validity"
    )


if __name__ == "__main__":
    main()
