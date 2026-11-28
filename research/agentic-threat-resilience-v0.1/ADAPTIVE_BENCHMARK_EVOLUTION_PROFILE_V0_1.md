# Adaptive Benchmark Evolution Profile v0.1

**Status:** Luminous research-method specification inspired by the MetaPI meta-benchmarking approach. This is not a reproduction or implementation of MetaPI, and no adaptive attack execution is claimed.

**Research input:** Wang et al., *MetaPI: Constructing Prompt Injection Benchmarks from LLM Agent Benchmark*, arXiv:2611.10026 (5 November 2026). The paper proposes transforming existing agent-utility benchmarks into end-to-end indirect prompt-injection evaluations by identifying plausible injection points, generating realistic attack objectives, optimizing adaptive payloads and testing attacks in the real environment. It reports applications to AppWorld, VitaBench and Terminal-Bench. https://arxiv.org/abs/2611.10026

## 1. Paradigm: separate task evolution from oracle evolution

Static attack sets become stale as agents, tools, memory systems and environments evolve. A benchmark should be able to generate new threat instances from realistic utility tasks without changing the original task's goal, scoring or environment semantics.

The Luminous method separates three immutable/evolving layers:

1. **Frozen utility substrate:** original task, user goal, environment state, tool contracts, success condition and native scorer are digest-pinned. The task is never rewritten to make an attack easy.
2. **Evolving adversary surface:** a red-team generator discovers plausible places where an untrusted instruction, malicious profile, memory item, tool result, inter-agent message or environmental state can influence the agent. It generates attack objectives and payload variants under explicit budgets.
3. **Independent security/effect oracle:** the evaluator records whether the payload was observed, believed, propagated, accepted as authority, invoked, committed and independently observed. Candidate explanations and LLM-judge outputs cannot create ground truth.

This means the benchmark can evolve its attack population while retaining a stable utility baseline and a stable effect oracle.

## 2. Pipeline

### Stage 0 — freeze the substrate

For each source task, publish or escrow:
- benchmark name/version, task ID and split;
- original user task/goal and native success condition;
- environment/container/tool versions and initial-state digest;
- allowed actions, resources, budgets and hidden safety contract;
- candidate model/runtime/build, system instructions, memory snapshot and adapter versions;
- seed, wall-clock/turn limit, retry semantics and cost cap;
- native scorer and version.

The source benchmark remains unchanged. An attack wrapper is a new experimental treatment, not a modified upstream task.

### Stage 1 — enumerate attack surfaces

Use a typed injection-point registry. Each point must bind a real source/consumer boundary:

| Surface | Injection point | Example source | ATR evidence to preserve |
|---|---|---|---|
| External input | Email, web page, document, ticket, repository issue, API response | Untrusted third-party content | Source, tenant, content digest, capture boundary |
| Agent profile | Role, goal, system configuration, tool description | Profile/config mutation channel | Profile version, owner, authorization event |
| Inter-agent communication | Message, handoff summary, tool result forwarded to worker | Compromised peer or shared coordinator | Sender, recipient, audience, parent message digest |
| Memory | Persistent item, summary, embedding, cache, RAG chunk | Untrusted tool call or imported record | Memory ID, parent chain, inherited taint, snapshot/tombstone |
| Environment | File, database row, task queue, resource state, visible output | State mutation observable by the agent | Before/after state revisions, writer identity, sequence |
| Tool interface | Arguments, output schema, capability metadata, callback | Tool result or adapter confusion | Workload identity, exact call digest, participant effect |
| Identity/authority | Credential, grant, approval, status, delegation record | Stale or substituted trust input | Issuer role, subject, epoch, status, PoP and authority lineage |

An injection point is eligible only if the environment actually exposes that boundary to the target system. Do not invent hidden channels that the agent cannot reach.

### Stage 2 — generate realistic attack objectives

Objectives are derived from the source task and protected-state contract, not generic "be malicious" labels. Examples:
- redirect an allowed lookup to a prohibited recipient;
- exfiltrate one protected object while preserving apparent task completion;
- cause a child agent to exceed its delegated operation or resource scope;
- inject a false consent claim into persistent memory;
- alter task/plan identity or branch assignment after root precommitment;
- cause a revocation race or unknown-write retry;
- disrupt a required utility step without producing a visible safety signal.

