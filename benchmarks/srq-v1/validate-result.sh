#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
schema="$script_dir/result-receipt-v1.schema.json"
corpus="$script_dir/scenarios.json"
allow_fixtures=false
evidence_root=""
receipt=""

usage() {
  echo "Usage: $0 [--allow-fixtures] [--evidence-root DIR] <result-receipt.json>" >&2
}

while (($#)); do
  case "$1" in
    --allow-fixtures)
      allow_fixtures=true
      shift
      ;;
    --evidence-root)
      if [[ $# -lt 2 || -z "$2" ]]; then
        usage
        exit 2
      fi
      evidence_root="$2"
      shift 2
      ;;
    -*)
      usage
      exit 2
      ;;
    *)
      if [[ -n "$receipt" ]]; then
        usage
        exit 2
      fi
      receipt="$1"
      shift
      ;;
  esac
done

if [[ -z "$receipt" ]]; then
  usage
  exit 2
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "BLOCKED: jq is required; no receipt validation result was produced." >&2
  exit 2
fi
[[ -f "$schema" ]] || { echo "FAIL: result schema is missing" >&2; exit 1; }
[[ -f "$corpus" ]] || { echo "FAIL: scenario corpus is missing" >&2; exit 1; }
[[ -f "$receipt" ]] || { echo "FAIL: result receipt is missing: $receipt" >&2; exit 1; }

# This is a reference semantic validator for SRQ's status/provenance invariants.
# It intentionally does not claim to implement the complete JSON Schema spec.
jq -e 'type == "object"' "$schema" >/dev/null

jq -e '
  .schema_version == 1 and
  .profile_id == "SRQ-1" and
  (.scenario_id | type == "string" and test("^[A-Z]+-[0-9]{3}$")) and
  (.status == "PASS" or .status == "FAIL" or .status == "NOT_RUN" or .status == "BLOCKED" or .status == "NOT_APPLICABLE") and
  (.reason | type == "string" and length > 0) and
  (.fixture | type == "boolean") and
  (.source.expected_source_sha | type == "string" and test("^[0-9a-f]{40}$")) and
  ((.source.event_sha == null) or (.source.event_sha | type == "string" and test("^[0-9a-f]{40}$"))) and
  ((.source.actual_checkout_sha == null) or (.source.actual_checkout_sha | type == "string" and test("^[0-9a-f]{40}$"))) and
  ((.source.source_matches_expected == null) or (.source.source_matches_expected | type == "boolean")) and
  (.execution.tier == "static" or .execution.tier == "model" or .execution.tier == "qemu" or .execution.tier == "physical") and
  (.execution.steps_executed | type == "boolean") and
  ((.execution.command == null) or (.execution.command | type == "string")) and
  ((.execution.exit_code == null) or (.execution.exit_code | type == "number" and floor == .)) and
  (.execution.environment.os == null or (.execution.environment.os | type == "string")) and
  (.execution.environment.architecture == null or (.execution.environment.architecture | type == "string")) and
  (.execution.environment.toolchain == null or (.execution.environment.toolchain | type == "string")) and
  (.evidence | type == "array" and all(.[]; (.artifact | type == "string" and length > 0 and test("^[A-Za-z0-9][A-Za-z0-9._/-]*$") and (startswith("/") | not) and (contains("..") | not) and (contains("//") | not)) and (.sha256 | type == "string" and test("^[0-9a-f]{64}$")) and (.kind == "log" or .kind == "receipt" or .kind == "capture" or .kind == "manifest" or .kind == "benchmark" or .kind == "other"))) and
  (.claim_ceiling == "none" or .claim_ceiling == "static_contract_only" or .claim_ceiling == "abstract_model_only" or .claim_ceiling == "qemu_guest_recovery_only" or .claim_ceiling == "physical_device_recovery_only") and
  (if .execution.tier == "static" then (.claim_ceiling == "none" or .claim_ceiling == "static_contract_only")
   elif .execution.tier == "model" then (.claim_ceiling == "none" or .claim_ceiling == "static_contract_only" or .claim_ceiling == "abstract_model_only")
   elif .execution.tier == "qemu" then (.claim_ceiling == "none" or .claim_ceiling == "static_contract_only" or .claim_ceiling == "abstract_model_only" or .claim_ceiling == "qemu_guest_recovery_only")
   else true end) and
  (.non_claims | type == "array" and length > 0 and all(.[]; type == "string" and length > 0)) and
  (if .status == "PASS" then
     (.source.actual_checkout_sha != null) and
     (.source.actual_checkout_sha == .source.expected_source_sha) and
     (.source.source_matches_expected == true) and
     (.execution.steps_executed == true) and
     (.execution.exit_code == 0) and
     (.evidence | length > 0)
   elif .status == "NOT_RUN" then
     (.source.actual_checkout_sha == null) and
     (.source.source_matches_expected == null) and
     (.execution.steps_executed == false) and
     (.execution.command == null) and
     (.execution.exit_code == null)
   elif .status == "NOT_APPLICABLE" then
     (.execution.steps_executed == false) and (.execution.exit_code == null)
   else true end)
