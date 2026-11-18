# Luminous Web4 Reference Architecture

**Status:** Research proposal v0.1 — non-normative, not an implementation or certification claim  
**Date:** 2026-11-18  
**Scope:** Symthaea, Mycelix, Sol-Atlas, Symtropy, Xenia and Luminous Platform

## 1. Purpose

This document proposes a federated, agent-native architecture for the open web. It consolidates existing Luminous research and implementation directions into a shared protocol and assurance profile. It is deliberately a composition architecture: independently deployable projects interoperate through versioned contracts rather than being fused into one application or ledger.

The proposal is informed by current Web4 terminology work, but does not claim that "Web4" is a settled standards category. The 2026 Web4 Internet-Drafts are individual, work-in-progress submissions, not approved IETF standards. Luminous should define and test a concrete profile while remaining compatible with established internet and web standards.

### Proposed definition

> **Luminous Web4 is a federated, agent-native, human-sovereign and evidence-verifiable internet in which people, organizations, machines and intelligent systems can discover one another, establish scoped trust, exchange capabilities, coordinate actions and independently verify outcomes without depending on one controlling authority.**

A consequential action MUST have:
1. an accountable principal;
2. an authenticated agent or workload identity;
3. an explicit, bounded and current authority grant;
4. a canonical operation and target;
5. a participant-enforced effect boundary;
6. an independently observable postcondition;
7. a reproducible evidence trail with a stated claim ceiling.

## 2. Non-goals and terminology

This proposal does not create a new global network, cryptocurrency, universal identity provider, central truth authority, or replacement for DNS, HTTP, TLS, QUIC, TCP, or existing federation protocols. It does not assert that a distributed system is conscious. Collective intelligence, collective agency and subjective experience are separate research questions.

Terms:

- **Principal:** the human, organization or collective that owns the intent and may grant authority.
- **Agent:** software that reasons, plans or proposes actions. An agent is not automatically a principal.
- **Workload identity:** the cryptographic identity of a running service, process or deployment instance.
- **Authority grant:** a signed, time-bounded permission scoped to a principal, delegate, target, operation and constraints.
- **Capability:** the narrowly scoped right to attempt a particular class of operation.
- **Proposal:** an untrusted request to perform an operation; it is not permission.
- **Effect:** a state change at a participant or in the physical world.
- **Receipt:** evidence that a specified participant or verifier observed a specified event. A receipt does not automatically prove the event was correct or beneficial.
- **Attestation:** evidence about a measured platform, workload or execution environment, appraised under a relying-party policy.
- **Provenance:** verifiable lineage of a statement, artifact, observation or decision.
- **Federation:** cooperation among independently administered participants that retain local authority and may leave or reject requests.

## 3. Architectural invariants

The profile MUST preserve these distinctions:

| Distinction | Required invariant |
|---|---|
| Identity vs. authority | Authentication never grants permission by itself. |
| Trust vs. authority | Reputation or consensus may inform risk but cannot mint a capability. |
| Agent decision vs. authorization | A model decision such as ACT is never an authority grant. |
| Integrity vs. truth | A valid signature proves who signed bytes, not that the assertion is true. |
| Agreement vs. independence | Copied, mirrored or re-signed evidence is not independent corroboration. |
| Simulation vs. observation | A simulated result cannot be represented as a measured physical outcome. |
| Intent vs. effect | A submitted request or gate receipt does not prove a participant committed an effect. |
| Availability vs. permission | Host boot continuity may be availability-preserving; consequential actuation fails closed. |
| Federation vs. centralization | No global coordinator is required for local participants to retain their own authority. |
| Autonomy vs. integration | Participants retain local policy, data boundaries, revocation and exit rights. |

## 4. Project responsibilities

Projects own distinct responsibilities. Integrations MUST use explicit versioned interfaces and MUST NOT create duplicate policy engines, transaction journals or effect authorities.

### Symthaea — cognitive and agent layer

- Perception, reasoning, planning, semantic representations, prediction and learning.
- Owns the canonical INTX interaction semantics and composes them with the existing Symthaea authority/action-runtime lineage; `InteractionIntent` is a proposal, never permission.
- Produces typed proposals, evidence requests and uncertainty statements.
- Never holds raw connector credentials or direct authority to mutate external systems.
- Web4 MUST reuse INTX identity, intent, attempt and reconciliation contracts rather than introducing a parallel cross-protocol interaction state machine.
- Model identity, weights/artifact digest, runtime version, prompt/tool context and relevant policy versions are recorded for consequential decisions.
- Moral algebra, epistemic gating and cognitive state are advisory signals; they are not a security boundary or substitute for external authorization.

