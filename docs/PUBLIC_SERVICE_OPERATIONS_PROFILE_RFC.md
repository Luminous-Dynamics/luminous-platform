# RFC: Public-Service Operations Profile for Luminous Platform

**Status:** Proposed architecture; not implemented or production-qualified  
**Date:** 2026-10-12  
**Scope:** Luminous Platform, Luminous Edge, Symthaea, Mycelix, Nixward, Xenia, Symtropy, and optional geospatial/visualization components  
**Initial jurisdiction:** City of Johannesburg / Gauteng, South Africa  
**Decision sought:** Agree on a narrow civic operations profile that reuses the shared work-case and event architecture instead of creating a separate, competing government platform.

## 1. Executive decision

Build a **Public-Service Operations Profile** on top of the shared Luminous operational kernel. Start with a privacy-minimised, read-only civic observability demonstrator using public municipal financial data and published service information. Use synthetic work cases for end-to-end workflow testing. Only progress to internal fault intake, dispatch, billing, procurement or resident-level records after a public authority approves a specific integration and data-processing purpose.

The first product should help answer four operational questions:

1. What service problem or public commitment is recorded, and which authoritative source supports that statement?
2. Which department or owner is responsible, and what is the permitted next step?
3. What has actually changed, what remains uncertain, and what evidence supports the status?
4. Did the intervention improve the measured service outcome compared with a defined baseline?

The system must **complement** South Africa's Digital Transformation Roadmap and existing municipal and national services. It must not create a parallel national identity, payment, eligibility or citizen master-data system. The national roadmap already prioritises digital public infrastructure, interoperable data exchange and more unified citizen-facing services; MyMzansi is the national channel initiative. Luminous should contribute an interoperable operational and evidence layer where a real gap is demonstrated, rather than competing with that programme.

This RFC defines a proposed integration profile, not a claim that any municipality has endorsed it, that private municipal systems are accessible, or that current Luminous components are ready for government deployment.

**Engagement posture:** the MyMzansi roadmap describes real-time operational data exchange and bulk anonymised data access as government digital-infrastructure initiatives. Its initiative page lists non-government access to experiment with agency models as an *indicative* Phase 2 milestone for Q2 2027. Treat this as a possible future engagement pathway through the Digital Services Unit—not an access entitlement, a guaranteed milestone, or permission to use administrative data. Until an official access process and its terms are published and approved, this project stays on public aggregate data and synthetic records. Reference: https://www.mymzansi.gov.za/roadmap/initiative-2

## 2. Why this profile fits the existing platform

The existing platform architecture already describes the right general model: a work case connects a request or detected need to a tenant, people, resources, workflow, authority decisions, planned work, observations, evidence and outcome. The civic profile should add jurisdiction-specific constraints and workflows to that model, not fork a second operational kernel.

The profile depends on the following existing work, whose status must be checked at the exact revision before implementation claims are made:

