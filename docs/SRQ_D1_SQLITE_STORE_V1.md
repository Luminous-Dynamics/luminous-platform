# SRQ D1 SQLite ChallengeConsumer adapter v1

**Status:** Rust implementation prototype and CI fault-injection test. It is not yet integrated into the deployed verifier service or a release gate, and its outcomes are capped at model / abstract-model evidence.

## What this changes

The D1 verifier previously exposed a `ChallengeConsumer` trait, while its durable-store semantics were exercised mostly through a shell script that emitted SQL. This crate (`tools/srq-d1-sqlite-store/`) implements that trait using pinned `rusqlite` and the canonical D1 schema in `benchmarks/srq-v1/sqlite-d1-challenge-consumption-reference.sql`.

The verifier/store composition is intended to be:

`verify_and_consume(COSE_Sign1, trusted key, expected challenge, durable store)`

COSE signature/claim verification remains in `srq-eat-cose-verifier`. The new store commits the verified observation receipt and one-time challenge pointer in the same SQLite transaction. The adapter itself does not authenticate signatures and should not be treated as a way to accept a caller-constructed `ChallengeConsumptionRequest`.

## Type-state boundary

The verifier's public `ChallengeConsumer` trait now accepts `VerifiedChallengeConsumption`, not a caller-constructible `ChallengeConsumptionRequest`. The capability wraps the request in a private field and exposes read-only access for durable-store implementations. Its value is created by `verify_and_consume` only after `verify_cose_eat` succeeds, preventing ordinary external callers from assembling arbitrary field values and passing them directly through this trait boundary. An external-crate `compile_fail` doctest checks that the private field cannot be forged from ordinary downstream code. This is compile-time API hardening, not protection against a modified verifier binary or privileged code inside the verifier crate.

This is a compile-time API boundary, not a complete process-isolation or binary-integrity guarantee. Code compiled into the verifier crate can create the private wrapper, and a malicious or compromised verifier binary can still bypass the intended call path. The SQLite store still rechecks every stored challenge binding, schema identity, nonce digest and freshness bound transactionally.

## Store contract