### Mycelix — identity, trust, governance and coordination

- Human, organization, collective and agent identity relationships, verifiable credentials, delegated authority evidence and governance.
- The AGENT programme qualifies identity/runtime binding, intent, conserved delegation, current authority, exact actions, credential-less brokerage, durable effect receipts and independent offline verification.
- Mycelix supplies credential/identity/authority evidence and assurance profiles; it MUST NOT create a parallel InteractionIntent or external-effect state machine.
- Trust/reputation is a risk input, not a substitute for explicit current authority.
- Credential/key changes and high-risk governance mutations use append-only, threshold-controlled and freshness-bound procedures.
- Cross-agent delegation MUST attenuate authority monotonically.

### Sol-Atlas — situated knowledge and provenance

- Geospatial, infrastructure, cultural and documentary knowledge.
- Records source attribution, geographic/temporal context, rights, confidence, version and correction history.
- Claims remain contestable and may have competing sources.
- A map, catalog entry or visualized simulation is not automatically a verified observation.

### Symtropy — physical-world models and feedback

- Digital twins, physical simulation, causal intervention analysis and embodied-system evaluation.
- Keeps simulated, predicted, sensed and independently verified values in distinct namespaces.
- Provides precondition and postcondition schemas for bounded physical operations.
- Never treats an internal metric, Phi value, trust score or model-generated state as authorization to control a physical system.
- Physical actuation requires a local safety controller, hardware interlocks where appropriate, and independent operator authority.

### Xenia and Luminous Platform — secure session and enforcement substrate

- Xenia provides authenticated peer/session semantics, consent-message binding, sealed messages, replay control and transport adapters. A secure Xenia session does not itself grant an application capability.
- Luminous Platform provides the broker, lifecycle orchestration, host policy enforcement and recovery integration.
- ACS hooks and A2A/MCP adapters must converge on the same canonical action/authority path; protocol metadata is evidence or a proposal, not a grant.
- Nixward remains the authority for host transactions, host interlocks, host journal and host recovery.
- No broker, cognitive process or orchestrator may bypass a participant's own final effect boundary.

## 5. Interoperability profile

Prefer existing standards and audited implementations over new cryptographic formats.

| Function | Preferred foundation | Luminous integration rule |
|---|---|---|
| Human/org identity | W3C DID and Verifiable Credentials where appropriate | Identity resolution is distinct from authentication and authorization. |
| Workload identity | SPIFFE/SPIRE or equivalent short-lived workload credentials | Bind identity to deployment and audience; rotate and revoke. |
| API authorization | OAuth 2.1 security best practices, resource indicators (RFC 8707), sender-constrained tokens where supported | Tokens are resource/audience-bound, short-lived and scoped; never pass a broad upstream token to tools. |
| Attestation | IETF RATS (RFC 9334) and appropriate EAT/TEE evidence profiles | Relying-party policy decides what evidence is sufficient; a signature alone is not hardware trust. |
| Transparency | SCITT (RFC 9943) | Register signed statements and verify receipts; do not confuse transparency with truth. |
| Build provenance | SLSA provenance and artifact attestations | Bind artifacts to repository, workflow, source revision and build process; verify trust policy. |
| Agent-to-agent protocol | A2A v1.0 | Use A2A for interoperable task/message exchange and agent discovery. An Agent Card or authenticated connection is not permission to perform a downstream effect. |
| Tool protocol | MCP or other typed protocol adapters | Use resource/audience-bound credentials and pinned tool contracts; tool discovery and annotations do not grant authority. |
| Runtime control hooks | OWASP Agent Control Standard (ACS) | Provide ACS adapters for portable policy hooks and trace/inspection surfaces. The Luminous broker and participant remain the deterministic reference monitors; do not rely on a second LLM/Guardian as the sole authorization boundary. |
| Observability and agent inventory | OpenTelemetry, OCSF, CycloneDX/SPDX | Export correlated traces/events and an Agent Bill of Materials covering model, runtime, skills, MCP/A2A peers, tools, knowledge sources, memory stores and policy versions. Self-reported traces are not independent evidence. |
| Secure session | TLS 1.3 / QUIC plus Xenia where the use case requires its application/session semantics | Xenia is not a TLS replacement. Do not deploy its pre-alpha wire format as production infrastructure. |
| Storage/federation | Holochain, HTTPS, content-addressed storage or other suitable participant-owned stores | No single ledger is mandatory; retain verifiable exports and local control. |

