# OMSI Cross-Industry Extension Architecture

**Status:** design proposal for discussion; not an adopted standard  
**Date:** 2026-10-15  
**Scope:** architecture and research priorities only; does not expand the normative scope of OMSI Core 0.1 or claim a working cross-industry connector

## Executive recommendation

**Reuse the rigorous parts across industries; do not flatten industry meaning into a universal business schema.**

The current OMSI proposal is deliberately focused on managed-service operations. Its useful reusable concepts include stable scoped identity, explicit authority boundaries, lifecycle semantics, evidence receipts, idempotency, recovery, and conformance tests. The field-service adapter work adds a complementary pattern: loss-aware projections of source records that preserve source meaning and state what cannot be inferred.

These are related but not identical problems. A maintenance log or supply-chain event may need to be exchanged without authorizing any action. A command to change a production line or administer a clinical system additionally needs a domain-specific authority and effect contract. The architecture must not require every fact to be an executable operation.

Recommended structure:

1. **Interchange and evidence envelope:** source identity, tenant/scope, source and snapshot revision, temporal meaning, provenance edges, typed claims, mapping decisions, evidence references, privacy constraints, and conformance metadata.
2. **OMSI operation contract:** the narrower manifest / authority grant / attempt / lifecycle / recovery / receipt model, required when a profile can cause a consequential effect.
3. **Industry profile:** domain concepts, permitted relationships, units, lifecycle meanings, safety rules, retention and assurance requirements.
4. **Vendor adapter:** source-version-pinned extraction and mapping rules; no authority to reinterpret unknown states or silently upgrade claims.
5. **Transport and encoding bindings:** HTTP, event streams, files, OPC UA, or other suitable mechanisms without changing the semantic contract.

This is a proposed separation of concerns, not a claim that the existing schemas already implement a universal interchange envelope.

## Why not make one giant schema?

A universal `WorkItem`, `Status`, or `Evidence` object would hide real differences:

- A maintenance task, a manufacturing process segment, a clinical observation, a construction issue, and a shipment event are not interchangeable merely because all have an identifier and a status.
- A vendor status like `Completed` is a source assertion; it does not establish that work was independently verified, that a treatment occurred, that product quality passed, or that goods were delivered to the right party.
- Time can represent a planned window, business-effective date, physical event time, source-record revision, database commit, export time, or ingestion time. These must not be conflated.
- An identifier is only unique in its declared namespace. A record name alone cannot establish a tenant, site, facility, patient, asset, project or consignment boundary.
- A boolean evidence flag, an attachment reference, a signed statement, an independently observed result and a legally recognized credential have different assurance.

The common layer should standardize how meaning and uncertainty are carried—not claim that the meanings are identical.

## Layer contracts

### A. Interchange and evidence envelope

A portable projection should carry the following when applicable:

| Contract | Minimum fields / semantics | Fail-closed boundary |
|---|---|---|
| Source identity | Source system and instance namespace, object type, stable external ID, source-contract/version pin | Reject ambiguous identity or unsupported source contract |
| Scope | Trusted binding ID and resolved tenant/security domain; source-side organization/site reference separately | Never derive authorization from a customer-supplied tenant/company field |
| Source revision | Native revision token or source version if supplied; preserve original representation | Missing revision is explicit; do not infer it from export time |
| Snapshot consistency | Snapshot/export ID, extraction interval, watermark/transaction token if available, declared consistency mode and omissions | A per-record export is not atomic unless the source proves it |
| Time | Distinct effective/event/recorded/modified/exported/ingested times; original precision and timezone semantics | No silent timezone invention, precision loss, or clock-trust claim |
| Provenance | Typed predecessor/derivation/creation relationships with scoped references; bounded traversal result | An edge is not proof that its target was retrieved or the chain is complete |
| Claims | Subject, predicate/claim type, value, source, scope, time applicability, and any known uncertainty | Preserve the source assertion; do not promote it into an independently verified fact |
| Mapping disposition | Versioned mapping rule plus separate reject, conflict, loss, unmapped and unavailable results | No guessed enum, false default, or silent omission |
| Evidence references | Evidence kind, stable reference, media type and digest where applicable, access/retention policy, acquisition context | A digest is an integrity identifier, not proof of origin, truth or custody |
| Privacy | Purpose, minimization rule, sensitivity and export exclusions appropriate to the profile | No raw sensitive fields or artifacts unless explicitly allowed and justified |
| Conformance | Profile/schema/adapter versions, exact source pins, test-vector set and exact evidence/receipt references | Stale, queued or incomplete verification cannot be relabeled as pass |

