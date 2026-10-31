# OpsBench Common-Core Bridge v0.1

**Status:** research protocol and preregistration contract; no comparative trials or performance results are included.  
**Purpose:** compare capability stages across enterprise operations and SRE agent evaluations without translating unlike benchmark scores.

## 1. Why the bridge exists

Public benchmark results answer different questions:

- EnterpriseOps-Gym measures strict stateful enterprise task completion. A task succeeds only when every required final-state verifier passes.
- ITBench reports root-cause entity attribution (AP at full recall). This is diagnosis/localization, not successful remediation.
- AI SRE Arena evaluates injected Kubernetes incidents through detection, investigation, mitigation recommendations and unsafe-advice judging. Its v1 report explicitly does not execute repairs or verify restoration.
- PagerDuty's 2026 SRE Agent Benchmark uses 17 scenarios and a three-model judging ensemble, but its published comparison reports one trial per agent and excludes some failed agent responses from analysis.
- OpsBench evaluates service-level outcomes, participant effect reconciliation, independent restoration evidence, recovery and economics.

The metrics are not interchangeable. A 56% root-cause attribution result is not a 56% restoration rate; a proposed fix is not an executed repair; a workflow trace is not proof of an external effect.

The machine-readable external benchmark crosswalk captures reported scope, primary metric and limitations as of October 31, 2026. It intentionally prohibits direct score translation.

## 2. Paradigm shift: measure the whole operational capability chain

Every benchmark episode should expose separate evaluator judgments for:

1. **Detection:** did the system recognize the actual fault and avoid inventing one?
2. **Localization:** did it identify the affected service, target or root-cause entity?
3. **Diagnosis:** is the explanation supported by fresh, relevant, contradictory and corroborating evidence?
4. **Decision:** did it act, clarify, request exact authority, reconcile, escalate or halt appropriately?
5. **Execution:** did the participant accept and commit the authorized effect exactly once under the declared connector contract?
6. **Recovery:** after response loss, coordinator crash or restart, did the workflow reconcile prior attempts before issuing new mutations?
7. **Service outcome:** did an independent observer establish the requested customer-facing postcondition?
8. **Safety:** did an evaluator complete the declared audit scope and identify unauthorized/collateral effects?
9. **Economics:** what was fully loaded cost and human-active effort per verified service outcome?

Each stage has its own denominator, oracle and claim ceiling. A successful localization must not be counted as a successful repair. A successful repair must not hide an unauthorized side effect. A signed trace does not prove external service health.

## 3. Common-core scenario design

The common-core set should begin with 30-50 scenario templates, then expand into evaluator-held private variants. Templates should include:

- incident diagnosis with stale or contradictory telemetry;
- a broken dependency where the target service can start but remains unhealthy;
- connector response loss before admission, after acceptance and after effect commit;
- stale approval, target rebinding and revoked authority;
- duplicate delivery and idempotency-key conflict;
- partial canary failure and fleet-batch stop;
- coordinator restart with one or more unresolved attempts;
- identity/offboarding or DNS changes with independent postconditions;
- data integrity / restore checks with separate service-health proof;
- untrusted operational text / prompt injection;
- unsafe remediation recommendation and correct escalation.

Split by fault mechanism and topology, not random rows. Public task templates may be used for development; the confirmatory set must be stored separately from the system-under-test trust domain. Commit to the private variant set before candidate runs and disclose it only after the study or by controlled independent audit.

The public OpsBench v0.1 corpus is an authoring corpus and is not a blind holdout. It must not be used as evidence of generalization or SOTA performance.

## 4. Arms and fairness controls

Include, where technically comparable:

1. Expert human operator using the ordinary documented workflow.
2. Best deterministic automation / maintained runbook.
3. General-purpose agent under the same task, permission, time and tool-call budgets.
4. Commercial SRE / enterprise agent where equivalent scenarios, tools, permissions and independent verification are available.
5. Luminous full stack.
6. Luminous ablations (for example: no Saga, no independent observer gate, or Symthaea advisory replaced with a fixed runbook).

Every arm receives the same initial snapshot, customer request, tenant scope, authority profile, fault schedule and independent oracle. Keep model versions, prompts, tool surfaces, policy, environment, time/tool budgets and run artifacts pinned. When product boundaries make exact tool parity impossible, disclose the mismatch and classify the result as a capability comparison rather than a matched performance comparison.

Do not cripple the deterministic baseline. Include the native capabilities it would ordinarily use.

## 5. Repetitions and statistical design

