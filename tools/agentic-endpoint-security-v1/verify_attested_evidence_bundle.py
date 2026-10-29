#!/usr/bin/env python3
"""Verify a signed Luminous evidence manifest and its portable evidence bundle.

The command delegates Sigstore cryptographic verification to GitHub CLI, checks the
certificate's source repository ID and attested manifest digest, then runs the
existing offline receipt/manifest consistency verifier. It does not prove that a
runner kernel or the underlying observation source was uncompromised.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

from verify_evidence_bundle import verify_bundle

REPO_NAME = "Luminous-Dynamics/luminous-platform"
MAIN_REF = "refs/heads/main"
SHA40 = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
CHECKSUM_LINE = re.compile(r"^([0-9a-f]{64})  ([A-Za-z0-9._-]+)$")
ATTESTATION_FILENAME = "evidence-manifest.sigstore.json"
PROBES: dict[str, dict[str, str]] = {
    "landlock": {
        "workflow": "Landlock pre-opened descriptor authority probe",
        "signer_workflow": (
            "github.com/Luminous-Dynamics/luminous-platform/"
            ".github/workflows/landlock-preopened-fd-probe.yml"
        ),
        "receipt": "landlock-preopened-fd-result.jsonl",
        "claim_ceiling": (
            "one file-descriptor ordering and exec boundary on the named Linux runner; "
            "not full descriptor hygiene"
        ),
    },
    "fd": {
        "workflow": "fd inheritance boundary probe",
        "signer_workflow": (
            "github.com/Luminous-Dynamics/luminous-platform/"
            ".github/workflows/fd-hygiene-probe.yml"
        ),
        "receipt": "fd-hygiene-result.jsonl",
        "claim_ceiling": (
            "one inherited descriptor and one exec boundary on the named Linux runner; "
            "not full descriptor hygiene"
        ),
    },
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def verify_attestation_bundle_file(attestation_dir: Path) -> Path:
    root = attestation_dir.resolve()
    require(root.is_dir(), "attestation directory does not exist")
    expected = {ATTESTATION_FILENAME, "SHA256SUMS"}
    entries = list(root.iterdir())
    require({entry.name for entry in entries} == expected,
            "attestation directory must contain exactly the bundle and SHA256SUMS")
    require(all(entry.is_file() and not entry.is_symlink() for entry in entries),
            "attestation directory must not contain directories or symbolic links")

    lines = (root / "SHA256SUMS").read_text(encoding="utf-8").splitlines()
    require(len(lines) == 1, "attestation SHA256SUMS must contain exactly one entry")
    match = CHECKSUM_LINE.fullmatch(lines[0])
    require(match is not None, "attestation SHA256SUMS entry is malformed")
    digest, filename = match.groups()
    require(filename == ATTESTATION_FILENAME,
            "attestation SHA256SUMS points to an unexpected filename")
    bundle = root / ATTESTATION_FILENAME
    require(hashlib.sha256(bundle.read_bytes()).hexdigest() == digest,
            "attestation bundle checksum mismatch")
    return bundle


def validate_verified_output(
    raw_output: str,
    manifest_path: Path,
    expected_repo_id: str,
) -> dict[str, Any]:
    try:
        decoded = json.loads(raw_output)
    except json.JSONDecodeError as exc:
        raise ValueError(f"GitHub CLI verification did not return valid JSON: {exc}") from exc
    require(isinstance(decoded, list) and bool(decoded),
            "GitHub CLI returned no verified attestations")

    expected_digest = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
    matching: list[dict[str, Any]] = []
    for item in decoded:
        if not isinstance(item, dict):
            continue
        result = item.get("verificationResult")
        if not isinstance(result, dict):
            continue
        signature = result.get("signature")
        certificate = signature.get("certificate") if isinstance(signature, dict) else None
        statement = result.get("statement")
        if not isinstance(certificate, dict) or not isinstance(statement, dict):
            continue

        source_repository_id = certificate.get("sourceRepositoryIdentifier")
        if str(source_repository_id) != expected_repo_id:
            continue
        if statement.get("predicateType") != "https://slsa.dev/provenance/v1":
            continue
        subjects = statement.get("subject")
        if not isinstance(subjects, list):
            continue
        subject_digest_matches = any(
            isinstance(subject, dict)
            and isinstance(subject.get("digest"), dict)
            and subject["digest"].get("sha256") == expected_digest
            for subject in subjects
        )
        if subject_digest_matches:
            matching.append({
                "repository_id": str(source_repository_id),
                "predicate_type": statement.get("predicateType"),
                "subject_sha256": expected_digest,
            })

    require(bool(matching),
            "no verified attestation binds the expected repository ID and manifest digest")
    return matching[0]


def build_gh_command(
    *,
    gh_executable: str,
    manifest_path: Path,
    attestation_bundle: Path,
    trusted_root: Path,
    expected_checkout_sha: str,
    probe: str,
) -> list[str]:
    contract = PROBES[probe]
    return [
        gh_executable,
        "attestation",
        "verify",
        str(manifest_path),
        "--repo",
        REPO_NAME,
        "--bundle",
        str(attestation_bundle),
        "--custom-trusted-root",
        str(trusted_root),
        "--signer-workflow",
        contract["signer_workflow"],
        "--source-ref",
        MAIN_REF,
        "--source-digest",
        expected_checkout_sha,
        "--predicate-type",
        "https://slsa.dev/provenance/v1",
        "--deny-self-hosted-runners",
        "--format",
        "json",
    ]


def verify_attested_bundle(
    *,
    evidence_dir: Path,
    attestation_dir: Path,
    trusted_root: Path,
    expected_checkout_sha: str,
    expected_run_id: str,
    expected_repo_id: str,
    probe: str,
    gh_executable: str = "gh",
) -> dict[str, Any]:
    require(probe in PROBES, "probe must be landlock or fd")
    require(SHA40.fullmatch(expected_checkout_sha) is not None,
            "expected checkout SHA must be 40 lowercase hexadecimal characters")
    require(expected_run_id.isdigit() and int(expected_run_id) > 0,
            "expected run ID must be a positive integer")
    require(expected_repo_id.isdigit() and int(expected_repo_id) > 0,
            "expected repository ID must be a positive integer")
    require(trusted_root.is_file() and not trusted_root.is_symlink(),
            "trusted root must be an existing regular non-symlink file")

    evidence_root = evidence_dir.resolve()
    manifest_path = evidence_root / "evidence-manifest.json"
    require(manifest_path.is_file() and not manifest_path.is_symlink(),
            "evidence-manifest.json is missing or is a symbolic link")

    # This first gate validates all bytes and derives qualification from the receipt,
    # run context and manifest. Do not ask the signature to bless a nonqualified bundle.
    consistency = verify_bundle(evidence_root, expected_checkout_sha, expected_run_id)
    require(consistency.get("status") == "candidate_pass_for_declared_microtest",
            "evidence bundle is internally consistent but not qualified for the declared micro-test")

    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    contract = PROBES[probe]
    require(manifest.get("workflow") == contract["workflow"],
            "manifest workflow does not match the selected probe")
    require(manifest.get("claim_ceiling") == contract["claim_ceiling"],
            "manifest claim ceiling does not match the selected probe")
    require(manifest.get("receipt_path") == contract["receipt"],
            "manifest receipt path does not match the selected probe")
    require(manifest.get("event_name") == "push",
            "only main-push evidence may be accepted as attested evidence")

    attestation_bundle = verify_attestation_bundle_file(attestation_dir)
    command = build_gh_command(
        gh_executable=gh_executable,
        manifest_path=manifest_path,
        attestation_bundle=attestation_bundle,
        trusted_root=trusted_root,
        expected_checkout_sha=expected_checkout_sha,
        probe=probe,
    )
    result = subprocess.run(command, check=False, capture_output=True, text=True, timeout=60)
    if result.returncode != 0:
        stderr = result.stderr.strip()[-4000:]
        raise ValueError(
            "GitHub CLI rejected the attestation bundle"
            + (f": {stderr}" if stderr else "")
        )

    signature_evidence = validate_verified_output(
        result.stdout,
        manifest_path,
        expected_repo_id,
    )
    return {
        "schema": "luminous.attested-evidence-verification.v1",
        "status": "verified_candidate_pass_for_declared_microtest",
        "probe": probe,
        "workflow": contract["workflow"],
        "expected_checkout_sha": expected_checkout_sha,
        "workflow_run_id": expected_run_id,
        "repository": REPO_NAME,
        "repository_id": expected_repo_id,
        "signer_workflow": contract["signer_workflow"],
        "source_ref": MAIN_REF,
        "manifest_sha256": signature_evidence["subject_sha256"],
        "receipt_sha256": manifest["receipt_sha256"],
        "claim_ceiling": contract["claim_ceiling"],
        "qualification_boundary": (
            "signature authenticates provenance under the supplied trust root; "
            "it does not prove the host kernel or observation source uncompromised"
        ),
        "consistency_verification": consistency,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-dir", type=Path, required=True)
    parser.add_argument("--attestation-dir", type=Path, required=True)
    parser.add_argument("--trusted-root", type=Path, required=True)
    parser.add_argument("--expected-checkout-sha", required=True)
    parser.add_argument("--expected-run-id", required=True)
    parser.add_argument("--expected-repo-id", required=True)
    parser.add_argument("--probe", choices=sorted(PROBES), required=True)
    parser.add_argument("--gh-executable", default="gh")
    args = parser.parse_args()

    try:
        report = verify_attested_bundle(
            evidence_dir=args.evidence_dir,
            attestation_dir=args.attestation_dir,
            trusted_root=args.trusted_root,
            expected_checkout_sha=args.expected_checkout_sha,
            expected_run_id=args.expected_run_id,
            expected_repo_id=args.expected_repo_id,
            probe=args.probe,
            gh_executable=args.gh_executable,
        )
    except (OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired) as exc:
        print(f"FAIL signed evidence verification: {exc}", file=sys.stderr)
        return 2
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
