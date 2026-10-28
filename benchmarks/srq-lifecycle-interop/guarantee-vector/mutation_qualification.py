#!/usr/bin/env python3
"""Mutation-adequacy benchmark for SRQ lifecycle assurance invariants.

This is a test/development harness, not product code. It copies the Rust crate to
an isolated temporary directory, applies one deliberately unsafe source mutation
at a time, and requires the named regression test to fail. A compile failure
does not count as a killed mutant.
"""
from __future__ import annotations

import argparse
from dataclasses import asdict, dataclass
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time


@dataclass(frozen=True)
class Mutation:
    mutation_id: str
    description: str
    before: str
    after: str
    target_test: str

    @property
    def test_function(self) -> str:
        return self.target_test.removeprefix("tests::")


MUTATIONS = (
    Mutation(
        "expired_authorization",
        "Remove the exclusive expiry check from authorization freshness.",
        """if proof.not_before_epoch < proof.expires_at_epoch
                && proof.not_before_epoch <= vector.trusted_epoch.value()
                && vector.trusted_epoch.value() < proof.expires_at_epoch""",
        """if proof.not_before_epoch < proof.expires_at_epoch
                && proof.not_before_epoch <= vector.trusted_epoch.value()
                && true""",
        "tests::authorization_window_is_inclusive_at_start_and_exclusive_at_expiry",
    ),
    Mutation(
        "stale_anchor_epoch",
        "Accept an anchor checkpoint observed at a different trusted epoch.",
        "if proof.observed_at_epoch == vector.trusted_epoch.value()",
        "if true",
        "tests::stale_anchor_checkpoint_cannot_be_replayed_at_a_newer_epoch",
    ),
    Mutation(
        "wrong_sink_capability",
        "Ignore sink identity when validating an atomic idempotency capability.",
        """&& !proof.idempotency_key.is_empty()
                && is_lower_hex_digest(&proof.sink_identity_digest)
                && proof.sink_identity_digest == vector.sink_identity_digest =>""",
        """&& !proof.idempotency_key.is_empty()
                && is_lower_hex_digest(&proof.sink_identity_digest)
                && true =>""",
        "tests::frozen_assurance_vector_matrix_matches_all_expected_decisions",
    ),
    Mutation(
        "receipt_effect_binding",
        "Ignore the receipt-to-effect digest binding in the no-redispatch path.",
        "|| proof.effect_digest != vector.effect_digest",
        "|| false",
        "tests::receipt_must_bind_request_effect_and_policy",
    ),
    Mutation(
        "retry_policy",
        "Dispatch again instead of reusing the sink's same-key retry contract.",
        """DeliveryContract::AtomicIdempotency(_) => {
                    Decision::RetrySameIdempotencyKey
                }""",
        """DeliveryContract::AtomicIdempotency(_) => {
                    Decision::DispatchOnce
                }""",
        "tests::frozen_assurance_vector_matrix_matches_all_expected_decisions",
    ),
)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def apply_exactly_once(source: str, mutation: Mutation) -> str:
    count = source.count(mutation.before)
    if count != 1:
        raise ValueError(
            f"{mutation.mutation_id}: expected exactly one mutation target; found {count}"
        )
    mutated = source.replace(mutation.before, mutation.after, 1)
    if mutated == source:
        raise ValueError(f"{mutation.mutation_id}: mutation produced no source change")
    return mutated


def load_manifest(crate_root: Path) -> dict:
    manifest_path = crate_root / "mutation-protocol-v1.json"
    return json.loads(manifest_path.read_text(encoding="utf-8"))


def preflight(crate_root: Path, source: str, manifest: dict) -> dict:
    failures: list[str] = []
    known_ids = {item["mutation_id"] for item in manifest["mutations"]}
    script_ids = {item.mutation_id for item in MUTATIONS}
    if known_ids != script_ids:
        failures.append("protocol mutation IDs do not exactly match the executable harness")
    for mutation in MUTATIONS:
        try:
            apply_exactly_once(source, mutation)
        except ValueError as exc:
            failures.append(str(exc))
        if not re.search(
            rf"(?m)^\s*fn\s+{re.escape(mutation.test_function)}\s*\(",
            source,
        ):
            failures.append(
                f"{mutation.mutation_id}: target test function not present: {mutation.target_test}"
            )
    if "qualification-fixtures = []" not in (crate_root / "Cargo.toml").read_text(encoding="utf-8"):
        failures.append("Cargo.toml does not contain the opt-in qualification-fixtures feature")
    return {
        "status": "STATIC_PREFLIGHT_PASS" if not failures else "STATIC_PREFLIGHT_FAIL",
        "mutation_count": len(MUTATIONS),
        "source_sha256": sha256_bytes(source.encode("utf-8")),
        "protocol_sha256": sha256_bytes(
            (crate_root / "mutation-protocol-v1.json").read_bytes()
        ),
        "mutation_harness_sha256": sha256_bytes(Path(__file__).read_bytes()),
        "failures": failures,
        "note": "Static target matching only; this is not executed mutation evidence.",
    }


