#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
validator="$script_dir/validate-result.sh"
pass_fixture="$script_dir/fixtures/result-pass-static.example.json"
not_run_fixture="$script_dir/fixtures/result-not-run.example.json"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

bash "$validator" "$pass_fixture"
bash "$validator" "$not_run_fixture"

expect_reject() {
  local label="$1"
  local fixture="$2"
  local filter="$3"
  local mutated="$tmpdir/$label.json"
  local output="$tmpdir/$label.out"

  jq "$filter" "$fixture" > "$mutated"
  if bash "$validator" "$mutated" >"$output" 2>&1; then
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
expect_reject pass-invalid-evidence-digest "$pass_fixture" '.evidence[0].sha256 = "not-a-digest"'
expect_reject unknown-status "$pass_fixture" '.status = "QUEUED"'
expect_reject empty-non-claims "$pass_fixture" '.non_claims = []'
expect_reject not-run-with-steps "$not_run_fixture" '.execution.steps_executed = true'
expect_reject not-run-with-exit-code "$not_run_fixture" '.execution.exit_code = 0'
expect_reject not-run-with-checkout "$not_run_fixture" '.source.actual_checkout_sha = .source.expected_source_sha'

echo "PASS: 11 SRQ result-receipt negative controls were rejected."
echo "NON-CLAIM: these examples test receipt-state invariants only; they do not execute or qualify the referenced scenarios."
