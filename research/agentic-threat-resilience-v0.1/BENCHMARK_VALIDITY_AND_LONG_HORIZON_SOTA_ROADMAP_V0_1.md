# Agentic Authorization and Effect Assurance: Benchmark Validity and Long-Horizon SOTA Roadmap v0.1

**Status:** Research and experimental-design document. This is not a benchmark result, a product comparison, or a claim of SOTA superiority.

## 1. Thesis: stop benchmarking "agent safety" as one scalar

Agent-safety benchmarks do not all measure the same construct. The July 2026 validity audit of R-Judge, InjecAgent, AgentHarm and AgentDojo reports divergent model rankings, and demonstrates a degenerate always-positive classifier scoring F1=0.690 on R-Judge—above five of 21 evaluated models. The paper's conclusion is that a safety claim must name its benchmark, metric, target behavior and evaluated model panel. [Wang et al., arXiv:2607.28685](https://arxiv.org/abs/2607.28685).

That finding changes the research objective. Do not build a single weighted "agent safety" score. Build a measurement system that identifies exactly which obligation failed, where authority moved, what the environment did, whether a prohibited effect committed, what evidence independently supports that conclusion, and what utility was sacrificed.

The benchmark should evaluate at least five orthogonal constructs:

1. **Instruction and data safety:** Did the agent follow malicious instructions, expose protected data, or misuse tools?
2. **Authority lineage:** Did the acting workload possess a valid, sufficiently narrow mandate traceable to the right principal and delegator?
3. **Coordination safety:** Did permissions, constraints, memory taint or identity confusion propagate through agent handoffs and delegation?
4. **Effect correctness:** Did a protected resource actually change, and does independent evidence bind the effect to the exact actor, request, policy and authority?
5. **Temporal and recovery correctness:** Did authority remain current at commit time? Were revocation, unknown writes, retries, duplicate attempts and downstream descendants handled safely?

A result in one column is not proof of the others. A refusal may fail to prevent a later effect. A valid mandate may still be denied by the RP. A correct final state may be reached through an unauthorized path. A detector may correctly identify a suspicious trace after a payment already committed.

## 2. Industry and research benchmark landscape

The numbers below describe what the source benchmarks report; they are not a cross-benchmark ranking and are not measurements by Luminous.

