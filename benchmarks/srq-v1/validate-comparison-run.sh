#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
lock_validator="$script_dir/validate-comparison-lock.sh"
schema="$script_dir/comparison-run-v1.schema.json"
allow_fixtures=false
evidence_root=""
comparison_lock=""
receipt=""

usage() {
  echo "Usage: $0 [--allow-fixtures] --evidence-root DIR --comparison-lock LOCK.json <comparison-run.json>" >&2
}
not_comparable() {
  echo "NOT_COMPARABLE: $*" >&2
  exit 1
}
while (($#)); do
  case "$1" in
    --allow-fixtures) allow_fixtures=true; shift ;;
    --evidence-root)
      if [[ $# -lt 2 || -z "$2" ]]; then usage; exit 2; fi
      evidence_root="$2"; shift 2 ;;
    --comparison-lock)
      if [[ $# -lt 2 || -z "$2" ]]; then usage; exit 2; fi
      comparison_lock="$2"; shift 2 ;;
    -*) usage; exit 2 ;;
    *)
      if [[ -n "$receipt" ]]; then usage; exit 2; fi
      receipt="$1"; shift ;;
  esac
done

[[ -n "$receipt" && -n "$comparison_lock" && -n "$evidence_root" ]] || { usage; exit 2; }
command -v jq >/dev/null 2>&1 || { echo "BLOCKED: jq is required; no comparison result was produced." >&2; exit 2; }
command -v sha256sum >/dev/null 2>&1 || { echo "BLOCKED: sha256sum is required; no comparison result was produced." >&2; exit 2; }
[[ -f "$schema" ]] || { echo "FAIL: comparison-run schema is missing" >&2; exit 1; }
[[ -f "$receipt" ]] || not_comparable "run receipt is missing: $receipt"
[[ -f "$comparison_lock" ]] || not_comparable "comparison lock is missing: $comparison_lock"
[[ -d "$evidence_root" ]] || not_comparable "evidence root is not a directory: $evidence_root"
[[ -x "$lock_validator" || -f "$lock_validator" ]] || { echo "FAIL: comparison-lock validator is missing" >&2; exit 1; }

# This is a semantic subset validator, not a complete JSON Schema implementation.
jq -e 'type == "object"' "$schema" >/dev/null
if ! jq -e '
  type == "object" and .schema_version == 1 and .profile_id == "SRQ-MATCHED-COMPARISON-RUN-V1" and
  (.fixture | type == "boolean") and
  (.comparison_lock | type == "object" and
    (.baseline_id | type == "string" and test("^[a-z0-9][a-z0-9.-]{1,80}$")) and
    (.lock_sha256 | type == "string" and test("^[0-9a-f]{64}$"))) and
  (.source | type == "object" and
    (.expected_srq_source_sha | type == "string" and test("^[0-9a-f]{40}$")) and
    (.actual_srq_checkout_sha | type == "string" and test("^[0-9a-f]{40}$")) and
    (.baseline_source | type == "object" and
      ((.resolved_commit_sha == null) or (.resolved_commit_sha | type == "string" and test("^[0-9a-f]{40}$"))) and
      ((.archive_sha3_256 == null) or (.archive_sha3_256 | type == "string" and test("^[0-9a-f]{64}$"))) and
      ((.sqlite_source_id == null) or (.sqlite_source_id | type == "string" and length > 0)))) and
  (.environment as $env | ($env | type == "object") and all(["os_image","architecture","kernel","filesystem","storage_class","runner_class"][]; . as $k | ($env[$k] | type == "string" and length > 0))) and
  (.pairing | type == "object" and .protocol == "paired_fault_trials_v1" and
    (.minimum_paired_trials | type == "number" and floor == . and . >= 5) and
    .order_policy == "randomized_per_pair" and
    (.order_seed | type == "number" and floor == . and . >= 0)) and
  (.artifacts as $artifacts | ($artifacts | type == "object") and
    all(["baseline_trials","srq_trials","fault_schedule"][]; . as $k |
      ($artifacts[$k] | type == "object" and
        (.path | type == "string" and test("^[A-Za-z0-9][A-Za-z0-9._/-]*$") and (startswith("/") | not) and (contains("..") | not) and (contains("//") | not)) and
        (.sha256 | type == "string" and test("^[0-9a-f]{64}$"))))) and
  (.non_claims | type == "array" and length > 0 and all(.[]; type == "string" and length > 0))
' "$receipt" >/dev/null; then
  not_comparable "run receipt does not satisfy the comparison-run v1 semantic contract"
fi

fixture="$(jq -r '.fixture' "$receipt")"
lock_fixture="$(jq -r '.fixture' "$comparison_lock")"
if [[ "$fixture" != "$lock_fixture" ]]; then
  not_comparable "run receipt fixture flag differs from the supplied lock's fixture flag"
fi
if [[ "$fixture" == "true" && "$allow_fixtures" != "true" ]]; then
  not_comparable "fixture run receipts are examples, not benchmark evidence; --allow-fixtures is test-only"
fi

# Validate the lock itself before comparing its exact bytes and frozen identities.
if [[ "$lock_fixture" == "true" ]]; then
  if [[ "$allow_fixtures" != "true" ]]; then
    not_comparable "fixture comparison locks require --allow-fixtures"
  fi
  bash "$lock_validator" --allow-fixtures "$comparison_lock" >/dev/null
else
  bash "$lock_validator" --evidence-root "$evidence_root" "$comparison_lock" >/dev/null
fi
actual_lock_sha="$(sha256sum "$comparison_lock" | cut -d ' ' -f 1)"
receipt_lock_sha="$(jq -r '.comparison_lock.lock_sha256' "$receipt")"
[[ "$receipt_lock_sha" == "$actual_lock_sha" ]] || not_comparable "comparison lock SHA-256 does not match exact lock-file bytes"

if ! jq -e --slurpfile lock "$comparison_lock" '
  .comparison_lock.baseline_id == $lock[0].baseline_id and
  .source.expected_srq_source_sha == $lock[0].srq_source_sha and
  .source.actual_srq_checkout_sha == $lock[0].srq_source_sha and
  .source.baseline_source == $lock[0].baseline_source and
  .environment == $lock[0].platform
' "$receipt" >/dev/null; then
  not_comparable "receipt source, baseline identity, or platform tuple differs from the frozen lock"
fi

# Compare the SHA-256 of the trial artifacts with the actual bytes under the evidence root.
resolve_artifact() {
  local key="$1" rel file expected actual
  rel="$(jq -r --arg key "$key" '.artifacts[$key].path' "$receipt")"
  expected="$(jq -r --arg key "$key" '.artifacts[$key].sha256' "$receipt")"
  [[ "$rel" != /* && "$rel" != *".."* && "$rel" != *"//"* ]] || not_comparable "unsafe trial artifact path: $rel"
  file="$evidence_root/$rel"
  [[ -f "$file" ]] || not_comparable "trial artifact is missing: $rel"
  actual="$(sha256sum "$file" | cut -d ' ' -f 1)"
  [[ "$actual" == "$expected" ]] || not_comparable "trial artifact digest mismatch: $rel"
  printf '%s\n' "$file"
}
baseline_trials="$(resolve_artifact baseline_trials)"
srq_trials="$(resolve_artifact srq_trials)"
min_trials="$(jq -r '.pairing.minimum_paired_trials' "$receipt")"

# Each file retains raw per-trial measurements. Summaries supplied by a caller are not trusted.
if ! jq -e --argjson min "$min_trials" '
  type == "object" and .schema_version == 1 and (.trials | type == "array" and length >= $min) and
  (.trials | all(.[]; 
    (.trial_id | type == "string" and test("^[A-Za-z0-9][A-Za-z0-9._:-]{0,120}$")) and
    (.fault_id | type == "string" and test("^[A-Za-z0-9][A-Za-z0-9._:-]{0,120}$")) and
    (.input_sha256 | type == "string" and test("^[0-9a-f]{64}$")) and
    (.duration_ns | type == "number" and floor == . and . > 0) and
    (.outcome == "old_complete" or .outcome == "new_complete" or .outcome == "recoverable_blocked" or .outcome == "state_lost" or .outcome == "state_inconsistent" or .outcome == "unavailable_unobserved" or .outcome == "unauthorized_accept" or .outcome == "replay_accept" or .outcome == "hybrid_state" or .outcome == "unclassified_fault") and
    (.execution_order == "baseline_first" or .execution_order == "srq_first"))) and
  (([.trials[].trial_id] | length) == ([.trials[].trial_id] | unique | length))
' "$baseline_trials" >/dev/null; then
  not_comparable "baseline raw-trial file is malformed, has duplicate IDs, or has fewer than $min trials"
fi
if ! jq -e --argjson min "$min_trials" '
  type == "object" and .schema_version == 1 and (.trials | type == "array" and length >= $min) and
  (.trials | all(.[]; 
    (.trial_id | type == "string" and test("^[A-Za-z0-9][A-Za-z0-9._:-]{0,120}$")) and
    (.fault_id | type == "string" and test("^[A-Za-z0-9][A-Za-z0-9._:-]{0,120}$")) and
    (.input_sha256 | type == "string" and test("^[0-9a-f]{64}$")) and
    (.duration_ns | type == "number" and floor == . and . > 0) and
    (.outcome == "old_complete" or .outcome == "new_complete" or .outcome == "recoverable_blocked" or .outcome == "state_lost" or .outcome == "state_inconsistent" or .outcome == "unavailable_unobserved" or .outcome == "unauthorized_accept" or .outcome == "replay_accept" or .outcome == "hybrid_state" or .outcome == "unclassified_fault") and
    (.execution_order == "baseline_first" or .execution_order == "srq_first"))) and
  (([.trials[].trial_id] | length) == ([.trials[].trial_id] | unique | length))
' "$srq_trials" >/dev/null; then
  not_comparable "SRQ raw-trial file is malformed, has duplicate IDs, or has fewer than $min trials"
fi

if ! jq -e --slurpfile baseline "$baseline_trials" --slurpfile srq "$srq_trials" '
  ($baseline[0].trials | length) == ($srq[0].trials | length) and
  (($baseline[0].trials | map(.trial_id) | sort) == ($srq[0].trials | map(.trial_id) | sort)) and
  all($baseline[0].trials[]; . as $b |
    any($srq[0].trials[]; .trial_id == $b.trial_id and .fault_id == $b.fault_id and .input_sha256 == $b.input_sha256 and .execution_order == $b.execution_order))
' "$receipt" >/dev/null; then
  not_comparable "baseline and SRQ do not have identical paired trial IDs, fault IDs, input digests, and recorded execution order"
fi

fault_schedule="$(resolve_artifact fault_schedule)"
locked_schedule_path="$(jq -r '.artifacts.fault_schedule' "$comparison_lock")"
locked_schedule_sha="$(jq -r '.fault_schedule_sha256' "$comparison_lock")"
receipt_schedule_path="$(jq -r '.artifacts.fault_schedule.path' "$receipt")"
receipt_schedule_sha="$(jq -r '.artifacts.fault_schedule.sha256' "$receipt")"
[[ "$receipt_schedule_path" == "$locked_schedule_path" ]] || not_comparable "trial receipt does not point to the fault schedule pinned by the comparison lock"
[[ "$receipt_schedule_sha" == "$locked_schedule_sha" ]] || not_comparable "trial receipt fault-schedule digest differs from the comparison lock"
actual_schedule_sha="$(sha256sum "$fault_schedule" | cut -d ' ' -f 1)"
[[ "$actual_schedule_sha" == "$locked_schedule_sha" ]] || not_comparable "fault schedule bytes differ from the comparison lock"

# The schedule is the predeclared denominator. Pairing two arms that both omit a fault is not coverage.
if ! jq -e '
  type == "object" and .schema_version == 1 and .profile_id == "SRQ-FAULT-SCHEDULE-V1" and
  (.faults | type == "array" and length > 0) and
  (([.faults[].fault_id] | length) == ([.faults[].fault_id] | unique | length)) and
  all(.faults[];
    (.fault_id | type == "string" and test("^[A-Za-z0-9][A-Za-z0-9._:-]{0,120}$")) and
    (.input_sha256 | type == "string" and test("^[0-9a-f]{64}$")) and
    (.minimum_trials | type == "number" and floor == . and . >= 1))
' "$fault_schedule" >/dev/null; then
  not_comparable "locked fault schedule is malformed or has duplicate fault IDs"
fi

check_schedule_coverage() {
  local label="$1" trials="$2"
  if ! jq -e --slurpfile schedule "$fault_schedule" '
    .trials as $trials |
    all($trials[];
      . as $trial |
      any($schedule[0].faults[];
        .fault_id == $trial.fault_id and .input_sha256 == $trial.input_sha256)) and
    all($schedule[0].faults[];
      . as $fault |
      ([$trials[] | select(.fault_id == $fault.fault_id and .input_sha256 == $fault.input_sha256)] | length) >= $fault.minimum_trials)
  ' "$trials" >/dev/null; then
    not_comparable "$label trial set does not cover every locked fault/input pair at its required repetition count"
  fi
}
check_schedule_coverage "baseline" "$baseline_trials"
check_schedule_coverage "SRQ" "$srq_trials"

paired_count="$(jq -r --slurpfile baseline "$baseline_trials" '. as $r | $baseline[0].trials | length' "$receipt")"
if [[ "$fixture" == "true" ]]; then
  printf 'VALID_FIXTURE_ONLY: paired-trial contract is structurally valid (baseline=%s, trials=%s); fixture is not benchmark evidence.\n' "$(jq -r '.comparison_lock.baseline_id' "$receipt")" "$paired_count"
else
  printf 'STRUCTURALLY_COMPARABLE: paired-trial contract is valid (baseline=%s, trials=%s, lock_sha256=%s).\n' "$(jq -r '.comparison_lock.baseline_id' "$receipt")" "$paired_count" "$actual_lock_sha"
fi
echo "NON-CLAIM: this validator checks byte integrity, pinned identities, platform equality, and paired trial keys. It does not authenticate signer/device identity, prove that randomized order was executed, prove complete corpus coverage, establish statistical significance, or show a safety/performance win or release qualification."
