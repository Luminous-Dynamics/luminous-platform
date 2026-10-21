#!/usr/bin/env bash
set -euo pipefail

readonly expected_repository="Luminous-Dynamics/luminous-platform"
readonly expected_workflow_path=".github/workflows/srq-v1.yml"

archive=""
source_sha=""
event_sha=""
source_ref=""
workflow_ref=""
workflow_sha=""
event_name=""
run_id=""
run_attempt=""

usage() {
  echo "Usage: $0 --archive PATH --expected-source-sha SHA40 --expected-event-sha SHA40 --expected-source-ref REF --expected-workflow-ref REF --expected-workflow-sha SHA40 --expected-event-name EVENT --expected-run-id ID --expected-run-attempt N" >&2
}

while (($#)); do
  case "$1" in
    --archive) [[ $# -ge 2 ]] || { usage; exit 2; }; archive="$2"; shift 2 ;;
    --expected-source-sha) [[ $# -ge 2 ]] || { usage; exit 2; }; source_sha="$2"; shift 2 ;;
    --expected-event-sha) [[ $# -ge 2 ]] || { usage; exit 2; }; event_sha="$2"; shift 2 ;;
    --expected-source-ref) [[ $# -ge 2 ]] || { usage; exit 2; }; source_ref="$2"; shift 2 ;;
    --expected-workflow-ref) [[ $# -ge 2 ]] || { usage; exit 2; }; workflow_ref="$2"; shift 2 ;;
    --expected-workflow-sha) [[ $# -ge 2 ]] || { usage; exit 2; }; workflow_sha="$2"; shift 2 ;;
    --expected-event-name) [[ $# -ge 2 ]] || { usage; exit 2; }; event_name="$2"; shift 2 ;;
    --expected-run-id) [[ $# -ge 2 ]] || { usage; exit 2; }; run_id="$2"; shift 2 ;;
    --expected-run-attempt) [[ $# -ge 2 ]] || { usage; exit 2; }; run_attempt="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

[[ -n "$archive" && -f "$archive" && ! -L "$archive" ]] || {
  echo "BLOCKED: archive is missing or is not a regular file." >&2
  exit 2
}
[[ "$source_sha" =~ ^[0-9a-f]{40}$ && "$event_sha" =~ ^[0-9a-f]{40}$ && "$workflow_sha" =~ ^[0-9a-f]{40}$ ]] || {
  echo "BLOCKED: source, event or workflow SHA is malformed." >&2
  exit 2
}
[[ -n "$source_ref" && -n "$workflow_ref" ]] || {
  echo "BLOCKED: expected source or workflow ref is empty." >&2
  exit 2
}
[[ "$workflow_ref" == "$expected_repository/$expected_workflow_path@$source_ref" ]] || {
  echo "BLOCKED: workflow ref does not match the caller-supplied source ref and workflow path." >&2
  exit 2
}
[[ "$event_name" =~ ^[a-z_]+$ ]] || {
  echo "BLOCKED: workflow event name is malformed." >&2
  exit 2
}
[[ "$run_id" =~ ^[0-9]+$ && "$run_attempt" =~ ^[0-9]+$ ]] || {
  echo "BLOCKED: run ID or attempt is malformed." >&2
  exit 2
}
command -v tar >/dev/null 2>&1 && command -v jq >/dev/null 2>&1 && command -v sha256sum >/dev/null 2>&1 || {
  echo "BLOCKED: tar, jq and sha256sum are required." >&2
  exit 2
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

entries="$(tar -tzf "$archive")" || {
  echo "BLOCKED: evidence archive is unreadable." >&2
  exit 1
}
expected_entries=$'srq-evidence-bundle/\nsrq-evidence-bundle/SHA256SUMS\nsrq-evidence-bundle/manifest.json\nsrq-evidence-bundle/srq-boot-observation-adapter-test.log\nsrq-evidence-bundle/srq-boot-reconciliation-model-test.log\nsrq-evidence-bundle/srq-challenge-promotion-model-test.log\nsrq-evidence-bundle/srq-evidence-003-model-result.json\nsrq-evidence-bundle/srq-process-kill-model-test.log\nsrq-evidence-bundle/srq-recovery-model-test.log\nsrq-evidence-bundle/srq-release-qualification.json\nsrq-evidence-bundle/srq-sqlite-promotion-reference.log\nsrq-evidence-bundle/srq-update-004-model-result.json'
actual_entries="$(printf '%s\n' "$entries" | LC_ALL=C sort)"
expected_entries_sorted="$(printf '%s\n' "$expected_entries" | LC_ALL=C sort)"
[[ "$actual_entries" == "$expected_entries_sorted" ]] || {
  echo "BLOCKED: produced archive inventory differs from the frozen SRQ bundle profile." >&2
  diff -u <(printf '%s\n' "$expected_entries_sorted") <(printf '%s\n' "$actual_entries") || true
  exit 1
}

mkdir "$tmp/extracted"
tar --no-same-owner --no-same-permissions -xzf "$archive" -C "$tmp/extracted"
bundle="$tmp/extracted/srq-evidence-bundle"
[[ -d "$bundle" && ! -L "$bundle" ]] || {
  echo "BLOCKED: extracted bundle directory is invalid." >&2
  exit 1
}
[[ -z "$(find "$bundle" -type l -print -quit)" ]] || {
  echo "BLOCKED: symbolic links are forbidden in the evidence bundle." >&2
  exit 1
}

actual_files="$(find "$bundle" -mindepth 1 -maxdepth 1 -type f -printf '%f\n' | LC_ALL=C sort)"
expected_files=$'SHA256SUMS\nmanifest.json\nsrq-boot-observation-adapter-test.log\nsrq-boot-reconciliation-model-test.log\nsrq-challenge-promotion-model-test.log\nsrq-evidence-003-model-result.json\nsrq-process-kill-model-test.log\nsrq-recovery-model-test.log\nsrq-release-qualification.json\nsrq-sqlite-promotion-reference.log\nsrq-update-004-model-result.json'
[[ "$actual_files" == "$expected_files" ]] || {
  echo "BLOCKED: extracted file inventory is incomplete or contains extra files." >&2
  exit 1
}

declared_files="$(awk '{print $2}' "$bundle/SHA256SUMS" | LC_ALL=C sort)"
expected_declared_files=$'srq-boot-observation-adapter-test.log\nsrq-boot-reconciliation-model-test.log\nsrq-challenge-promotion-model-test.log\nsrq-evidence-003-model-result.json\nsrq-process-kill-model-test.log\nsrq-recovery-model-test.log\nsrq-release-qualification.json\nsrq-sqlite-promotion-reference.log\nsrq-update-004-model-result.json'
[[ "$declared_files" == "$expected_declared_files" ]] || {
  echo "BLOCKED: checksum manifest inventory is incomplete or duplicated." >&2
  exit 1
}
while IFS= read -r line; do
  [[ "$line" =~ ^[0-9a-f]{64}\ \ [A-Za-z0-9._-]+$ ]] || {
    echo "BLOCKED: checksum manifest syntax is invalid." >&2
    exit 1
  }
done < "$bundle/SHA256SUMS"
(
  cd "$bundle"
  sha256sum --check --strict SHA256SUMS >/dev/null
) || {
  echo "BLOCKED: evidence bytes do not match their checksums." >&2
  exit 1
}

checksum_sha="$(sha256sum "$bundle/SHA256SUMS" | cut -d ' ' -f 1)"
process_kill_sha="$(sha256sum "$bundle/srq-process-kill-model-test.log" | cut -d ' ' -f 1)"
sqlite_reference_sha="$(sha256sum "$bundle/srq-sqlite-promotion-reference.log" | cut -d ' ' -f 1)"
boot_reconciliation_sha="$(sha256sum "$bundle/srq-boot-reconciliation-model-test.log" | cut -d ' ' -f 1)"
boot_observation_adapter_sha="$(sha256sum "$bundle/srq-boot-observation-adapter-test.log" | cut -d ' ' -f 1)"

jq -e \
  --arg repository "$expected_repository" \
  --arg workflow_path "$expected_workflow_path" \
  --arg source_ref "$source_ref" \
  --arg workflow_ref "$workflow_ref" \
  --arg workflow_sha "$workflow_sha" \
  --arg event_name "$event_name" \
  --arg run_id "$run_id" \
  --arg run_attempt "$run_attempt" \
  --arg source_sha "$source_sha" \
  --arg event_sha "$event_sha" \
  --arg checksum_sha "$checksum_sha" \
  --arg process_kill_sha "$process_kill_sha" \
  --arg sqlite_reference_sha "$sqlite_reference_sha" \
  --arg boot_reconciliation_sha "$boot_reconciliation_sha" \
  --arg boot_observation_adapter_sha "$boot_observation_adapter_sha" '
    .schema_version == 1 and
    .profile_id == "SRQ-1" and
    .repository == $repository and
    .workflow_path == $workflow_path and
    .workflow_ref == $workflow_ref and
    .workflow_sha == $workflow_sha and
    .event_name == $event_name and
    .run_id == $run_id and
    .run_attempt == $run_attempt and
    .source.expected_source_sha == $source_sha and
    .source.event_sha == $event_sha and
    .source.actual_checkout_sha == $source_sha and
    .source.source_matches_expected == true and
    ((.corpus_sha256 | type) == "string") and
    (.corpus_sha256 | test("^[0-9a-f]{64}$")) and
    .checksum_manifest_sha256 == $checksum_sha and
    .supplemental_observations == [{
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
    }]
  ' "$bundle/manifest.json" >/dev/null || {
  echo "BLOCKED: archive manifest does not match caller-supplied workflow identity or evidence digests." >&2
  exit 1
}

jq -e '
  .overall_status == "NOT_QUALIFIED" and
  .summary.total_release_gates == 22 and
  .summary.passed_at_qualifying_tier == 0
' "$bundle/srq-release-qualification.json" >/dev/null || {
  echo "BLOCKED: release report incorrectly raises or misstates SRQ qualification." >&2
  exit 1
}

printf 'PASS: produced SRQ bundle validated against supplied source, event and workflow identity; no signature or attestation is implied.\n'
