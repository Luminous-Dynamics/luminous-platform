import unittest

from history_oracle import InvalidHistory, check_history

SINK = "a" * 64
REQ = "b" * 64
EFF = "c" * 64
POL = "d" * 64


def ev(seq, kind, **fields):
    return {"seq": seq, "kind": kind, **fields}


def valid_history():
    return {
        "schema": "srq-sink-history-v1",
        "sink_identity_digest": SINK,
        "initial_fencing_token": 1,
        "events": [
            ev(0, "invoke", attempt_id="a1", idempotency_key="k1", request_digest=REQ, effect_digest=EFF, policy_digest=POL, fencing_token=1),
            ev(1, "effect_committed", attempt_id="a1", effect_id="e1", request_digest=REQ, effect_digest=EFF, policy_digest=POL, sink_identity_digest=SINK, fencing_token=1),
            ev(2, "receipt_committed", attempt_id="a1", receipt_id="r1", effect_id="e1", request_digest=REQ, effect_digest=EFF, policy_digest=POL, sink_identity_digest=SINK),
            ev(3, "response", attempt_id="a1", outcome="committed", receipt_id="r1"),
            ev(4, "invoke", attempt_id="a2", idempotency_key="k1", request_digest=REQ, effect_digest=EFF, policy_digest=POL, fencing_token=1),
            ev(5, "response", attempt_id="a2", outcome="committed", receipt_id="r1"),
        ],
    }


class HistoryOracleTests(unittest.TestCase):
    def test_valid_same_key_retry_reuses_receipt_without_second_effect(self):
        report = check_history(valid_history())
        self.assertEqual(report["safety_status"], "PASS")
        self.assertEqual(report["liveness_observations"]["effect_count"], 1)
        self.assertEqual(report["liveness_observations"]["receipt_count"], 1)

    def test_detects_key_reuse_for_different_intent(self):
        history = valid_history()
        history["events"][4]["effect_digest"] = "e" * 64
        report = check_history(history)
        self.assertIn("KEY_REUSED_FOR_DIFFERENT_INTENT", {v["code"] for v in report["violations"]})

    def test_detects_duplicate_logical_effects(self):
        history = valid_history()
        history["events"].append(ev(6, "effect_committed", attempt_id="a2", effect_id="e2",
            request_digest=REQ, effect_digest=EFF, policy_digest=POL,
            sink_identity_digest=SINK, fencing_token=1))
        report = check_history(history)
        codes = {v["code"] for v in report["violations"]}
        self.assertIn("DUPLICATE_LOGICAL_EFFECT", codes)
        self.assertIn("EFFECT_WITHOUT_RECEIPT", codes)

    def test_detects_receipt_wrong_effect_binding(self):
        history = valid_history()
        history["events"][2]["effect_digest"] = "e" * 64
        report = check_history(history)
        self.assertIn("RECEIPT_DOES_NOT_BIND_EFFECT", {v["code"] for v in report["violations"]})

    def test_detects_committed_response_without_receipt(self):
        history = valid_history()
        history["events"][3]["receipt_id"] = "missing"
        report = check_history(history)
        self.assertIn("COMMITTED_RESPONSE_WITHOUT_PRIOR_RECEIPT", {v["code"] for v in report["violations"]})

    def test_detects_stale_worker_effect_after_fence_advance(self):
        history = valid_history()
        history["events"].insert(1, ev(1, "fence_advanced", fencing_token=2))
        for index, event in enumerate(history["events"]):
            event["seq"] = index
        report = check_history(history)
        codes = {v["code"] for v in report["violations"]}
        self.assertIn("STALE_WORKER_EFFECT", codes)
        self.assertIn("INVOCATION_USED_STALE_OR_UNISSUED_FENCE", codes)

    def test_universal_rejection_is_not_promoted_to_liveness_success(self):
        history = {
            "schema": "srq-sink-history-v1", "sink_identity_digest": SINK,
            "events": [
                ev(0, "invoke", attempt_id="a1", idempotency_key="k1", request_digest=REQ,
                   effect_digest=EFF, policy_digest=POL, fencing_token=0),
                ev(1, "response", attempt_id="a1", outcome="rejected", receipt_id=None),
            ],
        }
        report = check_history(history)
        self.assertEqual(report["safety_status"], "PASS")
        self.assertEqual(report["liveness_observations"]["claim"], "NOT_EVALUATED")
        self.assertEqual(report["liveness_observations"]["effect_count"], 0)
        self.assertEqual(report["liveness_observations"]["rejected_response_count"], 1)

    def test_unknown_outcome_remains_unresolved(self):
        history = {
            "schema": "srq-sink-history-v1", "sink_identity_digest": SINK,
            "events": [
                ev(0, "invoke", attempt_id="a1", idempotency_key="k1", request_digest=REQ,
                   effect_digest=EFF, policy_digest=POL, fencing_token=0),
                ev(1, "response", attempt_id="a1", outcome="unknown", receipt_id=None),
            ],
        }
        report = check_history(history)
        self.assertEqual(report["liveness_observations"]["unresolved_or_unknown_attempt_ids"], ["a1"])

    def test_invalid_digest_is_invalid_history_not_a_security_kill(self):
        history = valid_history()
        history["events"][0]["request_digest"] = "not-a-digest"
        with self.assertRaises(InvalidHistory):
            check_history(history)

    def test_non_contiguous_sequence_is_safety_failure(self):
        history = valid_history()
        history["events"][0]["seq"] = 4
        report = check_history(history)
        self.assertIn("NON_CONTIGUOUS_SEQUENCE", {v["code"] for v in report["violations"]})

    def test_rejected_attempt_with_effect_is_violation(self):
        history = valid_history()
        history["events"][3]["outcome"] = "rejected"
        history["events"][3]["receipt_id"] = None
        report = check_history(history)
        self.assertIn("REJECTED_ATTEMPT_HAS_EFFECT", {v["code"] for v in report["violations"]})

    def test_unknown_kind_is_invalid_history(self):
        history = valid_history()
        history["events"][0]["kind"] = "maybe"
        with self.assertRaises(InvalidHistory):
            check_history(history)

    def test_empty_history_is_invalid_not_vacuous_pass(self):
        history = {"schema": "srq-sink-history-v1", "sink_identity_digest": SINK, "events": []}
        with self.assertRaises(InvalidHistory):
            check_history(history)


if __name__ == "__main__":
    unittest.main()
