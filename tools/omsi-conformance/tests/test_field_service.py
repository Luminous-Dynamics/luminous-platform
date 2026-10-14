import copy
import json
import sys
import unittest
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import omsi_validate as core
import field_service_validate as field_service

FIXTURES = ROOT / "fixtures"
CASES_PATH = FIXTURES / "field-service-cases.json"
NOW = datetime(2026, 10, 14, 12, 0, 0, tzinfo=timezone.utc)


def load_json(path):
    return core.load(path)


def apply_mutation(document, mutation):
    path = mutation["path"]
    parts = [part.replace("~1", "/").replace("~0", "~") for part in path.lstrip("/").split("/")]
    parent = document
    for part in parts[:-1]:
        parent = parent[int(part)] if isinstance(parent, list) else parent[part]
    leaf = parts[-1]
    operation = mutation["op"]
    if operation == "append":
        target = parent[int(leaf)] if isinstance(parent, list) else parent[leaf]
        target.append(copy.deepcopy(mutation["value"]))
    elif operation == "replace":
        if isinstance(parent, list):
            parent[int(leaf)] = copy.deepcopy(mutation["value"])
        else:
            if leaf not in parent:
                raise AssertionError(f"test mutation target does not exist: {path}")
            parent[leaf] = copy.deepcopy(mutation["value"])
    elif operation == "add":
        if isinstance(parent, list):
            if leaf == "-":
                parent.append(copy.deepcopy(mutation["value"]))
            else:
                parent.insert(int(leaf), copy.deepcopy(mutation["value"]))
        else:
            parent[leaf] = copy.deepcopy(mutation["value"])
    else:
        raise AssertionError(f"unsupported test mutation operation: {operation}")


class TestFieldServiceProfile(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.cases = load_json(CASES_PATH)
        field_service.Draft202012Validator.check_schema(load_json(field_service.SCHEMA_PATH))

    def test_schema_rejects_unrecognized_fields(self):
        record = load_json(FIXTURES / "field-service-plumbing-hvac.json")
        record["production_authorized"] = True
        with self.assertRaisesRegex(core.ConformanceError, "field-service schema error"):
            field_service.validate_record(record, NOW)

    def test_both_profession_fixtures_pass_semantic_checks(self):
        self.assertGreaterEqual(len(self.cases["positive_fixtures"]), 2)
        for fixture_name in self.cases["positive_fixtures"]:
            with self.subTest(fixture=fixture_name):
                field_service.validate_record(load_json(FIXTURES / fixture_name), NOW)

    def test_adversarial_manifest_is_nonempty_and_ids_are_unique(self):
        cases = self.cases["negative_cases"]
        self.assertGreaterEqual(len(cases), 12)
        ids = [case["id"] for case in cases]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertTrue(all(case.get("expected_error") for case in cases))

    def test_all_declared_negative_cases_are_rejected_for_expected_reason(self):
        for case in self.cases["negative_cases"]:
            with self.subTest(case=case["id"]):
                record = load_json(FIXTURES / case["fixture"])
                for mutation in case["mutations"]:
                    apply_mutation(record, mutation)
                with self.assertRaises(core.ConformanceError) as raised:
                    field_service.validate_record(record, NOW)
                self.assertIn(case["expected_error"], str(raised.exception))

    def test_explicit_evaluation_clock_is_mandatory_and_timezone_aware(self):
        record = load_json(FIXTURES / "field-service-plumbing-hvac.json")
        with self.assertRaisesRegex(core.ConformanceError, "evaluation time must include a timezone"):
            field_service.validate_record(record, datetime(2026, 10, 14, 12, 0, 0))


if __name__ == "__main__":
    unittest.main()
