#!/usr/bin/env python3
"""Synthetic WIMSE mandate Trust-Framework checker; not a crypto verifier."""
from __future__ import annotations

import copy
import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path("research/agentic-threat-resilience-v0.1")
SCHEMA = ROOT / "schemas/trust-framework-conformance-v0.1.schema.json"
CORPUS = ROOT / "scenarios/trust-framework-conformance-v0.1.json"


def evaluate_instruction(case: dict) -> tuple[str, list[str]]:
    tf = case["trust_framework"]
    mandate = case["mandate"]
    evidence = case["instruction_evidence"]
    invalid: set[str] = set()
    indeterminate: set[str] = set()

    method = tf["instruction_method"]
    if method == "unspecified":
        invalid.add("trust_framework_instruction_method_unspecified")
    elif method == "delegator_exact_mandate_signature":
        if evidence["evidence_kind"] != "delegator_signature":
            invalid.add("delegator_signature_required")
        if evidence["signer"] != mandate["delegator"] or evidence["signer_role"] != "delegator":
            invalid.add("delegator_signer_mismatch")
        if evidence["subject_digest"] != mandate["content_digest"]:
            invalid.add("mandate_content_digest_mismatch")
        if evidence["verification_state"] == "invalid":
            invalid.add("instruction_signature_invalid")
        elif evidence["verification_state"] == "unknown":
            indeterminate.add("instruction_signature_unresolved")
    elif method == "mandate_issuer_qualified_process":
        if evidence["evidence_kind"] != "issuer_process_attestation":
            invalid.add("qualified_issuer_process_required")
        if evidence["signer_role"] != "independent_assurance" or evidence["signer"] == mandate["issuer"]:
            if tf["require_role_separation"]:
                invalid.add("issuer_process_role_separation_failed")
        if evidence["subject_digest"] != mandate["content_digest"]:
            invalid.add("mandate_content_digest_mismatch")
        if evidence["process_assurance"] == "unqualified":
            invalid.add("issuer_process_unqualified")
        elif evidence["process_assurance"] == "unknown":
            indeterminate.add("issuer_process_assurance_unknown")
        if evidence["verification_state"] == "invalid":
            invalid.add("instruction_process_attestation_invalid")
        elif evidence["verification_state"] == "unknown":
            indeterminate.add("instruction_process_attestation_unresolved")

    if evidence["principal"] != mandate["principal"] or evidence["delegator"] != mandate["delegator"]:
        invalid.add("instruction_party_binding_mismatch")
    if evidence["status"] in ("withdrawn", "superseded"):
        invalid.add("instruction_withdrawn_or_superseded")
    elif evidence["status"] in ("stale", "unknown"):
        indeterminate.add("instruction_status_unresolved")
    elif evidence["status"] == "not_applicable":
        invalid.add("instruction_status_required")
    if evidence["checked_at"] >= evidence["expires_at"]:
        invalid.add("instruction_evidence_expired")
    if evidence["expires_at"] <= 0:
        invalid.add("instruction_evidence_invalid_expiry")

    codes = sorted(invalid | indeterminate)
    if invalid:
        return "invalid", codes
    if indeterminate:
        return "indeterminate", codes
    return "valid", codes


def evaluate_budget(case: dict) -> tuple[str, list[str]]:
    tf = case["trust_framework"]
    events = sorted(case["consumption_events"], key=lambda event: event["sequence"])
    invalid: set[str] = set()
    indeterminate: set[str] = set()
    sequences = [event["sequence"] for event in events]
    if len(sequences) != len(set(sequences)):
        invalid.add("duplicate_event_sequence")

    # Unknown attempts block every subsequent distinct consumption until the
    # original request is reconciled as absent or committed.
    pending: dict[str, dict] = {}
    committed_by_rp: dict[str, int] = {}
    total_committed = 0
    for event in events:
        request_id = event["request_id"]
        state = event["state"]

        if pending and state in ("committed", "unknown") and request_id not in pending:
            invalid.add("new_consumption_while_prior_outcome_unknown")

        if state == "unknown":
            pending[request_id] = event
            continue
        if state == "reconciled_absent":
            if request_id not in pending:
                invalid.add("absent_reconciliation_without_unknown")
            else:
                del pending[request_id]
            continue
        if state == "reconciled_committed":
            if request_id not in pending:
                invalid.add("commit_reconciliation_without_unknown")
            else:
                original = pending.pop(request_id)
                if original["idempotency_key"] != event["idempotency_key"]:
                    invalid.add("reconciliation_idempotency_key_changed")
                if original["ledger_id"] != event["ledger_id"] or original["rp"] != event["rp"]:
                    invalid.add("reconciliation_subject_changed")
                if original["units"] != event["units"]:
                    invalid.add("reconciliation_units_changed")
                total_committed += event["units"]
                committed_by_rp[event["rp"]] = committed_by_rp.get(event["rp"], 0) + event["units"]
            continue
        if state == "committed":
            total_committed += event["units"]
            committed_by_rp[event["rp"]] = committed_by_rp.get(event["rp"], 0) + event["units"]

    if pending:
        indeterminate.add("consumption_outcome_unresolved")

    if tf["authority_budget_ceiling"] < 0:
        invalid.add("invalid_authority_budget_ceiling")

    if tf["budget_mode"] == "shared_global_ledger":
        ledger_id = tf["shared_ledger_id"]
        budget = tf["global_budget"]
        if not ledger_id or budget is None:
            invalid.add("shared_ledger_profile_incomplete")
        if budget is not None and budget > tf["authority_budget_ceiling"]:
            invalid.add("shared_budget_exceeds_authority_ceiling")
        if any(event["ledger_id"] != ledger_id for event in events):
            invalid.add("relying_party_used_nonshared_ledger")
        if budget is not None and total_committed > budget:
            invalid.add("shared_budget_overspent")

    elif tf["budget_mode"] == "allocated_per_rp":
        allocations = tf["rp_allocations"]
        if not allocations:
            invalid.add("per_rp_allocations_missing")
        if sum(value["max_units"] for value in allocations.values()) > tf["authority_budget_ceiling"]:
            invalid.add("per_rp_allocations_exceed_authority_ceiling")
        for event in events:
            allocation = allocations.get(event["rp"])
            if allocation is None:
                invalid.add("relying_party_has_no_explicit_allocation")
                continue
            if event["ledger_id"] != allocation["ledger_id"]:
                invalid.add("relying_party_ledger_mismatch")
        for rp, consumed in committed_by_rp.items():
            allocation = allocations.get(rp)
            if allocation is None or consumed > allocation["max_units"]:
                invalid.add("per_rp_budget_overspent")

    codes = sorted(invalid | indeterminate)
    if invalid:
        return "invalid", codes
    if indeterminate:
        return "indeterminate", codes
    return "qualified", codes


