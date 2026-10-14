# OMSI research notes

**Research checkpoint:** 2026-10-14. Partial desk research, not a novelty study, legal review or independent stakeholder validation.

## Standards to reuse or map

| Area | Prior art | OMSI implication |
|---|---|---|
| Identity provisioning | SCIM, RFC 7644 — https://www.rfc-editor.org/rfc/rfc7644/ | Reuse user/group provisioning where useful; define explicit tenant/resource binding for operations. |
| Token audience/resource | OAuth Resource Indicators, RFC 8707 — https://www.rfc-editor.org/rfc/rfc8707.html | Use audience/resource restriction where OAuth is selected; do not invent a token format in Core 0.1. |
| Canonical JSON | RFC 8785 JCS — https://www.rfc-editor.org/rfc/rfc8785/ | Prefer a standard canonicalization algorithm unless a constrained subset is justified, cross-validated and independently reviewed. |
| Telemetry | OpenTelemetry — https://opentelemetry.io/docs/specs/otel/ | Reuse trace, metric and log conventions. |
| Cybersecurity commands | OASIS OpenC2 — https://www.oasis-open.org/standard/oc2-ls-v1-0/ | Map suitable security actions to OpenC2; keep service lifecycle and postcondition evidence separate. |
| Service topology/orchestration | TOSCA 2.0 — https://docs.oasis-open.org/tosca/TOSCA/v2.0/TOSCA-v2.0.html | Reuse topology concepts instead of cloning a complete infrastructure ontology. |
| Hardware management | DMTF Redfish — https://www.dmtf.org/standards/redfish | Integrate established hardware interfaces through a profile/adapter. |
| Controls and assessment | NIST OSCAL — https://pages.nist.gov/OSCAL/ | Reference established control catalogs, profiles and assessment objects. |
| Update metadata/delegation | The Update Framework — https://theupdateframework.io/ | Consider delegated metadata for distributing profiles/artifacts; do not equate provenance with operator authorization. |
| Incident handling | TM Forum TMF724 — https://www.tmforum.org/open-digital-architecture/open-apis/incident-management-api-TMF724/v4.0 | Avoid a competing universal case API; document loss-aware mapping. |
| MSP assurance | MSPAlliance UCS — https://mspalliance.com/ | Technical receipts can support audit, not replace organizational controls or certification. |
| MSP/RMM risk | CISA JCDC RMM Plan — https://www.cisa.gov/topics/partnerships-and-collaboration/joint-cyber-defense-collaborative/jcdc-remote-monitoring-and-management-cyber-defense-plan | Make tenant isolation, least privilege and blast-radius control foundational. |


| Event envelope | CloudEvents Core 1.0.2 (latest tagged release found) | Reuse `id`, `source`, `type`, `subject`, `time` and `dataschema` when publishing OMSI events through event buses/webhooks. CloudEvents is an envelope, not the OMSI state-transition or truth-verification model. https://github.com/cloudevents/spec/blob/main/cloudevents/spec.md |
| Trace correlation | W3C Trace Context | Propagate `traceparent`/`tracestate` for distributed diagnostics. A trace ID is not an operation ID, idempotency key, authorization subject, or durable audit sequence. https://www.w3.org/TR/trace-context/ |
| HTTP error body | RFC 9457 Problem Details | Use `application/problem+json` for HTTP API errors instead of inventing a competing generic error envelope; define OMSI-specific problem type URIs and do not leak secrets or privileged implementation details. https://www.rfc-editor.org/rfc/rfc9457.html |
| Industrial semantic and device interface | OPC UA (IEC 62541 family) | Use OPC UA information models, services and applicable companion specifications to identify/observe/control industrial assets. OMSI should wrap authorized operation intent, evidence and recovery semantics around qualified adapters, not replace plant-control protocols or safety functions. https://reference.opcfoundation.org/specs/OPC-10000-1/full |
| Server hardware management | DMTF Redfish Specification 1.23.1 and Redfish Data Model 2025.4 (latest entries listed by DMTF on 14 Oct 2026) | Map server/chassis/firmware operations through standard resource/action semantics and tested interoperability profiles where supported; do not assume a Redfish task accepted or completed equals verified host health. DMTF lists Redfish Specification 1.23.1 and Data Model 2025.4 dated 16 Jan 2026. Re-check before pinning an implementation profile. https://www.dmtf.org/standards/redfish |


