#!/usr/bin/env python3
"""Structural policy and artifact-binding checker for recovery attestations.

This is NOT a signature verifier. Use a supported DSSE/Sigstore or SPIFFE-aware
cryptographic verifier first. This tool checks the decoded in-toto Statement,
exact producer policy, raw subject bytes, and source-segment identity/chain.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from datetime import datetime
from pathlib import Path

STATEMENT_TYPE = "https://in-toto.io/Statement/v1"
PREDICATE_TYPE = "https://luminousdynamics.io/attestations/operational-recovery/v0.1"
PAYLOAD_TYPE = "application/vnd.in-toto+json"
PREDICATE_SCHEMA = "luminous-operational-recovery-attestation-v0.1"
POLICY_SCHEMA = "luminous-operational-evidence-publisher-policy-v0.1"
HEX64 = re.compile(r"^[a-f0-9]{64}$")
HEX40 = re.compile(r"^[a-f0-9]{40}$")
KINDS = {"coordinator", "authority_adapter", "participant", "participant_ledger", "independent_observer", "evaluator"}
MAX_DOCUMENT_BYTES = 2 * 1024 * 1024
MAX_MANIFEST_BYTES = 2 * 1024 * 1024
MAX_SEGMENT_BYTES = 8 * 1024 * 1024
MAX_SEGMENT_EVENTS = 4096
MAX_EVENT_LINE_BYTES = 128 * 1024
MAX_ID_BYTES = 256

EVENT_TYPES = {
    "plan_authorized", "dispatch_intent_durable", "participant_accepted", "effect_committed",
    "transport_response_lost", "attempt_status_observed", "effect_reconciled",
    "step_postcondition_verified", "coordinator_restarted", "recovery_scan_completed",
    "service_outcome_verified", "agent_declared_resolved",
}


class PolicyError(ValueError):
    pass


def _reject_constant(value):
    raise PolicyError(f"non-finite JSON number is prohibited: {value}")


def _strict_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise PolicyError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _load_bytes(data: bytes, where: str, *, max_bytes: int = MAX_DOCUMENT_BYTES):
    _need(len(data) <= max_bytes, f"{where}: document exceeds size limit")
    try:
        return json.loads(data.decode("utf-8"), object_pairs_hook=_strict_object, parse_constant=_reject_constant)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise PolicyError(f"{where}: invalid UTF-8 JSON: {exc}") from exc


def _canonical(value: object) -> bytes:
    try:
        return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode("utf-8")
    except (TypeError, ValueError) as exc:
        raise PolicyError(f"not canonically JSON encodable: {exc}") from exc


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _need(condition: bool, message: str):
    if not condition:
        raise PolicyError(message)


def _is_hex(value, pattern=HEX64) -> bool:
    return isinstance(value, str) and pattern.fullmatch(value) is not None


def _no_wildcards(value, where: str):
    if isinstance(value, str):
        _need("*" not in value and "?" not in value, f"{where}: wildcard identity is prohibited")
    elif isinstance(value, dict):
        for key, item in value.items():
            _no_wildcards(item, f"{where}.{key}")
    elif isinstance(value, list):
        for i, item in enumerate(value):
            _no_wildcards(item, f"{where}[{i}]")


def _validate_identity(identity: dict, where: str, trusted_oidc_issuers: set[str], trusted_spiffe_domains: set[str]):
    _need(isinstance(identity, dict), f"{where}: expected identity object")
    _no_wildcards(identity, where)
    profile = identity.get("profile")
    if profile == "sigstore_oidc":
        _need(set(identity) == {"profile", "oidc_issuer", "oidc_subject"}, f"{where}: OIDC identity fields must be exact")
        issuer, subject = identity["oidc_issuer"], identity["oidc_subject"]
        _need(isinstance(issuer, str) and issuer.startswith("https://"), f"{where}: HTTPS OIDC issuer required")
        _need(isinstance(subject, str) and len(subject) > 3, f"{where}: exact OIDC subject required")
        _need(issuer in trusted_oidc_issuers, f"{where}: OIDC issuer is not in the pinned trust allowlist")
    elif profile == "spiffe_x509_svid":
        _need(set(identity) == {"profile", "spiffe_id", "spiffe_trust_domain", "trust_bundle_sha256"}, f"{where}: SPIFFE identity fields must be exact")
        spiffe_id, trust_domain = identity["spiffe_id"], identity["spiffe_trust_domain"]
        _need(isinstance(spiffe_id, str) and spiffe_id.startswith("spiffe://"), f"{where}: SPIFFE ID required")
        remainder = spiffe_id[len("spiffe://"):]
        _need("/" in remainder and remainder.split("/", 1)[0] == trust_domain, f"{where}: SPIFFE trust domain mismatch")
        _need(trust_domain in trusted_spiffe_domains, f"{where}: SPIFFE trust domain is not in the pinned trust allowlist")
        _need(_is_hex(identity["trust_bundle_sha256"]), f"{where}: exact trust-bundle digest required")
    else:
        raise PolicyError(f"{where}: unsupported identity profile")


def _validate_policy(policy: dict) -> dict:
    _need(isinstance(policy, dict), "policy: expected object")
    _need(set(policy) == {"schema", "version", "status", "policy_id", "payload_type", "statement_type", "predicate_type", "trusted_oidc_issuers", "trusted_spiffe_trust_domains", "verification_requirements", "publishers"}, "policy: unexpected or missing top-level fields")
    _need(policy.get("schema") == POLICY_SCHEMA and type(policy.get("version")) is int and policy["version"] == 1, "policy: unsupported schema/version")
    _need(isinstance(policy.get("status"), str) and policy["status"] in {"example_only", "research_policy"}, "policy: invalid status")
    _need(isinstance(policy.get("policy_id"), str) and policy["policy_id"], "policy: policy_id required")
    _need(policy.get("payload_type") == PAYLOAD_TYPE, "policy: DSSE payload type mismatch")
    _need(policy.get("statement_type") == STATEMENT_TYPE, "policy: in-toto statement type mismatch")
    _need(policy.get("predicate_type") == PREDICATE_TYPE, "policy: predicate type mismatch")
    trusted_oidc_issuers = policy.get("trusted_oidc_issuers")
    trusted_spiffe_domains = policy.get("trusted_spiffe_trust_domains")
    _need(isinstance(trusted_oidc_issuers, list) and trusted_oidc_issuers and all(isinstance(x, str) for x in trusted_oidc_issuers) and len(trusted_oidc_issuers) == len(set(trusted_oidc_issuers)), "policy: exact trusted OIDC issuer allowlist required")
    _need(all(isinstance(x, str) and x.startswith("https://") for x in trusted_oidc_issuers), "policy: trusted OIDC issuers must be exact HTTPS identities")
    _need(isinstance(trusted_spiffe_domains, list) and trusted_spiffe_domains and all(isinstance(x, str) for x in trusted_spiffe_domains) and len(trusted_spiffe_domains) == len(set(trusted_spiffe_domains)), "policy: exact SPIFFE trust-domain allowlist required")
    _no_wildcards(trusted_oidc_issuers, "policy.trusted_oidc_issuers")
    _no_wildcards(trusted_spiffe_domains, "policy.trusted_spiffe_trust_domains")
    requirements = policy.get("verification_requirements")
    expected_requirements = {
        "require_signature": True,
        "require_exact_identity": True,
        "require_trusted_issuer": True,
        "require_certificate_valid_at_signing_time": True,
        "revocation_unavailable_result": "indeterminate",
        "require_transparency_inclusion": True,
        "require_transparency_checkpoint_consistency": True,
    }
    _need(requirements == expected_requirements, "policy: verification requirements must remain fail-closed")
    publishers = policy.get("publishers")
    _need(isinstance(publishers, list) and publishers, "policy: non-empty publishers required")
    by_role, identities, profiles = {}, set(), set()
    for i, publisher in enumerate(publishers):
        where = f"policy.publishers[{i}]"
        _need(isinstance(publisher, dict), f"{where}: expected object")
        required = {"source_id", "source_kind", "trust_domain_id", "identity_profile_id", "identity", "allowed_event_types", "transparency"}
        _need(set(publisher) == required, f"{where}: publisher fields must be exact")
        for field in ("source_id", "trust_domain_id", "identity_profile_id"):
            _need(isinstance(publisher[field], str) and publisher[field], f"{where}.{field}: required")
        _need(isinstance(publisher["source_kind"], str) and publisher["source_kind"] in KINDS, f"{where}.source_kind: unsupported")
        _need(publisher["identity_profile_id"] not in profiles, f"{where}: duplicate identity_profile_id")
        profiles.add(publisher["identity_profile_id"])
        key = (publisher["source_id"], publisher["source_kind"], publisher["trust_domain_id"])
        _need(key not in by_role, f"{where}: duplicate producer role mapping")
        by_role[key] = publisher
        _validate_identity(publisher["identity"], where + ".identity", set(trusted_oidc_issuers), set(trusted_spiffe_domains))
        identity_key = json.dumps(publisher["identity"], sort_keys=True, separators=(",", ":"))
        _need(identity_key not in identities, f"{where}: identity is reused across producer roles")
        identities.add(identity_key)
        allowed = publisher["allowed_event_types"]
        _need(isinstance(allowed, list) and allowed and all(isinstance(x, str) and x in EVENT_TYPES for x in allowed) and len(allowed) == len(set(allowed)), f"{where}: unique supported event types required")
        transparency = publisher["transparency"]
        _need(isinstance(transparency, dict) and set(transparency) == {"mode", "failure_action", "log_policy_id"} and transparency.get("mode") == "required" and transparency.get("failure_action") == "reject", f"{where}: transparency inclusion must fail closed with exact fields")
        _need(isinstance(transparency.get("log_policy_id"), str) and transparency["log_policy_id"], f"{where}: log policy id required")
    return by_role


def check_attestation(statement: dict, policy: dict, manifest_bytes: bytes, segment_bytes: bytes, policy_bytes: bytes) -> dict:
    publisher_map = _validate_policy(policy)
    _need(isinstance(statement, dict), "statement: expected object")
    _need(set(statement) == {"_type", "subject", "predicateType", "predicate"}, "statement: unexpected or missing top-level fields")
    _need(statement.get("_type") == STATEMENT_TYPE, "statement: wrong in-toto Statement type")
    _need(statement.get("predicateType") == PREDICATE_TYPE, "statement: wrong predicate type")
    subjects = statement.get("subject")
    _need(isinstance(subjects, list) and len(subjects) == 2, "statement: exactly manifest and event-segment subjects required")
    subject_map = {}
    for i, subject in enumerate(subjects):
        _need(isinstance(subject, dict) and set(subject) == {"name", "digest"}, f"statement.subject[{i}]: exact fields required")
        _need(isinstance(subject["name"], str) and subject["name"], f"statement.subject[{i}]: subject name required")
        digest = subject["digest"]
        _need(isinstance(digest, dict) and set(digest) == {"sha256"} and _is_hex(digest["sha256"]), f"statement.subject[{i}]: SHA-256 digest required")
        _need(subject["name"] not in subject_map, "statement: duplicate subject name")
        subject_map[subject["name"]] = digest["sha256"]

    predicate = statement.get("predicate")
    _need(isinstance(predicate, dict), "statement.predicate: expected object")
    _need(set(predicate) == {"schema", "version", "assertion_scope", "produced_at", "publisher_policy_sha256", "run_binding", "artifact_binding", "producer", "source_segment"}, "predicate: unexpected or missing fields")
    _need(predicate.get("schema") == PREDICATE_SCHEMA and type(predicate.get("version")) is int and predicate["version"] == 1, "predicate: unsupported schema/version")
    _need(predicate.get("publisher_policy_sha256") == _sha(policy_bytes), "predicate: publisher policy digest mismatch")
    _need(predicate.get("assertion_scope") == "producer_published_source_event_segment", "predicate: claim exceeds producer-segment scope")
    produced_at = predicate.get("produced_at")
    try:
        parsed_time = datetime.fromisoformat(produced_at.replace("Z", "+00:00"))
    except (AttributeError, ValueError) as exc:
        raise PolicyError("predicate.produced_at: invalid timestamp") from exc
    _need(parsed_time.tzinfo is not None, "predicate.produced_at: timezone required")

    artifact_binding = predicate.get("artifact_binding")
    _need(isinstance(artifact_binding, dict) and set(artifact_binding) == {"manifest_subject_name", "event_segment_subject_name"}, "predicate.artifact_binding: exact fields required")
    manifest_name = artifact_binding.get("manifest_subject_name")
    segment_name = artifact_binding.get("event_segment_subject_name")
    _need(isinstance(manifest_name, str) and manifest_name and isinstance(segment_name, str) and segment_name, "predicate.artifact_binding: subject names must be non-empty strings")
    _need(manifest_name in subject_map and segment_name in subject_map and manifest_name != segment_name, "predicate: subject names do not bind manifest and event segment")
    _need(subject_map[manifest_name] == _sha(manifest_bytes), "statement: raw manifest subject digest mismatch")
    _need(subject_map[segment_name] == _sha(segment_bytes), "statement: raw event segment subject digest mismatch")

    _need(len(manifest_bytes) <= MAX_MANIFEST_BYTES, "manifest: document exceeds size limit")
    _need(len(policy_bytes) <= MAX_DOCUMENT_BYTES, "publisher policy: document exceeds size limit")
    _need(len(segment_bytes) <= MAX_SEGMENT_BYTES, "event segment: document exceeds size limit")
    manifest = _load_bytes(manifest_bytes, "manifest", max_bytes=MAX_MANIFEST_BYTES)
    _need(isinstance(manifest, dict), "manifest: expected object")
    _need(manifest.get("schema") == "luminous-operational-recovery-run-v0.1" and type(manifest.get("version")) is int and manifest["version"] == 1, "manifest: unsupported schema/version")
    run_binding = predicate.get("run_binding")
    _need(isinstance(run_binding, dict) and set(run_binding) == {"run_id", "episode_id", "plan_digest", "source_commit_sha"}, "predicate.run_binding: exact fields required")
    frozen = manifest.get("run_binding")
    _need(isinstance(frozen, dict), "manifest.run_binding: expected object")
    for key in ("run_id", "episode_id", "plan_digest", "source_commit_sha"):
        _need(isinstance(frozen.get(key), str) and frozen[key], f"manifest.run_binding.{key}: required")
        _need(run_binding.get(key) == frozen.get(key), f"predicate.run_binding.{key}: does not match frozen manifest")
    _need(len(frozen["run_id"].encode("utf-8")) <= MAX_ID_BYTES and len(frozen["episode_id"].encode("utf-8")) <= MAX_ID_BYTES, "manifest run/episode identity exceeds size limit")
    _need(_is_hex(frozen["plan_digest"]) and _is_hex(frozen["source_commit_sha"], HEX40), "manifest run binding digests are malformed")

    producer = predicate.get("producer")
    _need(isinstance(producer, dict) and set(producer) == {"source_id", "source_kind", "trust_domain_id", "identity_profile_id"}, "predicate.producer: exact fields required")
    _need(all(isinstance(producer.get(k), str) and producer[k] for k in ("source_id", "source_kind", "trust_domain_id", "identity_profile_id")), "predicate.producer: exact non-empty identity fields required")
    role = (producer.get("source_id"), producer.get("source_kind"), producer.get("trust_domain_id"))
    _need(role in publisher_map, "publisher policy: no exact identity is authorized for this producer role")
    publisher = publisher_map[role]
    _need(producer.get("identity_profile_id") == publisher["identity_profile_id"], "publisher policy: identity profile mismatch")

    lines = segment_bytes.splitlines()
    _need(lines, "event segment: empty source stream")
    _need(len(lines) <= MAX_SEGMENT_EVENTS, "event segment: event count exceeds limit")
    _need(all(len(line) <= MAX_EVENT_LINE_BYTES for line in lines), "event segment: event line exceeds size limit")
    events = [_load_bytes(line, f"event segment line {i}", max_bytes=MAX_EVENT_LINE_BYTES) for i, line in enumerate(lines, 1)]
    source = producer["source_id"]
    source_kind = producer["source_kind"]
    trust_domain = producer["trust_domain_id"]
    stream = predicate.get("source_segment")
    _need(isinstance(stream, dict) and set(stream) == {"source_epoch", "first_sequence", "last_sequence", "event_count", "first_event_digest", "last_event_digest"}, "predicate.source_segment: exact fields required")
    epoch = stream.get("source_epoch")
    _need(type(epoch) is int and epoch >= 0, "source_segment.source_epoch: invalid")
    event_ids, previous_digest, previous_id = set(), None, None
    for i, event in enumerate(events, 1):
        _need(isinstance(event, dict), f"event {i}: expected object")
        _need(event.get("schema") == "luminous-operational-recovery-event-v0.1", f"event {i}: unsupported event schema")
        _need(event.get("run_id") == frozen.get("run_id") and event.get("episode_id") == frozen.get("episode_id") and event.get("plan_digest") == frozen.get("plan_digest"), f"event {i}: frozen run binding mismatch")
        _need(isinstance(event.get("event_id"), str) and event["event_id"] and len(event["event_id"].encode("utf-8")) <= MAX_ID_BYTES, f"event {i}: event ID required and size-bounded")
        _need(type(event.get("source_epoch")) is int and type(event.get("source_sequence")) is int, f"event {i}: integer epoch and sequence required")
        parent_ids = event.get("parent_event_ids")
        _need(isinstance(parent_ids, list) and all(isinstance(x, str) and x for x in parent_ids) and len(parent_ids) == len(set(parent_ids)), f"event {i}: unique causal parent IDs required")
        _need(event.get("source_id") == source and event.get("source_kind") == source_kind and event.get("trust_domain_id") == trust_domain, f"event {i}: producer role substitution")
        _need(event.get("source_epoch") == epoch and event.get("source_sequence") == i, f"event {i}: source epoch/sequence gap")
        _need(event.get("event_type") in EVENT_TYPES, f"event {i}: unsupported event type")
        _need(event.get("event_type") in publisher["allowed_event_types"], f"event {i}: event type not authorized for publisher role")
        _need(event.get("event_id") not in event_ids, f"event {i}: duplicate event id")
        event_ids.add(event.get("event_id"))
        _need(event.get("previous_event_digest") == previous_digest, f"event {i}: source hash-chain predecessor mismatch")
        if previous_id is not None:
            _need(previous_id in event.get("parent_event_ids", []), f"event {i}: missing happens-before edge to prior source event")
        unsigned = {k: v for k, v in event.items() if k != "event_digest"}
        calculated = _sha(_canonical(unsigned))
        _need(event.get("event_digest") == calculated, f"event {i}: event digest mismatch")
        previous_digest, previous_id = calculated, event.get("event_id")

    _need(type(stream.get("first_sequence")) is int and stream.get("first_sequence") == 1, "source segment must begin at sequence one")
    _need(type(stream.get("last_sequence")) is int and stream.get("last_sequence") == len(events) and type(stream.get("event_count")) is int and stream.get("event_count") == len(events), "source segment must cover the complete declared epoch prefix")
    _need(stream.get("first_event_digest") == events[0].get("event_digest"), "source segment first digest mismatch")
    _need(stream.get("last_event_digest") == events[-1].get("event_digest"), "source segment last digest mismatch")
    return {
        "status": "policy_match",
        "policy_id": policy["policy_id"],
        "policy_status": policy["status"],
        "production_usable": False,
        "producer_role": {"source_id": source, "source_kind": source_kind, "trust_domain_id": trust_domain},
        "identity_profile_id": publisher["identity_profile_id"],
        "event_count": len(events),
        "manifest_sha256": _sha(manifest_bytes),
        "event_segment_sha256": _sha(segment_bytes),
        "publisher_policy_sha256": _sha(policy_bytes),
        "cryptographic_signature_verification_performed": False,
        "transparency_inclusion_verification_performed": False,
        "causal_episode_semantics_verification_performed": False,
        "claim_ceiling": "Exact content binding and publisher-policy mapping only; DSSE signature, certificate/identity, transparency and #174 semantic verification are separate required gates.",
    }


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--statement", type=Path, required=True)
    parser.add_argument("--policy", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--event-segment", type=Path, required=True)
    parser.add_argument("--output", default="-")
    args = parser.parse_args(argv)
    try:
        statement = _load_bytes(args.statement.read_bytes(), str(args.statement))
        policy_bytes = args.policy.read_bytes()
        policy = _load_bytes(policy_bytes, str(args.policy))
        report = check_attestation(statement, policy, args.manifest.read_bytes(), args.event_segment.read_bytes(), policy_bytes)
        rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
        if args.output == "-":
            sys.stdout.write(rendered)
        else:
            Path(args.output).write_text(rendered, encoding="utf-8")
        return 0
    except (OSError, PolicyError, AttributeError, KeyError, TypeError, OverflowError) as exc:
        print(f"publisher-policy-check: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
