# Global Deployment and Business Model

**Status:** Proposed product and deployment architecture. This document is a roadmap, not a statement that the listed capabilities are implemented, independently assessed, or production-qualified.

## Executive direction

Build Luminous Platform as a **sovereignty-first operations and resilience platform**: organizations can operate infrastructure they control, apply bounded and reviewable change, recover from failures, and retain portable evidence of what was proposed, authorized, executed, and verified.

The product should not compete as a generic cloud, a general-purpose AI agent, or a replacement for every security control. Its initial wedge is narrower and more defensible:

> **Guardrailed infrastructure operations with independently reviewable evidence and tested recovery.**

This composes the existing project boundaries instead of making Sovereign Ops a second identity provider, endpoint manager, backup repository, network simulator, or policy authority.

The commercial objective is to sell repeatable operational outcomes—safer change, recoverability, reduced operator toil, and clearer evidence—while keeping local control and an open, inspectable core.

## Product boundaries

| Component | Intended responsibility | Boundary |
|---|---|---|
| **Sovereign Ops** | Coordinate intent, preflight, shadow evaluation, authorization requests, adapter dispatch, evidence collection, and operator reporting | Must not bypass the control that owns an action |
| **Nixward / host adapter** | Observe declared and actual host state; evaluate host-specific policy; perform narrowly scoped, authorized actions; verify resulting state | Owns host facts and host actuation, not org-wide identity or backup retention |
| **Spore** | Bootstrap, install, reconstruct, and rehydrate a declared endpoint | Must not own backup-destruction authority |
| **Xenia** | Operator authentication, short-lived role authority, session consent, action receipts, and revocation where implemented | Must remain the authority for its session and operator controls |
| **Mycelix** | Optional federated identity, governance, provenance, and verifiable credentials where justified | Must not receive raw telemetry or private operational data by default |
| **Symthaea** | Optional anomaly interpretation, planning, simulation support, summarization, and decision support | Model output is advisory; it never becomes authorization merely because it is confident |
| **Luminous Edge** | Compose a tested set of platform components and pin compatible versions | Must make the selected component versions and their provenance inspectable |

All integrations should use explicit, versioned contracts. A component may attest to a fact it owns; a coordinator may aggregate those claims but must not silently strengthen them.

## Market position and integration strategy

