# One Luminous Platform, Many Business Packs

**Decision:** optimize for one customer-facing Luminous Platform with a shared operations kernel and independently qualified business/vertical packs. Do not create a separate platform for every business idea.

This is an architectural direction, not a claim that a complete shared kernel or all packs are already implemented.

## The product model

### One umbrella: Luminous Platform

The front door should help a customer answer three questions:

- **Start:** what outcome am I trying to achieve, and which service pack fits?
- **Operate:** what is happening, what needs approval, and what evidence supports the current state?
- **Extend:** which integrations, policy packs, or deployment adapters are supported?

Avoid separate sign-ups, duplicate customer records, separate credential stores, and competing status dashboards for each vertical.

### One shared operations kernel

The reusable kernel should converge on a small set of stable, versioned contracts:

- tenant and resource identity/mapping;
- workflow state machines and idempotency;
- operator/organization authority references;
- normalized events and provider adapters;
- evidence envelopes, provenance, and explicit uncertainty;
- execution requests delegated to the subsystem that owns the target;
- export, audit, retention, and recovery semantics.

The kernel must not become a universal owner of identity, host state, backup storage, or domain safety. It coordinates existing owners through contracts. Unknown tenant mappings, unsupported adapters, missing authority, stale state, and unresolved evidence must block the relevant operation.

### Business packs, not separate platforms

A pack adds a domain workflow, service definition, schemas, checklists, presentation defaults, and tests. It does not duplicate the tenant model, policy engine, evidence model, authentication, release pipeline, or billing integration.

Recommended pack order:

1. **MSP Incident-to-Evidence** — start here, work beside the incumbent PSA, deliver a source-linked incident review and follow-through. The first service can be manual-first; live connectors stay unavailable until their authentication, tenant boundary, idempotency, revision handling, and CI gates pass.
2. **IT Operations and Recovery Assurance** — use supported host/deployment adapters for change review, drift, and recovery exercises. Production actuation comes later and is separately qualified.
3. **Security Operations** — incident and evidence workflows with bounded, explicitly authorized actions. Do not promise autonomous remediation as the first product.
4. **Community Infrastructure / Energy** — only after the common contracts stabilize and a domain-specific adapter, safety case, simulation, and physical-validation programme exists.

Do not market a future vertical as a current product merely because its domain model fits the kernel.

## Repository strategy: modular without multiplying platforms

A single repository is not a prerequisite for a single product, and a single product does not require every module to share a release cadence.

Keep a repository separate when it has a genuinely independent authority/security boundary, release lifecycle, platform/toolchain requirement, or external users. That supports current separation between:

- **Symthaea:** cognitive/reasoning and model capabilities;
- **Mycelix:** identity, governance, and optional federation/provenance;
- **Xenia:** operator/session/consent and authorization evidence;
- **Nixward / Spore / Sovereign Boot:** host operations, installation, and recovery responsibilities;
- **Sovereign Ops:** cross-system orchestration and evidence aggregation;
- **Luminous Edge:** tested composition of deployable platform modules;
- **Luminous Platform:** the product documentation, supported product contract, deployment profiles, business packs, getting-started paths, and release/compatibility view.

These names describe the intended product boundary; implementation and qualification status must still be checked per capability.

Avoid creating:
- a new platform repository for every industry;
- a duplicate identity or authorization engine inside each pack;
- a separate UI/tenant database per product surface when it serves the same customer and trust domain;
- a neutral protocol/core repository before the contract has at least two real independent consumers, portable tests, a stable owner, and a repeatable release procedure.

Extract a new shared core only when the dependency and maintenance evidence justify it. Until then, version and test the contracts in their current owners.

## How a pack should be delivered

Every pack should contain:

- a versioned pack manifest with intended customer, supported modes, prerequisites, included scope, exclusions, owner, and status;
- reusable intake, statement-of-work, delivery, and escalation templates;
- input/output schema versions and data minimization rules;
- manual, synthetic, and automated modes clearly distinguished;
- tests for tenant/resource confusion, duplicate/replayed events, stale revisions, incomplete evidence, and unauthorized writes;
- a supported-version table and explicit unsupported capabilities;
- deployment, upgrade, export/exit, and rollback guidance when applicable.

The pack's status may only describe the exact scope assessed. A passing unit test for a schema is not proof of a live provider integration, production tenant isolation, or safe actuation.

## When to split into a separate platform

A split is justified only when several of these are true:

- the domain needs a different security or safety boundary;
- it has independent operators, data ownership, and support commitments;
- it requires a fundamentally different lifecycle or deployment plane;
- it has domain-specific regulation or physical hazard controls that cannot be represented as a bounded adapter/profile;
- separate release ownership materially reduces risk rather than merely adding maintenance.

Even then, prefer a separate pack or bounded service with explicit interfaces first. Split the customer-facing platform only if users truly need different products, not just different workflows.

## Commercial implications

The single-platform model should make the entrepreneur's path simpler:

- one place to learn and choose an offer;
- one repeatable intake and delivery system;
- one set of evidence principles;
- optional local or partner-run deployment;
- domain-specific service packs;
- replaceable integrations and customer data export;
- no mandatory centralized identity or billing account for basic self-hosted use.

Independent providers can sell implementation, operations support, recovery exercises, training, and integration services. Luminous can earn from support, maintained adapters, release assurance, and advanced services without preventing customers from operating locally or leaving.

## Measures that show the architecture is working

- a new pack can reuse tenant, event, authority, and evidence contracts without duplicating them;
- an operator can onboard a customer without assembling all foundational repositories;
- a customer can run a supported service without Luminous-hosted dependencies unless explicitly selected;
- a pack can be upgraded or retired without migrating unrelated verticals;
- each operational claim is traceable to its owner and exact qualification evidence;
- adding a vertical increases reuse more than it increases new platform-specific concepts.

The design goal is **one coherent product experience, multiple service businesses, and independently owned technical components**.
