#!/usr/bin/env python3
"""Independent, test-only oracle for Secure Boot lifecycle transition cases.

Standard-library only. It does not import or execute the Rust implementation,
touch a device, invoke sbctl, generate Nix config, or authorize firmware changes.
"""
from __future__ import annotations

import hashlib
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any

MAX_BYTES = 4 * 1024 * 1024
SCHEMA_VERSION = "secure-boot-lifecycle-corpus/1.0.0"
PROFILE_ID = "sb-lifecycle-v1-synthetic"
VALID_STATES = {"enabled", "disabled", "setup_mode", "unknown"}
VALID_METHODS = {"secure_boot", "standard_boot", "unresolved"}
VALID_STAGES = {"prepare", "enroll", "verify_post_reboot"}
DECISIONS = {
    "blocked",
    "indeterminate",
    "standard_boot_allowed_unsecured",
    "configuration_preparation_allowed",
    "manual_enrollment_review_allowed",
    "pending_post_reboot_verification",
    "secure_boot_verified",
}
BOOL_FIELDS = [
    "profile_id_present",
    "version_pins_present",
    "user_confirmed",
    "exact_profile_qualified",
    "recovery_path_verified",
    "hardware_profile_matches",
    "state_matches_approval",
    "pki_bundle_matches",
    "key_generation_enabled",
    "configuration_prepared",
    "signing_key_verified",
    "boot_artifacts_verified",
    "operator_reviewed_command",
    "enrollment_attempted",
    "secure_boot_user_mode",
    "expected_boot_target_selected",
    "post_boot_health_observed",
    "recovery_retested",
]


def decide(case: dict[str, Any]) -> str:
    """Separately expressed lifecycle policy oracle."""
    state = case["observed_state"]
    method = case["method"]
    stage = case["stage"]

    if state == "unknown":
        return "indeterminate"
    if method == "unresolved" or not case["user_confirmed"]:
        return "blocked"
    if not case["profile_id_present"] or not case["version_pins_present"]:
        return "blocked"

    if method == "standard_boot":
        if stage != "prepare":
            return "blocked"
        return "standard_boot_allowed_unsecured" if state == "disabled" else "blocked"

    required_profile_proofs = (
        "exact_profile_qualified",
        "recovery_path_verified",
        "hardware_profile_matches",
        "state_matches_approval",
        "pki_bundle_matches",
    )
    if not all(case[name] for name in required_profile_proofs):
        return "blocked"

    if stage == "prepare":
        return (
            "configuration_preparation_allowed"
            if case["key_generation_enabled"]
            else "blocked"
        )

    if stage == "enroll":
        if (
            state == "setup_mode"
            and case["configuration_prepared"]
            and case["signing_key_verified"]
            and case["boot_artifacts_verified"]
            and case["operator_reviewed_command"]
        ):
            # This only permits a human to review/execute enrollment. It is not
            # a claim that an enrollment command ran or succeeded.
            return "manual_enrollment_review_allowed"
        return "blocked"

    if stage == "verify_post_reboot":
        verified = (
            state == "enabled"
            and case["configuration_prepared"]
            and case["enrollment_attempted"]
            and case["secure_boot_user_mode"]
            and case["expected_boot_target_selected"]
            and case["post_boot_health_observed"]
            and case["recovery_retested"]
        )
        return "secure_boot_verified" if verified else "pending_post_reboot_verification"

    return "blocked"


