-- Bound PostgreSQL outbox failure handling without allowing dead-lettered
-- predecessors to silently release later events for the same incident.

ALTER TABLE ops.outbox_events
  ADD COLUMN last_failure_code text,
  ADD COLUMN last_failure_at timestamptz,
  ADD COLUMN dead_lettered_at timestamptz;

ALTER TABLE ops.outbox_events
  DROP CONSTRAINT outbox_events_status_check;

ALTER TABLE ops.outbox_events
  ADD CONSTRAINT outbox_events_status_check
    CHECK (status IN ('PENDING', 'LEASED', 'DELIVERED', 'DEAD_LETTERED')),
  ADD CONSTRAINT outbox_failure_code_check
    CHECK (last_failure_code IS NULL OR last_failure_code IN (
      'TRANSIENT_NETWORK', 'REMOTE_RATE_LIMITED', 'REMOTE_SERVER_ERROR',
      'REMOTE_ACK_UNKNOWN', 'AUTHENTICATION_REJECTED', 'AUTHORIZATION_REJECTED',
      'DESTINATION_MISMATCH', 'TLS_IDENTITY_REJECTED', 'PAYLOAD_REJECTED',
      'REMOTE_CONTRACT_MISMATCH', 'UNCLASSIFIED', 'ATTEMPT_LIMIT_EXHAUSTED'
    )),
  ADD CONSTRAINT outbox_failure_timestamp_pair_check
    CHECK ((last_failure_code IS NULL) = (last_failure_at IS NULL)),
  ADD CONSTRAINT outbox_dead_letter_timestamp_check
    CHECK ((status = 'DEAD_LETTERED') = (dead_lettered_at IS NOT NULL)),
  ADD CONSTRAINT outbox_dead_letter_requires_failure_check
    CHECK (status <> 'DEAD_LETTERED' OR last_failure_code IS NOT NULL);

CREATE INDEX outbox_dead_lettered_idx
  ON ops.outbox_events (tenant_id, dead_lettered_at, incident_id, sequence_no)
  WHERE status = 'DEAD_LETTERED';

REVOKE UPDATE (status, attempts, lease_owner, lease_until, delivered_at)
  ON ops.outbox_events FROM luminous_ops_app;
REVOKE ALL ON FUNCTION ops.schedule_outbox_retry(text, text, text)
  FROM PUBLIC, luminous_ops_app;

GRANT SELECT ON ops.outbox_events TO luminous_ops_retry_owner;
GRANT UPDATE (
  status, attempts, available_at, lease_owner, lease_until, delivered_at,
  last_failure_code, last_failure_at, dead_lettered_at
) ON ops.outbox_events TO luminous_ops_retry_owner;

