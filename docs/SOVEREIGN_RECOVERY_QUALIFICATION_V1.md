# Sovereign Recovery Qualification (SRQ-1)

**Status: proposal and test-intent corpus only. No runtime qualification has been performed by this document.**

## Purpose

SRQ-1 defines a shared acceptance contract for Luminous Platform, Sovereign Ops, and Sovereign Boot. It turns the phrase “the machine recovered safely” into independently reviewable claims over a complete lifecycle:

**source and build identity → authorized update metadata → operator intent and authorization → installed generation → boot and recovery outcome → fresh observation → promotion or quarantine decision.**

This is an integration benchmark, not a new bootloader, package manager, remote-attestation protocol, or replacement for established standards. The first deliverable is the frozen adversarial scenario corpus at `benchmarks/srq-v1/scenarios.json`. The corpus is intended to anchor future executable conformance tests without waiting for CI runner availability.

## Architectural shift: qualify transitions, not green dashboards

A green workflow or a well-formed receipt does not prove the running machine is in the intended state. SRQ-1 follows five rules:

1. **Separate safety domains.** Missing animation, telemetry, or remote dashboards must not prevent a host from booting or entering recovery. Unauthorized installation, policy promotion, privilege changes, and claims of verified state must fail closed.
2. **Prove the transition.** Bind the requested target, approved policy, artifact digest, previous and resulting generations, verifier challenge, and observations to one explicit decision.
3. **Make unknown a real state.** Use PASS, FAIL, NOT_RUN, BLOCKED, and NOT_APPLICABLE as distinct results. Unknown or missing evidence is not PASS. A queued job is NOT_RUN.
4. **Make recovery independently verifiable.** A rollback command returning zero is not sufficient: record the generation actually selected after restart and compare it with the intended known-good state.
5. **Keep claims bounded.** A hash gives byte identity, not producer identity. A signature is useful only with a defined algorithm/key binding and a verifier. Software receipts do not establish hardware-rooted measured boot unless that separate path is implemented and qualified.

This resolves a dangerous ambiguity in “fail-open”: **fail open for non-authoritative presentation and monitoring dependencies; fail closed for authority, trust, update promotion, and security claims.** An availability fallback must never turn into an authorization bypass.

## Layer contracts

### Sovereign Boot

The renderer is optional to host boot and recovery. Renderer, GPU, display, theme, or receipt-decoration failures may remove decoration; they must not stop the system from reaching its normal target or recovery path. A missing or invalid lifecycle receipt may not be portrayed as proof that recovery succeeded. Static/reduced motion and deterministic fallback output must be independently testable.

