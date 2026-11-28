#!/usr/bin/env python3
"""Audit AgentCanary's Entry × Impact coverage contract without inventing runs."""
from __future__ import annotations

import json
import re
from pathlib import Path
from jsonschema import Draft202012Validator

ROOT = Path("research/agentic-threat-resilience-v0.1")
SCHEMA = ROOT / "schemas/agentcanary-entry-impact-coverage-v0.1.schema.json"
MATRIX = ROOT / "scenarios/agentcanary-entry-impact-coverage-v0.1.json"


def cell_contract_errors(cell: dict) -> list[str]:
    """Return coverage-state errors; never infer evidence from a status label."""
    state = cell["coverage_state"]
    task_ids = cell["upstream_task_ids"]
    refs = {
        "task_manifest_digest": cell["task_manifest_digest"],
        "run_manifest_id": cell["run_manifest_id"],
        "native_result_artifact": cell["native_result_artifact"],
        "native_result_digest": cell["native_result_digest"],
        "run_manifest_digest": cell["run_manifest_digest"],
        "atr_sidecar_artifact": cell["atr_sidecar_artifact"],
        "atr_sidecar_digest": cell["atr_sidecar_digest"],
        "independent_receipt_artifact": cell["independent_receipt_artifact"],
        "independent_receipt_digest": cell["independent_receipt_digest"],
    }
    errors: list[str] = []

    def require(*names: str) -> None:
        for name in names:
            if not refs[name] or not refs[name].strip():
                errors.append(f"missing_{name}")

    def forbid(*names: str) -> None:
        for name in names:
            if refs[name] is not None:
                errors.append(f"unexpected_{name}")

    if state == "uncovered":
        if task_ids:
            errors.append("uncovered_has_task_ids")
        forbid(*refs.keys())
        if cell["atr_observer_profile"] is not None:
            errors.append("uncovered_has_observer_profile")
    elif state == "task_pinned":
        if not task_ids:
            errors.append("task_pinned_missing_task_ids")
        require("task_manifest_digest")
        forbid("run_manifest_id", "native_result_artifact",
               "atr_sidecar_artifact", "independent_receipt_artifact")
        if cell["atr_observer_profile"] is not None:
            errors.append("task_pinned_has_observer_profile")
    elif state == "executed_native":
        if not task_ids:
            errors.append("executed_native_missing_task_ids")
        require("task_manifest_digest", "run_manifest_id", "run_manifest_digest",
                "native_result_artifact", "native_result_digest")
        forbid("atr_sidecar_artifact", "atr_sidecar_digest",
               "independent_receipt_artifact", "independent_receipt_digest")
        if cell["atr_observer_profile"] is not None:
            errors.append("executed_native_has_observer_profile")
    elif state == "atr_observed":
        if not task_ids:
            errors.append("atr_observed_missing_task_ids")
        require("task_manifest_digest", "run_manifest_id", "run_manifest_digest",
                "native_result_artifact", "native_result_digest",
                "atr_sidecar_artifact", "atr_sidecar_digest")
        forbid("independent_receipt_artifact", "independent_receipt_digest")
        if not cell["atr_observer_profile"] or not cell["atr_observer_profile"].strip():
            errors.append("atr_observed_missing_observer_profile")
    elif state == "independently_verified":
        if not task_ids:
            errors.append("independently_verified_missing_task_ids")
        require("task_manifest_digest", "run_manifest_id", "run_manifest_digest",
                "native_result_artifact", "native_result_digest",
                "atr_sidecar_artifact", "atr_sidecar_digest",
                "independent_receipt_artifact", "independent_receipt_digest")
        if not cell["atr_observer_profile"] or not cell["atr_observer_profile"].strip():
            errors.append("independently_verified_missing_observer_profile")

    digest_fields = (
        "task_manifest_digest",
        "run_manifest_digest",
        "native_result_digest",
        "atr_sidecar_digest",
        "independent_receipt_digest",
    )
    required_digests = {
        "task_pinned": ("task_manifest_digest",),
        "executed_native": ("task_manifest_digest", "run_manifest_digest", "native_result_digest"),
        "atr_observed": ("task_manifest_digest", "run_manifest_digest", "native_result_digest", "atr_sidecar_digest"),
        "independently_verified": digest_fields,
    }.get(state, ())
    for name in required_digests:
        value = cell[name]
        if not isinstance(value, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", value):
            errors.append(f"invalid_{name}")
    return errors


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

    entry_ids = [item["id"] for item in matrix["semantics"]["entries"]]
    impact_ids = [item["id"] for item in matrix["semantics"]["impacts"]]
    if len(entry_ids) != len(set(entry_ids)) or set(entry_ids) != {f"E{i}" for i in range(1, 6)}:
        raise SystemExit("Entry dimension IDs must be exactly E1..E5, without duplicates")
    if len(impact_ids) != len(set(impact_ids)) or set(impact_ids) != {f"I{i}" for i in range(1, 8)}:
        raise SystemExit("Impact dimension IDs must be exactly I1..I7, without duplicates")

    states = matrix["semantics"]["coverage_states"]
    required_states = ["uncovered", "task_pinned", "executed_native", "atr_observed", "independently_verified"]
    if states != required_states:
        raise SystemExit(f"coverage states must be ordered exactly as {required_states}")

    expected = {f"{entry}-{impact}" for entry in entry_ids for impact in impact_ids}
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
        expected_id = f"{cell['entry_id']}-{cell['impact_id']}"
        if cell["cell_id"] != expected_id:
            raise SystemExit(f"{cell['cell_id']}: entry_id/impact_id do not match cell_id")
        violations = cell_contract_errors(cell)
        if violations:
            raise SystemExit(f"{cell['cell_id']}: {', '.join(violations)}")

    # Mutation controls prove that status labels alone cannot advance coverage.
    positive = matrix["cells"][0].copy()
    positive.update({
        "coverage_state":"independently_verified",
        "upstream_task_ids":["task_9001"],
        "task_manifest_digest":"sha256:" + "a" * 64,
        "run_manifest_id":"run-0001",
        "run_manifest_digest":"sha256:" + "b" * 64,
        "native_result_artifact":"artifacts/native-result.json",
        "native_result_digest":"sha256:" + "c" * 64,
        "atr_observer_profile":"observer-v1",
        "atr_sidecar_artifact":"artifacts/atr-sidecar.json",
        "atr_sidecar_digest":"sha256:" + "d" * 64,
        "independent_receipt_artifact":"artifacts/independent-receipt.json",
        "independent_receipt_digest":"sha256:" + "e" * 64,
    })
    if cell_contract_errors(positive):
        raise SystemExit("fully evidenced independently_verified control should pass")

    mutations = []
    invalid = positive.copy()
    invalid["independent_receipt_artifact"] = None
    mutations.append(("missing-independent-receipt", invalid))

    invalid = positive.copy()
    invalid["atr_sidecar_artifact"] = None
    mutations.append(("missing-atr-sidecar", invalid))

    invalid = positive.copy()
    invalid["native_result_artifact"] = None
    mutations.append(("missing-native-result", invalid))

    invalid = positive.copy()
    invalid["task_manifest_digest"] = None
    mutations.append(("missing-task-manifest", invalid))

    invalid = positive.copy()
    invalid["independent_receipt_digest"] = None
    mutations.append(("missing-independent-receipt-digest", invalid))

    invalid = positive.copy()
    invalid["native_result_digest"] = "sha256:bad"
    mutations.append(("invalid-native-result-digest", invalid))

    invalid = positive.copy()
    invalid["run_manifest_digest"] = None
    mutations.append(("missing-run-manifest-digest", invalid))

    invalid = positive.copy()
    invalid["atr_observer_profile"] = None
    mutations.append(("missing-observer-profile", invalid))

    invalid = positive.copy()
    invalid["coverage_state"] = "uncovered"
    mutations.append(("status-downgrade-with-evidence", invalid))

    for label, candidate in mutations:
        if not cell_contract_errors(candidate):
            raise SystemExit(f"coverage mutation was incorrectly accepted: {label}")

    counts = {
        state: sum(cell["coverage_state"] == state for cell in matrix["cells"])
        for state in states
    }
    print(
        f"AgentCanary coverage contract valid: {len(actual)}/35 unique cells; "
        f"coverage={counts}; 9 evidence-state mutation controls rejected; "
        f"no task execution implied by schema validity"
    )


if __name__ == "__main__":
    main()
