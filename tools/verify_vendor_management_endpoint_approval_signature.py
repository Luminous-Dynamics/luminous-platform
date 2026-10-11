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
import re
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

TRUST_SCHEMA_PATH = ROOT / "schemas/vendor-management-approval-trust-store-v1.schema.json"
SIGNATURE_SCHEME = "ed25519-rfc8032-luminous-approval-v1"
SIGNING_DOMAIN = b"LUMINOUS:VENDOR-MANAGEMENT-ENDPOINT-APPROVAL:V1\x00"
SIGNATURE_BYTES = 64
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
EXCLUDED_PAYLOAD_FIELDS = frozenset({"signed_payload_digest", "signature_evidence_ref"})


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


def strict_json_loads(data: bytes | str) -> Any:
    """Reject duplicate keys, floating-point values and non-finite numbers."""
    return json.loads(
        data,
        object_pairs_hook=_unique_object,
        parse_float=_reject_float,
        parse_constant=_reject_constant,
    )


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False
    ).encode("utf-8", errors="strict")


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
        schema = strict_json_loads(TRUST_SCHEMA_PATH.read_bytes())
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
    except (UnicodeEncodeError, TypeError, ValueError):
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


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--approval", type=Path, required=True)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--signature-file", type=Path, required=True,
                        help="Raw detached Ed25519 signature (exactly 64 bytes).")
    parser.add_argument("--trust-store", type=Path, required=True)
    parser.add_argument("--trust-store-sha256", required=True,
                        help="64-character digest supplied by an independently protected channel.")
    parser.add_argument("--approval-schema", type=Path,
                        default=ROOT / "schemas/vendor-management-endpoint-approval-v1.schema.json")
    parser.add_argument("--profile-schema", type=Path,
                        default=ROOT / "schemas/vendor-management-endpoint-profile-v1.schema.json")
    args = parser.parse_args(argv)

    try:
        approval = strict_json_loads(args.approval.read_bytes())
        profile = strict_json_loads(args.profile.read_bytes())
        approval_schema = strict_json_loads(args.approval_schema.read_bytes())
        profile_schema = strict_json_loads(args.profile_schema.read_bytes())
        signature = args.signature_file.read_bytes()
        trust_store_raw = args.trust_store.read_bytes()
        binding_errors = approval_errors(approval, approval_schema, profile, profile_schema)
    except (OSError, UnicodeDecodeError, ValueError, TypeError):
        print("FAIL: INPUT_INVALID")
        return 2

    if binding_errors:
        print("FAIL: APPROVAL_PROFILE_BINDING_INVALID")
        return 1

    code = verify_approval_signature(
        approval=approval,
        signature=signature,
        trust_store_raw=trust_store_raw,
        expected_trust_store_sha256=args.trust_store_sha256,
        as_of=datetime.now(timezone.utc),
    )
    if code:
        print("FAIL: " + code)
        return 1

    print(
        "VERIFIED: signature, externally pinned signer trust, key/principal revocation "
        "timing, and approval time window; operational authorization NOT evaluated"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
