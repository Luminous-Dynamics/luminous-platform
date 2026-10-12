# Pre-RFC: OMSI Core Interoperability Contract

- **Working identifier:** OMSI-0001
- **Status:** pre-RFC draft for review; not an adopted standard
- **Date:** 2026-10-12
- **Proposed initial editor:** Luminous Dynamics; external co-authors and maintainers not appointed
- **Working name:** Open Managed Services Interoperability (OMSI); name/acronym clearance pending

## Summary

Define a vendor-neutral, transport-independent contract for managed-service operations: tenant and resource binding, typed operation manifests, explicit authority grants, lifecycle events, portable evidence receipts and interoperable conformance tests. An initial HTTP binding may use OpenAPI while core semantics remain independent of transport and implementation language.

## Motivation

Organizations assemble managed IT from service desk/PSA tools, RMM agents, cloud and endpoint managers, identity providers, backup products, security tools and specialist providers. Integrations can exchange records or commands without a shared cross-vendor contract for the exact tenant/resource targeted, the action authorized, what execution state means, or what evidence is sufficient to claim a postcondition was verified.

This proposal targets that boundary. It does not assume the overall problem is novel; it proposes a small contract over adjacent standards and service-specific profiles.

MSP/RMM compromise can have outsized impact because management infrastructure may provide paths into many customer networks. CISA's JCDC Remote Monitoring and Management Cyber Defense Plan is relevant prior art: https://www.cisa.gov/topics/partnerships-and-collaboration/joint-cyber-defense-collaborative/jcdc-remote-monitoring-and-management-cyber-defense-plan.

## Goals

1. Bind each consequential action to one tenant, stable target, action profile/verb, canonical argument digest and valid authority grant.
2. Distinguish intent, validation, authorization, acceptance, execution, application and independent verification.
3. Represent ambiguous execution as **indeterminate**; never infer success from a transport response or process exit code alone.
4. Make evidence portable and machine-validatable across providers and tools.
5. Define negative tests for cross-tenant access, replay, stale plans, authorization substitution, secret leakage and false verification.
6. Permit independent implementations without requiring Luminous code, Rust, NixOS, Symthaea, Mycelix, Xenia, central SaaS or distributed ledger.
7. Reuse mature protocols for APIs, identity, telemetry, security events, domain commands, topology, controls and update trust.

## Non-goals

- Defining a full PSA, ITSM, RMM, EDR, backup, accounting, procurement or licensing product.
- Standardizing provider pricing, service-level remedies, labor obligations or certification.
- Introducing a mandatory identity provider, PKI, signed-token format or cryptographic algorithm in the initial proposal.
- Making AI output, trust scores, ledger consensus or reputation a substitute for authorization.
- Claiming schema validation provides production security or that a receipt independently proves truth.

## Proposed core records

### Operation manifest

An immutable request with stable operation ID, tenant ID, target reference, action profile and verb, argument digest, preconditions, requested expiry, recovery reference, requester identity and idempotency key. The authorized subject is the canonical digest of the complete immutable manifest, not its human-readable name.

### Authority grant

An approval by a trusted issuer over the exact manifest digest, tenant/resource, authorized executor, audience, policy version, approval references, nonce and bounded validity interval. Implementations must verify issuer trust, principal identity, validity and revocation immediately before mutation. A grant is not itself execution.

### Lifecycle event

A uniquely identified, sequenced event correlated to an operation, with timestamp, actor and explicit transition. Replays, gaps, duplicates and illegal transitions must be detectable. A cancellation request is not evidence that the operation stopped.

### Evidence receipt

A structured claim about an operation's outcome, bound to the manifest and target, with executor identity, observed post-state, verification method and time, evidence digests, verifier identity and limitations. A verified outcome requires profile-defined postcondition evidence. Where independence is required, policy must establish distinct principals and sufficiently independent control/failure domains; two different name strings do not establish independence.

### Capability document

A statement of supported OMSI versions, profiles, actions, limits and optional extensions. An implementation must advertise its tested scope and must not imply universal MSP compatibility from passing the core tests.

## Lifecycle semantics

The normal path is:

**proposed → validated → authorized → accepted → running → applied → verified**

Failure, rejection, expiry, cancellation and indeterminate outcomes are typed states or terminal outcomes governed by profile rules. **Queued is not running; running is not applied; applied is not verified.** When connectivity is lost after acceptance, the client must reconcile target state before retrying. A non-idempotent action must not be blindly repeated.

## Security requirements

- The URL tenant, body tenant, target tenant and authenticated principal's permitted tenant must agree. Client-supplied tenant ID is never sufficient evidence of authorization.
- Authorization MUST be checked against authenticated principal, tenant, resource, exact action, manifest/argument digest, audience, expiry, policy and approval requirements.
- Issuer trust, key status and revocation MUST be rechecked at execution time.
- A change to any authorized manifest field creates a new subject and requires fresh authorization.
- The executor must apply the exact authorized plan; it must not reinterpret it into another command or substitute another target.
- Retry after uncertain delivery MUST reconcile state and honor idempotency semantics.
- Secret values MUST NOT appear in operation manifests, general events, error messages or portable receipts. Use a separate secret-delivery mechanism and reference.
- A verified receipt MUST contain profile-defined postcondition observations and verification time.
- Event/receipt storage and retention controls must be defined by deployment and customer policy; a digest alone is not a complete audit system.