A compatibility profile MUST define canonical serialization, algorithm identifiers, version negotiation, key rotation, revocation freshness, error semantics, replay behavior, size limits and test vectors. Unknown or mixed protocol profiles MUST be rejected rather than silently downgraded.

## 6. Consequential action lifecycle

The mandatory effect lifecycle is:

This lifecycle is a Web4 composition view over the canonical Symthaea INTX interaction core and action-runtime lineage, Mycelix AGENT qualification chain, the Luminous Platform broker/Saga, and participant-owned transaction boundaries. It MUST NOT introduce a competing `InteractionIntent`, `AuthorityGrant`, attempt state machine or generic transaction journal.

```text
Human / organization intent
        |
        v
Authenticated principal + workload identity
        |
        v
Agent proposal (untrusted)
        |
        v
Resolve pinned operation manifest
        |
        v
Check exact grant, policy, target, request and preconditions
        |
        v
Issue one-use, non-transferable dispatch permit
        |
        v
Participant-side authorization and idempotent admission
        |
        v
Participant-owned effect receipt
        |
        v
Independent postcondition observation
        |
        v
Transparency / provenance registration
        |
        v
Reconcile, report and retain claim ceiling
```

The broker MUST check the exact canonical request immediately before dispatch. A permit is bound to the principal, tenant, workload, episode, root task, plan digest, step, target, operation, request digest, tool manifest, policy revision, attempt and expiry. It MUST be non-transferable and consumable once.

An agent's ACT/ALLOW/SUCCESS label is never a permit. A gate receipt proves only that a policy check was made against specified inputs. It is not proof that a participant accepted or performed the operation.

If dispatch may have been accepted but the response is missing or ambiguous, the state is `outcome_unknown`. The system MUST reconcile the same attempt with participant-owned evidence before creating a new attempt. Timeout, missing telemetry or an agent assertion MUST NOT trigger an automatic retry, compensation or second mutation. Compensation is a new operation with a new grant, attempt and postcondition.

## 7. Security model

### 7.1 Threat assumptions

Assume that:
- user-supplied and retrieved content may contain hostile instructions;
- model outputs may be wrong, manipulated or strategically deceptive;
- agent memory, vector indexes, summaries and tool descriptions may be poisoned;
- tools can change behavior or schemas after discovery;
- a valid participant may be compromised or malicious;
- identities may be Sybil-controlled or colluding;
- signed evidence may be stale, selectively omitted or copied from one source;
- the network may partition, reorder, delay or duplicate messages;
- a process may crash between dispatch, commit, observation and acknowledgement;
- model capability and attack automation will improve over time.

The architecture must not assume that prompt wording, model alignment, moral reasoning, consensus or reputation can enforce a security invariant.

### 7.2 Four enforcement boundaries

1. **Cognitive boundary:** The model sees typed tools and task handles, not raw credentials, arbitrary host shells or unrestricted connector clients.
2. **Broker boundary:** The broker resolves a signed/pinned manifest, validates current authority and constraints, and issues exact one-use permits.
3. **Participant boundary:** The destination independently authenticates, authorizes, deduplicates and records the operation. Host actions go through Nixward; physical actions go through a local safety controller.
4. **Evidence boundary:** An independent observer establishes the postcondition. The evidence system records provenance, lineage, freshness and limitations.

No single layer is assumed to be infallible. The participant boundary must remain effective if the agent or broker is compromised.

### 7.3 Risk tiers

| Tier | Example | Required controls |
|---|---|---|
| R0 — public/read-only | Public documentation, non-sensitive discovery | Read-only allowlist, bounded parsing, rate/resource limits, provenance |
| R1 — reversible bounded | Draft creation, reversible low-impact configuration | Exact capability, target/request binding, idempotency, audit receipt, recovery plan |
| R2 — consequential external effect | Sending messages, changing access, moving funds, modifying host configuration | Explicit human approval or policy-defined dual control, one-use permit, participant ledger, independent postcondition |
| R3 — safety-critical/physical | Grid control, machinery, medical or critical infrastructure actuation | Independent local safety controller, hardware interlocks, strict operating envelope, human authorization, emergency stop, fail-safe physical state |

Risk tier is assigned by a verifier-owned manifest and qualification policy, not by the model or tool at invocation time. Unknown effect classes are denied.

