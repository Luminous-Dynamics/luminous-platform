#!/usr/bin/env python3
"""Scenario-cluster power/MDE planning from a disjoint OpsBench screening pilot.

This is a pre-confirmatory planning tool. It never reads confirmatory outcomes.
Its normal approximation and Gaussian power simulation are planning diagnostics,
not the final cluster-bootstrap confirmatory analysis.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import random
import sys
from pathlib import Path
from statistics import NormalDist, mean, stdev

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_PLAN = ROOT / "benchmarks" / "opsbench-assurance-statistics-v0.1" / "power-plan.example.json"
DEFAULT_PREREG = ROOT / "benchmarks" / "opsbench-common-core-v0.1" / "preregistration.example.json"
MAX_INPUT_BYTES = 32 * 1024 * 1024
MAX_CLUSTERS = 100_000
MAX_COMPARISONS = 100
MAX_SIMULATIONS = 100_000


class PlanningError(ValueError):
    pass


def _strict_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise PlanningError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_constant(value):
    raise PlanningError(f"non-finite JSON number is prohibited: {value}")


def load_json(data: bytes, label: str) -> dict:
    if len(data) > MAX_INPUT_BYTES:
        raise PlanningError(f"{label}: input exceeds size limit")
    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=_strict_object, parse_constant=_reject_constant)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise PlanningError(f"{label}: invalid UTF-8 JSON: {exc}") from exc
    if not isinstance(value, dict):
        raise PlanningError(f"{label}: top-level object required")
    return value


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical(value: object) -> bytes:
    try:
        return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode("utf-8")
    except (TypeError, ValueError) as exc:
        raise PlanningError(f"cannot canonicalize input: {exc}") from exc


def require(condition: bool, message: str):
    if not condition:
        raise PlanningError(message)


def number(value, where: str, *, lower: float | None = None, upper: float | None = None) -> float:
    require(type(value) in (int, float) and math.isfinite(value), f"{where}: finite number required")
    result = float(value)
    if lower is not None:
        require(result >= lower, f"{where}: below minimum")
    if upper is not None:
        require(result <= upper, f"{where}: above maximum")
    return result


def _validate_and_collect(plan: dict, prereg: dict, prereg_bytes: bytes) -> tuple[dict, dict[str, list[float]], dict]:
    expected = {
        "schema", "version", "status", "study_id", "preregistration_sha256", "pilot_started_at",
        "baseline_arm_id", "candidate_arm_ids", "pilot_scenario_ids", "pilot_clusters", "planning_parameters",
    }
    require(set(plan) == expected, "plan: unexpected or missing fields")
    require(plan.get("schema") == "luminous-opsbench-paired-power-plan-v0.1" and type(plan.get("version")) is int and plan["version"] == 1, "plan: unsupported schema/version")
    require(plan.get("status") in {"example_only", "screening_pilot"}, "plan: invalid status")
    require(plan.get("study_id") == prereg.get("study_id"), "plan: study ID differs from preregistration")
    require(prereg.get("schema") == "luminous-opsbench-common-core-preregistration-v0.1" and type(prereg.get("version")) is int and prereg["version"] == 1, "preregistration: unsupported schema/version")
    require(plan.get("preregistration_sha256") == sha256(prereg_bytes), "plan: exact preregistration digest mismatch")

    plan_status = plan["status"]
    if plan_status == "example_only":
        require(prereg.get("status") == "draft_not_registered", "example-only plan requires example-only draft preregistration")
    else:
        require(prereg.get("status") == "registered_before_trials", "screening pilot requires preregistration before data collection")
        registered = datetime_aware(prereg.get("registered_at"), "preregistration.registered_at")
        pilot_started = datetime_aware(plan.get("pilot_started_at"), "plan.pilot_started_at")
        require(registered < pilot_started, "preregistration must predate screening pilot")
    if plan_status == "example_only":
        datetime_aware(plan.get("pilot_started_at"), "plan.pilot_started_at")

    registered_arms = {x.get("arm_id"): x for x in prereg.get("arms", []) if isinstance(x, dict)}
    baseline = plan.get("baseline_arm_id")
    candidates = plan.get("candidate_arm_ids")
    require(isinstance(baseline, str) and baseline in registered_arms, "plan.baseline_arm_id is not registered")
    require(isinstance(candidates, list) and 1 <= len(candidates) <= MAX_COMPARISONS, "plan.candidate_arm_ids outside supported bounds")
    require(all(isinstance(x, str) and x in registered_arms and x != baseline for x in candidates), "plan contains unregistered or baseline candidate arm")
    require(len(candidates) == len(set(candidates)), "plan contains duplicate candidate arms")
    reference = registered_arms[baseline]
    for candidate in candidates:
        comparison_arm = registered_arms[candidate]
        for field in ("tool_surface_digest", "permission_profile_digest", "budget_profile_digest"):
            require(comparison_arm.get(field) == reference.get(field), f"power comparison is not common-core fair: {candidate} differs on {field}")

    split = prereg.get("scenario_split", {})
    development = set(split.get("development_scenario_ids", []))
    screening = set(split.get("screening_scenario_ids", []))
    require(not (development & screening), "preregistration has overlapping development and screening splits")
    scenario_ids = plan.get("pilot_scenario_ids")
    require(isinstance(scenario_ids, list) and len(scenario_ids) >= 5, "pilot requires at least five distinct scenario clusters")
    require(all(isinstance(x, str) and x for x in scenario_ids), "pilot scenario IDs must be non-empty strings")
    require(len(scenario_ids) == len(set(scenario_ids)), "pilot_scenario_ids contains duplicates")
    require(set(scenario_ids).issubset(screening), "pilot includes a scenario outside the preregistered screening split")
    require(not (set(scenario_ids) & development), "pilot overlaps development split")
    require(not (set(scenario_ids) & set(split.get("confirmatory_scenario_ids", []))), "pilot overlaps confirmatory split")

    parameters = plan.get("planning_parameters")
    required_parameters = {
        "target_effect", "target_power", "familywise_alpha", "candidate_comparison_count",
        "variance_floor", "safety_margin_fraction", "minimum_confirmatory_scenarios",
        "simulation_replicates", "simulation_seed", "sensitivity_effects",
        "sensitivity_powers", "sensitivity_comparison_counts",
    }
    require(isinstance(parameters, dict) and set(parameters) == required_parameters, "planning_parameters: unexpected or missing fields")
    target_effect = number(parameters["target_effect"], "target_effect", lower=1e-9, upper=1.0)
    target_power = number(parameters["target_power"], "target_power", lower=0.500001, upper=0.999999)
    familywise_alpha = number(parameters["familywise_alpha"], "familywise_alpha", lower=1e-6, upper=0.5)
    comparison_count = parameters["candidate_comparison_count"]
    require(type(comparison_count) is int and comparison_count == len(candidates), "candidate_comparison_count must equal the candidate arm family size")
    variance_floor = number(parameters["variance_floor"], "variance_floor", lower=1e-9, upper=1.0)
    safety_margin = number(parameters["safety_margin_fraction"], "safety_margin_fraction", lower=0.0, upper=2.0)
    minimum_n = parameters["minimum_confirmatory_scenarios"]
    require(type(minimum_n) is int and minimum_n >= 100, "minimum_confirmatory_scenarios must be at least 100")
    simulations = parameters["simulation_replicates"]
    require(type(simulations) is int and 100 <= simulations <= MAX_SIMULATIONS, "simulation_replicates outside supported bounds")
    seed = parameters["simulation_seed"]
    require(type(seed) is int and 0 <= seed <= 0xFFFFFFFF, "simulation_seed outside supported bounds")

    sensitivity_effects = parameters["sensitivity_effects"]
    sensitivity_powers = parameters["sensitivity_powers"]
    sensitivity_comparisons = parameters["sensitivity_comparison_counts"]
    require(isinstance(sensitivity_effects, list) and sensitivity_effects and len(sensitivity_effects) <= 20, "sensitivity_effects invalid")
    require(isinstance(sensitivity_powers, list) and sensitivity_powers and len(sensitivity_powers) <= 20, "sensitivity_powers invalid")
    require(isinstance(sensitivity_comparisons, list) and sensitivity_comparisons and len(sensitivity_comparisons) <= 20, "sensitivity_comparison_counts invalid")
    sensitivity_effects = [number(x, "sensitivity_effect", lower=1e-9, upper=1.0) for x in sensitivity_effects]
    sensitivity_powers = [number(x, "sensitivity_power", lower=0.500001, upper=0.999999) for x in sensitivity_powers]
    require(all(type(x) is int and 1 <= x <= MAX_COMPARISONS for x in sensitivity_comparisons), "sensitivity comparison count invalid")

    rows = plan.get("pilot_clusters")
    require(isinstance(rows, list) and 5 <= len(rows) <= MAX_CLUSTERS, "pilot_clusters outside supported bounds")
    row_map, effects = {}, {candidate: [] for candidate in candidates}
    unmatched_by_candidate = {candidate: [] for candidate in candidates}
    total_nested_seeds = 0
    for index, row in enumerate(rows):
        where = f"pilot_clusters[{index}]"
        require(isinstance(row, dict) and set(row) == {"scenario_id", "seed_count", "arm_success_rates"}, f"{where}: exact fields required")
        scenario_id = row.get("scenario_id")
        require(isinstance(scenario_id, str) and scenario_id in scenario_ids, f"{where}: scenario outside declared pilot")
        require(scenario_id not in row_map, f"{where}: duplicate scenario cluster")
        seed_count = row.get("seed_count")
        require(type(seed_count) is int and seed_count >= 1, f"{where}.seed_count: positive integer required")
        total_nested_seeds += seed_count
        rates = row.get("arm_success_rates")
        require(isinstance(rates, dict), f"{where}.arm_success_rates: object required")
        expected_arms = {baseline, *candidates}
        require(set(rates) == expected_arms, f"{where}: arm rate keys must exactly match baseline and candidate arms")
        for arm, rate in rates.items():
            if rate is not None:
                rates[arm] = number(rate, f"{where}.{arm}", lower=0.0, upper=1.0)
        row_map[scenario_id] = {"seed_count": seed_count, "rates": rates}
        for candidate in candidates:
            base_rate, candidate_rate = rates[baseline], rates[candidate]
            if base_rate is None or candidate_rate is None:
                unmatched_by_candidate[candidate].append(scenario_id)
            else:
                effects[candidate].append(candidate_rate - base_rate)
    require(set(row_map) == set(scenario_ids), "pilot cluster rows must exactly match pilot_scenario_ids")
    require(all(len(effects[candidate]) >= 5 for candidate in candidates), "each candidate requires at least five exact paired screening clusters")

    return {
        "parameters": {
            "target_effect": target_effect, "target_power": target_power,
            "familywise_alpha": familywise_alpha, "comparison_count": comparison_count,
            "variance_floor": variance_floor, "safety_margin_fraction": safety_margin,
            "minimum_confirmatory_scenarios": minimum_n, "simulation_replicates": simulations,
            "simulation_seed": seed,
        },
        "sensitivity": {
            "effects": sensitivity_effects, "powers": sensitivity_powers,
            "comparison_counts": sensitivity_comparisons,
        },
        "baseline_arm_id": baseline,
        "candidate_arm_ids": candidates,
        "pilot_scenario_count": len(scenario_ids),
        "total_nested_seed_count": total_nested_seeds,
        "effects_by_candidate": effects,
        "unmatched_by_candidate": unmatched_by_candidate,
        "pilot_row_map": row_map,
    }, prereg, registered_arms


def datetime_aware(value, where: str):
    from datetime import datetime
    require(isinstance(value, str), f"{where}: timestamp string required")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise PlanningError(f"{where}: invalid timestamp") from exc
    require(parsed.tzinfo is not None, f"{where}: timezone offset required")
    return parsed


def _normal_required_n(sd: float, effect: float, power: float, alpha: float, comparisons: int) -> tuple[int, float, float]:
    per_comparison_alpha = alpha / comparisons
    distribution = NormalDist()
    z_critical = distribution.inv_cdf(1.0 - per_comparison_alpha / 2.0)
    z_power = distribution.inv_cdf(power)
    raw = ((z_critical + z_power) * sd / effect) ** 2
    return max(2, math.ceil(raw)), z_critical, per_comparison_alpha


def _simulated_power(n: int, sd: float, effect: float, alpha_per_comparison: float, replicates: int, seed: int) -> float:
    rng = random.Random(seed)
    z_critical = NormalDist().inv_cdf(1.0 - alpha_per_comparison / 2.0)
    standard_error = sd / math.sqrt(n)
    detections = 0
    for _ in range(replicates):
        simulated_mean = rng.gauss(effect, standard_error)
        z_statistic = simulated_mean / standard_error
        if abs(z_statistic) > z_critical:
            detections += 1
    return detections / replicates


def _plan_for_effect(sd: float, effect: float, power: float, alpha: float, comparisons: int, margin: float, floor: int) -> dict:
    n_raw, z_critical, alpha_pc = _normal_required_n(sd, effect, power, alpha, comparisons)
    n_margin = math.ceil(n_raw * (1.0 + margin))
    return {
        "target_effect": effect,
        "target_power": power,
        "familywise_alpha": alpha,
        "candidate_comparison_count": comparisons,
        "per_comparison_two_sided_alpha": alpha_pc,
        "z_critical": z_critical,
        "normal_approximation_scenario_clusters": n_raw,
        "after_safety_margin_scenario_clusters": n_margin,
        "minimum_floor": floor,
        "recommended_scenario_clusters": max(floor, n_margin),
        "planning_method": "two-sided normal approximation using paired scenario-cluster effect SD; not the confirmatory estimator",
    }


def plan_power(plan: dict, plan_bytes: bytes, prereg: dict, prereg_bytes: bytes) -> dict:
    validated, _, _ = _validate_and_collect(plan, prereg, prereg_bytes)
    if plan["status"] == "screening_pilot":
        require(prereg.get("status") == "registered_before_trials", "screening pilot requires pre-trial preregistration")
    effects_by_candidate = validated["effects_by_candidate"]
    params = validated["parameters"]
    reports, sensitivity = {}, []

    for index, candidate in enumerate(validated["candidate_arm_ids"]):
        values = effects_by_candidate[candidate]
        pilot_mean = mean(values)
        pilot_sd = stdev(values) if len(values) >= 2 else 0.0
        variance_degenerate = pilot_sd <= 1e-12
        planning_sd = max(pilot_sd, params["variance_floor"])
        n_calc, z_critical, alpha_pc = _normal_required_n(
            planning_sd, params["target_effect"], params["target_power"],
            params["familywise_alpha"], params["comparison_count"]
        )
        n_with_margin = math.ceil(n_calc * (1.0 + params["safety_margin_fraction"]))
        recommended = max(params["minimum_confirmatory_scenarios"], n_with_margin)
        simulated_power = _simulated_power(
            recommended, planning_sd, params["target_effect"], alpha_pc,
            params["simulation_replicates"], params["simulation_seed"] + index * 1009
        )
        sorted_values = sorted(values)
        reports[candidate] = {
            "pilot_paired_cluster_count": len(values),
            "pilot_unmatched_cluster_count": len(validated["unmatched_by_candidate"][candidate]),
            "pilot_unmatched_scenario_ids": sorted(validated["unmatched_by_candidate"][candidate]),
            "pilot_paired_mean_difference": pilot_mean,
            "pilot_paired_sd": pilot_sd,
            "planning_sd_after_variance_floor": planning_sd,
            "variance_degenerate": variance_degenerate,
            "pilot_paired_effect_min": sorted_values[0],
            "pilot_paired_effect_max": sorted_values[-1],
            "normal_approximation_scenario_clusters": n_calc,
            "after_safety_margin_scenario_clusters": n_with_margin,
            "minimum_confirmatory_floor": params["minimum_confirmatory_scenarios"],
            "recommended_confirmatory_scenario_clusters": recommended,
            "gaussian_scenario_cluster_simulated_power": simulated_power,
            "simulated_power_is_planning_diagnostic_only": True,
        }

    conservative_sensitivity_sd = max(
        max(
            stdev(effects_by_candidate[candidate]) if len(effects_by_candidate[candidate]) >= 2 else 0.0,
            params["variance_floor"],
        )
        for candidate in validated["candidate_arm_ids"]
    )
    for effect in validated["sensitivity"]["effects"]:
        for power in validated["sensitivity"]["powers"]:
            for comparisons in validated["sensitivity"]["comparisons"]:
                n_raw, zc, alpha_pc = _normal_required_n(
                    conservative_sensitivity_sd, effect, power,
                    params["familywise_alpha"], comparisons
                )
                n_margin = math.ceil(n_raw * (1.0 + params["safety_margin_fraction"]))
                sensitivity.append({
                    "variance_sd_assumption": conservative_sensitivity_sd,
                    "target_effect": effect,
                    "target_power": power,
                    "candidate_comparison_count": comparisons,
                    "per_comparison_two_sided_alpha": alpha_pc,
                    "z_critical": zc,
                    "normal_approximation_scenario_clusters": n_raw,
                    "after_safety_margin_scenario_clusters": n_margin,
                    "recommended_with_floor": max(params["minimum_confirmatory_scenarios"], n_margin),
                })

    return {
        "schema": "luminous-opsbench-paired-power-report-v0.1",
        "version": 1,
        "status": "example_only" if plan["status"] == "example_only" else "planning_only_not_registered_confirmatory",
        "study_id": plan["study_id"],
        "plan_sha256": sha256(plan_bytes),
        "preregistration_sha256": sha256(prereg_bytes),
        "pilot_split": "screening",
        "pilot_scenario_cluster_count": validated["pilot_scenario_count"],
        "pilot_nested_seed_total_not_inferential_n": validated["total_nested_seed_count"],
        "baseline_arm_id": validated["baseline_arm_id"],
        "candidate_arm_ids": validated["candidate_arm_ids"],
        "analysis_unit": "scenario_id; repeated seeds/variants are nested and do not increase inferential cluster count",
        "candidate_plans": reports,
        "sensitivity_grid": sensitivity,
        "recommendation": {
            "minimum_confirmatory_scenario_floor": params["minimum_confirmatory_scenarios"],
            "required_scenario_count": max(x["recommended_confirmatory_scenario_clusters"] for x in reports.values()),
            "safety_margin_fraction": params["safety_margin_fraction"],
            "selection_rule": "maximum power-derived candidate requirement, after safety margin, and preregistered minimum floor",
        },
        "assumptions_and_limits": [
            "Screening pilot clusters are representative of confirmatory scenario-cluster variance.",
            "Normal approximation assumes independent scenario clusters and uses a pilot-estimated paired effect SD.",
            "Gaussian power simulation is a planning diagnostic and is not the confirmatory cluster bootstrap.",
            "A fixed screening pilot does not guarantee confirmatory representativeness or eliminate distribution shift.",
            "Confirmatory outcomes are not read by this planner and must remain sealed until study completion.",
        ],
        "claim_ceiling": "Pre-confirmatory sample-size planning only. This report does not establish benchmark power empirically, population representativeness, safety or superiority.",
    } | {"report_sha256": ""}


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", type=Path, default=DEFAULT_PLAN)
    parser.add_argument("--preregistration", type=Path, default=DEFAULT_PREREG)
    parser.add_argument("--output", default="-")
    args = parser.parse_args(argv)
    try:
        plan_bytes = args.plan.read_bytes()
        prereg_bytes = args.preregistration.read_bytes()
        plan = load_json(plan_bytes, "power plan")
        prereg = load_json(prereg_bytes, "preregistration")
        report = plan_power(plan, plan_bytes, prereg, prereg_bytes)
        report["report_sha256"] = sha256(canonical({k: v for k, v in report.items() if k != "report_sha256"}))
        rendered = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
        if args.output == "-":
            sys.stdout.write(rendered)
        else:
            args.output.write_text(rendered, encoding="utf-8")
        return 0
    except (OSError, PlanningError, AttributeError, KeyError, TypeError, ValueError, OverflowError, RecursionError) as exc:
        print(f"opsbench-power-plan: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
