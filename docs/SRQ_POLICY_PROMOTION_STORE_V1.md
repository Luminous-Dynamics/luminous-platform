# SRQ durable policy-promotion store prototype v1

**Status: research prototype; not integrated with the deployed updater.** This crate demonstrates the missing transaction boundary from issue #91. It does not qualify the production release path.

## Invariant being enforced

For one target and one freshly issued challenge, the following state transition is one SQLite transaction:

- record one signed decision ID and the signer key ID;
- consume the challenge, which is bound to target, audience, decision, expiry and a SHA-256 of its random nonce;
- compare-and-swap the current generation and previous approved-artifact/effective-policy/corpus/deployment identity;
- advance the target to exactly the next generation with the newly approved artifact/policy and persist the decision pointer.

The only public mutation method is `verify_and_promote(token, trusted_key)`. The internal verified-authorization type is private. It is created only after strict COSE_Sign1/Ed25519 verification and exact claim-inventory checks. Caller-provided fields alone cannot instantiate the trusted authorization record or call a separate public “promote verified fields” method.

## Transaction order

1. Verify the COSE signature and strict signed claim profile.
2. Begin `BEGIN IMMEDIATE`, which serializes local SQLite writers.
3. Re-read the challenge and target state; reject expired, consumed or stale inputs.
4. Check every signed binding against the persisted challenge and exact previous target state.
5. Insert a decision row.
6. Compare-and-swap the target state from previous generation, previous approved artifact, previous effective policy, corpus and immutable deployment.
7. Set the challenge's consumed-decision pointer exactly once.
8. Commit. Any earlier error drops the SQLite transaction and rolls back all changes.

After a process dies after commit but before the caller receives the acknowledgement, `committed_decision_for_challenge` reconciles the challenge pointer, decision row and active target state. It does not permit replay. `integrity_check` also runs SQLite's structural check and validates the lifecycle binding invariants.

## Storage policy

- Local SQLite file, WAL mode, `synchronous=FULL`, foreign keys enabled, 10-second bounded busy timeout.
- The store no longer enables `rusqlite`'s bundled SQLite source. It links through `pkg-config` to the maintained system SQLite library, and the CI installer builds SQLite 3.54.1 from a SHA3-256-pinned upstream archive, verifies the exact source ID, and exports the library/pkg-config paths before Rust compilation. Startup rejects SQLite 3.53.0–3.53.3 and 3.54.0 as well as withdrawn 3.52.x; it retains explicit known-fixed backport floors (3.44.6, 3.50.7, 3.51.3) and later patched lines (3.53.4+, 3.54.1+). This rejects known-buggy releases without claiming that any version is defect-free.
- Exact schema SQL inventory is checked on every open. Unknown database identity, unexpected schema objects or altered table/index SQL fail closed.
- The raw challenge nonce is never persisted; only its SHA-256 digest is stored.

## Actual process-termination fault schedule added

The crate's test-only writer hooks are placed after these SQL operations, but before commit:

1. Decision row inserted.
2. Target generation compare-and-swap completed.
3. Challenge-consumption pointer updated.

A fourth hook pauses immediately after the transaction commits but before the method returns its acknowledgement. The parent test launches a separate Rust test-harness process, waits for a synced marker, sends Linux SIGKILL, reopens the same SQLite database, runs integrity and lifecycle checks, and reconciles the decision ID.

The test intends 20 repetitions at each of the four boundaries (80 child-process SIGKILL trials). For the three pre-commit boundaries, it requires generation 7, no committed decision and an unconsumed challenge. For the post-commit/pre-ack boundary, it requires generation 8, exactly one recoverable decision and one consumed challenge. This is a live-kernel process termination test—not a kernel panic, VM reset, power interruption, or storage-controller durability test.

CI now also attempts to preserve the model test log and exact Cargo.lock as supplemental evidence in the fixed-inventory archive. The archive manifest labels the observation model / abstract_model_only; it is not a qualifying receipt. The current newest workflow is queued, so the 11-test suite and its 80 SIGKILL trials are not yet confirmed passing on the latest branch head.

## Tests and next gates

The unit/integration suite tests signed authorization, invalid signature without mutation, replay after reopen and original-decision reconciliation, expiry, mismatched target/generation/digest, a 64-connection race with exactly one commit, schema tampering, nonce minimization and SQLite version parsing.

The next production acceptance step is to call the transaction at the actual policy-promotion boundary, not from a parallel demo endpoint. Then add process termination at every SQL boundary (including commit-before-ack), repeat concurrency/fault trials, and compare the deployed adapter to this reference on a matched fault schedule. QEMU reset, storage failure and physical power interruption remain separate tiers.

## Trust and qualification boundaries

This prototype's `TrustedPromotionKey` is configuration supplied by a trusted caller; durable signer provisioning and key rotation are not implemented here. The challenge issuer must authenticate the deployment/policy intent before issuing a nonce. The code trusts local wall-clock time, and local SQLite does not provide distributed transactions or cross-host locks. A privileged attacker who can rewrite application binaries or the database can subvert a software-only store. It does not provide TPM quotes, measured boot, D2 reference-value appraisal, firmware root-of-trust or proof that the promoted deployment actually booted.

The store's `Consistent`/committed output is not release qualification. SRQ remains `NOT_QUALIFIED` until the real updater integration and every required gate's declared QEMU/physical tier passes. Related work: [issue #91](https://github.com/Luminous-Dynamics/luminous-platform/issues/91), [matched benchmark issue #94](https://github.com/Luminous-Dynamics/luminous-platform/issues/94), and [D2 TPM appraisal design](SRQ_D2_TPM_APPRAISAL_V1.md).


## Unknown-schema open must not mutate the database

The policy-promotion adapter now inspects the SQLite schema identity and object inventory before switching the persistent journal mode. It rejects nonempty unowned databases, unknown versions, mismatched application IDs, and version-tagged databases with no schema objects before making persistent mode changes. A new regression test checks both the rejection and preservation of the foreign database's `DELETE` journal mode and existing row.

The preflight is intentionally not treated as schema authentication: after the database proves it is the expected SRQ store, the adapter still configures WAL/`synchronous=FULL` and compares actual table/index/trigger SQL against the canonical embedded schema.
