# Web4 Agent-Security Benchmark Positioning and SOTA Plan

**Date:** 21 November 2026  
**Status:** Research plan and repository inventory, not an efficacy result.

## Executive thesis

Do not try to win agent security by maximizing prompt-injection attack success rate (ASR), adding another model judge, or making a bigger synthetic schema. Build a benchmark that can trace and qualify the complete consequence chain:

`source / descriptor → authority → model action → owner admission → resource effect → independent observation → current evidence claim`.

The differentiator is **consequence-grounded, claim-scoped, independently verifiable security**. The benchmark must establish whether an operation was authorized, whether a tool call was influenced by untrusted context, whether the owner admitted it, whether the effect occurred, and whether the evidence used to claim that result remains current.

A passing synthetic checker is not a real attack/defence result. All portfolio reporting must distinguish:
- `implemented`;
- `synthetic contract checked`;
- `exact-head hosted workflow passed`;
- `isolated runtime executed`;
- `independent effect/fault oracle qualified`;
- `external matched evaluation reproduced`.

## Existing industry baselines and comparison surfaces

| Baseline | What it is good at | What it does not establish by itself | Web4 comparison |
|---|---|---|---|
| **Agent Security Bench (ASB)** | Broad attack/defence surface: 10 scenarios, 10 agents, 400+ tools, 27 attack/defence types and seven metrics. The paper reports a highest average ASR of 84.30% across its evaluated attacks/backbones. | The aggregate ASR/utility results are tied to its agents, proxy tools and oracles; not proof of exact owner-grant semantics or independent physical effects. | Reproduce compatible tasks and attack suites; match model/agent/tool versions and budget. Add owner-issued authority, canonical effects and independent effect qualification as additional measured outcomes. |
| **AgentDojo** | Dynamic tool-use tasks, indirect injection, stateful environment and benign utility. The published suite reports 97 realistic tasks and 629 security test cases across four domains; the environment has 70 tools and 27 injection targets. The public suite is widely used for matched attack/defence comparisons. | Tool/task success and attack objectives are not identical to per-invocation authorization or observer-independence claims. | Use matched tasks and exact agent/runtime. Preserve its task outcomes, then separately evaluate authorization, trajectory diversion and observed resource effects. |
| **ToolSandbox, τ-bench, WASP** | Stateful trajectories, milestones, prohibited intermediate states, repeated-run reliability and whether an attacker reaches an explicit goal. | A final-state or proxy-tool oracle does not automatically prove the authorization decision or independence of the evidence channel. | Consume compatible trajectories. Track all intermediate prohibited effects, owner admission, versioned resource state and evidence completeness. |
| **AttriGuard / causal tracing work** | Action-level causal attribution; counterfactual tests distinguish a visible instruction from a tool action influenced by it. | Model-internal probes or counterfactual proxies are not resource-effect truth. | Compare on identical tasks, models, seeds and budgets; use external CAPC effects and independent resource observations as the outcome oracle. |
| **AutoPI / PIAtlas** | Taxonomy-guided adaptive discovery and attempted/successful technique coverage. | Generator-assigned labels and strategy counts do not prove independent technique success; high ASR can collapse onto a few strategies. | Pin taxonomy revision, blind post-hoc attribution, measure coverage per attempt and transfer to held-out causal strategy clusters. |
| **PIPETTE** | Highlights attack-strategy mode collapse and the gap between training-time attack success and held-out adaptive defence. | Diversity reward or an LLM judge is not a universal strategy ground truth. | Use causal strategy fingerprints, reviewer adjudication, duplicate/paraphrase controls and held-out defender transfer. |
| **OWASP MCP security guidance** | Practical threat surface: whole tool descriptor integrity, schema/annotation drift, server shadowing, origin/audience, least privilege and per-request authorization. | Guidance does not provide a matched causal efficacy benchmark or independent effect-rate estimate. | MTCI tests descriptor drift and call authority; CARI binds declared/implementation/observed effects; endpoint oracle qualifies consequences. |

