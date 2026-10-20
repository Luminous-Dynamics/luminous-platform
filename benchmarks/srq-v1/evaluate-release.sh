#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
corpus="$script_dir/scenarios.json"
receipt_validator="$script_dir/validate-result.sh"
expected_source_sha=""
evidence_root=""
output="-"
require_qualified=false
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
input_receipts="$tmpdir/input-receipts.txt"
summaries="$tmpdir/receipts.json"
: > "$input_receipts"
printf '[]\n' > "$summaries"

usage() {
  echo "Usage: $0 --expected-source-sha SHA40 --evidence-root DIR [--output REPORT.json] [--require-qualified] [receipt.json ...]" >&2
}

while (($#)); do
  case "$1" in
    --expected-source-sha)
      [[ $# -ge 2 ]] || { usage; exit 2; }
      expected_source_sha="$2"
      shift 2
      ;;
    --evidence-root)
      [[ $# -ge 2 ]] || { usage; exit 2; }
      evidence_root="$2"
      shift 2
      ;;
    --output)
      [[ $# -ge 2 ]] || { usage; exit 2; }
      output="$2"
      shift 2
      ;;
    --require-qualified)
      require_qualified=true
      shift
      ;;
    --)
      shift
      while (($#)); do
        printf '%s\n' "$1" >> "$input_receipts"
        shift
      done
      ;;
    -*)
      usage
      exit 2
      ;;
    *)
      printf '%s\n' "$1" >> "$input_receipts"
      shift
      ;;
  esac
done

if [[ ! "$expected_source_sha" =~ ^[0-9a-f]{40}$ ]]; then
  echo "BLOCKED: --expected-source-sha must be a full lowercase 40-hex source SHA." >&2
  exit 2
fi
if [[ -z "$evidence_root" || ! -d "$evidence_root" ]]; then
  echo "BLOCKED: --evidence-root must identify a directory containing referenced artifacts." >&2
  exit 2
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "BLOCKED: jq is required; qualification status was not determined." >&2
  exit 2
fi

bash "$script_dir/validate.sh" >/dev/null

while IFS= read -r receipt; do
  [[ -n "$receipt" ]] || continue
  [[ -f "$receipt" ]] || { echo "FAIL: receipt file is missing: $receipt" >&2; exit 1; }
  bash "$receipt_validator" --evidence-root "$evidence_root" "$receipt" >/dev/null

  receipt_expected="$(jq -r '.source.expected_source_sha' "$receipt")"
  scenario_id="$(jq -r '.scenario_id' "$receipt")"
  tier="$(jq -r '.execution.tier' "$receipt")"
  status="$(jq -r '.status' "$receipt")"
  steps_executed="$(jq -r '.execution.steps_executed' "$receipt")"
  receipt_sha="$(sha256sum "$receipt" | cut -d ' ' -f 1)"

  if [[ "$receipt_expected" != "$expected_source_sha" ]]; then
    echo "FAIL: receipt expected source differs from release source: $receipt" >&2
    exit 1
  fi
  if [[ "$steps_executed" == "true" ]]; then
    actual="$(jq -r '.source.actual_checkout_sha // ""' "$receipt")"
    matches="$(jq -r '.source.source_matches_expected // false' "$receipt")"
    if [[ "$actual" != "$expected_source_sha" || "$matches" != "true" ]]; then
      echo "FAIL: executed receipt is not bound to the exact release source SHA: $receipt" >&2
      exit 1
    fi
  fi
  if ! jq -e --arg id "$scenario_id" --arg tier "$tier" \
    'any(.scenarios[]; .id == $id and (.execution_tiers | index($tier)) != null)' "$corpus" >/dev/null; then
    echo "FAIL: receipt claims a scenario/tier not authorized by the corpus: $scenario_id / $tier" >&2
    exit 1
  fi
  if jq -e --arg id "$scenario_id" --arg tier "$tier" \
    'any(.[]; .scenario_id == $id and .tier == $tier)' "$summaries" >/dev/null; then
    echo "FAIL: duplicate result receipts for scenario/tier $scenario_id / $tier; resolve the conflict explicitly." >&2
    exit 1
  fi

  jq --arg id "$scenario_id" --arg tier "$tier" --arg status "$status" \
    --arg sha "$receipt_sha" --arg steps "$steps_executed" \
    '. + [{scenario_id:$id,tier:$tier,status:$status,receipt_sha256:$sha,steps_executed:($steps=="true")}]' \
    "$summaries" > "$tmpdir/receipts.next.json"
  mv "$tmpdir/receipts.next.json" "$summaries"
done < "$input_receipts"

corpus_sha="$(sha256sum "$corpus" | cut -d ' ' -f 1)"
jq --arg expected "$expected_source_sha" --arg corpus_sha "$corpus_sha" \
  --slurpfile observed "$summaries" '
  . as $corpus |
  [
    .scenarios[] |
    . as $scenario |
    [$observed[0][] | select(.scenario_id == $scenario.id)] as $all |
    [$all[] | select(.tier == $scenario.qualifying_tier)] as $required |
    {
      scenario_id: $scenario.id,
      qualifying_tier: $scenario.qualifying_tier,
      observed_tiers: ([$all[].tier] | unique),
      observations: [$all[] | {tier,status,receipt_sha256}],
      status: (
        if any($required[]; .status == "PASS" and .steps_executed == true) then "PASS"
        elif any($required[]; .status == "FAIL") then "FAIL"
        elif any($required[]; .status == "BLOCKED") then "BLOCKED"
        elif any($required[]; .status == "NOT_RUN") then "NOT_RUN"
        elif any($required[]; .status == "NOT_APPLICABLE") then "NOT_APPLICABLE"
        elif ($all | length) > 0 then "INSUFFICIENT_TIER"
        else "MISSING_EVIDENCE" end
      )
    }
  ] as $scenario_results |
  {
    schema_version: 1,
    profile_id: "SRQ-1",
    expected_source_sha: $expected,
    corpus_sha256: $corpus_sha,
    overall_status: (if all($scenario_results[]; .status == "PASS") then "QUALIFIED" else "NOT_QUALIFIED" end),
    summary: {
      total_release_gates: ($scenario_results | length),
      passed_at_qualifying_tier: ([$scenario_results[] | select(.status == "PASS")] | length),
      insufficient_tier: ([$scenario_results[] | select(.status == "INSUFFICIENT_TIER")] | length),
      failed: ([$scenario_results[] | select(.status == "FAIL")] | length),
      blocked: ([$scenario_results[] | select(.status == "BLOCKED")] | length),
      not_run: ([$scenario_results[] | select(.status == "NOT_RUN")] | length),
      not_applicable: ([$scenario_results[] | select(.status == "NOT_APPLICABLE")] | length),
      missing_evidence: ([$scenario_results[] | select(.status == "MISSING_EVIDENCE")] | length)
    },
    scenarios: $scenario_results,
    non_claims: [
      "QUALIFIED means only that configured SRQ-1 scenario/tier receipts passed this evaluator; it is not a blanket assertion that all security properties are proven.",
      "This report does not independently authenticate signer/device identity, trust roots, or runner integrity beyond recorded source and artifact hashes.",
      "The model tier cannot satisfy a scenario whose qualifying tier is QEMU or physical.",
      "A change to the corpus creates a new corpus digest and requires a fresh evaluation."
    ]
  }
' "$corpus" > "$tmpdir/report.json"

if [[ "$output" == "-" ]]; then
  cat "$tmpdir/report.json"
else
  cat "$tmpdir/report.json" > "$output"
fi

overall="$(jq -r '.overall_status' "$tmpdir/report.json")"
passed="$(jq -r '.summary.passed_at_qualifying_tier' "$tmpdir/report.json")"
total="$(jq -r '.summary.total_release_gates' "$tmpdir/report.json")"
printf 'SRQ-1 release status: %s (%s/%s gates met at their qualifying tier).\n' "$overall" "$passed" "$total" >&2

if [[ "$require_qualified" == "true" && "$overall" != "QUALIFIED" ]]; then
  echo "FAIL: --require-qualified requested but one or more release gates are not qualified." >&2
  exit 1
fi
