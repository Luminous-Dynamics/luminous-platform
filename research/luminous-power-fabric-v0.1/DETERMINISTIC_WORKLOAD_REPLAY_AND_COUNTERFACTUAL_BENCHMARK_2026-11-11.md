# Deterministic Workload Replay and Counterfactual Flexibility Benchmark v0.1

Research snapshot: 11 November 2026.

Status: **read-only deterministic replay engine and seventeen synthetic Rust unit tests authored; tests not executed**. This is a normalized workload scheduler/replay model. It is not a production scheduler, GPU controller, measured power model, grid-service qualification, or SOTA scheduling claim.

## 1. Why this increment matters

Power Fabric already has:
- matched pointwise power/QoS comparison (#392);
- fixed-fleet and capacity-scaled Joule Point analysis (#394);
- site/PCC compute-only attribution and repeated-event reliability (#396).

The missing link was an executable answer to a more fundamental question:

**Which jobs can actually move in time under a dispatch-constrained compute-capacity profile, and what does that do to useful work, deadlines, non-deferrable service, resource occupancy and recovery?**

A percent-of-load headline does not answer this. It can hide lost jobs, interactive-request delays, resource fragmentation, non-preemptible tasks already in flight, or a backlog that takes hours to drain. This replay compares the exact same normalized jobs in a full-capacity baseline and a dispatch-constrained candidate.

The first implementation deliberately stops at CPU/GPU/memory resource-time. It does not infer watts from generic GPU units.

## 2. Industry comparison

### LAD-Flex (May 2026)

Caprara et al., *Data center workload flexibility for power system demand response: Evidence from Alibaba traces* (DOI [10.1016/j.ijepes.2026.111940](https://doi.org/10.1016/j.ijepes.2026.111940)), use more than one million Alibaba cluster tasks in a one-week trace case. Their Latency-Aware Deferral for Flexibility method identifies latency-tolerant tasks and reports up to 22% **modelled** load shedding during flexibility windows. The data-availability statement says supporting data are available on request from the authors.

This is a useful reproduction target. It is not a measured 22% reduction at a facility PCC, a universal value, or an independently reproduced Luminous result.

### Heterogeneous batch shifting (September 2026)

Su, Du and Wang, *Grid Demand Flexibility Assessment of AI Data Centers via Batch Workload Temporal Shifting* (arXiv:2609.38020, 29 September 2026), map heterogeneous CPU/GPU/memory usage into grid-operation time intervals, preserve execution continuity, task-delay constraints and server-resource capacities, and couple the scheduler to a utilization-dependent server power model. They evaluate **peak demand shaving** and **maximum continuous sustained-reduction duration** as separate capabilities.

This is the closest direct method comparator for the new workload replay. The Luminous v0.1 engine adds explicit per-job source references, exact baseline/candidate schedules, hard start/completion bounds, a clear admission-only dispatch model, and resource-time accounting. It does not yet reproduce their utilization-to-power model or optimization formulation.

### OpenDC task and trace model

OpenDC's current workload specification supports submission time, duration, CPU/GPU/memory demand, deferrable status, deadlines, and task parent/child dependencies. Its documentation also distinguishes workload traces and task fragments, and documents deferrability/deadline semantics.

The Luminous normalized record adopts the subset needed for a first non-preemptive replay: release, latest start, hard finish deadline, constant runtime and a three-dimensional resource vector. Parent/child dependencies and time-varying per-task fragments are explicitly not silently flattened by this module.

### Alibaba production traces

Alibaba's current Cluster Trace catalogue lists:
- a six-month 2026 GPU trace covering up to 155,410 GPUs across 37,707 GPU servers;
- a 2026 GenAI serving trace with application, middleware and infrastructure layers;
- a six-month 2026 MaaS trace covering online and offline LLM inference workloads.

The repository encourages research use, but its root catalogue does not identify a clear dataset licence for every data product. A public download link or research invitation is not a blanket commercial-use, derivative-work or redistribution grant. The existing dataset registry (#391) remains the rights gate; obtain and preserve the exact data-product terms before importing any trace. No Alibaba dataset files are downloaded or included by this increment.

### Facility-level carbon consequences

Senga, Wang and Knittel's June 2026 *iScience* study, *Flexible data centers reduce power system costs but can increase emissions* (DOI [10.1016/j.isci.2026.116497](https://doi.org/10.1016/j.isci.2026.116497)), models Texas, the Mid-Atlantic and WECC. It finds system costs fall under its studied flexibility scenarios, while emissions outcomes depend on the generation mix; flexibility raises emissions in some Mid-Atlantic and high-renewable-cost scenarios.

Therefore lower energy price, lower site peak, lower system cost and lower CO2 are different outcomes. Later work must use location- and time-specific marginal emissions/power-system evidence. This replay does not make a carbon claim.

### Power evidence remains a separate join

NLR's GenAI Workload Power Profiles provide measured Kestrel workload power traces at 5/10 Hz and separate model-generated facility profiles. Argonne JLSE's Data Center Flexibility Dataset project plans to combine B200/H100 GPU/CPU/DRAM power and temperature telemetry with per-request TTFT/latency/tokens-per-second during control sweeps. The Argonne page is a project description, not a released dataset.

These resources illustrate the eventual join: normalized job schedule + measured resource utilization + measured power curve + a declared PCC/facility boundary. The workload replay alone cannot supply that join.

## 3. Normalized replay contract

The WorkloadReplayContract binds the replay to:
- immutable replay-contract ID;
- workload ID and dataset ID/version;
- dataset evidence-manifest reference;
- use-authorization or dataset-registry decision reference;
- normalization revision and scheduling-policy revision;
- explicit useful-work unit;
- full CPU/GPU/memory resource capacity.

Each WorkloadJob contains:
- stable job ID and source evidence reference;
- release timestamp;
- latest permitted start timestamp;
- hard completion deadline;
- non-preemptive constant runtime;
- CPU milli-core, GPU capacity-unit and memory-MiB demand;
- accepted useful-work units;
- NonDeferrable or Deferrable class.

A non-deferrable job has latest_start_ns equal to release_time_ns: the v0.1 contract gives it no grid-induced start slack. A deferrable job may have a later latest-start timestamp, but must still finish by its hard deadline.

Every job's resource vector must fit the full declared facility capacity. It is invalid to count an impossible job as flexible work.

## 4. Dispatch and scheduler semantics

A dispatch-capacity window provides temporary **admission limits** for CPU/GPU/memory. It does not cancel, pause, throttle, checkpoint or preempt jobs already running when the window begins. That is a specific workload-deferral model, not a model of GPU power caps or converter response.

The deterministic v0.1 policy is:
1. non-deferrable jobs before deferrable jobs;
2. earliest latest-start timestamp;
3. earliest completion deadline;
4. release timestamp;
5. stable job ID.

It is strict-priority and non-backfilling: a higher-priority ready job that cannot fit blocks lower-priority work until a resource completion or capacity transition. This avoids silently changing task order to inflate the flexibility result. It is a conservative baseline policy, not an optimal scheduler. Later increments should compare FCFS, EDF/backfill, LAD-Flex, and a matched optimization/MILP oracle using identical jobs, capacities and deadlines.

The engine is event-driven:
- release order is sorted once;
- a priority-ordered ready set holds released jobs;
- a completion set holds running-job finish events;
- CPU/GPU/memory usage is incrementally accounted;
- dispatch windows use ordered lookup;
- resource-time integration scans only overlapping windows.

This avoids rescanning the entire workload on each event. The source contains a 10,000-job deterministic regression fixture, but it has not been run or timed; there is no million-job throughput claim.

## 5. Outputs and units

The evaluator returns the same jobs' baseline and dispatch-constrained schedules, including per-job start/finish timestamps and deadline outcomes.

Aggregates include:
- total/completed/missed jobs;
- non-deferrable and deferrable misses;
- total/completed/missed useful-work units;
- useful-work retention versus the baseline;
- count and work units for delayed jobs that still completed;
- P50/P95/P99 and maximum strictly positive start delays;
- maximum recovery tail after the final dispatch window;
- baseline and candidate GPU resource-time during dispatch windows;
- GPU resource-time reduction when candidate occupancy is no greater than baseline;
- theoretical GPU admission-capacity restriction in GPU-unit-nanoseconds;
- total and longest dispatch-window duration.

The units are intentionally precise:
- **GPU-unit-nanoseconds** are scheduled resource occupancy integrated over time.
- **GPU admission restriction unit-nanoseconds** are the declared difference between full and capped admission capacity integrated over time.

Neither quantity is watts, joules, kWh, power shed, energy saved, PUE-adjusted facility demand, or accredited grid capacity. If a candidate misses work, lower GPU resource-time can reflect service failure; the report exposes lost useful work and deadline misses instead of presenting the lower occupancy as a successful flexibility result.

## 6. v0.1 validation contract

Twenty Rust unit tests are authored. They cover:
- deterministic identical baseline/empty-window replay;
- deadline-respecting deferrable shifts;
- non-deferrable work blocked from dispatch-induced delay;
- in-flight jobs continuing through dispatch;
- CPU/GPU/memory vector constraints;
- exact latest-start/deadline boundaries;
- capacity restoration and backlog recovery;
- resource-time not labelled as power/energy;
- stable tie-breaking;
- adjacent versus overlapping dispatch windows;
- impossible resource requests and invalid windows;
- invalid deadlines and duplicate IDs;
- missing provenance/use-authorization references;
- separate reporting for recovery tail and lost work;
- timestamp overflow;
- delay percentiles;
- event-sourced occupancy timeline and timestamped useful-work completion events;
- exact half-open boundary ordering at a simultaneous completion/dispatch/start timestamp;
- CPU/GPU/memory admission restriction integrals and conservation negative controls;
- a 10,000-job deterministic regression fixture.

**Tests are authored but not executed.** Compilation, cargo test, cargo fmt, cargo clippy and runtime/performance measurements are not claimed. The engine is read-only and uses synthetic fixtures only.

## 6A. Event-sourced timeline export and multi-resource flexibility tensor (v0.2)

The replay now emits both baseline and dispatch-constrained event-sourced outputs:

- **Occupancy segments** are exact half-open intervals [start_ns, end_ns) with active CPU milli-cores, GPU units, memory MiB, active job counts by deferrability class, full resource capacity, admission capacity and optional dispatch-window ID.
- **Completion events** carry exact finish timestamp, job ID, source evidence ID, deferrability class, useful-work units, work unit and replay-contract ID. These are the accepted-useful-work events for downstream time-bin integration.
- **Clock contract** carries a named monotonic clock ID and time-origin ID. A caller-supplied schedule digest reference is preserved for downstream evidence binding; this module does not compute or authenticate that digest.
- **Resource restriction tensor** reports CPU-millicore-nanoseconds, GPU-unit-nanoseconds and memory-MiB-nanoseconds separately. They are abstract admission-capacity restrictions, not watts or energy.
- **Conservation checks** require occupancy-segment resource-time to reconcile exactly with per-job scheduled resource-time during dispatch windows, and completion-event sums to reconcile with completed jobs and useful-work units.

At equal timestamps, the replay uses half-open semantics: complete jobs at t, apply the capacity-window boundary at t, then admit newly released jobs. The resulting timeline is intended as an input to issue #382's existing matched PCC/useful-work auditor. It does not duplicate PCC energy integration, power quality checks or stored-energy accounting.

This export makes the workload-to-power join possible without guessing how aggregate work maps to time. It still does not provide a measured power trace, power curve, facility overhead, carbon intensity or market qualification.

## 7. Proposed benchmark programme

Targets below are research objectives, not standards or results.

| Benchmark | Target | Why |
|---|---:|---|
| Replay determinism | 100% identical outputs for frozen inputs, policy and version | Required for reproducible counterfactuals |
| Non-deferrable service | Zero dispatch-induced non-deferrable misses in eligible runs | Protects interactive/critical workloads |
| Useful-work retention | >=99%, with every miss visible | Prevents power reduction by doing less work |
| Workload deadlines | Every completed job finishes inside its declared deadline | Preserves explicit service contract |
| Flexibility shape | Report peak reduction and maximum continuous duration separately | Prevents a single scalar from hiding duration limits |
| Queue behaviour | Publish P50/P95/P99 delay, misses, end-window backlog and recovery tail | Makes rebound/payback visible |
| Scale | 1M-job replay benchmark with declared memory/runtime limits and reproducible hardware | Makes current trace-scale evaluation practical; target values must be set after local measurement |
| SOTA replication | Reproduce LAD-Flex within 1 percentage point on the same authorized trace, assumptions and power model before claiming an extension | Ensures matched comparison |
| Power coupling | Calibrated utilization-to-power maps with uncertainty and matching PCC boundary | Converts resource-time into watts without hand-waving |
| Field qualification | Feed the measured event outputs into #396 only after independent site/PCC evidence | Prevents a simulator from certifying a real resource |

The intended improvement is not to claim "more than 22%" against a study-specific model using a different workload or power assumption. First reproduce the study exactly. Then extend it with measured power, multi-resource constraints, site attribution, uncertainty, recovery and repeated-event dependability.

## 8. Work still required before SOTA or deployment claims

The v0.1 engine intentionally omits:
- DAG dependencies and parent/child release propagation;
- time-varying per-job resource fragments;
- checkpoint/restart, preemption and migration costs;
- stochastic arrivals, SLA distributions and online serving queue models;
- network, storage, cooling, thermal, failure reserve and scheduler contention;
- workload utilization to board/server/PCC power conversion;
- carbon intensity, nodal marginal emissions and electricity market prices;
- a trace parser and verified adapters for each dataset format;
- multi-resource fleet scaling beyond one abstract resource pool.

These should be separate, testable increments. The next logical work is a permission-aware OpenDC/Alibaba normalized-trace adapter plus DAG-aware scheduling, then a separately evidenced utilization-to-power join. Avoid adding a power curve based on arbitrary TDP fractions.

## 9. Safety and claims

This is a deterministic offline replay/evaluation function. It has no connection to production schedulers, Kubernetes, Slurm, GPU drivers, power caps, storage/UPS, converters, market bidding or grid operators. No job is actually moved, stopped or throttled.

No external dataset binaries are bundled. No measured watt reduction, energy saving, carbon saving, site response, grid-service eligibility, utility accreditation or certification is claimed.

## Sources

- A. Caprara et al., [Data center workload flexibility for power system demand response: Evidence from Alibaba traces](https://doi.org/10.1016/j.ijepes.2026.111940), International Journal of Electrical Power & Energy Systems, May 2026.
- S. Su, L. Du and S. Wang, [Grid Demand Flexibility Assessment of AI Data Centers via Batch Workload Temporal Shifting](https://arxiv.org/abs/2609.38020), arXiv:2609.38020, 29 September 2026.
- [OpenDC workload input documentation](https://opendc.org/learn/documentation/Input/Workload), accessed 11 November 2026.
- [Alibaba Cluster Trace Program](https://github.com/alibaba/clusterdata), current catalogue accessed 11 November 2026.
- J. R. L. Senga, S. Wang and C. R. Knittel, [Flexible data centers reduce power system costs but can increase emissions](https://doi.org/10.1016/j.isci.2026.116497), iScience, 26 June 2026.
- NLR, [Dataset of Generative AI Workload Power Profiles](https://data.nlr.gov/submissions/312), DOI 10.7799/3025227, accessed 11 November 2026.
- Argonne JLSE, [Data Center Flexibility Dataset project](https://www.jlse.anl.gov/projects/benchmarking/data-center-flexibility-dataset), accessed 11 November 2026.
