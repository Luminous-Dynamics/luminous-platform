#!/usr/bin/env python3
"""Verify a vendor endpoint approval signature against an externally pinned trust store.

Success proves a current Ed25519 signature and signer/key time-window assessment only.
It does not establish fresh target identity, decision-revocation completeness, or
operational authorization. The trust-store digest must come from a protected channel.
"""
from __future__ import annotations

import argparse
import base64
import binascii
import hashlib
import json
import os
import re
import stat
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey
from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
TOOL_DIR = ROOT / "tools"
if str(TOOL_DIR) not in sys.path:
    sys.path.insert(0, str(TOOL_DIR))

from validate_vendor_management_endpoint_approval import (  # noqa: E402
    approval_errors,
    canonical_profile_digest,
)
from validate_vendor_adapter_registry import registry_errors  # noqa: E402

TRUST_SCHEMA_PATH = ROOT / "schemas/vendor-management-approval-trust-store-v1.schema.json"
SIGNATURE_SCHEME = "ed25519-rfc8032-luminous-approval-v1"
SIGNING_DOMAIN = b"LUMINOUS:VENDOR-MANAGEMENT-ENDPOINT-APPROVAL:V1\x00"
SIGNATURE_BYTES = 64
MAX_JSON_INPUT_BYTES = 4 * 1024 * 1024
MAX_SIGNATURE_INPUT_BYTES = SIGNATURE_BYTES
MAX_JSON_NESTING = 128
MAX_SAFE_JSON_INTEGER = 9_007_199_254_740_991
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
EXCLUDED_PAYLOAD_FIELDS = frozenset({"signed_payload_digest", "signature_evidence_ref"})


def _ensure_safe_json_integer_domain(value: Any) -> None:
    """Enforce numeric and nesting limits without recursive traversal."""
    pending: list[tuple[Any, int]] = [(value, 0)]
    while pending:
        current, depth = pending.pop()
        if type(current) is int:
            if abs(current) > MAX_SAFE_JSON_INTEGER:
                raise ValueError("integer outside safe interoperable JSON integer range")
        elif isinstance(current, dict):
            if depth >= MAX_JSON_NESTING:
                raise ValueError("JSON nesting exceeds configured limit")
            pending.extend((nested, depth + 1) for nested in current.values())
        elif isinstance(current, list):
            if depth >= MAX_JSON_NESTING:
                raise ValueError("JSON nesting exceeds configured limit")
            pending.extend((nested, depth + 1) for nested in current)


def read_bounded_file(path: Path, max_bytes: int = MAX_JSON_INPUT_BYTES) -> bytes:
    """Read a small regular file without following its final symlink or trusting its size."""
    if type(max_bytes) is not int or max_bytes < 0:
        raise ValueError("invalid input byte limit")
    if not hasattr(os, "O_NOFOLLOW") or not hasattr(os, "O_NONBLOCK"):
        raise OSError("safe bounded file reads are unsupported on this platform")

    flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | getattr(os, "O_CLOEXEC", 0)
    descriptor = os.open(path, flags)
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode):
            raise ValueError("input is not a regular file")
        if before.st_size > max_bytes:
            raise ValueError("input exceeds configured byte limit")

        chunks: list[bytes] = []
        bytes_read = 0
        while True:
            chunk = os.read(descriptor, min(64 * 1024, max_bytes + 1 - bytes_read))
            if not chunk:
                break
            chunks.append(chunk)
            bytes_read += len(chunk)
            if bytes_read > max_bytes:
                raise ValueError("input exceeds configured byte limit")

        after = os.fstat(descriptor)
        stable_fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
        if bytes_read != before.st_size or any(
            getattr(before, field) != getattr(after, field) for field in stable_fields
        ):
            raise ValueError("input changed during bounded read")
        return b"".join(chunks)
    finally:
        os.close(descriptor)


def _parse_integer(value: str) -> int:
    # RFC 8785's verified erratum warns that -0 canonicalizes to 0 and SHOULD
    # be rejected at parse time to prevent ambiguity with unparsed source bytes.
    if value == "-0":
        raise ValueError("negative zero is outside the canonical contract")
    parsed = int(value)
    _ensure_safe_json_integer_domain(parsed)
    return parsed


