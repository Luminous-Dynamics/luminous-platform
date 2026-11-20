#!/usr/bin/env python3
"""Test-only APAC v0.1 schema and semantic validator.

This validates synthetic preflight/commit composition cases only. It does not
implement Mycelix authority, WALP currentness, a broker, a participant, or an
effect executor.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "docs" / "security"


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot load {path.relative_to(ROOT)}: {exc}") from exc


def request_policy_digest_mismatch(case: dict[str, Any]) -> bool:
    policy = case["consumer_policy"]
    prereq = case["prerequisite_set"]
    request = case["request"]
    preflight = case["preflight"]
    commit = case["commit"]
    return any([
        request["consumer_policy_digest"] != policy["policy_digest"],
        preflight["policy_digest"] != policy["policy_digest"],
        commit["policy_digest_at_linearization"] != policy["policy_digest"],
        request["prerequisite_set_digest"] != prereq["set_digest"],
        preflight["prerequisite_set_digest"] != prereq["set_digest"],
        commit["prerequisite_set_digest_at_linearization"] != prereq["set_digest"],
        preflight["authority_grant_ref"] != request["authority_grant_ref"],
        commit["authority_grant_ref_at_linearization"] != request["authority_grant_ref"],
    ])


def assurance_claims_for_commit(case: dict[str, Any]) -> list[dict[str, Any]] | None:
    prereq = case["prerequisite_set"]
    commit = case["commit"]
    revalidation = commit["assurance_revalidation"]
    if commit["assurance_snapshot_matches_preflight"]:
        if revalidation["state"] != "not_attempted":
            return None
        return prereq["claims"]

    if revalidation["state"] != commit["revalidation_state"]:
        return None
    if revalidation["state"] != "passed":
        return []
    if (
        revalidation["request_digest"] != case["request"]["request_digest"]
        or revalidation["prerequisite_set_digest"] != prereq["set_digest"]
        or revalidation["snapshot_digest"] != commit["assurance_snapshot_digest_at_linearization"]
        or not revalidation["result_ref"]
        or not revalidation["result_digest"]
    ):
        return None

    old_claims = prereq["claims"]
    fresh_claims = revalidation["validated_claims"]
    if len(old_claims) != len(fresh_claims):
        return None
    old_by_id = {claim["claim_id"]: claim for claim in old_claims}
    if len(old_by_id) != len(old_claims):
        return None
    fresh_ids = [claim["claim_id"] for claim in fresh_claims]
    if set(fresh_ids) != set(old_by_id) or len(set(fresh_ids)) != len(fresh_ids):
        return None
    for fresh in fresh_claims:
        old = old_by_id[fresh["claim_id"]]
        for field in ("property", "subject_manifest_digest"):
            if fresh[field] != old[field]:
                return None
        changed = (
            fresh["qualification_result_ref"] != old["qualification_result_ref"]
            or fresh["qualification_result_digest"] != old["qualification_result_digest"]
        )
        if changed:
            if (
                fresh.get("predecessor_qualification_result_ref") != old["qualification_result_ref"]
                or fresh.get("predecessor_qualification_result_digest") != old["qualification_result_digest"]
                or fresh["qualification_result_ref"] == old["qualification_result_ref"]
                or fresh["qualification_result_digest"] == old["qualification_result_digest"]
            ):
                return None
        elif fresh.get("predecessor_qualification_result_ref") is not None or fresh.get("predecessor_qualification_result_digest") is not None:
            return None
    return fresh_claims


def assurance_revalidation_shape_valid(case: dict[str, Any]) -> bool:
    commit = case["commit"]
    revalidation = commit["assurance_revalidation"]
    if revalidation["state"] != commit["revalidation_state"]:
        return False
    if commit["assurance_snapshot_matches_preflight"]:
        return revalidation["state"] == "not_attempted" and not revalidation["validated_claims"]
    if revalidation["state"] == "not_attempted":
        return False
    return (
        revalidation["request_digest"] == case["request"]["request_digest"]
        and revalidation["prerequisite_set_digest"] == case["prerequisite_set"]["set_digest"]
        and revalidation["snapshot_digest"] == commit["assurance_snapshot_digest_at_linearization"]
        and bool(revalidation["result_ref"])
        and bool(revalidation["result_digest"])
    )


def authority_revalidation_shape_valid(case: dict[str, Any]) -> bool:
    commit = case["commit"]
    receipt = commit["authority_revalidation"]
    if receipt["state"] != commit["authority_revalidation_state"]:
        return False
    if commit["authority_snapshot_matches_preflight"]:
        return receipt["state"] == "not_attempted"
    if receipt["state"] == "not_attempted":
        return False
    return (
        receipt["request_digest"] == case["request"]["request_digest"]
        and receipt["grant_ref"] == case["request"]["authority_grant_ref"]
        and receipt["snapshot_digest"] == commit["authority_snapshot_digest_at_linearization"]
        and receipt["authority_state"] == commit["authority_state_at_linearization"]
        and bool(receipt["result_ref"])
        and bool(receipt["result_digest"])
    )


def assurance_result(case: dict[str, Any]) -> str:
    policy = case["consumer_policy"]
    prereq = case["prerequisite_set"]
    preflight = case["preflight"]
    commit = case["commit"]

    if preflight["request_digest"] != case["request"]["request_digest"] or commit["request_digest_at_linearization"] != case["request"]["request_digest"]:
        return "denied"
    if request_policy_digest_mismatch(case):
        return "denied"
    if not prereq["explicitly_declared"]:
        return "indeterminate"
    if not prereq["claims"]:
        if prereq["intentionally_empty"] and prereq["explicitly_declared"] and not prereq["required"] and policy["allow_explicit_empty_set"]:
            return "not_required"
        return "indeterminate"

    if commit["policy_digest_at_linearization"] != policy["policy_digest"]:
        return "denied"
    if commit["prerequisite_set_digest_at_linearization"] != prereq["set_digest"]:
        return "denied"

    assurance_snapshot_equal = (
        commit["assurance_snapshot_digest_at_linearization"] == preflight["assurance_snapshot_digest"]
    )
    if commit["assurance_snapshot_matches_preflight"] != assurance_snapshot_equal:
        return "denied"
    if not assurance_revalidation_shape_valid(case):
        return "denied"
    claims = assurance_claims_for_commit(case)
    if claims is None:
        return "denied"
    if not commit["assurance_snapshot_matches_preflight"]:
        if commit["revalidation_state"] == "failed":
            return "denied"
        if commit["revalidation_state"] == "indeterminate":
            return "indeterminate"
        if commit["revalidation_state"] != "passed":
            return "indeterminate"

    if commit["linearization_order"] == "unresolved_race":
        return "indeterminate"

    saw_indeterminate = False
    for claim in claims:
        if not claim["subject_matches"]:
            return "denied"
        state = claim["lifecycle_state_at_commit"]
        if state in {"expired","invalidated","withdrawn","superseded","requires_requalification","forked"}:
            return "denied"
        if state == "indeterminate":
            saw_indeterminate = True
            continue

        if policy["require_complete_dependency_coverage"]:
            if claim["dependency_coverage_at_commit"] != "complete" or claim["unresolved_dependency_count"] != 0:
                saw_indeterminate = True
                continue

        app = claim["applicability_at_commit"]
        if app in {"historical_only","inapplicable","requires_targeted_revalidation"}:
            return "denied"
        if app == "unknown":
            saw_indeterminate = True
            continue
        if app not in claim["acceptable_applicability"]:
            return "denied"
        if app == "conditionally_transferable" and not claim["bridge_ref"]:
            return "denied"

        required = claim["required_currentness_class"]
        actual = claim["actual_currentness_class"]
        if actual == "Unknown":
            saw_indeterminate = True
            continue
        if required == "CurrentThroughVerifiedCheckpoint" and actual != "CurrentThroughVerifiedCheckpoint":
            return "denied"
        if required == "TerminalInSuppliedView" and actual not in {"TerminalInSuppliedView","CurrentThroughVerifiedCheckpoint"}:
            return "denied"

    if saw_indeterminate:
        return "indeterminate"
    return "eligible"


def authority_result(case: dict[str, Any]) -> str:
    preflight = case["preflight"]
    commit = case["commit"]
    request = case["request"]
    if preflight["request_digest"] != request["request_digest"] or commit["request_digest_at_linearization"] != request["request_digest"]:
        return "denied"
    if preflight["authority_grant_ref"] != request["authority_grant_ref"] or commit["authority_grant_ref_at_linearization"] != request["authority_grant_ref"]:
        return "denied"
    authority_snapshot_equal = (
        commit["authority_snapshot_digest_at_linearization"] == preflight["authority_snapshot_digest"]
    )
    if commit["authority_snapshot_matches_preflight"] != authority_snapshot_equal:
        return "denied"
    if not authority_revalidation_shape_valid(case):
        return "denied"
    if not commit["authority_snapshot_matches_preflight"]:
        if commit["authority_revalidation_state"] == "failed":
            return "denied"
        if commit["authority_revalidation_state"] != "passed":
            return "indeterminate"
    state = commit["authority_state_at_linearization"]
    if state == "current":
        return "eligible"
    if state in {"revoked","expired","denied"}:
        return "denied"
    return "indeterminate"


def admission_result(authority: str, assurance: str) -> str:
    if authority == "denied" or assurance == "denied":
        return "deny"
    if authority == "indeterminate" or assurance == "indeterminate":
        return "hold"
    return "allow"


def main() -> int:
    schema = load_json(BASE / "assurance-prerequisite-admission.schema.v0.1.json")
    suite = load_json(BASE / "assurance-prerequisite-admission.fixtures.v0.1.json")
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(
        validator.iter_errors(suite),
        key=lambda error: tuple(str(part) for part in error.absolute_path),
    )
    if errors:
        for error in errors:
            loc = "/".join(str(part) for part in error.absolute_path) or "<root>"
            print(f"{loc}: {error.message}", file=sys.stderr)
        raise SystemExit(f"APAC schema validation failed: {len(errors)} error(s)")

    if suite["status"] != "synthetic_fixture_suite_not_executed":
        raise SystemExit("APAC fixtures must remain not executed")
    if suite["claim_ceiling"] != "synthetic_commit_boundary_conformance_only":
        raise SystemExit("APAC claim ceiling was weakened")
    if suite["no_aggregate_score"] is not True:
        raise SystemExit("APAC must not introduce an aggregate score")
    if suite["digest_semantics"] != "synthetic_fixed_shape_only_no_canonical_hash_recomputation":
        raise SystemExit("APAC digests are synthetic placeholders, not canonical cryptographic commitments")

    cases = suite["scenarios"]
    if len(cases) != 27:
        raise SystemExit(f"expected 27 APAC scenarios, found {len(cases)}")
    ids: set[str] = set()
    class_counts: dict[str, int] = {}
    for case in cases:
        sid = case["scenario_id"]
        if sid in ids:
            raise SystemExit(f"duplicate APAC scenario {sid}")
        ids.add(sid)
        cls = case["control_class"]
        class_counts[cls] = class_counts.get(cls, 0) + 1

        request = case["request"]
        policy = case["consumer_policy"]
        prereq = case["prerequisite_set"]
        preflight = case["preflight"]
        commit = case["commit"]
        effect = case["effect"]

        if request["consumer_policy_digest"] != policy["policy_digest"]:
            raise SystemExit(f"{sid}: request is not bound to consumer policy")
        if request["prerequisite_set_digest"] != prereq["set_digest"]:
            raise SystemExit(f"{sid}: request is not bound to prerequisite-set digest")
        if not assurance_revalidation_shape_valid(case):
            raise SystemExit(f"{sid}: assurance revalidation receipt is not bound to the exact request/set/snapshot")
        if not authority_revalidation_shape_valid(case):
            raise SystemExit(f"{sid}: authority revalidation receipt is not bound to exact request/grant/snapshot")
        if case["commit"]["assurance_revalidation"]["state"] == "passed":
            fresh = assurance_claims_for_commit(case)
            if fresh is None:
                raise SystemExit(f"{sid}: successful assurance revalidation lacks exact fresh claim results")
        if preflight["policy_digest"] != policy["policy_digest"]:
            raise SystemExit(f"{sid}: preflight policy digest mismatch")
        if preflight["prerequisite_set_digest"] != prereq["set_digest"]:
            raise SystemExit(f"{sid}: preflight prerequisite-set digest mismatch")

        expected_assurance = assurance_result(case)
        expected_authority = authority_result(case)
        expected_admission = admission_result(expected_authority, expected_assurance)
        actual = {
            "assurance_result": expected_assurance,
            "authority_result": expected_authority,
            "admission_decision": expected_admission,
        }
        expected = case["expected"]
        for key, value in actual.items():
            if expected[key] != value:
                raise SystemExit(f"{sid}: {key} expected {expected[key]}, computed {value}")

        if expected_admission == "allow":
            if effect["participant_effect_state"] not in {"committed","outcome_unknown"}:
                raise SystemExit(f"{sid}: allowed admission must have committed or unknown effect")
            if effect["attempt_count"] != 1:
                raise SystemExit(f"{sid}: allowed one-shot APAC fixture must preserve exactly one attempt")
        elif expected_admission in {"deny","hold"}:
            if effect["participant_effect_state"] != "no_effect" or effect["attempt_count"] != 0:
                raise SystemExit(f"{sid}: denied/held admission cannot claim a dispatched effect")

        if effect["participant_effect_state"] == "outcome_unknown":
            if effect["attempt_count"] != 1 or effect["new_attempt_before_reconciliation"]:
                raise SystemExit(f"{sid}: unknown effect must preserve one attempt and prohibit blind retry")
            if expected["task_result"] != "reconciliation_required":
                raise SystemExit(f"{sid}: unknown effect must require reconciliation")

        # An invalidation ordered after commit cannot retroactively change the
        # claim state used at the commit linearization point.
        if commit["linearization_order"] == "commit_before_claim_event":
            for claim in prereq["claims"]:
                if claim["lifecycle_state_at_commit"] != "current":
                    raise SystemExit(f"{sid}: post-commit invalidation cannot make the commit-time claim state non-current")
                if claim["lifecycle_state_after_commit"] == "current":
                    raise SystemExit(f"{sid}: post-commit invalidation control must record the later lifecycle state separately")

        if effect["new_attempt_before_reconciliation"]:
            raise SystemExit(f"{sid}: new attempt before reconciliation is forbidden")

    if not {"positive_control","negative_control","indeterminate_control"}.issubset(class_counts):
        raise SystemExit("APAC requires positive, negative and indeterminate controls")

    print(
        "APAC_SCHEMA=Draft2020-12_CHECKED "
        f"SCENARIOS={len(cases)} "
        f"CONTROLS={json.dumps(class_counts, sort_keys=True)} "
        "AUTHORITY_AND_ASSURANCE=SEPARATE "
        "COMMIT_TIME=BOUND "
        "UNKNOWN_EFFECT=ONE_ATTEMPT "
        "CLAIM=SYNTHETIC_ONLY"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
