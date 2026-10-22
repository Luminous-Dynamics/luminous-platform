#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
validator="$script_dir/validate-comparison-run.sh"
lock="$script_dir/fixtures/comparison-lock-tuf-v1.0.36.example.json"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
[[ -f "$lock" ]] || { echo "FAIL: comparison-lock fixture is missing" >&2; exit 1; }
mkdir -p "$tmpdir/evidence"
evidence_root="$tmpdir/evidence"
baseline_trials="$evidence_root/baseline-trials.json"
srq_trials="$evidence_root/srq-trials.json"
receipt="$tmpdir/comparison-run.json"
input_sha="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

jq -n --arg input "$input_sha" '{schema_version:1,trials:[range(0;5) as $i | {trial_id:("trial-"+($i|tostring)),fault_id:"sigkill-after-decision",input_sha256:$input,duration_ns:(10000000 + $i*1000),outcome:"old_complete",execution_order:(if ($i % 2) == 0 then "baseline_first" else "srq_first" end)}]}' > "$baseline_trials"
jq -n --arg input "$input_sha" '{schema_version:1,trials:[range(0;5) as $i | {trial_id:("trial-"+($i|tostring)),fault_id:"sigkill-after-decision",input_sha256:$input,duration_ns:(9000000 + $i*1500),outcome:(if $i == 4 then "recoverable_blocked" else "old_complete" end),execution_order:(if ($i % 2) == 0 then "baseline_first" else "srq_first" end)}]}' > "$srq_trials"
lock_sha="$(sha256sum "$lock" | cut -d ' ' -f 1)"
baseline_sha="$(sha256sum "$baseline_trials" | cut -d ' ' -f 1)"
srq_sha="$(sha256sum "$srq_trials" | cut -d ' ' -f 1)"
jq -n --slurpfile lock "$lock" --arg lock_sha "$lock_sha" --arg baseline_sha "$baseline_sha" --arg srq_sha "$srq_sha" '
  {schema_version:1,profile_id:"SRQ-MATCHED-COMPARISON-RUN-V1",fixture:true,
   comparison_lock:{baseline_id:$lock[0].baseline_id,lock_sha256:$lock_sha},
   source:{expected_srq_source_sha:$lock[0].srq_source_sha,actual_srq_checkout_sha:$lock[0].srq_source_sha,baseline_source:$lock[0].baseline_source},
   environment:$lock[0].platform,
   pairing:{protocol:"paired_fault_trials_v1",minimum_paired_trials:5,order_policy:"randomized_per_pair",order_seed:42},
   artifacts:{baseline_trials:{path:"baseline-trials.json",sha256:$baseline_sha},srq_trials:{path:"srq-trials.json",sha256:$srq_sha}},
   non_claims:["Synthetic fixture only.","Structural comparability is not a safety or performance win."]}
' > "$receipt"

bash "$validator" --allow-fixtures --evidence-root "$evidence_root" --comparison-lock "$lock" "$receipt"
if bash "$validator" --evidence-root "$evidence_root" --comparison-lock "$lock" "$receipt" >"$tmpdir/fixture-default.out" 2>&1; then
  echo "FAIL: fixture run receipt accepted without --allow-fixtures" >&2; exit 1
fi
echo "PASS: fixture comparison run rejected by default"

expect_reject_receipt() {
  local label="$1" filter="$2" mutated="$tmpdir/$1.json" out="$tmpdir/$1.out"
  jq "$filter" "$receipt" > "$mutated"
  if bash "$validator" --allow-fixtures --evidence-root "$evidence_root" --comparison-lock "$lock" "$mutated" >"$out" 2>&1; then
    echo "FAIL: matched-run validator accepted negative control '$label'" >&2; cat "$out" >&2; exit 1
  fi
  echo "PASS: matched-run negative control rejected ($label)"
}
expect_reject_receipt wrong-lock-digest '.comparison_lock.lock_sha256 = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"'
expect_reject_receipt wrong-baseline '.comparison_lock.baseline_id = "systemd-sysupdate-v261.4"'
expect_reject_receipt wrong-expected-source '.source.expected_srq_source_sha = "2222222222222222222222222222222222222222"'
expect_reject_receipt wrong-actual-checkout '.source.actual_srq_checkout_sha = "2222222222222222222222222222222222222222"'
expect_reject_receipt wrong-platform '.environment.runner_class = "different-runner"'
expect_reject_receipt wrong-baseline-source '.source.baseline_source.resolved_commit_sha = "ffffffffffffffffffffffffffffffffffffffff"'
expect_reject_receipt broken-baseline-artifact-sha '.artifacts.baseline_trials.sha256 = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"'
expect_reject_receipt unsafe-trial-path '.artifacts.baseline_trials.path = "../baseline-trials.json"'
expect_reject_receipt minimum-below-five '.pairing.minimum_paired_trials = 4'
expect_reject_receipt wrong-fixture-flag '.fixture = false'

