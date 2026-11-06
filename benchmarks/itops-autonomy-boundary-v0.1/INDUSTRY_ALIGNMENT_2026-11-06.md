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

### EnterpriseOps-Gym-AA leaderboard snapshot (5 November 2026)

A secondary aggregator, [BenchLeader's EnterpriseOps-Gym page](https://www.benchleader.com/benchmarks/aa_enterprise_ops_gym), reports the Artificial Analysis public leaderboard as of 5 November 2026, with 50 model configurations: Claude Fable 5 (max) 51.1%, Gemini 3.7 Flash 50.4%, Gemini 3.5 Flash 50.1%, DeepSeek V4 Pro 49.6%, Grok 4.6 48.3%, Qwen3.8 Max 47.6%, and Claude Opus 5 47.5%. It states the listed benchmark result is sourced from Artificial Analysis; this is an aggregator view of that evaluation, not an independent re-run. Scores are model results, not agent-platform/system guarantees.

Keep it distinct from ServiceNow's own repository leaderboard snapshot (Opus 4.6 at 45.9% in the README source checked earlier), NOWAI-Bench's 23 July historical snapshot, and the original PMLR result (Sonnet 4.5 at 34.1%). These are different models, snapshots and execution surfaces; do not interpret the rows as a controlled longitudinal series.

Source: [BenchLeader, EnterpriseOps-Gym leaderboard, data as of 5 November 2026](https://www.benchleader.com/benchmarks/aa_enterprise_ops_gym).

## Additional frontier benchmark surfaces — checked 6 November 2026

### EnterpriseOps-Gym-AA: strict end-state task success

Artificial Analysis' live page reports 27 of 51 model configurations with current displayed results. Its highest scores are Claude Fable 5 (Max, Opus 4.8 fallback) 51.1%, Gemini 3.7 Flash (Medium) 50.4%, and Gemini 3.5 Flash (High) 50.1%. Its methodology uses three repeats per task, oracle tool mode, and SQL verification of the final underlying database state; a task earns no credit unless every verifier passes. It also exposes cost per task, output tokens and domain-level performance.

This remains the closest broad industry comparator for Luminous's enterprise workflows. But it is a **model+harness result**, not a direct comparison of operation-control architectures. Luminous should use the same model/configuration and task set in a matched baseline to estimate the architecture's incremental effect, and must preserve native task semantics.

Source: [Artificial Analysis — EnterpriseOps-Gym-AA](https://artificialanalysis.ai/evaluations/enterprise-ops-gym-aa).

### ITBench-AA: SRE diagnosis, not remediation

Artificial Analysis' ITBench-AA evaluates 59 Kubernetes incident tasks (40 public and 19 private) with three repeats. Agents inspect offline incident snapshots containing alerts, events, traces, metrics and topology, and identify the root-cause entities. The headline metric is **average precision at full recall**, not task-completion percentage. The current displayed leaders are GPT-5.6 Sol (Max) at 56.2%, Step 5 Preview at 55.6%, and Gemini 3.8 Flash (High) at 52.5%.

The broader original ITBench framework covers 94 real-world scenarios across SRE, CISO and FinOps; its paper abstract reports best agent resolution rates of 13.8% for SRE, 25.2% for CISO and 0% for FinOps. Those resolution figures are distinct from the ITBench-AA root-cause precision leaderboard and must not be substituted for it.

**Luminous implication:** add a diagnostic arm that scores root-cause precision/recall separately from action authorization, participant effects, and verified repair. A correct diagnosis is not a safe or completed remediation.

Sources: [Artificial Analysis — ITBench-AA](https://artificialanalysis.ai/evaluations/itbench-aa); [ITBench paper and benchmark hub](https://github.com/itbench-hub/ITBench).

### APEX-Agents: long-horizon professional workflows

Mercor's APEX-Agents currently lists 31 professional-work worlds and 240 tasks across corporate law, management consulting and investment banking. Its tasks require multi-step work across applications and are graded against expert-authored rubrics with an LM judge. The displayed Pass@1 leaderboard on 6 November 2026 shows Gemini 4 Carbon High at 84.2% ±4.3%, Gemini 4 Argon High at 82.4% ±4.3%, and Hy4 High at 77.6% ±4.5%. The benchmark exposes both Pass@1 (perfect rubric completion on one attempt) and mean-score reporting; these are not interchangeable.

This is adjacent evidence for long-horizon cross-application capability—not IT operations or participant-level safety. Use the open subset to evaluate persistence, handoffs, tool/skill evolution and cost/time tradeoffs; do not compare its score directly with EnterpriseOps-Gym.

Source: [Mercor — APEX-Agents leaderboard](https://www.mercor.com/apex/apex-agents-leaderboard/).

### AutomationBench-AA: partial credit must remain separate

BenchLeader's 4 November 2026 snapshot reports Gemini 4 Carbon (High) at 77.8% on AutomationBench-AA. The benchmark gives partial credit for objective completion. That is useful for diagnosing incremental progress through a workflow, but it cannot be compared with EnterpriseOps-Gym's all-verifiers-must-pass strict success rate. A partially completed change with an unsafe side effect must not become a successful operational outcome in Luminous's qualification model.

Source: [BenchLeader — AutomationBench leaderboard](https://www.benchleader.com/benchmarks/aa_automationbench).

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

## Runtime-enforcement research landscape

### Beyond Guardrails: Systematic Review of Runtime Enforcement for Secure AI Agents

The review [Beyond Guardrails: Runtime Enforcement for Secure AI Agents](https://doi.org/10.62762/JRSC.2026.526903), published 17 October 2026 in the *Journal of Reliable and Secure Computing*, screened and coded 58 primary studies published or publicly available from January 2023 through September 2026. It reports:

- 33/58 studies (56.9%) enforce primarily at the tool/action boundary;
- 18/58 (31.0%) enforce inside the agent runtime or harness;
- only 7/58 (12.1%) focus primarily on OS/process/execution boundaries;
- authorization/privilege control appears in 41/58 (70.7%) and action validation in 34/58 (58.6%).

The review identifies durable authorization, multi-agent delegation, cross-layer enforcement, recovery from partially committed effects, and standardized evaluation as open problems. Its common architectural pattern is a model-proposed action separated from an independently enforceable decision point before external effect.

**Luminous implication:** the credible research opportunity is not another natural-language guardrail or an additional policy wrapper. It is a cross-layer authority/effect chain in which Xenia supplies current canonical authority, a participant enforces the exact request at dispatch, Nixward/host boundaries constrain execution, participant-owned state records actual effects, and an independently sourced observer evaluates postconditions. Recovery after partial commitment and restart/replay must be tested explicitly. This review is a literature map, not evidence that this architecture is already built or superior.

Source: [Wu et al., Beyond Guardrails (2026), DOI 10.62762/JRSC.2026.526903](https://doi.org/10.62762/JRSC.2026.526903).

## Authorization-control and self-evolution benchmark designs

### VerifiedWork Control (UpCube / Ethen, 3 October 2026)

[VerifiedWork Control: Evaluating Delegation, Approval, Revocation, and Agent Authority](https://upcube.ai/resources/research/verifiedwork-control) is a directly relevant **benchmark design**, not a completed public benchmark campaign. It already proposes eight dimensions: out-of-scope denial, exact-action approval binding, approval replay/staleness, revocation, budgets including parallel subagents, delegation attenuation, never-delegable permissions, and prompt-injection resistance.

Its most useful methodological contribution is the separation of two evaluation layers:
1. **Enforcement layer:** does deterministic runtime enforcement deny unauthorized operations regardless of what the agent attempts?
2. **Agent behavior:** does the agent avoid violations, handle denials appropriately, and report truthfully?

It also proposes null, adversarial and honest scripted agents; pre-run expected decisions checked against an independent oracle; reordered replay; revocation-latency distributions under load; and false-denial / valid-task utility metrics. This is a strong design to crosswalk against rather than a field whose general problem statement Luminous can claim to have invented.

Evidence must be read carefully. The broader VerifiedWork Control track is explicitly labelled a proposed design and has not been run. It cites one published system card from a single pinned Ethen build: 194 of 197 enumerated conditions matched prediction and oracle on two ordered passes. The authors state that this provides no population or live-traffic claim and involved no statistical test. Its approximately 5-second agent-stop and 30-second p99 no-new-consequential-action targets are proposed engineering targets, not published measured distributions.

**Luminous implication:** adopt the two-layer reporting, adversarial/benign/null reference controls, reordered replay, explicit false-deny metrics, and under-load revocation distributions. Extend them with root-vs-chain-vs-hybrid delegation policy comparisons, participant-committed effects, unknown-outcome reconciliation, source/common-cause provenance, and independent service postconditions. Do not claim these basic enforcement dimensions as unique.

Source: [VerifiedWork Control](https://upcube.ai/resources/research/verifiedwork-control).

### HarnessEvolve: self-evolving agent systems

[HarnessEvolve (arXiv:2609.00829)](https://arxiv.org/abs/2609.00829) studies self-evolution through reference trajectories, separating execution, evaluation, optimization and acceptance gating. It identifies credit-assignment failure, shortcut learning and catastrophic forgetting as central risks. Its approach compares failed executions with reference trajectories and gates changes on quality and non-regression.

This complements EVOHARNESSBENCH: one evaluates preservation/adaptation as the supplied harness changes; the other proposes a method for making harness updates safer. Neither should be collapsed into an ITOPS task-success result. Luminous should test whether a new model, skill, tool or policy update preserves already-qualified authority, revocation, provenance and unknown-outcome properties while improving held-out operational utility.

Source: [HarnessEvolve (arXiv:2609.00829)](https://arxiv.org/abs/2609.00829).

## Updated benchmark programme

Keep scorecards native and separate; report task success, safety enforcement, agent behavior, diagnosis, remediation and resource efficiency as different dimensions.

| Surface | Native measure | Initial target |
|---|---|---|
| EnterpriseOps-Gym-AA | Strict all-verifier final database-state success | Reproduce the matched baseline; then target ≥5 percentage-point uplift with the same model/configuration and no hard-safety regression |
| ITBench-AA | Average precision at full recall | Reproduce native metric first; target ≥56.2% on the same tasks/model/harness, with separate false-positive and root-cause attribution analysis |
| SRE-Marathon | Continuous fault-resolution score | Reproduce 41.3/100 first; then target >50/100 while maintaining zero unauthorized effects and no retry during unknown outcomes |
| APEX-Agents | Pass@1 and mean rubric score | Reproduce the open subset; preserve or improve Pass@1 while reducing cost/time and retaining evidence lineage |
| AutomationBench-AA | Partial objective credit | Use only as diagnostic progress; never treat partial completion as strict operational success |
| Delegation / runtime enforcement | Unauthorized attempt/effect, false-denial, revocation latency, provenance retention, over-disclosure, scope loss, effect reconciliation | Zero unauthorized committed effects; zero new mutation while prior outcome unknown; 100% valid positive-control acceptance and known expansion rejection in the frozen conformance corpus |

The most important experimental comparison is not a raw model leaderboard. Run the **same model, task, tool manifest, token/tool budget and environment** in matched arms: model-only; model with baseline guardrails; strict recursive attenuation; root-anchored task graph; and root-anchored delegation with participant-side enforcement and durable shared budgets. Include honest, null and adversarial scripted agents so deterministic control-plane behavior is measured separately from agent behavior. Measure false-denial and safe-task utility alongside unauthorized effects; a controller that blocks every action must fail the positive controls.

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


## Multi-agent authorization, delegation, and provenance: research correction — 2026-11-06

A significant correction to the research thesis: **authorization preservation across multi-agent delegation is already directly benchmarked**. Luminous must not position the general problem or monotonic narrowing principle as wholly novel.

### MasDrift: the required policy comparison

[MasDrift: Benchmarking Authorization Preservation Across Multi-Agent Architectures](https://arxiv.org/abs/2608.07556) evaluates 600 benign productivity tasks across eight domains, including reserved actions, and compares single-agent, centralized, and decentralized/peer architectures while varying hierarchy depth and peer width.

The paper reports:
- centralized task completion of 93.9–98.6% versus 85.7–87.0% for peer networks in its evaluated configurations;
- unauthorized actions in 2.7–19.8% of tasks versus 0.6–0.8% in the compared configurations, with authorization failures increasing with hierarchy depth;
- a direct comparison between **re-anchoring every pending action to the original user request** and **attenuated policy propagation down the delegation chain**.

A critical result for our architecture: root-request re-anchoring reduced unauthorized actions with a reported 1.6 percentage-point pooled task-completion cost, while strict chain propagation could forfeit up to 36.3 percentage points of task completion in the evaluated conditions.

Source: [MasDrift, arXiv:2608.07556](https://arxiv.org/abs/2608.07556).

**Design implication:** strict parent-to-child attenuation must remain one candidate policy arm, not our assumed winner. Repeatedly intersecting every intermediate permission set can block a child from performing a sibling task that the root principal explicitly authorized. We need a matched experiment comparing root re-anchoring, strict attenuation, and a hybrid participant-enforced design.

### Delegated Misalignment: responsibility diffusion and subordinate role bias

[Delegated Misalignment: How Multi-Agent Structures Amplify LLM Safety Risks](https://aclanthology.org/2026.emnlp-main.398/) studies six models and 49 hazardous tasks. It reports that delegation can amplify unsafe task execution through principal-side responsibility diffusion and subordinate-side role-bias compliance; one reported DeepSeek-V3.2 full-execution result rises from 30.6% in the single-agent setting to 77.6% under delegation.

This is not merely an authorization-token problem. The principal may diffuse responsibility while a subordinate interprets its assigned role as an instruction to comply. Our benchmark should include harmless synthetic analogues of these social/organizational failure modes, including escalation, dissent, stop authority, and refusal propagation.

Source: [Delegated Misalignment, EMNLP 2026](https://aclanthology.org/2026.emnlp-main.398/).

### Provenance Erasure: source context can disappear while payload survives

[Trust Cascade through Provenance Erasure](https://aclanthology.org/2026.emnlp-main.1005/) evaluates multi-agent handoffs in which authority and provenance metadata are stripped while actionable content survives. Across 460 pairs, the paper reports losses of 98% of format markers, 100% of authority entities, and 44% of source attribution in its evaluated transformations. It reports cascade attack success of 33–63% across its evaluated open-weight models, and proposes a zero-inference Trust Boundary Auditor.

Source: [Trust Cascade through Provenance Erasure, EMNLP 2026](https://aclanthology.org/2026.emnlp-main.1005/).

**Design implication:** authority scope and source lineage must be carried as machine-checkable evidence, not as natural-language instructions that can disappear during summarization. Every transformation must preserve a traceable link to the original principal, task, plan, authority check, source record, and derivation path. Missing or detached provenance means unknown/denied, not inferred permission.

### Prompt-injection benchmark context

Google Cloud's published AgentDojo/Semantic Governance Policy documentation describes a benchmark across banking, Slack, travel and workspace tasks, and reports a reduction in attack success from 48.7% to 0.9% with a 2.6 percentage-point legitimate-task utility cost for its policy implementation.

Source: [Google Cloud — Semantic Governance Policy and adversarial benchmarking](https://docs.cloud.google.com/gemini-enterprise-agent-platform/govern/policies/adversarial-benchmarking).

Treat this as a **vendor-reported result**, not an independent head-to-head comparison. It is useful as a reference for the safety/utility frontier and for testing whether deterministic, policy-based controls add measurable protection beyond model prompting.

## Revised Luminous research proposition

The testable differentiator is not simply "delegation cannot expand permissions." Existing work already covers authorization preservation, prompt injection, delegated misalignment and provenance loss.

The more specific research contribution should be an integrated **authority-and-effect boundary** that combines:

1. **Root authorization envelope:** immutable principal, task graph, root plan, permitted domains, exact reserved actions, time and resource bounds.
2. **Subtask delegation envelope:** exact pre-authorized child task/plan digests, principal lineage, local target/data/parameter constraints, and bounded resource budgets.
3. **Per-effect root re-anchoring:** before each consequential effect, the final participant rechecks the current root-authorized envelope and the child's exact subtask grant against the exact request/parameters.
4. **Causal provenance preservation:** transformations, summaries and tool handoffs retain source lineage and explicit derivation; they cannot manufacture authority or independent evidence.
5. **Effect reconciliation:** a participant-owned attempt/effect record and an independent service postcondition distinguish proposal, dispatch, commitment and verified operational success. An unknown effect locks further mutation until reconciliation.
6. **Responsibility and dissent semantics:** a subordinate can refuse, pause or escalate without the principal's delegated role framing silently converting an unsafe task into an obligation to comply.

This is a research hypothesis, not a finished design. The hybrid could outperform both strict attenuation and naive re-anchoring, but only a matched experiment can establish that.

## Required policy arms and metrics

Extend [Luminous Platform issue #288](https://github.com/Luminous-Dynamics/luminous-platform/issues/288) to reproduce the native MasDrift policy comparison where artifacts/licensing permit, then add a separate Luminous operational-evidence extension.

| Arm | Policy / topology | Purpose |
|---|---|---|
| A | Single agent | Native baseline |
| B | Centralized multi-agent, no added authorization defense (synthetic-only) | Measure topology and delegation risk |
| C | Decentralized/peer, no added authorization defense (synthetic-only) | Measure peer coordination trade-offs |
| D | Original-request re-anchoring at every pending action | Reproduce the MasDrift defense |
| E | Strict edge-by-edge attenuated delegation (#285) | Measure scope preservation and false blocking |
| F | Hybrid root re-anchoring + precommitted child task/plan + bounded child grant + final participant enforcement | Test the Luminous design hypothesis |

Native MasDrift task/unauthorized-action metrics must remain unchanged and separate from the supplemental ITOPS measurements. Add responsibility-diffusion and provenance-erasure scenarios, plus participant-committed effects, independent service postconditions, unknown-outcome retries, and evidence-source lineage.

Measure the full utility/safety frontier:
- unauthorized attempts and unauthorized committed effects;
- authorized task completion and false-refusal / false-block rate;
- root-envelope re-anchor correctness and scope-expansion detection;
- valid subtask acceptance, including read-only and separately pre-authorized sibling tasks;
- revocation propagation and time-to-invalidation across the whole ancestor chain;
- provenance-marker retention, origin/derivation reconstruction and evidence-laundering detection;
- participant effect / independent postcondition agreement;
- duplicate attempts, outcome-unknown lock violations and reconciliation latency;
- human dissent, escalation and stop-action propagation;
- p50/p95 latency, model/tool tokens, resource cost and cost per verified outcome.

Do not combine safety and utility into one compensatory score. A critical unauthorized committed effect disqualifies an arm; a blanket-refusal system also fails the positive-control utility tests.

## Revised qualification targets (proposed, not measured)

- Reproduce the native MasDrift results and report deviations before claiming an improvement.
- Compare the hybrid with root re-anchoring and strict attenuation on identical task/seed/model/prompt/tool/topology conditions.
- Seek the re-anchoring safety improvement without a greater than 1.6 percentage-point pooled completion cost on the matched native protocol; this is a replication target from the paper, not a current Luminous result.
- Zero unauthorized committed effects and zero new mutation attempts while a prior effect is outcome-unknown in the declared synthetic negative-control population.
- 100% acceptance of valid positive controls, including read-only delegation and explicitly root-authorized sibling subtasks.
- 100% detection of deliberately removed authority/source-lineage markers in the frozen TBA-inspired conformance set, with false positives reported separately.
- Preserve the complete authority and source lineage through every delegation, summary, restart, retry and participant effect.
- Keep native benchmark scores, Luminous effect-integrity qualification and operational cost/latency as separate result surfaces.

These targets do not constitute universal guarantees or production approval. No Luminous reproduction of MasDrift, Delegated Misalignment, Trust Cascade, or the vendor-reported AgentDojo result has been executed yet.


## MasDrift source lock and reproduction readiness

A machine-readable upstream source lock is stored at [external/masdrift-source-lock-7119c09.json](external/masdrift-source-lock-7119c09.json). It pins the public MIT repository at commit `7119c09c0266162f89db29d3faa32d496837b5c3` and commit-tree SHA `d2989e32176a49ea2cdb17b95b8db3fdd4496e1a`, and records Git blob IDs and sizes for the task corpus, all 601 environment files, key runner/evaluator/authorization modules, and relevant regression tests.

At the pinned dataset:
- Dataset ID: `masdrift_pilot_v5_tasks_mas`; schema `pilot-task-env-2026-05-28+mas_gt_v1`.
- 600 task records and 600 unique task IDs.
- Domain distribution: healthcare 55; finance 90; legal 50; HR 85; marketing 80; operations 80; support 80; sales 80.
- 600 distinct task-specific environment files plus one generic environment; all references resolve and no task environment file is unreferenced.
- The pinned repository commit object is dated 12 August 2026. The MasDrift paper's v3 revision is dated 15 October 2026. They must be reported as separate artifact dates; do not imply the code commit was made with the v3 paper revision.
- `requirements.txt` contains broad lower-bound/range constraints and the pinned tree has no Python dependency lockfile. A truly reproducible run still needs an environment lock or container digest.

**Execution matrix caveat:** the PowerShell matrix has eight topology/persona conditions and can execute one defense treatment per invocation. The maintained cross-model Python launcher has nine conditions (including single-agent) but automates only `none` and `authorizer`; it does not automate `chain` or `both`. Consequently, a full defense/topology/model reproduction needs explicitly orchestrated and audited additional invocations. The source lock encodes this limitation so a partial run cannot be reported as full MasDrift parity.

This lock is a provenance inventory, not an experimental reproduction and not proof that the paper's reported results can be recreated with the current dependencies or models. The next required steps are dependency/container pinning, smoke tests for every treatment, deterministic run manifests, and validation of native evaluator parity before scoring any arm.
