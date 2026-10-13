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

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[2]
SCHEMA_PATH = ROOT / "docs/schemas/acs-nixward-bridge-test-corpus-v1.schema.json"
CORPUS_PATH = ROOT / "docs/fixtures/acs-nixward-bridge-v1-cases.json"


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise ValueError(f"{path.relative_to(ROOT)}: unable to read strict UTF-8 JSON: {exc}") from exc


def main() -> int:
    try:
        schema = load_json(SCHEMA_PATH)
        corpus = load_json(CORPUS_PATH)
    except ValueError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    try:
        Draft202012Validator.check_schema(schema)
    except Exception as exc:  # jsonschema exposes several schema-validation exception subclasses
        print(f"FAIL: corpus schema is not valid Draft 2020-12: {exc}", file=sys.stderr)
        return 1

    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(
        validator.iter_errors(corpus),
        key=lambda err: (list(map(str, err.absolute_path)), err.message),
    )
    if errors:
        for err in errors:
            path = "$" + "".join(f"[{part}]" if isinstance(part, int) else f".{part}" for part in err.absolute_path)
            print(f"FAIL: {path}: {err.message}", file=sys.stderr)
        return 1

    cases = corpus["cases"]
    ids = [case["id"] for case in cases]
    expected_ids = [f"BR-{number:03d}" for number in range(1, 33)]
    if ids != expected_ids:
        print("FAIL: case IDs must be unique and ordered BR-001 through BR-032", file=sys.stderr)
        return 1

    by_id = {case["id"]: case for case in cases}
    must_not_mutate = {
        "BR-001", "BR-002", "BR-003", "BR-004", "BR-005",
        "BR-007", "BR-008", "BR-009", "BR-010", "BR-011",
        "BR-012", "BR-013", "BR-014", "BR-015", "BR-016",
        "BR-017", "BR-018", "BR-019", "BR-020", "BR-021",
        "BR-022", "BR-024", "BR-027", "BR-029", "BR-030",
        "BR-031", "BR-032",
    }
    violating = [case_id for case_id in sorted(must_not_mutate) if by_id[case_id]["max_mutating_effects"] != 0]
    if violating:
        print(f"FAIL: negative/control case must cap mutating effects at zero: {', '.join(violating)}", file=sys.stderr)
        return 1

    required_outcomes = {
        "BR-003": ("ALLOW_AT_HOOK_ONLY", "not_authorized"),
        "BR-018": ("DENY_ON_FAILURE", "none_or_rejected"),
        "BR-023": ("BLOCK_RETRY_UNTIL_RECONCILED", "indeterminate_requires_explicit_semantics"),
        "BR-031": ("QUALIFICATION_BLOCKED", "no_pass_claim"),
        "BR-032": ("HOST_BOOT_RECOVERY_CONTINUES", "no_authority_expansion"),
    }
    for case_id, expected in required_outcomes.items():
        case = by_id[case_id]
        actual = (case["expected_bridge_outcome"], case["expected_transaction"])
        if actual != expected:
            print(f"FAIL: {case_id} safety invariant: expected {expected!r}, got {actual!r}", file=sys.stderr)
            return 1

    related_issues = {entry["issue"] for entry in corpus["related_existing_work"]}
    if not {63, 70, 75}.issubset(related_issues):
        print("FAIL: corpus must retain links to Nixward issues #63, #70, and #75", file=sys.stderr)
        return 1

    print(f"PASS: Draft 2020-12 schema and corpus structure validated ({len(cases)} cases).")
    print("PASS: case IDs, required no-effect controls, and critical safety invariants validated.")
    print("LIMIT: this is corpus validation only; no product, ACS wire, host-hook, Nixward, VM, or security qualification was performed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
