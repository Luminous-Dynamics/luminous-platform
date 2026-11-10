# Compute-Preserving Grid-to-Chip Power Co-Design Benchmark v0.1

Research snapshot: 10 November 2026.

Status: **research specification and offline Rust audit implementation; authored tests are not yet executed**. The module accepts supplied time-series data. It is not a workload scheduler, power controller, EMT solver, grid-code evaluator, hardware test, or qualification authority.

## 1. The problem is changing

The optimization target is no longer a converter in isolation. It is the useful computation delivered through a physical power chain while preserving processor power integrity and the grid's operating envelope.

The relevant system spans:

workload -> accelerator die -> package / interposer -> board VRM -> rack bus -> power shelf -> facility bus -> PCC -> utility network

Cooling, UPS, batteries, EDLC, generators and grid-forming storage sit across different physical and temporal boundaries. Workload scheduling is a controllable demand-side resource, but it is not a substitute for local electrical regulation or independent protection.

Three recent strands make an integrated benchmark timely:

1. **Workload composition matters.** Microsoft Research's March 2026 *From Servers to Sites* paper models workload-state transitions and configuration-specific power distributions, then composes power from servers through racks, rows and facilities. The authors report median absolute energy error below 5% for most evaluated configurations while preserving temporal autocorrelation. This is a stronger starting point than flat rack nameplate power or a single static trace.
2. **Sites are not independent by default.** The August 2026 IEEE study *Spatially Correlated Load Dynamics in AI Data-Center-Dominated Grids* reports around 7x aggregate active-power variability and 11x frequency standard deviation relative to its independent-load reference in its studied IEEE 39-bus system. These are model- and network-specific outcomes, not universal planning multipliers. The key engineering implication is to measure and model covariance rather than assume geographic diversity.
3. **Facility dynamics are coupled.** PNNL's October 2026 EMT study includes UPS, co-generation, cooling and grid-forming storage in one data-centre model, and studies both fast load fluctuations and low-voltage faults. IEEE P4200, approved as an active project on 4 June 2026, is developing data-centre interconnection requirements covering ramp-rate management, oscillation, ride-through, post-fault recovery, power quality, monitoring and supporting models/data.

At the chip end, Google's 2026 lateral-versus-vertical power-delivery paper examines accelerator currents above 10 kA and couples PI, SI, thermal, mechanical/warpage and manufacturing trade-offs. A November 2026 industry VRM article describes a 16-phase 40 A to 360 A event at 800 A/us. That transient is a useful reproduction fixture; it is not a universal processor rail requirement.

Meanwhile, OCP is coordinating an open 800 VDC architecture with Google, Microsoft and NVIDIA and reports more than 80 ecosystem partners. Luminous should therefore optimize for interoperability and evidence portability, not assume it can establish a competing proprietary distribution standard.

## 2. Paradigm shifts

### 2.1 From peak efficiency to useful-work energy

A converter with better peak efficiency can still produce a worse data-centre outcome if it forces extra conversion stages, increases cooling demand, reduces accelerator utilization, or causes workload delay.

The primary service metric should be:

\[
E_{\mathrm{useful}} =
\frac{E_{\mathrm{PCC}}-\Delta E_{\mathrm{stored}}}
     {W_{\mathrm{completed}}}
\]

where W_completed is a declared, application-specific unit of useful work and:

\[
\Delta E_{\mathrm{stored}} =
E_{\mathrm{stored,final}}-E_{\mathrm{stored,initial}}
\]

The storage correction prevents a candidate from appearing more efficient simply because it depleted a battery or capacitor during the measurement window.

The unit of work must be meaningful:
- LLM inference: completed output tokens and accepted requests, stratified by prompt/output length and model/precision, with p50/p95/p99 latency and quality/SLO guardrails.
- Training: tokens or samples completed to a fixed validation-loss / quality target, with time-to-target and checkpoint/restart cost.
- Scientific/HPC workloads: validated application work or time-to-solution, not theoretical FLOPs alone.

### 2.2 From flat load assumptions to compositional workload traces

Power demand should be generated from workload state, model/hardware configuration, concurrency and scheduling. At minimum preserve prefill, decode, idle, memory-bound, communication-bound, checkpoint and cooling/thermal-control effects where applicable.

A generated trace must retain the measured trace sources, calibration and model revision, state-transition model, distributional assumptions and residual error. A synthetic trace is a scenario, not a measurement. Autocorrelation and burst structure matter as much as average energy.

### 2.3 From independent sites to covariance-aware fleets

For site powers P_i(t), the aggregate fluctuation is:

