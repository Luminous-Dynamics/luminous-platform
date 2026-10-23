#!/usr/bin/env bash
set -euo pipefail

readonly script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
readonly validator="$script_dir/validate.sh"
nixward_root="${1:-}" mycelix_root="${2:-}" sovereign_boot_root="${3:-}" boot_observation="${4:-}" output_dir="${5:-}"
if [[ -z "$nixward_root" || -z "$mycelix_root" || -z "$sovereign_boot_root" || -z "$boot_observation" || -z "$output_dir" ]]; then
  echo "Usage: $0 NIXWARD_ROOT MYCELIX_ROOT SOVEREIGN_BOOT_ROOT BOOT_OBSERVATION_JSON OUTPUT_DIR" >&2
  exit 2
fi

readonly receipt="$mycelix_root/research/evidence-plane/examples/synthetic-planned-receipt-v0.json"
readonly repository_root="$(git -C "$script_dir/../.." rev-parse --show-toplevel)"
readonly harness_commit="$(git -C "$repository_root" rev-parse HEAD)"
mkdir -p "$output_dir"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
report="$output_dir/srq-lifecycle-adversarial-assessment.json"
cases=0
controls=0

jq -cn \
  --arg commit "$harness_commit" \
  --arg script_sha256 "$(sha256sum "$script_dir/adversarial.sh" | cut -d ' ' -f 1)" \
  --arg script_blob "$(git -C "$repository_root" hash-object -- benchmarks/srq-lifecycle-interop/adversarial.sh)" \
  --arg validator_sha256 "$(sha256sum "$validator" | cut -d ' ' -f 1)" \
  --arg validator_blob "$(git -C "$repository_root" hash-object -- benchmarks/srq-lifecycle-interop/validate.sh)" \
  --arg receipt_sha256 "$(sha256sum "$receipt" | cut -d ' ' -f 1)" \
  '{
    schema:"srq-lifecycle-adversarial-assessment-v1",
    status:"RUNNING",
    release_status:"BLOCKED",
    harness:{repository:"Luminous-Dynamics/luminous-platform",commit_sha:$commit,adversarial_script_git_blob_sha1:$script_blob,adversarial_script_sha256:$script_sha256,validator_git_blob_sha1:$validator_blob,validator_sha256:$validator_sha256},
    fixture:{repository:"Luminous-Dynamics/mycelix",commit_sha:"b02cefab8474262aef23b61a4ac3811fe92e29bf",git_blob_sha1:"d1d3516e2778e0da89013348b16ed73af6e5e8a8",raw_sha256:$receipt_sha256},
    expected_case_count:15,
    rejected_case_count:0,
    all_expected_mutations_rejected:false,
    expected_positive_control_count:3,
    accepted_positive_control_count:0,
    all_positive_controls_accepted:false,
    positive_controls:[],
    cases:[]
  }' > "$report"

expect_reject() {
  local name="$1" filter="$2" expected_reason="$3" candidate="$tmp/$1.json"
  local status=0 reason="" candidate_sha256=""
  jq "$filter" "$receipt" > "$candidate"
  if bash "$validator" \
    --nixward-root "$nixward_root" \
    --mycelix-root "$mycelix_root" \
    --sovereign-boot-root "$sovereign_boot_root" \
    --boot-observation "$boot_observation" \
    --evidence-plane-receipt "$candidate" \
    --output /dev/null >"$tmp/$name.log" 2>&1; then
    echo "FAIL: adversarial case was accepted: $name" >&2
    cat "$tmp/$name.log" >&2
    exit 1
  else
    status=$?
  fi

  reason="$(grep -F "CONTRACT VIOLATION:" "$tmp/$name.log" | tail -n 1 || true)"
  if [[ -z "$reason" || "$reason" != *"$expected_reason"* ]]; then
    echo "FAIL: $name failed for an unexpected reason (exit $status)." >&2
    cat "$tmp/$name.log" >&2
    exit 1
  fi
  candidate_sha256="$(sha256sum "$candidate" | cut -d ' ' -f 1)"
  jq --arg id "$name" --arg filter "$filter" --arg reason "$reason" \
    --arg candidate_sha256 "$candidate_sha256" --argjson exit_status "$status" \
    '.cases += [{
      id:$id,
      input_kind:"mycelix_receipt",
      mutation_filter:$filter,
      expected_outcome:"reject",
      observed_outcome:"REJECTED_AS_EXPECTED",
      validator_exit_status:$exit_status,
      rejection_reason:$reason,
      candidate_raw_sha256:$candidate_sha256
    }]' "$report" > "$tmp/report.json"
  mv "$tmp/report.json" "$report"
  cases=$((cases + 1))
  echo "PASS: rejected inconsistent evidence mutation: $name"
}

