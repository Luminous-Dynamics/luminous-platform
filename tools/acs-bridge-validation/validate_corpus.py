#!/usr/bin/env python3
"""Validate the ACS↔Nixward bridge's frozen test-specification corpus.

This checks corpus/schema consistency only. It does not run ACS, Nixward,
host hooks, privileged actions, VM tests, or establish product conformance.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker, SchemaError

ROOT = Path(__file__).resolve().parents[2]
SCHEMA_PATH = ROOT / "docs/schemas/acs-nixward-bridge-test-corpus-v1.schema.json"
CORPUS_PATH = ROOT / "docs/fixtures/acs-nixward-bridge-v1-cases.json"


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """Reject ambiguous JSON objects instead of silently keeping the last key."""
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key!r}")
        result[key] = value
    return result


def loads_strict(text: str, *, source: str = "<input>") -> Any:
    """Parse UTF-8-decoded JSON while rejecting duplicate keys and non-JSON constants."""
    def reject_constant(value: str) -> None:
        raise ValueError(f"non-standard JSON numeric constant {value!r} in {source}")

    try:
        return json.loads(
            text,
            object_pairs_hook=_unique_object,
            parse_constant=reject_constant,
        )
    except (json.JSONDecodeError, ValueError) as exc:
        raise ValueError(f"{source}: invalid strict JSON: {exc}") from exc


def load_json(path: Path) -> Any:
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as exc:
        raise ValueError(f"{path.relative_to(ROOT)}: unable to read strict UTF-8 JSON: {exc}") from exc
    return loads_strict(text, source=str(path.relative_to(ROOT)))


def validate_documents(schema: Any, corpus: Any) -> list[str]:
    """Return deterministic error messages for schema, corpus, and frozen safety invariants."""
    errors: list[str] = []
    try:
        Draft202012Validator.check_schema(schema)
    except SchemaError as exc:
        return [f"corpus schema is not valid Draft 2020-12: {exc.message}"]

    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    schema_errors = sorted(
        validator.iter_errors(corpus),
        key=lambda err: (list(map(str, err.absolute_path)), err.message),
    )
    for err in schema_errors:
        path = "$" + "".join(f"[{part}]" if isinstance(part, int) else f".{part}" for part in err.absolute_path)
        errors.append(f"{path}: {err.message}")
    if errors:
        return errors

    cases = corpus["cases"]
    ids = [case["id"] for case in cases]
    expected_ids = [f"BR-{number:03d}" for number in range(1, 33)]
    if ids != expected_ids:
        # Guard the fixed-ID invariant before looking up safety-critical cases;
        # a malformed corpus must produce a validation error, never a KeyError.
        return ["case IDs must be unique and ordered BR-001 through BR-032"]

    by_id = {case["id"]: case for case in cases}
    must_not_mutate = {
        "BR-001", "BR-002", "BR-003", "BR-004", "BR-005",
        "BR-007", "BR-008", "BR-009", "BR-010", "BR-011",
        "BR-012", "BR-013", "BR-014", "BR-015", "BR-016",
        "BR-017", "BR-018", "BR-019", "BR-020", "BR-021",
        "BR-022", "BR-024", "BR-027", "BR-029", "BR-030",
        "BR-031", "BR-032",
    }
    violating = [
        case_id for case_id in sorted(must_not_mutate)
        if by_id[case_id]["max_mutating_effects"] != 0
    ]
    if violating:
        errors.append(
            "negative/control case must cap mutating effects at zero: "
            + ", ".join(violating)
        )

    required_outcomes = {
        "BR-003": ("ALLOW_AT_HOOK_ONLY", "not_authorized"),
        "BR-018": ("DENY_ON_FAILURE", "none_or_rejected"),
        "BR-020": ("DENY_OR_DISABLE_MUTATION_CAPABILITY", "none_or_rejected"),
        "BR-021": ("DENY_UNSUPPORTED_MODIFY", "none_or_rejected"),
        "BR-023": ("BLOCK_RETRY_UNTIL_RECONCILED", "indeterminate_requires_explicit_semantics"),
        "BR-031": ("QUALIFICATION_BLOCKED", "no_pass_claim"),
        "BR-032": ("HOST_BOOT_RECOVERY_CONTINUES", "no_authority_expansion"),
    }
    for case_id, expected in required_outcomes.items():
        case = by_id[case_id]
        actual = (case["expected_bridge_outcome"], case["expected_transaction"])
        if actual != expected:
            errors.append(
                f"{case_id} safety invariant: expected {expected!r}, got {actual!r}"
            )

    if "startup posture" not in by_id["BR-020"]["pass_condition"].lower() or "refuse" not in by_id["BR-020"]["pass_condition"].lower():
        errors.append("BR-020 must require out-of-band startup posture=refuse for handshake failure")
    if "acs-core conformance remains unclaimed" not in by_id["BR-021"]["pass_condition"].lower():
        errors.append("BR-021 must not claim ACS-Core conformance when a required disposition is unsupported")

    related_issues = {entry["issue"] for entry in corpus["related_existing_work"]}
    if not {63, 70, 75}.issubset(related_issues):
        errors.append("corpus must retain links to Nixward issues #63, #70, and #75")
    return errors


def main() -> int:
    try:
        schema = load_json(SCHEMA_PATH)
        corpus = load_json(CORPUS_PATH)
    except ValueError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    errors = validate_documents(schema, corpus)
    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1

    print(f"PASS: Draft 2020-12 schema and corpus structure validated ({len(corpus['cases'])} cases).")
    print("PASS: case IDs, required no-effect controls, and critical safety invariants validated.")
    print("LIMIT: corpus validation only; no product, ACS wire, host-hook, Nixward, VM, or security qualification was performed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
