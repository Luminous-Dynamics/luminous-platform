#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
evidence_root=""
lock_path=""
run_path=""
output=""

usage() {
  echo "Usage: $0 --evidence-root DIR --comparison-lock-path RELPATH --comparison-run-path RELPATH --output FILE" >&2
}
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
safe_relative() {
  local p="$1"
  [[ "$p" =~ ^[A-Za-z0-9][A-Za-z0-9._/-]*$ && "$p" != *".."* && "$p" != *"//"* ]] || return 1
}
while (($#)); do
  case "$1" in
    --evidence-root) [[ $# -ge 2 ]] || { usage; exit 2; }; evidence_root="$2"; shift 2 ;;
    --comparison-lock-path) [[ $# -ge 2 ]] || { usage; exit 2; }; lock_path="$2"; shift 2 ;;
    --comparison-run-path) [[ $# -ge 2 ]] || { usage; exit 2; }; run_path="$2"; shift 2 ;;
    --output) [[ $# -ge 2 ]] || { usage; exit 2; }; output="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done
[[ -n "$evidence_root" && -n "$lock_path" && -n "$run_path" && -n "$output" ]] || { usage; exit 2; }
command -v jq >/dev/null 2>&1 || { echo "BLOCKED: jq is required." >&2; exit 2; }
command -v sha256sum >/dev/null 2>&1 || { echo "BLOCKED: sha256sum is required." >&2; exit 2; }
[[ -d "$evidence_root" ]] || fail "evidence root is missing"
safe_relative "$lock_path" || fail "unsafe comparison lock path"
safe_relative "$run_path" || fail "unsafe comparison run path"

root_real="$(realpath -e "$evidence_root")"
resolve_file() {
  local relative="$1" resolved
  safe_relative "$relative" || fail "unsafe evidence path: $relative"
  resolved="$(realpath -e "$root_real/$relative")" || fail "missing evidence file: $relative"
  case "$resolved" in "$root_real"/*) ;; *) fail "evidence path escapes root: $relative" ;; esac
  [[ -f "$resolved" ]] || fail "evidence path is not a regular file: $relative"
  printf '%s\n' "$resolved"
}
lock_file="$(resolve_file "$lock_path")"
run_file="$(resolve_file "$run_path")"
[[ "$lock_file" != "$run_file" ]] || fail "lock and run receipt must be different subjects"

jq -e '
  type == "object" and .schema_version == 1 and
  .profile_id == "SRQ-MATCHED-COMPARISON-RUN-V1" and
  (.fixture | type == "boolean") and
  (.run_challenge | type == "string" and test("^[0-9a-f]{64}$")) and
  (.comparison_lock.baseline_id | type == "string" and test("^[a-z0-9][a-z0-9.-]{1,80}$")) and
  (.comparison_lock.lock_sha256 | type == "string" and test("^[0-9a-f]{64}$")) and
  (.artifacts | type == "object" and length >= 1) and
  all(.artifacts[]; (.path | type == "string" and test("^[A-Za-z0-9][A-Za-z0-9._/-]*$") and (contains("..") | not) and (contains("//") | not)) and (.sha256 | type == "string" and test("^[0-9a-f]{64}$"))) and
  (.non_claims | type == "array" and length > 0 and all(.[]; type == "string" and length > 0))
' "$run_file" >/dev/null || fail "run receipt does not match the comparison statement preconditions"
jq -e --slurpfile run "$run_file" '
  .fixture == $run[0].fixture and
  .baseline_id == $run[0].comparison_lock.baseline_id and
  .srq_source_sha == $run[0].source.actual_srq_checkout_sha
' "$lock_file" >/dev/null || fail "lock identity does not match the run receipt"

lock_sha="$(sha256sum "$lock_file" | cut -d ' ' -f 1)"
run_lock_sha="$(jq -r '.comparison_lock.lock_sha256' "$run_file")"
[[ "$lock_sha" == "$run_lock_sha" ]] || fail "run receipt's lock digest differs from actual lock bytes"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
paths_file="$tmpdir/paths.txt"
subjects_file="$tmpdir/subjects.ndjson"
: > "$subjects_file"
{
  printf '%s\n' "$lock_path" "$run_path"
  jq -r '.artifacts[] | .path' "$run_file"
} | LC_ALL=C sort -u > "$paths_file"

while IFS= read -r relative; do
  [[ -n "$relative" ]] || continue
  file="$(resolve_file "$relative")"
  actual_sha="$(sha256sum "$file" | cut -d ' ' -f 1)"
  if [[ "$relative" == "$lock_path" ]]; then
    [[ "$actual_sha" == "$run_lock_sha" ]] || fail "comparison lock subject digest mismatch"
  elif [[ "$relative" != "$run_path" ]]; then
    expected_sha="$(jq -r --arg p "$relative" '[.artifacts[] | select(.path == $p) | .sha256] | unique | if length == 1 then .[0] else empty end' "$run_file")"
    [[ -n "$expected_sha" && "$expected_sha" == "$actual_sha" ]] || fail "run receipt has no unique matching digest for subject $relative"
  fi
  jq -cnS --arg name "$relative" --arg digest "$actual_sha" '{name:$name,digest:{sha256:$digest}}' >> "$subjects_file"
done < "$paths_file"

lock_sha="$(sha256sum "$lock_file" | cut -d ' ' -f 1)"
run_sha="$(sha256sum "$run_file" | cut -d ' ' -f 1)"
jq -cnS --slurpfile run "$run_file" --slurpfile lock "$lock_file" --slurpfile subjects "$subjects_file" \
  --arg lock_path "$lock_path" --arg run_path "$run_path" --arg lock_sha "$lock_sha" --arg run_sha "$run_sha" '
  {
    _type:"https://in-toto.io/Statement/v1",
    subject:$subjects,
    predicateType:"https://luminousdynamics.io/attestations/srq-matched-comparison/v1",
    predicate:{
      schema_version:1,
      profile_id:"SRQ-MATCHED-COMPARISON-ATTESTATION-V1",
      evidence_status:(if $run[0].fixture then "fixture_only" else "submitted_for_appraisal" end),
      run_challenge:$run[0].run_challenge,
      comparison_lock_path:$lock_path,
      comparison_run_path:$run_path,
      comparison_lock_sha256:$lock_sha,
      comparison_run_sha256:$run_sha,
      baseline_id:$lock[0].baseline_id,
      source:$run[0].source,
      environment:$run[0].environment,
      pairing:$run[0].pairing,
      artifacts:$run[0].artifacts,
      non_claims:$run[0].non_claims,
      claim_ceiling:"artifact_integrity_and_receipt_binding_only"
    }
  }
' > "$output"
jq -e --slurpfile schema "$script_dir/comparison-statement-v1.schema.json" '
  ._type == "https://in-toto.io/Statement/v1" and
  .predicateType == "https://luminousdynamics.io/attestations/srq-matched-comparison/v1" and
  .predicate.claim_ceiling == "artifact_integrity_and_receipt_binding_only" and
  (.subject | type == "array" and length >= 2)
' "$output" >/dev/null || fail "generated statement failed its semantic check"
printf 'STATEMENT_BUILT: subject_count=%s evidence_status=%s\n' "$(jq -r '.subject | length' "$output")" "$(jq -r '.predicate.evidence_status' "$output")"
echo "NON-CLAIM: this command does not sign the statement or appraise runtime truth; use a trusted envelope signer/verifier and the downstream qualification policy."
