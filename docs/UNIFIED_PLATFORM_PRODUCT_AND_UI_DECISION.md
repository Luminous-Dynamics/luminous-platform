# Product Decision: Unified Sovereign Operations Platform

**Status:** Proposed direction for review  
**Date:** 2026-10-09  
**Applies to:** Xenia, Sovereign Ops, Nixward, Symthaea, Luminous Platform, future service-management product  
**Decision:** Build toward an owned operations platform; enter through one exceptional MSP workflow; use Leptos for the Rust-native operator/customer web surfaces, without making it the integration API.

## 1. Executive decision

**Yes: we should build our own version of the ConnectWise category. No: we should not begin with a ConnectWise clone.** ConnectWise is the first market and compatibility target, not the boundary of the platform vision.

The larger thesis is a **Sovereign Operations Platform**: a common operational system for organizations that need to coordinate people, services, assets, software, physical infrastructure, policies, work, evidence, and economic settlement. MSP service delivery is the first vertical because Xenia already owns an unusually relevant piece of the operational loop: remote sessions, explicit consent, scoped authorization, secure transport, and session evidence.

This could ultimately be broader than ConnectWise, but that is an architectural ambition—not evidence of market size, readiness, or guaranteed success. ConnectWise already markets a unified PSA surface, AI-assisted ticket triage, agreements/SLAs, timekeeping, billing, asset management, reporting, and a large integration ecosystem. “We also have AI” or “we use Rust” is not a defensible differentiation by itself.

Our bet should be that the platform makes important work **safer, easier to understand, easier to verify, and less fragmented across tools**, while remaining deployable under the customer's control.

## 2. The unifying model

The platform's central object should not be a ticket, remote session, or chat conversation alone. It should be a **work case**: a request or detected need connected to a tenant, responsible people, affected resources, policy/entitlement, planned work, authority decisions, actual observations, evidence, outcome, and (where appropriate) cost or settlement.

The invariant lifecycle is:

**detect/request → resolve context → understand → plan → authorize → act → verify → record evidence → communicate → settle/learn**

Not every case needs every stage. A read-only observation may need no privileged approval; a destructive change should require narrow authority, explicit approval, preconditions, a recovery plan, and postcondition verification. Billing must remain a separate decision from technical success.

This model can span multiple vertical packs without pretending that every domain has identical rules.

## 3. A platform kernel, not a monolith

The durable kernel should contain the following conceptual capabilities. These are product boundaries, not a proposal to create one crate for every bullet today.

| Kernel capability | Responsibility | Boundary |
|---|---|---|
| Tenant and resource graph | Organizations, people, sites, assets, services, entitlements, cases, external mappings, ownership and lifecycle | Data ownership and tenant access are explicit; no guessed association |
| Workflow/state machines | Typed transitions, deadlines, approvals, cancellation, retries, compensation and recovery | A valid transition is required; arbitrary state patching is not the workflow engine |
| Identity and authority | Actor identity, tenant binding, scoped capabilities, delegation, expiry, revocation, dual control | Authentication, consent, contract entitlement and action authorization remain distinct |
| Event/integration fabric | Stable resource IDs, CloudEvents, OpenAPI, adapter SDK, mapping/reconciliation, inbox/outbox, dead letters | Event validity is not trusted origin, permission, or exactly-once delivery |
| Evidence/provenance | Versioned receipts, hashes, attestations, redaction/classification, independent checking | Signature validity, trust, freshness, completeness and acceptance are separate claims |
| Execution plane | Xenia sessions, Nixward system transactions, future OS-specific agents and external executors | Executes only a typed, approved plan under the live authority boundary |
| Cognition/planning | Symthaea correlation, causal hypotheses, diagnosis, option generation, risk explanation and post-action learning | Advisory output; never the sole authority for a consequential action |
| Experience/API plane | Leptos web clients, stable APIs, notifications, search, audit views and accessibility | UI frameworks do not define the domain model or replace external API contracts |
| Operations/deployment | Self-hosted/managed/hybrid deployment, secrets lifecycle, upgrades, backups, restore, observability and export | The management plane should not be a dependency for host boot or local safe recovery |