def audit(path: Path) -> dict[str, Any]:
    raw = path.read_bytes()
    if len(raw) > MAX_BYTES:
        raise ValueError("lifecycle corpus exceeds 4 MiB")
    corpus = json.loads(raw)
    if not isinstance(corpus, dict):
        raise ValueError("lifecycle corpus root must be an object")
    if corpus.get("schema_version") != SCHEMA_VERSION:
        raise ValueError("unexpected lifecycle corpus version")
    if corpus.get("status") != "synthetic_only" or corpus.get("physical_target") != "none":
        raise ValueError("lifecycle corpus must be synthetic-only with no physical target")
    if corpus.get("profile_id") != PROFILE_ID or corpus.get("mutation_enabled") is not False:
        raise ValueError("lifecycle profile/no-mutation invariant failed")
    cases = corpus.get("cases")
    if not isinstance(cases, list) or not 25 <= len(cases) <= 10_000:
        raise ValueError("lifecycle case count outside allowed bounds")
    if corpus.get("case_count") != len(cases):
        raise ValueError("case_count differs from actual cases")

    errors: list[str] = []
    seen: set[str] = set()
    baseline_counts: Counter[str] = Counter()
    for index, case in enumerate(cases, start=1):
        expected_id = f"SBL-{index:03d}"
        if not isinstance(case, dict):
            errors.append(f"{expected_id}: case is not an object")
            continue
        case_id = case.get("case_id")
        if case_id != expected_id or case_id in seen:
            errors.append(f"{expected_id}: case ID is missing, duplicate, or out of order")
        seen.add(str(case_id))
        if case.get("stage") not in VALID_STAGES:
            errors.append(f"{expected_id}: unsupported stage")
        if case.get("observed_state") not in VALID_STATES:
            errors.append(f"{expected_id}: unsupported observed state")
        if case.get("method") not in VALID_METHODS:
            errors.append(f"{expected_id}: unsupported method")
        for field in BOOL_FIELDS:
            if not isinstance(case.get(field), bool):
                errors.append(f"{expected_id}: {field} must be boolean")
        stated = case.get("expected_decision")
        if stated not in DECISIONS:
            errors.append(f"{expected_id}: unsupported expected decision")
            continue
        try:
            derived = decide(case)
        except (KeyError, TypeError) as error:
            errors.append(f"{expected_id}: malformed case: {error}")
            continue
        if derived != stated:
            errors.append(f"{expected_id}: expected {stated}, oracle derived {derived}")
        baseline_counts[derived] += 1

    # Deterministic state/claim mutation sweep. This does not invoke the Rust
    # implementation; it is a separately expressed policy sanity oracle.
    rng = 0x53424C31
    states = ["enabled", "disabled", "setup_mode", "unknown"]
    methods = ["secure_boot", "standard_boot", "unresolved"]
    stages = ["prepare", "enroll", "verify_post_reboot"]
    distinct: set[str] = set()
    mutation_counts: Counter[str] = Counter()
    invariant_violations = 0

    def next_index(limit: int) -> int:
        nonlocal rng
        rng = (1664525 * rng + 1013904223) & 0xFFFFFFFF
        return (rng >> 16) % limit

    for trial in range(10_000):
        source = cases[trial % len(cases)]
        mutated = dict(source)
        mutated["observed_state"] = states[next_index(len(states))]
        mutated["method"] = methods[next_index(len(methods))]
        mutated["stage"] = stages[next_index(len(stages))]
        for field in BOOL_FIELDS:
            mutated[field] = bool(next_index(2))

        result = decide(mutated)
        mutation_counts[result] += 1
        distinct.add(
            source["case_id"]
            + "|"
            + "|".join(
                [mutated["stage"], mutated["observed_state"], mutated["method"]]
                + [f"{name}={int(mutated[name])}" for name in BOOL_FIELDS]
            )
        )

        if mutated["observed_state"] == "unknown" and result != "indeterminate":
            invariant_violations += 1
        if not mutated["user_confirmed"] and mutated["observed_state"] != "unknown" and result != "blocked":
            invariant_violations += 1
        if result == "secure_boot_verified":
            required = [
                mutated["method"] == "secure_boot",
                mutated["stage"] == "verify_post_reboot",
                mutated["observed_state"] == "enabled",
                mutated["user_confirmed"],
                mutated["exact_profile_qualified"],
                mutated["recovery_path_verified"],
                mutated["hardware_profile_matches"],
                mutated["state_matches_approval"],
                mutated["pki_bundle_matches"],
                mutated["configuration_prepared"],
                mutated["enrollment_attempted"],
                mutated["secure_boot_user_mode"],
                mutated["expected_boot_target_selected"],
                mutated["post_boot_health_observed"],
                mutated["recovery_retested"],
            ]
            if not all(required):
                invariant_violations += 1

    if len(seen) != len(cases):
        errors.append("lifecycle case identities are not unique")
    if invariant_violations:
        errors.append(f"{invariant_violations} fail-closed mutation invariants violated")

    return {
        "report_version": "secure-boot-lifecycle-reference-oracle/1.0.0",
        "status": "passed" if not errors else "blocked",
        "corpus_sha256": hashlib.sha256(raw).hexdigest(),
        "case_count": len(cases),
        "baseline_decision_counts": dict(sorted(baseline_counts.items())),
        "mutation_trials": 10_000,
        "distinct_mutated_assignments": len(distinct),
        "mutation_decision_counts": dict(sorted(mutation_counts.items())),
        "invariant_violations": invariant_violations,
        "physical_device_operations": 0,
        "firmware_mutations": 0,
        "claim_ceiling": "synthetic_lifecycle_policy_only_not_device_or_firmware_verification",
        "errors": errors[:100],
    }


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: lifecycle_reference_oracle.py <lifecycle-corpus.json>", file=sys.stderr)
        return 2
    try:
        result = audit(Path(sys.argv[1]))
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"lifecycle oracle error: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True, indent=2))
    return 0 if result["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
