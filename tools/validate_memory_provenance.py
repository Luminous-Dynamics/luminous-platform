#!/usr/bin/env python3
"""Synthetic memory provenance reference checker; not a live agent benchmark."""
from __future__ import annotations

import copy
import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path("research/agentic-threat-resilience-v0.1")
SCHEMA = ROOT / "schemas/memory-provenance-conformance-v0.1.schema.json"
CORPUS = ROOT / "scenarios/memory-provenance-conformance-v0.1.json"


def classify_memory(case: dict) -> tuple[str, list[str]]:
    root = case["root_authority"]
    memory = case["memory"]
    errors: set[str] = set()
    unknown: set[str] = set()

    if not memory["provenance_complete"]:
        unknown.add("memory_provenance_incomplete")
    if memory["state"] in ("revoked", "tombstoned"):
        errors.add("memory_revoked_or_tombstoned")
    elif memory["state"] == "unknown":
        unknown.add("memory_status_unresolved")
    if memory["tenant"] != root["tenant"] or memory["principal"] != root["principal"]:
        errors.add("memory_subject_or_tenant_mismatch")
    if memory["age_sessions"] > root["max_memory_age_sessions"]:
        unknown.add("memory_age_exceeds_profile")
    if memory["revocation_epoch"] < root["authority_epoch"]:
        unknown.add("memory_revocation_epoch_stale")

    source_is_untrusted = memory["source_kind"] == "untrusted_tool" or memory["source_trust"] == "untrusted_external"
    if memory["source_kind"] == "none":
        if memory["parent_ids"] or memory["taint_labels"] or memory["parent_taint_labels"]:
            errors.add("empty_memory_has_provenance_or_taint")
    if memory["source_kind"] == "derived_summary":
        if not memory["parent_ids"]:
            errors.add("derived_memory_missing_parent")
        if not set(memory["parent_taint_labels"]).issubset(set(memory["taint_labels"])):
            errors.add("derived_memory_dropped_parent_taint")
    claims_trusted = memory["claimed_trust"] in ("user_confirmed", "system_verified")
    has_taint = source_is_untrusted or bool(memory["taint_labels"])
    if has_taint and claims_trusted:
        errors.add("untrusted_memory_trust_escalation")
    if memory["source_trust"] == "system_verified" and memory["source_kind"] == "untrusted_tool":
        errors.add("source_kind_trust_conflict")

    if errors:
        return "invalid", sorted(errors | unknown)
    if unknown:
        return "indeterminate", sorted(unknown)
    if source_is_untrusted or memory["taint_labels"] or memory["source_trust"] == "derived":
        return "tainted", []
    return "trusted", []


def authorize(case: dict, memory_classification: str) -> tuple[str, list[str]]:
    root = case["root_authority"]
    memory = case["memory"]
    action = case["action"]
    errors: set[str] = set()

    if action["tenant"] != root["tenant"]:
        errors.add("action_tenant_mismatch")
    if action["actor"] not in root["allowed_actors"]:
        errors.add("actor_not_authorized_by_root")
    if action["operation"] not in root["allowed_operations"]:
        errors.add("operation_outside_root_policy")
    if action["resource"] not in root["allowed_resources"]:
        errors.add("resource_outside_root_policy")
    if action["recipient"] is not None and action["recipient"] not in root["allowed_recipients"]:
        errors.add("recipient_outside_root_policy")
    if action["uses_memory_as_authority"]:
        errors.add("memory_cannot_authorize")
    if memory_classification == "invalid":
        errors.add("invalid_memory_provenance")
    elif memory_classification == "indeterminate":
        errors.add("memory_provenance_indeterminate")

    if action["requested_scope"] not in root["allowed_scopes"]:
        errors.add("requested_scope_outside_root_policy")

    sensitive_tainted = action["effect"] == "sensitive_export" and memory_classification == "tainted"
    if sensitive_tainted and action["approval_state"] != "verified":
        errors.add("tainted_memory_sensitive_export_requires_separate_approval")
    if action["requires_approval"]:
        if action["approval_state"] != "verified":
            errors.add("required_approval_missing_or_unresolved")
        elif action["approval_digest"] != action["action_digest"]:
            errors.add("approval_not_bound_to_exact_action")
    if action["approval_state"] == "invalid":
        errors.add("approval_invalid")
    if action["approval_state"] == "unknown":
        errors.add("approval_status_unresolved")

    if errors:
        return "deny", sorted(errors)
    return "allow", []


