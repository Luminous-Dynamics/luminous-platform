#!/usr/bin/env python3
"""ACIArena-to-ATR 3x3 coverage contract; not a native benchmark runner."""
from __future__ import annotations

import copy
import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path("research/agentic-threat-resilience-v0.1")
SCHEMA = ROOT / "schemas/aciarena-coverage-contract-v0.1.schema.json"
FIXTURE = ROOT / "scenarios/aciarena-coverage-contract-v0.1.json"

SURFACES = {"external_input", "agent_profile", "inter_agent_message"}
OBJECTIVES = {"instruction_hijacking", "task_disruption", "information_exfiltration"}
ORACLES = {
    "instruction_hijacking": "root_task_and_plan_immutability",
    "task_disruption": "task_state_and_required_step_oracle",
    "information_exfiltration": "data_flow_and_external_egress_receipt",
}
REQUIRED_EVIDENCE = {
    "source_provenance",
    "actor_identity",
    "tenant_scope",
    "task_plan_binding",
    "authority_lineage",
    "participant_decision",
    "independent_effect_observer",
}


def validate_contract(corpus: dict) -> None:
    pairs = [(case["surface"], case["objective"]) for case in corpus["cases"]]
    expected = {(surface, objective) for surface in SURFACES for objective in OBJECTIVES}
    if len(pairs) != len(set(pairs)):
        raise ValueError("duplicate surface/objective pair")
    if set(pairs) != expected:
        missing = sorted(expected - set(pairs))
        extra = sorted(set(pairs) - expected)
        raise ValueError(f"3x3 coverage incomplete; missing={missing}, extra={extra}")
    ids = [case["case_id"] for case in corpus["cases"]]
    if len(ids) != len(set(ids)):
        raise ValueError("duplicate case IDs")
    for case in corpus["cases"]:
        if case["objective_oracle"] != ORACLES[case["objective"]]:
            raise ValueError(f"{case['case_id']}: wrong independent objective oracle")
        evidence = {key for key, value in case["required_evidence"].items() if value}
        missing_evidence = REQUIRED_EVIDENCE - evidence
        if missing_evidence:
            raise ValueError(f"{case['case_id']}: missing evidence {sorted(missing_evidence)}")


def mutation_controls(corpus: dict) -> int:
    total = 0

    def expect_reject(label: str, mutate) -> None:
        nonlocal total
        candidate = copy.deepcopy(corpus)
        mutate(candidate)
        try:
            validate_contract(candidate)
        except ValueError:
            total += 1
            return
        raise SystemExit(f"mutation control {label} was accepted")

    expect_reject("remove one surface-objective pair", lambda c: c["cases"].pop())
    expect_reject("duplicate one pair", lambda c: c["cases"].__setitem__(8, copy.deepcopy(c["cases"][0])))
    expect_reject("drop independent observer evidence", lambda c: c["cases"][0]["required_evidence"].update(independent_effect_observer=False))
    expect_reject("swap objective oracle", lambda c: c["cases"][0].update(objective_oracle="data_flow_and_external_egress_receipt"))
    expect_reject("drop authority lineage", lambda c: c["cases"][3]["required_evidence"].update(authority_lineage=False))
    return total


def main() -> None:
    schema = json.loads(SCHEMA.read_text())
    corpus = json.loads(FIXTURE.read_text())
    Draft202012Validator.check_schema(schema)
    errors = sorted(Draft202012Validator(schema).iter_errors(corpus), key=lambda e: list(e.absolute_path))
    if errors:
        for error in errors:
            location = "/".join(str(part) for part in error.absolute_path)
            print(f"schema {location}: {error.message}")
        raise SystemExit(f"{len(errors)} schema errors")

    validate_contract(corpus)
    if len(corpus["cases"]) != 9:
        raise SystemExit("expected exactly 9 ACIArena surface/objective combinations")
    mutations = mutation_controls(corpus)
    print(f"ACIArena ATR coverage contract valid: 9/9 surface/objective pairs; {mutations} mutation controls; no benchmark execution claimed")


if __name__ == "__main__":
    main()
