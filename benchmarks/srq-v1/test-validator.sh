#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
validator="$script_dir/validate.sh"
corpus="$script_dir/scenarios.json"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

"$validator" "$corpus"

expect_reject() {
  local label="$1"
  local filter="$2"
  local mutated="$tmpdir/$label.json"
  local output="$tmpdir/$label.out"

  jq "$filter" "$corpus" > "$mutated"
  if "$validator" "$mutated" >"$output" 2>&1; then
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
expect_reject changed-schema-version '.schema_version = 2'
expect_reject false-corpus-status '.qualification_status = "passed"'
expect_reject truncated-corpus '.scenarios = []'

echo "PASS: 10 SRQ-1 negative controls were rejected."
echo "NON-CLAIM: these tests validate corpus safeguards, not the runtime scenarios."
