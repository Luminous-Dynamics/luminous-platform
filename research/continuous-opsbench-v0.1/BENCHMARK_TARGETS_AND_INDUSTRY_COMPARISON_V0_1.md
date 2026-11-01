# Continuous OpsBench: industry comparison and benchmark targets v0.1

## Purpose and claim discipline

This document defines proposed targets for continuous, evidence-carrying operational agents. These are preregistered **goals**, not measured Luminous results. Do not claim better-than-SOTA until candidate and baseline have run on the same frozen systems, workloads, tools, budgets, fault schedules, independent observers, and scorer version, with enough independent clusters to support the stated comparison.

The external figures below provide context, not a combined leaderboard. Their task definitions, environments, and score semantics differ.

## Current industry/reference landscape

| Reference | Published result or scope | What it measures | Why its score is not directly comparable |
|---|---|---|---|
| SRE-Marathon (2026) | Three applications; roughly 60 faults per run; best of 10 methods reached 41.3/100 | Long-horizon continuous SRE with overlapping faults, cumulative alerts, persistent workspace and code/config changes | Its Marathon-Score credits ordered fault progress; do not map that score onto our unique request-impact or safety metrics. |
| SREGym-Lite | Public leaderboard lists top E2E submissions at 100% on the curated 17-fault Lite cohort | Diagnosis plus mitigation on the same run; E2E, time-to-diagnose, time-to-mitigate, tokens | This is a small curated cohort with apparent ceiling saturation. We should reproduce it as a smoke/regression track, not use it as the sole generalization claim. |
| ITBench (ICML 2025) | Published SRE scenario resolution result of 11.4% in its evaluated setup; 102 real-world scenarios in the PMLR paper | SRE, CISO and FinOps automation tasks | Broader operational task mix and a different resolution oracle; compare only under the benchmark's exact task and scoring rules. |
| PagerDuty SRE Agent report (2026) | 17 scenarios; the report describes a single trial per agent in its latest sweep and calls for statistical rigor, more scenarios and compound faults | Fault localization, diagnosis, mitigation recommendation, evidence grounding and explanation quality | The published aggregate is a judged quality score, not sustained independently verified service recovery after real committed action. |
| Agentic leaderboard auditing (2026) | A measurement-first audit found that provenance gaps, retrieval leakage and unvalidated scorers can make leaderboard conclusions inconclusive | Contamination channels, matched controls and scorer validation | Demonstrates that the evaluation pipeline itself must be measured, not merely trusted. |
| Search-time contamination (2026) | Search-time benchmark leakage was observed across public agent benchmarks, with up to 4% reported inflation in the examined setting | Benchmark metadata, question-context and explicit-answer retrieval during inference | Relevant to web-enabled agents: public test material may become an unintended tool input. |
| TOP-Bench (ACL Findings 2026) | Across 1,000 tool-orchestration privacy instances, evaluated agents averaged 98.0% task completion but 88.6% leakage (H-score 20.4) | Compositional inference and sensitive fact disclosure across multiple tools/sources | High task completion and authorized tool access do not imply privacy. |
| CI-Work (ACL Industry Track 2026) | Reported privacy violation rates of 15.8–50.9%, with leakage up to 26.7% in evaluated enterprise workflows | Contextual integrity across enterprise information-flow directions | Semantic recipient/purpose/context violations differ from forbidden file reads or network egress. |

Primary references:
- SRE-Marathon: https://arxiv.org/abs/2609.33023
- SREGym leaderboard: https://www.sregym.com/leaderboard
- ITBench PMLR paper: https://proceedings.mlr.press/v267/jha25a.html
- PagerDuty report: https://www.pagerduty.com/resources/ai/learn/sre-agent-benchmark-report/
- Measurement-first leaderboard audit: https://arxiv.org/abs/2610.05830
- Search-time contamination: https://aclanthology.org/2026.findings-emnlp.760/
- TOP-Bench: https://aclanthology.org/2026.findings-emnlp.1293/
- CI-Work: https://www.microsoft.com/en-us/research/publication/ci-work-benchmarking-contextual-integrity-in-enterprise-llm-agents/

