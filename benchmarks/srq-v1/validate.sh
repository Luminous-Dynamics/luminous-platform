#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
corpus="${1:-$script_dir/scenarios.json}"
baselines="${2:-$script_dir/industry-baselines.tsv}"
baseline_registry="${3:-$script_dir/industry-baseline-candidates-v1.json}"

if ! command -v jq >/dev/null 2>&1; then
  echo "BLOCKED: jq is required to validate the scenario corpus; no qualification result was produced." >&2
  exit 2
fi

if [[ ! -f "$corpus" ]]; then
  echo "FAIL: scenario corpus not found: $corpus" >&2
  exit 1
fi
if [[ ! -f "$baselines" ]]; then
  echo "FAIL: industry baseline matrix not found: $baselines" >&2
  exit 1
fi
if [[ ! -f "$baseline_registry" ]]; then
  echo "FAIL: industry baseline candidate registry not found: $baseline_registry" >&2
  exit 1
fi

jq -e '
  .profile_id == "SRQ-1" and
  .schema_version == 1 and
  .qualification_status == "proposed_not_executed" and
  (.scenarios | type == "array" and length >= 20) and
  (([.scenarios[].id] | length) == ([.scenarios[].id] | unique | length)) and
  all(.scenarios[];
    . as $scenario |
    ($scenario.id | type == "string" and test("^[A-Z]+-[0-9]{3}$")) and
    ($scenario.surface == "luminous-platform" or $scenario.surface == "sovereign-ops" or $scenario.surface == "sovereign-boot" or $scenario.surface == "all") and
    ($scenario.severity == "release_gate") and
    ($scenario.fault_injection | type == "string" and length > 0) and
    ($scenario.expected_state | type == "string" and length > 0) and
    ($scenario.forbidden_outcomes | type == "array" and length > 0 and all(.[]; type == "string" and length > 0)) and
    ($scenario.required_evidence | type == "array" and length > 0 and all(.[]; type == "string" and length > 0)) and
    ($scenario.execution_tiers | type == "array" and length > 0 and all(.[]; . == "static" or . == "model" or . == "qemu" or . == "physical")) and
    ($scenario.qualifying_tier == "static" or $scenario.qualifying_tier == "model" or $scenario.qualifying_tier == "qemu" or $scenario.qualifying_tier == "physical") and
    (($scenario.execution_tiers | index($scenario.qualifying_tier)) != null) and
    ($scenario.release_gate == true) and
    ($scenario.status == "proposed_not_executed" or $scenario.status == "partial_model_pass_not_qualified") and
    (($scenario.observed_tiers // []) | type == "array") and
    all(($scenario.observed_tiers // [])[]; . == "static" or . == "model" or . == "qemu" or . == "physical") and
    all(($scenario.observed_tiers // [])[]; . as $tier | ($scenario.execution_tiers | index($tier)) != null) and
    (if $scenario.status == "partial_model_pass_not_qualified"
     then (($scenario.observed_tiers // []) | length > 0)
     else (($scenario.observed_tiers // []) | length == 0)
     end)
  )
' "$corpus" >/dev/null

jq -e '
  .schema_version == 1 and
  .profile_id == "SRQ-INDUSTRY-BASELINE-CANDIDATES-V1" and
  .status == "candidate_registry_only_not_execution_lock" and
  .as_of_date == "2026-10-22" and
  .missing_run_lock_disposition == "NOT_COMPARABLE" and
  (.required_per_run_fields | type == "array" and length >= 8 and length == (unique | length)) and
  (.baselines | type == "array" and length == 11) and
  (([.baselines[].id] | length) == ([.baselines[].id] | unique | length)) and
  (([.baselines[].baseline] | length) == ([.baselines[].baseline] | unique | length)) and
  all(.baselines[];
    (.id | type == "string" and length > 0) and
    (.baseline | type == "string" and length > 0) and
    (.comparison_state == "pinned_reference_not_executed" or
     .comparison_state == "run_pin_required" or
     .comparison_state == "blocked_pending_scope_review" or
     .comparison_state == "protocol_reference_only") and
    (.source.reference_url | type == "string" and test("^https://[^[:space:]]+$")) and
    (.security_caveats | type == "array" and length > 0 and all(.[]; type == "string" and length > 0)) and
    (if .comparison_state == "pinned_reference_not_executed" then
      (.source.version_or_id | type == "string" and length > 0) and
      (((.source.resolved_commit_sha // "") | test("^[0-9a-f]{40}$")) or
       ((.source.archive_sha3_256 // "") | test("^[0-9a-f]{64}$")))
     elif .comparison_state == "blocked_pending_scope_review" then
      (.security_caveats | length > 0) and
      ((.source.resolved_commit_sha // "") | test("^[0-9a-f]{40}$"))
     elif .comparison_state == "run_pin_required" then
      (.source.version_or_id == null or .source.version_or_id == "")
     else (.source.version_or_id | type == "string" and length > 0)
     end)
  )
' "$baseline_registry" >/dev/null

while IFS=
  NR == 1 {
    if (NF != 6 || $1 != "baseline" || $2 != "baseline_scope" || $3 != "SRQ_comparison" || $4 != "measured_outputs" || $5 != "claim_boundary" || $6 != "reference_url") {
      print "FAIL: industry baseline header must contain the six SRQ v1 columns" > "/dev/stderr"
      bad = 1
    }
    next
  }
  {
    rows++
    if (NF != 6) {
      printf "FAIL: baseline row %d has %d columns; expected 6\n", NR, NF > "/dev/stderr"
      bad = 1
    }
    for (i = 1; i <= 6; i++) {
      if ($i == "") {
        printf "FAIL: baseline row %d has an empty column %d\n", NR, i > "/dev/stderr"
        bad = 1
      }
    }
    if ($6 !~ /^https:\/\/[^[:space:]]+$/) {
      printf "FAIL: baseline row %d reference_url must be an HTTPS URL\n", NR > "/dev/stderr"
      bad = 1
    }
    if ($1 in seen) {
      printf "FAIL: duplicate baseline name on row %d: %s\n", NR, $1 > "/dev/stderr"
      bad = 1
    }
    seen[$1] = 1
  }
  END {
    if (NR < 9 || rows < 8) {
      print "FAIL: industry baseline matrix must contain at least eight data rows" > "/dev/stderr"
      bad = 1
    }
    if (bad) exit 1
    printf "PASS: industry baseline matrix structure is valid (%d baselines).\n", rows
  }
' "$baselines"

count="$(jq '.scenarios | length' "$corpus")"
gates="$(jq '[.scenarios[] | select(.release_gate == true)] | length' "$corpus")"
digest="$(sha256sum "$corpus" | cut -d ' ' -f 1)"
baseline_digest="$(sha256sum "$baselines" | cut -d ' ' -f 1)"
echo "PASS: SRQ-1 corpus structure is valid ($count scenarios; $gates release gates; sha256=$digest)."
registry_digest="$(sha256sum "$baseline_registry" | cut -d ' ' -f 1)"
echo "PASS: industry-baselines.tsv sha256=$baseline_digest."
echo "PASS: industry-baseline-candidates-v1.json sha256=$registry_digest (registry only; per-run lock still required)."
echo "NON-CLAIM: no runtime scenario was executed; this is not a runtime or security qualification."
\t' read -r baseline_name _; do
  [[ -n "$baseline_name" && "$baseline_name" != "baseline" ]] || continue
  if ! jq -e --arg name "$baseline_name" 'any(.baselines[]; .baseline == $name)' "$baseline_registry" >/dev/null; then
    echo "FAIL: baseline matrix entry has no matching registry item: $baseline_name" >&2
    exit 1
  fi
done < "$baselines"

awk -F '\t' '
  NR == 1 {
    if (NF != 6 || $1 != "baseline" || $2 != "baseline_scope" || $3 != "SRQ_comparison" || $4 != "measured_outputs" || $5 != "claim_boundary" || $6 != "reference_url") {
      print "FAIL: industry baseline header must contain the six SRQ v1 columns" > "/dev/stderr"
      bad = 1
    }
    next
  }
  {
    rows++
    if (NF != 6) {
      printf "FAIL: baseline row %d has %d columns; expected 6\n", NR, NF > "/dev/stderr"
      bad = 1
    }
    for (i = 1; i <= 6; i++) {
      if ($i == "") {
        printf "FAIL: baseline row %d has an empty column %d\n", NR, i > "/dev/stderr"
        bad = 1
      }
    }
    if ($6 !~ /^https:\/\/[^[:space:]]+$/) {
      printf "FAIL: baseline row %d reference_url must be an HTTPS URL\n", NR > "/dev/stderr"
      bad = 1
    }
    if ($1 in seen) {
      printf "FAIL: duplicate baseline name on row %d: %s\n", NR, $1 > "/dev/stderr"
      bad = 1
    }
    seen[$1] = 1
  }
  END {
    if (NR < 9 || rows < 8) {
      print "FAIL: industry baseline matrix must contain at least eight data rows" > "/dev/stderr"
      bad = 1
    }
    if (bad) exit 1
    printf "PASS: industry baseline matrix structure is valid (%d baselines).\n", rows
  }
' "$baselines"

count="$(jq '.scenarios | length' "$corpus")"
gates="$(jq '[.scenarios[] | select(.release_gate == true)] | length' "$corpus")"
digest="$(sha256sum "$corpus" | cut -d ' ' -f 1)"
baseline_digest="$(sha256sum "$baselines" | cut -d ' ' -f 1)"
echo "PASS: SRQ-1 corpus structure is valid ($count scenarios; $gates release gates; sha256=$digest)."
echo "PASS: industry-baselines.tsv sha256=$baseline_digest."
echo "NON-CLAIM: no runtime scenario was executed; this is not a runtime or security qualification."
