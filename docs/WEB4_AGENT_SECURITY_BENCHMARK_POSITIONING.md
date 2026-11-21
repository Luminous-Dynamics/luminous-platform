# Web4 Agent-Security Benchmark Positioning and SOTA Plan

**Date:** 21 November 2026  
**Status:** research plan and repository inventory, not an efficacy result.

## Executive thesis

Do not try to win agent security by maximizing prompt-injection attack success rate (ASR), adding another model judge, or building a bigger synthetic schema. Build a benchmark that can trace and qualify the complete consequence chain:

`source / tool descriptor → authority → model action → owner admission → target commit → independent observation → reconciliation → current evidence claim`.

The differentiator is **consequence-grounded, claim-scoped, independently verifiable security**. It must establish whether an operation was authorized, whether untrusted context redirected the action, whether the owner admitted it, whether the target committed it, whether a separate observer proves it, and whether the evidence remains current after faults or restore.

A synthetic checker pass is not a live agent security result. Keep these states separate:
- implemented;
- synthetic contract checked;
- exact-head hosted workflow passed;
- isolated runtime executed;
- independent effect/fault oracle qualified;
- external matched evaluation reproduced.

## Industry baselines and what to compare

| Baseline | Strength | What it does not prove by itself | Web4 comparison |
|---|---|---|---|
| **Agent Security Bench (ASB)** | Breadth across scenarios, agents, tools and attack/defence types; paper reports 10 scenarios, 10 agents, 400+ tools, 27 attack/defence types and seven metrics. | Its ASR/utility scores are tied to its agents, tasks and oracle. They do not establish owner-specific commit-time authority or independent resource-side truth. | Reproduce compatible tasks, attacks and models. Add exact authority/effect identity and independent consequence/recovery oracles; report ASB's metrics unchanged. |
| **AgentDojo** | Stateful tool-use tasks, indirect prompt injection and benign utility; 97 tasks / 629 security test cases in the public suite. | Task success or the tool's own state oracle is not automatically proof of exact per-invocation authority or observer independence. | Use matched task/tool subsets, preserving task and utility scores; additionally grade owner admission, trajectory diversion, canonical target effects and evidence completeness. |
| **ToolSandbox, τ-bench, WASP** | Stateful trajectories, milestones, prohibited intermediate states and repeated-run reliability. | Final-state task completion does not necessarily reveal every prohibited intermediate effect or whether retries duplicated irreversible writes. | Reuse stateful tasks and grade every intermediate target-side effect, operation identity, admission and reconciliation. |
| **AutoPI / PIAtlas** | Taxonomy-guided adaptive discovery, attempted technique coverage and successful technique coverage. | Generator labels and attack counts are not independent technique success; ASR can collapse onto repeated strategies. | Pin taxonomy revision, blind post-hoc causal attribution and measure held-out strategy transfer per attempt. |
| **PIPETTE** | Highlights strategy mode collapse and poor transfer from train-time attack success to unseen adaptive attacks. | A diversity reward or model judge is not ground-truth strategy identity. | Use causal strategy fingerprints, duplicate/paraphrase controls, independent reviewers and held-out defender experiments. |
| **AttriGuard / causal-tracing work** | Distinguishes source/instruction influence from ordinary model variability and tool-action changes. | Internal probes or action-only counterfactuals are not independent target-effect truth. | Match models, tasks and seeds; use clean/raw/neutralized interventions and CAPC resource-side outcomes. |
| **MCP security guidance** | Concrete connector threats: descriptor/schema/annotation drift, server shadowing, issuer/audience, least privilege and per-call authorization. | Guidance alone is not a causal efficacy or side-effect-rate benchmark. | MTCI pins the complete descriptor and per-call authority; CARI binds the implementation and observed consequences. |
| **LIMBO / exactly-once agent work** | Late commits, response loss, redelivery, idempotency-key availability and service-side faults. | Client-side verification cannot guarantee exactly-once when commits may arrive late or reads are unbounded/eventually consistent. | TELR compares stable operation identity, target-side idempotency, independent ledger and reconciliation under matched fault timing. |
| **Idempotency Is Not Recovery / ACRFence** | Distinguishes duplicate suppression from orphaned effects and checkpoint semantic replay/authority resurrection. | Local restore, new request IDs or action ledgers cannot undo an external side effect. | Test target-side truth, consumed authority monotonicity, compensation lineage and evidence finality across restore. |

