# ITOPS industry and research benchmark crosswalk — 2026-11-06

This is a dated source snapshot checked on 6 November 2026. It separates benchmark implementations, task sets, runners, model configurations, and scoring surfaces. A leaderboard number is not portable between implementations merely because they reuse the same benchmark name.

## Enterprise operations: three distinct result surfaces

| Evaluation surface | Current result visible in source checked 2026-11-06 | What it means | Comparability warning |
|---|---:|---|---|
| ServiceNow Research, EnterpriseOps-Gym repository README | Claude Opus 4.6: 45.9%; Claude Sonnet 4.6: 42.2%; Gemini 3.1 Pro: 38.7% | Oracle-mode strict success on the official 1,150-task, eight-domain benchmark; task success requires all verification conditions | This is the ServiceNow repository's displayed leaderboard/configuration, not a longitudinal comparison with the original paper |
| Artificial Analysis, EnterpriseOps-Gym-AA | Claude Fable 5 (Max, Opus 4.8 fallback): 51.1%; Gemini 3.7 Flash: 50.4%; Gemini 3.5 Flash: 50.1% | Independent implementation of the benchmark, 1,117 oracle-mode tasks, strict final database-state success | Separate implementation, execution harness and evaluation surface; do not splice into the ServiceNow leaderboard |
| ServiceNow NOWAI-Bench README snapshot | Claude Fable 5 (with fallback): 51.1%; Gemini 3.5 Flash: 50.1%; Muse Spark 1.1: 47.2%; GPT-5.5: 46.6%; Kimi K3: 45.3% | A synchronized leaderboard snapshot explicitly dated 23 July 2026 | Historical snapshot only; its own README points readers to live leaderboards for current rankings |
| Original EnterpriseOps-Gym PMLR 2026 paper result | Claude Sonnet 4.5: 34.1% | Result reported in the original paper | Different model snapshot and experimental setup; not a controlled comparison to the current rows above |

The official EnterpriseOps-Gym benchmark includes 1,150 expert-curated stateful tasks across eight enterprise domains, 164 database tables and 512 functional tools. The repository README's table and the AA leaderboard are strict whole-task results: a task succeeds only when its required final-state verifiers pass. Per-verifier averages are diagnostic metrics and must not be reported as strict task success.

