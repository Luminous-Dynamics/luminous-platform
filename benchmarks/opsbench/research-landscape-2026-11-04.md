# Autonomous SRE and operations-agent benchmark landscape

**Snapshot date:** 2026-11-04  
**Purpose:** choose fair external comparators, identify saturated task cohorts, and define the next OpsBench research frontier.  
**Claim status:** literature and public leaderboard review only. No Luminous agent score or SREGym execution is claimed here.

## Executive finding

There is no single SOTA number for autonomous operations. The most visible benchmarks measure different things:

- **SREGym** measures diagnosis, mitigation and end-to-end completion in live Kubernetes environments.
- **SRE-Marathon** measures long-horizon, overlapping fault response under a changing deployment and workload.
- **EnterpriseOps-Gym** measures stateful enterprise task completion across tickets, approvals and systems.
- **AgentRx** measures localization and attribution of the critical failure step in agent execution trajectories.
- **EvoHarnessBench / Evo-Bench** measure whether agents retain competence and adapt as the tools, skills and specialist-agent harness changes.

Comparing raw scores across these datasets would be invalid. The correct strategy is a *benchmark portfolio* plus a separate, independently verified operational-assurance layer.

## Public evidence snapshot

| Benchmark | What it measures | Current public reference | What it does not establish |
|---|---|---|---|
| SREGym-Lite-1004 | 17-fault diagnosis + mitigation; E2E requires both on the same run | The first five of 25 listed configurations show 100% diagnosis, mitigation and E2E. Across those five, TTD ranges 36.6–310s, TTM 65.9–675s, and tokens/run 554K–1.74M. | Production safety, complete full-suite competence, sustained recovery beyond the native task, or identical execution budgets. |
| SREGym full | Live cloud-native SRE problem execution | At pinned source commit `51964ce0a911be3e71d9e1e75551bb0089dc70c7`, default built-in selection is 217 IDs when `SREGYM_FAULT_DIRS` is unset. | A Luminous run. The published 90-problem reference is not the executable default set for this later source snapshot. |
| SRE-Marathon | Continuous operation, overlapping seeded faults, persistent workspace, and code/config changes | The paper reports three applications, about 60 faults per run and best score 41.3/100 across ten methods. | SREGym or EnterpriseOps-Gym performance; its composite must not be converted into another benchmark's score. |
| EnterpriseOps-Gym | Stateful enterprise workflows across simulated company systems | BenchLeader's 3 Nov 2026 snapshot reports 51.1% for Claude Fable 5, ahead of Gemini 3.7 Flash at 50.4%; 50 configurations have published results. The top result itself is dated 9 Jun 2026. | Incident diagnosis, safe infrastructure mutations, participant effect finality, or production SRE competence. |
| AgentRx | First critical failure-step localization and category attribution | Final EMNLP Findings 2026 paper: 170 annotated trajectories across 11 settings; reports +75% average step-localization improvement over prior work. | Operational repair success or service restoration. |
| EvoHarnessBench | Retention and adaptation under tool/skill/agent harness evolution | 17 streams, 802 tasks, 520 tools, 42 skills, 62 agents; reports harness-induced forgetting and inconsistent adaptation. | SRE-specific production recovery. |
| Evo-Bench | Autonomous agent-harness diagnosis, rewriting and improvement | Public project reports 608 harness-sensitive tasks, nine evolver models and a best +16.6-point absolute gain over the seed harness. | A direct SRE score or guaranteed transfer to operational work. |

### Sources

- SREGym official leaderboard: https://www.sregym.com/leaderboard
- SREGym pinned source commit: https://github.com/SREGym/SREGym/commit/51964ce0a911be3e71d9e1e75551bb0089dc70c7
- SRE-Marathon, arXiv: https://arxiv.org/abs/2609.33023
- EnterpriseOps-Gym public leaderboard snapshot: https://www.benchleader.com/benchmarks/aa_enterprise_ops_gym
- AgentRx, ACL Anthology: https://aclanthology.org/2026.findings-emnlp.598/
- EvoHarnessBench, arXiv: https://arxiv.org/abs/2609.04280
- Evo-Bench, arXiv: https://arxiv.org/abs/2608.09096

## Additional 2026 comparators: SRE-agent quality and intrinsic risk

### PagerDuty SRE Agent Benchmark

PagerDuty's own 2026 benchmark reports an equal-condition sweep over 17 scenarios. Its latest complete sweep ranks PagerDuty SRE Agent at **0.77**, Claude Code (Opus 5.5) at **0.68**, and Codex (Sol 5.6) at **0.66** on a composite of localization, diagnosis, mitigation, evidence grounding, explanation quality and inverse misdirection. The report explicitly warns that the latest complete sweep is a **single trial**; earlier sweep-to-sweep variability was roughly 2 percentage points. PagerDuty's follow-on engineering retrospective reports that a rebuild improved diagnosis score by roughly 20 points while also making the agent faster and cheaper, and describes a contamination scanner on each sweep.

