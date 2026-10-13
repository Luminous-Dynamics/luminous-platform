from __future__ import annotations

import importlib.util
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "tools/acs-bridge-validation/verify_schema_lock.py"
SPEC = importlib.util.spec_from_file_location("acs_schema_lock_verifier", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"unable to import schema-lock verifier from {SCRIPT}")
verifier = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = verifier
SPEC.loader.exec_module(verifier)


class VendoredSchemaLockTests(unittest.TestCase):
    def test_committed_schema_bundle_matches_lock(self) -> None:
        errors = verifier.verify_lock(ROOT)
        self.assertEqual([], errors, "\\n".join(errors))

    def _copy_fixture(self) -> Path:
        temp = Path(tempfile.mkdtemp(prefix="acs-schema-lock-test-"))
        shutil.copytree(ROOT / "vendor/agent-control-standard", temp / "vendor/agent-control-standard")
        lock_target = temp / verifier.LOCK_PATH
        lock_target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / verifier.LOCK_PATH, lock_target)
        return temp

    def test_unsigned_upstream_pin_cannot_be_marked_verified(self) -> None:
        temp = self._copy_fixture()
        try:
            lock_path = temp / verifier.LOCK_PATH
            lock = json.loads(lock_path.read_text(encoding="utf-8"))
            lock["upstream_commit_signature"]["verified"] = True
            lock_path.write_text(json.dumps(lock), encoding="utf-8")
            errors = verifier.verify_lock(temp)
            self.assertTrue(any("unsigned/unverified" in error for error in errors), errors)
        finally:
            shutil.rmtree(temp)

    def test_lock_must_cover_all_upstream_schema_files(self) -> None:
        temp = self._copy_fixture()
        try:
            lock_path = temp / verifier.LOCK_PATH
            lock = json.loads(lock_path.read_text(encoding="utf-8"))
            lock["files"].pop()
            lock_path.write_text(json.dumps(lock), encoding="utf-8")
            errors = verifier.verify_lock(temp)
            self.assertTrue(any("exactly 45 locked upstream files" in error for error in errors), errors)
            self.assertTrue(any("all 43 JSON files" in error for error in errors), errors)
        finally:
            shutil.rmtree(temp)

    def test_modified_byte_breaks_git_blob_lock(self) -> None:
        temp = self._copy_fixture()
        try:
            target = temp / "vendor/agent-control-standard/specification/v0.1.0/request-envelope.json"
            target.write_bytes(target.read_bytes() + b"\\n")
            errors = verifier.verify_lock(temp)
            self.assertTrue(any("Git blob SHA mismatch" in error for error in errors), errors)
        finally:
            shutil.rmtree(temp)

    def test_missing_transitive_schema_is_rejected(self) -> None:
        temp = self._copy_fixture()
        try:
            (temp / "vendor/agent-control-standard/specification/v0.1.0/ask-details.json").unlink()
            errors = verifier.verify_lock(temp)
            self.assertTrue(any("missing or unreadable" in error for error in errors), errors)
            self.assertTrue(any("unresolved/off-package $ref" in error for error in errors), errors)
        finally:
            shutil.rmtree(temp)

    def test_external_ref_is_not_allowed_for_offline_validation(self) -> None:
        temp = self._copy_fixture()
        try:
            target = temp / "vendor/agent-control-standard/specification/v0.1.0/ask-details.json"
            document = json.loads(target.read_text(encoding="utf-8"))
            document["$ref"] = "https://example.invalid/untrusted.json"
            target.write_text(json.dumps(document), encoding="utf-8")
            errors = verifier.verify_lock(temp)
            self.assertTrue(any("external $ref prohibited" in error for error in errors), errors)
        finally:
            shutil.rmtree(temp)

    def test_extra_unlocked_file_is_rejected(self) -> None:
        temp = self._copy_fixture()
        try:
            extra = temp / "vendor/agent-control-standard/specification/v0.1.0/unlocked.json"
            extra.write_text("{}", encoding="utf-8")
            errors = verifier.verify_lock(temp)
            self.assertTrue(any("unlocked files exist" in error for error in errors), errors)
        finally:
            shutil.rmtree(temp)


if __name__ == "__main__":
    unittest.main()
