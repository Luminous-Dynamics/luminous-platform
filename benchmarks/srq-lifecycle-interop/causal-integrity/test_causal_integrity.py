import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import causal_integrity as ci


class CausalIntegrityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.protocol = ci.load_protocol()

    def test_protocol_frozen_inventories(self):
        self.assertEqual(len(self.protocol["attack_cases"]), 29)
        self.assertEqual(len(self.protocol["positive_controls"]), 3)

    def test_successful_promotion_is_structurally_accepted_but_not_release_qualified(self):
        candidate = ci.make_candidate("success")
        self.assertEqual(ci.validate_candidate(candidate, self.protocol), [])
        report = ci.run_corpus(self.protocol)
        self.assertEqual(report["qualification_status"], "BLOCKED")
        self.assertEqual(report["claim_ceiling"], "none")

    def test_terminal_failure_is_structurally_accepted(self):
        self.assertEqual(ci.validate_candidate(ci.make_candidate("failed"), self.protocol), [])

    def test_failure_then_recovery_is_structurally_accepted(self):
        self.assertEqual(ci.validate_candidate(ci.make_candidate("recovered"), self.protocol), [])

    def test_every_frozen_attack_is_rejected(self):
        for case in ci.build_corpus(self.protocol):
            with self.subTest(case=case["case_id"]):
                errors = ci.validate_candidate(case["candidate"], self.protocol)
                if case["expected"] == "REJECT":
                    self.assertTrue(errors, case["case_id"])
                else:
                    self.assertEqual(errors, [], (case["case_id"], errors))

    def test_positive_paths_survive_consistent_identity_and_event_id_rebinding(self):
        for mode in ("success", "failed", "recovered"):
            with self.subTest(mode=mode):
                candidate = ci.make_candidate(mode)
                tx = candidate["transaction"]
                tx.update({
                    "transaction_id": "1" * 64,
                    "target_machine_digest": "2" * 64,
                    "plan_digest": "3" * 64,
                    "artifact_digest": "4" * 64,
                    "policy_digest": "5" * 64,
                })
                new_ids = {}
                for event in candidate["events"]:
                    old_id = event["event_id"]
                    new_ids[old_id] = f"rebound-{mode}-{event['kind']}"
                for event in candidate["events"]:
                    event["event_id"] = new_ids[event["event_id"]]
                    event["transaction_id"] = tx["transaction_id"]
                    event["target_machine_digest"] = tx["target_machine_digest"]
                    event["plan_digest"] = tx["plan_digest"]
                    event["artifact_digest"] = tx["artifact_digest"]
                    payload = event["payload"]
                    kind = event["kind"]
                    if kind == "validated":
                        payload.update({
                            "validated_plan_digest": tx["plan_digest"],
                            "validated_target_machine_digest": tx["target_machine_digest"],
                            "validated_artifact_digest": tx["artifact_digest"],
                        })
                    elif kind == "authorized":
                        payload.update({
                            "authorized_plan_digest": tx["plan_digest"],
                            "authorized_target_machine_digest": tx["target_machine_digest"],
                            "authorized_artifact_digest": tx["artifact_digest"],
                            "policy_digest": tx["policy_digest"],
                            "signature_sha256": "9" * 64,
                        })
                    elif kind == "snapshotted":
                        payload.update({"target_machine_digest": tx["target_machine_digest"], "pre_state_digest": "6" * 64})
                    elif kind == "applied":
                        payload.update({
                            "authorization_event_id": new_ids["evt-authorized"],
                            "snapshot_event_id": new_ids["evt-snapshot"],
                            "applied_artifact_digest": tx["artifact_digest"],
                            "expected_post_state_digest": "7" * 64,
                            "generation_after": 23,
                        })
                    elif kind == "verified":
                        payload.update({"application_event_id": new_ids["evt-applied"], "observed_state_digest": "7" * 64, "observed_generation": 23})
                    elif kind == "outcome":
                        payload["verification_event_id"] = new_ids["evt-verified"]
                    elif kind == "promoted":
                        payload.update({"outcome_event_id": new_ids["evt-outcome"], "verification_event_id": new_ids["evt-verified"], "target_state_digest": "7" * 64})
                    elif kind == "failed":
                        payload["application_event_id"] = new_ids["evt-applied"]
                    elif kind == "recovered":
                        payload.update({
                            "failed_event_id": new_ids["evt-failed"],
                            "snapshot_event_id": new_ids["evt-snapshot"],
                            "restored_state_digest": "6" * 64,
                            "recovery_observation_sha256": "8" * 64,
                        })
                ci.seal(candidate)
                self.assertEqual(ci.validate_candidate(candidate, self.protocol), [])

    def test_duplicate_json_keys_are_rejected(self):
        with self.assertRaises(ValueError):
            ci.json.loads('{"x":1,"x":2}', object_pairs_hook=ci._unique_object)

    def test_protocol_drift_fails_closed(self):
        changed = copy.deepcopy(self.protocol)
        changed["target_metrics"]["expected_attack_case_count"] += 1
        with self.assertRaises(ValueError):
            ci.validate_protocol(changed)

    def test_manifest_cannot_substitute_attack_id_without_changing_executable_corpus(self):
        changed = copy.deepcopy(self.protocol)
        changed["attack_cases"][0] = "some-other-attack"
        with self.assertRaises(ValueError):
            ci.validate_protocol(changed)

    def test_report_counts_and_zero_error_gates(self):
        report = ci.run_corpus(self.protocol)
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(report["observed_case_count"], 32)
        self.assertEqual(report["attack_case_count"], 29)
        self.assertEqual(report["positive_control_count"], 3)
        for metric in ("false_accept_count", "false_reject_count", "unexpected_error_count", "false_release_pass_count"):
            self.assertEqual(report[metric], 0, metric)


if __name__ == "__main__":
    unittest.main(verbosity=2)
