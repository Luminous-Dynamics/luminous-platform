#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
wrapper="$script_dir/evaluate-attested-release.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin" "$tmp/work/srq-evidence-bundle"
bundle="$tmp/work/srq-evidence-bundle"
archive="$tmp/srq-evidence-bundle.tar.gz"
out="$tmp/output"
source_sha="0123456789abcdef0123456789abcdef01234567"
workflow_ref="Luminous-Dynamics/luminous-platform/.github/workflows/srq-v1.yml@refs/heads/main"
workflow_sha="2222222222222222222222222222222222222222"
run_id="12345"
run_attempt="1"

fail() { echo "FAIL: $*" >&2; exit 1; }

# This end-to-end contract test deliberately uses synthetic receipts and a CLI
# test double. It tests wrapper composition, not cryptographic attestation.
printf 'synthetic recovery-model evidence; contract test only\n' > "$bundle/srq-recovery-model-test.log"
printf 'synthetic process-kill observation; model-tier only\n' > "$bundle/srq-process-kill-model-test.log"
printf 'synthetic COSE verifier test; deterministic fixture only\n' > "$bundle/srq-eat-cose-verifier-test.log"
printf 'synthetic SQLite promotion reference; model-tier only\n' > "$bundle/srq-sqlite-promotion-reference.log"
printf 'synthetic boot-reconciliation observation; model-tier only\n' > "$bundle/srq-boot-reconciliation-model-test.log"
printf 'synthetic D0 adapter self-test; model-tier only\n' > "$bundle/srq-boot-observation-adapter-test.log"
printf 'synthetic challenge-model evidence; contract test only\n' > "$bundle/srq-challenge-promotion-model-test.log"

recovery_sha="$(sha256sum "$bundle/srq-recovery-model-test.log" | cut -d ' ' -f 1)"
challenge_sha="$(sha256sum "$bundle/srq-challenge-promotion-model-test.log" | cut -d ' ' -f 1)"
process_kill_sha="$(sha256sum "$bundle/srq-process-kill-model-test.log" | cut -d ' ' -f 1)"
sqlite_reference_sha="$(sha256sum "$bundle/srq-sqlite-promotion-reference.log" | cut -d ' ' -f 1)"
boot_reconciliation_sha="$(sha256sum "$bundle/srq-boot-reconciliation-model-test.log" | cut -d ' ' -f 1)"
boot_observation_adapter_sha="$(sha256sum "$bundle/srq-boot-observation-adapter-test.log" | cut -d ' ' -f 1)"
d1_cose_verifier_sha="$(sha256sum "$bundle/srq-eat-cose-verifier-test.log" | cut -d ' ' -f 1)"

make_receipt() {
  local scenario="$1" log="$2" log_sha="$3" destination="$4" command="$5"
  jq -n \
    --arg scenario "$scenario" \
    --arg source_sha "$source_sha" \
    --arg log "$log" \
    --arg log_sha "$log_sha" \
    --arg command "$command" \
    '{
      schema_version: 1,
      profile_id: "SRQ-1",
      scenario_id: $scenario,
      status: "PASS",
      reason: "Synthetic receipt used only to test the wrapper composition; no qualification claim.",
      source: {
        expected_source_sha: $source_sha,
        event_sha: $source_sha,
        actual_checkout_sha: $source_sha,
        source_matches_expected: true
      },
      execution: {
        tier: "model",
        steps_executed: true,
        command: $command,
        exit_code: 0,
        environment: {
          os: "ubuntu-contract-test",
          architecture: "x86_64",
          toolchain: "synthetic-test-double"
        }
      },
      evidence: [{artifact: $log, sha256: $log_sha, kind: "log"}],
      claim_ceiling: "abstract_model_only",
      non_claims: [
        "Synthetic contract test data only.",
        "The fake CLI does not verify any cryptographic attestation.",
        "This receipt is not product qualification evidence."
      ],
      fixture: false
    }' > "$destination"
}

make_receipt "UPDATE-004" "srq-recovery-model-test.log" "$recovery_sha" \
  "$bundle/srq-update-004-model-result.json" "synthetic recovery test contract"
make_receipt "EVIDENCE-003" "srq-challenge-promotion-model-test.log" "$challenge_sha" \
  "$bundle/srq-evidence-003-model-result.json" "synthetic challenge test contract"

# Confirm these receipts satisfy the existing byte-linked receipt/evaluator
# contracts but can only count as model-tier observations.
bash "$script_dir/validate-result.sh" --evidence-root "$bundle" "$bundle/srq-update-004-model-result.json" >/dev/null
bash "$script_dir/validate-result.sh" --evidence-root "$bundle" "$bundle/srq-evidence-003-model-result.json" >/dev/null
bash "$script_dir/evaluate-release.sh" \
  --expected-source-sha "$source_sha" \
  --evidence-root "$bundle" \
  --output "$bundle/srq-release-qualification.json" \
  "$bundle/srq-update-004-model-result.json" "$bundle/srq-evidence-003-model-result.json" >/dev/null
