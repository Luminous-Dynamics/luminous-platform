# ITOPS industry alignment snapshot — 2026-11-03

This note records the external evaluation surfaces used to design the ITOPS Autonomy Boundary Benchmark v0.1. It is a dated comparison, not a leaderboard claim. External scores are not directly comparable unless the task version, tool environment, verifier, run policy, and metric are identical.

## Current external baselines

### EnterpriseOps-Gym and EnterpriseOps-Gym-AA

ServiceNow Research's EnterpriseOps-Gym contains 1,150 expert-curated stateful tasks across eight enterprise domains, with 164 database tables and 512 functional tools. Its current repository documentation reports 9.15 average steps per task (up to 34), 5.3 average verification conditions, and approximately 89k average context tokens. The 2026 PMLR paper reports 34.1% success for Claude Sonnet 4.5 in its original evaluation and discusses failures in planning consistency, error recovery, and policy adherence.

**ServiceNow NOWAI-Bench snapshot:** the [NOWAI-Bench README](https://github.com/ServiceNow/NOWAI-Bench) says its synchronized EnterpriseOps-Gym leaderboard snapshot is dated 23 July 2026 and lists Claude Fable 5 (with fallback) at 51.1%, Gemini 3.5 Flash at 50.1%, Muse Spark 1.1 at 47.2%, GPT-5.5 at 46.6%, and Kimi K3 at 45.3%. The note says the live leaderboard contains the full, always-current rankings; the README table is a dated snapshot, not a live query.

**Artificial Analysis EnterpriseOps-Gym-AA, current page read 3 November 2026:** the independent evaluation page currently identifies Claude Fable 5 (Max, Opus 4.8 fallback) at 51.1%, Gemini 3.7 Flash at 50.4%, and Gemini 3.5 Flash at 50.1%. Its methodology states that it uses 1,117 oracle-mode tasks, three repeats per task, and strict task success: all final database-state verifiers must pass. This is a separately run evaluation surface and should not be silently combined with ServiceNow's own repository or NOWAI-Bench snapshots.

The older ServiceNow EnterpriseOps-Gym repository README table read on 3 November lists a separate Opus 4.6/Sonnet 4.6 configuration set (45.9%/42.2%). Preserve it as an archived source snapshot only; it is not the current top-line comparator. None of these figures is a controlled longitudinal comparison with the PMLR paper's 34.1% Sonnet 4.5 result. Model versions, runner profiles, dataset revisions, and scoring snapshots differ.

Artificial Analysis' independent EnterpriseOps-Gym-AA run uses a related but separately evaluated task suite and protocol. Its current public page reports a 51.1% leading strict task-success rate. A task passes only when every database-state verifier passes; there is no partial task credit. This is a different evaluation surface and must not be merged with ServiceNow's repository leaderboard.

Sources:
- [EnterpriseOps-Gym PMLR 2026 paper](https://proceedings.mlr.press/v306/malay26a.html)
- [ServiceNow EnterpriseOps-Gym repository and live README leaderboard](https://github.com/ServiceNow/EnterpriseOps-Gym)
- [EnterpriseOps-Gym-AA current evaluation and methodology](https://artificialanalysis.ai/evaluations/enterprise-ops-gym-aa)
- [EnterpriseOps-Gym-AA launch and result interpretation](https://artificialanalysis.ai/articles/announcing-enterpriseops-gym-aa)
- [ServiceNow NOWAI-Bench dated leaderboard snapshot](https://github.com/ServiceNow/NOWAI-Bench)

**What we adopt:** persistent state, long-horizon multi-tool work, executable final-state verification, cross-domain workflows, all-verifiers task-level success, repeated model snapshots, and per-domain reporting.

**What we add:** participant-owned attempt/effect accounting, explicit distinction between dispatched and committed effects, a lock on new attempts while an earlier result is unknown, exact-scope authorization, separately identified control domains, and evidence-bounded human handoff.

### ITBench and ITBench-AA

The original ITBench work covers IT automation in SRE, CISO, and FinOps. Artificial Analysis' independent ITBench-AA currently evaluates 59 Kubernetes incident-diagnosis tasks (40 public and 19 private), with three repeats per task. The leading score reported on its page is 56.2% average precision at full recall. That metric scores root-cause identification on offline incident snapshots; it is not a rate of successful infrastructure remediation or enterprise workflow completion.

Sources:
- [ITBench PMLR 2025 paper](https://proceedings.mlr.press/v267/jha25a.html)
- [ITBench-AA current evaluation and metric definition](https://artificialanalysis.ai/evaluations/itbench-aa)

**What we adopt:** structured incident evidence, clear root-cause targets, repeated runs, and domain-specific diagnostic tasks.

**What we add:** action authorization, side-effect truth, state reconciliation after ambiguous outcomes, post-action service observation, and a hard distinction between an accurate diagnosis and a verified operational change.

### IETF network-operations agent benchmarking draft

On 2 November 2026 the IETF Datatracker published individual Internet-Draft `draft-contreras-bpm-ai-agent-benchmarking-00`. It proposes vendor-independent assessment of network-operations agents, including system-under-test disclosure, task definitions, deterministic and perturbed conditions, performance/effectiveness/accuracy/efficiency/robustness/coverage/security metrics, repeated runs, confidence intervals, and reporting requirements.

This is a work-in-progress Internet-Draft. It is not an RFC, not an endorsed IETF standard, and may change or be replaced.

Source:
- [IETF Datatracker: Benchmarking Methodology for AI Agents in Network Operations, draft-00](https://datatracker.ietf.org/doc/draft-contreras-bpm-ai-agent-benchmarking/)

**What we adopt:** the report dimensions and the requirement to disclose the full SUT configuration, task conditions, perturbations, repetitions, and statistical method.

**What we add:** independent participant-effect evidence, explicit authorization/revocation semantics, no-retry-while-unknown invariants, exact plan binding, and a case-specific evidence claim ceiling.

## Methodology crosswalk

| External methodology dimension | ITOPS implementation | Additional qualification |
|---|---|---|
| System under test | `sut_profile` records agent build, provider/model/version/size, inference parameters, prompt/memory/tool-permission/guardrail digests, persistence scope, and human approval profile | SUT tool-manifest and guardrail digests are cross-checked against run-level connector and policy digests |
| Task definition | 23 candidate cases, including three multi-step cross-domain workflow DAGs | Every workflow step records domain, operation, dependencies, mutation flag, and expected evidence |
| Environment | Disposable synthetic environment profile, per-episode tenant isolation, no production routes | Independent process isolation is still an acceptance requirement for the future runner |
| Effectiveness | Case-specific decision, stage, handoff, effect, and service postcondition audit | A correct explanation cannot substitute for a missing participant-side effect |
| Accuracy | Evidence references, exact target binding, independent observer state | False claims of success are hard evidence-integrity violations |
| Efficiency | Tool-call counts, decision/total tokens, model/connector costs, latency | Report p50/p95 and cost per verified outcome; do not optimize only raw tokens |
| Robustness | Injected unknown outcomes, stale/future observations, conflicting records, prompt/connector threats | Expand to controlled perturbation matrices and repeated runs |
| Security compliance | Hard bounds for unauthorized dispatch, retry while unknown, wrong-scope grants, oracle integrity | Any critical violation disqualifies the arm; it is not averaged away by successful tasks |
| Reproducibility | Source, toolchain, environment, policy, connector, corpus, variant, oracle, run, arm, episode, and seed bindings | SUT profile binds the actual model/configuration; digest equality is enforced where duplicate run fields exist |
| Statistical reporting | Unique `(run_id, episode_id)` aggregation and exact zero-event one-sided 95% upper bound | The 3,000-episode rule assumes independent Bernoulli episodes under one pinned cohort; uniqueness alone does not prove independence |

## Qualification strategy

1. **Reproduce before differentiating.** Where licence and environment permit, run the same public ITBench or EnterpriseOps task version and preserve its native scoring metric. For EnterpriseOps, pin the exact Git commit, dataset split, model identifier/version, prompt, tool mode, orchestrator, concurrency, repeat count and scoring script from the target leaderboard row. The PMLR paper, live ServiceNow README leaderboard, and EnterpriseOps-Gym-AA are distinct result snapshots. Do not compare our 23-case score to any of them as though the task sets and execution conditions were identical.
2. **Instrument without changing task semantics.** In a paired experiment, capture the participant-owned attempt ledger, exact authority decision, effect receipt, independent service observation, and human handoff. The additional instrumentation must not alter the task, action permissions, or success verifier.
3. **Use hard safety gates before utility rankings.** An arm with an unauthorized dispatch, a new attempt while an earlier effect is unknown, an unsubstantiated success claim, or compromised oracle integrity does not qualify, even if its task-completion rate is high.
4. **Measure the utility frontier.** Among qualified arms, compare verified completion, correct refusal/reconciliation, p50/p95 latency, cost per verified outcome, tool calls, and active human review time.
5. **Keep a held-out adversarial set.** Freeze a reviewed training/development corpus separately from private or held-out variants. Publish corpus, variants, policy, SUT and oracle digests for every result.

## Proposed targets — not measured results

- **Critical safety:** zero unauthorized dispatches, zero retries while outcome-unknown, zero unsubstantiated success claims, and zero oracle-integrity failures across at least 3,000 independent negative-control episodes under one pinned configuration.
- **Statistical interpretation:** with zero observed events in 3,000 independent Bernoulli trials, the exact one-sided 95% Clopper–Pearson upper bound is approximately 0.0998%. This is not a zero-risk guarantee and does not justify production deployment.
- **Authorized utility:** target at least 80% verified completion only after the corpus contains at least ten independent positive-action cases with exact authority and preconditions. The current v0.1 catalog is too small to support this as a meaningful utility estimate.
- **Human efficiency:** target at least 30% lower median active human review time than a runbook-only baseline, with no increase in critical safety violations.
- **Reporting completeness:** 100% of measured runs carry complete SUT, tool, policy, environment, oracle, corpus, seed and statistical-method provenance.

## Current limits

The 23-case corpus remains `candidate_not_frozen`. The Rust qualification core and its 38 unit-test sources have not been compiled or executed in the current environment. The effect ledger and observer remain in-process models, not independent services. The control-domain references in the receipt are declared identifiers; they do not prove actual process isolation or trusted attestation. The full runner, independent oracles, agent adapters, external baseline runs, and independent case/oracle review remain tracked in [issue #213](https://github.com/Luminous-Dynamics/luminous-platform/issues/213).

No current SOTA, industry-leading, or production-safety claim is made.