\[
\operatorname{Var}\left(\sum_i P_i\right)
=
\sum_i \operatorname{Var}(P_i)
+
2\sum_{i<j}\operatorname{Cov}(P_i,P_j)
\]

The zero-covariance reference is:

\[
\sigma_{\mathrm{ind}} =
\sqrt{\sum_i \sigma_i^2}
\]

Luminous should report the measured aggregate RMS fluctuation and the ratio to this reference. This is a transparent covariance decomposition, not a grid-frequency simulation. Lagged correlation, spectral coherence and network propagation are later model stages; this v0.1 implementation only evaluates synchronous, zero-lag covariance.

### 2.4 From one buffer to timescale-separated actuators

Do not ask a single technology or controller to absorb every disturbance.

| Boundary / timescale | Main responsibility | Candidate mechanisms |
|---|---|---|
| Die/package, ns to µs | Local rail integrity and current-loop response | Package/interposer PDN, on-die and package decoupling, VRM |
| Board/rack, µs to ms | Bus transient and short hold-up | Local capacitors, bus capacitors, EDLC |
| Rack/facility, ms to seconds | Power ramp and short-duration reserve | Converter-coupled storage, UPS, battery |
| Facility/grid, seconds to minutes | Voltage/frequency support, reserve and recovery | Grid-forming ESS, generator, plant dispatch |
| Workload, seconds and longer | Shift or shape deferrable demand within service contracts | Admission, queueing, scheduling, power caps |

The exact crossover times depend on hardware and control loops. The table defines separate responsibility classes, not universal bandwidth cut-offs. Local protection and deterministic controls must remain functional without cloud or workload-orchestrator availability.

### 2.5 From isolated models to a common evidence thread

A common experiment identity should link:
- workload and software/configuration revision;
- accelerator, package, board, VRM and rack/power-shelf revisions;
- storage SOC and component operating state;
- measurement boundaries, channel definitions, clock domains and uncertainty;
- raw trace hashes, calibration references, instrument bandwidth and test procedure;
- model/simulator revision, parameters and correlation report;
- independent review, negative controls, anomalies and repair/requalification history.

This is where Luminous can differentiate: reproducible, cross-layer engineering evidence that can travel between suppliers and operators without pretending that one model validates every boundary.

## 3. Benchmark structure

A serious benchmark must score each level separately and then publish the end-to-end result.

| Level | Required measurements / outputs | Main comparator |
|---|---|---|
| 0. Evidence validity | Trace identity, hashes, calibration, bandwidth, sample quality, synchronization, uncertainty | Reproducibility and data-quality controls |
| 1. Useful compute | Accepted work, quality, throughput, p50/p95/p99 latency, time-to-target | Same workload, model, precision and service contract |
| 2. Die/package PDN | Z(f), target-impedance margin, load-step response, die-side voltage, current density, package temperature, warpage | Reproduced LPD baseline vs VPD / coupled-inductor variants |
| 3. Board/rack conversion | Efficiency maps, full-load and peak efficiency, loss breakdown, transient response, ripple, thermal gradients, phase/cell sharing, density and serviceability | Same voltage, power, cooling, workload and measurement boundary |
| 4. Storage | Charge/discharge kW, usable kWh, SOC range, response, round-trip loss, cycle/thermal ageing, degradation and availability | Same load trace, initial SOC, endpoint SOC accounting and service duration |
| 5. Facility/PCC | Active/reactive power, RMS and percentile ramp, band-limited PSD, PCC voltage/frequency, harmonics/power quality, nadir, overshoot, recovery, generator interaction | Same grid model, workload, facility cooling and protection assumptions |
| 6. Multi-site / grid | Cross-correlation, lagged correlation, aggregate ramp/reserve, grid strength and network response | Independent-load null vs observed correlated workload |
| 7. Lifecycle | Availability, failure rate, MTTR, repairability, requalification scope, component replacement, materials and total cost | Equivalent useful work and operational duty cycle |

Peak efficiency, energy density and one transient voltage minimum remain useful measurements, but none is a system-level verdict.

## 4. Matched-comparison protocol

The v0.1 protocol is deliberately strict:

1. Freeze the workload, work-unit definition, model/hardware configuration, boundary, PCC reference target and measurement window.
2. Capture baseline and candidate with the same exact timestamp vector. v0.1 rejects resampling and hidden interpolation between candidates.
3. Require valid samples, increasing timestamps, declared sample-gap limits and clock synchronization within budget.
4. Integrate PCC energy and useful work over irregular intervals using piecewise-linear trapezoidal integration.
5. Integrate squared power exactly for a piecewise-linear segment. Report both RMS fluctuation around the trace mean and RMS tracking error around the declared common PCC reference.
6. Correct effective input energy for the change in stored energy.
7. Report energy per useful-work unit, useful-work retention, PCC variability, tracking error, peak deviation and up/down ramp independently.
8. Evaluate multi-site covariance only with already-synchronized traces on identical timestamp vectors. Missing samples are not filled.
9. Preserve negative controls. Better smoothing does not count as improvement if useful work falls outside the declared loss budget.

For a site with AC service, PCC active power alone is not the complete power-quality record. Retain three-phase voltage/current, reactive power, power factor, frequency, harmonics and applicable measurement class at the PCC. The v0.1 Rust module currently audits active-power time series only.

## 5. Initial research gates

These are internal proposed goals, not IEEE/OCP requirements and not achieved Luminous results.

| Metric | Initial gate | Stretch | Qualification caveat |
|---|---:|---:|---|
| Useful-work retention | >=99.5% | >=99.9% | Same completed-work definition and application quality; also report tail latency/time-to-target |
| PCC RMS tracking-error reduction | >=20% | >=50% | Against same workload and reference target; must not worsen energy/work |
| PCC RMS fluctuation reduction | >=20% | >=50% | Use identical sample bandwidth and window |
| PCC up/down ramp | >=20% reduction in selected percentiles | >=40% | Publish sample rate, filter, event definition and maximum too |
| Effective PCC energy per useful work | No worse beyond combined uncertainty | >=3% lower | Include storage delta, cooling and all agreed facility loads |
| Processor rail transient | Reproduce the public 40 A -> 360 A / 800 A/us fixture | Improve the full waveform and recovery | Do not claim vendor waveform reproduction from a synthetic fixture |
| Package target impedance | Pass declared Z_target(f) with margin over the workload band | Lower impedance / less decoupling volume at equal thermal and mechanical constraints | No universal scalar threshold; derive from rail limits and load-current spectrum |
| Multi-site load model | Publish covariance and independent-reference amplification | Improve forecast error and reserve sizing on held-out traces | No assumed independence and no universal 7x factor |

Do not turn the 20%/50% PCC fluctuation targets into headline claims until throughput, time-to-completion, energy/work, cooling, availability and grid constraints are all held constant or explicitly accounted for.

## 6. How Luminous compares with the field

| Field frontier | Publicly visible state | Luminous opportunity | What would count as a real advance |
|---|---|---|---|
| 800 VDC distribution | OCP workstream, Google/Microsoft/NVIDIA alignment and a growing supplier ecosystem | Open evidence and qualification thread across multiple compliant implementations | Interoperability tests and complete loss/protection/grounding evidence, not a new incompatible bus |
| Grid interconnection | IEEE P4200 project is active and specifies data-centre-specific performance/model needs | P4200-oriented trace, model and evidence package | Mapping to published requirements after the standard is finalized; no pre-claim of conformity |
| Workload-to-site traces | Compositional LLM power synthesis and high-resolution GPU workload datasets exist | Reproducible trace lineage, useful-work normalization and cross-layer matched replay | Lower held-out energy/trace error without losing autocorrelation, workload quality or rare transients |
| Correlated grid loads | IEEE time-domain/spectral and EMT studies have shown correlations can materially alter aggregate behaviour | Generalized covariance/lag-aware fleet benchmark with trace lineage | Better predictions on independent multi-site data and validated network models |
| Processor delivery | 16-phase high-current VRM transient examples; package VPD research is active | Reproducible transient plus Z(f), package geometry and thermal/mechanical coupling | Full bandwidth, de-embedded impedance, matched workload and correlated package/lab measurements |
| Grid-forming storage | EMT studies model UPS, generation, cooling and GFM ESS as a coupled facility | Cross-timescale evidence tying storage response to preserved useful compute | Validated EMT/HIL/PHIL and eventually site measurements with delay/fallback/availability evidence |

The novelty is not that each subsystem has never been studied. The opportunity is to make the **same useful-work experiment** traverse all these boundaries with explicit identities, uncertainty, and evidence ceilings.

## 7. What v0.1 implements

The new Rust module at tools/measurement-audit/src/compute_power_benchmark.rs adds:

- audit_compute_power_trace: validates a typed PCC active-power / useful-work trace, integrates energy and work, corrects storage state change, and reports exact piecewise-linear RMS and ramp metrics.
- compare_compute_power_traces: rejects mismatched workload/boundary/target/initial SOC/timestamp vectors and reports useful-work retention plus independent energy and grid-shaping metrics.
- audit_multisite_load_correlation: calculates time-weighted site variances and synchronous pairwise covariance, then compares actual aggregate fluctuation against the zero-covariance reference.

