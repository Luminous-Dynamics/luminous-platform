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
EVENT_TYPES = {
    "plan_authorized", "dispatch_intent_durable", "participant_accepted", "effect_committed",
    "transport_response_lost", "attempt_status_observed", "effect_reconciled",
    "step_postcondition_verified", "coordinator_restarted", "recovery_scan_completed",
    "service_outcome_verified", "agent_declared_resolved",
}


class PolicyError(ValueError):
    pass


def _strict_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise PolicyError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _load_bytes(data: bytes, where: str):
    try:
        return json.loads(data.decode("utf-8"), object_pairs_hook=_strict_object)
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


def _validate_identity(identity: dict, where: str):
    _need(isinstance(identity, dict), f"{where}: expected identity object")
    _no_wildcards(identity, where)
    profile = identity.get("profile")
    if profile == "sigstore_oidc":
        _need(set(identity) == {"profile", "oidc_issuer", "oidc_subject"}, f"{where}: OIDC identity fields must be exact")
        issuer, subject = identity["oidc_issuer"], identity["oidc_subject"]
        _need(isinstance(issuer, str) and issuer.startswith("https://"), f"{where}: HTTPS OIDC issuer required")
        _need(isinstance(subject, str) and len(subject) > 3, f"{where}: exact OIDC subject required")
    elif profile == "spiffe_x509_svid":
        _need(set(identity) == {"profile", "spiffe_id", "spiffe_trust_domain", "trust_bundle_sha256"}, f"{where}: SPIFFE identity fields must be exact")
        spiffe_id, trust_domain = identity["spiffe_id"], identity["spiffe_trust_domain"]
        _need(isinstance(spiffe_id, str) and spiffe_id.startswith("spiffe://"), f"{where}: SPIFFE ID required")
        remainder = spiffe_id[len("spiffe://"):]
        _need("/" in remainder and remainder.split("/", 1)[0] == trust_domain, f"{where}: SPIFFE trust domain mismatch")
        _need(_is_hex(identity["trust_bundle_sha256"]), f"{where}: exact trust-bundle digest required")
    else:
        raise PolicyError(f"{where}: unsupported identity profile")


def _validate_policy(policy: dict) -> dict:
    _need(isinstance(policy, dict), "policy: expected object")
    _need(policy.get("schema") == POLICY_SCHEMA and type(policy.get("version")) is int and policy["version"] == 1, "policy: unsupported schema/version")
    _need(policy.get("status") in {"example_only", "research_policy"}, "policy: invalid status")
    _need(isinstance(policy.get("policy_id"), str) and policy["policy_id"], "policy: policy_id required")
    _need(policy.get("payload_type") == PAYLOAD_TYPE, "policy: DSSE payload type mismatch")
    _need(policy.get("statement_type") == STATEMENT_TYPE, "policy: in-toto statement type mismatch")
    _need(policy.get("predicate_type") == PREDICATE_TYPE, "policy: predicate type mismatch")
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
        _need(publisher["source_kind"] in KINDS, f"{where}.source_kind: unsupported")
        _need(publisher["identity_profile_id"] not in profiles, f"{where}: duplicate identity_profile_id")
        profiles.add(publisher["identity_profile_id"])
        key = (publisher["source_id"], publisher["source_kind"], publisher["trust_domain_id"])
        _need(key not in by_role, f"{where}: duplicate producer role mapping")
        by_role[key] = publisher
        _validate_identity(publisher["identity"], where + ".identity")
        identity_key = json.dumps(publisher["identity"], sort_keys=True, separators=(",", ":"))
        _need(identity_key not in identities, f"{where}: identity is reused across producer roles")
        identities.add(identity_key)
        allowed = publisher["allowed_event_types"]
        _need(isinstance(allowed, list) and allowed and len(allowed) == len(set(allowed)), f"{where}: unique allowed event types required")
        _need(all(isinstance(x, str) and x for x in allowed), f"{where}: invalid allowed event type")
        transparency = publisher["transparency"]
        _need(isinstance(transparency, dict) and transparency.get("mode") == "required" and transparency.get("failure_action") == "reject", f"{where}: transparency inclusion must fail closed")
        _need(isinstance(transparency.get("log_policy_id"), str) and transparency["log_policy_id"], f"{where}: log policy id required")
    return by_role


