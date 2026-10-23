#!/usr/bin/env bash
set -euo pipefail

readonly script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
readonly validator="$script_dir/validate.sh"
nixward_root="${1:-}" mycelix_root="${2:-}" sovereign_boot_root="${3:-}" boot_observation="${4:-}" output_dir="${5:-}"
if [[ -z "$nixward_root" || -z "$mycelix_root" || -z "$sovereign_boot_root" || -z "$boot_observation" || -z "$output_dir" ]]; then
  echo "Usage: $0 NIXWARD_ROOT MYCELIX_ROOT SOVEREIGN_BOOT_ROOT BOOT_OBSERVATION_JSON OUTPUT_DIR" >&2
  exit 2
fi

readonly receipt="$mycelix_root/research/evidence-plane/examples/synthetic-planned-receipt-v0.json"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cases=0

expect_reject() {
  local name="$1" filter="$2" candidate="$tmp/$1.json"
  jq "$filter" "$receipt" > "$candidate"
  if bash "$validator" \
    --nixward-root "$nixward_root" \
    --mycelix-root "$mycelix_root" \
    --sovereign-boot-root "$sovereign_boot_root" \
    --boot-observation "$boot_observation" \
    --evidence-plane-receipt "$candidate" \
    --output /dev/null >"$tmp/$name.log" 2>&1; then
    echo "FAIL: adversarial case was accepted: $name" >&2
    cat "$tmp/$name.log" >&2
    exit 1
  fi
  cases=$((cases + 1))
  echo "PASS: rejected inconsistent evidence mutation: $name"
}

# Every candidate is a disposable copy. The four immutable upstream pins remain untouched.
expect_reject "s0-domain-validated-status" '.run_status = "domain_validated"'
expect_reject "s0-independently-checked-status" '.run_status = "independently_checked"'
expect_reject "planned-gate-inflation" '.gates.source_integrity = "pass"'
expect_reject "planned-subject-commit-injection" '.subject.commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"'
expect_reject "planned-artifact-hash-injection" '.subject.artifact_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"'
expect_reject "planned-result-count-injection" '.benchmark.results_summary.observed_result_count = 1'
expect_reject "s3-without-independent-verification" '.evidence_tier = "S3" | .claim_ceiling = "independently_reproduced"'
expect_reject "s2-source-reproducible-without-source" '.evidence_tier = "S2" | .claim_ceiling = "source_reproducible"'
expect_reject "unknown-tier" '.evidence_tier = "S5"'

printf 'ADVERSARIAL_CORPUS_PASS: %s inconsistent-evidence mutations rejected.\n' "$cases"
