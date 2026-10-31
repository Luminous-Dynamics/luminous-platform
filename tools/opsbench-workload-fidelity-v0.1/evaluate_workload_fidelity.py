#!/usr/bin/env python3
"""Evaluate demand-conditioned workload fidelity for sustained SLO recovery.

The evaluator verifies exact frozen artifacts, independent workload demand,
phase coverage, class mix and customer-visible outcomes. It is an offline
research checker; it does not generate load or authenticate publishers.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from datetime import datetime, timezone
from pathlib import Path

MAX_INPUT_BYTES = 32 * 1024 * 1024
MAX_CLASSES = 32
MAX_PHASES = 100
PHASE_ORDER = ["nominal", "burst", "dependency_degradation", "retry_pressure", "recovery_hold"]
PHASE_KINDS = set(PHASE_ORDER)


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


def canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode("utf-8")


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


def finite(value, where: str, minimum=None, maximum=None) -> float:
    require(type(value) in (int, float) and math.isfinite(value), f"{where}: finite number required")
    result = float(value)
    if minimum is not None:
        require(result >= minimum, f"{where}: below minimum")
    if maximum is not None:
        require(result <= maximum, f"{where}: above maximum")
    return result


def _hex64(value, where):
    require(isinstance(value, str) and len(value) == 64 and all(c in "0123456789abcdef" for c in value), f"{where}: lowercase SHA-256 digest required")


def _identity(value, where):
    require(isinstance(value, dict) and set(value) == {"source_id", "trust_domain_id"}, f"{where}: exact identity object required")
    require(all(isinstance(value.get(k), str) and value[k] for k in ("source_id", "trust_domain_id")), f"{where}: non-empty source and trust domain required")


def _histogram_p95(histogram: dict, offered_count: int, where: str):
    require(isinstance(histogram, dict) and set(histogram) == {"unit", "buckets"}, f"{where}: exact histogram object required")
    require(histogram["unit"] == "s", f"{where}: histogram unit must be seconds")
    buckets = histogram["buckets"]
    require(isinstance(buckets, list) and 2 <= len(buckets) <= 1000, f"{where}: bucket list outside bounds")
    total, cumulative, last_bound, selected = 0, 0, 0.0, None
    rank = max(1, math.ceil(0.95 * offered_count))
    for index, bucket in enumerate(buckets):
        require(isinstance(bucket, dict) and set(bucket) == {"upper_bound_seconds", "count"}, f"{where}.buckets[{index}]: invalid bucket")
        count = bucket["count"]
        require(type(count) is int and count >= 0, f"{where}.buckets[{index}].count: non-negative integer required")
        bound = bucket["upper_bound_seconds"]
        if bound is None:
            require(index == len(buckets) - 1, f"{where}: +Inf bucket must be last")
        else:
            bound = finite(bound, f"{where}.buckets[{index}].upper_bound_seconds", minimum=0.0)
            require(bound > last_bound, f"{where}: histogram bounds must be strictly increasing")
            last_bound = bound
        total += count
        cumulative += count
        if selected is None and cumulative >= rank:
            selected = bound
    require(buckets[-1]["upper_bound_seconds"] is None, f"{where}: final +Inf bucket required")
    require(total == offered_count, f"{where}: histogram counts must equal all externally offered requests")
    return selected


def _validate_profile(profile: dict):
    require(profile.get("schema") == "luminous-opsbench-workload-profile-v0.1" and type(profile.get("version")) is int and profile["version"] == 1, "profile: unsupported schema/version")
    require(profile.get("status") in {"example_only", "registered_profile"}, "profile: invalid status")
    for key in ("profile_id", "study_id"):
        require(isinstance(profile.get(key), str) and profile[key], f"profile.{key}: required")
    require(type(profile.get("window_seconds")) is int and 1 <= profile["window_seconds"] <= 3600, "profile.window_seconds outside bounds")
    classes = profile.get("request_classes")
    require(isinstance(classes, list) and 2 <= len(classes) <= MAX_CLASSES, "profile.request_classes outside bounds")
    class_map = {}
    for i, item in enumerate(classes):
        where = f"profile.request_classes[{i}]"
        required = {
            "class_id", "critical", "min_weight", "max_weight", "target_offered_rps",
            "min_success_ratio", "min_business_correctness_ratio", "min_business_check_count",
            "min_ingress_ratio", "max_p95_latency_seconds", "max_server_retry_attempts_per_100_offered",
        }
        require(isinstance(item, dict) and set(item) == required, f"{where}: unexpected or missing fields")
        class_id = item["class_id"]
        require(isinstance(class_id, str) and class_id and class_id not in class_map, f"{where}: duplicate/empty class ID")
        require(type(item["critical"]) is bool, f"{where}.critical must be boolean")
        minimum = finite(item["min_weight"], f"{where}.min_weight", minimum=0.0, maximum=1.0)
        maximum = finite(item["max_weight"], f"{where}.max_weight", minimum=0.0, maximum=1.0)
        require(minimum <= maximum and maximum > 0, f"{where}: invalid class share interval")
        finite(item["target_offered_rps"], f"{where}.target_offered_rps", minimum=0.000001)
        finite(item["min_success_ratio"], f"{where}.min_success_ratio", minimum=0.0, maximum=1.0)
        finite(item["min_business_correctness_ratio"], f"{where}.min_business_correctness_ratio", minimum=0.0, maximum=1.0)
        require(type(item["min_business_check_count"]) is int and item["min_business_check_count"] >= 1, f"{where}: minimum business checks required")
        finite(item["min_ingress_ratio"], f"{where}.min_ingress_ratio", minimum=0.0, maximum=1.0)
        finite(item["max_p95_latency_seconds"], f"{where}.max_p95_latency_seconds", minimum=0.000001)
        finite(item["max_server_retry_attempts_per_100_offered"], f"{where}.max_server_retry_attempts_per_100_offered", minimum=0.0)
        class_map[class_id] = item
    require(sum(x["min_weight"] for x in classes) <= 1.0 + 1e-9, "profile: minimum class shares exceed 100%")
    require(sum(x["max_weight"] for x in classes) >= 1.0 - 1e-9, "profile: maximum class shares cannot cover 100%")
    return class_map


def _validate_schedule(schedule: dict, contract: dict, profile: dict):
    require(schedule.get("schema") == "luminous-opsbench-workload-phase-schedule-v0.1" and type(schedule.get("version")) is int and schedule["version"] == 1, "schedule: unsupported schema/version")
    require(schedule.get("status") in {"example_only", "registered_schedule"}, "schedule: invalid status")
    require(schedule.get("study_id") == contract["study_id"] and schedule.get("profile_id") == profile["profile_id"], "schedule: study/profile identity mismatch")
    require(schedule.get("window_seconds") == profile["window_seconds"], "schedule/profile window duration mismatch")
    phases = schedule.get("phases")
    require(isinstance(phases, list) and len(phases) == contract["expected_phase_count"] and 5 <= len(phases) <= MAX_PHASES, "schedule: phase count mismatch")
    kinds = [phase.get("kind") for phase in phases if isinstance(phase, dict)]
    require(kinds == PHASE_ORDER, "schedule: required challenge phases must occur in fixed order")
    phase_map, previous_end = {}, None
    for i, phase in enumerate(phases):
        where = f"schedule.phases[{i}]"
        required = {"phase_id", "kind", "started_at", "ended_at", "rate_multiplier_min", "rate_multiplier_max", "fault_id", "fault_receipt_sha256"}
        require(set(phase) == required, f"{where}: unexpected or missing fields")
        phase_id = phase["phase_id"]
        require(isinstance(phase_id, str) and phase_id and phase_id not in phase_map, f"{where}: duplicate/empty phase ID")
        start, end = parse_time(phase["started_at"], f"{where}.started_at"), parse_time(phase["ended_at"], f"{where}.ended_at")
        require(end > start and (end-start).total_seconds() == profile["window_seconds"], f"{where}: exact phase window duration required")
        if previous_end is not None:
            require(start == previous_end, f"{where}: phase schedule has a gap or overlap")
        previous_end = end
        low = finite(phase["rate_multiplier_min"], f"{where}.rate_multiplier_min", minimum=0.000001)
        high = finite(phase["rate_multiplier_max"], f"{where}.rate_multiplier_max", minimum=0.000001)
        require(low <= high, f"{where}: invalid offered-rate multiplier range")
        faulted = phase["kind"] in {"dependency_degradation", "retry_pressure"}
        if faulted:
            require(isinstance(phase["fault_id"], str) and phase["fault_id"], f"{where}: fault phase requires a named fault")
            _hex64(phase["fault_receipt_sha256"], f"{where}.fault_receipt_sha256")
        else:
            require(phase["fault_id"] is None and phase["fault_receipt_sha256"] is None, f"{where}: non-fault phase cannot claim a fault receipt")
        phase_map[phase_id] = phase
    return phase_map


def evaluate(contract: dict, contract_bytes: bytes, profile: dict, profile_bytes: bytes, schedule: dict, schedule_bytes: bytes, load_receipts: dict, load_bytes: bytes, observations: dict, observation_bytes: bytes, fault_receipts: dict, fault_receipt_bytes: bytes) -> dict:
    required_contract = {
        "schema", "version", "status", "study_id", "run_id", "episode_id", "service_id", "service_digest",
        "executor", "load_generator", "observer", "business_oracle", "fault_controller", "profile_sha256",
        "phase_schedule_sha256", "load_generator_receipts_sha256", "service_observations_sha256", "fault_controller_receipts_sha256", "expected_phase_count",
    }
    require(set(contract) == required_contract, "contract: unexpected or missing fields")
    require(contract.get("schema") == "luminous-opsbench-workload-fidelity-contract-v0.1" and type(contract.get("version")) is int and contract["version"] == 1, "contract: unsupported schema/version")
    require(contract.get("status") in {"example_only", "registered_contract"}, "contract: invalid status")
    for key in ("study_id", "run_id", "episode_id", "service_id"):
        require(isinstance(contract.get(key), str) and contract[key], f"contract.{key}: required")
    _hex64(contract["service_digest"], "contract.service_digest")
    for key in ("executor", "load_generator", "observer", "business_oracle", "fault_controller"):
        _identity(contract.get(key), f"contract.{key}")
    domains = [contract[k]["trust_domain_id"] for k in ("executor", "load_generator", "observer", "business_oracle", "fault_controller")]
    require(len(domains) == len(set(domains)), "contract: executor, load generator, observer, business oracle and fault controller must use distinct trust domains")
    for field, data in (
        ("profile_sha256", profile_bytes),
        ("phase_schedule_sha256", schedule_bytes),
        ("load_generator_receipts_sha256", load_bytes),
        ("service_observations_sha256", observation_bytes),
        ("fault_controller_receipts_sha256", fault_receipt_bytes),
    ):
        _hex64(contract.get(field), f"contract.{field}")
        require(contract[field] == sha256(data), f"contract.{field}: exact raw artifact digest mismatch")

    components = [profile, schedule, load_receipts, observations, fault_receipts]
    require(all(component.get("status") == contract["status"] for component in components), "component status does not match frozen contract lifecycle")
    class_map = _validate_profile(profile)
    require(profile["study_id"] == contract["study_id"], "profile: study identity mismatch")
    phase_map = _validate_schedule(schedule, contract, profile)
    for label, component in (("load receipts", load_receipts), ("service observations", observations)):
        require(component.get("study_id") == contract["study_id"] and component.get("run_id") == contract["run_id"] and component.get("episode_id") == contract["episode_id"] and component.get("service_id") == contract["service_id"], f"{label}: cross-run/service substitution")
        require(component.get("profile_sha256") == contract["profile_sha256"] and component.get("phase_schedule_sha256") == contract["phase_schedule_sha256"], f"{label}: profile/schedule digest mismatch")

    require(load_receipts.get("schema") == "luminous-opsbench-load-generator-receipts-v0.1" and type(load_receipts.get("version")) is int and load_receipts["version"] == 1, "load receipts: unsupported schema/version")
    require(observations.get("schema") == "luminous-opsbench-service-observations-v0.1" and type(observations.get("version")) is int and observations["version"] == 1, "service observations: unsupported schema/version")
    require(fault_receipts.get("schema") == "luminous-opsbench-fault-controller-receipts-v0.1" and type(fault_receipts.get("version")) is int and fault_receipts["version"] == 1, "fault receipts: unsupported schema/version")
    _identity(load_receipts.get("load_generator"), "load receipts.load_generator")
    _identity(observations.get("observer"), "service observations.observer")
    _identity(observations.get("business_oracle"), "service observations.business_oracle")
    require(load_receipts["load_generator"] == contract["load_generator"], "load receipts: publisher identity mismatch")
    require(observations["observer"] == contract["observer"], "service observations: observer identity mismatch")
    require(observations["business_oracle"] == contract["business_oracle"], "service observations: business oracle identity mismatch")
    _identity(fault_receipts.get("fault_controller"), "fault receipts.fault_controller")
    require(fault_receipts["fault_controller"] == contract["fault_controller"], "fault receipts: publisher identity mismatch")
    require(fault_receipts.get("study_id") == contract["study_id"] and fault_receipts.get("run_id") == contract["run_id"] and fault_receipts.get("episode_id") == contract["episode_id"] and fault_receipts.get("service_id") == contract["service_id"], "fault receipts: cross-run/service substitution")
    require(fault_receipts.get("phase_schedule_sha256") == contract["phase_schedule_sha256"], "fault receipts: phase schedule digest mismatch")
    faulted_phases = {pid: phase for pid, phase in phase_map.items() if phase["kind"] in {"dependency_degradation", "retry_pressure"}}
    fault_rows = fault_receipts.get("receipts")
    require(isinstance(fault_rows, list) and len(fault_rows) == len(faulted_phases), "fault receipts: missing or extra fault phase receipts")
    seen_fault_phases = set()
    for index, receipt in enumerate(fault_rows):
        where = f"fault_receipts.receipts[{index}]"
        required_fault_fields = {"phase_id", "fault_id", "started_at", "ended_at", "target_id", "action", "action_digest", "disposition"}
        require(isinstance(receipt, dict) and set(receipt) == required_fault_fields, f"{where}: unexpected or missing fields")
        phase_id = receipt["phase_id"]
        require(phase_id in faulted_phases and phase_id not in seen_fault_phases, f"{where}: unknown or duplicate fault phase")
        seen_fault_phases.add(phase_id)
        phase = faulted_phases[phase_id]
        require(receipt["fault_id"] == phase["fault_id"], f"{where}: fault ID does not match frozen schedule")
        require(receipt["started_at"] == phase["started_at"] and receipt["ended_at"] == phase["ended_at"], f"{where}: fault execution time does not match frozen phase window")
        require(isinstance(receipt["target_id"], str) and receipt["target_id"] and isinstance(receipt["action"], str) and receipt["action"], f"{where}: target/action required")
        _hex64(receipt["action_digest"], f"{where}.action_digest")
        require(receipt["disposition"] == "injected", f"{where}: fault was not confirmed injected")
    require(seen_fault_phases == set(faulted_phases), "fault receipts: fault phase coverage mismatch")

    def map_records(component, label):
        records = component.get("windows")
        require(isinstance(records, list) and len(records) == len(phase_map) * len(class_map), f"{label}: missing/extra class-window records")
        result = {}
        for index, row in enumerate(records):
            require(isinstance(row, dict), f"{label}.windows[{index}]: object required")
            key = (row.get("phase_id"), row.get("class_id"))
            require(key not in result, f"{label}: duplicate phase/class record")
            require(key[0] in phase_map and key[1] in class_map, f"{label}: unknown phase or request class")
            require(isinstance(row.get("window_id"), str) and row["window_id"], f"{label}: window ID required")
            phase = phase_map[key[0]]
            require(row["window_id"] == "window-" + key[0], f"{label}: window ID/phase mismatch")
            if label in {"load receipts", "service observations"}:
                require(row.get("started_at") == phase["started_at"] and row.get("ended_at") == phase["ended_at"], f"{label}: window time differs from frozen phase schedule")
            result[key] = row
        expected = {(phase_id, class_id) for phase_id in phase_map for class_id in class_map}
        require(set(result) == expected, f"{label}: phase/class coverage mismatch")
        return result

    load_map = map_records(load_receipts, "load receipts")
    observation_map = map_records(observations, "service observations")
    report_windows, per_class = [], {}
    total_offered, total_success = 0, 0
    critical_class_failure_count = 0
    for class_id, cls in class_map.items():
        per_class[class_id] = {"critical": cls["critical"], "offered_count": 0, "success_count": 0, "window_count": 0, "failed_window_count": 0, "indeterminate_window_count": 0}

    for phase_id, phase in phase_map.items():
        phase_offers = {class_id: load_map[(phase_id, class_id)]["offered_count"] for class_id in class_map}
        phase_total = sum(phase_offers.values())
        require(phase_total > 0, f"{phase_id}: no offered requests")
        for class_id, cls in class_map.items():
            key = (phase_id, class_id)
            load = load_map[key]
            obs = observation_map[key]
            where = f"window[{phase_id}/{class_id}]"
            required_load_fields = {"window_id", "phase_id", "class_id", "started_at", "ended_at", "offered_count", "offered_rps", "response_success_count", "response_error_count", "response_timeout_count", "response_drop_count", "peak_concurrency", "latency_histogram"}
            require(set(load) == required_load_fields, f"{where}: unexpected/missing load receipt fields")
            require(type(load["offered_count"]) is int and load["offered_count"] > 0, f"{where}: positive offered count required")
            duration = profile["window_seconds"]
            expected_rps = load["offered_count"] / duration
            rps = finite(load["offered_rps"], f"{where}.offered_rps", minimum=0.000001)
            require(abs(rps-expected_rps) <= 1e-9, f"{where}: offered_rps does not reconcile to count/window duration")
            require(type(load["peak_concurrency"]) is int and load["peak_concurrency"] > 0, f"{where}: peak concurrency required")
            counts = [load[k] for k in ("response_success_count", "response_error_count", "response_timeout_count", "response_drop_count")]
            require(all(type(x) is int and x >= 0 for x in counts) and sum(counts) == load["offered_count"], f"{where}: all offered requests must end in success/error/timeout/drop; no denominator loss")
            multiplier_low, multiplier_high = phase["rate_multiplier_min"], phase["rate_multiplier_max"]
            base_rps = cls["target_offered_rps"]
            load_fidelity_reasons = []
            if not base_rps * multiplier_low <= rps <= base_rps * multiplier_high:
                load_fidelity_reasons.append("offered demand is outside the frozen phase rate band")
            weight = load["offered_count"] / phase_total
            if not cls["min_weight"] <= weight <= cls["max_weight"]:
                load_fidelity_reasons.append("request-class mix is outside the frozen profile band")
            p95_upper = _histogram_p95(load["latency_histogram"], load["offered_count"], where + ".latency_histogram")

            required_obs = {"window_id", "phase_id", "class_id", "server_received_count", "server_rejected_count", "server_retry_attempt_count", "business_checked_count", "business_correct_count"}
            require(set(obs) == required_obs, f"{where}: unexpected/missing observer fields")
            received, rejected, retries, checked, correct = [obs[k] for k in ("server_received_count", "server_rejected_count", "server_retry_attempt_count", "business_checked_count", "business_correct_count")]
            require(all(type(x) is int and x >= 0 for x in (received, rejected, retries, checked, correct)), f"{where}: non-negative observer counters required")
            require(received <= load["offered_count"] and rejected <= received, f"{where}: observer ingress/rejection counts inconsistent")
            require(correct <= checked <= load["response_success_count"], f"{where}: business oracle checks must bind to client-observed successful responses")
            require(obs["started_at"] == phase_map[phase_id]["started_at"] and obs["ended_at"] == phase_map[phase_id]["ended_at"], f"{where}: observer timestamps do not match frozen phase window")

            offered = load["offered_count"]
            success_ratio = load["response_success_count"] / offered
            ingress_ratio = received / offered
            business_ratio = correct / checked if checked else None
            retry_rate = retries / offered * 100.0
            dispositions, reasons = [], []
            if load_fidelity_reasons:
                dispositions.append("indeterminate")
                reasons.extend(load_fidelity_reasons)
            if ingress_ratio < cls["min_ingress_ratio"]:
                dispositions.append("fail")
                reasons.append("independent observer ingress ratio is below the class floor")
            if rejected > 0:
                dispositions.append("fail")
                reasons.append("service-side rejected requests are non-zero")
            if success_ratio < cls["min_success_ratio"]:
                dispositions.append("fail")
                reasons.append("externally observed response success ratio is below the class floor")
            if checked < cls["min_business_check_count"]:
                dispositions.append("indeterminate")
                reasons.append("business correctness sample count is below the class minimum")
            elif business_ratio < cls["min_business_correctness_ratio"]:
                dispositions.append("fail")
                reasons.append("independent business correctness ratio is below the class floor")
            if p95_upper is None:
                dispositions.append("indeterminate")
                reasons.append("p95 latency is hidden in the overflow bucket")
            elif p95_upper > cls["max_p95_latency_seconds"]:
                dispositions.append("fail")
                reasons.append("external p95 latency exceeds the class ceiling")
            if retry_rate > cls["max_server_retry_attempts_per_100_offered"]:
                dispositions.append("fail")
                reasons.append("server retry attempts exceed the class retry budget")
            disposition = "fail" if "fail" in dispositions else ("indeterminate" if "indeterminate" in dispositions else "pass")
            total_offered += offered
            total_success += load["response_success_count"]
            class_report = per_class[class_id]
            class_report["offered_count"] += offered
            class_report["success_count"] += load["response_success_count"]
            class_report["window_count"] += 1
            class_report["failed_window_count"] += disposition == "fail"
            class_report["indeterminate_window_count"] += disposition == "indeterminate"
            if cls["critical"] and disposition == "fail":
                critical_class_failure_count += 1
            report_windows.append({
                "window_id": load["window_id"], "phase_id": phase_id, "phase_kind": phase["kind"], "class_id": class_id,
                "offered_count": offered, "observed_server_received_count": received, "observed_response_success_count": load["response_success_count"],
                "success_ratio": success_ratio, "ingress_ratio": ingress_ratio, "business_correctness_ratio": business_ratio,
                "server_retry_attempts_per_100_offered": retry_rate, "p95_latency_upper_bound_seconds": p95_upper,
                "load_fidelity_reasons": load_fidelity_reasons, "disposition": disposition, "reasons": reasons,
            })

    dispositions = [row["disposition"] for row in report_windows]
    computed = "fail" if "fail" in dispositions else ("indeterminate" if "indeterminate" in dispositions else "qualified")
    report = {
        "schema": "luminous-opsbench-workload-fidelity-report-v0.1",
        "version": 1,
        "status": "example_only" if contract["status"] == "example_only" else computed,
        "computed_disposition": computed,
        "study_id": contract["study_id"], "run_id": contract["run_id"], "episode_id": contract["episode_id"],
        "profile_sha256": contract["profile_sha256"], "phase_schedule_sha256": contract["phase_schedule_sha256"],
        "load_generator_receipts_sha256": contract["load_generator_receipts_sha256"],
        "service_observations_sha256": contract["service_observations_sha256"],
        "fault_controller_receipts_sha256": contract["fault_controller_receipts_sha256"],
        "overall_aggregate_response_success_ratio": total_success / total_offered if total_offered else None,
        "total_offered_requests": total_offered,
        "total_successful_responses": total_success,
        "critical_class_failure_count": critical_class_failure_count,
        "per_class": per_class,
        "windows": report_windows,
        "publisher_authenticity_verified": False,
        "population_capacity_claim_permitted": False,
        "automatic_execution_authorized": False,
        "claim_ceiling": "Workload fidelity and class-level service metrics over supplied synthetic receipts only. Does not authenticate publishers, prove load generator or observer independence, or establish production capacity.",
    }
    report["report_sha256"] = sha256(canonical(report))
    return report


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--schedule", type=Path, required=True)
    parser.add_argument("--load-receipts", type=Path, required=True)
    parser.add_argument("--service-observations", type=Path, required=True)
    parser.add_argument("--fault-receipts", type=Path, required=True)
    parser.add_argument("--output", default="-")
    args = parser.parse_args(argv)
    try:
        paths = {
            "contract": args.contract, "profile": args.profile, "schedule": args.schedule,
            "load receipts": args.load_receipts, "service observations": args.service_observations, "fault receipts": args.fault_receipts,
        }
        raw = {key: path.read_bytes() for key, path in paths.items()}
        docs = {key: load_json(data, key) for key, data in raw.items()}
        report = evaluate(
            docs["contract"], raw["contract"], docs["profile"], raw["profile"], docs["schedule"], raw["schedule"],
            docs["load receipts"], raw["load receipts"], docs["service observations"], raw["service observations"],
            docs["fault receipts"], raw["fault receipts"],
        )
        rendered = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
        if args.output == "-":
            sys.stdout.write(rendered)
        else:
            args.output.write_text(rendered, encoding="utf-8")
        return 0 if report["status"] == "example_only" else (1 if report["computed_disposition"] == "fail" else (2 if report["computed_disposition"] == "indeterminate" else 0))
    except (OSError, ContractError, AttributeError, KeyError, TypeError, ValueError, OverflowError, RecursionError) as exc:
        print(f"opsbench-workload-fidelity: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
