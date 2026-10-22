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
# Verify a non-fixture lock against actual input bytes under the evidence root.
evidence_root="$tmpdir/evidence"
mkdir -p "$evidence_root/fixtures/comparison-lock-artifacts"
printf 'benchmark fixture bytes\n' > "$evidence_root/fixtures/comparison-lock-artifacts/benchmark.bin"
printf '{"fixture":"configuration"}\n' > "$evidence_root/fixtures/comparison-lock-artifacts/config.json"
printf '{"fixture":"corpus"}\n' > "$evidence_root/fixtures/comparison-lock-artifacts/corpus.json"
printf '{"fixture":"fault-schedule"}\n' > "$evidence_root/fixtures/comparison-lock-artifacts/fault-schedule.json"
printf 'oracle fixture bytes\n' > "$evidence_root/fixtures/comparison-lock-artifacts/oracle.bin"
benchmark_sha="$(sha256sum "$evidence_root/fixtures/comparison-lock-artifacts/benchmark.bin" | cut -d ' ' -f 1)"
config_sha="$(sha256sum "$evidence_root/fixtures/comparison-lock-artifacts/config.json" | cut -d ' ' -f 1)"
corpus_sha="$(sha256sum "$evidence_root/fixtures/comparison-lock-artifacts/corpus.json" | cut -d ' ' -f 1)"
schedule_sha="$(sha256sum "$evidence_root/fixtures/comparison-lock-artifacts/fault-schedule.json" | cut -d ' ' -f 1)"
oracle_sha="$(sha256sum "$evidence_root/fixtures/comparison-lock-artifacts/oracle.bin" | cut -d ' ' -f 1)"
source_sha="2222222222222222222222222222222222222222"
jq --arg source "$source_sha" --arg benchmark "$benchmark_sha" --arg config "$config_sha" \
   --arg corpus "$corpus_sha" --arg schedule "$schedule_sha" --arg oracle "$oracle_sha" \
   '.fixture = false | .srq_source_sha = $source |
    .benchmark_artifact_sha256 = $benchmark | .configuration_sha256 = $config |
    .corpus_sha256 = $corpus | .fault_schedule_sha256 = $schedule | .outcome_oracle_sha256 = $oracle' \
   "$fixture" > "$tmpdir/matching-source.json"
SRQ_EXPECTED_SOURCE_SHA="$source_sha" bash "$validator" --evidence-root "$evidence_root" "$tmpdir/matching-source.json"
printf 'tampered benchmark bytes\n' > "$evidence_root/fixtures/comparison-lock-artifacts/benchmark.bin"
if SRQ_EXPECTED_SOURCE_SHA="$source_sha" bash "$validator" --evidence-root "$evidence_root" "$tmpdir/matching-source.json" >"$tmpdir/tampered-material.out" 2>&1; then
  echo "FAIL: non-fixture comparison lock accepted mutated input bytes" >&2
  cat "$tmpdir/tampered-material.out" >&2
  exit 1
fi
echo "PASS: non-fixture comparison lock rejected mutated material bytes"
echo "PASS: 7 comparison-lock negative controls were rejected, and locked material bytes were verified."
echo "NON-CLAIM: these tests validate lock structure and registry correspondence only, not an executed baseline comparison."
