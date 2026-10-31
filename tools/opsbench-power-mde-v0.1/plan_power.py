#!/usr/bin/env python3
"""Screening-only paired scenario-cluster power and MDE planner.

Planning estimates do not execute agents, inspect confirmatory outcomes, or
establish external population representativeness.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_PILOT = ROOT / "benchmarks" / "opsbench-power-mde-v0.1" / "pilot.example.json"
DEFAULT_PREREG = ROOT / "benchmarks" / "opsbench-common-core-v0.1" / "preregistration.example.json"
MAX_BYTES = 4 *  1024 * 1024
MAX_CLUSTERS = 100_000
VARIANCE_FLOOR = 0.05


class PlanningError(ValueError):
    pass


def require(condition: bool, message: str):
    if not condition:
        raise PlanningError(message)


def strict_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise PlanningError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def reject_constant(value):
    raise PlanningError(f"non-finite JSON constant: {value}")


def load_json(path: Path, label: str) -> tuple[dict, bytes]:
    data = path.read_bytes()
    require(len(data) <= MAX_BYTES, f"{label}: exceeds size limit")
    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=strict_object, parse_constant=reject_constant)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise PlanningError(f"{label}: invalid UTF-8 JSON: {exc}") from exc
    require(isinstance(value, dict), f"{label}: top-level object required")
    return value, data


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def normal_cdf(x: float) -> float:
    return 0.5 * (1.0 + math.erf(x / math.sqrt(2.0)))


def normal_ppf(p: float) -> float:
    """Acklam inverse-normal approximation, with finite domain checks."""
    require(0.0 < p < 1.0, "normal quantile probability must be inside (0,1)")
    a = (-3.969683028665376e1, 2.209460984245205e2, -2.759285104469687e2,
         1.383577518672690e2, -3.066479806614716e1, 2.506628277459239)
    b = (-5.447609879822406e1, 1.615858368580409e2, -1.556989798598866e2,
         6.680131188771972e1, -1.328068155288572e1)
    c = (-7.784894002430293e-3, -3.223964580411365e-1, -2.400758277161838,
         -2.549732539343734, 4.374664141464968, 2.938163982698783)
    d = (7.784695709041462e-3, 3.224671290700398e-1, 2.445134137142996, 3.754408661907416)
    plow = 0.02425
    phigh = 1.0 - plow
    if p < plow:
        q = math.sqrt(-2.0 * math.log(p))
        return (((((c[0]*q+c[1])*q+c[2])*q+c[3])*q+c[4])*q+c[5]) / ((((d[0]*q+d[1])*q+d[2])*q+d[3])*q+1.0)
    if p > phigh:
        q = math.sqrt(-2.0 * math.log(1.0-p))
        return -(((((c[0]*q+c[1])*q+c[2])*q+c[3])*q+c[4])*q+c[5]) / ((((d[0]*q+d[1])*q+d[2])*q+d[3])*q+1.0)
    q = p - 0.5
    r = q*q
    return (((((a[0]*r+a[1])*r+a[2])*r+a[3])*r+a[4])*r+a[5])*q / (((((b[0]*r+b[1])*r+b[2])*r+b[3])*r+b[4])*r+1.0)


def mean(values):
    return sum(values) / len(values)


def sample_sd(values):
    if len(values) < 2:
        return 0.0
    m = mean(values)
    return math.sqrt(sum((x-m)**2 for x in values) / (len(values)-1))


def quantile(values, probability):
    ordered = sorted(values)
    position = (len(ordered) - 1) * probability
    low = math.floor(position)
    high = math.ceil(position)
    if low == high:
        return ordered[low]
    weight = position - low
    return ordered[low] * (1.0 - weight) + ordered[high] * weight


def required_n(sd: float, effect: float, power: float, family_alpha: float, comparisons: int) -> int:
    adjusted_alpha = family_alpha / comparisons
    zcrit = normal_ppf(1.0 - adjusted_alpha / 2.0)
    zpower = normal_ppf(power)
    return max(2, math.ceil(((zcrit + zpower) * sd / effect) ** 2))


def simulate_power(sd: float, target_effect: float, n: int, power: float, alpha: float,
                   comparisons: int, replicates: int, seed: int) -> float:
    rng = random.Random(seed)
    critical = normal_ppf(1.0 - (alpha / comparisons) / 2.0)
    successes = 0
    # Resample centered screening-cluster residuals, preserving their empirical
    # shape. A zero-variance pilot uses a disclosed symmetric floor residual.
    residuals = getattr(simulate_power, "_pilot_residuals", None)
    if not residuals:
        residuals = [-sd, sd]
    for _ in range(replicates):
        values = [target_effect + residuals[rng.randrange(len(residuals))] for _ in range(n)]
        statistic = mean(values) / (sd / math.sqrt(n))
        if abs(statistic) >= critical:
            successes += 1
    return successes / replicates


def validate(pilot: dict, prereg: dict, prereg_bytes: bytes) -> list[float]:
    require(pilot.get("schema") == "luminous-opsbench-power-pilot-v0.1" and type(pilot.get("version")) is int and pilot["version"] == 1, "pilot: unsupported schema/version")
    require(pilot.get("status") in {"example_only", "screening"}, "planner accepts screening/example-only pilot data only; confirmatory input is prohibited")
    require(pilot.get("pilot_split") == "screening", "pilot must use the preregistered screening split")
    require(pilot.get("study_id") == prereg.get("study_id"), "pilot study_id does not match preregistration")
    require(pilot.get("preregistration_sha256") == digest(prereg_bytes), "pilot preregistration digest mismatch")
    require(prereg.get("schema") == "luminous-opsbench-common-core-preregistration-v0.1" and prereg.get("version") == 1, "preregistration schema/version unsupported")
    split = prereg.get("scenario_split", {})
    expected = split.get("screening_scenario_ids", [])
    development = set(split.get("development_scenario_ids", []))
    require(isinstance(expected, list) and len(expected) >= 2 and len(expected) == len(set(expected)), "preregistration screening split is invalid")
    declared = pilot.get("screening_scenario_ids")
    require(isinstance(declared, list) and set(declared) == set(expected) and len(declared) == len(expected), "pilot screening IDs must exactly match the preregistered screening split")
    require(not (set(declared) & development), "development/screening split contamination")
    require(pilot.get("baseline_arm_id") != pilot.get("candidate_arm_id"), "baseline and candidate arms must differ")
    prereg_arms = {a.get("arm_id"): a for a in prereg.get("arms", []) if isinstance(a, dict)}
    require(pilot["baseline_arm_id"] in prereg_arms and pilot["candidate_arm_id"] in prereg_arms, "pilot arms must be preregistered")
    effects = pilot.get("scenario_cluster_effects")
    require(isinstance(effects, list) and 2 <= len(effects) <= MAX_CLUSTERS, "pilot needs at least two paired scenario-cluster effects")
    effect_ids, values = [], []
    for index, row in enumerate(effects):
        require(isinstance(row, dict) and set(row) == {"scenario_id", "paired_difference"}, f"pilot effect {index}: exact fields required")
        scenario = row["scenario_id"]
        value = row["paired_difference"]
        require(scenario in set(expected), f"pilot effect {index}: scenario outside screening split")
        require(type(value) in (int, float) and math.isfinite(value) and -1.0 <= value <= 1.0, f"pilot effect {index}: paired difference must be finite and in [-1,1]")
        effect_ids.append(scenario)
        values.append(float(value))
    require(len(effect_ids) == len(set(effect_ids)), "pilot contains duplicate scenario-cluster effects")
    unmatched_candidate = pilot.get("unmatched_candidate_scenario_ids")
    unmatched_baseline = pilot.get("unmatched_baseline_scenario_ids")
    require(isinstance(unmatched_candidate, list) and isinstance(unmatched_baseline, list), "explicit unmatched scenario ID lists required")
    require(len(unmatched_candidate) == len(set(unmatched_candidate)) and len(unmatched_baseline) == len(set(unmatched_baseline)), "duplicate unmatched scenario IDs")
    require(not (set(unmatched_candidate) & set(effect_ids)) and not (set(unmatched_baseline) & set(effect_ids)), "unmatched clusters cannot also contribute paired effects")
    require(set(effect_ids) | set(unmatched_candidate) | set(unmatched_baseline) == set(expected), "every screening cluster must be paired or explicitly unmatched")
    require(not (set(unmatched_candidate) & set(unmatched_baseline)), "a scenario cannot be unmatched on both arms")
    analysis = pilot.get("analysis", {})
    require(isinstance(analysis, dict), "pilot analysis settings required")
    require(type(analysis.get("target_effect")) in (int, float) and 0 < analysis["target_effect"] <= 1, "target_effect must be in (0,1]")
    require(analysis.get("target_power") in (0.8, 0.9, 0.95), "target_power must be 0.80, 0.90 or 0.95")
    require(analysis.get("familywise_alpha") == 0.05, "familywise alpha is fixed at 0.05")
    require(type(analysis.get("candidate_comparison_count")) is int and 1 <= analysis["candidate_comparison_count"] <= 20, "candidate comparison count outside range")
    require(type(analysis.get("simulation_replicates")) is int and 1000 <= analysis["simulation_replicates"] <= 100000, "simulation replicates outside range")
    require(type(analysis.get("simulation_seed")) is int and 0 <= analysis["simulation_seed"] <= 0xFFFFFFFF, "simulation seed outside range")
    require(type(analysis.get("safety_margin_fraction")) in (int, float) and 0 <= analysis["safety_margin_fraction"] <= 1, "safety margin fraction outside range")
    return values


def plan(pilot: dict, prereg: dict, prereg_bytes: bytes) -> dict:
    values = validate(pilot, prereg, prereg_bytes)
    settings = pilot["analysis"]
    observed_mean = mean(values)
    observed_sd = sample_sd(values)
    variance_degenerate = observed_sd == 0.0
    planning_sd = max(observed_sd, VARIANCE_FLOOR)
    comparisons = settings["candidate_comparison_count"]
    alpha = settings["familywise_alpha"]
    target_effect = float(settings["target_effect"])
    target_power = float(settings["target_power"])
    raw_n = required_n(planning_sd, target_effect, target_power, alpha, comparisons)
    safety_margin = settings["safety_margin_fraction"]
    padded_n = math.ceil(raw_n * (1.0 + safety_margin))
    prereg_floor = int(prereg["scenario_split"]["confirmatory_scenario_count_target"])
    recommended = max(prereg_floor, padded_n)
    residuals = [value - observed_mean for value in values]
    if variance_degenerate:
        residuals = [-VARIANCE_FLOOR, VARIANCE_FLOOR]
    simulate_power._pilot_residuals = residuals
    simulation_power = simulate_power(
        planning_sd, target_effect, recommended, target_power, alpha, comparisons,
        settings["simulation_replicates"], settings["simulation_seed"],
    )
    sensitivity = []
    for mde in (0.05, 0.10, 0.15):
        for power in (0.80, 0.90, 0.95):
            for family_size in (1, 2, 4):
                n = required_n(planning_sd, mde, power, alpha, family_size)
                sensitivity.append({
                    "target_mde": mde, "target_power": power, "candidate_comparison_count": family_size,
                    "bonferroni_per_comparison_alpha": alpha / family_size,
                    "normal_approximation_scenario_count": n,
                })
    unmatched_candidate = pilot["unmatched_candidate_scenario_ids"]
    unmatched_baseline = pilot["unmatched_baseline_scenario_ids"]
    return {
        "schema": "luminous-opsbench-power-plan-v0.1",
        "version": 1,
        "status": "example_only" if pilot["status"] == "example_only" else "screening_plan_not_a_confirmatory_result",
        "study_id": pilot["study_id"],
        "pilot_artifact_id": pilot["pilot_artifact_id"],
        "pilot_sha256": digest(json.dumps(pilot, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()),
        "preregistration_sha256": digest(prereg_bytes),
        "pilot_scenario_cluster_count": len(values),
        "pilot_paired_mean_difference": observed_mean,
        "pilot_paired_sample_sd": observed_sd,
        "pilot_paired_effect_quantiles": {
            "p10": quantile(values, 0.10), "p25": quantile(values, 0.25),
            "p50": quantile(values, 0.50), "p75": quantile(values, 0.75),
            "p90": quantile(values, 0.90)
        },
        "planning_standard_deviation": planning_sd,
        "variance_degenerate_floor_applied": variance_degenerate,
        "unmatched_candidate_scenario_ids": unmatched_candidate,
        "unmatched_baseline_scenario_ids": unmatched_baseline,
        "unmatched_candidate_scenario_count": len(unmatched_candidate),
        "unmatched_baseline_scenario_count": len(unmatched_baseline),
        "target_effect": target_effect,
        "target_power": target_power,
        "familywise_alpha": alpha,
        "candidate_comparison_count": comparisons,
        "bonferroni_per_comparison_alpha": alpha / comparisons,
        "normal_approximation_required_scenario_count": raw_n,
        "safety_margin_fraction": safety_margin,
        "required_after_safety_margin": padded_n,
        "preregistered_confirmatory_floor": prereg_floor,
        "recommended_confirmatory_scenario_count": recommended,
        "simulated_power_at_recommended_count": simulation_power,
        "simulation_replicates": settings["simulation_replicates"],
        "simulation_seed": settings["simulation_seed"],
        "sensitivity": sensitivity,
        "planning_method": "two-sided normal approximation with Bonferroni family-wise alpha; empirical pilot SD with a conservative 5pp SD floor when pilot variance is zero; Gaussian cluster-level power simulation is a separate diagnostic",
        "limits": [
            "This tool accepts only the preregistered screening split and rejects confirmatory inputs.",
            "Scenario IDs, not repeated seeds, are the independent planning units.",
            "Unmatched clusters are reported and excluded from paired effect estimation; they are not converted to zero effects.",
            "Normal approximation and simulated power depend on screening-pilot representativeness and cannot guarantee confirmatory power under distribution shift.",
            "A fixed adversarial benchmark does not establish representative population sampling or a population-wide safety guarantee.",
            "The recommended count is a planning target, not evidence of superiority or safety certification."
        ]
    }


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pilot", type=Path, default=DEFAULT_PILOT)
    parser.add_argument("--preregistration", type=Path, default=DEFAULT_PREREG)
    parser.add_argument("--output", default="-")
    args = parser.parse_args(argv)
    try:
        pilot, _ = load_json(args.pilot, "pilot")
        prereg, prereg_bytes = load_json(args.preregistration, "preregistration")
        report = plan(pilot, prereg, prereg_bytes)
        rendered = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
        if args.output == "-":
            sys.stdout.write(rendered)
        else:
            Path(args.output).write_text(rendered, encoding="utf-8")
        return 0
    except (OSError, PlanningError, TypeError, KeyError, ValueError, OverflowError, RecursionError) as exc:
        print(f"opsbench-power-planner: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
