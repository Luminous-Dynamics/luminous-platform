#!/usr/bin/env python3
"""Validate global vendor adapter capability claims and fail-closed operation gates."""

from __future__ import annotations

import argparse
import re
import sys
from datetime import date
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from validate_hardware_portfolio import load_json

_SHA40 = re.compile(r"^[0-9a-f]{40}$")
_MATURITY = {
    "A0_DISCOVERED": 0,
    "A1_READ_ONLY_TESTED": 1,
    "A2_STATE_CONTRACT_TESTED": 2,
    "A3_CHANGE_QUALIFIED": 3,
    "A4_FIRMWARE_RECOVERY_QUALIFIED": 4,
    "A5_SERVICEABLE": 5,
}
_IMPLEMENTATION = {
    "NOT_IMPLEMENTED": 0,
    "PROTOTYPE_UNTESTED": 1,
    "LAB_TESTED": 2,
    "QUALIFIED": 3,
}
_HIGH_IMPACT = {
    "configuration_apply",
    "oem_firmware_update",
    "oem_firmware_recovery",
    "alternative_os_flash",
}
_RECOVERY_REQUIRED = {
    "configuration_apply",
    "oem_firmware_update",
    "oem_firmware_recovery",
    "alternative_os_flash",
}
_REQUIRED_RISK = {
    "inventory_read": "read_only",
    "health_read": "read_only",
    "version_read": "read_only",
    "configuration_export": "read_only",
    "entitlement_read": "read_only",
    "lifecycle_read": "read_only",
    "configuration_plan": "planning",
    "configuration_apply": "configuration_mutation",
    "oem_firmware_update": "disruptive_lifecycle",
    "oem_firmware_recovery": "disruptive_lifecycle",
    "alternative_os_flash": "reflash",
}


