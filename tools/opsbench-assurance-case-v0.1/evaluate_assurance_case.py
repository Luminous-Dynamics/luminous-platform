#!/usr/bin/env python3
"""Evaluate a structured operational assurance case without a composite score.

The evaluator checks graph integrity, exact evidence/run/scope binding, expiry,
required verifier results, assumptions and defeaters. It does not authenticate
the publishers of those records or re-run their underlying verifiers.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

MAX_INPUT_BYTES = 32 * 1024 * 1024
MAX_NODES = 100_000
REQUIRED_EVIDENCE = {
    "authority": {"authority_decision"},
    "causal_integrity": {"causal_trace_verification"},
    "publisher_authenticity": {"publisher_authenticity_verification"},
    "objective_outcome": {"business_oracle_result"},
    "service_slo": {"service_slo_result"},
    "safety": {"safety_assessment"},
    "study_integrity": {"sealed_corpus_verification"},
    "economics": {"economic_scorecard"},
}


class CaseError(ValueError):
    pass


def _strict_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise CaseError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_constant(value):
    raise CaseError(f"non-finite JSON number is prohibited: {value}")


def load_json(data: bytes, label: str) -> dict:
    if len(data) > MAX_INPUT_BYTES:
        raise CaseError(f"{label}: input exceeds size limit")
    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=_strict_object, parse_constant=_reject_constant)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise CaseError(f"{label}: invalid UTF-8 JSON: {exc}") from exc
    if not isinstance(value, dict):
        raise CaseError(f"{label}: top-level object required")
    return value


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def require(condition: bool, message: str):
    if not condition:
        raise CaseError(message)


def _parse_time(value, where: str) -> datetime:
    require(isinstance(value, str), f"{where}: timestamp string required")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise CaseError(f"{where}: invalid timestamp") from exc
    require(parsed.tzinfo is not None, f"{where}: timezone offset required")
    return parsed.astimezone(timezone.utc)


def _hex(value, length: int, where: str):
    require(isinstance(value, str) and len(value) == length and all(c in "0123456789abcdef" for c in value), f"{where}: lowercase hex digest of length {length} required")


def _unique_map(rows, key, where):
    require(isinstance(rows, list) and len(rows) <= MAX_NODES, f"{where}: array outside supported bounds")
    result = {}
    for index, row in enumerate(rows):
        require(isinstance(row, dict), f"{where}[{index}]: object required")
        identity = row.get(key)
        require(isinstance(identity, str) and identity, f"{where}[{index}].{key}: non-empty string required")
        require(identity not in result, f"{where}: duplicate {key} {identity}")
        result[identity] = row
    return result


def _validate_case(case: dict):
    required = {
        "schema", "version", "status", "case_id", "study_id", "run_id", "episode_id",
        "arm_id", "attempt_id", "evaluated_at", "scope_sha256", "executor",
        "top_level_claim_id", "required_claim_types", "claims", "arguments",
        "evidence", "assumptions", "defeaters",
    }
    require(set(case) == required, "case: unexpected or missing top-level fields")
    require(case.get("schema") == "luminous-opsbench-operational-assurance-case-v0.1" and type(case.get("version")) is int and case["version"] == 1, "case: unsupported schema/version")
    require(case.get("status") in {"example_only", "registered_case"}, "case: invalid status")
    for field in ("case_id", "study_id", "run_id", "episode_id", "arm_id"):
        require(isinstance(case.get(field), str) and case[field], f"case.{field}: non-empty string required")
    require(case["attempt_id"] is None or (isinstance(case["attempt_id"], str) and case["attempt_id"]), "case.attempt_id: string or null required")
    evaluated_at = _parse_time(case["evaluated_at"], "case.evaluated_at")
    _hex(case.get("scope_sha256"), 64, "case.scope_sha256")

    executor = case.get("executor")
    require(isinstance(executor, dict) and set(executor) == {"source_id", "trust_domain_id"}, "case.executor: exact identity fields required")
    require(all(isinstance(executor.get(k), str) and executor[k] for k in ("source_id", "trust_domain_id")), "case.executor: source and trust domain required")

    claim_types = {
        "top_level", "authority", "causal_integrity", "publisher_authenticity",
        "objective_outcome", "service_slo", "safety", "study_integrity", "economics",
    }
    required_types = case.get("required_claim_types")
    require(isinstance(required_types, list) and required_types and len(required_types) == len(set(required_types)), "case.required_claim_types: unique non-empty list required")
    require(all(isinstance(item, str) and item in REQUIRED_EVIDENCE for item in required_types), "case.required_claim_types: unsupported claim type")
    claims = _unique_map(case.get("claims"), "claim_id", "claims")
    arguments = _unique_map(case.get("arguments"), "argument_id", "arguments")
    evidence = _unique_map(case.get("evidence"), "evidence_id", "evidence")
    assumptions = _unique_map(case.get("assumptions"), "assumption_id", "assumptions")
    defeaters = _unique_map(case.get("defeaters"), "defeater_id", "defeaters")
    require(len(claims) >= 1 and len(arguments) >= 1, "case: at least one claim and argument required")

    root_id = case.get("top_level_claim_id")
    require(root_id in claims and claims[root_id].get("claim_type") == "top_level", "case.top_level_claim_id must identify a top_level claim")
    claim_ids_by_type = {}
    for claim_id, claim in claims.items():
        require(set(claim) == {"claim_id", "claim_type", "statement", "required", "argument_id"}, f"claims.{claim_id}: unexpected fields")
        claim_type = claim["claim_type"]
        require(claim_type in claim_types, f"claims.{claim_id}: unsupported claim type")
        require(isinstance(claim["statement"], str) and len(claim["statement"]) >= 10, f"claims.{claim_id}: statement too short")
        require(type(claim["required"]) is bool, f"claims.{claim_id}: required must be boolean")
        require(claim["argument_id"] in arguments, f"claims.{claim_id}: argument reference missing")
        claim_ids_by_type.setdefault(claim_type, []).append(claim_id)
        if claim_type in required_types:
            require(claim["required"] is True, f"claims.{claim_id}: required claim type incorrectly marked optional")
        elif claim_type != "top_level":
            require(claim["required"] is False, f"claims.{claim_id}: non-required claim type must be optional")
    for claim_type in required_types:
        require(len(claim_ids_by_type.get(claim_type, [])) == 1, f"case: exactly one claim of required type {claim_type} is required")
    require(len(claim_ids_by_type.get("top_level", [])) == 1 and claims[root_id]["required"], "case: exactly one required top-level claim is required")
    require(all(len(ids) <= 1 for kind, ids in claim_ids_by_type.items() if kind != "top_level"), "case: duplicate claim type is prohibited in v0.1")

    referenced_evidence, referenced_assumptions, referenced_defeaters = set(), set(), set()
    for argument_id, argument in arguments.items():
        exact = {"argument_id", "target_claim_id", "rule", "premise_claim_ids", "evidence_ids", "assumption_ids", "defeater_ids"}
        require(set(argument) == exact, f"arguments.{argument_id}: unexpected fields")
        target = argument["target_claim_id"]
        require(target in claims and claims[target]["argument_id"] == argument_id, f"arguments.{argument_id}: target/claim linkage mismatch")
        require(argument["rule"] == "all_required_premises_and_evidence_supported", f"arguments.{argument_id}: unsupported inference rule")
        for field, universe, referenced in (
            ("premise_claim_ids", claims, None),
            ("evidence_ids", evidence, referenced_evidence),
            ("assumption_ids", assumptions, referenced_assumptions),
            ("defeater_ids", defeaters, referenced_defeaters),
        ):
            values = argument[field]
            require(isinstance(values, list) and len(values) == len(set(values)), f"arguments.{argument_id}.{field}: unique reference list required")
            for value in values:
                require(value in universe, f"arguments.{argument_id}.{field}: unknown reference {value}")
                if referenced is not None:
                    referenced.add(value)
        if target == root_id:
            premises = set(argument["premise_claim_ids"])
            required_claim_ids = {cid for cid, claim in claims.items() if claim["required"] and cid != root_id}
            require(required_claim_ids.issubset(premises), "top-level argument must include every required claim as a premise")

    require(set(claims[cid]["argument_id"] for cid in claims) == set(arguments), "orphan argument or missing claim argument")
    for evidence_id, item in evidence.items():
        expected = {
            "evidence_id", "evidence_type", "verification_status", "source_id", "source_trust_domain_id",
            "verifier_id", "verifier_trust_domain_id", "artifact_sha256", "verifier_sha256",
            "source_commit_sha", "run_id", "episode_id", "arm_id", "scope_sha256",
            "observed_at", "valid_until", "attempt_id",
        }
        require(set(item) == expected, f"evidence.{evidence_id}: unexpected fields")
        require(item["evidence_type"] in set().union(*REQUIRED_EVIDENCE.values()), f"evidence.{evidence_id}: unsupported evidence type")
        require(item["verification_status"] in {"supported", "refuted", "indeterminate"}, f"evidence.{evidence_id}: invalid verification status")
        for field in ("source_id", "source_trust_domain_id", "verifier_id", "verifier_trust_domain_id"):
            require(isinstance(item[field], str) and item[field], f"evidence.{evidence_id}.{field}: required")
        require(item["verifier_trust_domain_id"] != executor["trust_domain_id"], f"evidence.{evidence_id}: verifier must be outside executor trust domain")
        require(item["verifier_trust_domain_id"] != item["source_trust_domain_id"], f"evidence.{evidence_id}: verifier and source trust domains must differ")
        _hex(item["artifact_sha256"], 64, f"evidence.{evidence_id}.artifact_sha256")
        _hex(item["verifier_sha256"], 64, f"evidence.{evidence_id}.verifier_sha256")
        _hex(item["source_commit_sha"], 40, f"evidence.{evidence_id}.source_commit_sha")
        for field in ("run_id", "episode_id", "arm_id"):
            require(item[field] == case[field], f"evidence.{evidence_id}.{field}: cross-case substitution")
        require(item["scope_sha256"] == case["scope_sha256"], f"evidence.{evidence_id}: scope digest mismatch")
        observed = _parse_time(item["observed_at"], f"evidence.{evidence_id}.observed_at")
        require(observed <= evaluated_at, f"evidence.{evidence_id}: observed after case evaluation time")
        if item["valid_until"] is not None:
            expiry = _parse_time(item["valid_until"], f"evidence.{evidence_id}.valid_until")
            require(expiry >= observed, f"evidence.{evidence_id}: validity ends before observation")
        require(item["attempt_id"] is None or (isinstance(item["attempt_id"], str) and item["attempt_id"]), f"evidence.{evidence_id}.attempt_id: string or null required")
        if item["attempt_id"] is not None:
            require(item["attempt_id"] == case["attempt_id"], f"evidence.{evidence_id}: cross-attempt substitution")

    for assumption_id, assumption in assumptions.items():
        exact = {"assumption_id", "statement", "owner", "scope_sha256", "status", "reviewed_at", "expires_at", "rationale"}
        require(set(assumption) == exact, f"assumptions.{assumption_id}: unexpected fields")
        require(isinstance(assumption["statement"], str) and len(assumption["statement"]) >= 10, f"assumptions.{assumption_id}: statement too short")
        require(isinstance(assumption["owner"], str) and assumption["owner"], f"assumptions.{assumption_id}: owner required")
        require(assumption["scope_sha256"] == case["scope_sha256"], f"assumptions.{assumption_id}: scope mismatch")
        require(assumption["status"] in {"accepted", "rejected", "unknown"}, f"assumptions.{assumption_id}: invalid status")
        reviewed = _parse_time(assumption["reviewed_at"], f"assumptions.{assumption_id}.reviewed_at")
        require(reviewed <= evaluated_at, f"assumptions.{assumption_id}: reviewed after evaluation time")
        if assumption["expires_at"] is not None:
            _parse_time(assumption["expires_at"], f"assumptions.{assumption_id}.expires_at")

    for defeater_id, defeater in defeaters.items():
        exact = {"defeater_id", "target_claim_id", "description", "critical", "status", "evidence_ids"}
        require(set(defeater) == exact, f"defeaters.{defeater_id}: unexpected fields")
        require(defeater["target_claim_id"] in claims, f"defeaters.{defeater_id}: unknown target claim")
        require(isinstance(defeater["description"], str) and len(defeater["description"]) >= 10, f"defeaters.{defeater_id}: description too short")
        require(type(defeater["critical"]) is bool, f"defeaters.{defeater_id}: critical must be boolean")
        require(defeater["status"] in {"open", "mitigated", "disproved"}, f"defeaters.{defeater_id}: invalid status")
        ids = defeater["evidence_ids"]
        require(isinstance(ids, list) and len(ids) == len(set(ids)), f"defeaters.{defeater_id}: unique evidence references required")
        for evidence_id in ids:
            require(evidence_id in evidence, f"defeaters.{defeater_id}: unknown evidence {evidence_id}")
            referenced_evidence.add(evidence_id)
        if defeater["status"] in {"mitigated", "disproved"}:
            require(ids, f"defeaters.{defeater_id}: resolution requires evidence")
        require(defeater_id in referenced_defeaters, f"defeaters.{defeater_id}: defeater is not attached to an argument")

    require(referenced_evidence == set(evidence), "case: orphan evidence record not referenced by any argument or defeater")
    require(referenced_assumptions == set(assumptions), "case: orphan assumption not referenced by any argument")
    require(referenced_defeaters == set(defeaters), "case: orphan defeater not referenced by any argument")
    return {
        "evaluated_at": evaluated_at,
        "claims": claims,
        "arguments": arguments,
        "evidence": evidence,
        "assumptions": assumptions,
        "defeaters": defeaters,
        "required_types": set(required_types),
        "root_id": root_id,
        "executor": executor,
    }


def evaluate(case: dict, case_bytes: bytes) -> dict:
    validated = _validate_case(case)
    claim_status: dict[str, str] = {}
    claim_reasons: dict[str, list[str]] = {}
    visiting = set()

    def evaluate_claim(claim_id: str) -> str:
        if claim_id in claim_status:
            return claim_status[claim_id]
        require(claim_id not in visiting, f"claim graph cycle detected at {claim_id}")
        visiting.add(claim_id)
        claim = validated["claims"][claim_id]
        argument = validated["arguments"][claim["argument_id"]]
        reasons = []
        evidence_rows = [validated["evidence"][x] for x in argument["evidence_ids"]]
        expected = REQUIRED_EVIDENCE.get(claim["claim_type"], set())
        actual_types = {row["evidence_type"] for row in evidence_rows}
        missing_types = expected - actual_types
        if missing_types:
            reasons.append("missing required evidence types: " + ",".join(sorted(missing_types)))

        statuses = []
        for row in evidence_rows:
            status = row["verification_status"]
            if row["valid_until"] is not None and _parse_time(row["valid_until"], f"evidence.{row['evidence_id']}.valid_until") < validated["evaluated_at"]:
                status = "indeterminate"
            statuses.append(status)
        if any(status == "refuted" for status in statuses):
            result = "refuted"
            reasons.append("one or more evidence verifiers refuted the claim")
        elif missing_types or any(status == "indeterminate" for status in statuses):
            result = "indeterminate"
            if any(status == "indeterminate" for status in statuses):
                reasons.append("one or more evidence verifiers returned indeterminate")
        else:
            result = "supported" if evidence_rows or claim["claim_type"] == "top_level" else "indeterminate"
            if result == "indeterminate":
                reasons.append("no supporting evidence or premises")

        for premise_id in argument["premise_claim_ids"]:
            premise_status = evaluate_claim(premise_id)
            if premise_status == "refuted":
                result = "refuted"
                reasons.append(f"premise {premise_id} is refuted")
            elif premise_status == "indeterminate" and result != "refuted":
                result = "indeterminate"
                reasons.append(f"premise {premise_id} is indeterminate")

        for assumption_id in argument["assumption_ids"]:
            assumption = validated["assumptions"][assumption_id]
            if assumption["status"] == "rejected":
                result = "refuted"
                reasons.append(f"assumption {assumption_id} is rejected")
            elif assumption["status"] == "unknown" and result != "refuted":
                result = "indeterminate"
                reasons.append(f"assumption {assumption_id} is unknown")
            expires_at = assumption["expires_at"]
            if expires_at is not None and _parse_time(expires_at, f"assumptions.{assumption_id}.expires_at") < validated["evaluated_at"]:
                result = "indeterminate" if result != "refuted" else result
                reasons.append(f"assumption {assumption_id} has expired")

        for defeater_id in argument["defeater_ids"]:
            defeater = validated["defeaters"][defeater_id]
            if defeater["status"] == "open":
                result = "refuted" if defeater["critical"] else ("indeterminate" if result != "refuted" else result)
                reasons.append(f"defeater {defeater_id} remains open")
            elif defeater["status"] in {"mitigated", "disproved"}:
                mitigation = []
                for evidence_id in defeater["evidence_ids"]:
                    item = validated["evidence"][evidence_id]
                    status = item["verification_status"]
                    if item["valid_until"] is not None and _parse_time(item["valid_until"], f"evidence.{evidence_id}.valid_until") < validated["evaluated_at"]:
                        status = "indeterminate"
                    mitigation.append(status)
                if not mitigation or any(s != "supported" for s in mitigation):
                    result = "refuted" if defeater["critical"] else ("indeterminate" if result != "refuted" else result)
                    reasons.append(f"defeater {defeater_id} resolution is not supported")

        visiting.remove(claim_id)
        claim_status[claim_id] = result
        claim_reasons[claim_id] = sorted(set(reasons))
        return result

    for claim_id in validated["claims"]:
        evaluate_claim(claim_id)

    required_claim_statuses = {
        claim_id: claim_status[claim_id]
        for claim_id, claim in validated["claims"].items()
        if claim["required"] and claim_id != validated["root_id"]
    }
    root_status = claim_status[validated["root_id"]]
    open_critical = [
        defeater_id for defeater_id, defeater in validated["defeaters"].items()
        if defeater["critical"] and defeater["status"] == "open"
    ]
    if open_critical or any(value == "refuted" for value in required_claim_statuses.values()) or root_status == "refuted":
        disposition = "blocked"
    elif any(value != "supported" for value in required_claim_statuses.values()) or root_status != "supported":
        disposition = "indeterminate"
    else:
        disposition = "eligible_for_human_review"

    economics_claim = next((claim_id for claim_id, claim in validated["claims"].items() if claim["claim_type"] == "economics"), None)
    economic_allowed = economics_claim is not None and claim_status[economics_claim] == "supported"
    if case["status"] == "example_only":
        output_status = "example_only"
    else:
        output_status = disposition
    report = {
        "schema": "luminous-opsbench-operational-assurance-report-v0.1",
        "version": 1,
        "status": output_status,
        "computed_disposition": disposition,
        "case_id": case["case_id"],
        "run_id": case["run_id"],
        "episode_id": case["episode_id"],
        "arm_id": case["arm_id"],
        "attempt_id": case["attempt_id"],
        "case_sha256": sha256(case_bytes),
        "top_level_claim_id": validated["root_id"],
        "top_level_claim_status": root_status,
        "eligible_for_human_review": disposition == "eligible_for_human_review" and case["status"] == "registered_case",
        "automatic_execution_authorized": False,
        "economic_claims_allowed": economic_allowed,
        "required_claim_statuses": required_claim_statuses,
        "claim_results": {
            claim_id: {"status": claim_status[claim_id], "reasons": claim_reasons[claim_id]}
            for claim_id in validated["claims"]
        },
        "open_critical_defeaters": open_critical,
        "publisher_authenticity_cryptographically_verified_by_this_aggregator": False,
        "claim_ceiling": "Structured assurance-case consistency and evidence-result aggregation only. Does not authenticate artifact publishers, independently validate source verifiers, grant mutation authority, or certify production safety.",
    }
    report["report_sha256"] = sha256(json.dumps(report, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode("utf-8"))
    return report


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--case", type=Path, required=True)
    parser.add_argument("--output", default="-")
    args = parser.parse_args(argv)
    try:
        case_bytes = args.case.read_bytes()
        case = load_json(case_bytes, "assurance case")
        report = evaluate(case, case_bytes)
        rendered = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
        if args.output == "-":
            sys.stdout.write(rendered)
        else:
            args.output.write_text(rendered, encoding="utf-8")
        return 0 if report["status"] in {"example_only", "eligible_for_human_review"} else (1 if report["status"] == "blocked" else 2)
    except (OSError, CaseError, AttributeError, KeyError, TypeError, ValueError, OverflowError, RecursionError) as exc:
        print(f"assurance-case-evaluator: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
