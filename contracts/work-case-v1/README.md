# Native Work Case Contract V1

**Status:** executable in-memory reference model and portable snapshot schema. This is not a production service, durable store, authorization server, or qualified PSA.

This is the first owned service-management domain slice for a Luminous ConnectWise-class platform. A work case joins the people, resources, service work, lifecycle, external mappings, and evidence references needed to manage an incident, request, problem, or change. External PSA identifiers are mappings, not local primary keys.

## Contents

- schema.json: JSON Schema Draft 2020-12 for the portable work-case snapshot.
- examples/incident-work-case.json: deterministic synthetic fixture.
- work_case_model.py: typed state machine, expected-revision checks, idempotency semantics, tenant-bound access, append-only timeline, evidence references, and external mapping uniqueness.
- tests/test_work_case_model.py: schema validation plus lifecycle, transition-matrix, duplicate/replay, concurrency, cross-tenant, stale revision, external mapping, and evidence-kind tests.

## Local reproduction

From the repository root:

~~~sh
python3 -m venv /tmp/work-case-v1-venv
/tmp/work-case-v1-venv/bin/python -m pip install --no-cache-dir -r contracts/operations-event-v1/requirements.txt
PYTHONDONTWRITEBYTECODE=1 /tmp/work-case-v1-venv/bin/python -m unittest discover -s contracts/work-case-v1/tests -p 'test_*.py' -v
~~~

The JSON Schema validation uses the already pinned jsonschema 4.26.0 dependency. Core state-machine behavior uses Python's standard library.

## Lifecycle rules

| Current state | Allowed next states |
|---|---|
| open | in_progress, waiting, cancelled |
| in_progress | waiting, resolved, cancelled |
| waiting | in_progress, resolved, cancelled |
| resolved | open (reopen with reason), closed |
| closed | none |
| cancelled | none |

Every mutation carries an expected revision, a tenant-bound principal, an idempotency key, command identifier, timestamp, and reason. Activity records include the actor, actor role, time, prior/new revisions, and typed activity label. A stale revision fails without changing the snapshot or timeline. An identical retry returns its original result; reuse of the same idempotency key for different semantics conflicts.

## Authority and data boundaries

- Tenant context comes from Principal, which a future authenticated API must construct. Commands do not accept caller-supplied tenant fields. A real service must authorize every API request and every referenced resource.
- A requester can open a case; lifecycle, assignment, evidence-linking, and external mapping mutations require technician, admin, or configured integration authority in this reference model. This role table is illustrative, not a universal organization policy.
- External provider/company/ticket identifiers remain explicit mappings and never become canonical local case IDs. An identifier already mapped to another local case conflicts; no best-guess matching.
- Evidence references carry a digest, classification, issuer, and evidence kind (observation, customer assertion, simulation, interpretation, or unverified). A digest proves byte identity only. Linking evidence does not verify it, establish its truth, or authorize execution.
- Summaries must be redacted/minimized before entering this boundary. The reference model does not detect secrets or scrub sensitive text.
- No command-execution endpoint or AI-approved state exists. Symthaea can recommend, explain, and prepare work; deterministic authority and the subsystem that owns an operation remain separate.

## Qualification limits

This model is deliberately in-memory. Its lock and idempotency map show reference state-machine behavior and duplicate-command collapse within one process only. They do not prove crash durability, database transaction atomicity, multi-process concurrency, tenant isolation in a real API, cryptographic verification, backup/restore, provider API semantics, SLA accuracy, billing correctness, or live ConnectWise interoperability.

Before production use, the next gates are an independently implemented state-machine oracle; persistent schema/migrations with unique constraints and database-backed idempotency; process-kill/crash/restart and backup/restore tests; tenant-negative tests across list/search/attachments/evidence/export/subscriptions; current-authorization checks including concurrent revocation; audit/retention/export policy; and provider-specific conformance before live synchronization.