def _reject_float(value: str) -> None:
    raise ValueError("floating-point JSON values are outside the canonical contract")


def _reject_constant(value: str) -> None:
    raise ValueError("non-finite JSON number is outside the canonical contract")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON object member")
        result[key] = value
    return result


def _enforce_json_nesting_limit(data: bytes | str) -> None:
    """Reject excessive structural nesting before the standard parser recurses."""
    text = data.decode("utf-8", errors="strict") if isinstance(data, bytes) else data
    depth = 0
    in_string = False
    escaped = False
    for char in text:
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char in "[{":
            depth += 1
            if depth > MAX_JSON_NESTING:
                raise ValueError("JSON nesting exceeds configured limit")
        elif char in "]}":
            # Malformed closing delimiters are left to json.loads; keep this
            # preflight scanner from underflowing on invalid input.
            depth = max(0, depth - 1)


def strict_json_loads(data: bytes | str) -> Any:
    """Reject duplicate keys, unsafe numbers, and excessive JSON nesting."""
    _enforce_json_nesting_limit(data)
    try:
        return json.loads(
            data,
            object_pairs_hook=_unique_object,
            parse_int=_parse_integer,
            parse_float=_reject_float,
            parse_constant=_reject_constant,
        )
    except RecursionError:
        raise ValueError("JSON nesting exceeds parser limits") from None


def canonical_json_bytes(value: Any) -> bytes:
    _ensure_safe_json_integer_domain(value)
    try:
        return json.dumps(
            value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False
        ).encode("utf-8", errors="strict")
    except RecursionError:
        raise ValueError("JSON nesting exceeds canonicalization limits") from None


def signature_message_bytes(approval: dict[str, Any]) -> bytes:
    """Return domain-separated canonical signed bytes, excluding circular metadata.

    The signer key reference and signature scheme are included. Only the digest of
    these bytes and the digest reference of the detached signature are excluded.
    """
    payload = {
        key: value for key, value in approval.items()
        if key not in EXCLUDED_PAYLOAD_FIELDS
    }
    return SIGNING_DOMAIN + canonical_json_bytes(payload)


def parse_time(value: Any) -> datetime | None:
    if not isinstance(value, str):
        return None
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        return None
    return parsed.astimezone(timezone.utc)


def _trust_store_errors(store: Any) -> list[str]:
    if not isinstance(store, dict):
        return ["TRUST_STORE_SCHEMA_INVALID"]
    try:
        schema = strict_json_loads(read_bounded_file(TRUST_SCHEMA_PATH))
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema, format_checker=FormatChecker())
        if any(validator.iter_errors(store)):
            return ["TRUST_STORE_SCHEMA_INVALID"]
    except Exception:
        return ["TRUST_STORE_SCHEMA_INVALID"]

    key_refs = [item["key_ref"] for item in store["keys"]]
    principal_refs = [item["principal_ref"] for item in store["principals"]]
    if len(key_refs) != len(set(key_refs)) or len(principal_refs) != len(set(principal_refs)):
        return ["TRUST_STORE_DUPLICATE_REFERENCE"]
    key_ref_set = set(key_refs)
    for principal in store["principals"]:
        if not set(principal["key_refs"]).issubset(key_ref_set):
            return ["TRUST_STORE_KEY_REFERENCE_UNRESOLVED"]
    for item in [*store["keys"], *store["principals"]]:
        valid_from = parse_time(item["valid_from"])
        valid_until = parse_time(item["valid_until"])
        if valid_from is None or valid_until is None or valid_until <= valid_from:
            return ["TRUST_STORE_VALIDITY_INTERVAL_INVALID"]
        if item["state"] == "revoked" and parse_time(item["revoked_at"]) is None:
            return ["TRUST_STORE_REVOCATION_TIME_INVALID"]
    return []


def _is_current(item: dict[str, Any], now: datetime) -> bool:
    valid_from = parse_time(item["valid_from"])
    valid_until = parse_time(item["valid_until"])
    return valid_from is not None and valid_until is not None and valid_from <= now < valid_until


