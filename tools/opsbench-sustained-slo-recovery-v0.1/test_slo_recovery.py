import copy
import hashlib
import json
import unittest
from pathlib import Path

from evaluate_slo_recovery import ContractError, evaluate

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "benchmarks" / "opsbench-sustained-slo-recovery-v0.1"


def fixtures():
    contract_bytes = (BASE / "contract.example.json").read_bytes()
    observations_bytes = (BASE / "windows.example.json").read_bytes()
    return json.loads(contract_bytes), contract_bytes, json.loads(observations_bytes), observations_bytes


def serialize(value):
    return (json.dumps(value, indent=2) + "\n").encode()


class SustainedSloRecoveryTests(unittest.TestCase):
    def test_example_requires_five_consecutive_windows_after_regression(self):
        contract, contract_bytes, observations, observations_bytes = fixtures()
        report = evaluate(contract, contract_bytes, observations, observations_bytes)
        self.assertEqual(report["status"], "example_only")
        recovery = report["sustained_recovery"]
        self.assertTrue(recovery["verified"])
        self.assertEqual(recovery["candidate_window_id"], "w04")
        self.assertEqual(recovery["sustained_from"], "2026-10-31T14:04:00Z")
        self.assertEqual(recovery["sustained_until"], "2026-10-31T14:09:00Z")
        self.assertEqual(recovery["time_to_recovery_candidate_seconds"], 240)
        self.assertEqual(recovery["time_to_sustained_recovery_seconds"], 540)
        self.assertEqual(recovery["regression_count"], 1)

    def test_latency_regression_breaks_the_hold_sequence(self):
        contract, contract_bytes, observations, observations_bytes = fixtures()
        window = next(row for row in observations["windows"] if row["window_id"] == "w03")
        report = evaluate(contract, contract_bytes, observations, observations_bytes)
        statuses = {row["window_id"]: row["status"] for row in report["sustained_recovery"]["window_results"]}
        self.assertEqual(statuses["w03"], "fail")
        self.assertTrue(report["sustained_recovery"]["verified"])
        self.assertEqual(report["sustained_recovery"]["regression_count"], 1)

    def test_short_observation_horizon_cannot_prove_sustained_recovery(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["windows"] = observations["windows"][1:3]
        report = evaluate(contract, contract_bytes, observations, serialize(observations))
        self.assertEqual(report["status"], "indeterminate")
        self.assertFalse(report["sustained_recovery"]["verified"])

    def test_missing_window_is_indeterminate_and_resets_hold(self):
        contract, _contract_bytes, observations, _ = fixtures()
        contract["timeline"]["evaluation_started_at"] = "2026-10-31T14:04:00Z"
        observations["windows"] = [w for w in observations["windows"][4:] if w["window_id"] != "w06"]
        contract_bytes = serialize(contract)
        observations["contract_sha256"] = hashlib.sha256(contract_bytes).hexdigest()
        report = evaluate(contract, contract_bytes, observations, serialize(observations))
        self.assertEqual(report["status"], "indeterminate")
        self.assertEqual(report["sustained_recovery"]["indeterminate_window_count"], 1)
        self.assertFalse(report["sustained_recovery"]["verified"])

    def test_low_request_volume_is_indeterminate_not_healthy(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["windows"][4]["request_count"] = 0
        observations["windows"][4]["successful_request_count"] = 0
        observations["windows"][4]["latency_histogram"] = None
        report = evaluate(contract, contract_bytes, observations, serialize(observations))
        w = next(row for row in report["sustained_recovery"]["window_results"] if row["window_id"] == "w04")
        self.assertEqual(w["status"], "indeterminate")
        self.assertFalse(report["sustained_recovery"]["verified"])

    def test_http_200_like_success_does_not_hide_business_correctness_failure(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["windows"][4]["correct_business_check_count"] = 900
        report = evaluate(contract, contract_bytes, observations, serialize(observations))
        w = next(row for row in report["sustained_recovery"]["window_results"] if row["window_id"] == "w04")
        self.assertEqual(w["status"], "fail")
        correctness = next(x for x in w["sli_results"] if x["kind"] == "business_correctness_ratio")
        self.assertEqual(correctness["disposition"], "fail")

    def test_tail_latency_is_non_compensatory(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["windows"][4]["latency_histogram"] = {
            "unit": "s",
            "buckets": [
                {"upper_bound_seconds": 0.05, "count": 800},
                {"upper_bound_seconds": 0.1, "count": 100},
                {"upper_bound_seconds": 0.25, "count": 90},
                {"upper_bound_seconds": 0.5, "count": 10},
                {"upper_bound_seconds": None, "count": 0},
            ],
        }
        report = evaluate(contract, contract_bytes, observations, serialize(observations))
        w = next(row for row in report["sustained_recovery"]["window_results"] if row["window_id"] == "w04")
        latency = next(x for x in w["sli_results"] if x["kind"] == "latency_quantile")
        self.assertEqual(latency["disposition"], "fail")
        self.assertFalse(report["sustained_recovery"]["verified"])

    def test_zero_traffic_never_passes_success_ratio(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["windows"][4]["request_count"] = 0
        observations["windows"][4]["successful_request_count"] = 0
        observations["windows"][4]["latency_histogram"] = None
        report = evaluate(contract, contract_bytes, observations, serialize(observations))
        w = next(row for row in report["sustained_recovery"]["window_results"] if row["window_id"] == "w04")
        self.assertEqual(w["status"], "indeterminate")

    def test_goodput_collapse_fails_even_if_requests_are_successful(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["windows"][4]["goodput_units"] = 10
        report = evaluate(contract, contract_bytes, observations, serialize(observations))
        w = report["sustained_recovery"]["window_results"][4]
        goodput = next(x for x in w["sli_results"] if x["kind"] == "goodput_minimum")
        self.assertEqual(goodput["disposition"], "fail")

    def test_saturation_regression_fails(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["windows"][4]["saturation_ratio"] = 0.99
        report = evaluate(contract, contract_bytes, observations, serialize(observations))
        w = report["sustained_recovery"]["window_results"][4]
        saturation = next(x for x in w["sli_results"] if x["kind"] == "saturation_maximum")
        self.assertEqual(saturation["disposition"], "fail")

    def test_synthetic_probe_failure_is_independent_sli(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["windows"][4]["successful_synthetic_probe_count"] = 15
        report = evaluate(contract, contract_bytes, observations, serialize(observations))
        w = report["sustained_recovery"]["window_results"][4]
        probe = next(x for x in w["sli_results"] if x["kind"] == "synthetic_probe_success_ratio")
        self.assertEqual(probe["disposition"], "fail")

    def test_missing_saturation_metric_is_indeterminate(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["windows"][4]["saturation_ratio"] = None
        report = evaluate(contract, contract_bytes, observations, serialize(observations))
        w = report["sustained_recovery"]["window_results"][4]
        self.assertEqual(w["status"], "indeterminate")

    def test_observer_trust_domain_substitution_rejected(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["windows"][0]["observer_trust_domain_id"] = contract["executor"]["trust_domain_id"]
        with self.assertRaisesRegex(ContractError, "observer identity does not match frozen contract"):
            evaluate(contract, contract_bytes, observations, serialize(observations))

    def test_business_oracle_identity_substitution_rejected(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["windows"][0]["business_oracle_source_id"] = "platform-coordinator"
        with self.assertRaisesRegex(ContractError, "business oracle identity does not match frozen contract"):
            evaluate(contract, contract_bytes, observations, serialize(observations))

    def test_contract_digest_mismatch_rejected(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["contract_sha256"] = "0" * 64
        with self.assertRaisesRegex(ContractError, "exact service contract digest mismatch"):
            evaluate(contract, contract_bytes, observations, serialize(observations))

    def test_histogram_total_must_match_request_count(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["windows"][4]["latency_histogram"]["buckets"][0]["count"] += 1
        with self.assertRaisesRegex(ContractError, "histogram count must equal request_count"):
            evaluate(contract, contract_bytes, observations, serialize(observations))

    def test_non_contiguous_window_grid_not_interpolated(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["windows"][1]["started_at"] = "2026-10-31T14:01:30Z"
        observations["windows"][1]["ended_at"] = "2026-10-31T14:02:30Z"
        with self.assertRaisesRegex(ContractError, "start is not aligned to window grid"):
            evaluate(contract, contract_bytes, observations, serialize(observations))

    def test_duplicate_time_slot_rejected(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["windows"].insert(1, copy.deepcopy(observations["windows"][0]))
        with self.assertRaisesRegex(ContractError, "duplicate time slot"):
            evaluate(contract, contract_bytes, observations, serialize(observations))

    def test_contract_requires_distinct_executor_observer_and_oracle_domains(self):
        contract, contract_bytes, observations, observations_bytes = fixtures()
        contract["business_oracle"]["trust_domain_id"] = contract["observer"]["trust_domain_id"]
        with self.assertRaisesRegex(ContractError, "three distinct trust domains"):
            evaluate(contract, serialize(contract), observations, observations_bytes)


    def test_boolean_gap_value_cannot_masquerade_as_zero(self):
        contract, _contract_bytes, observations, observations_bytes = fixtures()
        contract["timeline"]["maximum_gap_seconds"] = False
        with self.assertRaisesRegex(ContractError, "zero permitted observation gap"):
            evaluate(contract, serialize(contract), observations, observations_bytes)

    def test_observation_lifecycle_must_match_contract_status(self):
        contract, contract_bytes, observations, _ = fixtures()
        observations["status"] = "captured"
        with self.assertRaisesRegex(ContractError, "status does not match frozen contract lifecycle"):
            evaluate(contract, contract_bytes, observations, serialize(observations))


if __name__ == "__main__":
    unittest.main()
