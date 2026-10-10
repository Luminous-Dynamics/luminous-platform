#!/usr/bin/env python3
"""Validate a mutation receipt's authorization binding and durable effect journal."""

from __future__ import annotations

import argparse
import sys
from datetime import datetime
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from validate_hardware_portfolio import load_json
from validate_vendor_adapter_registry import registry_errors as vendor_registry_errors

_APPLY_STATES = {
    "APPLY_STARTED",
    "APPLIED_UNVERIFIED",
    "VERIFIED",
    "INDETERMINATE",
}
_EFFECT_FOR_STATE = {
    "PREPARED": {"NO_EFFECT"},
    "AUTHORIZATION_VERIFIED": {"NO_EFFECT"},
    "APPLY_STARTED": {"EFFECT_POSSIBLE"},
    "APPLIED_UNVERIFIED": {"EFFECT_APPLIED"},
    "VERIFIED": {"EFFECT_VERIFIED"},
    "FAILED": {"NO_EFFECT", "EFFECT_REJECTED", "EFFECT_PARTIAL"},
    "INDETERMINATE": {"EFFECT_INDETERMINATE"},
    "ABORTED": {"NO_EFFECT"},
}
_TRANSITIONS = {
    "PREPARED": {"AUTHORIZATION_VERIFIED", "FAILED", "ABORTED"},
    "AUTHORIZATION_VERIFIED": {"APPLY_STARTED", "FAILED", "ABORTED"},
    "APPLY_STARTED": {"APPLIED_UNVERIFIED", "FAILED", "INDETERMINATE"},
    "APPLIED_UNVERIFIED": {"VERIFIED", "FAILED", "INDETERMINATE"},
    # Reconciliation after uncertainty is read-only. Re-applying is a new
    # separately authorized operation, never a transition in the same receipt.
    "INDETERMINATE": {"APPLIED_UNVERIFIED", "VERIFIED", "FAILED"},
    "VERIFIED": set(),
    "FAILED": set(),
    "ABORTED": set(),
}


def parse_time(value: str, label: str, errors: list[str]) -> datetime | None:
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except (TypeError, ValueError):
        errors.append(f"{label} must be an ISO-8601 date-time")
        return None
    if parsed.tzinfo is None:
        errors.append(f"{label} must include an explicit timezone offset")
        return None
    return parsed


