#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
validator="$script_dir/validate-produced-bundle.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

source_sha="0123456789abcdef0123456789abcdef01234567"
event_sha="fedcba9876543210fedcba9876543210fedcba98"
workflow_sha="2222222222222222222222222222222222222222"
source_ref="refs/pull/88/merge"
workflow_ref="Luminous-Dynamics/luminous-platform/.github/workflows/srq-v1.yml@refs/pull/88/merge"
event_name="pull_request"
run_id="54321"
run_attempt="1"

fail() { echo "FAIL: $*" >&2; exit 1; }

make_archive() {
  local archive="$1" bad_observation="${2:-false}" bad_checksum="${3:-false}" bad_d1="${4:-false}" bad_d1_sqlite="${5:-false}"
  local work="$tmp/work" bundle="$tmp/work/srq-evidence-bundle"
  rm -rf "$work"
  mkdir -p "$bundle"

  for name in \
    srq-boot-observation-adapter-test.log \
    srq-boot-reconciliation-model-test.log \
    srq-challenge-promotion-model-test.log \
    srq-eat-cose-verifier-test.log \
    srq-evidence-003-model-result.json \
    srq-process-kill-model-test.log \
    srq-recovery-model-test.log \
    srq-release-qualification.json \
    srq-sqlite-d1-challenge-consumption-reference.log \
    srq-sqlite-promotion-reference.log \
    srq-update-004-model-result.json; do
    printf 'synthetic artifact for %s\n' "$name" > "$bundle/$name"
  done

  jq -n '{
    overall_status: "NOT_QUALIFIED",
    summary: {total_release_gates: 22, passed_at_qualifying_tier: 0}
  }' > "$bundle/srq-release-qualification.json"

  (
    cd "$bundle"
    sha256sum \
      srq-boot-observation-adapter-test.log \
      srq-boot-reconciliation-model-test.log \
      srq-challenge-promotion-model-test.log \
      srq-eat-cose-verifier-test.log \
      srq-evidence-003-model-result.json \
      srq-process-kill-model-test.log \
      srq-recovery-model-test.log \
      srq-release-qualification.json \
      srq-sqlite-d1-challenge-consumption-reference.log \
      srq-sqlite-promotion-reference.log \
      srq-update-004-model-result.json > SHA256SUMS
  )

  if [[ "$bad_checksum" == "true" ]]; then
    printf 'tampered after hashing\n' >> "$bundle/srq-recovery-model-test.log"
  fi

  local checksum_sha process_kill_sha sqlite_reference_sha boot_reconciliation_sha boot_observation_adapter_sha d1_cose_verifier_sha d1_sqlite_consumer_sha manifest_d1_sqlite_sha
  checksum_sha="$(sha256sum "$bundle/SHA256SUMS" | cut -d ' ' -f 1)"
  process_kill_sha="$(sha256sum "$bundle/srq-process-kill-model-test.log" | cut -d ' ' -f 1)"
  sqlite_reference_sha="$(sha256sum "$bundle/srq-sqlite-promotion-reference.log" | cut -d ' ' -f 1)"
  boot_reconciliation_sha="$(sha256sum "$bundle/srq-boot-reconciliation-model-test.log" | cut -d ' ' -f 1)"
  boot_observation_adapter_sha="$(sha256sum "$bundle/srq-boot-observation-adapter-test.log" | cut -d ' ' -f 1)"
  d1_cose_verifier_sha="$(sha256sum "$bundle/srq-eat-cose-verifier-test.log" | cut -d ' ' -f 1)"
  d1_sqlite_consumer_sha="$(sha256sum "$bundle/srq-sqlite-d1-challenge-consumption-reference.log" | cut -d ' ' -f 1)"
  manifest_d1_sqlite_sha="$d1_sqlite_consumer_sha"
  if [[ "$bad_observation" == "true" ]]; then
    boot_observation_adapter_sha="0000000000000000000000000000000000000000000000000000000000000000"
  fi
  if [[ "$bad_d1" == "true" ]]; then
    d1_cose_verifier_sha="0000000000000000000000000000000000000000000000000000000000000000"
  fi
  if [[ "$bad_d1_sqlite" == "true" ]]; then
    manifest_d1_sqlite_sha="0000000000000000000000000000000000000000000000000000000000000000"
  fi

  jq -n \
    --arg repository "Luminous-Dynamics/luminous-platform" \
    --arg workflow_path ".github/workflows/srq-v1.yml" \
    --arg workflow_ref "$workflow_ref" \
    --arg workflow_sha "$workflow_sha" \
    --arg event_name "$event_name" \
    --arg run_id "$run_id" \
    --arg run_attempt "$run_attempt" \
    --arg source_sha "$source_sha" \
    --arg event_sha "$event_sha" \
    --arg corpus_sha "$(printf 'a%.0s' {1..64})" \
    --arg checksum_sha "$checksum_sha" \
    --arg process_kill_sha "$process_kill_sha" \
    --arg sqlite_reference_sha "$sqlite_reference_sha" \
    --arg boot_reconciliation_sha "$boot_reconciliation_sha" \
    --arg boot_observation_adapter_sha "$boot_observation_adapter_sha" \
    --arg d1_cose_verifier_sha "$d1_cose_verifier_sha" \
    --arg d1_sqlite_consumer_sha "$manifest_d1_sqlite_sha" '{
      schema_version: 1,
      profile_id: "SRQ-1",
      repository: $repository,
      workflow_path: $workflow_path,
      workflow_ref: $workflow_ref,
      workflow_sha: $workflow_sha,
      event_name: $event_name,
      run_id: $run_id,
      run_attempt: $run_attempt,
      source: {
        expected_source_sha: $source_sha,
        event_sha: $event_sha,
        actual_checkout_sha: $source_sha,
        source_matches_expected: true
      },
      corpus_sha256: $corpus_sha,
      checksum_manifest_sha256: $checksum_sha,
      supplemental_observations: [{
        test_id: "SRQ-PROCESS-KILL-V1",
        scenario_id: "UPDATE-004",
        execution_tier: "model",
        claim_ceiling: "abstract_model_only",
        status: "PASS",
        evidence_artifact: "srq-process-kill-model-test.log",
        evidence_sha256: $process_kill_sha
      }, {
        test_id: "SRQ-SQLITE-REFERENCE-V1",
        scenario_id: "EVIDENCE-003",
        execution_tier: "model",
        claim_ceiling: "abstract_model_only",
        status: "PASS",
        evidence_artifact: "srq-sqlite-promotion-reference.log",
        evidence_sha256: $sqlite_reference_sha
      }, {
        test_id: "SRQ-BOOT-RECONCILIATION-V1",
        scenario_id: "RECOVERY-002",
        execution_tier: "model",
        claim_ceiling: "abstract_model_only",
        status: "PASS",
        evidence_artifact: "srq-boot-reconciliation-model-test.log",
        evidence_sha256: $boot_reconciliation_sha
      }, {
        test_id: "SRQ-BOOT-OBSERVATION-ADAPTER-D0-V1",
        scenario_id: "RECOVERY-002",
        execution_tier: "model",
        claim_ceiling: "abstract_model_only",
        status: "PASS",
        evidence_artifact: "srq-boot-observation-adapter-test.log",
        evidence_sha256: $boot_observation_adapter_sha
      }, {
        test_id: "SRQ-D1-COSE-VERIFY-V1",
        scenario_id: "RECOVERY-002",
        execution_tier: "model",
        claim_ceiling: "cryptographic_test_fixture_only",
        status: "PASS",
        evidence_artifact: "srq-eat-cose-verifier-test.log",
        evidence_sha256: $d1_cose_verifier_sha
      }, {
        test_id: "SRQ-D1-SQLITE-CONSUMER-REFERENCE-V1",
        scenario_id: "RECOVERY-002",
        execution_tier: "model",
        claim_ceiling: "abstract_model_only",
        status: "PASS",
        evidence_artifact: "srq-sqlite-d1-challenge-consumption-reference.log",
        evidence_sha256: $d1_sqlite_consumer_sha
      }]
    }' > "$bundle/manifest.json"

  tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner \
    -cf - -C "$tmp/work" srq-evidence-bundle | gzip -n > "$archive"
}

