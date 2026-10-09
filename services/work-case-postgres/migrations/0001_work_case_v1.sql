-- Work Case PostgreSQL schema v1. Apply using a privileged migration identity.
CREATE TABLE work_cases (
 tenant_id text NOT NULL CHECK (length(btrim(tenant_id)) BETWEEN 1 AND 128),
 case_id text NOT NULL CHECK (case_id ~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$'),
 kind text NOT NULL CHECK (kind IN ('incident','request','problem','change')),
 title text NOT NULL CHECK (length(btrim(title)) BETWEEN 1 AND 240),
 summary text NOT NULL CHECK (length(btrim(summary)) BETWEEN 1 AND 4000),
 customer_id text NOT NULL CHECK (length(btrim(customer_id)) BETWEEN 1 AND 128),
 site_id text CHECK (site_id IS NULL OR length(btrim(site_id)) BETWEEN 1 AND 128),
 asset_ids text[] NOT NULL DEFAULT '{}',
 priority smallint NOT NULL CHECK (priority BETWEEN 1 AND 5),
 state text NOT NULL DEFAULT 'open' CHECK (state IN ('open','in_progress','waiting','resolved','closed','cancelled')),
 revision bigint NOT NULL DEFAULT 1 CHECK (revision >= 1),
 assignee_id text CHECK (assignee_id IS NULL OR length(btrim(assignee_id)) BETWEEN 1 AND 128),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY (tenant_id, case_id)
);
CREATE TABLE case_activity (
 tenant_id text NOT NULL, case_id text NOT NULL,
 sequence bigint NOT NULL CHECK (sequence >= 1),
 activity_id text NOT NULL, command_id text NOT NULL CHECK (length(btrim(command_id)) BETWEEN 1 AND 128),
 actor_id text NOT NULL, actor_role text NOT NULL CHECK (actor_role IN ('technician','admin','integration','requester')),
 activity_type text NOT NULL, occurred_at timestamptz NOT NULL, reason text NOT NULL,
 prior_revision bigint NOT NULL CHECK (prior_revision >= 0), new_revision bigint NOT NULL CHECK (new_revision >= 1),
 details jsonb NOT NULL DEFAULT '{}'::jsonb,
 PRIMARY KEY (tenant_id, case_id, sequence), UNIQUE (tenant_id, activity_id),
 FOREIGN KEY (tenant_id, case_id) REFERENCES work_cases (tenant_id, case_id)
);
CREATE TABLE command_idempotency (
 tenant_id text NOT NULL, idempotency_key text NOT NULL CHECK (length(btrim(idempotency_key)) BETWEEN 1 AND 200),
 command_digest text NOT NULL CHECK (command_digest ~ '^[0-9a-f]{64}$'),
 result_case_id text NOT NULL, recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY (tenant_id, idempotency_key),
 FOREIGN KEY (tenant_id, result_case_id) REFERENCES work_cases (tenant_id, case_id) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE external_case_mappings (
 tenant_id text NOT NULL, connection_id text NOT NULL CHECK (length(btrim(connection_id)) BETWEEN 1 AND 128),
 provider text NOT NULL CHECK (provider ~ '^[a-z0-9][a-z0-9.-]{0,63}$'),
 external_id text NOT NULL CHECK (length(btrim(external_id)) BETWEEN 1 AND 256),
 case_id text NOT NULL, linked_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 PRIMARY KEY (tenant_id, connection_id, provider, external_id),
 FOREIGN KEY (tenant_id, case_id) REFERENCES work_cases (tenant_id, case_id)
);
CREATE TABLE case_evidence_refs (
 tenant_id text NOT NULL, case_id text NOT NULL,
 evidence_id text NOT NULL CHECK (length(btrim(evidence_id)) BETWEEN 1 AND 128),
 digest_sha256 text NOT NULL CHECK (digest_sha256 ~ '^[0-9a-f]{64}$'),
 classification text NOT NULL CHECK (classification IN ('public','internal','confidential','restricted')),
 evidence_kind text NOT NULL CHECK (evidence_kind IN ('observation','customer_assertion','simulation','interpretation','unverified')),
 issuer text NOT NULL CHECK (length(btrim(issuer)) BETWEEN 1 AND 128),
 created_at timestamptz NOT NULL,
 PRIMARY KEY (tenant_id, case_id, evidence_id),
 FOREIGN KEY (tenant_id, case_id) REFERENCES work_cases (tenant_id, case_id)
);
CREATE TABLE case_outbox (
 tenant_id text NOT NULL, outbox_id uuid NOT NULL, case_id text NOT NULL,
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
CREATE INDEX case_evidence_digest_idx ON case_evidence_refs (tenant_id, digest_sha256);

-- Missing transaction-local tenant context yields NULL and default-denies access.
ALTER TABLE work_cases ENABLE ROW LEVEL SECURITY;
ALTER TABLE work_cases FORCE ROW LEVEL SECURITY;
CREATE POLICY work_cases_tenant_policy ON work_cases USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')) WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), ''));
ALTER TABLE case_activity ENABLE ROW LEVEL SECURITY;
ALTER TABLE case_activity FORCE ROW LEVEL SECURITY;
CREATE POLICY case_activity_tenant_policy ON case_activity USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')) WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), ''));
ALTER TABLE command_idempotency ENABLE ROW LEVEL SECURITY;
ALTER TABLE command_idempotency FORCE ROW LEVEL SECURITY;
CREATE POLICY command_idempotency_tenant_policy ON command_idempotency USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')) WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), ''));
ALTER TABLE external_case_mappings ENABLE ROW LEVEL SECURITY;
ALTER TABLE external_case_mappings FORCE ROW LEVEL SECURITY;
CREATE POLICY external_case_mappings_tenant_policy ON external_case_mappings USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')) WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), ''));
ALTER TABLE case_evidence_refs ENABLE ROW LEVEL SECURITY;
ALTER TABLE case_evidence_refs FORCE ROW LEVEL SECURITY;
CREATE POLICY case_evidence_tenant_policy ON case_evidence_refs USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')) WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), ''));
ALTER TABLE case_outbox ENABLE ROW LEVEL SECURITY;
ALTER TABLE case_outbox FORCE ROW LEVEL SECURITY;
CREATE POLICY case_outbox_tenant_policy ON case_outbox USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')) WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), ''));
