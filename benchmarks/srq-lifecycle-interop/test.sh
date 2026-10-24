#!/usr/bin/env bash
set -euo pipefail
readonly script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
nixward_root="${1:-}" mycelix_root="${2:-}" sovereign_boot_root="${3:-}" output_dir="${4:-}"
if [[ -z "$nixward_root" || -z "$mycelix_root" || -z "$sovereign_boot_root" || -z "$output_dir" ]]; then
  echo "Usage: $0 NIXWARD_ROOT MYCELIX_ROOT SOVEREIGN_BOOT_ROOT OUTPUT_DIR" >&2
  exit 2
fi
mkdir -p "$output_dir"
schema_python="${SRQ_SCHEMA_PYTHON:-python3}"
export SRQ_SCHEMA_PYTHON="$schema_python"
schema_reports_dir="$output_dir/schema-validation"
mkdir -p "$schema_reports_dir"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export CARGO_TARGET_DIR="$tmp/cargo-target"
probe_manifest="$sovereign_boot_root/tools/srq-boot-observation-probe/Cargo.toml"
cargo fmt --manifest-path "$probe_manifest" -- --check
cargo test --manifest-path "$probe_manifest" --locked

root="$tmp/fixture-root"
store_hash="0123456789abcdfghijklmnpqrsvwxyz"
deployment_name="nixos-system-lifecycle-interop"
mkdir -p "$root/run" "$root/proc/sys/kernel/random" "$root/nix/store/$store_hash-$deployment_name"
printf '01234567-89ab-cdef-0123-456789abcdef\n' > "$root/proc/sys/kernel/random/boot_id"
ln -s "/nix/store/$store_hash-$deployment_name" "$root/run/booted-system"
ln -s "/nix/store/$store_hash-$deployment_name" "$root/run/current-system"
cargo run --manifest-path "$probe_manifest" --locked -- --root "$root" > "$tmp/boot-observation.json"

bash "$script_dir/validate.sh" --nixward-root "$nixward_root" --mycelix-root "$mycelix_root" --sovereign-boot-root "$sovereign_boot_root" --boot-observation "$tmp/boot-observation.json" --output "$output_dir/lifecycle-interop-assessment.json"
jq -e '
  .schema == "srq-lifecycle-interop-assessment-v1"
  and .contract_status == "PASS" and .release_status == "BLOCKED" and .claim_ceiling == "none"
  and .transaction.phase == "planned"
  and .boot_observation.evidence_tier == "D0_LOCAL_DIAGNOSTIC" and .boot_observation.status == "MATCH"
  and .boot_observation.authenticated == false and .boot_observation.release_qualifying == false
  and .boot_observation.security_bindings_complete == false
  and .evidence_plane.evidence_tier == "S0" and .evidence_plane.claim_ceiling == "synthetic_only"
  and .evidence_plane.result_state == "not_run"
  and (.release_blockers | index("transaction_not_promoted") != null)
  and (.release_blockers | index("boot_observation_d0_only") != null)
  and (.release_blockers | index("evidence_plane_synthetic_claim_ceiling") != null)
  and (.release_blockers | index("evidence_plane_not_independently_verified") != null)
  and (.sources.nixward_transaction.raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.mycelix_evidence_plane_receipt.raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.sovereign_boot_observation.raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.commit_sha | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.validator_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.validator_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.schema_validator_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.schema_validator_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.schema_validator_test_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.schema_validator_test_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.schema_requirements_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.schema_requirements_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.adversarial_script_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.adversarial_script_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.test_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.upstream_lock_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.corpus_protocol_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.corpus_protocol_raw_sha256 | test("^[0-9a-f]{64}$"))
  and .sources.interop_harness.corpus_protocol_id == "srq-lifecycle-adversarial-v1"
  and (.sources.interop_harness.corpus_schema_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.corpus_schema_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.corpus_validator_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.corpus_validator_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.corpus_validator_test_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.corpus_validator_test_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.transaction_phase_matrix_protocol_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.transaction_phase_matrix_protocol_raw_sha256 | test("^[0-9a-f]{64}$"))
  and .sources.interop_harness.transaction_phase_matrix_protocol_id == "srq-transaction-phase-presence-matrix-v1"
  and (.sources.interop_harness.transaction_phase_matrix_runner_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.transaction_phase_matrix_runner_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.transaction_phase_matrix_test_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.transaction_phase_matrix_test_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.claim_matrix_protocol_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.claim_matrix_protocol_raw_sha256 | test("^[0-9a-f]{64}$"))
  and .sources.interop_harness.claim_matrix_protocol_id == "srq-evidence-claim-ceiling-matrix-v1"
  and (.sources.interop_harness.claim_matrix_runner_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.claim_matrix_runner_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.claim_matrix_test_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.claim_matrix_test_raw_sha256 | test("^[0-9a-f]{64}$"))
