#!/usr/bin/env python3
"""Fail if previously committed source-review receipts are edited, reordered, or removed."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def check_review_history(baseline: dict[str, Any], current: dict[str, Any]) -> list[str]:
    """The baseline receipt list must remain an exact prefix of the current ledger."""
    old = baseline.get("source_review_receipts", [])
    new = current.get("source_review_receipts", [])
    if not isinstance(old, list) or not isinstance(new, list):
        return ["source_review_receipts must be arrays in baseline and current pack"]
    if len(new) < len(old):
        return [f"receipt history was shortened: baseline={len(old)} current={len(new)}"]
    violations = []
    for index, previous in enumerate(old):
        if previous != new[index]:
            receipt_id = previous.get("receipt_id", "<unknown>") if isinstance(previous, dict) else "<invalid>"
            violations.append(
                f"baseline receipt at index {index} ({receipt_id}) was changed, removed, or reordered"
            )
    return violations


def load_object(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return value


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
    baseline_receipts = baseline.get("source_review_receipts", [])
    current_receipts = current.get("source_review_receipts", [])
    result = {
        "baseline_receipt_count": len(baseline_receipts) if isinstance(baseline_receipts, list) else None,
        "current_receipt_count": len(current_receipts) if isinstance(current_receipts, list) else None,
        "passed": not errors,
        "errors": errors,
    }
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
