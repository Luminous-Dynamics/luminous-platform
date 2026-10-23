# SRQ Lifecycle Interoperability v1

**Status:** research conformance smoke, not a release qualification.

This work is intentionally narrow: prove that the existing Nixward transaction envelope, Sovereign Boot's actual D0 diagnostic, and Mycelix's shared evidence-plane planned receipt can be consumed together without silently increasing the strength of any claim.

## Why this is the next paradigm shift

Each repository has a useful local contract, but individually valid records do not automatically compose into a valid lifecycle decision. A transaction can be merely planned, an observation can be local and unauthenticated, and an evidence receipt can be synthetic and not run. Combining them must preserve those limits instead of creating a false green result.

The test uses immutable upstream commit pins and exact Git blob IDs rather than branch names or copied-upstream files:

- Nixward transaction fixture: commit  ae60256f0be73138a539ecda048e81056dd14a30; blob c89d5033a35e5c1be1b5c7de0a2436539922a800.
- Mycelix evidence-plane schema: commit b02cefab8474262aef23b61a4ac3811fe92e29bf; blob e2b90a995f70d0a771b04e7f5fc077e1c0394f86.
- Mycelix planned receipt: the same pinned commit; blob d1d3516e2778e0da89013348b16ed73af6e5e8a8.
- Sovereign Boot D0 probe source: commit 7f1c53924b1f0c2ba7b9734d875e50529837f057; blob de2126eefa1c2e41494bb2624a78e46576c39d27.

The workflow checks the checkout SHA and Git blob ID at runtime and computes SHA-256 over the exact raw fixture/observation bytes for the assessment report. The primary assessment also binds the exact lifecycle-interoperability harness commit, validator/test/adversarial-runner Git blob IDs, upstream-lock bytes, and assessment-schema bytes; it refuses to run if those tracked harness files have local staged or unstaged edits. The adversarial runner emits a second JSON assessment with every mutation filter, candidate raw-byte SHA-256, expected/observed rejection, validator exit status, rejection reason, and harness/fixture identities. CI retains both reports as artifacts.

## Executable contract

The benchmark script:

1. Checks and runs the actual pinned Rust D0 probe with cargo fmt and cargo test.
2. Builds a disposable filesystem fixture and captures the probe's actual JSON output; it does not maintain a hand-copied diagnostic output fixture.
3. Checks Nixward transaction fields and Mycelix evidence tier/claim consistency.
4. Emits an assessment containing commit IDs, Git blob IDs, raw-byte SHA-256 values, observed source tiers and explicit release blockers.
5. Runs three original negative controls: forged D0 authentication/qualification; S0-to-S4 claim inflation without source/verifier/gate evidence; and phase-only Nixward promotion without authorization/readback/outcome.
6. Runs a reusable adversarial evidence-mutation corpus. It rejects planned receipts with execution-tier statuses, any passing gate or injected subject identity/hash, nonzero result counts, S3/S4 claims without required independent-verification bindings, S2 source-reproducibility claims without source bytes/rights evidence, and unknown tiers.
7. Confirms the upstream fixture files and probe source remain unmodified.

The additional cross-field checks are deliberately a small safety-critical subset of the pinned Mycelix schema: they test run-state/tier/claim-ceiling consistency and cannot be represented as full schema validation. Every mutation uses disposable output; upstream source bytes and pins remain unchanged.

The validator performs a narrow field/invariant check. It does not implement the full JSON Schema 2020-12 validator from Mycelix, Nixward's full production transaction validator, SRQ's release evaluator, or a cryptographic attestation verifier. Exact pinning is not a substitute for those verifiers.

## Required result for this scenario

The integration assertion must distinguish these outcomes:

- contract_status = PASS means only that the pinned example records are structurally consistent with this narrow interoperability contract.
- release_status = BLOCKED is required because the Nixward record is planned; the Sovereign Boot record is D0_LOCAL_DIAGNOSTIC, unauthenticated and has no target/decision/challenge/policy/artifact/measurement binding; and the Mycelix receipt is S0 / synthetic_only / planned / not_run.
- The harness must never issue SRQ PASS. A separate qualifying-tier scenario evaluator must consume authenticated, source-bound, actually executed evidence before any release claim is considered.

A matching /run/booted-system and /run/current-system path still does not prove that the intended update was authorized, that the booted artifact was measured, or that recovery has been qualified.

## Industry alignment and intended contribution

This is a composition test, not a new attestation or deployment format. It follows the separation of evidence and appraisal described by [RFC 9334 / RATS](https://www.rfc-editor.org/rfc/rfc9334.html), software artifact provenance described by the [in-toto Attestation Framework](https://github.com/in-toto/attestation), and the [SLSA provenance predicate](https://slsa.dev/provenance/v1). For update security, [TUF](https://theupdateframework.github.io/specification/v1.0.37/index.html) already addresses repository threats including rollback, freeze, and mix-and-match attacks. For transactional deployment behavior, [OSTree's atomic-upgrade model](https://ostreedev.github.io/ostree/atomic-upgrades/) explicitly aims to leave the old or new bootable deployment after a crash or power interruption. Existing project wire formats remain authoritative for their own domains.

The competitive opportunity is the cross-layer guarantee: join authorization, transaction state, measured boot/runtime evidence, independent appraisal, rollback outcome, and claim ceiling without allowing a lower layer's "green" status to imply the whole system is safe. This harness does not yet prove that end-to-end guarantee or reproduce TUF/OSTree guarantees on NixOS; future fault-injection work must test these boundaries directly.

The value hypothesis is operational: catch trust-tier escalation and contract drift across repository boundaries before the components are deployed together. The success measure is 100% rejection of the frozen claim-escalation controls, byte-identical upstream inputs at their locked commits, and no false release PASS in the integration corpus.

## Non-claims

- No NixOS build, bootloader, QEMU, TPM, physical reset, power-loss, or production fleet qualification.
- No cryptographic validation of the D0 JSON; the probe explicitly produces non-authenticated diagnostics.
- No Mycelix source-truth, rights, stewardship, independent-reproduction, or domain-validation claim.
- No claim that a Git digest proves authorization or semantic correctness.
- No performance/SOTA claim and no assertion that a workflow being queued or passing on a later commit qualifies the target system.