mutate_baseline_and_expect_reject() {
  local label="$1" filter="$2" modified="$tmpdir/$1-trials.json" bad_receipt="$tmpdir/$1-receipt.json" out="$tmpdir/$1.out"
  jq "$filter" "$baseline_trials" > "$modified"
  local digest
  digest="$(sha256sum "$modified" | cut -d ' ' -f 1)"
  jq --arg path "$modified" --arg digest "$digest" '.artifacts.baseline_trials.path=$path | .artifacts.baseline_trials.sha256=$digest' "$receipt" > "$bad_receipt"
  # The absolute path is rejected before the digest check; for all other variants the test artifact must be under the evidence root.
  if [[ "$label" == "unsafe-absolute-path" ]]; then
    if bash "$validator" --allow-fixtures --evidence-root "$evidence_root" --comparison-lock "$lock" "$bad_receipt" >"$out" 2>&1; then echo "FAIL: unsafe absolute path was accepted" >&2; exit 1; fi
    echo "PASS: matched-run negative control rejected ($label)"
    return
  fi
  local rel="$label-trials.json"
  cp "$modified" "$evidence_root/$rel"
  jq --arg path "$rel" '.artifacts.baseline_trials.path=$path' "$bad_receipt" > "$tmpdir/$label-receipt-relative.json"
  if bash "$validator" --allow-fixtures --evidence-root "$evidence_root" --comparison-lock "$lock" "$tmpdir/$label-receipt-relative.json" >"$out" 2>&1; then
    echo "FAIL: matched-run validator accepted trial negative control '$label'" >&2; cat "$out" >&2; exit 1
  fi
  echo "PASS: matched-run negative control rejected ($label)"
}
mutate_baseline_and_expect_reject duplicate-trial-id '.trials[1].trial_id = .trials[0].trial_id'
mutate_baseline_and_expect_reject unknown-outcome '.trials[0].outcome = "QUEUED"'
mutate_baseline_and_expect_reject too-few-trials '.trials = .trials[0:4]'
mutate_baseline_and_expect_reject malformed-duration '.trials[0].duration_ns = -1'

# Cross-arm pairing defects must fail even when each artifact is individually well formed and re-hashed.
make_srq_variant_reject() {
  local label="$1" filter="$2" modified="$evidence_root/$1-srq-trials.json" bad_receipt="$tmpdir/$1-receipt.json" digest out="$tmpdir/$1.out"
  jq "$filter" "$srq_trials" > "$modified"
  digest="$(sha256sum "$modified" | cut -d ' ' -f 1)"
  jq --arg path "$(basename "$modified")" --arg digest "$digest" '.artifacts.srq_trials.path=$path | .artifacts.srq_trials.sha256=$digest' "$receipt" > "$bad_receipt"
  if bash "$validator" --allow-fixtures --evidence-root "$evidence_root" --comparison-lock "$lock" "$bad_receipt" >"$out" 2>&1; then
    echo "FAIL: matched-run validator accepted pairing defect '$label'" >&2; cat "$out" >&2; exit 1
  fi
  echo "PASS: matched-run pairing defect rejected ($label)"
}
make_srq_variant_reject mismatched-fault '.trials[0].fault_id = "different-fault"'
make_srq_variant_reject mismatched-input '.trials[0].input_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"'
make_srq_variant_reject mismatched-order '.trials[0].execution_order = "srq_first"'
make_srq_variant_reject missing-paired-trial '.trials = .trials[0:4]'

# Recorded digest must be checked against actual raw bytes.
printf '\n' >> "$baseline_trials"
if bash "$validator" --allow-fixtures --evidence-root "$evidence_root" --comparison-lock "$lock" "$receipt" >"$tmpdir/mutated-bytes.out" 2>&1; then
  echo "FAIL: matched-run validator accepted a mutated trial artifact" >&2; exit 1
fi
echo "PASS: matched-run validator rejected mutated raw trial bytes"

echo "PASS: matched-run comparator contract negative controls completed."
echo "NON-CLAIM: tests validate matching identities and paired trial structure; they do not execute a real industry baseline or establish superiority."
