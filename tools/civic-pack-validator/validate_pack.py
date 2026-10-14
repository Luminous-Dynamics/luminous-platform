#!/usr/bin/env python3
"""Deterministic structural and publication-gate validator for jurisdiction packs.

The validator is intentionally not a legal reviewer. It verifies data integrity,
review metadata, scope, and explicit publication gates. Run with a frozen --as-of
timestamp so the same inputs produce the same report.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any
from urllib.parse import urlsplit

from jsonschema import Draft202012Validator, FormatChecker, SchemaError

VALIDATOR_VERSION = "0.1.0"
TERMINAL_REVIEW_STATES = {"stale", "unavailable", "disputed", "withdrawn"}
CURRENT_CLAIM_TYPES = {"official_requirement", "official_guidance"}


def sha256_hex(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def parse_datetime(value: Any) -> datetime | None:
    if not isinstance(value, str):
        return None
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        return None
    return parsed.astimezone(timezone.utc)


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")


def _unique_check(values: list[str]) -> bool:
    return len(values) == len(set(values))


REQUIRED_CURRENT_REVIEW_SCOPE = (
    "authority_confirmed",
    "effective_status_checked",
    "content_reviewed",
    "applicability_reviewed",
    "reuse_rights_reviewed",
)
REQUIRED_VERIFIED_REQUIREMENT_SCOPE = (
    "source_support_checked",
    "jurisdiction_checked",
    "applicability_checked",
    "user_language_checked",
    "completion_effect_checked",
)
REQUIREMENT_CLAIM_FIELDS = (
    "requirement_id",
    "title",
    "claim_type",
    "summary",
    "applicability",
    "source_refs",
    "user_actions",
    "completion_effect",
)


def requirement_claim_sha256(requirement: dict[str, Any]) -> str:
    """Hash only the claim-bearing fields, excluding mutable review metadata."""
    claim = {field: requirement[field] for field in REQUIREMENT_CLAIM_FIELDS}
    return sha256_hex(canonical_json_bytes(claim))


def _same_timestamp(left: Any, right: Any) -> bool:
    if left is None or right is None:
        return left is right
    parsed_left = parse_datetime(left)
    parsed_right = parse_datetime(right)
    return parsed_left is not None and parsed_left == parsed_right


def validate_pack(
    schema: dict[str, Any],
    pack: dict[str, Any],
    *,
    schema_bytes: bytes,
    pack_bytes: bytes,
    as_of: datetime,
) -> dict[str, Any]:
    """Validate one pack and return a deterministic, machine-readable report."""
    if as_of.tzinfo is None or as_of.utcoffset() is None:
        raise ValueError("as_of must be timezone-aware")
    as_of = as_of.astimezone(timezone.utc)
    evaluated_at = as_of.isoformat().replace("+00:00", "Z")

    checks: list[dict[str, str]] = []

    def add(check_id: str, status: str, details: str) -> None:
        checks.append({"check_id": check_id, "status": status, "details": details})

    try:
        Draft202012Validator.check_schema(schema)
        add("schema.meta_schema", "PASS", "Schema is valid JSON Schema Draft 2020-12.")
    except SchemaError as exc:
        add("schema.meta_schema", "FAIL", f"Invalid schema: {exc.message}")
        return _report(schema, pack, schema_bytes, pack_bytes, evaluated_at, checks)

    instance_validator = Draft202012Validator(
        schema, format_checker=FormatChecker()
    )
    schema_errors = sorted(
        instance_validator.iter_errors(pack),
        key=lambda error: (list(map(str, error.absolute_path)), error.message),
    )
    if schema_errors:
        messages = [
            "$" + "".join(
                f"[{part}]" if isinstance(part, int) else f".{part}"
                for part in error.absolute_path
            ) + f": {error.message}"
            for error in schema_errors
        ]
        add(
            "pack.schema_validation",
            "FAIL",
            " | ".join(messages[:50]),
        )
        return _report(schema, pack, schema_bytes, pack_bytes, evaluated_at, checks)
    else:
        add(
            "pack.schema_validation",
            "PASS",
            "Pack conforms to its declared schema, including format checks.",
        )

    if not isinstance(pack, dict):
        add("pack.root_type", "FAIL", "Pack root must be an object.")
        return _report(schema, pack, schema_bytes, pack_bytes, evaluated_at, checks)

    sources = pack.get("sources", [])
    requirements = pack.get("requirements", [])
    workflow = pack.get("workflow", {})
    source_ids = [s.get("source_id", "") for s in sources if isinstance(s, dict)]
    requirement_ids = [
        r.get("requirement_id", "") for r in requirements if isinstance(r, dict)
    ]
    action_ids = [
        action.get("action_id", "")
        for req in requirements if isinstance(req, dict)
        for action in req.get("user_actions", []) if isinstance(action, dict)
    ]
    predicate_ids = [
        predicate.get("predicate_id", "")
        for req in requirements if isinstance(req, dict)
        for predicate in req.get("applicability", {}).get("predicates", [])
        if isinstance(predicate, dict)
    ]
    source_review_receipts = pack.get("source_review_receipts", [])
    review_receipt_ids = [
        receipt.get("receipt_id", "")
        for receipt in source_review_receipts if isinstance(receipt, dict)
    ]
    requirement_review_receipts = pack.get("requirement_review_receipts", [])
    requirement_review_receipt_ids = [
        receipt.get("receipt_id", "")
        for receipt in requirement_review_receipts if isinstance(receipt, dict)
    ]

    all_unique = (
        len(source_ids) == len(sources)
        and len(requirement_ids) == len(requirements)
        and len(action_ids) == sum(
            len(req.get("user_actions", []))
            for req in requirements if isinstance(req, dict)
        )
        and len(predicate_ids) == sum(
            len(req.get("applicability", {}).get("predicates", []))
            for req in requirements if isinstance(req, dict)
        )
        and len(review_receipt_ids) == len(source_review_receipts)
        and len(requirement_review_receipt_ids) == len(requirement_review_receipts)
        and _unique_check(source_ids)
        and _unique_check(requirement_ids)
        and _unique_check(action_ids)
        and _unique_check(predicate_ids)
        and _unique_check(review_receipt_ids)
        and _unique_check(requirement_review_receipt_ids)
    )
    add(
        "pack.unique_identifiers",
        "PASS" if all_unique else "FAIL",
        "Source, requirement, action, predicate, source-review receipt, and requirement-review receipt identifiers must be present and unique within the pack.",
    )

    source_by_id = {
        s.get("source_id"): s for s in sources if isinstance(s, dict)
    }
    receipts_by_source: dict[str, list[dict[str, Any]]] = {}
    last_reviewed_by_source: dict[str, datetime] = {}
    receipt_errors: list[str] = []
    for receipt in source_review_receipts:
        if not isinstance(receipt, dict):
            receipt_errors.append("review receipt must be an object")
            continue
        receipt_id = receipt.get("receipt_id", "<unknown>")
        source_id = receipt.get("source_id", "<unknown>")
        if source_id not in source_by_id:
            receipt_errors.append(f"{receipt_id}: unknown source_id {source_id}")
            continue
        receipts_by_source.setdefault(source_id, []).append(receipt)
        captured = parse_datetime(receipt.get("captured_at"))
        reviewed = parse_datetime(receipt.get("reviewed_at"))
        next_review = parse_datetime(receipt.get("next_review_at"))
        if captured is None or reviewed is None or next_review is None:
            receipt_errors.append(f"{receipt_id}: invalid review timestamp")
        elif captured > reviewed:
            receipt_errors.append(f"{receipt_id}: capture is later than review")
        elif next_review <= reviewed:
            receipt_errors.append(f"{receipt_id}: next_review_at must follow reviewed_at")
        if reviewed is not None:
            previous_review = last_reviewed_by_source.get(source_id)
            if previous_review is not None and reviewed <= previous_review:
                receipt_errors.append(f"{receipt_id}: review history is not strictly chronological for {source_id}")
            last_reviewed_by_source[source_id] = reviewed
        if not isinstance(receipt.get("reviewer_id"), str) or not receipt.get("reviewer_id", "").strip():
            receipt_errors.append(f"{receipt_id}: reviewer_id is missing")
        if not isinstance(receipt.get("reviewer_role"), str) or not receipt.get("reviewer_role", "").strip():
            receipt_errors.append(f"{receipt_id}: reviewer_role is missing")
        if receipt.get("decision") == "current":
            scope = receipt.get("review_scope", {})
            incomplete = [field for field in REQUIRED_CURRENT_REVIEW_SCOPE if scope.get(field) is not True]
            if incomplete:
                receipt_errors.append(f"{receipt_id}: current decision has incomplete review scope ({', '.join(incomplete)})")
            if not isinstance(receipt.get("snapshot_sha256"), str) or len(receipt.get("snapshot_sha256", "")) != 64:
                receipt_errors.append(f"{receipt_id}: current decision requires a source snapshot SHA-256")

    # Non-unreviewed states must be backed by the latest source-bound receipt.
    for source in sources:
        if not isinstance(source, dict):
            continue
        source_id = source.get("source_id", "<unknown>")
        state = source.get("review_state")
        history = receipts_by_source.get(source_id, [])
        pointer = source.get("review_receipt_id")
        if state == "unreviewed":
            if history or pointer is not None:
                receipt_errors.append(f"{source_id}: unreviewed state conflicts with review receipt history")
            continue
        if not history or not pointer:
            receipt_errors.append(f"{source_id}: review_state={state} has no linked review receipt")
            continue
        dated_history = [(parse_datetime(item.get("reviewed_at")), item) for item in history]
        if any(when is None for when, _ in dated_history):
            receipt_errors.append(f"{source_id}: receipt history has an invalid reviewed_at")
            continue
        timestamps = [when for when, _ in dated_history]
        if len(timestamps) != len(set(timestamps)):
            receipt_errors.append(f"{source_id}: receipt history has ambiguous duplicate reviewed_at timestamps")
            continue
        if any(left >= right for left, right in zip(timestamps, timestamps[1:])):
            receipt_errors.append(f"{source_id}: receipt history is not strictly chronological")
            continue
        latest = history[-1]
        if pointer != latest.get("receipt_id"):
            receipt_errors.append(f"{source_id}: review_receipt_id does not identify the latest receipt")
        if state != latest.get("decision"):
            receipt_errors.append(f"{source_id}: review_state does not match the latest receipt decision")
        pairs = (
            ("canonical_url", "canonical_url"),
            ("published_at", "published_at"),
            ("effective_from", "effective_from"),
            ("checked_at", "captured_at"),
            ("reviewed_at", "reviewed_at"),
            ("next_review_at", "next_review_at"),
            ("reviewer_id", "reviewer_id"),
            ("reviewer_role", "reviewer_role"),
            ("snapshot_sha256", "snapshot_sha256"),
            ("reuse", "reuse"),
            ("source_license_url", "source_license_url"),
            ("attribution", "attribution"),
            ("rights_notes", "rights_notes"),
        )
        for source_field, receipt_field in pairs:
            if source_field in {"published_at", "effective_from", "checked_at", "reviewed_at", "next_review_at"}:
                matches = _same_timestamp(source.get(source_field), latest.get(receipt_field))
            else:
                matches = source.get(source_field) == latest.get(receipt_field)
            if not matches:
                receipt_errors.append(f"{source_id}: {source_field} does not match latest receipt {pointer}")

    receipt_errors = sorted(set(receipt_errors))
    add(
        "source.review_receipts",
        "FAIL" if receipt_errors else "PASS",
        "Source review receipt consistency: " + ("; ".join(receipt_errors) if receipt_errors else "all records are source-bound and internally consistent."),
    )

    requirement_by_id = {
        req.get("requirement_id"): req for req in requirements if isinstance(req, dict)
    }
    source_receipt_by_id = {
        receipt.get("receipt_id"): receipt
        for receipt in source_review_receipts if isinstance(receipt, dict)
    }
    requirement_receipt_by_id = {
        receipt.get("receipt_id"): receipt
        for receipt in requirement_review_receipts if isinstance(receipt, dict)
    }
    requirement_receipts_by_requirement: dict[str, list[dict[str, Any]]] = {}
    requirement_receipt_errors: list[str] = []
    last_reviewed_by_requirement: dict[str, datetime] = {}
    for receipt in requirement_review_receipts:
        if not isinstance(receipt, dict):
            requirement_receipt_errors.append("requirement review receipt must be an object")
            continue
        receipt_id = receipt.get("receipt_id", "<unknown>")
        requirement_id = receipt.get("requirement_id", "<unknown>")
        requirement = requirement_by_id.get(requirement_id)
        if requirement is None:
            requirement_receipt_errors.append(f"{receipt_id}: unknown requirement_id {requirement_id}")
            continue
        requirement_receipts_by_requirement.setdefault(requirement_id, []).append(receipt)
        reviewed = parse_datetime(receipt.get("reviewed_at"))
        next_review = parse_datetime(receipt.get("next_review_at"))
        if reviewed is None or next_review is None:
            requirement_receipt_errors.append(f"{receipt_id}: invalid review timestamp")
        elif next_review <= reviewed:
            requirement_receipt_errors.append(f"{receipt_id}: next_review_at must follow reviewed_at")
        if reviewed is not None:
            previous_review = last_reviewed_by_requirement.get(requirement_id)
            if previous_review is not None and reviewed <= previous_review:
                requirement_receipt_errors.append(
                    f"{receipt_id}: review history is not strictly chronological for {requirement_id}"
                )
            last_reviewed_by_requirement[requirement_id] = reviewed
        if not isinstance(receipt.get("reviewer_id"), str) or not receipt.get("reviewer_id", "").strip():
            requirement_receipt_errors.append(f"{receipt_id}: reviewer_id is missing")
        if not isinstance(receipt.get("reviewer_role"), str) or not receipt.get("reviewer_role", "").strip():
            requirement_receipt_errors.append(f"{receipt_id}: reviewer_role is missing")
        for source_receipt_id in receipt.get("source_receipt_ids", []):
            source_receipt = source_receipt_by_id.get(source_receipt_id)
            if source_receipt is None:
                requirement_receipt_errors.append(
                    f"{receipt_id}: unknown source receipt {source_receipt_id}"
                )
            elif source_receipt.get("source_id") not in requirement.get("source_refs", []):
                requirement_receipt_errors.append(
                    f"{receipt_id}: source receipt {source_receipt_id} is outside this requirement's source_refs"
                )
        if receipt.get("decision") == "verified":
            scope = receipt.get("review_scope", {})
            incomplete = [
                field for field in REQUIRED_VERIFIED_REQUIREMENT_SCOPE
                if scope.get(field) is not True
            ]
            if incomplete:
                requirement_receipt_errors.append(
                    f"{receipt_id}: verified decision has incomplete review scope ({', '.join(incomplete)})"
                )

    for requirement in requirements:
        if not isinstance(requirement, dict):
            continue
        requirement_id = requirement.get("requirement_id", "<unknown>")
        state = requirement.get("review_state")
        history = requirement_receipts_by_requirement.get(requirement_id, [])
        pointer = requirement.get("review_receipt_id")
        if state == "draft":
            if history or pointer is not None or requirement.get("reviewer_id") is not None or requirement.get("reviewer_role") is not None:
                requirement_receipt_errors.append(
                    f"{requirement_id}: draft state conflicts with requirement review receipt metadata"
                )
            continue
        if not history or not pointer:
            requirement_receipt_errors.append(
                f"{requirement_id}: review_state={state} has no linked requirement review receipt"
            )
            continue
        dated_history = [(parse_datetime(item.get("reviewed_at")), item) for item in history]
        if any(when is None for when, _ in dated_history):
            requirement_receipt_errors.append(f"{requirement_id}: receipt history has an invalid reviewed_at")
            continue
        timestamps = [when for when, _ in dated_history]
        if len(timestamps) != len(set(timestamps)):
            requirement_receipt_errors.append(
                f"{requirement_id}: receipt history has ambiguous duplicate reviewed_at timestamps"
            )
            continue
        if any(left >= right for left, right in zip(timestamps, timestamps[1:])):
            requirement_receipt_errors.append(
                f"{requirement_id}: receipt history is not strictly chronological"
            )
            continue
        latest = history[-1]
        if pointer != latest.get("receipt_id"):
            requirement_receipt_errors.append(
                f"{requirement_id}: review_receipt_id does not identify the latest receipt"
            )
        if state != latest.get("decision"):
            requirement_receipt_errors.append(
                f"{requirement_id}: review_state does not match the latest receipt decision"
            )
        if requirement.get("reviewer_id") != latest.get("reviewer_id"):
            requirement_receipt_errors.append(
                f"{requirement_id}: reviewer_id does not match latest receipt"
            )
        if requirement.get("reviewer_role") != latest.get("reviewer_role"):
            requirement_receipt_errors.append(
                f"{requirement_id}: reviewer_role does not match latest receipt"
            )
        if latest.get("claim_sha256") != requirement_claim_sha256(requirement):
            requirement_receipt_errors.append(
                f"{requirement_id}: claim fields no longer match latest review receipt digest"
            )
        if state == "verified":
            expected_source_receipt_ids = []
            all_sources_current = True
            for source_ref in requirement.get("source_refs", []):
                source = source_by_id.get(source_ref)
                if (
                    source is None
                    or source.get("review_state") != "current"
                    or not source.get("review_receipt_id")
                ):
                    all_sources_current = False
                    continue
                expected_source_receipt_ids.append(source["review_receipt_id"])
            if not all_sources_current:
                requirement_receipt_errors.append(
                    f"{requirement_id}: verified claim references a missing or non-current source receipt"
                )
            if latest.get("source_receipt_ids") != expected_source_receipt_ids:
                requirement_receipt_errors.append(
                    f"{requirement_id}: source_receipt_ids do not exactly match the reviewed claim's sources"
                )

    requirement_receipt_errors = sorted(set(requirement_receipt_errors))
    add(
        "requirement.review_receipts",
        "FAIL" if requirement_receipt_errors else "PASS",
        "Requirement review receipt consistency: " + (
            "; ".join(requirement_receipt_errors)
            if requirement_receipt_errors
            else "all verified claims are bound to their current source receipts and claim digests."
        ),
    )

    unresolved = sorted({
        source_ref
        for req in requirements if isinstance(req, dict)
        for source_ref in req.get("source_refs", [])
        if source_ref not in source_by_id
    })
    add(
        "pack.source_references",
        "FAIL" if unresolved else "PASS",
        "Unresolved source_refs: " + (", ".join(unresolved) if unresolved else "none."),
    )

    allowed_business_types = set(workflow.get("business_types", []))
    out_of_scope: list[str] = []
    for requirement in requirements:
        if not isinstance(requirement, dict):
            continue
        requirement_id = requirement.get("requirement_id", "<unknown>")
        applicability = requirement.get("applicability", {})
        for business_type in applicability.get("business_types", []):
            if business_type not in allowed_business_types:
                out_of_scope.append(f"{requirement_id}:{business_type}")
    out_of_scope.sort()
    add(
        "pack.applicability_scope",
        "FAIL" if out_of_scope else "PASS",
        "Business types outside workflow scope: " + (
            ", ".join(out_of_scope) if out_of_scope else "none."
        ),
    )

    unsafe_urls: list[str] = []
    for source in sources:
        if not isinstance(source, dict):
            continue
        source_id = source.get("source_id", "<unknown>")
        parsed = urlsplit(source.get("canonical_url", ""))
        if (
            parsed.scheme != "https"
            or not parsed.hostname
            or parsed.username is not None
            or parsed.password is not None
        ):
            unsafe_urls.append(f"source:{source_id}")
    for requirement in requirements:
        if not isinstance(requirement, dict):
            continue
        req_id = requirement.get("requirement_id", "<unknown>")
        for action in requirement.get("user_actions", []):
            if not isinstance(action, dict) or not action.get("url"):
                continue
            parsed = urlsplit(action["url"])
            if (
                parsed.scheme != "https"
                or not parsed.hostname
                or parsed.username is not None
                or parsed.password is not None
            ):
                unsafe_urls.append(f"action:{req_id}:{action.get('action_id', '<unknown>')}")
    for receipt in source_review_receipts:
        if not isinstance(receipt, dict):
            continue
        parsed = urlsplit(receipt.get("canonical_url", ""))
        if (
            parsed.scheme != "https"
            or not parsed.hostname
            or parsed.username is not None
            or parsed.password is not None
        ):
            unsafe_urls.append(f"receipt:{receipt.get('receipt_id', '<unknown>')}")
    unsafe_urls.sort()
    add(
        "security.https_only",
        "FAIL" if unsafe_urls else "PASS",
        "All source and action URLs must be absolute HTTPS URLs. Unsafe entries: "
        + (", ".join(unsafe_urls) if unsafe_urls else "none."),
    )

    contract = pack.get("decision_contract", {})
    contract_ok = (
        contract.get("guidance_only") is True
        and contract.get("ai_may_create_requirements") is False
        and contract.get("stale_evidence_blocks_completion_claims") is True
        and contract.get("authority_outcome_requires_verifiable_receipt") is True
        and contract.get("user_checkbox_is_not_legal_evidence") is True
    )
    add(
        "safety.decision_contract",
        "PASS" if contract_ok else "FAIL",
        "Guidance-only, AI non-authority, evidence freshness, receipt, and user-assertion guards must all be enabled.",
    )

    blockers: list[str] = []
    source_problems: list[str] = []
    for source in sources:
        if not isinstance(source, dict):
            continue
        source_id = source.get("source_id", "<unknown>")
        state = source.get("review_state")
        checked = parse_datetime(source.get("checked_at"))
        reviewed = parse_datetime(source.get("reviewed_at"))
        next_review = parse_datetime(source.get("next_review_at"))
        effective = parse_datetime(source.get("effective_from"))

        issues: list[str] = []
        if state != "current":
            issues.append(f"review_state={state}")
        if checked is None:
            issues.append("checked_at missing or invalid")
        elif checked > as_of:
            issues.append("checked_at is in the future relative to --as-of")
        if reviewed is None:
            issues.append("reviewed_at missing or invalid")
        elif reviewed > as_of:
            issues.append("reviewed_at is in the future relative to --as-of")
        if checked is not None and reviewed is not None and reviewed < checked:
            issues.append("review predates last source check")
        if not isinstance(source.get("reviewer_role"), str) or not source.get("reviewer_role", "").strip():
            issues.append("reviewer_role missing")
        if next_review is None:
            issues.append("next_review_at missing or invalid")
        else:
            if next_review <= as_of:
                issues.append("review deadline reached")
            policy = pack.get("review_policy", {})
            interval_days = policy.get("review_interval_days")
            if reviewed is not None and isinstance(interval_days, int) and next_review > reviewed + timedelta(days=interval_days):
                issues.append("next_review_at exceeds configured review interval")
        if effective is not None and effective > as_of:
            issues.append("source not yet effective")
        if source.get("reuse") in {"not_assessed", "link_only"} and (
            "source_excerpt" in source or "source_text" in source
        ):
            issues.append("source text present without redistribution rights")
        if issues:
            source_problems.append(f"{source_id}: " + ", ".join(issues))

    for requirement in requirements:
        if not isinstance(requirement, dict):
            continue
        req_id = requirement.get("requirement_id", "<unknown>")
        refs = requirement.get("source_refs", [])
        state = requirement.get("review_state")
        if state in {"draft", "stale", "disputed", "withdrawn"}:
            blockers.append(f"{req_id}: requirement review_state={state}")
        if requirement.get("claim_type") in CURRENT_CLAIM_TYPES and not refs:
            blockers.append(f"{req_id}: current legal/guidance claim has no source")
        for ref in refs:
            source = source_by_id.get(ref)
            if source is None:
                continue
            src_state = source.get("review_state")
            next_review = parse_datetime(source.get("next_review_at"))
            reviewed = parse_datetime(source.get("reviewed_at"))
            checked = parse_datetime(source.get("checked_at"))
            reviewer_role = source.get("reviewer_role")
            if (
                src_state != "current"
                or checked is None
                or reviewed is None
                or not isinstance(reviewer_role, str)
                or not reviewer_role.strip()
                or next_review is None
                or next_review <= as_of
            ):
                blockers.append(
                    f"{req_id}: source {ref} is not reviewed and current as of {evaluated_at}"
                )
            if src_state in TERMINAL_REVIEW_STATES:
                blockers.append(f"{req_id}: source {ref} is {src_state}")

    source_problems = sorted(set(source_problems))
    blockers = sorted(set(blockers + source_problems))
    add(
        "publication.evidence_gates",
        "BLOCKED" if blockers else "PASS",
        "Publication blockers: " + ("; ".join(blockers) if blockers else "none."),
    )
    publication_state = pack.get("publication_state", "draft")
    eligible_state = publication_state in {"candidate", "published"}
    publication_eligible = bool(
        eligible_state
        and not blockers
        and all(check["status"] != "FAIL" for check in checks)
    )

    if publication_eligible:
        gate_status = "PASS"
        gate_details = "Pack passes the publication gates as of the supplied timestamp."
    elif publication_state == "published":
        gate_status = "FAIL"
        gate_details = "Pack is marked published but one or more publication gates are blocked: " + "; ".join(blockers)
    else:
        gate_status = "BLOCKED"
        gate_details = (
            f"Publication is not eligible for state={publication_state}; "
            + ("blockers: " + "; ".join(blockers) if blockers else "change state to candidate only after review.")
        )
    add("publication.gate", gate_status, gate_details)

    report = _report(schema, pack, schema_bytes, pack_bytes, evaluated_at, checks)
    report["publication_eligible"] = publication_eligible
    report["publication_blockers"] = blockers
    return report


def _report(
    schema: dict[str, Any],
    pack: dict[str, Any],
    schema_bytes: bytes,
    pack_bytes: bytes,
    evaluated_at: str,
    checks: list[dict[str, str]],
) -> dict[str, Any]:
    failures = [c for c in checks if c["status"] == "FAIL"]
    return {
        "report_version": "1",
        "validator_version": VALIDATOR_VERSION,
        "schema_version": pack.get("schema_version") if isinstance(pack, dict) else None,
        "pack_id": pack.get("pack_id") if isinstance(pack, dict) else None,
        "pack_version": pack.get("pack_version") if isinstance(pack, dict) else None,
        "evaluated_at": evaluated_at,
        "input_sha256": sha256_hex(pack_bytes),
        "schema_sha256": sha256_hex(schema_bytes),
        "validation_passed": not failures,
        "publication_eligible": False,
        "publication_blockers": [],
        "check_count": len(checks),
        "failed_check_count": len(failures),
        "blocked_check_count": sum(c["status"] == "BLOCKED" for c in checks),
        "checks": checks,
    }


def load_json(path: Path) -> tuple[dict[str, Any], bytes]:
    raw = path.read_bytes()
    value = json.loads(raw.decode("utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return value, raw


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--schema", required=True, type=Path)
    parser.add_argument("--pack", required=True, type=Path)
    parser.add_argument(
        "--as-of",
        required=True,
        help="Frozen timezone-aware ISO-8601 timestamp used for freshness gates.",
    )
    parser.add_argument("--report", type=Path, help="Write the report to this path.")
    parser.add_argument(
        "--require-publishable",
        action="store_true",
        help="Return exit code 2 when the pack is valid but not publication-eligible.",
    )
    args = parser.parse_args(argv)

    as_of = parse_datetime(args.as_of)
    if as_of is None:
        parser.error("--as-of must be a timezone-aware ISO-8601 timestamp")

    try:
        schema, schema_bytes = load_json(args.schema)
        pack, pack_bytes = load_json(args.pack)
        report = validate_pack(
            schema,
            pack,
            schema_bytes=schema_bytes,
            pack_bytes=pack_bytes,
            as_of=as_of,
        )
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
        print(json.dumps({"validation_passed": False, "error": str(exc)}, indent=2), file=sys.stderr)
        return 1

    rendered = json.dumps(report, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    if not report["validation_passed"]:
        return 1
    if args.require_publishable and not report["publication_eligible"]:
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
