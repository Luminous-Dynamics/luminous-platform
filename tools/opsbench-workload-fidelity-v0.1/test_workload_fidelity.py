import copy
import hashlib
import json
import unittest
from pathlib import Path

from evaluate_workload_fidelity import ContractError, evaluate, load_json

ROOT = Path(__file__).resolve().parents[2]
BENCH = ROOT / "benchmarks" / "opsbench-workload-fidelity-v0.1"


def fixtures():
    files = {
        "contract": "workload-contract.example.json",
        "profile": "workload-profile.example.json",
        "schedule": "phase-schedule.example.json",
        "load": "load-generator-receipts.example.json",
        "observations": "service-observations.example.json",
        "faults": "fault-controller-receipts.example.json",
    }
    raw = {key: (BENCH / name).read_bytes() for key, name in files.items()}
    docs = {key: load_json(data, key) for key, data in raw.items()}
    return docs, raw


def encode(value):
    return (json.dumps(value, indent=2, sort_keys=False) + "\n").encode()


def rebind(docs):
    """Refresh component cross-bindings and contract digests after an intentional mutation."""
    profile_digest = hashlib.sha256(encode(docs["profile"])).hexdigest()
    schedule_digest = hashlib.sha256(encode(docs["schedule"])).hexdigest()
    docs["load"]["profile_sha256"] = profile_digest
    docs["load"]["phase_schedule_sha256"] = schedule_digest
    docs["observations"]["profile_sha256"] = profile_digest
    docs["observations"]["phase_schedule_sha256"] = schedule_digest
    docs["faults"]["phase_schedule_sha256"] = schedule_digest
    docs["contract"]["profile_sha256"] = profile_digest
    docs["contract"]["phase_schedule_sha256"] = schedule_digest
    docs["contract"]["load_generator_receipts_sha256"] = hashlib.sha256(encode(docs["load"])).hexdigest()
    docs["contract"]["service_observations_sha256"] = hashlib.sha256(encode(docs["observations"])).hexdigest()
    docs["contract"]["fault_controller_receipts_sha256"] = hashlib.sha256(encode(docs["faults"])).hexdigest()


def run(docs):
    rebind(docs)
    raw = {key: encode(value) for key, value in docs.items()}
    return evaluate(
        docs["contract"], raw["contract"],
        docs["profile"], raw["profile"],
        docs["schedule"], raw["schedule"],
        docs["load"], raw["load"],
        docs["observations"], raw["observations"],
        docs["faults"], raw["faults"],
    )


def record(docs, phase_id, class_id, key):
    rows = docs[key]["windows"] if key in {"load", "observations"} else docs["faults"]["receipts"]
    if key == "faults":
        return next(x for x in rows if x["phase_id"] == phase_id)
    return next(x for x in rows if x["phase_id"] == phase_id and x["class_id"] == class_id)


def rebucket(offered):
    counts = [
        int(offered * 0.96),
        int(offered * 0.03),
        offered - int(offered * 0.99),
        0,
        0,
    ]
    return [
        {"upper_bound_seconds": 0.05, "count": counts[0]},
        {"upper_bound_seconds": 0.1, "count": counts[1]},
        {"upper_bound_seconds": 0.2, "count": counts[2]},
        {"upper_bound_seconds": 0.5, "count": counts[3]},
        {"upper_bound_seconds": None, "count": counts[4]},
    ]


