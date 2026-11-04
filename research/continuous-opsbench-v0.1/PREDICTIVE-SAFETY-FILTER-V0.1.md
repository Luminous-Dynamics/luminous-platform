# Predictive Operational Safety Filter v0.1 — Design Contract

Status: research design; no implementation or qualification result is claimed.

Related work: issue #250; closed-loop plant PR #243; participant topology CAS PR #242; effect gate #187; demand fidelity #184; evidence/VOI #185; sustained SLO recovery #181.

## 1. Purpose and boundary

The agent proposes an intervention. The existing effect gate determines whether the exact act is authorized. The participant checks topology and idempotency at commit. The independent observer later determines what happened.

A missing capability sits between proposal and authorization: a conservative, independent appraisal of whether the proposed action is operationally admissible under the state and model uncertainty visible at decision time.

The filter is deny/defer-only. It cannot grant authority, dispatch, mint a finality handle, commit an effect, update the plant, or replace participant-side compare-and-swap. A positive filter result is necessary at most; it is never sufficient authorization.

## 2. Decision pipeline

Proposed order:

1. Agent emits a typed candidate act.
2. Evidence adapter constructs a bounded belief set from permitted, provenance-bearing observations.
3. Filter checks the candidate against a pinned model set and frozen operational invariants.
4. Filter returns admit, reject, defer, or fallback-required, with a replayable explanation.
5. Existing effect gate independently authorizes the exact action and policy epoch.
6. Participant independently rechecks topology epoch/digest, idempotency and act binding inside its commit boundary.
7. Independent observers measure service, business and collateral outcomes.
8. OpsBench scores the entire chain.

Any change to the action, target, topology precondition, model, invariant set or policy invalidates the appraisal and requires a fresh decision. A filter receipt must bind exact digests; it must not be a reusable blanket permit.

## 3. Formal finite-plant contract

For v0.1, use a bounded discrete state space and explicit transition relation:

- State x: per-class queues, service capacities, retry pressure, dependency/quorum state and resource reserve.
- Action u: one typed intervention from a finite, versioned action catalogue.
- Model parameter θ: one member of a preregistered finite parameter set Θ.
- Disturbance w: one member of a preregistered finite disturbance set W.
- Transition f(x,u,θ,w): the exact versioned plant transition.
- Observation y: agent-visible evidence with source identity, source tick, age/clock uncertainty and declared measurement bounds.
- Belief set X(y): every state consistent with y under the declared observation model. The filter must not read evaluator-only plant truth.

The safe set S is a conjunction of non-compensatory invariants. Examples include critical-class latency/failure ceilings, minimum service floors for protected classes, queue bounds, retry limits, spare-capacity floor, quorum and collateral-impact limits. An aggregate objective cannot offset a protected-class breach.

A candidate action is robustly admissible only when every successor state over the full belief/model/disturbance product remains inside the frozen safe/viable envelope:

    for all x in X(y), θ in Θ, w in W:
        f(x, u, θ, w) is in K

K is the robust viability set, not merely the one-step safe set. For a finite action set, compute a bounded viability approximation by backward reachability:

    K[0] = S
    K[n+1] = { x in K[n] |
               exists a in A_fallback such that
               for all θ in Θ and w in W:
                   f(x, a, θ, w) is in K[n] }

The fallback catalogue is a set of candidate controls only. The filter cannot authorize or execute any fallback. Every actual fallback action still traverses the existing exact-scope authority and participant commit path.

The first implementation may use a conservative one-step K approximation if the finite state space is too large for the chosen horizon, but it must label that horizon explicitly and must not call it a general stability guarantee.

### Degraded-state recovery

A service may already be outside S when an incident arrives. Rejecting every repair would produce a safe-looking but useless deadlock. Therefore distinguish:

- Invariant-preserving mode: current belief set lies in K; admitted proposals must remain in K.
- Bounded-recovery mode: current belief set lies in a separately frozen degraded envelope D. A proposal is admissible only when all modeled successors remain within D, do not worsen any protected invariant, and enter a declared robust recovery set within the preregistered horizon.
- Unknown mode: the belief set, model bounds, disturbance set or required observer evidence is incomplete/contradictory. Return DEFER, not ADMIT.

Recovery progress thresholds and the degraded envelope must be fixed on a disjoint screening set before confirmatory runs.

## 4. Result contract

Every result binds:

- run, episode, arm, scenario, seed and dependence-cluster identities;
- candidate-act digest, target and participant identity;
- topology epoch and digest observed at appraisal;
- observation/source digests, source ticks, freshness and uncertainty;
- model revision, parameter-set digest and disturbance-set digest;
- invariant/safe-set digest and viability-horizon definition;
- filter implementation/build digest;
- exact reachable-set or enumeration commitment;
- disposition, violated invariant IDs, worst-case margins and explanation;
- appraisal timestamp/expiry and unique decision ID.

