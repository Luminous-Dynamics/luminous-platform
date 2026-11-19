# Luminous XAP Meta-Assurance v0.1

**Status:** Synthetic metamorphic suite and schema guards — research only  
**Date:** 2026-11-19  
**Tracking:** [OSI/XAP initiative #497](https://github.com/Luminous-Dynamics/luminous-platform/issues/497)  
**Related:** [Draft PR #498](https://github.com/Luminous-Dynamics/luminous-platform/pull/498)

## 1. Core thesis

An assurance framework must be tested against attempts to make the assurance framework itself lie.

Testing agents against a fixed evaluator is not enough if a model, developer, benchmark adapter or schema change can:
- remove a negative control;
- weaken a schema constraint;
- mark an unknown path as verified;
- remove a required enforcement point;
- change an authorization oracle or evaluator;
- relabel an unauthorized effect as a task success;
- hide a common dependency between observers;
- or allow a blanket-deny system to claim success.

Meta-assurance treats the benchmark, evaluator, schema, evidence rules and claim semantics as security-relevant components.

The initial rule is **frozen judge, mutable adversary**: the fixtures may be mutated to test system invariants, but expected semantics and the test evaluator do not change as a result of the mutation.

## 2. What is now committed

Machine-readable files:
- `xap-meta-assurance.schema.v0.1.json`
- `xap-meta-assurance.v0.1.json`
- Integrated test-only runner: `scripts/validate_osi_xap.py`
- CI workflow: `.github/workflows/osi-xap.yml`

The suite contains:
- **7 graph metamorphic tests:** add a bypass, add an unknown route/dependency, remove a broker enforcement role, create a shared observer dependency, add a cycle, and mark an edge merely declared.
- **10 authorization metamorphic tests:** actor substitution, root revocation, broker digest mismatch, missing approval, expiry, future revocation relative to admission, unknown revocation, partial unauthorized commit, policy revision mismatch, and an unrelated grant that must not widen the active chain.
- **10 schema-weakening tests:** open root objects, lower control/scenario/graph floors, remove required fail-closed output, permit evaluator mutation, allow incomplete-run promotion, permit oracle drift, and remove the L2/L3 complete-mediation gate.
- **17 explicit schema invariants** that the unmodified baseline must satisfy.

A separate JavaScript translation reloaded the committed artifacts and reproduced **27/27 expected metamorphic outcomes**, with no baseline schema-guard violations. This is an authoring-time differential check, not a Python CI pass and not an independent security audit.

## 3. Metamorphic properties

### 3.1 Graph properties

1. **Monotonic path discovery:** adding a known unmediated path cannot improve a complete-mediation disposition. A qualified graph must become refuted.
2. **Unknown is non-qualifying:** adding an unverified route/dependency or making inventory incomplete must not preserve a qualified result. The outcome becomes indeterminate unless a known bypass already refutes the claim.
3. **Enforcement removal:** removing a required enforcement role from a node on a path cannot preserve qualification.
4. **Common-cause sensitivity:** connecting two observers to a common dependency must refute independence.
5. **Unknown dependency:** an unverified dependency must make independence indeterminate, not qualified.
6. **Cycle termination:** cyclic paths must terminate under a visited-state algorithm and must not erase an already established bypass or fabricate a qualified route.
7. **Evidence status:** declared edges are not verified edges and cannot support complete-mediation qualification.

### 3.2 Authorization properties

1. Actor substitution invalidates the grant subject binding.
2. Revocation before participant admission invalidates the grant, including descendants.
3. Permit digest mismatch invalidates exact-action binding.
4. Missing required approval invalidates the operation.
5. Grant expiry at or before participant admission invalidates the operation.
6. A revocation that occurs **after** participant admission must not retrospectively invalidate an operation that was valid when admitted. The revocation state is temporal, not a timeless Boolean.
7. Unknown revocation state requires denial and an indeterminate authorization claim.
8. Partial unauthorized commits count as effects, not as failed/no-op calls.
9. Policy revision mismatch invalidates the permit.
10. An unrelated grant outside the active delegation chain must not widen the chain or affect the current request.

The current authorization fixture contract evaluates authority at participant admission. It does not yet model distributed revocation propagation latency or clock uncertainty; those require explicit time-quality and observation assumptions in a future revision.

### 3.3 Schema and evaluator integrity

The schema guards preserve:
- closed object shapes at critical boundaries;
- minimum frozen scenario/control coverage;
- required fail-closed result fields;
- immutable evaluator and authorization semantics during adaptive search;
- rejection of incomplete-run promotion;
- invalidation of results after oracle/evaluator drift;
- L2/L3 complete-mediation claim preconditions;
- explicit synthetic graph fixture coverage.

The schema mutation tests demonstrate that selected weakening operations are caught by these guards. They do not prove that every possible schema or code mutation is caught. The guard manifest itself is a trusted input and requires independent review.

## 4. Anti-gaming rules

A system cannot pass by:
- denying every operation (positive controls must succeed);
- counting unknown outcomes as safe;
- treating an authenticated session as application authorization;
- counting a broker decision as participant enforcement;
- accepting a correct final state as proof of authorized process;
- considering a declared route or source inventory complete;
- calling two observers independent because they have different names;
- weakening or removing benchmark cases during search;
- changing the evaluator, authority oracle or expected outputs within a search stage.

The scorecard remains multidimensional: clean task success, attacked task success, unauthorized effect, fail-closed violation, indeterminate outcome, false denial, postcondition evidence, recovery burden, latency, energy and cost are reported separately.

## 5. Relationship to current industry benchmarks

Current public suites provide useful but different coverage:
- AgentDojo, InjecAgent, ASB and AgentHarm provide established attack/task baselines.
- AgentDyn adds dynamic open-ended tasks, helpful third-party instructions and more realistic task complexity: 60 tasks and 560 injection cases.
- Google's November 12, 2026 Agent Anomaly Detection report tested 1,774 sessions across AgentDojo, InjecAgent, ASB, AgentDyn and AgentHarm. It reports post-execution detection of 87% of harmful instructions that were actually carried out; poisoned-memory detection was 4/18. This is a detection result, not a participant-side prevention or authorization result.

The gap we target is **authorization and assurance integrity**: exact principal, actor, consent, grant, delegation, revocation, participant admission, effect, observer independence and the integrity of the evaluator that makes the claim.

MAAP is a preliminary, review-derived minimum authorization-assurance reporting framework, not a validated score or certification. Luminous should preserve its claim-disclosure semantics and add executable cross-layer and participant/effect tests rather than rename established least-privilege or complete-mediation principles as new inventions.

## 6. Further research direction: independent verifier and effect typing

The next major qualification milestone is a second verifier implemented independently of the Python reference validator. It must be tested against the same hidden frozen fixtures and must not import or call the first implementation.

A likely systems direction is to bind operations to narrow capability/effect types:
- principal and exact actor;
- resource and operation;
- argument digest and context;
- policy epoch and validity;
- attenuating delegation lineage;
- one-use permit and idempotency key;
- preconditions and expected effect.

WASI already uses capability-based resource handles and explicitly avoids ambient authority. Luminous should build on those established principles where appropriate rather than inventing a replacement sandbox or resource namespace.

## 7. Claim ceiling

The suite is synthetic. 27/27 authoring-time matches do not prove live complete mediation, real-world graph completeness, production authorization safety, or superiority over industry systems. CI must pass on the exact PR head; a second independent verifier and exact-target participant/effect tests are still required.