def check_attestation(statement: dict, policy: dict, manifest_bytes: bytes, segment_bytes: bytes) -> dict:
    publisher_map = _validate_policy(policy)
    _need(isinstance(statement, dict), "statement: expected object")
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
    _need(predicate.get("schema") == PREDICATE_SCHEMA and type(predicate.get("version")) is int and predicate["version"] == 1, "predicate: unsupported schema/version")
    _need(predicate.get("assertion_scope") == "producer_published_source_event_segment", "predicate: claim exceeds producer-segment scope")
    produced_at = predicate.get("produced_at")
    try:
        parsed_time = datetime.fromisoformat(produced_at.replace("Z", "+00:00"))
    except (AttributeError, ValueError) as exc:
        raise PolicyError("predicate.produced_at: invalid timestamp") from exc
    _need(parsed_time.tzinfo is not None, "predicate.produced_at: timezone required")

    manifest_name = predicate.get("artifact_binding", {}).get("manifest_subject_name")
    segment_name = predicate.get("artifact_binding", {}).get("event_segment_subject_name")
    _need(manifest_name in subject_map and segment_name in subject_map and manifest_name != segment_name, "predicate: subject names do not bind manifest and event segment")
    _need(subject_map[manifest_name] == _sha(manifest_bytes), "statement: raw manifest subject digest mismatch")
    _need(subject_map[segment_name] == _sha(segment_bytes), "statement: raw event segment subject digest mismatch")

    manifest = _load_bytes(manifest_bytes, "manifest")
    run_binding = predicate.get("run_binding")
    _need(isinstance(run_binding, dict), "predicate.run_binding: expected object")
    frozen = manifest.get("run_binding", {})
    for key in ("run_id", "episode_id", "plan_digest", "source_commit_sha"):
        _need(run_binding.get(key) == frozen.get(key), f"predicate.run_binding.{key}: does not match frozen manifest")

    producer = predicate.get("producer")
    _need(isinstance(producer, dict), "predicate.producer: expected object")
    role = (producer.get("source_id"), producer.get("source_kind"), producer.get("trust_domain_id"))
    _need(role in publisher_map, "publisher policy: no exact identity is authorized for this producer role")
    publisher = publisher_map[role]
    _need(producer.get("identity_profile_id") == publisher["identity_profile_id"], "publisher policy: identity profile mismatch")

    lines = segment_bytes.splitlines()
    _need(lines, "event segment: empty source stream")
    events = [_load_bytes(line, f"event segment line {i}") for i, line in enumerate(lines, 1)]
    source = producer["source_id"]
    source_kind = producer["source_kind"]
    trust_domain = producer["trust_domain_id"]
    stream = predicate.get("source_segment")
    _need(isinstance(stream, dict), "predicate.source_segment: expected object")
    epoch = stream.get("source_epoch")
    _need(type(epoch) is int and epoch >= 0, "source_segment.source_epoch: invalid")
    event_ids, previous_digest, previous_id = set(), None, None
    for i, event in enumerate(events, 1):
        _need(isinstance(event, dict), f"event {i}: expected object")
        _need(event.get("source_id") == source and event.get("source_kind") == source_kind and event.get("trust_domain_id") == trust_domain, f"event {i}: producer role substitution")
        _need(event.get("source_epoch") == epoch and event.get("source_sequence") == i, f"event {i}: source epoch/sequence gap")
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

    _need(stream.get("first_sequence") == 1, "source segment must begin at sequence one")
    _need(stream.get("last_sequence") == len(events) and stream.get("event_count") == len(events), "source segment must cover the complete declared epoch prefix")
    _need(stream.get("first_event_digest") == events[0].get("event_digest"), "source segment first digest mismatch")
    _need(stream.get("last_event_digest") == events[-1].get("event_digest"), "source segment last digest mismatch")
    return {
        "status": "policy_match",
        "policy_id": policy["policy_id"],
        "producer_role": {"source_id": source, "source_kind": source_kind, "trust_domain_id": trust_domain},
        "identity_profile_id": publisher["identity_profile_id"],
        "event_count": len(events),
        "manifest_sha256": _sha(manifest_bytes),
        "event_segment_sha256": _sha(segment_bytes),
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
        policy = _load_bytes(args.policy.read_bytes(), str(args.policy))
        report = check_attestation(statement, policy, args.manifest.read_bytes(), args.event_segment.read_bytes())
        rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
        if args.output == "-":
            sys.stdout.write(rendered)
        else:
            Path(args.output).write_text(rendered, encoding="utf-8")
        return 0
    except (OSError, PolicyError) as exc:
        print(f"publisher-policy-check: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
