#!/usr/bin/env python3
"""Fail if previously committed source-review receipt history is rewritten."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

LEDGER_NAMES = ("source_review_receipts", "requirement_review_receipts")


def check_review_history(baseline: dict[str, Any], current: dict[str, Any]) -> list[str]:
    """Each receipt ledger in the baseline must remain an exact prefix."""
    violations: list[str] = []
    for ledger_name in LEDGER_NAMES:
        old = baseline.get(ledger_name, [])
        new = current.get(ledger_name, [])
        if not isinstance(old, list) or not isinstance(new, list):
            violations.append(
                f"{ledger_name} must be arrays in baseline and current pack"
            )
            continue
        if ledger_name not in baseline and ledger_name in current and new:
            violations.append(
                f"{ledger_name} was introduced with pre-populated history; "
                "introduce an empty ledger in an earlier commit first"
            )
        if len(new) < len(old):
            violations.append(
                f"{ledger_name} history was shortened: baseline={len(old)} current={len(new)}"
            )
            continue
        for index, previous in enumerate(old):
            if previous != new[index]:
                receipt_id = (
                    previous.get("receipt_id", "<unknown>")
                    if isinstance(previous, dict)
                    else "<invalid>"
                )
                violations.append(
                    f"{ledger_name} baseline receipt at index {index} ({receipt_id}) "
                    "was changed, removed, or reordered"
                )
    return violations


def load_object(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return value


def receipt_counts(pack: dict[str, Any]) -> dict[str, int | None]:
    counts: dict[str, int | None] = {}
    for ledger_name in LEDGER_NAMES:
        ledger = pack.get(ledger_name, [])
        counts[ledger_name] = len(ledger) if isinstance(ledger, list) else None
    return counts


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline-pack", required=True, type=Path)
    parser.add_argument("--pack", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        baseline = load_object(args.baseline_pack)
        current = load_object(args.pack)
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        print(json.dumps({"passed": False, "errors": [str(exc)]}, sort_keys=True))
        return 1
    errors = check_review_history(baseline, current)
    result = {
        "baseline_receipt_counts": receipt_counts(baseline),
        "current_receipt_counts": receipt_counts(current),
        "passed": not errors,
        "errors": errors,
    }
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
