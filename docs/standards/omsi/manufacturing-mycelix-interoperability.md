# OMSI × Mycelix Manufacturing Evidence Interoperability

**Status:** proposed integration contract; no cross-repository runtime bridge is implemented by this document  
**OMSI implementation:** [manufacturing semantic validator and replay envelope](https://github.com/Luminous-Dynamics/luminous-platform/tree/feat/omsi-manufacturing-rust-validator/tools/omsi-manufacturing-validator)  
**Mycelix counterpart proposal:** [OMSI interoperability proposal](https://github.com/Luminous-Dynamics/mycelix/blob/feat/omsi-evidence-interoperability/mycelix-workspace/OMSI_INTEROPERABILITY_PROPOSAL.md)

## Decision

**Integrate the systems through a versioned adapter and evidence contract; do not merge their responsibilities or make the OMSI validator depend on a live Holochain conductor.**

OMSI and Mycelix solve adjacent but different problems:

- **OMSI** defines manufacturing data semantics, bounded deterministic interpretation, conformance vectors, rule-specific reports, and replay checks over a precisely identified artifact.
- **Mycelix** supplies an agent-centric network of domain hApps, DID and verifiable-credential infrastructure, epistemic claim representation, trust/reputation facilities, cross-cluster routing, governance, and potential multi-party receipt storage.

Keep the OMSI validator usable offline, by a conventional factory, and by an independent verifier with no Mycelix membership. Mycelix can wrap OMSI evidence with identity, revocation, policy and collaboration capabilities. A valid OMSI report must not imply that a Mycelix trust score, community vote, credential, or DHT record proves the physical measurement is true.

## What exists in the repositories today

These are source-tree observations, not a claim that the integration is tested or deployed.

- The [Mycelix root README](https://github.com/Luminous-Dynamics/mycelix/blob/main/README.md) describes a Holochain-based CivOS with separate domain clusters and explicitly distinguishes project maturity from test counts.
- The [Mycelix architecture](https://github.com/Luminous-Dynamics/mycelix/blob/main/mycelix-workspace/ARCHITECTURE.md) describes cluster bridge zomes and a shared routing registry. It also records that not all bridge zomes have migrated to that registry.
- [Fabrication verification integrity code](https://github.com/Luminous-Dynamics/mycelix/blob/main/mycelix-workspace/happs/fabrication/zomes/verification/integrity/src/lib.rs) defines public `DesignVerification`, `SafetyClaim`, and `VerificationRequest` entry types. The [prints integrity zome](https://github.com/Luminous-Dynamics/mycelix/blob/main/mycelix-workspace/happs/fabrication/zomes/prints/integrity/src/lib.rs) defines public print-job/record and monitoring entry types. This is a plausible first integration surface for a pilot around a printed part, design verification, materials and machine evidence.
- The [Mycelix identity VC integrity zome](https://github.com/Luminous-Dynamics/mycelix/blob/main/mycelix-identity/zomes/verifiable_credential/integrity/src/lib.rs) models issuer, validity, credential status and proof fields. A real adapter must still call a supported verification path and test its proof, issuer, expiry and revocation behavior; field presence alone is not verification.
- The shared [routing registry](https://github.com/Luminous-Dynamics/mycelix/blob/main/mycelix-workspace/crates/mycelix-bridge-common/src/routing_registry.rs) includes manufacturing and supply-chain zome allowlist constants.
- **Packaging blocker:** the GitHub contents API returns `/srv/luminous-dynamics/mycelix-workspace/mycelix-manufacturing` as the contents of `mycelix-workspace/happs/manufacturing`. That is an absolute machine-local symlink target, not a portable repository path. Resolve the intended tracked path/submodule/pinned source and verify clean-checkout packaging before attempting to build an OMSI adapter against that Manufacturing role. Do not assume it is deployable from the current public checkout.

Sources above were inspected on 2026-10-15; paths and maturity can change. Source inspection does not establish successful builds, multi-agent behavior, security audit, or production deployment.

## Integration topology

```text
MES / QMS / metrology / fabrication exporter
                    |
                    v
      OMSI profile adapter (read-only)
                    |
                    v
     OMSI Rust semantic evaluator
       - schema/profile version
       - scope and source-instance checks
       - decimal/time/calibration checks
       - named comparison rule
                    |
                    v
      OMSI raw-byte replay envelope
      (exact input digest + report)
                    |
        local independent verification
                    |
                    v
    Mycelix interop adapter / coordinator
       - allowlisted operation
       - resolve actor and issuer credentials
       - apply authorization and disclosure policy
       - preserve OMSI outcome unchanged
                    |
                    v
       Mycelix evidence receipt / link
       - minimal metadata + artifact digest
       - issuer and credential references
       - revocation-check receipt/time
       - access-classification + source references
                    |
                    v
  Knowledge / Supplychain / Fabrication / Identity
  (each keeps its own domain responsibility)
```

The diagram is a target architecture. No adapter or Holochain entry type is implemented in the OMSI validator by this document.

## Responsibility matrix

| Concern | OMSI | Mycelix | Boundary |
|---|---|---|---|
| Manufacturing profile meaning and versions | Owner | Consumes pinned profile | No divergent semantic clones |
| Deterministic per-record validation | Owner | Replays or verifies via adapter | No online trust lookup inside the semantic rule |
| Input/report digest and replay consistency | Owner | Verifies and references envelope | Digest is not source authentication |
| Issuer/operator/lab DID and credentials | Consumes credential references if needed | Identity and credential services | Valid credential is not proof that measurement is accurate |
| Credential expiry/revocation | Declares evidence requirements | Resolves current status and records when/how checked | Historical decisions must preserve the status evidence used then |
| Epistemic classification | Emits domain evidence/result states | May annotate claims using the Knowledge/E-N/M system | E/N/M is not interchangeable with OMSI's assurance dimensions |
| Trust / reputation | Does not turn trust into a measurement fact | May prioritize review or inform policy under an explicit rule | Never use MATL as a calibration or quality acceptance threshold |
| Manufacturing/fabrication record references | Defines subject/profile/rule/evidence references | Owns Holochain record/link and collaboration workflow | Do not duplicate MES source-of-record responsibilities |
| Governance of shared policy | Publishes versioned rules and changes | Governance may coordinate endorsement/change approval | Governance chooses policy; it does not rewrite past calculations |
| Machine/process control or product release | Explicitly out of scope | No implied authority | Separate safety case and authorization program required |

## Proposed minimum receipt

A future Mycelix evidence receipt should be deliberately small. Exact Rust/HDI field names remain open until schema review.

| Field | Meaning |
|---|---|
| `receipt_contract` | Stable, versioned identifier for the receipt schema |
| `subject_ref` | Opaque or scoped reference to a part, lot, print job or observation; avoid exposing factory identifiers unnecessarily |
| `profile_id` / `profile_version` | Exact OMSI semantic contract used |
| `rule_id` / `rule_version` | Exact interpretation rule; never infer rule from current defaults |
| `input_digest` | Digest algorithm, digest, byte length and scope from the envelope |
| `envelope_digest` | Digest of the preserved envelope artifact using explicitly named byte semantics |
| `validator_identity` / `version` | Evaluator that produced the diagnostic report |
| `outcome` | OMSI outcome copied without reinterpretation |
| `evidence_dimensions` | Separate, typed state fields; no composite trust score replacing blockers |
| `issuer_did` / `credential_ref` | Claiming agent and credential identifier/reference |
| `credential_status_observed_at` | When revocation/status evidence was checked; not a timeless guarantee |
| `evidence_pointer` | Authorized locator or access-controlled retrieval hint, if a pointer is safe to publish |
| `visibility_class` / `retention_class` | Policy for publication, retention and disclosure |
| `supersedes` / `disputes` | Links to prior or contradictory receipts without destructive overwrite |

An evidence receipt is an index and attribution record, not the underlying high-rate sensor stream. The source claim, OMSI derived result, reviewer attestation, and organizational release decision must remain separate artifacts.

## Security and semantics rules

1. **Evidence and reputation are orthogonal.** Mycelix E/N/M describes empirical verifiability, normative agreement and materiality. MATL/reputation describes aspects of agents or submissions under a particular trust model. Neither may rewrite the OMSI measurement result.
2. **No consensus laundering.** A quorum of agents agreeing on a result does not make an expired calibration valid or change a deterministic decimal comparison. Multi-party reviews are distinct attestations and may help authorize policy decisions; preserve the underlying conflicts.
3. **No source-of-truth migration by accident.** OMSI stores no live PLC/SCADA control path. High-frequency telemetry and proprietary designs stay in the factory/source system or an authorized private store. Publish only a minimal, approved receipt; even hashes and opaque IDs can leak linkage or business activity.
4. **Deterministic Holochain validation.** Holochain integrity validation must be deterministic and must not call mutable external APIs or depend on current wall-clock state. Keep online issuer/revocation lookup in an appropriate coordinator/preflight verification stage; record the verification evidence and timestamp. Integrity validation should check self-contained structure and deterministic dependencies.
5. **Revocation is time-dependent.** A verifier distinguishes “status checked at time T” from “credential is currently valid.” New checks do not silently rewrite historical receipts.
6. **Privacy before publication.** Existing Fabrication print and verification entries are declared public. The integration must threat-model leakage and avoid putting raw measurements, proprietary drawings, sensitive operator information, or tenant IDs into public DHT entries.
7. **No reliance on a hash alone.** The current OMSI envelope is unsigned. For authenticated exchange, evaluate W3C Verifiable Credentials for issuer claims and a recognized attestation envelope such as in-toto/DSSE, with explicit trust roots, key rotation and revocation policy. Mycelix receipt signatures and Holochain author provenance do not, alone, prove the industrial source data is true.
8. **No high-rate replication.** Do not write each sensor sample or anomaly at machine sampling frequency to the DHT. Capture bounded windows/summaries and evidence artifacts under an explicit data-retention and disclosure policy.
9. **No implicit authorization.** A validated report cannot authorize product release, lot disposition, machine shutdown, parameter changes, or physical actuation. Any future workflow needs its own approver roles, hazard analysis, audit trail and safety case.

Holochain's own [validation guidance](https://developer.holochain.org/build/validation/) emphasizes deterministic validation results; W3C's [Verifiable Credentials Data Model 2.0](https://www.w3.org/TR/2025/REC-vc-data-model-2.0-20250515/) provides a standard issuer/holder/verifier model; the [in-toto Attestation Framework](https://github.com/in-toto/attestation/blob/main/spec/README.md) separates subject statement, predicate, envelope and bundle.

## Recommended pilot

Use one narrow, read-only workflow for a printed component:

1. An authorized Fabrication exporter supplies a synthetic print record, material passport reference, printer/instrument references, applicable calibration evidence and the exact specification revision.
2. The OMSI validator emits its existing semantic report and raw-byte replay envelope. Out-of-scope or missing evidence is kept explicit.
3. A local verifier independently checks the exact artifact and replays the report before any Mycelix publication.
4. Mycelix Identity verification resolves the submitting agent's and relevant lab/inspector's credential as required by policy; status is recorded with the time and method of verification.
5. A minimal receipt is recorded or linked through the allowlisted Fabrication/Knowledge/Supplychain path after the route and privacy review. The receipt points to a controlled artifact and does not replicate the raw telemetry.
6. A second agent verifies the receipt/envelope and reproduces the result. Add disagreement, expired/revoked credential, cross-tenant, altered input, altered report, missing revision, stale/partial snapshot and unauthorized-route negative tests.
7. Human review remains responsible for any actual product-release decision.

### Ordered gates

- **Gate 0 — Packaging:** resolve the absolute `happs/manufacturing` target and verify the actual Manufacturing/Fabrication hApp clean-checkout build path.
- **Gate 1 — Contract:** freeze the receipt fields, schema versioning, digest scopes, profile/rule identities, privacy classes and non-authority semantics.
- **Gate 2 — Local adapter:** no conductor/network dependency; validate the same synthetic fixture and reject tampering before publication.
- **Gate 3 — Identity and DHT receipt:** verify issuer/subject authorization and revocation behavior; add one allowlisted coordinator path and deterministic integrity validation.
- **Gate 4 — Multi-agent qualification:** execute Tryorama/sweettest or equivalent integration tests across at least two agents, including conflicts, missing DHT dependencies, replay, revocation, route restrictions and privacy tests.
- **Gate 5 — Domain review:** manufacturing-quality/metrology and security reviewers examine the rule and threat model; passing software tests alone is not evidence of measurement validity or factory readiness.

No gate should be bypassed merely because unit tests pass or a CI job is queued.

## Research and cross-repository references

- [OMSI semantic validator and replay envelope](https://github.com/Luminous-Dynamics/luminous-platform/tree/feat/omsi-manufacturing-rust-validator/tools/omsi-manufacturing-validator)
- [OMSI evidence-plane roadmap](https://github.com/Luminous-Dynamics/luminous-platform/blob/feat/omsi-manufacturing-rust-validator/docs/standards/omsi/manufacturing-evidence-plane-roadmap.md)
- [Mycelix architecture and bridge model](https://github.com/Luminous-Dynamics/mycelix/blob/main/mycelix-workspace/ARCHITECTURE.md)
- [Mycelix Fabrication integration guide](https://github.com/Luminous-Dynamics/mycelix/blob/main/mycelix-workspace/happs/fabrication/docs/INTEGRATION_GUIDE.md)
- [Mycelix Fabrication verification entry types](https://github.com/Luminous-Dynamics/mycelix/blob/main/mycelix-workspace/happs/fabrication/zomes/verification/integrity/src/lib.rs)
- [Mycelix Identity VC entry type](https://github.com/Luminous-Dynamics/mycelix/blob/main/mycelix-identity/zomes/verifiable_credential/integrity/src/lib.rs)
- [NIST Digital Thread for Manufacturing](https://www.nist.gov/programs-projects/digital-thread-manufacturing)
- [ISO 23952:2020 QIF](https://www.iso.org/standard/77461.html)
- [W3C Verifiable Credentials Data Model 2.0](https://www.w3.org/TR/2025/REC-vc-data-model-2.0-20250515/)
- [Holochain validation guidance](https://developer.holochain.org/build/validation/)
- [in-toto Attestation Framework](https://github.com/in-toto/attestation/blob/main/spec/README.md)

This is a proposed architecture. It does not state that OMSI and Mycelix are currently integrated, that the receipt type exists, or that either repository has been independently security-audited for this use.


## Shared benchmark gates

Use the [OMSI benchmark scorecard](./benchmark-scorecard-0.1.md) for the shared cross-repository acceptance framework. The first gates are semantic-vector exact match, zero unexpected acceptance in negative controls, replay mutation detection, and no silent field loss. Performance targets (semantic evaluation p95 ≤ 5 ms for bounded synthetic inputs and replay verification p95 ≤ 20 ms) remain provisional until a 1,000-case workload and a pinned reference machine exist.

Mycelix-specific acceptance adds at least two-agent receipt verification, credential-expiry/revocation negative tests, route-denial tests, and zero prohibited sensitive fields in public receipts. These are not satisfied by OMSI's local benchmark harness; they require a later adapter plus multi-agent integration tests. A passing throughput result cannot offset any scope, privacy, source-authenticity, or authority-boundary failure.
