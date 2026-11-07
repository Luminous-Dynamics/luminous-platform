# Decision-aware read-only probes v0.1

**Status:** draft integration contract and synthetic fixtures. No live collector, production probe, Symthaea EVSI implementation, or operational authorization is established by these files.

## Purpose

This package defines a narrow handoff from a cognitive planner's request for an observation to an independently enforced read-only Platform probe, then binds the resulting source observation and probe-impact evidence without copying either evidence payload into a new envelope.

It composes existing work rather than creating another planner, source registry, evidence graph, authorization engine, probe scheduler, collector, or effect ledger:

- Platform #185: risk-aware value of information and active diagnosis.
- Platform #257: diagnostic probe interference and independently measured externalities.
- Platform #214: temporal topology and causal-model freshness.
- Platform #299: cross-project adapter issue.
- Symthaea #10105: current robust expected/worst/CVaR information-gain planner.
- Symthaea #10109: proposed decision-aware EVSI/minimax-regret extension (issue, not yet an implemented API).
- Mycelix: source dependencies, provenance and common-cause semantics.

## Important upstream maturity distinction

At the time of this draft (7 November 2026), Symthaea PR #10105 provides a robust-EIG API. EIG is not EVSI. Symthaea #10109 specifies a future decision-aware EVSI/minimax-regret extension, but is still an issue. Therefore:

- point_eig, expected_eig, worst_case_eig and cvar_eig are supported proposal method labels for the current/future planner family.
- expected_evsi, worst_case_evsi, cvar_evsi and minimax_regret are reserved for the decision-aware extension and must not be emitted as implemented results until upstream code and exact-head qualification exist.
- A finite-scenario worst-case result is only worst case over the supplied finite scenario set. It is not a guarantee over all possible environments.
- A planner build digest and result digest identify inputs/results; they do not authenticate likelihoods, utility choices, evidence sources or truth.

## Flow

1. Symthaea emits a proposal referencing one entry in a pinned Platform probe-catalog snapshot.
2. Platform independently checks the principal's immutable intent and permitted purpose, principal/session/tenant/target scope, current policy, catalog entry, exact operation/schema digests, read-only capability, result sink, freshness requirements and cumulative session budget.
3. If admitted, the Platform/collector executes the catalogued observation. The planner receives no collector credentials or write-capable handle.
4. The collector emits its source observation using the existing canonical evidence/event path.
5. A separately controlled observer emits a probe-impact receipt through the #257 contract. Missing impact evidence is pending/unknown, never numeric zero.
6. The binding record links the proposal, authority decision, budget reservation, observation receipt, impact receipt and optional evaluator-owned decision-evaluation receipt by exact references/digests.
7. Symthaea may update its belief/decision from the acquired evidence. Any consequential mutation still requires a separate, fresh authorization and participant-side enforcement.

## Files

- decision-aware-probe-proposal.schema.json: planner-originated request. It can only request READ_ONLY_PROBE and binds the request to a catalog entry, target, topology epoch, operation/input/output digests, time/freshness requirements and requested resource ceiling.
- decision-aware-probe-binding.schema.json: cross-record outcome. It stores references/digests for the authority decision, reservation, observation, independent impact record and optional evaluation. It does not duplicate canonical receipt payloads.
- fixtures/proposal-positive.json: valid synthetic robust-EIG proposal.
- fixtures/proposal-write-capability-rejected.json: intentionally invalid: write-capable probe.
- fixtures/proposal-metric-mismatch-rejected.json: intentionally invalid: planner method and predicted metric disagree.
- fixtures/proposal-purpose-mismatch-rejected.json: structurally valid but semantically invalid: the requested purpose differs from the principal-authorized episode purpose.
- fixtures/proposal-external-sink-rejected.json: intentionally invalid: result sink leaves the principal-authorized session.
- fixtures/binding-completed.json: synthetic admitted/completed example with separate observation and impact receipts.
- fixtures/binding-unknown-impact.json: admitted probe whose collection/impact outcome remains unknown. It must not receive zero-impact or successful evaluation credit.
- fixtures/binding-denied.json: denied request with no collection and no impact measurement.
- fixtures/binding-completed-without-impact-rejected.json: intentionally invalid: completion is claimed without a canonical probe-impact receipt.

## Security and semantic rules