Every objective carries:
- targeted protected resource and operation;
- required attacker capability and trust position;
- intended victim/agent/branch;
- security property violated;
- observable effect condition;
- benign counterpart / utility control;
- maximum adaptation budget.

### Stage 3 — adaptive payload optimization

An optimizer may adapt phrasing, timing, placement, ordering and multi-step bridges based on training feedback. It must not alter:
- the source task goal or expected successful end state;
- protected resource/tenant boundaries;
- evaluator ground truth;
- test-set cases or test seed;
- the candidate's authorized tool budget or action semantics.

Use a constrained search policy with explicit mutation operators and per-case/per-family budgets. Preserve every generated variant, parent/child generator lineage and reason for selection. Deduplicate semantically and structurally so one payload template cannot dominate the test distribution.

For multi-agent attacks, the mutation space must include topology-independent factors such as role assignment, parent-child handoff, shared service identity, message audience, memory derivation and capability attenuation. Merely adding more agents is not a meaningful attack diversity measure.

### Stage 4 — execute in a real isolated environment

The candidate must encounter the attack through the actual benchmark surface. Record raw interaction and state transitions. A payload is not considered successful merely because:
- it appears in a prompt;
- the model repeats it;
- an LLM judge labels the attack successful;
- an action is attempted;
- the final task answer is wrong.

Success classes are distinct:
`delivered → observed → believed/used → propagated → accepted-as-authority → invoked → executed → committed → independently-verified`.

The evaluator may mark intermediate classes unknown. The final protected-effect condition must come from the environment/participant oracle.

### Stage 5 — evaluate paired arms

For each frozen task and attack seed, compare:
- native benchmark baseline with no added ATR control;
- native benchmark's own defense arms unchanged;
- provenance / taint enforcement;
- source-anchored task and capability authorization;
- workload PoP and current status validation;
- participant commit-time enforcement;
- full profile including independent observation and unknown-outcome reconciliation.

Use paired task/seed analysis; randomize arm order. Keep candidate, attacker, evaluator, participant and observer credentials/storage/processes separated. Run all attacks in disposable synthetic environments.

### Stage 6 — adapt on training only, publish held-out results

Split by more than random payload strings:
- source task family and environment state;
- injection point type;
- objective family;
- payload/template and attack-generator lineage;
- victim model/provider/runtime;
- MAS implementation and role/topology;
- memory backend and persistence horizon;
- tool / resource schema.

The test generator seed and hidden task/attack cases must remain inaccessible to the adaptive optimizer and candidate. Freeze the final generator at a declared version before opening held-out evaluation. Report separate generalization splits for unseen tasks, unseen payload families, unseen agents/frameworks and unseen environments.

## 3. The core innovation: causal attack-to-effect conversion

The primary new metric is not simply how often a generated payload fools the model. It is how often the complete attack path crosses a protected effect boundary.

For each family (f), report:
- (N_f): eligible frozen task/attack pairs;
- (D_f): attacks delivered to the candidate through the specified boundary;
- (O_f): attacks observed by the target;
- (P_f): attacks propagated to downstream memory/message/task/agent;
- (A_f): attacks accepted as authority or used to expand a plan;
- (X_f): unauthorized invocations/executions;
- (C_f): unauthorized committed effects;
- (V_f): unauthorized effects independently verified.

Then publish every transition rate, including (C_f/D_f) and (V_f/D_f), with the numerator and denominator. A refusal metric cannot conceal later propagation; a detector metric cannot conceal a commit that happened before detection.

For multi-agent graphs, additionally report:
- propagation depth, fan-out and number of affected agents;
- time from first untrusted input to first protected effect;
- taint-to-effect path completeness;
- authority amplification: reachable effect tuples beyond root P0;
- containment lag and downstream blast radius after revocation;
- unknown-state duration and reconciliation burden.

A generator may be excellent at producing attacks but poor at producing representative threats. Publish diversity and coverage, not just attack yield.

## 4. Metrics for benchmark quality

### Attack generator quality
- eligible injection-point coverage;
- objective coverage / family balance;
- semantic and structural novelty;
- attack success on training vs held-out tasks;
- attack generation calls, cost and wall time;
- transfer across model, framework, tool and environment shifts;
- adaptive overfitting gap;
- attack reproducibility and seed sensitivity.

