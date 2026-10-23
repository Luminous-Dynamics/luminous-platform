#!/usr/bin/env bash
set -euo pipefail
readonly script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
nixward_root="${1:-}" mycelix_root="${2:-}" sovereign_boot_root="${3:-}" output_dir="${4:-}"
if [[ -z "$nixward_root" || -z "$mycelix_root" || -z "$sovereign_boot_root" || -z "$output_dir" ]]; then
  echo "Usage: $0 NIXWARD_ROOT MYCELIX_ROOT SOVEREIGN_BOOT_ROOT OUTPUT_DIR" >&2
  exit 2
fi
mkdir -p "$output_dir"
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
  and (.sources.interop_harness.adversarial_script_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.adversarial_script_raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.sources.interop_harness.test_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.sources.interop_harness.upstream_lock_raw_sha256 | test("^[0-9a-f]{64}$"))
' "$output_dir/lifecycle-interop-assessment.json" >/dev/null
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
bash "$script_dir/adversarial.sh" "$nixward_root" "$mycelix_root" "$sovereign_boot_root" "$tmp/boot-observation.json" "$output_dir"
jq -e '
  .schema == "srq-lifecycle-adversarial-assessment-v1"
  and .status == "PASS" and .release_status == "BLOCKED"
  and .expected_case_count == 15 and .rejected_case_count == 15
  and .all_expected_mutations_rejected == true
  and .expected_positive_control_count == 3
  and .accepted_positive_control_count == 3
  and .all_positive_controls_accepted == true
  and (.positive_controls | length == 3)
  and ([.positive_controls[] | (.expected_outcome == "accept_record_but_keep_release_blocked" and .observed_outcome == "ACCEPTED_WITH_RELEASE_BLOCKED" and .release_status == "BLOCKED" and .claim_ceiling == "none" and (.candidate_raw_sha256 | test("^[0-9a-f]{64}$")))] | all)
  and (.harness.commit_sha | test("^[0-9a-f]{40}$"))
  and (.harness.adversarial_script_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.harness.adversarial_script_sha256 | test("^[0-9a-f]{64}$"))
  and (.harness.validator_git_blob_sha1 | test("^[0-9a-f]{40}$"))
  and (.harness.validator_sha256 | test("^[0-9a-f]{64}$"))
  and (.fixture.raw_sha256 | test("^[0-9a-f]{64}$"))
  and (.cases | length == 15)
  and ([.cases[] | (.validator_exit_status > 0 and .expected_outcome == "reject" and .observed_outcome == "REJECTED_AS_EXPECTED" and (.candidate_raw_sha256 | test("^[0-9a-f]{64}$")) and (.rejection_reason | startswith("CONTRACT VIOLATION:")))] | all)
' "$output_dir/srq-lifecycle-adversarial-assessment.json" >/dev/null

git -C "$nixward_root" diff --exit-code -- docs/fixtures/system-transaction-v1.json
git -C "$mycelix_root" diff --exit-code -- research/evidence-plane/receipt-v0.schema.json research/evidence-plane/examples/synthetic-planned-receipt-v0.json
git -C "$sovereign_boot_root" diff --exit-code -- tools/srq-boot-observation-probe/src/main.rs
echo "PASS: interoperability harness leaves upstream source and fixtures unchanged"