' "$output_dir/lifecycle-interop-assessment.json" >/dev/null

"$schema_python" "$script_dir/corpus_validate.py" \
  --schema "$script_dir/corpus-v1.schema.json" \
  --manifest "$script_dir/corpus-v1.json" \
  --lock "$script_dir/upstream-lock-v1.json" \
  --expect valid \
  --output "$schema_reports_dir/corpus-v1.json" >/dev/null
jq -e '
  .schema == "srq-corpus-validation-report-v1"
  and .result == "VALID"
  and .validator.installed_version == "4.26.0"
  and .protocol_id == "srq-lifecycle-adversarial-v1"
  and .semantic_error_count == 0
  and (.manifest_input.sha256 | test("^[0-9a-f]{64}$"))
  and (.schema_input.sha256 | test("^[0-9a-f]{64}$"))
  and (.lock_input.sha256 | test("^[0-9a-f]{64}$"))
' "$schema_reports_dir/corpus-v1.json" >/dev/null
echo "PASS: corpus manifest passes independent schema and semantic validation"

"$schema_python" "$script_dir/schema_validate.py" \
  --schema "$mycelix_root/research/evidence-plane/receipt-v0.schema.json" \
  --instance "$mycelix_root/research/evidence-plane/examples/synthetic-planned-receipt-v0.json" \
  --expect valid \
  --output "$schema_reports_dir/mycelix-planned-s0.json" >/dev/null
jq -e '
  .schema == "srq-json-schema-validation-report-v1"
  and .result == "VALID"
  and .expected == "valid"
  and .validator.required_version == "4.26.0"
  and .validator.installed_version == "4.26.0"
  and .validator.draft == "https://json-schema.org/draft/2020-12/schema"
  and .validator.format_assertion == true
  and (.schema_input.sha256 | test("^[0-9a-f]{64}$"))
  and (.instance_input.sha256 | test("^[0-9a-f]{64}$"))
' "$schema_reports_dir/mycelix-planned-s0.json" >/dev/null
echo "PASS: full JSON Schema Draft 2020-12 validation accepts the pinned planned S0 receipt"
echo "PASS: exact upstream inputs interoperate while the lifecycle remains release-blocked"

# Negative control 1: D0 may never claim authentication or release qualification.
jq '.authenticated = true | .release_qualifying = true' "$tmp/boot-observation.json" > "$tmp/forged-d0.json"
if bash "$script_dir/validate.sh" --nixward-root "$nixward_root" --mycelix-root "$mycelix_root" --sovereign-boot-root "$sovereign_boot_root" --boot-observation "$tmp/forged-d0.json" >/dev/null 2>&1; then
  echo "FAIL: forged D0 authentication/qualification claims were accepted" >&2; exit 1
fi
echo "PASS: forged D0 authentication/qualification claims fail closed"

# Negative control 2: S0 -> S4 claim inflation without source, verifier and gate evidence must fail.
jq '.evidence_tier = "S4" | .claim_ceiling = "domain_validated" | .run_status = "domain_validated" | .benchmark.results_summary.state = "available" | .benchmark.results_summary.observed_result_count = 1' \
  "$mycelix_root/research/evidence-plane/examples/synthetic-planned-receipt-v0.json" > "$tmp/inflated-s0.json"
if bash "$script_dir/validate.sh" --nixward-root "$nixward_root" --mycelix-root "$mycelix_root" --sovereign-boot-root "$sovereign_boot_root" --boot-observation "$tmp/boot-observation.json" --evidence-plane-receipt "$tmp/inflated-s0.json" >/dev/null 2>&1; then
  echo "FAIL: inflated S0 evidence claim was accepted" >&2; exit 1
fi
echo "PASS: claim-ceiling inflation without source/verifier/gate evidence fails closed"

# Negative control 3: a promoted phase label alone cannot satisfy a transaction.
jq '.phase = "promoted"' "$nixward_root/docs/fixtures/system-transaction-v1.json" > "$tmp/falsely-promoted-transaction.json"
if bash "$script_dir/validate.sh" --nixward-root "$nixward_root" --mycelix-root "$mycelix_root" --sovereign-boot-root "$sovereign_boot_root" --boot-observation "$tmp/boot-observation.json" --transaction-fixture "$tmp/falsely-promoted-transaction.json" >/dev/null 2>&1; then
  echo "FAIL: phase-only promotion was accepted" >&2; exit 1
