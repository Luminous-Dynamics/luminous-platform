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
schedule_src="$script_dir/fixtures/comparison-lock-artifacts/fault-schedule.json"
locked_schedule_path="$(jq -r '.artifacts.fault_schedule' "$lock")"
mkdir -p "$evidence_root/$(dirname "$locked_schedule_path")"
cp "$schedule_src" "$evidence_root/$locked_schedule_path"
schedule_sha="$(sha256sum "$evidence_root/$locked_schedule_path" | cut -d ' ' -f 1)"
baseline_trials="$evidence_root/baseline-trials.json"
srq_trials="$evidence_root/srq-trials.json"
receipt="$tmpdir/comparison-run.json"
input_sha="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

jq -n '{
  schema_version:1,
  trials:[range(0;15) as $i |
    (if ($i / 5 | floor) == 0 then "commit-before-ack"
     elif ($i / 5 | floor) == 1 then "sigkill-after-decision"
     else "sigkill-before-commit" end) as $fault |
    (if $fault == "commit-before-ack" then "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
     elif $fault == "sigkill-after-decision" then "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
     else "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" end) as $input |
    {trial_id:("trial-"+($i|tostring)),fault_id:$fault,input_sha256:$input,
     duration_ns:(10000000 + $i*1000),outcome:"old_complete",
     execution_order:(if ($i % 2) == 0 then "baseline_first" else "srq_first" end),fault_event_id:("baseline-inject-"+($i|tostring)),observation_event_id:("baseline-observe-"+($i|tostring))}]
}' > "$baseline_trials"
jq -n '{
  schema_version:1,
  trials:[range(0;15) as $i |
    (if ($i / 5 | floor) == 0 then "commit-before-ack"
     elif ($i / 5 | floor) == 1 then "sigkill-after-decision"
     else "sigkill-before-commit" end) as $fault |
    (if $fault == "commit-before-ack" then "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
     elif $fault == "sigkill-after-decision" then "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
     else "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" end) as $input |
    {trial_id:("trial-"+($i|tostring)),fault_id:$fault,input_sha256:$input,
     duration_ns:(9000000 + $i*1500),
     outcome:(if $i == 4 then "recoverable_blocked" else "old_complete" end),
     execution_order:(if ($i % 2) == 0 then "srq_first" else "baseline_first" end),fault_event_id:("srq-inject-"+($i|tostring)),observation_event_id:("srq-observe-"+($i|tostring))}]
}' > "$srq_trials"

write_observation_ledger() {
  local arm="$1" trials="$2" out="$3" state_digest
  if [[ "$arm" == "baseline" ]]; then state_digest="dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
  else state_digest="eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"; fi
  jq --arg digest "$state_digest" --slurpfile trials "$trials" '
    {schema_version:1,observations:[
      $trials[0].trials[] as $t |
      {observation_id:("obs-"+$t.trial_id),trial_id:$t.trial_id,fault_id:$t.fault_id,
       input_sha256:$t.input_sha256,outcome:$t.outcome,post_restart:true,
       observer_id:"fixture-independent-observer",state_digest_sha256:$digest}
    ]}
  ' > "$out"
}
write_event_ledger() {
  local arm="$1" trials="$2" observations="$3" out="$4" digest
  if [[ "$arm" == "baseline" ]]; then digest="dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
  else digest="eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"; fi
  jq --arg arm "$arm" --arg digest "$digest" --slurpfile trials "$trials" '
    {schema_version:1,events:(
      [$trials[0].trials[] as $t |
       {event_id:$t.fault_event_id,trial_id:$t.trial_id,fault_id:$t.fault_id,
        input_sha256:$t.input_sha256,event_kind:"fault_injection",source_role:"fault_injector",
        monotonic_ns:(100000 + (($t.trial_id|ltrimstr("trial-")|tonumber)*1000)),
        mechanism:"sigkill",trigger_confirmed:true}] +
      [$trials[0].trials[] as $t |
       {event_id:$t.observation_event_id,trial_id:$t.trial_id,fault_id:$t.fault_id,
        input_sha256:$t.input_sha256,event_kind:"post_restart_observation",source_role:"independent_observer",
        monotonic_ns:(1000000 + (($t.trial_id|ltrimstr("trial-")|tonumber)*1000)),
        observation_id:("obs-"+$t.trial_id),post_restart:true,outcome:$t.outcome,state_digest_sha256:$digest}]
    )}
  ' > "$out"
}
baseline_observations="$evidence_root/baseline-observations.json"
srq_observations="$evidence_root/srq-observations.json"
baseline_events="$evidence_root/baseline-events.json"
srq_events="$evidence_root/srq-events.json"
write_observation_ledger baseline "$baseline_trials" "$baseline_observations"
write_observation_ledger srq "$srq_trials" "$srq_observations"
write_event_ledger baseline "$baseline_trials" "$baseline_observations" "$baseline_events"
write_event_ledger srq "$srq_trials" "$srq_observations" "$srq_events"
baseline_events_sha="$(sha256sum "$baseline_events" | cut -d ' ' -f 1)"
srq_events_sha="$(sha256sum "$srq_events" | cut -d ' ' -f 1)"
baseline_observations_sha="$(sha256sum "$baseline_observations" | cut -d ' ' -f 1)"
srq_observations_sha="$(sha256sum "$srq_observations" | cut -d ' ' -f 1)"