def _revoked_at_or_before(item: dict[str, Any], now: datetime) -> bool:
    if item["state"] != "revoked":
        return False
    revoked_at = parse_time(item["revoked_at"])
    return revoked_at is not None and revoked_at <= now


def verify_approval_signature(
    approval: dict[str, Any],
    signature: bytes,
    trust_store_raw: bytes,
    expected_trust_store_sha256: str,
    expected_trust_store_revision: int,
    as_of: datetime,
) -> str | None:
    """Return a stable failure code, or None for valid signature/trust/time checks.

    The caller must validate approval schema and profile binding separately. This
    function does not treat a valid signature as an operational authorization grant.
    """
    if as_of.tzinfo is None or as_of.utcoffset() is None:
        return "AS_OF_TIME_INVALID"
    if not SHA256_RE.fullmatch(expected_trust_store_sha256):
        return "TRUST_STORE_DIGEST_PIN_INVALID"
    if hashlib.sha256(trust_store_raw).hexdigest() != expected_trust_store_sha256:
        return "TRUST_STORE_DIGEST_MISMATCH"
    try:
        trust_store = strict_json_loads(trust_store_raw)
    except (UnicodeDecodeError, ValueError, TypeError):
        return "TRUST_STORE_JSON_INVALID"
    store_errors = _trust_store_errors(trust_store)
    if store_errors:
        return store_errors[0]
    if not isinstance(expected_trust_store_revision, int) or isinstance(expected_trust_store_revision, bool) or expected_trust_store_revision < 1:
        return "TRUST_STORE_REVISION_PIN_INVALID"
    if trust_store["revision"] != expected_trust_store_revision:
        return "TRUST_STORE_REVISION_PIN_MISMATCH"

    if not isinstance(approval, dict) or approval.get("state") != "approved":
        return "DECISION_NOT_APPROVED"
    if approval.get("signature_scheme_ref") != SIGNATURE_SCHEME:
        return "SIGNATURE_SCHEME_UNSUPPORTED"
    if not isinstance(signature, bytes) or len(signature) != SIGNATURE_BYTES:
        return "SIGNATURE_LENGTH_INVALID"
    evidence_ref = "sha256:" + hashlib.sha256(signature).hexdigest()
    if approval.get("signature_evidence_ref") != evidence_ref:
        return "SIGNATURE_EVIDENCE_DIGEST_MISMATCH"

    not_before = parse_time(approval.get("not_before"))
    expires_at = parse_time(approval.get("expires_at"))
    now = as_of.astimezone(timezone.utc)
    if not_before is None or expires_at is None or expires_at <= not_before:
        return "APPROVAL_TIME_WINDOW_INVALID"
    if not_before > now or now >= expires_at:
        return "APPROVAL_NOT_CURRENT"

    try:
        message = signature_message_bytes(approval)
    except (UnicodeEncodeError, TypeError, ValueError, RecursionError):
        return "SIGNED_PAYLOAD_CANONICALIZATION_FAILED"
    if approval.get("signed_payload_digest") != hashlib.sha256(message).hexdigest():
        return "SIGNED_PAYLOAD_DIGEST_MISMATCH"

    approver_ref = approval.get("approver_ref")
    principal_matches = [item for item in trust_store["principals"] if item["principal_ref"] == approver_ref]
    if len(principal_matches) != 1:
        return "APPROVER_NOT_TRUSTED"
    principal = principal_matches[0]
    if "endpoint_approval_signer" not in principal["roles"]:
        return "APPROVER_ROLE_MISSING"
    if _revoked_at_or_before(principal, now):
        return "APPROVER_REVOKED_AT_AS_OF"
    if not _is_current(principal, now):
        return "APPROVER_NOT_CURRENT"

    key_ref = approval.get("signing_key_ref")
    if key_ref not in principal["key_refs"]:
        return "KEY_NOT_DELEGATED_TO_APPROVER"
    key_matches = [item for item in trust_store["keys"] if item["key_ref"] == key_ref]
    if len(key_matches) != 1:
        return "SIGNING_KEY_NOT_TRUSTED"
    key_record = key_matches[0]
    if _revoked_at_or_before(key_record, now):
        return "SIGNING_KEY_REVOKED_AT_AS_OF"
    if not _is_current(key_record, now):
        return "SIGNING_KEY_NOT_CURRENT"

    try:
        public_key_bytes = base64.b64decode(key_record["public_key_base64"], validate=True)
        public_key = Ed25519PublicKey.from_public_bytes(public_key_bytes)
        public_key.verify(signature, message)
    except (binascii.Error, ValueError):
        return "SIGNING_KEY_ENCODING_INVALID"
    except InvalidSignature:
        return "SIGNATURE_INVALID"
    return None



