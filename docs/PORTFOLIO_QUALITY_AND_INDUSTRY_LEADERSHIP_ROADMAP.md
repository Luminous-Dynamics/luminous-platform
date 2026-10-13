# Portfolio Quality and Industry-Leadership Roadmap

**Status:** Proposed portfolio decision; not implementation or qualification evidence  
**Revision date:** 2026-10-13  
**Scope:** Symthaea, Mycelix, Luminous Platform, and the shared interfaces between them

## Executive decision

Do not optimize for the number of features or domains covered. Optimize for a small set of trustworthy capabilities that compound across the portfolio, can be compared with strong existing alternatives, and produce customer value.

**First product wedge:** a sovereignty-first, verifiable operations control plane for bounded IT operations. Make it interoperate with the OWASP Agent Control Standard (ACS), use Symthaea for optional reasoning and diagnosis, preserve an independent deterministic authority boundary, route permitted effects through Nixward or a narrowly scoped executor, and use Mycelix when multi-party identity, consent, provenance, or evidence coordination adds value. Luminous Platform supplies the product integration and operator experience.

This is a proposal. It does not assert that the end-to-end product, ACS conformance, production safety, Mycelix integration, or customer value has already been demonstrated.

## Why this is the highest-leverage first bet

1. **It starts from assets that already exist.** Luminous Platform already has work on typed operations, target/pre-state binding, independent verification, durable effect records, and recovery. Symthaea has evidence-oriented evaluation work. Mycelix has agent-centric identity and coordination primitives plus an explicit maturity matrix.
2. **The initial risk domain is bounded.** A controlled IT-operations workload can be tested in a disposable VM before touching production hosts, civic systems, health records, or public infrastructure.
3. **The quality claims are measurable.** We can count unauthorized effects, stale-approval rejections, replay rejections, correct post-state observations, recovery outcomes, operator time, and resource consumption.
4. **The control plane is reusable.** The same separation of proposal, authority, execution, verification, and recordkeeping can later support vendor adapters, fleet operations, utilities, public services, and other domains.
5. **There is a timely interoperability target.** OWASP's Agent Control Standard describes runtime hooks, policy decisions, tracing, and agent inventory. Its current reference implementation also documents gaps—including an unauthenticated wire and a fail-open default—so compatibility must be verified against the versioned specification, and control-plane safety must not be inferred from a demo.