- Screening: at least five seeds per scenario to reveal brittle behavior.
- Confirmatory: target at least 500 distinct scenario templates/variants across the declared fault families, with at least five repeats per scenario where feasible.
- Pair by scenario_id, variant_id and seed.
- Treat scenario_id as the resampling cluster; repeated seeds are nested observations, not independent task samples.
- Report both trial-weighted and scenario-weighted rates.
- Use a fixed-seed scenario-cluster bootstrap for 95% uncertainty intervals. If there are too few scenario clusters, report the interval as unavailable rather than fabricate precision.
- Preserve all started trials. Timeouts, crashes, blocked and indeterminate outcomes remain in task-performance denominators; evaluator errors are separately reported, not silently dropped from the economics or effort accounting.
- Publish per-family results, unmatched trials, evaluator errors, missing evidence and the exact source/model/toolchain/policy/environment/connector/corpus/variant/oracle digests.

The preregistration schema requires scenario-level splitting, repeats, denominators, paired keys, safety gates and a multiple-comparisons policy before confirmatory runs begin.

## 6. Metrics: an assurance frontier, not a universal score

| Dimension | Measure |
|---|---|
| Detection | Detection precision/recall; false alarm rate |
| Localization | Root-cause entity precision/recall or AP at a declared recall |
| Diagnosis | Evidence grounding, calibration, contradiction handling |
| Decision | Paired decision accuracy, false abstention, unsafe action proposal |
| Execution | Participant acceptance, committed effects, duplicate effect rate |
| Recovery | Recovery completion; unresolved-attempt duration; restart reconciliation latency |
| Service | Strict independent service-task success; time-to-verified-restoration |
| Safety | Critical-effect incidence; false-resolution rate; safety-assessment coverage and indeterminate rate |
| Human | Human-active seconds per verified outcome; escalation completeness |
| Economics | Fully loaded cost per verified outcome, including failed work, labor, escalation, rework and allocated support |

The primary visual should be an **assurance frontier**: strict verified service-task success against critical-effect incidence, with observer coverage and indeterminate rate shown separately. Cost and human effort belong beside it, not as weights that compensate for safety failures.

A critical unauthorized effect, cross-tenant violation, duplicate business effect or false service-resolution claim is a non-compensatory gate. Do not average it away with successful episodes or cost savings.

## 7. Initial hypotheses (not results)

- At least 60% held-out strict service-task success on a preregistered ITSM/Hybrid subset.
- At least +10 percentage points against the strongest matched baseline, with scenario-clustered uncertainty.
- Zero duplicate business effects, false service resolutions, unauthorized effects and cross-tenant effects in canonical adversarial/counterfactual tests.
- 100% independent service verification for any resolved claim.
- At least 30% lower human-active time per verified outcome.
- At least 20% lower fully loaded cost per verified outcome.
- No increase in unsafe recommendations or false abstention relative to the matched agent baseline.

These targets are deliberately separate from EnterpriseOps-Gym's full-suite percentage. A 60% result on our ITSM/Hybrid subset does not beat a 51% score on a different benchmark. Direct superiority claims require the same task variants, harness, permissions, budgets, repetitions, model/tool versions and evaluator.

## 8. Implementation and dependencies

Compose with:
- OpsBench #149 (service outcome and task manifest);
- ITOPS autonomy boundary #167 (act / clarify / request authority / reconcile / escalate / halt);
- economic scorecard #170 (all-started denominators and scenario-cluster analysis);
- orthogonal safety assessment #172 (coverage and collateral findings);
- causal recovery-trace verifier #174 (attempt/effect/restart evidence).

No parallel service taxonomy, transaction journal, identity provider or task-success oracle is introduced.

## 9. Claim ceiling

The crosswalk and preregistration contract define an experiment; they are not results. The common-core screening and confirmatory runs must not begin until the evaluator, private variant store, participant manifests, observer manifest, fault schedule and source pins are independently frozen. The bridge does not certify a vendor, prove real-world safety or justify a SOTA claim by itself.

## Sources (scope descriptions; retrieved October 31, 2026)

- [ServiceNow EnterpriseOps-Gym](https://github.com/ServiceNow/EnterpriseOps-Gym) and [Artificial Analysis live evaluation](https://artificialanalysis.ai/evaluations/enterprise-ops-gym-aa).
- [ITBench repository](https://github.com/itbench-hub/ITBench) and [Artificial Analysis ITBench-AA](https://artificialanalysis.ai/evaluations/itbench-aa).
- [Edge Delta AI SRE Arena v1](https://edgedelta.com/arena) and [open implementation](https://github.com/edgedelta/project-arena).
- [PagerDuty SRE Agent Benchmark (2026)](https://www.pagerduty.com/resources/ai/learn/sre-agent-benchmark-report/).
- [OpsBench v0.1](https://github.com/Luminous-Dynamics/luminous-platform/pull/149).
