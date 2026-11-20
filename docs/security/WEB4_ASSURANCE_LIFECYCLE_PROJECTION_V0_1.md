# Web4 Assurance Lifecycle Projection (WALP) v0.1

**Status:** draft interoperability projection and synthetic conformance fixtures  
**Tracking:** [Platform #507](https://github.com/Luminous-Dynamics/luminous-platform/issues/507)  
**Canonical lifecycle owner:** [Symthaea ASSURE-004 #2618](https://github.com/Luminous-Dynamics/symthaea/issues/2618)  
**Evidence applicability owner:** [Mycelix AMSAP-005B #1087](https://github.com/Luminous-Dynamics/mycelix/issues/1087)  
**Date:** 2026-11-20

## 1. Purpose

WALP connects a Web4 Assurance Evidence Profile (WAEP) claim to canonical qualification lifecycle and evidence-applicability results. Its purpose is to prevent a historically supported claim from silently being treated as current after material subject, campaign, policy, provenance, route, hardware, observer or evidence changes.

A claim supported for exact subject S under campaign P at view V is not automatically current for changed subject S-prime, campaign P-prime or view V-prime.

WALP is a projection/interoperability contract. It is not a currentness engine, event store, evidence lease, dependency graph, time authority, policy engine, certification scheme or deployment controller.

## 2. Canonical ownership

| Concern | Canonical owner | WALP responsibility |
|---|---|---|
| Claim and evidence qualification | Symthaea ASSURE-000 | Reference exact claim/result identity |
| Exact subject identity | Symthaea ASSURE-001 | Bind subject manifest and material surfaces |
| Preregistered campaign semantics / registration view | Symthaea ASSURE-002 | Bind full campaign/view identity, not just a short label |
| Non-scalar evidence resolution | Symthaea ASSURE-003 | Reference exact resolver/result |
| Append-only invalidation, expiry, withdrawal, supersession and requalification | Symthaea ASSURE-004 | Project lifecycle outcome; do not reimplement resolver |
| Assurance capsule | Symthaea ASSURE-005 | Reference verifiable capsule where available |
| Evidence applicability / targeted revalidation | Mycelix AMSAP-005B | Consume applicability relation and evidence |
| Evidence reuse horizon | Mycelix EvidenceLease #181 | Reference lease output; do not create another lease algebra |
| Dependency impact / common-cause lineage | Mycelix | Consume qualified impact/dependency evidence |
| Claim-to-effect reporting | Platform WAEP | Add orthogonal lifecycle/applicability projection |

The WAEP draft separates claim disposition from evidence maturity. WALP adds lifecycle currentness and evidence applicability as separate dimensions; it must not collapse them into one PASS or score.

## 3. Five independent dimensions

1. Historical claim disposition: supported within scope, refuted, contradicted, indeterminate, unsupported or not claimed.
2. Evidence maturity: specified, structurally checked, exact-head executed, runtime boundary exercised, participant effect observed, independently reviewed or externally reproduced.
3. Evidence applicability: exact subject, controlled equivalent, conditionally transferable, requires targeted revalidation, historical only, inapplicable or unknown.
4. Qualification lifecycle: current, expired, invalidated, withdrawn, superseded, requires requalification, indeterminate or forked.
5. Consumer handling: the consuming policy may permit evaluation, restrict use, require human review or block reliance on a non-current claim.

A historical claim can remain supported within scope while WALP says invalidated or superseded for current use. The predecessor and its evidence are never rewritten. A requalified successor is a new result and does not inherit support, replication, independence or deployment conclusions by assertion.

## 4. Identity-bearing inputs

A WALP record binds exact references/digests for:
- WAEP schema/profile and claim identity;
- claim property and declared scope;
- ASSURE-000 claim/result;
- ASSURE-001 subject manifest and material surfaces;
- ASSURE-002 full campaign semantics, shared semantic commitments, registration view and currentness/checkpoint class where relevant;
- ASSURE-003 resolver/result;
- Mycelix evidence-applicability and dependency-impact outputs;
- EvidenceLease result/horizon where applicable;
- ASSURE-004 predecessor, transition and successor lineage;
- evidence establishing the observed change and any authoritative ordering/currentness anchor;
- consumer policy's required applicability/currentness classes and handling of unsatisfied requirements.

Human-readable version strings, product names, repository names, aliases or producer-authored current=true are not identity or currentness evidence.

## 5. Lifecycle semantics

Preserve the distinction:

- **Expired:** a preregistered horizon no longer holds; this is not evidence that a material invalidation event occurred.
- **Invalidated:** evidence establishes that a declared material invalidation condition occurred.
- **Withdrawn:** an authorized withdrawal transition is evidenced.
- **Superseded:** a valid successor result is evidenced; the predecessor remains historical.
- **Requires requalification:** a material/applicability change means the old result cannot support the requested target claim until a new qualification is completed.
- **Indeterminate:** missing, stale, unknown or incomplete event/currentness evidence prevents a conclusion.
- **Forked:** conflicting successors mean no unique current result exists.
- **Current:** the exact result is current only within the admitted subject, campaign, evidence, registration-view and consumer requirement envelope.

A declared invalidation predicate is not proof the event happened. Same ASSURE-000 core-plan digest does not prove the richer ASSURE-002 campaign semantics are unchanged. A terminal in a supplied registration view is not equivalent to currentness through a verified completeness checkpoint. A valid signature or capsule is not a deployment grant.

## 6. Applicability and targeted requalification

Mycelix's evidence-applicability relation remains independent from ASSURE-004 lifecycle status. A changed subject may require targeted revalidation of one property while unaffected evidence remains historically valid for other properties. Retaining any conclusion requires an explicit, evidenced applicability relation; it is never inferred from the same product label or digest reuse.

Unknown dependency coverage is not proof of no impact. Where a relevant dependency edge or event source is missing, the result is indeterminate/requalification-required. Do not invalidate every unrelated claim automatically: dependency impact should be property-specific and based on the canonical graph/applicability result.

## 7. Synthetic conformance suite

The v0.1 fixture set covers:
- exact current claims and bounded terminal-in-view claims;
- preregistered controlled-equivalence bridges;
- targeted requalification after one property dependency changes;
- model/prompt/tool/campaign/firmware drift;
- expiry vs invalidation vs withdrawal vs supersession;
- checkpoint-currentness mismatch;
- discovered registration successors;
- duplicate transitions, forks, cycles and missing predecessors;
- unknown dependency/observer lineage;
- unrelated dependency change with a justified non-impact result;
- requalified successors that do not carry forward unsupported conclusions.

The schema and test-only validator check projection consistency and anti-gaming invariants. They are not a replacement for the ASSURE-004 resolver or Mycelix applicability implementation.

## 8. Proposed qualification measures (not results)

| Metric | Proposed target |
|---|---:|
| Expected lifecycle/applicability fixture outcomes | 100% |
| Silent carry-forward across material drift | 0 |
| Historical predecessor mutation | 0 |
| False unique-current result on forked/cyclic/incomplete lineage | 0 |
| Transitions without exact predecessor and event evidence | 0 |
| Requalified results without exact new subject/campaign/evidence root | 0 |
| Deployment-authority inference from support/signature/capsule | 0 |
| False invalidation of demonstrably unaffected claims | 0 |
| Independent projection verifier implementations | At least 2 |

No scalar assurance score. Report claim property, subject, applicability, lifecycle state, dependency coverage, requalification scope, human review burden and downstream policy handling separately.

## 9. Execution and claim ceiling

The v0.1 suite is synthetic. It does not invoke ASSURE-004, Mycelix's dependency/applicability outputs, WAEP production claims, deployment policy or live participants. It does not prove continuous monitoring, complete dependency discovery, automatic revocation or production security. CI on the exact PR head and independent review are required before the projection is considered structurally qualified.

## References

- Symthaea ASSURE-004: https://github.com/Luminous-Dynamics/symthaea/issues/2618
- Symthaea ASSURE-003: https://github.com/Luminous-Dynamics/symthaea/issues/2617
- Mycelix AMSAP-005B: https://github.com/Luminous-Dynamics/mycelix/issues/1087
- Mycelix EvidenceLease: https://github.com/Luminous-Dynamics/mycelix/issues/181
- Mycelix LISE-ASSURANCE-INVALIDATION: https://github.com/Luminous-Dynamics/mycelix/issues/6219
- WAEP v0.1: https://github.com/Luminous-Dynamics/luminous-platform/issues/495
- XAP: https://github.com/Luminous-Dynamics/luminous-platform/pull/498