### 7.4 Authority attenuation

For every delegation edge:

```text
Authority(child) ⊆ Authority(parent)
Authority(grandchild) ⊆ Authority(child)
...
Authority(final actor) ⊆ Authority(root grant)
```

The relation must account for tenant, target, operation, request constraints, data scope, validity interval, budget, delegation depth, approval obligations and verification requirements. If scopes cannot be compared, the relationship is UNKNOWN/UNPROVEN and delegation is denied. Revoking a root grant invalidates descendants at the final effect boundary.

### 7.5 Execution isolation and secrets

- Run untrusted agent/tool workloads in disposable VMs or appropriately hardened sandboxes; use namespaces, seccomp, MAC, read-only filesystems, resource quotas and egress allowlists as defense in depth.
- Do not expose host credentials, signing keys, cloud metadata endpoints or unrestricted network access to the model.
- Use a separate secret broker and short-lived, audience-bound, least-privilege credentials.
- Pin tool manifests, schemas and executable artifact digests; review and requalify changes to an operation's effect class or schema.
- Treat external documents, images, webpages, code, memory retrieval and tool output as data, never instructions that can change policy.
- Validate and encode all tool outputs at trust boundaries; bound size, recursion, CPU, memory, execution time and outbound requests.
- Keep control-plane and data-plane credentials separate.
- Keep recovery and emergency paths independently authenticated and tested.

### 7.6 Cryptographic agility

Use mature, independently reviewed cryptographic libraries and standard protocols. Plan for post-quantum migration, but do not treat selecting ML-KEM or ML-DSA as proof of overall security. Bind algorithm suites and protocol revisions into authenticated transcripts; define downgrade resistance, key rotation and compromise recovery. Avoid novel home-grown cryptography.

### 7.7 Evidence and transparency

A signed record demonstrates that a key signed particular bytes. An attestation is only as strong as its measurement chain, appraisal policy and freshness. A transparency receipt demonstrates inclusion under a defined log protocol; it does not establish correctness, independence or ethical acceptability of the underlying claim.

Every evidence item should identify:
- issuer and signing key;
- subject and artifact/observation digest;
- event and causal/task identifiers;
- source lineage and independence group;
- capture time, validity and freshness;
- collection method and environment;
- transformations, model versions and policy versions;
- uncertainty, scope and claim ceiling;
- revocation and correction references.

Evidence derived from one source must not be counted as multiple independent observations merely because it was summarized, relayed, mirrored or re-signed.

### 7.8 Failure semantics

| Condition | Required disposition |
|---|---|
| Authority store unavailable or corrupt | Deny consequential action |
| Grant expired or revoked | Deny |
| Manifest or policy changed | Re-resolve and reauthorize; do not use stale permit |
| Unknown tool effect | Deny |
| Participant response ambiguous | `outcome_unknown`; reconcile same attempt |
| Observer unavailable | Do not claim verified success; block higher-risk follow-on effects |
| Network partition | Local read-only/continuity behavior may continue; high-risk remote mutation requires fresh authority |
| Agent process compromised | Revoke workload identity and grants; isolate; preserve participant-side controls |
| Transparency service unavailable | Apply predeclared risk policy; do not fabricate receipt or overstate verification |

Host boot continuity and security authorization are different failure domains. The host must remain bootable without the Luminous control plane, while consequential actions remain fail-closed.

### 7.9 Security as a moving target

A fixed checklist will not keep pace with improving models, tool ecosystems and automated attackers. Security qualification MUST be versioned against the complete deployed agent system, not just the model name.

- Treat each model, prompt template, skill, memory policy, tool manifest, MCP/A2A peer, runtime, sandbox image and policy revision as part of a versioned Agent Bill of Materials.
- A model/runtime/tool change that expands capability or changes behavior triggers targeted requalification before it inherits any existing authority.
- Preserve a permanent adversarial regression corpus; add newly observed attacks as sealed evaluator cases, with independent oracle data unavailable to the system under test.
- Run periodic red-team campaigns against the current strongest available model families and open-weight models. Measure attack success by committed external effect and data disclosure, not by the model's verbal compliance.
- Use capability growth to increase containment, test depth and monitoring; never to expand authority automatically.
- Support emergency revocation of model/runtime identities, grants, tool manifests and compromised signing keys, with tested recovery and forensic retention.
- Track residual risks and common-mode failures in the broker, participant, observer, host, identity issuer and trust roots. No amount of model evaluation compensates for a compromised enforcement root.