fi
echo "PASS: phase-only promotion without authorization/readback/outcome fails closed"

# Expand beyond hand-picked controls into a reusable cross-field mutation corpus.
jq -e '
  .schema == "srq-lifecycle-adversarial-protocol-v1"
  and .protocol_id == "srq-lifecycle-adversarial-v1"
  and .version == 1 and (.cases | length == 17) and (.positive_controls | length == 5)
  and ([.cases[].id] | length == (unique | length))
  and ([.positive_controls[].id] | length == (unique | length))
' "$script_dir/corpus-v1.json" >/dev/null
bash "$script_dir/adversarial.sh" "$nixward_root" "$mycelix_root" "$sovereign_boot_root" "$tmp/boot-observation.json" "$output_dir"
jq -e --slurpfile protocol "$script_dir/corpus-v1.json" '
  .schema == "srq-lifecycle-adversarial-assessment-v1"
  and .protocol.protocol_id == $protocol[0].protocol_id
  and .protocol.version == $protocol[0].version
  and (.protocol.git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.protocol.raw_sha256 | test("^[0-9a-f]{64}$"))
  and .status == "PASS" and .release_status == "BLOCKED" and .claim_ceiling == "none"
  and .expected_case_count == 17 and .rejected_case_count == 17
  and .all_expected_mutations_rejected == true
  and .expected_positive_control_count == 5
  and .accepted_positive_control_count == 5
  and .all_positive_controls_accepted == true
  and .observed_metrics.negative_false_accept_count == 0
  and .observed_metrics.positive_false_reject_count == 0
  and .observed_metrics.release_false_pass_count == 0
  and $protocol[0].target_metrics.negative_false_accept_count == .observed_metrics.negative_false_accept_count
  and $protocol[0].target_metrics.positive_false_reject_count == .observed_metrics.positive_false_reject_count
  and $protocol[0].target_metrics.release_false_pass_count == .observed_metrics.release_false_pass_count
  and .schema_validator.library == "python-jsonschema"
  and .schema_validator.version == "4.26.0"
  and .schema_validator.format_assertion == true
  and (.fixture.schema_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.harness.schema_validator_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.harness.schema_validator_sha256 | test("^[0-9a-f]{64}$"))
  and (.harness.schema_requirements_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.harness.schema_requirements_sha256 | test("^[0-9a-f]{64}$"))
  and ([.cases[] | select(.input_kind == "mycelix_receipt")] | (length == 9 and all(.[]; .schema_validation_result == "INVALID" and (.schema_validation_report_raw_sha256 | test("^[0-9a-f]{64}$")))))
  and ([.cases[] | select(.input_kind == "nixward_transaction")] | (length == 8 and all(.[]; .schema_validation_result == "NOT_APPLICABLE" and .schema_validation_report_raw_sha256 == null)))
  and ([.positive_controls[] | select(.input_kind == "mycelix_receipt")] | (length == 2 and all(.[]; .schema_validation_result == "VALID" and (.schema_validation_report_raw_sha256 | test("^[0-9a-f]{64}$")))))
  and ([.positive_controls[] | select(.input_kind == "nixward_transaction")] | (length == 3 and all(.[]; .schema_validation_result == "NOT_APPLICABLE" and .schema_validation_report_raw_sha256 == null)))
  and (.positive_controls | length == 5)
  and ([.positive_controls[] | (.expected_outcome == "accept_record_but_keep_release_blocked" and .observed_outcome == "ACCEPTED_WITH_RELEASE_BLOCKED" and .release_status == "BLOCKED" and .claim_ceiling == "none" and (.candidate_raw_sha256 | test("^[0-9a-f]{64}$")))] | all)
  and (.harness.commit_sha | test("^[0-9a-f]{40}$"))
  and (.harness.adversarial_script_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.harness.adversarial_script_sha256 | test("^[0-9a-f]{64}$"))
  and (.harness.validator_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.harness.validator_sha256 | test("^[0-9a-f]{64}$"))
  and (.fixture.raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.cases | length == 17)
  and ([.cases[] | (.validator_exit_status > 0 and .expected_outcome == "reject" and .observed_outcome == "REJECTED_AS_EXPECTED" and (.candidate_raw_sha256 | test("^[0-9a-f]{64}$")) and (.rejection_reason | startswith("CONTRACT VIOLATION:")))] | all)
' "$output_dir/srq-lifecycle-adversarial-assessment.json" >/dev/null

