# OMSI Field Service Profile 0.1 — Trades and Asset Maintenance

**Status:** incubation profile for review; not an adopted standard  
**Version:** 0.1-draft  
**Date:** 2026-10-14  
**Depends on:** OMSI Core pre-RFC 0.1; no stable wire-format commitment  
**Purpose:** test whether OMSI's operation, authority, lifecycle and evidence boundary can support physical-world service work without turning Core into a universal work-order schema.

## 1. Decision summary

OMSI should model *consequential operations and their evidence*, not replace every profession's service-management software. This profile adds a thin interoperability envelope for field work and explicitly delegates profession-specific records to existing domain standards or versioned adapters.

Initial use cases are plumbing/HVAC service, electrical inspection and repair, automotive maintenance, and small construction/maintenance contractors. These are examples, not a claim that one workflow or legal rule applies to all of them.

The reusable nucleus is:

1. a service case/work-order reference;
2. a site and asset reference;
3. an immutable operation manifest for each consequential action;
4. a bounded, attributable authorization decision;
5. a lifecycle/event history that distinguishes attempted work from verified outcomes;
6. evidence references with method, provenance, time, units and claim limits;
7. a closeout/handover record that preserves outstanding hazards, defects, parts, warranties and follow-up obligations.

Core remains transport-, language-, database- and vendor-neutral. A conforming implementation does not need Luminous products, AI, blockchain, Holochain or a central service.

## 2. Boundary: case lifecycle is not operation lifecycle

A service case and an OMSI operation are different state machines.

The *case/work-order* describes the customer's request and its business progress. A profile may map source states such as requested, triaged, estimated, awaiting approval, scheduled, dispatched, on site, diagnosed, work in progress, quality checked, handed over, invoiced, closed, disputed, blocked, or return visit required. These are candidate mapping vocabulary, not a universal mandatory sequence. Preserve the source system's original status and report lossy mappings.

An *operation* is one bounded consequential action such as energizing a circuit for a test, issuing a remote command, applying a repair configuration, releasing a vehicle, or marking a regulated inspection complete. It follows OMSI Core's operation lifecycle and needs its own immutable manifest, authority, attempt identity, outcome and evidence.

A work order reaching “closed” does not establish that every operation succeeded. A command accepted by a tool does not establish that the physical system reached its intended state. An invoice paid does not establish workmanship quality. A provider's completion statement is evidence of a provider claim—not independent verification.

## 3. Shared resource vocabulary

The profile uses references rather than imposing a complete shared ontology:

| Concept | Minimum interoperability intent | Domain-owned detail |
|---|---|---|
| Service case | Source namespace, external ID, tenant, source revision, correlation links | PSA/field-service case schema and source lifecycle |
| Work order / task | Stable reference, scope version, assigned party, status provenance | Dispatch, labor operations, project plan, dependencies |
| Site | Stable scoped reference, jurisdiction, location precision policy | Address system, floor/room hierarchy, geospatial/BIM representation |
| Asset / system | Stable reference, asset class, manufacturer/model when known, identity source | Plumbing network, electrical panel, vehicle, HVAC unit, building element |
| Practitioner / crew | Holder and issuer references, role, capability/credential references, validity | Trade license, endorsements, employer rules, apprentice supervision |
| Estimate / approval | Versioned scope and amount, currency, exclusions, approval actor and time | Taxes, rate cards, quotes, customer contract and local consumer rules |
| Material / part | Part identity, quantity, unit, lot/serial when applicable, provenance | Catalog, compatibility, inventory, hazardous-material classifications |
| Observation | Subject, value, unit, method, time, instrument/source and uncertainty when known | Pressure, temperature, voltage, diagnostic code, inspection checklist |
| Evidence | Immutable content digest or durable authorized reference, source, custody and disclosure scope | Photo, test report, scan, inspection certificate, sign-off |
| Handover / warranty | Scope completed, residual risks, exceptions, customer acknowledgment, next action | Profession-specific warranty terms, maintenance interval, statutory documents |

Unknown values must remain unknown rather than be guessed. A mapping adapter must not manufacture a global asset ID from a locally scoped label without recording the mapping authority and confidence.

## 4. Reference workflow

A typical physical-service journey can be expressed as the following sequence, with optional or repeated steps and profile-defined transitions:

