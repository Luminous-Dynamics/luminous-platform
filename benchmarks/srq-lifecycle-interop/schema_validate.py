#!/usr/bin/env python3
"""Full Draft 2020-12 validator for SRQ test fixtures; never a release evaluator.

This utility has no runtime authority and never changes a source fixture. It
rejects duplicate JSON keys and remote references, validates the schema itself,
enables registered format assertions, and emits a digest-bound result record.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import os
import platform
import stat
from pathlib import Path
from typing import Any

EXPECTED_JSONSCHEMA_VERSION = "4.26.0"
REPORT_SCHEMA = "srq-json-schema-validation-report-v1"
MAX_SCHEMA_BYTES = 2 * 1024 * 1024
MAX_INSTANCE_BYTES = 8 * 1024 * 1024


class DuplicateJsonKey(ValueError):
    pass


class NonStandardJsonConstant(ValueError):
    pass


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise DuplicateJsonKey(f"duplicate object key: {key!r}")
        value[key] = item
    return value


def reject_constant(value: str) -> None:
    raise NonStandardJsonConstant(f"non-standard JSON constant: {value}")


def load_strict_json(path: Path, maximum: int) -> tuple[Any, bytes]:
    # Open once without following symlinks; cap both the initial file size and
    # the bytes read in case the file grows after fstat.
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        fd = os.open(path, flags)
    except OSError as exc:
        raise ValueError(f"cannot securely open input: {path}: {exc}") from exc

    try:
        metadata = os.fstat(fd)
        if not stat.S_ISREG(metadata.st_mode):
            raise ValueError(f"input must be a regular file: {path}")
        if metadata.st_size > maximum:
            raise ValueError(f"input exceeds byte limit ({metadata.st_size} > {maximum}): {path}")
        with os.fdopen(fd, "rb", closefd=True) as stream:
            fd = -1
            raw = stream.read(maximum + 1)
    finally:
        if fd >= 0:
            os.close(fd)

    if len(raw) > maximum:
        raise ValueError(f"input grew beyond byte limit ({len(raw)} > {maximum}): {path}")
    parsed = json.loads(
        raw,
        object_pairs_hook=unique_object,
        parse_constant=reject_constant,
    )
    return parsed, raw


def find_remote_refs(node: Any, path: str = "$") -> list[str]:
    found: list[str] = []
    if isinstance(node, dict):
        ref = node.get("$ref")
        if isinstance(ref, str) and not ref.startswith("#"):
            found.append(f"{path}.$ref={ref!r}")
        for key, value in node.items():
            found.extend(find_remote_refs(value, f"{path}.{key}"))
    elif isinstance(node, list):
        for index, value in enumerate(node):
            found.extend(find_remote_refs(value, f"{path}[{index}]"))
    return found


def find_formats(node: Any) -> set[str]:
    found: set[str] = set()
    if isinstance(node, dict):
        fmt = node.get("format")
        if isinstance(fmt, str):
            found.add(fmt)
        for value in node.values():
            found.update(find_formats(value))
    elif isinstance(node, list):
        for value in node:
            found.update(find_formats(value))
    return found


def pointer(parts: Any) -> str:
    def escape(part: Any) -> str:
        return str(part).replace("~", "~0").replace("/", "~1")
    return "" if not parts else "/" + "/".join(escape(part) for part in parts)


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--schema", required=True, type=Path)
    parser.add_argument("--instance", required=True, type=Path)
    parser.add_argument("--expect", required=True, choices=("valid", "invalid"))
    parser.add_argument("--output", type=Path, help="Optional JSON report path.")
    args = parser.parse_args()

    report: dict[str, Any] = {
        "schema": REPORT_SCHEMA,
        "result": "ERROR",
        "expected": args.expect,
        "validator": {
            "library": "python-jsonschema",
            "required_version": EXPECTED_JSONSCHEMA_VERSION,
            "draft": "https://json-schema.org/draft/2020-12/schema",
            "format_assertion": True,
            "python_version": platform.python_version(),
        },
        "schema_input": {"path_label": args.schema.name, "sha256": None, "bytes": None},
        "instance_input": {"path_label": args.instance.name, "sha256": None, "bytes": None},
        "errors": [],
        "limitations": [
            "Structural JSON Schema validation only; hashes, signatures, source rights, physical truth and authorization are not verified.",
            "Remote $ref values are prohibited; the schema must be self-contained and use local references only.",
            "This test-only validator does not make an SRQ release decision.",
        ],
    }

    try:
        installed = importlib.metadata.version("jsonschema")
        report["validator"]["installed_version"] = installed
        if installed != EXPECTED_JSONSCHEMA_VERSION:
            raise RuntimeError(
                f"validator version mismatch: expected {EXPECTED_JSONSCHEMA_VERSION}, found {installed}"
            )

        from jsonschema import Draft202012Validator, FormatChecker
        from jsonschema.exceptions import SchemaError

        schema, schema_raw = load_strict_json(args.schema, MAX_SCHEMA_BYTES)
        instance, instance_raw = load_strict_json(args.instance, MAX_INSTANCE_BYTES)
        report["schema_input"].update(sha256=digest(schema_raw), bytes=len(schema_raw))
        report["instance_input"].update(sha256=digest(instance_raw), bytes=len(instance_raw))

        if not isinstance(schema, dict):
            raise ValueError("schema root must be a JSON object")
        if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
            raise ValueError("schema must explicitly declare JSON Schema Draft 2020-12")
        remote_refs = find_remote_refs(schema)
        if remote_refs:
            raise ValueError("remote $ref is forbidden: " + "; ".join(remote_refs))
        checker = FormatChecker()
        missing_formats = sorted(find_formats(schema) - set(checker.checkers))
        if missing_formats:
            raise RuntimeError("format assertion unavailable for: " + ", ".join(missing_formats))
        try:
            Draft202012Validator.check_schema(schema)
        except SchemaError as exc:
            raise ValueError(f"schema does not validate against the Draft 2020-12 meta-schema: {exc.message}") from exc

        validator = Draft202012Validator(schema, format_checker=checker)
        errors = sorted(
            validator.iter_errors(instance),
            key=lambda exc: (pointer(exc.absolute_path), pointer(exc.absolute_schema_path), exc.message),
        )
        if errors:
            report["result"] = "INVALID"
            report["errors"] = [
                {
                    "instance_pointer": pointer(error.absolute_path),
                    "schema_pointer": pointer(error.absolute_schema_path),
                    "message": error.message,
                }
                for error in errors
            ]
        else:
            report["result"] = "VALID"
    except Exception as exc:  # Keep expected invalid instances distinct from validator failures.
        report["error"] = f"{type(exc).__name__}: {exc}"
        report["result"] = "ERROR"

    encoded = json.dumps(report, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    print(encoded, end="")

    if report["result"] == "ERROR":
        return 2
    matched = report["result"] == args.expect.upper()
    return 0 if matched else 1


if __name__ == "__main__":
    raise SystemExit(main())