' "$receipt" >/dev/null

scenario="$(jq -r '.scenario_id' "$receipt")"
fixture="$(jq -r '.fixture' "$receipt")"
status="$(jq -r '.status' "$receipt")"

if ! jq -e --arg scenario "$scenario" 'any(.scenarios[]; .id == $scenario)' "$corpus" >/dev/null; then
  echo "FAIL: receipt references scenario '$scenario', which is not present in the SRQ-1 corpus." >&2
  exit 1
fi
if [[ "$fixture" == "true" && "$allow_fixtures" != "true" ]]; then
  echo "FAIL: fixture=true receipts are examples, not qualification evidence; --allow-fixtures is for tests only." >&2
  exit 1
fi
if [[ "$status" == "PASS" && "$fixture" == "false" && -z "$evidence_root" ]]; then
  echo "FAIL: non-fixture PASS receipts require --evidence-root so evidence bytes can be verified." >&2
  exit 1
fi

if [[ -n "$evidence_root" ]]; then
  [[ -d "$evidence_root" ]] || { echo "FAIL: evidence root is not a directory: $evidence_root" >&2; exit 1; }
  while IFS=$'\t' read -r artifact expected_sha; do
    [[ -n "$artifact" ]] || continue
    # Evidence locators must be relative and must not escape the supplied root.
    if [[ "$artifact" == /* ]]; then
      echo "FAIL: absolute evidence locator is not allowed: $artifact" >&2
      exit 1
    fi
    IFS='/' read -r -a components <<< "$artifact"
    for component in "${components[@]}"; do
      if [[ "$component" == ".." || "$component" == "." || -z "$component" ]]; then
        echo "FAIL: unsafe evidence locator: $artifact" >&2
        exit 1
      fi
    done
    file="$evidence_root/$artifact"
    if [[ ! -f "$file" ]]; then
      echo "FAIL: evidence artifact is missing: $artifact" >&2
      exit 1
    fi
    actual_sha="$(sha256sum "$file" | cut -d ' ' -f 1)"
    if [[ "$actual_sha" != "$expected_sha" ]]; then
      echo "FAIL: evidence digest mismatch for $artifact (expected=$expected_sha actual=$actual_sha)" >&2
      exit 1
    fi
    echo "PASS: evidence bytes verified ($artifact sha256=$actual_sha)"
  done < <(jq -r '.evidence[]? | [.artifact, .sha256] | @tsv' "$receipt")
fi

if [[ "$fixture" == "true" ]]; then
  printf 'VALID_FIXTURE_ONLY: result receipt structure is valid (scenario=%s example_status=%s); this fixture is not qualification evidence.\n' "$scenario" "$status"
else
  printf 'PASS: result receipt structure and status invariants are valid (scenario=%s status=%s fixture=false).\n' "$scenario" "$status"
fi
echo "NON-CLAIM: this reference subset validator does not implement the full JSON Schema specification or authenticate signer/device identity, trust roots, or runtime qualification."
