#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
evaluator="$script_dir/evaluate-release.sh"
if (($# < 3)); then
  echo "Usage: $0 EXPECTED_SOURCE_SHA EVIDENCE_ROOT RECEIPT..." >&2
  exit 2
fi
expected="$1"
evidence_root="$2"
shift 2
receipts="$@"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
report="$tmpdir/current-report.json"

# Current CI intentionally has model evidence for only two scenarios. The
# compiler must remain NOT_QUALIFIED because neither reaches its required tier.
bash "$evaluator" --expected-source-sha "$expected" --evidence-root "$evidence_root" \
  --output "$report" "$@"
jq -e '
  .overall_status == "NOT_QUALIFIED" and
  .summary.total_release_gates == 22 and
  .summary.passed_at_qualifying_tier == 0 and
  .summary.insufficient_tier == 2 and
  .summary.missing_evidence == 20 and
  any(.scenarios[]; .scenario_id == "UPDATE-004" and .qualifying_tier == "physical" and .status == "INSUFFICIENT_TIER") and
  any(.scenarios[]; .scenario_id == "EVIDENCE-003" and .qualifying_tier == "qemu" and .status == "INSUFFICIENT_TIER")
' "$report" >/dev/null
echo "PASS: model receipts cannot satisfy QEMU/physical qualifying tiers"

# Even a receipt labelled as physical cannot satisfy that gate when its own
# claim ceiling only promises an abstract model result.
jq '.execution.tier = "physical"' "$1" > "$tmpdir/high-tier-low-ceiling.json"
bash "$evaluator" --expected-source-sha "$expected" --evidence-root "$evidence_root" \
  --output "$tmpdir/low-ceiling-report.json" "$tmpdir/high-tier-low-ceiling.json"
jq -e 'any(.scenarios[]; .scenario_id == "UPDATE-004" and .status == "INSUFFICIENT_CLAIM_CEILING")' \
  "$tmpdir/low-ceiling-report.json" >/dev/null
echo "PASS: execution tier cannot override a lower claim ceiling"

if bash "$evaluator" --expected-source-sha "$expected" --evidence-root "$evidence_root" \
  --output "$tmpdir/required-report.json" --require-qualified "$@" >"$tmpdir/require-qualified.out" 2>&1; then
  echo "FAIL: --require-qualified accepted an incomplete profile" >&2
  exit 1
fi
jq -e '.overall_status == "NOT_QUALIFIED"' "$tmpdir/required-report.json" >/dev/null
echo "PASS: --require-qualified rejects incomplete SRQ coverage"

if bash "$evaluator" --expected-source-sha "$expected" --evidence-root "$evidence_root" \
  --output "$tmpdir/duplicate-report.json" "$@" "$1" >"$tmpdir/duplicate.out" 2>&1; then
  echo "FAIL: evaluator accepted duplicate receipts for one scenario/tier" >&2
  exit 1
fi
grep -Fq "duplicate result receipts" "$tmpdir/duplicate.out"
echo "PASS: duplicate scenario/tier receipts are rejected"

wrong_sha="ffffffffffffffffffffffffffffffffffffffff"
if [[ "$expected" == "$wrong_sha" ]]; then
  wrong_sha="eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
fi
if bash "$evaluator" --expected-source-sha "$wrong_sha" --evidence-root "$evidence_root" \
  --output "$tmpdir/wrong-head-report.json" "$@" >"$tmpdir/wrong-head.out" 2>&1; then
  echo "FAIL: evaluator accepted receipts bound to another release source SHA" >&2
  exit 1
fi
grep -Fq "expected source differs from release source" "$tmpdir/wrong-head.out"
echo "PASS: mismatched release source SHA is rejected"

mkdir -p "$tmpdir/empty-evidence"
if bash "$evaluator" --expected-source-sha "$expected" --evidence-root "$tmpdir/empty-evidence" \
  --output "$tmpdir/missing-artifact-report.json" "$@" >"$tmpdir/missing-artifact.out" 2>&1; then
  echo "FAIL: evaluator accepted missing evidence bytes" >&2
  exit 1
fi
grep -Fq "evidence artifact is missing" "$tmpdir/missing-artifact.out"
echo "PASS: missing evidence artifacts prevent qualification"

echo "PASS: release qualification compiler negative controls passed."
echo "NON-CLAIM: these checks establish fail-closed aggregation behavior, not production or hardware qualification."
