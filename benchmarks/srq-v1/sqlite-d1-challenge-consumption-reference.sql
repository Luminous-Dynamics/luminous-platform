PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = FULL;

-- Reference schema for one-use D1 EAT/COSE boot-observation challenge
-- consumption. The application MUST verify COSE and compute canonical record
-- digests before opening this transaction. SQL does not authenticate signatures.

CREATE TABLE d1_observation_challenges (
    challenge_id TEXT PRIMARY KEY NOT NULL,
    target_token TEXT NOT NULL,
    audience TEXT NOT NULL,
    decision_id TEXT NOT NULL,
    policy_generation INTEGER NOT NULL CHECK (policy_generation >= 0),
    approved_artifact_sha256 TEXT NOT NULL
        CHECK (length(approved_artifact_sha256) = 64
               AND approved_artifact_sha256 NOT GLOB '*[^0-9a-f]*'),
    effective_policy_sha256 TEXT NOT NULL
        CHECK (length(effective_policy_sha256) = 64
               AND effective_policy_sha256 NOT GLOB '*[^0-9a-f]*'),
    corpus_sha256 TEXT NOT NULL
        CHECK (length(corpus_sha256) = 64
               AND corpus_sha256 NOT GLOB '*[^0-9a-f]*'),
    expected_deployment_id TEXT NOT NULL,
    expected_signer_key_id_hex TEXT NOT NULL
        CHECK (length(expected_signer_key_id_hex) > 0
               AND expected_signer_key_id_hex NOT GLOB '*[^0-9a-f]*'),
    nonce_sha256 TEXT NOT NULL
        CHECK (length(nonce_sha256) = 64
               AND nonce_sha256 NOT GLOB '*[^0-9a-f]*'),
    issued_at_unix_seconds INTEGER NOT NULL CHECK (issued_at_unix_seconds >= 0),
    expires_at_unix_seconds INTEGER NOT NULL
        CHECK (expires_at_unix_seconds > issued_at_unix_seconds),
    required_assurance_tier TEXT NOT NULL CHECK (required_assurance_tier = 'D1'),
    consumed_receipt_sha256 TEXT UNIQUE
        REFERENCES d1_verified_observation_receipts(receipt_sha256)
        DEFERRABLE INITIALLY DEFERRED
) WITHOUT ROWID;

CREATE TABLE d1_verified_observation_receipts (
    receipt_sha256 TEXT PRIMARY KEY NOT NULL
        CHECK (length(receipt_sha256) = 64 AND receipt_sha256 NOT GLOB '*[^0-9a-f]*'),
    challenge_id TEXT NOT NULL UNIQUE
        REFERENCES d1_observation_challenges(challenge_id),
    profile_id TEXT NOT NULL CHECK (profile_id = 'srq-eat-boot-observation-v1'),
    signer_key_id_hex TEXT NOT NULL
        CHECK (length(signer_key_id_hex) > 0 AND signer_key_id_hex NOT GLOB '*[^0-9a-f]*'),
    target_token TEXT NOT NULL,
    audience TEXT NOT NULL,
    decision_id TEXT NOT NULL,
    policy_generation INTEGER NOT NULL CHECK (policy_generation >= 0),
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
    boot_instance_id TEXT NOT NULL,
    nonce_sha256 TEXT NOT NULL
        CHECK (length(nonce_sha256) = 64
               AND nonce_sha256 NOT GLOB '*[^0-9a-f]*'),
    challenge_issued_at_unix_seconds INTEGER NOT NULL CHECK (challenge_issued_at_unix_seconds >= 0),
    challenge_expires_at_unix_seconds INTEGER NOT NULL
        CHECK (challenge_expires_at_unix_seconds > challenge_issued_at_unix_seconds),
    verifier_now_unix_seconds INTEGER NOT NULL CHECK (verifier_now_unix_seconds >= 0),
    envelope_sha256 TEXT NOT NULL
        CHECK (length(envelope_sha256) = 64
               AND envelope_sha256 NOT GLOB '*[^0-9a-f]*'),
    committed_at_unix_seconds INTEGER NOT NULL CHECK (committed_at_unix_seconds >= 0)
) WITHOUT ROWID;