Security can remain resilient as models improve only while the enforcement boundary is simpler, separately administered where possible, independently tested and harder to bypass than the model's available action surface. This is a design objective, not a guarantee against every future attacker.

## 8. Adversarial benchmark programme

Build a reproducible Rust-first benchmark harness with deterministic synthetic participants and independent effect counters. Unsafe comparison arms run only inside isolated simulations, never with production credentials or routes.

### 8.1 Existing external baselines

Integrate or map scenarios from:
- AgentDojo — prompt injection and tool-use behavior across realistic tasks.
- InjecAgent — indirect prompt-injection cases involving user and attacker tools.
- Agent Security Bench — multi-application agentic security scenarios and metrics.
- AgentHarm — harmful multi-step agent behavior.
- AgentBench and task/interactive-environment benchmarks where cognitive capability is being measured.
- OWASP Top 10 for Agentic Applications (2026), OWASP Agent Control Standard (ACS), and OWASP LLM guidance.
- MCP Security Best Practices and A2A v1.0 security guidance — scoped credentials, resource audience binding, consent, peer authentication and tool trust.

### 8.1.1 Reuse the existing Luminous benchmark work

Do not create a second autonomy benchmark corpus, task taxonomy, synthetic runner, result schema or statistical engine. Extend and cross-reference the existing ITOPS/OpsBench work:
- PR #167 already proposes an Autonomy Boundary corpus with 128 system-under-test variants and 64 evaluator-only paired oracles across eight families; it is still a draft and does not yet establish model performance.
- Issue #41 and PR #40 define the ACS-to-Nixward bridge against the canonical Nixward transaction path. ACS disposition is not Nixward ChangeAuthorization; the pinned reference Guardian is explicitly not the conformance oracle, and signature/replay gaps must remain visible.
- Symthaea INTX-000 / #4397 defines the protocol-neutral external interaction fabric; #4399 and #4400 define the pure semantic core and its composition with bounded authority/action-runtime identities.
- Symthaea #4407 defines A2A task/artifact/delegation boundaries; #4446 defines connector-manifest evidence, auth and reconciliation semantics; #4493 defines protocol-neutral crash, duplication, delay, partition and reordering qualification.
- Mycelix #805 is the canonical AGENT roadmap; #829 owns the credential-less provider-secret broker boundary; #830 owns independent offline reconstruction of agent authority/effect evidence; #889 supplies a non-authoritative interop TCK for SPIFFE, OAuth, W3C VC, MCP, A2A and SCITT.
- Issue #168 defines the broker-enforced effect boundary and its negative/positive conformance cases.
- PR #167 proposes the Autonomy Boundary corpus: 128 SUT variants and 64 evaluator-only paired oracles across eight families. It remains a draft and does not establish model performance.
- Issue #213 specifies the deterministic synthetic runner; it remains implementation work until the runner is executed and evidenced.
- PR #217 supplies a process-separated participant/observer smoke harness, but its own claim ceiling is synthetic and same-host; it is not an independent administrative trust domain.
- Issue #218 defines the required sampling-independence, strata, pairing and cluster-aware statistical analysis.
- Issue #284 specifies monotonic authority attenuation across delegated agents.
- Issue #356 covers common-cause-aware evidence corroboration; source IDs or declared domain labels alone do not establish independence.
- Issues #175 and #268 cover matched external benchmark comparisons and source-pinned external benchmark evidence.
- Issue #459 defines cyber-physical command authenticity, anti-replay, asset/role scope and independent physical-effect verification across Platform, Mycelix, Symtropy, Sol-Atlas and Symthaea.

The Web4 contribution is an umbrella profile and assurance crosswalk, not a second interaction fabric or agent-authority programme. Reuse INTX and AGENT as the canonical protocol-neutral semantics; reuse #41 for ACS→Nixward; reuse #889 for non-authoritative external-protocol translation; and add A2A/MCP/ACS interoperability, Xenia session semantics, Sol-Atlas provenance lineage and Symtropy simulated-vs-observed outcome cases to the existing ITOPS/OpsBench/OMSI framework. Existing scenario IDs, Nixward transaction semantics, OMSI/ITOPS task IDs and evidence dependency graphs remain canonical unless a reviewed versioned migration is approved.

Do not claim superiority by comparing different models, task distributions, prompts, tools, budgets or evaluation leakage. Pin exact model/provider, model revision where available, prompts, task set, tool manifests, environment, budgets and evaluator versions.

