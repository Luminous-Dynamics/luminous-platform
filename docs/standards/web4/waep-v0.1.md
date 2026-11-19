# Luminous Web4 Assurance Evidence Profile (WAEP) v0.1

**Status:** draft, non-normative research/reporting profile  
**Issue:** [Platform #495](https://github.com/Luminous-Dynamics/luminous-platform/issues/495)  
**Date:** 2026-11-19  
**Claim ceiling:** WAEP defines how to report and compare security claims. It does not implement controls, certify deployments, or establish Web4 conformance.

## 1. Purpose

WAEP makes each security claim inspectable across the complete path required to justify it:

`claim → obligations → enforcement → observation → protected effect → residual assumptions`

The unit of analysis is one claim, not a product, repository, policy engine, paper, model, or aggregate system score.

WAEP is deliberately a reporting and evaluation contract. It does **not** create a new authorization engine, identity registry, token format, trust root, workflow runner, participant ledger, provenance graph, or certification scheme.

The machine-readable draft is [waep-v0.1.schema.json](waep-v0.1.schema.json). [waep-example.v0.1.json](waep-example.v0.1.json) is a synthetic, design-only example with no executed tests or achieved security claim.

## 2. Research basis and limits

A structured review published on 10 November 2026 screened 75 sources into a 30-source analytical corpus and 92 Source–Claim Records. In that bounded corpus, it found no evaluable revocation-assurance claim among 21 agent-focused sources, no delegation-containment/attenuation claim in the 30-source corpus, and no trajectory-wide effective-privilege measurement among the three evaluable least-privilege sources. It also found that terms such as mediation, auditability and final-state correctness often referred to different enforcement and observation boundaries.

These are **corpus-bounded evidence-coverage findings**. They are not proof that deployed products lack controls and are not estimates of the whole field. The paper's Minimum Authorization-Assurance Profile (MAAP) is a preliminary disclosure framework, not a validated score, certification instrument or universal standard. WAEP adopts the claim-to-evidence discipline; it does not claim to invent the underlying access-control principles or to validate MAAP.

Sources:
- Elmasry, “Authorization assurance for tool-using LLM agents: a structured review of enforcement semantics and effect-level security evidence,” *International Journal of Information Security*, 10 November 2026: https://doi.org/10.1007/s10207-026-01343-w
- The Agent Identity Standards Map 2026, November update: https://startwithidentity.com/reports/agent-identity-standards-map-2026/

The standards map identifies agent lifecycle accountability (owner, purpose and deprovisioning) as a remaining standards gap in its 30 October 2026 snapshot. It also notes that current revocation event standards are generic rather than agent-specific. This is a useful research lead, not permission to create a parallel identity system.

## 3. The claim model

A WAEP claim record answers six questions:

1. **What exactly is claimed?** State a property, principal, agent/workload, tenant, resource, operation, purpose, target, effect class and validity scope.
2. **What must be true for the claim to hold?** List obligations and their trigger conditions.
3. **Where is the property enforced?** Name the policy decision point (PDP), policy enforcement point (PEP), broker, participant effect boundary and any independent observer/verifier. Identify alternate paths and route-inventory limits.
4. **What was actually observed?** Bind evidence to exact artifacts, source identity, time, freshness, control domain, lineage and the stage observed.
5. **What protected effect occurred?** Distinguish proposed, authorized, dispatched, participant-accepted, committed, rejected, unknown, observed and reconciled.
6. **What remains assumed or uncovered?** State the trusted computing base, residual risks, excluded threats, unsupported paths and claim ceiling.

A signed request proves that a key signed bytes under a stated profile. It does not by itself prove principal legitimacy, current authority, complete mediation, successful execution, source independence or a correct physical result.

A policy decision does not prove that every route to the resource crossed the PEP. A control-plane trace does not prove a participant committed the effect. An expected final state does not prove that the sequence leading to it was authorized. Model narration is not participant-effect evidence.

## 4. Independent status axes

WAEP deliberately separates evidence maturity from claim disposition.

### Evidence maturity

These labels describe the strongest evidence present; they are not a score and must not be inferred from a neighboring project's status.

| Label | Meaning |
|---|---|
| `specified` | A versioned requirement or design exists |
| `structurally_checked` | Static/schema/shape checks executed |
| `source_test_authored` | Tests exist in source but are not necessarily executed |
| `exact_head_test_executed` | Tests ran against the exact named immutable source head |
| `runtime_boundary_exercised` | The declared implementation boundary was exercised |
| `participant_effect_observed` | Participant-owned effect evidence was independently observed |
| `independently_reviewed` | A reviewer independent of the implementer assessed the claim/evidence |
| `externally_reproduced` | An independent party reproduced the declared result |

Do not imply that these form a single universal assurance ladder. A claim may require some forms of evidence and not others. A CI pass cannot substitute for participant-effect observation when the claim concerns a real effect.

### Claim disposition

| Disposition | Meaning |
|---|---|
| `supported_within_scope` | Evidence supports the exact claim within its declared boundary and assumptions |
| `refuted` | The declared claim failed a required test or counterexample |
| `contradicted` | Evidence conflicts or the claim's own inputs are inconsistent |
| `indeterminate` | Missing, stale, ambiguous or incomplete evidence prevents a conclusion |
| `unsupported` | The implementation/profile does not provide the claimed property |
| `not_claimed` | No assurance assertion is being made |

A positive disposition must bind exact immutable source, policy, verifier, test, observer and evidence identities. Queued, skipped, missing, stale, unexecuted or superseded work is never positive evidence. The schema makes this more than a prose rule: a supported claim requires executed-run/evaluator commits, a nonzero denominator, a claim source commit, evidence artifacts and observations. For consequential, safety-critical or persistent effects, it additionally requires an independently owned participant oracle and an independent observation at participant acceptance, durable commit, physical effect, postcondition or reconciliation.

## 5. Property-to-obligation-to-evidence matrix

The matrix below is a minimum guide. A claim record should include only the obligations relevant to the property it actually asserts, while clearly recording conditional obligations and exclusions.

| Property | Required obligations | Enforcement / observation boundary | Essential negative controls |
|---|---|---|---|
| Identity binding | Exact principal, agent/workload and runtime-instance relationship | Identity owner and relying party; source-authenticated identity evidence | Wrong principal/runtime, stale binding, tenant substitution |
| Complete mediation | Enumerated effect routes; every route crosses the declared enforcement boundary | Actual PEP and protected participant; route inventory and bypass evidence | Direct API, alternate adapter, legacy route, background job, fallback path |
| Effective least privilege | Compare task-required, granted, available and exercised authority over the full trajectory | Every tool/effect entry and resource; include alternate credentials and delegated actors | Shared tool, argument widening, dynamic tool discovery, alternate credential, adjacent resource |
| Consent / purpose | Principal, data class/fields, recipient, purpose, expiry, withdrawal and policy version | Consent authority plus resource-side enforcement; observed data access/use | Purpose substitution, expiry, withdrawal, cached copy, delegated recipient |
| Currentness / revocation | Fresh authority/key/policy/consent state at use; declared partition/offline semantics | Every relevant PEP and final participant | Active session, cached token, disconnected participant, child agent, replayed old epoch |
| Delegation containment | Root/parent/child task and plan binding; scope attenuation; re-delegation rules | Delegation owner, broker and final participant | Scope expansion, actor substitution, sibling task, stale ancestor, provenance erasure |
| Information flow | Data labels/purpose survive reads, transforms, summaries, memory and handoffs | Source, transformation boundary and protected sink | Summarize-then-export, tool/child laundering, omitted lineage |
| Session composition | State-aware policy over committed, pending and unknown prior effects; atomic/linearizable admission where needed | Session admission and participant-owned state | Parallel overspend, individually valid forbidden composition, pending-effect bypass |
| One-use effect | Exact request/target/audience/attempt binding and atomic consumption at effect boundary | Participant-owned commit boundary | Replay, duplicate delivery, two-thread race, changed arguments |
| Reconciliation | Unknown outcomes remain unknown until participant evidence resolves the same logical action | Participant ledger and independent reconciler | Lost acknowledgement, crash after commit, blind retry, compensating action misreported as rollback |
| Independent observation | Observer identity, control domain, freshness, method and postcondition are explicit | Independent observer; source/lineage graph | Agent self-report, mirrored source, common host/clock/collector, stale observation |
| Lifecycle accountability | Owner, purpose, deployment identity, status and deprovisioning lineage are attributable | Owner-governed lifecycle records and downstream relying parties | Orphaned agent, owner transfer, suspended agent, retired identity, stale child runtime |

A row in this table is a test requirement, not a claim that the property is already implemented.

## 6. Authorization and effect oracles

For claims involving protected operations, evaluate two distinct questions with separately owned oracles.

**Authorization oracle:** Is this exact actor/principal, tenant, resource, operation, argument set, purpose, policy revision, time, consent state, delegation lineage and session state authorized?

**Participant/effect oracle:** Did the participant accept the exact attempt, commit an effect, reject it, observe no effect, or remain uncertain? What durable record or state observation supports that classification?

Keep the lifecycle states distinct:

`proposed → authorized → dispatched → participant_accepted → committed → independently_observed → reconciled`

A path may terminate earlier, be rejected, or remain `outcome_unknown`. Do not infer later stages from earlier ones. An unknown outcome must not authorize a new non-idempotent attempt until the prior logical action is reconciled.

The example file intentionally has no executed corpus, no participant-owned oracle and no measured metrics.

## 7. Benchmark protocol and reporting

Compare matched systems on identical task semantics, model/provider, tool inventory, prompts, policies, environment, resource budgets and observer/oracle rules. Pin exact versions and disclose unsupported/non-comparable cases.

Report each dimension separately:
- unauthorized dispatches and unauthorized committed effects;
- required/granted/available/exercised privilege by step and across the complete trajectory;
- route/adapter coverage and unexplained bypass paths;
- stale/revoked authority acceptance and revocation propagation latency;
- valid delegation completion, authority-expansion acceptance and false-denial rate;
- consent/purpose violations;
- unknown-outcome retries and unresolved effects;
- observer coverage, participant/effect evidence completeness and false receipt promotion;
- provenance/common-cause coverage;
- benign task completion, quality, false denials and human interventions;
- p50/p95/p99 decision and end-to-end latency, throughput, token/tool use, energy and cost.

Security failures are non-compensatory. Do not average an unauthorized committed effect into a utility score. Report per-property and per-attack-family results, positive/negative/indeterminate denominators, uncertainty, dependence groups, exclusions and missingness. A finite synthetic corpus does not provide a universal production-risk bound. Repeated trials sharing task templates, seeds, sessions, environments or infrastructure failure domains must be analysed under the existing dependence-aware plan (#218).

### Initial research gates (not achieved results)

- Zero unauthorized committed effects in each preregistered synthetic negative-control stratum.
- 100% rejection of frozen wrong-principal, stale/revoked, delegated-expansion, target/argument substitution and replay controls.
- 100% acceptance of deterministic eligible positive controls at the policy/effect oracle.
- Zero blind retries while a prior non-idempotent attempt is unknown.
- 100% of credited consequential effects bind the exact authorization inputs, participant receipt and required postcondition evidence.
- 100% of uncovered/unknown effect routes are disclosed; no complete-mediation claim while unexplained routes remain.
- Performance targets are set only after the safety and correctness gates are preserved; compare with matched published baselines, not unrelated headline numbers.

## 8. Integration boundaries

WAEP is a cross-project reporting profile that consumes existing owner contracts.

| Owner | Existing work WAEP should reference | WAEP must not create |
|---|---|---|
| Mycelix | AGENT #805 and AGENT-000…010; LISE #6216/#6226; CONSENT-LIFECYCLE #5240 | New principal IDs, grant semantics, consent authority, trust root or authority engine |
| Symthaea | INTX #4397, A2A #4407, skill lifecycle #11741 and composition #11557 | A parallel interaction/action state machine, agent authority or skill-effect authority |
| Luminous Platform | ITOPS #213/#218; delegation #284/#288; evidence #286/#287; composition #293/#294; evidence dependence #356; effect/finality #168/#206/#245; failure semantics #283/#310; sovereign-compute assurance-vector #370 | A second runner, effect ledger, common-cause graph, broker or host executor |
| Xenia | Existing canonical session, consent, authorization and revocation contracts | A duplicate session/identity authority |
| Nixward / participant | Host transaction journal, effect boundary and recovery owner | A parallel host transaction or recovery journal |
| Symtropy / local controller | Versioned model, physical observations, local safety controller and actuation boundary | Model output promoted into physical measurement or authorization |
| Sol-Atlas | Source-linked, rights-aware knowledge and provenance | Central truth authority or a new generic evidence graph |

The Mycelix AGENT-010 gate remains in force for authoritative A2A/MCP/OAuth/AP2 adapters. WAEP does not waive any upstream dependency. Protocol TCK results are translation evidence, not identity, authority or effect qualification.

## 9. Standards and research crosswalk

- **MAAP:** Elmasry's preliminary Minimum Authorization-Assurance Profile informs claim disclosure. WAEP is not MAAP certification or a validated replacement score.
- **NIST RATS (RFC 9334):** use Attester / Verifier / Relying Party separation; attestation does not automatically authorize an operation.
- **SCITT (RFC 9943):** transparency receipts support verifiable statement registration, not truth or completeness.
- **OAuth:** use established audience/resource binding, token exchange and rich authorization requests (RFC 8707, RFC 9728, RFC 8693, RFC 9396) where applicable; draft extensions remain drafts.
- **OpenID SSF / CAEP / RISC:** useful generic security-event and revocation signals; do not imply agent-specific event semantics that the profile does not define.
- **AuthZEN Authorization API 1.0:** a standardized authorization decision API; it is not a substitute for an effect-bound participant check.
- **A2A v1.0.1 / MCP 2026-07-28:** task/tool interoperability; discovery and protocol authentication do not create Luminous authority.
- **OWASP ACS v0.1.4:** portable runtime hooks; profile declarations and Guardian dispositions do not prove strict policy, non-bypassability or actual effect enforcement.

## 10. Validation and claim ceiling

For this first draft:
- Schema and example were authored and locally validated against Draft 2020-12 in the authoring environment.
- The example is synthetic and explicitly `not_executed`.
- No Rust implementation, live agent, participant, broker, observer, runtime campaign, independent review, CI workflow, cross-implementation test or production deployment is claimed.
- The schema is a reporting contract. It cannot authenticate evidence or prove that a claimed boundary is complete or independent.
- A WAEP record can make an unsupported claim visible; it does not make the claim true.

Promotion beyond a draft reporting profile requires independent review, exact-head automated checks, adversarial schema/semantic mutation tests, and at least two independently implemented producers/verifiers that agree on the declared profile.