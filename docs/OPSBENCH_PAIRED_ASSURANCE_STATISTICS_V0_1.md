# OpsBench Paired Assurance Statistics v0.1

**Status:** evaluator-side analysis prototype with synthetic example data. This is not a completed benchmark, production safety certification or SOTA result.

## 1. Research position: move from agent leaderboard to operational assurance

The industry is already moving beyond toy single-turn tool tasks, but the leading public evaluations measure different parts of the operational chain:

| Benchmark | Public scope | What it establishes | What it does not establish |
|---|---|---|---|
| EnterpriseOps-Gym / AA | 1,150 expert-curated tasks across 8 enterprise domains; 164 tables and 512 tools in the underlying environment | Stateful, multi-step enterprise workflow completion and policy-sensitive final-state checks | A comparable SRE recovery result; no automatic translation into our OpsBench score |
| ITBench-AA | 59 Kubernetes incidents (40 public, 19 private), 3 repeats; diagnosis scored as average precision at full recall | Root-cause entity localization from offline snapshots | Live remediation, crash-safe retries or verified customer-facing restoration |
| AI SRE Arena v1 | 21 injected Kubernetes faults, 72 judged investigations, 2 native and 2 control configurations | Detection, investigation, diagnosis and proposed mitigation under its published rubric | Production-scale reliability, executed repair, or a fully independent multi-run superiority study |
| PagerDuty SRE Agent Benchmark 2026 | 17 scenarios; three model judges; one trial per agent in the published sweep | Multi-dimensional investigation quality, including localization, diagnosis, mitigation and evidence grounding | Stable repeated-run reliability; failed agent runs are excluded from quality scores in the published analysis |
| OpsBench common-core | Intended to join stateful tasks, attempt reconciliation, recovery, independent service outcome, safety and economics | The complete operational capability chain when populated and independently run | No claim until evaluator, oracle, source, split and execution evidence are independently pinned |

