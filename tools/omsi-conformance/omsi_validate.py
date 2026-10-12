"""OMSI 0.1 draft conformance checks. Does not verify identities or signatures."""
from __future__ import annotations
import argparse
import hashlib
import json
import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parent
SCHEMAS = {
    "operation": ROOT / "schemas/0.1/operation-manifest.schema.json",
    "grant": ROOT / "schemas/0.1/authority-grant.schema.json",
    "event": ROOT / "schemas/0.1/operation-event.schema.json",
    "receipt": ROOT / "schemas/0.1/evidence-receipt.schema.json",
}
TRANSITIONS = {
    "none": {"proposed"}, "proposed": {"validated", "rejected", "cancelled", "expired"},
    "validated": {"authorized", "rejected", "cancelled", "expired"},
    "authorized": {"accepted", "rejected", "cancelled", "expired"},
    "accepted": {"running", "failed", "cancelled", "indeterminate"},
    "running": {"applied", "failed", "cancel_requested", "indeterminate"},
    "cancel_requested": {"cancelled", "running", "indeterminate"},
    "applied": {"verified", "verification_failed", "indeterminate"},
}

class ConformanceError(ValueError):
    pass

def strict_loads(raw: str) -> Any:
    def pairs(items):
        out = {}
        for k, v in items:
            if k in out:
                raise ConformanceError(f"duplicate JSON key: {k}")
            out[k] = v
        return out
    def integer(raw_number: str) -> int:
        if raw_number == "-0":
            raise ConformanceError("negative zero is forbidden in OMSI canonical JSON")
        return int(raw_number)
    def constant(value: str):
        raise ConformanceError(f"non-JSON numeric constant is forbidden: {value}")
    return json.loads(raw, object_pairs_hook=pairs, parse_int=integer, parse_constant=constant)

def load(path: str | Path) -> Any:
    return strict_loads(Path(path).read_text(encoding="utf-8"))

def parse_time(value: str) -> datetime:
    if not isinstance(value, str):
        raise ConformanceError("timestamp must be a string")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ConformanceError(f"invalid timestamp: {value}") from exc
    if parsed.tzinfo is None:
        raise ConformanceError("timestamp must include a timezone")
    return parsed.astimezone(timezone.utc)

def _normalise(value: Any, path: str = "/") -> Any:
    if isinstance(value, dict):
        for key in value:
            try:
                key.encode("utf-16be", errors="strict")
            except UnicodeEncodeError as exc:
                raise ConformanceError(f"invalid Unicode key at {path}") from exc
        return {k: _normalise(value[k], f"{path}{k}/") for k in sorted(value, key=lambda x: x.encode("utf-16be"))}
    if isinstance(value, list):
        return [_normalise(v, f"{path}[{i}]") for i, v in enumerate(value)]
    if isinstance(value, float):
        raise ConformanceError(f"floating-point values are forbidden in OMSI-JCS-0.1 at {path}")
    if isinstance(value, int) and not isinstance(value, bool) and abs(value) > 9007199254740991:
        raise ConformanceError(f"integer exceeds JavaScript safe range at {path}")
    if isinstance(value, str):
        try:
            value.encode("utf-8", errors="strict")
        except UnicodeEncodeError as exc:
            raise ConformanceError(f"invalid Unicode string at {path}") from exc
    return value

def canonical_bytes(value: Any) -> bytes:
    try:
        return json.dumps(_normalise(value), ensure_ascii=False, separators=(",", ":"), allow_nan=False).encode("utf-8")
    except (TypeError, ValueError, UnicodeEncodeError) as exc:
        if isinstance(exc, ConformanceError):
            raise
        raise ConformanceError(f"not canonically serializable: {exc}") from exc

def digest(value: Any, domain: str) -> str:
    """Hash canonical JSON with a fixed, explicit semantic domain separator."""
    try:
        domain_bytes = domain.encode("ascii", errors="strict")
    except (AttributeError, UnicodeEncodeError) as exc:
        raise ConformanceError("digest domain must be a stable ASCII identifier") from exc
    if not domain_bytes or b"\x00" in domain_bytes:
        raise ConformanceError("digest domain must be non-empty and cannot contain NUL")
    payload = b"OMSI-DIGEST/0.1\x00" + domain_bytes + b"\x00" + canonical_bytes(value)
    return "sha256:" + hashlib.sha256(payload).hexdigest()

