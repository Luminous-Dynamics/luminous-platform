-- Disposable CI test role only; never use this password outside a throwaway database.
CREATE ROLE work_case_app LOGIN PASSWORD 'work_case_test_password'
    NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT;
GRANT CONNECT ON DATABASE work_case_test TO work_case_app;
GRANT USAGE ON SCHEMA public TO work_case_app;
GRANT SELECT, INSERT ON work_cases TO work_case_app;
GRANT SELECT, INSERT ON case_activity TO work_case_app;
GRANT SELECT, INSERT ON command_idempotency TO work_case_app;
GRANT SELECT, INSERT ON external_case_mappings TO work_case_app;
GRANT SELECT, INSERT ON case_evidence_refs TO work_case_app;
GRANT SELECT, INSERT ON case_outbox TO work_case_app;
GRANT UPDATE (status, attempts, lease_owner, lease_until, delivered_at) ON case_outbox TO work_case_app;