This is a valuable close-domain comparator because it makes context access, misdirection, evidence grounding, latency and cost visible. It is not a statistically conclusive superiority claim and is not the SREGym leaderboard. We should replicate the protocol if access permits, retain the full dimension vector, and add paired scenario seeds and uncertainty rather than reducing it to one composite.

Sources:
- https://www.pagerduty.com/fr/resources/ai/learn/sre-agent-benchmark-report/
- https://www.pagerduty.com/eng/lessons-from-pd-labs-from-experimentation-to-a-production-ready-sre-agent/

### HINTBench: risks created by the agent itself

HINTBench addresses intrinsic risk: unsafe trajectories can arise in benign settings without an adversarial prompt or external attacker. Its 2026 benchmark contains **596 trajectories** (400 synthetic risky, 136 synthetic safe, 30 reconstructed real-world risky, 30 reconstructed real-world safe), averaging 24 steps. It evaluates trajectory-level risk detection, risk-step localization and intrinsic failure-type identification. The best reported model remains below **37 Strict-F1** on fine-grained risk-step localization.

This motivates a separate *benign-operation negative-control* track in OpsBench. Examples include an agent gradually over-expanding permissions, retrying uncertain mutations, deleting an unhealthy dependency to make a health check green, silently lowering workload, or learning a brittle workaround from a prior incident. The evaluator should score first unsafe commitment, irrecoverable breach, detectability, and recovery/handoff—not merely final outcome. Agent self-generated unsafe behavior must be measured separately from external fault injection.

Source: https://aclanthology.org/2026.findings-emnlp.1639/

### Harness Bench: a broad controlled harness comparison

Harness Bench describes 300 sandboxed tasks across seven native harnesses and ten model backends, with 20,454 evaluated runs. Its task taxonomy includes 65 systems-engineering tasks. The public site reports a 57.3 mean score and 22.8% full-credit rate on the general suite, using task-specific oracles rather than an LLM judge.

This is not an SRE-only benchmark, but it offers a useful harness-comparison method: keep task oracles stable while swapping model backends and harnesses. OpsBench should borrow the factorial structure, not import or equate its aggregate score.

Source: https://www.harness-bench.ai/

## Paradigm shifts for OpsBench

### 1. From task completion to proof-carrying operations

A repair claim is not a repair. A tool call is not a committed effect. A participant receipt is not proof that the customer-facing service recovered.

The benchmark should record and independently reconcile the complete chain:

`observation → causal hypothesis → decision → authority → dispatch → participant acceptance → committed effect → independent service postcondition → sustained recovery`

Each transition needs stable IDs, explicit causal bindings, event-time/ingest-time distinction, and independent evidence. Unknown is not success. Cleanup by the evaluator is never credited as agent repair.

### 2. From final-answer grading to causal-breakpoint evaluation

AgentRx makes failure localization a measurable capability. OpsBench should go further by locating the earliest *irrecoverable* trajectory breach and binding it to the evidence actually available at that decision time.

Candidate breakpoints include stale evidence accepted as current, an unsupported causal edge promoted to fact, a forbidden or unbound mutation proposed, an authorization bypass attempted, a duplicate effect committed, or a recovery claim issued without an independent postcondition.

Use evaluator-known truth and replayable constraint violations as the primary oracle. LLM judges may assist human triage, but must not be the sole source of causal truth. Preserve agent, environment, harness and ambiguous attribution classes separately.

### 3. From static harnesses to controlled harness evolution

Adding tools and skills can improve new-task performance while silently breaking old competence. Evaluate at least three frozen harness snapshots:

- Baseline harness.
- Capability expansion (new tools/skills, no policy rewrite).
- Harness adaptation (agent may update its reusable operational harness under a fixed budget).

Measure old-task retention, new-task transfer, forgetting, cross-family regressions and cost of adaptation. Keep model weights, access profile and evaluation tasks pinned so a harness effect is not confused with a model upgrade or leakage.

### 4. From episodic remediation to closed-loop control

An SRE agent is a controller acting on a partially observed, delayed, changing system. Measure stability, not just whether the final state is green: action churn, reversals, settling time, overshoot, retry amplification, queue behavior, collateral SLO impact and recovery after intervention stops.

PR #243's deterministic reference plant is an initial control-theoretic conformance model. It is not a production simulator. It should be extended with calibrated workload distributions, delayed actuators, multiple services, capacity coupling, retries, competing controllers and model-predictive or runbook baselines.

### 5. From token cost to cost per independently verified outcome

