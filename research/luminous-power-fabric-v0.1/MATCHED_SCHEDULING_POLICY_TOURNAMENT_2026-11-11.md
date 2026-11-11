# Matched Scheduling-Policy Tournament and Capability Frontier v0.1
**Date:** 11 November 2026  
**Issue:** [#402 — matched scheduling-policy tournament](https://github.com/Luminous-Dynamics/luminous-platform/issues/402)  
**Workload replay/timeline:** [PR #400](https://github.com/Luminous-Dynamics/luminous-platform/pull/400)  
**Power/QoS Pareto:** [Issue #392](https://github.com/Luminous-Dynamics/luminous-platform/issues/392)  
**Repeated-event site qualification:** [Issue #396](https://github.com/Luminous-Dynamics/luminous-platform/issues/396)  
**Calibrated state-power model:** [Issue / draft PR #401](https://github.com/Luminous-Dynamics/luminous-platform/pull/401)

## 1. Why a policy tournament is required

A deterministic scheduler is a reproducible baseline, not an optimality result. A single no-backfill policy cannot establish that a workload-shifting method is competitive, fair, or capable of maximizing useful work under the same dispatch constraint.

The tournament will compare policies on identical immutable workload inputs and report a **capability frontier**, not a single composite score. The benchmark unit is work that completes under the declared service contract, not merely a job that was admitted or a GPU that was idle.

Hard service, privacy, authority or evidence constraints are not exchangeable for a more attractive peak-shaving number.

## 2. Frozen input contract

Every policy receives the same:
- workload corpus and version, digest and explicit use authorization;
- normalized job/task identity, release time, latest start, hard deadline, runtime or time-varying fragment profile, CPU/GPU/memory demand, dependencies and accepted useful-work contract;
- full capacity profile and time-varying dispatch admission-capacity windows;
- service classes, SLO/TTFT/latency constraints and permitted deferral;
- replay clock, time origin, sample cadence and normalization revision.

Every arm must emit a schedule with per-job or per-fragment start/finish evidence, completion state, service outcome and resource occupancy. Compare only cases whose semantics can be represented by every arm. When a baseline lacks a feature (for example task DAGs, preemption, or task fragments), label the result **not comparable** rather than flattening the feature or silently changing its semantics.

The first implementation may consume #400's frozen constant-duration, non-preemptive job subset. OpenDC task+fragment imports and DAG dependencies are a later adapter stage; do not average time-varying fragments into one constant resource vector.

## 3. Policy arms and exact semantics

| Arm | Required semantics | Main failure mode to expose |
|---|---|---|
| FCFS/FIFO | Stable arrival order; explicit tie-break on release timestamp then job ID | Head-of-line blocking and missed deadlines |
| EDF | Earliest hard deadline among ready jobs; stable tie-break; latest-start feasibility checked before admission | Deadline myopia and resource fragmentation |
| EDF + backfill | EDF reservations; admit a later job only if it cannot delay the protected reservation; document reservation horizon and starvation guard | Reservation displacement, starvation and hidden deadline risk |
| Strict-priority/no-backfill | Existing #398/#400 reference semantics, pinned to exact policy revision | Reference-policy bias and avoidable idle capacity |
| LAD-Flex reproduction | Faithful published formulation and parameter mapping on the same lawfully accessible trace and assumptions | Non-reproducibility from unavailable/mismatched data or model inputs |
| Exact small-instance oracle | Independently formulated MILP or equivalent exact search; solver/version and proof/gap evidence retained | Heuristic regret and incorrect “optimal” claims |

No arm may preempt running work unless preemption/checkpoint/restart is an explicit shared workload capability with the same cost and failure semantics applied to every eligible policy.

The EDF-backfill arm must publish its reservation algorithm. A “backfill” label without a reservation guarantee is not sufficient.

## 4. Exact-oracle protocol

The oracle is for small tractable subsets and serves as a bound, not as a production scheduler. Freeze:
- decision variables and time discretization;
- all CPU/GPU/memory capacity constraints;
- releases, deadlines, dependencies, service classes and dispatch windows;
- the objective and any lexicographic/epsilon constraints;
- solver name/version, parameters, deterministic seed where applicable, time limit, status, incumbent, best bound and optimality gap.

Use lexicographic/epsilon-constrained objectives rather than an opaque weighted sum:
1. Satisfy hard authority/service/resource constraints.
2. Minimize hard service violations (target zero in a service-feasible fixture).
3. Maximize accepted useful work or minimum qualified flexibility, one preregistered objective at a time.
4. Compare delay, fairness, recovery tail and resource use as separate dimensions.

A result is “optimal” only when the solver proves optimality for the exact formulation. A timed-out incumbent is a feasible solution with a published gap, not an optimum. Do not publish heuristic “optimality gap” against an unproven oracle.

Start with a tiny exhaustive corpus, then grow the exact corpus by recorded solver/time limits. Report solver runtime and memory separately from policy runtime; do not include oracle cost in ordinary scheduler latency.

## 5. Independent scoreboards

Never collapse all dimensions into a synthetic scalar rank.

### A. Accepted useful work and service integrity
- total useful work and useful work accepted under the same contract;
- retention versus full-capacity baseline;
- non-deferrable and deferrable deadline misses;
- SLO/TTFT/latency violation fraction;
- p50/p95/p99 start delay, completion delay and maximum delay;
- starvation count/longest starvation and job-class/tenant-level tails;
- end-of-window backlog and post-dispatch recovery tail.

### B. Resource flexibility
- CPU millicore-ns, GPU-unit-ns and memory-MiB-ns restriction/occupancy as separate dimensions;
- peak and time-resolved resource-demand reduction;
- maximum sustainable dispatch duration and response delay;
- backlog growth, recovery slope and delayed-work rebound.

These are resource-unit-time statistics. **They are not watts, joules, kWh, MW or site/PCC power reduction.**

### C. Electrical and energy outcomes
- With #401, compare only modelled traces on supported exact configuration/state domains, and retain MODELLED labels and uncertainty.
- With #382, compare measured PCC input/output power, energy, energy per accepted useful work, ramp, peak and storage-adjusted results under matched baseline/workload/window/initial stored energy.
- With #396, evaluate repeated-event location-specific compute-derived capacity and response/sustain/rebound/reliability gates.

Do not duplicate #382's electrical integrator or #396's repeated-event qualification. Do not equate lower cost or load with reduced CO2; use separately sourced locational/temporal emissions evidence for that question.

### D. Fairness and owner externalities
- Per-tenant and per-class accepted work, delay and miss rates;
- owner-reserved capacity and minimum service floor;
- max-min/weighted fairness as additional reported metrics, not a hidden objective;
- cross-owner data movement, network egress and duplicated work where relevant;
- provider-side thermal, energy and capacity externalities when measured.

A high fleet average must not hide sacrificed owners, tenants or service classes.

### E. Reproducibility and uncertainty
- paired policies on identical workload seeds/trace windows;
- randomized policy execution order to control thermal/time drift in physical experiments;
- preregistered warm/cold state and repetition protocol;
- P50/P90/P99 and paired confidence intervals where sample assumptions permit;
- exact source SHAs, input/output digests, policy revisions, configuration fingerprints and denominator accounting;
- all late, missed, rejected and indeterminate jobs retained in the denominator.

Use block/cluster-aware intervals for correlated runs; do not treat adjacent time samples as independent policy trials.

## 6. Adversarial reference corpus

Synthetic tests must cover:
1. Equal-time job completion, capacity-window boundary and release.
2. Deadline cliff: exact latest feasible start versus one time unit late.
3. Resource fragmentation: CPU fits but GPU or memory does not.
4. Long-job head-of-line blocking and safe backfill reservation.
5. Starvation under continuous high-priority arrivals.
6. Bursty synchronized releases and recovery collisions.
7. Mixed service classes with strict non-deferrable protection.
8. A candidate that appears to shed load by losing useful work.
9. A candidate that delays work until after the measured window (rebound).
10. Heterogeneous runtimes and time-varying fragment demand once the adapter exists.
11. Dependency DAGs and parent/child completion constraints when represented.
12. Integer-boundary/overflow cases and deterministic tie-break replay.

Every fixture must have an independently computed expected schedule or an exact small-instance oracle result.

## 7. Competitive literature and industry position

- **LAD-Flex (Caprara et al., May 2026):** reports up to 22% modelled load shedding in a study-specific one-week Alibaba workload case. This is a useful policy reproduction target, not an independently measured facility reduction.
- **Su, Du and Wang (29 September 2026):** models heterogeneous CPU/GPU/memory demand, execution continuity, deadlines, peak shedding and maximum sustained reduction separately.
- **OpenDC:** task records and time-varying fragments provide a stronger workload representation than constant average resource demands.
- **IEA 4E EDNA (1 July 2026):** separates market-serving, grid-serving and system-serving flexibility. The benchmark will report these as distinct service classes.
- **2026 workload-migration field demonstration:** measured response-time feasibility for an inter-data-centre migration scenario and shows that data accessibility and pre-launch strategy affect service feasibility. It is not a universal fleet result.
- **Microsoft Research (March 2026):** provides the adjacent power-model comparator (state transitions + configuration-conditioned power distributions + autocorrelation), not a scheduler ranking.
- **MLPerf Power (2026):** supplies matched measured work/joule discipline; it does not measure grid flexibility or deadline-preserving deferral.

## 8. Proposed research targets, not achieved results or standards

| Gate | Internal initial target |
|---|---:|
| Frozen-input policy determinism | 100% identical schedule/output for identical engine + policy + input revisions |
| Service-feasible non-deferrable workload | 0 hard deadline misses |
| Useful-work retention | >=99%, with each class/tenant reported separately |
| Resource/event accounting | exact zero-residual conservation |
| Oracle reference | exact proof for tiny corpus; all time-limited cases disclose incumbent and gap |
| Uncertainty | paired 95% confidence intervals with appropriate run/session clustering |
| Fairness | no hard owner/service-floor violation; per-class tails reported |
| Benchmark denominator | 100% of eligible jobs accounted for as completed, missed, rejected, late or indeterminate |
| Competitive outcome | at least one reproducible, supported Pareto improvement over a matched baseline; publish counter-regimes and no-win regimes too |

These targets are internal research gates only. No target is claimed attained.

## 9. Execution sequence and work ownership

1. **Reference policy harness:** expose an interface over a frozen #400 input and deterministic output contract.
2. **FCFS and EDF:** add simple, auditable arms with stable tie-breaking.
3. **EDF backfill:** reservation-safe algorithm plus starvation fixtures.
4. **Exact oracle:** tiny independent corpus and solver-backed optimum/gap evidence.
5. **LAD-Flex reproduction:** only after lawful, exact trace and parameter access.
6. **Power join:** #401 modelled results; #382 measured electrical comparisons.
7. **Repeated service qualification:** #396 only after site/PCC measured gates and repeated-event protocol.

## 10. Claim boundary

This specification is not a scheduler implementation, optimization result, physical measurement, field test, carbon result, grid-service accreditation or market bid. The first tournament release is offline and counterfactual. The policy winner, if any, must emerge from matched evidence rather than be selected in advance.
