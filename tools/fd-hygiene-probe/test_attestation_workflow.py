#!/usr/bin/env python3
"""Regression tests for the trusted-main-only evidence-attestation workflow.

These tests inspect the workflow contract; they do not generate or cryptographically
verify a Sigstore attestation. The runtime receipt remains a narrow micro-test claim.
"""
from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW_PATH = ROOT / ".github/workflows/fd-hygiene-probe.yml"
ATTEST_SHA = "1e69f48acb82d1966a394da916b4c1698aa569d6"
DOWNLOAD_SHA = "634f93cb2916e3fdff6788551b99b062d0335ce0"
UPLOAD_SHA = "cf430e030ddbb5b0abf93d22962f4752f3646cd9"
REQUIRED_RECEIPT_FIELDS = (
    "negative_control_inherited_fd_readable",
    "cloexec_control_descriptor_absent_after_exec",
    "parent_descriptor_survived_marking",
    "parent_descriptor_cloexec_flag_set",
    "close_range_unshare_enabled",
    "close_range_cloexec_enabled",
)


def extract_attestation_job(workflow: str) -> str:
    match = re.search(r"(?m)^  attest-main-evidence:\s*$", workflow)
    if not match:
        return ""
    suffix = workflow[match.end():]
    next_job = re.search(r"(?m)^  [a-zA-Z0-9_-]+:\s*$", suffix)
    return suffix[:next_job.start()] if next_job else suffix


def validate_contract(workflow: str) -> list[str]:
    errors: list[str] = []
    if "jobs:\n" not in workflow:
        return ["missing jobs section"]
    pre_jobs = workflow.split("jobs:\n", 1)[0]
    if re.search(r"(?m)^\s+id-token:\s*write\s*$", pre_jobs):
        errors.append("OIDC signing permission leaks outside jobs")
    if re.search(r"(?m)^\s+attestations:\s*write\s*$", pre_jobs):
        errors.append("attestation-write permission leaks outside jobs")

    job = extract_attestation_job(workflow)
    if not job:
        return errors + ["missing isolated attestation job"]
    if "if: github.event_name == 'push' && github.ref == 'refs/heads/main'" not in job:
        errors.append("attestation job is not restricted to a push on main")
    if "needs: linux-fd-boundary" not in job:
        errors.append("attestation job does not depend on successful runtime-test job")

    permission_match = re.search(r"(?ms)^    permissions:\n(.*?)^    steps:\n", job)
    permissions = permission_match.group(1) if permission_match else ""
    expected_permissions = {
        "actions: read",
        "contents: read",
        "id-token: write",
        "attestations: write",
    }
    actual_permissions = {line.strip() for line in permissions.splitlines() if line.strip()}
    if actual_permissions != expected_permissions:
        errors.append("attestation job permissions are not the exact least-privilege set")

    for action in (
        "actions/download-artifact@" + DOWNLOAD_SHA,
        "actions/attest@" + ATTEST_SHA,
        "actions/upload-artifact@" + UPLOAD_SHA,
    ):
        if action not in job:
            errors.append("missing or unpinned action: " + action.split("@", 1)[0])

    if "subject-path: evidence/evidence-manifest.json" not in job:
        errors.append("attestation subject is not the evidence manifest")
    if "name: fd-hygiene-evidence-${{ github.run_id }}-${{ github.run_attempt }}" not in job:
        errors.append("attestation job does not download the exact run-and-attempt evidence artifact")
    if "name: fd-hygiene-attestation-${{ github.run_id }}-${{ github.run_attempt }}" not in job:
        errors.append("attestation artifact is not separate and run-bound")

    verify_step_match = re.search(
        r"(?ms)^      - name: Verify evidence manifest before signing\n(.*?)"
        r"^      - name: Create keyless Sigstore provenance attestation",
        job,
    )
    verify_step = verify_step_match.group(1) if verify_step_match else ""
    if not verify_step:
        errors.append("missing pre-signing verification step")
    for required in (
        "EXPECTED_CHECKOUT_SHA: ${{ github.sha }}",
        "EXPECTED_WORKFLOW_SHA: ${{ github.workflow_sha }}",
        "EXPECTED_RUN_ID: ${{ github.run_id }}",
        "EXPECTED_RUN_ATTEMPT: ${{ github.run_attempt }}",
        'manifest.get("status") == "candidate_pass_for_declared_microtest"',
        'manifest.get("claim_ceiling") == "one inherited descriptor and one exec boundary on the named Linux runner; not full descriptor hygiene"',
        'manifest.get("receipt_schema") == "luminous.fd-inheritance-boundary-result.v1"',
        'manifest.get("receipt_status") == "pass"',
        'manifest.get("expected_checkout_sha") == os.environ["EXPECTED_CHECKOUT_SHA"]',
        'manifest.get("workflow_sha") == os.environ["EXPECTED_WORKFLOW_SHA"]',
        'manifest.get("event_name") == "push"',
        'receipt.get("schema") == "luminous.fd-inheritance-boundary-result.v1"',
        'manifest.get("receipt_schema") == receipt.get("schema")',
        'manifest.get("receipt_status") == receipt.get("status")',
        'receipt.get("status") == "pass"',
        'receipt.get("scope") == "exec_boundary_descriptor_hygiene_probe_only"',
        'context["workflow_run_id"] == os.environ["EXPECTED_RUN_ID"]',
    ):
        if required not in verify_step:
            errors.append("pre-signing verifier missing guard: " + required)
    if "assert receipt.get(field) is True" not in verify_step:
        errors.append("pre-signing verifier does not require each receipt control to be true")
    for field in REQUIRED_RECEIPT_FIELDS:
        if field not in verify_step:
            errors.append("pre-signing verifier omits receipt control: " + field)

    if "gh attestation" in workflow:
        errors.append("producer workflow must not invoke an ad-hoc post-signing trust shortcut")
    if "pull_request_target:" in workflow:
        errors.append("privileged workflow must not use pull_request_target")
    if "expected_files = {" not in verify_step:
        errors.append("pre-signing verifier must pin the complete bundle inventory")
    if "entry.is_file() and not entry.is_symlink()" not in verify_step:
        errors.append("pre-signing verifier must reject symlinked bundle entries")
    if "checksum_names == expected_files - {" not in verify_step:
        errors.append("pre-signing verifier must check exact checksum inventory")
    if "sha256sum -c SHA256SUMS" not in verify_step:
        errors.append("pre-signing verifier must cross-check checksums with sha256sum")
    if "retention-days: 14" not in job:
        errors.append("signed evidence artifact retention must remain explicit")
    return errors


class AttestationWorkflowContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW_PATH.read_text(encoding="utf-8")

    def assert_contract_rejects(self, mutated: str, expected_error: str) -> None:
        errors = validate_contract(mutated)
        self.assertIn(expected_error, errors, errors)

    def test_real_workflow_satisfies_contract(self) -> None:
        self.assertEqual(validate_contract(self.workflow), [])

    def test_removing_main_push_guard_is_rejected(self) -> None:
        mutated = self.workflow.replace(
            "if: github.event_name == 'push' && github.ref == 'refs/heads/main'",
            "if: github.event_name == 'pull_request'",
            1,
        )
        self.assert_contract_rejects(mutated, "attestation job is not restricted to a push on main")

    def test_adding_oidc_permission_to_pr_job_is_rejected(self) -> None:
        marker = "permissions:\n  contents: read\n"
        mutated = self.workflow.replace(
            marker,
            "permissions:\n  contents: read\n  id-token: write\n",
            1,
        )
        self.assert_contract_rejects(mutated, "OIDC signing permission leaks outside jobs")

    def test_attestation_permission_leak_is_rejected(self) -> None:
        marker = "permissions:\n  contents: read\n"
        mutated = self.workflow.replace(
            marker,
            "permissions:\n  contents: read\n  attestations: write\n",
            1,
        )
        self.assert_contract_rejects(mutated, "attestation-write permission leaks outside jobs")

    def test_mutable_or_wrong_attest_action_is_rejected(self) -> None:
        mutated = self.workflow.replace("actions/attest@" + ATTEST_SHA, "actions/attest@v4")
        self.assert_contract_rejects(mutated, "missing or unpinned action: actions/attest")

    def test_signer_must_wait_for_runtime_gate(self) -> None:
        mutated = self.workflow.replace(
            "needs: linux-fd-boundary",
            "needs: non_security_job",
            1,
        )
        self.assert_contract_rejects(mutated, "attestation job does not depend on successful runtime-test job")

    def test_signed_subject_must_be_manifest(self) -> None:
        mutated = self.workflow.replace(
            "subject-path: evidence/evidence-manifest.json",
            "subject-path: evidence/run-context.txt",
            1,
        )
        self.assert_contract_rejects(mutated, "attestation subject is not the evidence manifest")

    def test_claim_ceiling_must_be_rechecked_before_signing(self) -> None:
        mutated = self.workflow.replace(
            'manifest.get("claim_ceiling") == "one inherited descriptor and one exec boundary on the named Linux runner; not full descriptor hygiene"',
            'manifest.get("claim_ceiling") == "proves all sandbox escapes impossible"',
            1,
        )
        self.assert_contract_rejects(
            mutated,
            'pre-signing verifier missing guard: manifest.get("claim_ceiling") == "one inherited descriptor and one exec boundary on the named Linux runner; not full descriptor hygiene"',
        )

    def test_missing_receipt_control_is_rejected(self) -> None:
        mutated = self.workflow.replace(
            f'receipt.get("{REQUIRED_RECEIPT_FIELDS[0]}") is True,',
            '"unrelated check": True,',
            1,
        )
        self.assert_contract_rejects(
            mutated,
            "pre-signing verifier omits receipt control: " + REQUIRED_RECEIPT_FIELDS[0],
        )

    def test_wrong_source_binding_is_rejected(self) -> None:
        mutated = self.workflow.replace(
            'manifest.get("expected_checkout_sha") == os.environ["EXPECTED_CHECKOUT_SHA"]',
            'manifest.get("expected_checkout_sha") == "trust-me"',
            1,
        )
        self.assert_contract_rejects(
            mutated,
            'pre-signing verifier missing guard: manifest.get("expected_checkout_sha") == os.environ["EXPECTED_CHECKOUT_SHA"]',
        )

    def test_attestation_job_permission_expansion_is_rejected(self) -> None:
        mutated = self.workflow.replace(
            "      attestations: write\n    steps:",
            "      attestations: write\n      contents: write\n    steps:",
            1,
        )
        self.assert_contract_rejects(
            mutated,
            "attestation job permissions are not the exact least-privilege set",
        )

    def test_attestation_artifact_must_be_separate(self) -> None:
        mutated = self.workflow.replace(
            "name: fd-hygiene-attestation-${{ github.run_id }}-${{ github.run_attempt }}",
            "name: fd-hygiene-evidence-${{ github.run_id }}-${{ github.run_attempt }}",
            1,
        )
        self.assert_contract_rejects(mutated, "attestation artifact is not separate and run-bound")

    def test_manifest_receipt_schema_must_be_exact(self) -> None:
        mutated = self.workflow.replace(
            'receipt.get("schema") == "luminous.fd-inheritance-boundary-result.v1"',
            'receipt.get("schema") == "luminous.other-result.v1"',
            1,
        )
        self.assert_contract_rejects(
            mutated,
            'pre-signing verifier missing guard: receipt.get("schema") == "luminous.fd-inheritance-boundary-result.v1"',
        )

    def test_strict_checksum_inventory_is_required(self) -> None:
        mutated = self.workflow.replace(
            "checksum_names == expected_files - {\"SHA256SUMS\"}",
            "checksum_names == set()",
            1,
        )
        self.assert_contract_rejects(
            mutated,
            "pre-signing verifier must check exact checksum inventory",
        )

    def test_bundle_symlinks_are_rejected(self) -> None:
        mutated = self.workflow.replace(
            "entry.is_file() and not entry.is_symlink()",
            "entry.is_file()",
            1,
        )
        self.assert_contract_rejects(
            mutated,
            "pre-signing verifier must reject symlinked bundle entries",
        )


if __name__ == "__main__":
    unittest.main()
