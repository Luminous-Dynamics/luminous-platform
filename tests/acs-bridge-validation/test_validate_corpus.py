from __future__ import annotations

import copy
import importlib.util
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VALIDATOR_PATH = ROOT / "tools/acs-bridge-validation/validate_corpus.py"
SPEC = importlib.util.spec_from_file_location("acs_bridge_validator", VALIDATOR_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"unable to import validator from {VALIDATOR_PATH}")
validator = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = validator
SPEC.loader.exec_module(validator)

SCHEMA = validator.load_json(validator.SCHEMA_PATH)
CORPUS = validator.load_json(validator.CORPUS_PATH)


class StrictJsonTests(unittest.TestCase):
    def test_duplicate_object_key_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "duplicate JSON object key"):
            validator.loads_strict('{"phase":"planned","phase":"promoted"}')

    def test_non_standard_nan_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "non-standard JSON numeric constant"):
            validator.loads_strict('{"count": NaN}')

    def test_malformed_json_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "invalid strict JSON"):
            validator.loads_strict('{"cases": [')


class CorpusMutationTests(unittest.TestCase):
    def test_committed_corpus_passes(self) -> None:
        self.assertEqual([], validator.validate_documents(SCHEMA, CORPUS))

    def test_missing_case_is_rejected(self) -> None:
        changed = copy.deepcopy(CORPUS)
        changed["cases"].pop()
        self.assertTrue(validator.validate_documents(SCHEMA, changed))

    def test_duplicate_case_id_is_rejected(self) -> None:
        changed = copy.deepcopy(CORPUS)
        changed["cases"][1]["id"] = changed["cases"][0]["id"]
        self.assertTrue(validator.validate_documents(SCHEMA, changed))

    def test_no_effect_control_cannot_allow_mutation(self) -> None:
        changed = copy.deepcopy(CORPUS)
        changed["cases"][17]["max_mutating_effects"] = 1  # BR-018
        errors = validator.validate_documents(SCHEMA, changed)
        self.assertTrue(any("must cap mutating effects at zero" in error for error in errors))

    def test_acs_allow_is_not_nixward_authority(self) -> None:
        changed = copy.deepcopy(CORPUS)
        changed["cases"][2]["expected_bridge_outcome"] = "ALLOW"  # BR-003
        errors = validator.validate_documents(SCHEMA, changed)
        self.assertTrue(any("BR-003 safety invariant" in error for error in errors))

    def test_ambiguous_effect_must_block_retry_until_reconciled(self) -> None:
        changed = copy.deepcopy(CORPUS)
        changed["cases"][22]["expected_bridge_outcome"] = "RECONCILE"  # BR-023
        errors = validator.validate_documents(SCHEMA, changed)
        self.assertTrue(any("BR-023 safety invariant" in error for error in errors))

    def test_missing_nixward_dependency_link_is_rejected(self) -> None:
        changed = copy.deepcopy(CORPUS)
        changed["related_existing_work"] = [
            item for item in changed["related_existing_work"] if item["issue"] != 75
        ]
        errors = validator.validate_documents(SCHEMA, changed)
        self.assertTrue(any("issues #63, #70, and #75" in error for error in errors))

    def test_unknown_top_level_field_is_rejected(self) -> None:
        changed = copy.deepcopy(CORPUS)
        changed["qualification_status"] = "PASS"
        self.assertTrue(validator.validate_documents(SCHEMA, changed))

    def test_schema_meta_validation_rejects_broken_schema(self) -> None:
        changed_schema = {"$schema": SCHEMA["$schema"], "type": "not-a-json-type"}
        errors = validator.validate_documents(changed_schema, CORPUS)
        self.assertTrue(any("not valid Draft 2020-12" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
