#!/usr/bin/env bash
set -euo pipefail
readonly script_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
readonly lock="$script_dir/upstream-lock-v1.json"
nixward_root="" mycelix_root="" sovereign_boot_root="" boot_observation="" output="-"
transaction_override="" evidence_override=""

usage() { echo "Usage: $0 --nixward-root DIR --mycelix-root DIR --sovereign-boot-root DIR --boot-observation JSON [--transaction-fixture JSON] [--evidence-plane-receipt JSON] [--output JSON]" >&2; }
while (($#)); do
  case "$1" in
    --nixward-root) [[ $# -ge 2 ]] || { usage; exit 2; }; nixward_root="$2"; shift 2 ;;
    --mycelix-root) [[ $# -ge 2 ]] || { usage; exit 2; }; mycelix_root="$2"; shift 2 ;;
    --sovereign-boot-root) [[ $# -ge 2 ]] || { usage; exit 2; }; sovereign_boot_root="$2"; shift 2 ;;
    --boot-observation) [[ $# -ge 2 ]] || { usage; exit 2; }; boot_observation="$2"; shift 2 ;;
    --transaction-fixture) [[ $# -ge 2 ]] || { usage; exit 2; }; transaction_override="$2"; shift 2 ;;
    --evidence-plane-receipt) [[ $# -ge 2 ]] || { usage; exit 2; }; evidence_override="$2"; shift 2 ;;
    --output) [[ $# -ge 2 ]] || { usage; exit 2; }; output="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done
for pair in "nixward_root:$nixward_root" "mycelix_root:$mycelix_root" "sovereign_boot_root:$sovereign_boot_root" "boot_observation:$boot_observation"; do
  [[ "${pair#*:}" != "" ]] || { echo "BLOCKED: missing required input ${pair%%:*}" >&2; exit 2; }
done
command -v jq >/dev/null 2>&1 || { echo "BLOCKED: jq is required; no assessment produced." >&2; exit 2; }
command -v sha256sum >/dev/null 2>&1 || { echo "BLOCKED: sha256sum is required; no assessment produced." >&2; exit 2; }
command -v git >/dev/null 2>&1 || { echo "BLOCKED: git is required; no source pin can be verified." >&2; exit 2; }

pin() { jq -ec --arg key "$1" '.pins[] | select(.key == $key)' "$lock"; }
pin_field() { pin "$1" | jq -er --arg field "$2" '.[$field]'; }
check_pinned_file() {
  local key="$1" root="$2" expected_commit expected_blob path actual_commit actual_blob
  expected_commit="$(pin_field "$key" commit_sha)"
  expected_blob="$(pin_field "$key" git_blob_sha1)"
  path="$(pin_field "$key" path)"
  [[ -d "$root/.git" ]] || { echo "BLOCKED: $key checkout has no Git metadata." >&2; return 1; }
  actual_commit="$(git -C "$root" rev-parse HEAD)"
  [[ "$actual_commit" == "$expected_commit" ]] || { echo "BLOCKED: $key commit mismatch: expected $expected_commit, found $actual_commit" >&2; return 1; }
  [[ -f "$root/$path" && ! -L "$root/$path" ]] || { echo "BLOCKED: $key must be a regular, non-symlink file: $path" >&2; return 1; }
  actual_blob="$(git -C "$root" hash-object -- "$path")"
  [[ "$actual_blob" == "$expected_blob" ]] || { echo "BLOCKED: $key Git blob mismatch: expected $expected_blob, found $actual_blob" >&2; return 1; }
}

# Always verify the immutable upstream files themselves, even when a negative
# control supplies a separate mutated copy for semantic testing.
check_pinned_file nixward_transaction_fixture "$nixward_root"
check_pinned_file mycelix_evidence_plane_schema "$mycelix_root"
check_pinned_file mycelix_planned_receipt "$mycelix_root"
check_pinned_file sovereign_boot_d0_probe_source "$sovereign_boot_root"

nixward_pinned="$nixward_root/$(pin_field nixward_transaction_fixture path)"
mycelix_receipt_pinned="$mycelix_root/$(pin_field mycelix_planned_receipt path)"
mycelix_schema="$mycelix_root/$(pin_field mycelix_evidence_plane_schema path)"
nixward_fixture="$nixward_pinned"
mycelix_receipt="$mycelix_receipt_pinned"
[[ -z "$transaction_override" ]] || nixward_fixture="$transaction_override"
[[ -z "$evidence_override" ]] || mycelix_receipt="$evidence_override"
[[ -f "$nixward_fixture" && ! -L "$nixward_fixture" ]] || { echo "BLOCKED: transaction fixture must be a regular file." >&2; exit 2; }
[[ -f "$mycelix_receipt" && ! -L "$mycelix_receipt" ]] || { echo "BLOCKED: evidence-plane receipt must be a regular file." >&2; exit 2; }
[[ -f "$boot_observation" && ! -L "$boot_observation" ]] || { echo "BLOCKED: boot observation must be a regular, non-symlink JSON file." >&2; exit 2; }

jq -e '
  .schema == "luminous-nixward-system-transaction-v1" and .version == 1
  and (.transaction_id | type == "string" and test("^[0-9a-f]{64}$"))
  and (.target_machine_digest | type == "string" and test("^[0-9a-f]{64}$"))
  and (.plan_digest | type == "string" and test("^[0-9a-f]{64}$"))
  and (.phase | IN("planned","validated","authorized","snapshotted","applied","verified","promoted","failed","recovered"))
  and has("authorization") and has("validation") and has("snapshot") and has("application") and has("verification") and has("outcome")
  and (if .phase == "promoted" then .authorization != null and .validation != null and .snapshot != null and .application != null and .verification != null and .outcome != null else true end)
' "$nixward_fixture" >/dev/null || { echo "CONTRACT VIOLATION: Nixward transaction fields or phase invariants are inconsistent." >&2; exit 1; }

jq -e '
  .schema == "srq-boot-observation-diagnostic-v1"
  and .evidence_tier == "D0_LOCAL_DIAGNOSTIC"
  and (.status | IN("MATCH","MISMATCH","BLOCKED"))
  and .authenticated == false and .release_qualifying == false
  and .target_id == null and .decision_id == null and .challenge_id == null
  and .effective_policy_digest == null and .artifact_digest == null and .measurement == null
  and (.boot_instance_id == null or (.boot_instance_id | type == "string"))
  and (.booted_deployment_id == null or (.booted_deployment_id | type == "string"))
  and (.current_userspace_deployment_id == null or (.current_userspace_deployment_id | type == "string"))
' "$boot_observation" >/dev/null || { echo "CONTRACT VIOLATION: D0 diagnostics may not claim authentication, release qualification, or security bindings." >&2; exit 1; }

jq -e '
  .schema_version == "0.1.0"
  and (.receipt_id | type == "string" and length > 0)
  and (.run_status | IN("planned","executed","independently_checked","domain_validated","rejected","superseded"))
  and (.evidence_tier | IN("S0","S1","S2","S3","S4"))
  and (.claim_ceiling | IN("synthetic_only","transcription_only","source_reproducible","independently_reproduced","domain_validated"))
  and (.gates | type == "object") and (.subject | type == "object")
  and (.independent_verification | type == "object") and (.benchmark.results_summary | type == "object")
  and (if .evidence_tier == "S0" then .claim_ceiling == "synthetic_only" else true end)
  and (if .run_status == "planned" then .benchmark.results_summary.state == "not_run" and .benchmark.results_summary.observed_result_count == 0 else true end)
  and (if .evidence_tier == "S4" or .claim_ceiling == "domain_validated"
       then (.subject.commit | type == "string" and test("^[a-fA-F0-9]{40,64}$"))
         and (.subject.artifact_sha256 | type == "string" and test("^[0-9a-f]{64}$"))
         and (.independent_verification.status != "none")
         and ([.gates[]] | all(. == "pass" or . == "not_applicable"))
       else true end)
' "$mycelix_receipt" >/dev/null || { echo "CONTRACT VIOLATION: Mycelix receipt fields do not support their declared evidence tier / claim ceiling." >&2; exit 1; }

# Enforce cross-field semantics needed by this narrow interop check. This is not
# a substitute for evaluating the complete upstream JSON Schema.
jq -e '
  (if .run_status == "planned" then
    .evidence_tier == "S0" and .claim_ceiling == "synthetic_only"
    and .subject.commit == null and .subject.artifact_sha256 == null
    and .benchmark.protocol_sha256 == null
    and .benchmark.pre_registered_at == null
    and .benchmark.split_lock_sha256 == null
    and .benchmark.results_summary.state == "not_run"
    and .benchmark.results_summary.observed_result_count == 0
    and ([.gates[]] | all(. == "unknown"))
    and (.independent_verification.status | IN("none","pending"))
    and .independent_verification.verifier_kind == "none"
    and .independent_verification.replayed_commit == null
   else true end)
  and (if .run_status == "independently_checked" then
    (.evidence_tier | IN("S3","S4"))
    and (.claim_ceiling | IN("independently_reproduced","domain_validated"))
   else true end)
  and (if .run_status == "domain_validated" then
    .evidence_tier == "S4" and .claim_ceiling == "domain_validated"
    and .gates.domain_validation == "pass"
   else true end)
  and (if .evidence_tier == "S0" then .claim_ceiling == "synthetic_only"
       elif .evidence_tier == "S1" then (.claim_ceiling | IN("synthetic_only","transcription_only"))
       elif .evidence_tier == "S2" then (.claim_ceiling | IN("synthetic_only","transcription_only","source_reproducible"))
       else true end)
  and (if .evidence_tier == "S2" and .claim_ceiling == "source_reproducible" then
    (.sources | length > 0)
    and ([.sources[] |
      .raw_bytes_ingested == true
      and (.sha256 | type == "string" and test("^[a-f0-9]{64}$"))
      and (.rights_status | IN("public_domain","open_license","permitted","permission_confirmed"))
      and (.contact_status | IN("permission_confirmed","not_required"))
    ] | all)
    and (.subject.artifact_sha256 | type == "string" and test("^[a-f0-9]{64}$"))
    and .gates.source_integrity == "pass"
    and .gates.rights_stewardship == "pass"
   else true end)
  and (if (.evidence_tier | IN("S3","S4")) then
    (.subject.commit | type == "string" and test("^[a-fA-F0-9]{40,64}$"))
    and (.subject.artifact_sha256 | type == "string" and test("^[a-f0-9]{64}$"))
    and .independent_verification.status == "passed"
    and .gates.independent_verification == "pass"
    and .gates.negative_controls == "pass"
    and .gates.exact_head == "pass"
    and .gates.source_integrity == "pass"
    and .gates.rights_stewardship == "pass"
   else true end)
  and (if (.evidence_tier == "S4" or .claim_ceiling == "domain_validated") then
    .evidence_tier == "S4" and .claim_ceiling == "domain_validated"
    and .gates.domain_validation == "pass"
   else true end)
' "$mycelix_receipt" >/dev/null || { echo "CONTRACT VIOLATION: Mycelix receipt cross-field semantics permit status/tier/claim escalation." >&2; exit 1; }

blockers="$(
  jq -cn --slurpfile tx "$nixward_fixture" --slurpfile boot "$boot_observation" --slurpfile evidence "$mycelix_receipt" '
    [
      (if $tx[0].phase != "promoted" then "transaction_not_promoted" else empty end),
      (if $tx[0].authorization == null then "transaction_authorization_missing" else empty end),
      (if $tx[0].verification == null then "transaction_verification_missing" else empty end),
      (if $tx[0].outcome == null then "transaction_outcome_missing" else empty end),
      (if $boot[0].evidence_tier == "D0_LOCAL_DIAGNOSTIC" then "boot_observation_d0_only" else empty end),
      (if $boot[0].authenticated != true then "boot_observation_not_authenticated" else empty end),
      (if $boot[0].release_qualifying != true then "boot_observation_not_release_qualifying" else empty end),
      (if ($boot[0].target_id == null or $boot[0].decision_id == null or $boot[0].challenge_id == null or $boot[0].effective_policy_digest == null or $boot[0].artifact_digest == null or $boot[0].measurement == null)
       then "boot_observation_missing_security_bindings" else empty end),
      (if $evidence[0].run_status == "planned" then "evidence_plane_not_executed" else empty end),
      (if $evidence[0].evidence_tier == "S0" or $evidence[0].claim_ceiling == "synthetic_only" then "evidence_plane_synthetic_claim_ceiling" else empty end),
      (if $evidence[0].subject.commit == null or $evidence[0].subject.artifact_sha256 == null then "evidence_plane_subject_unbound" else empty end),
      (if ([ $evidence[0].gates[] ] | any(. == "unknown" or . == "fail")) then "evidence_plane_gates_not_satisfied" else empty end),
      (if $evidence[0].independent_verification.status == "none" then "evidence_plane_not_independently_verified" else empty end),
      "interop_harness_does_not_issue_srq_release_pass"
    ] | unique
  '
)"

nixward_blob="$(pin_field nixward_transaction_fixture git_blob_sha1)"
mycelix_schema_blob="$(pin_field mycelix_evidence_plane_schema git_blob_sha1)"
mycelix_receipt_blob="$(pin_field mycelix_planned_receipt git_blob_sha1)"
boot_source_blob="$(pin_field sovereign_boot_d0_probe_source git_blob_sha1)"
nixward_raw_sha="$(sha256sum "$nixward_fixture" | cut -d ' ' -f 1)"
mycelix_schema_raw_sha="$(sha256sum "$mycelix_schema" | cut -d ' ' -f 1)"
mycelix_receipt_raw_sha="$(sha256sum "$mycelix_receipt" | cut -d ' ' -f 1)"
boot_raw_sha="$(sha256sum "$boot_observation" | cut -d ' ' -f 1)"

# Bind the assessment to the exact source code and contract scripts that emitted it.
repo_root="$(cd -- "$script_dir/../.." && pwd)"
interop_commit="$(git -C "$repo_root" rev-parse HEAD)"
interop_validator_path="benchmarks/srq-lifecycle-interop/validate.sh"
interop_test_path="benchmarks/srq-lifecycle-interop/test.sh"
interop_lock_path="benchmarks/srq-lifecycle-interop/upstream-lock-v1.json"
interop_schema_path="benchmarks/srq-lifecycle-interop/assessment-v1.schema.json"
git -C "$repo_root" diff --quiet -- "$interop_validator_path" "$interop_test_path" "$interop_lock_path" "$interop_schema_path" ".github/workflows/srq-lifecycle-interop.yml" "docs/SRQ_LIFECYCLE_INTEROP_V1.md" || {
  echo "BLOCKED: interop source has uncommitted modifications; refusing to bind the report to a different source SHA." >&2
  exit 2
}
git -C "$repo_root" diff --cached --quiet -- "$interop_validator_path" "$interop_test_path" "$interop_lock_path" "$interop_schema_path" ".github/workflows/srq-lifecycle-interop.yml" "docs/SRQ_LIFECYCLE_INTEROP_V1.md" || {
  echo "BLOCKED: interop source has staged modifications; refusing to bind the report to a different source SHA." >&2
  exit 2
}
interop_validator_blob="$(git -C "$repo_root" hash-object -- "$interop_validator_path")"
interop_test_blob="$(git -C "$repo_root" hash-object -- "$interop_test_path")"
interop_lock_blob="$(git -C "$repo_root" hash-object -- "$interop_lock_path")"
interop_schema_blob="$(git -C "$repo_root" hash-object -- "$interop_schema_path")"
interop_validator_sha256="$(sha256sum "$repo_root/$interop_validator_path" | cut -d ' ' -f 1)"
interop_test_sha256="$(sha256sum "$repo_root/$interop_test_path" | cut -d ' ' -f 1)"
interop_lock_sha256="$(sha256sum "$repo_root/$interop_lock_path" | cut -d ' ' -f 1)"
interop_schema_sha256="$(sha256sum "$repo_root/$interop_schema_path" | cut -d ' ' -f 1)"

report="$(
  jq -cn --slurpfile tx "$nixward_fixture" --slurpfile boot "$boot_observation" --slurpfile evidence "$mycelix_receipt" \
    --argjson blockers "$blockers" \
    --arg nixward_commit "$(pin_field nixward_transaction_fixture commit_sha)" --arg nixward_blob "$nixward_blob" --arg nixward_sha256 "$nixward_raw_sha" \
    --arg mycelix_commit "$(pin_field mycelix_planned_receipt commit_sha)" --arg mycelix_schema_blob "$mycelix_schema_blob" --arg mycelix_schema_sha256 "$mycelix_schema_raw_sha" \
    --arg mycelix_receipt_blob "$mycelix_receipt_blob" --arg mycelix_receipt_sha256 "$mycelix_receipt_raw_sha" \
    --arg boot_commit "$(pin_field sovereign_boot_d0_probe_source commit_sha)" --arg boot_source_blob "$boot_source_blob" --arg boot_sha256 "$boot_raw_sha" \
    --arg interop_commit "$interop_commit" \
    --arg interop_validator_blob "$interop_validator_blob" --arg interop_validator_sha256 "$interop_validator_sha256" \
    --arg interop_test_blob "$interop_test_blob" --arg interop_test_sha256 "$interop_test_sha256" \
    --arg interop_lock_blob "$interop_lock_blob" --arg interop_lock_sha256 "$interop_lock_sha256" \
    --arg interop_schema_blob "$interop_schema_blob" --arg interop_schema_sha256 "$interop_schema_sha256" \
    '{
      schema:"srq-lifecycle-interop-assessment-v1", contract_status:"PASS", release_status:"BLOCKED", claim_ceiling:"none", scenario_id:"LIFECYCLE-001",
      sources:{
        nixward_transaction:{repository:"Luminous-Dynamics/nixward",commit_sha:$nixward_commit,git_blob_sha1:$nixward_blob,raw_sha256:$nixward_sha256},
        mycelix_evidence_plane_schema:{repository:"Luminous-Dynamics/mycelix",commit_sha:$mycelix_commit,git_blob_sha1:$mycelix_schema_blob,raw_sha256:$mycelix_schema_sha256},
        mycelix_evidence_plane_receipt:{repository:"Luminous-Dynamics/mycelix",commit_sha:$mycelix_commit,git_blob_sha1:$mycelix_receipt_blob,raw_sha256:$mycelix_receipt_sha256},
        sovereign_boot_probe_source:{repository:"Luminous-Dynamics/sovereign-boot",commit_sha:$boot_commit,git_blob_sha1:$boot_source_blob},
        sovereign_boot_observation:{raw_sha256:$boot_sha256},
        interop_harness:{repository:"Luminous-Dynamics/luminous-platform",commit_sha:$interop_commit,
          validator_git_blob_sha1:$interop_validator_blob,validator_raw_sha256:$interop_validator_sha256,
          test_git_blob_sha1:$interop_test_blob,test_raw_sha256:$interop_test_sha256,
          upstream_lock_git_blob_sha1:$interop_lock_blob,upstream_lock_raw_sha256:$interop_lock_sha256,
          assessment_schema_git_blob_sha1:$interop_schema_blob,assessment_schema_raw_sha256:$interop_schema_sha256}
      },
      transaction:{schema:$tx[0].schema,phase:$tx[0].phase,transaction_id:$tx[0].transaction_id,target_machine_digest:$tx[0].target_machine_digest,plan_digest:$tx[0].plan_digest,authorization_present:($tx[0].authorization != null),verification_present:($tx[0].verification != null),outcome_present:($tx[0].outcome != null)},
      boot_observation:{schema:$boot[0].schema,evidence_tier:$boot[0].evidence_tier,status:$boot[0].status,authenticated:$boot[0].authenticated,release_qualifying:$boot[0].release_qualifying,security_bindings_complete:($boot[0].target_id != null and $boot[0].decision_id != null and $boot[0].challenge_id != null and $boot[0].effective_policy_digest != null and $boot[0].artifact_digest != null and $boot[0].measurement != null)},
      evidence_plane:{receipt_id:$evidence[0].receipt_id,run_status:$evidence[0].run_status,evidence_tier:$evidence[0].evidence_tier,claim_ceiling:$evidence[0].claim_ceiling,result_state:$evidence[0].benchmark.results_summary.state,independent_verification:$evidence[0].independent_verification.status},
      release_blockers:$blockers,
      invariant:"A structurally consistent set of lower-tier records does not imply that the lifecycle is qualified.",
      non_claims:[
        "This assessment is an interoperability contract check, not an SRQ result receipt.",
        "A D0 boot diagnostic is local observation, not authenticated runtime truth.",
        "The Nixward fixture is a planned transaction, not a completed authorized mutation.",
        "The Mycelix example is S0 synthetic/planned evidence, not an executed or independently verified run.",
        "Raw SHA-256 proves byte identity only; upstream Git pins do not establish operational truth or domain validity.",
        "This harness cannot issue a release PASS, certify NixOS recovery, or qualify TPM/physical power-loss behavior."
      ]
    }'
)"
if [[ "$output" == "-" ]]; then printf '%s\n' "$report"; else printf '%s\n' "$report" > "$output"; fi
printf 'INTEROP_CONTRACT_PASS_RELEASE_BLOCKED: pinned transaction + D0 observation + S0 evidence were composed without claim escalation.\n' >&2
