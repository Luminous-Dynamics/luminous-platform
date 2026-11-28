# Agent Security Benchmark Methodology Crosswalk for ATR v0.1

**Status:** Research crosswalk. It is not an implementation of an IETF standard, certification scheme or compliance attestation.

**Source framework:** [draft-han-bmwg-agent-security-benchmark-00](https://datatracker.ietf.org/doc/draft-han-bmwg-agent-security-benchmark/), published 5 July 2026, intended Informational, expires 6 January 2027. This is an Internet-Draft and has no standards authority merely by publication. Recheck the Datatracker version/status before using the crosswalk in an external submission.

## 1. What the draft contributes

The draft defines a broad agent-security evaluation framework with four top-level dimensions and 55 second-level metrics:
- Model-Native Security;
- Interaction Security;
- Operational Security;
- Basic Security.

Its proposed assessment methods cover static evaluation, dynamic evaluation, attack/defense evaluation, compliance evaluation and quantitative evaluation. It explicitly includes model, interaction, memory, planning, tool execution, identity/permission, runtime and infrastructure concerns.

The draft's second-level measures use pass-rate definitions, with timeouts counted as failures. It proposes hierarchical weighted arithmetic means to produce a 100-point overall score and risk bands.

ATR should reuse this broad lifecycle coverage and keep compatible pass-rate reporting where useful. It should not use a weighted aggregate as the only qualification or release criterion.

## 2. Crosswalk from the four dimensions to ATR evidence

| Draft dimension | Typical evaluation surfaces | ATR contribution | Required evidence |
|---|---|---|---|
| Model-Native Security | Robustness, adversarial inputs, safety alignment, calibration, long-context reasoning | Keep upstream model-native evaluation as a separate baseline; measure whether model errors can actually cross a protected boundary | Frozen model/configuration, task/seed, prompt/context digests; no claim that a model-level score proves runtime authorization |
| Interaction Security | User input, RAG, external content, MCP/plugin interfaces, cross-agent communication | Provenance-bearing input and memory graph; WPT/identity binding; taint propagation through messages, tools, summaries and agent handoffs | Source identity, trust class, content/action digests, authenticated workload, causal lineage, audience and tenant |
| Operational Security | Planning, long-term memory, task decomposition, multi-round execution, tool scheduling, autonomous iteration | Source-anchored P0, exact task/plan bindings, delegated capability attenuation, current status, commit-time PEP, unknown-outcome lock and independent effect receipt | Authority/mandate chain, branch assignment, invocation binding, policy/fencing epoch, participant commit record and observer receipt |
| Basic Security | Runtime, infrastructure, sandbox, storage, API credentials, identity and access control | Participant boundary, evaluator isolation, tenant/resource scoping, atomic budget/idempotency state, key/status lifecycle and observer separation | Exact runtime/container/kernel/config digest, trust-domain separation, participant transaction and raw observer evidence |

This mapping is additive. ATR is not a replacement for system hardening, model assessment, compliance reviews, privacy governance, secure sandboxing or vendor-specific threat modeling.

## 3. Why ATR does not collapse everything into one weighted score

The draft's 100-point weighted score is useful for a broad posture summary, but it can obscure non-compensable conditions. For example, high prompt robustness, compliance pass rates or model calibration cannot compensate for one unauthorized bank transfer, cross-tenant disclosure, revoked-capability commit or false effect receipt.

ATR therefore publishes two layers:

1. **Coverage/profile scorecard:** benchmark-compatible pass rates across the draft's four dimensions, with numerator, denominator, timeout/unknown count and confidence interval.
2. **Non-compensable qualification gates:** separately reported hard constraints for the declared protected-action profile.

Hard gates proposed for high-impact synthetic actions:
- no unauthorized committed effect;
- no cross-tenant read/write/export;
- no commit after the participant acknowledges the revocation/fencing barrier;
- no accepted wrong-role authority or mandate assertion;
- no task/plan/capability widening beyond root authority;
- no false independent-effect receipt;
- no replacement action/idempotency key while a prior consequential outcome remains unresolved.

A system that fails any hard gate cannot be assigned a "safe" overall grade solely because its weighted average is high. Utility remains a separate dimension; blocking all actions is not a successful defense.

## 4. Benchmark portfolio and construct coverage

No single benchmark covers the full lifecycle. ATR should preserve each benchmark's native tasks and scorer, then add an evidence sidecar.

| Research / benchmark | Main construct | ATR-specific gap addressed |
|---|---|---|
| AgentDojo / InjecAgent / AgentHarm | Tool-use injection and harmful compliance | Actual commit, authorization lineage and cross-tenant boundary |
| AgentLAB (ICML 2026) | Adaptive long-horizon attacks: intent hijack, chaining, drift, task injection, memory poisoning | Exact authority/branch lineage and protected-resource effects |
| Trojan Hippo Bench (AISec 2026) | Dormant memory payloads, persistence up to 100 benign sessions, memory backends/defenses | Taint-preserving provenance, trust non-escalation, revocation/tombstones and exact approval |
| MasDrift v3 | Multi-agent coordination and authorization preservation | Cross-domain Authority Assertions, workload identity and participant finality |
| OpenART | Evolving stateful long-horizon execution and runtime attack surfaces | Root authority, commit-time fencing and independent causal effect receipt |
| draft-han-bmwg-agent-security-benchmark-00 | Broad lifecycle metric/method framework (4 dimensions, 55 metrics) | Evidence-backed non-compensable gates, distinct attempt/execute/commit/observe states |

The benchmark portfolio must be stratified by:
- task horizon and dependency depth;
- parallel width and number of agents;
- memory persistence interval;
- number and type of cross-agent handoffs;
- resource criticality and reversibility;
- authority chain depth and issuer/trust-domain count;
- revocation timing and status freshness;
- tool/connector/network/runtime boundary;
- model and defense family.

A pooled average across these axes is descriptive only. Release decisions use the exact profile and criticality class.

## 5. Statistical and anti-gaming contract

Every report should provide:
- exact task corpus and split digests;
- candidate/attacker/judge/evaluator model IDs and versions;
- runtime, policy, prompt, tool and memory backend configuration;
- pre-registered exclusions, retry policy, budgets and timeout behavior;
- always-allow, always-deny and always-positive baselines;
- confusion matrix, per-family precision/recall, false-positive and false-negative rates;
- attempt, denial, execution, commit, independent observation and unknown counts separately;
- task/seed clustered confidence intervals, not naive per-tool-call independence;
- p50/p95/p99 latency, token and compute cost, operator intervention and reconciliation burden;
- all omitted, timed-out, indeterminate and unreconciled cases.

The evaluator must be isolated from the candidate. LLM judges can remain as benchmark-native labels but cannot serve as the sole source of authorization or protected-effect ground truth.

## 6. Immediate implementation mapping

- PR #542 contains the authorization-assurance, causal graph, cross-domain mandate, Trust-Framework, persistent-memory and benchmark adapter profiles. Issue #551 tracks the executable memory-store/tombstone integration.
- PR #548 contains the pure Rust delegation/root-reanchor predicates and the synthetic participant-finality reference model.
- #540 remains the first executable runner milestone: an isolated synthetic resource, candidate/evaluator separation, participant enforcement, fault injection and independent effect receipt.
- #550 tracks WIMSE Trust-Framework Section 12.2 item 11 (instruction evidence) and item 12 (shared vs allocated RP budgets).
- The Trojan Hippo adapter adds a memory-provenance corpus with 14 cases and 13 mutations; it is a synthetic contract, not upstream benchmark execution.

Do not make the first #540 vertical slice wait for complete integrations of AgentLAB, Trojan Hippo or OpenART. Use the first executed participant/effect loop as the adapter contract.

## References

- draft-han-bmwg-agent-security-benchmark-00, Datatracker: https://datatracker.ietf.org/doc/draft-han-bmwg-agent-security-benchmark/
- AgentLAB, ICML 2026: https://proceedings.mlr.press/v306/jiang26as.html
- Trojan Hippo Bench, AISec 2026: https://doi.org/10.1145/3847352.3848090
- MasDrift v3: https://arxiv.org/abs/2608.07556
- OpenART benchmark landscape: https://ai45lab.github.io/OpenART/benchmarks/
- ATR benchmark validity roadmap in this branch.
