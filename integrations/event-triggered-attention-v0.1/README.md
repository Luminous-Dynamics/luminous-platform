# Event-triggered sustained attention v0.1

**Status:** draft watch contract and synthetic conformance fixtures. No production event monitor, event broker, timer service, or autonomous runtime is implemented here.

## Purpose and reuse

This package defines a bounded watch request and cross-record result binding over existing Operations Event CloudEvents and durable inbox/outbox infrastructure. It composes Platform #188 (continuous SRE), #185 (risk-aware VOI), #257 (probe interference), #214 (temporal topology), #299/PR #302 (decision-aware probes), and #306 (event-triggered attention). It does not create another event envelope, delivery layer, scheduler, or authority engine.

External comparator: SentinelBench, Microsoft Research, June 2026: https://arxiv.org/abs/2606.05342. Its 100-task/10-environment evaluation contrasts fixed `sleep` polling with condition-based `wait_for`; the published GPT-5.4 low-reasoning baseline reports 75% vs 68% completion and $0.23 vs $1.17 median task cost. The 40-minute stretch reports 69/100 at $0.48 for wait_for vs 56/100 at $4.65 for sleep. These are SentinelBench figures, not SRE or Luminous results. SRE-Marathon (https://arxiv.org/abs/2609.33023) remains a separate fixed-cadence continuous SRE comparator; do not change its native score.

## Core boundaries

- Watch conditions use only `bounded_predicate_v1`; no executable callbacks, scripts or arbitrary model-generated code.
- Principal, session, intent, purpose, tenant and target remain bound.
- Xenia capability/revocation references in a request are untrusted references until resolved through the authenticated Xenia session.
- Requested budgets are ceilings, not reservations. The authoritative Platform participant must reserve them atomically.
- Event delivery reuses existing CloudEvents/inbox/outbox identity, cursor, sequence, acknowledgement and replay semantics.
- A wake may wake Symthaea, notify the principal or request human approval. It never authorizes mutation.
- Cursor gaps, missing events, stale time basis, revocation and unknown state remain explicit.
- Missing event/impact evidence is never counted as zero loss or zero cost.

## Files

- `watch-request.schema.json`: task, authority, source, condition, policy and resource ceilings.
- `watch-result.schema.json`: admission, terminal state, event delivery, wake disposition and zero-mutation claim ceiling.
- fixtures: valid watch, principal-purpose mismatch, forbidden executable callback, matched event, quiet expiry, dormancy revocation and delivery-gap escalation.
- `validator/`: Rust Draft 2020-12 and cross-record semantic audit.

## Semantic limits

JSON Schema and these fixtures cannot authenticate Xenia or a producer, prove durable cursor completeness, verify real sequence monotonicity, establish common-cause independence, guarantee event delivery, or prove runtime cancellation. A valid watch request is not proof that a watch was admitted. A wake is not proof that the requested condition is true. A model prediction is not event truth.

## Benchmark direction

Compare no-watch, fixed polling, adaptive polling, condition wait, durable subscription, event+heartbeat, Symthaea adaptive wake, human notification and evaluator-only oracle arms on identical hidden schedules. Preserve SentinelBench's native tasks/scores; report Luminous SRE extensions separately. Publish completion, event misses, false wakes, p50/p95/p99 reaction time, LLM calls/tokens/context, CPU/memory/I/O/network, cost, human burden, cursor integrity, source lineage and authorization outcomes. Hard authority and missed-ephemeral-event failures are non-compensatory.

## Claim ceiling

This is a schema/validator/fixture tranche only. It does not establish delivery reliability, real-time guarantees, zero-loss subscriptions, production revocation propagation, or performance superiority.

## Exact request/result binding

Each `watch_request_digest` is SHA-256 over the exact UTF-8 bytes of the submitted watch-request payload, including its terminating newline where present. The paired Rust auditor recomputes the digest and rejects a well-formed but unrelated value. A runtime must hash the exact request bytes it received and persist/replay those bytes; it must not parse and reserialize them before digest verification. This is a byte commitment, not proof that Xenia references or source assertions are authentic; those still require resolution through the canonical owner systems.

## Deterministic bounded predicate evaluator

`validator/src/predicate.rs` provides a pure, non-LLM reference evaluator for the declared operators: `equals`, `not_equals`, integer-only `greater_than` / `greater_or_equal`, string/array `contains`, `present`, and `absent`. It traverses object-only dotted field paths, performs no type coercion, and returns `Indeterminate` for missing comparison inputs or incompatible types. It rejects malformed paths and unknown operators. The caller must resolve `expected_value_digest` through the authorized source/adapter before supplying the expected JSON value. The evaluator does not fetch events, verify producers, schedule wakes, or grant mutation authority.