Sources and caveats:
- [EnterpriseOps-Gym, ICML/PMLR 2026](https://proceedings.mlr.press/v306/malay26a.html) describes 1,150 tasks, eight domains, 164 tables and 512 tools. The live [Artificial Analysis EnterpriseOps-Gym-AA page](https://artificialanalysis.ai/evaluations/enterprise-ops-gym-aa) currently reports a 51.1% leading strict task score and three repeats per task. The original paper's model snapshot and the current AA leaderboard are different reporting contexts; do not merge their scores.
- [ITBench-AA](https://artificialanalysis.ai/evaluations/itbench-aa) documents 59 tasks and three repeats, with diagnosis/localization scored as AP at full recall. Its score is not a repair success rate.
- [AI SRE Arena](https://edgedelta.com/arena) is a vendor-published October 2026 benchmark: 21 injected faults and 72 judged investigations; its report explicitly describes the setup and one judge.
- [PagerDuty's 2026 SRE Agent Benchmark](https://www.pagerduty.com/resources/ai/learn/sre-agent-benchmark-report/) reports 17 scenarios, three LLM judges and a single trial per agent in the published sweep. It explicitly says failed agent responses are excluded from some quality analyses and that the current work is weighted toward triage/diagnosis, with less remediation coverage.
- [Artificial Analysis Intelligence Index v5, October 27, 2026](https://artificialanalysis.ai/articles/artificial-analysis-intelligence-index-v5) reports that 60% of its index now uses private test sets or held-out answers. This is a useful industry signal: public task templates can remain reproducible, but confirmatory answers/variants need controlled access and a pre-trial commitment.


The methodological bar must also reflect the research literature. The NeurIPS 2025 Agentic Benchmark Checklist paper reports that benchmark setup/reward flaws can cause large over- or under-estimates and that applying its checklist reduced CVE-Bench overestimation by 33%. A 2026 EMNLP Findings study reports search-time contamination in public benchmark evaluation, with inflation up to 4% on the evaluated deep-research benchmarks. Those findings justify evaluator isolation, private variants, complete started-trial denominators and independently verified outcomes.

## 2. Paradigm shifts

### A. Measure repeatability, not just possibility

An agent can succeed once and fail repeatedly. For every scenario cluster report both:
- scenario pass-at-least-one-repeat rate: did any repeat succeed?
- scenario pass-all-repeats rate: did every repeat succeed?

These are deliberately not interchangeable. The first measures whether success is achievable under repetition; the second measures repeat reliability. The evaluator also reports the average per-scenario success rate and its scenario-cluster bootstrap interval.

### B. A missing observer is not a safe result

Objective completion and safety assessment remain orthogonal. A verified service task can coexist with an unauthorized effect. Conversely, an empty finding list with incomplete observer coverage is indeterminate, not pass. A critical finding is a non-compensatory failure.

### C. Statistical units match the claim

The resampling unit is scenario_id, not each seed or episode. Repeated seeds and variants remain grouped within a scenario when a bootstrap sample is drawn. Candidate and baseline trials are paired only on exact (scenario_id, variant_id, seed) keys. Unmatched keys are reported and are not silently discarded from each arm's denominators.

### D. Cost and human effort are outcome-denominated

Every started episode contributes its human-active time and recorded spend, including failures, timeouts, crashes, blocked episodes and evaluator errors. If any required fully loaded cost is unknown, the fully loaded total and cost per verified outcome remain null. Partial totals are not presented as complete.

### E. No "zero risk" claim from a fixed corpus

For fully audited scenario clusters, the analyzer reports a one-sided exact binomial upper bound for the fraction of scenario clusters containing at least one critical event. With zero observed event clusters and 500 independent, representative scenario clusters, the conditional 95% upper bound is approximately 0.598%, not zero.

That bound is useful only under a representative, independently sampled scenario-population assumption. A fixed adversarial benchmark suite does not prove that assumption. The report therefore marks population-sampling assumptions as unverified and never permits a global zero-failure claim.

### F. More than one candidate requires multiplicity control

Candidate-vs-baseline comparisons use paired scenario-cluster bootstrap intervals. When multiple candidate arms are analysed, the intervals use a Bonferroni family-wise adjustment across the candidate comparison family. The method is deliberately conservative; no cherry-picked unadjusted contrast should become the headline.

## 3. Result contract

The versioned result fixture is synthetic and example_only. Each trial records:
- exact scenario, variant, seed and arm;
- started episode state, including timeout/crash/blocked/indeterminate/evaluator error;
- evaluator-owned oracle outcome;
- whether the agent claimed resolution;
- time to independently verified restoration (required only on verified success);
- evaluator-owned safety status and orthogonal critical finding counts;
- human-active seconds and nullable fully loaded cost.

The analyzer checks the raw preregistration digest and source pins, matches arm versions and tool/permission/budget digests to the preregistration, verifies common-core fairness declarations, requires registration to predate the first trial for screening/confirmatory analyses, and rejects duplicate trial keys and malformed/non-finite JSON.

## 4. Analysis outputs

Per arm:
- strict independently verified task success, trial-weighted and scenario-weighted;
- scenario-cluster bootstrap interval;
- pass-at-least-one and pass-all-repeats rates with scenario-cluster intervals;
- episode-state and oracle-unknown counts;
- false resolution claims;
- mean, median and p90 time-to-verified-restoration;
- human-active seconds per verified outcome;
- fully loaded cost and cost per verified outcome, or null when any required cost is unknown;
- safety gate, observer coverage, critical finding counts, audited scenario clusters and conditional exact upper bound.

Per candidate vs baseline:
- exact paired-key intersection;
- matched and unmatched trial counts and digest-bound unmatched-key samples;
- paired trial-weighted and scenario-weighted differences;
- family-wise adjusted scenario-cluster bootstrap interval.

## 5. Run

The tool uses only the Python standard library and operates on evaluator-produced records. It does not execute the agent or mutate any service.

    python3 tools/opsbench-assurance-statistics-v0.1/analyze_assurance.py \
      --results benchmarks/opsbench-assurance-statistics-v0.1/results.example.json \
      --preregistration benchmarks/opsbench-common-core-v0.1/preregistration.example.json

The exact-head workflow compiles the analyzer/tests, parses the fixture JSON, generates the example-only report, runs the adversarial suite and preserves the report in the workflow log.

## 6. Benchmark targets and promotion gates

These are research targets, not measured results:

| Dimension | Initial target | Stretch target |
|---|---:|---:|
| Confirmatory held-out strict verified success (common-core ITSM/Hybrid subset) | 60% | 70% |
| Paired gain vs strongest matched baseline | +10 percentage points | +15 pp |
| Confirmatory scenario clusters | 500 | 1,000+ |
| Repeats per confirmatory scenario/arm | 5 | 10 |
| Independent service verification for every resolved claim | 100% | 100% |
| Unauthorized / duplicate / cross-tenant effects in canonical tests | 0 | 0 |
| False service-resolution claims in canonical tests | 0 | 0 |
| Human-active time per verified outcome vs matched baseline | -30% | -40% |
| Fully loaded cost per verified outcome vs matched baseline | -20% | -30% |

Do not promote based on a composite score. A candidate must first pass every non-compensatory safety gate; then compare success, restoration latency, indeterminate rate, human effort and fully loaded cost independently. The held-out confirmation must pin source, model, prompt, tool surface, permission profile, budget profile, fault schedule, evaluator/oracle, environment and scenario commitment before trials start.

A headline claim of "better than SOTA" requires:
1. same-harness, same-task, same-permission, same-budget and same-evaluator runs against the strongest available baseline;
2. blind held-out fault-mechanism/topology splits and contamination controls;
3. complete reporting of every started trial, including failed, crashed, timed-out and indeterminate episodes;
4. paired scenario-cluster uncertainty and family-wise adjustment;
5. complete independent safety assessment and service outcome;
6. reproducible raw evidence and exact artifact digests.

No current external benchmark score is directly comparable to this synthetic example or to the full OpsBench chain.

## 7. Known limits

This analyzer is not an attestation verifier. It trusts the input rows as already evaluator-produced; it cannot establish that the oracle, safety observer or cost sidecar is authentic or independent. It does not correct judge bias, validate task difficulty, prove random scenario sampling or establish external generalization. It is a calculation layer; producer authentication and causal evidence validation are separate gates in #174 and #178. The synthetic example is not a registered study and cannot support SOTA or safety claims.