def registry_errors(
    registry: dict[str, Any],
    schema: dict[str, Any],
) -> list[str]:
    """Return schema and cross-field contract violations."""
    errors: list[str] = []
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    for error in sorted(
        validator.iter_errors(registry),
        key=lambda item: list(map(str, item.path)),
    ):
        where = "$" + "".join(
            f"[{part}]" if isinstance(part, int) else f".{part}"
            for part in error.absolute_path
        )
        errors.append(f"{where}: {error.message}")
    if errors:
        return errors

    try:
        date.fromisoformat(registry["as_of"])
    except (TypeError, ValueError):
        errors.append("$.as_of must be an ISO calendar date")

    catalog: dict[str, dict[str, Any]] = {}
    for index, operation in enumerate(registry["operation_catalog"]):
        op = operation["operation"]
        if op in catalog:
            errors.append(f"$.operation_catalog[{index}].operation duplicates {op!r}")
        catalog[op] = operation
        expected_risk = _REQUIRED_RISK[op]
        if operation["risk_class"] != expected_risk:
            errors.append(
                f"$.operation_catalog[{index}].risk_class for {op} must be {expected_risk}"
            )
        expected_retry = expected_risk in {"read_only", "planning"}
        if operation["automatic_retry_after_uncertain_outcome"] != expected_retry:
            errors.append(
                f"$.operation_catalog[{index}] retry policy for {op} violates "
                "the no-blind-replay contract"
            )

    seen_adapter_ids: set[str] = set()
    for index, adapter in enumerate(registry["adapters"]):
        prefix = f"$.adapters[{index}]"
        adapter_id = adapter["adapter_id"]
        if adapter_id in seen_adapter_ids:
            errors.append(f"{prefix}.adapter_id duplicates {adapter_id!r}")
        seen_adapter_ids.add(adapter_id)

        maturity = adapter["maturity"]
        implementation = adapter["implementation_state"]
        maturity_level = _MATURITY[maturity]
        implementation_level = _IMPLEMENTATION[implementation]
        maturity_evidence = adapter["maturity_evidence_refs"]

        if maturity_level == 0:
            if implementation != "NOT_IMPLEMENTED":
                errors.append(f"{prefix}: A0_DISCOVERED must remain NOT_IMPLEMENTED")
            if adapter["model_scope_state"] != "family_research_only":
                errors.append(f"{prefix}: A0_DISCOVERED must not claim exact-model qualification")
            if maturity_evidence:
                errors.append(f"{prefix}: A0_DISCOVERED must not carry maturity qualification evidence")
        else:
            if not maturity_evidence:
                errors.append(f"{prefix}: maturity above A0 requires maturity_evidence_refs")
            if adapter["model_scope_state"] == "family_research_only":
                errors.append(f"{prefix}: maturity above A0 requires exact-model scope")
            if implementation_level < _IMPLEMENTATION["LAB_TESTED"]:
                errors.append(f"{prefix}: maturity above A0 requires LAB_TESTED or QUALIFIED implementation")
            if adapter["model_scope_state"] != "exact_models_tested":
                errors.append(f"{prefix}: tested maturity requires exact_models_tested scope")

        tested_operations = {
            item["operation"] for item in adapter["operations"] if item["state"] == "tested"
        }
        if maturity_level >= _MATURITY["A1_READ_ONLY_TESTED"] and not (
            tested_operations & {"inventory_read", "health_read", "version_read", "configuration_export"}
        ):
            errors.append(f"{prefix}: A1 or higher requires at least one tested read-only operation")
        if maturity_level >= _MATURITY["A2_STATE_CONTRACT_TESTED"] and "configuration_plan" not in tested_operations:
            errors.append(f"{prefix}: A2 or higher requires a tested configuration_plan operation")
        if maturity_level >= _MATURITY["A3_CHANGE_QUALIFIED"] and "configuration_apply" not in tested_operations:
            errors.append(f"{prefix}: A3 or higher requires a tested configuration_apply operation")
        if maturity_level >= _MATURITY["A4_FIRMWARE_RECOVERY_QUALIFIED"] and not (
            tested_operations & {"oem_firmware_update", "oem_firmware_recovery"}
        ):
            errors.append(f"{prefix}: A4 or higher requires tested OEM firmware update/recovery capability")

        if implementation == "NOT_IMPLEMENTED":
            for op in adapter["operations"]:
                if op["state"] in {"implemented_untested", "tested"}:
                    errors.append(
                        f"{prefix}.operations[{op['operation']}]: implemented state contradicts NOT_IMPLEMENTED"
                    )
        if implementation == "QUALIFIED" and maturity != "A5_SERVICEABLE":
            errors.append(f"{prefix}: QUALIFIED implementation requires A5_SERVICEABLE maturity")
        if maturity == "A5_SERVICEABLE":
            if implementation != "QUALIFIED":
                errors.append(f"{prefix}: A5_SERVICEABLE requires QUALIFIED implementation")
            if adapter["region_scope_state"] not in {"region_scoped", "global_scope_assessed"}:
                errors.append(f"{prefix}: A5_SERVICEABLE requires assessed regional scope")

        seen_operations: set[str] = set()
        for op_index, operation in enumerate(adapter["operations"]):
            op_prefix = f"{prefix}.operations[{op_index}]"
            op_name = operation["operation"]
            state = operation["state"]
            if op_name in seen_operations:
                errors.append(f"{op_prefix}.operation duplicates {op_name!r}")
            seen_operations.add(op_name)
            if op_name not in catalog:
                errors.append(f"{op_prefix}.operation is absent from operation_catalog")
                continue

            risk = catalog[op_name]["risk_class"]
            if state == "unsupported" and not operation.get("rationale"):
                errors.append(f"{op_prefix}: unsupported operation needs a documented rationale")

            if state == "tested":
                if not operation.get("evidence_refs"):
                    errors.append(f"{op_prefix}: tested operation requires evidence_refs")
                if not _SHA40.fullmatch(operation.get("tested_revision", "")):
                    errors.append(f"{op_prefix}: tested operation requires a full 40-character tested_revision")
                if adapter["model_scope_state"] != "exact_models_tested":
                    errors.append(f"{op_prefix}: tested operation requires exact_models_tested adapter scope")
                if adapter["implementation_state"] not in {"LAB_TESTED", "QUALIFIED"}:
                    errors.append(f"{op_prefix}: tested operation requires LAB_TESTED or QUALIFIED implementation")
                if not operation.get("tested_scope"):
                    errors.append(f"{op_prefix}: tested operation requires exact model/revision/software/region scope")

            if risk in {"configuration_mutation", "disruptive_lifecycle", "reflash"}:
                auto_retry = catalog[op_name]["automatic_retry_after_uncertain_outcome"]
                if auto_retry:
                    errors.append(f"{op_prefix}: state-changing operation must never be blindly retried")
                if state in {"implemented_untested", "tested"}:
                    controls = operation.get("security_controls")
                    if not controls:
                        errors.append(f"{op_prefix}: state-changing operation requires security_controls")
                    else:
                        if controls.get("action_digest_bound_authorization") is not True:
                            errors.append(f"{op_prefix}: authorization must bind to the exact action digest")
                        if controls.get("fresh_prestate_required") is not True:
                            errors.append(f"{op_prefix}: mutation requires fresh pre-state binding")
                        if controls.get("uncertain_outcome_policy") != "NEVER_AUTOMATIC_REPLAY":
                            errors.append(f"{op_prefix}: uncertain outcomes must not trigger automatic replay")
                        if op_name in _RECOVERY_REQUIRED and not controls.get("recovery_evidence_refs"):
                            errors.append(f"{op_prefix}: operation requires tested recovery evidence")

            if op_name == "alternative_os_flash" and state in {"implemented_untested", "tested"}:
                if maturity_level < _MATURITY["A4_FIRMWARE_RECOVERY_QUALIFIED"]:
                    errors.append(f"{op_prefix}: alternative OS flashing requires A4 or A5 maturity")
                if adapter["model_scope_state"] != "exact_models_tested":
                    errors.append(f"{op_prefix}: alternative OS flashing requires exact model/revision testing")
                controls = operation.get("security_controls") or {}
                if controls.get("uncertain_outcome_policy") != "NEVER_AUTOMATIC_REPLAY":
                    errors.append(f"{op_prefix}: flashing must never automatically replay after an uncertain outcome")

        # A family-level discovery baseline must not imply that the adapter works.
        if maturity == "A0_DISCOVERED" and any(
            operation["state"] not in {"planned", "unsupported"}
            for operation in adapter["operations"]
        ):
            errors.append(f"{prefix}: A0_DISCOVERED permits planned/explicitly unsupported operations only")

    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--registry",
        type=Path,
        default=Path("security/vendor-adapter-capability-registry-v1.json"),
    )
    parser.add_argument(
        "--schema",
        type=Path,
        default=Path("schemas/vendor-adapter-capability-registry-v1.schema.json"),
    )
    args = parser.parse_args(argv)

    try:
        registry = load_json(args.registry)
        schema = load_json(args.schema)
    except ValueError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    if not isinstance(registry, dict) or not isinstance(schema, dict):
        print("FAIL: registry and schema roots must be JSON objects", file=sys.stderr)
        return 1

    failures = registry_errors(registry, schema)
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1

    print(
        "PASS: vendor adapter capability contract is structurally valid; "
        "no adapter maturity, device qualification or deployment authorization is implied."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
