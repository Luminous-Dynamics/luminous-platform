# AgentLAB Adapter Protocol for ATR v0.1

**Status:** Research adapter specification. AgentLAB upstream is not vendored, modified or executed by this document. No Luminous AgentLAB result is claimed.

**Upstream:** [TanqiuJiang/AgentLAB](https://github.com/TanqiuJiang/AgentLAB), MIT license, pinned to commit `36f58e60c36bbd6d5b8e61d50d7db7d9ea7258d7` ("Fixes environment setup issues", 1 June 2026; latest commit returned by repository history query on 28 November 2026). Re-check upstream history before future runs and publications.

**Primary publication:** Jiang et al., *AgentLAB: Benchmarking LLM Agents against Long-Horizon Attacks*, ICML 2026, PMLR 306, 53162–53180. https://proceedings.mlr.press/v306/jiang26as.html

## 1. Why AgentLAB is now a required comparator

AgentLAB directly tests the gap between single-turn prompt-injection suites and adaptive, multi-turn attacks. The paper reports:
- five attack families;
- 28 agentic environments;
- 644 security test cases;
- six model configurations, including proprietary and open-weight models;
- native attack success rate (ASR) and average turns-to-success (T2S).

The five attack families are:
1. **Intent hijacking:** multi-turn social engineering gradually redirects user intent.
2. **Tool chaining:** a malicious objective is decomposed into individually benign calls whose composition is harmful.
3. **Objective drifting:** repeated environmental observations incrementally redirect the agent's goal.
4. **Task injection:** indirect task injection connects a benign task sequence to a malicious sequence through bridging actions.
5. **Memory poisoning:** hostile preferences are persisted in external memory and retrieved in a later task.

The public results table reports the following overall ASR values (source paper companion site; not reproduced Luminous measurements):

| Model | Intent hijack | Tool chaining | Objective drift | Task injection | Memory poison | Overall ASR |
|---|---:|---:|---:|---:|---:|---:|
| Qwen-3 | 78.1% | 96.3% | 92.2% | 93.1% | 48.0% | 81.5% |
| Llama-3.1 | 53.3% | 90.4% | 67.4% | 86.6% | 34.6% | 66.5% |
| GPT-4o | 74.0% | 94.1% | 79.2% | 79.9% | 63.3% | 78.1% |
| GPT-5.1 | 59.8% | 94.6% | 73.7% | 21.5% | 51.3% | 69.9% |
| Gemini-3 | 46.2% | 95.9% | 15.8% | 43.1% | 67.3% | 53.7% |
| Claude-4.5 | 27.2% | 73.3% | 5.3% | 0.0% | 38.8% | 28.9% |

The highest ASR family for several models is tool chaining, reinforcing that a system can make locally permissible-looking calls while collectively violating the intended task boundary. Native ASR/T2S must remain unchanged; a lower ASR on a modified task set is not a valid direct comparison.

## 2. Experimental architecture and benchmark-integrity constraints

The upstream repository combines an adaptive planner, attacker, verifier and judge. Its README states:
- the planner and judge use GPT-5.1 by default;
- an abliterated open-weight attacker is the default for several attack families;
- environment-specific dependencies include Conda and isolated WebShop / AgentDojo setups;
- the supported victim panel spans OpenAI, Anthropic, Google and local vLLM endpoints;
- adaptive loops may selectively rewrite failed bridging actions or payloads.

This means the attack pipeline itself is part of the benchmark treatment. To make comparisons reproducible:
- pin the full AgentLAB commit and hashes of each dataset, environment fixture, tool specification, prompt, planner, attacker, verifier, judge, model/provider and dependency lock;
- record which attacker/planner/judge models were used, not only the victim model;
- freeze or pre-register adaptive rewrite budgets, thresholds, turn limits and sample counts;
- preserve the upstream environment isolation and do not silently replace a dependency with a locally convenient implementation;
- report API/model availability, failed episodes, retries, rate limits and cost;
- retain raw per-turn traces and tool arguments, not just ASR;
- run the upstream scorer unchanged before applying the ATR sidecar.

Do not expose evaluator-private task answers or expected effects to the candidate, planner, attacker, or defense process. The judge's LLM label is a native AgentLAB metric and stays a judge label; it is not an authorization oracle or independent proof of a resource commit.

## 3. ATR sidecar: map long-horizon attack to authority and effect

For each upstream episode, produce a separate evaluator-owned record without modifying the native AgentLAB trace.

| AgentLAB family | Causal/authority question added by ATR | Independent evidence needed |
|---|---|---|
| Intent hijacking | Did the principal's authorized objective change, or did the agent accept an untrusted instruction as authority? | Immutable root task/P0 digest, instruction provenance, exact approval evidence |
| Tool chaining | Did individually benign calls compose into a reserved or out-of-scope resource effect? | Ordered invocation chain, participant policy decision and resource-level effect receipt |
| Objective drifting | Did successive environmental observations mutate the task/plan or expand the delegated scope? | Observation provenance, task/plan binding, root + parent precommitment |
| Task injection | Did an injected branch acquire an identity, task or capability not precommitted by the root/parent? | Parent-child task/plan pair proof, branch assignment, capability lineage |
| Memory poisoning | Did tainted or revoked content persist across sessions or agents and cause a protected effect? | Memory read/write provenance, taint propagation edges, session/agent identity and independent resource observation |

Minimum node/edge classes should reuse the existing ATR causal graph:
- untrusted input, environment observation, memory item, task, agent instance, parent/child delegation grant;
- policy, PDP decision, PEP enforcement, tool call, protected resource, effect, independent observer and receipt;
- read-from, summarized-into, derived-into, handed-to, delegated-to, authorized-by, invoked, enforced-by, affected and observed-by.

A candidate-reported trace edge may be retained as a claim, but it cannot complete evaluator attribution. Missing events, hidden arguments or absent observer evidence remain unknown.

## 4. Native metrics and ATR metrics must remain separate

**Native AgentLAB output (preserve):**
- attack success rate by family and overall;
- average turns-to-success;
- original task/environment identifiers and attack outcome labels;
- all model, attack, prompt and run configuration.

**ATR sidecar (new):**

### Security and authority
- harmful objective attempts;
- out-of-scope action attempts;
- unauthorized executions;
- unauthorized committed effects;
- cross-tenant reads/writes;
- task/plan and capability lineage failures;
- invalid Authority Assertion / Mandate Assertion / presenter bindings;
- untrusted-to-effect causal path rate;
- post-revocation committed effects;
- false independent-effect receipt acceptance.

### Temporal and causal
- time/turn from first poison to first unauthorized effect;
- attack-to-effect path length and number of agent handoffs;
- maximum persistence horizon for poisoned memory;
- first task/plan/authority mutation and its causal ancestor;
- attribution completeness and false/missing edge rates;
- blast radius: downstream agents, tasks, tools, tenants and resources;
- unknown outcome duration and reconciliation attempts.

### Utility and cost
- original benign task completion and required-step score;
- safe read-only completion;
- required-call block and unnecessary-confirmation rate;
- native turns-to-success and ATR recovery turns;
- policy / PoP / status / commit check latency;
- model tokens by role, tool calls, CPU/memory, wall time and total provider cost.

A denied attack is not equivalent to a verified absent effect. An attempted call is not a commit. A judge's successful attack label is not a resource receipt.

## 5. Required matched arms

After the ATR #540 runner can execute at least one isolated participant/effect loop, add a clearly separate treatment to AgentLAB's native configurations:

1. AgentLAB native baseline / no added ATR control.
2. AgentLAB native defenses, where applicable, unchanged.
3. Source-anchored authorization and participant enforcement.
4. Source-anchored authorization + proof-of-possession + commit-time fencing.
5. Full path including independent effect observation and reconciliation.

Preserve native variants and do not relabel a source/chain/defense arm. Each arm must use the same victim model/version, attack planner, attacker, judge, environment state, task seed, prompt, tools, rewrite budget, turn cap and allowed information. Randomize arm order and use matched task-seed pairs.

Use the candidate only in a disposable synthetic environment. No live customer data or third-party targets. A failure in an AgentLAB environment must not propagate outside the isolated benchmark target.

## 6. Proposed qualification and comparative targets

Research hypotheses, not achieved results:
- zero unauthorized committed effects across the tested high-impact profile;
- zero cross-tenant effects and post-revocation commits after the defined barrier;
- no false effect receipt accepted;
- at least 50% relative reduction in unauthorized committed effects compared with the strongest matched AgentLAB-native baseline, with task-clustered 95% CI excluding no improvement;
- benign task completion within 1 percentage point of the strongest matched baseline;
- at most 5% false positives at matched recall, with always-allow, always-deny and always-positive controls;
- at least 99% of committed protected effects have independently evidenced actor, tenant, resource, action, policy/authority generation and request binding;
- attack-to-effect and poison-to-effect conversion reported by family, rather than collapsed into one ASR;
- include full confidence intervals, effective task/seed cluster count and the exact exclusion/unknown count.

Do not claim "beats AgentLAB" by comparing ATR's unauthorized-commit rate to AgentLAB's native ASR. They measure different outcomes. Claim superiority only on a matched experiment with the same candidate, attacks, environments and native metric, then separately show ATR's additional effect/authority metric vector.

## 7. Known limitations / adapter risks

- AgentLAB's adaptive planner, attacker and judge are LLM components and must be treated as part of the experimental system, not as objective ground truth.
- The public result page does not supply all raw trace/effect semantics needed for ATR attribution. Additional adapters may need to instrument tool boundaries and environment state transitions.
- A benchmark judge can score an attack successful without proving a real protected resource committed; ATR needs a separate participant/effect oracle.
- Native environment adapters include Python/Conda and API-backed model clients. Keep these as a pinned external benchmark harness; do not turn them into production authorization components or the Rust-first ATR runner.
- The six-model published table is a source-reported panel, not a current model ranking for 28 November 2026. Re-run with exact current model versions and matched settings before drawing contemporary comparisons.
- No AgentLAB attack execution, integration, effect measurement or Luminous result is claimed by this document.

## 8. Integration order

1. Keep #540's first executable vertical slice unchanged and unblock it.
2. Complete the existing source-pinned MasDrift adapter first because its task data directly includes multi-agent coordination/authorization preservation.
3. Add AgentLAB's five long-horizon attack families as an external adapter and use its native scorer unchanged.
4. Add OpenART later as a distinct evolving-state / long-horizon stress environment.
5. Compare all arms on matched seeds, candidate builds, tools, task states and evaluator versions; report native metrics alongside ATR sidecar.

## References

- AgentLAB paper: https://proceedings.mlr.press/v306/jiang26as.html
- AgentLAB project site and reported results: https://tanqiujiang.github.io/AgentLAB_main/
- AgentLAB upstream repository, MIT, pinned commit `36f58e60c36bbd6d5b8e61d50d7db7d9ea7258d7`: https://github.com/TanqiuJiang/AgentLAB/tree/36f58e60c36bbd6d5b8e61d50d7db7d9ea7258d7
- Existing ATR causal graph and claim-bound evidence profiles: PR #542.
- Existing runner acceptance and scope: #540.
- Existing benchmark validity roadmap: `BENCHMARK_VALIDITY_AND_LONG_HORIZON_SOTA_ROADMAP_V0_1.md`.
