#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
validator="$script_dir/validate-result.sh"
pass_fixture="$script_dir/fixtures/result-pass-static.example.json"
not_run_fixture="$script_dir/fixtures/result-not-run.example.json"
comparison_lock_fixture="$script_dir/fixtures/comparison-lock-tuf-v1.0.36.example.json"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

bash "$validator" --allow-fixtures "$pass_fixture"
bash "$validator" --allow-fixtures "$not_run_fixture"

# A synthetic PASS fixture must never be accepted as qualification evidence by default.
if bash "$validator" "$pass_fixture" >"$tmpdir/fixture-evidence.out" 2>&1; then
  echo "FAIL: receipt validator accepted fixture=true as qualification evidence" >&2
  cat "$tmpdir/fixture-evidence.out" >&2
  exit 1
fi
echo "PASS: fixture receipt rejected as qualification evidence"

expect_reject() {
  local label="$1"
  local fixture="$2"
  local filter="$3"
  local mutated="$tmpdir/$label.json"
  local output="$tmpdir/$label.out"

  jq "$filter" "$fixture" > "$mutated"
  if bash "$validator" --allow-fixtures "$mutated" >"$output" 2>&1; then
    echo "FAIL: receipt validator accepted negative control '$label'" >&2
    cat "$output" >&2
    exit 1
  fi
  echo "PASS: result receipt negative control rejected ($label)"
}

expect_reject pass-wrong-checkout "$pass_fixture" '.source.actual_checkout_sha = "ffffffffffffffffffffffffffffffffffffffff"'
expect_reject pass-head-match-false "$pass_fixture" '.source.source_matches_expected = false'
expect_reject pass-no-steps "$pass_fixture" '.execution.steps_executed = false'
expect_reject pass-nonzero-exit "$pass_fixture" '.execution.exit_code = 1'
expect_reject pass-no-evidence "$pass_fixture" '.evidence = []'
expect_reject pass-without-evidence-root "$pass_fixture" '.fixture = false'
expect_reject pass-invalid-evidence-digest "$pass_fixture" '.evidence[0].sha256 = "not-a-digest"'
expect_reject unknown-status "$pass_fixture" '.status = "QUEUED"'
expect_reject empty-non-claims "$pass_fixture" '.non_claims = []'
expect_reject pass-elevated-claim-ceiling "$pass_fixture" '.claim_ceiling = "qemu_guest_recovery_only"'
expect_reject pass-unknown-claim-ceiling "$pass_fixture" '.claim_ceiling = "security_qualified"'
expect_reject unknown-scenario "$pass_fixture" '.scenario_id = "ZZZ-999"'
expect_reject not-run-with-steps "$not_run_fixture" '.execution.steps_executed = true'
expect_reject not-run-with-exit-code "$not_run_fixture" '.execution.exit_code = 0'
expect_reject not-run-with-checkout "$not_run_fixture" '.source.actual_checkout_sha = .source.expected_source_sha'

# Exercise byte-level evidence verification against a real temporary artifact.
evidence_root="$tmpdir/evidence-root"
mkdir -p "$evidence_root/fixtures"
printf 'synthetic evidence bytes\\n' > "$evidence_root/fixtures/srq-static-validator.log"
matching_sha="$(sha256sum "$evidence_root/fixtures/srq-static-validator.log" | cut -d ' ' -f 1)"
jq --arg digest "$matching_sha" '.evidence[0].sha256 = $digest' "$pass_fixture" > "$tmpdir/evidence-matching.json"
bash "$validator" --allow-fixtures --evidence-root "$evidence_root" "$tmpdir/evidence-matching.json"
echo "PASS: result receipt verified evidence bytes against the recorded SHA-256"

expect_evidence_reject() {
  local label="$1"
  local filter="$2"
  local mutated="$tmpdir/evidence-$label.json"
  local output="$tmpdir/evidence-$label.out"

  jq "$filter" "$tmpdir/evidence-matching.json" > "$mutated"
  if bash "$validator" --allow-fixtures --evidence-root "$evidence_root" "$mutated" >"$output" 2>&1; then
    echo "FAIL: evidence verifier accepted negative control '$label'" >&2
    cat "$output" >&2
    exit 1
  fi
  echo "PASS: evidence verification negative control rejected ($label)"
}

expect_evidence_reject digest-mismatch '.evidence[0].sha256 = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"'
expect_evidence_reject missing-artifact '.evidence[0].artifact = "fixtures/missing.log"'
expect_evidence_reject unsafe-locator '.evidence[0].artifact = "../outside.log"'


# Comparative receipts must bind the exact lock bytes and source/baseline identity.
lock_sha="$(sha256sum "$comparison_lock_fixture" | cut -d ' ' -f 1)"
jq --arg baseline "tuf-spec-v1.0.36" --arg lock_sha "$lock_sha" \
  '. + {comparison_mode: "matched_baseline_v1", comparison_lock: {baseline_id: $baseline, lock_sha256: $lock_sha}}' \
  "$pass_fixture" > "$tmpdir/comparison-valid.json"
bash "$validator" --allow-fixtures --comparison-lock "$comparison_lock_fixture" "$tmpdir/comparison-valid.json"

expect_comparison_reject() {
  local label="$1"
  local filter="$2"
  local mutated="$tmpdir/comparison-$label.json"
  local output="$tmpdir/comparison-$label.out"
  jq "$filter" "$tmpdir/comparison-valid.json" > "$mutated"
  if bash "$validator" --allow-fixtures --comparison-lock "$comparison_lock_fixture" "$mutated" >"$output" 2>&1; then
    echo "FAIL: comparison receipt accepted negative control '$label'" >&2
    cat "$output" >&2
    exit 1
  fi
  echo "PASS: comparative receipt negative control rejected ($label)"
}
expect_comparison_reject wrong-lock-digest '.comparison_lock.lock_sha256 = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"'
expect_comparison_reject wrong-baseline '.comparison_lock.baseline_id = "systemd-sysupdate-v261.4"'
expect_comparison_reject missing-lock-binding 'del(.comparison_lock)'
expect_comparison_reject missing-comparison-mode 'del(.comparison_mode)'

if bash "$validator" --allow-fixtures "$tmpdir/comparison-valid.json" >"$tmpdir/comparison-no-lock.out" 2>&1; then
  echo "FAIL: comparative receipt accepted without checking its lock file" >&2
  cat "$tmpdir/comparison-no-lock.out" >&2
  exit 1
fi
echo "PASS: comparative receipt rejected when lock file was not provided"

echo "PASS: 18 SRQ result-receipt negative controls plus 5 comparative-lock negative controls were rejected, plus default rejection of fixture evidence."
echo "NON-CLAIM: these examples test receipt-state and artifact-digest invariants only; they do not authenticate signer/device identity or qualify runtime scenarios."