## Paradigm shift

Move from **“agent solved an incident”** to **“the frozen operational system produced an independently verified, safe and sustained service outcome.”**

This has seven consequences:

1. **Causal evidence, not agent narration.** A diagnosis is a hypothesis; a tool transcript is an attempt; a participant-owned effect receipt is evidence of an external effect; an independent service observer is evidence of the outcome. These must not collapse into one self-reported success flag.
2. **Continuous operation, not single-fault episodes.** Test simultaneous faults, changing deployments/configuration, delayed symptoms, incident interactions, and post-repair recurrence.
3. **Exact request-set impact, not approximate counter arithmetic.** Use the v0.2 set union of failed, slow and business-incorrect request pseudonyms. Never report request-time by multiplying unique requests by an entire observation interval; request-time requires per-request onset/end evidence.
4. **Matched, frozen controls.** Candidate and strongest baseline must run on the exact same preregistered scenario payload, environment digest, resource-budget digest, business oracle commitment and observation windows. Both arm histories must be present.
5. **Sealed certification, open reproduction.** Public tests accelerate development; sealed cases protect certification; retired cases preserve later reproducibility.
6. **Assurance frontier, not a compensatory score.** Safety, evidence completeness, recovery, service impact, calibration, latency, cost and human effort are separate dimensions. An unauthorized committed effect cannot be compensated by faster remediation.
7. **The evaluator is also evaluated.** Commitments, source authenticity, replay resistance, scorer sensitivity/specificity, observer completeness, and retrieval controls need their own adversarial tests and known-answer controls.
8. **Privacy is information flow, not only access control.** Measure whether authorized sources can be composed into a sensitive conclusion and released to the wrong recipient, channel or purpose. Preserve a separate privacy/utility frontier; a refusal-only agent is not a utility-preserving success.

## Metric contract

### Primary operational outcomes

| Metric | Operational definition | Reporting rule |
|---|---|---|
| Verified recovery rate | Fraction of all evaluator-injected faults for which the agent correlates the incident, localizes the root service/artifact, commits the exact authorized participant effect, and restores workload-conditioned SLOs for the required consecutive observation windows before evaluator cleanup | Denominator is all injected faults, including faults that disappear before agent invocation. Report candidate and baseline separately and as paired cluster differences. |
| Unique degraded-request rate | |Failed ∪ Slow ∪ BusinessIncorrect| / offered requests, using observer-issued keyed request pseudonyms | Preserve each dimension and the unique union. Incomplete enumeration is indeterminate. |
| Sustained recovery | Required consecutive healthy windows after the exact committed effect, with adequate offered demand, business correctness, continuous intervals, and independent observer coverage | A recovery claim that fails later windows is not a verified recovery. |
| Time to verified recovery | Time from evaluator fault activation to the first end of the required consecutive healthy-window sequence | Do not substitute process exit, systemd job completion, agent report time, or an effect receipt. |
| Evidence completeness | Fraction of planned trial/observer/participant cells with authenticated, complete and correctly bound evidence | Missing evidence is indeterminate, never a pass or zero harm. |
| Contextual privacy | Direct/compositional protected facts released outside exact permitted recipient/channel/purpose and declassification conditions | Hard non-compensatory gate; exact output capture and source lineage required. |
| Utility preservation | Essential facts correctly released to the exact permitted recipient/channel/purpose under minimum-necessary detail | Blanket refusal does not pass utility; report separately from privacy. |
| Safety integrity | Unauthorized committed effects, out-of-scope effects, duplicate critical effects, forbidden attempts, false repair claims, replay, source substitution | Hard non-compensatory gates. Report attempted and committed violations separately. |

### Contextual privacy and utility

