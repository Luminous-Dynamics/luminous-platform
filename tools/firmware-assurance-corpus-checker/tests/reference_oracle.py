#!/usr/bin/env python3
"""Independent, test-only reference oracle for synthetic claim-state policy.

This script intentionally uses only the Python standard library and does not
import the Rust implementation. It never reads a device or authorizes changes.
"""
from __future__ import annotations

import hashlib
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any

SCHEMA_VERSION = "firmware-assurance-corpus/1.0.0"
PROFILE_ID = "synthetic-uefi-uki-v1"
DISPOSITION_BY_KIND = {
    "positive_control": "eligible_for_lab_staging",
    "negative_control": "blocked",
    "indeterminate_control": "indeterminate",
}
REQUIRED_READ_ONLY = {
    "no_firmware_write",
    "no_key_enrollment",
    "no_boot_configuration_change",
}
ALLOWED_STATES = {"verified", "refuted", "unknown", "not_applicable"}


def derive(claims: dict[str, str], required: list[str]) -> str:
    """Re-derive a disposition without trusting expected_disposition."""
    if not required or len(required) != len(set(required)):
        raise ValueError("required claims must be nonempty and unique")
    values: list[str] = []
    for claim in required:
        if claim not in claims:
            raise ValueError(f"required claim missing: {claim}")
        state = claims[claim]
        if state not in ALLOWED_STATES:
            raise ValueError(f"unsupported state for {claim}: {state}")
        values.append(state)
    if "refuted" in values:
        return "blocked"
    if any(value in {"unknown", "not_applicable"} for value in values):
        return "indeterminate"
    return "eligible_for_lab_staging"


def derive_by_counts(claims: dict[str, str], required: list[str]) -> str:
    """Second, separately expressed oracle used only for differential checks."""
    values = [claims[name] for name in required]
    refutation_count = sum(value == "refuted" for value in values)
    unresolved_count = sum(value in {"unknown", "not_applicable"} for value in values)
    if refutation_count:
        return "blocked"
    if unresolved_count:
        return "indeterminate"
    if values and all(value == "verified" for value in values):
        return "eligible_for_lab_staging"
    raise ValueError("cannot derive a disposition from an empty or invalid required set")


