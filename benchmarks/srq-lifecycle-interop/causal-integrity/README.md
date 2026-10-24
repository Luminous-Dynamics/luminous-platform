# SRQ transaction causal-integrity prototype

**Status: research prototype. A passing run is not release qualification.** This is a standard-library Python test harness for an explicitly normalized, synthetic transaction envelope. It does not modify the Nixward wire format or participate in the existing lifecycle release evaluator. The production release decision remains blocked.

## Why this slice exists

The existing phase/presence matrix checks whether required records exist for a phase. That is necessary, but record presence alone cannot prove that the records describe the same operation. This prototype tests the next contract layer: references and identities must form one causal chain from validation and authorization through snapshot, application, verification, outcome, promotion, and recovery.

## Run locally

From the repository root:

    python -m unittest discover -s benchmarks/srq-lifecycle-interop/causal-integrity -p 'test_*.py' -v
    python benchmarks/srq-lifecycle-interop/causal-integrity/causal_integrity.py --self-test --output /tmp/srq-causal-report.json

The standalone workflow runs the same unit tests and frozen synthetic corpus on the exact pull-request commit, then retains a JSON artifact containing the protocol and runner source digests, commit SHA, every candidate digest, per-case outcome, errors, and timing observation.

## What the candidate checker verifies

- The root transaction, target, plan, explicit artifact, and policy identities have the declared shape; each event repeats the transaction, target, plan, and artifact identities. Validation, authorization, and application must bind the same artifact digest.
- Event sequence numbers, previous-event digests, payload digests, and event-envelope digests are internally consistent.
- Validation and authorization bind the root plan and target; authorization also refers to the declared policy.
- Application references the preceding authorization and snapshot; verification references the application and agrees with its expected post-state digest and generation.
- A successful outcome references passing verification, and promotion references that same outcome and verification.
- Recovery references the failed event and original snapshot and declares a restored-state digest matching the snapshot pre-state.
- The candidate shape is closed, so extra release-status or claim-ceiling fields cannot be smuggled into this normalized envelope.

The frozen corpus includes 29 negative cases and three positive controls (32 cases total). The exact mutation operations live in causal-integrity-mutations-v1.json; its raw SHA-256 is pinned in the protocol and included in every report, and the runner checks the pin before executing cases. The expected target is zero false accepts, zero false rejects, zero unexpected errors, and no release/claim escalation. The positive controls intentionally cover success, terminal failure, and failure followed by recovery. Synthetic acceptance always remains release_status=BLOCKED and claim_ceiling=none.

## Industry comparison: compose existing guarantees; do not reinvent them

- [TUF](https://theupdateframework.github.io/specification/) already addresses authenticated update metadata, rollback, freeze, and mix-and-match threats. It is the update-security baseline, not a substitute for observing whether a particular host reached the intended post-state.
- [RFC 9334 / RATS](https://www.rfc-editor.org/rfc/rfc9334.html) separates evidence production, verifier appraisal, and relying-party decisions, and discusses freshness strategies. This prototype does not implement that cryptographic/freshness machinery.
- [in-toto Attestation v1.2](https://github.com/in-toto/attestation/blob/main/spec/README.md) provides authenticated metadata bound to explicit artifact subjects. The current prototype uses unkeyed synthetic digests and stable event IDs only; it is not an in-toto statement or authenticated envelope.
- [SLSA Provenance v1.2](https://slsa.dev/spec/v1.2/build-provenance) standardizes evidence about how software artifacts were produced. It informs artifact identity, but build provenance by itself is not proof of runtime target state or recovery.
- [OSTree atomic upgrades](https://ostreedev.github.io/ostree/atomic-upgrades/) documents an implementation whose design aims to leave either the old or new bootable deployment after a crash or power interruption. That guarantee must not be assumed for another deployment stack without testing it there.

The opportunity is a compositional contract that links update/artifact identity, transaction authorization, exact target identity, observed runtime state, independent appraisal, and recovery evidence while preserving the limits of each layer. This prototype demonstrates only the structural-binding part of that contract.

## Next benchmark ladder

1. **Now — structural causal binding:** frozen 29-attack / 3-control corpus; exact outcome inventory; zero false accepts/rejects/errors; report digests and exact-head provenance.
2. **Next — independent implementations:** port the contract oracle into Rust with no shared validation implementation, then differential-test Rust and the test harness over generated candidates. Any disagreement blocks promotion.
3. **Cryptographic binding:** verify actual signatures against a pinned trust policy; bind signed authorization to target, plan, policy, nonce, and expiry. A digest-shaped signature field is not signature verification.
4. **Freshness and replay:** use a verifier-issued challenge with persistent single-use state and race 64 concurrent submissions; target exactly one acceptance. Test stale challenges, wrong target/policy/artifact, and crash-safe nonce consumption.
5. **Execution and target reality:** in disposable QEMU/NixOS, compare the desired generation/artifact digest against independently collected booted state. Inject process termination at every durable transition boundary and explicitly classify old-good, new-good, recoverable, and blocked outcomes.
6. **Crash recovery:** a proposed target is 100 process kills per durable boundary plus 1,000 VM reset/restore trials, with zero silent partial promotions. These are future targets, not results from this PR.
7. **Scale and operations:** simulate 100, 1,000, 10,000, and 100,000 targets, reporting p50/p95/p99 verification and rollback latency, throughput, peak memory, retry volume, and maximum correlated failure blast radius. Keep simulated and physical-system evidence separate.

## Explicit limitations

- Candidate records and controls are synthetic; they do not establish that an action occurred.
- SHA-256 chain consistency is not authenticity. Someone able to rewrite the whole chain can recompute its digests.
- The signature field is a digest-shaped sentinel; no signature, certificate, signer trust, or key revocation is verified.
- A nonce is present for shape testing only. Freshness, replay-state persistence, trusted time, concurrency and nonce reuse are not checked here.
- Recovery is a cross-record consistency check, not an observed rollback or restored host state.
- The Python code is test-only. A production verifier should be Rust-first, independently reviewed, fail closed, and integrated only after exact-head end-to-end evidence.