## HTTP binding (proposed)

An initial API binding may provide separate endpoints for capability discovery, operation proposal, validation evidence, grants, execution requests, cancellation requests, lifecycle events and receipts. It must make it impossible to interpret proposal submission as authorization or execution. Requests require authenticated principal context, tenant scoping, idempotency behavior, structured errors and bounded pagination where applicable.

The HTTP binding is an implementation profile, not the entirety of OMSI. Event delivery may use polling or a separately standardized subscription mechanism without changing core record semantics.

## Prior art and relationships

- **TM Forum TMF724 Incident Management API:** standardized incident records and lifecycle events already exist; OMSI should not create a competing full ticket model without evidence of a specific gap. https://www.tmforum.org/open-digital-architecture/open-apis/incident-management-api-TMF724/v4.0 and https://github.com/tmforum-apis/TMF724_IncidentManagement.
- **SCIM (RFC 7644):** identity provisioning; it does not prescribe a universal tenant registration/mapping scheme. https://www.rfc-editor.org/rfc/rfc7644/.
- **OAuth Resource Indicators (RFC 8707):** use audience/resource restriction where OAuth is selected. https://www.rfc-editor.org/rfc/rfc8707.html.
- **OpenTelemetry:** reuse telemetry and trace context semantics. https://opentelemetry.io/docs/specs/otel/.
- **OpenC2 Language 1.0:** reuse suitable cybersecurity command semantics; it does not replace general service lifecycle or outcome evidence. https://www.oasis-open.org/standard/oc2-ls-v1-0/.
- **TOSCA 2.0:** reuse topology/orchestration concepts where they fit. https://docs.oasis-open.org/tosca/TOSCA/v2.0/TOSCA-v2.0.html.
- **OSCAL:** map controls, implementation and assessment information rather than create a competing control ontology. https://pages.nist.gov/OSCAL/.
- **RFC 8785 JCS:** candidate canonical JSON building block; cross-language vectors and independent review are required before freezing a profile. https://www.rfc-editor.org/rfc/rfc8785/.
- **CISA JCDC RMM Cyber Defense Plan:** supports the elevated emphasis on tenant isolation and control-plane blast-radius reduction.
- **MSPAlliance UCS 4.0:** an operational assurance/certification framework; OMSI may supply portable technical records to support assurance but must not claim to replace certification. https://mspalliance.com/mspalliance-launches-ucs-4-0-establishing-ai-governance-requirements-for-managed-service-providers/.

This is a partial desk review, not proof of novelty, completeness or trademark availability. Pin exact artifact versions and licenses when a profile depends on them.

## Alternatives considered

1. A Luminous-only API: faster initially but would produce a product integration contract, not an open standard.
2. Standardize every MSP domain at once: rejected for the first version; it would grow too broad before core operation/evidence semantics are tested.
3. Replace existing standards: rejected because it duplicates mature work and raises adoption costs.
4. Require blockchain or AI: rejected; neither is necessary for interoperability.
5. Transport-independent model plus an accessible HTTP binding: proposed; it lets conventional tools participate without making REST the only possible transport.

## Drawbacks and risks

OMSI can impose integration work, over-constrain vendor models, or fragment into incompatible profiles. Well-formed evidence may be based on false observations. Management control planes remain high-value targets. Canonicalization and signature mismatches can break authorization. Mitigation requires narrow scope, mappings to existing standards, independent verification, negative tests, external security review and capability-specific claims instead of generic compatibility badges.

## Open questions

- Is this the right interoperability gap to standardize, and who experiences it most acutely?
- What minimal cross-provider use case should be mandatory in Core 0.1?
- Should canonicalization adopt unmodified RFC 8785 or a constrained profile, and how are malformed JSON and duplicate keys handled before canonicalization?
- What signature and identity envelope should be profiled after independent cryptographic review?
- How should tenant identifiers and delegated provider identities be namespaced without requiring a single identity authority?
- What verifier independence is practical and testable?
- Which governance model, license and naming policy will make unaffiliated implementations credible?

## Acceptance gates before a stable release

1. Two independently developed implementations pass the same golden fixtures.
2. Cross-language canonicalization and cryptographic vectors cover Unicode, numeric, malformed-input and duplicate-key edge cases.
3. A disposable VM demonstrates validation, exact authorization, execution, post-state verification and recovery.
4. Cross-tenant tests exercise authenticated principal-to-tenant mapping and resource authorization, not just a payload field.
5. Fault injection covers timeout-after-acceptance, lost results, retry, partial application, cancellation races, revoked delegation and stale state.
6. A customer exports resources, cases and evidence to a second implementation without dependence on the original UI/database.
7. External security review, naming clearance and an open governance process are completed.

Until these gates pass, OMSI remains a pre-RFC proposal.
