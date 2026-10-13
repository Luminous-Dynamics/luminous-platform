# OMSI research notes

**Research checkpoint:** 2026-10-13. Partial desk research, not a novelty study, legal review or independent stakeholder validation.

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


| Event envelope | CloudEvents 1.0 | Reuse `id`, `source`, `type`, `subject`, `time` and `dataschema` when publishing OMSI events through event buses/webhooks. CloudEvents is an envelope, not the OMSI state-transition or truth-verification model. https://github.com/cloudevents/spec/blob/main/cloudevents/spec.md |
| Trace correlation | W3C Trace Context | Propagate `traceparent`/`tracestate` for distributed diagnostics. A trace ID is not an operation ID, idempotency key, authorization subject, or durable audit sequence. https://www.w3.org/TR/trace-context/ |
| HTTP error body | RFC 9457 Problem Details | Use `application/problem+json` for HTTP API errors instead of inventing a competing generic error envelope; define OMSI-specific problem type URIs and do not leak secrets or privileged implementation details. https://www.rfc-editor.org/rfc/rfc9457.html |
| Industrial semantic and device interface | OPC UA (IEC 62541 family) | Use OPC UA information models, services and applicable companion specifications to identify/observe/control industrial assets. OMSI should wrap authorized operation intent, evidence and recovery semantics around qualified adapters, not replace plant-control protocols or safety functions. https://reference.opcfoundation.org/specs/OPC-10000-1/full |
| Server hardware management | DMTF Redfish | Map server/chassis/firmware operations through standard resource/action semantics and tested interoperability profiles where supported; do not assume a Redfish task accepted or completed equals verified host health. Redfish 2025.4 was listed by DMTF as published 16 Jan 2026 during this research check. https://www.dmtf.org/standards/redfish |

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