CREATE OR REPLACE FUNCTION ops.claim_next_outbox(
  p_tenant_id text, p_worker_id text, p_lease_seconds integer
)
RETURNS TABLE (
  outbox_id text, incident_id text, sequence_no bigint, payload jsonb, attempts integer
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, ops, pg_temp
AS $claim$
BEGIN
  IF p_worker_id IS NULL OR btrim(p_worker_id) = ''
     OR p_lease_seconds IS NULL OR p_lease_seconds NOT BETWEEN 1 AND 3600 THEN
    RAISE EXCEPTION 'invalid outbox lease request' USING ERRCODE = '22023';
  END IF;
  IF p_tenant_id IS NULL OR p_tenant_id IS DISTINCT FROM ops.current_tenant_id() THEN
    RETURN;
  END IF;

  -- A worker can crash without recording failure; the next claimant also
  -- enforces the ceiling against expired attempt-12 leases.
  UPDATE ops.outbox_events AS o
  SET status = 'DEAD_LETTERED',
      last_failure_code = 'ATTEMPT_LIMIT_EXHAUSTED',
      last_failure_at = clock_timestamp(),
      dead_lettered_at = clock_timestamp(),
      lease_owner = NULL,
      lease_until = NULL,
      available_at = clock_timestamp()
  WHERE o.tenant_id = p_tenant_id
    AND o.tenant_id = ops.current_tenant_id()
    AND o.attempts >= 12
    AND (
      o.status = 'PENDING'
      OR (o.status = 'LEASED' AND o.lease_until <= clock_timestamp())
    );

  RETURN QUERY
    WITH candidate AS (
      SELECT o.tenant_id, o.outbox_id
      FROM ops.outbox_events AS o
      WHERE o.tenant_id = p_tenant_id
        AND o.available_at <= clock_timestamp()
        AND o.attempts < 12
        AND (
          o.status = 'PENDING'
          OR (o.status = 'LEASED' AND o.lease_until <= clock_timestamp())
        )
        AND NOT EXISTS (
          SELECT 1
          FROM ops.outbox_events AS prior
          WHERE prior.tenant_id = o.tenant_id
            AND prior.incident_id = o.incident_id
            AND prior.sequence_no < o.sequence_no
            AND prior.status <> 'DELIVERED'
        )
      ORDER BY o.created_at, o.outbox_id
      LIMIT 1
      FOR UPDATE OF o SKIP LOCKED
    ),
    claimed AS (
      UPDATE ops.outbox_events AS o
      SET status = 'LEASED',
          attempts = o.attempts + 1,
          lease_owner = p_worker_id,
          lease_until = clock_timestamp() + make_interval(secs => p_lease_seconds)
      FROM candidate AS c
      WHERE o.tenant_id = c.tenant_id AND o.outbox_id = c.outbox_id
      RETURNING o.outbox_id, o.incident_id, o.sequence_no, o.payload, o.attempts
    )
    SELECT claimed.outbox_id, claimed.incident_id, claimed.sequence_no,
           claimed.payload, claimed.attempts
    FROM claimed;
END
$claim$;

CREATE OR REPLACE FUNCTION ops.acknowledge_outbox(
  p_tenant_id text, p_outbox_id text, p_worker_id text
)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, ops, pg_temp
AS $ack$
BEGIN
  IF p_worker_id IS NULL OR btrim(p_worker_id) = '' THEN
    RAISE EXCEPTION 'invalid outbox worker' USING ERRCODE = '22023';
  END IF;
  UPDATE ops.outbox_events AS o
  SET status = 'DELIVERED',
      delivered_at = clock_timestamp(),
      lease_owner = NULL,
      lease_until = NULL
  WHERE o.tenant_id = p_tenant_id
    AND o.tenant_id = ops.current_tenant_id()
    AND o.outbox_id = p_outbox_id
    AND o.status = 'LEASED'
    AND o.lease_owner = p_worker_id
    AND o.lease_until > clock_timestamp();
  RETURN FOUND;
END
$ack$;

CREATE OR REPLACE FUNCTION ops.record_outbox_failure(
  p_tenant_id text, p_outbox_id text, p_worker_id text, p_failure_code text
)
RETURNS text
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, ops, pg_temp
AS $failure$
DECLARE
  v_attempts integer;
BEGIN
  IF p_worker_id IS NULL OR btrim(p_worker_id) = '' THEN
    RAISE EXCEPTION 'invalid outbox worker' USING ERRCODE = '22023';
  END IF;
  IF p_failure_code IS NULL OR p_failure_code NOT IN (
    'TRANSIENT_NETWORK', 'REMOTE_RATE_LIMITED', 'REMOTE_SERVER_ERROR',
    'REMOTE_ACK_UNKNOWN', 'AUTHENTICATION_REJECTED', 'AUTHORIZATION_REJECTED',
    'DESTINATION_MISMATCH', 'TLS_IDENTITY_REJECTED', 'PAYLOAD_REJECTED',
    'REMOTE_CONTRACT_MISMATCH', 'UNCLASSIFIED'
  ) THEN
    RAISE EXCEPTION 'invalid outbox failure code' USING ERRCODE = '22023';
  END IF;
  IF p_tenant_id IS NULL OR p_tenant_id IS DISTINCT FROM ops.current_tenant_id() THEN
    RETURN 'STALE_LEASE';
  END IF;

  SELECT o.attempts INTO v_attempts
  FROM ops.outbox_events AS o
  WHERE o.tenant_id = p_tenant_id
    AND o.outbox_id = p_outbox_id
    AND o.status = 'LEASED'
    AND o.lease_owner = p_worker_id
    AND o.lease_until > clock_timestamp()
  FOR UPDATE;
  IF NOT FOUND THEN
    RETURN 'STALE_LEASE';
  END IF;

  IF p_failure_code IN (
       'AUTHENTICATION_REJECTED', 'AUTHORIZATION_REJECTED',
       'DESTINATION_MISMATCH', 'TLS_IDENTITY_REJECTED',
       'PAYLOAD_REJECTED', 'REMOTE_CONTRACT_MISMATCH'
     ) OR v_attempts >= 12 THEN
    UPDATE ops.outbox_events AS o
    SET status = 'DEAD_LETTERED',
        last_failure_code = p_failure_code,
        last_failure_at = clock_timestamp(),
        dead_lettered_at = clock_timestamp(),
        available_at = clock_timestamp(),
        lease_owner = NULL,
        lease_until = NULL
    WHERE o.tenant_id = p_tenant_id AND o.outbox_id = p_outbox_id;
    RETURN 'DEAD_LETTERED';
  END IF;

  UPDATE ops.outbox_events AS o
  SET status = 'PENDING',
      last_failure_code = p_failure_code,
      last_failure_at = clock_timestamp(),
      dead_lettered_at = NULL,
      available_at = clock_timestamp() + make_interval(
        secs => LEAST(
          5.0::double precision * power(
            2.0::double precision,
            LEAST(GREATEST(v_attempts - 1, 0), 10)::double precision
          ),
          3600.0::double precision
        )
      ),
      lease_owner = NULL,
      lease_until = NULL
  WHERE o.tenant_id = p_tenant_id AND o.outbox_id = p_outbox_id;
  RETURN 'RESCHEDULED';
END
$failure$;

REVOKE ALL ON FUNCTION ops.claim_next_outbox(text, text, integer) FROM PUBLIC;
REVOKE ALL ON FUNCTION ops.acknowledge_outbox(text, text, text) FROM PUBLIC;
REVOKE ALL ON FUNCTION ops.record_outbox_failure(text, text, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ops.claim_next_outbox(text, text, integer) TO luminous_ops_app;
GRANT EXECUTE ON FUNCTION ops.acknowledge_outbox(text, text, text) TO luminous_ops_app;
GRANT EXECUTE ON FUNCTION ops.record_outbox_failure(text, text, text, text) TO luminous_ops_app;

GRANT CREATE ON SCHEMA ops TO luminous_ops_retry_owner;
ALTER FUNCTION ops.claim_next_outbox(text, text, integer) OWNER TO luminous_ops_retry_owner;
ALTER FUNCTION ops.acknowledge_outbox(text, text, text) OWNER TO luminous_ops_retry_owner;
ALTER FUNCTION ops.record_outbox_failure(text, text, text, text) OWNER TO luminous_ops_retry_owner;
REVOKE CREATE ON SCHEMA ops FROM luminous_ops_retry_owner;

COMMENT ON FUNCTION ops.claim_next_outbox(text, text, integer)
  IS 'Tenant-bound outbox claim with lease recovery, predecessor ordering, and finite delivery attempts.';
COMMENT ON FUNCTION ops.acknowledge_outbox(text, text, text)
  IS 'Acknowledges only a currently owned, unexpired tenant-scoped outbox lease.';
COMMENT ON FUNCTION ops.record_outbox_failure(text, text, text, text)
  IS 'Persists a stable failure code and bounded retry/dead-letter transition for a live lease.';
COMMENT ON TABLE ops.outbox_events
  IS 'At-least-once dispatch queue; dead-lettered predecessors continue to block later events.';
