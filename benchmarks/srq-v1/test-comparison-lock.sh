#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
validator="$script_dir/validate-comparison-lock.sh"
fixture="$script_dir/fixtures/comparison-lock-tuf-v1.0.36.example.json"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
bash "$validator" --allow-fixtures "$fixture"
if bash "$validator" "$fixture" >"$tmpdir/fixture-default.out" 2>&1; then
  echo "FAIL: fixture comparison lock accepted without --allow-fixtures" >&2
  cat "$tmpdir/fixture-default.out" >&2
  exit 1
fi
echo "PASS: fixture comparison lock rejected by default"
expect_reject() {
  local label="$1"
  local filter="$2"
  local mutated="$tmpdir/$label.json"
  local output="$tmpdir/$label.out"
  jq "$filter" "$fixture" > "$mutated"
  if bash "$validator" --allow-fixtures "$mutated" >"$output" 2>&1; then
    echo "FAIL: comparison lock accepted negative control '$label'" >&2
    cat "$output" >&2
    exit 1
  fi
  echo "PASS: comparison lock negative control rejected ($label)"
}
expect_reject unknown-baseline '.baseline_id = "unknown-baseline"'
expect_reject latest-version '.baseline_version = "latest"'
expect_reject wrong-tuf-commit '.baseline_source.resolved_commit_sha = "ffffffffffffffffffffffffffffffffffffffff"'
expect_reject missing-input-digest '.fault_schedule_sha256 = null'
expect_reject empty-excluded-scope '.scope.excluded = []'
expect_reject blocked-ostree-baseline '.baseline_id = "ostree-v2026.4" | .baseline_version = "v2026.4" | .baseline_source.resolved_commit_sha = "4f4445a8912f70c604fa84a057a8f4e904e35a24"'
expect_reject broken-artifact-digest '.benchmark_artifact_sha256 = "not-a-digest"'
jq '.fixture = false' "$fixture" > "$tmpdir/nonfixture.json"
if SRQ_EXPECTED_SOURCE_SHA="2222222222222222222222222222222222222222" bash "$validator" "$tmpdir/nonfixture.json" >"$tmpdir/wrong-source.out" 2>&1; then
  echo "FAIL: non-fixture lock accepted wrong SRQ source SHA" >&2
  cat "$tmpdir/wrong-source.out" >&2
  exit 1
fi
jq '.fixture = false | .srq_source_sha = "2222222222222222222222222222222222222222"' "$fixture" > "$tmpdir/matching-source.json"
SRQ_EXPECTED_SOURCE_SHA="2222222222222222222222222222222222222222" bash "$validator" "$tmpdir/matching-source.json"
echo "PASS: 7 comparison-lock negative controls were rejected."
echo "NON-CLAIM: these tests validate lock structure and registry correspondence only, not an executed baseline comparison."