_MATURITY_LEVELS = {
    "A0_DISCOVERED": 0,
    "A1_READ_ONLY_TESTED": 1,
    "A2_STATE_CONTRACT_TESTED": 2,
    "A3_CHANGE_QUALIFIED": 3,
    "A4_FIRMWARE_RECOVERY_QUALIFIED": 4,
    "A5_SERVICEABLE": 5,
}


def capability_scope_error_codes(
    approval: dict[str, Any],
    profile: dict[str, Any],
    registry: dict[str, Any],
) -> list[str]:
    """Bind signed read operations to exact, tested adapter and hardware/commercial scope.

    This is a capability gate, not an operational authorizer. The CLI separately pins
    the registry's raw bytes from a protected channel before using this result.
    """
    errors: list[str] = []
    if approval.get("state") != "approved":
        return ["DECISION_NOT_APPROVED"]

    profile_adapter = profile.get("adapter", {})
    target = approval.get("target", {})
    adapter_id = profile_adapter.get("adapter_id")
    candidates = [
        item for item in registry.get("adapters", [])
        if item.get("adapter_id") == adapter_id
    ]
    if len(candidates) != 1:
        return ["CAPABILITY_ADAPTER_NOT_UNIQUE"]
    adapter = candidates[0]

    if adapter.get("vendor") != profile_adapter.get("vendor"):
        errors.append("CAPABILITY_VENDOR_MISMATCH")
    family = profile_adapter.get("product_family")
    if family not in adapter.get("product_families", []):
        errors.append("CAPABILITY_PRODUCT_FAMILY_MISMATCH")
    if adapter.get("model_scope_state") != "exact_models_tested":
        errors.append("CAPABILITY_EXACT_MODEL_SCOPE_REQUIRED")
    if adapter.get("implementation_state") not in {"LAB_TESTED", "QUALIFIED"}:
        errors.append("CAPABILITY_IMPLEMENTATION_NOT_TESTED")
    if _MATURITY_LEVELS.get(adapter.get("maturity"), 0) < _MATURITY_LEVELS["A1_READ_ONLY_TESTED"]:
        errors.append("CAPABILITY_MATURITY_TOO_LOW")
    if adapter.get("region_scope_state") not in {"region_scoped", "global_scope_assessed"}:
        errors.append("CAPABILITY_REGION_SCOPE_NOT_ASSESSED")

    region = profile.get("scope", {}).get("hardware_or_commercial_region", {})
    region_value = region.get("value")
    if region.get("state") != "assessed" or not isinstance(region_value, str) or not region_value.strip():
        errors.append("CAPABILITY_HARDWARE_REGION_ASSESSMENT_REQUIRED")

    operations = adapter.get("operations", [])
    requested_operations = approval.get("allowed_operations", [])
    if not requested_operations:
        errors.append("CAPABILITY_OPERATION_SCOPE_EMPTY")
    for requested in requested_operations:
        matches = [item for item in operations if item.get("operation") == requested]
        if len(matches) != 1:
            errors.append("CAPABILITY_OPERATION_NOT_UNIQUE")
            continue
        operation = matches[0]
        if operation.get("state") != "tested":
            errors.append("CAPABILITY_OPERATION_NOT_TESTED")
            continue
        tested_scope = operation.get("tested_scope")
        if not isinstance(tested_scope, dict):
            errors.append("CAPABILITY_TESTED_SCOPE_MISSING")
            continue
        expected_scope = {
            "model": target.get("model"),
            "product_family": family,
            "hardware_revision": target.get("hardware_revision"),
            "software_version": target.get("software_version"),
            "region": region_value,
        }
        if any(tested_scope.get(key) != value for key, value in expected_scope.items()):
            errors.append("CAPABILITY_TESTED_SCOPE_MISMATCH")
        if operation.get("tested_revision") != profile_adapter.get("adapter_revision"):
            errors.append("CAPABILITY_TESTED_REVISION_MISMATCH")

    return sorted(set(errors))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--approval", type=Path, required=True)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--signature-file", type=Path, required=True,
                        help="Raw detached Ed25519 signature (exactly 64 bytes).")
    parser.add_argument("--trust-store", type=Path, required=True)
    parser.add_argument("--trust-store-sha256", required=True,
                        help="64-character digest supplied by an independently protected channel.")
    parser.add_argument("--trust-store-revision", required=True, type=int,
                        help="Monotonic trust-store revision pinned by an independently protected channel.")
    parser.add_argument("--capability-registry", type=Path, required=True)
    parser.add_argument("--capability-registry-sha256", required=True,
                        help="Registry digest supplied by an independently protected channel.")
    parser.add_argument("--capability-schema", type=Path,
                        default=ROOT / "schemas/vendor-adapter-capability-registry-v1.schema.json")
    parser.add_argument("--approval-schema", type=Path,
                        default=ROOT / "schemas/vendor-management-endpoint-approval-v1.schema.json")
    parser.add_argument("--profile-schema", type=Path,
                        default=ROOT / "schemas/vendor-management-endpoint-profile-v1.schema.json")
    args = parser.parse_args(argv)

    try:
        approval = strict_json_loads(read_bounded_file(args.approval))
        profile = strict_json_loads(read_bounded_file(args.profile))
        approval_schema = strict_json_loads(read_bounded_file(args.approval_schema))
        profile_schema = strict_json_loads(read_bounded_file(args.profile_schema))
        trust_store_raw = read_bounded_file(args.trust_store)
        capability_registry_raw = read_bounded_file(args.capability_registry)
        capability_schema = strict_json_loads(read_bounded_file(args.capability_schema))
        capability_registry = strict_json_loads(capability_registry_raw)
        binding_errors = approval_errors(approval, approval_schema, profile, profile_schema)
    except (OSError, UnicodeDecodeError, ValueError, TypeError):
        print("FAIL: INPUT_INVALID")
        return 2

    try:
        signature = read_bounded_file(args.signature_file, MAX_SIGNATURE_INPUT_BYTES)
    except (OSError, ValueError):
        print("FAIL: SIGNATURE_FILE_INVALID")
        return 2

    if binding_errors:
        print("FAIL: APPROVAL_PROFILE_BINDING_INVALID")
        return 1

    if not SHA256_RE.fullmatch(args.capability_registry_sha256):
        print("FAIL: CAPABILITY_REGISTRY_DIGEST_PIN_INVALID")
        return 1
    if hashlib.sha256(capability_registry_raw).hexdigest() != args.capability_registry_sha256:
        print("FAIL: CAPABILITY_REGISTRY_DIGEST_MISMATCH")
        return 1
    try:
        registry_contract_errors = registry_errors(capability_registry, capability_schema)
    except Exception:
        print("FAIL: CAPABILITY_REGISTRY_INVALID")
        return 1
    if registry_contract_errors:
        print("FAIL: CAPABILITY_REGISTRY_INVALID")
        return 1

    code = verify_approval_signature(
        approval=approval,
        signature=signature,
        trust_store_raw=trust_store_raw,
        expected_trust_store_sha256=args.trust_store_sha256,
        expected_trust_store_revision=args.trust_store_revision,
        as_of=datetime.now(timezone.utc),
    )
    if code:
        print("FAIL: " + code)
        return 1

    capability_codes = capability_scope_error_codes(approval, profile, capability_registry)
    if capability_codes:
        print("FAIL: " + ",".join(capability_codes))
        return 1

    print(
        "VERIFIED: signature and exact externally pinned tested read-capability scope; "
        "fresh target identity, complete decision revocation, and operational authorization "
        "NOT evaluated"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
