# SRQ D1 durable challenge-consumption SQLite reference v1

**Status:** executable SQL reference and fault-injection benchmark. This is not a production Rust `ChallengeConsumer`, not a hardware attestation, and not a release qualification.

## Why a separate consumption boundary exists

A valid COSE signature proves only the signed envelope's integrity and signer-key relationship under a configured trust policy. It does not by itself prove that the challenge is still unconsumed, fresh, target-bound, or transactionally associated with a durable verifier receipt. The SRQ D1 verifier therefore exposes a separate `ChallengeConsumer` contract. Its `ConsumedObservation` is an opaque non-serializable capability returned only after a consumer reports a syntactically valid durable-receipt digest; that type is still not release-gate approval.

The schema is `benchmarks/srq-v1/sqlite-d1-challenge-consumption-reference.sql`; the test harness is `benchmarks/srq-v1/test-sqlite-d1-challenge-consumption-reference.sh`.

## Atomic transaction contract

One `BEGIN IMMEDIATE` transaction must:

1. Match an outstanding challenge on challenge ID, target token, audience, decision ID, generation, approved artifact digest, effective policy digest, corpus digest, expected deployment identity, expected signer key ID, nonce digest, issuance/expiry interval, and assurance tier.
2. Insert the immutable verified-observation receipt.
3. Update the challenge's consumed-receipt pointer, conditional on it still being unconsumed and the verifier's current time being inside the challenge interval.
4. Require exactly one row to change and commit the receipt plus consumption pointer together. Every failure rolls back both.

The reference schema adds uniqueness constraints, foreign keys, a deferred circular reference so the receipt/pointer can commit atomically, and triggers that reject receipt/challenge mismatches, challenge-binding edits, challenge replay, receipt edits, and receipt deletion. Those constraints protect row consistency; they do **not** authenticate COSE or prove the caller obtained its clock from a trusted source.

## Fault-injection campaign

The CI reference harness has the following targets:

- 64 independent SQLite CLI processes racing for one challenge: exactly one durable receipt and one consumed pointer must win.
- Negative binding controls for wrong target, audience, decision, generation, approved artifact, effective policy, corpus, deployment, signer key, nonce, expiry, and issuance-time mismatch.
- Real Linux `SIGKILL` at four boundaries, 25 fresh-database trials each: just after `BEGIN IMMEDIATE`; after receipt insert but before challenge consumption; after consumption update but before commit; and after commit but before the caller receives acknowledgement.
- For each of the three pre-commit kill points, reopening must find no receipt and an unconsumed challenge. For each post-commit kill, reopening must find exactly one matching receipt and pointer, and `PRAGMA integrity_check` must return `ok`.
- The test preserves the exact per-trial outcomes, SQLite version, journal mode, synchronous setting and source-bound archive digest.

This is userspace process-kill testing against a live kernel. It does not simulate kernel panic, loss of the kernel page cache, VM reset, block-device cache reordering, physical power loss, cross-host consensus, or a production deployment adapter.

## Trust-clock boundary

The executable SQL fixture uses fixed test timestamps to make boundary conditions deterministic. In production, the adapter—not the caller—must obtain current time using its configured trusted-time policy, re-check expiry inside the write transaction, and never accept `now` or `verifier_now_unix_seconds` as a user-controlled API argument. The current verifier CLI uses the verifier host's system clock; it explicitly does not claim a hardware-backed trusted clock. RFC 9334 describes evidence freshness and warns that state may change after evidence is produced, so freshness needs a stated relying-party policy rather than a caller-provided boolean: https://www.rfc-editor.org/rfc/rfc9334.html

## What is still missing

- A production Rust SQLite-backed `ChallengeConsumer` implementing the trait in `tools/srq-eat-cose-verifier/src/lib.rs` using prepared statements, an immediate write transaction, an internally obtained current time, and the exact transaction invariants.
- A same-process integration test that calls `verify_and_consume` with a valid signed COSE EAT fixture and the real SQLite adapter.
- A separately reviewed, committed Rust dependency lockfile and exact-head `cargo test --locked` evidence for that adapter.
- Independent trusted-main signing and verification of the archive; PR contract tests and the fake CLI are not cryptographic signature evidence.
- QEMU/VM reset and physical durability testing for the production adapter.

The next tracked implementation should reuse SQLite rather than invent a new store protocol, while preserving TUF metadata verification and keeping runtime boot observation as a separate proof source. SQLite's own atomic-commit model and WAL durability settings are relevant baselines, but a passing reference script does not qualify the production updater: https://www.sqlite.org/atomiccommit.html and https://www.sqlite.org/wal.html

**Claim ceiling:** a passing run establishes the SQL fixture's expected behavior at the tested live-process fault boundaries on the named CI runner. It does not close issue #91, authenticate an actual target's booted system, prove physical power-loss durability, or change the SRQ aggregate `NOT_QUALIFIED` result.