def run_mutation(
    crate_root: Path,
    source: str,
    mutation: Mutation,
    temp_root: Path,
    cargo: str,
    timeout_seconds: int,
) -> dict:
    mutated_root = temp_root / mutation.mutation_id / "crate"
    mutated_root.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(
        crate_root,
        mutated_root,
        ignore=shutil.ignore_patterns("target", ".git", "__pycache__"),
    )
    lib_path = mutated_root / "src" / "lib.rs"
    mutated_source = apply_exactly_once(source, mutation)
    lib_path.write_text(mutated_source, encoding="utf-8")

    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(temp_root / "cargo-target")
    command = [
        cargo,
        "test",
        "--offline",
        "--manifest-path",
        str(mutated_root / "Cargo.toml"),
        "--features",
        "qualification-fixtures",
        mutation.target_test,
        "--",
        "--exact",
        "--nocapture",
    ]
    started = time.monotonic()
    try:
        completed = subprocess.run(
            command,
            cwd=mutated_root,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=timeout_seconds,
            check=False,
        )
    except subprocess.TimeoutExpired as exc:
        output = exc.stdout or ""
        if isinstance(output, bytes):
            output = output.decode("utf-8", errors="replace")
        return {
            "mutation_id": mutation.mutation_id,
            "target_test": mutation.target_test,
            "status": "INVALID_TIMEOUT",
            "returncode": None,
            "elapsed_seconds": round(time.monotonic() - started, 3),
            "output_tail": output[-4000:],
        }

    output = completed.stdout
    target_failed = re.search(
        rf"(?m)^test\s+{re.escape(mutation.target_test)}\s+\.\.\.\s+FAILED\s*$",
        output,
    ) is not None
    compile_failed = (
        "could not compile" in output.lower()
        or re.search(r"(?m)^error\[E\d+\]", output) is not None
    )
    killed = completed.returncode != 0 and target_failed and not compile_failed
    if killed:
        status = "KILLED"
    elif completed.returncode == 0:
        status = "SURVIVED"
    else:
        status = "INVALID_MUTANT_OR_UNRELATED_FAILURE"
    return {
        "mutation_id": mutation.mutation_id,
        "target_test": mutation.target_test,
        "status": status,
        "returncode": completed.returncode,
        "elapsed_seconds": round(time.monotonic() - started, 3),
        "mutated_source_sha256": sha256_bytes(mutated_source.encode("utf-8")),
        "output_tail": output[-4000:],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check-only",
        action="store_true",
        help="verify exact mutation anchors and test names without compiling Rust",
    )
    parser.add_argument("--output", type=Path, help="write machine-readable JSON report")
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--timeout-seconds", type=int, default=180)
    args = parser.parse_args()

    crate_root = Path(__file__).resolve().parent
    source_path = crate_root / "src" / "lib.rs"
    source = source_path.read_text(encoding="utf-8")
    manifest = load_manifest(crate_root)
    static_report = preflight(crate_root, source, manifest)
    if static_report["status"] != "STATIC_PREFLIGHT_PASS":
        report = static_report
        exit_code = 1
    elif args.check_only:
        report = static_report
        exit_code = 0
    else:
        results = []
        with tempfile.TemporaryDirectory(prefix="srq-mutation-") as temp:
            temp_root = Path(temp)
            for mutation in MUTATIONS:
                results.append(
                    run_mutation(
                        crate_root,
                        source,
                        mutation,
                        temp_root,
                        args.cargo,
                        args.timeout_seconds,
                    )
                )
        killed = sum(item["status"] == "KILLED" for item in results)
        report = {
            "schema": "srq-mutation-qualification-report-v1",
            "status": "PASS" if killed == len(MUTATIONS) else "FAIL",
            "claim_ceiling": "none",
            "qualification_status": "BLOCKED",
            "scope": "test-suite-mutation-adequacy-not-production-security",
            "harness_commit_sha": os.environ.get("GITHUB_SHA"),
            "source_head_sha": os.environ.get("SRQ_MUTATION_PR_HEAD_SHA") or os.environ.get("GITHUB_SHA"),
            **{
                key: value
                for key, value in static_report.items()
                if key not in {"status", "note", "failures"}
            },
            "mutation_count": len(MUTATIONS),
            "killed_mutant_count": killed,
            "survived_mutant_count": sum(item["status"] == "SURVIVED" for item in results),
            "invalid_mutant_count": sum(
                item["status"] not in {"KILLED", "SURVIVED"} for item in results
            ),
            "mutations": results,
            "note": (
                "A mutation counts as killed only when its named regression test fails. "
                "Compile errors and unrelated failures do not count. Passing this finite set "
                "does not prove the absence of other defects."
            ),
        }
        exit_code = 0 if report["status"] == "PASS" else 1

    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