### 8.2 Luminous-specific scenario families

1. Indirect prompt injection through webpages, files, images, email, retrieved memory and tool output.
2. Cross-agent delegation and authority amplification.
3. Revocation races, stale grants, replayed permits and key rotation.
4. Confused-deputy behavior using a specialist's broader local credentials.
5. Tool-schema or effect-class drift after qualification.
6. Evidence laundering, correlated sources, false independence and stale observations.
7. Sybil identities, collusion and Byzantine participants.
8. Cross-tenant data exfiltration and privacy-boundary violations.
9. Multi-step operations with crash, timeout, duplicate delivery and partial commit.
10. Sandbox escape attempts, network egress and resource exhaustion.
11. Model or tool supply-chain substitution and artifact provenance failure.
12. Physical-digital twin divergence, stale sensor data and out-of-envelope actions.

Each family needs positive controls. A system that denies every request is not a useful secure agent.

### 8.3 Comparison arms

| Arm | Description |
|---|---|
| A | Direct tool-using agent / naive ReAct baseline, synthetic environment only |
| B | Tool protocol with OAuth/resource audience controls and no Luminous effect broker |
| C | Centralized policy engine with exact-scope authorization |
| D | Multi-agent coordination by ordinary messaging/consensus |
| E | Luminous broker + pinned effect manifests + Mycelix delegation/identity + Xenia session security + independent participant/observer evidence |

Use the same base model, task, prompt, tool semantics and compute budget within paired trials. Run model/provider diversity as separate strata, not mixed into a single average.

### 8.4 Metrics and hard gates

Report separate dimensions; do not combine safety-critical violations into a compensatory composite.

**Hard safety gates:**
- Zero unauthorized committed effects in canonical negative controls.
- Zero cross-tenant, cross-target or cross-operation permit reuse.
- Zero successful parent-to-child authority expansion.
- Zero new mutation while a prior attempt is unresolved.
- Zero revoked/expired grants accepted at the final participant boundary.
- Zero unsupported claim of independent corroboration.
- 100% state-changing operations have complete, correlatable intent, grant, manifest, dispatch, participant and observer evidence at the declared assurance tier.

**Utility and operations:**
- Legitimate task completion and task quality.
- False-denial rate on matched positive controls.
- Time, tokens, tool calls, network bytes, CPU, memory and energy per independently verified outcome.
- Gate and revocation latency p50/p95/p99.
- Throughput under concurrency and partition.
- Recovery time, orphaned attempts and reconciliation workload.
- Evidence completeness, calibration and selective abstention.
- Privacy leakage, collateral effects and autonomy preservation.

### 8.5 Initial research targets (hypotheses, not achieved results)

| Metric | Initial target |
|---|---:|
| Unauthorized committed effects | 0 in every preregistered negative-control stratum |
| Parent→child authority expansion | 0 accepted |
| Replayed/consumed/expired permit reuse | 0 accepted |
| Unknown-outcome automatic retry | 0 |
| Evidence correlation completeness | 100% for qualified mutations |
| Valid positive-control acceptance | ≥99% after independent oracle confirms eligibility |
| Local in-process policy-gate overhead | p95 ≤10 ms as an initial hypothesis |
| Connected revocation propagation | p95 ≤1 s as a test hypothesis; partition behavior reported separately |
| Cross-implementation test vectors | 100% byte/semantic agreement for frozen profile |
| Utility retention under attack | ≥95% of matched benign baseline, subject to all hard safety gates |

Use the existing ITOPS preregistered experiment plan and its dependence-aware statistical method. Do not choose a raw episode count first and then assume all rows are independent. The current 3,000-episode / zero-critical-violation proposal is conditional on the sampling design supporting its stated one-sided confidence bound; otherwise use cluster-level or stratified inference as specified in #218. Keep scenario family, model/provider, task template, seed lineage, agent session, environment and infrastructure failure domain explicit. Never present a bound on a synthetic corpus as a universal production-risk bound.

