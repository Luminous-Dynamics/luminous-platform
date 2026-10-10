CREATE SCHEMA IF NOT EXISTS ops;

CREATE OR REPLACE FUNCTION ops.current_tenant_id()
RETURNS text
LANGUAGE sql
STABLE
AS $fn$
  SELECT NULLIF(current_setting('app.tenant_id', true), '')
$fn$;

CREATE TABLE ops.tenants (
  tenant_id text PRIMARY KEY
    CHECK (tenant_id ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,199}$'),
  created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);

CREATE TABLE ops.connector_connections (
  connection_id text PRIMARY KEY
    CHECK (connection_id ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,199}$'),
  tenant_id text NOT NULL REFERENCES ops.tenants(tenant_id),
  source_uri text NOT NULL CHECK (length(source_uri) BETWEEN 1 AND 512),
  producer_system text NOT NULL
    CHECK (producer_system IN ('xenia-peer', 'xenia-wire', 'sovereign-ops', 'nixward', 'symthaea', 'connectwise-psa', 'other')),
  revision_mode text NOT NULL DEFAULT 'opaque'
    CHECK (revision_mode IN ('opaque', 'numeric_suffix')),
  revision_prefix text,
  enabled boolean NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (connection_id, tenant_id),
  CHECK (
    (revision_mode = 'opaque' AND revision_prefix IS NULL)
    OR (revision_mode = 'numeric_suffix' AND revision_prefix IS NOT NULL AND length(revision_prefix) > 0)
  )
);

CREATE TABLE ops.resource_mappings (
  tenant_id text NOT NULL,
  connection_id text NOT NULL,
  external_company_id text NOT NULL CHECK (length(external_company_id) BETWEEN 1 AND 200),
  external_resource_type text NOT NULL CHECK (length(external_resource_type) BETWEEN 1 AND 64),
  external_resource_id text NOT NULL CHECK (length(external_resource_id) BETWEEN 1 AND 200),
  local_incident_id text NOT NULL CHECK (length(local_incident_id) BETWEEN 1 AND 200),
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  PRIMARY KEY (tenant_id, connection_id, external_company_id, external_resource_type, external_resource_id),
  FOREIGN KEY (connection_id, tenant_id)
    REFERENCES ops.connector_connections(connection_id, tenant_id)
);

CREATE TABLE ops.incident_heads (
  tenant_id text NOT NULL REFERENCES ops.tenants(tenant_id),
  incident_id text NOT NULL CHECK (length(incident_id) BETWEEN 1 AND 200),
  source_connection_id text,
  source_company_id text,
  source_resource_type text,
  source_resource_id text,
  current_revision text,
  state_digest text CHECK (state_digest IS NULL OR state_digest ~ '^[0-9a-f]{64}$'),
  state_payload jsonb,
  outbox_sequence bigint NOT NULL DEFAULT 0 CHECK (outbox_sequence >= 0),
  updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  PRIMARY KEY (tenant_id, incident_id),
  CHECK (
    (current_revision IS NULL AND state_digest IS NULL AND state_payload IS NULL
      AND source_connection_id IS NULL AND source_company_id IS NULL
      AND source_resource_type IS NULL AND source_resource_id IS NULL
      AND outbox_sequence = 0)
    OR
    (current_revision IS NOT NULL AND state_digest IS NOT NULL AND state_payload IS NOT NULL
      AND source_connection_id IS NOT NULL AND source_company_id IS NOT NULL
      AND source_resource_type IS NOT NULL AND source_resource_id IS NOT NULL)
  ),
  FOREIGN KEY (source_connection_id, tenant_id)
    REFERENCES ops.connector_connections(connection_id, tenant_id),
  FOREIGN KEY (
    tenant_id, source_connection_id, source_company_id,
    source_resource_type, source_resource_id
  ) REFERENCES ops.resource_mappings (
    tenant_id, connection_id, external_company_id,
    external_resource_type, external_resource_id
  )
);

