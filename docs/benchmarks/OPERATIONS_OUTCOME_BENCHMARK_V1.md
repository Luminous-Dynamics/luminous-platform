# Operations Outcome Benchmark v1

**Status:** Proposed, synthetic-only; not production qualification.

## Paradigm shift

Benchmark the full operational outcome, not isolated handlers, tickets closed, or green dashboard states. Follow an incident/change across event intake, tenant and authority checks, durable work-case state, dispatch, independent observation, receipt verification, and recovery.

The research question: can the platform prove only the authorized effects occurred, establish what actually happened after a failure, and tell an operator the truth about uncertainty?

This composes existing ownership boundaries: [#5 durable inbox/outbox](https://github.com/Luminous-Dynamics/luminous-platform/issues/5), [#7 native Work Case](https://github.com/Luminous-Dynamics/luminous-platform/issues/7), [#10 tenant-safe repository](https://github.com/Luminous-Dynamics/luminous-platform/issues/10), [#33 disposable NixOS execution](https://github.com/Luminous-Dynamics/luminous-platform/issues/33), [#42 recovery](https://github.com/Luminous-Dynamics/luminous-platform/issues/42), [#43 typed observation evidence](https://github.com/Luminous-Dynamics/luminous-platform/issues/43), and [#44 durable idempotency](https://github.com/Luminous-Dynamics/luminous-platform/issues/44). It must not introduce a competing case model, event bus, authorization service, executor, or recovery engine.

## Non-negotiable invariants

- **No unauthorized effects:** denied, expired, stale, conflicted or cross-tenant requests cause zero prohibited semantic effects.
- **No fabricated success:** verified requires fresh observation bound to the exact target, operation, scope and postcondition method. Exit code or receipt shape alone is insufficient.
- **No retry roulette:** same identity and semantics refer to the original effect; changed semantics conflict; ambiguous effects are reconciled before unsafe retry.
- **No silent regression:** out-of-order external state cannot overwrite newer state without an explicit conflict policy.
- **Recovery is not success:** preserve primary and recovery outcomes independently.
- **Evidence is honest:** missing, stale, contradictory or untrusted evidence never turns green by default.

All runs use synthetic identities and disposable environments. Never target a production host, customer tenant, live vendor API or real credential.

## Protocol

1. Freeze source SHA, contract revisions, corpus/schema hashes, manifests, toolchain, environment and expected outcomes.
2. Run every scenario in a resettable sandbox with deterministic inputs and labelled fault-injection points.
3. Count semantic effects across a whole scenario, not HTTP requests or worker attempts. Preserve failed and indeterminate results.
4. Capture intent, principal/tenant, authorization, immutable operation identity, state transitions, observed effects, post-state observations, verification method, recovery and evidence references separately.
5. Use an outcome oracle independent of the production decision function; mutation-test it using deliberately wrong outputs.
6. Publish raw per-case results and environment identity. Queued/absent CI is unknown, not pass. A mock is a control, not complete integration.

## Comparison arms

Run the same scenarios against (1) a timed, documented manual/incumbent workflow, (2) a safe contract-only control where meaningful, and (3) the Luminous closed-loop implementation. Never invent a baseline. Unsupported is not pass.

## Metrics and proposed targets

| Dimension | Metric | Gate / target |
|---|---|---|
| Authorization and tenant safety | Prohibited effects, cross-tenant reads/disclosures/mutations | 0 |
| Idempotency | Extra effects from duplicate or same-operation retry | 0 |
| Truthful verification | Verified result without valid fresh evidence | 0 |
| State integrity | Silent regression from stale/out-of-order revisions | 0 |
| Unknown outcomes | Blind retries or unknowns promoted to success | 0 |
| Evidence coverage | Scenarios with raw result and evidence | 100% |
| Operator value | Active time, decisions, handoffs, rework, evidence retrieval | Timed baseline comparison |
| Performance | p50/p95/p99 for intake, case read/transition, receipt lookup, event lag and reconciliation | Measure on pinned setup |
| Resource cost | CPU, RSS/peak memory, storage growth, backlog, recovery overhead | Measure; unavailable is unknown |
| Recovery | Time to known-safe or evidence-backed indeterminate state | Report by fault class |

Initial workload hypothesis: 10,000 synthetic cases, concurrency 1/8/32, warm/cold runs, pinned hardware/software, p50/p95/p99 and error counts. Candidate objectives—not release gates until baseline-tested—are p95 below 250 ms for local case reads and below 500 ms for local case transitions, excluding external provider latency. Report event, dispatch and reconciliation latency separately.

For product value, compare time-to-understand, time-to-verify and manual handoffs against a timed baseline. Set improvement thresholds after baseline collection. Never collapse safety, correctness, latency and cost into one aggregate score.

## Research alignment

- [CloudEvents](https://github.com/cloudevents/spec) defines event identity/context conventions, not exactly-once delivery or durable business idempotency.
- [SLSA verification](https://slsa.dev/spec/v1.2/verification_summary) distinguishes provenance recording from verification.
- [Uptane 2.1](https://uptane.org/docs/2.1.0/standard/uptane-standard) informs rollback, freeze and mix-and-match adversarial thinking for the separate firmware profile.
- [DORA metrics](https://dora.dev/guides/dora-metrics/) inform delivery/recovery measurement but do not replace outcome-integrity gates.

These sources motivate the design; they do not imply compliance or certification.

## Promotion gates

0. Corpus schema validation, unique IDs and recorded hashes.
1. Deterministic oracle, effect bounds, evidence requirements, negative assertions; mutation tests prove bad outputs are caught.
2. Map each scenario to an owning contract and test seam. Missing implementation is unsupported, not pass.
3. Run fault-injected end-to-end scenarios in a NixOS sandbox and independently inspect target state and receipts.
4. Compare operator workflows against frozen baseline.
5. Publish exact-SHA evidence and limitations; relevant code, policy, schema or environment changes invalidate affected qualification.

This work package does not prove a runner exists, any scenario passed, or a production deployment is qualified.
