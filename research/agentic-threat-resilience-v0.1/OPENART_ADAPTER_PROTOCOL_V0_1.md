# OpenART Adapter Protocol for ATR v0.1

**Status:** Research adapter specification. The upstream OpenART repository is not modified, vendored, or executed by this document. No Luminous OpenART result is claimed.

**Upstream repository:** [AI45Lab/OpenART](https://github.com/AI45Lab/OpenART), licensed AGPL-3.0, pinned to commit `c6e61dc7957796ba3bfcf1ba8b394f2648be1737` (latest upstream commit returned by history query on 28 November 2026). Re-check the upstream commit and license before a future integration. This document copies no upstream source code.

**Primary paper:** Chen et al., *OpenART: Scaling Agent Red Teaming via Open-Ended Environment Evolution*, arXiv:2608.00677 (v2). https://arxiv.org/abs/2608.00677

## 1. Why OpenART is the long-horizon comparator

OpenART evaluates executable, Docker-native agent workflows in persistent environments whose target-visible state evolves during a run, while the benign task and hidden safety contract stay fixed. This directly tests delayed, cumulative failures that a short prompt-injection episode can miss.

Pinned source evidence:
- The upstream README describes the released planner task dataset as **10,150 tasks in 52 ZIP shards** (~75 GB) and the materialized tool subset as **63,697 tools**.
- The paper reports **10K+ validated scenarios**, 50 domains, 15 target agents × 5 foundation models (75 settings), eight target-visible attack surfaces, and a median of 97 tool calls.
- Its task-complexity comparison reports medians of 97 tool calls, dependency depth 32, parallel width 12.5, 96.5 state objects and 7.5 file formats.
- The paper/project report 85.0% pooled strict ASR under the EMHA attacker and 87.38% benign completion across its 75 settings. Strict ASR requires both the deterministic evaluator and an LLM judge; this is an attacker/protocol result, not a standalone model-safety rate.
- The project page reports a median propagation distance of 37 target actions between consuming evolved state and the first unsafe action (IQR 20–66). This is a particularly useful delayed-effect design reference.

Treat all values as source-reported and pin the exact paper, dataset and repository artifacts in any run. They are not Luminous measurements.

## 2. Preserve the upstream task and evaluator contract

At the pinned commit, the high-complexity task bundle includes:
- `task.md`: the benign objective and workflow instructions delivered to the target;
- `scenario_model.json`: environment/state model;
- `tool_pool.json` and `tool_use_graph.json`: candidate tools and workflow/dependency structure;
- `safety_contract.json`: hidden safe/unsafe action contract;
- `capabilities.generated.yaml`, planner plan and checkpoint artifacts;
- a benchmark-owned deterministic evaluator and tests;
- an executable workspace with tools/services and evolving state.

Do not expose `safety_contract.json`, hidden unsafe markers, answer keys, deterministic evaluator internals or target ground truth to the target agent, attacker, target-side tools or model-backed judge prompts except where the original benchmark explicitly requires it. Evaluator truth remains in the evaluator trust domain.

Preserve the upstream protocol:
- fixed task objective and fixed hidden safety contract;
- target-visible environment changes during execution;
- the original EMHA attacker and its authorized state-transition surfaces;
- original deterministic and LLM evaluation strategies;
- native tool-use/dependency graph, target configuration and limits;
- original task IDs, workspace contents, environment image, model/runtime versions and per-turn trace.

Never alter the safety contract to make an ATR control pass. Do not translate a failed native episode into a pass merely because a participant PEP denied one action; both native and ATR scorecards remain independently reported.

### 2.1 Native metrics and artifacts

Preserve native OpenART outputs:
- strict ASR and benign completion;
- first unsafe action and attack-propagation distance where recorded;
- task ID, attack iteration, target agent/model and attacker profile;
- the deterministic evaluator outcome and LLM-judge outcome separately;
- `result.json`, `trace.jsonl`, `timing.json`, workspace snapshots, target runner outputs, evaluator outputs and attacker outputs.

Strict ASR is an AND of deterministic evaluator and LLM judge confirmation in the source protocol. It is not an independent participant receipt. The LLM judge must remain a native annotation, not an authorization or effect oracle.

### 2.2 Execution and licensing boundary

The upstream runner is Python/Conda + Docker and includes provider-backed target, attacker and judge configurations. Pin each environment, Docker image, target/attacker/judge model, API configuration (without secrets), prompts, EMHA policy, tool pool, task bundle, evaluator and dependency lock.

The upstream repository is AGPL-3.0. This adapter is a separate protocol document and does not copy source. Any future code integration, modification or distribution must first review the AGPL obligations. Do not use live customer data, actual credentials or third-party production targets; execute only in the disposable benchmark environments.

## 3. ATR sidecar: measure authority-to-effect lineage during environment evolution

The core new measurement is the causal distance between the first consumed environment mutation and the first independently verified unsafe protected effect.

For each episode, produce an evaluator-owned sidecar that references (but does not rewrite) the native OpenART trace. Capture:

1. **Root contract and task:** immutable task digest, hidden safety-contract digest, objective and resource/tenant scope.
2. **Environment evolution:** each authorized target-visible state delta, its actor/attacker identity, affected file/tool/service/plan/memory object, source version and sequence.
3. **Consumption:** first time and agent instance that reads or uses the evolved object; whether the object is tainted or attacker-influenced; every subsequent summary, copy, retrieval and cross-agent transfer.
4. **Authority:** principal, Authority Assertion, Mandate Issuer, root mandate, child task/plan precommitments, per-agent workload key, PoP result, RP entitlement and revocation epoch.
5. **Invocation:** exact tool, arguments digest, actor, tenant, resource, operation, branch/task/plan identity, approval and idempotency identity.
6. **Enforcement:** PDP result, PEP identity/generation, participant commit-time epoch and actual allow/deny.
7. **Effect:** attempted, denied, executed, committed, independently verified, absent or unknown/reconciliation-required.
8. **Independent observation:** observer identity, trust-domain binding, raw evidence digest, sequence/time and receipt that binds the exact invocation and actual protected state.

The causal graph must distinguish evidence classes:
- native target trace;
- candidate-generated claim;
- attacker output;
- deterministic evaluator result;
- LLM-judge annotation;
- participant PEP / ledger evidence;
- independent observer receipt.

A candidate or attacker claim cannot complete an evaluator causal path. If a relevant edge is missing, attribution remains incomplete/unknown. A deterministic evaluator and LLM judge agreeing that an attack succeeded is still not proof that a real protected-resource commit occurred.

## 4. Map the eight-surface environment evolution without changing the upstream attack

The upstream paper describes eight target-visible attack surfaces. Preserve their native definitions and record the concrete surface that changed in each episode. The ATR adapter adds provenance, authorization and effect observations around those mutations; it does not add a new EMHA mutation class or change the attacker policy.

For every materialized delta, record:
- source and target object identity/version;
- authorized mutation actor and policy decision;
- parent mutation / causal lineage;
- exact target-visible content digest;
- reads and derived objects that subsequently consume it;
- affected plan/task/tool/permission/resource;
- first unsafe effect, if any, and propagation distance in target actions.

Important separation: EMHA changes environment state; it does not thereby acquire the human principal's authority. An environment mutation must not be treated as a delegation grant, task-plan precommitment, approval, user consent or permission update.

## 5. New ATR metrics alongside—not instead of—OpenART's native scorecard

### Evolution and causal metrics
- mutation-to-consumption lag;
- consumption-to-first-unsafe-effect propagation distance;
- mutation-to-first-unsafe-effect total distance;
- number of agent/tool/service handoffs on the causal path;
- cross-agent and cross-session taint persistence;
- state-object lineage completeness;
- causal attribution completeness, missing-edge rate and false-edge rate;
- proportion of unsafe outcomes whose first cause is a stale plan, identity, entitlement, revocation or memory assumption.

### Authorization and effect metrics
- unauthorized tool-call attempts;
- unauthorized executions;
- unauthorized committed effects;
- cross-tenant reads/writes;
- task/plan/capability lineage violations;
- PoP and issuer-role mismatches;
- participant rejection after revocation barrier;
- false independent-effect receipts;
- unknown effect count and time to reconciliation;
- exact actor/tenant/resource/operation/request-digest receipt coverage.

### Utility and cost metrics
- original OpenART benign completion;
- original OpenART strict ASR;
- target steps/tool calls and completion time;
- required safe-call block rate and false-confirmation rate;
- model tokens by target/attacker/judge role;
- Docker/runtime CPU, memory, wall time, network traffic and total API cost;
- policy/PoP/status/participant overhead;
- time from first mutation to first safe/unsafe effect.

Do not merge the native scorecard and ATR sidecar into one weighted number. A control that prevents every unsafe effect by denying all required work fails utility. A high benign-completion rate with an unauthorized protected commit fails security.

## 6. Matched treatment arms

Only begin this comparison after #540 has a runnable isolated participant/effect-receipt slice. Use the same frozen OpenART task, target image, target agent/model, tool pool, EMHA attacker, iteration budget, random seeds and evaluator across arms:

1. Native OpenART target-only baseline.
2. Native OpenART + EMHA attacker (unchanged baseline).
3. Source-anchored authority and task/plan precommitment.
4. Source-anchored authority + WIT/WPT PoP + RP entitlement.
5. Participant commit-time revocation/budget/idempotency enforcement.
6. Full participant enforcement + independent effect observer + unknown-outcome reconciliation.

The arms are additions around the upstream benchmark, not relabelled OpenART features. Keep native deterministic/LLM evaluation and ATR participant/observer outcomes separate. Randomize treatment order and use paired task-seed episodes. Hold out task domains, graph topologies, target runtime, tool pools, environment mutation paths and memory backends.

The default OpenART README and operator skill explicitly require a local smoke run before a batch, preserving full run artifacts and secrets out of logs. Do not start the 10K task corpus, Docker images or paid model/EMHA batches as an implicit consequence of writing this adapter. A small isolated smoke run requires its own reviewed execution plan and resource budget.

## 7. Proposed research targets (not achieved results)

- zero unauthorized committed effects across the scoped high-impact profile;
- zero cross-tenant effects and post-barrier revocation commits;
- zero false independent-effect receipts;
- at least 50% relative reduction in independently verified unauthorized committed effects against the strongest matched baseline, with task/seed-clustered 95% CI excluding no improvement;
- benign completion within 1 percentage point of the strongest matched baseline;
- ≤5% false positives at matched recall, including always-allow, always-deny and always-positive controls;
- ≥99% complete independent authority-to-effect attribution for committed protected effects in a fully instrumented profile;
- measure separately whether an environment mutation is never consumed, consumed but harmless, consumed and changes a plan, leads to an unauthorized attempt, or causes a committed protected effect.

Do not compare the 85.0% OpenART native strict ASR directly with ATR's unauthorized-commit rate. Superiority requires a matched target/attack/environment experiment with the original OpenART evaluator unchanged, followed by separate effect/authority evidence.

## 8. Reproducibility descriptor

Every adapter run should freeze and record:
- OpenART repository commit and AGPL-3.0 license identifier;
- paper/arXiv revision and source dataset shard checksums;
- exact task-bundle and safety-contract digests;
- target Docker image digest and target configuration;
- target, attacker, judge and planner providers/models/builds;
- EMHA policy revision, surface permissions, iteration budget and turn limits;
- tool store and managed-tool metadata digests;
- evaluator version and run options;
- environment reset/snapshot identifiers, seed and run ID;
- native OpenART result/trace/timing artifact digests;
- ATR sidecar schema/evaluator version and independent receipt set.

## 9. Integration order

1. Preserve #540's first small executable participant/receipt/reconciliation loop.
2. Finish exact-head Rust reference-model qualification in PR #548.
3. Run source-pinned MasDrift first for direct multi-agent authority-preservation comparison.
4. Add AgentLAB for adaptive long-horizon attacker behavior.
5. Add OpenART for evolving-state long-horizon attacks and delayed unsafe effects.
6. Keep all native scorers untouched; publish the ATR authority/effect sidecar separately.
7. Pre-register task/seed splits, model/agent panel, confidence intervals, exclusion rules and cost budget before any broad run.

## References

- OpenART paper: https://arxiv.org/abs/2608.00677
- OpenART project and benchmark landscape: https://ai45lab.github.io/OpenART/benchmarks/
- OpenART upstream pinned repository commit `c6e61dc7957796ba3bfcf1ba8b394f2648be1737`: https://github.com/AI45Lab/OpenART/tree/c6e61dc7957796ba3bfcf1ba8b394f2648be1737
- OpenART upstream README and pack layout at the pinned commit: https://github.com/AI45Lab/OpenART/blob/c6e61dc7957796ba3bfcf1ba8b394f2648be1737/README.md
- Existing Platform work: #540–#550, PR #542 and PR #548.