- JSON Schema validity is structural validation, not authorization.
- Platform must resolve the catalog entry from its own pinned snapshot and compare all supplied digests with the canonical catalog record. An agent-supplied catalog entry is not authoritative.
- The capability value is constrained to read_only; mixed, write-capable, unknown or unbounded probes are not admitted by this v0.1 adapter.
- The agent cannot self-assert source independence. The binding records control-domain references; Mycelix's existing dependency/common-cause profile determines what relationships are established, shared or unknown.
- Two distinct control-domain identifiers are not, by themselves, proof of independence.
- The session budget version must be atomically reserved by the platform participant. Caller-supplied remaining budget is never trusted.
- Timeout, cancellation or transport loss does not prove the collector stopped. A collection with unresolved server-side execution remains unknown until reconciled.
- An impact receipt absent because an observer failed is not evidence that impact was zero.
- A model prediction from Symtropy remains a simulation result, not an observation or probe-impact receipt.
- A Sol Atlas projection is context, not a source observation or grant.
- A probe result cannot authorize a mutation. The #168 effect gate and actual participant boundary remain separate.

## Required semantic checks beyond JSON Schema

The schema intentionally cannot establish the following cross-record predicates. The adapter/evaluator must check them:

1. Proposal digest equals the canonical digest of the submitted proposal under the repository's pinned serialization rules.
2. Proposal episode, tenant, principal, session, target and topology bindings match the active episode and current server-side context.
3. Planner method equals predicted_quantity.metric; EVSI/minimax methods require the actual upstream method/API and utility-model digest.
4. Probe catalog snapshot/entry, target, operation, input schema, output contract and read-only capability match the pinned server-side catalog.
5. Requested resource ceiling is no greater than catalog, policy and current session limits.
6. Budget-state version is current at atomic reservation; concurrent agents cannot overspend one shared envelope.
7. Admission policy decision is source-authenticated, current, scoped to the exact proposal and bound to the exact catalog entry.
8. Collection only follows an admitted decision and committed reservation.
9. A completed collection has exactly one canonical observation receipt with source lineage, valid-time/observed-time and exact probe binding.
10. An observed impact receipt is separately sourced for the exact probe/run/target/time window. Its control-domain relationships are evaluated by Mycelix; no scalar independence is inferred.
11. Non-completed collection cannot carry a fabricated observation receipt; non-observed impact cannot carry a purported impact receipt.
12. An evaluated decision record is evaluator-owned and binds before/after evidence sets, utility-model digest, planner result and actual observation receipt.
13. Unknown collection or impact blocks any claim that the probe safely completed or had zero externality.
14. Any actuation after the probe is separately authorized at the actual participant; this contract grants none.

## Benchmark programme

Use the same sealed scenarios, probe catalog, initial evidence, session resource budget and evaluator across:

1. No-probe control.
2. Random eligible probe.
3. Cheapest-first.
4. Entropy-per-cost / point-EIG.
5. Robust expected-EIG.
6. Robust worst-case-EIG.
7. Robust lower-tail CVaR-EIG.
8. Decision-aware expected EVSI.
9. Worst-case/lower-tail EVSI and minimax-regret when upstream implementation exists.
10. Decision-aware selection with independently measured probe-interference externality.
11. Fixed runbook and evaluator oracle (oracle is diagnostic upper bound only; hidden truth remains evaluator-owned).

Report separate dimensions: decision regret; realized risk reduction; calibration of predicted gain; evidence freshness/lineage; duplicated-source over-credit; correct stop/abstain/escalation; probe impact; resource use; privacy; unauthorized probe attempts; cumulative budget violations; time to safe decision; human review; cost per verified decision.

The oracle is not a fair deployable baseline. It cannot share hidden truth with candidate agents.

## Initial research targets (not results)

- Zero write-capable or unknown-capability probe admissions in the frozen negative-control corpus.
- Zero unauthorized target/tenant/scope admissions.
- Zero duplicate/correlated observation over-credit in declared adversarial fixtures.
- Zero completed/safe probe claims without a canonical observation receipt and an impact state that is observed or explicitly unresolved.
- 100% detection of deliberately missing/stale/incorrect catalog, source, target, topology and budget bindings in the conformance corpus.
- 100% acceptance of valid positive controls, including legitimate read-only probes and correct no-probe/stop decisions.
- At least 10% lower realized decision regret versus point-EIG on a preregistered likelihood-misspecification corpus, without increased critical probe impact. Aspirational, to be revised after power analysis.
- At least 15% lower probe cost/latency than cheapest-first at non-inferior decision quality. Aspirational.
- All model-backed results require paired scenario-family analysis, held-out scenario/topology splits, confidence intervals and full accounting of failed, timed-out, missing and unknown outcomes.

## Claim ceiling

This draft establishes only a proposed structural integration contract and synthetic fixtures. It does not show that Symthaea currently implements the EVSI methods, that Platform has a live probe catalog/collector adapter, that Mycelix has authenticated a given source graph, that a probe is harmless, or that any model beats SOTA. A validated finite synthetic experiment qualifies only the exact scenario/catalog/policy/evaluator versions tested.

## Related issues

- Platform #185, #214, #257, #268, #274, #297, #299.
- Symthaea #10105 and #10109.