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
jq -e \
  --arg schema_env_install_report_sha256 "$(sha256sum "${SRQ_SCHEMA_INSTALL_REPORT:?missing hash-locked pip install report}" | cut -d ' ' -f 1)" \
  --arg schema_env_validation_report_sha256 "$(sha256sum "${SRQ_SCHEMA_ENV_VALIDATION_REPORT:?missing validated package report}" | cut -d ' ' -f 1)" \
  --arg schema_env_lock_sha256 "$(sha256sum "$script_dir/schema-env-lock-v1.json" | cut -d ' ' -f 1)" \
  --arg executable_mutations_sha256 "$(sha256sum "$script_dir/corpus-executable-mutations-v1.json" | cut -d ' ' -f 1)" \
  --arg executable_mutations_blob "$(git -C "$script_dir/../.." hash-object -- benchmarks/srq-lifecycle-interop/corpus-executable-mutations-v1.json)" \
  --arg phase_matrix_sha256 "$(sha256sum "$script_dir/transaction-phase-matrix.sh" | cut -d ' ' -f 1)" \
  --arg phase_matrix_blob "$(git -C "$script_dir/../.." hash-object -- benchmarks/srq-lifecycle-interop/transaction-phase-matrix.sh)" \
  --arg phase_protocol_sha256 "$(sha256sum "$script_dir/transaction-phase-state-protocol-v1.json" | cut -d ' ' -f 1)" \
  --arg phase_protocol_blob "$(git -C "$script_dir/../.." hash-object -- benchmarks/srq-lifecycle-interop/transaction-phase-state-protocol-v1.json)" \
  --arg phase_protocol_id "$(jq -er '.protocol_id' "$script_dir/transaction-phase-state-protocol-v1.json)" '
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
  and (.sources.interop_harness.executable_mutations_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and .sources.interop_harness.executable_mutations_git_blob_sha1 == $executable_mutations_blob
  and .sources.interop_harness.executable_mutations_raw_sha256 == $executable_mutations_sha256
  and (.sources.interop_harness.corpus_schema_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.corpus_schema_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.corpus_validator_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.corpus_validator_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.corpus_validator_test_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.corpus_validator_test_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.schema_environment_lock_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and .sources.interop_harness.schema_environment_lock_raw_sha256 == $schema_env_lock_sha256
  and (.sources.interop_harness.schema_environment_validator_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.schema_environment_validator_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.schema_environment_validator_test_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.schema_environment_validator_test_raw_sha256 | test("^[0-9a-f]{64}$"))
  and .sources.interop_harness.schema_install_report_raw_sha256 == $schema_env_install_report_sha256
  and .sources.interop_harness.schema_environment_validation_report_raw_sha256 == $schema_env_validation_report_sha256
  and .sources.interop_harness.phase_matrix_git_blob_sha1 == $phase_matrix_blob
  and .sources.interop_harness.phase_matrix_raw_sha256 == $phase_matrix_sha256
  and .sources.interop_harness.phase_matrix_protocol_git_blob_sha1 == $phase_protocol_blob
  and .sources.interop_harness.phase_matrix_protocol_raw_sha256 == $phase_protocol_sha256
  and .sources.interop_harness.phase_matrix_protocol_id == $phase_protocol_id
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
jq -e --slurpfile parent "$script_dir/corpus-v1.json" '
  .schema == "srq-lifecycle-executable-mutations-v1"
  and .version == 1 and .parent_protocol_id == $parent[0].protocol_id
  and ([.cases[] | {id,input_kind,expected_schema_result,expected_lifecycle_result}] | sort_by(.id))
      == ([$parent[0].cases[] | {id,input_kind,expected_schema_result,expected_lifecycle_result}] | sort_by(.id))
  and ([.positive_controls[] | {id,input_kind,expected_schema_result,expected_lifecycle_result,expected_evidence_tier,expected_evidence_claim_ceiling,expected_transaction_phase}] | sort_by(.id))
      == ([$parent[0].positive_controls[] | {id,input_kind,expected_schema_result,expected_lifecycle_result,expected_evidence_tier,expected_evidence_claim_ceiling,expected_transaction_phase}] | sort_by(.id))
  and (.cases | all(.[]; (.transform_filter | type == "string" and length > 0) and (.expected_failure_reason | type == "string" and length > 0)))
  and (.positive_controls | all(.[]; (.transform_filter | type == "string" and length > 0)))
' "$script_dir/corpus-executable-mutations-v1.json" >/dev/null
bash "$script_dir/adversarial.sh" "$nixward_root" "$mycelix_root" "$sovereign_boot_root" "$tmp/boot-observation.json" "$output_dir"
jq -e --slurpfile protocol "$script_dir/corpus-v1.json" --slurpfile executable "$script_dir/corpus-executable-mutations-v1.json" \
  --arg mutation_sha256 "$(sha256sum "$script_dir/corpus-executable-mutations-v1.json" | cut -d ' ' -f 1)" \
  --arg mutation_blob "$(git -C "$script_dir/../.." hash-object -- benchmarks/srq-lifecycle-interop/corpus-executable-mutations-v1.json)" '
  .schema == "srq-lifecycle-adversarial-assessment-v1"
  and .executable_mutations.schema == $executable[0].schema
  and .executable_mutations.version == $executable[0].version
  and .executable_mutations.parent_protocol_id == $protocol[0].protocol_id
  and .executable_mutations.raw_sha256 == $mutation_sha256
  and .executable_mutations.git_blob_sha1 == $mutation_blob
  and ([.cases[] | {id,input_kind,mutation_filter}] | sort_by(.id))
      == ([$executable[0].cases[] | {id,input_kind,mutation_filter:.transform_filter}] | sort_by(.id))
  and ([.positive_controls[] | {id,input_kind,mutation_filter}] | sort_by(.id))
      == ([$executable[0].positive_controls[] | {id,input_kind,mutation_filter:.transform_filter}] | sort_by(.id))
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

bash "$script_dir/transaction-phase-matrix.sh" "$nixward_root" "$mycelix_root" "$sovereign_boot_root" "$tmp/boot-observation.json" "$output_dir"
jq -e --slurpfile protocol "$script_dir/transaction-phase-state-protocol-v1.json" \
  --arg head_sha "$(git -C "$script_dir/../.." rev-parse HEAD)" \
  --arg phase_matrix_sha256 "$(sha256sum "$script_dir/transaction-phase-matrix.sh" | cut -d ' ' -f 1)" \
  --arg phase_matrix_blob "$(git -C "$script_dir/../.." hash-object -- benchmarks/srq-lifecycle-interop/transaction-phase-matrix.sh)" \
  --arg phase_protocol_sha256 "$(sha256sum "$script_dir/transaction-phase-state-protocol-v1.json" | cut -d ' ' -f 1)" \
  --arg phase_protocol_blob "$(git -C "$script_dir/../.." hash-object -- benchmarks/srq-lifecycle-interop/transaction-phase-state-protocol-v1.json)" \
  --arg phase_protocol_id "$(jq -er '.protocol_id' "$script_dir/transaction-phase-state-protocol-v1.json")" '
  .schema == "srq-transaction-phase-state-space-assessment-v1"
  and .status == "PASS"
  and .release_status == "BLOCKED" and .claim_ceiling == "none"
  and .expected_state_count == 576 and .observed_state_count == 576
  and .expected_accept_count == 175 and .expected_reject_count == 401
  and .false_accept_count == 0 and .false_reject_count == 0
  and .unexpected_error_count == 0 and .false_release_pass_count == 0
  and .accepted_with_blocked_release_count == 175
  and .harness.commit_sha == $head_sha
  and .harness.script_git_blob_sha1 == $phase_matrix_blob
  and .harness.script_raw_sha256 == $phase_matrix_sha256
  and .protocol.protocol_id == $phase_protocol_id
  and .protocol.protocol_id == $protocol[0].protocol_id
  and .protocol.version == $protocol[0].version
  and .protocol.git_blob_sha1 == $phase_protocol_blob
  and .protocol.raw_sha256 == $phase_protocol_sha256
  and .expected_state_count == $protocol[0].oracle.expected_state_count
  and .observed_state_count == $protocol[0].oracle.expected_state_count
  and .expected_accept_count == $protocol[0].oracle.expected_accept_count
  and .expected_reject_count == $protocol[0].oracle.expected_reject_count
  and .false_accept_count == $protocol[0].target_metrics.false_accept_count
  and .false_reject_count == $protocol[0].target_metrics.false_reject_count
  and .unexpected_error_count == $protocol[0].target_metrics.unexpected_error_count
  and .false_release_pass_count == $protocol[0].target_metrics.false_release_pass_count
  and (.source_fixture.commit_sha | test("^[0-9a-f]{40}$"))
  and (.source_fixture.git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.source_fixture.raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.cases | length == 576)
  and ([.cases[] | (.candidate_raw_sha256 | test("^[0-9a-f]{64}$"))] | all)
' "$output_dir/transaction-phase-matrix-assessment.json" >/dev/null
echo "PASS: 576 phase/evidence states match the minimum-prerequisite oracle with zero false accepts, false rejects, unexpected errors or false release passes"

git -C "$nixward_root" diff --exit-code -- docs/fixtures/system-transaction-v1.json
git -C "$mycelix_root" diff --exit-code -- research/evidence-plane/receipt-v0.schema.json research/evidence-plane/examples/synthetic-planned-receipt-v0.json
git -C "$sovereign_boot_root" diff --exit-code -- tools/srq-boot-observation-probe/src/main.rs
echo "PASS: interoperability harness leaves upstream source and fixtures unchanged"
