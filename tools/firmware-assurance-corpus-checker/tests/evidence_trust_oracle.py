#!/usr/bin/env python3
"""Independent standard-library oracle for synthetic evidence-trust cases.

The corpus stores synthetic verifier classifications. This tool does not perform
cryptographic verification, consult revocation services, or touch a real device.
"""
from __future__ import annotations

import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

MAX_BYTES = 4 * 1024 * 1024
FIELDS = {
    "signature_status": ("valid", "invalid", "unknown"),
    "signer_status": ("authorized", "unauthorized", "unknown"),
    "subject_status": ("match", "mismatch", "unknown"),
    "artifact_status": ("match", "mismatch", "unknown"),
    "policy_status": ("match", "mismatch", "unknown"),
    "revocation_status": ("clear", "revoked", "unknown"),
    "freshness_status": ("fresh", "expired", "unknown"),
    "nonce_status": ("current", "replay", "unknown"),
    "counter_status": ("monotonic", "rollback", "unknown"),
    "provenance_status": ("authenticated", "unauthenticated", "unknown"),
}
POSITIVE = {k: values[0] for k, values in FIELDS.items()}
NEGATIVE = {k: values[1] for k, values in FIELDS.items()}
UNKNOWN = "unknown"


def decide(case: dict[str, object]) -> str:
    """Independent expression of the policy: bad beats unknown; all-good passes."""
    for field, bad in NEGATIVE.items():
        if case[field] == bad:
            return "rejected"
    if any(case[field] == UNKNOWN for field in FIELDS):
        return "indeterminate"
    if all(case[field] == good for field, good in POSITIVE.items()):
        return "accepted"
    return "indeterminate"


def audit(path: Path) -> dict[str, object]:
    raw = path.read_bytes()
    if len(raw) > MAX_BYTES:
        raise ValueError("evidence trust corpus exceeds 4 MiB")
    corpus = json.loads(raw)
    if not isinstance(corpus, dict):
        raise ValueError("corpus root must be an object")
    if corpus.get("schema_version") != "firmware-evidence-trust-corpus/1.0.0":
        raise ValueError("unsupported corpus version")
    if corpus.get("status") != "synthetic_only" or corpus.get("physical_target") != "none":
        raise ValueError("corpus must be synthetic-only")
    if corpus.get("mutation_enabled") is not False:
        raise ValueError("mutation_enabled must remain false")
    cases = corpus.get("cases")
    if not isinstance(cases, list) or len(cases) != 32 or corpus.get("case_count") != len(cases):
        raise ValueError("corpus must contain exactly 32 cases")
    errors: list[str] = []
    counts: Counter[str] = Counter()
    seen: set[str] = set()
    for index, case in enumerate(cases, start=1):
        expected_id = f"EV-{index:03d}"
        if not isinstance(case, dict):
            errors.append(f"{expected_id}: case must be an object")
            continue
        if case.get("case_id") != expected_id or expected_id in seen:
            errors.append(f"{expected_id}: ID is missing, duplicated, or unordered")
        seen.add(expected_id)
        for field, states in FIELDS.items():
            if case.get(field) not in states:
                errors.append(f"{expected_id}: invalid or missing {field}")
        expected = case.get("expected_decision")
        derived = decide(case)
        if expected != derived:
            errors.append(f"{expected_id}: expected {expected}, derived {derived}")
        counts[derived] += 1

    # Deterministic adversarial sweep over all ten dimensions. Never use model output.
    seed = 0x45563031
    distinct: set[tuple[str, ...]] = set()
    violations = 0
    trials = 10_000
    for _ in range(trials):
        mutated: dict[str, object] = {}
        for field, states in FIELDS.items():
            seed = (seed * 1_664_525 + 1_013_904_223) & 0xFFFFFFFF
            mutated[field] = states[(seed >> 16) % len(states)]
        distinct.add(tuple(str(mutated[field]) for field in FIELDS))
        result = decide(mutated)
        has_bad = any(mutated[field] == NEGATIVE[field] for field in FIELDS)
        has_unknown = any(mutated[field] == UNKNOWN for field in FIELDS)
        expected_result = "rejected" if has_bad else "indeterminate" if has_unknown else "accepted"
        if result != expected_result:
            violations += 1
        if result == "accepted" and (has_bad or has_unknown):
            violations += 1

    if counts["accepted"] < 1 or counts["rejected"] < 10 or counts["indeterminate"] < 10:
        errors.append("baseline corpus lacks accepted/negative/indeterminate controls")
    if violations:
        errors.append(f"{violations} fail-closed invariant violations")
    return {
        "report_version": "firmware-evidence-trust-oracle/1.0.0",
        "status": "passed" if not errors else "blocked",
        "corpus_sha256": hashlib.sha256(raw).hexdigest(),
        "case_count": len(cases),
        "baseline_decision_counts": dict(sorted(counts.items())),
        "mutation_trials": trials,
        "distinct_assignments": len(distinct),
        "invariant_violations": violations,
        "physical_device_operations": 0,
        "firmware_mutations": 0,
        "cryptographic_verification_performed": False,
        "claim_ceiling": "synthetic_policy_conformance_not_evidence_authentication",
        "errors": errors[:100],
    }


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: evidence_trust_oracle.py <corpus.json>", file=sys.stderr)
        return 2
    try:
        result = audit(Path(sys.argv[1]))
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"evidence trust oracle error: {exc}", file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True, indent=2))
    return 0 if result["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
