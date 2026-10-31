#!/usr/bin/env python3
"""Fail-closed evaluator for sustained, customer-visible SLO recovery windows.

This checks a frozen contract against independent observation records. It does
not authenticate metric publishers, prove observer independence, or run a load
test. Missing/insufficient measurements are indeterminate, never healthy.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

MAX_INPUT_BYTES = 32 * 1024 * 1024
MAX_WINDOWS = 100_000
MAX_SLI = 32
MAX_BUCKETS = 1000
MAX_WINDOW_SECONDS = 86_400
SLI_KINDS = {
    "request_success_ratio",
    "synthetic_probe_success_ratio",
    "business_correctness_ratio",
    "latency_quantile",
    "goodput_minimum",
    "saturation_maximum",
}


class ContractError(ValueError):
    pass


def _strict_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ContractError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_constant(value):
    raise ContractError(f"non-finite JSON number is prohibited: {value}")


def load_json(data: bytes, label: str) -> dict:
    if len(data) > MAX_INPUT_BYTES:
        raise ContractError(f"{label}: input exceeds size limit")
    try:
        result = json.loads(data.decode("utf-8"), object_pairs_hook=_strict_object, parse_constant=_reject_constant)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ContractError(f"{label}: invalid UTF-8 JSON: {exc}") from exc
    if not isinstance(result, dict):
        raise ContractError(f"{label}: top-level object required")
    return result


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def require(condition: bool, message: str):
    if not condition:
        raise ContractError(message)


def parse_time(value, where: str) -> datetime:
    require(isinstance(value, str), f"{where}: timestamp string required")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ContractError(f"{where}: invalid timestamp") from exc
    require(parsed.tzinfo is not None, f"{where}: timezone offset required")
    return parsed.astimezone(timezone.utc)


def finite_number(value, where: str, *, minimum: float | None = None) -> float:
    require(type(value) in (int, float) and math.isfinite(value), f"{where}: finite number required")
    result = float(value)
    if minimum is not None:
        require(result >= minimum, f"{where}: value below minimum")
    return result


def _ratio(numerator, denominator, where: str) -> float:
    require(type(numerator) is int and type(denominator) is int, f"{where}: integer counts required")
    require(denominator > 0 and 0 <= numerator <= denominator, f"{where}: invalid numerator/denominator")
    return numerator / denominator


def _histogram_quantile(histogram, quantile: float, expected_count: int, where: str):
    if histogram is None:
        return None, "latency histogram missing"
    require(isinstance(histogram, dict) and set(histogram) == {"unit", "buckets"}, f"{where}: malformed histogram")
    require(histogram["unit"] == "s", f"{where}: latency histogram unit must be seconds")
    buckets = histogram["buckets"]
    require(isinstance(buckets, list) and 2 <= len(buckets) <= MAX_BUCKETS, f"{where}: bucket count outside supported bounds")
    total, previous_bound, cumulative = 0, 0.0, 0
    selected = None
    target_rank = max(1, math.ceil(quantile * expected_count))
    for index, bucket in enumerate(buckets):
        require(isinstance(bucket, dict) and set(bucket) == {"upper_bound_seconds", "count"}, f"{where}.buckets[{index}]: malformed bucket")
        count = bucket["count"]
        require(type(count) is int and count >= 0, f"{where}.buckets[{index}]: non-negative integer count required")
        bound = bucket["upper_bound_seconds"]
        if bound is None:
            require(index == len(buckets) - 1, f"{where}: +Inf bucket must be last")
        else:
            bound = finite_number(bound, f"{where}.buckets[{index}].upper_bound_seconds", minimum=0.0)
            require(bound > previous_bound, f"{where}: histogram bounds must be strictly increasing")
            previous_bound = bound
        total += count
        cumulative += count
        if selected is None and cumulative >= target_rank:
            selected = bound
    require(buckets[-1]["upper_bound_seconds"] is None, f"{where}: final +Inf bucket is required")
    require(total == expected_count, f"{where}: histogram count must equal request_count")
    return selected, None if selected is not None else "latency quantile unavailable"


def _validate_contract(contract: dict):
    expected_fields = {"schema", "version", "status", "study_id", "episode_id", "run_id", "service_id", "service_digest", "executor", "observer", "business_oracle", "baseline_reference", "timeline", "required_slis"}
    require(set(contract) == expected_fields, "contract: unexpected or missing top-level fields")
    require(contract.get("schema") == "luminous-opsbench-service-objective-contract-v0.1" and type(contract.get("version")) is int and contract["version"] == 1, "contract: unsupported schema/version")
    require(contract.get("status") in {"example_only", "preregistered"}, "contract: invalid status")
    for field in ("study_id", "episode_id", "run_id", "service_id"):
        require(isinstance(contract.get(field), str) and contract[field], f"contract.{field}: non-empty string required")
    require(isinstance(contract.get("service_digest"), str) and len(contract["service_digest"]) == 64 and all(c in "0123456789abcdef" for c in contract["service_digest"]), "contract.service_digest: lowercase SHA-256 required")

    parties = {}
    for name in ("executor", "observer", "business_oracle"):
        item = contract.get(name)
        require(isinstance(item, dict) and set(item) == {"source_id", "trust_domain_id"}, f"contract.{name}: exact identity fields required")
        require(all(isinstance(item[k], str) and item[k] for k in ("source_id", "trust_domain_id")), f"contract.{name}: non-empty source and trust domain required")
        parties[name] = item
    trust_domains = [parties[x]["trust_domain_id"] for x in parties]
    source_ids = [parties[x]["source_id"] for x in parties]
    require(len(set(trust_domains)) == 3, "executor, observer and business oracle must use three distinct trust domains")
    require(len(set(source_ids)) == 3, "executor, observer and business oracle must use three distinct source identities")

    baseline = contract.get("baseline_reference")
    require(isinstance(baseline, dict) and set(baseline) == {"window_started_at", "window_ended_at", "measurement_sha256", "workload_profile_sha256"}, "contract.baseline_reference: exact fields required")
    baseline_start = parse_time(baseline["window_started_at"], "baseline.window_started_at")
    baseline_end = parse_time(baseline["window_ended_at"], "baseline.window_ended_at")
    require(baseline_start < baseline_end, "baseline window must have positive duration")
    for digest in ("measurement_sha256", "workload_profile_sha256"):
        value = baseline[digest]
        require(isinstance(value, str) and len(value) == 64 and all(c in "0123456789abcdef" for c in value), f"baseline.{digest}: lowercase SHA-256 required")

    timeline = contract.get("timeline")
    required_timeline = {
        "incident_started_at", "evaluation_started_at", "evaluation_ended_at", "window_seconds",
        "hold_window_count", "minimum_request_count", "minimum_synthetic_probe_count",
        "minimum_business_check_count", "maximum_gap_seconds",
    }
    require(isinstance(timeline, dict) and set(timeline) == required_timeline, "contract.timeline: exact fields required")
    incident = parse_time(timeline["incident_started_at"], "timeline.incident_started_at")
    eval_start = parse_time(timeline["evaluation_started_at"], "timeline.evaluation_started_at")
    eval_end = parse_time(timeline["evaluation_ended_at"], "timeline.evaluation_ended_at")
    require(baseline_end <= incident <= eval_start < eval_end, "baseline, incident and evaluation timestamps are not ordered")
    window_seconds = timeline["window_seconds"]
    hold_count = timeline["hold_window_count"]
    require(type(window_seconds) is int and 1 <= window_seconds <= MAX_WINDOW_SECONDS, "timeline.window_seconds outside supported bounds")
    require(type(hold_count) is int and 2 <= hold_count <= 1000, "timeline.hold_window_count outside supported bounds")
    for field in ("minimum_request_count", "minimum_synthetic_probe_count", "minimum_business_check_count"):
        require(type(timeline[field]) is int and timeline[field] >= 1, f"timeline.{field} must be a positive integer")
    require(type(timeline["maximum_gap_seconds"]) is int and timeline["maximum_gap_seconds"] == 0, "v0.1 requires zero permitted observation gap")
    duration = (eval_end - eval_start).total_seconds()
    require(duration % window_seconds == 0, "evaluation duration must be an integer number of windows")
    expected_window_count = int(duration // window_seconds)
    require(1 <= expected_window_count <= MAX_WINDOWS, "expected window count outside supported bounds")

    slis = contract.get("required_slis")
    require(isinstance(slis, list) and 1 <= len(slis) <= MAX_SLI, "contract.required_slis outside supported bounds")
    seen = set()
    for index, sli in enumerate(slis):
        where = f"required_slis[{index}]"
        require(isinstance(sli, dict), f"{where}: object required")
        sli_id, kind = sli.get("sli_id"), sli.get("kind")
        require(isinstance(sli_id, str) and sli_id and sli_id not in seen, f"{where}: duplicate/empty SLI ID")
        seen.add(sli_id)
        require(kind in SLI_KINDS, f"{where}: unsupported SLI kind")
        if kind in {"request_success_ratio", "synthetic_probe_success_ratio", "business_correctness_ratio"}:
            require(set(sli) == {"sli_id", "kind", "minimum_ratio"}, f"{where}: exact ratio SLI fields required")
            ratio = finite_number(sli["minimum_ratio"], f"{where}.minimum_ratio", minimum=0.0)
            require(ratio <= 1.0, f"{where}.minimum_ratio cannot exceed 1")
        elif kind == "latency_quantile":
            require(set(sli) == {"sli_id", "kind", "quantile", "maximum_seconds"}, f"{where}: exact latency SLI fields required")
            q = finite_number(sli["quantile"], f"{where}.quantile", minimum=0.0)
            maximum = finite_number(sli["maximum_seconds"], f"{where}.maximum_seconds", minimum=0.0)
            require(0.0 < q <= 1.0 and maximum > 0.0, f"{where}: invalid quantile/latency target")
        elif kind == "goodput_minimum":
            require(set(sli) == {"sli_id", "kind", "minimum_units_per_second"}, f"{where}: exact goodput SLI fields required")
            require(finite_number(sli["minimum_units_per_second"], f"{where}.minimum_units_per_second", minimum=0.0) > 0, f"{where}: goodput target must be positive")
        elif kind == "saturation_maximum":
            require(set(sli) == {"sli_id", "kind", "maximum_ratio"}, f"{where}: exact saturation SLI fields required")
            maximum = finite_number(sli["maximum_ratio"], f"{where}.maximum_ratio", minimum=0.0)
            require(maximum <= 1.0, f"{where}: saturation maximum cannot exceed 1.0")
    return {
        "parties": parties,
        "timeline": timeline,
        "incident": incident,
        "eval_start": eval_start,
        "eval_end": eval_end,
        "window_seconds": window_seconds,
        "hold_count": hold_count,
        "expected_window_count": expected_window_count,
        "slis": slis,
    }


def _evaluate_window(window: dict, contract: dict, validated: dict, index: int) -> dict:
    where = f"windows[{index}]"
    require(isinstance(window, dict), f"{where}: object required")
    required = {
        "window_id", "started_at", "ended_at", "observer_source_id", "observer_trust_domain_id",
        "business_oracle_source_id", "business_oracle_trust_domain_id", "request_count",
        "successful_request_count", "synthetic_probe_count", "successful_synthetic_probe_count",
        "business_check_count", "correct_business_check_count", "latency_histogram",
        "goodput_units", "saturation_ratio",
    }
    require(set(window) == required, f"{where}: unexpected or missing fields")
    require(isinstance(window["window_id"], str) and window["window_id"], f"{where}.window_id required")
    started = parse_time(window["started_at"], f"{where}.started_at")
    ended = parse_time(window["ended_at"], f"{where}.ended_at")
    require(ended > started and (ended - started).total_seconds() == validated["window_seconds"], f"{where}: exact window duration required")

    observer = validated["parties"]["observer"]
    oracle = validated["parties"]["business_oracle"]
    require(window["observer_source_id"] == observer["source_id"] and window["observer_trust_domain_id"] == observer["trust_domain_id"], f"{where}: observer identity does not match frozen contract")
    require(window["business_oracle_source_id"] == oracle["source_id"] and window["business_oracle_trust_domain_id"] == oracle["trust_domain_id"], f"{where}: business oracle identity does not match frozen contract")

    counts = {}
    for field in ("request_count", "successful_request_count", "synthetic_probe_count", "successful_synthetic_probe_count", "business_check_count", "correct_business_check_count"):
        require(type(window[field]) is int and window[field] >= 0, f"{where}.{field}: non-negative integer required")
        counts[field] = window[field]
    require(counts["successful_request_count"] <= counts["request_count"], f"{where}: successful requests exceed request count")
    require(counts["successful_synthetic_probe_count"] <= counts["synthetic_probe_count"], f"{where}: successful probes exceed probe count")
    require(counts["correct_business_check_count"] <= counts["business_check_count"], f"{where}: correct checks exceed business check count")

    min_req = validated["timeline"]["minimum_request_count"]
    min_probe = validated["timeline"]["minimum_synthetic_probe_count"]
    min_business = validated["timeline"]["minimum_business_check_count"]
    reasons, results = [], []
    for sli in validated["slis"]:
        kind, sli_id = sli["kind"], sli["sli_id"]
        value, disposition, reason = None, "indeterminate", None
        if kind == "request_success_ratio":
            if counts["request_count"] < min_req:
                reason = "request sample count below minimum"
            else:
                value = _ratio(counts["successful_request_count"], counts["request_count"], where)
                disposition = "pass" if value >= sli["minimum_ratio"] else "fail"
        elif kind == "synthetic_probe_success_ratio":
            if counts["synthetic_probe_count"] < min_probe:
                reason = "synthetic probe count below minimum"
            else:
                value = _ratio(counts["successful_synthetic_probe_count"], counts["synthetic_probe_count"], where)
                disposition = "pass" if value >= sli["minimum_ratio"] else "fail"
        elif kind == "business_correctness_ratio":
            if counts["business_check_count"] < min_business:
                reason = "business correctness sample count below minimum"
            else:
                value = _ratio(counts["correct_business_check_count"], counts["business_check_count"], where)
                disposition = "pass" if value >= sli["minimum_ratio"] else "fail"
        elif kind == "latency_quantile":
            if counts["request_count"] < min_req:
                reason = "request sample count below minimum for latency quantile"
            else:
                value, reason = _histogram_quantile(window["latency_histogram"], sli["quantile"], counts["request_count"], where + ".latency_histogram")
                if value is not None:
                    disposition = "pass" if value <= sli["maximum_seconds"] else "fail"
        elif kind == "goodput_minimum":
            units = window["goodput_units"]
            if units is None:
                reason = "goodput measurement missing"
            else:
                units = finite_number(units, where + ".goodput_units", minimum=0.0)
                value = units / validated["window_seconds"]
                disposition = "pass" if value >= sli["minimum_units_per_second"] else "fail"
        elif kind == "saturation_maximum":
            saturation = window["saturation_ratio"]
            if saturation is None:
                reason = "saturation measurement missing"
            else:
                value = finite_number(saturation, where + ".saturation_ratio", minimum=0.0)
                require(value <= 1.0, f"{where}.saturation_ratio cannot exceed 1")
                disposition = "pass" if value <= sli["maximum_ratio"] else "fail"
        if reason:
            reasons.append(f"{sli_id}: {reason}")
        results.append({"sli_id": sli_id, "kind": kind, "value": value, "disposition": disposition, "reason": reason})

    statuses = {item["disposition"] for item in results}
    status = "fail" if "fail" in statuses else ("indeterminate" if "indeterminate" in statuses else "pass")
    return {
        "window_id": window["window_id"],
        "started_at": window["started_at"],
        "ended_at": window["ended_at"],
        "status": status,
        "request_count": counts["request_count"],
        "synthetic_probe_count": counts["synthetic_probe_count"],
        "business_check_count": counts["business_check_count"],
        "sli_results": results,
        "reasons": reasons,
    }


def evaluate(contract: dict, contract_bytes: bytes, observations: dict, observation_bytes: bytes) -> dict:
    validated = _validate_contract(contract)
    expected_observation_fields = {"schema", "version", "status", "contract_sha256", "study_id", "episode_id", "run_id", "windows"}
    require(set(observations) == expected_observation_fields, "observations: unexpected or missing top-level fields")
    require(observations.get("schema") == "luminous-opsbench-service-observation-windows-v0.1" and type(observations.get("version")) is int and observations["version"] == 1, "observations: unsupported schema/version")
    require(observations.get("status") in {"example_only", "captured"}, "observations: invalid status")
    expected_observation_status = "example_only" if contract["status"] == "example_only" else "captured"
    require(observations["status"] == expected_observation_status, "observations: status does not match frozen contract lifecycle")
    for field in ("study_id", "episode_id", "run_id"):
        require(observations.get(field) == contract.get(field), f"observations.{field}: does not match contract")
    contract_digest = sha256(contract_bytes)
    require(observations.get("contract_sha256") == contract_digest, "observations: exact service contract digest mismatch")
    windows = observations.get("windows")
    require(isinstance(windows, list) and len(windows) <= MAX_WINDOWS, "observations.windows outside supported bounds")

    indexed = {}
    previous_start = None
    for i, window in enumerate(windows):
        result = _evaluate_window(window, contract, validated, i)
        start = parse_time(window["started_at"], f"windows[{i}].started_at")
        end = parse_time(window["ended_at"], f"windows[{i}].ended_at")
        require(validated["eval_start"] <= start and end <= validated["eval_end"], f"windows[{i}]: outside evaluation interval")
        offset = (start - validated["eval_start"]).total_seconds()
        require(offset % validated["window_seconds"] == 0, f"windows[{i}]: start is not aligned to window grid")
        slot = int(offset // validated["window_seconds"])
        require(slot not in indexed, f"windows[{i}]: duplicate time slot")
        if previous_start is not None:
            require(start > previous_start, "observation windows must be ordered by start time")
        previous_start = start
        indexed[slot] = result

    timeline_results = []
    for slot in range(validated["expected_window_count"]):
        expected_start = validated["eval_start"] + timedelta(seconds=slot * validated["window_seconds"])
        expected_end = expected_start + timedelta(seconds=validated["window_seconds"])
        if slot in indexed:
            row = indexed[slot]
            require(parse_time(row["started_at"], f"{row['window_id']}.started_at") == expected_start, "window start mismatch")
            require(parse_time(row["ended_at"], f"{row['window_id']}.ended_at") == expected_end, "window end mismatch")
            timeline_results.append(row)
        else:
            timeline_results.append({
                "window_id": f"missing-slot-{slot}",
                "started_at": expected_start.isoformat().replace("+00:00", "Z"),
                "ended_at": expected_end.isoformat().replace("+00:00", "Z"),
                "status": "indeterminate",
                "request_count": 0,
                "synthetic_probe_count": 0,
                "business_check_count": 0,
                "sli_results": [],
                "reasons": ["observation window missing"],
            })

    consecutive = 0
    candidate_start = None
    ever_passed = False
    regression_count = 0
    indeterminate_count = 0
    for row in timeline_results:
        if row["status"] == "pass":
            if consecutive == 0:
                candidate_start = row["started_at"]
            consecutive += 1
            ever_passed = True
        elif row["status"] == "fail":
            if ever_passed:
                regression_count += 1
            consecutive = 0
            candidate_start = None
        else:
            indeterminate_count += 1
            consecutive = 0
            candidate_start = None

    hold_count = validated["hold_count"]
    sustained = consecutive >= hold_count
    final_row = timeline_results[-1] if timeline_results else None
    start_candidate = timeline_results[-consecutive]["started_at"] if consecutive else None
    sustained_from = start_candidate if sustained else None
    sustained_until = final_row["ended_at"] if sustained and final_row else None
    candidate_seconds = None
    sustained_seconds = None
    if start_candidate:
        candidate_seconds = (parse_time(start_candidate, "candidate start") - validated["incident"]).total_seconds()
    if sustained_until:
        sustained_seconds = (parse_time(sustained_until, "sustained end") - validated["incident"]).total_seconds()

    if sustained:
        status = "example_only" if contract["status"] == "example_only" or observations["status"] == "example_only" else "sustained_recovery_verified"
    elif any(row["status"] == "fail" for row in timeline_results):
        status = "not_recovered"
    else:
        status = "indeterminate"

    report = {
        "schema": "luminous-opsbench-sustained-recovery-report-v0.1",
        "version": 1,
        "status": status,
        "contract_sha256": contract_digest,
        "windows_sha256": sha256(observation_bytes),
        "window_count": len(windows),
        "sustained_recovery": {
            "verified": sustained,
            "candidate_window_id": timeline_results[-consecutive]["window_id"] if sustained and consecutive else None,
            "sustained_from": sustained_from,
            "sustained_until": sustained_until,
            "hold_window_count": hold_count,
            "time_to_recovery_candidate_seconds": candidate_seconds,
            "time_to_sustained_recovery_seconds": sustained_seconds,
            "regression_count": regression_count,
            "indeterminate_window_count": indeterminate_count,
            "window_results": timeline_results,
        },
        "claim_ceiling": "SLO window evaluation over supplied records only. Does not authenticate observer/oracle publishers, prove workload representativeness, or establish safety assessment completeness. Example-only records cannot qualify production recovery.",
    }
    report["report_sha256"] = sha256(json.dumps(report, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode("utf-8"))
    return report


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--observations", type=Path, required=True)
    parser.add_argument("--output", default="-")
    args = parser.parse_args(argv)
    try:
        contract_bytes = args.contract.read_bytes()
        observation_bytes = args.observations.read_bytes()
        contract = load_json(contract_bytes, "contract")
        observations = load_json(observation_bytes, "observations")
        report = evaluate(contract, contract_bytes, observations, observation_bytes)
        rendered = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
        if args.output == "-":
            sys.stdout.write(rendered)
        else:
            args.output.write_text(rendered, encoding="utf-8")
        return 0 if report["status"] in {"example_only", "sustained_recovery_verified"} else (1 if report["status"] == "not_recovered" else 2)
    except (OSError, ContractError, AttributeError, KeyError, TypeError, ValueError, OverflowError, RecursionError) as exc:
        print(f"opsbench-slo-recovery: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
