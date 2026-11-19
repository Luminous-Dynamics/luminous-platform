#!/usr/bin/env python3
"""Test-only WAEP Draft 2020-12 schema and fixture validator.

This script is build/test tooling. It is not a production authorization engine,
evidence verifier, or source of security authority.
"""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
WAEP = ROOT / "docs" / "standards" / "web4"


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot load {path.relative_to(ROOT)}: {exc}") from exc


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return "sha256:" + digest.hexdigest()


def main() -> int:
    schema_path = WAEP / "waep-v0.1.schema.json"
    example_path = WAEP / "waep-example.v0.1.json"
    vectors_path = WAEP / "waep-conformance-vectors.v0.1.json"
    receipt_path = WAEP / "waep-synthetic-participant-receipt.v0.1.json"
    corpus_path = WAEP / "waep-synthetic-corpus.v0.1.json"

    schema = load_json(schema_path)
    example = load_json(example_path)
    suite = load_json(vectors_path)
    receipt = load_json(receipt_path)
    corpus = load_json(corpus_path)

    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise SystemExit("WAEP schema must declare Draft 2020-12")
    if schema.get("$id") != "urn:luminous:web4:waep:0.1":
        raise SystemExit("unexpected WAEP schema identifier")

    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())

    example_errors = sorted(
        validator.iter_errors(example),
        key=lambda error: tuple(str(part) for part in error.absolute_path),
    )
    if example_errors:
        for error in example_errors:
            location = "/".join(str(part) for part in error.absolute_path) or "<root>"
            print(f"example {location}: {error.message}", file=sys.stderr)
        raise SystemExit(f"WAEP example failed with {len(example_errors)} error(s)")

    if example["claim"]["disposition"] != "not_claimed":
        raise SystemExit("design-only example must remain not_claimed")
    if example["evaluation"]["run_state"] != "not_executed":
        raise SystemExit("design-only example must remain not_executed")

    # Bind fixture references to actual committed file bytes. These are
    # synthetic teaching fixtures, not authenticated participant evidence.
    receipt_digest = sha256_file(receipt_path)
    corpus_digest = sha256_file(corpus_path)
    empty_digest = "sha256:" + hashlib.sha256(b"").hexdigest()

    if receipt.get("synthetic") is not True:
        raise SystemExit("participant receipt fixture must explicitly be synthetic")
    if corpus.get("corpus_id") != "waep-synthetic-2case-v0.1":
        raise SystemExit("unexpected synthetic corpus ID")
    if len(corpus.get("cases", [])) != 2:
        raise SystemExit("synthetic corpus fixture must contain exactly two cases")
    if example["evaluation"]["test_corpus"]["digest"] != empty_digest:
        raise SystemExit("not-executed example must bind the actual empty-corpus digest")

    vectors = suite.get("vectors")
    if suite.get("profile") != "luminous-web4-waep-conformance-vectors" or not isinstance(vectors, list):
        raise SystemExit("invalid WAEP conformance-vector envelope")

    failures = 0
    expected_valid = 0
    expected_invalid = 0
    for vector in vectors:
        vector_id = vector.get("vector_id", "<missing-vector-id>")
        expected = vector.get("expected")
        if expected not in {"valid", "invalid"}:
            print(f"{vector_id}: invalid expected disposition", file=sys.stderr)
            failures += 1
            continue

        if expected == "valid":
            expected_valid += 1
        else:
            expected_invalid += 1

        record = vector.get("record", {})
        evaluation = record.get("evaluation", {})
        if evaluation.get("run_state") == "completed":
            if evaluation.get("test_corpus", {}).get("digest") != corpus_digest:
                print(f"{vector_id}: synthetic corpus digest mismatch", file=sys.stderr)
                failures += 1
                continue

        artifact_binding_failed = False
        for artifact in record.get("claim", {}).get("evidence", []):
            if artifact.get("artifact_id") == "artifact-participant-receipt":
                if artifact.get("digest") != receipt_digest:
                    print(f"{vector_id}: synthetic participant-receipt digest mismatch", file=sys.stderr)
                    failures += 1
                    artifact_binding_failed = True
        if artifact_binding_failed:
            continue

        errors = sorted(
            validator.iter_errors(record),
            key=lambda error: tuple(str(part) for part in error.absolute_path),
        )
        actual = "valid" if not errors else "invalid"
        if actual != expected:
            print(f"{vector_id}: expected {expected}, got {actual}", file=sys.stderr)
            for error in errors[:8]:
                location = "/".join(str(part) for part in error.absolute_path) or "<root>"
                print(f"  {location}: {error.message}", file=sys.stderr)
            failures += 1
        else:
            print(f"{vector_id}: {actual} as expected")

    if failures:
        raise SystemExit(f"WAEP conformance failed: {failures} mismatch(es) across {len(vectors)} vectors")

    print(
        "WAEP_SCHEMA=Draft2020-12_CHECKED "
        f"EXAMPLE=VALID VECTORS={len(vectors)} MATCHED "
        f"POSITIVE_CONTROLS={expected_valid} NEGATIVE_CONTROLS={expected_invalid} "
        "SYNTHETIC_DIGESTS=BOUND CLAIM_CEILING=UNCHANGED"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
