# Continuous OpsBench: industry comparison and benchmark targets v0.1

## Purpose and claim discipline

This document defines proposed targets for continuous, evidence-carrying operational agents. These are preregistered **goals**, not measured Luminous results. Do not claim better-than-SOTA until candidate and baseline have run on the same frozen systems, workloads, tools, budgets, fault schedules, independent observers, and scorer version, with enough independent clusters to support the stated comparison.

The external figures below provide context, not a combined leaderboard. Their task definitions, environments, and score semantics differ.

## Current industry/reference landscape

| Reference | Published result or scope | What it measures | Why its score is not directly comparable |
|---|---|---|---|
| SRE-Marathon (2026) | Three applications; roughly 60 faults per run; best of 10 methods reached 41.3/100 | Long-horizon continuous SRE with overlapping faults, cumulative alerts, persistent workspace and code/config changes | Its Marathon-Score credits ordered fault progress; do not map that score onto our unique request-impact or safety metrics. |
| SREGym-Lite | The current public leaderboard has top E2E submissions at 100% on the 17-fault Lite cohort; it reports diagnosis, mitigation, E2E, time and tokens | Live Kubernetes diagnosis + mitigation; third-party results on the same run are labeled as verified by maintainers | A small cohort is saturated at the top. Reproduce it as a compatibility track, not the primary generalization claim. The separate SREGym-0508 cohort contains 90 faults. |
| SRE-Marathon (2026) | Across three apps and roughly 60 faults per run, best of 10 methods reached 41.3/100 | Long-horizon operations with overlapping faults, cumulative alerts, persistent workspace and code/config changes | Strong continuous benchmark comparator; its ordered-progress score is not our independently verified service-outcome estimand. |
| ITBench (ICML 2025) | Published SRE resolution result of 11.4% in its evaluated setup; 102 real-world scenarios in the PMLR paper | SRE, CISO and FinOps automation tasks | Broader operational task mix and a different resolution oracle; compare only under the benchmark's exact task and scoring rules. |
| SentinelBench (2026) | 100 tasks across 10 synthetic web environments; task completion, reaction time and resource use | Long-running monitoring, passive/active tasks, external state changes and no-operation controls | Not an SRE benchmark, but directly relevant to event-aware waiting, alert monitoring and resource-efficient sustained attention. |
| Agent reliability profile (ICML 2026) | 15 models evaluated with 12 metrics spanning consistency, robustness, predictability and safety | Pass consistency, perturbation robustness, calibration/predictability, and harm/compliance | Complements task accuracy; use as a reliability-profile template rather than copying a single aggregate. |
| CI-Work (ACL Industry 2026) | Reported privacy violations 15.8–50.9% and leakage up to 26.7% in its evaluated enterprise workflows | Whether agents communicate essential content while withholding sensitive context | A direct contextual-integrity comparator for the privacy track; do not average its leakage rate with operational success. |
| Capable but Careless (EMNLP 2026) | 11 of 15 tested computer-use agents leaked in more than half of scenarios; reported average leakage 67.9% | Cross-app contextual disclosure, task ambiguity and recipient alignment | Useful adjacent test for computer-use agents; not an SRE score. |
| AgentSuite / COBA (ICML 2026) | Proposes component-based benchmark auditing across User, Environment, Ground Truth and Evaluation | Detecting benchmark validity defects and scorer/task/environment artifacts | Adopt a similar component audit before certifying an OpsBench cohort. |
| Agentic leaderboard auditing and search-time contamination (2026) | Search-time contamination was observed across seven public benchmarks, with up to 4% reported inflation in the examined setting | Retrieval leakage, provenance, holdout contamination and grader validity | Public cases can become test-time tool inputs. Active certification must use sealed cases and publish retrieval/trajectory provenance. |

Primary references:
- SRE-Marathon: https://arxiv.org/abs/2609.33023
- SREGym leaderboard and 17-fault cohort: https://www.sregym.com/leaderboard and https://www.sregym.com/problems/cohorts/sregym-lite
- SREGym 90-fault cohort: https://www.sregym.com/problems/cohorts/sregym-0508
- ITBench PMLR paper: https://proceedings.mlr.press/v267/jha25a.html
- SentinelBench (100 tasks / 10 environments): https://www.microsoft.com/en-us/research/publication/sentinelbench-a-benchmark-for-long-running-monitoring-agents/
- Agent reliability profile (12 metrics / four dimensions): https://proceedings.mlr.press/v306/rabanser26a.html
- CI-Work: https://aclanthology.org/2026.acl-industry.103/
- Capable but Careless: https://aclanthology.org/2026.emnlp-main.55/
- AgentSuite / COBA: https://proceedings.mlr.press/v306/suh26a.html
- Search-time contamination: https://aclanthology.org/2026.findings-emnlp.760/
- Princeton agent-evaluation best practices: https://sage.cs.princeton.edu/trends/

## Paradigm shift

Move from **“agent solved an incident”** to **“the frozen operational system produced an independently verified, safe and sustained service outcome.”**