An envelope field may be optional because a source does not expose it. Absence must not be silently turned into a negative fact or a successful check. Source-native content can remain in a restricted, namespaced extension when the profile allows it; consumers must not mistake that extension for universally understood semantics.

### B. OMSI operation contract

The operation contract applies only when the profile represents requested or consequential effects. It should retain OMSI's existing separation of immutable intent, authority, execution, attempts, recovery and postcondition verification. It must not be weakened just to accommodate read-only record exchange.

A profile which authorizes physical or high-impact effects must define, at minimum:

- The exact effect and target that are authorized, including preconditions and units where applicable.
- Who may authorize and execute it, including scope, expiry, revocation and separation-of-duty requirements.
- What a safe state transition means, including indeterminate outcomes and race conditions.
- Profile-specific postconditions and how they are observed; whether the verifier must be independent of the executor.
- Recovery/compensation semantics, including cases where an effect cannot be rolled back.
- Which actions are prohibited through the adapter even if a source API happens to expose them.

A general-purpose record reader must not gain write authority merely because it uses the same envelope.

### C. Industry profile

Each profile defines the domain objects and meaning that cannot be standardized safely at the envelope level:

- entities and relationships;
- source lifecycle states and legal transitions;
- units, quantities, measurement and calibration semantics;
- event/effective-time interpretation;
- required evidence, qualification or approval rules;
- hazard classes, operating limits and human-review gates;
- data sensitivity, lawful purpose, retention and permitted recipients;
- mapping and negative-control test vectors.

A profile can reference established standards rather than duplicate them. The adapter's job is to document the version and subset it supports, identify non-equivalence and preserve unmapped data safely.

### D. Vendor adapter

Adapters are pinned to an exact product release/commit and relevant schema/controller behavior. They should be deterministic and read-only by default, and should expose both the projection and its limitations.

An adapter must not infer complete history from a single record, tenant authorization from a source company field, credential validity from assignment, evidence authenticity from attachment presence, or verified outcomes from a lifecycle status. A live connector is a separate qualification step, not a consequence of passing synthetic fixtures.

## Bounded revision and amendment-lineage contract

The ERPNext `amended_from` edge is a useful test of this architecture. A reference to a predecessor is not evidence that the predecessor has been read or that the entire lineage is present. The next design should define a bounded reader before adding a broader provenance-completeness claim.

### Request boundary

Every lineage read should be explicitly bound to:

- a pinned source product/version and source-instance namespace;
- a trusted tenant/site binding and allowed record types;
- one root record reference and a version/revision policy;
- hard-configured maximum nodes, depth, pages/retries and execution budget;
- a declared snapshot-consistency strategy, when the source supports one.

Bounds must be enforced by the reader, not merely reported after traversal. Values are policy/configuration chosen for each adapter and recorded with the result; they should not be silently relaxed by the input record.

### Traversal rules

1. Resolve each node by the complete namespaced key (source instance, object type, external ID and, where available, revision).
2. Maintain a visited set of those keys. A self-edge or cycle invalidates a complete-lineage claim. Preserve diagnostic references only if doing so is safe; never loop indefinitely.
3. Validate every edge against the same trusted source and scope. A cross-instance or cross-tenant reference is a trust-boundary rejection, not a node to fetch opportunistically.
4. Preserve the raw source revision token and source-native modified-time representation per node. Parse timestamps only under a pinned, tested source-time policy; retain timezone and precision uncertainty.
5. Detect a missing ancestor, a source-side not-found, an inaccessible predecessor, a page/record/depth limit, pagination gaps, duplicate keys with conflicting revisions, revoked authorization, and source mutation during traversal.
6. If the source supports a consistent snapshot, use and record its token/watermark. Otherwise declare the weaker per-record read mode. Recheck a root/version token or equivalent before finalizing where possible; if the source changes mid-walk, return an inconsistent/incomplete result rather than pretending the walk was atomic.
7. Sort returned nodes and edges deterministically for identical input, source revisions and policy. Preserve discovery diagnostics and do not collapse different revisions of one logical record.
8. Keep source acquisition time distinct from record event time, source revision time and database transaction time. A clock conversion rule must be explicit and tested; naive timestamps must not be presumed UTC.

### Result semantics

At minimum, the adapter should emit a distinct, machine-readable result for:

