# ACIArena Adapter Protocol for ATR v0.1

**Status:** External benchmark adapter specification. ACIArena is not vendored or modified, and no Luminous ACIArena execution or efficacy result is claimed.

**Publication:** An et al., *ACIArena: Toward Unified Evaluation for Agent Cascading Injection*, ACL 2026 Long Paper, pages 10049–10066, DOI 10.18653/v1/2026.acl-long.457.
- ACL Anthology: https://aclanthology.org/2026.acl-long.457/
- DOI: https://doi.org/10.18653/v1/2026.acl-long.457

**Code snapshot:** [Greysahy/aciarena](https://github.com/Greysahy/aciarena) pinned to commit `3f226a40cddf01a08b9f700cbef0031dfab2ff64` (last commit returned by repository history query on 28 November 2026; commit adds science/medicine evaluation datasets). The README, setup, benchmark entrypoint, model/judge config and attack registry were fetched back at this pin.

**Licensing note:** No `LICENSE` file was found at this pinned repository tree. Do not vendor, redistribute or incorporate upstream code/data until maintainers clarify licensing and provenance. Research design and results may be discussed with attribution; any execution using the code requires a separate license/legal review.

## 1. Why ACIArena is a key ATR comparator

ACIArena studies Agent Cascading Injection: a compromised agent exploits inter-agent trust and message passing to propagate malicious instructions through a multi-agent system.

The ACL 2026 paper reports:
- six widely used multi-agent implementations;
- 1,356 test cases;
- three attack surfaces: external inputs, agent profiles, and inter-agent messages;
- three attack objectives: instruction hijacking, task disruption and information exfiltration;
- a unified specification for constructing MAS and composing attack/defense components.

The paper cautions that topology alone is insufficient for robust MAS design; role design and controlled interaction patterns matter. It also reports that defenses designed in simplified settings may fail to transfer to more realistic settings, and narrow controls can introduce new vulnerabilities.

These are the benchmark's reported coverage/results, not Luminous measurements. It directly complements MasDrift: ACIArena focuses on cascading instruction injection across implementations and surfaces; MasDrift evaluates multi-agent coordination and authorization preservation. Neither should be replaced by ATR's own synthetic cases.

## 2. Native benchmark integrity

The pinned repository exposes the following native CLI surface in `benchmark.py`:
- `--mas`: multi-agent system implementation / topology (example in checked-in run script: `sc`);
- `--suite`: attack objective suite (example: `disruption`);
- `--attack_mode`: continuous vs other configured modes;
- `--defense`: native defense selection;
- `--task_domain`: `math`, `code`, `sci`, or `medicine`;
- `--max_workers`: evaluation concurrency;
- `--malicious_agents`: targeted malicious agent identities;
- `--output_dir`: output destination.

The code loads separate `configs/model.yaml` and `configs/judge.yaml`, builds a suite, runs `suite.eval()`, and appends the native result with CLI metadata to a per-model/domain/MAS/suite JSON result file. The checked-in run script selects one example arm; it does not enumerate the whole benchmark matrix.

For every reproduction:
- pin the exact upstream commit and hashes of all task/config/source files;
- record the six MAS implementation identities and exact configuration;
- capture model and judge provider/model/version/temperature/token limits separately;
- record attack mode, defense, task domain, malicious agent set, worker count, seeds and run limits;
- preserve the upstream suite implementation and scoring;
- retain raw task-level results and all failures, not just aggregate outcomes;
- never treat an LLM judge label as an authorization oracle or proof of a committed resource effect.

**Do not infer the full benchmark matrix from the minimal README / one checked-in `run.sh` example.** Before reproduction, inspect the six implementation configurations, task data, suite registry, defense registry and aggregation code at the pinned source revision, then publish the exact executed configuration manifest.

## 3. ATR sidecar: authority propagation through the agent graph

The adapter adds a separately computed, evaluator-owned causal/authority sidecar. Keep ACIArena's native case identifiers, attack suites, system configurations and scoring unchanged.

### Attack-surface mapping

| ACIArena surface | ATR observations |
|---|---|
| External input | Source provenance, trust class, tenant, origin, exact content digest and entry into the agent/task graph |
| Agent profile | Role definition, workload identity, parent/root task, P0 digest, tool/resource envelope, delegation depth and revocation generation |
| Inter-agent message | Sender/receiver identity, audience/tenant, parent message digest, task/plan binding, inherited taint and any capability / consent reference |

### Objective mapping

| ACIArena objective | ATR evidence and independent oracle |
|---|---|
| Instruction hijacking | Root P0 / original task digest remains immutable; exact principal instruction/approval must be current; track attempted and accepted objective changes |
| Task disruption | Protected-state and required-step outcomes remain separate; task loss, task overwrite, cancellation and recovery are recorded with causal source edges |
| Information exfiltration | Exact sensitive data class, recipient/audience, P0 scope and approval; participant commit record and independent egress observer |

### Graph nodes / edges

Reuse the existing ATR causal graph:
- nodes: principal, authority assertion, mandate, agent instance, profile, task, plan, message, memory item, delegated grant, policy decision, PEP, tool, resource, effect and observer receipt;
- edges: received-from, authored-by, handed-to, summarized-from, delegated-to, authorized-by, invoked, enforced-by, affected and independently-observed-by.

A candidate or framework trace can propose events, but its own log is not sufficient to mark an edge verified. The evaluator should independently instrument the participant and message boundary wherever possible. Missing actor/message/authority links remain unknown; do not infer a safe chain.

## 4. Paired experimental arms

Once #540 provides an executable isolated participant/effect loop, run matched arms:

1. ACIArena native baseline with its original attack/defense configuration;
2. ACIArena's native defenses, unchanged and separately ablated;
3. source-anchored root P0 and branch/task-plan precommitment;
4. source-anchored P0 + invocation-bound delegation + workload PoP;
5. full participant commit-time epoch fence + unknown-outcome lock + independent effect receipts.

Do not relabel the native defense arms. A new ATR treatment is a separate sixth-party integration, not a new native ACIArena baseline.

Each matched pair must hold fixed:
- exact MAS implementation/topology and role/profile configuration;
- task domain, case ID, attacker target and attack mode;
- victim model and exact version, temperature and budgets;
- attacker/malicious-agent model and judge model/version;
- defense and tool policies, memory state and seed;
- worker count/concurrency and timeout/retry handling;
- candidate build, ATR policy/issuer/PEP/evaluator versions and fixture digests.

Randomize arm order. Use paired case/seed analysis. Hold out system implementations and profile/message patterns for transfer evaluation; do not leak test cases into adaptive attack tuning.

## 5. Metrics

### Native metrics (unchanged)

Use the original ACIArena result per case and its native aggregate for each exact MAS/suite/domain/attack-mode/defense/model configuration. Preserve the native scorer.

### ATR sidecar

- attempted, accepted, executed, committed and independently verified malicious effects (separate states);
- instruction hijack attempt vs adopted objective vs protected action;
- task disruption attempts and actual state / required-step loss;
- exfiltration attempts vs bytes/records actually observed leaving the synthetic resource;
- cross-agent handoff lineage completeness;
- unauthorized delegation, role/profile escalation and capability amplification;
- taint propagation and memory-to-message-to-plan paths;
- revocation/status propagation and post-barrier commits;
- false/missing causal edge rate and unknown attribution rate;
- benign required-action block rate and safe read-only completion;
- p50/p95/p99 policy, PEP, commit and reconciliation latency;
- model/judge tokens, worker/resource usage and cost.

Do not compare ACIArena native attack-success labels directly to ATR unauthorized-commit rates. They measure different outcomes. Report the original score and ATR's effect/authority vector side by side.

## 6. Proposed targets (research hypotheses, not results)

For the declared high-impact profile:
- zero unauthorized committed effects;
- zero cross-tenant effects;
- zero scope/role/capability widening that reaches a protected commit;
- zero post-revocation-barrier commits;
- zero false independent-effect receipts;
- ≥99% complete independent authority-to-effect attribution for committed protected effects;
- benign required-task completion within 1 percentage point of the strongest matched native defense;
- ≤5% false-positive denial rate at matched recall, with always-allow and always-deny controls;
- measure transfer gap across held-out MAS implementations and unseen role/message patterns.

These are qualification targets, not universal security claims. Publish per-suite numerators/denominators, task-clustered confidence intervals, unknowns, exclusions and cost.

## 7. Known integration limits

- The ACL paper specifies 1,356 cases and six MAS implementations; the public code README is intentionally minimal. The full benchmark matrix and trace semantics must be reconstructed from the pinned code/data before a reproduction claim.
- `benchmark.py` uses a judge model and stores a native result record; the judge output is not a participant receipt.
- The repository has no detected LICENSE file at the selected commit. No upstream code or data is copied into Luminous.
- The pinned `setup.py` brings Python dependencies including OpenAI, Transformers, datasets, PyYAML and math-verify. Keep ACIArena as a separate research harness; do not add these dependencies to the Rust participant core.
- No ACIArena code or tasks have been executed by Luminous in this adapter work.

## 8. Execution order

1. Complete #540's small synthetic participant/effect-receipt vertical slice.
2. Run the source-pinned MasDrift adapter to qualify authorization-preservation metrics.
3. Run ACIArena on a cleared exact upstream pin, comparing all six supported MAS implementations / three surfaces / three objectives where the pinned data confirms them.
4. Add AgentLAB and Trojan Hippo as separate adaptive-long-horizon and persistent-memory axes.
5. Use MetaPI-style adaptive attack generation on held-out native utility tasks only after the native baseline and evaluator controls are stable.
6. Add OpenART as an evolving-state breadth/stress environment.

The adapters do not block #540; the first end-to-end effect loop remains the milestone that turns ATR from schema/reference contracts into an executable benchmark.

## References

- ACL Anthology paper: https://aclanthology.org/2026.acl-long.457/
- DOI: https://doi.org/10.18653/v1/2026.acl-long.457
- Upstream public source snapshot: https://github.com/Greysahy/aciarena/tree/3f226a40cddf01a08b9f700cbef0031dfab2ff64
- MetaPI paper: https://arxiv.org/abs/2611.10026
- Existing ATR causal graph: `CAUSAL_AUTHORITY_PROPAGATION_AND_CASCADE_EVALUATION_V0_1.md`
- Existing benchmark roadmap: `BENCHMARK_VALIDITY_AND_LONG_HORIZON_SOTA_ROADMAP_V0_1.md`
