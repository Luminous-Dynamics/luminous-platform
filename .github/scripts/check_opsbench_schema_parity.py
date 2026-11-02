#!/usr/bin/env python3
"""Check strict JSON schema fields against the Rust record structs.

CI-only source-of-truth guard. This is not production runtime code and has no
third-party dependencies. Each mapped record must have an exact field-set match
and every declared field must be required for these immutable evidence records.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "research" / "continuous-opsbench-v0.1"


def rust_fields(source: str, struct_name: str) -> set[str]:
    pattern = re.compile(
        rf"pub struct {re.escape(struct_name)}\s*\{{(.*?)^\}}",
        re.MULTILINE | re.DOTALL,
    )
    match = pattern.search(source)
    if not match:
        raise ValueError(f"Rust struct not found: {struct_name}")
    fields = set(re.findall(r"^\s*pub\s+([A-Za-z_][A-Za-z0-9_]*)\s*:", match.group(1), re.MULTILINE))
    if not fields:
        raise ValueError(f"No public fields found for Rust struct: {struct_name}")
    return fields


def schema_record(document: dict, pointer: tuple[str, ...], label: str) -> dict:
    value = document
    for component in pointer:
        try:
            value = value[component]
        except (KeyError, TypeError) as exc:
            raise ValueError(f"{label}: missing schema path {'/'.join(pointer)}") from exc
    if not isinstance(value, dict):
        raise ValueError(f"{label}: schema record is not an object")
    if value.get("type") != "object" or value.get("additionalProperties") is not False:
        raise ValueError(f"{label}: record must be an object with additionalProperties=false")
    properties = value.get("properties")
    required = value.get("required")
    if not isinstance(properties, dict) or not isinstance(required, list):
        raise ValueError(f"{label}: missing properties/required")
    property_names = set(properties)
    required_names = set(required)
    if property_names != required_names:
        raise ValueError(
            f"{label}: immutable record must require every field; "
            f"optional={sorted(property_names - required_names)}, "
            f"unknown_required={sorted(required_names - property_names)}"
        )
    return value


MAPPINGS = [
    ("runtime.rs", "RuntimeChallengeReceipt", "runtime-challenge-receipt.schema.json", ()),
    ("runtime.rs", "RuntimeTrialAttestation", "runtime-trial-attestation.schema.json", ()),
    ("sealed.rs", "RunManifest", "sealed-run-manifest.schema.json", ("$defs", "manifest")),
    ("sealed.rs", "FreezeReceipt", "sealed-run-manifest.schema.json", ("$defs", "freeze_receipt")),
    ("sealed.rs", "FrozenAgentIdentity", "sealed-run-manifest.schema.json", ("$defs", "agent")),
    ("sealed.rs", "BaselineIdentity", "sealed-run-manifest.schema.json", ("$defs", "baseline")),
    ("sealed.rs", "PlannedTrial", "sealed-run-manifest.schema.json", ("$defs", "trial")),
    ("sealed.rs", "ScenarioReceipt", "sealed-run-evidence.schema.json", ("$defs", "scenario_receipt")),
    ("sealed.rs", "ScenarioReveal", "sealed-run-evidence.schema.json", ("$defs", "scenario_reveal")),
    ("sealed.rs", "TrialEvent", "sealed-run-evidence.schema.json", ("$defs", "trial_event")),
]


def main() -> int:
    rust_sources = {
        "runtime.rs": (BASE / "src" / "runtime.rs").read_text(encoding="utf-8"),
        "sealed.rs": (BASE / "src" / "sealed.rs").read_text(encoding="utf-8"),
    }
    schemas: dict[str, dict] = {}
    errors: list[str] = []
    for _, _, schema_name, _ in MAPPINGS:
        if schema_name not in schemas:
            path = BASE / schema_name
            try:
                schemas[schema_name] = json.loads(path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError) as exc:
                errors.append(f"{schema_name}: cannot parse schema: {exc}")

    for source_name, struct_name, schema_name, pointer in MAPPINGS:
        if schema_name not in schemas:
            continue
        label = f"{source_name}:{struct_name}"
        try:
            rust = rust_fields(rust_sources[source_name], struct_name)
            record = schema_record(schemas[schema_name], pointer, label)
            schema = set(record["properties"])
            if rust != schema:
                errors.append(
                    f"{label}: Rust/schema field mismatch; "
                    f"missing_in_schema={sorted(rust - schema)}, "
                    f"missing_in_rust={sorted(schema - rust)}"
                )
            else:
                print(f"parity OK: {label} ({len(rust)} fields)")
        except ValueError as exc:
            errors.append(str(exc))

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"validated {len(MAPPINGS)} Rust/schema record mappings")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