def decision(instruction: str, budget: str) -> str:
    if instruction == "invalid" or budget == "invalid":
        return "deny"
    if instruction == "indeterminate" or budget == "indeterminate":
        return "unknown_reconciliation_required"
    return "allow"


def run_mutation_controls(cases: list[dict]) -> int:
    by_id = {case["case_id"]: case for case in cases}
    mutations: list[tuple[str, dict, str, str]] = []

    def changed(case_id: str, label: str, mutate, want_instruction: str, want_budget: str):
        item = copy.deepcopy(by_id[case_id])
        mutate(item)
        mutations.append((label, item, want_instruction, want_budget))

    changed("TF-001", "wrong-exact-digest", lambda c: c["instruction_evidence"].update(subject_digest="other"), "invalid", "qualified")
    changed("TF-001", "wrong-delegator-signer", lambda c: c["instruction_evidence"].update(signer="other", signer_role="other"), "invalid", "qualified")
    changed("TF-001", "withdrawn-instruction", lambda c: c["instruction_evidence"].update(status="withdrawn"), "invalid", "qualified")
    changed("TF-001", "unresolved-instruction-status", lambda c: c["instruction_evidence"].update(status="unknown"), "indeterminate", "qualified")
    changed("TF-007", "unqualified-issuer-process", lambda c: c["instruction_evidence"].update(process_assurance="unqualified"), "invalid", "qualified")
    changed("TF-007", "issuer-self-approval", lambda c: c["instruction_evidence"].update(signer=c["mandate"]["issuer"], signer_role="mandate_issuer"), "invalid", "qualified")
    changed("TF-001", "wrong-rp-ledger", lambda c: c["consumption_events"][0].update(ledger_id="local-rp-ledger"), "valid", "invalid")
    changed("TF-001", "global-budget-overrun", lambda c: c["consumption_events"].append({"sequence": 2, "rp": "rp-two", "request_id": "request-2", "idempotency_key": "idem-2", "ledger_id": "ledger-global-1", "units": 2, "state": "committed"}), "valid", "invalid")
    changed("TF-011", "remove-rp-allocation", lambda c: c["trust_framework"]["rp_allocations"].pop("rp-two"), "valid", "invalid")
    changed("TF-013", "commit-before-unknown-reconcile", lambda c: c["consumption_events"].append({"sequence": 2, "rp": "rp-two", "request_id": "request-2", "idempotency_key": "idem-2", "ledger_id": "ledger-global-1", "units": 1, "state": "committed"}), "valid", "invalid")

    for label, case, expected_instruction, expected_budget in mutations:
        actual_instruction, _ = evaluate_instruction(case)
        actual_budget, _ = evaluate_budget(case)
        if (actual_instruction, actual_budget) != (expected_instruction, expected_budget):
            raise SystemExit(
                f"mutation {label}: expected {(expected_instruction, expected_budget)}, "
                f"got {(actual_instruction, actual_budget)}"
            )
    return len(mutations)


def main() -> None:
    schema = json.loads(SCHEMA.read_text())
    corpus = json.loads(CORPUS.read_text())
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)
    errors = sorted(validator.iter_errors(corpus), key=lambda error: list(error.absolute_path))
    if errors:
        for error in errors:
            location = "/".join(str(part) for part in error.absolute_path)
            print(f"schema {location}: {error.message}")
        raise SystemExit(f"{len(errors)} schema error(s)")

    cases = corpus["cases"]
    ids = [case["case_id"] for case in cases]
    if len(ids) != len(set(ids)):
        raise SystemExit("duplicate case IDs")
    if len(cases) != 13:
        raise SystemExit(f"expected 13 cases, got {len(cases)}")

    for case in cases:
        actual_instruction, instruction_codes = evaluate_instruction(case)
        actual_budget, budget_codes = evaluate_budget(case)
        actual_decision = decision(actual_instruction, actual_budget)
        expected = case["expected"]
        if actual_instruction != expected["instruction_outcome"]:
            raise SystemExit(f"{case['case_id']}: instruction expected {expected['instruction_outcome']} got {actual_instruction}; codes={instruction_codes}")
        if actual_budget != expected["budget_outcome"]:
            raise SystemExit(f"{case['case_id']}: budget expected {expected['budget_outcome']} got {actual_budget}; codes={budget_codes}")
        if actual_decision != expected["decision"]:
            raise SystemExit(f"{case['case_id']}: decision expected {expected['decision']} got {actual_decision}")

    mutation_count = run_mutation_controls(cases)
    print(f"trust-framework conformance corpus valid: {len(cases)}/{len(cases)} cases; {mutation_count} mutations; synthetic profile only")


if __name__ == "__main__":
    main()