CREATE TABLE ops.inbox_events (
  tenant_id text NOT NULL,
  connection_id text NOT NULL,
  source_uri text NOT NULL CHECK (length(source_uri) BETWEEN 1 AND 512),
  event_id text NOT NULL CHECK (length(event_id) BETWEEN 1 AND 200),
  content_digest text NOT NULL CHECK (content_digest ~ '^[0-9a-f]{64}$'),
  outcome text NOT NULL DEFAULT 'PROCESSING' CHECK (outcome IN (
    'PROCESSING', 'ACCEPTED', 'DUPLICATE_EVENT', 'DUPLICATE_EFFECT',
    'DUPLICATE_REVISION', 'STALE_REVISION', 'QUARANTINED'
  )),
  received_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  PRIMARY KEY (tenant_id, connection_id, source_uri, event_id),
  FOREIGN KEY (connection_id, tenant_id)
    REFERENCES ops.connector_connections(connection_id, tenant_id)
);

CREATE TABLE ops.business_effects (
  tenant_id text NOT NULL,
  connection_id text NOT NULL,
  idempotency_key text NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 200),
  semantic_digest text NOT NULL CHECK (semantic_digest ~ '^[0-9a-f]{64}$'),
  source_uri text NOT NULL,
  event_id text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  PRIMARY KEY (tenant_id, connection_id, idempotency_key),
  FOREIGN KEY (connection_id, tenant_id)
    REFERENCES ops.connector_connections(connection_id, tenant_id),
  FOREIGN KEY (tenant_id, connection_id, source_uri, event_id)
    REFERENCES ops.inbox_events(tenant_id, connection_id, source_uri, event_id)
);

CREATE TABLE ops.incident_activity (
  tenant_id text NOT NULL,
  activity_id text NOT NULL CHECK (activity_id ~ '^[0-9a-f]{64}$'),
  incident_id text NOT NULL,
  connection_id text NOT NULL,
  source_uri text NOT NULL,
  event_id text NOT NULL,
  revision text NOT NULL,
  state_digest text NOT NULL CHECK (state_digest ~ '^[0-9a-f]{64}$'),
  recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  PRIMARY KEY (tenant_id, activity_id),
  FOREIGN KEY (tenant_id, incident_id)
    REFERENCES ops.incident_heads(tenant_id, incident_id),
  FOREIGN KEY (connection_id, tenant_id)
    REFERENCES ops.connector_connections(connection_id, tenant_id),
  FOREIGN KEY (tenant_id, connection_id, source_uri, event_id)
    REFERENCES ops.inbox_events(tenant_id, connection_id, source_uri, event_id)
);

CREATE TABLE ops.outbox_events (
  tenant_id text NOT NULL,
  outbox_id text NOT NULL CHECK (outbox_id ~ '^[0-9a-f]{64}$'),
  incident_id text NOT NULL,
  connection_id text NOT NULL,
  sequence_no bigint NOT NULL CHECK (sequence_no > 0),
  source_uri text NOT NULL,
  source_event_id text NOT NULL,
  payload jsonb NOT NULL,
  status text NOT NULL DEFAULT 'PENDING'
    CHECK (status IN ('PENDING', 'LEASED', 'DELIVERED')),
  attempts integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
  lease_owner text,
  lease_until timestamptz,
  available_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  delivered_at timestamptz,
  PRIMARY KEY (tenant_id, outbox_id),
  UNIQUE (tenant_id, incident_id, sequence_no),
  FOREIGN KEY (tenant_id, incident_id)
    REFERENCES ops.incident_heads(tenant_id, incident_id),
  FOREIGN KEY (connection_id, tenant_id)
    REFERENCES ops.connector_connections(connection_id, tenant_id),
  FOREIGN KEY (tenant_id, connection_id, source_uri, source_event_id)
    REFERENCES ops.inbox_events(tenant_id, connection_id, source_uri, event_id),
  CHECK (
    (status = 'LEASED' AND lease_owner IS NOT NULL AND lease_until IS NOT NULL)
    OR
    (status <> 'LEASED' AND lease_owner IS NULL AND lease_until IS NULL)
  ),
  CHECK ((status = 'DELIVERED') = (delivered_at IS NOT NULL))
);

