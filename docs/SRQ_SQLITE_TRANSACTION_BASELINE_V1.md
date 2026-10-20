# SRQ-1 SQLite promotion transaction baseline v1

**Status:** executable SQLite reference fixture; not the production updater or promotion store.

## Why compare with SQLite instead of building a new database?

OSTree already targets old-or-new deployment behavior for upgrades. SQLite has mature ACID transaction and crash-recovery machinery and documents a dedicated crash-test approach that varies interruptions and validates the recovered database. A custom snapshot protocol should not be presented as state-of-the-art merely because it passes a small unit-test model.

The reference DDL is in benchmarks/srq-v1/sqlite-promotion-reference.sql. It models target policy state, a promotion-decision record, and an explicit challenge-consumption ledger. The writer operation must atomically commit all three or none of them. The companion test-sqlite-promotion-reference.sh checks a successful transaction, a stale-generation compare-and-swap rollback, and rejection of a committed challenge replay.

## Required transaction contract

### Current executable reference observation (2026-10-20)

On source head 751c959f7b6e96d2542d546b20ea6e5f8276009c, CI ran the reference script with SQLite 3.45.1, WAL journal mode and synchronous=FULL. All four cases passed: committed state/decision/challenge together; stale-generation compare-and-swap rolls back all writes; duplicate challenge replay is rejected; and 64 independent writer processes contend on one target/challenge with exactly one durable promotion. The race then verified generation 1, one decision row, one challenge-consumption row, and PRAGMA integrity_check = ok. See [workflow run 39431318386](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39431318386).

This is a small SQL contract fixture, not the production store, and it is not included in the signed SRQ evidence bundle yet. It is preserved as the separate Actions artifact srq-sqlite-reference-evidence. Its PASS does not satisfy or upgrade a QEMU/physical gate and does not prove correctness for kernel crash, VM reset or physical power loss.

The production adapter should use one SQLite write transaction (BEGIN IMMEDIATE) and:

1. Verify target binding, current generation, expected previous effective-policy digest, policy/corpus digests, decision identity, authorization and freshness under a well-defined trust policy.
2. Insert the decision row and challenge-consumption row under database uniqueness constraints.
3. Compare-and-swap the target policy state from the expected generation/digest to exactly the next generation/digests.
4. Require exactly one row to change. Any conflict or expired authorization rolls back the transaction, including the provisional challenge consumption.
5. Commit once. If the process dies after commit but before the caller receives the acknowledgement, query by decision ID and return the already-committed outcome; do not repeat the promotion.
6. Configure and report journal/synchronous settings on every writing connection. The current reference selects WAL and synchronous=FULL; preserve the actual SQLite version and connection options in benchmark evidence.

SQLite documents that WAL with synchronous=FULL synchronizes the WAL on each transaction commit; WAL with synchronous=NORMAL may roll back a recent transaction after power loss even while retaining database consistency. The configured durability policy is therefore part of the test input, not an implementation detail. See the [SQLite synchronous pragma documentation](https://www.sqlite.org/pragma.html) and [SQLite atomic commit overview](https://www.sqlite.org/atomiccommit.html).

## Qualification and test matrix

The current script is a small SQL contract check, not a production qualification. The next adapter must test:

- 64 concurrent callers and independent processes racing on the same target/challenge, requiring exactly one committed decision;
- stale generation, wrong target, wrong policy digest, wrong corpus digest, bad signer, expiry and challenge replay;
- SIGKILL before transaction commit and after commit but before acknowledgement, with a fresh connection and full integrity checks after reopening;
- forced process exit on each SQL write boundary, plus database integrity checks and verification that decision, consumed challenge and active state agree;
- actual VM reset/power-loss tests against the production adapter, with old-or-new complete state, p50/p95/p99 recovery latency, and explicit data-loss/failure counts.

The live-kernel process-kill harness elsewhere in SRQ does not simulate loss of the kernel page cache. SQLite smoke tests are not a substitute for its production integration tests, crash tests, or power-loss tests.

## Boundaries

This is a single-database reference; it does not provide cross-host consensus or make a database on arbitrary network storage safe. Trusted time, signer/target identity, cryptographic verification, policy semantics, migrations, backup/restore, schema downgrade rules and the real updater remain explicit requirements. Local transaction safety should be composed with TUF for update-metadata threat models rather than used as a replacement for TUF.

## Comparative claim rule

Compare the SQLite reference and Luminous's production adapter using the same frozen challenge set and fault schedule. Publish both the passing and rejected paths, the SQLite version/journal settings, raw logs, source/artifact digests, and crash outcomes. Do not call the reference fixture an industry win; it is the baseline against which an integration must demonstrate additional cross-layer correctness.