| HTTP API description | OpenAPI Specification 3.2.1 (published 10 Sep 2026; current version checked 14 Oct 2026) | Use OpenAPI for the HTTP binding's machine-readable interface contract, pinned to an exact version; it describes the API but does not define authorization, async completion, idempotency persistence, or target-state truth. The older companion starter's 3.1.1 parsing result is not evidence for a 3.2.1 binding. https://spec.openapis.org/oas/v3.2.1.html |
| HTTP retry/idempotency header | IETF `draft-ietf-httpapi-idempotency-key-header-07` | The version surfaced by the IETF Datatracker expired 18 Apr 2026 and is not an RFC. Do not call this an adopted standard or make OMSI correctness depend on that header. Define operation-level idempotency/replay semantics in OMSI Core and treat any HTTP header mapping as a versioned optional binding subject to future status review. https://datatracker.ietf.org/doc/draft-ietf-httpapi-idempotency-key-header/ |
| Async job errors | RFC 9457 plus OMSI operation resource/status contract | RFC 9457 gives a standard HTTP error envelope, not an operation lifecycle or retry-safety theorem. An async-job-specific IETF draft found in this review had expired 30 Aug 2026; don't rely on it as a stable standard. OMSI should expose a durable operation reference/status model and report whether the outcome is pending or indeterminate separately from the error body. https://www.rfc-editor.org/rfc/rfc9457.html |


## Integration rule: reuse envelopes, preserve OMSI semantics

The standards above should be layered, not merged into a single universal schema:

- **CloudEvents** is a portable event envelope. OMSI owns operation identity, legal lifecycle transitions, idempotency, authority binding and the meaning of applied/verified/indeterminate.
- **W3C Trace Context / OpenTelemetry** provides diagnostic correlation and telemetry conventions. Trace context can be missing, restarted, sampled or forwarded across trust boundaries; it MUST NOT establish authority or be the only durable audit link.
- **RFC 9457** standardizes HTTP error representation. It MUST NOT be used to imply that an asynchronous operation has stopped or that a retry is safe. Return a separate operation reference and current state when work was accepted but is still running or indeterminate.
- **OPC UA and Redfish** are candidate domain-facing interfaces. Their success responses, events, task states and telemetry must be mapped to OMSI's profile-specific observations and independently evaluated postconditions. OMSI should not become a competing industrial control or hardware-management protocol.
- Conformance claims should be scoped to exact protocol versions, profiles, adapter revisions, and executed test vectors. A standards mapping table is not proof of adapter conformance.

A particularly important distinction for asynchronous APIs is that transport/HTTP acceptance, event publication, process completion and target-state verification are different propositions. Loss of a response after acceptance MUST trigger idempotency/reconciliation behavior, not an automatic repeat of a potentially non-idempotent mutation.

## Public repository documentation inspected

Selected public documentation was read for Luminous Platform, Nixward, Luminous Nix, Xenia Wire/Peer, Symthaea, Mycelix and Luminous Edge.

- Nixward documents an evidence-bound transaction lifecycle and says cognition and natural-language conversation are advisory, not authorization primitives.
- Luminous Platform describes a sovereignty-first NixOS stack, a suitable home for an ecosystem proposal but not proof of full MSP coverage.
- The Luminous Nix README references an older snapshot and says only a subset of intents was validated end-to-end; marketing-style accuracy claims should not be used as qualification evidence without reruns.
- Xenia Wire/Peer documents pre-alpha limitations; it remains optional.
- Mycelix identifies itself as pre-alpha and says maturity and multi-agent integration coverage vary by cluster; federation is not a Core 0.1 dependency.
- Symthaea documents important distinctions between working implementation evidence and regulatory compliance claims; OMSI should maintain the same discipline.


## Cross-repository design lessons: effect and receipt algebra

The Mycelix repositories contain closely related semantic work that OMSI should map to rather than duplicate:

