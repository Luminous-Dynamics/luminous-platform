#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
verifier="$script_dir/verify-evidence-bundle.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
source_sha="0123456789abcdef0123456789abcdef01234567"
workflow_ref="Luminous-Dynamics/luminous-platform/.github/workflows/srq-v1.yml@refs/heads/main"
run_id="12345"
run_attempt="1"
fail() { echo "FAIL: $*" >&2; exit 1; }

make_archive() {
  local archive="$1" bad_checksum="${2:-false}" bad_supplemental="${3:-none}"
  local work="$tmp/work" bundle="$tmp/work/srq-evidence-bundle"
  rm -rf "$work"
  mkdir -p "$bundle"
  for name in \
    srq-challenge-promotion-model-test.log \
    srq-evidence-003-model-result.json \
    srq-process-kill-model-test.log \
    srq-recovery-model-test.log \
    srq-release-qualification.json \
    srq-sqlite-promotion-reference.log \
    srq-update-004-model-result.json; do
    printf 'synthetic evidence for %s\n' "$name" > "$bundle/$name"
  done
  (
    cd "$bundle"
    sha256sum \
      srq-challenge-promotion-model-test.log \
      srq-evidence-003-model-result.json \
      srq-process-kill-model-test.log \
      srq-recovery-model-test.log \
      srq-release-qualification.json \
      srq-sqlite-promotion-reference.log \
      srq-update-004-model-result.json > SHA256SUMS
  )
  if [[ "$bad_checksum" == "true" ]]; then
    printf 'tampered after hashing\n' >> "$bundle/srq-recovery-model-test.log"
  fi
  local checksum_sha process_kill_sha sqlite_reference_sha manifest_process_kill_sha manifest_sqlite_sha
  checksum_sha="$(sha256sum "$bundle/SHA256SUMS" | cut -d ' ' -f 1)"
  process_kill_sha="$(sha256sum "$bundle/srq-process-kill-model-test.log" | cut -d ' ' -f 1)"
  sqlite_reference_sha="$(sha256sum "$bundle/srq-sqlite-promotion-reference.log" | cut -d ' ' -f 1)"
  manifest_process_kill_sha="$process_kill_sha"
  manifest_sqlite_sha="$sqlite_reference_sha"
  if [[ "$bad_supplemental" == "process-kill" ]]; then
    manifest_process_kill_sha="0000000000000000000000000000000000000000000000000000000000000000"
  elif [[ "$bad_supplemental" == "sqlite" ]]; then
    manifest_sqlite_sha="0000000000000000000000000000000000000000000000000000000000000000"
  fi
  jq -n \
    --arg repo "Luminous-Dynamics/luminous-platform" \
    --arg workflow_path ".github/workflows/srq-v1.yml" \
    --arg workflow_ref "$workflow_ref" \
    --arg workflow_sha "2222222222222222222222222222222222222222" \
    --arg event_name "workflow_dispatch" \
    --arg run_id "$run_id" \
    --arg run_attempt "$run_attempt" \
    --arg source_sha "$source_sha" \
    --arg checksum_sha "$checksum_sha" \
    --arg corpus_sha "$(printf 'a%.0s' {1..64})" \
    --arg process_kill_sha "$manifest_process_kill_sha" \
    --arg sqlite_reference_sha "$manifest_sqlite_sha" \
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
      }]
    }' > "$bundle/manifest.json"
  tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner -cf - -C "$tmp/work" srq-evidence-bundle \
    | gzip -n > "$archive"
}

run_verifier() {
  local archive="$1" expected_sha="$2" expected_ref="$3" destination="$4"
  bash "$verifier" --archive "$archive" \
    --expected-source-sha "$expected_sha" \
    --expected-source-ref "$expected_ref" \
    --expected-workflow-ref "$workflow_ref" \
    --expected-run-id "$run_id" \
    --expected-run-attempt "$run_attempt" \
    --extract-to "$destination"
}

make_archive "$tmp/good.tar.gz"
run_verifier "$tmp/good.tar.gz" "$source_sha" refs/heads/main "$tmp/extracted-good" > "$tmp/manifest-path"
[[ -f "$(cat "$tmp/manifest-path")" ]] || fail "valid bundle manifest was not returned"
echo "PASS: bundle source, workflow, run, process-kill and SQLite inventories, claim ceilings and hashes verify"

if run_verifier "$tmp/good.tar.gz" "1111111111111111111111111111111111111111" refs/heads/main "$tmp/extracted-wrong-source" >/dev/null 2>&1; then
  fail "wrong expected source SHA was accepted"
fi
echo "PASS: source-SHA mismatch fails closed"

if run_verifier "$tmp/good.tar.gz" "$source_sha" refs/heads/research "$tmp/extracted-wrong-ref" >/dev/null 2>&1; then
  fail "non-main source ref was accepted"
fi
echo "PASS: untrusted source ref fails closed"

make_archive "$tmp/bad-checksum.tar.gz" true
if run_verifier "$tmp/bad-checksum.tar.gz" "$source_sha" refs/heads/main "$tmp/extracted-bad-checksum" >/dev/null 2>&1; then
  fail "tampered evidence bytes were accepted"
fi
echo "PASS: evidence mutation fails byte verification"

make_archive "$tmp/bad-process-kill.tar.gz" false process-kill
if run_verifier "$tmp/bad-process-kill.tar.gz" "$source_sha" refs/heads/main "$tmp/extracted-bad-process-kill" >/dev/null 2>&1; then
  fail "false process-kill evidence digest was accepted"
fi
echo "PASS: process-kill supplemental digest mismatch fails closed"

make_archive "$tmp/bad-sqlite.tar.gz" false sqlite
if run_verifier "$tmp/bad-sqlite.tar.gz" "$source_sha" refs/heads/main "$tmp/extracted-bad-sqlite" >/dev/null 2>&1; then
  fail "false SQLite evidence digest was accepted"
fi
echo "PASS: SQLite supplemental digest mismatch fails closed"
