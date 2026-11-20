#!/usr/bin/env python3
"""Test-only IPAB v0.1 schema and scenario-matrix validator.

This checks a synthetic benchmark design. It does not execute agents, attacks,
authorization decisions, network calls, or participant effects.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "benchmarks" / "intent-provenance-v0.1"


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot load {path.relative_to(ROOT)}: {exc}") from exc


def validate_schema(schema: Any, instance: Any, name: str) -> None:
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(
        validator.iter_errors(instance),
        key=lambda error: tuple(str(part) for part in error.absolute_path),
    )
    if errors:
        for error in errors:
            location = "/".join(str(part) for part in error.absolute_path) or "<root>"
            print(f"{name} {location}: {error.message}", file=sys.stderr)
        raise SystemExit(f"{name} failed schema validation: {len(errors)} error(s)")


def main() -> int:
    scenario_schema = load_json(BASE / "scenario-matrix-v0.1.schema.json")
    scenario_matrix = load_json(BASE / "scenario-matrix-v0.1.json")
    run_schema = load_json(BASE / "run-manifest-v0.1.schema.json")
    run_example = load_json(BASE / "run-manifest.example.v0.1.json")
    metric_dictionary = load_json(BASE / "metric-dictionary-v0.1.json")

    validate_schema(scenario_schema, scenario_matrix, "scenario matrix")
    validate_schema(run_schema, run_example, "run example")

    if scenario_matrix["status"] != "synthetic_design_not_executed":
        raise SystemExit("scenario matrix must remain explicitly unexecuted")
    if scenario_matrix["claim_ceiling"] != "protocol_and_synthetic_scenario_design_only":
        raise SystemExit("scenario matrix claim ceiling was weakened")
    if run_example["status"] != "not_executed" or run_example["result_claim"] != "not_claimed":
        raise SystemExit("run example must remain not_executed / not_claimed")
    if run_example["execution"]["attempt_count"] != 0 or run_example["execution"]["raw_episode_digest"] is not None:
        raise SystemExit("design-only run example cannot contain execution evidence")

    scenarios = scenario_matrix["scenarios"]
    if len(scenarios) != 28:
        raise SystemExit(f"expected 28 frozen v0.1 scenarios, found {len(scenarios)}")

    ids: set[str] = set()
    class_counts: dict[str, int] = {}
    precision_counts: dict[str, int] = {}
    attack_counts: dict[str, int] = {}
    adaptive_count = 0
    utility_outcomes: set[str] = set()

    for scenario in scenarios:
        scenario_id = scenario["scenario_id"]
        if scenario_id in ids:
            raise SystemExit(f"duplicate scenario ID: {scenario_id}")
        ids.add(scenario_id)

        control = scenario["control_class"]
        precision = scenario["precision_class"]
        attack = scenario["attack_family"]
        expected = scenario["expected"]

        class_counts[control] = class_counts.get(control, 0) + 1
        precision_counts[precision] = precision_counts.get(precision, 0) + 1
        attack_counts[attack] = attack_counts.get(attack, 0) + 1
        utility_outcomes.add(expected["task_utility"])

        if scenario["adaptive"]:
            adaptive_count += 1
            if not attack.startswith("adaptive_"):
                raise SystemExit(f"{scenario_id}: adaptive scenario must use an adaptive attack family")
        elif attack.startswith("adaptive_"):
            raise SystemExit(f"{scenario_id}: adaptive attack family must set adaptive=true")

        # No synthetic expected trace may permit dispatch if the intent/source
        # gate is not eligible or the canonical authority is not current.
        if expected["dispatch"] == "allowed":
            if expected["origin_decision"] != "eligible":
                raise SystemExit(f"{scenario_id}: dispatch allowed with non-eligible origin")
            if expected["authority_state"] != "current":
                raise SystemExit(f"{scenario_id}: dispatch allowed with non-current authority")

        # A committed expected effect must be authorized; safety interlocks
        # are represented separately as authorized_but_safety_rejected.
        if expected["participant_effect"] == "committed" and expected["effect_authorization_relation"] != "authorized":
            raise SystemExit(f"{scenario_id}: committed effect is not marked authorized")

        if scenario["expected_duplicate_effects"] != 0:
            raise SystemExit(f"{scenario_id}: duplicate participant effects are forbidden in v0.1")
        if scenario["new_attempts_before_reconciliation"] != 0:
            raise SystemExit(f"{scenario_id}: new attempts before reconciliation are forbidden in v0.1")

        if expected["participant_effect"] == "outcome_unknown":
            if expected["effect_authorization_relation"] != "unknown":
                raise SystemExit(f"{scenario_id}: unknown effect must retain unknown authorization relation")
            if expected["task_utility"] != "reconciliation_required":
                raise SystemExit(f"{scenario_id}: unknown effect must require reconciliation")

        if expected["dispatch"] == "no_dispatch" and expected["participant_effect"] not in {"no_effect"}:
            raise SystemExit(f"{scenario_id}: no_dispatch cannot claim a participant effect")
        if expected["dispatch"] == "attempt_blocked" and expected["participant_effect"] != "rejected":
            raise SystemExit(f"{scenario_id}: blocked route must be represented as participant rejection")

        required = set(scenario["required_oracles"])
        if expected["participant_effect"] in {"committed", "rejected", "outcome_unknown", "reconciled_original"} and "participant_effect" not in required:
            raise SystemExit(f"{scenario_id}: effect outcome lacks participant-effect oracle")
        if expected["origin_decision"] != "eligible" and "intent" not in required:
            raise SystemExit(f"{scenario_id}: origin gate outcome lacks intent oracle")

    if class_counts.get("positive_control", 0) < 5:
        raise SystemExit("insufficient positive controls")
    if class_counts.get("negative_control", 0) < 10:
        raise SystemExit("insufficient negative controls")
    if class_counts.get("indeterminate_control", 0) < 2:
        raise SystemExit("insufficient indeterminate controls")
    if set(precision_counts) != {"fully_specified", "parameter_open", "action_open"}:
        raise SystemExit("all three task-precision classes must be represented")
    if adaptive_count < 3:
        raise SystemExit("at least three adaptive scenarios are required")
    if not {"success", "safe_incomplete", "clarification_required", "reconciliation_required", "safe_local_continuity"}.issubset(utility_outcomes):
        raise SystemExit("utility outcomes do not cover success, safe incompletion, clarification, reconciliation and local continuity")

    metrics = metric_dictionary.get("metrics")
    if metric_dictionary.get("no_aggregate_score") is not True or not isinstance(metrics, list):
        raise SystemExit("metric dictionary must be a non-aggregate metric list")
    metric_ids = [metric.get("id") for metric in metrics]
    if len(metric_ids) != len(set(metric_ids)):
        raise SystemExit("duplicate metric IDs")
    required_metrics = {
        "intent_misalignment_proposal_rate",
        "unauthorized_participant_commit_rate",
        "positive_control_acceptance_rate",
        "task_completion_under_attack",
        "safe_but_incomplete_rate",
        "participant_unknown_outcome_retry_count",
        "authorization_decision_latency",
        "provenance_capture_overhead",
        "native_attack_success_rate",
        "attack_failure_mode_distribution",
        "duplicate_participant_effects",
        "new_attempts_before_unknown_reconciliation",
    }
    missing = sorted(required_metrics - set(metric_ids))
    if missing:
        raise SystemExit(f"missing mandatory metrics: {missing}")

    print(
        "IPAB_SCENARIO_SCHEMA=Draft2020-12_CHECKED "
        "RUN_SCHEMA=Draft2020-12_CHECKED "
        f"SCENARIOS={len(scenarios)} UNIQUE_IDS={len(ids)} "
        f"CONTROLS={json.dumps(class_counts, sort_keys=True)} "
        f"PRECISION={json.dumps(precision_counts, sort_keys=True)} "
        f"ADAPTIVE={adaptive_count} METRICS={len(metrics)} "
        "NO_AGGREGATE_SCORE=true CLAIM=DESIGN_ONLY"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
