#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
validator="$script_dir/validate.sh"
corpus="$script_dir/scenarios.json"
baselines="$script_dir/industry-baselines.tsv"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

bash "$validator" "$corpus" "$baselines"

expect_reject() {
  local label="$1"
  local filter="$2"
  local mutated="$tmpdir/$label.json"
  local output="$tmpdir/$label.out"

  jq "$filter" "$corpus" > "$mutated"
  if bash "$validator" "$mutated" "$baselines" >"$output" 2>&1; then
    echo "FAIL: validator accepted negative control '$label'" >&2
    cat "$output" >&2
    exit 1
  fi
  echo "PASS: negative control rejected ($label)"
}

expect_reject duplicate-id '.scenarios[0].id = .scenarios[1].id'
expect_reject empty-required-evidence '.scenarios[0].required_evidence = []'
expect_reject empty-forbidden-outcomes '.scenarios[0].forbidden_outcomes = []'
expect_reject unknown-surface '.scenarios[0].surface = "untrusted-repo"'
expect_reject unknown-execution-tier '.scenarios[0].execution_tiers = ["mystery"]'
expect_reject missing-release-gate '.scenarios[0].release_gate = false'
expect_reject executed-status-claim '.scenarios[0].status = "pass"'
expect_reject unauthorized-partial-status '.scenarios[0].status = "partial_model_pass_not_qualified"'
expect_reject partial-without-observed-tier '.scenarios[9].observed_tiers = []'
expect_reject unknown-observed-tier '.scenarios[9].observed_tiers = ["mystery"]'
expect_reject changed-schema-version '.schema_version = 2'
expect_reject false-corpus-status '.qualification_status = "passed"'
expect_reject truncated-corpus '.scenarios = []'

expect_baseline_reject() {
  local label="$1"
  local awk_program="$2"
  local mutated="$tmpdir/baseline-$label.tsv"
  local output="$tmpdir/baseline-$label.out"

  awk -F '\t' -v OFS='\t' "$awk_program" "$baselines" > "$mutated"
  if bash "$validator" "$corpus" "$mutated" >"$output" 2>&1; then
    echo "FAIL: validator accepted baseline negative control '$label'" >&2
    cat "$output" >&2
    exit 1
  fi
  echo "PASS: baseline negative control rejected ($label)"
}

expect_baseline_reject missing-column 'NR == 3 { $5 = "" } 1'
expect_baseline_reject wrong-column-count 'NR == 3 { $5 = ""; NF = 4 } 1'
expect_baseline_reject duplicate-name 'NR == 3 { $1 = "NixOS generations" } 1'
expect_baseline_reject invalid-reference-url 'NR == 3 { $6 = "not-a-url" } 1'
expect_baseline_reject truncated-matrix 'NR <= 6 { print }'

echo "PASS: 13 corpus and 5 baseline negative controls were rejected."
echo "NON-CLAIM: these tests validate specification safeguards, not the runtime scenarios."