Report total tokens, model/inference cost, tools, infrastructure, human-active time, rework and time to sustained restoration. The economically meaningful denominator is **independently verified sustained recoveries**, not tool calls or self-reported completions.

## Benchmark portfolio and fair comparison

| Track | Primary question | Use | Comparison rule |
|---|---|---|---|
| SREGym-Lite | Can the runner integrate and complete known SRE tasks? | Smoke and compatibility | Keep as a separate 17-fault cohort; ceiling effects make it a poor frontier ranking. |
| SREGym full | Can agents diagnose and mitigate its full native task set? | Native SRE task comparison | Pin exact source, submodule, all images, runtime/cluster, scorer, agents and budgets. Use native scoring unchanged. |
| SRE-Marathon | Can agents operate continuously through overlapping faults and changes? | Long-horizon control benchmark | Reproduce exact upstream protocol and scoring before making any comparative claim. |
| EnterpriseOps-Gym / ITBench | Can agents complete enterprise workflows and attribute root causes? | Enterprise task and diagnosis coverage | Compare only exact shared tasks/permissions/budgets; never translate one score into another. |
| AgentRx | Can the system localize why its own execution failed? | Trajectory diagnosis sidecar | Use auditable constraints and human-adjudicated labels; keep this distinct from operational success. |
| Harness-evolution tracks | Does operational competence survive changes in tools and skills? | Continual capability retention | Freeze model and tasks; hash each harness version; disclose all adaptations and regressions. |
| Sealed OpsBench assurance | Did an authorized action really occur and did service recover? | Luminous differentiation | Keep evaluator truth hidden; use participant and independent service evidence; score separately from native benchmarks. |

## Proposed target ladder

These are **research hypotheses**, not measured results or acceptance waivers. Confirmatory thresholds must be preregistered and powered.

| Dimension | Initial qualification target | Stretch target |
|---|---:|---:|
| Native SREGym full E2E | At least +5 percentage points vs the strongest matched baseline, with cluster-level interval | +10 pp |
| Continuous SRE-Marathon | Reproduce upstream protocol first; then exceed the strongest matched reference by +5 absolute score points | +10 points |
| Unauthorized / duplicate committed effects | Zero in canonical positive and negative controls | Zero across the entire confirmatory corpus |
| Credited repairs with independent participant + service evidence | 100% | 100% |
| First critical-breakpoint localization | At least 90% exact or within one event on adjudicated trajectories | 95% |
| Non-ambiguous failure attribution | Macro-F1 at least 0.90 | 0.95 |
| Confidence calibration | ECE at most 0.05 | 0.03 |
| Degraded observability | Retain at least 90% of full-observability verified-success rate at 25% loss for a preregistered loss model; abstain safely at severe loss | Improve selective risk/coverage frontier |
| Harness expansion | No more than 2 pp retention loss on frozen tasks, absent preregistered trade-off | Positive transfer with zero critical-safety regression |
| Human-active time / fully loaded cost | 30% / 20% reduction per verified sustained recovery | 50% / 40% |

## Statistical and integrity contract

- Scenario/seed/topology clusters, not individual tool calls, are the inference unit.
- Use matched initial snapshots, fault schedule, workload, permissions, cadence and budgets.
- Randomize arm order; preregister seeds, strata, MDE/power, stopping rules, multiplicity and missing-run policy.
- Preserve every started run in denominators. Timeouts, crashes, missing evidence and evaluator failures remain separately visible.
- Use paired counterfactual twins: full vs degraded telemetry; no-action vs intervention; fixed vs evolved harness.
- Publish per-family results and intervals, worst-family behavior, calibration and risk-coverage; do not let aggregate improvements conceal safety regressions.
- Keep sealed confirmatory faults, topologies and changes disjoint from development and public task material.
- Report native benchmark axes and Luminous assurance metrics separately. No composite score may compensate for a critical unauthorized or duplicate effect.

## Current engineering evidence and claim ceiling

- PR #241: temporal topology, freshness and causal-hypothesis evaluator; exact-head test/Clippy evidence recorded in the PR.
- PR #242: participant-side topology epoch compare-and-commit reference; in-memory concurrency tests only.
- PR #243: causal next-tick controller semantics, streaming/replay parity, exact decision-observation/action binding, and deterministic autoscaler baselines. Synthetic conformance only; no live LLM/cloud qualification.
- PR #244: SREGym source identity and 217-ID built-in selection snapshot; manifest deliberately blocked. No benchmark execution or score.
- PR #245: participant act-bound finality reference; in-memory E0 evidence only, no durable participant transaction or production cryptographic verifier.

**The competitive objective is not to claim the highest public benchmark number. It is to demonstrate an independently reproducible operational episode in which diagnosis, authorized intervention, participant-confirmed effect, and sustained customer-facing recovery all hold under matched, evolving and partially observed conditions.**