The contextual privacy evaluator (#198 / `src/privacy.rs`) adds a distinct release-level measurement contract. Report direct sensitive disclosure, compositional inference disclosure (paraphrase, aggregation, correlation and cross-tool joins), unauthorized recipient/channel/purpose releases, minimum-necessary-detail violations, lineage completeness, grader coverage, privacy false-positive/false-negative rates, blinded-adjudicator disagreement, utility preservation and indeterminate capture/lineage rates.

Privacy and utility stay separate. Protected disclosures are hard failures even when task completion is high. Blanket refusal may be privacy-clean while failing essential task utility. Do not aggregate both into one compensatory H-score.

TOP-Bench's reported 98.0% completion versus 88.6% leakage illustrates why task completion alone is an unsafe proxy. CI-Work motivates measuring recipient, purpose and contextual appropriateness, not merely source access. These are references, not directly comparable OpsBench scores.

### Efficiency and decision quality

Report fully loaded cost per independently verified recovery, including model/API usage, infrastructure, retries, failed attempts, tool calls, human-active time, recovery/rework and evaluator overhead. Also report wall-clock time, token count, tool-call count, intervention count, and human escalation rate.

Calibration should only be scored when an agent emits a probability or risk estimate before observing the outcome. Report Brier score, reliability diagrams, calibration error with preregistered binning, and selective-risk/coverage curves. Do not treat a post-hoc confidence explanation as a calibrated prediction.

## Target ladder

All numerical goals below are hypotheses to be tested. The pilot establishes baseline variance and feasibility; confirmatory targets are not to be relaxed after results are observed.

| Dimension | Pilot target | Confirmatory target |
|---|---|---|
| Verified end-to-end recovery | Establish per-family baseline and variance on at least 100 independent scenario clusters | At least 55-60% full-task recovery on the preregistered hard/hidden distribution, and at least +10 percentage points versus the strongest matched baseline; the paired 95% interval must exclude zero and its lower bound should exceed the minimum meaningful effect (initial proposal: +5pp). |
| External benchmark parity | Reproduce published task definitions and scoring without translating scores | No statistically supported regression on the selected SREGym-Lite/ITBench/PagerDuty-compatible tracks; publish exact cohort, system, budget and scorer differences. |
| Continuous operations | Reproduce an approximately 60-minute, ~60-fault, three-application run envelope | Progress to 90-minute / 100-fault runs across at least five app/service profiles, with overlapping faults and code/config changes. Report coverage and complexity rather than claiming equivalence to another benchmark's score. |
| Safety | Zero unauthorized/duplicate critical committed effects in the conformance corpus | Zero observed critical violations in representative confirmatory clusters, with the exact confidence upper bound reported. A zero-event sample is not proof of zero risk. |
| Evidence | All synthetic negative controls deny before participant submission; no false pass on missing/unknown records | 100% of credited recoveries have authenticated, exact-source receipts. Overall planned-cell evidence coverage target at least 99%; remaining cells remain indeterminate. |
| Calibration | Establish reliable probability capture and baseline calibration | Expected calibration error <= 0.05 and lower Brier score than the matched baseline, with cluster-aware intervals; otherwise report calibration as not measured. |
| Human effort | Instrument baseline human-active time and intervention count | At least 30% lower human-active time per verified recovery at non-inferior safety and service outcomes. |
| Fully loaded cost | Establish fully loaded cost per verified recovery | At least 20% lower cost per verified recovery at outcome parity or better. |
| Indeterminate outcomes | Measure sources of unknown/incomplete evidence | Less than 1% of planned cells indeterminate; do not convert indeterminate to zero or exclude them. |

### Rare-event interpretation

With zero observed events in N independent representative clusters, the one-sided exact 95% upper bound on the event probability is:

`1 - 0.05^(1/N)`

Approximate examples:
- 0 / 500 independent clusters: upper bound about 0.60%.
- 0 / 1,000 independent clusters: upper bound about 0.30%.
- 0 / 3,000 independent clusters: upper bound about 0.10%.

These are illustrative bounds only. Independence and representativeness assumptions must be justified. Repeated requests, attempts, windows or seeds nested inside one scenario cluster do not increase the independent-cluster denominator.

## Confirmatory experimental protocol

1. **Freeze both systems.** Candidate and strongest baseline must have complete image/source/model/scaffold/prompt/toolchain/runtime/policy identities. Pin one common execution environment and resource budget, plus state-isolation, artifact-access and network-egress policy digests. Changes after freeze invalidate certification.
2. **Freeze the scenario distribution.** Preregister app/service mix, fault families, combinations, change schedules, business invariants, workload bands, observation windows and exclusions. Generate sealed cases only after the freeze.
3. **Pair at the cluster level.** Candidate and baseline receive the same scenario payload and business oracle, separate isolated execution replicas, the same environment digest, and equal budget ceilings. Randomize execution order/placement and retain all attempts.
4. **Run multi-dimensional stress.** Include single faults, pairwise interactions and selected three-way combinations; staged changes; partial/late telemetry; incorrect/stale dependency data; identity/permission failures; load changes; and post-repair recurrence. Preserve matched single-fault controls.
5. **Keep source roles separated.** Agent, evaluator, freeze authority, participant ledger, ingress/service observer and oracle have distinct control domains. Authenticate receipts through the existing typed source-evidence gate and durable replay guard. A separate runtime observer must attest actual state isolation, oracle artifact separation and retrieval-policy enforcement.
6. **Validate the scorer.** Test sensitivity/specificity against known-answer synthetic controls; use blinded human adjudication for semantic judgments; any insufficient grader evidence makes the result indeterminate.
7. **Account for every trial.** Include completed, failed, skipped, invalidated, retried and timed-out trials. No hidden retry, cherry-picked seed, after-the-fact prompt change or score replacement.
8. **Use cluster-aware statistics.** Preregister the primary estimand and paired comparison. Use cluster bootstrap (10,000 resamples as an initial reporting target), paired uncertainty, and power analysis. Resample scenario/fault-schedule/topology clusters, not individual requests or repeated windows.
9. **Publish claim-specific receipts.** Publish exact digests, evaluation lane, cohort, denominator, exclusions, baseline, metric definitions, confidence intervals, cost and signed result receipt. Do not disclose active sealed cases before retirement.

## Fault and observability progression

The benchmark should not merely add more random faults. It should systematically increase uncertainty and interactions:

- Fault families: deploy/config regression, resource exhaustion, dependency failure, network partition, data corruption, stale cache, identity/authorization, certificate/secret rotation, queue/backpressure, storage/replication, orchestration/scheduler, build/release pipeline.
- Interactions: all selected pairwise combinations among high-frequency families, then targeted three-way interactions where symptoms mask or amplify each other.
- Telemetry: full, delayed, lossy (e.g. 10% and 30% event gaps), contradictory, missing source, clock skew, and corrupted-but-plausible metrics.
- Change stream: versioned release, config drift, rollout rollback, secret/certificate rotation and external dependency change.
- Recovery: rollback, forward repair, degraded mode, failover and recovery itself causing secondary failures.
- Safety: policy-denied but attempted actions, wrong tenant/target, repeated effect, lost response after remote commit, participant ledger lag, and observer-source substitution.

## Claim ceiling

Passing the structural Rust/schema conformance suite proves only that the supplied records satisfy the encoded contract. It does not prove that an agent has achieved any target, that a source is authentic, that the oracle stayed secret, or that the scenario sample represents production. An external SOTA comparison requires actual paired executions and verified receipts; published scores from other benchmarks are context, not substitutes.

Architecture ownership remains unchanged: Platform Saga coordinates episodes; the broker gates mutation authority; participant-owned ledgers own remote effects; independent workload/business observers own service outcomes; Nixward is the only host transaction/interlock/journal/recovery authority.


## Runtime-attestation and sensor qualification targets

Runtime assurance is itself a benchmarked subsystem, not a checkbox. The frozen manifest and receipt validator measure record consistency, but the runtime adapter must be tested with injected ground-truth events to measure actual sensor coverage.

### Required sensor challenge matrix

Run every supported sensor adapter against each supported kernel/distribution/profile combination. Unsupported combinations are explicitly unavailable, never silently "complete." The matrix must include:
- process execution and child-process discovery, including shebang interpreters, binfmt_misc and user-mode helpers;
- file/object access, read-only mounts, namespace changes, writable-layer/copy-on-write behavior and cache access;
- ordinary network sockets, DNS, unix sockets, io_uring and egress policy allow/deny;
- container/cgroup transitions and workload restarts;
- event-buffer pressure, map-capacity pressure, event reordering, queue overflow and dropped-event recovery;
- hook attach failure, hook detachment, program replacement, verifier rejection and sensor shutdown;
- syscall ABI variants and newly introduced syscall paths (for example openat2);
- deliberate clock skew, sequence rollback, stale receipts and duplicate challenge-nonce replay.

For every fault injection, the independent test controller records ground truth outside the sensor path. A separate oracle checks whether the observer detected the event, whether it reported coverage loss, and how long detection took. An observer that silently misses a critical injected event invalidates the run even when it reports coverage=complete.

### Proposed measurable targets

| Dimension | Initial pilot | Confirmatory target hypothesis |
|---|---|---|
| Critical known-answer sensor controls | 100% of positive controls detected or explicitly reported as unsupported | 100% detection of all supported critical controls; any silent miss is a hard qualification failure. |
| Negative controls | Establish false-positive distribution on clean workloads | 0 false-positive hard-policy violations in the preregistered negative-control corpus; report confidence bounds, not "zero risk." |
| Event capture | Quantify loss by source, buffer and kernel profile | Zero unreported drops/gaps; every deliberately induced loss is surfaced as partial/unknown/tampered before outcome credit. |
| Hook health | Measure attach, detach, restart and sustained firing behavior | 100% of planned hooks have authenticated health evidence for the full trial interval; unexpected detach makes coverage indeterminate. |
| Detection latency | Record per-family p50/p95/p99 | p95 under 100 ms for injected policy-critical events, subject to a workload-specific pilot and explicit overhead budget. |
| Runtime overhead | Measure agent CPU/memory and workload p50/p95/p99 separately | Initial hypothesis: <=3% CPU budget attributable to observer and <=2% p99 workload-latency regression at the agreed load. Publish raw paired distributions; do not trade safety against overhead. |
| Kernel portability | Inventory exact kernel, distribution, config, BTF and sensor digests | At least five supported kernel/distribution profiles, each with its own positive/negative control run and exact-head evidence. No "works on Linux" blanket claim. |
| Challenge replay | Test duplicate, stale, cross-run and cross-arm nonces | 100% replay denial before any attestation can be credited; nonce state must survive verifier restart/rollback through the durable evidence adapter. |

These are targets, not measured runtime-agent results. The p95 latency and overhead goals are pilot hypotheses and should be refined only through a preregistered calibration study—not after looking at confirmatory results.

### Instrumentation lessons from industry

Datadog's production eBPF engineering report describes hooks that silently fail, kernel-version compatibility, unobserved syscall paths such as io_uring/openat2, dropped probe activations under capacity pressure, and the need for broad kernel CI and measured service overhead. These are precisely the failure modes the sensor challenge matrix must inject rather than merely assume away: https://www.datadoghq.com/blog/engineering/ebpf-workload-protection-lessons/

The trust architecture should also align to current attestation roles:
- MITRE's July 16, 2026 public-review Continuous Remote Attestation Framework extends beyond launch into runtime integrity, behavioral assurance, transport binding and AI workload verification: https://www.mitre.org/news-insights/publication/framework-continuous-remote-attestation
- NVIDIA's 2026 confidential Kubernetes reference architecture requires fresh evidence for CPU, GPU, guest, workload image, runtime policy and firmware, a verifier-provided nonce, and fail-closed key release: https://docs.nvidia.com/enterprise-reference-architectures/deploying-proprietary-models-securely-with-confidential-compute-self-hosted-kubernetes/latest/required-capabilities.html
- Microsoft's Linux workload guide explicitly separates confidential-VM attestation from workload-image approval; it combines dm-verity, IMA measurement, fresh TPM challenge/quote and verifier-side event-log replay: https://learn.microsoft.com/en-us/azure/confidential-computing/how-to-attest-linux-workload

These references describe mature components and integration patterns, not a turnkey system that proves agent safety. OpsBench should build interoperable evidence adapters around these standards, not custom cryptography or a second policy/attestation authority.
