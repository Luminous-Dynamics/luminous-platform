# Deployment Profile Contract

**Status:** Proposed contract, not implemented runtime behavior or a certification scheme.

This document defines what a Luminous Platform deployment profile must say so an operator can decide whether it is suitable for a particular environment. A profile is a versioned, reviewable declaration of capability, dependency, authority, and recovery expectations—not a marketing tier.

## Design goals

1. **Portable declaration:** a profile describes required capabilities and trust boundaries without assuming a specific server, central SaaS control plane, or repository checkout layout.
2. **Fail-closed authority:** unknown, expired, mismatched, or unverifiable authority never grants a privileged action.
3. **Boot/recovery independence:** optional coordination services must not be required for local boot or documented recovery.
4. **Evidence-bound status:** every capability status identifies its source and evidence level. A missing observation is not a healthy state.
5. **Local sovereignty:** keys, raw telemetry, and sensitive operational data remain local unless a policy explicitly authorizes external sharing.
6. **Explicit portability:** a deployment declares the adapter/platform combinations actually tested. An abstract interface does not qualify an untested platform.

## Profile document

Use UTF-8 YAML for human-authored profiles, strict schema validation, and a canonical JSON representation before hashing or signing. Do not sign arbitrary YAML serialization: key order, aliases, scalar types, and parser differences can create ambiguous byte representations.

A profile must have a stable schema version, identifier, profile owner, supported platforms, policy defaults, data-handling rules, dependency requirements, upgrade/rollback requirements, and explicit evidence claims.

### Example: local NixOS pilot

The following is a **proposed example**, not a ready-to-apply system configuration. Adapter names and capability labels are illustrative until the matching implementations and conformance tests exist.

```yaml
schema: luminous.deployment-profile/v1
metadata:
  id: org.example.local-nixos-pilot
  version: 0.1.0
  status: proposed
  owner: customer-organization
  description: Local-only pilot with no remote privileged control plane

target:
  os: nixos
  architectures: [x86_64-linux]
  execution_mode: local
  supported_hardware:
    - virtual-machine
  unsupported_hardware:
    - unqualified-physical-host

authority:
  default_action: deny
  privileged_execution: disabled
  require_fresh_authorization: true
  require_target_binding: true
  require_operation_binding: true
  require_attempt_identity: true
  reject_replay: true
  break_glass:
    enabled: false
    documented_out_of_band_recovery: required

services:
  coordinator:
    required_for_boot: false
    required_for_local_recovery: false
  remote_control:
    enabled: false
  telemetry_export:
    enabled: false

data:
  raw_telemetry: local-only
  evidence_export: operator-approved
  retention_policy: customer-defined
  secrets_in_browser_storage: prohibited
  customer_key_custody: customer-controlled

lifecycle:
  installation: documented
  upgrade: pinned-inputs-and-rollback-plan
  rollback: required
  uninstall: documented
  export_exit_path: required

evidence:
  required_fields:
    - schema_version
    - target_identity
    - operation_id
    - attempt_id
    - source_revision
    - build_digest
    - policy_revision
    - authorization_reference
    - observed_result
    - limitations
  unknown_evidence: unproven
  simulation_as_live_evidence: prohibited
  command_success_as_recovery_proof: prohibited

qualification:
  claimed_level: M0
  required_checks:
    - clean-install
    - local-boot-with-coordinator-absent
    - unauthorized-action-refusal
    - stale-authorization-refusal
    - evidence-export-and-independent-parse
  unqualified_capabilities:
    - production-host-actuation
    - destructive-recovery
    - cross-organization-federation
```

## Required semantics

### Capability claims

Each capability is described independently. Do not assign one platform-wide green status when only a subset of capabilities has evidence.

At minimum, distinguish:

- `unavailable`: no implementation exists for this capability in this build/profile;
- `configured`: configuration declares the capability, but no runtime observation proves it active;
- `observed`: the runtime emitted in-scope evidence that a verifier accepted;
- `simulated`: a model or sandbox exercised the scenario; it is not physical/live proof;
- `shadow-qualified`: evaluation ran against live inputs without production actuation;
- `controlled`: an explicitly authorized and bounded action was dispatched and the resulting state independently observed;
- `recovery-exercised`: the defined recovery scenario was run, and post-recovery checks passed;
- `independently-assessed`: a named assessor reviewed a stated scope and limitations.