Start with the existing repositories. Do not duplicate Xenia's consent/session implementation in the PSA layer, do not place a whole CRM in Xenia, and do not expand Nixward outside its qualified NixOS domain by analogy.

## 4. How the vision grows beyond ConnectWise

The long-term unification is in the operational kernel and trust model, not in a universal screen full of every feature.

### Vertical pack A — MSP service delivery (first)

Customer portal, service desk, asset/CMDB, agreements and SLA policy, technician dispatch, knowledge, remote session workflow, evidence, time review, reporting, and later quote-to-cash/finance adapters.

### Vertical pack B — Internal IT and sovereign endpoint operations

Fleet inventory, configuration drift, approved remediation, patch/upgrade campaigns, change management, recovery evidence, and compliance reporting. Reuse Nixward only for supported NixOS operations; introduce separate OS agents only with their own capability/evidence contracts and qualification.

### Vertical pack C — Security operations

Alert normalization, incident cases, asset and identity context, evidence collection, containment plans, dual control for high-impact actions, case chronology, and post-incident review. Integrate existing SIEM/EDR/identity systems instead of trying to reproduce every security product.

### Vertical pack D — Energy and infrastructure operations

Asset lifecycle, maintenance windows, work orders, telemetry, safety constraints, engineering changes, inspection records, provenance, and resource/cost accounting. Any control of physical infrastructure would require domain-specific hazard analysis and separate execution gates; the shared kernel alone is not sufficient authorization.

### Vertical pack E — Civic/cooperative and other service organizations (later)

Case management, community resources, service commitments, grants or credits, governance and accountability. These are potential future verticals, not first-release commitments.

The shared kernel creates leverage only if each vertical keeps its distinct domain rules and evidence semantics. Avoid a universal workflow language that hides important differences in safety, contract, or financial logic.

## 5. Competitive strategy

### Build

- A first-class connection between case, resource, approved plan, execution, evidence, and outcome.
- Strong resource/tenant isolation, consent and revocation, operator/customer-visible authority, and independently checkable operational records.
- A clean, versioned integration contract and a growing adapter ecosystem.
- Safe automation with dry-run, risk explanation, explicit approvals, postcondition checks, staged rollout, and defined recovery.
- Data portability and customer-owned deployment choices from the beginning.
- A coherent operator workflow that lowers ticket administration rather than merely offering more dashboard panels.

### Integrate before replacing

- ConnectWise PSA and then a second service desk / RMM as external adapters.
- Identity providers, mail, notifications, documentation/knowledge, SIEM/EDR, backup, cloud inventory and accounting systems through stable APIs.
- Existing financial/accounting systems as the system of record for tax, accounting and payment rails until a mature finance domain has its own controls.

### Defer

- Feature-for-feature parity across every ConnectWise screen.
- Payroll, a full general ledger, universal CRM, full marketplace, every OS agent, broad device control, or automated physical infrastructure control in the initial milestone.
- Any plan to make the entire system depend on one AI model, one identity provider, one database vendor, or a single distributed ledger.

## 6. Leptos: adopt for the Rust-native experience, not as the whole platform

**Recommendation: use Leptos 0.8.x as the default frontend framework for new Rust-native Luminous operator and customer web applications, beginning with the Xenia Operations console.** The current public xenia-peer/apps/sovereign-admin already uses Leptos 0.8 with CSR; mycelix-leptos-client exists for browser-side Holochain interactions; and Sol Atlas has another Leptos/WebGL application. This gives us existing code and lessons to reuse rather than starting a framework migration from zero.

The current public docs.rs release observed during this decision is Leptos 0.8.22. Use a committed Cargo.lock and test the exact Rust/WASM targets; do not treat a broad 0.8 manifest constraint as an exact release pin.

### Recommended UI split

- **Operations console:** Leptos CSR initially for the high-interaction, authenticated operator surface and reuse of the existing sovereign-admin work. Evaluate SSR/hydration or islands for routes where first paint, direct links, or constrained WASM downloads demonstrably matter.
- **Customer portal and public documentation:** prefer SSR/progressive enhancement where it improves direct navigation, accessibility, initial render, and non-JavaScript behavior. Use islands for isolated interactive controls when appropriate rather than hydrating every page by default.
- **Long-lived remote screen/audio/input surfaces:** keep the media/session data plane in dedicated client modules and Xenia APIs. Do not route high-frequency media through Leptos server functions or rebuild a second transport stack inside UI components.
- **Mycelix Music:** do not rewrite its existing Next.js/React application as part of this project. Integrate its domain through contracts where needed; reconsider its UI stack only under its own product roadmap and migration evidence.
- **Other teams/ecosystems:** publish APIs and SDKs so a TypeScript, native, mobile, or third-party client can use the same platform without adopting Leptos.