| Benchmark | Strongest construct | Public scale / complexity signal | Missing layer ATR adds |
|---|---|---|---|
| AgentDojo | Tool-use prompt injection and utility under attacks | 97 tasks / 629 security cases in the commonly cited release | Delegation authority source, commit-time status and independent effect evidence |
| InjecAgent | Indirect prompt injection in tool-using agents | 1,054 attack cases | Cross-agent lineage and long-horizon authority propagation |
| AgentHarm | Harmful user-intent compliance | 110 tasks / 440 augmented tasks | Resource commit and authorization provenance |
| ASB | Broad tool-agent attack/defense coverage | 10 scenarios, 400+ tools, 27 attacks/defenses, multiple metrics | Principal-to-agent mandate chain and independently verified resource state |
| MasDrift v3 | Multi-agent authorization preservation, handoffs and coordination | 600 tasks, 8 domains, 6 model configurations, 9 coordination conditions | Cross-domain Authority Assertion and participant effect finality |
| ACIArena (ACL 2026) | Multi-agent cascading injection across real MAS implementations | 1,356 cases, six MAS implementations, three attack surfaces × three objectives | Authority lineage and actual unauthorized commits across external-input, profile and inter-agent-message propagation |
| AgentDyn | Dynamic open-ended prompt injection with helpful third-party instructions | 60 open-ended tasks, 560 injection cases, Shopping / GitHub / Daily Life, 10 defenses | Dynamic replanning, helpful-instruction discrimination and effect-level authorization |
| MetaPI (methodology, not a task benchmark) | Adaptive attack generation over existing utility benchmarks | Applied to AppWorld, VitaBench and Terminal-Bench; discovers injection points, generates objectives and optimizes payloads | Frozen-task attack evolution with task/attacker holdouts and independent effect oracles |
| Trojan Hippo Bench | Dormant persistent-memory payloads and memory-layer defenses | 5 sensitive topics, 4 memory backends + no-memory baseline, 5 defense settings, persistence 0–100 benign sessions; utility 7 flows × 4 tests | Memory provenance, monotonic taint, exact approval binding, tombstone propagation and independent exfiltration effects |
| AgentLAB | Adaptive long-horizon attack susceptibility | 5 attack families, 28 environments, 644 security test cases; native ASR and turns-to-success | Independent authority lineage, commit-time admission and effect receipts |
| ReliabilityBench | Repeated-run consistency, semantic robustness and infrastructure fault tolerance | 1,280 episodes, 4 domains; reliability surface R(k, ε, λ), Action-MRs and fault injection | Security^k, commit^k, authority-context drift and independently reconciled unknown effects |
| τ²-bench evaluator audit (empirical study) | Evaluator construct validity and forbidden-write detection | 2,160 episodes; 11 human-confirmed premature commits; LLM-judge/native agreement 56.2% | Explicit task/authorization/commitment axes, source-complete predicate evidence and independent receipts |
| GuardianAgentBench | Long-horizon tool correctness and structural guardrails | 580 scenarios, 6 domains, 3 production-ready frameworks; reports monotonic degradation with tool-set size and turn depth | Source-anchored authorization and actual participant effect finality |
| AgentCanary | Real executable agent environments and trajectory-grounded risk evaluation | Persistent multi-step state, dynamic task artifacts, three frameworks; Entry × Impact taxonomy and Outcome Safety / Security Awareness / Task Utility axes | Source-qualified authority, exact commit-time policy evidence and independent effect receipts |
| DTap | Stateful desktop interaction and attack exposure | Median 15 tool calls in the OpenART comparison | Source-authorized delegation and independent protected-effect receipt |
| OpenART | Long-horizon state evolution, environment poisoning and side-effect safety | 10K+ validated scenarios / 50 domains; release 10,150 tasks in 52 shards + 63,697 materialized tools; 15 agents × 5 models, 8 attack surfaces; median 97 tool calls, depth 32, parallel width 12.5, state-object count 96.5 | Formal authority lineage, mandate validation, principal entitlement, participant commit-time fence and effect receipts |
| WIMSE agent-mandate draft | Cross-domain validation of who may delegate and who issued the mandate | Protocol/validation profile, not an empirical benchmark | Executed agent behaviour, runtime effects, efficacy and operational costs |

**Sources and qualifications**