Track visual performance separately from host safety. The existing [Sovereign Boot issue #9](https://github.com/Luminous-Dynamics/sovereign-boot/issues/9) proposes a QEMU p95 stop-request-to-DRM-release target of 750 ms. SRQ preserves that explicit proposal; it is **not** a measured result or a physical-hardware guarantee. QEMU, software Vulkan, physical GPU, physical display, and secure-boot results remain separate evidence classes.

### Luminous Platform

Every update decision must bind the artifact actually staged, the authorized metadata/policy, the target identity, and the generation actually activated. Interrupted activation must recover to one complete generation, never a hybrid. If offline recovery is claimed, the test must disable the network and demonstrate that the local recovery inputs are verified. If no authorized recovery artifact is available, the correct result is explicit degraded/maintenance state, not a conveniently invented success.

A challenge-bound receipt must be checked alongside normal conformance evidence. Production promotion requires durable, atomic challenge consumption joined to the promotion transaction; an in-memory replay guard alone does not qualify after process restart or under concurrent workers. Verifier time, attestation trust, and signer/device identity must remain explicit dependencies rather than fields asserted by the receipt itself.

### Sovereign Ops

Ops intents are proposals until the separately defined authorization conditions are satisfied. Missing, mismatched, expired, or target-ambiguous authorization may not trigger host mutation. A missing cryptographic integration must produce a typed error or explicitly unsigned observation—not placeholder proof or signature bytes. Fleet aggregation must preserve per-target failures, unreachable hosts, and partial progress rather than letting successful nodes hide an unverified node.

## Benchmark corpus and execution tiers

The scenario IDs and required evidence are in `benchmarks/srq-v1/scenarios.json`. It covers optional renderer failure, DRM handoff, reduced motion, network isolation, missing operator approval, absent proof/signing implementations, invalid artifacts, rollback and freeze attacks, interrupted activation, replay/freshness, verifier restarts, concurrent promotion, byte mutation, offline recovery, missing recovery artifacts, corrupt receipts, clock uncertainty, partial fleets, and absent or wrong-head CI evidence.

Execute the corpus in ordered tiers and label each result:

- **Static contract:** schema, manifest, generated-source parity, workflow-graph, and policy-shape checks.
- **Deterministic model tests:** mutation, replay, rollback, transaction, and concurrency tests using frozen fixtures.
- **Disposable QEMU/NixOS VM:** boot targets, restart, power-cut simulation, handoff, and offline recovery. Never modify the CI host bootloader or EFI variables.
- **Physical hardware:** firmware/TPM/secure-boot claims, physical-display behavior, real GPU/driver behavior, and device-specific recovery. A VM result cannot stand in for this tier.

Each receipt must bind at least the scenario/corpus digest, source commit, actual build/checkout commit, artifact and relevant metadata digests, test tier and environment, observed pre/post generation, raw test outcome, timing distribution, evidence artifact hashes, and explicit non-claims. Missing evidence is recorded as missing. Do not silently substitute a PR merge SHA for the reviewed head SHA.

## Result receipts: one schema, five honest outcomes

`benchmarks/srq-v1/result-receipt-v1.schema.json` defines a versioned record for one scenario observation. It keeps four different identities separate: expected source SHA, workflow/event SHA, actual checked-out source SHA, and artifact digest. The runner must not silently relabel one as another.

| Status | Meaning | Minimum decision rule |
|---|---|---|
| `PASS` | The declared check actually ran and met its specific acceptance criteria | Expected source equals actual checkout; execution is recorded; exit code is zero; at least one hash-linked artifact exists |
| `FAIL` | The check ran and its acceptance criteria were not met | Preserve the failure reason and available evidence; do not convert a failed test into success because artifacts uploaded |
| `NOT_RUN` | The scenario did not execute | Actual checkout SHA remains null; execution and exit code remain unasserted/null |
| `BLOCKED` | A prerequisite prevented a valid determination | Record the missing prerequisite and what did/did not execute; never treat the blocker as a pass |
| `NOT_APPLICABLE` | A documented reason makes this scenario inapplicable for this specific target | Explicit reason required; this does not satisfy another required release gate |

The companion `validate-result.sh` checks SRQ's core status/provenance invariants, and the example fixtures plus negative controls prove that a mismatched source head, a claimed pass without evidence, invalid digest shape, or a `NOT_RUN` record with executed steps is rejected. This is intentionally a reference-subset validator—not a full JSON Schema engine, artifact authenticator, or runtime qualification tool.

**Do not aggregate statuses into a single green score.** One required `FAIL`, `NOT_RUN`, or `BLOCKED` keeps the affected release gate unqualified. `NOT_APPLICABLE` needs scope-owner justification and cannot be used to make a failed or missing test disappear.

## Proposed benchmark gates

These are **qualification targets**, not achieved measurements.

| Dimension | Proposed gate | Why it matters |
|---|---|---|
| Unauthorized state change | Zero unauthorized promotions across the frozen release-gate corpus and at least 100,000 deterministic adversarial mutations before a production security claim | A single bypass is a failure; aggregate averages conceal it |
| Replay and stale evidence | Zero acceptance of expired, replayed, wrong-target, wrong-policy, wrong-generation, or mutated challenge-bound evidence | Freshness and byte binding are decision inputs, not decorative telemetry |
| Concurrent challenge use | For 64 concurrent attempts with the same challenge, exactly one transaction may consume it and promote; repeat across crash/restart boundaries | Prevents race and durability defects from invalidating single-threaded tests |
| Interrupted activation | For every declared commit checkpoint, the system recovers to the previous complete or next complete generation; never a hybrid. Exercise at least 100 trials per checkpoint before a production reliability claim | Tests the recovery boundary instead of only successful install |
| Boot independence | In 1,000 disposable VM trials with the optional renderer absent or faulted, zero renderer-caused failures to reach the defined boot/recovery target | Enforces the architectural requirement that decoration is not a boot dependency |
| DRM handoff | Preserve the existing issue #9 proposed p95 <= 750 ms QEMU target; publish physical-device results separately | Separates release latency from visual frame-rate claims |
| Recovery time and data loss | Publish p50/p95/p99 time to known-good operation and retained-state/data-loss measurements for each supported device class; set a stricter SLO only after a reproducible baseline is captured | Avoids inventing a universal recovery-time claim across unlike hardware |
| Fleet integrity | Test 1, 10, 100, and 1,000 synthetic targets; no lost per-target failures, cross-target receipt reuse, or aggregation of unreachable as compliant; report p50/p95/p99 reconciliation time and memory | Measures operational usefulness without disguising partial results |
| Qualification integrity | All required gates must be PASS on the exact reviewed head with present evidence; otherwise release status is NOT_QUALIFIED | A green badge is not a substitute for an evidence inventory |

The 100,000 mutations, 64-way concurrency, 100 trials per checkpoint, and 1,000 VM trials are proposed workload targets. They do not imply statistical proof of zero risk. Record the generator version, seed, corpus digest, number of trials, false accepts/rejects, and untested fault classes.

## Industry baselines: compose, do not reimplement

- **NIST SP 800-193, Platform Firmware Resiliency Guidelines**: use its protect/detect/recover framing for firmware resilience and separate firmware-level qualification from OS recovery. https://csrc.nist.gov/pubs/sp/800/193/final
- **The Update Framework (TUF)**: use established metadata roles and verification behavior for rollback, freeze, mix-and-match, and key-compromise threat models. SRQ tests Luminous integration; it should not create a bespoke replacement for TUF security semantics. https://theupdateframework.io/docs/security/
- **SLSA provenance**: bind build inputs and trusted builder claims to artifact verification. Build provenance describes how an artifact was produced; it does not by itself prove which generation is currently running on a host. Use the current specification rather than pinning an obsolete level description. https://slsa.dev/spec/v1.2/
- **IETF RFC 9334 (RATS)**: use the Attester/Verifier/Relying Party vocabulary and treat freshness as an explicit verifier policy. Freshness narrows replay windows but does not eliminate all races or make untrusted clocks authoritative. https://www.rfc-editor.org/rfc/rfc9334.html
- **NixOS generation rollback**: use the existing boot-menu generation mechanism as a functional baseline, then measure whether the intended generation was actually selected and service returned. https://nixos.org/manual/nixos/stable/
- **CIS Benchmarks**: use relevant OS configuration guidance as a hardening baseline rather than inventing generic settings that conflict with the deployed OS. https://www.cisecurity.org/cis-benchmarks

The distinctive engineering opportunity is the **cross-layer join**: no single adjacent tool establishes the complete chain from authorized artifact and intent through the booted generation and freshly verified recovery outcome. SRQ should prove that join with reproducible adversarial cases—not claim superiority by feature count.

## Comparative benchmark plan: measurable value over the baseline

SRQ-1 should not be marketed as a replacement for these systems. It composes their relevant strengths and measures the integration gap each leaves to the system owner.

| Baseline | What it already does | SRQ integration test | Evidence needed before claiming added value |
|---|---|---|---|
| NixOS generations | Selects an earlier or newer system configuration at boot | Update, force interruption at each activation checkpoint, reboot, and verify the generation that actually came up | Old-or-new complete-generation result, measured recovery time, retained data/state |
| OSTree atomic deployments / systemd-sysupdate A/B | Supports atomic deployment or multi-version resource updates | Run matching power-loss, partial artifact, and fallback scenarios with the selected baseline and Luminous path | Comparable device, same fault injection points, artifact hashes, failure/recovery distributions |
| TUF | Protects update metadata and software selection against rollback, freeze, mix-and-match, and key-compromise cases | Reuse TUF threat cases and conformance fixtures; test Luminous's policy/target/generation binding around the updater | Zero unauthorized acceptances, explicit metadata errors, no reimplementation of TUF cryptographic policy |
| SLSA / in-toto provenance | Describes artifact source/build process and provides verifiable attestations | Verify that the selected artifact satisfies an expected source, builder, and build definition, then independently verify the resulting deployed generation | Attestation verification result plus separate runtime generation observation |
| RATS / Keylime-style measured boot | Defines remote-attestation roles and, in Keylime's case, TPM-based boot/runtime integrity monitoring | Keep freshness/replay tests separate from hardware-rooted attestation tests; bind the two only through an explicitly qualified adapter | Freshness result, verifier policy, attester evidence, trust-anchor chain, device/firmware-specific test evidence |
| Batfish differential reachability | Analyzes reachability changes between network snapshots and exposes unintended effects | Use it as the pre-deployment network-analysis baseline; separately exercise live effective-policy observation and fresh challenge-bound receipt acceptance | Matched network snapshots, differential result, observed effective-policy digest, and promotion decision |
| NIST SP 800-193 | Frames firmware resilience as protection, detection, and recovery | Run firmware-level faults only on supported hardware; keep OS rollback and firmware recovery claims separate | Hardware/firmware tuple, recovery outcome, trust-root and firmware evidence |
| CIS Benchmarks | Supplies established OS configuration hardening guidance | Map applicable controls to the actual NixOS profile rather than creating a conflicting generic score | Versioned control mapping, exceptions, independent configuration assessment |

### How to make a defensible “better than baseline” claim

1. Freeze the threat model and inputs. Run baseline and Luminous paths against the same deterministic fault corpus. Do not compare a unit test to a hardware test or a software receipt to TPM-backed attestation.
2. Report safety first: false accepts and unauthorized promotions are release blockers, not trade-offs to average against speed. Report false rejects separately.
3. Report distributions, not a headline average: p50/p95/p99 latency, recovery time, host count, concurrency, memory/CPU, and failed/unreachable targets.
4. Make the integration benefit falsifiable: SRQ should demonstrate that a fresh, authorized, artifact-bound decision corresponds to the generation actually selected after restart, with complete failure evidence. It must show this on the same cases where the baseline mechanism alone only supplies a narrower property.
5. Publish gaps. If a firmware, TPM, key-management, or physical-power-loss property is not implemented or tested, it remains `NOT_QUALIFIED`; a richer JSON receipt cannot close that gap.

The machine-readable reference list is `benchmarks/srq-v1/industry-baselines.tsv`. It records comparison scope and deliberately avoids unsupported overall superiority rankings.

## Implementation order

1. Land and freeze this proposal plus the scenario corpus; validate its structure independently of CI.
2. Map each scenario to an existing executable test, a missing test, or a test blocked on a real trust/hardware primitive. Do not mark an unimplemented path as covered.
3. Start with deterministic negative controls and VM tests, then durable nonce/promotion transaction tests and crash injection.
4. Produce a common receipt schema and aggregator that preserves claim ceiling, exact SHAs, artifacts, and NOT_RUN states.
5. Only after stable baselines, publish comparisons against NixOS rollback alone, TUF metadata checks, SLSA artifact provenance, and RATS-style freshness boundaries. Publish raw results and test hardware; never compare unlike test tiers as equivalent.

## Current qualification status

**NOT_QUALIFIED / PROPOSAL_ONLY.** This document and corpus define future work. They do not show that boot, update, recovery, fleet operations, secure boot, TPM measurement, cryptographic proofs, or physical hardware have passed. The separate open PRs and issues remain subject to their own exact-head evidence.