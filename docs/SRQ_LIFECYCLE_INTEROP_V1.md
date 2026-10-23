# SRQ Lifecycle Interoperability v1

**Status:** research conformance smoke, not a release qualification.

This work is intentionally narrow: prove that the existing Nixward transaction envelope, Sovereign Boot's actual D0 diagnostic, and Mycelix's shared evidence-plane planned receipt can be consumed together without silently increasing the strength of any claim.

## Why this is the next paradigm shift

Each repository has a useful local contract, but individually valid records do not automatically compose into a valid lifecycle decision. A transaction can be merely planned, an observation can be local and unauthenticated, and an evidence receipt can be synthetic and not run. Combining them must preserve those limits instead of creating a false green result.

The test uses immutable upstream commit pins and exact Git blob IDs rather than branch names or copied-upstream files:

- Nixward transaction fixture: commit  ae60256f0be73138a539ecda048e81056dd14a30; blob c89d5033a35e5c1be1b5c7de0a2436539922a800.
- Mycelix evidence-plane schema: commit b02cefab8474262aef23b61a4ac3811fe92e29bf; blob e2b90a995f70d0a771b04e7f5fc077e1c0394f86.
- Mycelix planned receipt: the same pinned commit; blob d1d3516e2778e0da89013348b16ed73af6e5e8a8.
- Sovereign Boot D0 probe source: commit 9d183163a0f2bb101e6dd0ad3609afde04f1c5a4; blob 2b05462ab98e90abc085d2834e69dbf2ebae9337.

The workflow checks the checkout SHA and Git blob ID at runtime and computes SHA-256 over the exact raw fixture/observation bytes for the assessment report. The primary assessment also binds the exact lifecycle-interoperability harness commit, validator/test/adversarial-runner Git blob IDs, upstream-lock bytes, and assessment-schema bytes; it refuses to run if those tracked harness files have local staged or unstaged edits. The adversarial runner emits a second JSON assessment with every mutation filter, candidate raw-byte SHA-256, expected/observed rejection, validator exit status, rejection reason, and harness/fixture identities. CI retains both reports as artifacts.

## Executable contract

The benchmark script:

1. Checks and runs the actual pinned Rust D0 probe with cargo fmt and cargo test.
2. Builds a disposable filesystem fixture and captures the probe's actual JSON output; it does not maintain a hand-copied diagnostic output fixture.
3. Checks Nixward transaction fields and Mycelix evidence tier/claim consistency.
4. Emits an assessment containing commit IDs, Git blob IDs, raw-byte SHA-256 values, observed source tiers and explicit release blockers.
5. Runs three original negative controls: forged D0 authentication/qualification; S0-to-S4 claim inflation without source/verifier/gate evidence; and phase-only Nixward promotion without authorization/readback/outcome.
6. Runs a reusable adversarial evidence-mutation corpus. It rejects planned receipts with execution-tier statuses, any passing gate or injected subject identity/hash, nonzero result counts, S3/S4 claims without required independent-verification bindings, S2 source-reproducibility claims without source bytes/rights evidence, and unknown tiers.
7. Accepts two deliberately synthetic positive-control *shapes* (S2/source-reproducible and S4/domain-validated) while asserting the lifecycle assessment itself remains `release_status=BLOCKED` and `claim_ceiling=none`.
8. Confirms the upstream fixture files and probe source remain unmodified.

The additional cross-field checks are deliberately a small safety-critical subset of the pinned Mycelix schema: they test run-state/tier/claim-ceiling consistency and cannot be represented as full schema validation. The positive controls use sentinel URLs, hashes, commit identities and verifier references; they exercise structural acceptance only and are explicitly not real source, execution, independent-verification or domain-validation evidence. Every mutation uses disposable output; upstream source bytes and pins remain unchanged.

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

## Benchmark ladder: targets, not results

These are proposed gates for follow-on work, not results measured by this PR. Start with deterministic attacks and simulated recovery; only graduate to physical/fleet claims when those environments have been exercised.

| Benchmark | Industry reference / fair baseline | Target |
|---|---|---|
| Claim-ceiling monotonicity | Mycelix receipt rules; RFC 9334 evidence appraisal | Reject 100% of pre-registered escalations; zero false release PASS; zero false rejects across explicit valid controls. |
| Source and report identity | Git object IDs plus raw-byte SHA-256; in-toto subject binding | Reject 100% of wrong-commit, wrong-blob, one-byte-drift, symlink-substitution, and artifact/subject mismatch cases. |
| Freshness and replay (future D1 verifier) | RATS freshness requirements and TUF rollback/freeze/mix-and-match threat classes | Reject every stale/replayed nonce, wrong target, wrong policy digest, and wrong artifact binding; under 64 concurrent submissions of one single-use challenge, exactly one acceptance. |
| Crash/rollback consistency | [OSTree atomic-upgrade behavior](https://ostreedev.github.io/ostree/atomic-upgrades/) and NixOS generations | Inject 100 process kills at each durable transition boundary, then 1,000 VM reset/restore trials; observe old or new known-good state or an explicitly blocked recovery—never silent partial promotion. Report QEMU and physical-device outcomes separately. |
| Independent appraisal | in-toto authenticated attestations, SLSA provenance, RATS verifier/relying-party split | Reject invalid signatures and policy/target mismatches; any verifier disagreement blocks promotion; record verifier implementation identity and replayed commit. |
| Fleet behavior | TUF client/update safety plus atomic deployment mechanisms | Exercise 100, 1,000, 10,000, then 100,000 simulated targets; report p50/p95/p99 detection, promotion and rollback latency, throughput, peak memory, retries, and maximum failure blast radius. No fleet-safety claim from simulation alone. |

Benchmark integrity requirements: freeze and hash the attack corpus before running; pin baseline revisions; include both valid and invalid controls; report exact hardware/runtime and resource budgets; retain raw per-case outcomes and tool identities; separate test-harness self-consistency from externally verified evidence. A high rejection percentage is meaningless if valid controls are also rejected, and speed is not a substitute for recovery correctness.

## Non-claims

- No NixOS build, bootloader, QEMU, TPM, physical reset, power-loss, or production fleet qualification.
- No cryptographic validation of the D0 JSON; the probe explicitly produces non-authenticated diagnostics.
- No Mycelix source-truth, rights, stewardship, independent-reproduction, or domain-validation claim.
- No claim that a Git digest proves authorization or semantic correctness.
- No performance/SOTA claim and no assertion that a workflow being queued or passing on a later commit qualifies the target system.
