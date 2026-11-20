# Web4 Assurance-Prerequisite Admission Contract (APAC) v0.1

**Status:** draft composition contract and synthetic conformance fixtures  
**Tracking:** [WEB4-ASSURANCE-ADMISSION-001 #509](https://github.com/Luminous-Dynamics/luminous-platform/issues/509)  
**Composes:** AUTH-CORE-001 #501, WAEP #495, WALP #507, XAP #497, Mycelix SOV-AI-AUTH-016/#4753 and -017/#4779, LISE invalidation #6219  
**Date:** 2026-11-20

## 1. Purpose

APAC specifies how a Web4 participant consumes claim-specific assurance prerequisites at the final effect boundary without turning evidence into authority or creating another currentness service.

The governing distinction:

**a claim current at preflight is not necessarily current at commit**

A consumer policy may require exact security/qualification claims before an effect is admitted. The participant must bind those claims, their exact subject and lifecycle/applicability results, and the policy's requirements to the same action attempt that carries the canonical Mycelix authority decision.

## 2. Authority and assurance remain independent

Two predicates must be independently established:

1. **Effect authority:** Mycelix canonical grant/permit authorizes the exact principal, actor, participant, target, operation, argument digest, purpose, context, validity, policy epoch and attempt at commit.
2. **Assurance prerequisites:** every claim required by the consumer policy remains applicable and current for the exact subject/property at commit, under the required dependency-coverage and currentness strength.

A current assurance claim is not permission to act. A valid effect capability does not prove that required assurance claims remain current. The participant joins both predicates only at its final admission boundary.

APAC does not mint a grant, add a capability type, choose policy, or elevate a claim into authority.

## 3. Finite prerequisite set

A consumer policy names a finite set of claim prerequisites. Each item binds:
- WAEP profile/schema and claim identity;
- claim property and exact target subject manifest;
- exact qualification result reference/digest;
- exact ASSURE-004 lifecycle result reference/digest;
- exact Mycelix applicability result and dependency snapshot;
- required lifecycle/currentness and acceptable applicability;
- explicit failure behavior.

The prerequisite set has a canonical digest and is bound to the action request and attempt. An operation with no assurance prerequisites must carry an explicit empty set plus a policy that permits it. Missing prerequisite information is not equivalent to an empty set.

WALP supplies the lifecycle/applicability projection. APAC does not recalculate it. Mycelix owns canonical authority and commit-time state comparison. The participant performs the final check.

## 4. Preflight and commit

At preflight, bind the exact request, capability/authority snapshot, consumer policy revision, prerequisite-set digest and assurance-result snapshot.

At the effect linearization point, the participant must either:
- atomically compare the bound authority and assurance snapshots to the qualified current state; or
- obtain a fresh owner-qualified evaluation and bind that result into the commit receipt.

If a required claim is invalidated, expired, withdrawn, superseded, inapplicable or below the required currentness class before commit, the participant denies or holds according to policy. Partial/unknown dependency coverage cannot satisfy a completeness requirement.

If an invalidation occurs **after** a valid commit, the historical commit remains valid under its recorded commit-time context. The later event changes future admissibility; it does not rewrite history.

If the participant response is lost after dispatch, the attempt remains outcome_unknown and must be reconciled under its original attempt identity. No blind retry.

## 5. Selective impact and sovereignty

APAC evaluates only the finite claims required by the specific operation. A changed claim blocks operations whose prerequisite sets depend on its affected property; it does not globally revoke unrelated capabilities or invalidate unaffected claims. Targeted requalification can restore a prerequisite only with a new exact result and a consumer-acceptable applicability/currentness relation.

An offline participant can continue only operations explicitly permitted by its local policy and available qualified evidence. It must not fabricate global currentness when it cannot refresh a required snapshot.

## 6. Synthetic contract artifacts

- assurance-prerequisite-admission.schema.v0.1.json: Draft 2020-12 schema for prerequisite-set identity, exact request binding, preflight/commit snapshots and effect outcomes.
- assurance-prerequisite-admission.fixtures.v0.1.json: 33 synthetic scenarios (9 positive, 19 negative, 5 indeterminate), including explicit-empty-set controls, stale/withdrawn/expired/superseded claims, partial dependency inventory, property-specific non-impact, conditional transfer, post-commit invalidation, claim-before-commit races, authority snapshot revalidation, policy/prerequisite-set substitution, partition/unknown-outcome and reconciliation.
- scripts/validate_assurance_prerequisite_admission.py: test-only schema and deterministic reference evaluator.
- .github/workflows/assurance-prerequisite-admission.yml: read-only, path-scoped conformance workflow.

The evaluator is a synthetic reference oracle only. It does not invoke Mycelix, WALP, a broker or a participant.


The request, preflight and commit records separately bind the exact request digest, authority grant reference, consumer policy digest, prerequisite-set digest and authority/assurance snapshot digests. A snapshot-match boolean must agree with the digest comparison. A changed snapshot is accepted only through the corresponding owner-qualified revalidation result; an indeterminate revalidation holds the effect.

The suite also distinguishes:
- policy revision substitution from authority revocation;
- prerequisite-set substitution from claim invalidation;
- assurance snapshot mismatch with successful revalidation from failed/indeterminate revalidation;
- authority snapshot mismatch with successful revalidation from failed/indeterminate revalidation;
- claim invalidation before commit from invalidation after a valid commit.

These are synthetic binding semantics, not cryptographic hash verification or a live linearizability proof.

## 7. Metrics and proposed gates (not results)

Report independent metrics:
- stale assurance prerequisite acceptance at participant commit;
- stale/unknown authority acceptance at commit;
- unauthorized participant commits;
- false block rate for still-current/applicable claims;
- over-blocking of operations unrelated to invalidated claims;
- exact preflight-to-commit snapshot match rate;
- commit-time compare/revalidation success and latency;
- unknown-outcome reconciliation and duplicate-attempt counts;
- local safe-operation availability during federation loss.

Proposed finite-corpus gates:
- zero commits with known invalidated/expired/withdrawn/superseded/inapplicable or insufficiently current required claims;
- zero commits with partial/unknown dependency coverage when completeness is required;
- zero stale authority/assurance snapshots accepted across the preflight-to-commit race;
- 100% expected deterministic outcomes and 100% eligible positive-control acceptance;
- zero false global blocking of operations whose required claim set is unaffected;
- zero retroactive rewriting of an effect validly committed before a later invalidation;
- zero new attempt while the previous effect is outcome_unknown.

These are proposed conformance targets, not measured results or a universal security guarantee. There is no aggregate score.

## 8. Ownership and relationship to existing work

| Responsibility | Canonical owner |
|---|---|
| Claim disposition/evidence maturity and reporting | Platform WAEP #495 |
| Claim applicability/lifecycle projection | Platform WALP #507, consuming owner-native results |
| Grant, actor, resource, operation, purpose, context and delegation | Mycelix AGENT |
| Stale decision-to-effect and commit-time revalidation | Mycelix #4753 / #4779 |
| Invalidation event and dependency-impact benchmark | Mycelix #6219 |
| Final admission and effect/idempotency | Participant |
| Independent effect/postcondition observation | Qualified observer |

The formal race model and canonical atomicity semantics remain in Mycelix #4753/#4779. APAC defines only the cross-project prerequisite-set binding and conformance surface.

## 9. Claim ceiling

The v0.1 suite is synthetic. It does not establish integration with Mycelix commit-time checks or WALP, live participant enforcement, complete dependency discovery, atomicity on any real effect backend, or physical/OT safety. CI and independent implementation/review are required before a structural conformance claim.


### Digest limitation

The fixture values use synthetic fixed-shape digest strings so schemas and binding invariants can be tested deterministically. v0.1 does **not** compute or verify canonical set/request/claim hashes, signature validity, transparency receipts or cryptographic attestations. A runtime adapter must consume the canonical digest/serialization commitments from its owner-native WAEP/WALP/Mycelix/ASSURE interfaces; it must not treat the fixture's sample digest bytes as proof.


## 10. Fresh revalidation receipt requirement

A changed assurance snapshot cannot be accepted on a bare \`revalidation_state = passed\` flag. The commit record must carry an owner-qualified revalidation receipt bound to the exact request digest, prerequisite-set digest and commit-time assurance snapshot digest. A successful receipt must include one fresh claim evaluation for every required prerequisite, with the same claim ID, property and subject scope. The evaluator uses these fresh commit-time claim results—not the preflight claim records.

If a qualification result changes, the new result must identify the exact predecessor reference and digest and use a distinct successor identity/digest. A successful revalidation that reports a claim as invalidated still denies the operation. An omitted, duplicated, substituted or scope-shifted claim makes revalidation invalid. The equivalent authority revalidation receipt binds the exact request, grant and authority snapshot, and records the authority state at commit.

This is the difference between *asserting that revalidation passed* and supplying evidence that the exact prerequisites were re-evaluated at the effect boundary.


## 11. Independent typed implementation

\`tools/assurance-prerequisite-admission-v0-1\` contains a dependency-free Rust reference evaluator with typed authority, assurance, lifecycle, applicability, currentness, dependency and revalidation states. Its tests independently exercise exact receipt binding, fresh claim-set correspondence, predecessor-linked qualification replacement, invalidation, unknown outcomes and commit-time authority composition.

The Python checker evaluates the 33 JSON fixtures and JSON Schema; the Rust crate tests the normalized semantic contract through a separate typed implementation. The Rust crate intentionally does not parse the JSON fixture corpus, verify signatures/digests, execute participant effects or establish distributed atomicity. CI runs both implementations on the exact PR head. Until that exact-head run completes successfully, both remain authored checks, not passed evidence.