- **Native Work Case V1** — proposed canonical case/resource contract, typed state transitions, tenant isolation, idempotency, append-only activity, and classified evidence references. Tracked in [luminous-platform issue #7](https://github.com/Luminous-Dynamics/luminous-platform/issues/7).
- **Operations Event V1** — the shared versioned, vendor-neutral event contract, based on CloudEvents 1.0 structured JSON, with trusted source/tenant binding and durable delivery semantics. The related contract work and PR are not substitutes for a passing exact-head qualification run.
- **Government and regulated infrastructure product line** — product boundaries and jurisdiction-specific qualification are tracked in [issue #19](https://github.com/Luminous-Dynamics/luminous-platform/issues/19). This civic profile is a workflow layer within that line, not an accreditation claim.
- **Xenia Operations Fabric** — the general separation between work records, consent-scoped remote sessions, operational intent, execution and evidence should be reused, not duplicated.

If the Work Case or Operations Event contract has not passed its current exact-head tests, the civic profile remains a documented consumer proposal. A queued CI job, a test count, a synthetic demonstration, or an authored schema is not a pass.

## 3. Bounded component responsibilities

| Component | Civic-profile responsibility | Boundary |
|---|---|---|
| **Luminous Platform** | Optional deployment and operations environment for connectors, case APIs, evidence views and monitoring | It must not become a prerequisite for host boot, safe recovery or existing government channels |
| **Luminous Edge** | Compose pinned, reviewable NixOS deployment profiles for a test node, office/site node or approved service | The civic service is an optional application; do not enable boot-path changes or privileged host features by default |
| **Work-case / service-management plane** | Own tenant-scoped service cases, lifecycle, responsible owner, deadlines, external mappings and activity history | A case, inbound event, webhook or model suggestion cannot itself authorise an external action |
| **Operations Event contract** | Normalize and validate events from approved source adapters; preserve event identity, source identity, causality and revision semantics | Valid JSON does not prove trusted origin, fresh source state, lawful processing or permission |
| **Symthaea** | Advisory case classification, duplicate detection, cross-source correlation, workload estimation, anomaly explanations and candidate next steps | Recommendations are advisory. Deterministic rules and authorised staff retain authority; absence or low confidence routes to a safe review path |
| **Mycelix** | Potential later integration for selective-disclosure credentials, provenance, attestations and participatory governance where its maturity and authority model are fit for purpose | No forced migration to a decentralised identity layer. Mycelix Civic is research/pre-alpha in current project documentation and must not be treated as a deployed government service |
| **Nixward** | Observe and manage supported NixOS nodes through its evidence-bound transaction and recovery model | NixOS-specific. Do not treat it as a universal municipal OT controller or use AI output as mutation authority |
| **Sovereign Ops** | Policy, operational intent, fleet coordination and receipt verification where the service is implemented and qualified | No hidden direct host mutation and no substitution for the municipal department's legal/operational authority |
| **Xenia** | Optional, consent-scoped remote support for authorised staff endpoints during a separately approved support session | Remote support is not a way around change control. Never connect to municipal assets without explicit scope and authorisation |
| **xenia-wire / xenia-peer** | Candidate transport/session components after their independent maturity gates | Current repository documentation identifies these as pre-alpha. Do not place them in a production civic deployment on the basis of design intent alone |
| **Symtropy** | Offline simulation of synthetic work queues, staffing constraints, repair dispatch and resource-allocation policies | Simulation output is a hypothesis, not validated truth about Johannesburg, and never directly controls real operations |
| **Sol-Atlas / geospatial UI** | Optional map and spatial-context presentation after checking the existing repository contracts and maturity | A map is a view, not the authoritative work ledger; precise resident locations must not be exposed by default |
| **Prism and other research tools** | Optional, separately specified analysis/visualisation interfaces if they meet a concrete need | Keep their security and data-flow boundaries explicit; do not make a new dependency merely to complete a diagram |

This separation is intentional: a civic deployment should still support ordinary manual operation if Symthaea, Mycelix, an optional map or a remote-support service is unavailable.

### Language and runtime rule

The civic profile follows [`LANGUAGE_AND_RUNTIME_POLICY.md`](LANGUAGE_AND_RUNTIME_POLICY.md): the first release has no client-side WASM application, JavaScript loader, handwritten JavaScript/TypeScript or Node.js runtime. Use Rust/Leptos server-rendered HTML, standard links and HTTP forms. A future client-side WASM proposal requires separate review, and must not become a requirement for the core workflow. Acceptance must include a no-JavaScript workflow check.

A separate but related finding: Symthaea's current `symthaea-browser` is a Rust browser agent over Chromium/CDP, not a browser engine. Its current CDP wrapper still sends authored JavaScript strings into page contexts for some observations and actions. This has not yet been removed. The planned removal of routine page-evaluation helpers is tracked in [Symthaea Browser pure-Rust strategy PR #7427](https://github.com/Luminous-Dynamics/symthaea/pull/7427) and [implementation issue #7426](https://github.com/Luminous-Dynamics/symthaea/issues/7426). The civic UI may claim zero JavaScript runtime requirement; the wider Luminous stack must not claim zero authored/injected JavaScript until that separate controller work has been implemented and qualified.

## 4. Deployment and data-flow model

The first deployment should be read-only from source systems and should not require access to an internal municipal network.

1. **Approved source adapters** retrieve only public datasets and public notices. Each adapter records the source URL or dataset identifier, retrieval time, source-reported period, licence/terms, parser version and raw-content digest where lawful.
2. **Normalization and validation** convert those observations into versioned, typed records. Preserve original values and units. Distinguish zero, null, missing, not published, stale and invalid values.
3. **Evidence ledger** stores provenance and correction history. A correction is a new linked observation; it does not erase the earlier observation. The system records what it knows and what it cannot verify.
4. **Case / work plane** joins the observation to a case only when the mapping is supported. Ambiguous asset, supplier, department or location mappings remain unresolved; do not infer authoritative identity from a fuzzy match.
5. **Deterministic rules** implement explicit deadlines, routing policies, urgency classes and permissions. Keep the rules versioned and testable.
6. **Symthaea advisory lane** supplies suggested classification, possible duplicates, risk explanations and candidate next steps with uncertainty. Every recommendation references the evidence and model/configuration version used.
7. **Human review** shows source freshness, missing records, conflicting observations and alternative interpretations. Reviewers can reject a recommendation and record a reason.
8. **Outcome capture** measures actual verified changes. Publishing a recommendation, sending a notification, assigning a case or closing a ticket does not itself prove that a physical service was restored.

**Authority invariant:** observation, model output, case state and evidence are separate from permission to act. Any eventual write-capable adapter must use server-established tenant context, a narrow capability, a typed command, a current expected revision, idempotency, a named authoriser and postcondition verification. If authorisation or current-state evidence is missing, refuse the write and preserve the existing manual route.

**Availability invariant:** when the Luminous platform or AI is offline, people must still be able to use existing emergency, customer-service and municipal channels. Fail closed for privileged actions; fail safe for service availability.

### Source catalogue contract

The source inventory is now drafted under [`contracts/public-source-catalogue-v1/`](https://github.com/Luminous-Dynamics/luminous-platform/tree/docs/public-service-operations-profile-rfc-001/contracts/public-source-catalogue-v1/). Its JSON Schema and examples are intentionally metadata-only: each source is marked `catalogue_only` with ingestion disabled. The catalogue records the Municipal Finance API's conflicting HTTP/HTTPS endpoint descriptions, the eTender publication-coverage limitation, and the absence of an established Johannesburg Water machine-readable case feed. Future adapters must keep zero, raw null, absent rows, invalid values, and stale data distinct; preserve source revision and retrieval provenance; enforce HTTPS; and fail visibly on incomplete pagination. See the [conformance requirements](https://github.com/Luminous-Dynamics/luminous-platform/blob/docs/public-service-operations-profile-rfc-001/contracts/public-source-catalogue-v1/CONFORMANCE.md). This is a contract proposal, not a tested connector.

## 5. Practical South African data starting points

Use public data first so that the initial demonstrator does not need resident identifiers or confidential operational records.

### 5.1 Municipal finance and capital maintenance

National Treasury's [Municipal Finance Data / Municipal Money API](https://municipaldata.treasury.gov.za/docs) publishes machine-readable municipal budget and financial-performance data. The documentation describes a GET-only JSON API; its data-quality notice says completeness and trustworthiness depend on what municipalities submit, and the site marks the 2026 Q2 figures as under verification.

Use this to build a provenance-aware view of selected City of Johannesburg budget, capital acquisition, repairs-and-maintenance, creditor and expenditure measures. Show the financial period, unit, amount type, source status and data freshness. Do not infer a specific broken pipe, unpaid supplier or failed project from aggregate numbers alone.

### 5.2 Public procurement records

The [National Treasury Transparency Portal](https://data.etenders.gov.za/) exposes procurement information in Open Contracting Data Standard (OCDS) CSV, JSON and API formats. Its publication policy explicitly says it represents only a subset of data captured on eTender; municipal governments and state-owned entities are not necessarily represented unless they share data with National Treasury.

Therefore, every dashboard must display coverage and source limitations. Absence from the portal is **not** evidence that a tender, supplier, contract or payment does not exist. Any anomaly is a lead for human review, not a finding of fraud or misconduct.

### 5.3 Published water-service information

[Johannesburg Water](https://www.johannesburgwater.co.za/) publicly lists fault-logging channels, service information and customer-service standards. Its public pages include response targets for different incident types. These can seed a **public-service standard reference** and a synthetic case test pack.

Do not assume that those webpages are a machine-readable API, that public notices reveal the current internal work-order state, or that public turnaround targets are a guarantee for every incident. Preserve the date and exact source for every captured notice. An authorised internal integration is required before the system can claim to dispatch crews or verify repairs.

### 5.4 Initial data policy

For the first demonstrator:
- use public aggregate finance/procurement data and publicly published service notices, subject to source terms;
- use synthetic residents, assets, work orders, suppliers and case histories for workflow demonstrations;
- do not ingest resident names, account numbers, identity documents, precise household locations, private contact details, health information or confidential procurement material;
- do not scrape behind logins, bypass access controls, or assume that a public page grants permission to republish every item;
- publish source coverage, retrieval timestamps, provenance and limitations with all derived results.

## 6. Initial workflows

### Workflow A — civic operational observability (first)

Build a reproducible dashboard and machine-readable evidence pack for a small set of public financial/service indicators. The system should show what a source says, the period it covers, how fresh and complete it appears, how it was normalized, and what cannot be inferred from it.

Deliverables: a source catalogue, adapter contract, deterministic normalization tests, provenance records, a synthetic case lifecycle, and a plain-language limitations panel. This is the no-private-data demonstrator and can be built before a municipal partnership exists.

### Workflow B — water/service-request coordination (after written approval)

Integrate an approved fault feed or authorised case export. Deduplicate related reports, preserve original report IDs, recommend a priority based on documented rules, route to the proper owner and show service-standard timers. A human dispatcher remains responsible for operational prioritisation. Closure requires a specified form of evidence, and residents retain a clear path to challenge an incorrect status.

### Workflow C — billing, supplier payment and procurement reconciliation (later)

Join approved case, billing, contract and payment records using documented identifiers and data ownership. Surface inconsistencies, missing approvals, duplicate records or unexpected delays for review. Keep the source record and an allegation separate. Do not assign corruption, reliability or deservingness scores to named individuals.

### Explicitly out of scope for the initial profile

- autonomous water/electricity network control, emergency dispatch or public-safety decisions;
- automated benefit eligibility, credit decisions, disconnections, enforcement, licensing, permit denial or other adverse decisions about a person;
- facial recognition, resident surveillance, political profiling or person-level “risk” rankings;
- automated fraud findings or publication of unreviewed accusations;
- a replacement national ID, payment rail, case-management system or MyMzansi citizen channel;
- claims of accreditation, external audit, production readiness or municipal endorsement absent evidence.

## 7. Privacy, rights, and public accountability

The deployment must have a documented lawful purpose, data-flow inventory, role-based access, retention schedule, incident response, correction process and named accountable operator before personal information is introduced.

South Africa's Protection of Personal Information Act (POPIA) includes section 71, which restricts certain decisions with legal consequences or substantial effects that are based solely on automated processing of personal information intended to profile a person, subject to statutory provisions and safeguards. This is one reason not to automate adverse individual decisions. A deployment-specific legal review is still required; this RFC is not legal advice.

Minimum safeguards:

- **Data minimisation:** prefer aggregate or synthetic data; collect personal information only when necessary, authorised and documented.
- **Purpose limitation:** no secondary use for unrelated monitoring, political inference, advertising or model training by default.
- **Human accountability:** a named public official owns each consequential decision; the UI shows evidence, uncertainty and a contest/correction route.
- **Fair service:** priority policies must be documented and audited for geographical and socioeconomic disparities. Do not let noisy reports or internet access alone determine which neighbourhood is served.
- **Access control:** separate resident-facing, staff, investigator and administrator privileges. Enforce tenant/resource isolation at the server and storage layer, not only in the UI.
- **Auditability:** log who accessed or changed records and why, while avoiding unbounded logging of personal data or secrets.
- **Security:** threat-model connectors, imported files, dependency updates, credentials, backups, tenant isolation, offline recovery and compromise of the AI service.
- **Accessibility:** support low-bandwidth use, mobile browsers and non-digital/manual channels; plan for plain-language and relevant South African language support rather than smartphone-only service delivery.
- **Procurement neutrality:** define interfaces and export formats so a city is not locked into Luminous. Make source code, licences, operating costs, limitations and migration paths reviewable.

For the first phase, the safest person-level record is no person-level record at all.

## 8. Symthaea and Symtropy qualification plan

Do not start by measuring whether an LLM's summary sounds plausible. Use a separate deterministic baseline and an oracle based on independently specified expected outcomes.

The first evaluation corpus should be synthetic and include at least: duplicate reports about one incident; nearby but distinct incidents; ambiguous location and asset mappings; missing or stale source observations; contradictory public notices; malformed and replayed events; one source using a changed field/unit; a high-urgency case that must not be buried by popularity; and an AI outage.

Measure:

- source-normalization error and data coverage;
- duplicate precision/recall on labelled synthetic pairs;
- priority-policy violations and missed urgent cases;
- false joins between different assets, residents or suppliers;
- calibration and abstention when evidence is insufficient;
- workload/queue simulation against a deterministic baseline;
- ability to reproduce every recommendation from frozen inputs, model/configuration versions and rules;
- equity outcomes across synthetic neighbourhood profiles;
- operator correction rate and time saved without reduction in outcome quality.

Symtropy should simulate alternative queue/crew policies under explicit assumptions, including unavailability, travel constraints, incomplete reports and delayed parts. The model must report assumptions and sensitivity. A simulation win is not evidence of a real-world service improvement until compared with actual outcomes under an approved evaluation.

## 9. Rollout gates

| Phase | Permitted activity | Gate to proceed |
|---|---|---|
| **0 — offline/public-data demonstrator** | Public aggregate data, public notices, synthetic people/cases/assets; read-only | Reproducible sources, explicit freshness/coverage, deterministic tests, privacy review |
| **1 — shadow evaluation** | Approved historical or live feed; no writes; staff do not rely on recommendations for decisions | Data-use authorisation, threat model, baseline, independently labelled evaluation, rollback and outage behaviour |
| **2 — restricted operational pilot** | One named workflow and service owner; human-authorised case updates only | Signed operating procedure, least-privilege credentials, support/incident plan, data retention, audit review and acceptance thresholds |
| **3 — expansion** | Additional departments and connectors, one at a time | Separate data/controller approval and safety case for each new domain; exact-head test evidence and operational results |

No phase automatically authorises the next. A failed gate means remain in the current phase, repair the design or stop.

## 10. Acceptance criteria for the first deliverable

- A versioned source catalogue lists owner, source URL, terms/licence, retrieval method, update cadence, period, fields, unit, coverage and known gaps.
- All displayed values preserve provenance and distinguish zero from missing, null, stale and not published.
- The demonstrator operates using only public data and synthetic case records; secrets and private municipal credentials are absent.
- The case workflow uses typed legal transitions and idempotent commands from the shared work-case contract; the profile does not invent a parallel case state machine.
- Imported events bind to an allowlisted source and server-established tenant context. Invalid, conflicting, stale or replayed inputs cannot silently alter case state.
- Every AI output is marked advisory, bound to the evidence/model/configuration revision, and unable to invoke privileged mutations directly.
- A disabled or unavailable AI leaves the existing manual workflow usable. Loss of authority evidence blocks privileged writes, not access to the official service channel.
- Tests cover stale and malformed data, missing/zero distinctions, duplicate/replay delivery, wrong-tenant access, conflicting revisions, unavailable adapters, and human override.
- Exact-head tests run visibly and pass. Queued, cancelled, skipped, stale-revision or empty-discovery results are not passes.
- The public interface states what is measured, what is unknown and what must not be inferred; no person or supplier is publicly accused based on a model score.
- Success is reported against a pre-registered baseline, including failure cases and unintended effects.

## 11. Proposed next actions

1. **Stabilise shared contracts first.** Resolve the exact-head CI/runner assignment blocker already tracked in [issue #14](https://github.com/Luminous-Dynamics/luminous-platform/issues/14) and qualify Work Case V1 plus Operations Event V1. Do not stack an operational civic integration on an unqualified base.
2. **Build the public-data slice.** The executable follow-on is tracked in [issue #35](https://github.com/Luminous-Dynamics/luminous-platform/issues/35). Start with the source catalogue and observation contract, then validate synthetic Municipal Finance observations and published Johannesburg Water service guidance. Keep all ingestion disabled until the endpoint/terms gates pass.
3. **Publish a small evidence pack.** Include fixture data, source inventory, deterministic baseline results, adversarial tests, screenshots or CLI output, limitations and exact source revisions.
4. **Map each later connector to an owner.** Before accessing live case-management, billing, procurement or infrastructure systems, obtain written data-use authority and define least-privilege read/write scope.
5. **Only then approach a municipal pilot sponsor.** Propose a bounded shadow-mode evaluation with a named service owner and clear exit criteria; do not pitch “automating government” or promise savings before data supports them.

## 12. Reference sources

- South Africa's Roadmap for the Digital Transformation of Government (May 2025): https://www.gov.za/documents/other/south-africas-roadmap-digital-transformation-government-12-may-2025
- MyMzansi digital transformation roadmap: https://www.mymzansi.gov.za/roadmap
- National Treasury Municipal Finance API documentation: https://municipaldata.treasury.gov.za/docs
- National Treasury Transparency Portal and OCDS API: https://data.etenders.gov.za/
- Transparency Portal publication policy / scope limitations: https://data.etenders.gov.za/Home/LearnMore
- Protection of Personal Information Act 4 of 2013, section 71: https://www.justice.gov.za/legislation/acts/2013-004.pdf
- Johannesburg Water service information: https://www.johannesburgwater.co.za/services/
- Johannesburg Water customer fault/contact channels: https://www.johannesburgwater.co.za/contact-us/
- Luminous Platform Work Case V1 tracker: https://github.com/Luminous-Dynamics/luminous-platform/issues/7
- Government and regulated infrastructure product line: https://github.com/Luminous-Dynamics/luminous-platform/issues/19
- Hosted-runner qualification blocker: https://github.com/Luminous-Dynamics/luminous-platform/issues/14

---

**Implementation note:** This RFC is an architecture proposal and source map. It does not constitute a functioning connector, service, legal opinion, municipal partnership or production qualification.