class WorkloadFidelityTests(unittest.TestCase):
    def test_example_is_not_qualified_despite_excellent_aggregate_success(self):
        docs, _raw = fixtures()
        report = run(docs)
        self.assertEqual(report["status"], "example_only")
        self.assertEqual(report["computed_disposition"], "fail")
        self.assertGreater(report["overall_aggregate_response_success_ratio"], 0.99)
        self.assertEqual(report["critical_class_failure_count"], 2)
        self.assertFalse(report["publisher_authenticity_verified"])
        self.assertFalse(report["population_capacity_claim_permitted"])
        self.assertFalse(report["automatic_execution_authorized"])
        self.assertEqual(report["total_offered_requests"], 27000)

    def test_critical_class_fails_even_when_routine_class_is_healthy(self):
        docs, _ = fixtures()
        report = run(docs)
        critical = report["per_class"]["critical-payment"]
        routine = report["per_class"]["routine-read"]
        self.assertEqual(critical["disposition"], "fail")
        self.assertGreater(critical["failed_window_count"], 0)
        self.assertEqual(routine["disposition"], "fail")
        self.assertGreater(routine["failed_window_count"], 0)
        self.assertGreater(report["overall_aggregate_response_success_ratio"], 0.99)

    def test_under_delivered_load_is_indeterminate_not_qualified(self):
        docs, _ = fixtures()
        load = record(docs, "nominal", "critical-payment", "load")
        old_count = load["offered_count"]
        load["offered_count"] = 480
        load["offered_rps"] = 8.0
        load["response_success_count"] = 480
        load["response_error_count"] = 0
        load["response_timeout_count"] = 0
        load["response_drop_count"] = 0
        load["latency_histogram"]["buckets"] = rebucket(480)
        obs = record(docs, "nominal", "critical-payment", "observations")
        obs["server_received_count"] = 480
        obs["business_checked_count"] = 480
        obs["business_correct_count"] = 480
        report = run(docs)
        window = next(x for x in report["windows"] if x["phase_id"] == "nominal" and x["class_id"] == "critical-payment")
        self.assertEqual(window["disposition"], "indeterminate")
        self.assertIn("offered demand is outside the frozen phase rate band", window["load_fidelity_reasons"])
        self.assertEqual(old_count, 600)

    def test_service_side_request_shedding_fails_independent_ingress_floor(self):
        docs, _ = fixtures()
        load = record(docs, "nominal", "critical-payment", "load")
        load["response_success_count"] = 550
        load["response_error_count"] = 50
        obs = record(docs, "nominal", "critical-payment", "observations")
        obs["server_received_count"] = 550
        obs["server_rejected_count"] = 50
        obs["business_checked_count"] = 550
        obs["business_correct_count"] = 550
        report = run(docs)
        window = next(x for x in report["windows"] if x["phase_id"] == "nominal" and x["class_id"] == "critical-payment")
        self.assertEqual(window["disposition"], "fail")
        self.assertLess(window["ingress_ratio"], 0.999)

    def test_nonzero_service_rejections_are_explicit_failure(self):
        docs, _ = fixtures()
        load = record(docs, "nominal", "critical-payment", "load")
        load["response_success_count"] = 599
        load["response_error_count"] = 1
        obs = record(docs, "nominal", "critical-payment", "observations")
        obs["server_rejected_count"] = 1
        obs["business_checked_count"] = 599
        obs["business_correct_count"] = 599
        report = run(docs)
        window = next(x for x in report["windows"] if x["phase_id"] == "nominal" and x["class_id"] == "critical-payment")
        self.assertEqual(window["disposition"], "fail")
        self.assertIn("service-side rejected requests are non-zero", window["reasons"])

    def test_retry_storm_exceeds_per_class_budget(self):
        docs, _ = fixtures()
        obs = record(docs, "retry-pressure", "critical-payment", "observations")
        report = run(docs)
        window = next(x for x in report["windows"] if x["phase_id"] == "retry-pressure" and x["class_id"] == "critical-payment")
        self.assertGreater(window["server_retry_attempts_per_100_offered"], 5)
        self.assertIn("server retry attempts exceed the class retry budget", window["reasons"])
        self.assertEqual(window["disposition"], "fail")

    def test_business_correctness_is_an_independent_noncompensatory_gate(self):
        docs, _ = fixtures()
        obs = record(docs, "nominal", "critical-payment", "observations")
        obs["business_correct_count"] = 590
        report = run(docs)
        window = next(x for x in report["windows"] if x["phase_id"] == "nominal" and x["class_id"] == "critical-payment")
        self.assertLess(window["business_correctness_ratio"], 0.999)
        self.assertIn("independent business correctness ratio is below the class floor", window["reasons"])
        self.assertEqual(window["disposition"], "fail")

    def test_class_omission_is_rejected(self):
        docs, _ = fixtures()
        docs["observations"]["windows"] = [
            row for row in docs["observations"]["windows"]
            if not (row["phase_id"] == "burst" and row["class_id"] == "critical-payment")
        ]
        with self.assertRaisesRegex(ContractError, "missing/extra class-window records"):
            run(docs)

    def test_duplicate_phase_class_record_is_rejected(self):
        docs, _ = fixtures()
        docs["load"]["windows"].append(copy.deepcopy(docs["load"]["windows"][0]))
        with self.assertRaisesRegex(ContractError, "missing/extra class-window records"):
            run(docs)

    def test_wrong_fault_id_is_rejected(self):
        docs, _ = fixtures()
        record(docs, "dependency-degradation", None, "faults")["fault_id"] = "different-fault"
        with self.assertRaisesRegex(ContractError, "fault ID does not match frozen schedule"):
            run(docs)

    def test_missing_fault_controller_receipt_is_rejected(self):
        docs, _ = fixtures()
        docs["faults"]["receipts"] = docs["faults"]["receipts"][:1]
        with self.assertRaisesRegex(ContractError, "missing or extra fault phase receipts"):
            run(docs)

    def test_fault_controller_trust_domain_cannot_alias_executor(self):
        docs, _ = fixtures()
        docs["contract"]["fault_controller"]["trust_domain_id"] = docs["contract"]["executor"]["trust_domain_id"]
        docs["faults"]["fault_controller"]["trust_domain_id"] = docs["contract"]["executor"]["trust_domain_id"]
        with self.assertRaisesRegex(ContractError, "must use distinct trust domains"):
            run(docs)

    def test_observer_timestamps_must_match_exact_scheduled_window(self):
        docs, _ = fixtures()
        record(docs, "burst", "critical-payment", "observations")["ended_at"] = "2026-10-31T14:02:01Z"
        with self.assertRaisesRegex(ContractError, "observer timestamps do not match"):
            run(docs)

    def test_phase_gap_is_rejected(self):
        docs, _ = fixtures()
        docs["schedule"]["phases"][2]["started_at"] = "2026-10-31T14:02:01Z"
        with self.assertRaisesRegex(ContractError, "phase schedule has a gap or overlap"):
            run(docs)

    def test_offered_demand_class_mix_drift_is_indeterminate(self):
        docs, _ = fixtures()
        load = record(docs, "nominal", "routine-read", "load")
        load["offered_count"] = 7200
        load["offered_rps"] = 7200 / 60
        load["response_success_count"] = 7200
        load["response_error_count"] = 0
        load["response_timeout_count"] = 0
        load["response_drop_count"] = 0
        load["latency_histogram"]["buckets"] = rebucket(7200)
        obs = record(docs, "nominal", "routine-read", "observations")
        obs["server_received_count"] = 7200
        obs["business_checked_count"] = 7200
        obs["business_correct_count"] = 7200
        report = run(docs)
        window = next(x for x in report["windows"] if x["phase_id"] == "nominal" and x["class_id"] == "routine-read")
        self.assertEqual(window["disposition"], "indeterminate")
        self.assertTrue(any("class mix" in reason for reason in window["load_fidelity_reasons"]))

    def test_invalid_loadgen_outcome_accounting_rejected(self):
        docs, _ = fixtures()
        load = record(docs, "nominal", "critical-payment", "load")
        load["response_drop_count"] = -1
        with self.assertRaisesRegex(ContractError, "all offered requests must end in success/error/timeout/drop"):
            run(docs)

    def test_artifact_digest_mismatch_rejected_without_rebinding(self):
        docs, raw = fixtures()
        docs["contract"]["profile_sha256"] = "0" * 64
        contract_bytes = encode(docs["contract"])
        with self.assertRaisesRegex(ContractError, "exact raw artifact digest mismatch"):
            evaluate(
                docs["contract"], contract_bytes,
                docs["profile"], raw["profile"],
                docs["schedule"], raw["schedule"],
                docs["load"], raw["load"],
                docs["observations"], raw["observations"],
                docs["faults"], raw["faults"],
            )

    def test_loadgen_identity_substitution_rejected(self):
        docs, _ = fixtures()
        docs["load"]["load_generator"]["source_id"] = "executor"
        with self.assertRaisesRegex(ContractError, "publisher identity mismatch"):
            run(docs)

    def test_dropped_request_is_counted_not_removed_from_denominator(self):
        docs, _ = fixtures()
        load = record(docs, "nominal", "critical-payment", "load")
        load["response_success_count"] = 599
        load["response_drop_count"] = 1
        obs = record(docs, "nominal", "critical-payment", "observations")
        obs["business_checked_count"] = 599
        obs["business_correct_count"] = 599
        report = run(docs)
        self.assertEqual(report["total_response_drops"], 1)
        self.assertEqual(report["total_offered_requests"], 27000)
        window = next(x for x in report["windows"] if x["phase_id"] == "nominal" and x["class_id"] == "critical-payment")
        self.assertEqual(window["disposition"], "fail")
        self.assertEqual(window["response_drop_count"], 1)

    def test_report_is_deterministic_for_identical_frozen_artifacts(self):
        docs, _ = fixtures()
        first = run(copy.deepcopy(docs))
        second = run(copy.deepcopy(docs))
        self.assertEqual(first["report_sha256"], second["report_sha256"])
        self.assertEqual(first["windows"], second["windows"])


    def test_payload_profile_substitution_rejected(self):
        docs, _ = fixtures()
        record(docs, "nominal", "critical-payment", "load")["payload_profile_id"] = "easy-read-payload"
        with self.assertRaisesRegex(ContractError, "payload profile substitution"):
            run(docs)

    def test_client_timeout_substitution_rejected(self):
        docs, _ = fixtures()
        record(docs, "nominal", "critical-payment", "load")["client_timeout_ms"] = 5000
        with self.assertRaisesRegex(ContractError, "client timeout differs"):
            run(docs)

    def test_concurrency_outside_frozen_range_rejected(self):
        docs, _ = fixtures()
        record(docs, "nominal", "critical-payment", "load")["peak_concurrency"] = 50
        with self.assertRaisesRegex(ContractError, "peak concurrency outside frozen class range"):
            run(docs)

    def test_fault_action_digest_mismatch_rejected(self):
        docs, _ = fixtures()
        record(docs, "retry-pressure", None, "faults")["action_digest"] = "c" * 64
        with self.assertRaisesRegex(ContractError, "fault action digest does not match frozen schedule"):
            run(docs)

    def test_raw_object_mismatch_is_rejected_before_semantic_evaluation(self):
        docs, raw = fixtures()
        wrong_profile = copy.deepcopy(docs["profile"])
        wrong_profile["profile_id"] = "mutated-in-memory"
        with self.assertRaisesRegex(ContractError, "profile: decoded object does not match exact supplied bytes"):
            evaluate(
                docs["contract"], raw["contract"],
                wrong_profile, raw["profile"],
                docs["schedule"], raw["schedule"],
                docs["load"], raw["load"],
                docs["observations"], raw["observations"],
                docs["faults"], raw["faults"],
            )


    def test_nonzero_timeout_fails_even_when_aggregate_success_remains_high(self):
        docs, _ = fixtures()
        load = record(docs, "nominal", "critical-payment", "load")
        load["response_success_count"] = 599
        load["response_timeout_count"] = 1
        obs = record(docs, "nominal", "critical-payment", "observations")
        obs["business_checked_count"] = 599
        obs["business_correct_count"] = 599
        report = run(docs)
        window = next(x for x in report["windows"] if x["phase_id"] == "nominal" and x["class_id"] == "critical-payment")
        self.assertEqual(window["disposition"], "fail")
        self.assertIn("timed-out requests are non-zero", window["reasons"])

    def test_drop_count_has_its_own_failure_reason(self):
        docs, _ = fixtures()
        load = record(docs, "nominal", "critical-payment", "load")
        load["response_success_count"] = 599
        load["response_drop_count"] = 1
        obs = record(docs, "nominal", "critical-payment", "observations")
        obs["business_checked_count"] = 599
        obs["business_correct_count"] = 599
        report = run(docs)
        window = next(x for x in report["windows"] if x["phase_id"] == "nominal" and x["class_id"] == "critical-payment")
        self.assertIn("dropped requests are non-zero", window["reasons"])
        self.assertEqual(window["disposition"], "fail")



if __name__ == "__main__":
    unittest.main()