Do not rebuild existing infrastructure automation merely to own every layer. The adjacent ecosystem already provides useful primitives: [Colmena](https://github.com/nix-community/colmena) is a stateless NixOS deployment tool with parallel deployment, while [OpenTofu](https://opentofu.org/docs/v1.7/intro/) offers declarative infrastructure changes, plans, approval before modification, and a broad provider model.

The opportunity for Luminous is the layer that coordinates **authority and evidence across systems**:

- keep existing tools responsible for the mechanical changes they already execute well;
- make plans, approvals, targets, dispatch attempts, observed outcomes, and rollback evidence refer to the same typed operation;
- normalize what an adapter can prove without pretending all adapters provide equivalent guarantees;
- show an operator exactly which facts are missing before a consequential action can be authorized;
- support local recovery and evidence export without requiring a proprietary central service.

This is a product hypothesis to validate, not proof that no competing product already offers every feature. Maintain a feature comparison as releases and competitors change. Do not win by accumulating more integrations; win by making cross-system changes safer, more explainable, and recoverable.

## Operating and authority model

Use a predictable lifecycle for consequential operations:

1. **Observe**: capture current state with source, scope, freshness, and known limitations.
2. **Propose**: construct a typed intent with a unique operation ID, target set, desired outcome, expiry, and rollback or compensation plan.
3. **Preflight**: evaluate policy, dependency health, blast radius, resource impact, and whether the available evidence is sufficient.
4. **Shadow / simulate**: evaluate without changing production state; label modeled outcomes as modeled.
5. **Authorize**: obtain the authority required for this exact operation, target, scope, and time window.
6. **Dispatch**: ask the owning adapter to execute an idempotent, narrowly scoped operation. Bind the dispatch to a unique attempt and fencing token where concurrent or delayed work could be dangerous.
7. **Reconcile**: independently observe resulting state rather than trusting the command's exit code.
8. **Attest and report**: bind the intent, policy revision, authorization, attempt, source/build identity, observations, and result into a verifiable evidence record.
9. **Recover**: where necessary, execute a separate authorized rollback or recovery flow and verify the recovered state.

A timeout, duplicate dispatch, stale approval, unknown authority, invalid signature, missing evidence, or ambiguous target must not be interpreted as permission to proceed. Retrying an operation must not silently create a second privileged action.

### Two different meanings of failure

Avoid the ambiguous phrase **“fail-open”** as a platform-wide security promise.

- **Boot and availability safety:** the platform's optional services must not be required for the host to boot or for an administrator to regain local recovery access.
- **Authority safety:** a missing policy service, unverifiable identity, expired authorization, or unknown action must not grant additional privileges.
- **Observation degradation:** monitoring may degrade visibly without converting missing observations into a healthy status.
- **Recovery safety:** recovery must remain possible through a separately documented and tested path when the normal control plane is unavailable.

These are different contracts and need different automated tests.

## Deployment profiles

The first implementation should be **NixOS-first**, while keeping the adapter and evidence contracts portable enough to support other operating systems later. Do not claim cross-platform support until each target has its own tested adapter and lifecycle.

### Profile A — Local / small organization

- Single organization-controlled host or small Linux fleet.
- Local admin interface and policy.
- No required Luminous-hosted control plane.
- Explicit local recovery path and exportable evidence.
- Suitable for a home lab, small IT provider, cooperative, or small organization.

### Profile B — Self-hosted fleet

- Multiple sites and hosts, with one organization's trust boundary.
- Pinned component versions and declared configuration.
- Role-scoped operators and per-action authorization.
- Local telemetry and evidence retention policies.
- Tested rollout, rollback, backup restore, and lost-control-plane scenarios.

### Profile C — Hybrid / regional federation

- Local execution and local data ownership; optional regional coordination.
- Cross-site exchange limited to signed policy references, revocation facts, sanitized evidence summaries, and explicitly approved operational data.
- Different jurisdictions can use different retention, access, localization, and incident-routing policies.
- Regional connectivity is an optimization, not a hidden prerequisite for local operations.

### Profile D — Offline / high-assurance

- Signed and pinned update artifacts, offline verification, controlled trust-root rotation, and explicit re-enrollment/revocation processes.
- Reproducible or independently verifiable build evidence where practicable.
- No unreviewed remote actuation.
- Documented break-glass authority and post-event audit.
- A profile label is not a certification; its exact tests and limitations must be published.

## Global deployment requirements

“Globally deployable” means a documented and supportable deployment can be installed, operated, upgraded, recovered, and removed in materially different environments. It does not mean one configuration is automatically legal or suitable in every country.

Every supported release should specify:

- **Installation and exit:** supported hardware/OS matrix, clean-install path, upgrade path, export format, uninstall path, and recovery if an upgrade fails.
- **Data authority:** what data is collected, where it is stored, who can access it, how long it is retained, how it is exported, and how deletion is verified.
- **Key ownership:** whether keys remain on customer-controlled machines or HSMs; how backups, rotation, revocation, loss, and recovery are handled.
- **Network assumptions:** required outbound connections, offline behavior, DNS/time dependencies, proxy support, and whether any remote control service is mandatory.
- **Tenant isolation:** separate organization identifiers, keys, roles, policy namespaces, storage, and evidence access; explicit cross-tenant checks in tests.
- **Localization:** time zones, locale-aware timestamps, translated operator-facing incident guidance, accessible UI, and language-independent machine-readable receipts.
- **Supply-chain evidence:** pinned source revisions, locked dependencies, build provenance, signed releases, vulnerability response, and a supported update channel.
- **Jurisdiction profiles:** configurable records and retention, privacy/data-processing terms, incident reporting, sector-specific expectations, and regional support routes. Keep legal profiles versioned and reviewed; do not market them as legal advice or universal compliance.

Use NIST CSF 2.0 as a flexible risk-management vocabulary, not as an automatic compliance badge. The June 2026 NIST IR 8374 Rev. 1 ransomware profile is a useful external baseline for the resilience product line. The EU Cyber Resilience Act's vulnerability/incident reporting obligations began applying on 11 September 2026, while its main obligations apply from 11 December 2027. Where the product or its distribution falls within scope, the team needs a maintained applicability assessment and incident process; this document is not that assessment.

References:
- [NIST CSF 2.0 Tiers quick-start guide](https://csrc.nist.gov/pubs/sp/1302/final)
- [NIST IR 8374 Rev. 1 — Ransomware Risk Management](https://csrc.nist.gov/pubs/ir/8374/r1/final)
- [European Commission — Cyber Resilience Act reporting obligations](https://digital-strategy.ec.europa.eu/en/policies/cra-reporting)
- [SLSA build provenance](https://slsa.dev/spec/v1.2/provenance)

## Evidence and assurance contract

Use a versioned evidence envelope for each exercise and consequential operation. At minimum it should identify:

- schema and policy/profile version;
- organization, environment, target, operation, run, and dispatch-attempt identifiers;
- issuer and trust-root identity;
- source revision, build/artifact digest, and adapter version;
- start/end times, clock source/uncertainty where relevant, and freshness window;
- intent and authorization references;
- evidence references and integrity digests;
- evidence type/strength and exactly what it demonstrates;
- result, unresolved facts, limitations, and verifier version;
- signature or other integrity mechanism appropriate to the source.

A digest proves byte identity, not truth. A signature proves that a key signed a record, not that the underlying claim is true. A simulation is not a physical exercise. A successful service restart is not a successful disaster recovery. Preserve these distinctions in UI, APIs, reports, and release notes.

Recommended **internal** maturity labels (not an external certification scheme):

- **M0 — Described:** requirements and limitations documented; no execution claim.
- **M1 — Observed:** live observations collected and bound to the declared subject.
- **M2 — Simulated:** a deterministic scenario runs in a model or sandbox; model scope is stated.
- **M3 — Shadow-qualified:** decisions are evaluated against live state without production actuation.
- **M4 — Controlled operation:** a specific action class has authorization, bounded dispatch, post-action reconciliation, and tested rollback.
- **M5 — Recovery exercised:** a destructive or representative recovery exercise demonstrates restoration and post-recovery checks.
- **Independent assessment:** reported separately, with assessor, scope, date, exceptions, and evidence. It is not inferred from an internal maturity label.

Do not collapse these dimensions into one green status. A product can be qualified for one action class and unproven for another.

## Product and repository strategy

Use [One Luminous Platform, Many Business Packs](PLATFORM_AND_REPOSITORY_STRATEGY.md) as the governing product-structure proposal. The customer-facing model should be one Luminous Platform and one shared operations kernel, with independently qualified service packs. Keep technical repositories separate only where authority, release lifecycle, or external consumers justify the boundary; do not create a new platform per vertical.

To make the business repeatable for people who are not platform developers, use [Start a Luminous Business](START_A_LUMINOUS_BUSINESS.md) and the [MSP Incident-to-Evidence starter pack](../business-packs/msp-incident-to-evidence/README.md). It provides a manual-first first offer and templates; it does not pretend that live integrations or unqualified platform capabilities exist.

## Business model

### Sell outcomes, not lock-in

Keep an inspectable open-source core under the repository's declared license and charge for services and operational value. Candidate revenue streams:

1. **Implementation and migration:** initial inventory, threat/risk profile, policy setup, deployment, and recovery-path validation.
2. **Support and lifecycle management:** version compatibility, upgrades, rollback planning, incident response, and defined support commitments.
3. **Private / managed control-plane convenience:** optional hosting where customers choose it; self-hosted parity and export must remain viable.
4. **Verified resilience exercises:** recurring restore drills, ransomware-survival exercises, evidence packets, remediation tracking, and independent-assessment coordination.
5. **Integration and vertical profiles:** tested adapters and policy packs for particular operating environments, with clear compatibility matrices and support windows.
6. **Partner delivery:** regional systems integrators, MSPs, cooperatives, and local implementation providers using a documented deployment and support model.
7. **Hardware / appliance option later:** preconfigured edge systems only after hardware support, secure update, recovery, and lifecycle obligations are proven.

Do not make customer telemetry, customer-owned keys, or the ability to leave the platform the thing customers pay to unlock. The business should earn recurring revenue from lower operational risk, useful maintenance, and demonstrated resilience.

### Initial packaging hypothesis

| Offer | Customer value | Commercial approach |
|---|---|---|
| **Community / self-hosted** | Inspectable core, local control, base profiles, exportable evidence | Free software; community documentation |
| **Professional deployment** | Supported install, configuration, migration, and runbooks | Fixed-scope implementation package |
| **Operations support** | Upgrade help, compatibility commitments, incident assistance | Recurring support contract |
| **Resilience service** | Scheduled restore drills, test evidence, corrective-action tracking | Annual service package priced by scope/estate |
| **Federation / enterprise** | Multi-site governance, integration work, support and operating controls | Contracted deployment and support |
| **Partner program** | Repeatable local delivery and a supported implementation toolkit | Training, certification of partner competence, and services revenue |

These are testable packaging hypotheses, not established market demand or recommended final prices. Validate which problem customers will pay to solve before building billing, tenancy, or a broad hosted service.

### Go-to-market sequence

1. **Engineering qualification:** produce repeatable local install, upgrade, authority-boundary, evidence-verification, and recovery results.
2. **Internal operational use:** use the product on a bounded, non-critical estate with owner-reviewed deterministic evidence and an independently usable recovery route.
3. **Paid design engagements:** solve a specific customer's operational problem under an explicit scope, with human approval and no implied production-readiness claim.
4. **Repeatable service:** turn repeated steps into documented profiles, migration tools, compatible-version rules, and bounded support promises.
5. **Regional partner delivery:** localize service delivery and legal/support documents instead of assuming a single global operating entity can serve every market directly.

Customer discovery is a commercial validation step; it must not replace automated tests, deterministic simulation, evidence review, or technical qualification.

## Reusable business models built on the same kernel

Do not launch all verticals at once. Reuse a common kernel—identity/authority references, policy, intent, evidence, adapter contracts, and recovery—then add domain-specific adapters and success criteria.

| Vertical model | Symthaea contribution | Mycelix contribution | Why it could fit |
|---|---|---|---|
| **Sovereign IT operations** (first) | Change-risk interpretation, anomaly triage, simulation support, operator explanations | Optional identity and governance federation; provenance for shared defensive artifacts | Closest to the current product boundary and existing engineering |
| **Managed resilience / recovery** (adjacent) | Failure scenario analysis and recovery-plan comparison | Portable exercise credentials and provenance where multiple parties need to trust results | Clear recurring service; measurable restore objectives |
| **Community energy / microgrids** (later) | Forecasting, fault/anomaly detection, maintenance planning under explicit safety limits | Cooperative membership, decision records, provenance of energy/maintenance claims | Strong sovereignty fit, but requires domain partners, safety engineering, and physical validation |
| **Community broadband / local infrastructure** (later) | Capacity and fault analysis; suggested topology changes | Shared governance and accountability across operators | Local ownership and federated operations without a single global operator |
| **Repair / circular-production networks** (exploratory) | Predictive maintenance and diagnostic support | Provenance and verifiable service/component history | Potential to coordinate repair capability and trust across independent businesses |

In every vertical, Symthaea should propose and explain; domain policy and designated humans or certified controllers retain authority over consequential actions. Mycelix should federate only the data and claims that participants have agreed to share. Neither system removes the need for engineering validation or sector-specific safety controls.

## Prioritized engineering programme

The sequencing below is intentionally evidence-led. A roadmap item is not a completed control until its exit evidence exists.

### P0 — Make the trust boundary real

- Eliminate browser-persisted privileged secrets; implement a real authentication and session lifecycle before exposing the administration interface to untrusted networks.
- Ensure cryptographic claims are only emitted by implemented and independently checked mechanisms. Stub, placeholder, or deterministic demo outputs must return an explicit unsupported/unproven result rather than a proof-shaped success.
- Remove machine-specific absolute dependency paths; define pinned, reproducible dependency boundaries for clean-clone builds.
- Ensure signed policy verification binds a canonical, versioned policy representation and its authority context, not just an arbitrary set of rules.
- Keep privileged actuation disabled by default until the exact path has an explicit authorization contract and tested refusal behavior.

**Exit evidence:** a clean, independent build; automated negative tests for missing/invalid/stale/replayed authority; proof that secrets never enter browser persistence or client logs; proof claims validated by a separate verifier; and a documented network-exposure boundary.

### P1 — Freeze shared contracts

- Versioned intent, policy, dispatch, receipt, evidence, and adapter schemas.
- Idempotency keys, attempt identity, fencing, expiry, replay protection, and clear clock/freshness semantics.
- Explicit ownership and authority tests between Sovereign Ops, Nixward, Xenia, Mycelix, Spore, and backup/network adapters.
- Independent reference checker and deterministic evidence fixtures.

**Exit evidence:** compatibility tests, corpus counts, negative vectors, and a verifier that does not share the implementation's core decision logic.

### P2 — Qualify one complete operational workflow

Start with a narrow workflow such as **propose a NixOS change → evaluate in shadow mode → request approval → perform an authorized bounded change → verify actual resulting generation → prove rollback/recovery path**.

**Exit evidence:** exact source/build identity, target binding, an operator receipt, post-action observations, replay/staleness refusal tests, and demonstrated rollback. Do not infer the outcome from a green CI run alone.

### P3 — Package and support

- Pinned Luminous Edge release matrix and clean-clone installation guide.
- Backup/export/restore, uninstall, key rotation/revocation, and offline recovery instructions.
- Security reporting contact and vulnerability-response process.
- Release provenance, signed artifacts, compatibility windows, and an explicit support policy.
- Deployment profiles for each supported environment.

**Exit evidence:** a second machine can install, verify, upgrade, roll back, and recover from the documented artifacts without relying on undocumented developer-machine state.

### P4 — Commercial and vertical expansion

- Run limited paid deployment engagements and measure support effort and customer outcomes.
- Build service runbooks and cost models from actual operations, not projected scale.
- Expand into a new vertical only when the common interfaces are stable and a domain-specific scenario can be simulated and independently reviewed.
- Add hosted multi-tenant services only after tenant isolation, privacy boundaries, security operations, and export/exit workflows have been tested.

## Operating metrics

Track metrics that an operator or customer can verify:

- time to restore a declared service to a verified state;
- percentage of privileged actions bound to the correct intent, authorization, attempt, and post-action evidence;
- rate of stale, replayed, unauthorized, or out-of-scope requests correctly refused;
- clean-build and reproducibility results by supported environment;
- upgrade and rollback success across the supported version matrix;
- evidence completeness, freshness, and independent-verifier agreement;
- service availability and recovery when optional coordination services are offline;
- onboarding and recurring support hours per deployment;
- customer-observed time saved and reductions in unplanned downtime.

Model accuracy, code volume, number of integrations, or a single aggregate “health score” should not substitute for these outcomes.

## Decision rule

**Prefer one end-to-end workflow with a narrow, honest guarantee over many incomplete modules.** Keep the host bootable, keep authority explicit, keep data under the customer's control, and let independently checkable evidence—not AI confidence or marketing language—determine what the platform is allowed to claim.