The module includes synthetic unit tests for irregular timestamps, storage depletion, matched workload preservation, a deliberately smoothed but under-performing candidate, synchronized/in-phase and anti-phase sites, and fail-closed data-quality conditions.

**Validation state:** Rust tests are authored, not executed. No rustc, Cargo, rustfmt or Clippy run is claimed in the current environment. No schema validator, EMT, HIL/PHIL, physical prototype, site data or independent certification is claimed. The existing PR #379 transient fixture remains a synthetic arithmetic fixture, not a reproduced vendor waveform.

## 8. High-value next increments

1. **Frequency-dependent package PDN.** Derive Z_target(f) from a declared rail tolerance and transient-current spectrum; implement a de-embedded impedance import format, anti-resonance checks, bandwidth/uncertainty gates, and time-domain correlation. Keep the scalar Delta V / Delta I bound as a coarse screen only.
2. **Workload trace adapters and replay.** Add measured GPU/LLM training and inference data adapters, state composition, sequence/traffic mix, thermal/cooling channels and held-out trace generation. Maintain immutable raw data hashes and data licensing.
3. **Power-system dynamic twin.** Couple facility, UPS, EDLC/battery, generator, GFM controls and network impedance in a declared EMT co-simulation; add controller delay, jitter, packet loss, saturation and local fallback. Validate each model before composing it.
4. **Evidence manifest and requalification graph.** Bind component/board/package revisions, model versions, measurements, uncertainty, repair records and supplier conformance claims. A changed package stack, magnetic, capacitor, firmware or control revision should invalidate only the affected evidence set, with dependency edges explaining why.
5. **Independent benchmark corpus.** Publish synthetic fixtures plus legally redistributable measured traces, negative controls and exact evaluation scripts. Hold out workloads/sites/packages for blind comparisons.

## 9. Safety, authority and claim ceilings

- This software is read-only. It does not issue power, workload, charging, islanding or protection commands.
- Workload shaping is a performance-affecting supervisory choice; it must never be the only response to a fast electrical disturbance.
- Local deterministic regulation, protection, interlocks and safe failure behavior remain separate and independently qualified.
- No high-voltage construction, energization, grid connection, high-energy storage test, arc-fault test or package fabrication is authorized by this analysis.
- Synthetic, vendor-reported, analytical, simulated, correlated-simulation, measured, HIL/PHIL and independently qualified evidence remain distinct.
- A benchmark audit pass means only that the declared data and calculation contract passed. It does not mean the system is safe, grid compliant, manufacturable, reliable or better than SOTA.

## Sources

- Microsoft Research, March 2026: [From Servers to Sites: Compositional Power Trace Generation of LLM Inference for Infrastructure Planning](https://www.microsoft.com/en-us/research/publication/from-servers-to-sites-compositional-power-trace-generation-of-llm-inference-for-infrastructure-planning/).
- IEEE Transactions on Power Systems, 31 August 2026: [Spatially Correlated Load Dynamics in AI Data-Center-Dominated Grids](https://ieeexplore.ieee.org/abstract/document/11673204/).
- PNNL / IEEE Access, 16 October 2026: [Using Grid-Forming Energy Storage Systems to Provide Dynamic Active Power Support for Hyperscale Data Centers](https://www.pnnl.gov/publications/using-grid-forming-energy-storage-systems-provide-dynamic-active-power-support).
- IEEE SA: [P4200 — Data Center Interconnection and Interoperability](https://standards.ieee.org/ieee/4200/12632/).
- Open Compute Project: [Powering the Next Era of AI: LVDC transition](https://www.opencompute.org/news/powering-the-next-era-of-ai-how-google-microsoft-and-nvidia-are-standardizing-and-accelerating-the-industry-transition-to-lvdc/).
- Google Research / IEEE Transactions on Components, Packaging and Manufacturing Technology (2026): [Towards 10kAmp AI Chip](https://research.google/pubs/towards-10kamp-ai-chip-a-system-perspective-from-lateral-to-vertical-power-delivery/).
- EEWorld, 4 November 2026: [How do multiphase VRMs support AI accelerator transient loads?](https://www.eeworldonline.com/how-do-multiphase-vrms-support-ai-accelerator-transient-loads/).
- Scientific Data (2026): [Characterization of high-resolution AI data center training workloads on single and multiple GPU nodes](https://www.nature.com/articles/s41597-026-07496-6).