def receipt_errors(receipt: dict[str, Any], schema: dict[str, Any]) -> list[str]:
    """Return schema and cross-field/effect-journal violations."""
    errors: list[str] = []
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    for error in sorted(
        validator.iter_errors(receipt),
        key=lambda item: list(map(str, item.path)),
    ):
        where = "$" + "".join(
            f"[{part}]" if isinstance(part, int) else f".{part}"
            for part in error.absolute_path
        )
        errors.append(f"{where}: {error.message}")
    if errors:
        return errors

    created = parse_time(receipt["created_at"], "$.created_at", errors)
    prestate_at = parse_time(
        receipt["intent"]["prestate_observed_at"],
        "$.intent.prestate_observed_at",
        errors,
    )
    auth = receipt["authorization"]
    intent = receipt["intent"]
    state = receipt["state"]
    journal = receipt["journal"]
    recovery = receipt["recovery"]
    evidence = receipt["evidence"]

    if receipt["target"]["identity_digest"] != intent["target_digest"]:
        errors.append("$.intent.target_digest must match $.target.identity_digest")

    if auth["state"] == "verified":
        if not auth["signature_evidence_refs"]:
            errors.append("$.authorization verified state requires signature evidence")
        if auth["authorized_action_digest"] != intent["action_digest"]:
            errors.append("$.authorization must bind to the exact intent.action_digest")
        if auth["authorized_target_digest"] != intent["target_digest"]:
            errors.append("$.authorization must bind to the exact intent.target_digest")
        if auth["authorized_prestate_digest"] != intent["prestate_digest"]:
            errors.append("$.authorization must bind to the exact intent.prestate_digest")
        if auth["authorized_policy_revision"] != intent["policy_revision"]:
            errors.append("$.authorization must bind to the exact intent.policy_revision")
        if auth["authorized_profile_digest"] != intent["profile_digest"]:
            errors.append("$.authorization must bind to the exact intent.profile_digest")
        if auth["requested_by_ref"] in auth["approver_refs"]:
            errors.append("$.authorization requester and approver must be distinct principals")
        verified_at = parse_time(auth["verified_at"], "$.authorization.verified_at", errors)
        expires_at = parse_time(auth["expires_at"], "$.authorization.expires_at", errors)
        if prestate_at and verified_at and verified_at < prestate_at:
            errors.append("$.authorization.verified_at cannot precede the pre-state observation")
        if verified_at and expires_at and expires_at <= verified_at:
            errors.append("$.authorization.expires_at must be after verified_at")
    else:
        verified_at = None
        expires_at = parse_time(auth["expires_at"], "$.authorization.expires_at", errors)
        if any(item["state"] in _APPLY_STATES for item in journal):
            errors.append("$.authorization must be verified before APPLY_STARTED or any later effect state")

    event_states = [item["state"] for item in journal]
    if auth["state"] == "verified" and "AUTHORIZATION_VERIFIED" in event_states:
        auth_event = next(item for item in journal if item["state"] == "AUTHORIZATION_VERIFIED")
        auth_event_time = parse_time(
            auth_event["occurred_at"],
            "$.journal[AUTHORIZATION_VERIFIED].occurred_at",
            errors,
        )
        if created and auth_event_time and auth_event_time < created:
            errors.append("$.journal AUTHORIZATION_VERIFIED cannot predate receipt creation")
        if auth_event_time and verified_at and auth_event_time != verified_at:
            errors.append("$.authorization.verified_at must match the AUTHORIZATION_VERIFIED journal timestamp")
    if auth["state"] == "verified" and "AUTHORIZATION_VERIFIED" not in event_states:
        errors.append("$.authorization verified state must have an AUTHORIZATION_VERIFIED journal event")
    if "AUTHORIZATION_VERIFIED" in event_states and auth["state"] != "verified":
        errors.append("$.journal AUTHORIZATION_VERIFIED event requires verified authorization")
    if event_states[0] != "PREPARED":
        errors.append("$.journal must begin with PREPARED")
    if event_states[-1] != state:
        errors.append("$.state must equal the last journal event state")

    if created:
        if prestate_at and prestate_at > created:
            errors.append("$.intent.prestate_observed_at cannot be later than receipt creation")
        previous_time = None
        for index, event in enumerate(journal):
            event_time = parse_time(event["occurred_at"], f"$.journal[{index}].occurred_at", errors)
            if event_time and event_time < created:
                errors.append(f"$.journal[{index}].occurred_at cannot predate receipt creation")
            if event_time and previous_time and event_time < previous_time:
                errors.append("$.journal timestamps must be monotonically non-decreasing")
            if event_time:
                previous_time = event_time
            if index and event["state"] not in _TRANSITIONS[event_states[index - 1]]:
                errors.append(
                    f"$.journal[{index}] invalid transition "
                    f"{event_states[index - 1]} -> {event['state']}"
                )
            if event["state"] == "APPLY_STARTED":
                if event.get("prestate_digest") != intent["prestate_digest"]:
                    errors.append(f"$.journal[{index}].prestate_digest must revalidate the authorized pre-state")
                if auth["state"] != "verified":
                    errors.append("$.authorization must be verified before APPLY_STARTED")
                if event_time and verified_at and event_time < verified_at:
                    errors.append(f"$.journal[{index}] APPLY_STARTED cannot predate authorization")
                if event_time and expires_at and event_time >= expires_at:
                    errors.append(f"$.journal[{index}] approval was expired at APPLY_STARTED")
                if not recovery["recovery_evidence_refs"]:
                    errors.append("$.recovery.recovery_evidence_refs are required before any mutation starts")
                if receipt["operation"] == "alternative_os_flash" and not recovery["known_good_image_ref"]:
                    errors.append("$.recovery.known_good_image_ref is required for alternative_os_flash")
        if len(event_states) > 1 and event_states.count("APPLY_STARTED") > 1:
            errors.append("$.journal cannot repeat APPLY_STARTED; use read-only reconciliation, not blind replay")

    allowed_effects = _EFFECT_FOR_STATE[state]
    if receipt["effect_state"] not in allowed_effects:
        errors.append(
            f"$.effect_state for state {state} must be one of {sorted(allowed_effects)}"
        )

    if state == "VERIFIED":
        if not evidence["raw_artifact_refs"]:
            errors.append("$.evidence.raw_artifact_refs are required for VERIFIED")
        if not evidence["independent_verifier_ref"]:
            errors.append("$.evidence.independent_verifier_ref is required for VERIFIED")
        if evidence["independent_verifier_ref"] in {event["actor_ref"] for event in journal}:
            errors.append("$.evidence.independent_verifier_ref must be independent of the executing actors")
        if evidence["verified_state_digest"] != intent["desired_state_digest"]:
            errors.append("$.evidence.verified_state_digest must match intent.desired_state_digest")
        verified_event = journal[-1]
        if verified_event["state"] == "VERIFIED" and not verified_event["evidence_refs"]:
            errors.append("$.journal VERIFIED event must reference evidence")
        if not recovery["recovery_evidence_refs"]:
            errors.append("$.recovery.recovery_evidence_refs are required for a verified mutation")

    if state == "FAILED":
        failed_event = journal[-1]
        prior_states = event_states[:-1]
        if "APPLIED_UNVERIFIED" in prior_states and event_states[-2] == "APPLIED_UNVERIFIED":
            if receipt["effect_state"] != "EFFECT_PARTIAL":
                errors.append(
                    "a direct FAILED transition from APPLIED_UNVERIFIED must record EFFECT_PARTIAL, not NO_EFFECT or EFFECT_REJECTED"
                )
            if not failed_event["evidence_refs"]:
                errors.append("FAILED after APPLIED_UNVERIFIED requires evidence of the partial effect and remediation state")
        elif any(item in _APPLY_STATES for item in prior_states) and not failed_event["evidence_refs"]:
            errors.append("FAILED after an effect may have started requires reconciliation/rejection evidence")
        if receipt["effect_state"] == "EFFECT_PARTIAL" and not failed_event["evidence_refs"]:
            errors.append("EFFECT_PARTIAL requires evidence references on the terminal FAILED event")

    if state == "INDETERMINATE" and receipt["effect_state"] != "EFFECT_INDETERMINATE":
        errors.append("indeterminate state must never be described as successful or safely failed")

    if receipt["operation"] in {"oem_firmware_update", "oem_firmware_recovery", "alternative_os_flash"}:
        if not recovery["recovery_plan_ref"]:
            errors.append("$.recovery.recovery_plan_ref is mandatory for firmware operations")
        if state in _APPLY_STATES and not recovery["recovery_evidence_refs"]:
            errors.append("$.recovery.recovery_evidence_refs are mandatory before firmware effects")

    if state == "ABORTED" and receipt["effect_state"] != "NO_EFFECT":
        errors.append("ABORTED is only valid when no effect was applied")

    return errors



