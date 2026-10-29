import hashlib
import json
import unittest
from pathlib import Path
from differential_reference import reference_accepts_witness, reference_has_linearization
from differential_qualification import deterministic_corpus, history_from_shapes, operation_shape
from linearizability_oracle import check_history

class DifferentialQualificationTests(unittest.TestCase):
    def test_operation_shape_domain_has_72_unique_shapes(self):
        encoded={json.dumps(operation_shape(0,index),sort_keys=True) for index in range(72)}
        self.assertEqual(len(encoded),72)

    def test_independent_reference_accepts_valid_concurrent_retry(self):
        history=history_from_shapes((0,3))
        history["operations"]=[
          {"op_id":"a","start_ns":1,"end_ns":8,"idempotency_key":"key","request_digest":"b"*64,"effect_digest":"c"*64,"policy_digest":"d"*64,"result":"committed","effect_id":"effect-a","receipt_id":"receipt-a"},
          {"op_id":"b","start_ns":2,"end_ns":6,"idempotency_key":"key","request_digest":"b"*64,"effect_digest":"c"*64,"policy_digest":"d"*64,"result":"committed","effect_id":"effect-a","receipt_id":"receipt-a"}]
        expected,witness=reference_has_linearization(history)
        actual=check_history(history)
        self.assertTrue(expected)
        self.assertIsNotNone(witness)
        self.assertTrue(reference_accepts_witness(history,actual["linearization"]))
        self.assertEqual(actual["status"],"LINEARIZABLE")

    def test_independent_reference_rejects_duplicate_effects(self):
        history=history_from_shapes((0,3))
        history["operations"]=[
          {"op_id":"a","start_ns":1,"end_ns":8,"idempotency_key":"key","request_digest":"b"*64,"effect_digest":"c"*64,"policy_digest":"d"*64,"result":"committed","effect_id":"effect-a","receipt_id":"receipt-a"},
          {"op_id":"b","start_ns":2,"end_ns":6,"idempotency_key":"key","request_digest":"b"*64,"effect_digest":"c"*64,"policy_digest":"d"*64,"result":"committed","effect_id":"effect-b","receipt_id":"receipt-b"}]
        expected,witness=reference_has_linearization(history)
        actual=check_history(history)
        self.assertFalse(expected)
        self.assertIsNone(witness)
        self.assertEqual(actual["status"],"NON_LINEARIZABLE")

    def test_search_budget_is_inconclusive(self):
        history=history_from_shapes((0,3))
        self.assertEqual(check_history(history,max_states=1)["status"],"UNKNOWN_LIMIT")

    def test_corpus_counts_are_frozen_and_unique(self):
        records=list(deterministic_corpus())
        tiers={name:sum(1 for tier,_ in records if tier==name) for name in ("exhaustive_2op","sampled_3op","sampled_4op")}
        self.assertEqual(tiers,{"exhaustive_2op":5184,"sampled_3op":512,"sampled_4op":128})
        self.assertEqual(len(records),5824)
        fingerprints={json.dumps(history,sort_keys=True) for _,history in records}
        self.assertEqual(len(fingerprints),5824)

    def test_canonical_corpus_hash_matches_frozen_protocol(self):
        root = Path(__file__).resolve().parent
        protocol = json.loads((root / "differential-protocol-v1.json").read_text(encoding="utf-8"))
        digest = hashlib.sha256()
        for _, history in deterministic_corpus():
            canonical = json.dumps(history, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
            digest.update(canonical.encode("utf-8") + b"\n")
        self.assertEqual(digest.hexdigest(), protocol["corpus_sha256"])

    def test_selected_corpus_slice_agrees(self):
        for tier,history in list(deterministic_corpus())[:160]:
            expected,_=reference_has_linearization(history)
            actual=check_history(history)
            with self.subTest(tier=tier,history=history):
                self.assertIn(actual["status"],{"LINEARIZABLE","NON_LINEARIZABLE"})
                self.assertEqual(actual["status"]=="LINEARIZABLE",expected)
                if actual["status"]=="LINEARIZABLE":
                    self.assertTrue(reference_accepts_witness(history,actual["linearization"]))

if __name__=="__main__":
    unittest.main()