These labels form a maturity vocabulary, not a universal linear score. For example, a capability may be observed but not authorized for actuation. The evidence must state the exact meaning of any result.

### Authority must bind the exact operation

Before privileged dispatch, bind authorization to at least:

- stable organization and actor/principal reference;
- operation and dispatch-attempt identifiers;
- exact target or target-set commitment;
- action/capability identifier and parameter digest;
- policy revision and required constraints;
- expiry and freshness policy;
- replay/uniqueness semantics;
- authority issuer, verification result, and revocation status where supported.

If the target, action, parameters, policy, or expiry changes, prior authorization must not silently transfer. Re-evaluate and obtain a new authorization where required. A Symthaea assessment or confidence value is not authorization.

### Dispatch and reconciliation

- The intent and authorized operation remain immutable after authorization.
- Bind approval to the exact plan digest, target identity, relevant observed-state revision, policy revision, and parameters—not merely to a branch, configuration name, or generic "approved" flag.
- Immediately before dispatch, compare the current target state and resulting plan with the frozen approved operation. If either changed materially, invalidate the authorization and require re-planning and re-authorization; never silently apply a stale plan.
- A saved plan file is executable authority in some infrastructure tools, so its possession alone must not substitute for Luminous' separate actor/operation authorization and freshness checks. See [OpenTofu plan](https://opentofu.org/docs/v1.13/cli/commands/plan/) and [apply](https://opentofu.org/docs/v1.13/cli/commands/apply/) semantics.
- Retries use explicit attempt identity and idempotency semantics; a retry is not silently a second action.
- A timeout is an unknown result until reconciled against the target system.
- Concurrency-sensitive dispatch uses fencing or an equivalent owner-enforced mechanism.
- Command exit status is recorded but never substitutes for a post-action observation.
- Rollback is a separately scoped operation with its own authority and result evidence.
- Unknown, stale, missing, duplicate, or contradictory evidence leads to `unproven` or refusal, never silent success.

### Data sovereignty and federation

A profile must specify, rather than imply:

- where raw telemetry, identifiers, keys, backups, and evidence are stored;
- which classes of data may cross the trust boundary;
- what is redacted, aggregated, or pseudonymized before export;
- who authorizes export and federation;
- retention and deletion behavior;
- how the operator recovers if the remote coordinator is unavailable;
- how an organization leaves the system with usable configuration and evidence.

Content hashes are not automatically privacy-preserving: low-entropy or guessable values can be tested by an observer. Do not publish hashes of sensitive low-entropy values as if a hash were redaction.

### Dependency and supply-chain declarations

For every required component, declare a release/version or immutable revision, a content digest when available, its owner, its security boundary, its supported platforms, and its required permissions. Build provenance should identify the source revision, builder, build definition, dependencies, and output artifact, following the applicable SLSA provenance schema. Provenance describes how an artifact was produced; it does not prove the artifact is benign.

### Failure-domain declaration

Profiles must explicitly test what happens when each dependency becomes unavailable or compromised. Include at least:

- coordinator unavailable;
- local identity/authorization verifier unavailable;
- telemetry source stale or inconsistent;
- time source uncertain or unavailable;
- network partition or peer compromise;
- invalid or revoked signing key;
- storage full or evidence sink unavailable;
- update download interrupted;
- upgrade boots incorrectly;
- restore source or recovery dependency unavailable;
- operator credentials lost or compromised.

For every case, define which functions remain available, which privileged actions are denied, what evidence can still be preserved, and how recovery is performed out of band.

## Validation protocol

A conforming profile is not valid merely because it parses. Validation should happen in distinct layers:

1. **Syntax:** strict schema and type validation; reject unknown critical fields and invalid enums.
2. **Consistency:** reject contradictory settings (for example, remote control enabled while no authentication authority is specified).
3. **Dependency closure:** verify that all required adapters and immutable dependencies are identified.
4. **Policy checks:** reject dangerous defaults such as privileged execution enabled with no authority path.
5. **Evidence contract checks:** validate required identity, freshness, digest, issuer, and limitation fields.
6. **Scenario tests:** run positive and negative test vectors against a simulated or test system.
7. **Exact target qualification:** exercise the stated OS/hardware/adapter combination, not a neighboring environment.
8. **Recovery qualification:** test upgrade rollback, data restore, export/exit, and coordinator-independent recovery.
9. **Independent verification:** where a qualified status depends on a receipt or proof, verify it using a separate verifier implementation.
10. **Release binding:** record the profile digest, platform revision, adapter revisions, test corpus version, runner environment, and produced evidence digests.

Missing prerequisites must produce an explicit blocked/unproven result. An absent test runner, skipped test, missing artifact, unknown schema version, or old successful receipt is not a pass.

## Proposed conformance suite

Keep a small dependency-light corpus that every adapter can run, plus platform-specific integration tests.

| Test family | Representative negative case | Required decision |
|---|---|---|
| Schema | Unknown major schema version | Reject |
| Identity | Receipt belongs to another organization or target | Reject |
| Freshness | Expired or pre-exercise receipt | Unproven / reject |
| Authorization | Missing, revoked, or invalid authority | Deny privileged action |
| Substitution | Target/action/parameters changed after approval | Reject authorization reuse |
| Replay | Same single-use operation dispatched twice | Refuse second dispatch or prove idempotent non-duplication |
| Runtime observation | Command succeeded but target state is not observed | Unproven |
| Recovery | Backup exists but restore was not exercised | Not recovery-qualified |
| Provenance | Artifact digest does not match receipt | Reject |
| Independence | The verifier trusts a producer's boolean instead of checking the receipt | Conformance failure |
| Partition | Remote coordinator unavailable | Preserve local safety/recovery; deny unapproved privileged work |
| Export/exit | Customer cannot retrieve configuration/evidence without provider access | Conformance failure for sovereignty profile |

This suite should be machine-readable when implemented. The table alone is specification, not evidence that tests already exist or pass.

## Standards alignment and limits

Use external guidance as an input, not a certification label:

- [SLSA Build Provenance v1.2](https://slsa.dev/spec/v1.2/build-provenance) for how a software artifact was produced and how consumers can verify that provenance.
- [NIST CSF 2.0 Tiers quick-start guide](https://csrc.nist.gov/pubs/sp/1302/final) for characterizing cybersecurity risk-governance practices, not replacing product-specific tests.
- [NIST IR 8374 Rev. 1](https://csrc.nist.gov/pubs/ir/8374/r1/final) for current ransomware-risk-management outcomes.
- [CISA #StopRansomware Guide](https://www.cisa.gov/stopransomware/ransomware-guide) for least privilege, protected offline backups, and actual restoration exercises.
- [EU Cyber Resilience Act reporting guidance](https://digital-strategy.ec.europa.eu/en/policies/cra-reporting) for current product/security incident reporting context; maintain a separate reviewed applicability assessment for each product and role.

A profile may report a crosswalk to selected framework outcomes. It must not say "compliant" or "certified" just because profile tests passed or a crosswalk exists.

## Implementation order

1. Freeze the minimal profile schema and semantics before building a profile-management UI.
2. Add a strict validator and a small deterministic fixture corpus.
3. Qualify one local NixOS virtual-machine profile in observe/shadow mode.
4. Bind the results to exact source/build/adapter identities.
5. Exercise local boot and recovery with optional coordination unavailable.
6. Add bounded actuation only after the authority and reconciliation tests pass.
7. Add self-hosted multi-node, offline, and federated profiles as independently qualified targets.
8. Publish a compatibility matrix with each release and remove unsupported claims from marketing/docs.

The standard of success is not how many profiles exist; it is whether an operator can determine exactly what a profile permits, what it proves, what it does not prove, and how to recover when its assumptions fail.
