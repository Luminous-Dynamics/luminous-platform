import copy
import unittest
from history_checker import check_history


def base_trace():
    return {
        "schema": "srq-side-effect-history-v1",
        "trace_id": "trace-001",
        "sink_id": "sink-a",
        "fencing_required": True,
        "operations": [
            {"op_id": "op-1", "key": "k-1", "request_digest": "sha256:a1", "outcome": "success", "transaction_id": "tx-1", "effect_id": "effect-1", "receipt_id": "receipt-1", "invoke_seq": 1, "return_seq": 5},
            {"op_id": "op-2", "key": "k-1", "request_digest": "sha256:a1", "outcome": "success", "transaction_id": "tx-1", "effect_id": "effect-1", "receipt_id": "receipt-1", "invoke_seq": 2, "return_seq": 6},
            {"op_id": "op-3", "key": "k-1", "request_digest": "sha256:b2", "outcome": "rejected", "reject_reason": "idempotency_key_payload_mismatch", "invoke_seq": 3, "return_seq": 7},
        ],
        "transactions": [
            {"transaction_id": "tx-1", "key": "k-1", "request_digest": "sha256:a1", "effect_id": "effect-1", "receipt_id": "receipt-1", "fencing_token": 7, "commit_seq": 4}
        ],
        "fence_events": [{"event_seq": 1, "token": 7}],
    }


class HistoryCheckerTests(unittest.TestCase):
    def test_valid_retry_mismatch_and_fenced_atomic_transaction(self):
        self.assertEqual(check_history(base_trace())["status"], "PASS")

    def test_duplicate_effect_for_same_key_is_rejected(self):
        trace = base_trace()
        second = copy.deepcopy(trace["transactions"][0])
        second.update(transaction_id="tx-2", effect_id="effect-2", receipt_id="receipt-2", commit_seq=5)
        trace["transactions"].append(second)
        report = check_history(trace)
        self.assertEqual(report["status"], "FAIL")
        self.assertIn("MULTIPLE_EFFECTS_FOR_KEY", {v["code"] for v in report["violations"]})

    def test_partial_atomic_transaction_is_rejected(self):
        trace = base_trace()
        trace["transactions"][0]["receipt_id"] = ""
        report = check_history(trace)
        self.assertEqual(report["status"], "FAIL")
        self.assertIn("PARTIAL_ATOMIC_TRANSACTION", {v["code"] for v in report["violations"]})

    def test_success_cannot_claim_another_receipt_or_effect(self):
        trace = base_trace()
        trace["operations"][0]["effect_id"] = "effect-forged"
        report = check_history(trace)
        self.assertIn("SUCCESS_RECEIPT_EFFECT_MISMATCH", {v["code"] for v in report["violations"]})

    def test_stale_fencing_token_is_rejected(self):
        trace = base_trace()
        trace["fence_events"].append({"event_seq": 3, "token": 8})
        report = check_history(trace)
        self.assertIn("STALE_OR_WRONG_FENCE", {v["code"] for v in report["violations"]})

    def test_fence_token_rollback_is_rejected(self):
        trace = base_trace()
        trace["fence_events"] = [{"event_seq": 1, "token": 8}, {"event_seq": 3, "token": 7}]
        report = check_history(trace)
        self.assertIn("FENCE_NOT_MONOTONIC", {v["code"] for v in report["violations"]})

    def test_fencing_token_reuse_is_rejected(self):
        trace = base_trace()
        trace["fence_events"] = [{"event_seq": 1, "token": 7}, {"event_seq": 3, "token": 8}, {"event_seq": 4, "token": 7}]
        report = check_history(trace)
        self.assertIn("FENCE_TOKEN_REUSED", {v["code"] for v in report["violations"]})

    def test_different_intent_cannot_succeed_under_same_key(self):
        trace = base_trace()
        trace["operations"].append({"op_id": "op-4", "key": "k-1", "request_digest": "sha256:b2", "outcome": "success", "transaction_id": "tx-1", "effect_id": "effect-1", "receipt_id": "receipt-1"})
        report = check_history(trace)
        self.assertIn("SUCCESS_INTENT_BINDING_MISMATCH", {v["code"] for v in report["violations"]})
        self.assertIn("KEY_REUSED_FOR_DIFFERENT_INTENT", {v["code"] for v in report["violations"]})

    def test_divergent_retry_result_is_rejected(self):
        trace = base_trace()
        trace["operations"][1]["receipt_id"] = "receipt-other"
        report = check_history(trace)
        self.assertIn("RETRY_RESULT_DIVERGENCE", {v["code"] for v in report["violations"]})

    def test_success_without_transaction_is_rejected(self):
        trace = base_trace()
        trace["operations"][0]["transaction_id"] = "missing-tx"
        report = check_history(trace)
        self.assertIn("SUCCESS_WITHOUT_COMMITTED_TRANSACTION", {v["code"] for v in report["violations"]})

    def test_missing_fence_rejected_when_required(self):
        trace = base_trace()
        trace["fence_events"] = []
        report = check_history(trace)
        self.assertIn("EFFECT_WITHOUT_FENCE", {v["code"] for v in report["violations"]})


if __name__ == "__main__":
    unittest.main(verbosity=2)
