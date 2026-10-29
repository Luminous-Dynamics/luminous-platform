from __future__ import annotations

import hashlib
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from verify_attested_evidence_bundle import (
    PROBES,
    build_gh_command,
    validate_verified_output,
    verify_attestation_bundle_file,
    verify_attested_bundle,
)

EXPECTED_SHA = "a" * 40
EXPECTED_RUN_ID = "1234567890"
EXPECTED_REPO_ID = "1407662539"
WORKFLOW_SHA = "b" * 40
STEP_OUTCOMES = {
    "format": "success",
    "rust_tests": "success",
    "runtime_test": "success",
    "clippy": "success",
}


def make_evidence(root: Path, probe: str = "landlock") -> Path:
    contract = PROBES[probe]
    if probe == "landlock":
        receipt_name = "landlock-preopened-fd-result.jsonl"
        receipt = {
            "schema": "luminous.landlock-preopened-fd-result.v1",
            "status": "pass",
            "checkout_sha": EXPECTED_SHA,
            "kernel_release": "7.0.0-1017-azure",
            "landlock_abi": 8,
            "preopened_write_succeeded_after_landlock": True,
            "parent_oracle_observed_preopened_write": True,
            "cloexec_descriptor_absent_after_exec": True,
            "new_path_write_open_denied_after_landlock": True,
            "parent_oracle_safe_control_unchanged": True,
            "parent_original_fd_cloexec": True,
            "full_descriptor_hygiene_coverage": False,
            "scope": "preopened_writable_fd_ordering_microtest",
        }
    else:
        receipt_name = "fd-hygiene-result.jsonl"
        receipt = {
            "schema": "luminous.fd-inheritance-boundary-result.v1",
            "status": "pass",
            "checkout_sha": EXPECTED_SHA,
            "kernel_release": "7.0.0-1017-azure",
            "negative_control_inherited_fd_readable": True,
            "cloexec_control_descriptor_absent_after_exec": True,
            "parent_descriptor_survived_marking": True,
            "parent_descriptor_cloexec_flag_set": True,
            "close_range_unshare_enabled": True,
            "close_range_cloexec_enabled": True,
            "scope": "exec_boundary_descriptor_hygiene_probe_only",
        }

    root.mkdir(parents=True, exist_ok=True)
    receipt_bytes = (
        json.dumps(receipt, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode("utf-8")
    (root / receipt_name).write_bytes(receipt_bytes)
    context = {
        "workflow_run_id": EXPECTED_RUN_ID,
        "workflow_run_attempt": "1",
        "event_name": "push",
        "workflow": contract["workflow"],
        "expected_checkout_sha": EXPECTED_SHA,
        "format_outcome": "success",
        "rust_test_outcome": "success",
        "runtime_test_outcome": "success",
        "clippy_outcome": "success",
    }
    (root / "run-context.txt").write_text(
        "".join(f"{key}={value}\n" for key, value in context.items()),
        encoding="utf-8",
    )
    manifest = {
        "schema": "luminous.ci-evidence-bundle-manifest.v1",
        "status": "candidate_pass_for_declared_microtest",
        "workflow": contract["workflow"],
        "workflow_sha": WORKFLOW_SHA,
        "workflow_run_id": EXPECTED_RUN_ID,
        "workflow_run_attempt": "1",
        "event_name": "push",
        "runner_os": "Linux",
        "runner_arch": "X64",
        "expected_checkout_sha": EXPECTED_SHA,
        "step_outcomes": STEP_OUTCOMES,
        "receipt_path": receipt_name,
        "receipt_sha256": hashlib.sha256(receipt_bytes).hexdigest(),
        "receipt_status": "pass",
        "receipt_schema": receipt["schema"],
        "kernel_release": receipt["kernel_release"],
        "landlock_abi": receipt.get("landlock_abi"),
        "receipt_parse_error": None,
        "claim_ceiling": contract["claim_ceiling"],
        "manifest_authenticity": "not_signed; checksum verifies bundle consistency only",
    }
    (root / "evidence-manifest.json").write_text(
        json.dumps(manifest, sort_keys=True, separators=(",", ":")),
        encoding="utf-8",
    )
    lines = []
    for filename in (receipt_name, "run-context.txt", "evidence-manifest.json"):
        digest = hashlib.sha256((root / filename).read_bytes()).hexdigest()
        lines.append(f"{digest}  {filename}\n")
    (root / "SHA256SUMS").write_text("".join(lines), encoding="utf-8")
    return root


def make_attestation_dir(root: Path, payload: bytes = b"synthetic-sigstore-bundle") -> Path:
    root.mkdir(parents=True, exist_ok=True)
    (root / "evidence-manifest.sigstore.json").write_bytes(payload)
    digest = hashlib.sha256(payload).hexdigest()
    (root / "SHA256SUMS").write_text(
        f"{digest}  evidence-manifest.sigstore.json\n",
        encoding="utf-8",
    )
    return root


def gh_output_for(manifest_path: Path, repository_id: str = EXPECTED_REPO_ID) -> str:
    digest = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
    payload = [{
        "attestation": {},
        "verificationResult": {
            "signature": {
                "certificate": {
                    "sourceRepositoryIdentifier": repository_id,
                },
            },
            "statement": {
                "predicateType": "https://slsa.dev/provenance/v1",
                "subject": [{
                    "name": "evidence-manifest.json",
                    "digest": {"sha256": digest},
                }],
            },
        },
    }]
    return json.dumps(payload)


class AttestedEvidenceVerifierTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.evidence_dir = make_evidence(self.root / "evidence", "landlock")
        self.attestation_dir = make_attestation_dir(self.root / "attestation")
        self.trusted_root = self.root / "trusted_root.jsonl"
        self.trusted_root.write_text('{"trusted_root":"synthetic-test-only"}\n', encoding="utf-8")
        self.manifest_path = self.evidence_dir / "evidence-manifest.json"

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_valid_verified_output_binds_repo_and_manifest_digest(self) -> None:
        result = validate_verified_output(
            gh_output_for(self.manifest_path),
            self.manifest_path,
            EXPECTED_REPO_ID,
        )
        self.assertEqual(result["repository_id"], EXPECTED_REPO_ID)
        self.assertEqual(
            result["subject_sha256"],
            hashlib.sha256(self.manifest_path.read_bytes()).hexdigest(),
        )

    def test_wrong_repository_id_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "expected repository ID"):
            validate_verified_output(
                gh_output_for(self.manifest_path, "999999"),
                self.manifest_path,
                EXPECTED_REPO_ID,
            )

    def test_wrong_subject_digest_is_rejected(self) -> None:
        payload = [{
            "verificationResult": {
                "signature": {"certificate": {"sourceRepositoryIdentifier": EXPECTED_REPO_ID}},
                "statement": {
                    "predicateType": "https://slsa.dev/provenance/v1",
                    "subject": [{"digest": {"sha256": "0" * 64}}],
                },
            },
        }]
        with self.assertRaisesRegex(ValueError, "manifest digest"):
            validate_verified_output(json.dumps(payload), self.manifest_path, EXPECTED_REPO_ID)

    def test_wrong_predicate_type_is_rejected(self) -> None:
        payload = json.loads(gh_output_for(self.manifest_path))
        payload[0]["verificationResult"]["statement"]["predicateType"] = "https://example.test/other"
        with self.assertRaisesRegex(ValueError, "expected repository ID and manifest digest"):
            validate_verified_output(json.dumps(payload), self.manifest_path, EXPECTED_REPO_ID)

    def test_empty_or_malformed_gh_output_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "no verified attestations"):
            validate_verified_output("[]", self.manifest_path, EXPECTED_REPO_ID)
        with self.assertRaisesRegex(ValueError, "valid JSON"):
            validate_verified_output("{", self.manifest_path, EXPECTED_REPO_ID)

    def test_attestation_directory_has_exact_file_inventory(self) -> None:
        (self.attestation_dir / "unexpected.txt").write_text("x", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "exactly the bundle and SHA256SUMS"):
            verify_attestation_bundle_file(self.attestation_dir)

    def test_attestation_bundle_digest_is_checked(self) -> None:
        bundle = self.attestation_dir / "evidence-manifest.sigstore.json"
        bundle.write_bytes(b"modified")
        with self.assertRaisesRegex(ValueError, "checksum mismatch"):
            verify_attestation_bundle_file(self.attestation_dir)

    def test_command_binds_source_and_signer(self) -> None:
        command = build_gh_command(
            gh_executable="gh",
            manifest_path=self.manifest_path,
            attestation_bundle=self.attestation_dir / "evidence-manifest.sigstore.json",
            trusted_root=self.trusted_root,
            expected_checkout_sha=EXPECTED_SHA,
            probe="landlock",
        )
        self.assertIn("--source-digest", command)
        self.assertIn(EXPECTED_SHA, command)
        self.assertIn("--source-ref", command)
        self.assertIn("refs/heads/main", command)
        self.assertIn("--deny-self-hosted-runners", command)
        self.assertIn(PROBES["landlock"]["signer_workflow"], command)
        self.assertIn("https://slsa.dev/provenance/v1", command)

    def test_valid_signed_bundle_verifies_end_to_end_with_mocked_gh(self) -> None:
        output = gh_output_for(self.manifest_path)
        with patch(
            "verify_attested_evidence_bundle.subprocess.run",
            return_value=subprocess.CompletedProcess([], 0, output, ""),
        ) as run:
            report = verify_attested_bundle(
                evidence_dir=self.evidence_dir,
                attestation_dir=self.attestation_dir,
                trusted_root=self.trusted_root,
                expected_checkout_sha=EXPECTED_SHA,
                expected_run_id=EXPECTED_RUN_ID,
                expected_repo_id=EXPECTED_REPO_ID,
                probe="landlock",
            )
        self.assertEqual(report["status"], "verified_candidate_pass_for_declared_microtest")
        self.assertEqual(report["repository_id"], EXPECTED_REPO_ID)
        self.assertEqual(report["claim_ceiling"], PROBES["landlock"]["claim_ceiling"])
        command = run.call_args.args[0]
        self.assertIn("--signer-workflow", command)
        self.assertIn(PROBES["landlock"]["signer_workflow"], command)

    def test_nonzero_gh_verification_is_rejected(self) -> None:
        with patch(
            "verify_attested_evidence_bundle.subprocess.run",
            return_value=subprocess.CompletedProcess([], 1, "", "signature invalid"),
        ):
            with self.assertRaisesRegex(ValueError, "GitHub CLI rejected"):
                verify_attested_bundle(
                    evidence_dir=self.evidence_dir,
                    attestation_dir=self.attestation_dir,
                    trusted_root=self.trusted_root,
                    expected_checkout_sha=EXPECTED_SHA,
                    expected_run_id=EXPECTED_RUN_ID,
                    expected_repo_id=EXPECTED_REPO_ID,
                    probe="landlock",
                )

    def test_wrong_workflow_for_selected_probe_is_rejected(self) -> None:
        manifest_file = self.evidence_dir / "evidence-manifest.json"
        manifest = json.loads(manifest_file.read_text(encoding="utf-8"))
        manifest["workflow"] = "unrelated workflow"
        manifest_file.write_text(json.dumps(manifest, sort_keys=True, separators=(",", ":")), encoding="utf-8")
        context_path = self.evidence_dir / "run-context.txt"
        context_path.write_text(
            context_path.read_text(encoding="utf-8").replace(
                "workflow=" + PROBES["landlock"]["workflow"],
                "workflow=unrelated workflow",
            ),
            encoding="utf-8",
        )
        self._refresh_context_checksum()
        self._refresh_manifest_checksum()
        with self.assertRaisesRegex(ValueError, "manifest workflow"):
            verify_attested_bundle(
                evidence_dir=self.evidence_dir,
                attestation_dir=self.attestation_dir,
                trusted_root=self.trusted_root,
                expected_checkout_sha=EXPECTED_SHA,
                expected_run_id=EXPECTED_RUN_ID,
                expected_repo_id=EXPECTED_REPO_ID,
                probe="landlock",
            )

    def test_unqualified_bundle_never_reaches_signature_verifier(self) -> None:
        manifest_file = self.evidence_dir / "evidence-manifest.json"
        manifest = json.loads(manifest_file.read_text(encoding="utf-8"))
        manifest["status"] = "not_qualified"
        manifest["step_outcomes"]["clippy"] = "failure"
        manifest_file.write_text(json.dumps(manifest, sort_keys=True, separators=(",", ":")), encoding="utf-8")
        context_path = self.evidence_dir / "run-context.txt"
        context = context_path.read_text(encoding="utf-8").replace(
            "clippy_outcome=success",
            "clippy_outcome=failure",
        )
        context_path.write_text(context, encoding="utf-8")
        self._refresh_manifest_checksum()
        self._refresh_context_checksum()
        with patch("verify_attested_evidence_bundle.subprocess.run") as run:
            with self.assertRaisesRegex(ValueError, "not qualified"):
                verify_attested_bundle(
                    evidence_dir=self.evidence_dir,
                    attestation_dir=self.attestation_dir,
                    trusted_root=self.trusted_root,
                    expected_checkout_sha=EXPECTED_SHA,
                    expected_run_id=EXPECTED_RUN_ID,
                    expected_repo_id=EXPECTED_REPO_ID,
                    probe="landlock",
                )
        run.assert_not_called()

    def _refresh_context_checksum(self) -> None:
        checksum_path = self.evidence_dir / "SHA256SUMS"
        entries = checksum_path.read_text(encoding="utf-8").splitlines()
        updated = []
        for line in entries:
            digest, filename = line.split("  ", 1)
            if filename == "run-context.txt":
                digest = hashlib.sha256((self.evidence_dir / filename).read_bytes()).hexdigest()
            updated.append(f"{digest}  {filename}\n")
        checksum_path.write_text("".join(updated), encoding="utf-8")

    def _refresh_manifest_checksum(self) -> None:
        checksum_path = self.evidence_dir / "SHA256SUMS"
        entries = checksum_path.read_text(encoding="utf-8").splitlines()
        updated = []
        for line in entries:
            digest, filename = line.split("  ", 1)
            if filename == "evidence-manifest.json":
                digest = hashlib.sha256((self.evidence_dir / filename).read_bytes()).hexdigest()
            updated.append(f"{digest}  {filename}\n")
        checksum_path.write_text("".join(updated), encoding="utf-8")


if __name__ == "__main__":
    unittest.main()