run_validator() {
  local archive="$1" expected_source="$2" expected_event="$3" expected_workflow="$4" output="$5"
  bash "$validator" \
    --archive "$archive" \
    --expected-source-sha "$expected_source" \
    --expected-event-sha "$expected_event" \
    --expected-source-ref "$source_ref" \
    --expected-workflow-ref "$workflow_ref" \
    --expected-workflow-sha "$expected_workflow" \
    --expected-event-name "$event_name" \
    --expected-run-id "$run_id" \
    --expected-run-attempt "$run_attempt" > "$output"
}

make_archive "$tmp/good.tar.gz"
run_validator "$tmp/good.tar.gz" "$source_sha" "$event_sha" "$workflow_sha" "$tmp/good.out"
grep -Fq 'PASS: produced SRQ bundle validated' "$tmp/good.out"
echo "PASS: produced archive validates with matching source/event/workflow identities"

if run_validator "$tmp/good.tar.gz" "1111111111111111111111111111111111111111" "$event_sha" "$workflow_sha" "$tmp/bad-source.out" >/dev/null 2>&1; then
  fail "source SHA mismatch was accepted"
fi
echo "PASS: source SHA mismatch fails closed"

if run_validator "$tmp/good.tar.gz" "$source_sha" "1111111111111111111111111111111111111111" "$workflow_sha" "$tmp/bad-event.out" >/dev/null 2>&1; then
  fail "event SHA mismatch was accepted"
