# Power Flexibility versus QoS: Pareto Benchmark v0.1

Research snapshot: 10 November 2026.

Status: **offline matched-run evaluator and eleven synthetic Rust unit tests authored; tests not executed**. This is not a GPU power-cap controller, workload scheduler, demand-response aggregator, grid service, HIL/PHIL test or product qualification.

## 1. Why a new benchmark is needed

A data centre can reduce power by doing less work, delaying requests, increasing latency, changing output quality, consuming stored energy, or shifting energy into a recovery period. A benchmark that reports only average watts or energy per token cannot distinguish genuine flexibility from an unacceptable service regression.

Recent public work identifies the necessary inputs:
- **ELF: Energy-Latency Frontier** (Zenodo 10.5281/zenodo.23074732) sweeps board power limits and graphics clocks across 20 inference models and four GPU platforms, measuring board power, throughput and a four-phase latency split. It is a direct power-versus-service comparator.
- **LLM-PowerTrace** (arXiv:2610.38315, 28 October 2026) defines energy/token, P99 ramp rate, normalized spectral entropy and a per-node GPU synchronization factor. It describes 100 ms per-GPU telemetry and controlled model/serving configurations. The complete dataset is described as released upon acceptance, so verify the actual dataset release and licence separately.
- **Argonne JLSE's Data Center Flexibility project** plans power-cap/clock/batch/concurrency sweeps while measuring GPU/CPU/DRAM power and per-request TTFT, latency and tokens/second. It explicitly targets power flexibility and performance flexibility functions. It is a project plan, not a currently verified public dataset.
- **Microsoft Research's From Servers to Sites** composes workload states and power profiles from GPU/server to facility levels, reporting median absolute energy error below 5% for most evaluated configurations while preserving temporal autocorrelation.

The v0.1 Luminous evaluator does not attempt to recreate these studies. It defines a strict matched-comparison and Pareto contract into which their permissible measurements can eventually be imported.

## 2. Benchmark contract

Freeze the following before comparing a control action:
- workload/model/precision and accepted-output quality contract;
- request arrival mix, input/output length distribution, concurrency and prompt/replay seed;
- serving stack, GPU model, hardware/firmware and cooling boundary;
- PCC or other clearly identified electrical boundary;
- measurement window and timebase;
- initial stored energy for every storage element inside the boundary;
- QoS/SLO definition and the useful-work unit.

Baseline and candidate must use the same workload contract, request mix, quality contract, physical boundary, exact time window and initial stored energy. This first version rejects a mismatched scenario rather than attempting to resample or normalize away the mismatch.

Candidate power can be modified, but the service contract cannot silently change.

## 3. Required metrics

| Axis | Measurement | Why it matters |
|---|---|---|
| Power capability | Mean/peak active power, absolute and fractional shed | Quantifies available reduction |
| Dynamic response | Response latency, P99 ramp, settling, overshoot and recovery | Quantifies how quickly and cleanly demand can change |
| Useful work | Accepted output tokens, completed requests or validated training work | Prevents reducing power by doing less |
| Service quality | TTFT, p50/p95/p99 latency, SLO violation rate and task quality | Preserves operator/customer contract |
| Energy | PCC energy corrected for storage delta; joules per accepted work unit | Exposes energy penalties and storage depletion |
| Rebound | Recovery-window energy, restoration peak and ramp | Prevents shifting a burst to after the control action |
| Thermal/availability | Steady-state temperatures, throttling, failure/fallback and repeatability | Distinguishes sustainable flexibility from a short transient |

For a run inside a declared storage boundary:

E_effective = E_PCC - (E_stored,final - E_stored,initial)

and:

E_per_work = E_effective / W_completed

A depleted store increases effective input energy even when the instantaneous PCC trace is lower. The new evaluator uses this corrected energy per work unit.

P99 ramp is reported independently from mean power shed. A strategy can reduce mean power while creating sharper ramps or synchronized device transitions.

## 4. Feasibility before ranking

The v0.1 operating-point gate contains:
- Minimum mean power-shed fraction.
- Maximum useful-work loss.
- Maximum P99 latency increase.
- Maximum P99 time-to-first-token increase.
- Maximum SLO violation fraction.
- Maximum energy-per-work regression.
- Maximum recovery-rebound energy as a fraction of storage-adjusted energy saved.
- Minimum sustained action duration.

All limits are explicit inputs. The current test fixture uses a proposed 5% power-shed floor, 1% maximum useful-work loss, 5% P99 latency/TTFT increase, 1% SLO violation ceiling, no energy-per-work regression, 10% rebound ceiling and 60-second minimum sustained action. These are experimental gates, not standards, vendor requirements or demonstrated Luminous performance.

A candidate enters the Pareto frontier only when it passes the full service contract and the minimum power-shed gate. A 30% shed candidate that loses 17% of useful work or violates the latency contract is excluded, even when its electrical ramp is substantially smoother.