- `complete_under_declared_scope`: every reachable predecessor in the declared relation was resolved under the pinned source/snapshot policy and all configured completeness conditions passed;
- `incomplete_missing_ancestor`: a referenced predecessor could not be found;
- `incomplete_unavailable_ancestor`: a predecessor exists or may exist but cannot be read;
- `incomplete_truncated_by_limit`: depth, node, page or execution bounds were reached;
- `incomplete_non_atomic_snapshot`: only if the caller needs atomic lineage and the source could not provide/establish it;
- `inconsistent_source_during_read`: revision/watermark checks show mutation or contradictory versions during traversal;
- `rejected_scope_or_source_mismatch`: trusted binding or source namespace does not match;
- `rejected_cycle`: a cycle violates the declared lineage relation;
- `rejected_malformed_reference`: nullability/format rules are violated.

Use the specific taxonomy that fits the pinned source contract; do not convert every incomplete case into rejection when a useful partial graph can be preserved safely. Crucially, **only the complete outcome may support a completeness claim**, and only within the exact stated scope and consistency level. Even then it does not prove that the upstream source itself recorded every event that occurred in the real world.

### Required lineage negative controls

- Root refers to itself.
- Two nodes form a cycle.
- A valid predecessor exists outside the returned snapshot.
- An ancestor is inaccessible under the current principal.
- A reference crosses a site, tenant or source-instance boundary.
- The same logical record is returned with conflicting revisions.
- Pagination repeats or skips an item.
- The root changes while ancestors are being traversed.
- Maximum depth, node count, page count and time budget each terminate predictably.
- Source timestamp is naive, timezone-configured, precision-limited or contradictory with the export clock.
- A partial graph cannot serialize as `complete_under_declared_scope`.
- A consistent snapshot and a non-atomic per-record read produce different declared assurance, even when their visible node sets happen to match.

## Candidate industry expansion matrix

The sequence below prioritizes reuse of the current field-service work. It is a research roadmap, not evidence that any adapter exists.

**Pilot status:** the first synthetic manufacturing asset-operations profile, eight conformance vectors, and a JSON Schema for the fixture file are now proposed in [the profile](profiles/manufacturing-asset-operations/0.1/PROFILE.md), [vector set](profiles/manufacturing-asset-operations/0.1/CONFORMANCE-VECTORS.json), and [fixture schema](profiles/manufacturing-asset-operations/0.1/CONFORMANCE-VECTORS.schema.json). The schema describes fixture structure only; there is not yet an executable profile-specific semantic validator. These files do not imply a vendor adapter, live source access or industrial control capability.

| Priority | Industry/profile | Existing standards to map to | Why it fits | Distinct risks and first test slice |
|---|---|---|---|---|
| 1 | Manufacturing and industrial asset operations | ISA-95 / IEC 62264; OPC UA information models where used; ISO 55000 asset-management vocabulary | Strong bridge from maintenance assets/tasks to production, quality and operations without replacing manufacturing semantics | Start with read-only asset-maintenance work order + quality/measurement record; test equipment hierarchy, units, calibration, production-vs-business time, interlocks and no-actuation |
| 2 | Supply chain and traceability | GS1 EPCIS and Core Business Vocabulary (CBV) | Already event-centric and designed for interoperable visibility across organizations | Start with synthetic receive/ship/transform events; test object identity, location, business step, custody link, event time, sensor values and cross-company boundaries |
| 3 | Construction and built-asset lifecycle | buildingSMART IFC (ISO 16739-1), plus appropriate issue/document exchange standards | Connects models, locations, built assets, inspections, commissioning and ongoing maintenance | Start with asset/space reference + defect/inspection evidence; test GUID stability, model version, spatial context, change lineage and inspection status vs approval |
| 4 | Healthcare administration and interoperability | HL7 FHIR Provenance and the applicable FHIR release/profile | Clear precedent for version-specific provenance, actors, clinical resources and audit semantics | Start only with synthetic, non-clinical operational or equipment-maintenance records; validate privacy, identity, consent/purpose and release-specific semantics before considering patient-linked data |
| 5 | Energy and utilities | Selected IEC domain models and OPC UA profiles, with asset/telemetry semantics declared explicitly | Reuses scoped assets, observations, events and work records | Start with read-only equipment telemetry + maintenance case; units, sampling intervals, clock quality, calibration, alarms and safety boundaries are mandatory |

