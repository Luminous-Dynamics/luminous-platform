#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
validator="$script_dir/validate-result.sh"
pass_fixture="$script_dir/fixtures/result-pass-static.example.json"
not_run_fixture="$script_dir/fixtures/result-not-run.example.json"
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
expect_reject pass-invalid-evidence-digest "$pass_fixture" '.evidence[0].sha256 = "not-a-digest"'
expect_reject unknown-status "$pass_fixture" '.status = "QUEUED"'
expect_reject empty-non-claims "$pass_fixture" '.non_claims = []'
expect_reject unknown-scenario "$pass_fixture" '.scenario_id = "ZZZ-999"'
expect_reject not-run-with-steps "$not_run_fixture" '.execution.steps_executed = true'
expect_reject not-run-with-exit-code "$not_run_fixture" '.execution.exit_code = 0'
expect_reject not-run-with-checkout "$not_run_fixture" '.source.actual_checkout_sha = .source.expected_source_sha'

echo "PASS: 12 SRQ result-receipt negative controls were rejected, plus default rejection of fixture evidence."
echo "NON-CLAIM: these examples test receipt-state invariants only; they do not execute or qualify the referenced scenarios."