ACTION_ARGUMENTS_DOMAIN = "org.omsi.action-arguments/0.1"
OPERATION_MANIFEST_DOMAIN = "org.omsi.operation-manifest/0.1"

def _reject_secrets(value: Any, path: str = "/") -> None:
    bad = {"password", "passwd", "passphrase", "secret", "privatekey", "accesstoken", "refreshtoken", "apikey", "clientsecret", "credential", "credentials"}
    # Reference fields are not an exemption: only opaque URI-shaped references
    # are allowed. Reject raw values, userinfo, query strings and whitespace.
    reference_uri = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*://[^\s/?#@][^\s?#@]*$")
    if isinstance(value, dict):
        for key, child in value.items():
            norm = key.lower().replace("-", "_")
            compact = norm.replace("_", "")
            if norm in {"secret_refs", "credential_refs"}:
                if not isinstance(child, list):
                    raise ConformanceError(f"reference field at {path}{key} must be an array of opaque URIs")
                for i, ref in enumerate(child):
                    if not isinstance(ref, str) or not reference_uri.fullmatch(ref):
                        raise ConformanceError(f"invalid secret/credential reference at {path}{key}[{i}]")
                continue
            if compact in bad or norm.endswith(("_password", "_private_key", "_access_token", "_refresh_token", "_api_key", "_client_secret")):
                raise ConformanceError(f"secret-like field at {path}{key}; use an out-of-band reference")
            _reject_secrets(child, f"{path}{key}/")
    elif isinstance(value, list):
        for i, child in enumerate(value):
            _reject_secrets(child, f"{path}[{i}]/")

def validate_schema(kind: str, value: Any) -> None:
    try:
        schema = load(SCHEMAS[kind])
    except KeyError as exc:
        raise ConformanceError(f"unknown document kind: {kind}") from exc
    Draft202012Validator.check_schema(schema)
    # All contract documents must be serializable by the documented restricted
    # canonical JSON profile; schema-valid floats/unsafe integers are not enough.
    canonical_bytes(value)
    errors = sorted(Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(value), key=lambda e: list(map(str, e.absolute_path)))
    if errors:
        error = errors[0]
        at = "/" + "/".join(map(str, error.absolute_path)) if error.absolute_path else "/"
        raise ConformanceError(f"{kind} schema error at {at}: {error.message}")

def validate_operation(op: dict[str, Any], now: datetime | None = None) -> None:
    validate_schema("operation", op)
    canonical_bytes(op)
    _reject_secrets(op)
    if op["tenant_id"] != op["target"]["tenant_id"]:
        raise ConformanceError("operation and target tenant differ")
    if op["action"]["arguments_digest"] != digest(op["action"]["arguments"], ACTION_ARGUMENTS_DOMAIN):
        raise ConformanceError("action argument digest mismatch")
    created = parse_time(op["created_at"])
    observed = parse_time(op["preconditions"]["observed_at"])
    expiry = parse_time(op["authorization"]["expires_at"])
    if expiry <= created:
        raise ConformanceError("authorization expiry must be after creation")
    if observed > created:
        raise ConformanceError("precondition observation is from the future")
    if (created - observed).total_seconds() > op["preconditions"]["max_age_seconds"]:
        raise ConformanceError("precondition observation is stale")
    if now is not None and expiry <= now.astimezone(timezone.utc):
        raise ConformanceError("operation authorization window has expired")

