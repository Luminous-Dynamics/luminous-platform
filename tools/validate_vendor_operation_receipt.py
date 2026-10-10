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
    "FAILED": {"NO_EFFECT", "EFFECT_REJECTED"},
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
    if event_states[0] != "PREPARED":
        errors.append("$.journal must begin with PREPARED")
    if event_states[-1] != state:
        errors.append("$.state must equal the last journal event state")

    if created:
        previous_time = None
        for index, event in enumerate(journal):
            event_time = parse_time(event["occurred_at"], f"$.journal[{index}].occurred_at", errors)
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
    args = parser.parse_args(argv)

    try:
        receipt = load_json(args.receipt)
        schema = load_json(args.schema)
    except ValueError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    if not isinstance(receipt, dict) or not isinstance(schema, dict):
        print("FAIL: receipt and schema roots must be JSON objects", file=sys.stderr)
        return 1

    failures = receipt_errors(receipt, schema)
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1

    print(
        "PASS: operation effect receipt contract is structurally valid; "
        "a PREPARED receipt is not authorization to mutate a device."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