CREATE TABLE ops.quarantined_events (
  quarantine_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  tenant_id text NOT NULL,
  connection_id text NOT NULL,
  source_uri text NOT NULL,
  event_id text NOT NULL,
  event_digest text NOT NULL CHECK (event_digest ~ '^[0-9a-f]{64}$'),
  reason_code text NOT NULL CHECK (reason_code ~ '^[A-Z][A-Z0-9_]{1,79}$'),
  recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (tenant_id, connection_id, source_uri, event_id, event_digest, reason_code),
  FOREIGN KEY (connection_id, tenant_id)
    REFERENCES ops.connector_connections(connection_id, tenant_id),
  FOREIGN KEY (tenant_id, connection_id, source_uri, event_id)
    REFERENCES ops.inbox_events(tenant_id, connection_id, source_uri, event_id)
);

CREATE INDEX outbox_claimable_idx
  ON ops.outbox_events (tenant_id, available_at, created_at, outbox_id)
  WHERE status IN ('PENDING', 'LEASED');

CREATE INDEX inbox_received_idx
  ON ops.inbox_events (tenant_id, received_at);

CREATE INDEX activity_incident_idx
  ON ops.incident_activity (tenant_id, incident_id, recorded_at, activity_id);

DO $rls$
DECLARE
  table_name text;
BEGIN
  FOREACH table_name IN ARRAY ARRAY[
    'tenants', 'connector_connections', 'resource_mappings', 'incident_heads',
    'inbox_events', 'business_effects', 'incident_activity', 'outbox_events',
    'quarantined_events'
  ]
  LOOP
    EXECUTE format('ALTER TABLE ops.%I ENABLE ROW LEVEL SECURITY', table_name);
    EXECUTE format('ALTER TABLE ops.%I FORCE ROW LEVEL SECURITY', table_name);
    EXECUTE format(
      'CREATE POLICY tenant_isolation ON ops.%I USING (tenant_id = ops.current_tenant_id()) WITH CHECK (tenant_id = ops.current_tenant_id())',
      table_name
    );
  END LOOP;
END
$rls$;

GRANT USAGE ON SCHEMA ops TO luminous_ops_app, luminous_ops_retry_owner;
GRANT EXECUTE ON FUNCTION ops.current_tenant_id() TO luminous_ops_app, luminous_ops_retry_owner;

-- Read access is tenant-filtered by RLS. Mutation privileges are column-scoped
-- so runtime code cannot rewrite identity/digest/payload fields after insertion.
GRANT SELECT ON ops.tenants, ops.connector_connections, ops.resource_mappings,
  ops.incident_heads, ops.inbox_events, ops.business_effects, ops.incident_activity,
  ops.outbox_events, ops.quarantined_events TO luminous_ops_app;

GRANT INSERT (tenant_id, connection_id, source_uri, event_id, content_digest)
  ON ops.inbox_events TO luminous_ops_app;
GRANT UPDATE (outcome) ON ops.inbox_events TO luminous_ops_app;

GRANT INSERT (tenant_id, connection_id, idempotency_key, semantic_digest, source_uri, event_id)
  ON ops.business_effects TO luminous_ops_app;

-- An incident head must first be inserted as an empty placeholder. The store
-- then updates mutable state only after locking the row and proving mappings.
GRANT INSERT (tenant_id, incident_id) ON ops.incident_heads TO luminous_ops_app;
GRANT UPDATE (
  source_connection_id, source_company_id, source_resource_type, source_resource_id,
  current_revision, state_digest, state_payload, outbox_sequence, updated_at
) ON ops.incident_heads TO luminous_ops_app;