fi
echo "PASS: event SHA mismatch fails closed"

if run_validator "$tmp/good.tar.gz" "$source_sha" "$event_sha" "3333333333333333333333333333333333333333" "$tmp/bad-workflow.out" >/dev/null 2>&1; then
  fail "workflow SHA mismatch was accepted"
fi
echo "PASS: workflow SHA mismatch fails closed"

make_archive "$tmp/bad-observation.tar.gz" true
if run_validator "$tmp/bad-observation.tar.gz" "$source_sha" "$event_sha" "$workflow_sha" "$tmp/bad-observation.out" >/dev/null 2>&1; then
  fail "supplemental observation digest mismatch was accepted"
fi
echo "PASS: D0 observer digest mismatch fails closed"

make_archive "$tmp/bad-d1.tar.gz" false false true
if run_validator "$tmp/bad-d1.tar.gz" "$source_sha" "$event_sha" "$workflow_sha" "$tmp/bad-d1.out" >/dev/null 2>&1; then
  fail "D1 verifier test digest mismatch was accepted"
fi
echo "PASS: D1 verifier fixture digest mismatch fails closed"

make_archive "$tmp/bad-d1-sqlite.tar.gz" false false false true
if run_validator "$tmp/bad-d1-sqlite.tar.gz" "$source_sha" "$event_sha" "$workflow_sha" "$tmp/bad-d1-sqlite.out" >/dev/null 2>&1; then
  fail "D1 SQLite reference digest mismatch was accepted"
fi
echo "PASS: D1 SQLite consumption digest mismatch fails closed"

make_archive "$tmp/bad-checksum.tar.gz" false true
if run_validator "$tmp/bad-checksum.tar.gz" "$source_sha" "$event_sha" "$workflow_sha" "$tmp/bad-checksum.out" >/dev/null 2>&1; then
  fail "tampered evidence bytes were accepted"
fi
echo "PASS: tampered evidence bytes fail closed"
