#!/usr/bin/env python3
"""Independent causal evidence verifier for operational recovery episodes.

This evaluator-side tool checks supplied event records. It does not execute
operations, authenticate signatures, prove source independence, or establish
filesystem durability.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import defaultdict
from datetime import datetime
from pathlib import Path

MANIFEST_SCHEMA = "luminous-operational-recovery-run-v0.1"
EVENT_SCHEMA = "luminous-operational-recovery-event-v0.1"
HEX_256 = re.compile(r"^[a-f0-9]{64}$")
HEX_160 = re.compile(r"^[a-f0-9]{40}$")
EVENT_TYPES = {
    "plan_authorized", "dispatch_intent_durable", "participant_accepted",
    "effect_committed", "transport_response_lost", "attempt_status_observed",
    "effect_reconciled", "step_postcondition_verified", "coordinator_restarted",
    "recovery_scan_completed", "service_outcome_verified", "agent_declared_resolved",
}
SOURCE_KINDS = {"coordinator", "authority_adapter", "participant", "participant_ledger", "independent_observer", "evaluator"}

class TraceError(ValueError):
    pass

def canonical_bytes(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode("utf-8")

def digest_record(record: dict) -> str:
    unsigned = {k: v for k, v in record.items() if k != "event_digest"}
    return hashlib.sha256(canonical_bytes(unsigned)).hexdigest()

def _require(obj, keys, where):
    if not isinstance(obj, dict):
        raise TraceError(f"{where}: expected object")
    missing = [k for k in keys if k not in obj]
    if missing:
        raise TraceError(f"{where}: missing fields {missing}")

def _sha(value, where, size=64):
    pattern = HEX_256 if size == 64 else HEX_160
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        raise TraceError(f"{where}: invalid SHA-{'256' if size == 64 else '1'} hex")

def _time(value, where):
    if not isinstance(value, str):
        raise TraceError(f"{where}: expected date-time string")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise TraceError(f"{where}: invalid date-time") from exc
    if parsed.tzinfo is None:
        raise TraceError(f"{where}: timezone is required")
    return parsed

def _strings(values, where, allow_empty=False):
    if not isinstance(values, list):
        raise TraceError(f"{where}: expected array")
    if not allow_empty and not values:
        raise TraceError(f"{where}: array may not be empty")
    if any(not isinstance(x, str) or not x for x in values) or len(values) != len(set(values)):
        raise TraceError(f"{where}: expected unique non-empty strings")
    return set(values)

def validate_manifest(manifest: dict) -> dict:
    _require(manifest, ("schema", "version", "run_binding", "executor", "authority", "participants", "observers", "required_steps"), "manifest")
    if manifest["schema"] != MANIFEST_SCHEMA or type(manifest["version"]) is not int or manifest["version"] != 1:
        raise TraceError("manifest: unsupported schema/version")
    binding = manifest["run_binding"]
    _require(binding, ("run_id", "episode_id", "plan_digest", "expected_service_digest", "source_commit_sha", "policy_digest", "corpus_digest", "scenario_variant_digest", "fault_matrix_digest", "observer_manifest_digest"), "manifest.run_binding")
    for key in ("run_id", "episode_id"):
        if not isinstance(binding[key], str) or not binding[key]:
            raise TraceError(f"manifest.run_binding.{key}: required non-empty string")
    for key in ("plan_digest", "expected_service_digest", "policy_digest", "corpus_digest", "scenario_variant_digest", "fault_matrix_digest", "observer_manifest_digest"):
        _sha(binding[key], f"manifest.run_binding.{key}")
    _sha(binding["source_commit_sha"], "manifest.run_binding.source_commit_sha", 40)
    executor = manifest["executor"]
    _require(executor, ("source_id", "trust_domain_id"), "manifest.executor")
    if not all(isinstance(executor[k], str) and executor[k] for k in ("source_id", "trust_domain_id")):
        raise TraceError("manifest.executor: invalid identity")
    authority = manifest["authority"]
    _require(authority, ("source_id", "trust_domain_id"), "manifest.authority")
    if not all(isinstance(authority[k], str) and authority[k] for k in ("source_id", "trust_domain_id")):
        raise TraceError("manifest.authority: invalid identity")
    if authority["source_id"] == executor["source_id"] or authority["trust_domain_id"] == executor["trust_domain_id"]:
        raise TraceError("manifest.authority: authority must be distinct from the executor")

    participants = {}
    if not isinstance(manifest["participants"], list) or not manifest["participants"]:
        raise TraceError("manifest.participants: expected non-empty array")
    for i, item in enumerate(manifest["participants"]):
        w = f"manifest.participants[{i}]"
        _require(item, ("source_id", "trust_domain_id", "participant_kind"), w)
        if any(not isinstance(item[k], str) or not item[k] for k in ("source_id", "trust_domain_id", "participant_kind")):
            raise TraceError(f"{w}: empty field")
        if item["source_id"] in participants or item["source_id"] in {executor["source_id"], authority["source_id"]}:
            raise TraceError(f"{w}: duplicate/reserved source_id")
        if item["trust_domain_id"] in {executor["trust_domain_id"], authority["trust_domain_id"]}:
            raise TraceError(f"{w}: participant must not share executor or authority trust domain")
        participants[item["source_id"]] = item

    observers = {}
    observer_ids = set()
    if not isinstance(manifest["observers"], list) or not manifest["observers"]:
        raise TraceError("manifest.observers: expected non-empty array")
    for i, item in enumerate(manifest["observers"]):
        w = f"manifest.observers[{i}]"
        _require(item, ("source_id", "observer_id", "trust_domain_id", "scope"), w)
        if any(not isinstance(item[k], str) or not item[k] for k in ("source_id", "observer_id", "trust_domain_id")):
            raise TraceError(f"{w}: empty identity")
        if item["source_id"] in observers or item["source_id"] in participants or item["source_id"] in {executor["source_id"], authority["source_id"]}:
            raise TraceError(f"{w}: duplicate or reserved source_id")
        if item["observer_id"] in observer_ids:
            raise TraceError(f"{w}: duplicate observer_id")
        observer_ids.add(item["observer_id"])
        participant_domains = {p["trust_domain_id"] for p in participants.values()}
        if item["trust_domain_id"] in participant_domains | {executor["trust_domain_id"], authority["trust_domain_id"]}:
            raise TraceError(f"{w}: observer must be in a distinct trust domain")
        scope = _strings(item["scope"], w+".scope")
        observers[item["source_id"]] = {**item, "scope_set": scope}

    steps = {}
    if not isinstance(manifest["required_steps"], list) or not manifest["required_steps"]:
        raise TraceError("manifest.required_steps: expected non-empty array")
    for i, step in enumerate(manifest["required_steps"]):
        w = f"manifest.required_steps[{i}]"
        _require(step, ("step_id", "participant_source_id", "target_digest", "operation_id", "request_digest", "operation_binding_digest", "idempotency_key_digest", "expected_postcondition_digest"), w)
        for key in ("step_id", "participant_source_id", "operation_id"):
            if not isinstance(step[key], str) or not step[key]:
                raise TraceError(f"{w}.{key}: required non-empty string")
        if step["step_id"] in steps:
            raise TraceError(f"{w}: duplicate step_id")
        if step["participant_source_id"] not in participants:
            raise TraceError(f"{w}: participant is not declared")
        for key in ("target_digest", "request_digest", "operation_binding_digest", "idempotency_key_digest", "expected_postcondition_digest"):
            _sha(step[key], w+"."+key)
        steps[step["step_id"]] = step
    return {"binding": binding, "executor": executor, "authority": authority, "participants": participants, "observers": observers, "steps": steps}

def validate_event(event: dict, index: int, binding: dict):
    w = f"event[{index}]"
    _require(event, ("schema", "event_id", "run_id", "episode_id", "plan_digest", "source_id", "source_kind", "trust_domain_id", "source_epoch", "source_sequence", "previous_event_digest", "parent_event_ids", "event_type", "recorded_at", "data", "event_digest"), w)
    if event["schema"] != EVENT_SCHEMA:
        raise TraceError(f"{w}: wrong event schema")
    for key in ("event_id", "run_id", "episode_id", "source_id", "trust_domain_id"):
        if not isinstance(event[key], str) or not event[key]:
            raise TraceError(f"{w}.{key}: required non-empty string")
    for key in ("run_id", "episode_id", "plan_digest"):
        expected = binding[key]
        if event[key] != expected:
            raise TraceError(f"{w}.{key}: run binding mismatch")
    if event["source_kind"] not in SOURCE_KINDS:
        raise TraceError(f"{w}.source_kind: unsupported")
    if event["event_type"] not in EVENT_TYPES:
        raise TraceError(f"{w}.event_type: unsupported")
    if type(event["source_epoch"]) is not int or event["source_epoch"] < 0:
        raise TraceError(f"{w}.source_epoch: invalid")
    if type(event["source_sequence"]) is not int or event["source_sequence"] < 1:
        raise TraceError(f"{w}.source_sequence: invalid")
    if event["previous_event_digest"] is not None:
        _sha(event["previous_event_digest"], w+".previous_event_digest")
    _strings(event["parent_event_ids"], w+".parent_event_ids", allow_empty=True)
    _time(event["recorded_at"], w+".recorded_at")
    if not isinstance(event["data"], dict):
        raise TraceError(f"{w}.data: expected object")
    _sha(event["event_digest"], w+".event_digest")
    if digest_record(event) != event["event_digest"]:
        raise TraceError(f"{w}: event digest mismatch")
    if event["event_type"] in {"plan_authorized", "dispatch_intent_durable", "participant_accepted", "effect_committed", "transport_response_lost", "attempt_status_observed", "effect_reconciled", "step_postcondition_verified"}:
        if not isinstance(event.get("attempt_id"), str) or not event.get("attempt_id") or not isinstance(event.get("step_id"), str) or not event.get("step_id"):
            raise TraceError(f"{w}: this event type requires step_id and attempt_id")

def _ancestors(event_id, by_id, memo=None):
    memo = {} if memo is None else memo
    if event_id in memo:
        return memo[event_id]
    result = set()
    for parent_id in by_id[event_id]["parent_event_ids"]:
        result.add(parent_id)
        result.update(_ancestors(parent_id, by_id, memo))
    memo[event_id] = result
    return result

def _add(violations, code, event_id, detail):
    violations.append({"code": code, "event_id": event_id, "detail": detail})

def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def verifier_source_digest() -> str:
    return _sha256_bytes(Path(__file__).read_bytes())

def verify(manifest: dict, events: list[dict]) -> dict:
    m = validate_manifest(manifest)
    binding = m["binding"]
    if not isinstance(events, list) or not events:
        return {"schema": "luminous-operational-recovery-verification-v0.1", "version": 1, "status": "indeterminate",
            "run_id": binding["run_id"], "episode_id": binding["episode_id"], "source_commit_sha": binding["source_commit_sha"],
            "manifest_digest": _sha256_bytes(canonical_bytes(manifest)), "event_log_digest": _sha256_bytes(canonical_bytes([])),
            "participant_manifest_digest": _sha256_bytes(canonical_bytes(manifest["participants"])),
            "observer_manifest_digest": _sha256_bytes(canonical_bytes(manifest["observers"])),
            "verifier_source_sha256": verifier_source_digest(), "violations": [],
            "indeterminate_reasons": ["no event records supplied"], "event_count": 0}
    by_id = {}
    for i, event in enumerate(events, 1):
        validate_event(event, i, binding)
        if event["event_id"] in by_id:
            raise TraceError(f"duplicate event_id {event['event_id']}")
        by_id[event["event_id"]] = event
    for event in events:
        for parent_id in event["parent_event_ids"]:
            if parent_id not in by_id:
                raise TraceError(f"{event['event_id']}: missing causal parent {parent_id}")
    # Explicit happens-before graph must be acyclic. Timestamps are never used to infer order.
    visiting, visited = set(), set()
    def visit(event_id):
        if event_id in visiting:
            raise TraceError(f"causal cycle at {event_id}")
        if event_id in visited:
            return
        visiting.add(event_id)
        for parent_id in by_id[event_id]["parent_event_ids"]:
            visit(parent_id)
        visiting.remove(event_id); visited.add(event_id)
    for event_id in by_id:
        visit(event_id)
    ancestors = {event_id: _ancestors(event_id, by_id) for event_id in by_id}

    # Verify append-only per-source/per-epoch hash chains independently.
    streams = defaultdict(list)
    for event in events:
        streams[(event["source_id"], event["source_epoch"])].append(event)
    for stream, rows in streams.items():
        rows.sort(key=lambda x: x["source_sequence"])
        prior = None
        for expected_seq, row in enumerate(rows, 1):
            if row["source_sequence"] != expected_seq:
                raise TraceError(f"source stream {stream}: non-contiguous sequence")
            if row["previous_event_digest"] != prior:
                raise TraceError(f"source stream {stream}: previous digest mismatch at {row['event_id']}")
            if expected_seq > 1 and rows[expected_seq-2]["event_id"] not in row["parent_event_ids"]:
                raise TraceError(f"source stream {stream}: event does not causally follow prior source event")
            prior = row["event_digest"]

    # A coordinator epoch transition must explicitly descend from the exact
    # final record of the previous epoch, not merely an arbitrary old event.
    coordinator_id = m["executor"]["source_id"]
    coordinator_epochs = sorted(epoch for source, epoch in streams if source == coordinator_id)
    if coordinator_epochs:
        if coordinator_epochs[0] != 0 or coordinator_epochs != list(range(coordinator_epochs[-1] + 1)):
            raise TraceError("coordinator source epochs must be contiguous from zero")
        for epoch in coordinator_epochs[1:]:
            prior_rows = streams[(coordinator_id, epoch - 1)]
            current_rows = streams[(coordinator_id, epoch)]
            first = current_rows[0]
            if first["event_type"] != "coordinator_restarted":
                raise TraceError(f"coordinator epoch {epoch}: first event must be coordinator_restarted")
            if prior_rows[-1]["event_id"] not in first["parent_event_ids"]:
                raise TraceError(f"coordinator epoch {epoch}: restart must link the prior epoch head")

    violations, indeterminate = [], []
    event_types = defaultdict(list)
    attempts = defaultdict(list)
    step_verifications = defaultdict(list)
    for e in events:
        event_types[e["event_type"]].append(e)
        if e.get("attempt_id"):
            attempts[e["attempt_id"]].append(e)
        if e.get("event_type") == "step_postcondition_verified":
            step_verifications[e.get("step_id")].append(e)

    def ancestors_of(e):
        return ancestors[e["event_id"]]
    def ancestor_of_type(e, typ, attempt_id=None):
        return any(by_id[a]["event_type"] == typ and (attempt_id is None or by_id[a].get("attempt_id") == attempt_id) for a in ancestors_of(e))
    def exact_step(e):
        sid = e.get("step_id")
        if sid not in m["steps"]:
            _add(violations, "unknown_step", e["event_id"], str(sid)); return None
        return m["steps"][sid]
    def check_step_binding(e, step, data):
        for key in ("target_digest", "operation_id", "request_digest", "operation_binding_digest", "idempotency_key_digest"):
            if data.get(key) != step[key]:
                _add(violations, "step_binding_mismatch", e["event_id"], key)

    # Authorization and write-ahead dispatch must bind exactly to the frozen manifest.
    auth_by_attempt = {}
    dispatch_by_attempt = {}
    for e in events:
        typ = e["event_type"]
        if typ in {"plan_authorized", "dispatch_intent_durable", "participant_accepted", "effect_committed", "transport_response_lost", "attempt_status_observed", "effect_reconciled", "step_postcondition_verified"}:
            step = exact_step(e)
            if step is None:
                continue
            data = e["data"]
            if typ in {"plan_authorized", "dispatch_intent_durable"}:
                check_step_binding(e, step, data)
                if data.get("grant_digest") is None or not isinstance(data.get("grant_digest"), str) or not HEX_256.fullmatch(data["grant_digest"]):
                    _add(violations, "missing_grant_digest", e["event_id"], "exact grant digest required")
                if typ == "plan_authorized":
                    if e["source_kind"] != "authority_adapter" or e["source_id"] != m["authority"]["source_id"] or e["trust_domain_id"] != m["authority"]["trust_domain_id"]:
                        _add(violations, "authority_source_mismatch", e["event_id"], "authorization must be emitted by the separately declared authority adapter")
                    if data.get("authorization_valid") is not True:
                        _add(violations, "authorization_not_valid", e["event_id"], "upstream adapter did not attest exact authorization")
                    if e.get("attempt_id") in auth_by_attempt:
                        _add(violations, "duplicate_authorization", e["event_id"], e.get("attempt_id"))
                    auth_by_attempt[e.get("attempt_id")] = e
                else:
                    auth = auth_by_attempt.get(e.get("attempt_id"))
                    if auth is None or auth["event_id"] not in ancestors_of(e):
                        _add(violations, "dispatch_without_authorization", e["event_id"], "authorization is not a causal predecessor")
                    elif data.get("grant_digest") != auth["data"].get("grant_digest"):
                        _add(violations, "grant_rebinding", e["event_id"], "dispatch grant differs from exact authorization")
                    if e["source_kind"] != "coordinator" or e["source_id"] != m["executor"]["source_id"] or e["trust_domain_id"] != m["executor"]["trust_domain_id"]:
                        _add(violations, "dispatch_source_mismatch", e["event_id"], "dispatch intent must be owned by the declared coordinator")
                    if data.get("durability_attested") is not True:
                        _add(violations, "dispatch_not_durable", e["event_id"], "durable dispatch-intent attestation absent")
                    if e.get("attempt_id") in dispatch_by_attempt:
                        _add(violations, "duplicate_dispatch_intent", e["event_id"], e.get("attempt_id"))
                    dispatch_by_attempt[e.get("attempt_id")] = e
            else:
                dispatch = dispatch_by_attempt.get(e.get("attempt_id"))
                if dispatch is None or dispatch["event_id"] not in ancestors_of(e):
                    _add(violations, "participant_event_without_dispatch", e["event_id"], "no exact durable dispatch ancestor")
                participant = m["participants"].get(step["participant_source_id"])
                if typ in {"participant_accepted", "effect_committed", "effect_reconciled", "attempt_status_observed"}:
                    if participant is None or e["source_id"] != participant["source_id"]:
                        _add(violations, "participant_source_mismatch", e["event_id"], "event not emitted by declared participant")
                    elif e["trust_domain_id"] != participant["trust_domain_id"]:
                        _add(violations, "participant_trust_domain_mismatch", e["event_id"], "participant trust domain differs from manifest")
                    expected_kind = "participant_ledger" if typ in {"effect_reconciled", "attempt_status_observed"} else "participant"
                    if e["source_kind"] != expected_kind:
                        _add(violations, "participant_source_kind_mismatch", e["event_id"], expected_kind)
                if typ == "participant_accepted":
                    for key in ("request_digest", "target_digest", "operation_id", "operation_binding_digest", "idempotency_key_digest"):
                        if data.get(key) != step[key]:
                            _add(violations, "accepted_request_mismatch", e["event_id"], key)
                if typ == "effect_committed":
                    if not data.get("effect_id") or not isinstance(data.get("effect_digest"), str) or not HEX_256.fullmatch(data["effect_digest"]):
                        _add(violations, "invalid_effect_receipt", e["event_id"], "effect ID and digest required")
                    for key in ("request_digest", "target_digest", "operation_id", "operation_binding_digest", "idempotency_key_digest"):
                        if data.get(key) != step[key]:
                            _add(violations, "effect_binding_mismatch", e["event_id"], key)
                    if not ancestor_of_type(e, "participant_accepted", e.get("attempt_id")):
                        _add(violations, "effect_without_acceptance", e["event_id"], "effect must follow participant acceptance")
                if typ == "transport_response_lost":
                    if e["source_kind"] != "coordinator" or e["source_id"] != m["executor"]["source_id"] or e["trust_domain_id"] != m["executor"]["trust_domain_id"]:
                        _add(violations, "response_loss_source_mismatch", e["event_id"], "response loss must be recorded by the declared caller")
                if typ == "attempt_status_observed":
                    outcome = data.get("outcome")
                    if outcome not in {"never_accepted", "accepted_pending", "effect_committed", "unknown"}:
                        _add(violations, "invalid_attempt_status", e["event_id"], str(outcome))
                    if any(data.get(key) != step[key] for key in ("request_digest", "target_digest", "operation_id", "operation_binding_digest", "idempotency_key_digest")):
                        _add(violations, "attempt_status_binding_mismatch", e["event_id"], "ledger observation not bound to exact logical request")
                    if outcome == "never_accepted" and (data.get("consistency") != "strong" or data.get("authoritative") is not True):
                        _add(violations, "weak_no_effect_proof", e["event_id"], "only strong authoritative attempt-ledger evidence may prove never accepted")
                    if outcome == "never_accepted":
                        contradictory = [x for x in attempts.get(e.get("attempt_id"), [])
                            if x["event_type"] in {"participant_accepted", "effect_committed"}]
                        if contradictory:
                            _add(violations, "never_accepted_contradicts_acceptance", e["event_id"],
                                 "attempt ledger says never accepted despite a participant acceptance/effect record")
                if typ == "effect_reconciled":
                    for key in ("request_digest", "target_digest", "operation_id", "operation_binding_digest", "idempotency_key_digest"):
                        if data.get(key) != step[key]:
                            _add(violations, "reconcile_binding_mismatch", e["event_id"], key)
                    if not ancestor_of_type(e, "effect_committed", e.get("attempt_id")):
                        _add(violations, "reconcile_without_effect", e["event_id"], "participant effect receipt is not a causal predecessor")
                    if not isinstance(data.get("effect_digest"), str) or not HEX_256.fullmatch(data["effect_digest"]):
                        _add(violations, "invalid_reconcile_digest", e["event_id"], "effect digest required")
                    else:
                        commits=[by_id[a] for a in ancestors_of(e) if by_id[a]["event_type"]=="effect_committed" and by_id[a].get("attempt_id")==e.get("attempt_id")]
                        if not any(x["data"].get("effect_digest")==data["effect_digest"] and x["data"].get("effect_id")==data.get("effect_id") for x in commits):
                            _add(violations, "reconcile_effect_digest_mismatch", e["event_id"], "reconciled effect identity/digest differs from participant commit")

    # Never permit more than one committed effect for a declared logical key.
    effects_by_key = defaultdict(list)
    for e in event_types["effect_committed"]:
        step = m["steps"].get(e.get("step_id"))
        if step:
            effects_by_key[step["idempotency_key_digest"]].append(e)
    for key, rows in effects_by_key.items():
        if len(rows) > 1:
            _add(violations, "duplicate_business_effect", rows[-1]["event_id"], f"{len(rows)} effects for idempotency key {key}")

    # A new attempt after an ambiguous response requires strong, authoritative
    # never-accepted proof and a fresh, different grant.
    attempts_by_step = defaultdict(list)
    for attempt_id, dispatch in dispatch_by_attempt.items():
        attempts_by_step[dispatch.get("step_id")].append((attempt_id, dispatch))
    for step_id, rows in attempts_by_step.items():
        rows.sort(key=lambda pair: (pair[1]["source_epoch"], pair[1]["source_sequence"]))
        for (prior_id, prior_dispatch), (next_id, next_dispatch) in zip(rows, rows[1:]):
            prior_events = attempts.get(prior_id, [])
            reconciled = any(e["event_type"] == "effect_reconciled" for e in prior_events)
            no_effect = [e for e in prior_events if e["event_type"] == "attempt_status_observed" and e["data"].get("outcome") == "never_accepted" and e["data"].get("consistency") == "strong" and e["data"].get("authoritative") is True]
            if not reconciled and not no_effect:
                _add(violations, "retry_after_unknown", next_dispatch["event_id"], f"prior attempt {prior_id} has no authoritative terminal reconciliation")
            elif reconciled:
                _add(violations, "retry_after_effect_commit", next_dispatch["event_id"], f"prior attempt {prior_id} already has a reconciled business effect")
            else:
                proof_is_causal = any(proof["event_id"] in ancestors_of(next_dispatch) for proof in no_effect)
                old_grant = auth_by_attempt.get(prior_id, {}).get("data", {}).get("grant_digest")
                new_grant = auth_by_attempt.get(next_id, {}).get("data", {}).get("grant_digest")
                if not proof_is_causal:
                    _add(violations, "retry_without_causal_no_effect_proof", next_dispatch["event_id"], "authoritative never-accepted proof must precede new dispatch")
                if next_id == prior_id or not new_grant or new_grant == old_grant:
                    _add(violations, "retry_without_fresh_authority", next_dispatch["event_id"], "new attempt requires a new grant and attempt ID")

    # Crash/restart barrier: every recovery scan must be causally downstream of
    # a restart, and every pre-restart dispatched attempt must be accounted for.
    restarts = event_types["coordinator_restarted"]
    for scan in event_types["recovery_scan_completed"]:
        if not any(r["event_id"] in ancestors_of(scan) for r in restarts):
            _add(violations, "recovery_scan_without_restart", scan["event_id"], "recovery scan must follow a coordinator restart")
    for restart in restarts:
        if restart["source_id"] != m["executor"]["source_id"] or restart["source_kind"] != "coordinator" or restart["trust_domain_id"] != m["executor"]["trust_domain_id"]:
            _add(violations, "restart_source_mismatch", restart["event_id"], "restart must be owned by the declared coordinator")
        later_dispatches = [d for d in event_types["dispatch_intent_durable"] if restart["event_id"] in ancestors_of(d)]
        scans = [s for s in event_types["recovery_scan_completed"] if restart["event_id"] in ancestors_of(s)]
        for dispatch in later_dispatches:
            if not scans or not any(s["event_id"] in ancestors_of(dispatch) for s in scans):
                _add(violations, "dispatch_without_recovery_scan", dispatch["event_id"], "post-restart dispatch requires a causal recovery scan")
            elif any(s["data"].get("unresolved_attempt_ids") for s in scans if s["event_id"] in ancestors_of(dispatch)):
                _add(violations, "dispatch_with_unresolved_attempts", dispatch["event_id"], "recovery scan left unresolved attempts")
        for scan in scans:
            if scan["source_id"] != m["executor"]["source_id"] or scan["source_kind"] != "coordinator" or scan["trust_domain_id"] != m["executor"]["trust_domain_id"]:
                _add(violations, "recovery_scan_source_mismatch", scan["event_id"], "recovery scan must be owned by the declared coordinator")
            _require(scan["data"], ("scanned_attempt_ids", "unresolved_attempt_ids", "reconciled_attempt_ids"), scan["event_id"]+".data")
            scanned = _strings(scan["data"]["scanned_attempt_ids"], scan["event_id"]+".scanned_attempt_ids", allow_empty=True)
            unresolved = _strings(scan["data"]["unresolved_attempt_ids"], scan["event_id"]+".unresolved_attempt_ids", allow_empty=True)
            reconciled_ids = _strings(scan["data"]["reconciled_attempt_ids"], scan["event_id"]+".reconciled_attempt_ids", allow_empty=True)
            prior_attempts = {a for a,d in dispatch_by_attempt.items() if d["event_id"] in ancestors_of(restart)}
            if not prior_attempts.issubset(scanned):
                _add(violations, "recovery_scan_incomplete", scan["event_id"], f"not scanned: {sorted(prior_attempts-scanned)}")
            for attempt_id in scanned - prior_attempts:
                _add(violations, "recovery_scan_scanned_unknown_attempt", scan["event_id"], attempt_id)
            for attempt_id in reconciled_ids | unresolved:
                if attempt_id not in prior_attempts:
                    _add(violations, "recovery_scan_references_unknown_attempt", scan["event_id"], attempt_id)
            if scanned != (reconciled_ids | unresolved):
                _add(violations, "recovery_scan_accounting_incomplete", scan["event_id"], "every scanned attempt must be reconciled or remain unresolved")
            if reconciled_ids & unresolved:
                _add(violations, "recovery_scan_state_conflict", scan["event_id"], "attempt cannot be both reconciled and unresolved")
            for attempt_id in prior_attempts:
                related = attempts.get(attempt_id, [])
                effect_proof = any(x["event_type"] == "effect_reconciled" and x["event_id"] in ancestors_of(scan) for x in related)
                no_effect_proof = any(x["event_type"] == "attempt_status_observed" and x["event_id"] in ancestors_of(scan)
                    and x["data"].get("outcome") == "never_accepted" and x["data"].get("consistency") == "strong"
                    and x["data"].get("authoritative") is True for x in related)
                proven_terminal = effect_proof or no_effect_proof
                if attempt_id in reconciled_ids and not proven_terminal:
                    _add(violations, "recovery_scan_claim_unproven", scan["event_id"], attempt_id)
                elif proven_terminal and attempt_id not in reconciled_ids:
                    _add(violations, "recovery_scan_omits_reconciled_attempt", scan["event_id"], attempt_id)
                elif not proven_terminal and attempt_id not in unresolved:
                    _add(violations, "recovery_scan_unknown_omitted", scan["event_id"], attempt_id)

    # Step verification requires exact reconciled effect and an independent observer.
    verified_steps = set()
    for e in event_types["step_postcondition_verified"]:
        step = m["steps"].get(e.get("step_id"))
        if step is None:
            _add(violations, "unknown_verified_step", e["event_id"], str(e.get("step_id"))); continue
        obs = m["observers"].get(e["source_id"])
        data = e["data"]
        if e["source_kind"] != "independent_observer" or obs is None or e["trust_domain_id"] != obs["trust_domain_id"]:
            _add(violations, "observer_source_mismatch", e["event_id"], "step verifier not in declared observer inventory")
        if obs and e.get("step_id") not in obs["scope"] and step["target_digest"] not in obs["scope"]:
            _add(violations, "observer_scope_mismatch", e["event_id"], "observer is not declared for this step/target")
        for key in ("observer_id", "target_digest", "expected_postcondition_digest"):
            expected = obs.get(key) if obs else None
            if key == "target_digest": expected = step["target_digest"]
            if key == "expected_postcondition_digest": expected = step["expected_postcondition_digest"]
            if data.get(key) != expected:
                _add(violations, "observer_binding_mismatch", e["event_id"], key)
        if data.get("observed_postcondition_digest") != step["expected_postcondition_digest"]:
            _add(violations, "postcondition_mismatch", e["event_id"], "observed digest does not match expected")
        if not ancestor_of_type(e, "effect_reconciled", e.get("attempt_id")):
            _add(violations, "postcondition_without_effect_reconciliation", e["event_id"], "no reconciled participant effect is a causal predecessor")
        captured = _time(data.get("captured_at"), e["event_id"]+".captured_at")
        valid_until = _time(data.get("valid_until"), e["event_id"]+".valid_until")
        if valid_until <= captured:
            _add(violations, "expired_postcondition_observation", e["event_id"], "observation expiry must follow capture")
        if not isinstance(data.get("evidence_digest"), str) or not HEX_256.fullmatch(data["evidence_digest"]):
            _add(violations, "missing_observation_digest", e["event_id"], "observer evidence digest required")
        verified_steps.add(e["step_id"])

    # Service outcome is a distinct, independent final check over every required step.
    for e in event_types["service_outcome_verified"]:
        data = e["data"]
        if e["source_kind"] != "independent_observer" or e["source_id"] not in m["observers"]:
            _add(violations, "service_observer_mismatch", e["event_id"], "service oracle not in observer inventory")
        obs = m["observers"].get(e["source_id"])
        if obs and e["trust_domain_id"] != obs["trust_domain_id"]:
            _add(violations, "service_observer_trust_mismatch", e["event_id"], "trust domain mismatch")
        if obs and data.get("observer_id") != obs["observer_id"]:
            _add(violations, "service_observer_identity_mismatch", e["event_id"], "observer ID does not match manifest")
        if obs and "service" not in obs["scope"]:
            _add(violations, "service_observer_scope_mismatch", e["event_id"], "observer is not declared for service-level verification")
        for sid, step in m["steps"].items():
            if sid not in verified_steps:
                _add(violations, "service_outcome_before_step_verification", e["event_id"], sid)
            step_ancestors = [v for v in step_verifications.get(sid, []) if v["event_id"] in ancestors_of(e)]
            if not step_ancestors:
                _add(violations, "service_outcome_missing_causal_step", e["event_id"], sid)
            else:
                service_capture = _time(data.get("captured_at"), e["event_id"]+".captured_at")
                for step_event in step_ancestors:
                    step_valid_until = _time(step_event["data"].get("valid_until"), step_event["event_id"]+".valid_until")
                    if service_capture > step_valid_until:
                        _add(violations, "stale_step_observation_at_service_check", e["event_id"], sid)
        if data.get("expected_service_digest") != binding["expected_service_digest"]:
            _add(violations, "expected_service_digest_mismatch", e["event_id"], "service check does not bind the frozen expected digest")
        if not isinstance(data.get("observed_service_digest"), str) or not HEX_256.fullmatch(data["observed_service_digest"]):
            _add(violations, "invalid_observed_service_digest", e["event_id"], "observed service digest required")
        if data.get("observed_service_digest") != data.get("expected_service_digest"):
            _add(violations, "service_digest_mismatch", e["event_id"], "service digest mismatch")
        service_capture = _time(data.get("captured_at"), e["event_id"]+".captured_at")
        service_valid_until = _time(data.get("valid_until"), e["event_id"]+".valid_until")
        if service_valid_until <= service_capture:
            _add(violations, "expired_service_observation", e["event_id"], "service observation validity window is empty")
        if not isinstance(data.get("evidence_digest"), str) or not HEX_256.fullmatch(data["evidence_digest"]):
            _add(violations, "missing_service_evidence_digest", e["event_id"], "service observer evidence digest required")

    # A dispatched attempt is settled only by an authoritative never-accepted
    # proof or by participant effect reconciliation. Missing evidence remains unknown.
    unresolved_attempts = []
    for attempt_id, dispatch in dispatch_by_attempt.items():
        related = attempts.get(attempt_id, [])
        settled_no_effect = any(e["event_type"] == "attempt_status_observed" and e["data"].get("outcome") == "never_accepted" and e["data"].get("consistency") == "strong" and e["data"].get("authoritative") is True for e in related)
        settled_effect = any(e["event_type"] == "effect_reconciled" for e in related)
        if not (settled_no_effect or settled_effect):
            unresolved_attempts.append(attempt_id)
    service_events = event_types["service_outcome_verified"]
    for e in service_events:
        if unresolved_attempts:
            _add(violations, "service_outcome_with_unresolved_attempts", e["event_id"], str(sorted(unresolved_attempts)))
    for e in event_types["agent_declared_resolved"]:
        if e["source_kind"] != "coordinator" or e["source_id"] != m["executor"]["source_id"] or e["trust_domain_id"] != m["executor"]["trust_domain_id"]:
            _add(violations, "agent_claim_source_mismatch", e["event_id"], "agent claim must be emitted by the declared executor")
        if e["data"].get("claim") != "resolved":
            _add(violations, "invalid_agent_claim", e["event_id"], "agent_declared_resolved event must contain claim=resolved")
        if not any(s["event_id"] in ancestors_of(e) for s in service_events):
            _add(violations, "false_resolution_claim", e["event_id"], "resolved claim lacks a causal independent service verification")

    if violations:
        status = "invalid"
    elif not service_events:
        status = "indeterminate"
        indeterminate.append("no independent service outcome verification was supplied")
    elif unresolved_attempts:
        status = "indeterminate"
        indeterminate.append("unresolved attempts: " + ", ".join(sorted(unresolved_attempts)))
    else:
        status = "valid"
    if not service_events:
        missing_steps = sorted(set(m["steps"]) - verified_steps)
        if missing_steps:
            indeterminate.append("required steps not yet independently verified: " + ", ".join(missing_steps))
        if unresolved_attempts:
            indeterminate.append("unresolved attempts: " + ", ".join(sorted(unresolved_attempts)))
    participants_digest = _sha256_bytes(canonical_bytes(manifest["participants"]))
    observers_digest = _sha256_bytes(canonical_bytes(manifest["observers"]))
    manifest_digest = _sha256_bytes(canonical_bytes(manifest))
    event_log_digest = _sha256_bytes(canonical_bytes(events))
    return {
        "schema": "luminous-operational-recovery-verification-v0.1", "version": 1,
        "status": status, "run_id": binding["run_id"], "episode_id": binding["episode_id"],
        "source_commit_sha": binding["source_commit_sha"], "manifest_digest": manifest_digest,
        "event_log_digest": event_log_digest, "participant_manifest_digest": participants_digest,
        "observer_manifest_digest": observers_digest, "verifier_source_sha256": verifier_source_digest(),
        "scenario_variant_digest": binding["scenario_variant_digest"], "fault_matrix_digest": binding["fault_matrix_digest"],
        "event_count": len(events),
        "verified_required_steps": sorted(verified_steps),
        "required_step_count": len(m["steps"]), "service_outcome_verifications": len(service_events),
        "unresolved_attempt_ids": sorted(unresolved_attempts),
        "false_resolution_claims": sum(v["code"] == "false_resolution_claim" for v in violations),
        "violations": violations, "indeterminate_reasons": indeterminate,
        "claim_ceiling": "Causal record consistency only; signatures, source authenticity, observer independence, filesystem durability and real service truth require independent qualification."
    }

def read_jsonl(path):
    rows=[]
    with Path(path).open(encoding="utf-8") as f:
        for n,line in enumerate(f,1):
            if not line.strip(): continue
            try: rows.append(json.loads(line))
            except json.JSONDecodeError as exc: raise TraceError(f"{path} line {n}: {exc}") from exc
    return rows

def main(argv=None):
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest"); parser.add_argument("events"); parser.add_argument("--output",default="-")
    args=parser.parse_args(argv)
    try:
        manifest=json.loads(Path(args.manifest).read_text(encoding="utf-8"))
        result=verify(manifest,read_jsonl(args.events))
        text=json.dumps(result,indent=2,sort_keys=True)+"\n"
        if args.output == "-": sys.stdout.write(text)
        else: Path(args.output).write_text(text,encoding="utf-8")
        return 0 if result["status"] == "valid" else 1 if result["status"] == "invalid" else 2
    except (OSError,json.JSONDecodeError,TraceError) as exc:
        print(f"recovery-trace: {exc}",file=sys.stderr); return 2

if __name__ == "__main__":
    raise SystemExit(main())