_REQUIRED_MATURITY = {
    "configuration_apply": "A3_CHANGE_QUALIFIED",
    "oem_firmware_update": "A4_FIRMWARE_RECOVERY_QUALIFIED",
    "oem_firmware_recovery": "A4_FIRMWARE_RECOVERY_QUALIFIED",
    "alternative_os_flash": "A4_FIRMWARE_RECOVERY_QUALIFIED",
}
_MATURITY_ORDER = {
    "A0_DISCOVERED": 0,
    "A1_READ_ONLY_TESTED": 1,
    "A2_STATE_CONTRACT_TESTED": 2,
    "A3_CHANGE_QUALIFIED": 3,
    "A4_FIRMWARE_RECOVERY_QUALIFIED": 4,
    "A5_SERVICEABLE": 5,
}


def registry_binding_errors(receipt: dict[str, Any], registry: dict[str, Any]) -> list[str]:
    """Require any authorized/effectful receipt to match the exact tested registry capability."""
    errors: list[str] = []
    adapters = {item["adapter_id"]: item for item in registry.get("adapters", [])}
    catalog = {item["operation"]: item for item in registry.get("operation_catalog", [])}
    adapter_id = receipt["adapter_id"]
    operation_name = receipt["operation"]
    adapter = adapters.get(adapter_id)
    catalog_operation = catalog.get(operation_name)

    if adapter is None:
        return [f"$.adapter_id {adapter_id!r} is absent from the capability registry"]
    if catalog_operation is None:
        return [f"$.operation {operation_name!r} is absent from the registry operation catalog"]

    adapter_operation = next(
        (item for item in adapter.get("operations", []) if item["operation"] == operation_name),
        None,
    )
    if adapter_operation is None:
        return [f"$.operation {operation_name!r} is not declared by adapter {adapter_id!r}"]

    target = receipt["target"]
    if target["vendor"] != adapter["vendor"]:
        errors.append("$.target.vendor must match the declared adapter vendor")
    if target["product_family"] not in adapter["product_families"]:
        errors.append("$.target.product_family must be explicitly declared by the adapter")

    if adapter_operation["state"] == "unsupported":
        errors.append("$.operation cannot be prepared or authorized for a registry-declared unsupported capability")

    journal = receipt["journal"]
    auth_verified = receipt["authorization"]["state"] == "verified"
    effect_may_have_started = any(event["state"] in _APPLY_STATES for event in journal)
    needs_tested_scope = auth_verified or effect_may_have_started

    if not needs_tested_scope:
        return errors

    if adapter_operation["state"] != "tested":
        errors.append(
            "$.operation requires registry state=tested before authorization or any possible device effect"
        )
        return errors

    expected_maturity = _REQUIRED_MATURITY[operation_name]
    if _MATURITY_ORDER[adapter["maturity"]] < _MATURITY_ORDER[expected_maturity]:
        errors.append(
            f"$.adapter_id requires at least {expected_maturity} for {operation_name}"
        )
    if adapter["implementation_state"] not in {"LAB_TESTED", "QUALIFIED"}:
        errors.append("$.adapter_id requires a LAB_TESTED or QUALIFIED implementation before authorization")
    if adapter["model_scope_state"] != "exact_models_tested":
        errors.append("$.adapter_id requires exact_models_tested scope before authorization")

    tested_scope = adapter_operation.get("tested_scope") or {}
    target = receipt["target"]
    scope_fields = {
        "model": "model",
        "product_family": "product_family",
        "hardware_revision": "hardware_revision",
        "software_version": "software_version",
        "region": "region",
    }
    for scope_key, target_key in scope_fields.items():
        if tested_scope.get(scope_key) != target.get(target_key):
            errors.append(
                f"$.target.{target_key} must exactly match registry tested_scope.{scope_key}"
            )

    if receipt.get("adapter_revision") != adapter_operation.get("tested_revision"):
        errors.append(
            "$.adapter_revision must match the registry tested_revision for the exact operation"
        )
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--receipt",
        type=Path,
        default=Path("tests/fixtures/vendor-operation-effect-receipt-prepared-v1.json"),
    )
    parser.add_argument(
        "--schema",
        type=Path,
        default=Path("schemas/vendor-operation-effect-receipt-v1.schema.json"),
    )
    parser.add_argument(
        "--registry",
        type=Path,
        default=Path("security/vendor-adapter-capability-registry-v1.json"),
    )
    parser.add_argument(
        "--registry-schema",
        type=Path,
        default=Path("schemas/vendor-adapter-capability-registry-v1.schema.json"),
    )
    args = parser.parse_args(argv)

    try:
        receipt = load_json(args.receipt)
        schema = load_json(args.schema)
        registry = load_json(args.registry)
        registry_schema = load_json(args.registry_schema)
    except ValueError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    if not all(isinstance(item, dict) for item in (receipt, schema, registry, registry_schema)):
        print("FAIL: receipt, receipt schema, registry and registry schema roots must be JSON objects", file=sys.stderr)
        return 1

    registry_failures = vendor_registry_errors(registry, registry_schema)
    if registry_failures:
        for failure in registry_failures:
            print(f"FAIL: registry: {failure}", file=sys.stderr)
        return 1

    failures = receipt_errors(receipt, schema)
    failures.extend(registry_binding_errors(receipt, registry))
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1

    print(
        "PASS: operation effect receipt and registry binding are structurally valid; "
        "a PREPARED receipt is not authorization to mutate a device."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
