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
from typing import Any\nfrom urllib.parse import urlsplit

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

    all_unique = (
        len(source_ids) == len(sources)
        and len(requirement_ids) == len(requirements)
        and len(action_ids) == sum(
            len(req.get("user_actions", []))
            for req in requirements if isinstance(req, dict)
        )
        and _unique_check(source_ids)
        and _unique_check(requirement_ids)
        and _unique_check(action_ids)
    )
    add(
        "pack.unique_identifiers",
        "PASS" if all_unique else "FAIL",
        "Source, requirement, and action identifiers must be present and unique within the pack.",
    )

    source_by_id = {
        s.get("source_id"): s for s in sources if isinstance(s, dict)
    }
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
        if parsed.scheme != "https" or not parsed.hostname:
            unsafe_urls.append(f"source:{source_id}")
    for requirement in requirements:
        if not isinstance(requirement, dict):
            continue
        req_id = requirement.get("requirement_id", "<unknown>")
        for action in requirement.get("user_actions", []):
            if not isinstance(action, dict) or not action.get("url"):
                continue
            parsed = urlsplit(action["url"])
            if parsed.scheme != "https" or not parsed.hostname:
                unsafe_urls.append(f"action:{req_id}:{action.get('action_id', '<unknown>')}")
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
