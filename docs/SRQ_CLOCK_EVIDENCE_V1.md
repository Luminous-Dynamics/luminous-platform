# SRQ Clock Evidence Model v1

Status: research model; not a production clock or release-qualification result.

Related implementation gap: [issue #104 — model clock rollback and freshness as explicit evidence tiers](https://github.com/Luminous-Dynamics/luminous-platform/issues/104). Related cross-layer profile: [SRQ-1 qualification PR #88](https://github.com/Luminous-Dynamics/luminous-platform/pull/88).

## Paradigm shift: time is evidence, not an unquestioned scalar

A timestamp from `SystemTime` does not by itself establish freshness. Transactionally checking the wall clock immediately before commit reduces a time-of-check/time-of-use window, but it does not make the wall clock authenticated or rollback resistant. Conversely, a monotonic clock gives useful ordering only within a boot and cannot be compared naïvely after reboot.

SRQ should attach an explicit assurance tier to every freshness decision and make the policy's minimum tier non-downgradable:

| Tier | Meaning | What it does not prove |
| --- | --- | --- |
| A0 — host wall clock | Operational timestamp from the host | No rollback resistance or authenticated time |
| A1 — boot monotonic | Monotonic ordering within a known boot instance | No cross-reboot time comparison |
| A2 — persistent high-water mark | Detects a timestamp lower than a previously committed observation | Does not authenticate the first observation or resist restoration of the same storage domain |
| A3 — authenticated evidence | Trusted verifier authenticates a target/challenge/decision-bound freshness response | Depends on trust anchor, verifier, policy, key lifecycle, freshness protocol and device association |

These tiers are not interchangeable. A system requiring A3 must block on A3 unavailability; it must not silently accept A0/A1/A2 instead.

## Executable scope in this change

`tools/srq-clock-evidence-model/` is a dependency-free Rust reference state machine. It models:

- strict minimum-tier policy with no automatic fallback;
- A1 monotonic ticks bound to a boot ID, rejecting regressions and unexplained reboot changes;
- A2 rejection when no prior high-water value exists, when time moves behind the committed high-water value, or when an unauthenticated forward jump exceeds the configured bound;
- A3 issuer, policy, target, challenge, decision, issue-time, expiry and reference-time binding;
- a compare-by-revision commit step that rejects stale appraisals;
- high-water advancement only after the associated authorization is reported committed;
- two deterministic 10,000-case negative corpora for binding mutations and time rollback/forward-jump mutations.

The model's `signature_verified_by_trusted_verifier` field is an abstract input used to test state-machine behavior. It is not a cryptographic verifier and must not be populated directly from untrusted receipt JSON. A real adapter must accept a private, typed verification capability created only after validating the actual signed evidence with a pinned trust policy. Similarly, the model's state is in memory; it does not provide durable storage semantics.

## Transaction invariant for a real adapter

The eventual durable adapter must appraise and consume freshness inside the same atomic transaction as the corresponding decision/receipt, or use a separately specified recoverable protocol with equivalent crash-state guarantees. It must not persist an extreme unauthenticated time observation before authentication succeeds, and an authorization rejection must not advance the accepted-time high-water mark. A real adapter also needs to handle:

- atomic update of time high-water, receipt, decision and one-use challenge;
- trusted source identity and downgrade-resistant policy versioning;
- reboot/boot-instance changes for monotonic clocks;
- forward jumps and long-unavailable clocks without an unsafe fallback;
- process kill, VM reset and physical power loss as separate fault tiers;
- restoration of an old database snapshot, which defeats a high-water mark stored only in that same rollback domain.

For A3, the expected trust anchor, issuer/key identity, nonce lifecycle, target association, challenge expiry, revocation/rotation behavior and verifier time/reference source need their own authenticated policy. A signed timestamp alone is not sufficient freshness if it can be replayed.

## Benchmarks and qualification targets

Targets below are proposed, not results:

- zero accepted rollback/forward-jump cases in a frozen, digest-bound 100,000-mutation corpus;
- zero A0/A1/A2 downgrade accepts when the configured minimum is A3;
- exactly one committed high-water/decision/challenge outcome across 64 concurrent consumers of the same challenge;
- zero persistent mutation after wrong target, stale decision, rejected authentication, expired evidence, or clock-policy mismatch;
- zero ambiguous outcomes across at least 100 process-kill trials at each transaction boundary, with post-commit acknowledgement-loss recovery returning the same decision;
- publish p50/p95/p99 appraisal and commit/recovery latency separately, alongside false accepts/rejects, unavailable-source duration, rollback/forward-jump rejection counts, source/tier/policy identifiers, source/build digests, and raw evidence.

Model tests cannot satisfy the durable-store, QEMU, TPM, or physical-device gates. The first comparative experiment should run a frozen time mutation corpus against the state-machine model and the actual D1 store adapter using the same policy vectors; only later should it introduce VM reboot and controlled clock/RTC faults.

## Industry baseline and intended contribution

Use [RFC 9334 (RATS)](https://www.rfc-editor.org/rfc/rfc9334.html) for the separation of attester evidence, verifier appraisal and relying-party decisions; [RFC 9711 (EAT)](https://www.rfc-editor.org/rfc/rfc9711.html) for token claims and freshness-profile design; [NIST SP 800-193](https://csrc.nist.gov/pubs/sp/800/193/final) for platform resiliency and recovery principles; and [TUF](https://theupdateframework.io/docs/security/) for update metadata rollback/freeze protections. This work should compose those mechanisms, not claim to replace them.

The measurable hypothesis is that SRQ can preserve a traceable, non-downgradable freshness claim from policy through durable decision and post-boot observation, while explaining exactly what each time source cannot establish. It does not claim a new trusted-clock primitive or superiority to an industry implementation.

## Current status

This change supplies the initial model and tests only. The crate has not been compiled in this editing environment; its GitHub workflow is expected to compile and run it, but a queued workflow is not a pass. No clock, anti-rollback, production-store, TPM, VM or physical power-loss qualification is claimed.
