-- SRQ durable policy-promotion store v1.
-- This is a single-host SQLite contract. It must be combined with authenticated
-- authorization verification and post-boot reconciliation by the caller.
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = FULL;

CREATE TABLE IF NOT EXISTS promotion_targets (
    target_id TEXT PRIMARY KEY NOT NULL,
    generation INTEGER NOT NULL CHECK (generation >= 0),
    approved_artifact_sha256 TEXT NOT NULL
        CHECK (length(approved_artifact_sha256) = 64
               AND approved_artifact_sha256 NOT GLOB '*[^0-9a-f]*'),
    effective_policy_sha256 TEXT NOT NULL
        CHECK (length(effective_policy_sha256) = 64
               AND effective_policy_sha256 NOT GLOB '*[^0-9a-f]*'),
    corpus_sha256 TEXT NOT NULL
        CHECK (length(corpus_sha256) = 64
               AND corpus_sha256 NOT GLOB '*[^0-9a-f]*'),
    deployment_id TEXT NOT NULL,
    last_decision_id TEXT
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS promotion_challenges (
    challenge_id TEXT PRIMARY KEY NOT NULL,
    target_id TEXT NOT NULL REFERENCES promotion_targets(target_id),
    audience TEXT NOT NULL,
    decision_id TEXT NOT NULL UNIQUE,
    previous_generation INTEGER NOT NULL CHECK (previous_generation >= 0),
    next_generation INTEGER NOT NULL CHECK (next_generation = previous_generation + 1),
    previous_approved_artifact_sha256 TEXT NOT NULL
        CHECK (length(previous_approved_artifact_sha256) = 64
               AND previous_approved_artifact_sha256 NOT GLOB '*[^0-9a-f]*'),
    previous_effective_policy_sha256 TEXT NOT NULL
        CHECK (length(previous_effective_policy_sha256) = 64
               AND previous_effective_policy_sha256 NOT GLOB '*[^0-9a-f]*'),
    approved_artifact_sha256 TEXT NOT NULL
        CHECK (length(approved_artifact_sha256) = 64
               AND approved_artifact_sha256 NOT GLOB '*[^0-9a-f]*'),
    next_effective_policy_sha256 TEXT NOT NULL
        CHECK (length(next_effective_policy_sha256) = 64
               AND next_effective_policy_sha256 NOT GLOB '*[^0-9a-f]*'),
    corpus_sha256 TEXT NOT NULL
        CHECK (length(corpus_sha256) = 64
               AND corpus_sha256 NOT GLOB '*[^0-9a-f]*'),
    deployment_id TEXT NOT NULL,
    expected_signer_key_id_hex TEXT NOT NULL,
    nonce_sha256 TEXT NOT NULL
        CHECK (length(nonce_sha256) = 64
               AND nonce_sha256 NOT GLOB '*[^0-9a-f]*'),
    issued_at_unix_seconds INTEGER NOT NULL CHECK (issued_at_unix_seconds >= 0),
    expires_at_unix_seconds INTEGER NOT NULL CHECK (expires_at_unix_seconds > issued_at_unix_seconds),
    consumed_decision_id TEXT UNIQUE,
    UNIQUE (challenge_id, target_id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS promotion_decisions (
    decision_id TEXT PRIMARY KEY NOT NULL,
    target_id TEXT NOT NULL REFERENCES promotion_targets(target_id),
    challenge_id TEXT NOT NULL UNIQUE,
    previous_generation INTEGER NOT NULL CHECK (previous_generation >= 0),
    next_generation INTEGER NOT NULL CHECK (next_generation = previous_generation + 1),
    previous_approved_artifact_sha256 TEXT NOT NULL
        CHECK (length(previous_approved_artifact_sha256) = 64
               AND previous_approved_artifact_sha256 NOT GLOB '*[^0-9a-f]*'),
    previous_effective_policy_sha256 TEXT NOT NULL
        CHECK (length(previous_effective_policy_sha256) = 64
               AND previous_effective_policy_sha256 NOT GLOB '*[^0-9a-f]*'),
    approved_artifact_sha256 TEXT NOT NULL
        CHECK (length(approved_artifact_sha256) = 64
               AND approved_artifact_sha256 NOT GLOB '*[^0-9a-f]*'),
    next_effective_policy_sha256 TEXT NOT NULL
        CHECK (length(next_effective_policy_sha256) = 64
               AND next_effective_policy_sha256 NOT GLOB '*[^0-9a-f]*'),
    corpus_sha256 TEXT NOT NULL
        CHECK (length(corpus_sha256) = 64
               AND corpus_sha256 NOT GLOB '*[^0-9a-f]*'),
    deployment_id TEXT NOT NULL,
    signer_key_id_hex TEXT NOT NULL,
    envelope_sha256 TEXT NOT NULL
        CHECK (length(envelope_sha256) = 64
               AND envelope_sha256 NOT GLOB '*[^0-9a-f]*'),
    committed_at_unix_seconds INTEGER NOT NULL CHECK (committed_at_unix_seconds >= 0),
    UNIQUE (decision_id, target_id, next_generation),
    FOREIGN KEY (challenge_id, target_id)
        REFERENCES promotion_challenges(challenge_id, target_id)
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS promotion_decisions_target_generation
    ON promotion_decisions(target_id, next_generation);

-- App code requires target state, decision and challenge pointer to agree after
-- every commit/reopen. The transaction inserts the decision, CAS-updates the
-- target, then consumes the exact challenge before COMMIT.