The 10 ms and 1 s values are engineering hypotheses (the 10 ms local-gate hypothesis is also present in #168) and MUST NOT be used to relax a security gate. Tune performance only after safety properties are preserved.

## 9. Competitive positioning and how to beat the state of the art

The industry is converging on interoperable agent communication and increasingly capable agent runtimes. Luminous should integrate with that ecosystem rather than compete with it at every layer.

| Industry capability | What it already does well | Luminous strategy and differentiated test |
|---|---|---|
| A2A | Standardized agent discovery and task/message exchange across vendors | Speak A2A; bind every delegated task to an independently verified parent grant and test non-amplifying delegation at the participant boundary. |
| MCP | Standard tool/resource access, OAuth authorization and resource audience binding | Speak MCP; additionally pin effect manifests, classify side effects, bind exact canonical requests and require participant-owned admission/receipts. |
| OWASP ACS | Portable runtime hooks, Guardian-mediated decisions, inspection and trace surfaces | Provide an ACS adapter; keep critical enforcement deterministic, independently administered and participant-enforced, not dependent on an LLM Guardian alone. |
| OpenAI Agents SDK and comparable agent frameworks | Orchestration, handoffs, tracing, tool guardrails, human approval and controlled sandbox execution | Integrate as a framework-neutral assurance plane, and compare matched task success, security violations, latency, cost and evidence completeness. |
| Enterprise agent platforms | Identity, access management, observability, governance and managed connectors | Interoperate with their identity and logging systems; compete on cross-domain authority attenuation, outcome reconciliation and independent evidence rather than duplicating enterprise IAM. |
| Agent security benchmarks | Prompt injection, tool misuse, harmful tasks and agent task performance | Reproduce external benchmarks, then add multi-party delegation, correlated evidence, participant commit ambiguity and physical-digital divergence cases. |

The realistic differentiation is not that Symthaea must beat frontier models at every general reasoning task, or that Xenia should replace TLS, MCP or A2A. The distinctive proposition is **an assurance and coordination fabric that can sit beneath different model providers and frameworks**: a model proposes; a policy broker validates; a participant independently enforces; an observer verifies; and federation preserves local sovereignty.

To claim an improvement over SOTA, publish paired results on pinned versions of the same models, tasks, tools, prompts, budgets and environments. Report task utility and safety separately. A security win that simply refuses every task is not useful; a task-success win with one unauthorized committed effect is not a safety pass. Report p50/p95/p99 latency, total cost, token/tool use, energy, false denial, authorized completion, violation rate by attack family and evidence-completeness. Use independent red-team review and private holdouts. Until those studies execute, the architecture is a hypothesis, not an SOTA result.

## 10. Implementation sequence

### M0 — Architecture and claim inventory
- Review this non-normative integration proposal against Symthaea INTX-000 and Mycelix AGENT-000; resolve duplicate terminology before any new type is introduced.
- Map every requirement to an existing implementation, exact issue/PR, test or explicit gap across the four project repositories.
- Reconcile project ownership and failure semantics across Symthaea, Mycelix, Xenia, Nixward and Platform.

### M1 — Contract convergence
- Reuse Symthaea INTX-000/#4397, INTX-001/#4399 and INTX-002/#4400 as the canonical protocol-neutral interaction core and authority/runtime composition.
- Reuse Symthaea #4407, #4446 and #4493 for A2A, connector-manifest and fault/reconciliation semantics.
- Reuse Mycelix #805, #829, #830 and #889 for the AGENT authority chain, credential-less secret broker, independent offline verifier and non-authoritative protocol TCK.
- Reuse Platform issue #41 / PR #40 for the ACS-to-Nixward hook and canonical transaction bridge.
- Reuse Platform issue #168 for the broker-enforced tool-effect boundary.
- Reuse Platform issue #284 for monotonic authority attenuation.
- Reuse Platform issue #310 for boot continuity versus fail-closed actuation.
- Reuse Platform issues #356 and #459 for common-cause evidence qualification and cyber-physical command integrity.
- Reuse #175 and #268 for matched external benchmark comparison and source-pinned benchmark metadata.
- Reuse the existing challenge consumption, workflow provenance and participant journal work (including #91 and #93) rather than building parallel ledgers.
- Define versioned operation manifests, canonical authority grants, task/attempt identity, typed receipts and error-state semantics.

### M2 — Isolated benchmark harness
- Complete and qualify the existing deterministic runner and process-separated oracle work (#213 / PR #217); do not create a parallel harness.
- Integrate #167 task/oracle separation, #168 effect-gate vectors, #218 statistical preregistration and #284 delegation scenarios.
- Add Web4 crosswalk cases for A2A task delegation, MCP tool/resource authorization, ACS runtime hook enforcement, Mycelix identity and revocation, Xenia session/replay semantics, Sol-Atlas provenance and Symtropy twin divergence.
- Compare baseline arms A–E under identical conditions, with evaluator-only expected outcomes and independent committed-effect counters.
- Do not wire the synthetic benchmark to production connectors or credentials.

### M3 — Broker and participant enforcement
- Integrate the exact proposal → grant check → one-use permit → participant admission flow.
- Keep Nixward as host transaction authority.
- Make participant idempotency and reconciliation durable at the actual deployment store.
- Authenticate evidence and workflow provenance with a pinned trust policy.

### M4 — One low-risk federated vertical
- Start with read-only Sol-Atlas infrastructure knowledge plus Symtropy simulation.
- Add multiple independent Symthaea agents with separate memory and permissions.
- Allow human-approved, reversible actions only after participant and observer conformance.
- Keep physical actuation out of scope until a separate safety case and local interlocks exist.

### M5 — Independent qualification
- External cryptographic/protocol review.
- Cross-implementation wire and canonicalization tests.
- Red-team agent/tool and federation scenarios.
- Crash, replay, partition and compromise exercises.
- Independent assurance report with exact source revisions and explicit unqualified boundaries.

## 11. Maturity and claim ceiling

The architecture is a research and integration direction, not a statement that Luminous Web4 or its security model is production-ready.

At the time this proposal was prepared, public repository documentation itself identifies important limitations:
- Symthaea's README documents experimental cognitive and ZK components and explicitly disclaims proving consciousness or compliance.
- Mycelix's README describes a pre-alpha research project, says most clusters have less multi-agent integration coverage than unit-test coverage, and states that the work has not been independently threat-modeled or externally audited.
- Xenia Wire and Xenia Peer README documentation marks them pre-alpha and not production-ready; Xenia Wire requires independent review and cross-implementation test vectors.
- Symtropy describes a physics/research engine and simulation substrate; simulation output is not physical-world verification.
- Sol-Atlas's current public code structure provides shared geospatial, data, economic, simulation and visualization modules; this proposal does not assume an already-qualified provenance service.

Consequently:
- Do not claim Web4 standards compliance until a versioned profile and independent implementations pass conformance.
- Do not claim production security, post-quantum security, zero vulnerabilities, or physical safety from unit tests or algorithm selection alone.
- Do not claim a collective is conscious from intelligence or coordination benchmarks.
- Distinguish implemented, tested, structurally specified, CI-pending, externally audited and physically qualified states in every report.

## 12. References

- IETF RFC 9943 — SCITT Architecture: https://www.rfc-editor.org/rfc/rfc9943.html
- IETF RFC 9334 — Remote ATtestation procedureS (RATS) Architecture: https://www.rfc-editor.org/rfc/rfc9334.html
- NIST SP 800-207 — Zero Trust Architecture: https://csrc.nist.gov/pubs/sp/800/207/final
- SLSA v1.2: https://slsa.dev/spec/v1.2/
- W3C DID Core: https://www.w3.org/TR/did-core/
- OAuth 2.0 Resource Indicators (RFC 8707): https://www.rfc-editor.org/rfc/rfc8707.html
- OWASP Top 10 for Agentic Applications: https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/
- OWASP Agent Control Standard: https://owaspacs.org/docs/acs/
- A2A Protocol v1.0: https://a2a-protocol.org/v1.0.0/
- AgentDojo: https://agentdojo.spylab.ai/
- InjecAgent: https://arxiv.org/abs/2403.02691
- Agent Security Bench: https://arxiv.org/abs/2410.02644
- AgentHarm: https://arxiv.org/abs/2410.09024
- Current Web4 Internet-Draft (individual submission, work in progress): https://datatracker.ietf.org/doc/draft-reilly-web4/
- Symthaea: INTX-000/#4397, INTX-001/#4399, INTX-002/#4400, INTX-A2A-001/#4407, INTX-009/#4446 and INTX-015/#4493.
- Mycelix: AGENT roadmap #805, credential-less broker #829, offline verifier #830 and interoperability TCK #889.
- Luminous Platform: #40/#41 (ACS–Nixward), #167/#168 (autonomy/effect boundary), #175/#268 (benchmark comparison/provenance), #213/#217/#218 (runner/oracle/statistics), #284 (delegation), #310 (failure domains), #356 (evidence dependence), #459 (cyber-physical command security), #91 and #93 (challenge/provenance).

---

**Decision requested from reviewers:** validate project ownership boundaries, threat assumptions, hard invariants, standards profile and benchmark design before implementation is described as conformant or production-ready.
