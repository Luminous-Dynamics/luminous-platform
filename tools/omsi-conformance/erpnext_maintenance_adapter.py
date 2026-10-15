"""Read-only ERPNext Asset Maintenance Log to OMSI intake projection."""
from __future__ import annotations
from datetime import date, datetime, timezone
from pathlib import Path
from typing import Any
from urllib.parse import quote
from zoneinfo import ZoneInfo, ZoneInfoNotFoundError
from jsonschema import Draft202012Validator, FormatChecker
import omsi_validate as core

ROOT = Path(__file__).resolve().parent
SCHEMAS = {
    "snapshot": ROOT / "schemas/0.1/erpnext-maintenance-log-snapshot.schema.json",
    "binding": ROOT / "schemas/0.1/erpnext-tenant-binding.schema.json",
    "intake": ROOT / "schemas/0.1/erpnext-maintenance-intake.schema.json",
}
PINNED_RELEASE = "v16.51.0"
PINNED_COMMIT = "fd97695de7bfeb1d7728a7370c3e128d0cc1509e"
PINNED_LICENSE = "GPL-3.0"
STATUS_MAP = {"Planned": "scheduled", "Completed": "closed", "Cancelled": "cancelled", "Overdue": "overdue"}


def _fail(message: str) -> None:
    raise core.ConformanceError(message)


def _validate(kind: str, value: Any) -> None:
    schema = core.load(SCHEMAS[kind])
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(value),
        key=lambda error: tuple(map(str, error.absolute_path)),
    )
    if errors:
        error = errors[0]
        pointer = "/" + "/".join(str(part).replace("~", "~0").replace("/", "~1") for part in error.absolute_path)
        _fail(f"ERPNext adapter {kind} schema error at {pointer}: {error.message}")


def _ref(instance: str, doctype: str, name: str) -> str:
    # Escape source-controlled components so the delimiter cannot make IDs ambiguous.
    encoded_instance = quote(instance, safe=":/.-_~")
    encoded_name = quote(name, safe="-._~")
    return f"{encoded_instance}|{doctype}|{encoded_name}"


def _date(value: str | None) -> date | None:
    return date.fromisoformat(value) if value is not None else None


