# ADR-0001: Compose an established durable-execution runtime for service episodes

- **Status:** Provisional recommendation; benchmark before adoption
- **Date:** 2026-10-31
- **Scope:** Platform service-episode coordination only
- **Related:** [Issue #165](https://github.com/Luminous-Dynamics/luminous-platform/issues/165), [Saga reference model PR #153](https://github.com/Luminous-Dynamics/luminous-platform/pull/153), [Nixward participant boundary #162](https://github.com/Luminous-Dynamics/luminous-platform/issues/162)

## Decision

**Do not build a general-purpose durable workflow engine in Luminous Platform.** Keep the Luminous service-episode state machine as a domain contract and safety/evidence policy, then benchmark it on established durable-execution runtimes.

Provisional test order:

1. **Temporal Rust SDK — primary implementation candidate.** Its Rust SDK was announced generally available in September 2026 and the published `temporalio-sdk` crate has reached 1.1.1 (2026-10-30). It supplies durable workflow event history, deterministic workflow replay, Activities and workers using native Rust/Tokio APIs.
2. **Restate Rust SDK — primary challenger.** The published `restate-sdk` documentation lists version 0.13.0 and durable handlers/workflows, journaled result replay, durable timers/promises, and configurable retry policies. Its retry defaults require particularly explicit classification of retryable versus terminal/ambiguous side-effect outcomes.
3. **DBOS Rust — experimental challenger only.** DBOS announced a first look at its Rust SDK at v0.5 on 2026-09-29 and described it as under active development. Its Postgres-backed checkpointing and transactional ownership-token approach are valuable design references, but the v0.5 Rust SDK should not be treated as production-equivalent to its more mature SDKs.
4. **Minimal in-memory reference — non-durable control only.** Retain the current Rust state machine as a deterministic semantic oracle and unit-test model, not as a candidate for production recovery.

The benchmark must decide whether Temporal or Restate fits best; this ADR does not declare either selected for production.

## Context and evidence

The current Saga v1 crate is a reference state machine. It checks normalized transitions in memory but does not implement a durable journal, crash recovery, participant runtime, cryptographic trust verification or authenticated observer channel.

Rebuilding workflow history, scheduling, timers, worker polling, ownership, replay, retries, versioning, visibility and operational tooling would duplicate substantial existing systems work. Luminous's differentiating contract should instead be:

- exact plan/step/target/operation/request/authorization binding;
- durable dispatch-intent barrier;
- attempt-specific participant-effect reconciliation;
- explicit unknown outcomes instead of guesswork;
- separately authorized compensation;
- independently measured service postconditions;
- evidence and economic accounting that do not depend on agent assertions.

The engine owns durable *coordination execution*. A participant-owned attempt ledger owns whether a specific external action was accepted or committed. Independent observers own whether the external postcondition is satisfied. Those responsibilities cannot be collapsed into one workflow status.

### Current public implementation signals (checked 2026-10-31)

| Runtime | Current Rust signal | Relevant primitives | Important limitation |
|---|---|---|---|
| Temporal | Rust SDK GA in Sep 2026; docs.rs `temporalio-sdk` 1.1.1 on 2026-10-30 | Durable event history, deterministic replay, Activities, retries, worker model | Workflow/activity history does not prove an arbitrary external service did or did not commit an effect; replayed activities can be retried according to policy |
| Restate | `restate-sdk` 0.13.0 published docs | Durable handler/workflow journal, `ctx.run` recorded results, promises, timers, typed Rust API | Default retries are infinite unless terminal errors/policies stop them; non-idempotent activity effects need explicit idempotency and reconciliation |
| DBOS | Rust SDK v0.5 under active development as of 2026-09-29 | Postgres-backed checkpoints; documented workflow ownership tokens checked transactionally to fence zombie executors | Rust maturity is explicitly preview/active development; use as experimental evidence, not production recommendation |
| Custom Platform runtime | In-repo Saga v1 in-memory reference | Precise Luminous domain state machine | Not durable; would require engineering and qualification of a general execution engine |

Official references:
- [Temporal: Rust SDK generally available, 2026-09-23](https://temporal.io/blog/build-durable-applications-rust-temporal-rust-sdk-now-generally-available)
- [Temporal Rust SDK guide](https://docs.temporal.io/develop/rust)
- [Temporal Event History](https://docs.temporal.io/encyclopedia/event-history)
- [Temporal Workflow replay and Activities](https://docs.temporal.io/workflows)
- [Temporal Rust SDK crate](https://docs.rs/temporalio-sdk/latest/temporalio_sdk/)
- [Restate Rust SDK 0.13.0](https://docs.rs/restate-sdk/latest/restate_sdk/)
- [Restate journaled actions](https://docs.rs/restate-sdk/latest/restate_sdk/context/trait.ContextSideEffects.html)
- [Restate retry/error semantics](https://docs.rs/restate-sdk/latest/restate_sdk/errors/)
- [DBOS announcement: Rust v0.5 actively developed, 2026-09-29](https://dbos.dev/blog/whats-new-in-dbos-september-2026)
- [DBOS workflow ownership and zombie executor fencing](https://docs.dbos.dev/explanations/concurrent-executions)
- [DBOS architecture and Postgres checkpointing](https://docs.dbos.dev/architecture)

## Critical semantic distinction: durable workflow != exactly-once external effects

A durable runtime can persist decisions and step results, replay a workflow, or retry an activity. It cannot independently prove whether a third-party API accepted a request if the response was lost and the provider exposes no attempt-specific authoritative record.

Therefore:

- Do not equate a runtime's “exactly once workflow” language with exactly-once side effects across arbitrary remote systems.
- Every mutating activity/handler must carry a stable logical idempotency key and unique attempt ID, with a canonical request digest.
- A retry with a new attempt is permitted only after an authoritative participant ledger proves the previous attempt was never accepted, or the provider's documented idempotency contract makes repeated delivery safe for the exact request.
- If evidence is insufficient, preserve `outcome_unknown` and require reconciliation or human adjudication.
- A successful Activity/handler return is not a service-resolution oracle. Fresh independent observations must still bind the exact target and requested postcondition.
- No host mutation may bypass Nixward's own transaction interlock, attempt identity, journal and recovery contract.

### Runtime-specific cautions

**Temporal:** workflow code must be deterministic under replay. External I/O, LLM calls and non-deterministic operations belong in Activities. Activity result history is useful for replay, but an Activity's remote side effect may have occurred before a worker crash or lost completion response. Activity retry settings and external idempotency must be treated as separate controls.

**Restate:** the Rust SDK's `ctx.run` journals results and replays recorded results instead of re-running already recorded blocks. The SDK also documents retry policies and currently defaults to infinite retries for retryable failures. The adapter must explicitly bound/reclassify retries for ambiguous non-idempotent operations; a retry policy is not an external effect oracle.

**DBOS:** its ownership-token pattern is a strong reference for fencing zombie workers: the current owner token is checked in the same database transaction as checkpoint persistence. The Rust SDK's v0.5 development status remains a maturity caveat. If tested, pin the exact crate commit/version and report migrations and compatibility boundaries.

## Target architecture

Use a narrow, engine-neutral interface around the domain contract:

- `EpisodeWorkflow`: deterministic orchestration of plan state and step dependencies.
- `AuthorityPort`: obtains/validates exact current grants from the designated authority service. The engine is not an authority issuer.
- `ParticipantPort`: submits one exact attempt, queries the same attempt, and exposes provider-specific idempotency/ledger capability. Unknown is a first-class result.
- `EvidencePort`: fetches signed/qualified independent observation references and verifies trust-domain separation, freshness, plan/target binding and evidence digests.
- `HostParticipantPort`: references the Nixward transaction ID, exact ChangePlan digest, attempt identity and receipt references. It cannot launch host operations or maintain a second host transaction record.
- `ServiceOraclePort`: independently verifies service-level health and expected service digest.
- `EpisodeReadModel`: rebuildable projection for user-facing state, search and reporting; not a second authoritative event history.

For the Temporal arm, the Temporal event history is the durable orchestration history. For Restate, the Restate journal is the durable handler/workflow history. For DBOS, its workflow/step checkpoint rows are the execution record. The Platform must not duplicate those as a second event-sourcing engine. Persist only domain projections and evidence/reference records needed for audit and cross-system joins, with source-of-truth ownership explicitly declared.

The Saga v1 state machine remains the executable *policy projection* used by tests and conformance checks. Each runtime adapter translates runtime callbacks and durable results into explicit Saga transition inputs. Runtime-specific retries must never bypass the Luminous transition guards.

## Benchmark protocol

Use the same 24 crash cases from [Service-Episode Durability v0.1](../benchmarks/service-episode-durability-v0.1/crash-matrix.json), identical synthetic participant behavior, exact grants, failure schedules, and participant-owned side-effect counter across all arms.

Required arms:

- Temporal Rust 1.x pinned exact crate and server version.
- Restate Rust 0.13.x pinned exact server version and retry configuration.
- DBOS Rust v0.5 pinned commit/version (experimental).
- Current in-memory Saga reference (non-durable control).
- Optional minimal PostgreSQL coordinator prototype only if it has a concrete advantage over using an established runtime; do not pre-commit to building it.

Measure separately:
- lost acknowledged workflow events / checkpoint outcomes;
- duplicate participant effects;
- dispatch before durable intent;
- stale/zombie owner writes after fencing;
- automatic retry with an unknown participant outcome;
- valid history replay and malformed history fail-closed rates;
- restart-to-reconciled-state RTO p50/p95/p99;
- durable commit latency and throughput p50/p95/p99;
- event/checkpoint storage amplification and retention/compaction cost;
- unresolved attempts requiring operator adjudication and active operator minutes;
- per-1,000 verified service outcomes infrastructure, licensing and human-active cost.

Hard safety gates are not weighted into a composite score. A single unauthorized dispatch, duplicate business effect, lost acknowledged event under the declared fault model, or automatic replay with unknown effect disqualifies that arm for the test case. Report all workload versions, engine/server/SDK pins, storage settings, retry configurations, seeds, and confidence intervals.

## Acceptance before runtime selection

1. Both Temporal and Restate pass the same adapter contract tests and full synthetic 24-case matrix (or have explicit, attributable failures).
2. A subprocess-kill/reopen harness proves the actual engine's recovery behavior; an in-memory clone is not accepted as crash evidence.
3. External side effects are counted by an independent participant-owned ledger/oracle, not inferred from engine history or coordinator logs.
4. The adapter proves that only a fresh exact authority grant can issue a new attempt after a strong never-accepted result.
5. Unknown side-effect outcomes stay unknown; retries are not enabled merely to improve task completion.
6. Nixward ownership remains intact and host participant support stays read-only/reference-only until Nixward's worker/attempt contract is qualified.
7. Compare operator burden, cost, deployment/upgrade burden and Rust ergonomics before selecting an engine.

## Consequences

**Positive:** avoids recreating workflow infrastructure; improves reliability sooner; supports Rust without production Python/JS glue; lets Luminous focus on authorization, evidence, recovery semantics and service outcomes.

**Negative:** introduces an external runtime dependency and its versioning/upgrade/operational surface; requires careful adaptation between a generic workflow engine and Luminous's stronger side-effect uncertainty policy; hosted service offerings may add cost or data residency constraints.

**Rejected for now:** a bespoke Platform journal/runtime as the production foundation. Reconsider only if measured engine limitations block the safety contract or materially worsen total cost/operability under the benchmark.

## Claim ceiling

This ADR is a provisional architecture recommendation based on published vendor documentation and crate release state as of 2026-10-31. No engine has been integrated, built, fault-injected or benchmarked in this PR. No engine selection, production readiness, universal exactly-once effect or superiority claim is made.