def validate_grant(op: dict[str, Any], grant: dict[str, Any], now: datetime | None = None) -> None:
    validate_operation(op, now=now)
    validate_schema("grant", grant)
    _reject_secrets(grant)
    pairs = {
        "tenant_id": (grant["tenant_id"], op["tenant_id"]),
        "target_resource_id": (grant["target_resource_id"], op["target"]["resource_id"]),
        "subject_manifest_digest": (grant["subject_manifest_digest"], digest(op, OPERATION_MANIFEST_DOMAIN)),
        "action_profile": (grant["action_profile"], op["action"]["profile"]),
        "action_verb": (grant["action_verb"], op["action"]["verb"]),
        "authorized_principal": (grant["authorized_principal"], op["authorization"]["authorized_principal"]),
        "audience": (grant["audience"], op["authorization"]["audience"]),
        "nonce": (grant["nonce"], op["authorization"]["nonce"]),
        "policy_ref": (grant["policy_ref"], op["authorization"]["policy_ref"]),
        "correlation_id": (grant["correlation_id"], op["correlation_id"]),
        "approval_refs": (grant["approval_refs"], op["authorization"]["approval_refs"]),
    }
    for key, (actual, expected) in pairs.items():
        if actual != expected:
            raise ConformanceError(f"grant {key} does not match operation")
    issued = parse_time(grant["issued_at"])
    not_before = parse_time(grant.get("not_before", grant["issued_at"]))
    expiry = parse_time(grant["expires_at"])
    if issued < parse_time(op["created_at"]):
        raise ConformanceError("grant cannot predate the operation manifest")
    if not_before < issued or expiry <= not_before:
        raise ConformanceError("invalid grant validity interval")
    if expiry > parse_time(op["authorization"]["expires_at"]):
        raise ConformanceError("grant exceeds manifest expiry")
    if now is not None and not (not_before <= now.astimezone(timezone.utc) < expiry):
        raise ConformanceError("grant is not currently valid")
    # Intentional limitation: proof shape is schema-checked, but signature bytes,
    # issuer trust, key status and revocation are NOT verified by this starter.

def validate_receipt(
    receipt: dict[str, Any],
    expected: dict[str, Any] | None = None,
    events: list[dict[str, Any]] | None = None,
) -> None:
    validate_schema("receipt", receipt)
    _reject_secrets(receipt)
    if receipt["tenant_id"] != receipt["target"]["tenant_id"]:
        raise ConformanceError("receipt target is in another tenant")
    if events is not None and expected is None:
        raise ConformanceError("event-bound receipt validation requires the operation manifest")
    if expected is not None:
        validate_operation(expected)
        expected_values = [
            ("manifest digest", receipt["subject_manifest_digest"], digest(expected, OPERATION_MANIFEST_DOMAIN)),
            ("tenant", receipt["tenant_id"], expected["tenant_id"]),
            ("operation ID", receipt["operation_id"], expected["id"]),
            ("target", receipt["target"]["resource_id"], expected["target"]["resource_id"]),
            ("correlation ID", receipt["correlation_id"], expected["correlation_id"]),
        ]
        for key, actual, want in expected_values:
            if actual != want:
                raise ConformanceError(f"receipt {key} mismatch")
    if events is not None:
        # Validate and bind the full event chain before using its timestamps.
        validate_events(events, expected)

    issued = parse_time(receipt["issued_at"])
    manifest_created = parse_time(expected["created_at"]) if expected is not None else None
    if manifest_created is not None and issued < manifest_created:
        raise ConformanceError("receipt cannot predate the operation manifest")

    ver = receipt["verification"]
    observed_raw = ver.get("observed_at")
    observed = parse_time(observed_raw) if observed_raw is not None else None
    if observed is not None and observed > issued:
        raise ConformanceError("verification observation cannot follow receipt issuance")
    if manifest_created is not None and observed is not None and observed < manifest_created:
        raise ConformanceError("verification observation cannot predate the operation manifest")
    if events is not None and observed is not None:
        applied_events = [event for event in events if event["to_state"] == "applied"]
        if applied_events and observed < parse_time(applied_events[-1]["occurred_at"]):
            raise ConformanceError("verification observation predates operation application")

    if receipt["outcome"] == "verified":
        if ver["result"] != "verified":
            raise ConformanceError("verified outcome needs verified evidence")
        if observed is None or not ver.get("post_state_digest"):
            raise ConformanceError("verified receipt requires observation time and post-state digest")
        if events is not None:
            applied = [event for event in events if event["to_state"] == "applied"]
            if not applied:
                raise ConformanceError("verified receipt requires an applied event")
            if not events or events[-1]["to_state"] != "verified":
                raise ConformanceError("verified receipt requires a terminal verified event")
            verified_at = parse_time(events[-1]["occurred_at"])
            if verified_at < observed:
                raise ConformanceError("verified event predates verification observation")
            if issued < verified_at:
                raise ConformanceError("receipt issuance predates verified event")
        if ver.get("independent_verifier_required"):
            if not ver.get("verifier_identity") or ver.get("verifier_identity") == receipt["executor"]:
                raise ConformanceError("independent verifier must be identified separately from executor")
            if not ver.get("executor_authority_domain") or not ver.get("verifier_authority_domain") or ver["executor_authority_domain"] == ver["verifier_authority_domain"]:
                raise ConformanceError("independent verification requires distinct declared authority domains")
    elif ver["result"] == "verified":
        raise ConformanceError("non-verified outcome cannot claim verified evidence")
    if events is not None and receipt["outcome"] != "verified" and events[-1]["to_state"] == "verified":
        raise ConformanceError("non-verified receipt conflicts with verified event sequence")