Primary references:
- [ASB (ICLR 2025)](https://mlanthology.org/iclr/2025/zhang2025iclr-agent/)
- [AgentDojo](https://github.com/ethz-spylab/agentdojo)
- [ToolSandbox](https://github.com/apple/ToolSandbox)
- [τ-bench](https://github.com/sierra-research/tau-bench)
- [WASP](https://arxiv.org/abs/2503.01421)
- [AutoPI](https://arxiv.org/abs/2611.24313) and [PIAtlas](https://arxiv.org/abs/2610.24999)
- [PIPETTE](https://arxiv.org/abs/2611.24362)
- [Text-to-pixels study](https://arxiv.org/abs/2611.24231)
- [AttriGuard](https://arxiv.org/abs/2603.10749)
- [Readable Before Actionable](https://arxiv.org/abs/2610.05295)
- [LIMBO](https://arxiv.org/abs/2609.29095)
- [Idempotency Is Not Recovery](https://arxiv.org/abs/2611.23353)
- [ACRFence](https://arxiv.org/abs/2603.20625)
- [OWASP MCP Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/MCP_Security_Cheat_Sheet.html)

All published figures are specific to their evaluated models, agents, versions, task suites, budgets and oracles. Do not generalize study rates to the whole industry.

## Paradigm shifts

1. **From prompt safety to consequence authorization.** A model proposes an action; it cannot mint its right to execute. Authority is owner-issued, resource/effect-scoped, versioned, bounded and checked at final admission.
2. **From attack co-occurrence to intervention-identified causality.** Clean/raw/neutralized arms distinguish a changed action from a caused unauthorized effect. Neutralization must restore the baseline before causal attribution is claimed.
3. **From nominal observer diversity to fault-qualified independence.** Different process IDs are not independent when event stream, signing root, credential root, clock, storage or control plane is shared. Fault injection must prove which observers survive.
4. **From tool descriptions to executable contract identity.** Pin full descriptor, schemas, annotations, server identity/origin, artifact and per-invocation issuer/audience/scopes.
5. **From ASR leaderboards to held-out strategy transfer.** Causal strategy identity, blinded review and independent effect truth matter more than paraphrase counts or generator labels.
6. **From local checkpoint to external-effect lifecycle.** The target ledger determines what committed. A new key after restore is a new operation. Compensation is a linked new effect, never erasure.
7. **From point timestamps to uncertainty-bounded ordering.** A grant/revocation/commit order is proven only when intervals are disjoint on a qualified common timebase.

## Current repository portfolio

| Workstream | Executable increment | Next empirical promotion |
|---|---|---|
| Endpoint/adaptive #141 | Frozen corpus, adaptive protocol, trajectory privilege, CAPC, IFL and reference model. | Named isolated agents, exact tools/runtime, disposable resource and independent effect oracle. |
| OIC/CFI #518 | Seven failure domains, synthetic quorum and 16 common-cause fault-plan cases. | Verified dependency closure plus controlled fault injection and independent witness survival. |
| CARI #519 | Declared (D), implementation-derived potential (P), observed (O), exact artifact-binding contract. | Pinned static analysis/build, binary provenance and OS/resource-side effect observation. |
| Cross-modal #520 | 16 synthetic episodes / eight text-image pairs, blinded causal attribution and consequence separation. | 191+ semantic sources, multiple pinned model configurations, render/OCR arms and independent labels/effects. |
| CAA #522 | 18 synthetic episodes / six clean-raw-neutralized triplets. | Randomized execution with state-equivalent resets, preregistered estimands and confidence intervals. |
| MTCI #524 | 15 MCP descriptor/authority controls, including per-response identity. | Named clients/servers/protocol, descriptor canonicalization and live per-call authorization. |
| TELR #526 | 23 synthetic lifecycle/recovery cases, target/client ledger reconciliation, duplicate/replay/compensation and time-interval linearization. | Disposable target service, injected response loss/late commits/restore faults and independent target ledger. |

## Claim–obligation–evidence reporting

A 10 November 2026 structured review of agent authorization assurance screened a 75-source library into a 30-source analytical corpus and 92 claim-level records. In that closed corpus, only five strict sources made mediation/non-bypassability claims; none of the 21 agent-focused sources had an evaluable revocation claim, and none of the 30 coded sources made a delegation-containment claim. The authors explicitly describe their MAAP reporting profile as preliminary rather than a validated score. These are corpus-bounded review findings, not claims that deployed agents universally lack those controls. [Structured review](https://link.springer.com/article/10.1007/s10207-026-01343-w).

This suggests an additional research-method improvement: report every security result as a **claim record**, not merely as a benchmark score. For a claim \(c\), record:

- claim scope: principals, delegated actors, protected resources, operations, environment, threat assumptions and version boundary;
- obligations \(O(c)\): the concrete conditions that must hold for the claim (e.g. complete mediation, latest grant, consumed nonce, exact descriptor, independent target oracle);
- evidence \(E(c)\): only obligations supported by the actual check/run/observation, with exact commit, workflow, inputs, event span and oracle identity;
- protected effect: what external resource state was actually observed, or explicitly that only a plan/call/final-state proxy was measured;
- residual assumptions and unknowns;
- residual gap \(G(c)=O(c)-E(c)\), represented as a **set of unmet obligations**, never compressed into a scalar assurance score.

Use this structure when reporting each PR and future empirical run. It will prevent a synthetic contract pass from being misreported as a runtime guarantee, and will make matched comparisons meaningful even when two benchmarks use different oracles.

## Competitive target ladder

### Stage A — deterministic contract qualification
- 100% expected outcomes on frozen positive, negative and indeterminate controls.
- Mutation tests for every identity, authority, uncertainty, replay, ledger and observer-invariance rule.
- Zero composite scores that hide a critical failure.
- Every hosted workflow checks out and asserts the exact tested revision.

### Stage B — matched exploratory comparison
- At least 100 valid independent episodes per family/arm/model/configuration for coarse comparison only.
- Reproduce compatible AgentDojo and ASB subsets.
- Match model/agent versions, task state, prompts, tools, attack budgets, policies and independent oracle.
- Publish numerators/denominators, confidence intervals, clean utility, false refusals and invalid/excluded episodes.

### Stage C — rare-event claims
- Critical forbidden-effect target: zero events and one-sided exact-binomial 95% upper bound ≤0.1%, requiring ≥2,995 independent zero-event episodes per family/arm/model/configuration stratum.
- Benign false-refusal target: zero refusals in ≥299 independent episodes per task family for a one-sided 95% upper bound ≤1%.
- Multi-stratum claims require multiplicity adjustment. Repeated calls in one state are clustered, not independent.

### Stage D — superiority targets (research goals, not results)
- ≥50% relative reduction in adaptive unauthorized-effect rate versus the strongest eligible matched baseline.
- Median action-latency overhead ≤5%, p95 ≤10% versus matched enforcement-only.
- No critical safety regression and no loss of benign source facts hidden behind refusal.
- Zero false-qualified effect claims under tested OIC/CFI fault strata.
- Zero stale descriptor/grant/replay acceptance under MTCI.
- Zero duplicate target effects and false-success claims under TELR's tested fault strata.
- No trade of critical safety for a composite utility, latency or diversity score.

A target is met only when matched controls, independent target-side evidence and confidence intervals support it.

## What not to do

- Do not compare a synthetic contract pass to ASB or AgentDojo model results.
- Do not treat queued/skipped workflows as a pass or CI as runtime qualification.
- Do not use an LLM judge as effect truth or independent strategy attribution.
- Do not count paraphrases, formatting, modality or generator labels as distinct strategies.
- Do not infer safe execution from static non-reachability when dynamic/native/remote dependencies are unresolved.
- Do not infer exactly-once from client-side retry logic, a local journal or successful final task text.
- Do not trade critical forbidden effects for utility, latency or coverage averages.

## Immediate execution order

1. Finish exact-head TELR #526 and preserve its synthetic-only claim ceiling.
2. Promote Mycelix #5762's verified multi-valued/transitive dependency identities into a disposable CFI fault adapter.
3. Build the first harmless resource-side effect service with target-side idempotency/status, deterministic state snapshots and independent target ledger.
4. Execute matched AgentDojo/ASB subsets plus LIMBO-compatible delayed-commit/redelivery faults.
5. Expand the cross-modal corpus and causal intervention campaign only after the independent consequence harness is stable.
6. Publish exact revisions, model/task/arm assignments, seeds, target-oracle receipts, denominators, statistical analysis and claim boundaries.