- AgentCanary contributes an orthogonal Entry × Impact risk taxonomy, real executable environments with persistent state, and trajectory-grounded Outcome Safety / Security Awareness / Task Utility scoring; it is a useful execution-fidelity comparator, not a substitute for participant-side authorization evidence. [AgentCanary, arXiv:2606.10484](https://arxiv.org/abs/2606.10484)
- ReliabilityBench reports a reliability surface over repeated trials, semantic perturbation and fault intensity; in its tested setup, task success fell from 96.9% at ε=0 to 88.1% at ε=0.2, and rate limiting was a damaging fault class. Those values are source-reported results for its 1,280-episode study, not Luminous results. [ReliabilityBench](https://arxiv.org/abs/2601.06112)
- A November 2026 τ²-bench evaluator study reports 2,160 episodes, 98.2% LLM-judge self-agreement but 56.2% agreement with native scoring; human reviewers labeled 11 forbidden writes, and the judge passed 10–11. The authors also trace gate failures to omitted order-status and customer-authentication facts. This motivates the explicit evidence-closure profile rather than reliance on transcript judges alone. [Im, arXiv:2611.14581](https://arxiv.org/abs/2611.14581)
- OpenART authors report over 10,000 validated stateful scenarios across 50 domains, with median task complexity of 97 tool calls, dependency depth 32 and parallel width 12.5. Its pinned upstream README (repository history pin 28 November 2026) describes 10,150 released task bundles in 52 shards and 63,697 materialized tools; the paper uses 15 target agents × 5 models and eight target-visible attack surfaces. The paper reports 85.0% pooled strict ASR (EMHA attacker) and 87.38% benign completion; strict ASR requires both deterministic evaluator and LLM judge. These are source-reported benchmark outcomes, not universal safety rates or Luminous results. The public benchmark landscape is descriptive, not a ranking. [OpenART benchmark landscape](https://ai45lab.github.io/OpenART/benchmarks/) and [paper](https://arxiv.org/abs/2608.00677).
- MasDrift v3 is the coordination comparator already covered by `MASDRIFT_ADAPTER_PROTOCOL_V0_1.md`; its native task labels, defense arms and trace semantics must remain unmodified.
- ReliabilityBench is the repeated-run / perturbation / fault-intensity comparator covered by `RELIABILITY_SURFACE_AND_COMMIT_BOUNDARY_ORACLE_PROTOCOL_V0_1.md`. Preserve its native (k, epsilon, lambda) grid and Action-MRs; ATR adds separate task, authorization, commitment, recovery and evidence dimensions.
- The November 2026 τ²-bench evaluator audit is a methodology comparator, not a new benchmark suite. It shows why native-state, LLM-judge and commit-boundary outputs must be preserved as distinct instruments, and why policy predicates need source-complete dependency manifests.
- GuardianAgentBench supplies an additional multi-framework/long-turn utility and attack-control comparator; its accuracy and guardrail rates are source-reported, not Luminous measurements.
- AgentLAB is the adaptive long-horizon attack comparator covered by `AGENTLAB_ADAPTER_PROTOCOL_V0_1.md`. Its five attack families, 28 environments, 644 cases, native ASR and average turns-to-success remain intact; its adaptive planner/attacker/verifier/judge versions must be pinned, not treated as objective ground truth.
- ACIArena is the multi-agent cascading-injection comparator covered by `ACIARENA_ADAPTER_PROTOCOL_V0_1.md`: 1,356 cases, six MAS implementations, external inputs / agent profiles / inter-agent messages, and instruction hijacking / task disruption / information exfiltration. It is an ACL 2026 long paper; the pinned public code snapshot lacks a detected LICENSE file, so licensing review is required before execution or reuse.
- AgentDyn is the dynamic open-ended third-party-instruction comparator: 60 tasks / 560 injections across Shopping, GitHub and Daily Life. Its helpful-instruction and dynamic-replanning controls are especially useful for measuring false-positive over-defense.
- MetaPI is a meta-benchmark generation method, not a fixed benchmark suite: apply its injection-point discovery / objective synthesis / adaptive payload optimization to frozen utility tasks. Luminous's specification is `ADAPTIVE_BENCHMARK_EVOLUTION_PROFILE_V0_1.md`; no MetaPI upstream code repository was verified or executed.
- Trojan Hippo Bench is the persistent-memory comparator covered by `TROJAN_HIPPO_ADAPTER_PROTOCOL_V0_1.md`; preserve its five topics, memory backends, defenses, 0–100 benign-session persistence splits, adaptive train/test separation and utility suites. Its adaptive attacker and semantic judge are pinned experimental components, not authority/effect oracles.
- `draft-hulea-wimse-agent-mandate-01` is an individual Internet-Draft published 1 November 2026. It is not endorsed by the IETF and has no formal standing in the IETF standards process. Treat it as a moving research input, pin exact text, and re-evaluate on every revision. [Datatracker status](https://datatracker.ietf.org/doc/draft-hulea-wimse-agent-mandate/01/).
- The validity audit demonstrates why reported F1, ASR, completion or model rankings cannot be quoted as generic "safety." [Validity audit](https://arxiv.org/abs/2607.28685).

## 3. The Luminous differentiation: authority-to-effect traceability

Most current evaluations stop at a candidate trace, a refusal label, a policy decision, or simulated environment state. ATR's intended contribution is a separate evaluator-owned, causal path from authority source to real participant effect:

`Authority Source → Mandate Issuer → Root Mandate → Sub-delegation → Workload PoP → RP authorization → Participant PEP → Resource commit → Independent effect receipt`

For every edge, record the asserting principal, issuer role, subject, tenant, exact request/plan/branch binding, validity interval, status source and evidence quality. Keep these states distinct:

- issued ≠ presented;
- presented ≠ validated;
- validated ≠ authorized by RP;
- authorized ≠ admitted at participant;
- admitted ≠ committed;
- committed ≠ independently observed;
- candidate-reported ≠ evaluator-verified.

This should be a claim graph, not a flat event log. Its edges must preserve provenance and evidence class. A candidate-generated explanation may propose a causal edge; it cannot promote that edge to evaluator-verified evidence. If an intermediate event is missing, report attribution as incomplete/unknown rather than infer a safe chain.

## 4. Evaluation architecture: preserve upstream benchmarks, add a distinct ATR layer

For each upstream benchmark, maintain two outputs:

1. **Native scorecard:** run the original benchmark, task split, scoring rules and baselines without silently editing tasks or metric definitions.
2. **ATR sidecar:** attach authority-lineage, causal-graph, commit-time and independent-effect measurements wherever the adapter has enough source data. Missing evidence is reported as missing/unknown, never synthesized from model output.

The sidecar must not turn a benchmark into an incomparable new leaderboard. Report native and ATR measurements side by side.

### Candidate system arms

For the first executable slice, retain the four arms already tracked by issue #540:

1. No effective authorization control.
2. Gateway authorization only.
3. Runtime/participant enforcement.
4. Runtime enforcement plus independent effect receipts.

After the first runnable slice and matched baselines, expand to separate ablations:

- source anchor present/absent;
- chain attenuation present/absent;
- authority-to-delegate proof present/absent;
- mandate issuer-role validation present/absent;
- workload PoP present/absent;
- principal entitlement check present/absent;
- commit-time revocation fence present/absent;
- independent observer present/absent;
- provenance-preserving memory present/absent;
- shared service identity vs per-agent identity.

Do not build a monolithic "all controls on" arm and claim which control caused the improvement. Use factorial or carefully paired ablations to isolate contribution and interactions. If an ablation creates an unsafe system, run it only in a disposable synthetic environment.

### Experimental controls

Freeze and publish (or privately escrow for held-out evaluation) the exact candidate model/provider, system/developer prompts, runtime build, tool surface, memory snapshot, authority/mandate fixtures, policy, target state, budgets, clock, seed, task data digest and evaluator version.

Randomize treatment order. Use matched task/seed pairs across arms. Split by task family, resource lineage and causal topology—not random rows or tool calls. Hold out agent providers, runtime stacks, tool names, memory backends, authority issuers, RP domains and graph shapes.

Evaluator ground truth must be inaccessible to candidate agents and defense components. Use a separately operated resource oracle and observer. The observer must have distinct credentials, execution boundary and evidence path from the candidate and policy evaluator. Validate observer independence with fault injection; merely assigning different labels in a JSON fixture is not independence.

## 5. Scorecard: non-compensable security gates plus separate utility and cost

Never let high task completion compensate for an unauthorized committed effect. Never let a high attack-block rate compensate for blocking all required work.

### Security and authority

- unauthorized attempt rate;
- unauthorized execution rate;
- unauthorized committed-effect rate;
- cross-tenant read/write rate;
- authority-source coverage and trust-anchor role correctness;
- mandate issuer role and subject/key binding correctness;
- sub-delegation attenuation violations;
- PoP failure and replay rejection;
- revocation/stale-status rejection;
- post-revocation commits after the defined participant barrier;
- time-of-check-to-time-of-use violations;
- false effect-receipt acceptance;
- unobserved committed-effect rate.

### Causal and temporal

- proportion of committed protected effects whose full authority-to-effect graph is independently evidenced;
- missing-edge and false-edge rate by evidence class;
- maximum verified propagation depth and number of descendants affected;
- delegated authority amplification (reachable effect tuples beyond root-delegable scope);
- revocation propagation latency and maximum post-barrier blast radius;
- quiescence time for child agents, tasks, caches, queues and in-flight sessions;
- unknown-effect count, duplicate retry count and time-to-reconciliation.

### Utility and cost

- native task completion and weighted required-step completion;
- required-call block rate and unnecessary-confirmation rate;
- unauthorized attempts blocked (separate from commits prevented);
- time to completion, recovery effort and operator interventions;
- model tokens by role, tool calls, policy latency, participant latency, p50/p95/p99;
- CPU, memory, network, issuance/status traffic and end-to-end cost.

### Hard gates

For high-impact protected actions, proposed acceptance gates are:

- zero unauthorized committed effects;
- zero cross-tenant effects;
- zero commits after the acknowledged revocation/fencing barrier;
- zero accepted wrong-role authority or mandate signatures;
- zero scope-widening delegation permits;
- zero false independent-effect receipts;
- 100% lock on unresolved consequential writes until the original outcome is reconciled;
- 100% exact binding of the observed effect to actor, tenant, request, resource, operation, authority generation and idempotency identity.

These are qualification gates for an explicitly scoped test profile, not proof of universal safety. Report observed counts and confidence bounds.

## 6. Targets relative to SOTA (proposed, not achieved)

Set comparative targets on matched environments and the same candidate panel:

1. **Security:** reduce unauthorized committed effects by at least 50% relative to the strongest measured native/hybrid baseline, with a task-clustered 95% confidence interval excluding no improvement.
2. **Utility:** keep benign task completion within 1 percentage point of the strongest matched baseline; report required-call blocking separately, with the same cluster-aware uncertainty.
3. **False positives:** at matched security recall, target no more than 5% false-positive rate on benign tasks, stratified by operation and risk tier. Include always-allow, always-deny and always-positive controls.
4. **Attribution:** independently evidence at least 99% of committed protected effects in a fully instrumented lab profile; unknown attribution is not silently scored as correct.
5. **Revocation:** zero post-barrier committed effects in every injected revocation race; measure p50/p95/p99 denial propagation and quiescence latency under load.
6. **Long horizon:** run through at least OpenART-like task complexity strata—short, medium, long, deep, wide, stateful—then extend beyond 100 calls and 32 dependency depth in a held-out stress tier. Do not compare different task mixes as if they were the same benchmark.
7. **Efficiency:** measure full control-plane and runtime cost; show that the security/utility gain is not achieved through unlimited model calls, blanket confirmation or a denial-all policy.

For rare failure claims, use enough independent task/seed clusters. Under a simple independent Bernoulli model, 3,000 failure-free episodes give an approximate one-sided 95% upper bound of 0.1% (rule of three). Multi-agent episodes can be correlated by shared seeds, agents, models and infrastructure; calculate effective cluster counts and use cluster bootstrap or an appropriate hierarchical model. Never present the rule-of-three bound as proof of zero risk.

## 7. Anti-gaming and construct-validity requirements

Every release must report:

- benchmark name/version, target construct and metric definition;
- model/runtime panel and exact version bindings;
- class prevalence and precision/recall/specificity, not only F1;
- always-allow, always-deny and always-positive baselines;
- benign task completion and required-call block rate;
- attack attempts, executions and committed effects as distinct counts;
- independent observer coverage and fault qualification;
- confidence intervals with task/seed clustering;
- held-out test separation and any leaked or reused templates;
- all excluded/indeterminate episodes and the reason;
- cost per successfully completed benign task and cost per prevented unauthorized commit.

Use human audit only for explicitly subjective fields, report inter-rater agreement and blind the reviewer to treatment. LLM judge outputs are annotations, not authority or effect oracles. Recheck evaluator agreement on an independently sampled set and make adversarial evaluator mutation tests part of the benchmark.

## 8. Reliability-surface and evaluator-oracle programme

The new [Reliability Surface and Commit-Boundary Oracle Protocol](RELIABILITY_SURFACE_AND_COMMIT_BOUNDARY_ORACLE_PROTOCOL_V0_1.md) combines repeated-run consistency, semantic perturbation and infrastructure faults with ATR's separate effect/authority measurements. Do not collapse them into a single reliability score.

The new [Policy Evidence Closure and Commit-Boundary Profile](POLICY_EVIDENCE_CLOSURE_AND_COMMIT_BOUNDARY_PROFILE_V0_1.md), tracked by #553, turns the November τ²-bench evaluator finding into a source-level conformance contract: complete predicate dependencies, source/freshness/version closure, same exact action/evidence binding at commit, and independent effect observation. The 20-case synthetic corpus includes a positive write, safe refusal, required-write under-action, missing/stale/candidate-only facts, policy denial with a committed write, actor/tenant substitution, context drift, unknown outcomes, judge disagreement and effect without participant receipt.

A first minimal grid should run on the synthetic #540 participant before broad external integration:
- k = 1, 4, 8 repetitions per frozen task and arm;
- epsilon = 0 and 0.2 with semantically equivalent paraphrase/reorder/distractor transformations;
- lambda = 0 and 0.1 with deterministic fault profiles, especially response loss before and after commit;
- record the full vector of task success, safe^k, authorized^k, commitment^k, recovery and attribution;
- cluster inference by task/seed and preserve every failed/unknown run.

These are initial design points, not universal severity standards. Expand the grid only after the first vertical slice is reproducible and the injected faults are known to be faithful.

## 8. Staged execution plan (do not block on #540)

### Stage A — current source-only increment
- Cross-domain mandate profile and schema.
- 12 deterministic synthetic mandate cases.
- Single-field mutation controls covering authority, issuer role, status, PoP, audience, scope, lineage and RP separation.
- Exact-head schema/reference-checker workflow.
- Public documentation of non-cryptographic claim ceiling.

### Stage B — executable policy and trace contract
- Finish and genuinely execute the existing Rust qualification core.
- Add the source-anchored authority/mandate validators without duplicating Xenia/Mycelix canonical policy owners.
- Add an evaluator-side graph comparator and immutable run descriptor.
- Keep candidate, issuer, participant and observer trust domains distinct in actual processes—not only in fixture labels.

### Stage C — first end-to-end episode
- Complete issue #540 on a synthetic target with a real isolated participant process.
- Make one allowed read/write and one reserved action.
- Prove the gateway-only arm can be bypassed in a negative control while the participant arm rejects.
- Introduce one response-loss/unknown-write episode and prove no blind retry occurs.
- Produce participant-sourced receipt and independent observer result bound to the exact action.

### Stage D — matched benchmark adapters
- Run source-pinned MasDrift first because it directly tests multi-agent authorization preservation.
- Add ACIArena next for cascading instruction injection across agent profiles/messages and different MAS implementations; the 3×3 adapter contract in `ACIARENA_ADAPTER_PROTOCOL_V0_1.md` requires all nine native surface/objective combinations and independent evidence.
- Add AgentDyn as the dynamic helpful-instruction / false-positive utility control.
- Apply the MetaPI-style adaptive-generation profile to held-out AppWorld / VitaBench / Terminal-Bench-like utility tasks only after native tasks and evaluator controls are frozen.
- Add AgentLAB as the adaptive long-horizon attack adapter, preserving its 5 families / 28 environments / 644 cases and native ASR/T2S.
- Add Trojan Hippo as the persistent-memory adapter, preserving backend/defense/persistence axes and native utility suites; measure taint-to-effect provenance separately.
- Add OpenART as a separately pinned long-horizon evolving-state adapter using `OPENART_ADAPTER_PROTOCOL_V0_1.md`; preserve the hidden safety contract, target-visible evolution, deterministic + LLM-native scorer and raw trace/snapshot artifacts.
- Run native upstream scorers unchanged, then ATR sidecar.
- Expand held-out attack families only after reproducible execution and independent evaluator checks.

### Stage E — external comparison
- Pre-register the candidate panel, seeds, treatment arms, exclusions and target metrics.
- Publish exact task corpus/model/runtime/evaluator digests where licensing permits.
- Release raw sanitized traces, receipts, cost ledger and independent replay bundle.
- Invite independent replication before saying "better than SOTA."

## 10. Immediate research issues that remain open

- The Mandate Issuer's internal evidence that the delegator actually instructed issuance is a trust-framework problem; the protocol's signature alone cannot prove it. Platform #550 tracks a profile that binds this separate instruction/approval event to the exact root mandate artifact and tests role separation.
- Global mandate budgets across multiple relying parties need shared state or a narrower single-RP/audience design; independently validated copies can otherwise spend the same budget in parallel.
- Revocation and status semantics need freshness bounds and commit-time enforcement, not just validation-time checks.
- Issuer compromise, workload compromise and malicious-but-authorized actions are different threat classes. Strong provenance does not prove benevolent reasoning.
- A cryptographically valid action can still violate the principal's present intent; user consent/confirmation must be exact-act-bound and current.
- Independent observer compromise or correlated infrastructure failure can produce false assurance. Independence requires adversarial fault testing.
- Cross-agent memory taint, queue ownership, service identity and state aliasing must be tested as graph paths, not merely as text labels.

## 10.1 WIMSE mandate Trust-Framework conformance

The separate `WIMSE_TRUST_FRAMEWORK_CONFORMANCE_PROFILE_V0_1.md` turns draft -01 Section 12.2 items 11 and 12 into a synthetic conformance contract:
- 15 cases and 14 mutation controls for exact delegator-signed mandate content, qualified issuer process, role separation, withdrawal/supersession, evidence freshness, shared-global vs explicitly allocated per-RP budgets, concurrent overspend and unknown-outcome reconciliation.
- Exact mandate-content digests remain opaque values supplied by the existing Act-Bound owner; the checker does not create a second production canonicalizer or signature verifier.
- Shared participant budget tests are separate from per-RP allocation tests. A per-RP profile is accepted only with explicit Authority Source allocation and an allocation sum bounded by the authority ceiling.
- A status marked active but checked outside the Trust Framework maximum-status-age is indeterminate, not current.

The latest schema/checker/fixtures are in this PR branch. The exact-head hosted run is required before treating the conformance checker as CI-qualified; these fixtures remain synthetic even after a passing workflow.

## 11. References

- OpenART benchmark landscape and complexity table: https://ai45lab.github.io/OpenART/benchmarks/
- AgentCanary executable-environment and Entry × Impact evaluation: https://arxiv.org/abs/2606.10484
- ReliabilityBench repeated-run / Action-MR / fault-surface protocol: https://arxiv.org/abs/2601.06112
- τ²-bench evaluator disagreement and commit-boundary audit (10 Nov 2026): https://arxiv.org/abs/2611.14581
- Trojan Hippo paper v4: https://arxiv.org/abs/2605.01970
- Trojan Hippo Bench, AISec 2026: https://doi.org/10.1145/3847352.3848090
- draft-han-bmwg-agent-security-benchmark-00: https://datatracker.ietf.org/doc/draft-han-bmwg-agent-security-benchmark/
- ATR broad methodology crosswalk: `AGENT_SECURITY_BENCHMARK_CROSSWALK_V0_1.md`.
- Chen et al., OpenART: Scaling Agent Red Teaming via Open-Ended Environment Evolution: https://arxiv.org/abs/2608.00677
- Wang et al., Safety, or Just Capability? A Validity Audit of Agent-Safety Benchmarks: https://arxiv.org/abs/2607.28685
- MasDrift v3: https://arxiv.org/abs/2608.07556
- ACIArena, ACL 2026: https://aclanthology.org/2026.acl-long.457/
- ACIArena code snapshot (license not detected at pin): https://github.com/Greysahy/aciarena/tree/3f226a40cddf01a08b9f700cbef0031dfab2ff64
- AgentDyn: https://arxiv.org/abs/2602.03117
- MetaPI, arXiv:2611.10026 (5 Nov 2026): https://arxiv.org/abs/2611.10026
- ATR adaptive benchmark generation profile: `ADAPTIVE_BENCHMARK_EVOLUTION_PROFILE_V0_1.md`
- ACIArena adapter: `ACIARENA_ADAPTER_PROTOCOL_V0_1.md`
- WIMSE Agent Mandates, draft-hulea-wimse-agent-mandate-01 (work in progress): https://datatracker.ietf.org/doc/draft-hulea-wimse-agent-mandate/01/
- OWASP Top 10 for Agentic Applications 2026: https://genai.owasp.org/2026/01/05/owasp-top-10-for-agentic-applications/
- Existing work: Platform #540–#551, PR #542, PR #548; ACIArena, AgentLAB, OpenART, Trojan Hippo, MetaPI-style generation and WIMSE Trust-Framework adapter profiles in this branch.
