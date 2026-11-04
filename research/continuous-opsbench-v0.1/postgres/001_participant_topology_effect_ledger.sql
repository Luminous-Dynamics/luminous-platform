-- PostgreSQL participant CAS/effect-ledger reference migration v0.1.
-- Intended only for a disposable integration environment until crash/race
-- qualification is complete. PostgreSQL transactions are used for local DB
-- effects only; this does not make arbitrary remote provider effects atomic.

BEGIN;

CREATE SCHEMA IF NOT EXISTS opsbench_participant;

DO $$
BEGIN
    CREATE TYPE opsbench_participant.effect_state AS ENUM (
        'accepted_pending',
        'outcome_unknown',
        'committed',
        'never_accepted',
        'rejected'
    );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END
$$;

CREATE TABLE IF NOT EXISTS opsbench_participant.participant_scope_head (
    participant_id TEXT NOT NULL CHECK (length(participant_id) > 0),
    scope_id TEXT NOT NULL CHECK (length(scope_id) > 0),
    topology_epoch BIGINT NOT NULL CHECK (topology_epoch >= 0),
    topology_digest BYTEA NOT NULL CHECK (octet_length(topology_digest) = 32),
    participant_sequence BIGINT NOT NULL DEFAULT 0 CHECK (participant_sequence >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (participant_id, scope_id)
);

-- This table is populated only by the trusted source-authentication adapter.
-- Database constraints validate the normalized binding; they do not verify a
-- signature, trust root, principal identity or publisher authenticity.
CREATE TABLE IF NOT EXISTS opsbench_participant.verified_authorization_receipt (
    receipt_id TEXT PRIMARY KEY CHECK (length(receipt_id) > 0),
    receipt_sha256 BYTEA NOT NULL CHECK (octet_length(receipt_sha256) = 32),
    decision TEXT NOT NULL CHECK (decision IN ('allow', 'deny')),
    participant_id TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    attempt_id TEXT NOT NULL,
    logical_action_key TEXT NOT NULL,
    operation TEXT NOT NULL,
    target TEXT NOT NULL,
    request_sha256 BYTEA NOT NULL CHECK (octet_length(request_sha256) = 32),
    expected_topology_epoch BIGINT NOT NULL CHECK (expected_topology_epoch >= 0),
    expected_topology_sha256 BYTEA NOT NULL CHECK (octet_length(expected_topology_sha256) = 32),
    policy_epoch BIGINT NOT NULL CHECK (policy_epoch >= 0),
    policy_sha256 BYTEA NOT NULL CHECK (octet_length(policy_sha256) = 32),
    not_before TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    authenticated_publisher_id TEXT NOT NULL,
    authenticated_at TIMESTAMPTZ NOT NULL,
    CHECK (not_before < expires_at),
    CHECK (length(participant_id) > 0 AND length(scope_id) > 0),
    CHECK (length(tenant_id) > 0 AND length(attempt_id) > 0),
    CHECK (length(logical_action_key) > 0 AND length(operation) > 0 AND length(target) > 0),
    CHECK (length(authenticated_publisher_id) > 0)
);

CREATE TABLE IF NOT EXISTS opsbench_participant.logical_effect (
    participant_id TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    logical_action_key TEXT NOT NULL,
    effect_id TEXT NOT NULL CHECK (length(effect_id) > 0),
    operation TEXT NOT NULL,
    target TEXT NOT NULL,
    request_sha256 BYTEA NOT NULL CHECK (octet_length(request_sha256) = 32),
    current_attempt_id TEXT NOT NULL,
    state opsbench_participant.effect_state NOT NULL,
    expected_topology_epoch BIGINT NOT NULL CHECK (expected_topology_epoch >= 0),
    expected_topology_sha256 BYTEA NOT NULL CHECK (octet_length(expected_topology_sha256) = 32),
    committed_topology_epoch BIGINT,
    committed_topology_sha256 BYTEA,
    participant_sequence BIGINT,
    receipt_sha256 BYTEA,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    committed_at TIMESTAMPTZ,
    PRIMARY KEY (participant_id, scope_id, tenant_id, logical_action_key),
    UNIQUE (participant_id, scope_id, effect_id),
    UNIQUE (participant_id, scope_id, participant_sequence),
    CHECK (
        (state = 'committed'
            AND committed_topology_epoch IS NOT NULL
            AND committed_topology_sha256 IS NOT NULL
            AND octet_length(committed_topology_sha256) = 32
            AND participant_sequence IS NOT NULL
            AND participant_sequence > 0
            AND receipt_sha256 IS NOT NULL
            AND octet_length(receipt_sha256) = 32
            AND committed_at IS NOT NULL)
        OR
        (state <> 'committed'
            AND committed_topology_epoch IS NULL
            AND committed_topology_sha256 IS NULL
            AND participant_sequence IS NULL
            AND receipt_sha256 IS NULL
            AND committed_at IS NULL)
    )
);

CREATE TABLE IF NOT EXISTS opsbench_participant.effect_attempt (
    participant_id TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    logical_action_key TEXT NOT NULL,
    attempt_id TEXT NOT NULL,
    authorization_receipt_id TEXT NOT NULL
        REFERENCES opsbench_participant.verified_authorization_receipt(receipt_id),
    state opsbench_participant.effect_state NOT NULL,
    expected_topology_epoch BIGINT NOT NULL CHECK (expected_topology_epoch >= 0),
    expected_topology_sha256 BYTEA NOT NULL CHECK (octet_length(expected_topology_sha256) = 32),
    opened_transaction_id xid8 NOT NULL,
    receipt_sha256 BYTEA,
    opened_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    completed_at TIMESTAMPTZ,
    PRIMARY KEY (participant_id, scope_id, tenant_id, attempt_id),
    FOREIGN KEY (participant_id, scope_id, tenant_id, logical_action_key)
        REFERENCES opsbench_participant.logical_effect
            (participant_id, scope_id, tenant_id, logical_action_key),
    CHECK (
        (state = 'committed'
            AND receipt_sha256 IS NOT NULL
            AND octet_length(receipt_sha256) = 32
            AND completed_at IS NOT NULL)
        OR
        (state <> 'committed'
            AND receipt_sha256 IS NULL
            AND completed_at IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS effect_attempt_reconciliation_idx
    ON opsbench_participant.effect_attempt
       (participant_id, scope_id, tenant_id, logical_action_key, state, opened_at);

-- The local-effect protocol is begin -> apply the actual business-row change ->
-- finish, all in ONE PostgreSQL transaction. A deferred trigger rejects commit
-- if the caller forgets to finish an accepted local attempt.
CREATE OR REPLACE FUNCTION opsbench_participant.require_local_attempt_finalized()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
    current_state opsbench_participant.effect_state;
BEGIN
    SELECT attempt.state
      INTO current_state
      FROM opsbench_participant.effect_attempt AS attempt
     WHERE attempt.participant_id = NEW.participant_id
       AND attempt.scope_id = NEW.scope_id
       AND attempt.tenant_id = NEW.tenant_id
       AND attempt.attempt_id = NEW.attempt_id;

    IF current_state = 'accepted_pending' THEN
        RAISE EXCEPTION 'local participant attempt % was not finalized before transaction commit',
            NEW.attempt_id
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS effect_attempt_must_finalize
    ON opsbench_participant.effect_attempt;

CREATE CONSTRAINT TRIGGER effect_attempt_must_finalize
AFTER INSERT ON opsbench_participant.effect_attempt
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW
EXECUTE FUNCTION opsbench_participant.require_local_attempt_finalized();

-- Start or reconcile a local database effect.
-- Lock order: participant_scope_head -> logical_effect -> effect_attempt.
-- Topology publication uses participant_scope_head, so the topology check and
-- eventual local effect finalization serialize against the same row lock.
CREATE OR REPLACE FUNCTION opsbench_participant.begin_local_effect(
    p_authorization_receipt_id TEXT,
    p_effect_id TEXT,
    p_participant_id TEXT,
    p_scope_id TEXT,
    p_tenant_id TEXT,
    p_attempt_id TEXT,
    p_logical_action_key TEXT,
    p_operation TEXT,
    p_target TEXT,
    p_request_sha256 BYTEA,
    p_expected_topology_epoch BIGINT,
    p_expected_topology_sha256 BYTEA
)
RETURNS TABLE (
    disposition TEXT,
    effect_id TEXT,
    attempt_id TEXT,
    committed_topology_epoch BIGINT,
    committed_topology_sha256 BYTEA,
    participant_sequence BIGINT,
    receipt_sha256 BYTEA
)
LANGUAGE plpgsql
AS $$
DECLARE
    auth opsbench_participant.verified_authorization_receipt%ROWTYPE;
    head opsbench_participant.participant_scope_head%ROWTYPE;
    logical opsbench_participant.logical_effect%ROWTYPE;
    current_time_utc TIMESTAMPTZ := clock_timestamp();
    logical_exists BOOLEAN := FALSE;
BEGIN
    IF p_effect_id IS NULL OR length(p_effect_id) = 0
       OR p_participant_id IS NULL OR length(p_participant_id) = 0
       OR p_scope_id IS NULL OR length(p_scope_id) = 0
       OR p_tenant_id IS NULL OR length(p_tenant_id) = 0
       OR p_attempt_id IS NULL OR length(p_attempt_id) = 0
       OR p_logical_action_key IS NULL OR length(p_logical_action_key) = 0
       OR p_operation IS NULL OR length(p_operation) = 0
       OR p_target IS NULL OR length(p_target) = 0
       OR p_request_sha256 IS NULL OR octet_length(p_request_sha256) <> 32
       OR p_expected_topology_sha256 IS NULL
       OR octet_length(p_expected_topology_sha256) <> 32 THEN
        RAISE EXCEPTION 'invalid participant mutation binding'
            USING ERRCODE = '22023';
    END IF;

    SELECT receipt.*
      INTO auth
      FROM opsbench_participant.verified_authorization_receipt AS receipt
     WHERE receipt.receipt_id = p_authorization_receipt_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'verified authorization receipt not found'
            USING ERRCODE = '28000';
    END IF;

    IF auth.decision <> 'allow'
       OR auth.participant_id <> p_participant_id
       OR auth.scope_id <> p_scope_id
       OR auth.tenant_id <> p_tenant_id
       OR auth.attempt_id <> p_attempt_id
       OR auth.logical_action_key <> p_logical_action_key
       OR auth.operation <> p_operation
       OR auth.target <> p_target
       OR auth.request_sha256 <> p_request_sha256
       OR auth.expected_topology_epoch <> p_expected_topology_epoch
       OR auth.expected_topology_sha256 <> p_expected_topology_sha256 THEN
        RAISE EXCEPTION 'authorization receipt does not bind the exact mutation request'
            USING ERRCODE = '28000';
    END IF;

    IF current_time_utc < auth.not_before OR current_time_utc >= auth.expires_at THEN
        RAISE EXCEPTION 'authorization receipt is not currently valid'
            USING ERRCODE = '28000';
    END IF;

    -- This is the shared serialization point with topology publication.
    SELECT scope_head.*
      INTO head
      FROM opsbench_participant.participant_scope_head AS scope_head
     WHERE scope_head.participant_id = p_participant_id
       AND scope_head.scope_id = p_scope_id
     FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'participant scope has not been provisioned'
            USING ERRCODE = 'P0002';
    END IF;

    SELECT action.*
      INTO logical
      FROM opsbench_participant.logical_effect AS action
     WHERE action.participant_id = p_participant_id
       AND action.scope_id = p_scope_id
       AND action.tenant_id = p_tenant_id
       AND action.logical_action_key = p_logical_action_key
     FOR UPDATE;

    logical_exists := FOUND;

    IF logical_exists THEN
        IF logical.operation <> p_operation
           OR logical.target <> p_target
           OR logical.request_sha256 <> p_request_sha256
           OR logical.effect_id <> p_effect_id THEN
            RAISE EXCEPTION 'idempotency key reused for a different logical effect'
                USING ERRCODE = '23505';
        END IF;

        IF logical.state = 'committed' THEN
            RETURN QUERY SELECT
                'idempotent_replay'::TEXT,
                logical.effect_id,
                logical.current_attempt_id,
                logical.committed_topology_epoch,
                logical.committed_topology_sha256,
                logical.participant_sequence,
                logical.receipt_sha256;
            RETURN;
        END IF;

        IF logical.state IN ('accepted_pending', 'outcome_unknown') THEN
            RETURN QUERY SELECT
                'outcome_unknown'::TEXT,
                logical.effect_id,
                logical.current_attempt_id,
                logical.committed_topology_epoch,
                logical.committed_topology_sha256,
                logical.participant_sequence,
                logical.receipt_sha256;
            RETURN;
        END IF;
    END IF;

    -- Replay is reconciled before current topology comparison. A new attempt,
    -- including one after a strong NeverAccepted finding, must use the current
    -- topology precondition and a fresh exact authorization receipt.
    IF head.topology_epoch <> p_expected_topology_epoch
       OR head.topology_digest <> p_expected_topology_sha256 THEN
        RAISE EXCEPTION 'stale participant topology precondition'
            USING ERRCODE = '40001';
    END IF;

    IF logical_exists THEN
        UPDATE opsbench_participant.logical_effect
           SET current_attempt_id = p_attempt_id,
               state = 'accepted_pending',
               expected_topology_epoch = p_expected_topology_epoch,
               expected_topology_sha256 = p_expected_topology_sha256
         WHERE participant_id = p_participant_id
           AND scope_id = p_scope_id
           AND tenant_id = p_tenant_id
           AND logical_action_key = p_logical_action_key;
    ELSE
        INSERT INTO opsbench_participant.logical_effect (
            participant_id, scope_id, tenant_id, logical_action_key,
            effect_id, operation, target, request_sha256, current_attempt_id,
            state, expected_topology_epoch, expected_topology_sha256
        ) VALUES (
            p_participant_id, p_scope_id, p_tenant_id, p_logical_action_key,
            p_effect_id, p_operation, p_target, p_request_sha256, p_attempt_id,
            'accepted_pending', p_expected_topology_epoch, p_expected_topology_sha256
        );
    END IF;

    INSERT INTO opsbench_participant.effect_attempt (
        participant_id, scope_id, tenant_id, logical_action_key,
        attempt_id, authorization_receipt_id, state,
        expected_topology_epoch, expected_topology_sha256, opened_transaction_id
    ) VALUES (
        p_participant_id, p_scope_id, p_tenant_id, p_logical_action_key,
        p_attempt_id, p_authorization_receipt_id, 'accepted_pending',
        p_expected_topology_epoch, p_expected_topology_sha256, pg_current_xact_id()
    );

    RETURN QUERY SELECT
        'attempt_started'::TEXT,
        p_effect_id,
        p_attempt_id,
        NULL::BIGINT,
        NULL::BYTEA,
        NULL::BIGINT,
        NULL::BYTEA;
END;
$$;

-- Complete a local effect after the caller has performed the actual business
-- state change in the same transaction. The scope-head row lock acquired by
-- begin_local_effect remains held through this function and transaction commit.
CREATE OR REPLACE FUNCTION opsbench_participant.finish_local_effect(
    p_participant_id TEXT,
    p_scope_id TEXT,
    p_tenant_id TEXT,
    p_logical_action_key TEXT,
    p_attempt_id TEXT,
    p_receipt_sha256 BYTEA
)
RETURNS TABLE (
    effect_id TEXT,
    committed_topology_epoch BIGINT,
    committed_topology_sha256 BYTEA,
    participant_sequence BIGINT,
    receipt_sha256 BYTEA
)
LANGUAGE plpgsql
AS $$
DECLARE
    head opsbench_participant.participant_scope_head%ROWTYPE;
    logical opsbench_participant.logical_effect%ROWTYPE;
    attempt opsbench_participant.effect_attempt%ROWTYPE;
    next_sequence BIGINT;
    completed_at_utc TIMESTAMPTZ := clock_timestamp();
BEGIN
    IF p_receipt_sha256 IS NULL OR octet_length(p_receipt_sha256) <> 32 THEN
        RAISE EXCEPTION 'receipt digest must be 32 bytes'
            USING ERRCODE = '22023';
    END IF;

    SELECT scope_head.*
      INTO head
      FROM opsbench_participant.participant_scope_head AS scope_head
     WHERE scope_head.participant_id = p_participant_id
       AND scope_head.scope_id = p_scope_id
     FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'participant scope not found'
            USING ERRCODE = 'P0002';
    END IF;

    SELECT action.*
      INTO logical
      FROM opsbench_participant.logical_effect AS action
     WHERE action.participant_id = p_participant_id
       AND action.scope_id = p_scope_id
       AND action.tenant_id = p_tenant_id
       AND action.logical_action_key = p_logical_action_key
     FOR UPDATE;

    IF NOT FOUND OR logical.state <> 'accepted_pending'
       OR logical.current_attempt_id <> p_attempt_id THEN
        RAISE EXCEPTION 'logical effect is not pending for this exact attempt'
            USING ERRCODE = '40001';
    END IF;

    SELECT row_attempt.*
      INTO attempt
      FROM opsbench_participant.effect_attempt AS row_attempt
     WHERE row_attempt.participant_id = p_participant_id
       AND row_attempt.scope_id = p_scope_id
       AND row_attempt.tenant_id = p_tenant_id
       AND row_attempt.attempt_id = p_attempt_id
     FOR UPDATE;

    IF NOT FOUND
       OR attempt.state <> 'accepted_pending'
       OR attempt.opened_transaction_id <> pg_current_xact_id() THEN
        RAISE EXCEPTION 'local effect must begin and finish in the same database transaction'
            USING ERRCODE = '40001';
    END IF;

    IF head.topology_epoch <> logical.expected_topology_epoch
       OR head.topology_digest <> logical.expected_topology_sha256 THEN
        RAISE EXCEPTION 'topology changed before local effect finalization'
            USING ERRCODE = '40001';
    END IF;

    UPDATE opsbench_participant.participant_scope_head AS scope_head
       SET participant_sequence = scope_head.participant_sequence + 1,
           updated_at = completed_at_utc
     WHERE scope_head.participant_id = p_participant_id
       AND scope_head.scope_id = p_scope_id
     RETURNING scope_head.participant_sequence INTO next_sequence;

    UPDATE opsbench_participant.effect_attempt
       SET state = 'committed',
           receipt_sha256 = p_receipt_sha256,
           completed_at = completed_at_utc
     WHERE participant_id = p_participant_id
       AND scope_id = p_scope_id
       AND tenant_id = p_tenant_id
       AND attempt_id = p_attempt_id;

    UPDATE opsbench_participant.logical_effect
       SET state = 'committed',
           committed_topology_epoch = head.topology_epoch,
           committed_topology_sha256 = head.topology_digest,
           participant_sequence = next_sequence,
           receipt_sha256 = p_receipt_sha256,
           committed_at = completed_at_utc
     WHERE participant_id = p_participant_id
       AND scope_id = p_scope_id
       AND tenant_id = p_tenant_id
       AND logical_action_key = p_logical_action_key;

    RETURN QUERY SELECT
        logical.effect_id,
        head.topology_epoch,
        head.topology_digest,
        next_sequence,
        p_receipt_sha256;
END;
$$;

CREATE OR REPLACE FUNCTION opsbench_participant.publish_topology(
    p_participant_id TEXT,
    p_scope_id TEXT,
    p_next_epoch BIGINT,
    p_next_digest BYTEA
)
RETURNS VOID
LANGUAGE plpgsql
AS $$
DECLARE
    head opsbench_participant.participant_scope_head%ROWTYPE;
BEGIN
    IF p_next_digest IS NULL OR octet_length(p_next_digest) <> 32 THEN
        RAISE EXCEPTION 'topology digest must be 32 bytes'
            USING ERRCODE = '22023';
    END IF;

    SELECT scope_head.*
      INTO head
      FROM opsbench_participant.participant_scope_head AS scope_head
     WHERE scope_head.participant_id = p_participant_id
       AND scope_head.scope_id = p_scope_id
     FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'participant scope not found'
            USING ERRCODE = 'P0002';
    END IF;

    IF p_next_epoch <= head.topology_epoch THEN
        RAISE EXCEPTION 'topology epoch must increase monotonically'
            USING ERRCODE = '22023';
    END IF;

    UPDATE opsbench_participant.participant_scope_head
       SET topology_epoch = p_next_epoch,
           topology_digest = p_next_digest,
           updated_at = clock_timestamp()
     WHERE participant_id = p_participant_id
       AND scope_id = p_scope_id;
END;
$$;

-- Default is deny. Deployment grants INSERT on verified_authorization_receipt
-- only to the source-authentication adapter. Participant service roles receive
-- EXECUTE on the functions and SELECT on receipts, not arbitrary ledger writes.
REVOKE ALL ON SCHEMA opsbench_participant FROM PUBLIC;
REVOKE ALL ON ALL TABLES IN SCHEMA opsbench_participant FROM PUBLIC;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA opsbench_participant FROM PUBLIC;

COMMIT;