## 5. Pareto interpretation

The non-dominated frontier seeks to:
- maximize mean power shed;
- maximize useful-work retention;
- maximize P99 ramp reduction;
- minimize P99 latency and TTFT increase;
- minimize energy-per-work regression;
- minimize recovery rebound.

It intentionally does not return a single scalar winner. Different operators may assign different value to response speed, energy savings, latency slack and available reserve. The frontier lets the decision-maker see the trade-offs rather than burying them in an arbitrary weighting.

A candidate that is dominated on every objective is discarded from the frontier. A service-feasible point can remain non-dominated even when another point offers more power shed but worse ramp reduction or latency.

## 6. v0.1 implementation

The new Rust module tools/measurement-audit/src/power_flexibility.rs adds:
- evaluate_power_flexibility_sweep: compares one baseline against a set of controlled candidates.
- FlexibilityGate: caller-declared work, QoS, energy, rebound, duration and power-shed criteria.
- Per-candidate metrics for power shed, useful-work retention, P99 latency/TTFT, SLO, energy per work, P99 ramp, response latency, power overshoot, recovery settling, energy saved and rebound.
- A Pareto-frontier extraction that includes only candidates meeting the service and power-flexibility gates.

The evaluator requires a measurement-contract-valid flag and evidence ID for each run, but it does not authenticate those evidence references. Input traces/measurements must first pass the existing measurement and evidence workflows.

Twelve synthetic unit tests cover a successful 5% shed, a lower-power/lower-work negative control, latency and TTFT failures, storage depletion, rebound excess, Pareto dominance, workload/boundary/time/SOC mismatch, unknown rebound, invalid measurement contracts, duplicate runs and invalid gate values.

**Tests are authored but not executed.** There is no claim of Rust compilation, formatting, Clippy, live GPU control, physical PCC measurement, HIL/PHIL, EMT, demand-response participation or utility qualification.

## 7. Industry comparison and what would count as an advance

| Comparator | Existing strength | Luminous next step |
|---|---|---|
| ELF | Direct power-cap/DVFS versus throughput and phase latency | Reproduce its matched frontier, then add recovery, storage-state and facility/PCC effects |
| LLM-PowerTrace | Per-GPU trace, energy/token, P99 ramp, spectral entropy and synchronization metrics | Match these metrics and extend them to QoS-constrained PCC response, storage and rebound |
| Argonne JLSE planned project | Direct power-flexibility and performance-flexibility functions | Add cross-boundary trace provenance, reproducible control-action evidence and lifecycle/degradation |
| Microsoft From Servers to Sites | Workload-state composition and facility-scale trace synthesis | Couple generated facility load to real control-action tests and matched grid-facing service contracts |

Luminous should not claim novelty for energy/token, P99 ramps, spectral entropy or power-cap/DVFS sweeps alone. The differentiated contribution is the evidence-linked chain from a fixed useful-work contract through controlled GPU response, rack/facility storage and PCC behaviour, with explicit uncertainty and requalification.

## 8. Proposed experimental targets

Initial internal research gates:
- 5% sustained mean-power shed over 60 seconds with >=99% useful-work retention and <=5% P99 latency/TTFT increase.
- Stretch: 10% power shed under the same service contract.
- At least 20% reduction in P99 ramp under the same workload and sampling/filter definition.
- No energy-per-work regression outside the combined uncertainty budget.
- Recovery rebound energy <=10% of storage-adjusted energy savings over a declared recovery window.
- Repeat all claims after thermal equilibrium and across multiple independent workload seeds/configurations.

Report the entire Pareto frontier and confidence/uncertainty intervals. Never choose a single best point by averaging away tail latency, workload quality or synchronized ramps.

## 9. Next increments

1. Inspect ELF's actual files, high-rate power-cap actuation trace and exact sample timing; use it as the first permitted external dataset comparison.
2. Obtain an independent GPU testbed and capture power-cap/DVFS changes with timestamped requests and verified power instrumentation.
3. Add request-level task quality and time-to-completion, not just throughput counters.
4. Add multirack synchronization and cross-site covariance from aligned workload traces.
5. Add cooling, UPS, storage and PCC measurement boundaries, then correlate with EMT/HIL/PHIL where available.
6. Add controlled repeated events and thermal/degradation tests before claiming sustainable flexibility.

## 10. Safety and claim ceiling

- This is read-only offline evaluation. It does not execute control actions.
- A Pareto result is not permission to offer grid services or alter GPU power limits.
- Low-rate workload data cannot validate package/VRM microsecond transients.
- No high-voltage energization, grid dispatch, battery fault injection or destructive tests are authorized.
- Simulation, vendor-reported, public dataset, physical measurement and independent qualification remain distinct evidence types.