expect_transaction_reject() {
  local name="$1" filter="$2" expected_reason="$3"
  local candidate="$tmp/$name.json" status=0 reason="" candidate_sha256=""
  jq "$filter" "$nixward_root/docs/fixtures/system-transaction-v1.json" > "$candidate"
  if bash "$validator" \
    --nixward-root "$nixward_root" \
    --mycelix-root "$mycelix_root" \
    --sovereign-boot-root "$sovereign_boot_root" \
    --boot-observation "$boot_observation" \
    --transaction-fixture "$candidate" \
    --output /dev/null >"$tmp/$name.log" 2>&1; then
    echo "FAIL: transaction phase mutation was accepted: $name" >&2
    cat "$tmp/$name.log" >&2
    exit 1
  else
    status=$?
  fi

  reason="$(grep -F "CONTRACT VIOLATION:" "$tmp/$name.log" | tail -n 1 || true)"
  if [[ -z "$reason" || "$reason" != *"$expected_reason"* ]]; then
    echo "FAIL: $name failed for an unexpected reason (exit $status)." >&2
    cat "$tmp/$name.log" >&2
    exit 1
  fi
  candidate_sha256="$(sha256sum "$candidate" | cut -d ' ' -f 1)"
  jq --arg id "$name" --arg filter "$filter" --arg reason "$reason" \
    --arg candidate_sha256 "$candidate_sha256" --argjson exit_status "$status" \
    '.cases += [{
      id:$id,
      input_kind:"nixward_transaction",
      mutation_filter:$filter,
      expected_outcome:"reject",
      observed_outcome:"REJECTED_AS_EXPECTED",
      validator_exit_status:$exit_status,
      rejection_reason:$reason,
      candidate_raw_sha256:$candidate_sha256
    }]' "$report" > "$tmp/report.json"
  mv "$tmp/report.json" "$report"
  cases=$((cases + 1))
  echo "PASS: rejected impossible Nixward phase/evidence combination: $name"
}

expect_accept() {
  local name="$1" filter="$2" expected_tier="$3" expected_claim="$4"
  local candidate="$tmp/$name.json" assessment="$tmp/$name-assessment.json"
  local candidate_sha256="" actual=""

  jq "$filter" "$receipt" > "$candidate"
  if ! bash "$validator" \
    --nixward-root "$nixward_root" \
    --mycelix-root "$mycelix_root" \
    --sovereign-boot-root "$sovereign_boot_root" \
    --boot-observation "$boot_observation" \
    --evidence-plane-receipt "$candidate" \
    --output "$assessment" >"$tmp/$name.log" 2>&1; then
    echo "FAIL: valid structural positive control was rejected: $name" >&2
    cat "$tmp/$name.log" >&2
    exit 1
  fi

  if ! jq -e --arg tier "$expected_tier" --arg claim "$expected_claim" '
      .schema == "srq-lifecycle-interop-assessment-v1"
      and .contract_status == "PASS"
      and .release_status == "BLOCKED"
      and .claim_ceiling == "none"
      and .evidence_plane.evidence_tier == $tier
      and .evidence_plane.claim_ceiling == $claim
      and (.release_blockers | length > 0)
    ' "$assessment" >/dev/null; then
    echo "FAIL: positive control changed the expected tier or unblocked the release: $name" >&2
    cat "$assessment" >&2
    exit 1
  fi

  candidate_sha256="$(sha256sum "$candidate" | cut -d ' ' -f 1)"
  jq --arg id "$name" --arg filter "$filter" --arg tier "$expected_tier" \
    --arg claim "$expected_claim" --arg sha256 "$candidate_sha256" \
    '.positive_controls += [{
      id:$id,
      mutation_filter:$filter,
      fixture_semantics:"synthetic structural positive control; digests and source identities are sentinels, not real-world evidence",
      expected_outcome:"accept_record_but_keep_release_blocked",
      observed_outcome:"ACCEPTED_WITH_RELEASE_BLOCKED",
      release_status:"BLOCKED",
      claim_ceiling:"none",
      evidence_tier:$tier,
      evidence_claim_ceiling:$claim,
      candidate_raw_sha256:$sha256
    }]' "$report" > "$tmp/report.json"
  mv "$tmp/report.json" "$report"
  controls=$((controls + 1))
  echo "PASS: accepted structurally consistent $expected_tier control while keeping lifecycle blocked: $name"
}