1. Intake: create or map the service case; capture customer intent and the reported symptom as a claim.
2. Triage: classify urgency and risks; identify missing facts. Symthaea may recommend questions or diagnostic checks, but the system records those as recommendations.
3. Qualification: check the assigned practitioner's required credentials and scope against the relevant jurisdiction's authoritative source where available. A cached credential must carry issuer, retrieval time and status limits.
4. Estimate: produce a versioned scope, parts/labor estimate, exclusions and expiration. Changes to scope create a new estimate version.
5. Approval: record the customer's acceptance of that specific version, amount and work scope. Silence, a booking, or a generic account consent must not silently authorize extra work.
6. Dispatch and arrival: record assignee, site, appointment and handover changes. Offline activity may be queued locally, but synchronization is not equivalent to server acceptance.
7. Diagnosis and execution: create separate OMSI operations for consequential actions. Check authorization and preconditions at execution time, not only at planning time.
8. Verification: state what was measured or inspected, by which method, against which acceptance criterion, and by whom. The verifier's independence is a policy fact requiring evidence, not a boolean asserted by the provider.
9. Handover: list work performed, materials, customer-visible results, known defects, isolations left in place, safety instructions, limitations, warranty and next steps.
10. Billing and follow-up: exchange invoices through the selected business-document adapter; schedule maintenance and record callbacks/claims without erasing prior history.

Not all service types need every step. The profile must not force a mechanic, plumber and construction crew through the same workflow.

## 5. Authorization and safety rules

An OMSI authorization grant permits one bounded operation, not a whole job by implication. Implementations should evaluate the intersection of:

- authenticated tenant membership and role;
- target/site/asset scope;
- immutable operation manifest and current plan/scope version;
- customer approval for the relevant scope when required;
- required professional credential, qualification or supervision;
- safety preconditions, instrument status and applicable procedure;
- grant validity, expiry, revocation and replay state;
- the execution adapter's declared capability and environment.

If any required condition cannot be established, the outcome is *not authorized*, *blocked* or *indeterminate* as appropriate. Do not use an AI recommendation, trust score, successful login, provider reputation or invoice payment as authority.

The profile does not authorize software to perform licensed or safety-critical trade work autonomously. It does not override physical interlocks, lockout/tagout, electrical isolation, vehicle safety procedures, building codes, permits, inspections, manufacturer instructions, or competent-authority decisions. Jurisdictional policy packs must be reviewed and maintained separately; a credential record is not itself a complete legal compliance determination.

Remote assistance must be scoped separately. A service relationship or customer ticket never grants unattended access. Use Xenia only as an optional consent-gated remote-session adapter with its own target binding, session duration, operator permissions, revocation and action record. Physical-site work still needs its own accountable practitioner and safety process.

## 6. Evidence profile: claims are not interchangeable

Evidence should declare at least:

- evidence ID and type;
- claim/operation/work-order references;
- issuer or producing principal and source system;
- observed time and separately recorded ingestion/retrieval time when relevant;
- method, procedure version and applicable acceptance criterion;
- subject asset/site reference and scope;
- units, calibration/certification reference and uncertainty where applicable;
- content digest or durable authorized reference, plus access/privacy classification;
- correction/supersession link and known limitations.

Use explicit result classes where relevant:

- **reported_by_provider** — provider assertion only;
- **acknowledged_by_customer** — a named customer acknowledged a bounded statement;
- **measured_by_instrument** — instrument output with method and provenance, not necessarily proof that the instrument was calibrated or used correctly;
- **inspected_by_qualified_party** — inspection claim plus credential and scope evidence;
- **independently_verified** — only where the profile defines independence criteria and the required evidence is present;
- **disputed**, **unavailable**, **stale**, or **not_applicable** — explicit states, never silently coerced to success.

A photograph of a replaced pipe is not proof that a leak test passed. A diagnostic code clearing is not proof that the underlying vehicle fault is repaired. A contractor's upload of an inspection form is not proof that the responsible authority accepted it. A certificate's format or signature does not by itself prove that the issuer was competent, authorized, current or applicable to the exact work.

Corrections append a new claim linked to the superseded claim; they do not erase the original. Independent reviewers must not be treated as independent merely because they have different usernames or organizations declared in an unverified payload.

## 7. Standards and adapter strategy

OMSI should reuse and map standards where they fit; it should not promote this table into an implicit dependency list.

