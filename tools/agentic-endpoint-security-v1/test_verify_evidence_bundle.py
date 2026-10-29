import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from verify_evidence_bundle import verify_bundle


EXPECTED_SHA = "a" * 40
EXPECTED_RUN_ID = "1234567890"
WORKFLOW_SHA = "b" * 40
STEP_KEYS = ("format", "rust_tests", "runtime_test", "clippy")


def receipt_for(schema):
    common = {
        "schema": schema,
        "status": "pass",
        "checkout_sha": EXPECTED_SHA,
        "kernel_release": "6.11.0-1018-azure",
    }
    if schema == "luminous.landlock-preopened-fd-result.v1":
        common.update({
            "landlock_abi": 3,
            "preopened_write_succeeded_after_landlock": True,
            "parent_oracle_observed_preopened_write": True,
            "cloexec_descriptor_absent_after_exec": True,
            "new_path_write_open_denied_after_landlock": True,
            "parent_oracle_safe_control_unchanged": True,
            "parent_original_fd_cloexec": True,
            "full_descriptor_hygiene_coverage": False,
            "scope": "preopened_writable_fd_ordering_microtest",
        })
    else:
        common.update({
            "negative_control_inherited_fd_readable": True,
            "cloexec_control_descriptor_absent_after_exec": True,
            "parent_descriptor_survived_marking": True,
            "parent_descriptor_cloexec_flag_set": True,
            "close_range_unshare_enabled": True,
            "close_range_cloexec_enabled": True,
            "scope": "exec_boundary_descriptor_hygiene_probe_only",
        })
    return common


class EvidenceBundleVerifierTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)

    def tearDown(self):
        self.temp.cleanup()

    def write_bundle(self, receipt, outcomes=None, manifest_overrides=None,
                     context_overrides=None):
        outcomes = dict.fromkeys(STEP_KEYS, "success") if outcomes is None else dict(outcomes)
        result_name = (
            "landlock-preopened-fd-result.jsonl"
            if receipt.get("schema") == "luminous.landlock-preopened-fd-result.v1"
            else "fd-hygiene-result.jsonl"
        )
        result_bytes = (json.dumps(receipt, sort_keys=True, separators=(",", ":")) + "\n").encode()
        (self.root / result_name).write_bytes(result_bytes)

        context = {
            "workflow_run_id": EXPECTED_RUN_ID,
            "workflow_run_attempt": "1",
            "event_name": "pull_request",
            "workflow": "Landlock pre-opened descriptor authority probe",
            "expected_checkout_sha": EXPECTED_SHA,
            "format_outcome": outcomes["format"],
            "rust_test_outcome": outcomes["rust_tests"],
            "runtime_test_outcome": outcomes["runtime_test"],
            "clippy_outcome": outcomes["clippy"],
        }
        if context_overrides:
            context.update(context_overrides)
        (self.root / "run-context.txt").write_text(
            "".join(f"{key}={value}\n" for key, value in context.items()),
            encoding="utf-8",
        )

        spec = {
            "luminous.landlock-preopened-fd-result.v1": {
                "scope": "preopened_writable_fd_ordering_microtest",
                "true_fields": (
                    "preopened_write_succeeded_after_landlock",
                    "parent_oracle_observed_preopened_write",
                    "cloexec_descriptor_absent_after_exec",
                    "new_path_write_open_denied_after_landlock",
                    "parent_oracle_safe_control_unchanged",
                    "parent_original_fd_cloexec",
                ),
            },
            "luminous.fd-inheritance-boundary-result.v1": {
                "scope": "exec_boundary_descriptor_hygiene_probe_only",
                "true_fields": (
                    "negative_control_inherited_fd_readable",
                    "cloexec_control_descriptor_absent_after_exec",
                    "parent_descriptor_survived_marking",
                    "parent_descriptor_cloexec_flag_set",
                    "close_range_unshare_enabled",
                    "close_range_cloexec_enabled",
                ),
            },
        }.get(receipt.get("schema"), {"scope": None, "true_fields": ()})
        valid = (
            receipt.get("status") == "pass"
            and receipt.get("checkout_sha") == EXPECTED_SHA
            and receipt.get("kernel_release") not in (None, "", "unknown")
            and receipt.get("scope") == spec["scope"]
            and all(receipt.get(key) is True for key in spec["true_fields"])
            and all(value == "success" for value in outcomes.values())
        )
        if receipt.get("schema") == "luminous.landlock-preopened-fd-result.v1":
            valid = valid and type(receipt.get("landlock_abi")) is int and receipt["landlock_abi"] >= 3
            valid = valid and receipt.get("full_descriptor_hygiene_coverage") is False
        manifest = {
            "schema": "luminous.ci-evidence-bundle-manifest.v1",
            "status": "candidate_pass_for_declared_microtest" if valid else "not_qualified",
            "workflow": context["workflow"],
            "workflow_sha": WORKFLOW_SHA,
            "workflow_run_id": EXPECTED_RUN_ID,
            "workflow_run_attempt": "1",
            "event_name": "pull_request",
            "runner_os": "Linux",
            "runner_arch": "X64",
            "expected_checkout_sha": EXPECTED_SHA,
            "step_outcomes": outcomes,
            "receipt_path": result_name,
            "receipt_sha256": hashlib.sha256(result_bytes).hexdigest(),
            "receipt_status": receipt.get("status"),
            "receipt_schema": receipt.get("schema"),
            "kernel_release": receipt.get("kernel_release"),
            "landlock_abi": receipt.get("landlock_abi"),
            "receipt_parse_error": None,
            "claim_ceiling": "one operation and one exec boundary; not full descriptor hygiene",
            "manifest_authenticity": "not_signed; checksum verifies bundle consistency only",
        }
        if manifest_overrides:
            manifest.update(manifest_overrides)
        (self.root / "evidence-manifest.json").write_text(
            json.dumps(manifest, sort_keys=True, separators=(",", ":")),
            encoding="utf-8",
        )
        names = (result_name, "run-context.txt", "evidence-manifest.json")
        checksum_lines = []
        for name in names:
            checksum = hashlib.sha256((self.root / name).read_bytes()).hexdigest()
            checksum_lines.append(f"{checksum}  {name}\n")
        (self.root / "SHA256SUMS").write_text("".join(checksum_lines), encoding="utf-8")
        return self.root

    def test_valid_landlock_bundle_qualifies_for_its_declared_scope(self):
        self.write_bundle(receipt_for("luminous.landlock-preopened-fd-result.v1"))
        report = verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)
        self.assertEqual(report["status"], "candidate_pass_for_declared_microtest")
        self.assertEqual(report["receipt_status"], "pass")

    def test_valid_close_range_bundle_qualifies_for_its_declared_scope(self):
        self.write_bundle(receipt_for("luminous.fd-inheritance-boundary-result.v1"))
        report = verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)
        self.assertEqual(report["status"], "candidate_pass_for_declared_microtest")

    def test_expected_checkout_sha_is_external_and_must_match(self):
        self.write_bundle(receipt_for("luminous.fd-inheritance-boundary-result.v1"))
        with self.assertRaisesRegex(ValueError, "manifest checkout SHA"):
            verify_bundle(self.root, "c" * 40, EXPECTED_RUN_ID)

    def test_expected_run_id_is_external_and_must_match(self):
        self.write_bundle(receipt_for("luminous.fd-inheritance-boundary-result.v1"))
        with self.assertRaisesRegex(ValueError, "manifest run ID"):
            verify_bundle(self.root, EXPECTED_SHA, "987654321")

    def test_receipt_mutation_breaks_checksum(self):
        self.write_bundle(receipt_for("luminous.fd-inheritance-boundary-result.v1"))
        path = self.root / "fd-hygiene-result.jsonl"
        path.write_bytes(path.read_bytes() + b" ")
        with self.assertRaisesRegex(ValueError, "checksum mismatch"):
            verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)

    def test_checksum_list_cannot_omit_a_file(self):
        self.write_bundle(receipt_for("luminous.fd-inheritance-boundary-result.v1"))
        path = self.root / "SHA256SUMS"
        lines = path.read_text(encoding="utf-8").splitlines()
        path.write_text("\n".join(lines[:2]) + "\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "exactly one runtime receipt"):
            verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)

    def test_false_control_cannot_be_qualified(self):
        receipt = receipt_for("luminous.fd-inheritance-boundary-result.v1")
        receipt["cloexec_control_descriptor_absent_after_exec"] = False
        self.write_bundle(receipt)
        report = verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)
        self.assertEqual(report["status"], "not_qualified")

    def test_failed_clippy_cannot_be_qualified(self):
        self.write_bundle(
            receipt_for("luminous.fd-inheritance-boundary-result.v1"),
            outcomes={"format": "success", "rust_tests": "success",
                      "runtime_test": "success", "clippy": "failure"},
        )
        report = verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)
        self.assertEqual(report["status"], "not_qualified")

    def test_false_qualified_manifest_is_rejected(self):
        self.write_bundle(
            receipt_for("luminous.fd-inheritance-boundary-result.v1"),
            outcomes={"format": "success", "rust_tests": "success",
                      "runtime_test": "success", "clippy": "failure"},
            manifest_overrides={"status": "candidate_pass_for_declared_microtest"},
        )
        with self.assertRaisesRegex(ValueError, "does not follow receipt"):
            verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)

    def test_step_outcome_mismatch_with_context_is_rejected(self):
        self.write_bundle(
            receipt_for("luminous.fd-inheritance-boundary-result.v1"),
            context_overrides={"runtime_test_outcome": "failure"},
        )
        with self.assertRaisesRegex(ValueError, "outcomes disagree"):
            verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)

    def test_receipt_digest_field_is_checked_even_if_checksums_are_recomputed(self):
        self.write_bundle(
            receipt_for("luminous.fd-inheritance-boundary-result.v1"),
            manifest_overrides={"receipt_sha256": "0" * 64},
        )
        with self.assertRaisesRegex(ValueError, "receipt digest"):
            verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)

    def test_unsupported_receipt_schema_is_rejected(self):
        receipt = receipt_for("luminous.fd-inheritance-boundary-result.v1")
        receipt["schema"] = "luminous.unknown.v1"
        self.write_bundle(receipt)
        with self.assertRaisesRegex(ValueError, "unsupported runtime receipt schema"):
            verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)

    def test_not_emitted_receipt_is_validly_not_qualified(self):
        receipt = {
            "schema": "luminous.fd-inheritance-boundary-result.v1",
            "status": "not_emitted",
            "scope": "exec_boundary_descriptor_hygiene_probe_only",
        }
        self.write_bundle(receipt)
        report = verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)
        self.assertEqual(report["status"], "not_qualified")
        self.assertEqual(report["receipt_status"], "not_emitted")

    def test_manifest_must_disclose_checksums_are_not_authentication(self):
        self.write_bundle(
            receipt_for("luminous.fd-inheritance-boundary-result.v1"),
            manifest_overrides={"manifest_authenticity": "authenticated"},
        )
        with self.assertRaisesRegex(ValueError, "do not authenticate"):
            verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)

    def test_manifest_landlock_abi_must_match_receipt(self):
        self.write_bundle(
            receipt_for("luminous.landlock-preopened-fd-result.v1"),
            manifest_overrides={"landlock_abi": 9},
        )
        with self.assertRaisesRegex(ValueError, "ABI does not match"):
            verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)

    def test_extra_files_are_not_silently_ignored(self):
        self.write_bundle(receipt_for("luminous.fd-inheritance-boundary-result.v1"))
        (self.root / "unexpected.txt").write_text("extra", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "unexpected files"):
            verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)

    def test_pathlike_checksum_entries_are_rejected(self):
        self.write_bundle(receipt_for("luminous.fd-inheritance-boundary-result.v1"))
        path = self.root / "SHA256SUMS"
        text = path.read_text(encoding="utf-8").replace("fd-hygiene-result.jsonl", "../fd-hygiene-result.jsonl")
        path.write_text(text, encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "malformed"):
            verify_bundle(self.root, EXPECTED_SHA, EXPECTED_RUN_ID)


if __name__ == "__main__":
    unittest.main()
