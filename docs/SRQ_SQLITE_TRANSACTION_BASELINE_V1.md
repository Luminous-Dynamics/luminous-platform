# SRQ-1 SQLite promotion transaction baseline v1

**Status:** executable SQLite reference fixture; not the production updater or promotion store.

## Why compare with SQLite instead of building a new database?

OSTree already targets old-or-new deployment behavior for upgrades. SQLite has mature ACID transaction and crash-recovery machinery and documents a dedicated crash-test approach that varies interruptions and validates the recovered database. A custom snapshot protocol should not be presented as state-of-the-art merely because it passes a small unit-test model.

The reference DDL is in benchmarks/srq-v1/sqlite-promotion-reference.sql. It models target policy state, a promotion-decision record, and an explicit challenge-consumption ledger. The writer operation must atomically commit all three or none of them. The companion test-sqlite-promotion-reference.sh checks a successful transaction, a stale-generation compare-and-swap rollback, and rejection of a committed challenge replay.

## Stronger relational invariants added to the reference

The schema now uses composite foreign keys to make several transaction invariants database-enforced rather than relying only on calling code:

- `target_policy_state(last_decision_id, target_id, generation)` must refer to a promotion decision for the same target and current generation.
- `challenge_consumptions(decision_id, target_id, challenge_id, consumed_generation)` must refer to a decision with the same target, challenge and next generation.
- Unique parent keys make those composite references valid SQLite foreign-key targets.

This is intentionally defense in depth: a bug that pairs a valid decision ID with a different valid target or generation should fail the SQL constraint. It still does not prove that a challenge was cryptographically authorized, fresh, or issued by a trusted identity.

The current branch adds a negative control that attempts such a target-mismatch write and requires the entire transaction to roll back. It also extends real SQLite process termination to four locations: after the decision row, after challenge consumption, after target-state update but before COMMIT, and after COMMIT but before acknowledgement. The first three must reopen as generation zero with no decision/consumption rows; the last must reopen as exactly one complete, decision-ID-discoverable promotion.

**These expanded checks were added to the current branch but are awaiting the exact-head run result.** They are not counted as passed evidence until that run completes and its logs are inspected.

## Required transaction contract

### Current executable reference observation (2026-10-20)

On exact tested head [6338f6f31d34a5e7d0c2b0ed1b1b879ee9a0b4bd](https://github.com/Luminous-Dynamics/luminous-platform/commit/6338f6f31d34a5e7d0c2b0ed1b1b879ee9a0b4bd), [workflow run 39432653655 passed](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39432653655) with SQLite 3.45.1, WAL mode and synchronous=FULL. The four cases passed: atomic state/decision/challenge commit, stale-generation all-or-nothing rollback, duplicate challenge replay rejection, and 64 independent writer processes contending for one target/challenge with exactly one durable promotion. The final state had one decision, one challenge-consumption row, generation 1, and PRAGMA integrity_check = ok.

Four CI runs now provide exploratory per-contender script elapsed-time distributions (not pure SQLite commit latency; timing includes CLI/process startup and lock waiting):

| Workflow run | All-contender p50 / p95 / p99 | Sole winner elapsed |
|---|---:|---:|
| [39431749407](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39431749407) | 82.1 / 138.6 / 150.8 ms | 47.4 ms |
| [39432653655](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39432653655) | 34.7 / 68.4 / 87.2 ms | 11.5 ms |
| [39433030759](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39433030759) | 48.1 / 79.2 / 94.9 ms | 42.8 ms |
| [39433712275](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39433712275) | 22.0 / 57.7 / 62.3 ms | 9.0 ms |

The spread is a useful warning: these are small GitHub-hosted-runner smoke measurements, not a stable performance benchmark or a comparison against another updater on matched hardware. Repeated measurements on a frozen machine/storage tuple are required before drawing latency conclusions. The next benchmark should report the raw per-contender timings and repeat the 64-way race at least 30 times on a fixed host before publishing comparative percentiles.

The SQLite log is now included as a digest-bound supplemental observation in the deterministic SRQ archive subject and is checked by the archive verifier and wrapper contract tests. The actual signed GitHub attestation has **not** yet been minted or verified: the trusted-main signing job is intentionally skipped on pull-request events. The reference remains a small SQL contract fixture, not the production store; it cannot satisfy or upgrade a QEMU/physical gate and does not prove kernel-crash, VM-reset or physical-power-loss behavior.

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
