#!/usr/bin/env bash
set -euo pipefail
readonly trusted_repo="Luminous-Dynamics/luminous-platform"
readonly trusted_workflow_path=".github/workflows/srq-v1.yml"
readonly trusted_source_ref="refs/heads/main"
archive="" expected_source_sha="" expected_source_ref="" expected_workflow_ref="" expected_workflow_sha=""
expected_run_id="" expected_run_attempt="" extract_to=""
usage() { echo "Usage: $0 --archive PATH --expected-source-sha SHA40 --expected-source-ref refs/heads/main --expected-workflow-ref REF --expected-workflow-sha SHA40 --expected-run-id ID --expected-run-attempt N --extract-to EMPTY_DIR" >&2; }
while (($#)); do
  case "$1" in
    --archive) [[ $# -ge 2 ]] || { usage; exit 2; }; archive="$2"; shift 2 ;;
    --expected-source-sha) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_source_sha="$2"; shift 2 ;;
    --expected-source-ref) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_source_ref="$2"; shift 2 ;;
    --expected-workflow-ref) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_workflow_ref="$2"; shift 2 ;;
    --expected-workflow-sha) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_workflow_sha="$2"; shift 2 ;;
    --expected-run-id) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_run_id="$2"; shift 2 ;;
    --expected-run-attempt) [[ $# -ge 2 ]] || { usage; exit 2; }; expected_run_attempt="$2"; shift 2 ;;
    --extract-to) [[ $# -ge 2 ]] || { usage; exit 2; }; extract_to="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done
[[ -n "$archive" && -f "$archive" && ! -L "$archive" ]] || { echo "BLOCKED: archive is missing or is not a regular file." >&2; exit 2; }
[[ "$expected_source_sha" =~ ^[0-9a-f]{40}$ ]] || { echo "BLOCKED: source SHA is malformed." >&2; exit 2; }
[[ "$expected_workflow_sha" =~ ^[0-9a-f]{40}$ ]] || { echo "BLOCKED: workflow SHA is malformed." >&2; exit 2; }
[[ "$expected_source_ref" == "$trusted_source_ref" ]] || { echo "BLOCKED: only refs/heads/main is trusted." >&2; exit 2; }
[[ "$expected_workflow_ref" == "${trusted_repo}/${trusted_workflow_path}@${trusted_source_ref}" ]] || { echo "BLOCKED: workflow ref does not match fixed policy." >&2; exit 2; }
[[ "$expected_run_id" =~ ^[0-9]+$ && "$expected_run_attempt" =~ ^[0-9]+$ ]] || { echo "BLOCKED: run ID/attempt must be decimal integers." >&2; exit 2; }
[[ -n "$extract_to" ]] || { echo "BLOCKED: --extract-to is required." >&2; exit 2; }
mkdir -p "$extract_to"
if [[ -n "$(find "$extract_to" -mindepth 1 -print -quit)" ]]; then echo "BLOCKED: extraction destination must be empty." >&2; exit 2; fi
command -v tar >/dev/null 2>&1 && command -v jq >/dev/null 2>&1 && command -v sha256sum >/dev/null 2>&1 || { echo "BLOCKED: tar, jq and sha256sum are required." >&2; exit 2; }

entries="$(tar -tzf "$archive")" || { echo "BLOCKED: evidence archive is unreadable." >&2; exit 1; }
expected_entries=$'srq-evidence-bundle/\nsrq-evidence-bundle/SHA256SUMS\nsrq-evidence-bundle/manifest.json\nsrq-evidence-bundle/srq-challenge-promotion-model-test.log\nsrq-evidence-bundle/srq-evidence-003-model-result.json\nsrq-evidence-bundle/srq-process-kill-model-test.log\nsrq-evidence-bundle/srq-recovery-model-test.log\nsrq-evidence-bundle/srq-release-qualification.json\nsrq-evidence-bundle/srq-sqlite-promotion-reference.log\nsrq-evidence-bundle/srq-update-004-model-result.json'
actual_entries="$(printf '%s\n' "$entries" | LC_ALL=C sort)"
expected_entries_sorted="$(printf '%s\n' "$expected_entries" | LC_ALL=C sort)"
[[ "$actual_entries" == "$expected_entries_sorted" ]] || { echo "BLOCKED: archive contents do not match the fixed evidence inventory." >&2; exit 1; }

tar --no-same-owner --no-same-permissions -xzf "$archive" -C "$extract_to"
bundle_dir="$extract_to/srq-evidence-bundle"
[[ -d "$bundle_dir" && ! -L "$bundle_dir" ]] || { echo "BLOCKED: evidence bundle directory is invalid." >&2; exit 1; }
link_found="$(find "$bundle_dir" -type l -print -quit)"
[[ -z "$link_found" ]] || { echo "BLOCKED: symbolic links are not permitted." >&2; exit 1; }
actual_files="$(find "$bundle_dir" -mindepth 1 -maxdepth 1 -type f -printf '%f\n' | LC_ALL=C sort)"
expected_files=$'SHA256SUMS\nmanifest.json\nsrq-challenge-promotion-model-test.log\nsrq-evidence-003-model-result.json\nsrq-process-kill-model-test.log\nsrq-recovery-model-test.log\nsrq-release-qualification.json\nsrq-sqlite-promotion-reference.log\nsrq-update-004-model-result.json'
[[ "$actual_files" == "$expected_files" ]] || { echo "BLOCKED: extracted file inventory is invalid." >&2; exit 1; }

declared_files="$(awk '{print $2}' "$bundle_dir/SHA256SUMS" | LC_ALL=C sort)"
expected_declared_files=$'srq-challenge-promotion-model-test.log\nsrq-evidence-003-model-result.json\nsrq-process-kill-model-test.log\nsrq-recovery-model-test.log\nsrq-release-qualification.json\nsrq-sqlite-promotion-reference.log\nsrq-update-004-model-result.json'
[[ "$declared_files" == "$expected_declared_files" ]] || { echo "BLOCKED: checksum manifest references unexpected paths." >&2; exit 1; }
while IFS= read -r line; do
  [[ "$line" =~ ^[0-9a-f]{64}\ \ [A-Za-z0-9._-]+$ ]] || { echo "BLOCKED: checksum manifest syntax is invalid." >&2; exit 1; }
done < "$bundle_dir/SHA256SUMS"
(cd "$bundle_dir" && sha256sum --check --strict SHA256SUMS >/dev/null) || { echo "BLOCKED: evidence bytes do not match their checksums." >&2; exit 1; }

manifest="$bundle_dir/manifest.json"
checksum_sha="$(sha256sum "$bundle_dir/SHA256SUMS" | cut -d ' ' -f 1)"
process_kill_sha="$(sha256sum "$bundle_dir/srq-process-kill-model-test.log" | cut -d ' ' -f 1)"
sqlite_reference_sha="$(sha256sum "$bundle_dir/srq-sqlite-promotion-reference.log" | cut -d ' ' -f 1)"
jq -e \
  --arg source_sha "$expected_source_sha" \
  --arg source_ref "$expected_source_ref" \
  --arg workflow_ref "$expected_workflow_ref" \
  --arg workflow_sha "$expected_workflow_sha" \
  --arg run_id "$expected_run_id" \
  --arg run_attempt "$expected_run_attempt" \
  --arg repo "$trusted_repo" \
  --arg workflow_path "$trusted_workflow_path" \
  --arg checksum_sha "$checksum_sha" \
  --arg process_kill_sha "$process_kill_sha" \
  --arg sqlite_reference_sha "$sqlite_reference_sha" '
    .schema_version == 1 and
    .profile_id == "SRQ-1" and
    .repository == $repo and
    .workflow_path == $workflow_path and
    .workflow_ref == $workflow_ref and
    .workflow_sha == $workflow_sha and
    .event_name == "workflow_dispatch" and
    .run_id == $run_id and
    .run_attempt == $run_attempt and
    .source.expected_source_sha == $source_sha and
    .source.event_sha == $source_sha and
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
    }]
  ' "$manifest" >/dev/null || { echo "BLOCKED: evidence manifest does not match trusted workflow identity." >&2; exit 1; }
printf '%s\n' "$manifest"
