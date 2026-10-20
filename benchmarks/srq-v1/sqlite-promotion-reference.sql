-- SRQ-1 SQLite transaction reference (design fixture; not yet production code).
-- SQLite baseline: WAL + synchronous=FULL. Set these on each writer connection.
-- See docs/SRQ_SQLITE_TRANSACTION_BASELINE_V1.md before using in an adapter.
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = FULL;

CREATE TABLE IF NOT EXISTS target_policy_state (
    target_id TEXT PRIMARY KEY NOT NULL,
    generation INTEGER NOT NULL CHECK (generation >= 0),
    approved_policy_digest TEXT NOT NULL
        CHECK (length(approved_policy_digest) = 64
               AND approved_policy_digest NOT GLOB '*[^0-9a-f]*'),
    effective_policy_digest TEXT NOT NULL
        CHECK (length(effective_policy_digest) = 64
               AND effective_policy_digest NOT GLOB '*[^0-9a-f]*'),
    corpus_digest TEXT NOT NULL
        CHECK (length(corpus_digest) = 64
               AND corpus_digest NOT GLOB '*[^0-9a-f]*'),
    last_decision_id TEXT
) WITHOUT ROWID;

-- Rows are committed decisions only: insert/update all related rows in ONE
-- transaction and never persist a pre-commit "accepted" status separately.
CREATE TABLE IF NOT EXISTS promotion_decisions (
    decision_id TEXT PRIMARY KEY NOT NULL,
    target_id TEXT NOT NULL REFERENCES target_policy_state(target_id),
    challenge_id TEXT NOT NULL,
    previous_generation INTEGER NOT NULL CHECK (previous_generation >= 0),
    next_generation INTEGER NOT NULL
        CHECK (next_generation = previous_generation + 1),
    previous_effective_policy_digest TEXT NOT NULL
        CHECK (length(previous_effective_policy_digest) = 64
               AND previous_effective_policy_digest NOT GLOB '*[^0-9a-f]*'),
    approved_policy_digest TEXT NOT NULL
        CHECK (length(approved_policy_digest) = 64
               AND approved_policy_digest NOT GLOB '*[^0-9a-f]*'),
    effective_policy_digest TEXT NOT NULL
        CHECK (length(effective_policy_digest) = 64
               AND effective_policy_digest NOT GLOB '*[^0-9a-f]*'),
    corpus_digest TEXT NOT NULL
        CHECK (length(corpus_digest) = 64
               AND corpus_digest NOT GLOB '*[^0-9a-f]*'),
    verifier_id TEXT NOT NULL,
    UNIQUE (target_id, challenge_id)
) WITHOUT ROWID;

-- Explicit replay ledger. The target/challenge pair and decision ID must be
-- unique. Foreign key means a consumption cannot exist without its decision.
CREATE TABLE IF NOT EXISTS challenge_consumptions (
    target_id TEXT NOT NULL,
    challenge_id TEXT NOT NULL,
    decision_id TEXT NOT NULL UNIQUE
        REFERENCES promotion_decisions(decision_id),
    consumed_generation INTEGER NOT NULL CHECK (consumed_generation > 0),
    PRIMARY KEY (target_id, challenge_id),
    FOREIGN KEY (target_id) REFERENCES target_policy_state(target_id)
) WITHOUT ROWID;

-- Required writer transaction (parameter names are illustrative):
--
--   BEGIN IMMEDIATE;
--   -- Re-read current state under the writer lock. Validate target, expected
--   -- generation, previous effective-policy digest, challenge target/binding,
--   -- trusted freshness deadline, signer policy and decision/corpus digests.
--   INSERT INTO promotion_decisions (...) VALUES (...);
--   INSERT INTO challenge_consumptions (...) VALUES (...);
--   UPDATE target_policy_state
--      SET generation = :next_generation,
--          approved_policy_digest = :approved_policy_digest,
--          effective_policy_digest = :effective_policy_digest,
--          corpus_digest = :corpus_digest,
--          last_decision_id = :decision_id
--    WHERE target_id = :target_id
--      AND generation = :previous_generation
--      AND effective_policy_digest = :previous_effective_policy_digest;
--   -- The adapter MUST require changes() = 1. Otherwise ROLLBACK.
--   -- Re-check freshness immediately before COMMIT using a trusted clock; if
--   -- expired or any invariant fails, ROLLBACK (which also unconsumes challenge).
--   COMMIT;
--
-- A unique-constraint failure, stale generation, digest mismatch, or freshness
-- failure must roll back all three tables' changes. If a client is killed after
-- COMMIT but before it receives the acknowledgement, query by decision_id and
-- return the stored committed outcome; do not retry as a new promotion.
--
-- Boundaries: this schema does not implement authorization, crypto verification,
-- trusted time, SQLite connection management, or cross-host consensus. A local
-- SQLite transaction is not a distributed lock for multiple independent hosts.
