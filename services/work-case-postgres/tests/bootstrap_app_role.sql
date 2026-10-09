-- Disposable CI test role only; never use this password outside a throwaway database.
CREATE ROLE work_case_app LOGIN PASSWORD 'work_case_test_password'
    NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT;
GRANT CONNECT ON DATABASE work_case_test TO work_case_app;
-- Runtime SQL must not be able to create objects that could shadow trusted relations/functions.
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
REVOKE CREATE ON SCHEMA public FROM work_case_app;
GRANT USAGE ON SCHEMA public TO work_case_app;
GRANT SELECT, INSERT ON work_cases TO work_case_app;
GRANT UPDATE (state, revision, updated_at) ON work_cases TO work_case_app;
GRANT SELECT, INSERT ON case_activity TO work_case_app;
GRANT SELECT ON command_idempotency TO work_case_app;
GRANT INSERT (tenant_id, idempotency_key, command_digest, result_case_id) ON command_idempotency TO work_case_app;
GRANT UPDATE (result_payload) ON command_idempotency TO work_case_app;
GRANT SELECT ON external_case_mappings TO work_case_app;
GRANT SELECT ON case_evidence_refs TO work_case_app;
GRANT SELECT, INSERT ON case_outbox TO work_case_app;
-- Dispatcher writes remain ungranted until a separately qualified dispatcher boundary exists.
