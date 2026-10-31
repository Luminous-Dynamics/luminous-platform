import copy
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))

from check_profile import load_json, validate, validate_references


VECTOR_PATH = ROOT / "benchmarks" / "omsi-itops-profile-v0.1" / "conformance-vectors.json"
SCHEMA_PATH = ROOT / "benchmarks" / "omsi-itops-profile-v0.1" / "schema.json"
REFERENCES_PATH = ROOT / "benchmarks" / "omsi-itops-profile-v0.1" / "upstream-crosswalk.json"


class ITOPSProfileContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.vectors = load_json(VECTOR_PATH)
        cls.schema = load_json(SCHEMA_PATH)
        cls.references = load_json(REFERENCES_PATH)

    def assert_rejected(self, vectors=None, schema=None, references=None):
        with self.assertRaises(ValueError):
            validate(
                copy.deepcopy(self.vectors if vectors is None else vectors),
                copy.deepcopy(self.schema if schema is None else schema),
                copy.deepcopy(self.references if references is None else references),
            )

    def test_frozen_profile_vectors_and_upstream_references_pass(self):
        result = validate(self.vectors, self.schema, self.references)
        self.assertEqual(result, {
            "vector_count": 12,
            "opsbench_case_count": 20,
            "saga_fault_count": 12,
        })

    def test_duplicate_vector_id_is_rejected(self):
        changed = copy.deepcopy(self.vectors)
        changed["vectors"][1]["id"] = changed["vectors"][0]["id"]
        self.assert_rejected(vectors=changed)

    def test_unexpected_vector_id_is_rejected(self):
        changed = copy.deepcopy(self.vectors)
        changed["vectors"][0]["id"] = "ITOPS-FAKE-999"
        self.assert_rejected(vectors=changed)

    def test_unknown_opsbench_mapping_is_rejected(self):
        changed = copy.deepcopy(self.vectors)
        changed["vectors"][0]["maps_to"]["opsbench_case"] = "FAKE-999"
        self.assert_rejected(vectors=changed)

    def test_unknown_saga_fault_mapping_is_rejected(self):
        changed = copy.deepcopy(self.vectors)
        changed["vectors"][0]["maps_to"]["saga_fault"] = "FAKE-999"
        self.assert_rejected(vectors=changed)

    def test_unmapped_vector_is_rejected(self):
        changed = copy.deepcopy(self.vectors)
        changed["vectors"][0]["maps_to"] = {"opsbench_case": None, "saga_fault": None}
        self.assert_rejected(vectors=changed)

    def test_false_service_resolution_label_is_rejected(self):
        changed = copy.deepcopy(self.vectors)
        changed["vectors"][0]["expected"]["profile_disposition"] = "service_verified"
        self.assert_rejected(vectors=changed)

    def test_tenant_boundary_weakening_is_rejected(self):
        changed = copy.deepcopy(self.vectors)
        tenant = next(v for v in changed["vectors"] if v["id"] == "ITOPS-TENANT-001")
        tenant["expected"]["profile_disposition"] = "allowed"
        tenant["expected"]["action_authority"] = "cross_tenant_read"
        self.assert_rejected(vectors=changed)

    def test_current_state_readback_cannot_be_promoted_to_retry_proof(self):
        changed = copy.deepcopy(self.vectors)
        item = next(v for v in changed["vectors"] if v["id"] == "ITOPS-NOEFFECT-001")
        item["expected"]["profile_disposition"] = "retry_eligible"
        item["expected"]["action_authority"] = "retry_allowed"
        self.assert_rejected(vectors=changed)

    def test_forward_grant_cannot_authorize_compensation(self):
        changed = copy.deepcopy(self.vectors)
        item = next(v for v in changed["vectors"] if v["id"] == "ITOPS-COMP-001")
        item["expected"]["profile_disposition"] = "compensation_authorized"
        item["expected"]["action_authority"] = "forward_grant"
        self.assert_rejected(vectors=changed)

    def test_stale_or_wrong_plan_service_observation_cannot_resolve(self):
        changed = copy.deepcopy(self.vectors)
        item = next(v for v in changed["vectors"] if v["id"] == "ITOPS-SERVICE-001")
        item["expected"]["profile_disposition"] = "service_verified"
        self.assert_rejected(vectors=changed)

    def test_evaluator_only_visibility_cannot_be_weakened(self):
        changed = copy.deepcopy(self.vectors)
        changed["visibility"] = "runtime_input"
        self.assert_rejected(vectors=changed)

    def test_seeded_fault_or_answer_key_cannot_leak_into_sut_input(self):
        changed = copy.deepcopy(self.vectors)
        changed["vectors"][0]["input"]["seeded_fault"] = "hidden evaluator fact"
        self.assert_rejected(vectors=changed)

        changed = copy.deepcopy(self.vectors)
        changed["vectors"][0]["input"]["telemetry"] = {"answer_key": "healthy"}
        self.assert_rejected(vectors=changed)

    def test_opsbench_upstream_commit_and_blob_must_stay_pinned(self):
        changed = copy.deepcopy(self.references)
        changed["upstreams"][0]["commit"] = "main"
        self.assert_rejected(references=changed)

        changed = copy.deepcopy(self.references)
        changed["upstreams"][0]["blob_sha"] = "0" * 40
        self.assert_rejected(references=changed)

    def test_saga_upstream_commit_and_blob_must_stay_pinned(self):
        changed = copy.deepcopy(self.references)
        changed["upstreams"][1]["commit"] = "research/latest"
        self.assert_rejected(references=changed)

        changed = copy.deepcopy(self.references)
        changed["upstreams"][1]["blob_sha"] = "f" * 40
        self.assert_rejected(references=changed)

    def test_cross_benchmark_metric_separation_cannot_be_removed(self):
        changed = copy.deepcopy(self.references)
        changed["comparability_rules"] = [
            "all scores are directly comparable"
        ]
        self.assert_rejected(references=changed)

    def test_schema_must_freeze_exact_vector_count_and_visibility(self):
        changed = copy.deepcopy(self.schema)
        changed["properties"]["vectors"]["minItems"] = 11
        self.assert_rejected(schema=changed)

        changed = copy.deepcopy(self.schema)
        changed["properties"]["visibility"]["const"] = "sut_input"
        self.assert_rejected(schema=changed)

    def test_duplicate_json_object_keys_are_rejected(self):
        import tempfile
        with tempfile.TemporaryDirectory() as temporary_directory:
            path = Path(temporary_directory) / "duplicate.json"
            path.write_text('{"profile":"a","profile":"b"}', encoding="utf-8")
            with self.assertRaises(ValueError):
                load_json(path)

    def test_reference_id_sets_are_unique_and_complete(self):
        changed = copy.deepcopy(self.references)
        changed["opsbench_case_ids"].append(changed["opsbench_case_ids"][0])
        with self.assertRaises(ValueError):
            validate_references(changed)


if __name__ == "__main__":
    unittest.main()