GRANT INSERT (
  tenant_id, activity_id, incident_id, connection_id, source_uri, event_id, revision, state_digest
) ON ops.incident_activity TO luminous_ops_app;

GRANT INSERT (
  tenant_id, outbox_id, incident_id, connection_id, sequence_no, source_uri, source_event_id, payload
) ON ops.outbox_events TO luminous_ops_app;
GRANT UPDATE (status, attempts, lease_owner, lease_until, delivered_at)
  ON ops.outbox_events TO luminous_ops_app;

-- The SECURITY DEFINER owner is a separate NOLOGIN/NOBYPASSRLS role, not the
-- cluster administrator that commonly applies migrations. Keep its direct
-- table rights scoped to the exact update the retry function performs.
GRANT SELECT ON ops.outbox_events TO luminous_ops_retry_owner;
GRANT UPDATE (status, available_at, lease_owner, lease_until)
  ON ops.outbox_events TO luminous_ops_retry_owner;

-- Runtime cannot directly mutate available_at. This narrowly granted function
-- only reschedules the current tenant's still-live lease held by the named
-- worker and applies a database-controlled exponential retry schedule.
CREATE OR REPLACE FUNCTION ops.schedule_outbox_retry(
  p_tenant_id text,
  p_outbox_id text,
  p_worker_id text
)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, ops, pg_temp
AS $retry$
BEGIN
  UPDATE ops.outbox_events
  SET status = 'PENDING',
      available_at = clock_timestamp() + make_interval(
        secs => LEAST(
          5.0::double precision * power(
            2.0::double precision,
            LEAST(GREATEST(attempts - 1, 0), 10)::double precision
          ),
          3600.0::double precision
        )
      ),
      lease_owner = NULL,
      lease_until = NULL
  WHERE tenant_id = p_tenant_id
    AND tenant_id = ops.current_tenant_id()
    AND outbox_id = p_outbox_id
    AND status = 'LEASED'
    AND lease_owner = p_worker_id
    AND lease_until > clock_timestamp();

  RETURN FOUND;
END
$retry$;

REVOKE ALL ON FUNCTION ops.schedule_outbox_retry(text, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ops.schedule_outbox_retry(text, text, text) TO luminous_ops_app;

-- Never leave this SECURITY DEFINER routine owned by the privileged migration
-- connection. The deploy/migration principal must be permitted to SET ROLE to
-- luminous_ops_retry_owner (or be a cluster administrator) for this ownership
-- transfer. CREATE is temporary for the ownership change and then removed.
GRANT CREATE ON SCHEMA ops TO luminous_ops_retry_owner;
ALTER FUNCTION ops.schedule_outbox_retry(text, text, text) OWNER TO luminous_ops_retry_owner;
REVOKE CREATE ON SCHEMA ops FROM luminous_ops_retry_owner;

GRANT INSERT (
  tenant_id, connection_id, source_uri, event_id, event_digest, reason_code
) ON ops.quarantined_events TO luminous_ops_app;
GRANT USAGE, SELECT ON SEQUENCE ops.quarantined_events_quarantine_id_seq TO luminous_ops_app;

COMMENT ON SCHEMA ops IS 'Sovereign Operations PostgreSQL state; tenant-scoped and row-level security protected.';
COMMENT ON TABLE ops.inbox_events IS 'Stores event identity and digest only; never stores the raw inbound CloudEvent.';
COMMENT ON TABLE ops.business_effects IS 'Durable idempotency fence per connector and canonical tenant.';
COMMENT ON TABLE ops.incident_activity IS 'Append-only, minimized provenance for accepted local state changes.';
COMMENT ON TABLE ops.outbox_events IS 'At-least-once dispatch queue; sequence is serialized per incident head row.';
COMMENT ON TABLE ops.quarantined_events IS 'Minimal quarantine receipts: digest, reason code, and timestamp only.';
