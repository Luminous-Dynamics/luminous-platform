#!/usr/bin/env python3
"""Risk-aware value-of-information evaluator for synthetic ops diagnosis.

This is an offline benchmark simulator. It does not execute probes, mutate
services, authorize operations, or authenticate evidence publishers.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import random
import sys
from pathlib import Path

MAX_INPUT_BYTES = 32 * 1024 * 1024
MAX_HYPOTHESES = 32
MAX_ACTIONS = 64
MAX_PROBES = 128
MAX_SCENARIOS = 10_000
STRATEGIES = (
    "random",
    "cheapest_first",
    "entropy_per_cost",
    "fixed_runbook",
    "risk_adjusted_voi",
    "oracle_perfect_information",
)


class StudyError(ValueError):
    pass


def _strict_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise StudyError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_constant(value):
    raise StudyError(f"non-finite JSON value is prohibited: {value}")


def load_json(data: bytes, label: str) -> dict:
    if len(data) > MAX_INPUT_BYTES:
        raise StudyError(f"{label}: input exceeds size bound")
    try:
        value = json.loads(
            data.decode("utf-8"),
            object_pairs_hook=_strict_object,
            parse_constant=_reject_constant,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise StudyError(f"{label}: invalid UTF-8 JSON: {exc}") from exc
    if not isinstance(value, dict):
        raise StudyError(f"{label}: top-level object required")
    return value


def canonical(value: object) -> bytes:
    try:
        return json.dumps(
            value, sort_keys=True, separators=(",", ":"),
            ensure_ascii=False, allow_nan=False,
        ).encode("utf-8")
    except (TypeError, ValueError) as exc:
        raise StudyError(f"cannot canonicalize JSON: {exc}") from exc


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def require(condition: bool, message: str):
    if not condition:
        raise StudyError(message)


def finite(value, where: str, minimum=None, maximum=None) -> float:
    require(type(value) in (int, float) and math.isfinite(value), f"{where}: finite number required")
    result = float(value)
    if minimum is not None:
        require(result >= minimum, f"{where}: below minimum")
    if maximum is not None:
        require(result <= maximum, f"{where}: above maximum")
    return result


def validate_study(study: dict) -> dict:
    root = {
        "schema", "version", "status", "study_id", "scope_id", "hypotheses",
        "actions", "probes", "authorized_probe_sources", "policy",
        "fixed_runbook_order", "scenarios",
    }
    require(set(study) == root, "study: unexpected or missing root fields")
    require(study.get("schema") == "luminous-opsbench-active-evidence-study-v0.1"
            and type(study.get("version")) is int and study["version"] == 1,
            "study: unsupported schema/version")
    require(study.get("status") in {"example_only", "registered_study"}, "study: invalid status")
    for key in ("study_id", "scope_id"):
        require(isinstance(study.get(key), str) and study[key], f"study.{key}: required")

    hypothesis_rows = study["hypotheses"]
    require(isinstance(hypothesis_rows, list) and 2 <= len(hypothesis_rows) <= MAX_HYPOTHESES,
            "study.hypotheses outside supported bounds")
    hypotheses = {}
    for row in hypothesis_rows:
        require(isinstance(row, dict) and set(row) == {"hypothesis_id", "family"},
                "hypothesis: unexpected or missing fields")
        hid, family = row["hypothesis_id"], row["family"]
        require(isinstance(hid, str) and hid and hid not in hypotheses, "hypothesis ID duplicate/empty")
        require(isinstance(family, str) and family, f"hypothesis {hid}: family required")
        hypotheses[hid] = family
    hids = set(hypotheses)

    source_rows = study["authorized_probe_sources"]
    require(isinstance(source_rows, list) and source_rows, "authorized_probe_sources must be non-empty")
    authorized = set()
    for row in source_rows:
        require(isinstance(row, dict) and set(row) == {"source_id", "trust_domain_id", "scope_id"},
                "authorized probe source: unexpected or missing fields")
        key = (row["source_id"], row["trust_domain_id"], row["scope_id"])
        require(all(isinstance(v, str) and v for v in key), "authorized probe source identity fields required")
        require(key not in authorized, "duplicate authorized probe source")
        authorized.add(key)

    probe_rows = study["probes"]
    require(isinstance(probe_rows, list) and 2 <= len(probe_rows) <= MAX_PROBES,
            "study.probes outside supported bounds")
    probes = {}
    for row in probe_rows:
        required = {
            "probe_id", "source_id", "trust_domain_id", "correlation_group",
            "read_only", "scope_id", "freshness_ttl_seconds", "cost_units",
            "latency_ms", "privacy_penalty_units", "human_seconds", "likelihoods",
        }
        require(isinstance(row, dict) and set(row) == required, "probe: unexpected or missing fields")
        pid = row["probe_id"]
        require(isinstance(pid, str) and pid and pid not in probes, "probe ID duplicate/empty")
        for key in ("source_id", "trust_domain_id", "correlation_group", "scope_id"):
            require(isinstance(row[key], str) and row[key], f"probe {pid}: {key} required")
        require(row["read_only"] is True, f"probe {pid}: write-capable probes are forbidden in v0.1")
        require(row["scope_id"] == study["scope_id"], f"probe {pid}: scope differs from study scope")
        identity = (row["source_id"], row["trust_domain_id"], row["scope_id"])
        require(identity in authorized, f"probe {pid}: source/trust domain/scope not authorized")
        require(type(row["freshness_ttl_seconds"]) is int and row["freshness_ttl_seconds"] >= 1,
                f"probe {pid}: freshness TTL invalid")
        require(type(row["latency_ms"]) is int and row["latency_ms"] >= 0,
                f"probe {pid}: latency invalid")
        finite(row["cost_units"], f"probe {pid}.cost_units", minimum=0.0)
        finite(row["privacy_penalty_units"], f"probe {pid}.privacy_penalty_units", minimum=0.0)
        finite(row["human_seconds"], f"probe {pid}.human_seconds", minimum=0.0)
        likelihoods = row["likelihoods"]
        require(isinstance(likelihoods, dict) and set(likelihoods) == hids,
                f"probe {pid}: likelihood matrix must cover every hypothesis exactly")
        for hid, outcomes in likelihoods.items():
            require(isinstance(outcomes, dict) and set(outcomes) == {"signal", "clear"},
                    f"probe {pid}/{hid}: exact signal/clear likelihood required")
            signal = finite(outcomes["signal"], f"probe {pid}/{hid}.signal", minimum=0.0, maximum=1.0)
            clear = finite(outcomes["clear"], f"probe {pid}/{hid}.clear", minimum=0.0, maximum=1.0)
            require(abs(signal + clear - 1.0) <= 1e-9,
                    f"probe {pid}/{hid}: likelihoods must sum to one")
        probes[pid] = row

    actions = {}
    for row in study["actions"]:
        required = {"action_id", "target_hypothesis", "mutating", "loss_by_hypothesis"}
        require(isinstance(row, dict) and set(row) == required, "action: unexpected or missing fields")
        aid = row["action_id"]
        require(isinstance(aid, str) and aid and aid not in actions, "action ID duplicate/empty")
        require(type(row["mutating"]) is bool, f"action {aid}: mutating must be boolean")
        target = row["target_hypothesis"]
        if row["mutating"]:
            require(target in hids, f"action {aid}: mutating action must target a known hypothesis")
        else:
            require(target is None, f"action {aid}: non-mutating action must have null target")
        losses = row["loss_by_hypothesis"]
        require(isinstance(losses, dict) and set(losses) == hids,
                f"action {aid}: loss matrix must cover every hypothesis")
        for hid, loss in losses.items():
            finite(loss, f"action {aid}/{hid} loss", minimum=0.0, maximum=1.0)
        actions[aid] = row
    mutating_targets = {action["target_hypothesis"] for action in actions.values() if action["mutating"]}
    require(mutating_targets == hids, "study must declare at least one mutating response for every hypothesis")
    require(any(not action["mutating"] for action in actions.values()),
            "study requires a non-mutating escalation/abstention option")

    policy = study["policy"]
    policy_keys = {
        "max_action_risk", "max_cost_units", "max_latency_ms",
        "max_privacy_penalty_units", "max_human_seconds", "max_probes", "random_seed",
    }
    require(isinstance(policy, dict) and set(policy) == policy_keys,
            "policy: unexpected or missing fields")
    finite(policy["max_action_risk"], "policy.max_action_risk", minimum=0.0, maximum=1.0)
    for key in ("max_cost_units", "max_privacy_penalty_units", "max_human_seconds"):
        finite(policy[key], f"policy.{key}", minimum=0.0)
    require(type(policy["max_latency_ms"]) is int and policy["max_latency_ms"] >= 0,
            "policy.max_latency_ms invalid")
    require(type(policy["max_probes"]) is int and 1 <= policy["max_probes"] <= MAX_PROBES,
            "policy.max_probes invalid")
    require(type(policy["random_seed"]) is int and 0 <= policy["random_seed"] <= 0xFFFFFFFF,
            "policy.random_seed invalid")

    runbook = study["fixed_runbook_order"]
    require(isinstance(runbook, list) and set(runbook) == set(probes)
            and len(runbook) == len(probes),
            "fixed_runbook_order must be a permutation of all probe IDs")

    scenarios = study["scenarios"]
    require(isinstance(scenarios, list) and 2 <= len(scenarios) <= MAX_SCENARIOS,
            "study.scenarios outside supported bounds")
    scenario_ids = set()
    for scenario in scenarios:
        required = {"scenario_id", "truth_hypothesis", "prior", "outcome_deck"}
        require(isinstance(scenario, dict) and set(scenario) == required,
                "scenario: unexpected or missing fields")
        sid, truth = scenario["scenario_id"], scenario["truth_hypothesis"]
        require(isinstance(sid, str) and sid and sid not in scenario_ids, "scenario ID duplicate/empty")
        scenario_ids.add(sid)
        require(truth in hids, f"scenario {sid}: unknown truth hypothesis")
        prior = scenario["prior"]
        require(isinstance(prior, dict) and set(prior) == hids, f"scenario {sid}: prior must cover every hypothesis")
        for hid, probability in prior.items():
            finite(probability, f"scenario {sid} prior/{hid}", minimum=0.0, maximum=1.0)
        require(abs(sum(prior.values()) - 1.0) <= 1e-9, f"scenario {sid}: prior probabilities must sum to one")
        deck = scenario["outcome_deck"]
        require(isinstance(deck, list) and len(deck) == len(probes),
                f"scenario {sid}: outcome deck must contain one entry per probe")
        deck_ids = set()
        for observation in deck:
            required_observation = {
                "probe_id", "evidence_id", "source_id", "trust_domain_id", "outcome", "age_seconds",
            }
            require(isinstance(observation, dict) and set(observation) == required_observation,
                    f"scenario {sid}: malformed outcome-deck entry")
            pid = observation["probe_id"]
            require(pid in probes and pid not in deck_ids, f"scenario {sid}: unknown/duplicate probe in outcome deck")
            deck_ids.add(pid)
            probe = probes[pid]
            require(observation["source_id"] == probe["source_id"]
                    and observation["trust_domain_id"] == probe["trust_domain_id"],
                    f"scenario {sid}/{pid}: oracle outcome publisher identity mismatch")
            require(isinstance(observation["evidence_id"], str) and observation["evidence_id"],
                    f"scenario {sid}/{pid}: evidence ID required")
            require(observation["outcome"] in {"signal", "clear"},
                    f"scenario {sid}/{pid}: unknown observation outcome")
            require(type(observation["age_seconds"]) is int and observation["age_seconds"] >= 0,
                    f"scenario {sid}/{pid}: age_seconds invalid")
    return {"hypotheses": list(hypotheses), "actions": actions, "probes": probes, "scenarios": scenarios}


def normalize(distribution: dict[str, float]) -> dict[str, float]:
    total = sum(distribution.values())
    if total <= 0 or not math.isfinite(total):
        raise StudyError("posterior normalization has zero/non-finite mass")
    return {key: value / total for key, value in distribution.items()}


def posterior(prior: dict[str, float], likelihoods: dict, outcome: str) -> dict[str, float]:
    weighted = {
        hypothesis: prior[hypothesis] * likelihoods[hypothesis][outcome]
        for hypothesis in prior
    }
    return normalize(weighted)


def bayes_risk(distribution: dict[str, float], actions: dict) -> tuple[float, str]:
    expected = {
        action_id: sum(distribution[h] * action["loss_by_hypothesis"][h] for h in distribution)
        for action_id, action in actions.items()
    }
    action_id = min(expected, key=lambda name: (expected[name], name))
    return expected[action_id], action_id


def entropy(distribution: dict[str, float]) -> float:
    return -sum(p * math.log2(p) for p in distribution.values() if p > 0.0)


def constrained_decision_risk(distribution: dict[str, float], actions: dict, max_action_risk: float) -> float:
    mutating = {aid: action for aid, action in actions.items() if action["mutating"]}
    safe = {aid: action for aid, action in actions.items() if not action["mutating"]}
    mutating_risk, _ = bayes_risk(distribution, mutating)
    if mutating_risk <= max_action_risk:
        return mutating_risk
    safe_risk, _ = bayes_risk(distribution, safe)
    return safe_risk


def probe_information(distribution: dict[str, float], probe: dict, actions: dict, max_action_risk: float) -> dict:
    current_risk = constrained_decision_risk(distribution, actions, max_action_risk)
    current_entropy = entropy(distribution)
    expected_risk = 0.0
    expected_entropy = 0.0
    outcome_probabilities = {}
    for outcome in ("signal", "clear"):
        probability = sum(
            distribution[h] * probe["likelihoods"][h][outcome]
            for h in distribution
        )
        outcome_probabilities[outcome] = probability
        if probability > 0:
            updated = posterior(distribution, probe["likelihoods"], outcome)
            risk = constrained_decision_risk(updated, actions, max_action_risk)
            expected_risk += probability * risk
            expected_entropy += probability * entropy(updated)
    evsi = max(0.0, current_risk - expected_risk)
    entropy_reduction = max(0.0, current_entropy - expected_entropy)
    net_value = evsi - probe["cost_units"] - probe["privacy_penalty_units"]
    return {
        "current_bayes_risk": current_risk,
        "expected_posterior_bayes_risk": expected_risk,
        "evsi": evsi,
        "entropy_reduction": entropy_reduction,
        "net_value": net_value,
        "outcome_probabilities": outcome_probabilities,
    }


def _best_mutating_action(distribution: dict[str, float], actions: dict) -> tuple[float, str | None]:
    candidates = {
        aid: action for aid, action in actions.items()
        if action["mutating"]
    }
    if not candidates:
        return math.inf, None
    risk, aid = bayes_risk(distribution, candidates)
    return risk, aid


def _safe_nonmutating_action(distribution: dict[str, float], actions: dict) -> tuple[float, str]:
    candidates = {
        aid: action for aid, action in actions.items()
        if not action["mutating"]
    }
    if not candidates:
        raise StudyError("no non-mutating escalation/abstention action available")
    return bayes_risk(distribution, candidates)


def _feasible(probe: dict, spent: dict, policy: dict, count: int) -> bool:
    if count >= policy["max_probes"]:
        return False
    return (
        spent["cost_units"] + probe["cost_units"] <= policy["max_cost_units"] + 1e-12
        and spent["latency_ms"] + probe["latency_ms"] <= policy["max_latency_ms"]
        and spent["privacy_penalty_units"] + probe["privacy_penalty_units"] <= policy["max_privacy_penalty_units"] + 1e-12
        and spent["human_seconds"] + probe["human_seconds"] <= policy["max_human_seconds"] + 1e-12
    )


def _strategy_choice(
    strategy: str,
    candidates: list[str],
    distribution: dict[str, float],
    probes: dict,
    actions: dict,
    runbook: list[str],
    rng: random.Random,
    max_action_risk: float,
) -> str | None:
    if not candidates:
        return None
    if strategy == "random":
        return candidates[rng.randrange(len(candidates))]
    if strategy == "cheapest_first":
        return min(candidates, key=lambda pid: (
            probes[pid]["cost_units"] + probes[pid]["privacy_penalty_units"],
            probes[pid]["latency_ms"], pid,
        ))
    if strategy == "entropy_per_cost":
        def score(pid):
            info = probe_information(distribution, probes[pid], actions, max_action_risk)
            denom = probes[pid]["cost_units"] + probes[pid]["privacy_penalty_units"]
            return (info["entropy_reduction"] / max(denom, 1e-12), info["entropy_reduction"], -probes[pid]["latency_ms"], pid)
        return max(candidates, key=score)
    if strategy == "fixed_runbook":
        for pid in runbook:
            if pid in candidates:
                return pid
        return None
    if strategy == "risk_adjusted_voi":
        ranked = [
            (probe_information(distribution, probes[pid], actions, max_action_risk)["net_value"], pid)
            for pid in candidates
        ]
        value, pid = max(ranked, key=lambda item: (item[0], item[1]))
        return pid if value > 0.0 else None
    raise StudyError(f"unknown strategy: {strategy}")


def _argmax(distribution: dict[str, float]) -> tuple[str, float]:
    key = max(distribution, key=lambda h: (distribution[h], h))
    return key, distribution[key]


def _brier(distribution: dict[str, float], truth: str) -> float:
    return sum((distribution[h] - (1.0 if h == truth else 0.0)) ** 2 for h in distribution)


def _calibration(rows: list[dict], bins: int = 10) -> dict:
    buckets = [{"lower": i / bins, "upper": (i + 1) / bins, "count": 0, "confidence_sum": 0.0, "accuracy_sum": 0.0} for i in range(bins)]
    for row in rows:
        index = min(bins - 1, int(row["confidence"] * bins))
        bucket = buckets[index]
        bucket["count"] += 1
        bucket["confidence_sum"] += row["confidence"]
        bucket["accuracy_sum"] += row["correct"]
    total = len(rows)
    ece = 0.0
    for bucket in buckets:
        count = bucket["count"]
        bucket["mean_confidence"] = bucket["confidence_sum"] / count if count else None
        bucket["empirical_accuracy"] = bucket["accuracy_sum"] / count if count else None
        if count:
            ece += count / total * abs(bucket["mean_confidence"] - bucket["empirical_accuracy"])
    return {"expected_calibration_error": ece if total else None, "bins": buckets}


def _run_policy(study: dict, validated: dict, scenario: dict, strategy: str, strategy_index: int) -> dict:
    hypotheses = validated["hypotheses"]
    probes = validated["probes"]
    actions = validated["actions"]
    policy = study["policy"]
    distribution = normalize(dict(scenario["prior"]))
    deck = {row["probe_id"]: row for row in scenario["outcome_deck"]}
    rng = random.Random(policy["random_seed"] + strategy_index * 1_000_003 + sum(ord(c) for c in scenario["scenario_id"]))
    spent = {"cost_units": 0.0, "latency_ms": 0, "privacy_penalty_units": 0.0, "human_seconds": 0.0}
    queried, correlation_groups, evidence_ids = set(), set(), set()
    observations = []
    prequential = []
    predicted_gain_values, realized_gain_values = [], []
    initial_risk = constrained_decision_risk(distribution, actions, policy["max_action_risk"])
    initial_entropy = entropy(distribution)
    stop_reason = "no_probe_selected"
    final_action, final_risk = None, None

    def current_decision():
        risk, action_id = _best_mutating_action(distribution, actions)
        if action_id is not None and risk <= policy["max_action_risk"]:
            return action_id, risk, True
        safe_risk, safe_action = _safe_nonmutating_action(distribution, actions)
        return safe_action, safe_risk, False

    while True:
        action, action_risk, direct = current_decision()
        if direct:
            final_action, final_risk = action, action_risk
            stop_reason = "mutating_action_risk_below_threshold"
            break
        candidates = [
            pid for pid, probe in probes.items()
            if pid not in queried
            and probe["correlation_group"] not in correlation_groups
            and _feasible(probe, spent, policy, len(queried))
        ]
        choice = _strategy_choice(
            strategy, candidates, distribution, probes, actions,
            study["fixed_runbook_order"], rng, policy["max_action_risk"],
        )
        if choice is None:
            final_action, final_risk = action, action_risk
            stop_reason = "non_positive_net_voi" if strategy == "risk_adjusted_voi" and candidates else "budget_or_probe_exhausted"
            break

        probe = probes[choice]
        queried.add(choice)
        correlation_groups.add(probe["correlation_group"])
        spent["cost_units"] += probe["cost_units"]
        spent["latency_ms"] += probe["latency_ms"]
        spent["privacy_penalty_units"] += probe["privacy_penalty_units"]
        spent["human_seconds"] += probe["human_seconds"]
        deck_entry = deck[choice]
        before = dict(distribution)
        before_risk = constrained_decision_risk(before, actions, policy["max_action_risk"])
        predicted = probe_information(before, probe, actions, policy["max_action_risk"])
        predicted_gain_values.append(predicted["evsi"])

        stale = deck_entry["age_seconds"] > probe["freshness_ttl_seconds"]
        duplicate = deck_entry["evidence_id"] in evidence_ids
        evidence_ids.add(deck_entry["evidence_id"])
        updated = False
        impossible = False
        outcome = deck_entry["outcome"]
        outcome_mass = sum(
            distribution[h] * probe["likelihoods"][h][outcome]
            for h in distribution
        )
        if not stale and not duplicate and outcome_mass > 1e-15:
            distribution = posterior(distribution, probe["likelihoods"], outcome)
            updated = True
        elif not stale and not duplicate:
            # An observation assigned zero probability by the model is a
            # model/data contradiction. Preserve uncertainty and expose it.
            impossible = True
        after_risk = constrained_decision_risk(distribution, actions, policy["max_action_risk"])
        realized_gain_values.append(max(0.0, before_risk - after_risk))
        top_hypothesis, confidence = _argmax(distribution)
        prequential.append({
            "confidence": confidence,
            "correct": top_hypothesis == scenario["truth_hypothesis"],
        })
        observations.append({
            "probe_id": choice,
            "evidence_id": deck_entry["evidence_id"],
            "outcome": outcome if updated else "unavailable",
            "stale": stale,
            "duplicate": duplicate,
            "impossible_under_model": impossible,
            "posterior_updated": updated,
            "predicted_evsi": predicted["evsi"],
            "predicted_entropy_reduction": predicted["entropy_reduction"],
            "realized_bayes_risk_reduction": max(0.0, before_risk - after_risk),
            "posterior_after": dict(distribution),
        })
        stop_reason = "continue"
        # Loop checks the direct-action threshold and remaining budget before another probe.

    top_hypothesis, confidence = _argmax(distribution)
    final_action_def = actions[final_action]
    truth = scenario["truth_hypothesis"]
    realized_loss = final_action_def["loss_by_hypothesis"][truth]
    unsafe = bool(final_action_def["mutating"] and final_action_def["target_hypothesis"] != truth)
    return {
        "scenario_id": scenario["scenario_id"],
        "strategy": strategy,
        "action_id": final_action,
        "action_mutating": final_action_def["mutating"],
        "action_target": final_action_def["target_hypothesis"],
        "unsafe_recommendation": unsafe,
        "expected_action_risk": final_risk,
        "realized_decision_loss": realized_loss,
        "direct_recommendation": final_action_def["mutating"],
        "escalated": final_action == "escalate",
        "abstained": final_action == "abstain",
        "probe_count": len(queried),
        "cost_units": spent["cost_units"],
        "latency_ms": spent["latency_ms"],
        "privacy_penalty_units": spent["privacy_penalty_units"],
        "human_seconds": spent["human_seconds"],
        "initial_bayes_risk": initial_risk,
        "final_bayes_risk": final_risk,
        "total_realized_risk_reduction": max(0.0, initial_risk - final_risk),
        "mean_predicted_evsi_per_probe": sum(predicted_gain_values) / len(predicted_gain_values) if predicted_gain_values else 0.0,
        "mean_realized_risk_reduction_per_probe": sum(realized_gain_values) / len(realized_gain_values) if realized_gain_values else 0.0,
        "mean_predicted_entropy_reduction": sum(x["predicted_entropy_reduction"] for x in observations) / len(observations) if observations else 0.0,
        "duplicate_evidence_count": sum(x["duplicate"] for x in observations),
        "stale_evidence_count": sum(x["stale"] for x in observations),
        "impossible_observation_count": sum(x["impossible_under_model"] for x in observations),
        "final_posterior": distribution,
        "top_hypothesis": top_hypothesis,
        "top_hypothesis_confidence": confidence,
        "multiclass_brier_score": _brier(distribution, truth),
        "calibration_correct": top_hypothesis == truth,
        "observations": observations,
        "stop_reason": stop_reason,
    }


def _oracle_upper_bound(study: dict, scenario: dict) -> dict:
    truth = scenario["truth_hypothesis"]
    action = next(
        a for a in study["actions"]
        if a["mutating"] and a["target_hypothesis"] == truth
    )
    return {
        "scenario_id": scenario["scenario_id"],
        "strategy": "oracle_perfect_information",
        "action_id": action["action_id"],
        "action_mutating": True,
        "action_target": truth,
        "unsafe_recommendation": False,
        "expected_action_risk": 0.0,
        "realized_decision_loss": action["loss_by_hypothesis"][truth],
        "direct_recommendation": True,
        "escalated": False,
        "abstained": False,
        "probe_count": 0,
        "cost_units": 0.0,
        "latency_ms": 0,
        "privacy_penalty_units": 0.0,
        "human_seconds": 0.0,
        "initial_bayes_risk": constrained_decision_risk(
            scenario["prior"], {a["action_id"]: a for a in study["actions"]},
            study["policy"]["max_action_risk"],
        ),
        "final_bayes_risk": 0.0,
        "total_realized_risk_reduction": 0.0,
        "mean_predicted_evsi_per_probe": 0.0,
        "mean_realized_risk_reduction_per_probe": 0.0,
        "mean_predicted_entropy_reduction": 0.0,
        "duplicate_evidence_count": 0,
        "stale_evidence_count": 0,
        "final_posterior": {h: (1.0 if h == truth else 0.0) for h in scenario["prior"]},
        "top_hypothesis": truth,
        "top_hypothesis_confidence": 1.0,
        "multiclass_brier_score": 0.0,
        "calibration_correct": True,
        "observations": [],
        "stop_reason": "evaluator_only_perfect_information_upper_bound",
    }


def _summarize(rows: list[dict]) -> dict:
    n = len(rows)
    calibration = _calibration([
        {"confidence": row["top_hypothesis_confidence"], "correct": row["calibration_correct"]}
        for row in rows
    ])
    return {
        "scenario_count": n,
        "direct_recommendation_count": sum(row["direct_recommendation"] for row in rows),
        "escalation_count": sum(row["escalated"] for row in rows),
        "abstention_count": sum(row["abstained"] for row in rows),
        "unsafe_recommendation_count": sum(row["unsafe_recommendation"] for row in rows),
        "mean_probes": sum(row["probe_count"] for row in rows) / n,
        "mean_cost_units": sum(row["cost_units"] for row in rows) / n,
        "mean_latency_ms": sum(row["latency_ms"] for row in rows) / n,
        "mean_privacy_penalty_units": sum(row["privacy_penalty_units"] for row in rows) / n,
        "mean_human_seconds": sum(row["human_seconds"] for row in rows) / n,
        "mean_final_brier_score": sum(row["multiclass_brier_score"] for row in rows) / n,
        "mean_realized_decision_loss": sum(row["realized_decision_loss"] for row in rows) / n,
        "mean_final_expected_action_risk": sum(row["final_bayes_risk"] for row in rows) / n,
        "mean_predicted_evsi_per_probe": sum(row["mean_predicted_evsi_per_probe"] for row in rows) / n,
        "mean_realized_risk_reduction_per_probe": sum(row["mean_realized_risk_reduction_per_probe"] for row in rows) / n,
        "mean_predicted_entropy_reduction": sum(row["mean_predicted_entropy_reduction"] for row in rows) / n,
        "duplicate_evidence_count": sum(row["duplicate_evidence_count"] for row in rows),
        "stale_evidence_count": sum(row["stale_evidence_count"] for row in rows),
        "impossible_observation_count": sum(row["impossible_observation_count"] for row in rows),
        "calibration": calibration,
    }


def evaluate(study: dict) -> dict:
    validated = validate_study(study)
    strategy_rows = {name: [] for name in STRATEGIES}
    per_scenario = {}
    for scenario_index, scenario in enumerate(validated["scenarios"]):
        per_scenario[scenario["scenario_id"]] = {}
        for strategy_index, strategy in enumerate(STRATEGIES[:-1]):
            result = _run_policy(study, validated, scenario, strategy, strategy_index)
            strategy_rows[strategy].append(result)
            per_scenario[scenario["scenario_id"]][strategy] = result
        oracle = _oracle_upper_bound(study, scenario)
        strategy_rows["oracle_perfect_information"].append(oracle)
        per_scenario[scenario["scenario_id"]]["oracle_perfect_information"] = oracle

    report = {
        "schema": "luminous-opsbench-active-evidence-report-v0.1",
        "version": 1,
        "status": "example_only" if study["status"] == "example_only" else "analysis_only_not_a_policy_qualification",
        "study_id": study["study_id"],
        "scenario_count": len(validated["scenarios"]),
        "strategies": {name: _summarize(rows) for name, rows in strategy_rows.items()},
        "scenario_results": per_scenario,
        "claim_ceiling": (
            "Synthetic decision-theoretic simulator over authored priors, likelihoods, losses and evidence decks only. "
            "It does not establish real probe calibration, source independence, operational diagnosis quality, "
            "production utility, or authority to execute a response. The oracle upper bound sees hidden truth and "
            "is evaluator-only. Example-only data are never confirmatory evidence."
        ),
    }
    report["report_sha256"] = sha256(canonical(report))
    return report


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--study", type=Path, required=True)
    parser.add_argument("--output", default="-")
    args = parser.parse_args(argv)
    try:
        study_bytes = args.study.read_bytes()
        study = load_json(study_bytes, "study")
        report = evaluate(study)
        rendered = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
        if args.output == "-":
            sys.stdout.write(rendered)
        else:
            args.output.write_text(rendered, encoding="utf-8")
        return 0
    except (OSError, StudyError, AttributeError, KeyError, TypeError, ValueError, OverflowError, RecursionError) as exc:
        print(f"opsbench-active-evidence-voi: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
