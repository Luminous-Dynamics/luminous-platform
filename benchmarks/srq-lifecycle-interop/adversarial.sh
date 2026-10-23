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
readonly repository_root="$(git -C "$script_dir/../.." rev-parse --show-toplevel)"
readonly harness_commit="$(git -C "$repository_root" rev-parse HEAD)"
mkdir -p "$output_dir"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
report="$output_dir/srq-lifecycle-adversarial-assessment.json"
cases=0

jq -cn \
  --arg commit "$harness_commit" \
  --arg script_sha256 "$(sha256sum "$script_dir/adversarial.sh" | cut -d ' ' -f 1)" \
  --arg script_blob "$(git -C "$repository_root" hash-object -- benchmarks/srq-lifecycle-interop/adversarial.sh)" \
  --arg validator_sha256 "$(sha256sum "$validator" | cut -d ' ' -f 1)" \
  --arg validator_blob "$(git -C "$repository_root" hash-object -- benchmarks/srq-lifecycle-interop/validate.sh)" \
  --arg receipt_sha256 "$(sha256sum "$receipt" | cut -d ' ' -f 1)" \
  '{
    schema:"srq-lifecycle-adversarial-assessment-v1",
    status:"RUNNING",
    release_status:"BLOCKED",
    harness:{repository:"Luminous-Dynamics/luminous-platform",commit_sha:$commit,adversarial_script_git_blob_sha1:$script_blob,adversarial_script_sha256:$script_sha256,validator_git_blob_sha1:$validator_blob,validator_sha256:$validator_sha256},
    fixture:{repository:"Luminous-Dynamics/mycelix",commit_sha:"b02cefab8474262aef23b61a4ac3811fe92e29bf",git_blob_sha1:"d1d3516e2778e0da89013348b16ed73af6e5e8a8",raw_sha256:$receipt_sha256},
    expected_case_count:9,
    rejected_case_count:0,
    all_expected_mutations_rejected:false,
    cases:[]
  }' > "$report"

expect_reject() {
  local name="$1" filter="$2" expected_reason="$3" candidate="$tmp/$1.json"
  local status=0 reason="" candidate_sha256=""
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
  else
    status=$?
  fi

  reason="$(grep -F "CONTRACT VIOLATION:" "$tmp/$name.log" | tail -n 1 || true)"
  if [[ -z "$reason" || "$reason" != *"$expected_reason"* ]]; then
    echo "FAIL: $name failed for an unexpected reason (exit $status)." >&2
    cat "$tmp/$name.log" >&2
    exit 1
  fi
  candidate_sha256="$(sha256sum "$candidate" | cut -d ' ' -f 1)"
  jq --arg id "$name" --arg filter "$filter" --arg reason "$reason" \
    --arg candidate_sha256 "$candidate_sha256" --argjson exit_status "$status" \
    '.cases += [{
      id:$id,
      mutation_filter:$filter,
      expected_outcome:"reject",
      observed_outcome:"REJECTED_AS_EXPECTED",
      validator_exit_status:$exit_status,
      rejection_reason:$reason,
      candidate_raw_sha256:$candidate_sha256
    }]' "$report" > "$tmp/report.json"
  mv "$tmp/report.json" "$report"
  cases=$((cases + 1))
  echo "PASS: rejected inconsistent evidence mutation: $name"
}

# Each case must fail on a contract invariant, not on a missing tool or bad source pin.
# Candidates are disposable copies; the pinned upstream receipt is never edited.
expect_reject "s0-domain-validated-status" '.run_status = "domain_validated"' "cross-field semantics"
expect_reject "s0-independently-checked-status" '.run_status = "independently_checked"' "cross-field semantics"
expect_reject "planned-gate-inflation" '.gates.source_integrity = "pass"' "cross-field semantics"
expect_reject "planned-subject-commit-injection" '.subject.commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"' "cross-field semantics"
expect_reject "planned-artifact-hash-injection" '.subject.artifact_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"' "cross-field semantics"
expect_reject "planned-result-count-injection" '.benchmark.results_summary.observed_result_count = 1' "declared evidence tier / claim ceiling"
expect_reject "s3-without-independent-verification" '.evidence_tier = "S3" | .claim_ceiling = "independently_reproduced"' "cross-field semantics"
expect_reject "s2-source-reproducible-without-source" '.evidence_tier = "S2" | .claim_ceiling = "source_reproducible"' "cross-field semantics"
expect_reject "unknown-tier" '.evidence_tier = "S5"' "declared evidence tier / claim ceiling"

jq --argjson count "$cases" '
  .status = "PASS"
  | .rejected_case_count = ([.cases[] | select(.observed_outcome == "REJECTED_AS_EXPECTED")] | length)
  | .all_expected_mutations_rejected = (.cases | length == $count and all(.[]; .observed_outcome == "REJECTED_AS_EXPECTED"))
' "$report" > "$tmp/final-report.json"
mv "$tmp/final-report.json" "$report"
[[ "$cases" -eq 9 ]] || { echo "FAIL: expected exactly 9 adversarial cases, found $cases" >&2; exit 1; }
jq -e '.schema == "srq-lifecycle-adversarial-assessment-v1" and .status == "PASS" and .release_status == "BLOCKED" and .expected_case_count == 9 and .rejected_case_count == 9 and .all_expected_mutations_rejected == true' "$report" >/dev/null
printf 'ADVERSARIAL_CORPUS_PASS: %s inconsistent-evidence mutations rejected; report: %s\n' "$cases" "$report"
