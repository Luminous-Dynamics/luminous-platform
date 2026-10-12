# OMSI research notes

**Research checkpoint:** 2026-10-12. Partial desk research, not a novelty study, legal review or independent stakeholder validation.

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

## Public repository documentation inspected

Selected public documentation was read for Luminous Platform, Nixward, Luminous Nix, Xenia Wire/Peer, Symthaea, Mycelix and Luminous Edge.

- Nixward documents an evidence-bound transaction lifecycle and says cognition and natural-language conversation are advisory, not authorization primitives.
- Luminous Platform describes a sovereignty-first NixOS stack, a suitable home for an ecosystem proposal but not proof of full MSP coverage.
- The Luminous Nix README references an older snapshot and says only a subset of intents was validated end-to-end; marketing-style accuracy claims should not be used as qualification evidence without reruns.
- Xenia Wire/Peer documents pre-alpha limitations; it remains optional.
- Mycelix identifies itself as pre-alpha and says maturity and multi-agent integration coverage vary by cluster; federation is not a Core 0.1 dependency.
- Symthaea documents important distinctions between working implementation evidence and regulatory compliance claims; OMSI should maintain the same discipline.

## Unanswered research questions

- Do independent MSPs agree this operation/evidence boundary is their highest-value interoperability gap?
- Will leading PSA/RMM vendors implement it?
- Is the working name legally and commercially available?
- Are proposed schemas backward-compatible, secure and useful at industry scale?
- Can operation evidence be trusted independently across real vendors?
- Does the local conformance starter match real operational semantics across platforms?

Next step: structured review with independent MSPs, a PSA/RMM integrator, identity/security engineers and an external protocol reviewer before freezing any wire format.