| Concern | Candidate prior art | Integration rule |
|---|---|---|
| Case/incident exchange | TM Forum TMF724 and existing PSA/field-service APIs | Maintain provider-specific IDs and source status. Test both directions and report every lossy mapping; do not clone a universal ticket schema. |
| Events | CloudEvents 1.0.2 | Reuse event envelope fields where suitable. CloudEvents does not define OMSI authorization, transition validity or truth. |
| API description | OpenAPI | Publish versioned interface descriptions; do not confuse schema validity with correct side effects or authorization. |
| Credentials | W3C Verifiable Credentials Data Model 2.0 | Exchange issuer/holder/verifier claims with status and privacy controls; still check issuer trust, scope, currentness and jurisdiction. |
| Orders/invoices | OASIS UBL 2.4 and local e-invoicing profiles | Use business-document adapters; OMSI does not define taxes, payment rails or invoice law. |
| Built assets | buildingSMART IFC 4.3 / ISO 16739-1:2024 | Refer to BIM object IDs and exchange files where present; do not require every small service provider to run BIM. |
| Asset / part traceability | GS1 EPCIS, product identifiers and domain catalogs | Add only where custody, serial/lot tracking or supply-chain interoperability justifies it. |
| Diagnostics | OpenTelemetry for software telemetry; OPC UA or applicable manufacturer/industry protocols for industrial assets | Do not treat general telemetry or a device protocol as a field-service case model or safety case. |

All adapters must pin the specific schema/specification version and artifact digest, license/applicability notes, mapping version, and known semantic gaps. Verify the latest official artifact before freezing an implementation. Local legal and licensing rules remain authoritative for the task and jurisdiction.

## 8. Privacy and retention

Field-service data can expose homes, occupants, vehicle identifiers, access points, building layouts, business operations and security weaknesses. Store the minimum necessary. Do not put raw photographs, identity documents, door codes, precise private addresses, access tokens or full diagnostic dumps onto a public or replicated ledger.

Keep sensitive evidence in controlled storage and exchange only scoped references and integrity metadata. Define retention, customer access, correction, deletion where legally applicable, export, redaction and dispute holds. Hashes can still permit correlation and are not automatically anonymous. Consent to perform a repair is not blanket consent to reuse photos, recordings or telemetry for AI training.

Mycelix may carry verifiable claims or provenance references, but federation and public anchoring are optional profile implementations—not prerequisites. A credential revocation or correction must propagate according to an explicit status/freshness policy.

## 9. Failure and adversarial cases

A conformance suite for this profile must include positive and negative fixtures for at least:

1. Provider-reported completion mislabelled as independent verification.
2. Work order from tenant A paired with a site, credential, approval or asset belonging to tenant B.
3. Estimate changes after approval without renewed approval.
4. Expired, revoked, wrong-jurisdiction or wrong-scope credential.
5. Measurement with missing/mismatched units, stale calibration, or impossible value bounds.
6. Duplicate/replayed execution request; same idempotency key with a different manifest.
7. Offline operation synchronized after its authority or approval expired.
8. Work performed but a postcondition check fails or is unavailable.
9. Return visit or defect discovered after a case was closed.
10. A remote session revoked during work; ensure subsequent actions are denied and ambiguous in-flight effects are reconciled.
11. Photo/document evidence accessible to an unauthorized tenant or recipient.
12. Mapping loses priority, lifecycle, currency, units, warranty terms or a safety exception without a loss declaration.
13. Unsafe automation request blocked by a missing human/qualified-practitioner approval.
14. Correction or disputed completion erases or rewrites prior evidence rather than linking a superseding record.

Unknown action/profile, unknown critical field, missing evidence reference or unresolved mapping conflict must fail closed for consequential actions.

## 10. Acceptance gates before claiming interoperability

- Two independent provider/export implementations map into the profile without requiring the same vendor or Luminous software.
- One plumbing/HVAC workflow and one materially different workflow (e.g. automotive or construction) produce fixtures using the same common envelope and distinct domain adapters.
- Round-trip tests preserve source IDs, original statuses, amounts/currencies, units, timestamps, approval version, exception and evidence provenance; any loss is machine-detectable.
- Negative controls above are rejected by an executable validator, not merely documented.
- Cross-language golden vectors and a second independent implementation are required before canonicalization or wire-format stability claims.
- Authorization, credential revocation, offline reconciliation and evidence access receive separate tests; schema conformance alone is not a production security qualification.
- A practitioner/customer walkthrough demonstrates reduced duplicate entry and a real operational benefit without weakening safety or privacy.
- Independent tradespeople, MSPs, customer representatives and relevant domain/security reviewers review the profile before it is described as a community standard.