-- A signed envelope is not accepted merely because an application inserted a
-- receipt-shaped row: the receipt fields must match the still-unconsumed
-- challenge and the verifier's current-time value must be in the allowed
-- interval. These triggers enforce record consistency, not COSE authenticity.
CREATE TRIGGER d1_receipt_must_match_fresh_challenge
BEFORE INSERT ON d1_verified_observation_receipts
WHEN NOT EXISTS (
    SELECT 1
      FROM d1_observation_challenges AS c
     WHERE c.challenge_id = NEW.challenge_id
       AND c.target_token = NEW.target_token
       AND c.audience = NEW.audience
       AND c.decision_id = NEW.decision_id
       AND c.policy_generation = NEW.policy_generation
       AND c.approved_artifact_sha256 = NEW.approved_artifact_sha256
       AND c.effective_policy_sha256 = NEW.effective_policy_sha256
       AND c.corpus_sha256 = NEW.corpus_sha256
       AND c.expected_deployment_id = NEW.deployment_id
       AND c.expected_signer_key_id_hex = NEW.signer_key_id_hex
       AND c.nonce_sha256 = NEW.nonce_sha256
       AND c.issued_at_unix_seconds = NEW.challenge_issued_at_unix_seconds
       AND c.expires_at_unix_seconds = NEW.challenge_expires_at_unix_seconds
       AND c.required_assurance_tier = 'D1'
       AND c.consumed_receipt_sha256 IS NULL
       AND NEW.verifier_now_unix_seconds >= c.issued_at_unix_seconds
       AND NEW.verifier_now_unix_seconds < c.expires_at_unix_seconds
)
BEGIN
    SELECT RAISE(ABORT, 'D1 receipt does not match a fresh unconsumed challenge');
END;

CREATE TRIGGER d1_challenge_binding_is_immutable
BEFORE UPDATE ON d1_observation_challenges
WHEN OLD.challenge_id IS NOT NEW.challenge_id
  OR OLD.target_token IS NOT NEW.target_token
  OR OLD.audience IS NOT NEW.audience
  OR OLD.decision_id IS NOT NEW.decision_id
  OR OLD.policy_generation IS NOT NEW.policy_generation
  OR OLD.approved_artifact_sha256 IS NOT NEW.approved_artifact_sha256
  OR OLD.effective_policy_sha256 IS NOT NEW.effective_policy_sha256
  OR OLD.corpus_sha256 IS NOT NEW.corpus_sha256
  OR OLD.expected_deployment_id IS NOT NEW.expected_deployment_id
  OR OLD.expected_signer_key_id_hex IS NOT NEW.expected_signer_key_id_hex
  OR OLD.nonce_sha256 IS NOT NEW.nonce_sha256
  OR OLD.issued_at_unix_seconds IS NOT NEW.issued_at_unix_seconds
  OR OLD.expires_at_unix_seconds IS NOT NEW.expires_at_unix_seconds
  OR OLD.required_assurance_tier IS NOT NEW.required_assurance_tier
BEGIN
    SELECT RAISE(ABORT, 'D1 challenge authorization bindings are immutable');
END;

CREATE TRIGGER d1_challenge_consumed_once
BEFORE UPDATE OF consumed_receipt_sha256 ON d1_observation_challenges
WHEN OLD.consumed_receipt_sha256 IS NOT NULL
  OR NEW.consumed_receipt_sha256 IS NULL
BEGIN
    SELECT RAISE(ABORT, 'D1 challenge is already consumed or cannot be unconsumed');
END;

CREATE TRIGGER d1_receipt_is_immutable
BEFORE UPDATE ON d1_verified_observation_receipts
BEGIN
    SELECT RAISE(ABORT, 'D1 verified observation receipts are immutable');
END;

CREATE TRIGGER d1_receipt_cannot_be_deleted
BEFORE DELETE ON d1_verified_observation_receipts
BEGIN
    SELECT RAISE(ABORT, 'D1 verified observation receipts are append-only');
END;
