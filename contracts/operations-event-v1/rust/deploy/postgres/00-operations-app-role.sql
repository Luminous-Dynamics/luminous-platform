-- Execute as a cluster administrator before applying SQLx schema migrations.
-- This role is intentionally non-login, non-owner, and unable to bypass RLS.
DO $role$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'luminous_ops_app') THEN
    CREATE ROLE luminous_ops_app NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;
  END IF;
END
$role$;

ALTER ROLE luminous_ops_app NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;

-- SECURITY DEFINER routines must not inherit superuser or BYPASSRLS powers
-- merely because the migration was run by a cluster administrator.
DO $retry_role$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'luminous_ops_retry_owner') THEN
    CREATE ROLE luminous_ops_retry_owner NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;
  END IF;
END
$retry_role$;

ALTER ROLE luminous_ops_retry_owner NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;