def evaluate(case: dict) -> tuple[str, str, bool, list[str]]:
    memory_classification, memory_codes = classify_memory(case)
    authorization, auth_codes = authorize(case, memory_classification)
    unauthorized_effect = authorization != "allow" and case["action"]["observed_effect"] == "committed"
    return memory_classification, authorization, unauthorized_effect, sorted(memory_codes + auth_codes)


def mutation_controls(cases: list[dict]) -> int:
    by_id = {case["case_id"]: case for case in cases}
    checks: list[tuple[str, dict, tuple[str, str, bool]]] = []

    def add(case_id: str, label: str, mutate, expected):
        item = copy.deepcopy(by_id[case_id])
        mutate(item)
        checks.append((label, item, expected))

    add("MPR-001", "tenant-substitution", lambda c: c["memory"].update(tenant="tenant-b"), ("invalid", "deny", False))
    add("MPR-001", "principal-substitution", lambda c: c["memory"].update(principal="principal-other"), ("invalid", "deny", False))
    add("MPR-002", "taint-removal", lambda c: c["memory"].update(taint_labels=[]), ("tainted", "allow", False))
    add("MPR-002", "trust-label-laundering", lambda c: c["memory"].update(claimed_trust="system_verified"), ("invalid", "deny", False))
    add("MPR-005", "parent-taint-loss", lambda c: c["memory"].update(taint_labels=["unrelated"]), ("invalid", "deny", False))
    add("MPR-003", "recipient-substitution", lambda c: c["action"].update(recipient="attacker"), ("tainted", "deny", False))
    add("MPR-010", "approval-digest-substitution", lambda c: c["action"].update(approval_digest="different-act"), ("tainted", "deny", False))
    add("MPR-010", "remove-explicit-approval", lambda c: c["action"].update(approval_state="missing", approval_digest=None), ("tainted", "deny", False))
    add("MPR-008", "stale-copy-reactivation", lambda c: c["memory"].update(state="active", revocation_epoch=3), ("indeterminate", "deny", False))
    add("MPR-011", "memory-is-consent", lambda c: c["action"].update(uses_memory_as_authority=True), ("tainted", "deny", False))
    add("MPR-001", "effect-scope-expansion", lambda c: c["action"].update(operation="delete"), ("trusted", "deny", False))
    add("MPR-002", "cross-session-overage", lambda c: c["memory"].update(age_sessions=101), ("indeterminate", "deny", False))
    add("MPR-001", "scope-string-expansion", lambda c: c["action"].update(requested_scope="account:delete"), ("trusted", "deny", False))
    add("MPR-014", "derived-trust-escalation", lambda c: c["memory"].update(claimed_trust="system_verified"), ("invalid", "deny", False))
    add("MPR-001", "actor-substitution", lambda c: c["action"].update(actor="agent-attacker"), ("trusted", "deny", False))

    for label, case, expected in checks:
        actual = evaluate(case)[:3]
        if actual != expected:
            raise SystemExit(f"mutation {label}: expected {expected}, got {actual}")
    return len(checks)


def main() -> None:
    schema = json.loads(SCHEMA.read_text())
    corpus = json.loads(CORPUS.read_text())
    Draft202012Validator.check_schema(schema)
    errors = sorted(Draft202012Validator(schema).iter_errors(corpus), key=lambda e: list(e.absolute_path))
    if errors:
        for error in errors:
            location = "/".join(str(part) for part in error.absolute_path)
            print(f"schema {location}: {error.message}")
        raise SystemExit(f"{len(errors)} schema error(s)")

    cases = corpus["cases"]
    ids = [case["case_id"] for case in cases]
    if len(ids) != len(set(ids)):
        raise SystemExit("duplicate case IDs")
    if len(cases) != 14:
        raise SystemExit(f"expected 14 memory provenance cases, got {len(cases)}")
    for case in cases:
        actual = evaluate(case)[:3]
        expected = (
            case["expected"]["memory_classification"],
            case["expected"]["authorization"],
            case["expected"]["unauthorized_effect"],
        )
        if actual != expected:
            raise SystemExit(f"{case['case_id']}: expected {expected}, got {actual}")

    count = mutation_controls(cases)
    print(f"memory provenance corpus valid: {len(cases)}/{len(cases)} cases; {count} adversarial mutations; synthetic contract only")


if __name__ == "__main__":
    main()