Sources checked:
- [ServiceNow Research — EnterpriseOps-Gym repository and leaderboard](https://github.com/ServiceNow/EnterpriseOps-Gym)
- [Artificial Analysis — EnterpriseOps-Gym-AA leaderboard and methodology](https://artificialanalysis.ai/evaluations/enterprise-ops-gym-aa)
- [Artificial Analysis — EnterpriseOps-Gym-AA announcement and result interpretation](https://artificialanalysis.ai/articles/announcing-enterpriseops-gym-aa)
- [ServiceNow — NOWAI-Bench dated snapshot](https://github.com/ServiceNow/NOWAI-Bench)
- [Original EnterpriseOps-Gym PMLR 2026 paper](https://proceedings.mlr.press/v306/malay26a.html)

## Continuous site reliability and harness evolution

**SRE-Marathon** is a complementary continuous-operation benchmark, not another EnterpriseOps score. Its September 2026 paper describes a two-zone Kubernetes deployment, persistent agent workspace, fixed-cadence invocations, overlapping seeded faults and code/configuration changes through the same repair pipeline. Across three applications and roughly sixty faults per run, the best of ten methods reports 41.3/100. The authors identify a gap between correlation/localization and completing repairs while faults remain active.

Source: [SRE-Marathon (arXiv:2609.33023)](https://arxiv.org/abs/2609.33023).

**EVOHARNESSBENCH** evaluates agents as tools, skills and specialist agents change. Its September 2026 paper reports 17 multi-stage streams, 802 tasks, 520 tools, 42 skills and 62 agents. It finds harness expansion can degrade previously accessible competence and that retention and adaptation are not automatically aligned.

Source: [EVOHARNESSBENCH (arXiv:2609.04280)](https://arxiv.org/abs/2609.04280).

These benchmarks imply that a credible operational agent must be tested across time, changing faults, changing tools and persistent memory—not only on isolated tickets.

## Standards and problem statements

The November 2026 AINETOps problem-statement Internet-Draft describes fragmented cross-agent telemetry, loss of task context across multi-agent chains, and the lack of standardized semantics for delegated scope. It explicitly identifies the need for downstream permissions to narrow rather than expand across a chain. The separate November 2026 AI-agent benchmarking Internet-Draft proposes SUT disclosure, task/condition definitions, security-compliance metrics, repeated runs and statistical reporting.

These are individual Internet-Drafts, not adopted IETF standards or normative requirements:
- [AINETOps Problem Statement, draft-rqd-ainetops-problem-statement-00](https://datatracker.ietf.org/doc/html/draft-rqd-ainetops-problem-statement-00)
- [Benchmarking Methodology for AI Agents in Network Operations, draft-contreras-bpm-ai-agent-benchmarking-00](https://datatracker.ietf.org/doc/draft-contreras-bpm-ai-agent-benchmarking/)

## What Luminous should measure—and what it should not claim

Do not claim that the delegation-narrowing principle itself is novel; the AINETOps draft already identifies it. The research contribution to test is whether the rule can be made executable and enforced at the final participant boundary across nested task/plan digests, tenant/target/operation/parameter/data constraints, expiry, budgets, revocation, risk, approvals, rollback and postconditions—with source lineage and independently observed effects.

Keep these result surfaces separate:

1. **Native task performance:** run the public EnterpriseOps-Gym task set under a pinned official-compatible setup and preserve its native final-state scorer. Report per-domain success, strict whole-task success, repeats, confidence intervals, and resource use.
2. **Operational evidence integrity:** report unauthorized dispatches, committed effects, retries while outcome is unknown, effect/observer agreement, stale observations, false success claims and claim-ceiling violations. These are hard qualification gates, not score bonuses.
3. **Delegation conformance:** report valid delegation acceptance and completion, parent-to-child expansion attempts detected, final participant denials, revoked-ancestor rejection, lineage completeness and evidence-laundering detection. Include positive controls so blanket refusal cannot pass.
4. **Continuous operations:** reproduce SRE-Marathon under its native protocol before any comparison; report fault detection, localization, repair-before-cleanup, sustained recovery, workload impact and collateral effects.
5. **Harness evolution:** reproduce EVOHARNESSBENCH's retention and adaptation setting; report old-task retention, new capability acquisition, regressions, forgetting and safety-gate invariance across harness revisions.

## Qualification targets (proposed, not measured)

- Zero unauthorized committed effects and zero new mutation attempts while a prior effect outcome is unknown.
- Zero false success claims when participant effect or independent service postcondition is missing, stale, conflicting or unqualified.
- 100% rejection of known delegated scope expansions and 100% acceptance of valid positive controls in the frozen conformance corpus.
- Every delegated effect binds to root task, exact parent/child task and plan digests, current authority/revocation state, the final participant attempt, and the evidence lineage.
- No duplicated, mirrored, re-signed or summarized observation receives independence credit without independent source evidence.
- Native strict EnterpriseOps-Gym success is first reproduced under matched conditions; only then target a preregistered improvement over the strongest matched baseline, with confidence intervals and unchanged task semantics.
- For SRE-Marathon, first reproduce the 41.3/100 paper-reported baseline under the native protocol. A >50/100 target is aspirational only after replication and must not trade away the hard safety gates.
- For harness evolution, target no regression on the predeclared safety-critical retention set while demonstrating measurable adaptation on the new-capability set.

These are research targets, not observed results, an assurance case for production, or permission to deploy. The v0.1 ITOPS corpus remains a candidate set; the synthetic qualification core is not a complete agent runner or an independent hosted oracle. No external baseline has yet been reproduced by Luminous.
