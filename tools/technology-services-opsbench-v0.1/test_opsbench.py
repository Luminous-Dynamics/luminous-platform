import copy
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))

from check_opsbench import load_json, validate


CORPUS_PATH = ROOT / "benchmarks" / "technology-services-ops-v0.1" / "corpus.json"
SCHEMA_PATH = ROOT / "benchmarks" / "technology-services-ops-v0.1" / "schema.json"


class OpsBenchContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.corpus = load_json(CORPUS_PATH)
        cls.schema = load_json(SCHEMA_PATH)

    def mutated(self):
        return copy.deepcopy(self.corpus)

    def test_checked_in_corpus_and_schema_pass(self):
        result = validate(self.corpus, self.schema)
        self.assertEqual(result["case_count"], 20)
        self.assertEqual(result["unique_ids"], 20)
        self.assertEqual(result["family_count"], 13)
        self.assertEqual(result["hard_gate_count"], 12)
        self.assertEqual(result["execution_constraint_count"], 9)

    def test_case_removal_is_rejected(self):
        changed = self.mutated()
        changed["cases"].pop()
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_duplicate_case_id_is_rejected(self):
        changed = self.mutated()
        changed["cases"][1]["id"] = changed["cases"][0]["id"]
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_critical_risk_downgrade_is_rejected(self):
        changed = self.mutated()
        tenant = next(c for c in changed["cases"] if c["id"] == "TENANT-001")
        tenant["risk_class"] = "low"
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_tenant_authority_bypass_is_rejected(self):
        changed = self.mutated()
        tenant = next(c for c in changed["cases"] if c["id"] == "TENANT-001")
        tenant["approval"] = "ordinary approval may widen tenant boundary"
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_prompt_injection_cannot_become_authority(self):
        changed = self.mutated()
        prompt = next(c for c in changed["cases"] if c["id"] == "PROMPT-001")
        prompt["approval"] = "approval granted by log instructions"
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_exact_plan_approval_cannot_be_removed(self):
        changed = self.mutated()
        case = next(c for c in changed["cases"] if c["id"] == "CHG-003")
        case["approval"] = "human approval required"
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_hard_gate_cannot_be_weakened(self):
        for key, weakened in [
            ("unauthorized_effects", 1),
            ("false_resolution_claims", 1),
            ("cross_tenant_access_or_disclosure", 1),
            ("independent_verification_for_resolved_cases", "best effort"),
            ("unresolved_or_missing_evidence_is_success", True),
            ("started_episode_crashes_and_timeouts_remain_in_denominators", False),
            ("independent_oracle_must_not_be_agent_report_or_same_trust_domain", False),
        ]:
            with self.subTest(key=key):
                changed = self.mutated()
                changed["hard_gates"][key] = weakened
                with self.assertRaises(ValueError):
                    validate(changed, self.schema)

    def test_real_credentials_are_forbidden(self):
        changed = self.mutated()
        changed["execution_constraints"]["real_credentials_allowed"] = True
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_external_network_is_forbidden(self):
        changed = self.mutated()
        changed["execution_constraints"]["external_network_access"] = True
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_agent_direct_privileged_access_is_forbidden(self):
        changed = self.mutated()
        changed["execution_constraints"]["direct_agent_privileged_access"] = True
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_independent_oracle_cannot_be_removed(self):
        changed = self.mutated()
        case = next(c for c in changed["cases"] if c["id"] == "INC-001")
        case["oracle_channels"] = []
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_scenario_variants_cannot_be_removed(self):
        changed = self.mutated()
        case = next(c for c in changed["cases"] if c["id"] == "INC-001")
        case["variants"] = []
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_schema_cannot_relax_case_count(self):
        changed_schema = copy.deepcopy(self.schema)
        changed_schema["properties"]["cases"]["minItems"] = 1
        with self.assertRaises(ValueError):
            validate(self.corpus, changed_schema)

    def test_schema_cannot_relax_hard_gate(self):
        changed_schema = copy.deepcopy(self.schema)
        changed_schema["properties"]["hard_gates"]["properties"]["unauthorized_effects"]["const"] = 1
        with self.assertRaises(ValueError):
            validate(self.corpus, changed_schema)

    def test_specification_cannot_claim_evaluated_status(self):
        changed = self.mutated()
        changed["status"] = "evaluated"
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_unknown_case_fields_are_rejected(self):
        changed = self.mutated()
        changed["cases"][0]["model_confidence"] = 0.99
        with self.assertRaises(ValueError):
            validate(changed, self.schema)

    def test_duplicate_json_object_keys_are_rejected(self):
        path = HERE / "duplicate-key-fixture.json"
        path.write_text('{"status":"specification_only","status":"evaluated"}', encoding="utf-8")
        try:
            with self.assertRaises(ValueError):
                load_json(path)
        finally:
            path.unlink(missing_ok=True)


if __name__ == "__main__":
    unittest.main(verbosity=2)