Relevant current work to reuse rather than duplicate:
- Luminous Platform [#23 — global vendor adapters and Mycelix evidence plane](https://github.com/Luminous-Dynamics/luminous-platform/issues/23)
- Luminous Platform [#24 — base-update audit/status reconciliation](https://github.com/Luminous-Dynamics/luminous-platform/issues/24)
- Luminous Platform [#38 — NixOS host with OCI and cross-OS VM workload profiles](https://github.com/Luminous-Dynamics/luminous-platform/issues/38)
- Symthaea [#2217 — cross-domain adversarial cybersecurity qualification](https://github.com/Luminous-Dynamics/symthaea/issues/2217)
- Symthaea [#6800 — execution-bound context versus supplemental evidence annotations](https://github.com/Luminous-Dynamics/symthaea/issues/6800)

This roadmap does not supersede those issues or authorize duplicating an existing owner’s implementation.

## Target architecture and non-negotiable boundaries

| Layer | Primary responsibility | Must not become |
|---|---|---|
| Symthaea | Optional diagnosis, candidate generation, prediction, uncertainty reporting, and task-specific cognitive evaluation | An authority merely because it produced a convincing explanation |
| Independent Guardian / deterministic verifier | Authenticate requests; evaluate versioned policy and exact capabilities; allow, deny, request approval, or defer; emit traceable decisions | A self-certifying executor or an LLM with unrestricted access to its own policy |
| Luminous Platform / Nixward | Operator workflow, typed plans, narrowly scoped execution, pre-state checks, transaction state, post-state verification, and recovery | A broad arbitrary-shell or root capability exposed to a general agent |
| Mycelix | Where needed: participant identity, consent, provenance, multi-party evidence and governance across organizational boundaries | The sole synchronous authority dependency for local safety or host boot |
| Evidence and telemetry | Bind claims to exact source, policy, environment, artifacts, verifier, scenario, and observed result | A dashboard that upgrades queued, skipped, stale, or missing evidence into success |

### Safety rule

**Optional intelligence and platform services may fail without preventing normal host boot or recovery. Privileged mutations must fail closed when the required authorization, verifier, policy, or fresh pre-state is unavailable.**

These are different failure domains, not a contradiction:
- **Fail-open for host availability:** failure of an optional platform component does not strand a machine or block its recovery path.
- **Fail-closed for authority and effects:** no required proof, no verified permission, stale target state, unauthenticated decision, or ambiguous authorization means no mutation.
- **Indeterminate is not failure-to-apply and is not success:** after a timeout, observe and reconcile actual state before retrying a non-idempotent operation.
- **Execution acknowledgement is not verified outcome:** read the post-state through an independent observation path when the operation requires it.
- **A Mycelix or peer-network outage must not silently broaden local authority.** Cross-organization evidence may be asynchronously recorded where safe; local authorization policy remains explicit and bounded.

## Portfolio priorities

### P0 — Establish trustworthy evidence and a shared action contract

**Why first:** everything else depends on knowing exactly what was tested, what was authorized, and what actually occurred.

Work:
- Inventory current contracts in Luminous Platform, Nixward, Symthaea Evidence Fabric, and Mycelix before adding new abstractions.
- Produce a field-by-field reuse/gap matrix. Identify canonical owner, version, identity, and migration path for each shared concept.
- Standardize or explicitly map the minimum action/evidence fields: request and attempt identity; actor and tenant; target identity; operation/capability; canonical action digest; policy and verifier identity/version; fresh pre-state digest; expiry/nonce; executor build identity; authorization decision; effect-journal state; observed post-state; failure/recovery state; and artifact/source digests.
- Keep execution-bound evidence distinct from later annotations, reviews, or attestations. Bind exact source, test subject, workflow/environment, and artifact to each qualification result.
- Preserve terminal states such as `PASS`, `FAIL`, `BLOCKED`, `STALE`, and `UNKNOWN`. Pending, queued, skipped, missing, or obsolete evidence is never `PASS`.
- Use SLSA/in-toto provenance formats where applicable rather than inventing a parallel software-build attestation format.
- Add an ACS 0.1.0 interoperability and security gap matrix. Pin the version and test inputs; do not claim full standard conformance from one working hook.

Exit gate:
- An independent checker can identify the exact subject, policy, verifier, environment, test corpus, and terminal result for each claimed pass.
- Changing the target, action, relevant policy, pre-state, or tested artifact invalidates the prior authorization/qualification.
- No new cross-repository protocol is introduced until the reuse/gap review establishes that an existing contract cannot meet the need.

### P1 — Ship one verifiable operations vertical slice

**Initial slice:** read-only host/service diagnosis and proposed remediation, followed by a single low-risk reversible operation in an isolated disposable NixOS VM. Production mutation is out of scope until the lab gates pass.

Path:
1. Observe host state and identify the exact host/build.
2. Produce a typed, bounded plan and human-readable diff.
3. Bind authorization to the precise target, action digest, pre-state, policy revision, caller, and expiry.
4. Have an independent verifier evaluate the exact operation.
5. Run only the allowlisted operation through the narrow executor.
6. Record effect state durably and independently observe the post-state.
7. Reconcile timeouts, interrupted processes, restarts, and duplicate requests without blind replay.
8. Produce a privacy-aware evidence receipt the operator can inspect and export.

Integrate the applicable ACS hook and decision semantics where feasible. The policy engine must not trust a decision simply because it arrived from a process named “Guardian”; authenticate the wire according to the pinned contract, verify the response/request binding, and test the failure posture.

Exit gate:
- The same scenario can be replayed with fixed inputs and exact artifact identities.
- Every authorization is scoped to one intended operation; replay, stale pre-state, wrong target, wrong tenant, expired approval, altered arguments, and verifier loss do not cause a mutation.
- Crashes at every transaction boundary result in a safe, testable state; an indeterminate non-idempotent operation is reconciled before retry.
- Host boot/recovery remains independent of the optional UI, AI, network, or evidence service.

### P2 — Create a credible comparative evaluation

Use deterministic simulation and captured evidence first; then owner review and a controlled pilot. External human testing is not a prerequisite for core engineering qualification.

Compare the same task and hardware/environment across:
1. a competent operator using the existing NixOS workflow;
2. the existing declarative or automation workflow (for example, NixOS tooling or an applicable Ansible playbook);
3. an agent with the same tools but without the independent Guardian;
4. the Luminous/ACS-compatible guarded workflow;
5. where useful, the ACS reference implementation with its deployed policy engine, while recording the exact supported-hook and authentication scope.

Report separate metrics; do not combine them into a single opaque score:
- task success and correct final state;
- unauthorized or out-of-scope effects;
- correct rejection of each negative fixture;
- stale-context and replay resistance;
- time to safe completion and operator interventions;
- false denials and justified escalations;
- timeout, interruption, recovery, and duplicate-request outcomes;
- end-to-end p50/p95 latency, CPU, memory, and energy where measurable;
- completeness and independent verifiability of the receipt;
- privacy leakage and secrets appearing in logs;
- scope of novelty/generalization actually tested.

Pre-register scenario families, seed sets, expected outcomes, exclusions, and claim ceilings. Keep the evaluator independent from the evaluated agent and executor. Include benign controls so a blocked action is not counted as a security success unless the action's reachability/capability was established under a matched permissive control where appropriate.

Hard gates:
- Zero unauthorized mutations across the explicitly enumerated in-scope security corpus. This means zero on that corpus—not proof that all attacks are impossible.
- Every expected negative fixture must reject for the expected reason; missing or unexecuted fixtures block the run.
- All accepted actions must meet the exact authorization and post-state contracts.
- No secrets in evidence outputs; sensitive topology and customer data stay in controlled storage.
- Failures and inconclusive results remain visible in the published report.

### P3 — Bring Mycelix into a genuinely multi-party workflow

Integrate Mycelix where it adds real value: participant-controlled identity, consent, provenance, signed contribution/witness records, evidence exchange, and cross-organization governance. Do not make peer availability a hidden dependency of local action authority.

Start with one narrow, representative workflow, not the entire domain map. Prefer an already runnable and truthfully documented application or an isolated demonstrator whose build and deployment status have been verified at the exact head.

Required evidence:
- Real multi-agent integration across independent agent chains, not just unit tests or several in-process objects.
- Negative tests for invalid signatures, replay, conflicting claims, stale credentials, permission revocation, malformed records, partial connectivity, restart/recovery, and data-availability loss.
- Clear data classification: what is public, hashed-but-potentially-sensitive, encrypted/private, locally retained, or eligible for shared publication.
- A documented dispute, correction, revocation, and key-recovery path.
- Interoperability with conventional identity and policy systems; no requirement that every organization migrate to Mycelix to use the local operations product.

### P4 — Expand only after the reusable core is qualified

After the first operations slice and customer signal:
- add vendor adapters through the existing capability-manifest/plan/apply contract;
- add fleet orchestration, with exact target and scope binding and staged rollout;
- adapt the same evidence and authority pattern to utilities, municipal services, housing, cooperative finance, and other domains;
- expand Symthaea research in HDC, cognitive dynamics, privacy-preserving proofs, and specialized edge inference when benchmarks show value on defined workloads;
- deepen Mycelix governance and cooperative infrastructure where a real multi-party need is demonstrated.

Do not make all these domains simultaneous P0 work. New verticals must reuse the qualified contracts or document why an exception is required.

## 30/60/90-day proposed gates

These are proposed planning windows from acceptance of this roadmap, not claims about current implementation status.

| Window | Intended result | Evidence required to exit |
|---|---|---|
| Days 0–14 | Cross-repository contract and maturity map; ACS versioned gap matrix; choose the exact isolated-VM operation | Source links/SHAs, owner mapping, unresolved gaps, fixed scenario definitions, dependency and safety review |
| Days 15–30 | Read-only-to-dry-run flow: observation → typed plan → scoped authorization → independent decision → receipt | Schema/property tests, negative fixtures, authenticated request/response binding, exact-head results |
| Days 31–60 | One reversible lab operation with post-state check and crash/retry reconciliation | Disposable-VM evidence, 100% in-scope negative-fixture rejection, zero unauthorized effects on the frozen corpus, recovery matrix |
| Days 61–90 | Comparative report and controlled pilot decision | Same-task baseline results, p50/p95 and resource data, known limitations, operator review, customer/problem validation, go/no-go record |

If a gate fails, fix the blocking contract rather than expanding scope around it. If a dependency is unavailable, record the blocker and keep the capability unqualified.

## What “industry-leading” must mean

The goal is not to claim “best at everything” in a single release. It is to build repeatable evidence of leadership by domain.

- **Security:** demonstrate the specific authority boundaries and attack classes tested; obtain independent review of the highest-risk boundary before broad deployment.
- **Reliability:** measure partial failure, recovery, drift, restart, upgrade, and degraded-network behavior, not just the happy path.
- **Cognition:** benchmark Symthaea against strong, relevant baselines on the exact workload. Do not infer general intelligence, consciousness, or broad superiority from internal component benchmarks.
- **Privacy and sovereignty:** measure what leaves a device/organization, who can decrypt it, what hashes or metadata can reveal, and how users export/revoke their data.
- **Performance:** compare identical workloads and hardware. Report p50/p95, memory, CPU/energy where relevant, quality, and overhead introduced by verification.
- **Interoperability:** implement standards and existing APIs where they solve the problem; publish precise conformance matrices and gaps.
- **Usability:** an operator can understand why an action was permitted, denied, deferred, or found indeterminate, and can recover without needing to understand every internal subsystem.
- **Customer value:** measure time, operational risk, failure rates, maintenance burden, and total cost against the current workflow.

A 30% reduction in operator time at no worse safety/error rate is a useful **initial commercial hypothesis** to test in a pilot, not a promised result or a fact. The pilot must establish a defensible baseline before drawing a conclusion.

## Portfolio operating rules

1. Keep no more than two cross-repository P0/P1 initiatives active at once: (a) shared evidence/authority integrity, and (b) the first end-to-end product slice.
2. Each work item declares an owner, exact integration point, preregistered acceptance conditions, evidence artifact, and explicit nonclaims.
3. Prefer extending existing issue/PR owners to making parallel versions of shared contracts.
4. Keep feature flags and research prototypes clearly separated from supported production paths.
5. Do not count LOC, crate count, test count, or protocol count as product value by themselves.
6. Do not weaken a proof to get a green dashboard. Missing evidence is not a pass; stale success is not current qualification.
7. Keep human approval for high-impact, irreversible, safety-critical, or legally consequential actions until the applicable threat model and validation evidence justify a narrower policy.
8. Treat model output, policy decision, execution attempt, external effect, and observed outcome as distinct events.
9. Maintain a public claims register with claim, exact subject/revision, evaluation method, result, limitations, independent review status, and date.
10. Review the priorities against customer evidence every 30 days. New research is promoted into the product path only when it clears a defined value and verification gate.

## Research and standards baseline

- [OWASP Top 10 for Agentic Applications 2026](https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/) — threat taxonomy for agent goal hijack, tool misuse, identity/privilege abuse, supply-chain risks, and related risks.
- [OWASP Agent Control Standard (ACS)](https://genai.owasp.org/resource/agent-control-standard-acs/) and [reference implementation repository](https://github.com/GenAI-Security-Project/agent-control-standard) — candidate interoperability target for runtime hooks, policy decisions, tracing, and agent inventory. Pin and test the exact version; the current reference README declares unsupported hooks and security gaps.
- [NIST AI RMF Generative AI Profile](https://www.nist.gov/publications/artificial-intelligence-risk-management-framework-generative-artificial-intelligence) — risk-management structure for mapping, measuring, and managing generative-AI risks.
- [SLSA build provenance v1.2](https://slsa.dev/spec/v1.2/build-provenance) — established provenance model for the origin and build process of software artifacts.
- [Holochain application architecture](https://developer.holochain.org/concepts/2_application_architecture/) — source for the agent-centric and application-modularity model underlying Mycelix; the framework does not remove the need to test each application’s business rules, recovery, privacy, and real multi-agent behavior.

## Status at publication

This document is a strategic proposal and prioritization artifact. It does **not** indicate that the milestones have been implemented, that ACS is fully integrated, that the current system has passed these tests, or that any external security audit or production pilot has occurred. Progress should be recorded against exact commits and evidence links as each gate is executed.
