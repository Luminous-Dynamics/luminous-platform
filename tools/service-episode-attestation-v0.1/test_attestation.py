from pathlib import Path
import copy
import json
import unittest

from check_attestation import validate

ROOT = Path(__file__).resolve().parents[2]
PREDICATE = json.loads((ROOT / "benchmarks/service-episode-attestation-v0.1/predicate.schema.json").read_text())
BUNDLE = json.loads((ROOT / "benchmarks/service-episode-attestation-v0.1/bundle.schema.json").read_text())
VECTORS = json.loads((ROOT / "benchmarks/service-episode-attestation-v0.1/conformance-vectors.json").read_text())
VECTOR_SCHEMA = json.loads((ROOT / "benchmarks/service-episode-attestation-v0.1/conformance-vectors.schema.json").read_text())


class AttestationContractTests(unittest.TestCase):
    def test_frozen_conformance_inventory_passes(self):
        self.assertEqual(validate(VECTORS, PREDICATE, BUNDLE, VECTOR_SCHEMA)["vector_count"], 32)

    def test_duplicate_vector_id_rejected(self):
        changed = copy.deepcopy(VECTORS)
        changed["vectors"][1]["id"] = changed["vectors"][0]["id"]
        with self.assertRaises(ValueError):
            validate(changed, PREDICATE, BUNDLE, VECTOR_SCHEMA)

    def test_missing_vector_rejected(self):
        changed = copy.deepcopy(VECTORS)
        changed["vectors"].pop()
        with self.assertRaises(ValueError):
            validate(changed, PREDICATE, BUNDLE, VECTOR_SCHEMA)

    def test_sut_input_cannot_include_expected_result(self):
        changed = copy.deepcopy(VECTORS)
        changed["vectors"][0]["input"]["expected"] = {"decision":"accept"}
        with self.assertRaises(ValueError):
            validate(changed, PREDICATE, BUNDLE, VECTOR_SCHEMA)

    def test_vectors_must_remain_evaluator_only(self):
        changed = copy.deepcopy(VECTORS)
        changed["vectors"][0]["visibility"] = "sut_input"
        with self.assertRaises(ValueError):
            validate(changed, PREDICATE, BUNDLE, VECTOR_SCHEMA)

    def test_coordinator_cannot_replace_observer_role(self):
        changed = copy.deepcopy(VECTORS)
        changed["trust_policy"]["required_signer_roles"].remove("independent_observer")
        with self.assertRaises(ValueError):
            validate(changed, PREDICATE, BUNDLE, VECTOR_SCHEMA)

    def test_independence_is_control_domain_not_label(self):
        changed = copy.deepcopy(VECTORS)
        changed["trust_policy"]["independent_observer_requires_distinct_control_domain"] = False
        with self.assertRaises(ValueError):
            validate(changed, PREDICATE, BUNDLE, VECTOR_SCHEMA)

    def test_key_id_is_not_identity_proof(self):
        changed = copy.deepcopy(VECTORS)
        changed["trust_policy"]["keyid_is_identity_proof"] = True
        with self.assertRaises(ValueError):
            validate(changed, PREDICATE, BUNDLE, VECTOR_SCHEMA)

    def test_unknown_effect_must_remain_indeterminate(self):
        changed = copy.deepcopy(VECTORS)
        case = next(x for x in changed["vectors"] if x["id"] == "VSA-SEM-001")
        case["expected"]["decision"] = "accept"
        with self.assertRaises(ValueError):
            validate(changed, PREDICATE, BUNDLE, VECTOR_SCHEMA)

    def test_missing_participant_signature_is_rejected(self):
        changed = copy.deepcopy(VECTORS)
        case = next(x for x in changed["vectors"] if x["id"] == "VSA-ROLE-004")
        case["expected"]["decision"] = "accept"
        with self.assertRaises(ValueError):
            validate(changed, PREDICATE, BUNDLE, VECTOR_SCHEMA)

    def test_unknown_major_predicate_version_rejected(self):
        changed = copy.deepcopy(VECTORS)
        case = next(x for x in changed["vectors"] if x["id"] == "VSA-SCHEMA-001")
        case["expected"]["decision"] = "accept"
        with self.assertRaises(ValueError):
            validate(changed, PREDICATE, BUNDLE, VECTOR_SCHEMA)

    def test_duplicate_nonce_case_is_rejected(self):
        changed = copy.deepcopy(VECTORS)
        case = next(x for x in changed["vectors"] if x["id"] == "VSA-FRESH-003")
        case["expected"]["decision"] = "accept"
        with self.assertRaises(ValueError):
            validate(changed, PREDICATE, BUNDLE, VECTOR_SCHEMA)

    def test_signature_payload_type_is_pinned_in_schema(self):
        changed = copy.deepcopy(BUNDLE)
        changed["properties"]["payload_type"]["const"] = "application/json"
        with self.assertRaises(ValueError):
            validate(VECTORS, PREDICATE, changed, VECTOR_SCHEMA)

    def test_bundle_requires_independent_observer(self):
        changed = copy.deepcopy(BUNDLE)
        changed["properties"]["required_roles"]["const"].remove("independent_observer")
        with self.assertRaises(ValueError):
            validate(VECTORS, PREDICATE, changed, VECTOR_SCHEMA)

    def test_predicate_schema_cannot_drop_attempt_id(self):
        changed = copy.deepcopy(PREDICATE)
        changed["properties"]["binding"]["required"].remove("attempt_id")
        with self.assertRaises(ValueError):
            validate(VECTORS, changed, BUNDLE, VECTOR_SCHEMA)

    def test_predicate_schema_cannot_drop_nonce(self):
        changed = copy.deepcopy(PREDICATE)
        changed["properties"]["freshness"]["required"].remove("nonce")
        with self.assertRaises(ValueError):
            validate(VECTORS, changed, BUNDLE, VECTOR_SCHEMA)

    def test_bundle_schema_cannot_allow_empty_signatures(self):
        changed = copy.deepcopy(BUNDLE)
        changed["properties"]["attestations"]["items"]["properties"]["signatures"]["minItems"] = 0
        with self.assertRaises(ValueError):
            validate(VECTORS, PREDICATE, changed, VECTOR_SCHEMA)

    def test_payload_tampering_has_hard_reject_vector(self):
        case = next(x for x in VECTORS["vectors"] if x["id"] == "VSA-SIG-001")
        self.assertEqual(case["expected"]["decision"], "reject")

    def test_cross_tenant_binding_has_hard_reject_vector(self):
        case = next(x for x in VECTORS["vectors"] if x["id"] == "VSA-BIND-001")
        self.assertEqual(case["expected"]["decision"], "reject")

    def test_valid_unknown_effect_cannot_be_accepted(self):
        case = next(x for x in VECTORS["vectors"] if x["id"] == "VSA-SEM-001")
        self.assertEqual(case["expected"]["decision"], "indeterminate")

    def test_predicate_schema_cannot_drop_attempt_id(self):
        changed = copy.deepcopy(PREDICATE)
        changed["properties"]["binding"]["required"].remove("attempt_id")
        with self.assertRaises(ValueError):
            validate(VECTORS, changed, BUNDLE, VECTOR_SCHEMA)

    def test_predicate_schema_cannot_drop_nonce(self):
        changed = copy.deepcopy(PREDICATE)
        changed["properties"]["freshness"]["required"].remove("nonce")
        with self.assertRaises(ValueError):
            validate(VECTORS, changed, BUNDLE, VECTOR_SCHEMA)

    def test_conformance_schema_cannot_shrink_inventory(self):
        changed = copy.deepcopy(VECTOR_SCHEMA)
        changed["properties"]["vectors"]["maxItems"] = 32
        with self.assertRaises(ValueError):
            validate(VECTORS, PREDICATE, BUNDLE, changed)


if __name__ == "__main__":
    unittest.main()
