# SRQ boot-observation reconciliation v1

**Status:** dependency-free Rust contract model; not a production verifier, runtime attestation system, or release qualification.

## Paradigm shift: evidence should reconcile across lifecycle boundaries

An update artifact can have valid build provenance and still fail to be the artifact that actually booted. A promotion record can be durable yet the machine can boot the previous generation, another target's generation, or a runtime with a different effective-policy digest.

The invariant this model tests is:

**promotion intent → durable decision + one-time challenge consumption → a fresh post-commit observation challenge → observed booted generation → independently reconciled target/policy/artifact/generation identity**

This differs from checking each component separately. The layers can each emit a valid-looking record while disagreeing with one another. The join must refuse to promote that contradiction into an end-to-end success.

### Immutable deployment identity is separate from the logical generation counter

A logical SRQ generation number (such as 42) cannot be assumed to equal a NixOS generation number or an OSTree deployment identity. The intent, durable record and observation challenge now also carry an immutable `deployment_id`: a canonical Nix store path such as `/nix/store/<32-char-hash>-nixos-system-…`, or a 64-character OSTree deployment checksum. The observation contains both `booted_deployment_id` and `current_userspace_deployment_id`.

This follows the actual NixOS module contract in Sovereign Boot: `/run/booted-system` identifies the system that booted, while `/run/current-system` can change during a live `nixos-rebuild switch/test`. A live activation must not be treated as proof of rebooting into the authorized deployment. The model returns Blocked unless both identities agree with the intended deployment and with each other. This conservative rule detects mixed or not-yet-rebooted transitions; it does not claim to read these paths from a real machine.

## Why issue a separate post-commit challenge?

The challenge consumed to authorize promotion and the challenge used to request a post-boot observation have different roles. Reusing the promotion nonce for runtime observation makes freshness harder to reason about. This model represents the post-commit boot-observation challenge as a separate object bound to target, decision ID, expected generation and effective-policy digest.

The model does **not** authenticate that challenge or prove it was issued after commit. The calling verifier must verify its authenticated issuance, audience/target scope, freshness policy and one-time semantics. Likewise, it must authenticate the boot observation through the selected evidence mechanism before relying on reconciliation.

## Implemented contract

Source: `tools/srq-boot-reconciliation-model/src/lib.rs`

The dependency-free model accepts four records:
- `PromotionIntent`: target, decision, promotion challenge, immutable deployment identity, previous/next logical generation, approved/effective policy digests, artifact digest and conformance corpus digest.
- `CommittedPromotion`: durable decision identity, target, consumed challenge, immutable deployment identity, logical generation and the same bound digests.
- `BootObservationChallenge`: a distinct post-commit challenge bound to target, decision, expected logical generation, immutable deployment identity and effective-policy digest.
- `BootObservation`: target, decision, challenge ID, boot-instance ID, the booted deployment identity, the current userspace deployment identity, logical generation, effective-policy digest, artifact digest and a measurement digest.

It returns `Consistent`, `PendingBootChallenge`, `PendingBootObservation`, or `Blocked(reasons)`. Only `Consistent` means the supplied records structurally agree, and even then no authenticity is implied. The test explicitly proves that a different but correctly formatted measurement digest does not turn a structurally consistent record into a trusted attestation.

The test suite includes exact-match reconciliation, missing evidence states, prior-generation boot rejection, target/decision/challenge mismatch, policy/artifact/corpus mismatch, nonconsecutive and overflow transitions, malformed identifiers/digests, 10,000 deterministic single-field boot-observation mutations, and 100,000 deterministic mutations distributed across all four record types. The mutation campaigns require zero contradictory records to return Consistent.

## First executed result (2026-10-20)

[Exact-head workflow run 39470466328](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39470466328) passed on source `59d6774ed3d3e67b2be7327aa06b1788b9d3aa5b`: all 21 tests passed. This includes 100,000 deterministic cross-layer single-field mutations, 10,000 boot-observation mutations, and explicit negative cases for mismatched immutable deployment IDs in the durable record and post-commit challenge, booting the previous NixOS store path while `/run/current-system` points at the expected newer path, and current-vs-booted path divergence. The archive-inventory/hash suite passed too, including false-digest controls for the SQLite and boot-reconciliation logs.

This is a deterministic contract-model result, not a 100,000-trial field reliability estimate. The mutation cases target structural consistency, not cryptographic signature validity or the truth of an observed boot measurement. The same workflow still reports `NOT_QUALIFIED`, 0/22 release gates met at qualifying tier, and its trusted-main signing job is skipped on pull-request events.

## Industry comparison: compose existing guarantees rather than replace them

- **OSTree** documents atomic transitions among bootable deployments and old-or-new outcomes across interrupted upgrades. SRQ should not claim to beat that property using a model. It should test whether the specific authorized decision can be traced to the deployment actually observed after boot. [OSTree atomic upgrades](https://github.com/ostreedev/ostree/blob/main/docs/atomic-upgrades.md)
- **NixOS generations** offer explicit previous-generation selection and rollback. A comparison must record what generation actually booted, not only that a rollback command was issued. [NixOS manual](https://nixos.org/manual/nixos/stable/)
- **SLSA provenance / GitHub artifact attestations** can bind an artifact to its producing workflow and source identity, but provenance is useful only when a consumer verifies it against expected identity. That still does not, by itself, prove what is currently running. [SLSA artifact verification](https://slsa.dev/spec/v1.2/verifying-artifacts) and [GitHub artifact attestations](https://docs.github.com/en/actions/concepts/security/artifact-attestations)
- **TUF** remains the update-metadata verification and rollback/freeze threat-model baseline. This layer is not a substitute for TUF. [TUF specification](https://github.com/theupdateframework/specification/blob/master/tuf-spec.md)
- **RFC 9334 RATS** distinguishes attester, verifier and relying-party responsibilities. SRQ must keep input appraisal, relying policy and evidence freshness explicit rather than labeling digest matching as remote attestation. [RFC 9334](https://www.rfc-editor.org/rfc/rfc9334.html)

## Acceptance gates for the next integration

These are targets, not measured achievements:

1. Zero false reconciliations in the boot-only and cross-layer mutation campaigns, including deployment-ID mutations; retain deterministic seeds and malformed-record fixtures.
2. Zero success when durable promotion is missing or the active boot generation, target, decision, challenge, effective-policy digest or artifact digest differs.
3. Separate observability states: awaiting a fresh challenge, awaiting a boot observation, structurally consistent, blocked, and cryptographically unverified. Never map pending/unverified to PASS.
4. At integration time, obtain the booted-generation fact from a defined source (for example, a verifier challenge response that includes current target identity and booted deployment digest). Document trust anchors, measurement coverage, freshness, replay prevention and failure behavior for that source.
5. Run identical positive/negative fixtures against Luminous and an OSTree/NixOS deployment path on comparable VM tiers. Report disagreement-detection coverage, false accept/reject counts, p50/p95/p99 observation latency and the fault tier.
6. Add QEMU reboot/fallback and later physical reset/power-interruption evidence. Do not claim the model qualifies either.

## Qualification boundary

The Rust model establishes only deterministic structural reconciliation for the records supplied to it. It does not verify cryptographic signatures, TPM/secure-boot state, measured-boot chains, firmware, trusted time, source-workflow provenance, actual booted state, freshness, QEMU, or physical hardware. It remains supplemental model-tier evidence and cannot satisfy a QEMU or physical SRQ gate.
