# OMSI Technology-Services Operations Profile 0.1

**Profile ID:** omsi:technology-services-operations:0.1-incubation  
**Status:** incubation profile; not a normative OMSI release or certification  
**Date:** 2026-10-30  
**Related:** [OMSI portfolio protocol PR #157](https://github.com/Luminous-Dynamics/luminous-platform/pull/157), [OpsBench PR #149](https://github.com/Luminous-Dynamics/luminous-platform/pull/149), [Service-Episode Saga PR #153](https://github.com/Luminous-Dynamics/luminous-platform/pull/153), [profile issue #160](https://github.com/Luminous-Dynamics/luminous-platform/issues/160)

## 1. Purpose and scope

This profile describes how managed IT, enterprise technology operations and technology-service providers exchange evidence about a customer service episode. It composes with the existing OMSI cross-industry evidence/interchange architecture; it does **not** define a second universal evidence envelope, ITSM, service catalogue, identity system, workflow engine, host transaction protocol or benchmark corpus.

The profile covers the semantic boundary between:
- customer request and requested service outcome;
- service, tenant, asset, dependency and ownership references;
- measured, reported, derived, inferred, stale, conflicting and unknown operational evidence;
- diagnostic hypotheses and recommendations;
- approved plans and exact operation authority;
- connector attempts, idempotency identities and external effects;
- independently verified step postconditions;
- compensation as a new operation;
- independently verified global service outcome;
- escalation, SLA status, human intervention, elapsed time and cost.

It applies to managed IT services, Linux/Windows/macOS fleet operations, cloud/platform operations, security operations, enterprise application operations and adjacent technology-support workflows. Healthcare, financial, aviation and industrial-control authority remains governed by those industries' own profiles and applicable controls.

## 2. Design decision

**The OMSI common layer carries evidence and authority boundaries; ITOPS preserves IT service semantics.**

Reuse the existing OMSI source/scope/time/evidence/provenance contract and operation/effect/recovery structure. ITOPS adds vocabulary and constraints for service episode, service objective and customer-facing disposition; service/asset/dependency/tenant references; connector capability and operation references; exact plan, approval, attempt and idempotency bindings; external effect observation and postcondition verification; service-level SLO/SLA and customer-visible outcome; operator intervention and cost accounting.

An OMSI mapping must be loss-aware. If a source system does not expose tenant identity, stable resource identity, operation idempotency or evidence freshness, the mapper records the missing capability. It must not infer or synthesize it.

## 3. Capability ownership and no-duplication map

| Concern | Owner | ITOPS profile rule |
|---|---|---|
| Portable evidence/interchange semantics | OMSI common architecture | Reuse; do not create a second envelope |
| Customer case, request, change and CMDB records | Customer's ITSM system (e.g. ServiceNow/Jira Service Management) | Reference source IDs and revisions; do not become a competing system of record |
| Connector policy, tenant scope and service-episode coordination | Luminous Platform | Orchestrates typed, scoped operations and evidence |
| Host transaction, NixOS generation, authorization and recovery | Nixward | Remains authoritative for host mutations; no second host journal/authorization service |
| Bounded diagnosis and observation selection | Symthaea | Hypotheses/recommendations remain distinct from measured facts and permission |
| Identity, delegation and provenance | Mycelix, when qualified | Carries accountable delegation; reputation or a signed claim is not proof of truth or authority |
| Physical simulation | Symtropy, where a specific model is validated | Not used as a generic IT service oracle |
| Service observability and state | Customer telemetry plus independent observers | Source-native systems remain explicit; dashboards/traces are observations, not ground truth |

The profile does not mandate a vendor, transport or orchestration product. OpenTelemetry, existing ITSM APIs, OIDC, systemd/D-Bus, NixOS and established durable workflow runtimes should be reused where fit.

For host participants, the Platform saga stores references to Nixward's existing [System Transaction Wire v1](https://github.com/Luminous-Dynamics/nixward/blob/main/docs/SYSTEM_TRANSACTION_WIRE_V1.md), exact ChangePlan/transaction identity, target-machine digest, attempt identity and Nixward-owned receipt digests. It must not mirror Nixward's journal, interlock, authorization verifier or worker ledger. The Platform adapter contract is tracked in [issue #162](https://github.com/Luminous-Dynamics/luminous-platform/issues/162); implementation depends on Nixward's [journal-owned worker recovery issue #96](https://github.com/Luminous-Dynamics/nixward/issues/96). Until that work is implemented and independently qualified, a host participant adapter remains read-only/reference-only.

## 4. Evidence classes and semantic distinctions

| Class | Meaning | Not equivalent to |
|---|---|---|
| measured | Instrumented value with declared method/source/time | Root cause |
| reported | A source/operator/customer stated a condition | Independently observed condition |
| derived | Deterministic calculation from declared premises | Measurement |
| inferred | Hypothesis supported by observations | Verified fact |
| modelled | Model output under a declared model/version/domain | External service state |
| simulated | Result in a simulation environment | Production behavior |
| stale | Observation outside its validity/freshness policy | Current state |
| conflicting | Independent records disagree | Permission to choose the favorable value |
| unknown | Evidence unavailable or insufficient | Healthy, absent or successful |

A diagnosis from Symthaea is an inference. A process exit is a reported execution fact. A ticket close is an ITSM state transition. A service-health result is only as strong as the independent probe, time window, scope and observation semantics behind it.

## 5. Service-episode lifecycle

An episode binds a customer request to a service objective and a graph of independently authorized participant operations.

Recommended lifecycle:

requested → scoped → observed → diagnosed → planned → approved → dispatched → reconciled → step_verified → service_verified → resolved

Alternative dispositions include awaiting_approval, escalated, partially_complete, outcome_unknown, indeterminate, recovery_required, rejected_before_dispatch and security_violation_observed.

These are profile-level business dispositions, not replacements for the lower-level Nixward transaction or saga state machines.

### 5.1 Request and service scope

A request should identify:
- customer/tenant scope from a trusted binding, not a free-form prompt;
- requested service and version/revision where applicable;
- affected resource references and dependency graph revision;
- desired service postcondition and its independent observer;
- urgency, SLA clock, support window and escalation owner;
- permitted actions, excluded resources and approval requirements.

Source-native resource identifiers remain paired with their source namespace/instance. Display names and aliases do not establish resource identity.

### 5.2 Observation and diagnosis

Each observation retains source, source revision, trusted scope, event/observation/recorded/ingested times, freshness, method, evidence class, unit/quantity semantics where relevant, quality/conflict state and provenance.

A diagnostic hypothesis must link to its supporting and contradicting observations. Confidence is descriptive metadata; it is not authority, a substitute for fresh evidence or an automatic promotion signal.

When evidence is insufficient or contradictory, the valid outcome is an explicit uncertainty or escalation state.

### 5.3 Plan and authority

Every consequential operation binds tenant and subject identity, episode and immutable plan digest, participant/step ID, exact target/resource identity, connector operation identifier, canonical request digest, idempotency identity and attempt ID, expected postcondition digest, and approval grant identity/validity/revocation context.

A grant is checked by the authority adapter. An OMSI projection or ITOPS profile validator does not mint, extend or broaden permissions. Changing the plan, target, request, operation or scope requires new exact authority.

### 5.4 Dispatch and effect reconciliation

Dispatch intent is recorded before a connector call. A lost reply, timeout, worker crash or cancellation after dispatch yields an unknown outcome, not a failed attempt and not permission to replay.

An ordinary current-state read showing no visible change does not prove that the remote system never accepted the request. Retry eligibility requires a documented idempotency guarantee for the exact request/key or an independent, attempt-specific authoritative ledger proof that the exact attempt was never accepted, followed by the required fresh authority.

The profile references [Saga Contract v1](https://github.com/Luminous-Dynamics/luminous-platform/blob/research/service-episode-saga-v1-2026-10-30/docs/SERVICE_EPISODE_SAGA_CONTRACT_V1.md) for detailed retry, cancellation and compensation semantics.

### 5.5 Compensation and recovery

Compensation is a new request with its own plan binding, exact authority, dispatch intent and independent verification. It is not a generic rollback and may use the same connector operation name if the request/binding is distinct.

A host generation rollback, DNS correction, identity re-enrolment, ticket correction and customer communication are different domain operations. A sent notification cannot be unsent; a correction is a new event. The profile must preserve that asymmetry.

### 5.6 Service verification and closure

A service episode is resolved only when every required participant operation has its expected postcondition independently verified; no participant remains authorized, dispatched, ambiguous, in compensation or otherwise unresolved; a distinct service-level observer confirms the requested service objective; the observation binds the exact episode plan and is inside its policy-defined freshness window; and no hard safety, tenant or forbidden-effect gate has been violated.

The agent's narrative, its own telemetry, a closed ticket, a database row or a signed receipt alone cannot establish global service restoration.

After verified closure, the episode is immutable. Later work is a new episode linked to the prior one.

## 6. Time, SLO/SLA and economic semantics

Keep request-received time, first human/agent acknowledgement, investigation start/end, dispatch-intent and remote-effect time, step-verification time, service-restoration verification time, customer communication time, SLA due time, paused intervals and clock authority distinct.

Do not collapse time-to-acknowledge, time-to-mitigate, time-to-restore and time-to-verify into a single "resolution time".

Cost accounting should declare its denominator and include model/runtime cost, connector/API usage, compute, human active time, escalation, rework, customer impact where measurable and the amortized integration/support burden. Do not claim labour savings by excluding human review or transferring effort to customer staff.

## 7. Security, privacy and authority limits

- Tenant identity and target scope are enforced outside model prompts and remain bound through all connector calls.
- Operational text, logs, tickets, package descriptions, email and attached files are untrusted evidence, never an authority source.
- Model outputs are advisory; capability policy and exact grants control mutation.
- Secrets remain in credential brokers; the agent receives opaque credential handles or narrowly scoped operations.
- All connector requests must have explicit idempotency, timeout, retry, consistency and observation semantics.
- Source data rights, retention, purpose limitation, residency and training permission remain attached to evidence.
- No cross-tenant read/write is permitted through ordinary approval.
- Customer notifications, payments, deletion, credential rotation, firewall/security-policy changes and destructive restoration require explicit domain-specific authorization.
- Missing or contradictory policy evidence fails closed for privileged actions.
- No profile conformance vector grants real credentials, external network access or production/physical actuation authority.

## 8. Conformance vectors and evaluation boundaries

The synthetic vector set in benchmarks/omsi-itops-profile-v0.1/conformance-vectors.json tests only the ITOPS semantic projection and claim boundaries. It is not a second OpsBench task corpus and not a saga state-machine implementation.

Each vector links, when applicable, to an existing OpsBench service task ID, an existing Saga fault ID, an expected profile-level disposition, required retained evidence distinctions and forbidden claims/authority.

Evaluation results must remain separated:
- diagnostic quality (for example, ITBench-AA AP@full-recall);
- task completion (for example, an EnterpriseOps-Gym task's own database-state verifier);
- operation/step effects (Saga state/effect reconciliation);
- service outcome (OpsBench independent service oracle);
- profile conformance (ITOPS semantic mapping vectors);
- economics (human time, elapsed time, rework and cost per verified outcome).

Never translate one score into another or combine unlike task sets into a universal score.

## 9. Baseline and benchmark programme

Start with comparable baselines: an expert operator following the customer runbook; deterministic automation using the customer's established tools; a general-purpose agent under identical tools, permissions, context and budgets; and the Luminous bounded workflow using the same scenario snapshot and independent evaluator.

Report task-family outcomes, uncertainty, started/failed/timeout/indeterminate denominators, human interventions, unauthorized/forbidden effects, false resolution claims, duplicate effects, time-to-verified-restoration, recovery, rework and total cost.

In addition to task success, adapt the 2026 ICML reliability framework's four dimensions—**consistency, robustness, predictability and safety**—to operations. That work proposes twelve metrics across these dimensions and finds that capability gains alone produce only small reliability improvements. ITOPS must preserve the full per-metric profile and must not collapse it into a single weighted score. Operational adaptations should include:

- **Consistency:** success and disposition variance across independent repeats; exact-state reproducibility; sensitivity to harmless prompt/ordering/seed variation.
- **Robustness:** degradation under stale, missing, noisy, contradictory and adversarial evidence; dependency drift; connector faults; partial outages; and multi-fault conditions.
- **Predictability:** calibrated abstention; failure-mode and severity distributions; time-to-safe-hold; ability to identify when the environment is outside the validated domain.
- **Safety:** unauthorized/forbidden effects, cross-tenant exposure, false resolution, replay/duplicate effects, blast radius and quality of recovery evidence. Critical safety gates remain non-compensatory.

The twelve source-paper metric definitions must be preserved if the paper's benchmark protocol is reproduced. The operational measures above are ITOPS adaptations, not a claim that the paper defines IT operations metrics.

Suggested research targets (not current results):
- zero unauthorized and forbidden effects in the frozen adversarial suite;
- zero false service-resolution claims in canonical tests;
- 100% explicit evidence for every resolved case;
- at least 90% task success initially and at least 95% on a mature, held-out task family before broadening autonomous scope;
- at least 30% lower median active operator time and 20% lower total cost per verified outcome on a matched pilot, without degraded customer or safety outcomes.

Hard safety gates are non-compensatory. Zero observed events in a small synthetic suite is not a fleet-wide safety guarantee. Publish exact counts, confidence intervals and trial independence assumptions.

### Public baseline snapshot — 30 October 2026

- **EnterpriseOps-Gym-AA:** Artificial Analysis reports a top strict task-success score of **51.1%** in its current all-domain leaderboard. The underlying ServiceNow benchmark describes 1,150 tasks across eight domains, 512 tools and 164 database tables. A task receives no partial credit when a required final-state verifier fails. This is the closest public comparator for long-horizon enterprise workflows, but the aggregate is not an ITSM-only score and a passing database-state verifier is not automatically an external Linux/service-health or Nixward recovery certificate. See [the live evaluation](https://artificialanalysis.ai/evaluations/enterprise-ops-gym-aa) and [the benchmark repository](https://github.com/ServiceNow/EnterpriseOps-Gym). <https://artificialanalysis.ai/evaluations/enterprise-ops-gym-aa>
- **ITBench-AA:** the current diagnostic evaluation contains 59 Kubernetes incident snapshots (40 public and 19 private) with three repeats per task. Its headline is average precision at full recall for root-cause entity attribution; the current displayed leader is **56.2%**. This is a diagnosis metric, not an end-to-end service-resolution rate. The benchmark's own page distinguishes the original ITBench paper's resolution-rate metric from its new root-cause metric. See [ITBench-AA methodology and results](https://artificialanalysis.ai/evaluations/itbench-aa). <https://artificialanalysis.ai/evaluations/itbench-aa>
- **Agent reliability:** the ICML 2026 reliability study proposes twelve metrics across consistency, robustness, predictability and safety, and finds that higher task capability has translated into only small gains in reliability. ITOPS therefore keeps a multi-dimensional reliability profile beside task-success reporting; it does not create one weighted winner. See [Rabanser et al., PMLR 306 (2026)](https://proceedings.mlr.press/v306/rabanser26a.html). <https://proceedings.mlr.press/v306/rabanser26a.html>

### Competitive targets (research goals, not results)

For a preregistered ITSM/Hybrid common-core subset, target:
- **At least 60% strict end-to-end task completion** on a held-out evaluation slice after the adapter and execution semantics are stable.
- **At least 10 percentage points absolute improvement** over the strongest matched baseline under identical task snapshots, permissions, tool schemas, model/run budgets, evaluator and repetitions.
- **No weakening of hard gates:** zero unauthorized effects, cross-tenant disclosure, forbidden side effects or false service-resolution claims in the qualification corpus.
- **100% independent postcondition evidence** for every resolved episode; all started failures, timeouts and indeterminate outcomes remain in denominators.
- **At least 20% lower total cost per independently verified outcome** and 30% lower median human-active time in a matched prospective pilot, after including review, escalation, integration and rework.

The 60%/10-point goals are not a direct claim against EnterpriseOps-Gym's 51.1% full-suite headline or ITBench-AA's 56.2% diagnostic score. Only a task-matched common-core trial can support an improvement claim. Do not publish a full-suite comparison from an ITSM/Hybrid subset.

### Relevant common benchmarks

- [ITBench](https://proceedings.mlr.press/v267/jha25a.html) and [ITBench-AA](https://artificialanalysis.ai/evaluations/itbench-aa): diagnosis and IT automation comparators; retain their own tasks/metrics.
- [EnterpriseOps-Gym](https://github.com/ServiceNow/EnterpriseOps-Gym) and [EnterpriseOps-Gym-AA](https://artificialanalysis.ai/evaluations/enterprise-ops-gym-aa): long-horizon enterprise workflow and final database-state comparison.
- [Project Arena](https://github.com/edgedelta/project-arena), [PagerDuty SRE benchmark](https://www.pagerduty.com/resources/ai/learn/sre-agent-benchmark-report/) and [RootCauseBench](https://github.com/edgedelta/root-cause-bench): diagnostic/forensic comparators, not interchangeable with service restoration.
- [Towards a Science of AI Agent Reliability (ICML 2026)](https://proceedings.mlr.press/v306/rabanser26a.html): a twelve-metric framework across consistency, robustness, predictability and safety; adapt the dimensions without collapsing them into a composite score.
- [OpenTelemetry semantic conventions](https://opentelemetry.io/docs/specs/semconv/) and [DORA metrics](https://dora.dev/guides/dora-metrics/): reusable operational measurement conventions, not stand-alone proof of service health.

## 10. Capability and evidence promotion

The profile is currently **E0/E1 incubation**:
- profile scope, semantic distinctions and safety boundaries are specified;
- synthetic conformance vectors and a structural schema are authored;
- no semantic OMSI adapter, vendor connector, independently evaluated operational workflow, production fleet, external assessor or customer pilot is claimed.

Promotion to E2 requires an independent implementation of the profile mapping and a verifier that reconstructs all claims without trusting the mapper's self-report. E3 requires legally usable external source data and held-out validation. E4 requires authorized, read-only vendor connectors and security/privacy review. E5 requires a prospective shadow pilot with an accountable operator, frozen baseline and complete intervention accounting. E6 is deployment-specific production assurance, not a certificate conferred by the profile.

## 11. References and supersession

The profile is subordinate to, and must be revised with OMSI cross-industry extension architecture PR #52; OMSI industry profile portfolio protocol PR #157; OpsBench service-episode task/evidence contracts PR #149; Saga orchestration contract PR #153; and Nixward System Transaction Architecture.

If the common OMSI evidence or operation schema changes, create a new ITOPS profile version and a migration note. Do not silently reinterpret old records or benchmark results.

**Claim ceiling:** this is a synthetic, profile-level contract. It does not make any current Symthaea, Symtropy, Mycelix, Nixward or Luminous Platform component a production-ready service provider, and does not certify interoperability, reliability, security or SOTA superiority.
