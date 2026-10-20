# Luminous Edge durable freshness replay ledger v1

**Claim ceiling:** durable nonce-consumption and schema-consistency checks, not signed evidence, attestation, trusted time, or atomic control of an external device.

## Why a durable ledger is necessary

The challenge verifier in the freshness branch rejects a duplicate nonce in its in-memory guard. That protects repeated calls within one verifier instance, but process restart or parallel verifier instances lose that set. A verifier should not approve the same successful challenge/receipt again simply because it restarted.

This addition provides a durable SQLite-backed unique constraint and transactional insert. It uses rusqlite 0.40.2 with the bundled SQLite library, WAL journaling, FULL synchronous mode, and an IMMEDIATE write transaction. See [rusqlite transaction behavior](https://docs.rs/rusqlite/0.40.2/rusqlite/enum.TransactionBehavior.html) and the [transaction API](https://docs.rs/rusqlite/0.40.2/rusqlite/struct.Transaction.html).

## API and flow

- Module: tools/edge-policy-observation-reconciler/src/replay_ledger.rs
- Database wrapper: SqliteReplayLedger
- Event type: ReplayConsumption
- Combined verifier + durable insert: verify_and_consume_durable(...)

The combined helper executes the existing freshness checks and frozen-corpus reconciliation first. Only after they pass does it persist the nonce consumption in an immediate SQLite transaction. A unique primary-key constraint on the SHA-256 digest of the nonce allows at most one committed consumption per nonce across separate processes using the same database. The stored record also binds challenge ID, verifier time, exact receipt-byte digest, policy generation, and target token.

If verification fails, the durable insert is never attempted. If storage, schema validation, locking, or the unique constraint fails, the combined helper returns an error. The caller must not promote the policy unless the helper succeeds.

## Durability and safety choices

- The nonce itself is not stored; only its SHA-256 digest is retained.
- The database file is created with Unix mode 0600; existing files granting group/other permissions are rejected.
- Symbolic-link paths and in-memory databases are rejected.
- SQLite WAL mode and FULL synchronous mode are enabled; writes use an IMMEDIATE transaction and a 5-second busy timeout.
- The nonce table has a primary-key uniqueness constraint; concurrent independent connections racing to consume the same nonce should produce exactly one success.
- Schema version and exact column shape are checked whenever the ledger is used.
- Records are not automatically deleted. This avoids accidentally reopening replay opportunities after expiry. Capacity/retention policy is a future design decision that must preserve uniqueness guarantees.
- The ledger path's parent directory must be trusted and permission-restricted; filesystem durability still depends on the storage device and filesystem.
- This is not authentication: a local attacker who can replace or rewrite the database, forge the receipt/envelope, or modify the verifier can defeat these assumptions. The database and verifier must be within the trusted control plane.
- Durable nonce consumption is atomic inside SQLite, but cannot be made atomic with an external access point/firewall policy commit by this library alone. The caller must not promote before success and needs a reconciliation/rollback protocol for crashes between durable consumption and external actuation.

## Tests

The module tests cover:

1. First consumption succeeds; second use is rejected.
2. Replay remains rejected after closing and reopening the database.
3. Two separate connections racing on one nonce produce exactly one successful insert.
4. Invalid nonces and in-memory databases fail closed.
5. A database with a changed table schema is rejected.
6. Stored data contains only the digest, not the original nonce.
7. End-to-end freshness verification commits the nonce before returning and rejects the same receipt after a simulated verifier restart.

These deterministic tests are not a crash-consistency campaign. Future qualification should add process-kill tests during the write/commit boundary, filesystem/SQLite fault injection, lock timeout and corruption tests, and a concurrent multi-process stress run over at least 100,000 challenges.

## Competitive position

Freshness nonces and replay resistance are established concepts, not novel by themselves. IETF RFC 9334 describes nonce-based freshness models and their need for verifier state; NIST SP 800-63B also describes challenge/nonce-based replay resistance. This module is a Rust platform implementation detail, not a replacement for those architectures or a claim of attestation.

The useful next benchmark is operational: quantify cost per durable receipt, p50/p95/p99 transaction latency under concurrent writers, duplicate-rejection accuracy, restart replay rejection, and recovery after hard process termination. Report raw counts, SQLite version, filesystem, storage medium, synchronization settings, and every failed/inconclusive trial.
