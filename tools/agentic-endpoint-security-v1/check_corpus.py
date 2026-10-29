#!/usr/bin/env python3
"""Test-only contract checker for the agentic endpoint benchmark corpus.

This script validates the frozen scenario manifest. It is not part of an
endpoint runtime and does not execute attacks, contact networks, or assert
that any endpoint control actually prevented an effect.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

EXPECTED_OUTCOMES = {
    "AES-001": "ALLOW_AND_OBSERVE",
    "AES-002": "BLOCK_BEFORE_EFFECT",
    "AES-003": "BLOCK_BEFORE_EFFECT",
    "AES-004": "BLOCK_BEFORE_EFFECT",
    "AES-005": "BLOCK_BEFORE_EFFECT",
    "AES-006": "BLOCK_BEFORE_EFFECT",
    "AES-007": "BLOCK_BEFORE_EFFECT",
    "AES-008": "BLOCK_BEFORE_EFFECT",
    "AES-009": "BLOCK_BEFORE_EFFECT",
    "AES-010": "BLOCK_BEFORE_EFFECT",
    "AES-011": "BLOCKED_INCOMPLETE_EVIDENCE",
    "AES-012": "BLOCKED_INCOMPLETE_EVIDENCE",
    "AES-013": "BLOCKED_INCOMPLETE_EVIDENCE",
    "AES-014": "ALERT_AND_DEGRADE",
    "AES-015": "UNQUALIFIED",
    "AES-016": "BLOCK_BEFORE_EFFECT",
    "AES-017": "BLOCK_BEFORE_EFFECT",
    "AES-018": "BLOCK_BEFORE_EFFECT",
    "AES-019": "BLOCKED_INCOMPLETE_EVIDENCE",
    "AES-020": "ALLOW_AND_OBSERVE",
}

REQUIRED_SCENARIO_KEYS = {
    "id",
    "family",
    "name",
    "intent",
    "expected_outcome",
    "required_assertions",
    "claim_ceiling",
}

REQUIRED_CONSTRAINTS = {
    "isolated_disposable_lab_required": True,
    "explicit_owner_authorization_required": True,
    "external_network_access": False,
    "real_credentials_or_customer_data": False,
    "host_mutation_outside_fixture": False,
    "destructive_payloads": False,
    "default_high_risk_disposition": "BLOCK",
}

REQUIRED_SCORING_RULES = {
    "never_aggregate_away_critical_forbidden_effects": True,
    "zero_false_accepts_required_for_frozen_negative_cases": True,
    "report_counts_and_denominators": True,
    "report_confidence_intervals_for_rate_metrics": True,
    "report_results_by_execution_tier": True,
    "queued_or_skipped_ci_is_not_execution_evidence": True,
    "a_valid_signature_or_hash_does_not_establish_source_truth": True,
}


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def validate(document: Any) -> tuple[int, dict[str, int]]:
    require(isinstance(document, dict), "top-level JSON value must be an object")
    require(
        document.get("schema") == "luminous.agentic-endpoint-security-corpus.v1",
        "unexpected or missing corpus schema identifier",
    )
    require(document.get("version") == 1, "v1 checker requires corpus version 1")
    require(
        isinstance(document.get("status"), str)
        and "not yet independently reviewed" in document["status"],
        "corpus must not imply independent review or execution",
    )

    outcomes = document.get("outcome_vocabulary")
    require(isinstance(outcomes, list) and outcomes, "outcome_vocabulary must be a nonempty list")
    require(
        all(isinstance(item, str) for item in outcomes) and len(set(outcomes)) == len(outcomes),
        "outcome vocabulary must contain unique strings",
    )
    require(
        {"ALLOW_AND_OBSERVE", "BLOCK_BEFORE_EFFECT", "BLOCKED_INCOMPLETE_EVIDENCE",
         "ALERT_AND_DEGRADE", "RECOVERED_AND_VERIFIED", "UNQUALIFIED"}.issubset(set(outcomes)),
        "required disposition is absent from outcome vocabulary",
    )

    constraints = document.get("execution_constraints")
    require(isinstance(constraints, dict), "execution_constraints must be an object")
    for key, value in REQUIRED_CONSTRAINTS.items():
        require(constraints.get(key) is value or constraints.get(key) == value,
                f"unsafe or missing execution constraint: {key} must equal {value!r}")

    scoring = document.get("scoring_rules")
    require(isinstance(scoring, dict), "scoring_rules must be an object")
    for key, value in REQUIRED_SCORING_RULES.items():
        require(scoring.get(key) is value or scoring.get(key) == value,
                f"missing or weakened scoring rule: {key}")

    scenarios = document.get("scenarios")
    require(isinstance(scenarios, list), "scenarios must be a list")
    require(len(scenarios) == len(EXPECTED_OUTCOMES), "scenario count changed; review protocol versioning")

    seen: set[str] = set()
    counts: dict[str, int] = {}
    for index, scenario in enumerate(scenarios):
        require(isinstance(scenario, dict), f"scenario {index} must be an object")
        missing = REQUIRED_SCENARIO_KEYS - set(scenario)
        require(not missing, f"scenario {index} missing keys: {sorted(missing)}")

        scenario_id = scenario["id"]
        require(isinstance(scenario_id, str) and scenario_id, f"scenario {index} has invalid id")
        require(scenario_id not in seen, f"duplicate scenario id: {scenario_id}")
        seen.add(scenario_id)

        for key in ("family", "name", "intent", "claim_ceiling"):
            require(isinstance(scenario[key], str) and scenario[key].strip(),
                    f"{scenario_id}: {key} must be a nonempty string")

        assertions = scenario["required_assertions"]
        require(
            isinstance(assertions, list)
            and assertions
            and all(isinstance(item, str) and item.strip() for item in assertions),
            f"{scenario_id}: required_assertions must be a nonempty list of strings",
        )

        outcome = scenario["expected_outcome"]
        require(outcome in outcomes, f"{scenario_id}: undeclared outcome {outcome!r}")
        require(scenario_id in EXPECTED_OUTCOMES, f"unexpected scenario id {scenario_id}; version the oracle")
        require(
            outcome == EXPECTED_OUTCOMES[scenario_id],
            f"{scenario_id}: expected_outcome drifted from frozen oracle "
            f"{EXPECTED_OUTCOMES[scenario_id]!r} to {outcome!r}",
        )
        counts[outcome] = counts.get(outcome, 0) + 1

    require(seen == set(EXPECTED_OUTCOMES), "scenario IDs differ from frozen v1 inventory")
    return len(scenarios), counts


def main() -> int:
    default_path = (
        Path(__file__).resolve().parents[2]
        / "benchmarks"
        / "agentic-endpoint-security-v1"
        / "corpus.json"
    )
    corpus_path = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else default_path
    try:
        document = json.loads(corpus_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        fail(f"cannot read valid UTF-8 JSON from {corpus_path}: {exc}")

    count, outcomes = validate(document)
    summary = ", ".join(f"{name}={outcomes[name]}" for name in sorted(outcomes))
    print(f"PASS corpus contract: {count} scenarios; frozen IDs/outcomes and lab constraints valid.")
    print(f"Outcome counts: {summary}")
    print("SCOPE: manifest integrity only; no runtime enforcement or prevention claim.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