lock_sha="$(sha256sum "$lock" | cut -d ' ' -f 1)"
baseline_sha="$(sha256sum "$baseline_trials" | cut -d ' ' -f 1)"
srq_sha="$(sha256sum "$srq_trials" | cut -d ' ' -f 1)"
jq -n --slurpfile lock "$lock" --arg lock_sha "$lock_sha" --arg baseline_sha "$baseline_sha" --arg srq_sha "$srq_sha" --arg schedule_sha "$schedule_sha" --arg schedule_path "$locked_schedule_path" --arg be_sha "$baseline_events_sha" --arg se_sha "$srq_events_sha" --arg bo_sha "$baseline_observations_sha" --arg so_sha "$srq_observations_sha" '
  {schema_version:1,profile_id:"SRQ-MATCHED-COMPARISON-RUN-V1",fixture:true,
   comparison_lock:{baseline_id:$lock[0].baseline_id,lock_sha256:$lock_sha},
   source:{expected_srq_source_sha:$lock[0].srq_source_sha,actual_srq_checkout_sha:$lock[0].srq_source_sha,baseline_source:$lock[0].baseline_source},
   environment:$lock[0].platform,
   pairing:{protocol:"paired_fault_trials_v1",minimum_paired_trials:5,order_policy:"randomized_per_pair",order_seed:42},
   artifacts:{baseline_trials:{path:"baseline-trials.json",sha256:$baseline_sha},srq_trials:{path:"srq-trials.json",sha256:$srq_sha},fault_schedule:{path:$schedule_path,sha256:$schedule_sha},baseline_events:{path:"baseline-events.json",sha256:$be_sha},srq_events:{path:"srq-events.json",sha256:$se_sha},baseline_observations:{path:"baseline-observations.json",sha256:$bo_sha},srq_observations:{path:"srq-observations.json",sha256:$so_sha}},
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
# Both arms can agree and still omit a predeclared fault; coverage is checked against the locked schedule.
make_both_arms_reject() {
  local label="$1" filter="$2"
  local b="$evidence_root/$label-baseline-trials.json" s="$evidence_root/$label-srq-trials.json"
  local br="$tmpdir/$label-receipt.json" out="$tmpdir/$label.out" bd sd
  jq "$filter" "$baseline_trials" > "$b"
  jq "$filter" "$srq_trials" > "$s"
  bd="$(sha256sum "$b" | cut -d ' ' -f 1)"
  sd="$(sha256sum "$s" | cut -d ' ' -f 1)"
  jq --arg bp "$(basename "$b")" --arg bd "$bd" --arg sp "$(basename "$s")" --arg sd "$sd"     '.artifacts.baseline_trials={path:$bp,sha256:$bd} | .artifacts.srq_trials={path:$sp,sha256:$sd}' "$receipt" > "$br"
  if bash "$validator" --allow-fixtures --evidence-root "$evidence_root" --comparison-lock "$lock" "$br" >"$out" 2>&1; then
    echo "FAIL: matched-run validator accepted schedule coverage defect '$label'" >&2; cat "$out" >&2; exit 1
  fi
  echo "PASS: locked fault schedule coverage rejected ($label)"
}
make_both_arms_reject missing-scheduled-fault '.trials |= map(select(.fault_id != "sigkill-before-commit"))'
make_both_arms_reject unknown-fault '.trials[0].fault_id = "unregistered-fault"'
make_both_arms_reject wrong-scheduled-input '.trials[0].input_sha256 = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"'
make_both_arms_reject underrepresented-fault '.trials[0].fault_id = "sigkill-after-decision" | .trials[0].input_sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"'


mutate_witness_reject() {
  local label="$1" artifact_key="$2" filter="$3"
  local source_path target_file receipt_variant digest out
  source_path="$(jq -r --arg key "$artifact_key" '.artifacts[$key].path' "$receipt")"
  target_file="$evidence_root/$label.json"
  jq "$filter" "$evidence_root/$source_path" > "$target_file"
  digest="$(sha256sum "$target_file" | cut -d ' ' -f 1)"
  receipt_variant="$tmpdir/$label-receipt.json"
  jq --arg key "$artifact_key" --arg path "$label.json" --arg digest "$digest" \
    '.artifacts[$key].path=$path | .artifacts[$key].sha256=$digest' "$receipt" > "$receipt_variant"
  out="$tmpdir/$label.out"
  if bash "$validator" --allow-fixtures --evidence-root "$evidence_root" --comparison-lock "$lock" "$receipt_variant" >"$out" 2>&1; then
    echo "FAIL: execution-witness validator accepted negative control '$label'" >&2; cat "$out" >&2; exit 1
  fi
  echo "PASS: execution-witness negative control rejected ($label)"
}
mutate_witness_reject missing-injector-event baseline_events '.events |= map(select(.event_kind != "fault_injection" or .trial_id != "trial-0"))'
mutate_witness_reject unconfirmed-injector baseline_events '.events |= map(if .event_kind == "fault_injection" and .trial_id == "trial-0" then .trigger_confirmed = false else . end)'
mutate_witness_reject observer-before-trigger baseline_events '.events |= map(if .event_kind == "post_restart_observation" and .trial_id == "trial-0" then .monotonic_ns = 1 else . end)'
mutate_witness_reject observer-outcome-mismatch baseline_events '.events |= map(if .event_kind == "post_restart_observation" and .trial_id == "trial-0" then .outcome = "state_lost" else . end)'
mutate_witness_reject observer-digest-mismatch baseline_events '.events |= map(if .event_kind == "post_restart_observation" and .trial_id == "trial-0" then .state_digest_sha256 = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff" else . end)'
mutate_witness_reject observation-not-after-restart baseline_observations '.observations |= map(if .trial_id == "trial-0" then .post_restart = false else . end)'
mutate_witness_reject missing-observation baseline_observations '.observations |= map(select(.trial_id != "trial-0"))'

printf '\n' >> "$baseline_trials"
if bash "$validator" --allow-fixtures --evidence-root "$evidence_root" --comparison-lock "$lock" "$receipt" >"$tmpdir/mutated-bytes.out" 2>&1; then
  echo "FAIL: matched-run validator accepted a mutated trial artifact" >&2; exit 1
fi
echo "PASS: matched-run validator rejected mutated raw trial bytes"

echo "PASS: matched-run comparator contract negative controls completed."
echo "NON-CLAIM: tests validate matching identities and paired trial structure; they do not execute a real industry baseline or establish superiority."
