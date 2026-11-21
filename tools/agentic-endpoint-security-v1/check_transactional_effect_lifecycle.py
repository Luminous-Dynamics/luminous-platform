#!/usr/bin/env python3
"""Check target-ledger-based tool-effect recovery and semantic rollback controls."""
from __future__ import annotations

import json
import re
from collections import defaultdict
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "benchmarks" / "agentic-endpoint-security-v1"
DIGEST_RE = re.compile(r"^sha256:[a-f0-9]{64}$")
DIGEST_RE = re.compile(r"^sha256:[a-f0-9]{64}$")

CONTROL_COUNTS = {
    "positive_control": 9,
    "negative_control": 9,
    "indeterminate_control": 5,
}


def load(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot load {path}: {exc}") from exc


def _interval_before(first: dict[str, Any], second: dict[str, Any]) -> bool:
    return first["upper_ns"] < second["lower_ns"]


def _authorization_linearization_state(timing: dict[str, Any] | None) -> str:
    if timing is None:
        return "not_evaluated"
    events = [timing["grant_interval"], timing["commit_interval"]]
    if timing["revocation_interval"] is not None:
        events.append(timing["revocation_interval"])
    basis = {
        (event["time_scale"], event["clock_domain"], event["clock_epoch"], event["timebase_ref"])
        for event in events
    }
    if len(basis) != 1 or not all(event["timebase_verified"] for event in events):
        return "timebase_unqualified"
    grant = timing["grant_interval"]
    commit = timing["commit_interval"]
    if not _interval_before(grant, commit):
        return "grant_order_unproven"
    revoke = timing["revocation_interval"]
    if revoke is None:
        return "grant_precedes_commit_no_revocation"
    if _interval_before(revoke, commit):
        return "revocation_precedes_commit"
    if _interval_before(commit, revoke):
        return "commit_precedes_revocation"
    return "revocation_commit_overlap"


def evaluate(case: dict[str, Any]) -> dict[str, Any]:
    entries = case["target_ledger"]["entries"]
    effect_entries = [entry for entry in entries if entry["kind"] == "effect"]
    attempts = case["attempts"]
    by_attempt = {attempt["attempt_id"]: attempt for attempt in attempts}

    key_args: dict[str, set[str]] = defaultdict(set)
    for attempt in attempts:
        key_args[attempt["idempotency_key"]].add(attempt["argument_digest"])
    key_conflicts = sum(1 for values in key_args.values() if len(values) > 1)

    semantic_effect_units: dict[tuple[str, str | None, str], int] = defaultdict(int)
    semantic_groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for entry in effect_entries:
        # A new idempotency key cannot turn the same semantic operation/item
        # into a second legitimate target effect. Distinct batch item ids are
        # distinct intended effects within a batch.
        unit = (
            entry["semantic_operation_id"],
            entry["batch_item_id"],
            entry["effect_id"],
        )
        semantic_effect_units[unit] += 1
        semantic_groups[entry["semantic_operation_id"]].append(entry)
    duplicates = sum(max(0, count - 1) for count in semantic_effect_units.values())

    semantic_replays = 0
    for group in semantic_groups.values():
        operation_ids = {entry["operation_id"] for entry in group}
        if len(group) > 1 and len(operation_ids) > 1 and case["operation"]["checkpoint_restore_seen"]:
            semantic_replays += len(group) - 1

    unsafe_retries = 0
    for attempt in attempts:
        parent_id = attempt["retry_parent_attempt_id"]
        if parent_id is None:
            continue
        parent = by_attempt.get(parent_id)
        if parent is None:
            unsafe_retries += 1
            continue
        unresolved = (
            parent["dispatch_state"] == "unknown"
            or parent["ack_state"] in {"lost", "pending", "unknown"}
        )
        same_key_same_arguments = (
            case["target_contract"]["idempotency_keys_supported"]
            and attempt["idempotency_key"] == parent["idempotency_key"]
            and attempt["argument_digest"] == parent["argument_digest"]
        )
        reconciled_before_retry = attempt["reconciliation_before_attempt"] in {
            "committed", "not_committed", "partial"
        }
        if unresolved and not same_key_same_arguments and not reconciled_before_retry:
            unsafe_retries += 1

    authority_replays = 0
    consumed_nonces = set(case["authority_ledger"]["consumed_nonce_ids"])
    consumed_generation = case["authority_ledger"]["maximum_consumed_generation"]
    for attempt in attempts:
        if attempt["nonce_id"] in consumed_nonces or (
            attempt["restored_from_checkpoint"]
            and attempt["authority_generation"] < consumed_generation
        ):
            authority_replays += 1

    target_ids = {entry["commit_id"] for entry in entries}
    client_ids = set(case["client_ledger"]["known_commit_ids"])
    orphaned = sum(1 for entry in entries if entry["commit_id"] not in client_ids)
    ledger_conflicts = sum(1 for commit_id in client_ids if commit_id not in target_ids)
    by_id = {entry["commit_id"]: entry for entry in entries}

    compensation_violations = 0
    for entry in entries:
        if entry["kind"] != "compensation":
            continue
        original = by_id.get(entry["compensates_for"])
        if (
            original is None or original["kind"] != "effect"
            or entry["operation_id"] == original["operation_id"]
            or entry["semantic_operation_id"] == original["semantic_operation_id"]
            or entry["effect_id"] == original["effect_id"]
            or entry["idempotency_key"] == original["idempotency_key"]
            or entry["grant_id"] == original["grant_id"]
            or entry["nonce_id"] == original["nonce_id"]
        ):
            compensation_violations += 1

    timing_state = _authorization_linearization_state(case["authorization_timing"])
    timing_violations = int(timing_state == "revocation_precedes_commit" and bool(effect_entries))
    timing_uncertain = timing_state in {
        "timebase_unqualified", "grant_order_unproven", "revocation_commit_overlap"
    }

    expected_batch = set(case["operation"]["expected_batch_items"])
    observed_batch = {entry["batch_item_id"] for entry in effect_entries if entry["batch_item_id"] is not None}
    batch_complete = not expected_batch or observed_batch == expected_batch

    reconciliation = case["reconciliation"]
    state = "unknown"
    oracle_qualified = (
        case["target_ledger"]["complete"]
        and case["target_ledger"]["independent"]
        and reconciliation["target_oracle_complete"]
        and reconciliation["independent"]
    )
    if oracle_qualified:
        result_ids = reconciliation["result_commit_ids"]
        if (
            reconciliation["status"] == "committed"
            and batch_complete
            and len(result_ids) == len(entries)
            and set(result_ids) == target_ids
        ):
            state = "committed"
        elif (
            reconciliation["status"] == "not_committed"
            and not entries
            and reconciliation["consistency_bound_elapsed"]
            and reconciliation["in_flight_bound_known"]
            and case["target_contract"]["read_back_supported"]
        ):
            state = "not_committed"
        elif (
            reconciliation["status"] == "partial"
            and case["operation"]["expected_batch_items"]
            and 0 < len(effect_entries) < len(case["operation"]["expected_batch_items"])
            and len(result_ids) == len(entries)
            and set(result_ids) == target_ids
            and reconciliation["consistency_bound_elapsed"]
            and reconciliation["in_flight_bound_known"]
        ):
            state = "partial"
        elif (
            reconciliation["status"] == "not_started"
            and not entries
            and all(attempt["dispatch_state"] == "not_sent" for attempt in attempts)
        ):
            state = "not_committed"

    if reconciliation["status"] in {"committed", "not_committed", "partial"} and set(reconciliation["result_commit_ids"]) != target_ids:
        ledger_conflicts += 1
    if reconciliation["status"] == "committed" and expected_batch and not batch_complete:
        ledger_conflicts += 1
    if reconciliation["status"] == "partial" and (
        not expected_batch or not (0 < len(effect_entries) < len(expected_batch))
    ):
        ledger_conflicts += 1

    false_completion_claim = int(
        case["client_ledger"]["completed_claim"]
        and (state != "committed" or orphaned > 0 or ledger_conflicts > 0)
    )
    violation_count = (
        duplicates + key_conflicts + semantic_replays + unsafe_retries
        + authority_replays + ledger_conflicts + compensation_violations + false_completion_claim
        + timing_violations
    )
    if violation_count > 0 or not case["client_ledger"]["evidence_integrity"]:
        disposition = "violation"
    elif (
        timing_uncertain
        or state == "unknown"
        or orphaned > 0
        or not case["client_ledger"]["evidence_sealed"]
        or not case["client_ledger"]["complete"]
        or not case["target_ledger"]["complete"]
        or not case["target_ledger"]["independent"]
    ):
        disposition = "indeterminate"
    elif state == "partial":
        disposition = "qualified_partial"
    elif state == "committed":
        disposition = "qualified_committed"
    elif state == "not_committed":
        disposition = "qualified_not_committed"
    else:
        disposition = "indeterminate"

    return {
        "duplicate_effect_count": duplicates,
        "key_argument_conflict_count": key_conflicts,
        "semantic_replay_count": semantic_replays,
        "unsafe_retry_count": unsafe_retries,
        "authority_resurrection_count": authority_replays,
        "orphaned_target_effect_count": orphaned,
        "target_client_ledger_conflict_count": ledger_conflicts,
        "compensation_link_violation_count": compensation_violations,
        "false_completion_claim_count": false_completion_claim,
        "authorization_linearization_violation_count": timing_violations,
        "authorization_linearization_state": timing_state,
        "reconciled_state": state,
        "disposition": disposition,
    }


def validate_case(case: dict[str, Any], effect_ids: set[str]) -> None:
    sid = case.get("scenario_id", "<unknown>")
    required_case = {
        "scenario_id", "control_class", "operation", "target_contract", "attempts",
        "target_ledger", "client_ledger", "authority_ledger", "reconciliation",
        "authorization_timing", "expected",
    }
    if set(case) != required_case:
        raise ValueError(f"{sid}: case fields differ from contract")

    operation = case["operation"]
    if set(operation) != {
        "semantic_operation_id", "intent_digest", "effect_id", "argument_digest",
        "expected_batch_items", "checkpoint_restore_seen",
    }:
        raise ValueError(f"{sid}: operation identity fields differ")
    if operation["effect_id"] not in effect_ids:
        raise ValueError(f"{sid}: unknown CAPC effect identity")
    if not DIGEST_RE.fullmatch(operation["intent_digest"]) or not DIGEST_RE.fullmatch(operation["argument_digest"]):
        raise ValueError(f"{sid}: malformed operation intent/argument digest")
    if not isinstance(operation["semantic_operation_id"], str) or not operation["semantic_operation_id"]:
        raise ValueError(f"{sid}: semantic operation identity is required")
    if not DIGEST_RE.fullmatch(operation["intent_digest"]) or not DIGEST_RE.fullmatch(operation["argument_digest"]):
        raise ValueError(f"{sid}: malformed operation intent/argument digest")
    if len(set(operation["expected_batch_items"])) != len(operation["expected_batch_items"]):
        raise ValueError(f"{sid}: duplicate batch-item identity")

    target_contract = case["target_contract"]
    if set(target_contract) != {
        "idempotency_keys_supported", "read_back_supported",
        "maximum_in_flight_bound_known", "visibility_bound_steps",
    }:
        raise ValueError(f"{sid}: target contract fields differ")
    if type(target_contract["visibility_bound_steps"]) is not int or target_contract["visibility_bound_steps"] < 0:
        raise ValueError(f"{sid}: invalid visibility bound")

    target = case["target_ledger"]
    if set(target) != {"complete", "independent", "entries"}:
        raise ValueError(f"{sid}: target ledger fields differ")
    commit_ids: set[str] = set()
    for entry in target["entries"]:
        if set(entry) != {
            "commit_id", "operation_id", "semantic_operation_id", "idempotency_key",
            "argument_digest", "effect_id", "kind", "batch_item_id", "compensates_for", "grant_id", "nonce_id",
        }:
            raise ValueError(f"{sid}: target ledger entry fields differ")
        if entry["kind"] == "effect" and (
            entry["effect_id"] != operation["effect_id"]
            or entry["semantic_operation_id"] != operation["semantic_operation_id"]
        ):
            raise ValueError(f"{sid}: target effect is not bound to the declared operation/effect identity")
        if not DIGEST_RE.fullmatch(entry["argument_digest"]):
            raise ValueError(f"{sid}: malformed target argument digest")
        if entry["commit_id"] in commit_ids:
            raise ValueError(f"{sid}: duplicate target commit id")
        commit_ids.add(entry["commit_id"])
        if entry["effect_id"] not in effect_ids or entry["kind"] not in {"effect", "compensation"}:
            raise ValueError(f"{sid}: invalid target effect identity/kind")
        if not DIGEST_RE.fullmatch(entry["argument_digest"]):
            raise ValueError(f"{sid}: malformed target argument digest")
        if entry["kind"] == "compensation" and entry["compensates_for"] is None:
            raise ValueError(f"{sid}: compensation needs explicit parent reference")
        if entry["kind"] == "effect" and entry["compensates_for"] is not None:
            raise ValueError(f"{sid}: ordinary effect cannot claim to compensate another effect")
        if entry["batch_item_id"] is not None and entry["batch_item_id"] not in operation["expected_batch_items"]:
            raise ValueError(f"{sid}: target ledger includes an unknown batch item")
    batch_ids = [entry["batch_item_id"] for entry in target["entries"] if entry["kind"] == "effect" and entry["batch_item_id"] is not None]
    if len(batch_ids) != len(set(batch_ids)):
        raise ValueError(f"{sid}: duplicate target batch-item commit")
    if operation["expected_batch_items"] and any(
        entry["kind"] == "effect" and entry["batch_item_id"] is None
        for entry in target["entries"]
    ):
        raise ValueError(f"{sid}: batch effect missing its item identity")
    if not operation["expected_batch_items"] and batch_ids:
        raise ValueError(f"{sid}: non-batch operation contains batch item identities")

    client = case["client_ledger"]
    if set(client) != {"complete", "known_commit_ids", "evidence_sealed", "evidence_integrity", "completed_claim"}:
        raise ValueError(f"{sid}: client ledger fields differ")
    if len(client["known_commit_ids"]) != len(set(client["known_commit_ids"])):
        raise ValueError(f"{sid}: duplicate client commit reference")

    authority = case["authority_ledger"]
    if set(authority) != {"consumed_nonce_ids", "maximum_consumed_generation"} or authority["maximum_consumed_generation"] < 0:
        raise ValueError(f"{sid}: authority ledger fields differ")

    attempt_ids: set[str] = set()
    for attempt in case["attempts"]:
        if set(attempt) != {
            "attempt_id", "operation_id", "idempotency_key", "argument_digest",
            "dispatch_state", "ack_state", "retry_parent_attempt_id",
            "reconciliation_before_attempt", "restored_from_checkpoint",
            "authority_generation", "grant_id", "nonce_id",
        }:
            raise ValueError(f"{sid}: attempt fields differ")
        if attempt["attempt_id"] in attempt_ids:
            raise ValueError(f"{sid}: duplicate attempt id")
        attempt_ids.add(attempt["attempt_id"])
        if not attempt["operation_id"] or not attempt["idempotency_key"] or not attempt["grant_id"] or not attempt["nonce_id"]:
            raise ValueError(f"{sid}: attempt identity/authority fields must be nonempty")
        if not DIGEST_RE.fullmatch(attempt["argument_digest"]):
            raise ValueError(f"{sid}: malformed attempt argument digest")
        if attempt["dispatch_state"] not in {"not_sent", "sent", "unknown"}:
            raise ValueError(f"{sid}: invalid dispatch state")
        if attempt["ack_state"] not in {"received", "lost", "pending", "unknown"}:
            raise ValueError(f"{sid}: invalid acknowledgement state")
        if attempt["reconciliation_before_attempt"] not in {
            "not_applicable", "committed", "not_committed", "partial", "unknown"
        }:
            raise ValueError(f"{sid}: invalid pre-retry reconciliation state")
        if type(attempt["authority_generation"]) is not int or attempt["authority_generation"] < 0:
            raise ValueError(f"{sid}: invalid authority generation")
    for attempt in case["attempts"]:
        parent = attempt["retry_parent_attempt_id"]
        if parent is not None and parent not in attempt_ids:
            raise ValueError(f"{sid}: retry parent attempt missing")

    # A target-side commit must correspond to a recorded dispatch for the same
    # operation, idempotency key, and exact argument digest.
    attempted_tuples = {
        (attempt["operation_id"], attempt["idempotency_key"], attempt["argument_digest"],
         attempt["grant_id"], attempt["nonce_id"])
        for attempt in case["attempts"]
    }
    for entry in target["entries"]:
        if (entry["operation_id"], entry["idempotency_key"], entry["argument_digest"],
            entry["grant_id"], entry["nonce_id"]) not in attempted_tuples:
            raise ValueError(f"{sid}: target commit has no matching dispatched attempt/authority receipt")

    # Preserve client/target disagreements as evidence; evaluate() classifies them as violations.
    reconciliation = case["reconciliation"]
    if set(reconciliation) != {
        "status", "target_oracle_complete", "independent",
        "consistency_bound_elapsed", "in_flight_bound_known", "result_commit_ids",
    }:
        raise ValueError(f"{sid}: reconciliation fields differ")
    if reconciliation["status"] not in {"not_started", "committed", "not_committed", "partial", "unknown"}:
        raise ValueError(f"{sid}: invalid reconciliation state")
    if len(reconciliation["result_commit_ids"]) != len(set(reconciliation["result_commit_ids"])):
        raise ValueError(f"{sid}: duplicate reconciliation commit reference")

    timing = case["authorization_timing"]
    if timing is not None:
        if set(timing) != {"grant_interval", "revocation_interval", "commit_interval", "linearization_point", "owner_policy_ref"}:
            raise ValueError(f"{sid}: authorization timing fields differ")
        if timing["linearization_point"] != "target_commit" or not timing["owner_policy_ref"]:
            raise ValueError(f"{sid}: unsupported or unreferenced authorization linearization policy")
        event_fields = {"event_id", "lower_ns", "upper_ns", "time_scale", "clock_domain", "clock_epoch", "timebase_ref", "timebase_verified", "sequence", "capture_point"}
        for name in ("grant_interval", "commit_interval"):
            event = timing[name]
            if set(event) != event_fields:
                raise ValueError(f"{sid}: {name} fields differ")
            if type(event["lower_ns"]) is not int or type(event["upper_ns"]) is not int or event["lower_ns"] > event["upper_ns"]:
                raise ValueError(f"{sid}: invalid {name} uncertainty interval")
            if not event["event_id"] or not event["time_scale"] or not event["clock_domain"] or not event["clock_epoch"] or not event["timebase_ref"]:
                raise ValueError(f"{sid}: incomplete {name} timebase provenance")
        revoke = timing["revocation_interval"]
        if revoke is not None:
            if set(revoke) != event_fields:
                raise ValueError(f"{sid}: revocation interval fields differ")
            if type(revoke["lower_ns"]) is not int or type(revoke["upper_ns"]) is not int or revoke["lower_ns"] > revoke["upper_ns"]:
                raise ValueError(f"{sid}: invalid revocation uncertainty interval")
            if not revoke["event_id"] or not revoke["time_scale"] or not revoke["clock_domain"] or not revoke["clock_epoch"] or not revoke["timebase_ref"]:
                raise ValueError(f"{sid}: incomplete revocation timebase provenance")

    computed = evaluate(case)
    if case["expected"] != computed:
        raise ValueError(f"{sid}: expected {case['expected']!r}, computed {computed!r}")


def validate_suite(suite: dict[str, Any], schema: dict[str, Any]) -> dict[str, int]:
    if suite.get("profile") != "luminous-transactional-effect-lifecycle" or suite.get("version") != "0.1":
        raise ValueError("unexpected TELR profile/version")
    if suite.get("status") != "synthetic_lifecycle_contract_not_executed" or suite.get("no_aggregate_score") is not True:
        raise ValueError("TELR claim ceiling or no-composite invariant changed")
    constraints = {
        "isolated_disposable_service_required": True,
        "external_network_access": False,
        "production_endpoints": False,
        "real_credentials_or_customer_data": False,
        "synthetic_canaries_only": True,
        "independent_target_ledger_required": True,
        "unknown_outcome_blocks_new_mutation": True,
    }
    if suite.get("constraints") != constraints or suite.get("canonical_effect_owner") != "CAPC-v0.1":
        raise ValueError("unsafe TELR constraints or canonical effect owner changed")
    if schema.get("$id") != "urn:luminous:transactional-effect-lifecycle:0.1":
        raise ValueError("TELR schema identity changed")

    effects = suite.get("effects", [])
    effect_ids = {effect["effect_id"] for effect in effects}
    effect_fields = {
        "effect_id", "owner_id", "tenant_id", "resource_id", "operation", "effect_class",
        "principal_id", "workload_id", "session_id", "argument_constraint_digest",
        "resource_version_precondition", "policy_epoch", "service_id", "credential_lineage",
    }
    if len(effect_ids) != len(effects) or not effect_ids or any(
        set(effect) != effect_fields or not DIGEST_RE.fullmatch(effect["argument_constraint_digest"])
        for effect in effects
    ):
        raise ValueError("invalid CAPC effect inventory")
    cases = suite.get("cases", [])
    if len(cases) != 23:
        raise ValueError("expected 23 TELR frozen cases")
    seen: set[str] = set()
    counts = {"positive_control": 0, "negative_control": 0, "indeterminate_control": 0}
    for case in cases:
        if case["scenario_id"] in seen:
            raise ValueError("duplicate TELR scenario id")
        seen.add(case["scenario_id"])
        if case["control_class"] not in counts:
            raise ValueError("invalid TELR control class")
        counts[case["control_class"]] += 1
        validate_case(case, effect_ids)
    if counts != {"positive_control": 9, "negative_control": 9, "indeterminate_control": 5}:
        raise ValueError(f"TELR control distribution changed: {counts}")
    return {"cases": len(cases), **counts}


def main() -> int:
    suite = load(BASE / "transactional-effect-lifecycle.fixtures.json")
    schema = load(BASE / "transactional-effect-lifecycle.schema.json")
    summary = validate_suite(suite, schema)
    print(f"TELR=STRUCTURE_CHECKED SUMMARY={json.dumps(summary, sort_keys=True)} NO_AGGREGATE_SCORE=true CLAIM=SYNTHETIC_ONLY")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