Dispositions:

- ADMIT_FOR_AUTHORIZATION — all required robust conditions pass.
- REJECT_UNSAFE — at least one enumerated admissible successor violates an invariant.
- DEFER_FOR_EVIDENCE — freshness, source, model or state uncertainty is insufficient.
- FALLBACK_REQUIRED — the candidate is rejected and a separately specified fallback class may be considered by the normal authorization path.

Missing fields, unsupported model revisions, expired evidence, contradictory sources, arithmetic overflow or enumeration truncation must never become a positive result.

## 5. Independent oracle and differential tests

Do not let the filter and its oracle share the same reachability routine.

- Filter: conservative set propagation / interval enclosure over the bounded model.
- Oracle: independently written exhaustive enumerator over every state, action, θ and w in the small finite corpus.
- Mutation harness: changes one digest, bound, source, topology epoch, invariant, action field or result disposition at a time.
- Positive controls: safe action with exact fresh evidence; boundary-safe action; harmless unrelated topology change.
- Negative controls: one-unit-outside-boundary action; stale observation; under-bounded Θ/W; missing class constraint; changed target/action after appraisal; topology change before participant commit; arithmetic overflow; partial reachability computation.

The oracle establishes only finite-corpus correctness. It does not authenticate real sensor sources or prove that actual disturbances lie inside W.

## 6. Required experimental arms

Use matched scenario/seed blocks and identical demand, fault schedule, observation budget, action budget and model access:

1. No-action twin.
2. Deterministic runbook / hysteresis baseline.
3. Agent proposal policy in isolated synthetic-only mode.
4. Same agent with authorization-only gating.
5. Same agent with authorization plus predictive filter.
6. Same agent with filter plus VOI-driven evidence requests and abstention.
7. Oracle-safe controller as a diagnostic upper bound only.

Ablations remove the filter, the model-uncertainty margin, the viability horizon, and the VOI integration one at a time. Preserve every timeout, rejection, deferral and incomplete run in the appropriate denominator.

## 7. Metrics and non-compensatory reporting

Report per scenario family and protected workload class:

- false-safe admission count/rate and exact denominator;
- false rejection rate and lost safe-control opportunities;
- defer/abstain rate and evidence-request cost;
- reachable-set width and worst-case invariant margin;
- SLO-violation integral, queue/retry amplification and collateral impact;
- settling time, bounded-recovery success and no-intervention-tail persistence;
- action churn, reversals and controller conflicts;
- authorized dispatches, topology-stale rejections, committed effects and duplicate effects;
- time, tokens, compute, memory, human-active effort and fully loaded cost.

Safety violations are hard gates, not terms in a weighted score. Report confidence bounds at the actual independent unit; repeated seeds inside one scenario family are not automatically independent samples.

## 8. Initial qualification gates

These are hypotheses for synthetic qualification, not measured results or production thresholds:

- Every admitted action agrees with the independent exhaustive oracle on the canonical finite corpus.
- Zero false-safe admissions in the preregistered negative-control corpus.
- No positive admission when required evidence/model inputs are unknown, stale, contradictory or unsupported.
- No safety-filter approval can bypass exact authorization or participant topology CAS.
- Report false-rejection and defer rates; a filter that rejects every action fails usefulness.
- Demonstrate bounded recovery without critical-class sacrifice on a disjoint confirmatory scenario set.
- Show a paired improvement over authorization-only gating without hiding additional cost or human intervention.

For any sampled zero-event claim, publish the exact one-sided confidence bound and its sampling assumptions. Zero observed events is not a proof of zero production risk.

## 9. Implementation sequence

1. Define Rust types and strict schemas for finite state, parameter/disturbance sets, invariant set, candidate act and appraisal receipt.
2. Implement the smallest deterministic two-workload plant adapter on top of PR #243's transition semantics.
3. Implement conservative filter and an independent brute-force oracle.
4. Add exact-boundary, one-unit-outside, stale-evidence, missing-bound and overflow tests.
5. Add paired no-filter / filter / filter+VOI experiments.
6. Connect appraisal receipts to the existing proposal/effect-gate interface without creating a new policy engine or ledger.
7. Preserve participant CAS as the final topology race check.
8. Freeze a sealed confirmatory corpus, run exact-head CI and publish a claim-limited report.

## 10. Claim ceiling

A passing implementation establishes conformance only for the pinned finite state space, transition relation, parameter/disturbance bounds, invariant set and horizon. It does not prove model completeness, sensor truth, disturbance containment, safety under arbitrary distribution shift, or production stability. Stronger claims require validated plant bounds, independently authenticated field evidence, and a deployment-specific assurance case.
