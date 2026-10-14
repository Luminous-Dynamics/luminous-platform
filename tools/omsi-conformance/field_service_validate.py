"""Synthetic OMSI Field Service profile 0.1 reference checks.

This is a conformance prototype. It does not authenticate issuers, validate real
professional licenses, inspect physical systems, verify evidence truth, or
authorize production operations.
"""
from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

import omsi_validate as core

ROOT = Path(__file__).resolve().parent
SCHEMA_PATH = ROOT / "schemas/0.1/field-service-record.schema.json"


def _schema_validator() -> Draft202012Validator:
    schema = core.load(SCHEMA_PATH)
    Draft202012Validator.check_schema(schema)
    return Draft202012Validator(schema, format_checker=FormatChecker())


def _pointer(path: tuple[Any, ...]) -> str:
    if not path:
        return "/"
    return "/" + "/".join(str(part).replace("~", "~0").replace("/", "~1") for part in path)


def _fail(message: str) -> None:
    raise core.ConformanceError(message)


def validate_record(record: dict[str, Any], now: datetime) -> None:
    """Validate schema plus cross-record invariants at an explicit evaluation time."""
    if now.tzinfo is None:
        _fail("evaluation time must include a timezone")
    now = now.astimezone(timezone.utc)
    validator = _schema_validator()
    errors = sorted(validator.iter_errors(record), key=lambda error: tuple(map(str, error.absolute_path)))
    if errors:
        error = errors[0]
        _fail(f"field-service schema error at {_pointer(tuple(error.absolute_path))}: {error.message}")

    # The common OMSI JSON profile rejects float values rather than silently
    # allowing ambiguous cross-language number/digest behavior.
    core.canonical_bytes(record)
    tenant = record["tenant_id"]
    created = core.parse_time(record["created_at"])
    if created > now:
        _fail("field-service record creation time is in the future")

    case = record["service_case"]
    work = record["work_order"]
    site = record["site"]
    asset = record["asset"]
    practitioner = record["practitioner"]
    estimate = record["estimate"]
    approval = record["approval"]

    tenant_objects = (
        ("work order", work),
        ("site", site),
        ("asset", asset),
        ("practitioner", practitioner),
        ("approval", approval),
    )
    for label, obj in tenant_objects:
        if obj["tenant_id"] != tenant:
            _fail(f"tenant mismatch: {label} belongs to another tenant")

    if case["normalized_status"] != work["status"]:
        _fail("service case and work-order lifecycle mapping conflict")
    if case["mapping_losses"] and not case["mapping_loss_disclosed"]:
        _fail("mapping loss is not disclosed")
    if work["case_external_id"] != case["external_id"]:
        _fail("work order does not bind the source case identifier")
    if work["site_id"] != site["id"] or site["jurisdiction"] != work["jurisdiction"]:
        _fail("work order site or jurisdiction binding mismatch")
    if work["asset_id"] != asset["id"]:
        _fail("work order asset binding mismatch")
    if asset["tenant_id"] != site["tenant_id"]:
        _fail("site and asset tenant mismatch")

    scope_digest = work["scope_digest"]
    scope_version = work["scope_version"]
    if estimate["scope_digest"] != scope_digest or estimate["scope_version"] != scope_version:
        _fail("estimate does not match current work scope")
    if approval["status"] != "accepted":
        _fail("current work scope has no accepted customer approval")
    if approval["estimate_id"] != estimate["id"] or approval["estimate_version"] != estimate["version"]:
        _fail("approval does not bind the current estimate version")
    if (
        approval["scope_version"] != scope_version
        or approval["scope_digest"] != scope_digest
        or approval["currency"] != estimate["currency"]
        or approval["approved_total_minor"] != estimate["total_minor"]
    ):
        _fail("approval does not match current work scope, amount, or currency")
    approved_at = core.parse_time(approval["approved_at"])
    if approved_at < created or approved_at > core.parse_time(estimate["valid_until"]):
        _fail("approval time falls outside record/estimate validity")
    if core.parse_time(estimate["valid_until"]) <= approved_at:
        _fail("estimate expired before approval")

    credentials = practitioner["credentials"]
    credential_ids = [credential["credential_id"] for credential in credentials]
    if len(credential_ids) != len(set(credential_ids)):
        _fail("duplicate practitioner credential ID")
    credentials_by_id = {credential["credential_id"]: credential for credential in credentials}
    max_age = work["credential_status_max_age_seconds"]
    for credential in credentials:
        if credential["tenant_id"] != tenant:
            _fail("tenant mismatch: practitioner credential belongs to another tenant")

    def credentials_at(instant: datetime) -> dict[str, dict[str, Any]]:
        """Evaluate the recorded credential status at the operation/evidence time.

        Historical jobs must not become invalid merely because a credential later
        expires. The status snapshot must nevertheless predate the action and be
        within the profile's declared freshness window at the time of use.
        """
        valid: dict[str, dict[str, Any]] = {}
        for credential in credentials:
            checked_at = core.parse_time(credential["checked_at"])
            valid_until = core.parse_time(credential["valid_until"])
            if credential["status"] != "valid":
                continue
            if credential["jurisdiction"] != work["jurisdiction"]:
                continue
            if checked_at > instant or (instant - checked_at).total_seconds() > max_age:
                continue
            if valid_until <= instant:
                continue
            valid[credential["credential_id"]] = credential
        return valid

    operations = record["operations"]
    operation_ids: dict[str, dict[str, Any]] = {}
    idempotency: dict[str, tuple[str, str]] = {}
    for operation in operations:
        op_id = operation["operation_id"]
        if op_id in operation_ids:
            _fail("duplicate operation ID")
        if operation["tenant_id"] != tenant:
            _fail("tenant mismatch: operation belongs to another tenant")
        if (
            operation["site_id"] != site["id"]
            or operation["asset_id"] != asset["id"]
            or operation["scope_version"] != scope_version
        ):
            _fail("operation target or scope binding mismatch")
        if core.parse_time(operation["authorized_at"]) < approved_at:
            _fail("operation was authorized before customer approval")
        expires_at = core.parse_time(operation["authority_expires_at"])
        authorized_at = core.parse_time(operation["authorized_at"])
        applied_at = core.parse_time(operation["applied_at"])
        if expires_at <= authorized_at:
            _fail("operation authority has an invalid validity interval")
        if authorized_at > core.parse_time(estimate["valid_until"]):
            _fail("operation authorized after estimate expiry")
        if applied_at < authorized_at:
            _fail("operation application predates authorization")
        if operation["state"] in {"applied", "verified"} and applied_at > expires_at:
            _fail("operation applied after authority expiry")
        for instant in (authorized_at, applied_at):
            credentials_at_use = credentials_at(instant)
            for required_scope in work["required_practitioner_scopes"]:
                if not any(required_scope in credential["scopes"] for credential in credentials_at_use.values()):
                    _fail("no current credential covers required scope")
        if operation["state"] in {"authorized", "accepted", "running"} and expires_at <= now:
            _fail("active operation authority has expired")
        idem = operation["idempotency_key"]
        manifest = operation["manifest_digest"]
        prior = idempotency.get(idem)
        if prior is not None:
            if prior[0] != manifest:
                _fail("idempotency key reused with different manifest")
            if prior[1] != op_id:
                _fail("idempotency key reused across different operation IDs")
        idempotency[idem] = (manifest, op_id)
        operation_ids[op_id] = operation

    evidence_by_id: dict[str, dict[str, Any]] = {}
    for evidence in record["evidence"]:
        evidence_id = evidence["id"]
        if evidence_id in evidence_by_id:
            _fail("duplicate evidence ID")
        if evidence["tenant_id"] != tenant:
            _fail("tenant mismatch: evidence belongs to another tenant")
        if (
            evidence["site_id"] != site["id"]
            or evidence["asset_id"] != asset["id"]
            or evidence["scope_version"] != scope_version
        ):
            _fail("evidence target or scope binding mismatch")
        op = operation_ids.get(evidence["operation_id"])
        if op is None:
            _fail("evidence references an unknown operation")
        if evidence["operation_manifest_digest"] != op["manifest_digest"]:
            _fail("evidence operation manifest digest mismatch")
        occurred_at = core.parse_time(evidence["occurred_at"])
        if occurred_at < created:
            _fail("evidence predates the service record")
        if occurred_at < core.parse_time(op["applied_at"]):
            _fail("evidence observation predates operation application")

        kind = evidence["evidence_kind"]
        status = evidence["claim_status"]
        expected_status = {
            "provider_report": "reported",
            "customer_acknowledgement": "acknowledged",
            "instrument_measurement": "measured",
            "qualified_inspection": "inspected",
            "independent_verification": "verified",
            "handover_record": "acknowledged",
            "dispute": "disputed",
            "correction": "reported",
        }[kind]
        if status != expected_status:
            _fail("evidence kind and claim status conflict")

        if kind == "instrument_measurement":
            measurement = evidence.get("measurement")
            if measurement is None:
                _fail("instrument measurement lacks typed measurement data")
            requirement = next(
                (r for r in work["measurement_requirements"] if r["requirement_id"] == measurement["requirement_ref"]),
                None,
            )
            if requirement is None:
                _fail("measurement references an unknown requirement")
            if measurement["calibration_status"] != "valid":
                _fail("measurement calibration is not current")
            if (
                measurement["method_ref"] != requirement["method_ref"]
                or measurement["unit_code"] != requirement["unit_code"]
                or measurement["scale"] != requirement["scale"]
            ):
                _fail("measurement does not match required profile")
            if not requirement["min_scaled"] <= measurement["value_scaled"] <= requirement["max_scaled"]:
                _fail("measurement is outside the declared synthetic acceptance interval")

        if kind == "qualified_inspection":
            inspection = evidence.get("inspection")
            if inspection is None:
                _fail("qualified inspection lacks typed inspection data")
            credentials_at_inspection = credentials_at(occurred_at)
            credential = credentials_at_inspection.get(inspection["credential_ref"])
            if credential is None:
                _fail("inspection references a credential that is not current")
            if not set(work["required_practitioner_scopes"]).intersection(credential["scopes"]):
                _fail("inspection credential lacks the required practitioner scope")
            if inspection["result"] != "passed":
                _fail("qualified inspection did not pass")

        if kind == "independent_verification":
            verification = evidence.get("verification")
            if verification is None:
                _fail("independent verification lacks typed verification data")
            if verification["verifier_id"] != evidence["recorded_by"]:
                _fail("verification actor does not match evidence recorder")
            if verification["verifier_id"] == op["executor_id"]:
                _fail("independent verifier must differ from executor")
            if verification["verifier_authority_domain"] == op["executor_authority_domain"]:
                _fail("independent verification lacks distinct authority domains")
            if verification["result"] != "passed":
                _fail("verified evidence must record a passing verification result")

        evidence_by_id[evidence_id] = evidence

    for evidence in evidence_by_id.values():
        if evidence["evidence_kind"] == "correction":
            supersedes = evidence.get("supersedes_evidence_id")
            prior = evidence_by_id.get(supersedes) if supersedes else None
            if prior is None or prior["id"] == evidence["id"]:
                _fail("correction must supersede existing evidence")
            if core.parse_time(evidence["occurred_at"]) <= core.parse_time(prior["occurred_at"]):
                _fail("correction must occur after the evidence it supersedes")

    required_measurements = {r["requirement_id"] for r in work["measurement_requirements"]}
    observed_measurements = {
        evidence["measurement"]["requirement_ref"]
        for evidence in evidence_by_id.values()
        if evidence["evidence_kind"] == "instrument_measurement" and evidence["claim_status"] == "measured"
    }
    if not required_measurements.issubset(observed_measurements):
        _fail("required measurement evidence is missing")

    for op in operations:
        if op["required_verification"] and op["state"] == "verified":
            if not any(
                evidence["operation_id"] == op["operation_id"]
                and evidence["evidence_kind"] == "independent_verification"
                and evidence["claim_status"] == "verified"
                and evidence["verification"]["result"] == "passed"
                for evidence in evidence_by_id.values()
            ):
                _fail("verified operation lacks linked independent verification evidence")

    closeout = record["closeout"]
    missing_refs = set(closeout["evidence_refs"]) - set(evidence_by_id)
    if missing_refs:
        _fail("closeout references missing evidence")
    acknowledgement = evidence_by_id.get(closeout["customer_ack_evidence_id"])
    if (
        acknowledgement is None
        or acknowledgement["evidence_kind"] != "customer_acknowledgement"
        or acknowledgement["claim_status"] != "acknowledged"
    ):
        _fail("closeout customer acknowledgement reference is invalid")
    if closeout["status"] == "handed_over":
        if work["status"] != "handed_over":
            _fail("handover closeout conflicts with work-order lifecycle")
        if not any(
            evidence["id"] in closeout["evidence_refs"]
            and evidence["evidence_kind"] == "handover_record"
            and evidence["claim_status"] == "acknowledged"
            for evidence in evidence_by_id.values()
        ):
            _fail("handover closeout lacks a linked handover record")

