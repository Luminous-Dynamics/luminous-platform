#!/usr/bin/env python3
"""Dependency-free checker for observer-independence closure fixtures."""
from __future__ import annotations

import itertools
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "benchmarks" / "agentic-endpoint-security-v1"
DOMAIN_FIELDS = (
    "process_runtime",
    "event_stream",
    "signing_root",
    "credential_root",
    "clock_authority",
    "storage_backend",
    "control_plane",
    "operator",
    "provider",
)


def load(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot load {path}: {exc}") from exc


def _domain_state(witness: dict[str, Any], required: list[str]) -> tuple[bool, bool]:
    values = [witness["domains"].get(name) for name in required]
    unknown = any(
        not isinstance(value, str) or not value or value.strip().casefold() == "unknown"
        for value in values
    )
    return not unknown, unknown


def _pairwise_shared(
    left: dict[str, Any], right: dict[str, Any], required: list[str]
) -> list[str]:
    return [
        domain for domain in required
        if left["domains"][domain] == right["domains"][domain]
    ]


def _maximum_independent_sets(
    eligible: list[dict[str, Any]], required: list[str]
) -> list[list[str]]:
    maximum: list[list[str]] = []
    max_size = 0
    for size in range(1, len(eligible) + 1):
        for subset in itertools.combinations(eligible, size):
            independent = all(
                not _pairwise_shared(left, right, required)
                for left, right in itertools.combinations(subset, 2)
            )
            if not independent:
                continue
            ids = sorted(item["witness_id"] for item in subset)
            if size > max_size:
                max_size = size
                maximum = [ids]
            elif size == max_size:
                maximum.append(ids)
    return sorted(maximum)


def evaluate_case(case: dict[str, Any], profile: dict[str, Any]) -> dict[str, Any]:
    required = profile["required_disjoint_domains"]
    quorum = profile["minimum_independent_witnesses"]
    witnesses = case["witnesses"]
    witness_ids = [witness["witness_id"] for witness in witnesses]
    if len(witness_ids) != len(set(witness_ids)):
        raise ValueError(f'{case["scenario_id"]}: duplicate witness identity')
    subject = case["subject_domains"]

    unknown_ids: list[str] = []
    known_ids: list[str] = []
    subject_correlated: list[str] = []
    eligible: list[dict[str, Any]] = []

    for witness in witnesses:
        known, unknown = _domain_state(witness, required)
        if unknown:
            unknown_ids.append(witness["witness_id"])
            continue
        known_ids.append(witness["witness_id"])
        if any(witness["domains"][domain] == subject[domain] for domain in required):
            subject_correlated.append(witness["witness_id"])
            continue
        eligible.append(witness)

    correlated_pairs: list[dict[str, Any]] = []
    for left, right in itertools.combinations(eligible, 2):
        shared = _pairwise_shared(left, right, required)
        if shared:
            correlated_pairs.append({
                "left": left["witness_id"],
                "right": right["witness_id"],
                "shared_domains": shared,
            })
    correlated_pairs.sort(key=lambda pair: (pair["left"], pair["right"]))

    maximum_sets = _maximum_independent_sets(eligible, required)
    maximum_size = len(maximum_sets[0]) if maximum_sets else 0

    agreement = "none"
    disposition = "unqualified"
    if maximum_size >= quorum:
        set_outcomes: list[str] = []
        by_id = {witness["witness_id"]: witness for witness in eligible}
        for witness_ids in maximum_sets:
            observations = {by_id[witness_id]["observation"] for witness_id in witness_ids}
            set_outcomes.append(
                next(iter(observations)) if len(observations) == 1 else "conflict"
            )

        if set_outcomes and all(value == "effect" for value in set_outcomes):
            agreement = "effect"
            disposition = "qualified_effect"
        elif set_outcomes and all(value == "no_effect" for value in set_outcomes):
            agreement = "no_effect"
            disposition = "qualified_no_effect"
        else:
            agreement = (
                "conflict"
                if "conflict" in set_outcomes or len(set(set_outcomes)) > 1
                else "unknown"
            )
            disposition = "indeterminate"
    elif unknown_ids:
        agreement = "unknown"
        disposition = "indeterminate"

    return {
        "witness_count": len(witnesses),
        "known_domain_witnesses": len(known_ids),
        "unknown_domain_witnesses": len(unknown_ids),
        "subject_correlated_witnesses": sorted(subject_correlated),
        "correlated_pair_count": len(correlated_pairs),
        "correlated_pairs": correlated_pairs,
        "maximum_independent_witnesses": maximum_size,
        "maximum_independent_sets": maximum_sets,
        "quorum": quorum,
        "observation_agreement": agreement,
        "final_disposition": disposition,
    }


def validate_structure(suite: dict[str, Any], schema: dict[str, Any]) -> None:
    if suite.get("profile") != "luminous-observer-independence-closure":
        raise ValueError("unexpected observer-independence profile")
    if suite.get("version") != "0.1" or suite.get("status") != "synthetic_fixture_suite_not_executed":
        raise ValueError("observer-independence version/status was changed")
    if suite.get("no_aggregate_score") is not True:
        raise ValueError("observer-independence suite must not introduce aggregate score")
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise ValueError("observer-independence schema must declare Draft 2020-12")
    if schema.get("$id") != "urn:luminous:observer-independence-closure:0.1":
        raise ValueError("unexpected observer-independence schema identity")
    if schema.get("$defs", {}).get("case", {}).get("additionalProperties") is not False:
        raise ValueError("observer-independence case schema must reject unknown fields")

    profiles = suite.get("claim_profiles")
    if set(profiles or {}) != {"critical_effect_truth_v0"}:
        raise ValueError("v0.1 must freeze its one claim profile")
    profile = profiles["critical_effect_truth_v0"]
    if profile.get("profile_id") != "critical_effect_truth_v0":
        raise ValueError("claim profile identity mismatch")
    if profile.get("required_disjoint_domains") != list(DOMAIN_FIELDS[:7]):
        raise ValueError("critical effect-truth separation domains cannot be weakened")
    if profile.get("minimum_independent_witnesses") != 2:
        raise ValueError("critical effect-truth minimum independent quorum cannot be weakened")

    cases = suite.get("cases")
    if not isinstance(cases, list) or len(cases) != 16:
        raise ValueError(
            f"expected 16 observer-independence scenarios, found "
            f"{len(cases) if isinstance(cases, list) else 'non-array'}"
        )

    for case in cases:
        if set(case) != {
            "scenario_id", "control_class", "claim_profile_id", "subject_domains",
            "witnesses", "expected",
        }:
            raise ValueError(f'{case.get("scenario_id", "<unknown>")}: case fields differ from contract')
        if case["claim_profile_id"] not in profiles:
            raise ValueError(f'{case["scenario_id"]}: unknown claim profile')
        if set(case["subject_domains"]) != set(DOMAIN_FIELDS):
            raise ValueError(f'{case["scenario_id"]}: subject domain fields differ from contract')
        for dimension in DOMAIN_FIELDS:
            if not isinstance(case["subject_domains"][dimension], str) or not case["subject_domains"][dimension]:
                raise ValueError(f'{case["scenario_id"]}: subject domain {dimension} must be known')
        witness_ids = [witness["witness_id"] for witness in case["witnesses"]]
        if len(witness_ids) != len(set(witness_ids)):
            raise ValueError(f'{case["scenario_id"]}: duplicate witness identity')
        evidence_refs = [witness["evidence_ref"] for witness in case["witnesses"] if witness["evidence_ref"]]
        if len(evidence_refs) != len(set(evidence_refs)):
            raise ValueError(f'{case["scenario_id"]}: reused evidence reference')
        attestation_refs = [witness["attestation_ref"] for witness in case["witnesses"] if witness["attestation_ref"]]
        if len(attestation_refs) != len(set(attestation_refs)):
            raise ValueError(f'{case["scenario_id"]}: reused attestation reference')
        for witness in case["witnesses"]:
            if set(witness["domains"]) != set(DOMAIN_FIELDS):
                raise ValueError(f'{case["scenario_id"]}: witness domain fields differ from contract')
            if witness["observation"] not in {"effect", "no_effect", "unknown"}:
                raise ValueError(f'{case["scenario_id"]}: invalid witness observation')


def validate_suite(suite: dict[str, Any], schema: dict[str, Any]) -> dict[str, int]:
    validate_structure(suite, schema)
    profile = suite["claim_profiles"]["critical_effect_truth_v0"]
    seen: set[str] = set()
    counts: dict[str, int] = {}
    for case in suite["cases"]:
        scenario_id = case["scenario_id"]
        if scenario_id in seen:
            raise ValueError(f"duplicate scenario id: {scenario_id}")
        seen.add(scenario_id)
        control_class = case["control_class"]
        counts[control_class] = counts.get(control_class, 0) + 1
        actual = evaluate_case(case, profile)
        if actual != case["expected"]:
            raise ValueError(
                f"{scenario_id}: expected {case['expected']!r}, computed {actual!r}"
            )
    expected_counts = {
        "positive_control": 5,
        "negative_control": 8,
        "indeterminate_control": 3,
    }
    if counts != expected_counts:
        raise ValueError(f"unexpected observer-independence control distribution: {counts!r}")
    return counts


def main() -> int:
    suite = load(BASE / "observer-independence-closure.fixtures.json")
    schema = load(BASE / "observer-independence-closure.schema.json")
    counts = validate_suite(suite, schema)
    print(
        f"OIC_SCHEMA=STRUCTURE_CHECKED CASES={len(suite['cases'])} "
        f"CONTROLS={json.dumps(counts, sort_keys=True)} "
        "NO_AGGREGATE_SCORE=true CLAIM=SYNTHETIC_ONLY"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