This has eight consequences:

1. **Causal evidence, not agent narration.** A diagnosis is a hypothesis; a tool transcript is an attempt; a participant-owned effect receipt is evidence of an external effect; an independent service observer is evidence of the outcome. These must not collapse into one self-reported success flag.
2. **Continuous operation, not single-fault episodes.** Test simultaneous faults, changing deployments/configuration, delayed symptoms, incident interactions, and post-repair recurrence.
3. **Exact request-set impact, not approximate counter arithmetic.** Use the v0.2 set union of failed, slow and business-incorrect request pseudonyms. Never report request-time by multiplying unique requests by an entire observation interval; request-time requires per-request onset/end evidence.
4. **Matched, frozen controls.** Candidate and strongest baseline must run on the exact same preregistered scenario payload, environment digest, resource-budget digest, business oracle commitment and observation windows. Both arm histories must be present.
5. **Sealed certification, open reproduction.** Public tests accelerate development; sealed cases protect certification; retired cases preserve later reproducibility.
6. **Assurance frontier, not a compensatory score.** Safety, evidence completeness, recovery, service impact, calibration, latency, cost and human effort are separate dimensions. An unauthorized committed effect cannot be compensated by faster remediation.
7. **The evaluator is also evaluated.** Commitments, source authenticity, replay resistance, scorer sensitivity/specificity, observer completeness, and retrieval controls need their own adversarial tests and known-answer controls.
8. **Waiting is a first-class behavior.** Continuous agents should be scored for detecting exogenous state changes, choosing when to re-observe, no-op restraint, alert fatigue, time-to-reaction and resource use. Repeated polling must not be treated as productive activity.

## Metric contract

### Primary operational outcomes

| Metric | Operational definition | Reporting rule |
|---|---|---|
| Verified recovery rate | Fraction of all evaluator-injected faults for which the agent correlates the incident, localizes the root service/artifact, commits the exact authorized participant effect, and restores workload-conditioned SLOs for the required consecutive observation windows before evaluator cleanup | Denominator is all injected faults, including faults that disappear before agent invocation. Report candidate and baseline separately and as paired cluster differences. |
| Unique degraded-request rate | |Failed ∪ Slow ∪ BusinessIncorrect| / offered requests, using observer-issued keyed request pseudonyms | Preserve each dimension and the unique union. Incomplete enumeration is indeterminate. |
| Sustained recovery | Required consecutive healthy windows after the exact committed effect, with adequate offered demand, business correctness, continuous intervals, and independent observer coverage | A recovery claim that fails later windows is not a verified recovery. |
| Time to verified recovery | Time from evaluator fault activation to the first end of the required consecutive healthy-window sequence | Do not substitute process exit, systemd job completion, agent report time, or an effect receipt. |
| Evidence completeness | Fraction of planned trial/observer/participant cells with authenticated, complete and correctly bound evidence | Missing evidence is indeterminate, never a pass or zero harm. |
| Safety integrity | Unauthorized committed effects, out-of-scope effects, duplicate critical effects, forbidden attempts, false repair claims, replay, source substitution | Hard non-compensatory gates. Report attempted and committed violations separately. |

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

## Monitoring and waiting track

Adopt a SentinelBench-inspired, but operations-specific, set of tasks alongside the SRE continuous track:
- passive wait-for-signal tasks (do not act before the event);
- active monitoring tasks (periodically inspect a source to reveal new information);
- relative-baseline tasks (remember a metric's initial value);
- no-op tasks where the event never occurs;
- urgent ephemeral tasks where delayed reaction causes real loss;
- alert-churn and noisy-signal tasks with false positives and correlated events.

Measure event-detection recall/precision, time-to-detection, time-to-correct-action, false wakeups, polling/tool-call count, tokens, energy where available, cost per correctly handled event, missed-event rate, and no-op violation rate. The benchmark should vary event-time distribution and observation affordances. It must include observer-known event timelines so that delayed detection is distinguishable from an event that never occurred.

## Benchmark component audit (before an agent run)

Every benchmark cohort should have a validity dossier covering four components:
- **User/task contract:** ambiguity, instruction consistency, required/forbidden outcomes, and task-to-oracle traceability.
- **Environment/tools:** deterministic reset, versioned state, tool semantics, permissions, fault injectors, and environment drift.
- **Ground truth:** oracle provenance, label completeness, independent positive/negative known-answer cases, and ambiguity policy.
- **Evaluation:** exact code/digest, metric semantics, failure handling, statistical plan, and scorer mutation tests.

Inject controlled defects into each component and verify that the audit rejects or flags them. This adopts the component-level idea from AgentSuite/COBA; it does not claim their pipeline has been implemented here.

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

This sensor-fitness work is tracked separately in [issue #205](https://github.com/Luminous-Dynamics/luminous-platform/issues/205). These references describe mature components and integration patterns, not a turnkey system that proves agent safety. OpsBench should build interoperable evidence adapters around these standards, not custom cryptography or a second policy/attestation authority.
