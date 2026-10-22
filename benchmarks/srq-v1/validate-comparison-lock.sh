#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
registry="$script_dir/industry-baseline-candidates-v1.json"
schema="$script_dir/comparison-lock-v1.schema.json"
allow_fixtures=false
lock=""
usage() { echo "Usage: $0 [--allow-fixtures] [--registry PATH] <comparison-lock.json>" >&2; }
while (($#)); do
  case "$1" in
    --allow-fixtures) allow_fixtures=true; shift ;;
    --registry)
      if [[ $# -lt 2 || -z "$2" ]]; then usage; exit 2; fi
      registry="$2"; shift 2 ;;
    -*) usage; exit 2 ;;
    *)
      if [[ -n "$lock" ]]; then usage; exit 2; fi
      lock="$1"; shift ;;
  esac
done
[[ -n "$lock" ]] || { usage; exit 2; }
command -v jq >/dev/null 2>&1 || { echo "BLOCKED: jq is required; no lock result was produced." >&2; exit 2; }
command -v sha256sum >/dev/null 2>&1 || { echo "BLOCKED: sha256sum is required; no lock result was produced." >&2; exit 2; }
[[ -f "$lock" ]] || { echo "FAIL: comparison lock not found: $lock" >&2; exit 1; }
[[ -f "$registry" ]] || { echo "FAIL: baseline candidate registry not found: $registry" >&2; exit 1; }
[[ -f "$schema" ]] || { echo "FAIL: comparison lock schema is missing" >&2; exit 1; }
# Semantic contract validator; this intentionally does not claim full JSON Schema implementation.
jq -e '
  type == "object" and .schema_version == 1 and .profile_id == "SRQ-COMPARISON-LOCK-V1" and
  (.fixture | type == "boolean") and
  (.locked_at_utc | type == "string" and test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$")) and
  (.baseline_id | type == "string" and test("^[a-z0-9][a-z0-9-]{1,80}$")) and
  (.baseline_version | type == "string" and length > 0 and (test("(^|/)(latest|main|master)($|/)"; "i") | not)) and
  (.baseline_source | type == "object") and
  ((.baseline_source.resolved_commit_sha == null) or (.baseline_source.resolved_commit_sha | type == "string" and test("^[0-9a-f]{40}$"))) and
  ((.baseline_source.archive_sha3_256 == null) or (.baseline_source.archive_sha3_256 | type == "string" and test("^[0-9a-f]{64}$"))) and
  ((.baseline_source.sqlite_source_id == null) or (.baseline_source.sqlite_source_id | type == "string" and length > 0)) and
  (.srq_source_sha | type == "string" and test("^[0-9a-f]{40}$")) and
  (.benchmark_artifact_sha256 | type == "string" and test("^[0-9a-f]{64}$")) and
  (.configuration_sha256 | type == "string" and test("^[0-9a-f]{64}$")) and
  (.corpus_sha256 | type == "string" and test("^[0-9a-f]{64}$")) and
  (.fault_schedule_sha256 | type == "string" and test("^[0-9a-f]{64}$")) and
  (.outcome_oracle_sha256 | type == "string" and test("^[0-9a-f]{64}$")) and
  (.platform as $p | type == "object" and all(["os_image","architecture","kernel","filesystem","storage_class","runner_class"][]; . as $key | ($p[$key] | type == "string" and length > 0))) and
  (.scope | type == "object" and (.included | type == "array" and length > 0 and all(.[]; type == "string" and length > 0)) and (.excluded | type == "array" and length > 0 and all(.[]; type == "string" and length > 0)))
' "$lock" >/dev/null
fixture="$(jq -r '.fixture' "$lock")"
if [[ "$fixture" == "true" && "$allow_fixtures" != "true" ]]; then
  echo "FAIL: fixture=true comparison locks are examples, not executable run locks." >&2
  exit 1
fi
if [[ "$fixture" == "false" ]]; then
  expected="$(printenv SRQ_EXPECTED_SOURCE_SHA || true)"
  [[ "$expected" =~ ^[0-9a-f]{40}$ ]] || {
    echo "BLOCKED: non-fixture lock validation requires SRQ_EXPECTED_SOURCE_SHA to bind the exact SRQ source head." >&2
    exit 2
  }
  actual="$(jq -r '.srq_source_sha' "$lock")"
  [[ "$actual" == "$expected" ]] || {
    echo "FAIL: lock SRQ source SHA does not match expected source head (expected=$expected actual=$actual)." >&2
    exit 1
  }
fi
baseline_id="$(jq -r '.baseline_id' "$lock")"
if ! jq -e --arg id "$baseline_id" --slurpfile lock "$lock" '
  (.baselines | map(select(.id == $id))) as $matches |
  ($matches | length == 1) and
  ($matches[0].comparison_state == "pinned_reference_not_executed") and
  ($lock[0].baseline_version == $matches[0].source.version_or_id) and
  (($lock[0].baseline_source.resolved_commit_sha // null) == ($matches[0].source.resolved_commit_sha // null)) and
  (($lock[0].baseline_source.archive_sha3_256 // null) == ($matches[0].source.archive_sha3_256 // null)) and
  (($lock[0].baseline_source.sqlite_source_id // null) == ($matches[0].source.sqlite_source_id // null))
' "$registry" >/dev/null; then
  echo "FAIL: baseline is unknown, unpinned, protocol-only, scope-blocked, or does not match the frozen registry identity." >&2
  exit 1
fi
lock_sha="$(sha256sum "$lock" | cut -d ' ' -f 1)"
printf 'VALID_LOCK_ONLY: baseline_id=%s baseline_version=%s lock_sha256=%s fixture=%s\n' "$baseline_id" "$(jq -r '.baseline_version' "$lock")" "$lock_sha" "$fixture"
echo "NON-CLAIM: lock structure and registry binding do not prove that comparator artifacts were executed or that the baseline is secure."