def validate_events(events: list[dict[str, Any]], operation: dict[str, Any] | None = None) -> None:
    if not isinstance(events, list) or not events:
        raise ConformanceError("event sequence must be a non-empty array")
    # Validate every record before dereferencing identifiers; malformed input
    # must fail as a contract error, never leak KeyError/TypeError to the CLI.
    for event in events:
        validate_schema("event", event)
        _reject_secrets(event)
    if operation is not None:
        validate_operation(operation)
    seen = set()
    first = events[0]
    if operation is not None:
        for key, actual, expected in (
            ("operation_id", first["operation_id"], operation["id"]),
            ("tenant_id", first["tenant_id"], operation["tenant_id"]),
            ("correlation_id", first["correlation_id"], operation["correlation_id"]),
        ):
            if actual != expected:
                raise ConformanceError(f"event sequence {key} does not match manifest")
    prev_state = None
    prev_time = None
    for seq, event in enumerate(events, 1):
        for key in ("tenant_id", "operation_id", "correlation_id"):
            if event[key] != first[key]:
                raise ConformanceError(f"event sequence mixes {key}")
        if event["event_id"] in seen:
            raise ConformanceError("duplicate event id")
        seen.add(event["event_id"])
        if event["sequence"] != seq:
            raise ConformanceError("event sequence must be contiguous from 1")
        at = parse_time(event["occurred_at"])
        if operation is not None and at < parse_time(operation["created_at"]):
            raise ConformanceError("event cannot predate the operation manifest")
        if prev_time is not None and at < prev_time:
            raise ConformanceError("event timestamps must be nondecreasing")
        if prev_state is None and event["from_state"] != "none":
            raise ConformanceError("first event must start from none")
        if prev_state is not None and event["from_state"] != prev_state:
            raise ConformanceError("event state chain is broken")
        if event["to_state"] not in TRANSITIONS.get(event["from_state"], set()):
            raise ConformanceError(f"illegal transition {event['from_state']} -> {event['to_state']}")
        prev_state, prev_time = event["to_state"], at

def main() -> int:
    parser = argparse.ArgumentParser(description="Validate OMSI 0.1 draft documents; no crypto verification")
    parser.add_argument("kind", choices=["operation", "grant", "receipt", "events"])
    parser.add_argument("document", help="JSON path")
    parser.add_argument("--manifest", help="operation manifest for grant/receipt/event binding")
    parser.add_argument("--events", help="lifecycle event JSON for cross-document receipt chronology checks")
    parser.add_argument("--now", help="explicit RFC3339 time for expiry checks")
    args = parser.parse_args()
    try:
        if args.events and args.kind != "receipt":
            raise ConformanceError("--events is only valid with receipt")
        doc = load(args.document)
        now = parse_time(args.now) if args.now else None
        if args.kind == "operation":
            if now is None:
                raise ConformanceError("--now is required for operation validation")
            validate_operation(doc, now)
        elif args.kind == "grant":
            if not args.manifest or now is None:
                raise ConformanceError("grant requires --manifest and --now")
            validate_grant(load(args.manifest), doc, now)
        elif args.kind == "receipt":
            if args.events and not args.manifest:
                raise ConformanceError("--events requires --manifest")
            validate_receipt(
                doc,
                load(args.manifest) if args.manifest else None,
                load(args.events) if args.events else None,
            )
        else:
            validate_events(doc, load(args.manifest) if args.manifest else None)
        print(f"PASS: {args.kind} document passed OMSI 0.1 draft checks")
        return 0
    except (OSError, json.JSONDecodeError, ConformanceError) as exc:
        print(f"FAIL: {exc}")
        return 1

if __name__ == "__main__":
    raise SystemExit(main())
