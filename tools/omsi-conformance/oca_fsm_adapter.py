"""Read-only OCA Field Service 19.0 snapshot -> OMSI intake projection.

This consumes synthetic, version-pinned exports. It never calls an Odoo server,
writes records, authenticates a real credential, or produces authorization or
verified-work claims.
"""
from __future__ import annotations
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from jsonschema import Draft202012Validator, FormatChecker
import omsi_validate as core

ROOT = Path(__file__).resolve().parent
SCHEMAS = {
    "source": ROOT / "schemas/0.1/oca-fsm-order-snapshot.schema.json",
    "binding": ROOT / "schemas/0.1/oca-tenant-binding.schema.json",
    "stage_map": ROOT / "schemas/0.1/oca-stage-map.schema.json",
    "intake": ROOT / "schemas/0.1/oca-field-service-intake.schema.json",
}
PINNED_REPOSITORY_COMMIT = "8b2fa2592dda401726834ccf8030d34e241a69ad"
PINNED_CORE_ADDON_VERSION = "19.0.1.2.3"
PINNED_OPTIONAL_ADDONS = {
    "fieldservice_activity": ("19.0.1.0.1", "AGPL-3"),
    "fieldservice_account": ("19.0.1.0.0", "AGPL-3"),
}
ACTIVE_STATUSES = {
    "requested", "assessed", "awaiting_approval", "approved", "scheduled",
    "dispatched", "on_site", "diagnosed", "in_progress", "quality_checked",
}


def _fail(message: str) -> None:
    raise core.ConformanceError(message)


def _validate_schema(kind: str, value: Any) -> None:
    schema = core.load(SCHEMAS[kind])
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(value),
        key=lambda error: tuple(map(str, error.absolute_path)),
    )
    if errors:
        error = errors[0]
        pointer = "/" + "/".join(str(part) for part in error.absolute_path)
        _fail(f"OCA adapter {kind} schema error at {pointer}: {error.message}")


def _timestamp(value: str | None) -> datetime | None:
    return core.parse_time(value) if value is not None else None


def _source_ref(instance: str, model: str, record_id: str) -> str:
    return f"{instance}|{model}|{record_id}"