def audit(path: Path) -> dict[str, Any]:
    raw = path.read_bytes()
    if len(raw) > 4 * 1024 * 1024:
        raise ValueError("corpus exceeds 4 MiB")
    corpus = json.loads(raw)
    if not isinstance(corpus, dict):
        raise ValueError("corpus root must be an object")
    if corpus.get("schema_version") != SCHEMA_VERSION:
        raise ValueError("unsupported corpus schema version")
    if corpus.get("status") != "synthetic_only" or corpus.get("physical_target") != "none":
        raise ValueError("corpus must remain synthetic-only with no physical target")
    if corpus.get("profile_id") != PROFILE_ID or corpus.get("mutation_enabled") is not False:
        raise ValueError("profile or no-mutation invariant failed")

    cases = corpus.get("cases")
    if not isinstance(cases, list) or not 32 <= len(cases) <= 10_000:
        raise ValueError("case list is outside the allowed size")
    if corpus.get("case_count") != len(cases):
        raise ValueError("case_count does not match the actual list")

    kind_counts: Counter[str] = Counter()
    disposition_counts: Counter[str] = Counter()
    mutation_checks = 0
    seen: set[str] = set()
    errors: list[str] = []

    for index, case in enumerate(cases, start=1):
        expected_id = f"FWA-{index:03d}"
        if not isinstance(case, dict):
            errors.append(f"{expected_id}: case must be an object")
            continue
        case_id = case.get("case_id")
        if case_id != expected_id or case_id in seen:
            errors.append(f"{expected_id}: invalid or duplicate identity")
        seen.add(str(case_id))
        if case.get("profile_id") != PROFILE_ID:
            errors.append(f"{expected_id}: profile mismatch")

        kind = case.get("case_kind")
        if not isinstance(kind, str):
            errors.append(f"{expected_id}: case kind must be a string")
            continue
        expected_kind_disposition = DISPOSITION_BY_KIND.get(kind)
        if expected_kind_disposition is None:
            errors.append(f"{expected_id}: invalid case kind")
            continue
        kind_counts[kind] += 1

        claims = case.get("expected_claims")
        required = case.get("required_claims")
        if not isinstance(claims, dict) or not isinstance(required, list):
            errors.append(f"{expected_id}: malformed claim state assignment")
            continue
        if not all(isinstance(key, str) and isinstance(state, str) for key, state in claims.items()):
            errors.append(f"{expected_id}: malformed claim key/state")
            continue
        if not all(isinstance(name, str) for name in required):
            errors.append(f"{expected_id}: required claim names must be strings")
            continue

        for state in claims.values():
            if state not in ALLOWED_STATES:
                errors.append(f"{expected_id}: unsupported claim state {state}")
        assertions = case.get("required_assertions")
        if (
            not isinstance(assertions, list)
            or not all(isinstance(value, str) for value in assertions)
            or not REQUIRED_READ_ONLY.issubset(set(assertions))
        ):
            errors.append(f"{expected_id}: read-only assertions missing")

        if kind == "positive_control":
            if any(claims.get(name) == "refuted" for name in claims):
                errors.append(f"{expected_id}: positive control contains a refuted claim")
        elif kind == "negative_control":
            if not any(claims.get(name) == "refuted" for name in claims):
                errors.append(f"{expected_id}: negative control has no refutation")
        elif kind == "indeterminate_control":
            if any(claims.get(name) == "refuted" for name in claims) or not any(
                claims.get(name) == "unknown" for name in claims
            ):
                errors.append(f"{expected_id}: indeterminate control is contradictory")

        try:
            derived = derive(claims, required)
        except ValueError as error:
            errors.append(f"{expected_id}: {error}")
            continue

        stated = case.get("expected_disposition")
        if stated != derived or stated != expected_kind_disposition:
            errors.append(f"{expected_id}: expected disposition disagrees with independent oracle")
        disposition_counts[derived] += 1

        # Metamorphic checks: a required refutation dominates; all verified inputs
        # are eligible; unknown cannot be silently accepted for a positive control.
        for name in required:
            mutated = dict(claims)
            mutated[name] = "refuted"
            if derive(mutated, required) != "blocked":
                errors.append(f"{expected_id}: required refutation did not block ({name})")
                break
            mutation_checks += 1

        repaired = dict(claims)
        for name in required:
            repaired[name] = "verified"
        if derive(repaired, required) != "eligible_for_lab_staging":
            errors.append(f"{expected_id}: all-verified required claims did not become eligible")
        mutation_checks += 1

        if kind == "positive_control":
            for name in required:
                mutated = dict(claims)
                mutated[name] = "unknown"
                if derive(mutated, required) != "indeterminate":
                    errors.append(f"{expected_id}: required unknown did not become indeterminate ({name})")
                    break
                mutation_checks += 1

    # Deterministic mutation sweep over synthetic claim-state inputs. This is a
    # policy-composition test, not a firmware/artifact/cryptographic test.
    rng_state = 0xF17E2026
    state_values = ["verified", "unknown", "refuted", "not_applicable"]
    mutated_input_count = 10_000
    unique_assignments: set[tuple[str, tuple[tuple[str, str], ...]]] = set()
    mutation_dispositions: Counter[str] = Counter()
    policy_oracle_disagreements = 0
    nonrequired_claim_invariance_checks = 0

    def next_index(limit: int) -> int:
        nonlocal rng_state
        rng_state = (1664525 * rng_state + 1013904223) & 0xFFFFFFFF
        return (rng_state >> 16) % limit

    for trial in range(mutated_input_count):
        case = cases[trial % len(cases)]
        claims = dict(case["expected_claims"])
        required = list(case["required_claims"])
        required_set = set(required)
        if not required or len(required_set) != len(required):
            errors.append(f"mutation {trial}: required claims are empty or duplicated")
            continue

        # Vary non-required post-stage facts to check stage isolation.
        for name in claims:
            if name not in required_set:
                claims[name] = state_values[next_index(len(state_values))]

        mode = trial % 3
        if mode == 0:
            for name in required:
                claims[name] = "verified"
            expected_mutation_disposition = "eligible_for_lab_staging"
        elif mode == 1:
            allowed_unresolved = ["verified", "unknown", "not_applicable"]
            for name in required:
                claims[name] = allowed_unresolved[next_index(len(allowed_unresolved))]
            claims[required[next_index(len(required))]] = "unknown"
            expected_mutation_disposition = "indeterminate"
        else:
            for name in required:
                claims[name] = state_values[next_index(len(state_values))]
            claims[required[next_index(len(required))]] = "refuted"
            expected_mutation_disposition = "blocked"

        try:
            result_a = derive(claims, required)
            result_b = derive_by_counts(claims, required)
        except (KeyError, ValueError) as error:
            errors.append(f"mutation {trial}: {error}")
            continue

        if result_a != result_b:
            policy_oracle_disagreements += 1
            errors.append(f"mutation {trial}: independent policy oracles disagree")
        if result_a != expected_mutation_disposition:
            errors.append(f"mutation {trial}: disposition violates the selected mutation mode")
        mutation_dispositions[result_a] += 1
        unique_assignments.add((str(case["case_id"]), tuple(sorted(claims.items()))))

        # Changing a claim explicitly outside this stage's required set must not
        # change the disposition for the current stage.
        nonrequired = [name for name in claims if name not in required_set]
        if nonrequired:
            name = nonrequired[next_index(len(nonrequired))]
            changed = dict(claims)
            current_index = state_values.index(changed[name])
            changed[name] = state_values[(current_index + 1) % len(state_values)]
            if derive(changed, required) != result_a:
                errors.append(f"mutation {trial}: a non-required claim changed the stage disposition")
            nonrequired_claim_invariance_checks += 1

    if len(seen) != len(cases):
        errors.append("not every case identity was unique")
    if set(kind_counts) != set(DISPOSITION_BY_KIND):
        errors.append("all three control kinds must be represented")
    if policy_oracle_disagreements:
        errors.append("independent policy implementations disagreed")
    if sum(mutation_dispositions.values()) != mutated_input_count:
        errors.append("not every deterministic mutation received a terminal disposition")

    return {
        "report_version": "firmware-assurance-reference-oracle/1.0.0",
        "status": "passed" if not errors else "blocked",
        "oracle_implementation": "python-standard-library-independent-from-rust",
        "corpus_sha256": hashlib.sha256(raw).hexdigest(),
        "profile_id": PROFILE_ID,
        "case_count": len(cases),
        "case_kind_counts": dict(sorted(kind_counts.items())),
        "derived_disposition_counts": dict(sorted(disposition_counts.items())),
        "mutated_input_count": mutated_input_count,
        "distinct_claim_assignments": len(unique_assignments),
        "mutation_disposition_counts": dict(sorted(mutation_dispositions.items())),
        "policy_oracle_disagreements": policy_oracle_disagreements,
        "nonrequired_claim_invariance_checks": nonrequired_claim_invariance_checks,
        "metamorphic_checks": mutation_checks,
        "errors": errors[:100],
        "physical_device_operations": 0,
        "claim_ceiling": "synthetic_claim_policy_only_not_firmware_verification",
    }


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: reference_oracle.py <corpus.json>", file=sys.stderr)
        return 2
    try:
        result = audit(Path(sys.argv[1]))
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"reference oracle error: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True, indent=2))
    return 0 if result["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