- **Explicit identity and schema fingerprint:** a genuinely empty SQLite file receives an application ID and schema version. A nonempty unversioned file, unknown version, missing/extra table/trigger, altered table or trigger SQL, or mismatched application ID is rejected; the verifier builds the expected schema using the same SQLite engine and compares each persisted schema object's SQL definition, not just its name.
- **Durability configuration:** each opened connection requests WAL and `synchronous=FULL`, enables foreign keys, and uses a bounded busy timeout. If SQLite cannot honor WAL/FULL, the store fails closed.
- **Known WAL-reset corruption policy:** startup queries the linked `sqlite_version()` and accepts the upstream-documented fixed lines: 3.44.6+, 3.50.7+, 3.51.3+, and 3.53.0+. The WAL-reset bug affects WAL-mode databases when multiple connections write/checkpoint concurrently. SQLite lists 3.51.3 as the patch release and documents backports to 3.44.6 and 3.50.7; the withdrawn 3.52.0 line is explicitly rejected. This gate addresses that one known bug, not unrelated engine defects. The linked Rust engine's version and `sqlite_source_id()` are printed by the test suite. See [SQLite's WAL-reset bug notes](https://www.sqlite.org/wal.html#walresetbug).
- **Nonce minimization:** challenge issuance persists only SHA-256(nonce), not the raw nonce. `IssuedChallenge` debug formatting also redacts it.
- **Freshness at transaction start and commit:** `BEGIN IMMEDIATE` is acquired before reading the verifier host's Unix wall clock and comparing the challenge, so rival SQLite writers cannot both pass an earlier stale check. The store now reads the clock again after staging the receipt/challenge writes, immediately before commit; if expiry has passed, the transaction is dropped and both writes roll back. A deterministic clock-sequence test proves this case. The wall clock itself is **not authenticated trusted time** and can be manipulated or rolled back; hardware-backed/remote time assurance and rollback detection remain unresolved production requirements. Expired/future challenges, wrong target/audience/decision/generation/digests/deployment/signer/nonce, and replays are rejected.
- **Atomic acceptance:** one immutable receipt row is inserted, the challenge is pointed at that receipt with a compare-and-swap that must affect exactly one row, and the transaction commits once. A failure before commit leaves neither receipt nor consumption.
- **Lost acknowledgement:** the read-only `committed_receipt_for_challenge` lookup reconciles a transaction that committed before the caller died, without allowing the challenge to be consumed a second time.
- **Append-only issuance:** SQL triggers reject updates to immutable challenge bindings and deletion of challenge issuance records. Receipt rows are immutable and cannot be deleted.
- **Recovery inspection:** `integrity_check()` runs SQLite integrity and foreign-key checks and separately checks the one-to-one receipt/challenge pointer invariant.

The canonical database schema is shared with the D1 SQL reference; it does not create a parallel set of table definitions in Rust. The adapter also includes a negative test that drops and replaces the challenge-deletion trigger with a same-name no-op trigger and requires schema verification to reject reopening the database.

## Executable checks

The suite now also has an external-crate integration test at `tools/srq-d1-sqlite-store/tests/verified_consumption.rs`. It constructs a real Ed25519-signed COSE/EAT payload using the public API, registers the exact target-bound challenge, calls `verify_and_consume` with the SQLite adapter, and reopens the committed receipt. Before the valid token, it mutates a signed token byte and requires signature failure to leave both the receipt and consumed-challenge counts at zero. After success, it proves replay is rejected through the same public API. This is stronger composition coverage than the unit-test store mocks, but remains software-key/model evidence and not measured boot or hardware attestation. The external test passed on code head a66ebfc6e6e03f34d08a58e48eee07843b1742d4 in [run 39645017004](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39645017004), and the later documentation-only head ab790de72f86f11566118559ac8309ebc3e4856f passed in [run 39645077549](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39645077549). The real trusted-main GitHub attestation was skipped on both PR runs.

The Rust unit suite covers:
- commit, reopen and receipt-pointer reconciliation;
- replay after reopen;
- wrong target and exact-expiry negative controls, including expiry between the first transaction check and the commit boundary;
- rejection of a nonempty unversioned database;
- 64 independent SQLite connections racing to consume one challenge;
- actual child-process `SIGKILL` at three boundaries in the Rust adapter itself: after receipt insert (uncommitted), after challenge pointer update (uncommitted), and after commit but before acknowledgement.

The SIGKILL tests use test-only fault hooks that are compiled only under `cfg(test)`. The intended invariant is old/unconsumed state for the two pre-commit kills, and one complete committed receipt for the post-commit-before-ack kill. Each trial reopens the file through a fresh `SqliteD1Store` instance and checks SQLite integrity and the receipt/challenge relationship. These child-process tests call the adapter's private transaction helper directly so each SQLite boundary can be interrupted deterministically; the separate `srq-eat-cose-verifier` tests exercise `verify_and_consume` against test stores. Thus API composition and storage durability have component coverage, but not yet an end-to-end signed-COSE-to-real-SQLite process-kill test.

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
2. The Cargo lockfile generated by the hosted test build is now committed at `tools/srq-d1-sqlite-store/Cargo.lock`; the CI command uses `cargo test --locked`. Review the locked dependency graph, transitive features, and license/security policy before deployment.
3. Run the same seeded target/policy/nonce/expiry mutation corpus against the Rust adapter and SQL reference; preserve false accepts/rejects and raw seeds.
4. Test durable store unavailability and schema/trigger tampering negative controls, then separately qualify the existing policy-promotion store and boot-observation reconciliation.
5. Only after the integration is real, run VM reset and controlled power-loss experiments on disposable infrastructure. Never treat live-kernel `SIGKILL` as a power-loss surrogate.
