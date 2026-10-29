import json
import unittest
from pathlib import Path
from linearizability_oracle import InvalidHistory, check_history

SINK, REQ, EFF, POL = "a"*64, "b"*64, "c"*64, "d"*64
EFF2 = "e"*64

def op(op_id, start_ns, end_ns, key, intent=EFF, result="committed", effect_id="e1", receipt_id="r1"):
    item = {"op_id":op_id,"start_ns":start_ns,"end_ns":end_ns,"idempotency_key":key,
            "request_digest":REQ,"effect_digest":intent,"policy_digest":POL,"result":result}
    if result == "committed":
        item.update({"effect_id":effect_id,"receipt_id":receipt_id})
    return item

def history(operations):
    return {"schema":"srq-linearizability-history-v1","clock_domain":"single_observer_monotonic",
            "sink_identity_digest":SINK,"operations":operations}

def fixture(name):
    return json.loads((Path(__file__).parent/"fixtures"/name).read_text(encoding="utf-8"))

class LinearizabilityOracleTests(unittest.TestCase):
    def test_concurrent_retry_can_have_a_witness(self):
        report=check_history(history([op("a",10,50,"k",effect_id="e1",receipt_id="r1"),
                                     op("b",12,40,"k",effect_id="e1",receipt_id="r1")]))
        self.assertEqual(report["status"],"LINEARIZABLE")
        self.assertEqual(set(report["linearization"]),{"a","b"})

    def test_overlapping_duplicate_effects_are_non_linearizable(self):
        report=check_history(history([op("a",10,50,"k",effect_id="e1",receipt_id="r1"),
                                     op("b",12,40,"k",effect_id="e2",receipt_id="r2")]))
        self.assertEqual(report["status"],"NON_LINEARIZABLE")
        self.assertIsNone(report["linearization"])

    def test_real_time_order_forbids_conflict_before_key_creation(self):
        report=check_history(history([op("create",10,20,"k",intent=EFF,effect_id="e1",receipt_id="r1"),
                                     op("conflict",1,5,"k",intent=EFF2,result="key_conflict")]))
        self.assertEqual(report["status"],"NON_LINEARIZABLE")

    def test_overlapping_conflict_may_follow_key_creation(self):
        report=check_history(history([op("create",1,20,"k",intent=EFF,effect_id="e1",receipt_id="r1"),
                                     op("conflict",4,8,"k",intent=EFF2,result="key_conflict")]))
        self.assertEqual(report["status"],"LINEARIZABLE")

    def test_unknown_then_committed_retry_can_be_explained(self):
        report=check_history(history([op("timeout",1,5,"k",result="unknown"),
                                     op("retry",6,10,"k",effect_id="e1",receipt_id="r1")]))
        self.assertEqual(report["status"],"LINEARIZABLE")

    def test_distinct_keys_cannot_reuse_effect_identifier(self):
        report=check_history(history([op("a",1,5,"ka",effect_id="same",receipt_id="ra"),
                                     op("b",2,7,"kb",effect_id="same",receipt_id="rb")]))
        self.assertEqual(report["status"],"NON_LINEARIZABLE")

    def test_search_limit_is_inconclusive(self):
        report=check_history(history([op("a",1,5,"ka",effect_id="ea",receipt_id="ra"),
                                     op("b",2,7,"kb",effect_id="eb",receipt_id="rb")]),max_states=1)
        self.assertEqual(report["status"],"UNKNOWN_LIMIT")
        self.assertIsNone(report["linearization"])

    def test_empty_history_is_invalid(self):
        with self.assertRaises(InvalidHistory): check_history(history([]))

    def test_duplicate_operation_id_is_invalid(self):
        with self.assertRaises(InvalidHistory):
            check_history(history([op("a",1,2,"ka"),op("a",3,4,"kb")]))

    def test_inverted_interval_is_invalid(self):
        with self.assertRaises(InvalidHistory): check_history(history([op("a",5,4,"ka")]))

    def test_claim_ceiling_is_blocked(self):
        report=check_history(history([op("a",1,2,"ka")]))
        self.assertEqual(report["qualification_status"],"BLOCKED")
        self.assertEqual(report["claim_ceiling"],"none")

    def test_valid_fixture(self):
        self.assertEqual(check_history(fixture("valid-concurrent-same-key.json"))["status"],"LINEARIZABLE")

    def test_duplicate_fixture(self):
        self.assertEqual(check_history(fixture("invalid-duplicate-effect.json"))["status"],"NON_LINEARIZABLE")

    def test_realtime_fixture(self):
        self.assertEqual(check_history(fixture("invalid-realtime-conflict.json"))["status"],"NON_LINEARIZABLE")

if __name__ == "__main__": unittest.main()
