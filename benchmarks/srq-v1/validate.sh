#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
corpus="${1:-$script_dir/scenarios.json}"

if ! command -v jq >/dev/null 2>&1; then
  echo "BLOCKED: jq is required to validate the scenario corpus; no qualification result was produced." >&2
  exit 2
fi

if [[ ! -f "$corpus" ]]; then
  echo "FAIL: scenario corpus not found: $corpus" >&2
  exit 1
fi

jq -e '
  .profile_id == "SRQ-1" and
  .schema_version == 1 and
  .qualification_status == "proposed_not_executed" and
  (.scenarios | type == "array" and length >= 20) and
  (([.scenarios[].id] | length) == ([.scenarios[].id] | unique | length)) and
  all(.scenarios[];
    (.id | type == "string" and test("^[A-Z]+-[0-9]{3}$")) and
    (.surface == "luminous-platform" or .surface == "sovereign-ops" or .surface == "sovereign-boot" or .surface == "all") and
    (.severity == "release_gate") and
    (.fault_injection | type == "string" and length > 0) and
    (.expected_state | type == "string" and length > 0) and
    (.forbidden_outcomes | type == "array" and length > 0 and all(.[]; type == "string" and length > 0)) and
    (.required_evidence | type == "array" and length > 0 and all(.[]; type == "string" and length > 0)) and
    (.execution_tiers | type == "array" and length > 0 and all(.[]; . == "static" or . == "model" or . == "qemu" or . == "physical")) and
    (.release_gate == true) and
    (.status == "proposed_not_executed")
  )
' "$corpus" >/dev/null

count="$(jq '.scenarios | length' "$corpus")"
gates="$(jq '[.scenarios[] | select(.release_gate == true)] | length' "$corpus")"
digest="$(sha256sum "$corpus" | cut -d ' ' -f 1)"
echo "PASS: SRQ-1 corpus structure is valid ($count scenarios; $gates release gates; sha256=$digest)."
echo "NON-CLAIM: no scenario was executed; this is not a runtime or security qualification."
