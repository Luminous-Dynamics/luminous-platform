-- Durable one-use appraisal for signed SRQ matched-comparison envelopes.
-- This schema binds an external issuer challenge, trusted Ed25519 key identity,
-- baseline/source scope and a single immutable successful appraisal receipt.
-- SQL enforces transaction consistency; Rust still verifies DSSE, statement
-- semantics, closed-world subjects and subject bytes before opening the write transaction.

CREATE TABLE srq_comparison_challenges (
    challenge_id TEXT PRIMARY KEY NOT NULL,
    run_challenge_sha256 TEXT NOT NULL UNIQUE
        CHECK (length(run_challenge_sha256) = 64 AND run_challenge_sha256 NOT GLOB '*[^0-9a-f]*'),
    expected_key_id TEXT NOT NULL CHECK (length(expected_key_id) BETWEEN 1 AND 121),
    expected_public_key_sha256 TEXT NOT NULL
        CHECK (length(expected_public_key_sha256) = 64 AND expected_public_key_sha256 NOT GLOB '*[^0-9a-f]*'),
    baseline_id TEXT NOT NULL CHECK (length(baseline_id) BETWEEN 2 AND 81),
    expected_srq_source_sha TEXT NOT NULL
        CHECK (length(expected_srq_source_sha) = 40 AND expected_srq_source_sha NOT GLOB '*[^0-9a-f]*'),
    issued_at_unix_seconds INTEGER NOT NULL CHECK (issued_at_unix_seconds >= 0),
    expires_at_unix_seconds INTEGER NOT NULL CHECK (expires_at_unix_seconds > issued_at_unix_seconds),
    consumed_receipt_sha256 TEXT UNIQUE
        REFERENCES srq_comparison_envelopes(receipt_sha256)
        DEFERRABLE INITIALLY DEFERRED
) WITHOUT ROWID;

CREATE TABLE srq_comparison_envelopes (
    receipt_sha256 TEXT PRIMARY KEY NOT NULL
        CHECK (length(receipt_sha256) = 64 AND receipt_sha256 NOT GLOB '*[^0-9a-f]*'),
    challenge_id TEXT NOT NULL UNIQUE REFERENCES srq_comparison_challenges(challenge_id),
    payload_sha256 TEXT NOT NULL
        CHECK (length(payload_sha256) = 64 AND payload_sha256 NOT GLOB '*[^0-9a-f]*'),
    key_id TEXT NOT NULL,
    public_key_sha256 TEXT NOT NULL
        CHECK (length(public_key_sha256) = 64 AND public_key_sha256 NOT GLOB '*[^0-9a-f]*'),
    baseline_id TEXT NOT NULL,
    srq_source_sha TEXT NOT NULL
        CHECK (length(srq_source_sha) = 40 AND srq_source_sha NOT GLOB '*[^0-9a-f]*'),
    subject_count INTEGER NOT NULL CHECK (subject_count >= 3),
    consumed_at_unix_seconds INTEGER NOT NULL CHECK (consumed_at_unix_seconds >= 0),
    run_challenge_sha256 TEXT NOT NULL
        CHECK (length(run_challenge_sha256) = 64 AND run_challenge_sha256 NOT GLOB '*[^0-9a-f]*')
) WITHOUT ROWID;

CREATE TRIGGER srq_comparison_receipt_must_match_fresh_challenge
BEFORE INSERT ON srq_comparison_envelopes
WHEN NOT EXISTS (
    SELECT 1 FROM srq_comparison_challenges AS c
     WHERE c.challenge_id = NEW.challenge_id
       AND c.run_challenge_sha256 = NEW.run_challenge_sha256
       AND c.expected_key_id = NEW.key_id
       AND c.expected_public_key_sha256 = NEW.public_key_sha256
       AND c.baseline_id = NEW.baseline_id
       AND c.expected_srq_source_sha = NEW.srq_source_sha
       AND c.consumed_receipt_sha256 IS NULL
       AND NEW.consumed_at_unix_seconds >= c.issued_at_unix_seconds
       AND NEW.consumed_at_unix_seconds < c.expires_at_unix_seconds
)
BEGIN
    SELECT RAISE(ABORT, 'comparison receipt does not match a fresh unconsumed challenge');
END;

CREATE TRIGGER srq_comparison_challenge_bindings_immutable
BEFORE UPDATE ON srq_comparison_challenges
WHEN OLD.challenge_id IS NOT NEW.challenge_id
  OR OLD.run_challenge_sha256 IS NOT NEW.run_challenge_sha256
  OR OLD.expected_key_id IS NOT NEW.expected_key_id
  OR OLD.expected_public_key_sha256 IS NOT NEW.expected_public_key_sha256
  OR OLD.baseline_id IS NOT NEW.baseline_id
  OR OLD.expected_srq_source_sha IS NOT NEW.expected_srq_source_sha
  OR OLD.issued_at_unix_seconds IS NOT NEW.issued_at_unix_seconds
  OR OLD.expires_at_unix_seconds IS NOT NEW.expires_at_unix_seconds
BEGIN
    SELECT RAISE(ABORT, 'comparison challenge bindings are immutable');
END;

CREATE TRIGGER srq_comparison_challenge_consumed_once
BEFORE UPDATE OF consumed_receipt_sha256 ON srq_comparison_challenges
WHEN OLD.consumed_receipt_sha256 IS NOT NULL OR NEW.consumed_receipt_sha256 IS NULL
BEGIN
    SELECT RAISE(ABORT, 'comparison challenge is consumed or cannot be unconsumed');
END;

CREATE TRIGGER srq_comparison_challenge_cannot_be_deleted
BEFORE DELETE ON srq_comparison_challenges
BEGIN
    SELECT RAISE(ABORT, 'comparison challenge records are append-only');
END;

CREATE TRIGGER srq_comparison_envelope_is_immutable
BEFORE UPDATE ON srq_comparison_envelopes
BEGIN
    SELECT RAISE(ABORT, 'comparison appraisal receipts are immutable');
END;

CREATE TRIGGER srq_comparison_envelope_cannot_be_deleted
BEFORE DELETE ON srq_comparison_envelopes
BEGIN
    SELECT RAISE(ABORT, 'comparison appraisal receipts are append-only');
END;