### The boundary that matters

Leptos server functions are HTTP endpoints and must be treated as network-facing APIs: authenticate the caller, validate input, enforce tenant/resource authorization per request, rate-limit where relevant, and sanitize the response. Do not rely on hidden UI controls, client-side state, Rust types, or a Leptos macro as an authorization boundary.

Use Leptos server functions for UI-local convenience where useful, but keep **OpenAPI/REST or another documented API for stable external operations**, and CloudEvents for event exchange. Domain authorization, durable idempotency, transaction/outbox semantics, and evidence verification belong in backend services with tests independent of the UI.

Do not place provider credentials, long-lived bearer tokens, session traffic keys, signer seeds, or privileged authority in browser storage. Use a reviewed authentication/session design and keep signing in the intended signer/authority component. Browser-held UI state may help navigation; it must not grant permission.

### Reuse gates before calling Leptos the standard

1. Create the first console shell with shared design tokens and accessible primitives, not a bespoke UI kit before basic flows exist.
2. Verify production builds for WASM size, startup time, large tables, search, filtering, keyboard navigation, screen-reader labels, and low-bandwidth operation.
3. Demonstrate authentication and tenant isolation on every server API path and ensure server-side error messages do not expose internal data.
4. Keep browser/API contracts independent so Leptos can be replaced per client without rewriting the operational kernel.
5. Keep the dependency graph deliberate; avoid one giant shared UI crate that forces every product to update in lockstep.

Leptos is a good default for these Rust-first surfaces, not an ideological requirement for all of Luminous Dynamics.

## 7. Architecture principles for a genuinely better unified platform

1. **One operational graph, many views.** A case, asset, approval, session, evidence record and work entry share stable IDs and explicit provenance. Views may differ by role; the underlying relationships should not fork silently.
2. **Authority follows the action.** Recheck current identity, tenant, scope, expiry, revocation and preconditions at the mutation boundary. A ticket, event, signed note, AI recommendation or prior login never grants arbitrary execution.
3. **Evidence is a product feature.** Make evidence understandable to technicians and customers. A hash is only an integrity reference; it is not by itself proof that an observation is truthful or that a policy was satisfied.
4. **Recoverable by design.** Use durable inbox/outbox, idempotency and explicit conflict states; fault-inject process death around commits, acknowledgements, and external effects. Do not claim exactly-once delivery across a network.
5. **AI is plural and bounded.** Support replaceable cognitive backends and deterministic fallback; expose confidence, uncertainty, alternative hypotheses and reasons. Measure whether advice improves verified outcomes rather than treating intelligence as an implicit authority class.
6. **Sovereignty without operational burden.** Provide a supported self-hosted path with automated backup/restore, safe upgrades, health diagnostics, export, migration and minimal telemetry. Add managed hosting as an option without making it mandatory.
7. **Secure defaults and honest profiles.** Separate public/control/session/media planes, bound resource use, enforce tenant isolation server-side, minimize sensitive data, publish supported-platform matrices, and distinguish implemented, internally tested, independently checked, externally audited, and production-qualified claims.
8. **Adoption before breadth.** Make the first 30 minutes excellent: guided setup, import, one useful workflow, clear failure explanations, and an obvious way to return data to the source system.
9. **Open integration surface, controlled mutation surface.** Broad read/interoperability should be easy; high-impact writes should require narrow capabilities and richer evidence/approval.
10. **Human and customer agency.** Show the proposed action, target, expected effects, risk, rollback, and evidence before approval. Make cancellation, revocation, appeal, export, and audit history discoverable.

## 8. Delivery gates

### M1 — Prove value while still coexisting with a PSA

Complete the synthetic incident-to-evidence loop, verify schema and adapter model on exact CI head, then build a simulated provider adapter with explicit tenant/resource mappings, durable inbox/outbox semantics, and crash tests. No live endpoint assumption and no remote action until authority/evidence prerequisites are ready.