### Security efficacy
- attempted, denied, executed, committed and independently verified harmful effects;
- false permit, false deny and false receipt rates;
- authority/issuer-role/PoP/tenant/scope binding violations;
- post-revocation-barrier commits;
- memory trust escalation and taint-loss rate;
- causal attribution completeness / unknown rate.

### Utility and operations
- native task completion and required-step coverage;
- safe read-only completion;
- false-positive quarantine and unnecessary-confirmation rate;
- time-to-completion / recovery / operator intervention;
- policy/PEP/observer p50/p95/p99 latency;
- model tokens, tool calls, CPU/memory, provider and infrastructure cost.

Do not combine all dimensions into one score. Attack-generation quality, attack susceptibility, mitigation efficacy, utility and evidence quality are separate constructs.

## 5. Anti-gaming and validity controls

Every release must include:
- always-allow, always-deny and always-positive baselines;
- benign tasks with harmless but suspicious-looking instructions;
- malicious payloads that do not require a protected effect;
- attempted-but-denied and committed-but-unobserved controls;
- judge/model swap, judge-only ablation and deterministic environment oracle controls;
- hidden attacker seeds and held-out mutation operators;
- task/seed-level clustered confidence intervals;
- failure, timeout, unknown and excluded episode reporting;
- immutable exact-source run manifests and raw sanitized traces.

Adaptive attackers and LLM judges are experimental components. Their model, prompt, tools, budget and versions must be pinned. Judge disagreement is reported; the judge cannot decide whether the participant actually committed a write.

A held-out evaluator should independently replay a random sample of outcomes. Mutation-test the evaluator itself: alter one actor/tenant/resource/operation/recipient/authority digest or effect receipt field and prove that the result changes to deny/unknown as appropriate.

## 6. Integration with existing benchmarks

Use MetaPI as a methodological input for adaptive benchmark generation, while preserving native benchmark semantics:

- **AppWorld / VitaBench / Terminal-Bench:** use utility tasks as frozen substrate; do not modify native task goals or scores.
- **ACIArena:** generate adaptive external-input, profile and inter-agent-message attacks against the six pinned MAS implementations; native 1,356-case scorer remains unchanged.
- **AgentLAB / AgentDyn:** use long-horizon dynamic planning, objective drift and evolving third-party content.
- **Trojan Hippo:** use memory insertion, 0–100 benign-session delay, summary/embedding/RAG propagation and sensitive-topic activation.
- **MasDrift:** preserve coordination topologies and native authorization/utility metrics; add authority/effect sidecar.
- **OpenART:** use target-visible environment evolution and long-horizon stateful stress; never reveal its hidden safety contract to the candidate or attacker.

If an upstream benchmark lacks reproducible source or license terms, document the limitation and do not vendor its code/data.

## 7. Initial Luminous execution path

1. #540 first proves a small participant/effect loop with independent observation and response-loss reconciliation.
2. Run a frozen native subset of MasDrift and ACIArena to verify message/authority/effect instrumentation.
3. Add MetaPI-style attack generation over a small held-out source utility suite; preserve native tasks.
4. Add Trojan Hippo memory persistence and issue #551 tombstone/taint integration.
5. Add AgentLAB and OpenART long-horizon/evolving-state stress.
6. Publish a fixed, pre-registered candidate panel and held-out generator release only after evaluator and participant isolation are independently qualified.

This sequence prevents an elaborate adaptive red-team generator from becoming a false substitute for a trustworthy effect oracle.

## References

- MetaPI, arXiv:2611.10026 (5 Nov 2026): https://arxiv.org/abs/2611.10026
- ACIArena, ACL 2026: https://aclanthology.org/2026.acl-long.457/
- AgentDyn, arXiv:2602.03117: https://arxiv.org/abs/2602.03117
- Trojan Hippo Bench, AISec 2026: https://doi.org/10.1145/3847352.3848090
- Agent Security Bench, ICLR 2025: https://proceedings.iclr.cc/paper_files/paper/2025/hash/5750f91d8fb9d5c02bd8ad2c3b44456b-Abstract-Conference.html
- Existing ATR runner: issue #540.
- Existing causal authority graph: issue #543 / PR #542.