## 11. Staged implementation

**Stage A — contract only:** keep this as an incubation profile with synthetic fixtures and a field-by-field mapping table. No production mutations or credential authority.

**Stage B — read-only adapters:** import/export service cases, assets, estimates and evidence references from existing systems. Preserve source-of-truth boundaries; no autonomous dispatch or equipment control.

**Stage C — bounded workflow pilot:** approvals, scheduling, offline capture and handover for one trade, with real practitioners and jurisdiction-specific review. Measure administrative time, first-time-fix rate, repeat visits and unsafe/incorrect recommendations against a baseline.

**Stage D — multi-provider interoperability:** add a second independently implemented connector, disputes/corrections, and verifiable credential/status checks. Only then consider cross-business marketplace and cooperative purchasing workflows.

## 12. Relationship to Luminous components

- **Luminous Platform:** owns service workflow, customer approval, tenant isolation, job records, document adapters and operator interfaces.
- **Symthaea:** can help triage symptoms, propose diagnostic plans, detect missing evidence and estimate uncertainty. Its outputs remain advisory until a bounded action has been independently authorized.
- **Mycelix:** may exchange practitioner credentials, attestations, corrections and provider relationships with privacy and appeal controls. Reputation never substitutes for qualification or authority.
- **Xenia:** optional consent-gated remote-session path; not the case manager, credential authority or safety interlock.
- **Nixward:** suitable for its own bounded host-management profile, not for controlling trade equipment by analogy.
- **Symtropy / simulation:** can model scheduling, inventories, maintenance and coordination hypotheses; simulated outcomes must not be promoted to measured field evidence.

## 13. Cross-project integration seams

This profile should compose with current public proposals without treating any draft PR as a completed dependency:

- **Luminous Platform Work Case V1 — [PR #8](https://github.com/Luminous-Dynamics/luminous-platform/pull/8):** the case/work-order lifecycle, tenant scope, revisions, idempotency and evidence references belong at the service workflow layer. The OMSI operation lifecycle remains a separate state machine. Read that PR's current qualification scope before treating the Rust/PostgreSQL slice as production-ready.
- **Mycelix life/civic service-journey contract — [PR #5019](https://github.com/Luminous-Dynamics/mycelix/pull/5019):** reuse compatible ideas for source evidence, service journeys, jurisdiction-bound applicability, currentness and conflict representation through a versioned adapter. That contract is civic-service focused; it is not a universal physical-trade work-order model and must not be pulled in as a hard dependency.
- **Symthaea life/civic domain mastery — [PR #7443](https://github.com/Luminous-Dynamics/symthaea/pull/7443):** reuse its evaluation discipline for publication versus applicability, source freshness, conflicting sources, provider claims versus independent observation, and bounded claims. Keep these as cognition/evaluation patterns, not an authority primitive.
- **Symthaea MSP diagnostic wedge — [logparse scope](https://github.com/Luminous-Dynamics/symthaea/blob/main/crates/core/symthaea-logparse/README.md):** this remains a research spike; the README says it is not yet wired to the real Symthaea HDC encoder. Do not generalize the diagnostic model to trades until the IT-operations evaluation passes its defined gate and a physical-trade dataset/protocol is separately validated.
- **Xenia:** may secure an optional remote-support session with bounded consent and revocation. It does not replace onsite responsibility, trade safety procedures, credential checking or work-order state.
- **Mycelix trust/federation:** optional for exchanging issuer-backed credentials and correction-linked attestations. Do not make global reputation, a ledger or consensus the source of legal authority.

The cross-project contract should be IDs, versioned references, authority scope and evidence semantics—not shared database tables, implicit trust between services, or a circular crate dependency.

## 14. Open questions for reviewers

1. Is “Field Service” a suitable profile name, or should the profile split into “asset maintenance” and “project delivery” before 0.1?
2. Which existing service-case and work-order models provide representative, legally shareable exports for plumbing/HVAC and automotive?
3. Which fields belong in the interoperable minimum versus a profession-specific extension?
4. What minimum evidence can a customer verify without exposing home/business security details?
5. Which organizations or practitioner groups can independently review the safety and credential mappings?
6. Is an explicit jurisdiction-pack manifest with source version, effective date, review date and expiry required in the first profile?

**Claim boundary:** this document is a proposal. It has not been through an industry standards process, legal review, independent implementation, live field trial, security assessment, or conformance run. It does not establish that any Luminous component is certified for regulated trade work.
