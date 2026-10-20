#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
corpus="${1:-$script_dir/scenarios.json}"
baselines="${2:-$script_dir/industry-baselines.tsv}"

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