"$schema_python" "$script_dir/phase_matrix.py" \
  --protocol "$script_dir/transaction-phase-matrix-v1.json" \
  --lock "$script_dir/upstream-lock-v1.json" \
  --transaction-fixture "$nixward_root/docs/fixtures/system-transaction-v1.json" \
  --validator "$script_dir/validate.sh" \
  --nixward-root "$nixward_root" \
  --mycelix-root "$mycelix_root" \
  --sovereign-boot-root "$sovereign_boot_root" \
  --boot-observation "$tmp/boot-observation.json" \
  --output "$output_dir/transaction-phase-matrix-assessment.json"
jq -e '
  .schema == "srq-transaction-phase-presence-matrix-assessment-v1"
  and .result == "PASS"
  and .protocol.protocol_id == "srq-transaction-phase-presence-matrix-v1"
  and (.protocol.git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.protocol.raw_sha256 | test("^[0-9a-f]{64}$"))
  and .expected_case_count == 576 and .completed_case_count == 576
  and .expected_accept_count == 175 and .observed_accept_count == 175
  and .false_accept_count == 0 and .false_reject_count == 0 and .release_false_pass_count == 0
  and (.cases | length == 576)
  and ([.cases[].id] | length == (unique | length))
  and ([.cases[] | select(.result != "MATCH")] | length == 0)
  and .phase_summary.planned.case_count == 64
  and .phase_summary.planned.expected_accept_count == 64
  and .phase_summary.promoted.expected_accept_count == 1
  and .phase_summary.promoted.false_accept_count == 0
  and .phase_summary.promoted.false_reject_count == 0
  and (.performance_ms.p50 | type == "number")
  and (.performance_ms.p95 | type == "number")
  and (.performance_ms.p99 | type == "number")
' "$output_dir/transaction-phase-matrix-assessment.json" >/dev/null

"$schema_python" "$script_dir/claim_matrix.py" \
  --protocol "$script_dir/claim-ceiling-matrix-v1.json" \
  --lock "$script_dir/upstream-lock-v1.json" \
  --schema "$mycelix_root/research/evidence-plane/receipt-v0.schema.json" \
  --base-receipt "$mycelix_root/research/evidence-plane/examples/synthetic-planned-receipt-v0.json" \
  --schema-validator "$script_dir/schema_validate.py" \
  --validator "$script_dir/validate.sh" \
  --nixward-root "$nixward_root" \
  --mycelix-root "$mycelix_root" \
  --sovereign-boot-root "$sovereign_boot_root" \
  --boot-observation "$tmp/boot-observation.json" \
  --output "$output_dir/claim-ceiling-matrix-assessment.json"
jq -e '
  .schema == "srq-evidence-claim-ceiling-matrix-assessment-v1"
  and .result == "PASS"
  and .protocol.protocol_id == "srq-evidence-claim-ceiling-matrix-v1"
  and (.protocol.git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.protocol.raw_sha256 | test("^[0-9a-f]{64}$"))
  and .schema_input.commit_sha == "b02cefab8474262aef23b61a4ac3811fe92e29bf"
  and .schema_input.git_blob_sha1 == "e2b90a995f70d0a771b04e7f5fc077e1c0394f86"
  and (.schema_input.raw_sha256 | test("^[0-9a-f]{64}$"))
  and .expected_case_count == 25 and .completed_case_count == 25
  and .expected_accept_count == 8 and .observed_accept_count == 8 and .schema_accepted_count == 8
  and .false_accept_count == 0 and .false_reject_count == 0
  and .schema_adapter_disagreement_count == 0 and .release_false_pass_count == 0
  and ([.cases[].id] | length == (unique | length))
  and ([.cases[] | select(.result != "MATCH")] | length == 0)
  and .tier_summary.S0.expected_accept_count == 1
  and .tier_summary.S1.expected_accept_count == 2
  and .tier_summary.S2.expected_accept_count == 3
  and .tier_summary.S3.expected_accept_count == 1
  and .tier_summary.S4.expected_accept_count == 1
  and ([.cases[] | select(.expected_accept) | (.assessment_release_status == "BLOCKED" and .assessment_claim_ceiling == "none")] | all)
  and (.performance_ms.p50 | type == "number")
  and (.performance_ms.p95 | type == "number")
  and (.performance_ms.p99 | type == "number")
' "$output_dir/claim-ceiling-matrix-assessment.json" >/dev/null

git -C "$nixward_root" diff --exit-code -- docs/fixtures/system-transaction-v1.json
git -C "$mycelix_root" diff --exit-code -- research/evidence-plane/receipt-v0.schema.json research/evidence-plane/examples/synthetic-planned-receipt-v0.json
git -C "$sovereign_boot_root" diff --exit-code -- tools/srq-boot-observation-probe/src/main.rs
echo "PASS: interoperability harness leaves upstream source and fixtures unchanged"
