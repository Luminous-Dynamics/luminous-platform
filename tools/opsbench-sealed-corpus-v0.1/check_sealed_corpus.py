#!/usr/bin/env python3
"""Verify sealed OpsBench corpus commitments and inspect evaluator access audit.

This is a structural/integrity checker. It does not authenticate the publisher
of a registration record or access log, enforce IAM/network controls, or prove
that the audit collector observed every access path.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from datetime import datetime
from pathlib import Path

DOMAIN = b"luminous-opsbench-sealed-corpus-v0.1\x00"
MAX_DOCUMENT_BYTES = 16 * 1024 * 1024
MAX_ITEMS = 100_000
MAX_AUDIT_EVENTS = 2_000_000
MAX_EVENT_LINE = 64 * 1024
REQUIRED_COVERAGE = {"filesystem", "network", "tool_calls", "process", "secrets", "artifact_store"}


class ContractError(ValueError):
    pass


def _strict_object(pairs):
    out = {}
    for key, value in pairs:
        if key in out:
            raise ContractError(f"duplicate JSON key: {key}")
        out[key] = value
    return out


def _reject_constant(value):
    raise ContractError(f"non-finite JSON number is prohibited: {value}")


def load_json(data: bytes, label: str, *, limit: int = MAX_DOCUMENT_BYTES) -> dict:
    if len(data) > limit:
        raise ContractError(f"{label}: document exceeds {limit} byte limit")
    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=_strict_object, parse_constant=_reject_constant)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ContractError(f"{label}: invalid UTF-8 JSON: {exc}") from exc
    if not isinstance(value, dict):
        raise ContractError(f"{label}: top-level value must be an object")
    return value


def require(condition: bool, message: str):
    if not condition:
        raise ContractError(message)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def parse_time(value, where: str) -> datetime:
    require(isinstance(value, str), f"{where}: timestamp string required")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ContractError(f"{where}: invalid timestamp") from exc
    require(parsed.tzinfo is not None, f"{where}: timezone offset required")
    return parsed


def _hex64(value, where: str):
    require(isinstance(value, str) and len(value) == 64 and all(c in "0123456789abcdef" for c in value), f"{where}: lowercase SHA-256 digest required")


def check_corpus_commitment(manifest: dict, manifest_bytes: bytes, registration: dict, salt: bytes, preregistration_bytes: bytes) -> dict:
    require(manifest.get("schema") == "luminous-opsbench-sealed-corpus-manifest-v0.1" and type(manifest.get("version")) is int and manifest["version"] == 1, "manifest: unsupported schema/version")
    require(registration.get("schema") == "luminous-opsbench-sealed-corpus-registration-v0.1" and type(registration.get("version")) is int and registration["version"] == 1, "registration: unsupported schema/version")
    require(registration.get("commitment_scheme") == "sha256-domain-separated-secret-salt-v0.1", "registration: unsupported commitment scheme")
    require(registration.get("commitment_domain") in (None, "luminous-opsbench-sealed-corpus-v0.1"), "registration: unexpected commitment domain")
    require(len(salt) >= 32, "salt: minimum 32 bytes required")
    require(manifest.get("study_id") == registration.get("study_id"), "manifest/registration study ID mismatch")
    require(manifest.get("preregistration_sha256") == registration.get("preregistration_sha256"), "manifest/registration preregistration digest mismatch")
    _hex64(registration.get("preregistration_sha256"), "registration.preregistration_sha256")
    require(registration.get("preregistration_sha256") == sha256(preregistration_bytes), "registration: exact preregistration digest mismatch")
    preregistration = load_json(preregistration_bytes, "preregistration")
    require(preregistration.get("study_id") == manifest.get("study_id"), "manifest/preregistration study ID mismatch")
    expected_preregistration_status = {
        "example_only": "draft_not_registered",
        "sealed_before_trials": "registered_before_trials",
        "revealed_after_study": "completed",
    }[manifest.get("status")]
    require(preregistration.get("status") == expected_preregistration_status, "preregistration status does not match corpus status")
    raw_manifest_digest = sha256(manifest_bytes)
    require(registration.get("manifest_sha256") == raw_manifest_digest, "registration: exact raw manifest digest mismatch")
    require(registration.get("salt_sha256") == sha256(salt), "registration: salt digest mismatch")
    commitment = sha256(DOMAIN + salt + b"\x00" + manifest_bytes)
    require(registration.get("commitment_sha256") == commitment, "registration: salted corpus commitment mismatch")

    manifest_status = manifest.get("status")
    registration_status = registration.get("status")
    require(manifest_status in {"example_only", "sealed_before_trials", "revealed_after_study"}, "manifest: invalid status")
    require(registration_status in {"example_only", "sealed_before_trials", "revealed_after_study"}, "registration: invalid status")
    require(manifest_status == registration_status, "manifest/registration status mismatch")
    registered_at = parse_time(registration.get("registered_at"), "registration.registered_at")
    first_trial_at = parse_time(registration.get("first_trial_at"), "registration.first_trial_at")
    require(registered_at < first_trial_at, "registration must precede first scored trial")
    if registration_status == "revealed_after_study":
        revealed_at = parse_time(registration.get("revealed_at"), "registration.revealed_at")
        require(revealed_at >= first_trial_at, "post-study reveal timestamp precedes first trial")

    generator = manifest.get("generator")
    require(isinstance(generator, dict), "manifest.generator: expected object")
    require(isinstance(generator.get("version"), str) and generator["version"], "manifest.generator.version required")
    require(isinstance(generator.get("source_commit_sha"), str) and len(generator["source_commit_sha"]) == 40, "manifest.generator.source_commit_sha must be a Git SHA")
    _hex64(generator.get("recipe_sha256"), "manifest.generator.recipe_sha256")
    _hex64(generator.get("evaluator_sha256"), "manifest.generator.evaluator_sha256")
    require(manifest.get("split_unit") == "fault_mechanism_and_topology_clusters", "manifest: split must be by fault mechanism and topology clusters")
    target = manifest.get("confirmatory_scenario_count_target")
    require(type(target) is int and target >= 500, "manifest: confirmatory target must be at least 500 scenarios")
    items = manifest.get("items")
    require(isinstance(items, list) and 1 <= len(items) <= MAX_ITEMS, "manifest.items outside supported bounds")

    keys, split_by_scenario, split_by_family, split_by_topology = set(), {}, {}, {}
    split_counts = {"development": set(), "screening": set(), "confirmatory": set()}
    for index, item in enumerate(items):
        where = f"manifest.items[{index}]"
        require(isinstance(item, dict), f"{where}: expected object")
        required = {
            "scenario_id", "variant_id", "split", "fault_family_group_id", "topology_group_id",
            "task_prompt_sha256", "answer_key_sha256", "oracle_sha256", "environment_sha256",
            "policy_sha256", "tool_surface_sha256", "license_id",
        }
        require(set(item) == required, f"{where}: unexpected or missing fields")
        for field in ("scenario_id", "variant_id", "fault_family_group_id", "topology_group_id", "license_id"):
            require(isinstance(item[field], str) and item[field], f"{where}.{field}: non-empty string required")
        split = item["split"]
        require(split in split_counts, f"{where}.split: invalid split")
        key = (item["scenario_id"], item["variant_id"])
        require(key not in keys, f"{where}: duplicate scenario/variant key")
        keys.add(key)
        previous_split = split_by_scenario.setdefault(item["scenario_id"], split)
        require(previous_split == split, f"{where}: one scenario ID appears in multiple splits")
        family_split = split_by_family.setdefault(item["fault_family_group_id"], split)
        require(family_split == split, f"{where}: fault-family group leakage across splits")
        topology_split = split_by_topology.setdefault(item["topology_group_id"], split)
        require(topology_split == split, f"{where}: topology group leakage across splits")
        split_counts[split].add(item["scenario_id"])
        for field in ("task_prompt_sha256", "answer_key_sha256", "oracle_sha256", "environment_sha256", "policy_sha256", "tool_surface_sha256"):
            _hex64(item[field], f"{where}.{field}")

    if manifest_status != "example_only":
        require(len(split_counts["confirmatory"]) >= target, "manifest: confirmatory scenario count below committed target")
        require(all(split_counts[name] for name in ("development", "screening", "confirmatory")), "manifest: all three splits must be represented")
    return {
        "status": "commitment_match",
        "study_id": manifest["study_id"],
        "manifest_sha256": raw_manifest_digest,
        "preregistration_sha256": sha256(preregistration_bytes),
        "commitment_sha256": commitment,
        "salt_sha256": sha256(salt),
        "split_scenario_counts": {key: len(value) for key, value in split_counts.items()},
        "item_count": len(items),
        "example_only": manifest_status == "example_only",
        "commitment_publisher_authenticity_verified": False,
        "manifest_privacy_verified": False,
        "claim_ceiling": "Exact byte commitment and split-structure checks only; does not authenticate the registration publisher, prove custody/secrecy, or establish real IAM/network enforcement.",
    }


def check_access_audit(audit: dict, policy: dict) -> dict:
    require(audit.get("schema") == "luminous-opsbench-evaluator-access-audit-v0.1" and type(audit.get("version")) is int and audit["version"] == 1, "audit: unsupported schema/version")
    require(policy.get("schema") == "luminous-opsbench-evaluator-access-policy-v0.1" and type(policy.get("version")) is int and policy["version"] == 1, "access policy: unsupported schema/version")
    require(isinstance(policy.get("policy_id"), str) and policy["policy_id"], "access policy.policy_id required")
    forbidden = policy.get("sut_forbidden_resource_classes")
    allowed = policy.get("sut_allowed_resource_classes")
    action_rows = policy.get("sut_allowed_actions")
    endpoints = policy.get("allowed_service_endpoints")
    channels = policy.get("required_audit_channels")
    require(isinstance(forbidden, list) and forbidden and all(isinstance(x, str) and x for x in forbidden) and len(forbidden) == len(set(forbidden)), "access policy: invalid forbidden resource classes")
    require(isinstance(allowed, list) and allowed and all(isinstance(x, str) and x for x in allowed) and len(allowed) == len(set(allowed)), "access policy: invalid allowed resource classes")
    require(not (set(forbidden) & set(allowed)), "access policy: a resource class cannot be both allowed and forbidden")
    require(isinstance(action_rows, list) and action_rows, "access policy: explicit resource/action rules required")
    allowed_actions = {}
    for row in action_rows:
        require(isinstance(row, dict) and set(row) == {"resource_class", "actions"}, "access policy: malformed resource/action rule")
        resource_class, actions = row["resource_class"], row["actions"]
        require(isinstance(resource_class, str) and resource_class and isinstance(actions, list) and actions and all(x in {"read", "write", "mount", "execute", "egress"} for x in actions) and len(actions) == len(set(actions)), "access policy: invalid allowed action set")
        require(resource_class not in allowed_actions, "access policy: duplicate resource action rule")
        allowed_actions[resource_class] = set(actions)
    require(set(allowed_actions) == set(allowed), "access policy: resource/action map must exactly cover allowed resource classes")
    require(isinstance(endpoints, list) and all(isinstance(x, str) and x for x in endpoints) and len(endpoints) == len(set(endpoints)), "access policy: invalid endpoint allowlist")
    require(isinstance(channels, list) and channels and set(channels) == REQUIRED_COVERAGE, "access policy: required audit channels must cover all six v0.1 channels")

    coverage = audit.get("coverage")
    require(isinstance(coverage, dict) and set(coverage) == REQUIRED_COVERAGE, "audit.coverage: exact channel inventory required")
    require(all(type(coverage[channel]) is bool for channel in REQUIRED_COVERAGE), "audit.coverage: boolean status required")
    events = audit.get("events")
    require(isinstance(events, list) and len(events) <= MAX_AUDIT_EVENTS, "audit.events outside supported bounds")
    event_ids, blocked_attempts, violations, unknown_events = set(), 0, [], 0
    for index, event in enumerate(events):
        where = f"audit.events[{index}]"
        require(isinstance(event, dict), f"{where}: expected object")
        required = {"event_id", "timestamp", "principal_role", "action", "resource_class", "disposition", "endpoint", "evidence_sha256"}
        require(set(event) == required, f"{where}: unexpected or missing fields")
        require(isinstance(event["event_id"], str) and event["event_id"] and event["event_id"] not in event_ids, f"{where}: duplicate/empty event ID")
        event_ids.add(event["event_id"])
        parse_time(event["timestamp"], f"{where}.timestamp")
        require(event["principal_role"] in {"sut", "evaluator", "observer", "harness"}, f"{where}: invalid principal role")
        require(event["action"] in {"read", "write", "mount", "execute", "egress"}, f"{where}: invalid action")
        require(isinstance(event["resource_class"], str) and event["resource_class"], f"{where}: resource class required")
        require(event["disposition"] in {"allowed", "denied", "unknown"}, f"{where}: invalid disposition")
        _hex64(event["evidence_sha256"], f"{where}.evidence_sha256")
        is_forbidden = event["principal_role"] == "sut" and event["resource_class"] in forbidden
        is_unlisted = event["principal_role"] == "sut" and event["resource_class"] not in allowed and event["resource_class"] not in forbidden
        bad_action = event["principal_role"] == "sut" and event["resource_class"] in allowed_actions and event["action"] not in allowed_actions[event["resource_class"]]
        bad_endpoint = event["principal_role"] == "sut" and event["action"] == "egress" and event["endpoint"] not in endpoints
        if event["disposition"] == "unknown":
            unknown_events += 1
        if is_forbidden or is_unlisted or bad_action or bad_endpoint:
            if event["disposition"] == "allowed":
                violations.append({"event_id": event["event_id"], "reason": "subject accessed forbidden/unlisted resource or unapproved endpoint"})
            elif event["disposition"] == "denied":
                blocked_attempts += 1
        if event["action"] == "egress":
            require(event["endpoint"] is None or isinstance(event["endpoint"], str), f"{where}: endpoint must be string or null")
        else:
            require(event["endpoint"] is None, f"{where}: endpoint is only valid for egress events")

    canary_hits = audit.get("canary_hits")
    require(type(canary_hits) is int and canary_hits >= 0, "audit.canary_hits must be a non-negative integer")
    if canary_hits:
        violations.append({"event_id": None, "reason": "benchmark contamination canary observed"})
    coverage_complete = audit.get("status") in {"example_only", "complete"} and all(coverage.values()) and unknown_events == 0
    if violations:
        status = "fail"
    elif not coverage_complete:
        status = "indeterminate"
    else:
        status = "pass"
    return {
        "status": status,
        "policy_id": policy["policy_id"],
        "event_count": len(events),
        "blocked_forbidden_access_attempts": blocked_attempts,
        "unknown_access_event_count": unknown_events,
        "canary_hits": canary_hits,
        "coverage_complete": coverage_complete,
        "coverage": coverage,
        "violations": violations,
        "audit_event_provenance_verified": False,
        "iam_network_enforcement_verified": False,
        "claim_ceiling": "Observed access-ledger classification only; the checker does not authenticate audit events or prove that all access channels were observed.",
    }


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--registration", type=Path, required=True)
    parser.add_argument("--salt-file", type=Path, required=True)
    parser.add_argument("--preregistration", type=Path, required=True)
    parser.add_argument("--access-policy", type=Path, required=True)
    parser.add_argument("--access-audit", type=Path, required=True)
    parser.add_argument("--output", default="-")
    args = parser.parse_args(argv)
    try:
        manifest_bytes = args.manifest.read_bytes()
        registration_bytes = args.registration.read_bytes()
        salt = args.salt_file.read_bytes()
        manifest = load_json(manifest_bytes, "manifest")
        registration = load_json(registration_bytes, "registration")
        policy = load_json(args.access_policy.read_bytes(), "access policy")
        audit = load_json(args.access_audit.read_bytes(), "access audit")
        preregistration_bytes = args.preregistration.read_bytes()
        commitment_report = check_corpus_commitment(manifest, manifest_bytes, registration, salt, preregistration_bytes)
        audit_report = check_access_audit(audit, policy)
        overall = "fail" if audit_report["status"] == "fail" else ("indeterminate" if audit_report["status"] == "indeterminate" else "pass")
        report = {
            "schema": "luminous-opsbench-sealed-corpus-report-v0.1",
            "version": 1,
            "status": overall,
            "commitment": commitment_report,
            "access_audit": audit_report,
            "claim_ceiling": "The example only demonstrates byte commitment and structural split/access checks. It is not a production custody, confidentiality, network-isolation or signed-audit qualification.",
        }
        rendered = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
        if args.output == "-":
            sys.stdout.write(rendered)
        else:
            args.output.write_text(rendered, encoding="utf-8")
        return 0 if overall == "pass" else (1 if overall == "fail" else 2)
    except (OSError, ContractError, AttributeError, KeyError, TypeError, ValueError, OverflowError, RecursionError) as exc:
        print(f"sealed-corpus-check: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