def map_maintenance_log(snapshot: dict[str, Any], binding: dict[str, Any], now: datetime) -> dict[str, Any]:
    """Map one snapshot using the trusted binding's IANA site timezone.

    now must be timezone-aware; date-only claims are compared with the current
    date in binding["site_timezone"]. Source date-only values stay dates. This
    function never invents a time, writes source records, verifies credentials,
    or establishes workmanship.
    """
    if now.tzinfo is None or now.utcoffset() is None:
        _fail("ERPNext evaluation clock must include a timezone")
    _validate("snapshot", snapshot)
    _validate("binding", binding)
    core.canonical_bytes(snapshot)
    core.canonical_bytes(binding)
    try:
        site_zone = ZoneInfo(binding["site_timezone"])
    except (ZoneInfoNotFoundError, ValueError):
        _fail("ERPNext tenant binding site timezone is not a valid IANA timezone")
    site_today = now.astimezone(site_zone).date()

    source, plan, task, log = snapshot["source"], snapshot["plan"], snapshot["task"], snapshot["log"]
    if source["release_tag"] != PINNED_RELEASE or source["repository_commit"] != PINNED_COMMIT:
        _fail("unsupported ERPNext source revision; review and re-pin the mapping")
    if source["license"] != PINNED_LICENSE:
        _fail("ERPNext source license indicator changed; require review")
    if binding["status"] != "active":
        _fail("ERPNext tenant binding is not active")
    if binding["source_instance_namespace"] != source["instance_namespace"]:
        _fail("ERPNext tenant binding source instance mismatch")
    if binding["source_company_id"] != plan["company_id"]:
        _fail("ERPNext plan company does not match explicit tenant binding")
    if core.parse_time(binding["configured_at"]) > now:
        _fail("ERPNext tenant binding configuration is dated in the future")
    if core.parse_time(snapshot["exported_at"]) > now:
        _fail("ERPNext source snapshot export is dated in the future")

    if plan["task_id"] != task["name"]:
        _fail("ERPNext plan/task reference mismatch")
    if log["asset_maintenance_id"] != plan["name"]:
        _fail("ERPNext log/plan reference mismatch")
    if log["task_id"] != task["name"]:
        _fail("ERPNext log/task reference mismatch")

    status, docstatus = log["maintenance_status"], log["docstatus"]
    completion, due = _date(log["completion_date"]), _date(log["due_date"])
    last_completion = _date(task["last_completion_date"])
    if status == "Completed" and completion is None:
        _fail("ERPNext Completed log requires a completion date")
    if status != "Completed" and completion is not None:
        _fail("ERPNext non-completed log cannot have a completion date")
    if docstatus == 1 and status not in {"Completed", "Cancelled"}:
        _fail("ERPNext submitted log must be Completed or Cancelled")
    if completion and completion > site_today:
        _fail("ERPNext completion date is in the future")
    if last_completion and last_completion > site_today:
        _fail("ERPNext task last-completion date is in the future")

    conflicts: set[str] = set()
    # Frappe document state and the custom maintenance status are separate.
    # Preserve their disagreement instead of assuming undocumented coupling.
    if docstatus == 2 and status != "Cancelled":
        conflicts.add("document_cancelled_but_maintenance_status_not_cancelled")
    if status == "Cancelled" and docstatus != 2:
        conflicts.add("maintenance_status_cancelled_but_document_not_cancelled")
    losses: set[str] = {
        "professional_credential_status_not_provided_by_source",
        "independent_verification_not_provided_by_source",
        "certificate_authenticity_not_established",
        "source_event_history_not_exported",
        "site_timezone_and_source_clock_not_independently_attested",
    }
    if task["certificate_required"] != log["has_certificate"]:
        conflicts.add("task_certificate_requirement_disagrees_with_log_flag")
    if log["has_certificate"] and not log["certificate_attachment_present"]:
        conflicts.add("certificate_flag_without_attachment_reference")
    if task["certificate_required"] and not log["certificate_attachment_present"]:
        conflicts.add("required_certificate_attachment_missing")
    if log["actions_performed_present"]:
        losses.add("raw_actions_performed_content_omitted_by_privacy_policy")
    if log["certificate_attachment_present"]:
        losses.add("certificate_bytes_and_attachment_path_omitted_by_privacy_policy")

    output = {
        "schema": "org.omsi.erpnext-maintenance-intake/0.1",
        "id": _ref(source["instance_namespace"], "Asset Maintenance Log", log["name"]),
        "tenant_id": binding["tenant_id"],
        "mapping_context": {
            "adapter_id": "org.omsi.erpnext-maintenance-adapter/0.1",
            "binding_id": binding["binding_id"],
            "site_timezone": binding["site_timezone"],
            "evaluated_at": now.astimezone(timezone.utc).isoformat().replace("+00:00", "Z"),
        },
        "source": {
            "system": "ERPNext", "instance_namespace": source["instance_namespace"],
            "release_tag": source["release_tag"], "repository_commit": source["repository_commit"],
            "license": source["license"], "doctype": "Asset Maintenance Log",
            "record_id": log["name"], "plan_record_id": plan["name"], "task_record_id": task["name"],
            "company_id": plan["company_id"], "exported_at": snapshot["exported_at"],
        },
        "maintenance": {
            "source_status": status, "source_docstatus": docstatus, "normalized_status": "cancelled" if docstatus == 2 or status == "Cancelled" else STATUS_MAP[status],
            "plan_ref": _ref(source["instance_namespace"], "Asset Maintenance", plan["name"]),
            "task_ref": _ref(source["instance_namespace"], "Asset Maintenance Task", task["name"]),
            "asset_ref": _ref(source["instance_namespace"], "Asset", plan["asset_name"]),
            "maintenance_type": task["maintenance_type"], "periodicity": task["periodicity"],
            "due_date": log["due_date"], "next_task_due_date": task["next_due_date"],
            "completion_date": log["completion_date"],
            "last_task_completion_date": task["last_completion_date"],
            "amended_from_ref": _ref(source["instance_namespace"], "Asset Maintenance Log", log["amended_from"]) if log["amended_from"] else None,
            "mapping_conflicts": sorted(conflicts), "mapping_losses": sorted(losses),
        },
        "source_claims": {
            "certificate_required": task["certificate_required"], "has_certificate_flag": log["has_certificate"],
            "certificate_attachment_present": log["certificate_attachment_present"],
            "actions_performed_present": log["actions_performed_present"],
            "assigned_user_present": task["assigned_user_present"],
        },
        "assurance": {
            "workmanship_quality": "not_verified",
            "practitioner_credential_status": "not_provided_by_source",
            "independent_verification": "not_provided_by_source",
            "certificate_authenticity": "not_established",
        },
        "privacy": {
            "raw_actions_performed_exported": False, "certificate_bytes_exported": False,
            "customer_contact_exported": False,
        },
    }
    _validate("intake", output)
    return output
