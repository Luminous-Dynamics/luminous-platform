-- Work Case PostgreSQL schema v1. Apply with the privileged migration role.
CREATE TABLE work_cases (
 tenant_id uuid NOT NULL, case_id uuid NOT NULL,
 kind text NOT NULL CHECK (kind IN ('INCIDENT','REQUEST','PROBLEM','CHANGE')),
 title text NOT NULL CHECK (length(btrim(title)) BETWEEN 1 AND 300),
 state text NOT NULL DEFAULT 'OPEN' CHECK (state IN ('OPEN','IN_PROGRESS','WAITING','RESOLVED','CLOSED','CANCELLED')),
 revision bigint NOT NULL DEFAULT 1 CHECK (revision >= 1),
 payload jsonb NOT NULL DEFAULT '{}'::jsonb,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY (tenant_id, case_id)
);
CREATE TABLE case_activity (
 tenant_id uuid NOT NULL, case_id uuid NOT NULL, sequence bigint NOT NULL CHECK (sequence >= 1),
 activity_id uuid NOT NULL, command_id uuid NOT NULL, actor_id uuid NOT NULL,
 actor_role text NOT NULL CHECK (actor_role IN ('OPERATOR','INTEGRATION','SYSTEM')),
 activity_type text NOT NULL, reason text NOT NULL,
 prior_revision bigint NOT NULL CHECK (prior_revision >= 0), new_revision bigint NOT NULL CHECK (new_revision >= 1),
 details jsonb NOT NULL DEFAULT '{}'::jsonb, occurred_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY (tenant_id, case_id, sequence), UNIQUE (tenant_id, activity_id),
 FOREIGN KEY (tenant_id, case_id) REFERENCES work_cases (tenant_id, case_id)
);
CREATE TABLE command_idempotency (
 tenant_id uuid NOT NULL, idempotency_key uuid NOT NULL,
 command_digest text NOT NULL CHECK (command_digest ~ '^[0-9a-f]{64}$'),
 result_case_id uuid NOT NULL, recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY (tenant_id, idempotency_key),
 FOREIGN KEY (tenant_id, result_case_id) REFERENCES work_cases (tenant_id, case_id) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE external_case_mappings (
 tenant_id uuid NOT NULL, connection_id uuid NOT NULL,
 provider text NOT NULL CHECK (length(btrim(provider)) BETWEEN 1 AND 100),
 external_id text NOT NULL CHECK (length(btrim(external_id)) BETWEEN 1 AND 300),
 case_id uuid NOT NULL, linked_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY (tenant_id, connection_id, provider, external_id),
 FOREIGN KEY (tenant_id, case_id) REFERENCES work_cases (tenant_id, case_id)
);
CREATE TABLE case_outbox (
 tenant_id uuid NOT NULL, outbox_id uuid NOT NULL, case_id uuid NOT NULL,
 revision bigint NOT NULL CHECK (revision >= 1), event_type text NOT NULL, payload jsonb NOT NULL,
 status text NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING','LEASED','DELIVERED','DEAD_LETTER')),
 attempts integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
 lease_owner uuid, lease_until timestamptz, created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 delivered_at timestamptz, PRIMARY KEY (tenant_id, outbox_id),
 UNIQUE (tenant_id, case_id, revision, event_type),
 FOREIGN KEY (tenant_id, case_id) REFERENCES work_cases (tenant_id, case_id)
);
CREATE INDEX case_activity_timeline_idx ON case_activity (tenant_id, case_id, sequence);
CREATE INDEX case_outbox_ready_idx ON case_outbox (status, lease_until, created_at, tenant_id, outbox_id);
CREATE INDEX case_mappings_case_idx ON external_case_mappings (tenant_id, case_id);

-- Missing transaction-local tenant context yields NULL and therefore default-deny.
ALTER TABLE work_cases ENABLE ROW LEVEL SECURITY;
ALTER TABLE work_cases FORCE ROW LEVEL SECURITY;
CREATE POLICY work_cases_tenant_policy ON work_cases USING
 (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
 WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);
ALTER TABLE case_activity ENABLE ROW LEVEL SECURITY;
ALTER TABLE case_activity FORCE ROW LEVEL SECURITY;
CREATE POLICY case_activity_tenant_policy ON case_activity USING
 (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
 WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);
ALTER TABLE command_idempotency ENABLE ROW LEVEL SECURITY;
ALTER TABLE command_idempotency FORCE ROW LEVEL SECURITY;
CREATE POLICY command_idempotency_tenant_policy ON command_idempotency USING
 (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
 WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);
ALTER TABLE external_case_mappings ENABLE ROW LEVEL SECURITY;
ALTER TABLE external_case_mappings FORCE ROW LEVEL SECURITY;
CREATE POLICY external_case_mappings_tenant_policy ON external_case_mappings USING
 (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
 WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);
ALTER TABLE case_outbox ENABLE ROW LEVEL SECURITY;
ALTER TABLE case_outbox FORCE ROW LEVEL SECURITY;
CREATE POLICY case_outbox_tenant_policy ON case_outbox USING
 (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
 WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);