expect_transaction_accept() {
  local name="$1" filter="$2"
  local candidate="$tmp/$name.json" assessment="$tmp/$name-assessment.json" candidate_sha256=""
  jq "$filter" "$nixward_root/docs/fixtures/system-transaction-v1.json" > "$candidate"
  if ! bash "$validator" \
    --nixward-root "$nixward_root" \
    --mycelix-root "$mycelix_root" \
    --sovereign-boot-root "$sovereign_boot_root" \
    --boot-observation "$boot_observation" \
    --transaction-fixture "$candidate" \
    --output "$assessment" >"$tmp/$name.log" 2>&1; then
    echo "FAIL: valid transaction-phase control was rejected: $name" >&2
    cat "$tmp/$name.log" >&2
    exit 1
  fi
  if ! jq -e '
      .schema == "srq-lifecycle-interop-assessment-v1"
      and .contract_status == "PASS"
      and .release_status == "BLOCKED"
      and .claim_ceiling == "none"
      and .transaction.phase == "validated"
      and (.release_blockers | length > 0)
    ' "$assessment" >/dev/null; then
    echo "FAIL: transaction positive control changed phase semantics or unblocked the release: $name" >&2
    cat "$assessment" >&2
    exit 1
  fi
  candidate_sha256="$(sha256sum "$candidate" | cut -d ' ' -f 1)"
  jq --arg id "$name" --arg filter "$filter" --arg sha256 "$candidate_sha256" \
    '.positive_controls += [{
      id:$id,
      input_kind:"nixward_transaction",
      mutation_filter:$filter,
      fixture_semantics:"synthetic valid-phase structural control; validation record is a sentinel",
      expected_outcome:"accept_record_but_keep_release_blocked",
      observed_outcome:"ACCEPTED_WITH_RELEASE_BLOCKED",
      release_status:"BLOCKED",
      claim_ceiling:"none",
      transaction_phase:"validated",
      candidate_raw_sha256:$sha256
    }]' "$report" > "$tmp/report.json"
  mv "$tmp/report.json" "$report"
  controls=$((controls + 1))
  echo "PASS: accepted validated-phase record while keeping lifecycle blocked: $name"
}

# Negative controls: mutations must fail on the intended contract invariant,
# not on a missing tool, broken checkout pin or malformed input.
expect_reject "s0-domain-validated-status" '.run_status = "domain_validated"' "cross-field semantics"
expect_reject "s0-independently-checked-status" '.run_status = "independently_checked"' "cross-field semantics"
expect_reject "planned-gate-inflation" '.gates.source_integrity = "pass"' "cross-field semantics"
expect_reject "planned-subject-commit-injection" '.subject.commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"' "cross-field semantics"
expect_reject "planned-artifact-hash-injection" '.subject.artifact_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"' "cross-field semantics"
expect_reject "planned-result-count-injection" '.benchmark.results_summary.observed_result_count = 1' "declared evidence tier / claim ceiling"
expect_reject "s3-without-independent-verification" '.evidence_tier = "S3" | .claim_ceiling = "independently_reproduced"' "cross-field semantics"
expect_reject "s2-source-reproducible-without-source" '.evidence_tier = "S2" | .claim_ceiling = "source_reproducible"' "cross-field semantics"
expect_reject "unknown-tier" '.evidence_tier = "S5"' "declared evidence tier / claim ceiling"

# The phase label must be supported by all prerequisite records in the
# transition chain. These candidates mutate only the phase on a planned record.
tx_reason="Nixward transaction fields or phase invariants are inconsistent"
expect_transaction_reject "validated-without-validation-record" '.phase = "validated"' "$tx_reason"
expect_transaction_reject "authorized-without-prior-records" '.phase = "authorized"' "$tx_reason"
expect_transaction_reject "snapshotted-without-prior-records" '.phase = "snapshotted"' "$tx_reason"
expect_transaction_reject "applied-without-prior-records" '.phase = "applied"' "$tx_reason"
expect_transaction_reject "verified-without-prior-records" '.phase = "verified"' "$tx_reason"
expect_transaction_reject "promoted-without-any-evidence-records" '.phase = "promoted"' "$tx_reason"