### M2 — Thin vertical slice

Build the Leptos operations shell for incident timeline, resource context, authorization/approval state, Xenia session evidence, integration health, and conflict resolution. Make it useful before adding sales, finance, payroll, or a broad plugin store.

### M3 — Design-partner alpha

Integrate a supported external PSA under a narrowly scoped test account after recording its exact product/API family/authentication/permissions/rate limits. Track time-to-triage, time-to-verified-resolution, technician admin effort, sync failures, and customer trust in the evidence view. Use deterministic simulation for control-flow proof; use a small, explicit design-partner program to validate usability and purchasing need.

### M4 — Own the service desk where the advantage is clear

Only then add native ticketing/case management, SLA clocks, customer portal, agreements, time review, knowledge and reporting. Integrate finance initially; implement billing only when contract semantics and reconciliation tests are ready.

### M5 — Add vertical packs one at a time

Expand into wider IT operations, security operations and then energy/infrastructure domains with their own threat/safety reviews. Every new executor and vertical brings additional qualification work; a common kernel does not transfer qualification automatically.

## 9. Evidence required before claiming we are “better”

Compare actual end-to-end outcomes against the current stack:

- median time from incoming issue to correct asset/context and authorized support;
- time to independently verified resolution, not merely command completion;
- technician time spent on duplicate data entry, notes and reconciliation;
- exact evidence completeness and verification error rate;
- duplicate events, conflicting updates, sync lag, and successful recovery after crashes;
- cross-tenant negative test violations (must remain zero);
- customer-reported clarity, consent confidence and ease of export;
- safe-remediation rollback rate and successful postcondition checks;
- setup time, upgrade/restore success, and cost per managed asset/case.

No claim that this platform is bigger or better than every existing product should be made before comparative evidence exists.

## References

- [ConnectWise PSA](https://www.connectwise.com/platform/psa) — broad PSA scope including ticketing, projects, time, finance, sales, reporting and workflow automation.
- [ConnectWise Service Desk and Agentic AI](https://www.connectwise.com/solutions/service-desk-ticketing) — confirms AI-assisted ticket triage is already part of the competitive field.
- [ConnectWise SLA management](https://www.connectwise.com/platform/psa/slas) and [Marketplace/API integrations](https://www.connectwise.com/platform/integrations).
- [Leptos official site](https://www.leptos.dev/), [Leptos book](https://book.leptos.dev/), [server functions](https://book.leptos.dev/server/25_server_functions.html), [islands guide](https://book.leptos.dev/islands.html), and [current crate docs](https://docs.rs/leptos/latest/leptos/).
- [CloudEvents core specification](https://github.com/cloudevents/spec/blob/main/cloudevents/spec.md) and [JSON event format](https://github.com/cloudevents/spec/blob/main/cloudevents/formats/json-format.md).
- [CISA Guide to Securing Remote Access Software](https://www.cisa.gov/resources-tools/resources/guide-securing-remote-access-software).
- Existing implementation anchors: [Xenia sovereign-admin](https://github.com/Luminous-Dynamics/xenia-peer/tree/main/apps/sovereign-admin), [Mycelix Leptos client](https://github.com/Luminous-Dynamics/mycelix-leptos-client), [Sol Atlas Leptos](https://github.com/Luminous-Dynamics/sol-atlas-leptos), and [Mycelix Music](https://github.com/Luminous-Dynamics/Mycelix-Music).


## Production language and runtime boundary

Rust is the default implementation language for production services, contract validators, authorization decisions, persistence boundaries, connector workers, and privileged operations. Production daemon/API/worker builds must not require a Python interpreter or import Python modules to make an authorization, validation, or execution decision.

Python may be used temporarily for explicitly labelled research scripts, migration tooling, and non-production reference/conformance models. Such models are specifications and test aids—not shipping implementations or independent production authorities. New runtime behavior should be implemented and tested in Rust, with shared fixtures and adversarial cases used to establish contract parity. The Holochain share-package validator and its normative contract tests are Rust-native; the existing Operations Event Python modules remain clearly marked reference models until their behavior has been ported into the production Rust service.
