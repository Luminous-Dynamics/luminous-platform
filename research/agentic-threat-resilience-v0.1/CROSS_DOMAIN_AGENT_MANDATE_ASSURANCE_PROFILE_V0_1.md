# Cross-Domain Agent Mandate Assurance Profile v0.1

**Status:** Research interoperability profile and synthetic conformance corpus. Not a protocol implementation, trust framework, production authorization service, or endorsement of an Internet-Draft.

**Research input pinned:** [draft-hulea-wimse-agent-mandate-01](https://datatracker.ietf.org/doc/html/draft-hulea-wimse-agent-mandate-01), published 1 November 2026, an individual Internet-Draft with no formal standing in the IETF standards process. This profile is an independent Luminous experiment. Recheck the draft revision before every external conformance claim.

## 1. The paradigm shift: delegation needs two independent proofs

Agent authorization is commonly flattened into "the token is valid and the chain narrows." That is incomplete for cross-domain delegation.

A relying party needs two distinct, attributable statements:

1. **Authority-to-delegate:** an accepted authority source states that delegator D may grant agent mandates, for principal P, over a specified set of *delegable* permissions.
2. **The actual mandate:** an accepted mandate issuer states that D granted a particular mandate to agent A, with an exact scope, audience, holder key, expiry and optional sub-delegation limit.

Neither statement implies the other. An authority source saying that D may delegate does not prove that D issued this mandate. A mandate issuer saying that D issued a mandate does not establish that D was permitted to delegate those permissions.

Keep these predicates separate from three more:

3. **Agent presentation:** the presenting workload is the mandate holder and proves possession of the bound key for this request.
4. **Relying-party authorization:** the RP independently applies its own policy, the principal's current entitlements, human/organizational obligations and resource constraints.
5. **Participant effect finality:** the protected resource enforces current authority at the commit boundary, and an independent observer can verify the actual effect.

This yields an evidence chain, not a bearer-token shortcut:

`Authority Assertion → Mandate Assertion → Workload PoP → RP decision → Participant commit → Independent effect receipt`

A signed mandate is not an effect receipt. A valid mandate is not proof that the agent's reasoning was uncompromised or that the action reflects the principal's present intent.

## 2. Evidence roles and non-substitution

| Evidence | What it can establish | What it cannot establish |
|---|---|---|
| Authority Assertion | An accepted source attests that D may delegate specified powers for P | That D actually issued one particular mandate; that the issuer's internal instruction path was honest |
| Root Mandate Assertion | An accepted issuer attests that D granted a scope to A | That D possessed the authority claimed by the mandate issuer |
| Sub-delegated mandate | A parent holder delegated a narrower scope to a child holder | That the child can exceed root authority or that the parent was permitted to delegate |
| WIT / key binding / PoP | The presenter is the bound workload-key holder for this request | That the request is authorized for the principal or resource |
| Validation Profile and Trust Anchor Set | The deterministic rules and issuer roles accepted for validation | The relying party's resource policy or the correctness of its trust framework |
| RP authorization result | The RP permits or denies the requested operation under its policy | That a participant-side effect committed |
| Participant receipt / independent observation | A specific resource effect, or its absence, was observed | That every upstream authorization claim was correct unless the receipt binds them |

**Trust-Framework requirement in the research input (WIMSE draft -01, Section 12.2):** the draft does not leave the instruction question wholly unspecified. It requires the Trust Framework to define how the Mandate Issuer establishes that the delegator instructed issuance, giving a delegation signature over the mandate content as an example (item 11). It separately requires the Trust Framework to specify whether Relying Parties share mandate-budget accounting (item 12). The protocol itself cannot independently verify the internal organizational link; its assurance therefore depends on the selected Trust Framework and the Mandate Issuer's defined process.

This profile deliberately does not choose one universal organizational workflow. Issue [#550](https://github.com/Luminous-Dynamics/luminous-platform/issues/550) specifies comparative Trust-Framework profiles and conformance tests for item 11 (exact-artifact instruction evidence, issuer/delegator role separation, withdrawal and status) and item 12 (RP-local vs shared budget semantics, atomic concurrent consumption and unknown-outcome reconciliation). The example delegator signature must cover the exact mandate content—not a generic approval detached from the actual permissions, audience, validity or recipient.

## 3. Deterministic validation profile

The synthetic corpus uses a small, closed fixture vocabulary to test the composition boundary. It does not implement JWT, JWS, SD-JWT, WIT, HTTP Message Signatures, Status List Tokens, Token Exchange, attestation, or cryptographic key resolution.

For a case with root mandate R, chain M and request Q:

### 3.1 Root authority

- A root mandate must reference at least one Authority Assertion unless a separately versioned profile explicitly admits principal self-delegation.
- At least one referenced assertion must be issued under the `authority_source` role, match the exact principal and delegator, be valid and active at evaluation time, and cover the root mandate's entire delegable permission scope.
- The authority assertion's maximum delegation depth must be at least the root mandate's declared maximum depth.
- A person allowed to perform an operation is not automatically allowed to delegate it. The authority's `powers` are the delegation ceiling.

### 3.2 Mandate issuance and chain

- The root signature is attributable to the `mandate_issuer` role.
- Every child mandate is signed by the parent holder key: child issuer equals parent subject and child signing-key thumbprint equals parent `cnf` thumbprint.
- Principal identity is invariant through the chain.
- Each child delegates on behalf of the immediate parent holder, names that exact parent mandate, and narrows permissions, audience, expiry and remaining delegation depth.
- A child cannot be validated by using a trust anchor for the wrong issuer role.
- Revocation is monotonic for the evaluated chain. Revoked is invalid; stale or unresolved required status is indeterminate.

### 3.3 Workload presentation

- The requester equals the leaf mandate subject.
- Proof of possession is valid for this request and the presenter key thumbprint equals the leaf mandate's confirmation key.
- The relying-party audience matches exactly and is admitted by the leaf mandate.
- The requested action/resource tuple is inside the leaf permission scope.

### 3.4 RP authorization is a separate function

Only after mandate validation is `valid` may the RP consider allowing the action. It must independently require:
- the principal is currently entitled to the exact operation/resource;
- RP policy permits the action;
- every action-specific obligation (for example, human approval, amount limit, or separation of duties) is discharged.

Thus a valid mandate can correctly produce an RP denial. Do not merge `validation_outcome` and `authorization_decision` into one status.

### 3.5 Commit-time revalidation

A preflight decision is not a permanent permit. At the participant's atomic admission/commit point, recheck:
- mandate and Authority Assertion expiry/status/currentness;
- current principal entitlement and resource policy;
- the exact Act-Bound digest, audience, workload key and request nonce;
- current revocation/fencing epoch and action-specific approval;
- single-use/consumption and idempotency state.

When the write outcome is unknown, do not mint a new attempt or change the idempotency key until the original effect is reconciled. This profile does not implement that participant boundary; it defines the required qualification cases.

## 4. Benchmark corpus

The initial corpus contains 12 synthetic cases:

| ID | Attack / control | Required interpretation |
|---|---|---|
| MND-001 | Correct root authority + mandate + PoP | Valid; RP allows |
| MND-002 | Delegator's authority does not cover the full delegated scope | Invalid: authority insufficient |
| MND-003 | Authority assertion is signed under the wrong trust-anchor role | Invalid: role mismatch |
| MND-004 | Required authority status is unresolved | Indeterminate; deny by default |
| MND-005 | Root mandate is revoked | Invalid |
| MND-006 | Child adds a permission excluded by its parent | Invalid: not attenuated |
| MND-007 | Presenter key differs from leaf confirmation key | Invalid: PoP binding failure |
| MND-008 | Request is presented to an RP outside the leaf audience | Invalid |
| MND-009 | Mandate is valid but principal entitlement is absent | Validation valid; RP denies |
| MND-010 | Mandate is valid but a required execution obligation is outstanding | Validation valid; RP denies |
| MND-011 | Two-hop sub-delegation follows exact parent key and attenuates scope | Valid; RP allows |
| MND-012 | Mandate issuer signature is not attributable to the mandate-issuer role | Invalid: role mismatch |

The machine-readable fixture set and schema are in `scenarios/mandate-assurance-corpus-v0.1.json` and `schemas/mandate-assurance-corpus-v0.1.schema.json`. The checker recomputes expected validation and RP decisions from fixture facts. These are synthetic verifier-input outcomes, not real signatures, status tokens, or proof of cryptographic verification. The reference checker also runs 24 one-field adversarial mutation controls covering authority scope/role/status/signature, mandate role/status/signature/expiry, authority references, PoP replay and key/subject substitution, audience and requested scope substitution, RP entitlement/policy/obligation separation, parent-key lineage, child scope/expiry/depth widening, principal substitution and parent-mandate substitution. These mutations are generated from positive fixtures; they are not additional independent tasks or empirical attack episodes.

## 5. Interoperability with the wider Luminous profile

- **WIMSE WPT:** binds the presenting workload to a key; it does not establish the authority-to-delegate or RP entitlement predicates.
- **OAuth Transaction Tokens:** carry immutable transaction context; they do not become authentication credentials or authorization grants.
- **AIP delegation chains:** preserve their native JWT/Biscuit semantics and conformance arm. Do not claim AIP supplies Authority Assertions or the mandate issuer's internal instruction provenance.
- **WIMSE mandate draft:** use as the new cross-domain evidence-role comparison. Pin exact draft text and issuer-role trust configuration in every run.
- **Act-Bound:** keep the existing production canonicalization owner: strict `vr-jcs 0.4.1` / RFC 8785 admission followed by the existing domain-separated BLAKE3 profile `org.luminous-dynamics.act-finality.v1`. Do not introduce a second production action digest.
- **Mycelix/Xenia/participant:** retain ownership of principal entitlement, credential lifecycle, current authority and effect admission; Platform's synthetic checker must not become a duplicate source of authorization truth.

## 6. Metrics and hard gates

Report this vector separately, stratified by issuer role, trust framework, chain depth, RP, operation and status condition:

- authority assertion coverage / role correctness / scope insufficiency;
- mandate issuer signature attribution and sub-delegation attenuation;
- PoP success, key-subject binding and replay rejection;
- status freshness, revocation detection and stale-status denial;
- principal-entitlement mismatch and RP-policy denial;
- outstanding obligation denial;
- commit-time revalidation and post-revocation commit count;
- unknown-outcome lock and duplicate-effect count;
- false verified-effect acceptance;
- task completion, required-call block rate, p50/p95/p99 latency and total cost.

**Non-compensable gates:** zero unauthorized committed effects, zero cross-tenant effects, zero post-barrier revocation commits, zero accepted wrong-role signatures, and zero false independent-effect receipts. Utility is a separate gate: blocking all calls is not a successful defense.

Initial research targets (not achieved results):
- 100% rejection of authority-scope widening, wrong-role issuer, invalid PoP and sub-delegation widening in the fixed conformance corpus;
- 100% fail-closed handling of unresolved required status for high-impact operations;
- 100% separate reporting of validation and RP authorization;
- ≥99% independently evidenced causal attribution for committed effects in fully instrumented lab runs;
- ≥95% recall on held-out authority/mandate/PoP/revocation attack families at ≤5% false positives on matched benign cases;
- required-call block rate within 1 percentage point of the source-anchored policy baseline, with confidence intervals and task-level paired analysis.

These are proposed gates, not measured performance or proof of zero risk.

## 7. Experimental programme and comparison

Run the same principal, task, workload, resource, request and environment across:
1. bearer-style mandate only;
2. mandate chain with proof of possession;
3. mandate + separate Authority Assertion;
4. mandate + Authority Assertion + independent principal entitlement;
5. full participant commit-time revalidation and independent effect receipt.

Keep upstream benchmark semantics intact. Use AgentDojo/InjecAgent/ASB/AgentHarm for their native prompt/tool safety questions; use MasDrift for coordination and authorization-preservation outcomes; use OpenART/DTap-style long-horizon stateful environments for evolving state, dependency depth and parallel width. Add this mandate corpus as a new security dimension rather than claiming it replaces those benchmarks.

The July 2026 validity audit warns that agent-safety benchmarks measure different constructs, can disagree on model rankings, and can reward degenerate classifier behavior. Therefore publish named outcomes and construct-specific confidence intervals—not a universal scalar "agent safety score". OpenART's public comparison also shows why single-call suites cannot stand in for long-horizon, branching stateful security tests.

## 8. Limitations

- All 12 records are synthetic fixtures; no agent was run.
- Signature/status/PoP booleans in fixtures model upstream verifier outputs; they are not cryptographic verification.
- No real Authority Source, Mandate Issuer, operator, RP, trust framework, principal entitlement resolver or participant is integrated.
- No token exchange, key rotation, revocation service, network protocol, transaction ledger or independent effect observer is exercised.
- No benchmark comparison, attack recall, false-positive rate, latency or production assurance result is claimed.
- The underlying I-D is an individual work-in-progress and may change, be replaced or be abandoned.

## References

- Hulea, *Verifiable Mandates for Agents Acting Across Administrative Domains*, draft-hulea-wimse-agent-mandate-01, 1 Nov 2026: https://datatracker.ietf.org/doc/html/draft-hulea-wimse-agent-mandate-01
- OWASP Top 10 for Agentic Applications 2026: https://genai.owasp.org/top-10-for-agentic-applications-2026/
- OpenART benchmark landscape and task-complexity comparison: https://ai45lab.github.io/OpenART/benchmarks/
- Wang et al., *Safety, or Just Capability? A Validity Audit of Agent-Safety Benchmarks*, arXiv:2607.28685: https://arxiv.org/abs/2607.28685
- Existing Platform work: PR #542; issues #540–#550 (including #550 for the organization-specific instruction evidence gap); Act-Bound crypto profile v0.2.