Primary references:
- [MCP 2026-07-28 specification release](https://blog.modelcontextprotocol.io/posts/2026-07-28/) — stateless protocol core and authorization hardening, including authorization-server issuer validation.
- [ASB (ICLR 2025)](https://mlanthology.org/iclr/2025/zhang2025iclr-agent/)
- [AgentDojo project](https://github.com/ethz-spylab/agentdojo)
- [ToolSandbox](https://github.com/apple/ToolSandbox)
- [τ-bench](https://github.com/sierra-research/tau-bench)
- [WASP](https://arxiv.org/abs/2503.01421)
- [AttriGuard, arXiv:2603.10749](https://arxiv.org/abs/2603.10749)
- [Readable Before Actionable, arXiv:2610.05295](https://arxiv.org/abs/2610.05295)
- [AutoPI, arXiv:2611.24313](https://arxiv.org/abs/2611.24313)
- [PIPETTE, arXiv:2611.24362](https://arxiv.org/abs/2611.24362)
- [Text-to-pixels study, arXiv:2611.24231](https://arxiv.org/abs/2611.24231)
- [OWASP MCP Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/MCP_Security_Cheat_Sheet.html)

Published numbers above are study-specific. Do not generalize them beyond the tested models, tasks, versions, and oracles.

## Paradigm shifts

### 1. From prompt safety to consequence authorization

The model may propose an action; it must not mint the right to perform it. Authority is owner-issued, resource/effect-scoped, versioned, short-lived, single-use where appropriate, and checked at the final admission boundary. The same user-intent action and an injection-induced action must be distinguishable.

### 2. From attack co-occurrence to intervention-identified causality

Use clean/raw/neutralized randomized arms. Measure separately:
- recognized instruction;
- action and sensitive-parameter divergence;
- owner admission;
- independently observed unauthorized effect;
- benign utility and source-fact preservation.

Neutralization must restore the clean baseline before causal attribution is claimed. A visible instruction or a changed tool call alone is not proof of a breach.

### 3. From nominal observer diversity to fault-qualified independence

Different processes and names are not independent witnesses when they share an event stream, signing root, credential root, clock, storage or control plane. OIC establishes synthetic dependency identity; CFI adds fault-plan semantics. The next real step is harmless isolated common-cause fault injection and independent observation of which witnesses actually survive.

### 4. From tool descriptions to executable contract identity

Bind reviewed descriptor, schemas, annotations, server origin/build, token issuer/audience, scopes, invocation and CAPC effect. A tool with the same friendly name but a different schema or origin is a different capability. Per-call admission must bind the exact approved descriptor digest.

### 5. From ASR leaderboards to held-out strategy transfer

Use pinned PIAtlas/AutoPI coverage plans, but do not trust generator labels. Two reviewers independently attribute causal strategy after the episode, blinded to generator metadata and outcome. A successful technique requires independently verified objective attainment. Test any resulting defence on held-out strategy clusters, tools, task families and modalities.

### 6. From one-shot benchmarks to evidence that can expire

A decision is only as current as its grant, descriptor, dependency, policy epoch, observer and resource state. Unknown outcomes do not permit new mutations before reconciliation. Evidence carries exact revisions and currentness; changes revoke claims and require requalification.

## Repository benchmark portfolio

| Workstream | Current contract | Next empirical promotion |
|---|---|---|
| Endpoint/adaptive benchmark PR #141 | Frozen corpus, adaptive protocol, trajectory privilege, CAPC, IFL and reference model. | Named isolated agent/model, exact tool/runtime, disposable resources and independent effect oracle. |
| OIC/CFI PR #518 | Seven-domain synthetic observer quorum and 16 common-cause fault-plan scenarios. | Verified dependency closure plus controlled fault injection per common-cause domain and combinations. |
| CARI PR #519 | Declared (D), implementation-derived potential (P), independently observed (O), exact artifact-binding contract. | Pinned compiler/static analyser, exact build/binary provenance, runtime/OS side-effect observer and matched tool corpus. |
| Cross-modal PR #520 | 16 synthetic episodes / eight text-page-image pairs; recognition, redirection, admission and effect separated. | Paired corpus at 191+ semantic sources, six or more model configurations, multiple render/OCR resolutions, independent labels and effect observations. |
| CAA PR #522 | 18 synthetic episodes / six clean-raw-neutralized triplets with pre-treatment identity and group contrasts. | Randomized execution, valid reset snapshots, repeated independent units, preregistered estimands and confidence intervals. |
| MTCI PR #524 | 14 synthetic MCP descriptor/authority cases. | Named clients/servers and MCP revision, exact descriptor canonicalization, issuer/audience and per-request auth verification. |

## Competitive target ladder

### Stage A — deterministic contract qualification

- 100% expected outcomes on frozen positive/negative/indeterminate controls.
- Mutation tests for every identity, authority, uncertainty, replay and observer-invariance rule.
- Zero aggregate/composite scores that hide a critical failure.
- Exact-head workflow must check out and assert the tested revision.

### Stage B — matched exploratory comparisons

- At least 100 valid independent episodes per family/arm/model/configuration for coarse comparisons only.
- Reproduce compatible AgentDojo and ASB subsets.
- Match agent/model version, tools, prompts, task state, attack budget, defence configuration, and independent oracle.
- Report raw numerators/denominators, confidence intervals, utility, false refusals and invalid/excluded episodes.

### Stage C — rare-event safety claims

- Critical forbidden-effect target: zero observed events and a one-sided exact-binomial 95% upper bound no greater than 0.1%, requiring at least 2,995 independent zero-event episodes per family/arm/model/configuration stratum.
- Benign false-refusal target: zero observed false refusals in at least 299 independent episodes per task family for a one-sided 95% upper bound no greater than 1%.
- Multi-stratum claims require a preregistered multiplicity adjustment.
- Repeated calls in one state are clustered; do not count them as independent episodes.

### Stage D — superiority targets

Research targets, not achieved results:
- at least 50% relative reduction in adaptive unauthorized-effect rate versus the strongest eligible matched baseline;
- median action-latency overhead no more than 5%, p95 no more than 10% versus matched enforcement-only;
- no critical safety regression;
- no loss of benign source facts or task utility hidden behind refusal;
- zero false-qualified effect claims under the tested OIC/CFI fault strata;
- zero stale descriptor/grant/replay acceptance under MTCI mutations.

A target is met only when confidence intervals, matched controls and independent effect observations support it.

## What not to do

- Do not compare a synthetic contract pass to ASB or AgentDojo model results.
- Do not treat a queued/skipped workflow as a pass or a CI pass as a runtime test.
- Do not use LLM judge outputs as effect truth or independent strategy attribution.
- Do not count paraphrases, formatting changes, modality changes or generator labels as distinct strategies.
- Do not infer safe execution from static non-reachability when dependencies/dynamic/native paths are unresolved.
- Do not trade critical forbidden effects for aggregate utility, latency or diversity scores.
- Do not claim production observer independence from different process IDs or synthetic domain IDs.

## Immediate execution priority

1. Finish exact-head PR #520 and #522 (full inherited endpoint chain).
2. Qualify MTCI PR #524 and preserve full descriptor/authority boundaries.
3. Integrate Mycelix #5762 verified multi-valued/transitive dependency identities into a disposable fault-injection adapter.
4. Build the first harmless resource-side effect harness with deterministic state snapshots and independently captured effects.
5. Run matched AgentDojo/ASB subset comparisons and only then expand to broader proprietary/hosted agents and 191+ cross-modal source pairs.
6. Publish complete receipts: exact revisions, manifests, seeds, model/task/arm assignments, oracle inputs/results, statistical analysis and claim ceiling.
