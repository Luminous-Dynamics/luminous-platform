# RFC: Xenia Operations Fabric — a sovereign, AI-native alternative to the MSP stack

**Status:** Proposed architecture; no runtime implementation or production-readiness claim  
**Date:** 2026-10-09  
**Scope:** Xenia, Sovereign Ops, Nixward, Luminous Platform, and a future service-management/integration component  
**Decision sought:** Align the product boundary and integration contracts before adding more cross-repository features

## Executive decision

Build toward a better ConnectWise in two compatible steps:

1. **First, make Xenia a better operational spine for an MSP.** Join a ticket or alert to the right customer and asset, perform consent-bound remote support, capture independently verifiable evidence, verify the result, and synchronize the resulting work record back to the existing PSA.
2. **Then replace PSA capabilities incrementally**, only after the shared workflow is reliable and actual users validate the service desk, contracts, SLAs, timekeeping, billing, and reporting.

Do not attempt a feature-for-feature ConnectWise clone before proving that loop. Do not place the PSA data model in `xenia-peer` or overload `sovereign-ops`. Xenia is the privileged remote-session and evidence layer; Sovereign Ops is the governance/policy/operational-intent layer; Nixward is the NixOS realization and verification layer. A separate service-management plane should own customer-facing work and business records.

This architecture is an evolution path, not a claim that Xenia is production-ready. The [xenia-peer README](https://github.com/Luminous-Dynamics/xenia-peer) and [xenia-wire README](https://github.com/Luminous-Dynamics/xenia-wire) currently identify the relevant components as pre-alpha/non-production.

## Why compete, and where the opening is

ConnectWise describes a broad MSP suite: PSA capabilities such as ticketing, SLAs, agreements, project management, time tracking, and billing, alongside RMM features such as monitoring, patching, scripting, reporting, and remote access. Its integration marketplace and APIs are also a major part of the product, not an optional afterthought.

We should compete on the part that can be meaningfully better—not merely recreate every screen:

- **Evidence-linked service delivery:** each important diagnosis, approval, remote action, verification result, and time record traces back to the same incident and asset.
- **Safer automation:** AI may detect, summarize, recommend, and prepare plans; policy and explicit authority decide what may execute.
- **Customer-verifiable operations:** customers can see who accessed which asset, under what approved scope, what actions were requested, what was actually observed, and how the result was verified.
- **Sovereign deployment choices:** self-hosting, data minimization, portable records, and clear export/migration paths should be product properties.
- **Interoperability by design:** first-class adapters let teams adopt Xenia without replacing their PSA, RMM, identity, or billing systems on day one.

The security model is central to the product opportunity. CISA and partner agencies warn that attackers abuse legitimate remote-access and RMM tools, and recommend auditing remote-access software, securing remote access, monitoring it, and segmenting environments. That makes an explainable authorization and evidence trail a real operational requirement—not just a compliance feature.

## Repository boundaries

| Component | Owns | Must not become |
|---|---|---|
| [xenia-wire](https://github.com/Luminous-Dynamics/xenia-wire) | Versioned sealed-envelope protocol and wire-level conformance | Ticket database, CRM, billing engine, or source of organizational authority |
| [xenia-peer](https://github.com/Luminous-Dynamics/xenia-peer) | Peer sessions, transport/session policy, operator authorization, consent, remote interaction, session evidence | The authoritative customer, contract, ticket, or invoice system |
| [sovereign-ops](https://github.com/Luminous-Dynamics/sovereign-ops) | Operational intent, policy/governance, fleet-facing workflows, attestation/provenance and controlled execution boundaries | A grab-bag that absorbs all PSA/CRM/finance domains |
| [nixward](https://github.com/Luminous-Dynamics/nixward) | NixOS observation and evidence-bound system transactions: observe → plan → validate → authorize → snapshot → apply → verify → promote/recover | A universal cross-platform RMM agent or ticket master |
| [luminous-edge](https://github.com/Luminous-Dynamics/luminous-edge) / [luminous-platform](https://github.com/Luminous-Dynamics/luminous-platform) | Deployment composition and system-level architecture | An alternate authorization service or a requirement for host boot |
| **Future service-management plane (name/repository TBD)** | Tenants, customers, sites, contacts, assets/CMDB, incidents/requests/changes, service agreements, SLA clocks, technician work/time, customer portal, integrations, reports, and eventually billing | A controller that can execute privileged actions simply because a ticket, webhook, or model suggested one |
| Symthaea | Advisory reasoning, correlation, recommendations, operator assistance | A source of executable authority or a replacement for deterministic policy |

Mycelix and other identity/trust systems can later supply optional integrations. The first operational loop must not depend on a wholesale migration to them or on a particular distributed-ledger substrate.

## The first end-to-end workflow

The smallest valuable workflow is **incident → approved support → verified outcome → synchronized work record**:

1. A monitoring alert or customer request creates or updates an incident.
2. The service-management plane resolves the tenant, customer, site, asset, service entitlement, and applicable SLA. Unknown or conflicting mappings remain unresolved rather than being guessed.
3. Xenia establishes a session only after peer authentication and the required consent/authorization checks. The operator receives the minimum scoped capabilities for that session.
4. The operator or an advisory assistant gathers bounded diagnostics. A suggested fix becomes a concrete plan; it is not executable just because a model recommended it.
5. For a consequential action, approval binds the exact plan digest, target asset, permitted operations, approver, and expiry. Revocation or scope change invalidates the relevant authority.
6. Xenia and the execution component record observed outcomes. A session evidence artifact references the session identity, consent/authorization state, actions, timestamps, hashes, redactions, and termination/revocation result without containing raw key material.
7. A verifier checks the exported evidence independently of the original daemon. The ticket is updated with concise, redacted evidence references and the verified result.
8. Time entries are produced from explicit, reviewable work records—not inferred from a session's wall-clock duration alone. SLA calculations consume trustworthy event timestamps and documented pause/clock rules.
9. Closure and billing remain separate policy decisions. A successful command does not automatically mean the incident is resolved, the customer accepts the result, or the work is billable.

The key product insight is that the remote session should be a typed part of the incident lifecycle, rather than a separate button that leaves technicians to copy/paste notes into another system.

## Shared integration contract

Cross-repository integrations should use small, versioned contracts and adapters, not shared internal implementation assumptions.

### Resource identity

At minimum, the service-management plane needs stable, opaque IDs for:

- tenant, customer, site, contact, and technician;
- managed asset and asset enrollment/ownership;
- incident/request/problem/change;
- service agreement, entitlement, SLA policy, and service window;
- session, approval, action plan, action result, evidence artifact;
- time entry, integration connection, and external-system mapping.

Every resource has an owning tenant. Every operation checks authorization against both the operation and the resolved resource/tenant relationship; a user-supplied ID or a UI-hidden button is never sufficient authorization.

External system IDs belong in explicit mapping records alongside local IDs. Do not use ConnectWise IDs as the permanent internal primary key.

### Event envelope

Use the **CloudEvents 1.0 structured JSON event format** as the interoperable base rather than inventing a bespoke common envelope. The first local profile is under [`contracts/operations-event-v1/`](../contracts/operations-event-v1/README.md), validated by JSON Schema Draft 2020-12.

The profile uses CloudEvents core attributes (`specversion`, `id`, `source`, `type`, `subject`, `time`, `datacontenttype`, `dataschema`) and a small allowlisted set of Luminous extensions for canonical tenant identity, business idempotency, first-observation time, correlation/causation, producer metadata, actor metadata, and an optional authorization-evidence reference. Domain fields and classified evidence references live in `data` and are validated against the exact URI in `dataschema`.

Use documented JSON Schema/OpenAPI boundaries for service APIs and event payloads; avoid making Rust struct layout or bincode field order the business integration contract. Consumers should deduplicate CloudEvents by `source + id`, and separately enforce durable idempotency for business effects with the authenticated connection, canonical tenant, and `idempotencykey`. CloudEvents `time` is producer-reported occurrence time; `observedat` records the first trusted ingestion boundary. Neither establishes globally trusted time by itself.

Cryptographic signatures, where needed, must sign a separately specified canonical representation. Never treat the CloudEvents envelope, `source` URI, producer label, or event signature alone as proof that the producer is authorized or that an action may execute. This local profile intentionally rejects unknown extension fields; external events must be normalized through an explicit adapter.

### Delivery and adapter behavior

- At-least-once delivery is expected; every consumer must be idempotent.
- Use a durable outbox or equivalent so a committed business change cannot silently lose its event.
- Authenticate and validate webhooks; bound payload size, retries, and processing time.
- Support replay-safe cursors, duplicate detection, reconciliation jobs, retry limits, and a visible dead-letter state.
- Record the source system and fields owned by each connector to prevent echo loops and destructive last-writer-wins behavior.
- Preserve the original external ID, event reference, and error detail for support and audit.
- Treat imported data and external events as untrusted input, even after transport authentication.
- Never put credentials, session keys, raw secrets, or sensitive screen/audio contents in an integration event.

A connector failure should reduce synchronization confidence visibly. It must not silently grant authority, erase local evidence, or make a privileged operation appear successful.

### Concrete contract slice now present in this RFC branch

The first machine-readable seam is under [`contracts/operations-event-v1/`](../contracts/operations-event-v1/README.md):

- JSON Schema Draft 2020-12 for a strict CloudEvents 1.0 structured-JSON profile;
- the first vendor-neutral normalized incident-snapshot payload schema;
- a synthetic CloudEvent fixture only (not real customer data), tested against both envelope and payload schemas;
- adversarial tests for missing tenant identity, malformed correlation IDs, unknown execution-like fields, malformed evidence digest, unsupported producer labels, invalid schema URIs, invalid actor kinds, malformed timestamps, missing incident summaries, extra payload fields, unsupported CloudEvents versions, and malformed runtime source values;
- a synthetic, non-production adapter model requiring explicit company→tenant and ticket→incident mappings, preserving the source company ID and adapter-defined revision token;
- receiver-side revalidation of source system/resource type, company→tenant, and company+ticket→local-incident mappings before an event may enter the inbox;
- an in-memory inbox reference state machine with adversarial cases for replay despite a later observedat timestamp, event-ID conflict, business idempotency conflict, tenant/source/actor mismatch, source-system/resource-type mismatch, malformed runtime values, per-tenant idempotency scope, forged tenant/resource pairings, and unmapped source companies;
- a PR CI workflow that discovers and runs both schema and adapter-model tests using a pinned checkout action.

The normalized type is `io.luminousdynamics.ops.incident.snapshot.v1`, avoiding the incorrect implication that every source revision is a creation event. The profile separates CloudEvents `time` from the Luminous `observedat` extension, requires canonical `tenantid`, carries actor and optional authority-evidence references as metadata rather than grants, and keeps classified evidence references inside the event-specific `data` schema. The schema accepts no command-execution field.

**Qualification boundary:** the suite validates envelope/payload schemas, synthetic normalization and trusted mapping, an in-memory idempotency model, and a separate SQLite reference model for atomic local inbox/state/outbox writes, replay/conflict semantics, transaction fault injection, concurrent duplicate submissions, conservative revision ordering, and retryable outbox leases. These results apply only to the tested local model. They do not validate actual ConnectWise webhooks, authenticate a provider, prove tenant isolation for a production database, establish distributed multi-writer correctness, prove power-loss behavior, verify evidence signatures, or execute an end-to-end adapter. Those remain separate gates.

## ConnectWise-first integration without vendor lock-in

Implement ConnectWise as the first PSA adapter because it validates whether the architecture can coexist with the incumbent platform while leaving a path to native replacement.

Initial mapping:

| ConnectWise concept | Internal concept | Initial policy |
|---|---|---|
| Company / contact | Customer / contact | Explicit external-ID mapping; ambiguous matches require human resolution |
| Configuration / device | Managed asset | Track source and freshness; do not silently overwrite conflicting ownership |
| Service ticket | Incident/request | Preserve both IDs and use idempotent upsert semantics |
| Agreement / SLA | Service entitlement / SLA policy | Import for visibility and clock calculation; keep one declared authoritative owner |
| Time entry | Technician work record | Sync only after a reviewable source record exists |
| Session/evidence link | Session + evidence references | Redacted by default; link back to independently verifiable material |
| Invoice / accounting data | Future finance adapter | Out of first release; no dual-write finance system |

Use least-privilege integration credentials, separate read permissions from write permissions where the provider supports it, and give each customer/tenant a clearly isolated connection. Build a sandbox/test adapter before using any real customer account.

Do not implement one guessed "ConnectWise authentication" path for every product surface. The current ConnectWise Developer Network says API documentation/access and test environments are product-specific and onboarding may be approval-gated ([Getting Started](https://developer.connectwise.com/Best_Practices/Getting_Started)). A separate ConnectWise Cloud Services guide documents OAuth 2.0 client credentials, subscription-key handling, bearer-token reuse, and strict authentication-endpoint rate limits ([Authentication](https://developers.cloudservices.connectwise.com/Guides/Authentication)). That Cloud Services page is not sufficient evidence that the selected PSA endpoints use the same mechanism. Before adapter code is written, record the exact product, API family/base URL, approved authentication scheme, token lifetime/cache policy, permission scope, pagination/filter semantics, rate-limit behavior, and retry/conditional-update support. Treat credentials as secrets and never store them in event payloads or the browser.

The first release should **read enough to locate the incident and asset, then write a minimal status/note/evidence update back**. It should not attempt to synchronize every configurable field or become a bidirectional finance system. Define field ownership and conflict resolution before adding more write operations.

When a native service desk eventually exists, customers should be able to migrate resource and history records with an exportable manifest, preserving external IDs and evidence hashes. Keep the ConnectWise adapter as an interoperability option rather than a temporary hack that must be deleted.

## Security and safety invariants

1. **Tenant isolation is server-side.** Test cross-tenant reads, writes, attachments, search, exports, and event subscription—not just API happy paths.
2. **Authentication is not authorization.** A valid technician, API token, session, or vendor webhook does not automatically authorize an action on every asset.
3. **Consent, policy approval, and contractual entitlement are distinct.** A customer may permit a remote session without authorizing an unbounded script; a ticket may exist without implying asset ownership or contract coverage.
4. **Capabilities are scoped, directional, short-lived, and revocable.** Bind them to the exact session/asset and action or lane. Re-check authority at the mutation boundary, including after concurrent revocation.
5. **Automation is non-authoritative by default.** AI can triage and draft plans. A deterministic policy plus the required human/customer approval gates execution. Destructive actions require explicit, auditable approval and a recovery plan.
6. **Evidence is tamper-evident and independently verifiable.** Keep signature validity, trusted identity, authorization, freshness, completeness, and policy acceptance as separate claims.
7. **Audit data is useful but minimized.** Store event metadata and references by default; protect sensitive payloads separately; support redaction without pretending redaction preserves evidence of removed content.
8. **Recovery is explicit.** Define timeout, partial failure, cancellation, revocation, rollback, and offline behavior. Visibility and health reporting may degrade gracefully; privileged actuation must fail closed when required authority or evidence is unavailable.
9. **No hidden or unbounded remote execution.** No webhook, LLM output, imported ticket text, or synchronization conflict can be interpreted as an executable command.
10. **Protocol maturity remains honest.** A successful test, optional PQC algorithm, or internal implementation diversity is not an independent security audit or production qualification.

These invariants align with the explicit capability and offline-session-evidence work already tracked in [xenia-peer #449](https://github.com/Luminous-Dynamics/xenia-peer/issues/449) and [xenia-peer #450](https://github.com/Luminous-Dynamics/xenia-peer/issues/450). They should be composed with existing Xenia work, not reimplemented as a competing authority system.

## Delivery sequence and gates

### Gate 0 — Establish trust boundaries

- Reconcile the live status of existing session-capability and session-evidence work before building consumers on top of it.
- Resolve placeholder/unproven proof and signature output in Sovereign Ops; only exact, executed, correctly scoped evidence may be represented as a pass.
- Freeze the first resource IDs, event envelope, API schemas, and connector field-ownership rules.
- Create a threat model for tenant isolation, webhook abuse, credential leakage, cross-tenant search, session replay/revocation, and confused-deputy behavior.
- Keep existing pre-alpha/no-production claims visible.

**Exit evidence:** schema validators, threat-model review, negative authorization cases, deterministic fixtures, and a recorded list of unsupported claims.

### Gate 1 — Prove the incident-to-evidence loop

- Use a fake PSA server and synthetic tenant/asset data.
- Exercise ticket ingestion, asset resolution, authorization, session creation, consent, session termination, evidence export/verification, and ticket update.
- Inject duplicate, reordered, delayed, stale, malformed, and unauthenticated events; kill consumers between commit and acknowledgement.
- Verify that revocation prevents new privileged work and that an ambiguous asset mapping cannot redirect a session.
- Do not require a production customer or external human tester to prove the control-flow invariants; use deterministic simulations and retain any later owner playtest as a separate product-quality step.

**Exit evidence:** exact-head CI, deterministic integration transcripts, independently checked session receipt fixtures, tenant-isolation negative tests, and a human-readable evidence packet.

### Gate 2 — Operational alpha

- ConnectWise adapter in read-mostly mode, then narrowly scoped writes.
- Internal service-desk view that unifies incidents, assets, current status, session history, approvals, and evidence.
- Customer-visible activity and approval records with redacted evidence links.
- Operator-visible sync health, mapping conflicts, dead letters, and reconciliation.

**Exit evidence:** rehearsed installation/upgrade/uninstall, backup/restore, audit export, connector credential rotation, replay/recovery tests, and a clearly bounded supported platform matrix.

### Gate 3 — Safe fleet remediation

- Add target-specific agents and capabilities by OS, beginning with the platforms that can actually be qualified.
- Support observation-only posture, dry-run plans, staged rollout, explicit approval, post-change verification, and rollback/containment.
- Integrate Nixward only for its declared NixOS domain; do not generalize its realization model to Windows/macOS by naming convention.
- Require per-action evidence and stop rollout automatically on failed postconditions.

**Exit evidence:** adversarial authorization/revocation tests, per-OS execution receipts, rollback fault injection, and an explicit list of environments not yet qualified.

### Gate 4 — Replace PSA features selectively

Only after the workflow is used successfully should the native service-management plane expand to service catalog, scheduling, contracts/agreements, SLA calendars, technician time review, customer portal, reporting, quoting, invoicing, and finance integration. Preserve adapter interoperability and a complete export path throughout.

Do not implement payroll, a full accounting ledger, a universal CRM, or every marketplace integration as part of the first milestone.

## Measurements that tell us whether it is actually better

Measure against a baseline instead of equating feature count with competitiveness:

- mean time to acknowledge, begin authorized support, and verify resolution;
- alerts merged per actionable incident and false-positive/duplicate-ticket rate;
- technician time spent on ticket administration versus diagnosis/remediation;
- percentage of privileged actions with complete, independently verifiable evidence;
- number of cross-tenant authorization failures in the negative suite (target: zero);
- synchronization lag, duplicate writes, unresolved mappings, and dead-letter recovery rate;
- SLA attainment and customer-visible status freshness;
- failed/rolled-back remediation rate and the percentage of plans that are dry-run and approved before execution;
- export completeness and time required to migrate away from a provider.

## Decisions intentionally left open

- Final product and repository name for the service-management plane.
- Whether its first hosting model is single-tenant self-hosted, multi-tenant hosted, or both; the data model should preserve tenant isolation either way.
- Which second PSA/RMM adapter follows ConnectWise.
- Native identity provider, notification, document/knowledge-base, and accounting integrations.
- Commercial licensing and support model.

These should be settled by technical fit and adoption evidence, not by expanding the first implementation pre-emptively.

## Product scope and frontend decision

The product direction and framework decision are now detailed in [Unified Platform Product and UI Decision](UNIFIED_PLATFORM_PRODUCT_AND_UI_DECISION.md). The key decisions are:

- build toward an owned service-management product, but begin with a ConnectWise-compatible MSP workflow rather than immediate feature parity;
- define the broader vision as a Sovereign Operations Platform with distinct vertical packs and shared resource, authority, workflow, integration, execution, and evidence boundaries;
- use Leptos 0.8.x as the default for new Rust-native operator/customer web surfaces, reusing Xenia's existing sovereign-admin and ecosystem experience;
- keep OpenAPI/REST and CloudEvents as stable integration contracts; Leptos server functions are UI convenience endpoints, not the domain authorization or integration boundary;
- do not rewrite Mycelix Music's existing Next.js/React interface as part of this effort;
- expand into adjacent verticals only after measured evidence establishes that the first workflow is useful, safe, and operationally supportable.

This is a strategic architecture decision, not a claim that the platform is already bigger, faster, or safer than incumbents. That requires comparative operational evidence.

## References

- [ConnectWise PSA feature overview](https://www.connectwise.com/platform/psa): ticketing, service delivery, time, billing, projects, and reporting.
- [ConnectWise SLA management](https://www.connectwise.com/platform/psa/slas): agreements, service expectations, response matrices, and contract tracking.
- [ConnectWise RMM packages](https://www.connectwise.com/rmm-packages): monitoring, patching, scripting, remote access, and integrations.
- [ConnectWise integrations marketplace](https://www.connectwise.com/platform/integrations): APIs, third-party integrations, and integration ecosystem.
- [CISA: Guide to Securing Remote Access Software](https://www.cisa.gov/resources-tools/resources/guide-securing-remote-access-software): threat patterns and remote-access security practices.
- [CISA #StopRansomware Guide](https://www.cisa.gov/stopransomware/ransomware-guide): authorized RMM, logging, and network segmentation recommendations.
- [OWASP API Security Top 10 — Broken Object Level Authorization](https://api-security.owasp.org/editions/2023/en/0xa1-broken-object-level-authorization/): resource-level authorization requirements.
- [Xenia Peer](https://github.com/Luminous-Dynamics/xenia-peer), [Xenia Wire](https://github.com/Luminous-Dynamics/xenia-wire), [Sovereign Ops](https://github.com/Luminous-Dynamics/sovereign-ops), [Nixward](https://github.com/Luminous-Dynamics/nixward), and [Luminous Platform](https://github.com/Luminous-Dynamics/luminous-platform).


## Production implementation language and trust boundary

The production operations runtime is Rust-first: typed work-case and event models, authorization checks, connector workers, state transitions, persistence adapters, contract validation, evidence handling, and privileged operation boundaries belong in Rust. Python reference models under `contracts/` and one-off research tools are non-production aids only; they must not become runtime dependencies or authorities. Keep the production service's correctness and authorization decisions independent of Python availability.

The Holochain share-package validator is implemented in Rust, embeds the reviewed JSON Schema, explicitly enables format assertions, refuses network schema-reference retrieval, and returns deterministic diagnostics without echoing untrusted values. This is local contract validation only; publication authority remains in an authenticated adapter, and remote execution authority remains outside the Holochain payload.
