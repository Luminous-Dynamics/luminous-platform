#!/usr/bin/env python3
"""Dependency-free semantic checker for the OIC common-cause fault-plan contract."""
from __future__ import annotations

import copy
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "benchmarks" / "agentic-endpoint-security-v1"
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import check_observer_independence_closure as oic  # noqa: E402

PROFILE_ID = "critical_effect_truth_v0"
REQUIRED_DOMAINS = (
    "process_runtime",
    "event_stream",
    "signing_root",
    "credential_root",
    "clock_authority",
    "storage_backend",
    "control_plane",
)
CASE_FIELDS = {
    "scenario_id", "control_class", "oic_scenario_id",
    "fault_plan_committed_before_episode", "fault_plan_visibility",
    "evaluation_input", "posthoc_ground_truth", "expected",
}


def load(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"cannot load {path}: {exc}") from exc


def _known(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip()) and value.strip().casefold() != "unknown"


def _base_index(oic_suite: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {case["scenario_id"]: case for case in oic_suite["cases"]}


def impacted_witnesses(base_case: dict[str, Any], fault_plan: list[dict[str, str]]) -> list[str]:
    return sorted({
        witness["witness_id"]
        for witness in base_case["witnesses"]
        if any(
            witness["domains"].get(fault["domain"]) == fault["dependency_ref"]
            for fault in fault_plan
        )
    })


def evaluate_episode(case: dict[str, Any], oic_suite: dict[str, Any]) -> dict[str, Any]:
    """Evaluate only observer evidence. Deliberately ignores posthoc_ground_truth."""
    base = _base_index(oic_suite)[case["oic_scenario_id"]]
    supplied = case["evaluation_input"]
    by_id = {w["witness_id"]: w for w in base["witnesses"]}
    active = [copy.deepcopy(by_id[witness_id]) for witness_id in supplied["available_witness_ids"]]
    active.extend(copy.deepcopy(record["replacement_witness"]) for record in supplied["recovered_witnesses"])
    replay_case = copy.deepcopy(base)
    replay_case["witnesses"] = active
    profile = oic_suite["claim_profiles"][PROFILE_ID]
    result = oic.evaluate_case(replay_case, profile)
    return {
        "surviving_witness_ids": sorted(w["witness_id"] for w in active),
        "maximum_independent_witnesses": result["maximum_independent_witnesses"],
        "observation_agreement": result["observation_agreement"],
        "final_disposition": result["final_disposition"],
    }


def _validate_recovery(case: dict[str, Any], base: dict[str, Any], impacted: list[str]) -> None:
    original = {w["witness_id"]: w for w in base["witnesses"]}
    seen: set[str] = set()
    faults = case["posthoc_ground_truth"]["fault_plan"]
    for record in case["evaluation_input"]["recovered_witnesses"]:
        witness_id = record["witness_id"]
        if witness_id in seen or witness_id not in impacted:
            raise ValueError(f'{case["scenario_id"]}: duplicate or non-impacted recovery witness')
        seen.add(witness_id)
        prior = original[witness_id]
        if record["from_attestation_ref"] != prior["attestation_ref"]:
            raise ValueError(f'{case["scenario_id"]}: recovery predecessor attestation mismatch')
        if record["to_evidence_epoch"] <= record["from_evidence_epoch"]:
            raise ValueError(f'{case["scenario_id"]}: recovery evidence epoch must strictly increase')
        replacement = record["replacement_witness"]
        if replacement["witness_id"] != witness_id:
            raise ValueError(f'{case["scenario_id"]}: replacement witness identity mismatch')
        if replacement["evidence_ref"] == prior["evidence_ref"] or replacement["attestation_ref"] == prior["attestation_ref"]:
            raise ValueError(f'{case["scenario_id"]}: recovery reused pre-fault evidence or attestation')
        relevant_faults = [
            fault for fault in faults
            if prior["domains"].get(fault["domain"]) == fault["dependency_ref"]
        ]
        if not relevant_faults or any(
            replacement["domains"].get(fault["domain"]) == fault["dependency_ref"]
            for fault in relevant_faults
        ):
            raise ValueError(f'{case["scenario_id"]}: replacement still depends on a faulted identity')
        if any(not _known(replacement["domains"].get(domain)) for domain in REQUIRED_DOMAINS):
            raise ValueError(f'{case["scenario_id"]}: recovered witness has unknown required domain')


def validate_case(case: dict[str, Any], oic_suite: dict[str, Any]) -> dict[str, Any]:
    sid = case.get("scenario_id", "<unknown>")
    if set(case) != CASE_FIELDS:
        raise ValueError(f"{sid}: unexpected case fields")
    if case["fault_plan_committed_before_episode"] is not True or case["fault_plan_visibility"] != "withheld_until_disposition_commit":
        raise ValueError(f"{sid}: fault plan must be committed before execution and hidden until disposition commit")
    base_index = _base_index(oic_suite)
    if case["oic_scenario_id"] not in base_index:
        raise ValueError(f"{sid}: unknown OIC scenario")
    base = base_index[case["oic_scenario_id"]]
    truth = case["posthoc_ground_truth"]
    if set(truth) != {"fault_plan", "fault_actually_injected", "resource_effect_truth"}:
        raise ValueError(f"{sid}: post-hoc truth fields differ from contract")
    if truth["fault_actually_injected"] not in {"confirmed", "not_injected", "unknown"}:
        raise ValueError(f"{sid}: invalid injection outcome")
    if truth["resource_effect_truth"] not in {"effect", "no_effect", "unknown"}:
        raise ValueError(f"{sid}: invalid resource truth")
    if not isinstance(truth["fault_plan"], list):
        raise ValueError(f"{sid}: fault plan must be an array")
    if bool(truth["fault_plan"]) != (truth["fault_actually_injected"] == "confirmed"):
        raise ValueError(f"{sid}: synthetic injection label inconsistent with plan presence")

    seen_faults: set[tuple[str, str]] = set()
    for fault in truth["fault_plan"]:
        if set(fault) != {"domain", "dependency_ref"} or fault["domain"] not in REQUIRED_DOMAINS or not _known(fault["dependency_ref"]):
            raise ValueError(f"{sid}: malformed fault target")
        key = (fault["domain"], fault["dependency_ref"])
        if key in seen_faults:
            raise ValueError(f"{sid}: duplicate fault target")
        seen_faults.add(key)
        if not any(w["domains"].get(fault["domain"]) == fault["dependency_ref"] for w in base["witnesses"]):
            raise ValueError(f"{sid}: fault target absent from referenced OIC inventory")

    impacted = impacted_witnesses(base, truth["fault_plan"])
    supplied = case["evaluation_input"]
    if set(supplied) != {"available_witness_ids", "recovered_witnesses"}:
        raise ValueError(f"{sid}: evaluation input fields differ from contract")
    available = supplied["available_witness_ids"]
    base_ids = {w["witness_id"] for w in base["witnesses"]}
    if len(available) != len(set(available)) or not set(available).issubset(base_ids):
        raise ValueError(f"{sid}: duplicate or unknown available witness")
    recovered_ids = [r["witness_id"] for r in supplied["recovered_witnesses"]]
    if len(recovered_ids) != len(set(recovered_ids)):
        raise ValueError(f"{sid}: duplicate recovered witness identity")
    if set(available) != base_ids - set(impacted) - set(recovered_ids):
        raise ValueError(f"{sid}: available witnesses must equal unaffected inventory minus fresh recoveries")
    _validate_recovery(case, base, impacted)

    evaluated = evaluate_episode(case, oic_suite)
    expected = case["expected"]
    computed = {"impacted_witness_ids": impacted, **evaluated}
    if expected != computed:
        raise ValueError(f"{sid}: expected {expected!r}, computed {computed!r}")
    if expected["final_disposition"] == "qualified_effect" and truth["resource_effect_truth"] != "effect":
        raise ValueError(f"{sid}: qualified effect contradicts independent post-hoc resource truth")
    if expected["final_disposition"] == "qualified_no_effect" and truth["resource_effect_truth"] != "no_effect":
        raise ValueError(f"{sid}: qualified no-effect contradicts independent post-hoc resource truth")
    return evaluated


def validate_suite(suite: dict[str, Any], schema: dict[str, Any], oic_suite: dict[str, Any]) -> dict[str, int]:
    if suite.get("profile") != "luminous-observer-common-cause-fault-injection" or suite.get("version") != "0.1":
        raise ValueError("unexpected CFI profile or version")
    if suite.get("status") != "synthetic_fault_plan_contract_not_executed" or suite.get("no_aggregate_score") is not True:
        raise ValueError("CFI claim ceiling or non-aggregate invariant changed")
    if suite.get("oic_profile_id") != PROFILE_ID or suite.get("required_disjoint_domains") != list(REQUIRED_DOMAINS) or suite.get("minimum_independent_witnesses") != 2:
        raise ValueError("CFI may not replace or weaken the canonical OIC profile")
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema" or schema.get("$id") != "urn:luminous:observer-common-cause-fault-injection:0.1":
        raise ValueError("unexpected CFI schema identity")
    cases = suite.get("cases")
    if not isinstance(cases, list) or len(cases) != 16:
        raise ValueError(f"expected 16 CFI scenarios, found {len(cases) if isinstance(cases, list) else 'non-array'}")
    seen: set[str] = set()
    counts: Counter[str] = Counter()
    for case in cases:
        sid = case.get("scenario_id")
        if sid in seen:
            raise ValueError(f"duplicate CFI scenario id: {sid}")
        seen.add(sid)
        counts[case["control_class"]] += 1
        validate_case(case, oic_suite)
    expected_counts = {"positive_control": 5, "negative_control": 8, "indeterminate_control": 3}
    if dict(counts) != expected_counts:
        raise ValueError(f"unexpected CFI control distribution: {dict(counts)!r}")
    return dict(counts)


def main() -> int:
    suite = load(BASE / "observer-common-cause-fault-injection.fixtures.json")
    schema = load(BASE / "observer-common-cause-fault-injection.schema.json")
    oic_suite = oic.load(oic.BASE / "observer-independence-closure.fixtures.json")
    oic_schema = oic.load(oic.BASE / "observer-independence-closure.schema.json")
    oic.validate_suite(oic_suite, oic_schema)
    counts = validate_suite(suite, schema, oic_suite)
    print(f"CFI_SCHEMA=STRUCTURE_CHECKED CASES=16 CONTROLS={json.dumps(counts, sort_keys=True)} NO_AGGREGATE_SCORE=true CLAIM=SYNTHETIC_ONLY")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