References and release pins must be verified when each profile is implemented. These source standards are complementary domain models, not schemas to merge indiscriminately into OMSI.

### Why manufacturing first?

Manufacturing is the best next experiment because it shares the strongest conceptual overlap with field service—assets, work, maintenance, inspections and evidence—while introducing meaningful new constraints: production context, measurement and units, calibration, quality dispositions, and operational technology boundaries. It can test whether the envelope is genuinely reusable without beginning with patient data or direct industrial actuation.

For orientation: ISA describes ISA-95 as an enterprise/control integration model spanning manufacturing and business functions ([ISA-95](https://www.isa.org/standards-and-publications/isa-standards/isa-95-standard)); ISO 55000:2024 provides asset-management terminology, overview and principles ([ISO 55000](https://www.iso.org/standard/83053.html)). GS1 describes EPCIS/CBV as standards for sharing interoperable event visibility and supply-chain context ([EPCIS & CBV](https://www.gs1.org/standards/epcis)). buildingSMART describes IFC as an open model for machine-interpretable built-asset information ([IFC](https://www.buildingsmart.org/standards/bsi-standards/industry-foundation-classes/)). FHIR Provenance tracks activities, agents and versions that produced, revised, deleted or signed a resource ([FHIR Provenance R5](https://hl7.org/fhir/R5/provenance.html)). These references establish useful prior art, not OMSI conformance.

## Cross-profile conformance architecture

Separate tests into three reusable layers:

1. **Envelope conformance:** namespaced IDs, tenant binding, source/version pin, temporal precision, snapshot consistency, provenance traversal, privacy minimization, deterministic serialization, and explicit loss/conflict/unmapped behavior.
2. **Domain conformance:** meaningful entity links, units, lifecycle rules, safety/qualification boundaries and domain-specific evidence assertions.
3. **Adapter/integration qualification:** exact source revision, synthetic and reviewed snapshots, pagination/revocation/staleness/failure injection, live tenant isolation, no-write tests where read-only is claimed, and independent consumer interoperability.

Every profile should inherit the envelope's negative controls and add its own domain controls. Passing envelope tests must not imply domain conformance; passing synthetic adapter tests must not imply a live integration or production readiness. Keep exact-head CI evidence, fixtures and result receipts attached to the exact version under review. A queued or stale workflow is not a pass.

## Profile authoring template

Each new profile should provide:

1. Scope, non-goals, intended users and prohibited uses.
2. Normative source standards and exact versions/subsets.
3. Canonical domain entities, identifier namespaces and relationships.
4. Temporal/event semantics, units and measurement rules.
5. Lifecycle mapping table retaining raw states and legal transitions.
6. Claim/evidence types and explicit assurance ceilings.
7. Privacy, safety, authorization and retention requirements.
8. Adapter source pins, mapping losses/conflicts and extension policy.
9. Positive fixtures, negative controls, round-trip/consumer tests and capability declaration.
10. Conformance level and evidence gaps; no maturity or certification label without a separate governance scheme.

## Scope and naming guardrail

The words “Managed Services” in OMSI's working name imply a narrower problem than all industrial interoperability. This note does **not** rename OMSI, claim trademark clearance, or expand Core 0.1 automatically. There are two credible paths for later review:

- keep OMSI narrowly focused on managed-service operations and publish reusable domain profiles where they clearly fit; or
- define a broader, separately named interoperability/evidence envelope that OMSI and other profiles can depend on, with governance and namespace ownership reviewed openly.

Choose between these only after independent feedback and prior-art/name review. Do not claim a universal industry standard while the proposal remains a Luminous-authored pre-RFC.

## Proposed next milestones

1. Agree on the boundary between the interchange/evidence envelope and the existing operation/authority core.
2. Specify the bounded lineage result contract and add it as a reusable invariant to the field-service conformance matrix.
3. Create one synthetic manufacturing maintenance/quality fixture set and map it to the proposed envelope without a live connector.
4. Compare the same envelope fields against the current OCA and ERPNext projections; record irreducible differences instead of forcing equivalence.
5. Add an independently implemented consumer or verifier and exact-head golden vectors.
6. Obtain external domain, security, privacy and interoperability review before presenting this as a general-purpose standard.

## Limitations

This document is an architecture proposal informed by a bounded desk review. It does not establish novelty, universal applicability, legal or regulatory compliance, certification, live interoperability, source-system truth, cryptographic authenticity, atomic snapshots, or production safety. No implementation status should be inferred from the roadmap.