# Positive controls use obviously synthetic sentinels and explicit limitations.
# They exercise structural acceptance only; no test data is an actual attestation.
s2_filter='.run_status = "executed"
  | .evidence_tier = "S2"
  | .claim_ceiling = "source_reproducible"
  | .receipt_id = "synthetic:positive-control:S2"
  | .subject.repository = "synthetic://lifecycle-positive-control/S2"
  | .subject.artifact_path = "fixtures/synthetic-positive-control-s2.json"
  | .subject.commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  | .subject.artifact_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
  | .sources = [{
      source_id:"synthetic-source-1",
      url:"https://example.org/srq-test-fixture.bin",
      observed_at:"2026-10-23T00:00:00Z",
      media_type:"application/octet-stream",
      raw_bytes_ingested:true,
      byte_count:42,
      sha256:"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      rights_status:"open_license",
      contact_status:"not_required",
      temporal_coverage:null,
      spatial_coverage:null,
      metadata_conflicts:[],
      artifact_path:"fixtures/synthetic-source.bin"
    }]
  | .benchmark.protocol_sha256 = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
  | .benchmark.pre_registered_at = "2026-10-23T00:00:00Z"
  | .benchmark.split_strategy = "temporal_holdout"
  | .benchmark.split_lock_sha256 = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
  | .benchmark.results_summary.state = "available"
  | .benchmark.results_summary.observed_result_count = 1
  | .benchmark.results_summary.note = "Synthetic shape control only; no empirical result is asserted."
  | .benchmark.primary_metrics = ["structural positive control only; no empirical measurement"]
  | .gates.source_integrity = "pass"
  | .gates.rights_stewardship = "pass"
  | .gates.transform_reproducibility = "pass"
  | .gates.split_lock = "pass"
  | .gates.budget_parity = "pass"
  | .gates.negative_controls = "pass"
  | .gates.exact_head = "pass"
  | .gates.safety_critical_regression = "pass"
  | .gates.independent_verification = "unknown"
  | .gates.domain_validation = "unknown"
  | .limitations = ["Synthetic structural positive control only; every source URL and digest is a sentinel.", "Does not prove real source ingestion, rights, reproducibility or domain validity."]'
expect_accept "valid-s2-source-reproducible-shape" "$s2_filter" "S2" "source_reproducible"

s4_filter='.run_status = "domain_validated"
  | .evidence_tier = "S4"
  | .claim_ceiling = "domain_validated"
  | .receipt_id = "synthetic:positive-control:S4"
  | .subject.repository = "synthetic://lifecycle-positive-control/S4"
  | .subject.artifact_path = "fixtures/synthetic-positive-control-s4.json"
  | .subject.commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  | .subject.artifact_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
  | .benchmark.protocol_sha256 = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
  | .benchmark.pre_registered_at = "2026-10-23T00:00:00Z"
  | .benchmark.split_lock_sha256 = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
  | .benchmark.results_summary.state = "available"
  | .benchmark.results_summary.observed_result_count = 1
  | .benchmark.results_summary.note = "Synthetic shape control only; no empirical result is asserted."
  | .benchmark.primary_metrics = ["structural positive control only; no empirical measurement"]
  | .gates = (.gates | with_entries(.value = "pass"))
  | .independent_verification.status = "passed"
  | .independent_verification.verifier_kind = "independent_implementation"
  | .independent_verification.verifier_ref = "synthetic-positive-control-only"
  | .independent_verification.replayed_commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  | .independent_verification.note = "Synthetic shape test; not a real independent verification."
  | .limitations = ["Synthetic structural positive control only; identities, hashes and verification references are sentinels.", "Does not establish any real execution, independent reproduction or domain validation."]'
expect_accept "valid-s4-domain-validated-shape" "$s4_filter" "S4" "domain_validated"
expect_transaction_accept "valid-validated-phase-with-validation-record" '.phase = "validated" | .validation = {"status":"pass","validator_ref":"synthetic-control-only"}'

jq --argjson negative_count "$cases" --argjson positive_count "$controls" '
  .status = "PASS"
  | .rejected_case_count = ([.cases[] | select(.observed_outcome == "REJECTED_AS_EXPECTED")] | length)
  | .all_expected_mutations_rejected = (.cases | length == $negative_count and all(.[]; .observed_outcome == "REJECTED_AS_EXPECTED"))
  | .accepted_positive_control_count = ([.positive_controls[] | select(.observed_outcome == "ACCEPTED_WITH_RELEASE_BLOCKED")] | length)
  | .all_positive_controls_accepted = (.positive_controls | length == $positive_count and all(.[]; .observed_outcome == "ACCEPTED_WITH_RELEASE_BLOCKED"))
' "$report" > "$tmp/final-report.json"
mv "$tmp/final-report.json" "$report"
[[ "$cases" -eq 15 ]] || { echo "FAIL: expected exactly 15 adversarial cases, found $cases" >&2; exit 1; }
[[ "$controls" -eq 3 ]] || { echo "FAIL: expected exactly 3 positive controls, found $controls" >&2; exit 1; }
jq -e '
  .schema == "srq-lifecycle-adversarial-assessment-v1"
  and .status == "PASS" and .release_status == "BLOCKED"
  and .expected_case_count == 15 and .rejected_case_count == 15 and .all_expected_mutations_rejected == true
  and .expected_positive_control_count == 3 and .accepted_positive_control_count == 3
  and .all_positive_controls_accepted == true
  and (.positive_controls | all(.[]; .observed_outcome == "ACCEPTED_WITH_RELEASE_BLOCKED"))
' "$report" >/dev/null
printf 'ADVERSARIAL_CORPUS_PASS: %s invalid mutations rejected; %s valid-shape controls accepted; lifecycle remains BLOCKED. Report: %s\n' "$cases" "$controls" "$report"