def map_order(
    snapshot: dict[str, Any],
    binding: dict[str, Any],
    stage_map: dict[str, Any],
    now: datetime,
) -> dict[str, Any]:
    """Return an intake projection, not a fully qualified service record."""
    if now.tzinfo is None:
        _fail("OCA mapping evaluation time must include a timezone")
    now = now.astimezone(timezone.utc)
    _validate_schema("source", snapshot)
    _validate_schema("binding", binding)
    _validate_schema("stage_map", stage_map)
    source, order = snapshot["source"], snapshot["order"]
    instance, stage = source["instance_namespace"], order["stage"]

    if source["repository_commit"] != PINNED_REPOSITORY_COMMIT:
        _fail("unsupported OCA repository commit; review and pin a new source profile")
    if source["core_addon_version"] != PINNED_CORE_ADDON_VERSION:
        _fail("unsupported OCA fieldservice addon version; requalify adapter mapping")
    if stage_map["source_commit"] != source["repository_commit"]:
        _fail("stage-map source commit does not match the source snapshot")

    present_addons: dict[str, dict[str, str]] = {}
    for addon in source["optional_addons"]:
        name = addon["name"]
        if name in present_addons:
            _fail("duplicate optional addon declaration")
        present_addons[name] = addon
        if (addon["version"], addon["license"]) != PINNED_OPTIONAL_ADDONS[name]:
            _fail(f"unsupported OCA optional addon version or license: {name}")

    activity_present = "fieldservice_activity" in present_addons
    invoice_present = "fieldservice_account" in present_addons
    if snapshot["activity_addon_state"] == "not_installed":
        if activity_present or snapshot["activities"]:
            _fail("activity addon state conflicts with source module manifest")
    elif snapshot["activity_addon_state"] == "exported":
        if not activity_present:
            _fail("activity data marked exported but activity addon is absent")
    elif snapshot["activity_addon_state"] == "unavailable":
        if not activity_present or snapshot["activities"]:
            _fail("activity data unavailable state conflicts with snapshot")

    if snapshot["invoice_addon_state"] == "not_installed":
        if invoice_present or snapshot["invoice_refs"]:
            _fail("invoice addon state conflicts with source module manifest")
    elif snapshot["invoice_addon_state"] == "exported":
        if not invoice_present:
            _fail("invoice links marked exported but accounting addon is absent")
    elif snapshot["invoice_addon_state"] == "unavailable":
        if not invoice_present or snapshot["invoice_refs"]:
            _fail("invoice data unavailable state conflicts with snapshot")

    if binding["status"] != "active":
        _fail("OCA tenant binding is not active")
    if binding["source_instance_namespace"] != instance:
        _fail("OCA tenant binding source instance mismatch")
    if binding["source_company_id"] != order["company_id"]:
        _fail("OCA company does not match explicit tenant binding")
    if _timestamp(binding["configured_at"]) > now:
        _fail("OCA tenant binding configuration is dated in the future")
    if _timestamp(order["write_date"]) > now:
        _fail("OCA source revision is dated in the future")

    request_early, request_late = _timestamp(order["request_early"]), _timestamp(order["request_late"])
    scheduled_start, scheduled_end = _timestamp(order["scheduled_date_start"]), _timestamp(order["scheduled_date_end"])
    actual_start, actual_end = _timestamp(order["date_start"]), _timestamp(order["date_end"])
    if request_early and request_late and request_early > request_late:
        _fail("OCA request window ends before it starts")
    if scheduled_start and scheduled_end and scheduled_start > scheduled_end:
        _fail("OCA scheduled window ends before it starts")
    if actual_start and actual_end and actual_start > actual_end:
        _fail("OCA actual work interval ends before it starts")

    entries = stage_map["entries"]
    ids = [entry["stage_id"] for entry in entries]
    if len(ids) != len(set(ids)):
        _fail("duplicate source stage ID in stage-map configuration")
    matching = [entry for entry in entries if entry["stage_id"] == stage["id"]]
    mapping_losses: set[str] = {
        "jurisdiction_not_present_in_order_snapshot",
        "professional_credential_status_not_present_in_source",
        "estimate_and_customer_scope_approval_not_present_in_source",
        "authorized_operation_manifest_not_present_in_source",
        "independent_verification_not_present_in_source",
        "source_event_history_not_exported",
        "invoice_payment_status_not_established",
    }
    mapping_conflicts: set[str] = set()
    if not matching:
        normalized_status = "unmapped"
        mapping_losses.add("source_stage_unmapped")
    else:
        entry = matching[0]
        if (
            entry["source_name"] != stage["name"]
            or entry["stage_type"] != stage["stage_type"]
            or entry["source_is_closed"] != stage["is_closed"]
        ):
            _fail("stage mapping fingerprint conflicts with source stage")
        normalized_status = entry["normalized_status"]
        if stage["is_closed"] and normalized_status in ACTIVE_STATUSES:
            _fail("closed source stage cannot map to an active normalized status")
        if not stage["is_closed"] and normalized_status in {"closed", "invoiced"}:
            _fail("open source stage cannot map to a closed normalized status")

    activity_state = snapshot["activity_addon_state"]
    required = [activity for activity in snapshot["activities"] if activity["required"]]
    if activity_state == "not_installed":
        required_state = "addon_not_installed"
        mapping_losses.add("activity_addon_not_installed")
    elif activity_state == "unavailable":
        required_state = "addon_unavailable"
        mapping_losses.add("activity_snapshot_unavailable")
    elif not required:
        required_state = "none_reported"
    elif any(activity["state"] != "done" for activity in required):
        required_state = "required_activity_incomplete"
    else:
        required_state = "all_complete"

    if required_state == "required_activity_incomplete" and stage["is_closed"]:
        mapping_conflicts.add("source_stage_closed_with_required_activity_incomplete")
    for activity in snapshot["activities"]:
        if activity["state"] == "done" and (
            activity["completed_on"] is None or activity["completed_by"] is None
        ):
            mapping_conflicts.add("completed_activity_missing_actor_or_time")

    signature = order["signature_present"]
    if stage["require_signature"] and not signature:
        mapping_conflicts.add("source_stage_requires_signature_but_signature_absent")
    if signature and (not order["signed_by_present"] or order["signed_on"] is None):
        mapping_conflicts.add("signature_presence_lacks_signer_or_time_metadata")
    if order["description_present"] or order["todo_present"] or order["resolution_present"]:
        mapping_losses.add("raw_html_content_omitted_by_privacy_policy")
    if signature:
        mapping_losses.add("raw_signature_payload_omitted_by_privacy_policy")

    invoice_state = snapshot["invoice_addon_state"]
    if invoice_state == "not_installed":
        invoice_link_state = "not_installed"
        mapping_losses.add("invoice_addon_not_installed")
    elif invoice_state == "unavailable":
        invoice_link_state = "unavailable"
        mapping_losses.add("invoice_link_snapshot_unavailable")
    elif snapshot["invoice_refs"]:
        invoice_link_state = "linked_references_present"
    else:
        invoice_link_state = "no_invoice_references"

    if (actual_start is None) != (actual_end is None):
        mapping_conflicts.add("partial_actual_work_interval")

    output = {
        "schema": "org.omsi.field-service-intake/0.1",
        "id": _source_ref(instance, "fsm.order", order["source_record_id"]),
        "tenant_id": binding["tenant_id"],
        "source": {
            "system": source["system"], "instance_namespace": instance, "model": "fsm.order",
            "record_id": order["source_record_id"], "display_name": order["name"],
            "revision": order["write_date"], "repository_commit": source["repository_commit"],
            "core_addon_version": source["core_addon_version"], "core_addon_license": source["core_addon_license"],
            "company_id": order["company_id"], "binding_id": binding["binding_id"],
        },
        "service_case": {
            "source_stage_id": stage["id"], "source_stage_name": stage["name"], "source_stage_type": stage["stage_type"],
            "source_is_closed": stage["is_closed"], "normalized_status": normalized_status,
            "mapping_profile": stage_map["profile_id"], "mapping_losses": sorted(mapping_losses),
            "mapping_conflicts": sorted(mapping_conflicts), "mapping_loss_disclosed": True,
        },
        "work_order": {
            "source_order_id": order["source_record_id"],
            "location_ref": _source_ref(instance, "fsm.location", order["location_id"]),
            "worker_refs": [
                _source_ref(instance, "fsm.person", worker_id)
                for worker_id in sorted(set(order["person_ids"]) | ({order["person_id"]} if order["person_id"] else set()))
            ],
            "team_ref": _source_ref(instance, "fsm.team", order["team_id"]) if order["team_id"] else None,
            "equipment_refs": [_source_ref(instance, "fsm.equipment", equipment_id) for equipment_id in sorted(order["equipment_ids"])],
            "priority_source_value": order["priority"],
            "request_window": {"earliest": order["request_early"], "latest": order["request_late"]},
            "scheduled_window": {"start": order["scheduled_date_start"], "end": order["scheduled_date_end"]},
            "actual_interval": {"start": order["date_start"], "end": order["date_end"]},
        },
        "source_claims": {
            "stage_says_closed": stage["is_closed"], "resolution_present": order["resolution_present"],
            "customer_signature_present": signature, "customer_signature_timestamp": order["signed_on"],
            "required_activities_state": required_state, "activity_addon_state": activity_state,
            "invoice_link_state": invoice_link_state,
            "invoice_refs": [_source_ref(instance, "account.move", invoice_id) for invoice_id in sorted(snapshot["invoice_refs"])],
        },
        "assurance": {
            "customer_scope_approval": "not_provided_by_source",
            "practitioner_credential_status": "not_provided_by_source",
            "authorized_operation": "not_provided_by_source",
            "independent_verification": "not_provided_by_source",
            "workmanship_quality": "not_verified", "invoice_payment": "not_established",
        },
        "privacy": {
            "raw_signature_exported": False, "raw_html_exported": False, "precise_contact_exported": False,
        },
    }
    _validate_schema("intake", output)
    return output