- [MYC-EFFECT-CONVERGE-001](https://github.com/Luminous-Dynamics/mycelix/issues/2687) explicitly separates intent, dispatch attempt, provider acceptance, domain mutation/commit, durable receipt, reconciliation, external finality, and legal/obligation discharge. It warns against extracting a generic effect core or a universal `Success`/`Committed`/`Finalized` enum before independent domain families are qualified. Its candidate common layer is a weak coordination envelope of typed references, while domains keep their own meaning of effect and finality.
- [LEX-NET-028](https://github.com/Luminous-Dynamics/mycelix/issues/1215) defines anti-receipt-laundering rules: a transport receipt is not a recognition receipt, a valid signature is not issuer authority, an execution attempt is not effect confirmation, and one receipt type cannot be silently relabeled as a stronger type. It recommends immutable, typed receipt lineage and explicit qualified transitions.
- [FIN-SYNC-002C](https://github.com/Luminous-Dynamics/mycelix/issues/2521) separates recovery/compensation classification from authority and execution. Compensation is a new forward effect, not deletion of the original effect. Recovery lineage should bind the root effect and prior recovery receipts, with profile-bounded generations/depth and duplicate/cycle handling.

### OMSI design consequences

1. Keep the Core outcome vocabulary deliberately weak. Shared words such as `accepted`, `applied`, `verified`, or `recovered` require profile-specific definitions and typed evidence; they must not imply generic external finality.
2. Give distinct receipt types distinct schema identifiers and subject commitments. A signed or schema-valid weaker receipt does not become a stronger receipt without an explicit qualified transition.
3. Use linked, append-only operation and recovery records. Do not make every domain's recovery mean rollback; some changes can be reverted, while others require quarantine, safe isolation, manual reconciliation or a new compensating effect.
4. Bind retry and compensation to stable semantic operation/effect identity, not wall-clock time or attempt identity. An unknown result after dispatch is not proof of no effect and cannot by itself authorize another attempt.
5. Preserve the authority ceiling: OMSI may validate identity and lifecycle bindings, but it must not create recovery/compensation authority, execute effects, or claim provider/domain finality.

This is an architectural mapping, not evidence that the separate Mycelix mechanisms are already unified, qualified, or production-ready. The Mycelix issue explicitly says the common effect layer should not be extracted until domain-owned effect theorems are reconciled and qualified.

## Unanswered research questions

- Do independent MSPs agree this operation/evidence boundary is their highest-value interoperability gap?
- Will leading PSA/RMM vendors implement it?
- Is the working name legally and commercially available?
- Are proposed schemas backward-compatible, secure and useful at industry scale?
- Can operation evidence be trusted independently across real vendors?
- Does the local conformance starter match real operational semantics across platforms?

Next step: structured review with independent MSPs, a PSA/RMM integrator, identity/security engineers and an external protocol reviewer before freezing any wire format.


## Field-service and physical-profession research checkpoint — 2026-10-14

This is a focused desk-research snapshot, not an exhaustive market map, implementation test, legal review, or novelty claim. It informs the Field Service profile at `profiles/field-service/0.1/PROFILE.md`.

### Existing products: do not rebuild commodity workflows without evidence

| Existing project/product | Publicly documented capability | Design implication for OMSI / Luminous Platform |
|---|---|---|
| ERPNext | Maintenance schedules, visits, assigned personnel, asset maintenance plans and logs. Docs: https://docs.frappe.io/erpnext/maintenance-schedule, https://docs.frappe.io/erpnext/maintenance-visit, https://docs.frappe.io/erpnext/asset-maintenance | Treat scheduling, assets, recurrence and visit capture as established product territory; prioritize import/export and evidence/authorization interop. |
| Odoo Community Association field-service addons | Modular models for service locations, workers, orders, activities, agreements, calendars, CRM linkage, availability and invoice linkage. Repo: https://github.com/OCA/field-service | Map to an existing ecosystem instead of forcing providers to replace their system of record. Verify target Odoo/addon version and license before implementation. |
| openMAINT / CMDBuild | Asset/property inventory; scheduled and corrective maintenance; logistic and economic management; GIS/BIM support with IFC. Docs: https://www.openmaint.org/en/product/project, https://www.openmaint.org/en/product/modules, https://www.openmaint.org/en/product/modules/facility-maintenance | Existing CMMS/facilities tools already cover much of the work-order and asset layer. BIM/IFC should be optional when present, not a prerequisite for a small trade business. |
| OpenFieldService | A recent self-hostable open-source field-service project documents CRM, sites/equipment histories, dispatch, work orders, estimates, invoices/payments, service plans and technician mobile workflows. Repo: https://github.com/clawnify/OpenFieldService | Re-check maturity, license, security and maintainership before adoption. Its feature surface is evidence that basic lead-to-payment workflows are not a defensible novelty claim. |

**Product conclusion:** a second generic scheduler, CRM, job board or invoicing application is not the strongest initial differentiator. Prefer an interoperable service-operation and evidence profile that can integrate with existing tools, then build native UI/workflow only where pilots demonstrate a concrete gap. An open-source competitor's existence does not establish that it is secure, mature, production-ready or suitable for a specific deployment.

### Standards to reuse, with boundaries

| Area | Official primary source and status checked | What it can contribute | What it does not establish |
|---|---|---|---|
| Professional credentials | W3C Verifiable Credentials Data Model 2.0, W3C Recommendation, 15 May 2025 — https://www.w3.org/TR/2025/REC-vc-data-model-2.0-20250515/ | Portable issuer/holder/verifier claims and tamper-evident presentations, with privacy/accessibility considerations. | A valid proof does not by itself make a claim true, the issuer competent, the credential current, or the holder legally permitted for the exact job; verifier policy and status checks remain necessary. |
| Built assets / BIM | IFC 4.3.2.0 official documentation; IFC 4.3 published as ISO 16739-1:2024 — https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/ | Vendor-neutral structured building/infrastructure information across lifecycle stages, including maintainable asset references where data exists. | Does not define work-order authorization, service-case semantics or whether a repair passed an inspection. Do not require BIM for ordinary residential service. |
| Business documents | OASIS UBL 2.4 Standard, 20 June 2024 — https://docs.oasis-open.org/ubl/UBL-2.4.html | Reusable structured documents including orders, order responses and invoices; supports adapters to customer/procurement systems. | Does not define trade safety, legal tax compliance in every jurisdiction, payment settlement or proof of work quality. |
| Cross-enterprise traceability | GS1 EPCIS 2.0 / current ratified artefacts — https://ref.gs1.org/standards/epcis/2.0.1/, https://ref.gs1.org/standards/ | Share visibility-event data about objects/process steps, including what, when, where and business context; useful for parts, serial/lot identity or custody when needed. | Not a universal case schema and not a substitute for authorization, calibration, or inspection evidence. Use only when physical traceability creates value. |
| Software telemetry | OpenTelemetry semantic conventions — https://opentelemetry.io/docs/concepts/semantic-conventions/ | Consistent names for software logs, metrics, traces, profiles and resource attributes, useful for platform diagnostics and event correlation. | Software telemetry semantics are not a substitute for physical measurement units, calibrated instrument records, trade-specific acceptance criteria or customer acknowledgment. |

### Profile architecture implications

1. **Separate the business case from the operation.** A service case/work order contains business context and may span multiple visits, revisions and operations. Each consequential operation has its own immutable manifest, attempt identity, authorization and outcome; closing a work order does not prove all physical postconditions.
2. **Make evidence type and provenance explicit.** Provider report, customer acknowledgment, instrument reading, qualified inspection and independently verified result must remain distinct types/states. A schema-valid or signed artifact proves neither real-world truth nor legal applicability on its own.
3. **Treat jurisdiction and time as first-class scope.** Credential checks, licenses, permits, acceptance criteria and code references require jurisdiction, issuer/source, applicable scope, effective/retrieval/check times and freshness policy. A cached or stale source must never silently become current authority.
4. **Preserve privacy in the physical world.** Photos and diagnostics may reveal home interiors, precise locations, vehicle identifiers, access arrangements, layouts or business vulnerabilities. Keep raw sensitive content in access-controlled storage; exchange scoped references, digests and purpose-bound disclosures. A content hash is not anonymization.
5. **Pilot adapters before replacing product surfaces.** The first integration study should select one existing workflow system (for example ERPNext or an OCA field-service installation), record its exact version/license/export shapes, and test loss-aware export/import of source IDs, statuses, estimate versions, units, timestamps and evidence classification. No integration should be claimed from reading product documentation alone.
6. **Test a genuinely different second domain.** Plumbing/HVAC plus automotive or construction tests whether the shared envelope is actually generalizable. Keep measurements, tool calibration, parts, safety procedures, legal obligations and domain objects in explicit extension profiles rather than generic nullable fields.

### Research limits

The sources above establish that usable prior art and standards exist; they do not establish commercial demand, user willingness to switch, successful field deployment, standards conformity, or the novelty of OMSI. Before freezing a stable profile, obtain independently implemented exports, interview practitioners/customers, check the exact licenses, and run round-trip plus adversarial conformance tests. All cited versions should be re-checked at implementation time.
