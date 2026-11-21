# Progress-Frontier Transactional Effects Benchmark v0.1

**Status:** synthetic contract and deterministic reference checker. No agent, transaction runtime, target service or fault injection has executed.

## Why this benchmark exists

TELR tests retries, late commits, stable operation identity and reconciliation. PFTE adds the orthogonal problem of parallel speculative branches and concurrent access to shared resources.

The May 2026 v2 paper [*Atomix: Timely, Transactional Tool Use for Reliable Agentic Workflows*](https://arxiv.org/abs/2602.14849) separates two conditions for safe settlement: a sealed footprint (which effects/reads belong to the transaction) and per-resource frontiers proving that earlier conflicting work cannot still arrive. Atomix distinguishes bufferable effects, externalized reversible effects and irreversible effects. PFTE tests those semantics with the existing CAPC effect identity and OIC/CFI witness claim ceiling; it does not clone or implement Atomix's runtime.

## Core rule

A transaction at epoch E may commit only when:
1. its resource/effect footprint is sealed and unchanged;
2. every resource it reads/writes/touches has a complete, independent, OIC/CFI-qualified frontier at or beyond E;
3. no earlier conflicting transaction for any touched resource remains in flight;
4. read/write resource-version preconditions are still current;
5. only the selected winning branch releases buffered effects;
6. irreversible actions cannot escape before settlement.

A lagging frontier holds only transactions touching that resource. A frontier on an unrelated resource must not create a global lock. A frontier that advances through a known unresolved earlier epoch is contradictory, not "high confidence".

## Effect classes

- Bufferable: no target visibility before commit; abort suppresses the staged effect.
- Externalized reversible: the target has seen the effect; abort requires a separately observed compensation linked to that exact original effect, in reverse dependency order.
- Irreversible: never externalize from a pending or losing branch; release only after a winning, eligible settlement.
- Compensation: a new effect, never erasure of the original.

## Frozen panel

22 synthetic controls: 8 positive, 8 negative, 6 indeterminate. The cases cover commit after frontier, unsealed footprint, unqualified/behind/false frontier, unrelated-resource lag, conflicting versus disjoint branches, abort suppression, reversible compensation and missing compensation, irreversible early release, checkpoint semantic replay, footprint mutation, stale versions, partial commit, incomplete compensation observation, frontier regression and multi-resource commit gating.

## Metrics

Separate frontier false advancement, footprint mutation, stale resource versions, missing versions, in-flight conflicts, losing-branch releases, early irreversible effects, bufferable abort leaks, uncompensated external effects, compensation-order violations, semantic replay, reconciliation mismatch and duplicate visible effect instances. Also measure p50/p95 settlement latency, throughput, contention queue depth, false blocking of disjoint resources, compensation cost, availability and benign task utility. No composite score.

## Research targets (not results)

- zero losing-branch target effects in frozen and runtime-tested panels;
- zero irreversible speculative dispatches;
- 100% correct ordering for conflicting writes and zero stale-write acceptance;
- zero false frontier advancement under common-cause witness faults;
- all disjoint-resource positive controls settle without a global frontier block;
- 100% bufferable abort suppression;
- 100% reverse-dependency compensation and zero false-clean abort after compensation failure;
- zero semantic replay after checkpoint restore;
- 100% deterministic outcomes on frozen mutation controls.

Investigate median settlement overhead below 5% and p95 below 10% of matched tool latency as performance goals, not achieved numbers.

## Competitive evaluation

Compare against Atomix v2 under matched branch/resource/epoch/fault traces; also ablate immediate effects, checkpoint rollback, idempotency-only, Saga-only, global serialization and resource-scoped frontier settlement. Combine LIMBO late-commit conditions with ACRFence restore/replay. Use identical captured model proposals first to isolate settlement, then live-agent runs for end-to-end effects. The target-side independent ledger—not branch logs or model statements—is the effect reference.

A superiority claim requires matched models, tools, resource overlap, branch count, fault schedule, hardware, frontier observer qualification and target ledger.

## Claim ceiling

A passing checker proves only deterministic synthetic semantics. Runtime isolation or serializability needs a disposable multi-branch environment, instrumented target services, independently qualified frontier witnesses, controlled fault schedules and a target-side effect ledger.