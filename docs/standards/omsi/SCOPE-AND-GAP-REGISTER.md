# OMSI scope and gap register — 0.1

**Status:** design note; not adopted. Updated 2026-10-12.

## Proposed core

The initial core is a transaction/authority/evidence kernel, not an end-to-end MSP product schema.

| Capability | Intended contract | Evidence / limitation |
|---|---|---|
| Operation manifest | Tenant/target binding, action profile, immutable intent, preconditions, expiry, recovery reference | Draft record and semantic checks exist in the local starter kit; no live executor |
| Authority grant | Exact manifest digest, issuer, executor, tenant/resource, audience, policy, approvals, nonce and validity | Local checks cover key binding; no signature/trust-store/revocation implementation |
| Lifecycle events | Stable event ID, sequence, correlation, timestamp and legal transitions | Local state-machine checks; no durable event store or production delivery service |
| Evidence receipt | Operation/target binding, post-state observations, verifier, method, time and limits | Local checks require post-state evidence for verified outcome; cannot prove observation truth |
| NixOS change profile | Pinned source/lock, host target, mode, desired closure and recovery reference | Schema-only draft; no VM application/recovery test |
| IT service profile | Tenant-scoped resource inventory and minimal case handoff | Incubation only; no live asset synchronization, PSA connector or complete ITSM semantics |
| HTTP binding | Explicit propose/grant/execute/cancel separation | OpenAPI draft exists in local kit; no live server/client contract test or dedicated OpenAPI validator |

The executable starter and test report were built locally during proposal development. They are design evidence only, have not been independently reviewed, and are not a certification. This RFC-only PR does not include the full executable kit; an implementation PR should add it as reviewable source, schemas and fixtures after this proposal's scope is reviewed.

## Explicitly out of scope for Core 0.1

- Customer onboarding, provider organization and contract lifecycle.
- Inventory reconciliation, complete service catalog, entitlements, maintenance windows and service-instance lifecycle.
- Full incidents/requests/problems/changes workflows and universal PSA mappings.
- SLA clock semantics, pause/exclusion rules, credits, invoicing, procurement and license reconciliation.
- Backup catalog, immutable storage, restore orchestration and measured recovery objectives.
- Production RMM agents or Windows/macOS/Linux endpoint parity.
- Identity-provider connectors, EDR integrations, cloud and network-device adapters.
- Cryptographic identity, authorization signatures, key discovery/rotation/revocation and provider federation.
- Production reliability, availability, disaster recovery or safety certification.

These are deliberate scope boundaries, not existing capabilities. Implementations must describe tested profiles accurately and must not claim universal MSP replacement from passing core tests.

## Proposed sequencing

1. **OMSI Core:** tenant/resource binding, capability discovery, operation manifests, grants, lifecycle events, evidence receipts and errors.
2. **Service Model:** Provider, Tenant, Contact, Site, ManagedResource, ServiceOffering, ServiceInstance, Entitlement, MaintenanceWindow and ServiceObjective, reusing adjacent models.
3. **Case Interchange:** compare cross-vendor examples and existing ticket standards; standardize only a loss-aware cross-provider minimum.
4. **Operations:** Nixward reference executor in disposable VMs, exact authorization, durable replay/idempotency state, recovery and verification.
5. **Domain profiles:** observability, cybersecurity, backup, identity, networking/PBX, cloud, endpoints and compliance.
6. **Federation:** scoped provider delegation, revocation, provenance, export/import and customer exit after identity and operations semantics harden.

## Gates before a 1.0 claim

1. Two independent implementations consume identical fixtures without shared private types.
2. Canonicalization and cryptographic vectors pass independent review.
3. A disposable-VM operation demonstrates planning, validation, authorization, apply, verification and recovery.
4. Authenticated cross-tenant tests cover principal mapping and resource policy.
5. Fault injection covers timeout after acceptance, retry, partial apply, cancellation race, revoked delegation and stale target.
6. A customer exports relevant records/evidence into a second implementation.
7. Independent security review, name clearance and open governance are in place.

## Prior-art conclusion

Adjacent standards exist for identity provisioning, audience restriction, telemetry, cybersecurity commands, topology, hardware management, security controls and update trust. This supports reuse-first design but does not prove OMSI is novel. The proposed gap must be validated with independent MSPs, PSA/RMM vendors, customers and security reviewers.