jq -e '.overall_status == "NOT_QUALIFIED" and .summary.passed_at_qualifying_tier == 0' \
  "$bundle/srq-release-qualification.json" >/dev/null || fail "model-only evaluation unexpectedly qualified"

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
    srq-sqlite-promotion-reference.log \
    srq-update-004-model-result.json > SHA256SUMS
)
checksum_sha="$(sha256sum "$bundle/SHA256SUMS" | cut -d ' ' -f 1)"
corpus_sha="$(sha256sum "$script_dir/scenarios.json" | cut -d ' ' -f 1)"
jq -n \
  --arg repo "Luminous-Dynamics/luminous-platform" \
  --arg workflow_path ".github/workflows/srq-v1.yml" \
  --arg workflow_ref "$workflow_ref" \
  --arg workflow_sha "$workflow_sha" \
  --arg event_name "workflow_dispatch" \
  --arg run_id "$run_id" \
  --arg run_attempt "$run_attempt" \
  --arg source_sha "$source_sha" \
  --arg corpus_sha "$corpus_sha" \
  --arg checksum_sha "$checksum_sha" \
  --arg process_kill_sha "$process_kill_sha" \
  --arg sqlite_reference_sha "$sqlite_reference_sha" \
  --arg boot_reconciliation_sha "$boot_reconciliation_sha" \
  --arg boot_observation_adapter_sha "$boot_observation_adapter_sha" \
  --arg d1_cose_verifier_sha "$d1_cose_verifier_sha" \
  '{
    schema_version: 1,
    profile_id: "SRQ-1",
    repository: $repo,
    workflow_path: $workflow_path,
    workflow_ref: $workflow_ref,
    workflow_sha: $workflow_sha,
    event_name: $event_name,
    run_id: $run_id,
    run_attempt: $run_attempt,
    source: {
      expected_source_sha: $source_sha,
      event_sha: $source_sha,
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
    }]
  }' > "$bundle/manifest.json"

tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner \
  -cf - -C "$tmp/work" srq-evidence-bundle | gzip -n > "$archive"

cat > "$tmp/bin/gh" <<'FAKE_GH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--version" ]]; then
  printf 'gh version 2.102.0 (test double)\n'
  exit 0
fi
[[ "${1:-}" == "attestation" && "${2:-}" == "verify" ]] || exit 91
case "${FAKE_GH_MODE:-pass}" in
  pass) printf '[{"verificationResult":{"testDouble":true}}]\n' ;;
  reject) echo "synthetic signature rejection" >&2; exit 1 ;;
  *) printf '[]\n' ;;
esac
FAKE_GH
chmod +x "$tmp/bin/gh"

mkdir -p "$out"
PATH="$tmp/bin:$PATH" FAKE_GH_MODE=pass \
  bash "$wrapper" \
    --archive "$archive" \
    --expected-source-sha "$source_sha" \
    --expected-source-ref refs/heads/main \
    --expected-workflow-ref "$workflow_ref" \
    --expected-workflow-sha "$workflow_sha" \
    --expected-run-id "$run_id" \
    --expected-run-attempt "$run_attempt" \
    --output-envelope "$out/envelope.json" \
    --output-report "$out/evaluation.json" \
    --output-verification "$out/verification.json" >/dev/null

jq -e '
  .integrity_status == "ATTESTED_EVIDENCE_VERIFIED" and
  .evaluation.overall_status == "NOT_QUALIFIED" and
  .evaluation.total_release_gates == 22 and
  .evaluation.passed_at_qualifying_tier == 0
' "$out/envelope.json" >/dev/null || fail "attested-integrity result did not preserve non-qualified release status"
jq -e '.overall_status == "NOT_QUALIFIED" and .summary.passed_at_qualifying_tier == 0' \
  "$out/evaluation.json" >/dev/null || fail "fresh report was missing or incorrectly qualified"
jq -e 'type == "array" and length == 1' "$out/verification.json" >/dev/null \
  || fail "verifier JSON was not preserved"
echo "PASS: full wrapper recomputes qualification and keeps verified integrity separate from NOT_QUALIFIED"

if PATH="$tmp/bin:$PATH" FAKE_GH_MODE=reject \
  bash "$wrapper" \
    --archive "$archive" \
    --expected-source-sha "$source_sha" \
    --expected-source-ref refs/heads/main \
    --expected-workflow-ref "$workflow_ref" \
    --expected-workflow-sha "$workflow_sha" \
    --expected-run-id "$run_id" \
    --expected-run-attempt "$run_attempt" \
    --output-envelope "$out/rejected-envelope.json" \
    --output-report "$out/rejected-evaluation.json" \
    --output-verification "$out/rejected-verification.json" >/dev/null 2>&1; then
  fail "attestation-rejected bundle was accepted by full wrapper"
fi
[[ ! -f "$out/rejected-envelope.json" ]] || fail "wrapper emitted an integrity envelope after failed attestation"
echo "PASS: full wrapper stops before evaluation outputs when attestation verification fails"
