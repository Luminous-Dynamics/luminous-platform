# SRQ D1 SQLite ChallengeConsumer adapter v1

**Status:** Rust implementation prototype and CI fault-injection test. It is not yet integrated into the deployed verifier service or a release gate, and its outcomes are capped at model / abstract-model evidence.

## What this changes

The D1 verifier previously exposed a `ChallengeConsumer` trait, while its durable-store semantics were exercised mostly through a shell script that emitted SQL. This crate (`tools/srq-d1-sqlite-store/`) implements that trait using pinned `rusqlite` and the canonical D1 schema in `benchmarks/srq-v1/sqlite-d1-challenge-consumption-reference.sql`.

The verifier/store composition is intended to be:

`verify_and_consume(COSE_Sign1, trusted key, expected challenge, durable store)`

COSE signature/claim verification remains in `srq-eat-cose-verifier`. The new store commits the verified observation receipt and one-time challenge pointer in the same SQLite transaction. The adapter itself does not authenticate signatures and should not be treated as a way to accept a caller-constructed `ChallengeConsumptionRequest`.

## Store contract

- **Explicit identity:** a genuinely empty SQLite file receives an application ID and schema version. A nonempty unversioned file, unknown version, missing/extra table or trigger, or mismatched application ID is rejected; the store does not silently adopt or reinitialize it.
- **Durability configuration:** each opened connection requests WAL and `synchronous=FULL`, enables foreign keys, and uses a bounded busy timeout. If SQLite cannot honor WAL/FULL, the store fails closed.
- **Nonce minimization:** challenge issuance persists only SHA-256(nonce), not the raw nonce. `IssuedChallenge` debug formatting also redacts it.
- **Freshness under the writer lock:** `BEGIN IMMEDIATE` is acquired before reading trusted verifier time and comparing the challenge. Expired/future challenges, wrong target/audience/decision/generation/digests/deployment/signer/nonce, and replays are rejected.
- **Atomic acceptance:** one immutable receipt row is inserted, the challenge is pointed at that receipt with a compare-and-swap that must affect exactly one row, and the transaction commits once. A failure before commit leaves neither receipt nor consumption.
- **Lost acknowledgement:** the read-only `committed_receipt_for_challenge` lookup reconciles a transaction that committed before the caller died, without allowing the challenge to be consumed a second time.
- **Append-only issuance:** SQL triggers reject updates to immutable challenge bindings and deletion of challenge issuance records. Receipt rows are immutable and cannot be deleted.
- **Recovery inspection:** `integrity_check()` runs SQLite integrity and foreign-key checks and separately checks the one-to-one receipt/challenge pointer invariant.

The canonical database schema is shared with the D1 SQL reference; it does not create a parallel set of table definitions in Rust.

## Executable checks

The Rust unit suite covers:
- commit, reopen and receipt-pointer reconciliation;
- replay after reopen;
- wrong target and exact-expiry negative controls;
- rejection of a nonempty unversioned database;
- 64 independent SQLite connections racing to consume one challenge;
- actual child-process `SIGKILL` at three boundaries in the Rust adapter itself: after receipt insert (uncommitted), after challenge pointer update (uncommitted), and after commit but before acknowledgement.

The SIGKILL tests use test-only fault hooks that are compiled only under `cfg(test)`. The intended invariant is old/unconsumed state for the two pre-commit kills, and one complete committed receipt for the post-commit-before-ack kill. Each trial reopens the file through a fresh `SqliteD1Store` instance and checks SQLite integrity and the receipt/challenge relationship.

The standalone SQL script remains a second reference, including its 100 process-kill trials. Agreement between two implementations is useful, but is not an independent proof if both encode the same mistaken assumptions.

## What is not qualified

This is not yet wired into the production challenge issuance or verifier command path. The caller that issues the challenge must authenticate and persist the intended target/policy/deployment/signer binding before it sends the challenge. The COSE verifier must remain the only trusted source of verified request fields. The D1 record is a boot-observation challenge/receipt transaction, not the separate policy-promotion transaction tracked by [issue #91](https://github.com/Luminous-Dynamics/luminous-platform/issues/91).

The adapter does not provide:
- protection against a privileged attacker rewriting the database or replacing application binaries;
- durable signing-key provisioning, trusted time, TPM/secure-boot measurements, or hardware-rooted evidence;
- distributed transactions/locks across hosts or safety for arbitrary network filesystems;
- kernel crash, VM reset, sudden power loss, filesystem/device-cache failure, or physical-hardware qualification;
- a qualification-tier upgrade. The aggregate SRQ profile must remain `NOT_QUALIFIED` until all required gates have evidence at their declared tiers and claim ceilings.

## Next production steps

1. Run `verify_and_consume` through the new adapter in the actual D1 verifier service, with a trusted issuer registering the challenge before it is sent.
2. Generate and commit the exact Cargo lockfile; review the resolved dependency graph and license/security policy before deployment.
3. Run the same seeded target/policy/nonce/expiry mutation corpus against the Rust adapter and SQL reference; preserve false accepts/rejects and raw seeds.
4. Test durable store unavailability and schema/trigger tampering negative controls, then separately qualify the existing policy-promotion store and boot-observation reconciliation.
5. Only after the integration is real, run VM reset and controlled power-loss experiments on disposable infrastructure. Never treat live-kernel `SIGKILL` as a power-loss surrogate.
